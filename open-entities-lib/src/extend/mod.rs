//! Extending the engine from a Rust game: its own components and systems, without a fork.
//!
//! - [`Api::register_component`] makes a component a field of YAML templates, spawn overrides and
//!   map entries, puts it in the world export, and feeds it into the state hash.
//! - [`add_systems`] runs a game's systems in one of the [`SimSet`] phases of every tick.
//! - [`impl_state_hash_via_serde!`](crate::impl_state_hash_via_serde) derives [`StateHash`] from
//!   the component's `Serialize` impl.
//!
//! Both calls belong to setup: components before [`Api::load_templates_yaml`], and nothing after
//! the first [`Api::step`]. Every peer of a lockstep match, and every replay of it, must make the
//! same calls in the same order.
//!
//! Like [`Api::core_mut`], this module is a documented door out of the `bevy_ecs`-free API: a
//! game's components and systems are `bevy_ecs` types, so code that goes through it is tied to
//! the `bevy_ecs` version this crate builds against.
//!
//! JavaScript cannot register Rust components. A game that does register some still sees them in
//! `getWorldAsJson()`, which is the same [`WorldSnapshot`](crate::WorldSnapshot) export.

mod serde_hash;

use bevy_ecs::prelude::Component;
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_ecs::system::ScheduleSystem;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::api::Api;
use crate::component_registry::custom_descriptor;
use crate::state_hash::{StateHash, StateHasher};

pub use crate::simulation::SimSet;

/// The lowest [`StateHash::TAG`] a game's component may use; the tags below belong to the engine.
pub const FIRST_GAME_TAG: u8 = 64;

/// Keys that mean something else in a template, a map entry or an export row.
const RESERVED_FIELDS: [&str; 4] = ["", "template", "id", "entity_type"];

/// Why [`Api::register_component`] or [`add_systems`] refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegisterError {
    /// A component is already registered under this field — a built-in or an earlier call.
    DuplicateField(String),
    /// The field is empty or a key with another meaning: `template`, `id`, `entity_type`.
    ReservedField(String),
    /// Another registered component already uses this hash tag.
    DuplicateTag {
        /// The tag.
        tag: u8,
        /// The field being registered.
        field: String,
        /// The field that has the tag.
        existing: String,
    },
    /// The tag is below [`FIRST_GAME_TAG`], in the engine's range.
    ReservedTag(u8),
    /// The component cannot be part of the state hash: it holds an `f32`/`f64`, or its serde
    /// shape cannot be checked. Simulation state is integers.
    NotHashable {
        /// The field being registered.
        field: String,
        /// What was found, and where in the type.
        reason: String,
    },
    /// Templates are already loaded: they were checked without this component.
    TemplatesLoaded,
    /// The match has started; a component or system added now would desync from every peer and
    /// replay that did not add it at the same point.
    AfterFirstStep {
        /// [`Api::current_tick`] at the time.
        tick: u64,
    },
    /// The schedule did not build with the added systems.
    Schedule(String),
}

impl std::fmt::Display for RegisterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateField(field) => {
                write!(f, "component field `{field}` is already registered")
            }
            Self::ReservedField(field) => write!(f, "`{field}` cannot be a component field"),
            Self::DuplicateTag {
                tag,
                field,
                existing,
            } => write!(
                f,
                "state hash tag {tag} of `{field}` is already used by `{existing}`"
            ),
            Self::ReservedTag(tag) => write!(
                f,
                "state hash tag {tag} is reserved for the engine; game components use {FIRST_GAME_TAG} and up"
            ),
            Self::NotHashable { field, reason } => {
                write!(f, "component `{field}` cannot be hashed: {reason}")
            }
            Self::TemplatesLoaded => f.write_str("register components before load_templates_yaml"),
            Self::AfterFirstStep { tick } => write!(
                f,
                "the simulation is at tick {tick}; extend it before the first step"
            ),
            Self::Schedule(message) => write!(f, "the schedule does not build: {message}"),
        }
    }
}

impl std::error::Error for RegisterError {}

