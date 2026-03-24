use crate::components::{Position, Velocity};
use crate::systems::DeltaTime;
use bevy_ecs::prelude::*;

/// System: Update position based on velocity and delta time.
/// Uses `Res<DeltaTime>` when present; otherwise assumes 1.0 (backward compatible).
pub fn move_system(mut query: Query<(&mut Position, &Velocity)>, dt: Option<Res<DeltaTime>>) {
    let dt_sec = dt.map(|d| d.0).unwrap_or(1.0);
    for (mut pos, vel) in &mut query {
        pos.x += vel.vx * dt_sec;
        pos.y += vel.vy * dt_sec;
    }
}
