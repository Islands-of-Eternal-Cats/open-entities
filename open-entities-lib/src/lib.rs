//! Headless RTS entity simulation built on `bevy_ecs`, driven through one facade: [`Api`].
//!
//! # Integration boundary
//!
//! A host (game client, WASM bindings, tests) talks to the library through [`Api`] only:
//!
//! | Concern | Calls |
//! |---|---|
//! | Content | [`Api::load_templates_yaml`], [`Api::spawn_entity`], [`Api::load_map_yaml`] |
//! | Time | [`Api::tick`] |
//! | Orders | [`Api::order_move_to`], [`Api::order_stop`] |
//! | Groups | [`Api::create_group`], [`Api::add_to_group`], [`Api::order_group_move_to`], … |
//! | Missions | [`Api::create_mission`], [`Api::assign_group`], [`Api::is_mission_completed`], … |
//! | Boarding | [`Api::board`], [`Api::order_board`], [`Api::unboard`], [`Api::passengers`], … |
//! | Lifecycle | [`Api::is_alive`], [`Api::despawn`] |
//! | State out | [`Api::world_snapshot`] |
//!
//! Every entity is named by an [`EntityId`] — an `{index, generation}` pair. It is the same pair
//! [`Api::world_snapshot`] reports as each entity's `id`, so ids from a snapshot can be fed straight
//! back into orders. A despawned id never resolves again, even when its index is reused.
//!
//! Data goes in as YAML ([`EntityComponents`] is the shape of a template, a spawn override and a
//! map entry) and comes out as a [`WorldSnapshot`] — plain data with a stable `Serialize` shape,
//! so the host picks the wire format (the WASM bindings send JSON). No `bevy_ecs` type appears on
//! this path, so the ECS version is not part of the contract.
//!
//! [`Api::core`] / [`Api::core_mut`] are the deliberate exception: they expose the ECS
//! [`World`](bevy_ecs::world::World) for custom systems and queries. Code that goes through them,
//! or uses [`components`] and [`systems`] directly, is tied to this crate's `bevy_ecs` version.
//!
//! # Quick start
//!
//! ```
//! use open_entities::components::{MoveTarget, Position};
//! use open_entities::{Api, EntityComponents};
//!
//! let mut api = Api::new();
//! api.load_templates_yaml(
//!     "entities:\n  scout:\n    base_move_speed: 5.0\n    velocity: { vx: 0.0, vy: 0.0 }\n",
//! )?;
//!
//! let scout = api.spawn_entity(
//!     "scout",
//!     EntityComponents {
//!         position: Some(Position { x: 0.0, y: 0.0 }),
//!         ..Default::default()
//!     },
//! )?;
//!
//! let report = api.order_move_to(&[scout], MoveTarget { x: 10.0, y: 0.0 });
//! assert_eq!(report.ordered, 1);
//!
//! for _ in 0..10 {
//!     api.tick(100)?; // milliseconds; 0 is an error, values above 100 are clamped
//! }
//!
//! let snapshot = api.world_snapshot();
//! assert_eq!(snapshot.entities.len(), 1);
//! assert_ne!(snapshot.entities[0].components.position, Some(Position { x: 0.0, y: 0.0 }));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Errors
//!
//! Each operation family has its own error enum ([`ImportError`], [`MapError`], [`TickError`],
//! [`GroupError`], [`MissionError`], [`BoardError`]); all implement
//! [`std::error::Error`] and [`Display`](std::fmt::Display). Orders that take many ids do not fail:
//! they skip what they cannot apply and say so in an [`OrderReport`].
//!
//! Behavioural contracts live in `docs/design/` (group missions, vehicle seats).

#![warn(missing_docs)]
#![warn(clippy::pedantic)]
// Exact float comparison is the point in tests: the systems snap a value to its target, and a snap
// either happened or it did not. Epsilon comparisons there would stop testing what is being tested.
#![cfg_attr(test, allow(clippy::float_cmp))]

pub mod api;
pub mod boarding;
pub mod components;
pub mod core;
pub mod export;
pub mod groups;
pub mod import;
pub mod map;
pub mod missions;
pub mod orders;
pub mod simulation;
pub mod systems;

mod component_registry;
mod entity_components;

pub use api::Api;
pub use boarding::{BOARDING_RANGE, BoardError};
pub use core::Core;
pub use entity_components::EntityComponents;
pub use export::{EntitySnapshot, WorldSnapshot};
pub use groups::GroupError;
pub use import::ImportError;
pub use map::{MapBounds, MapError};
pub use missions::MissionError;
pub use orders::{EntityId, OrderReport};
pub use simulation::TickError;

/// Returns the canonical hello-world greeting.
#[must_use]
pub const fn hello() -> &'static str {
    "Hello, world!"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_returns_greeting() {
        assert_eq!(hello(), "Hello, world!");
    }
}
