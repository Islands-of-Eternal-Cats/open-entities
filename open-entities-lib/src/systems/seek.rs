use crate::components::{DEFAULT_MAX_SPEED, MaxSpeed, MoveTarget, Position, Velocity};
use bevy_ecs::prelude::*;

const STOP_DIST: f32 = 0.75;

/// Steer velocity toward [`MoveTarget`], stop and remove target when close.
pub fn seek_move_target_system(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &Position,
        &mut Velocity,
        &MoveTarget,
        Option<&MaxSpeed>,
    )>,
) {
    for (entity, pos, mut vel, target, max_speed) in &mut query {
        let speed = max_speed.map(|m| m.0).unwrap_or(DEFAULT_MAX_SPEED);
        let dx = target.at.x - pos.x;
        let dy = target.at.y - pos.y;
        let dist_sq = dx * dx + dy * dy;
        if dist_sq <= STOP_DIST * STOP_DIST {
            vel.vx = 0.0;
            vel.vy = 0.0;
            commands.entity(entity).remove::<MoveTarget>();
        } else {
            let dist = dist_sq.sqrt();
            vel.vx = (dx / dist) * speed;
            vel.vy = (dy / dist) * speed;
        }
    }
}
