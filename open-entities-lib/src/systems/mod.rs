pub mod movement;
pub mod seek;

pub use movement::movement_system;
pub use seek::seek_system;

/// Maximum allowed tick delta (milliseconds); larger values are clamped.
pub const MAX_DT_MS: u32 = 100;

/// Distance at or below which seek treats the entity as arrived (world units).
pub const ARRIVAL_THRESHOLD: f32 = 0.1;
