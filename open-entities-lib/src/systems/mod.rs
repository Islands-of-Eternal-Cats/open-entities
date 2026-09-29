//! The ECS systems [`Core`](crate::Core) runs each tick of [`TICK_MS`](crate::TICK_MS), in order: mission steering, boarding
//! approach, seek, movement, passenger sync, mission completion, replanner.
//!
//! Public for hosts that build their own schedule through [`Api::core_mut`](crate::Api::core_mut);
//! [`Api::step`](crate::Api::step) already runs them.

mod boarding;
mod missions;
mod movement;
mod seek;

pub use boarding::{boarding_approach_system, passenger_sync_system};
pub use missions::{mission_completion_system, mission_steering_system, replanner_system};
pub use movement::movement_system;
pub use seek::seek_system;

/// Distance at or below which seek treats the entity as arrived (world units).
pub const ARRIVAL_THRESHOLD: f32 = 0.1;
