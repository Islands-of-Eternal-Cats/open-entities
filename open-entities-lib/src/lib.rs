//! Headless RTS entity simulation built on `bevy_ecs`, driven through one facade: [`Api`].
//!
//! # Integration boundary
//!
//! A host (game client, WASM bindings, tests) talks to the library through [`Api`] only:
//!
//! | Concern | Calls |
//! |---|---|
//! | Content | [`Api::load_templates_yaml`], [`Api::spawn_entity`], [`Api::load_map_yaml`] |
//! | Time | [`Api::step`], [`Api::current_tick`] |
//! | Commands | [`Api::submit`], [`Api::schedule`] → [`StepReport`] from [`Api::step`] |
//! | Determinism | [`Api::state_hash`], [`Replay::run`] |
//! | Orders | [`Api::order_move_to`], [`Api::order_stop`] |
//! | Groups | [`Api::create_group`], [`Api::add_to_group`], [`Api::order_group_move_to`], … |
//! | Missions | [`Api::create_mission`], [`Api::assign_group`], [`Api::is_mission_completed`], … |
//! | Boarding | [`Api::board`], [`Api::order_board`], [`Api::unboard`], [`Api::passengers`], … |
//! | Lifecycle | [`Api::is_alive`], [`Api::despawn`] |
//! | State out | [`Api::world_snapshot`] (debugging, saves) |
//! | Render | [`Api::write_frame`] (positions, every tick), [`Api::meta_delta`] (the rest, on change) |
//! | Extension | [`Api::register_component`], [`extend::add_systems`], [`impl_state_hash_via_serde!`] |
//!
//! Every entity is named by an [`EntityId`] — an `{index, generation}` pair. It is the same pair
//! [`Api::world_snapshot`] reports as each entity's `id`, so ids from a snapshot can be fed straight
//! back into orders. A despawned id never resolves again, even when its index is reused.
//!
//! Orders reach a running match as [`Command`]s: data, applied at the start of a known tick, so
//! two peers with the same command log stay in step and a [`Replay`] reproduces a match exactly.
//! The immediate order methods in the table are what commands call; a networked host uses
//! [`Api::submit`] and [`Api::schedule`] only.
//!
//! Data goes in as YAML ([`EntityComponents`] is the shape of a template, a spawn override and a
//! map entry) and comes out as a [`WorldSnapshot`] — plain data with a stable `Serialize` shape,
//! so the host picks the wire format (the WASM bindings send JSON). No `bevy_ecs` type appears on
//! this path, so the ECS version is not part of the contract.
//!
//! [`Api::core`] / [`Api::core_mut`] and the [`extend`] module are the deliberate exceptions: they
//! expose the ECS [`World`](bevy_ecs::world::World) and the schedule for a game's own components,
//! systems and queries. Code that goes through them, or uses [`components`] and [`systems`]
//! directly, is tied to this crate's `bevy_ecs` version.
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
//!         position: Some(Position::from_units(0.0, 0.0)),
//!         ..Default::default()
//!     },
//! )?;
//!
//! let report = api.order_move_to(&[scout], MoveTarget::from_units(10.0, 0.0));
//! assert_eq!(report.ordered, 1);
//!
//! for _ in 0..20 {
//!     api.step(); // one tick of TICK_MS (50 ms)
//! }
//! assert_eq!(api.current_tick(), 20);
//!
//! let snapshot = api.world_snapshot();
//! assert_eq!(snapshot.entities.len(), 1);
//! // Simulation state is integer milli-units: 5 units/s is 250 per tick, 20 ticks reach 5.0.
//! assert_eq!(snapshot.entities[0].components.position, Some(Position { x: 5000, y: 0 }));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Errors
//!
//! Each operation family has its own error enum ([`ImportError`], [`MapError`],
//! [`GroupError`], [`MissionError`], [`BoardError`], [`CommandError`], [`ScheduleError`],
//! [`ReplayError`], [`RegisterError`], [`ComponentError`]); all implement
//! [`std::error::Error`] and [`Display`](std::fmt::Display). Orders that take many ids do not fail:
//! they skip what they cannot apply and say so in an [`OrderReport`].
//!
//! Behavioural contracts live in `docs/design/` (group missions, vehicle seats).

#![warn(missing_docs)]
#![warn(clippy::pedantic)]

pub mod api;
pub mod boarding;
pub mod commands;
pub mod components;
pub mod core;
pub mod export;
pub mod extend;
pub mod groups;
pub mod import;
pub mod map;
pub mod missions;
pub mod orders;
pub mod replay;
pub mod simulation;
pub mod state_hash;
pub mod systems;
pub mod units;

mod component_registry;
mod entity_components;

pub use api::Api;
pub use boarding::{BOARDING_RANGE, BoardError};
pub use commands::{
    Command, CommandError, CommandOutcome, CommandResult, CommandSeq, ScheduleError, StepReport,
};
pub use component_registry::ComponentError;
pub use core::Core;
pub use entity_components::EntityComponents;
pub use export::{
    EntityMeta, EntitySnapshot, FRAME_HEADER_LEN, FRAME_STRIDE, MetaDelta, WorldSnapshot,
};
pub use extend::RegisterError;
pub use groups::GroupError;
pub use import::ImportError;
pub use map::{MapBounds, MapError};
pub use missions::MissionError;
pub use orders::{EntityId, OrderReport};
pub use replay::{REPLAY_VERSION, Replay, ReplayCommand, ReplayError};
pub use simulation::TICK_MS;
pub use state_hash::StateHash;
pub use units::MILLI_PER_UNIT;

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
