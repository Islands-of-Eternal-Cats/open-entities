use crate::components::{BaseMoveSpeed, MoveTarget, Position, Velocity};
use crate::systems::DeltaTime;
use bevy_ecs::prelude::*;

/// System: Update position based on velocity and delta time.
/// Only entities with [`BaseMoveSpeed`] integrate velocity into position.
/// Uses `Res<DeltaTime>` when present; otherwise assumes 1.0 (backward compatible).
pub fn move_system(
    mut commands: Commands,
    mut query: Query<
        (Entity, &mut Position, &mut Velocity, Option<&MoveTarget>),
        With<BaseMoveSpeed>,
    >,
    dt: Option<Res<DeltaTime>>,
) {
    let dt_sec = dt
        .map(|d| d.0)
        .filter(|v| v.is_finite() && *v > 0.0)
        .unwrap_or(0.0);

    if dt_sec == 0.0 {
        return;
    }

    for (entity, mut pos, mut vel, target) in &mut query {
        if let Some(target) = target {
            let step_x = vel.vx * dt_sec;
            let step_y = vel.vy * dt_sec;
            let next_x = pos.x + step_x;
            let next_y = pos.y + step_y;

            let rem_x = target.at.x - pos.x;
            let rem_y = target.at.y - pos.y;
            let rem_sq = rem_x * rem_x + rem_y * rem_y;
            let step_sq = step_x * step_x + step_y * step_y;

            if step_sq >= rem_sq {
                pos.x = target.at.x;
                pos.y = target.at.y;
                vel.vx = 0.0;
                vel.vy = 0.0;
                commands.entity(entity).remove::<MoveTarget>();
                continue;
            }

            pos.x = next_x;
            pos.y = next_y;
            continue;
        }

        pos.x += vel.vx * dt_sec;
        pos.y += vel.vy * dt_sec;
    }
}
