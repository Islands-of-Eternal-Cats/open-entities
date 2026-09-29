//! Steering towards a [`MoveTarget`](crate::components::MoveTarget) and arrival.

use bevy_ecs::prelude::*;

use crate::components::{BaseMoveSpeed, MoveTarget, OrderSource, PassengerOf, Position, Velocity};
use crate::simulation::ArrivedThisTick;

use super::{ARRIVAL_RADIUS, length};

/// Steers entities toward their [`MoveTarget`] and detects arrival.
///
/// All in milli-units: `d = target − position`, `dist = isqrt(dx² + dy²)`. An entity arrives when
/// `dist ≤ max(ARRIVAL_RADIUS, speed_per_tick)` — already within [`ARRIVAL_RADIUS`], or the step
/// it would take this tick reaches the target. Either way the position snaps to the
/// target, velocity is zeroed, [`MoveTarget`] and its [`OrderSource`] are removed, and the
/// entity is recorded in
/// [`ArrivedThisTick`] so [`movement_system`](super::movement_system) skips it this frame.
///
/// Otherwise `velocity = d × speed_per_tick / dist`, in `i64`, truncated toward zero.
///
/// Without the step check, a unit faster than `2 * ARRIVAL_RADIUS` per tick overshoots the target
/// every tick, turns around on the next one and oscillates forever.
///
/// # Panics
///
/// Never: the length of two `i32` differences fits `i64`, and each velocity component is at most
/// `speed` in magnitude.
#[allow(clippy::needless_pass_by_value)] // Bevy `Res` system parameters
pub fn seek_system(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &mut Position,
            &MoveTarget,
            &BaseMoveSpeed,
            &mut Velocity,
        ),
        // A passenger's position belongs to its vehicle; steering it here would give it two owners.
        Without<PassengerOf>,
    >,
    mut arrived: ResMut<ArrivedThisTick>,
) {
    for (entity, mut position, target, speed, mut velocity) in &mut query {
        let dx = i64::from(target.x) - i64::from(position.x);
        let dy = i64::from(target.y) - i64::from(position.y);
        let dist = length(dx, dy);
        let speed = speed.0.max(0);

        if dist <= u64::from(ARRIVAL_RADIUS.max(speed).unsigned_abs()) {
            position.x = target.x;
            position.y = target.y;
            velocity.vx = 0;
            velocity.vy = 0;
            // Arrival releases the claim too: whoever gave this order is done with it.
            commands
                .entity(entity)
                .remove::<(MoveTarget, OrderSource)>();
            arrived.0.insert(entity);
            continue;
        }

        // dist > speed ≥ 0 here, and |dx|, |dy| ≤ dist, so each quotient fits in `speed`.
        let dist = i64::try_from(dist).expect("length of two i32 differences fits i64");
        let speed = i64::from(speed);
        velocity.vx = i32::try_from(dx * speed / dist).expect("|vx| <= speed");
        velocity.vy = i32::try_from(dy * speed / dist).expect("|vy| <= speed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{BaseMoveSpeed, MoveTarget, Position, Velocity};
    use crate::simulation::ArrivedThisTick;
    use bevy_ecs::prelude::{Schedule, World};

    fn run_seek(world: &mut World) {
        let mut schedule = Schedule::default();
        schedule.add_systems(seek_system);
        world.insert_resource(ArrivedThisTick::default());
        schedule.run(world);
        world.flush();
    }

    #[test]
    fn seek_sets_velocity_toward_target() {
        let mut world = World::new();
        world.spawn((
            Position { x: 0, y: 0 },
            MoveTarget { x: 3000, y: 4000 },
            BaseMoveSpeed(500),
            Velocity { vx: 0, vy: 0 },
        ));

        run_seek(&mut world);

        let velocity = world.query::<&Velocity>().single(&world).expect("velocity");
        assert_eq!(*velocity, Velocity { vx: 300, vy: 400 });
    }

    #[test]
    fn seek_arrival_snaps_and_removes_target() {
        let mut world = World::new();
        let entity = world
            .spawn((
                Position { x: 19_950, y: 0 },
                MoveTarget { x: 20_000, y: 0 },
                BaseMoveSpeed(100),
                Velocity { vx: 50, vy: 0 },
            ))
            .id();

        run_seek(&mut world);

        let position = world.get::<Position>(entity).expect("position");
        assert_eq!(*position, Position { x: 20_000, y: 0 });
        let velocity = world.get::<Velocity>(entity).expect("velocity");
        assert_eq!(*velocity, Velocity { vx: 0, vy: 0 });
        assert!(world.get::<MoveTarget>(entity).is_none());
    }

    #[test]
    fn seek_arrives_when_step_would_overshoot() {
        let mut world = World::new();
        let entity = world
            .spawn((
                Position { x: 0, y: 0 },
                MoveTarget { x: 1000, y: 0 },
                BaseMoveSpeed(2250),
                Velocity { vx: 0, vy: 0 },
            ))
            .id();

        // 45 units/s is 2250 milli-units per tick, far past the target 1000 away.
        run_seek(&mut world);

        let position = world.get::<Position>(entity).expect("position");
        assert_eq!(*position, Position { x: 1000, y: 0 });
        let velocity = world.get::<Velocity>(entity).expect("velocity");
        assert_eq!(*velocity, Velocity { vx: 0, vy: 0 });
        assert!(world.get::<MoveTarget>(entity).is_none());
        assert!(world.resource::<ArrivedThisTick>().0.contains(&entity));
    }

    #[test]
    fn seek_keeps_full_speed_while_step_is_short_of_target() {
        let mut world = World::new();
        world.spawn((
            Position { x: 0, y: 0 },
            MoveTarget { x: 20_000, y: 0 },
            BaseMoveSpeed(2250),
            Velocity { vx: 0, vy: 0 },
        ));

        run_seek(&mut world);

        let velocity = world.query::<&Velocity>().single(&world).expect("velocity");
        assert_eq!(*velocity, Velocity { vx: 2250, vy: 0 });
    }
}
