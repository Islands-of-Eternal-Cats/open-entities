use crate::components::{MoveTarget, Position, Velocity};
use bevy_ecs::prelude::*;

const STOP_DIST: f32 = 0.75;
const MOVE_SPEED: f32 = 45.0;

/// Steer velocity toward [`MoveTarget`], stop and remove target when close.
pub fn seek_move_target_system(
    mut commands: Commands,
    mut query: Query<(Entity, &Position, &mut Velocity, &MoveTarget)>,
) {
    for (entity, pos, mut vel, target) in &mut query {
        let dx = target.x - pos.x;
        let dy = target.y - pos.y;
        let dist_sq = dx * dx + dy * dy;
        if dist_sq <= STOP_DIST * STOP_DIST {
            vel.vx = 0.0;
            vel.vy = 0.0;
            commands.entity(entity).remove::<MoveTarget>();
        } else {
            let dist = dist_sq.sqrt();
            vel.vx = (dx / dist) * MOVE_SPEED;
            vel.vy = (dy / dist) * MOVE_SPEED;
        }
    }
}