impl Api {
    /// Registers a game's component under `field`.
    ///
    /// From then on `field` is a component key in YAML templates, spawn overrides
    /// ([`EntityComponents::extra`](crate::EntityComponents::extra)), map entries and
    /// [`Command::Spawn`](crate::Command::Spawn); the component appears under `field` in
    /// [`Api::world_snapshot`] and is hashed by [`Api::state_hash`] after the built-ins, in
    /// registration order, with its [`StateHash::TAG`].
    ///
    /// The type's serde shape is checked once, here: a float anywhere in it — a field, an option,
    /// a list element, an enum variant — is refused, because simulation state is integers.
    ///
    /// # Errors
    ///
    /// [`RegisterError::AfterFirstStep`] after the first [`Api::step`],
    /// [`RegisterError::TemplatesLoaded`] after [`Api::load_templates_yaml`],
    /// [`RegisterError::ReservedField`], [`RegisterError::DuplicateField`],
    /// [`RegisterError::ReservedTag`], [`RegisterError::DuplicateTag`], and
    /// [`RegisterError::NotHashable`] for a float or a shape that cannot be checked. Nothing is
    /// registered on error.
    pub fn register_component<T>(&mut self, field: &str) -> Result<(), RegisterError>
    where
        T: Component + Serialize + DeserializeOwned + StateHash,
    {
        self.check_not_started()?;
        if self.templates.is_some() {
            return Err(RegisterError::TemplatesLoaded);
        }
        if RESERVED_FIELDS.contains(&field) {
            return Err(RegisterError::ReservedField(field.to_owned()));
        }
        if self.registry.fields().any(|known| known == field) {
            return Err(RegisterError::DuplicateField(field.to_owned()));
        }
        if T::TAG < FIRST_GAME_TAG {
            return Err(RegisterError::ReservedTag(T::TAG));
        }
        serde_hash::check::<T>().map_err(|reason| RegisterError::NotHashable {
            field: field.to_owned(),
            reason,
        })?;
        self.registry.register(custom_descriptor::<T>(field))
    }

    /// Every registered component field, built-ins first, in registration order — the order of
    /// the state hash.
    #[must_use]
    pub fn component_fields(&self) -> Vec<&str> {
        self.registry.fields().collect()
    }

    fn check_not_started(&self) -> Result<(), RegisterError> {
        match self.current_tick() {
            0 => Ok(()),
            tick => Err(RegisterError::AfterFirstStep { tick }),
        }
    }
}

/// Runs `systems` in `set` on every tick, after the set's built-ins and after the systems added to
/// the set before, chained in the order given.
///
/// Every system of the simulation then has a fixed place in one line, so ambiguity detection
/// (an error in debug builds) passes and the run is deterministic as long as each system is:
/// integers only, ties broken by [`EntityId`](crate::EntityId), no iteration over `std`
/// `HashMap`/`HashSet`.
///
/// # Errors
///
/// [`RegisterError::AfterFirstStep`] after the first [`Api::step`]; [`RegisterError::Schedule`]
/// when the schedule does not build — the `Api` should then be dropped.
pub fn add_systems<M>(
    api: &mut Api,
    set: SimSet,
    systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
) -> Result<(), RegisterError> {
    api.check_not_started()?;
    api.core
        .add_user_systems(set, systems)
        .map_err(RegisterError::Schedule)
}

/// Feeds `value` into the state hash through its `Serialize` impl; what
/// [`impl_state_hash_via_serde!`](crate::impl_state_hash_via_serde) expands to.
///
/// Fields go in declaration order, integers little-endian at their own width, `bool` as one byte,
/// strings and byte strings with a `u64` length, options and enum variants with a marker, lists
/// and maps with a `u64` length.
///
/// # Panics
///
/// On an `f32`/`f64`, or a list or map whose length serde does not know up front.
/// [`Api::register_component`] checks the type's shape for both, so a registered component only
/// gets here with a float the check could not reach.
pub fn hash_via_serde<T: Serialize + ?Sized>(value: &T, hasher: &mut StateHasher) {
    if let Err(err) = serde_hash::write(value, hasher) {
        panic!("component cannot be hashed: {err}");
    }
}

/// Implements [`StateHash`](crate::StateHash) for a component through its `Serialize` impl, with
/// the given tag (64 and up; the engine owns the tags below).
///
/// ```
/// use bevy_ecs::prelude::Component;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Component, Serialize, Deserialize)]
/// struct Fuel(u32);
///
/// open_entities::impl_state_hash_via_serde!(Fuel, 64);
/// ```
///
/// A float in the type is refused by [`Api::register_component`](crate::Api::register_component).
#[macro_export]
macro_rules! impl_state_hash_via_serde {
    ($ty:ty, $tag:expr) => {
        impl $crate::StateHash for $ty {
            const TAG: u8 = $tag;
            fn hash_fields(&self, hasher: &mut $crate::state_hash::StateHasher) {
                $crate::extend::hash_via_serde(self, hasher);
            }
        }
    };
}
