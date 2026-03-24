//! Time resource for frame-based simulation.

use bevy_ecs::prelude::*;

/// Delta time in seconds for the current tick.
/// Insert before running the update schedule so systems (e.g. movement) use real time.
#[derive(Resource, Clone, Copy, Debug)]
pub struct DeltaTime(pub f32);
