//! The ECS systems [`Core`](crate::Core) runs each tick of [`TICK_MS`](crate::TICK_MS), in order: mission steering, boarding
//! approach, seek, movement, passenger sync, mission completion, replanner.
//!
//! Public for hosts that build their own schedule through [`Api::core_mut`](crate::Api::core_mut);
//! [`Api::step`](crate::Api::step) already runs them.

#![deny(clippy::float_arithmetic)]

mod boarding;
mod missions;
mod movement;
mod seek;

pub use boarding::{boarding_approach_system, passenger_sync_system};
pub use missions::{mission_completion_system, mission_steering_system, replanner_system};
pub use movement::movement_system;
pub use seek::seek_system;

/// Distance at or below which seek treats the entity as arrived, in milli-units (0.1 map units).
pub const ARRIVAL_RADIUS: i32 = 100;

/// Length of `(dx, dy)` in milli-units, truncated: the integer square root of the squared length.
///
/// The squares are summed in `u128`: two `i32` differences squared can together exceed `u64`.
#[must_use]
pub fn length(dx: i64, dy: i64) -> u64 {
    // The root of a sum of two squares of values below 2^33 is below 2^34.
    #[allow(clippy::cast_possible_truncation)]
    let root = squared_length(dx, dy).isqrt() as u64;
    root
}

/// `dx² + dy²`, exact.
#[must_use]
pub fn squared_length(dx: i64, dy: i64) -> u128 {
    let dx = u128::from(dx.unsigned_abs());
    let dy = u128::from(dy.unsigned_abs());
    dx * dx + dy * dy
}

/// `true` when `(dx, dy)` is no longer than `range` (negative counts as zero); exact, no square root.
#[must_use]
pub fn within(dx: i64, dy: i64, range: i32) -> bool {
    let range = u128::from(range.max(0).unsigned_abs());
    squared_length(dx, dy) <= range * range
}
