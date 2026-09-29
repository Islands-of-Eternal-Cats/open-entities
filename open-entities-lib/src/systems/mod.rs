//! The ECS systems [`Core`](crate::Core) runs each tick, in order: mission steering, boarding
//! approach, seek, movement, passenger sync, mission completion, replanner.
//!
//! Public for hosts that build their own schedule through [`Api::core_mut`](crate::Api::core_mut);
//! [`Api::tick`](crate::Api::tick) already runs them.

mod boarding;
mod missions;
mod movement;
mod seek;

pub use boarding::{boarding_approach_system, passenger_sync_system};
pub use missions::{mission_completion_system, mission_steering_system, replanner_system};
pub use movement::movement_system;
pub use seek::seek_system;

/// Maximum allowed tick delta (milliseconds); larger values are clamped.
pub const MAX_DT_MS: u32 = 100;

/// Distance at or below which seek treats the entity as arrived (world units).
pub const ARRIVAL_THRESHOLD: f32 = 0.1;
