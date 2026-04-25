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
            let (step_x, step_y) = vel.delta_for_dt(dt_sec);
            let next_pos = pos.shifted(step_x, step_y);

            let rem_sq = target.remaining_dist_sq_from(*pos);
            let step_sq = vel.step_len_sq_for_dt(dt_sec);

            if step_sq >= rem_sq {
                pos.x = target.at.x;
                pos.y = target.at.y;
                *vel = Velocity::zero();
                commands.entity(entity).remove::<MoveTarget>();
                continue;
            }

            *pos = next_pos;
            continue;
        }

        let (step_x, step_y) = vel.delta_for_dt(dt_sec);
        *pos = pos.shifted(step_x, step_y);
    }
}
