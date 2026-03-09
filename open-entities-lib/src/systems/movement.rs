use crate::components::{Position, Velocity};
use bevy_ecs::prelude::*;

/// System: Update position based on velocity
pub fn move_system(mut query: Query<(&mut Position, &Velocity)>) {
    for (mut pos, vel) in &mut query {
        pos.x += vel.vx;
        pos.y += vel.vy;
    }
}
