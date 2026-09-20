use bevy_ecs::prelude::*;

use crate::components::{BaseMoveSpeed, MoveTarget, OrderSource, PassengerOf, Position, Velocity};
use crate::simulation::{ArrivedThisTick, SimDelta};

use super::ARRIVAL_THRESHOLD;

/// Steers entities toward their [`MoveTarget`] and detects arrival.
///
/// An entity arrives when it is already within [`ARRIVAL_THRESHOLD`] of the target, or when the
/// step it would take this tick (`speed * dt`) reaches it. Either way the position snaps to the
/// target, velocity is zeroed, [`MoveTarget`] and its [`OrderSource`] are removed, and the
/// entity is recorded in
/// [`ArrivedThisTick`] so [`movement_system`](super::movement_system) skips it this frame.
///
/// Without the step check, a unit faster than `2 * ARRIVAL_THRESHOLD / dt` overshoots the target
/// every tick, turns around on the next one and oscillates forever.
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
    delta: Res<SimDelta>,
) {
    for (entity, mut position, target, speed, mut velocity) in &mut query {
        let dx = target.x - position.x;
        let dy = target.y - position.y;
        let dist = dx.hypot(dy);
        let speed = speed.0.max(0.0);
        let step = speed * delta.dt_secs;

        if dist <= ARRIVAL_THRESHOLD || step >= dist {
            position.x = target.x;
            position.y = target.y;
            velocity.vx = 0.0;
            velocity.vy = 0.0;
            // Arrival releases the claim too: whoever gave this order is done with it.
            commands
                .entity(entity)
                .remove::<(MoveTarget, OrderSource)>();
            arrived.0.insert(entity);
            continue;
        }

        let inv = speed / dist;
        velocity.vx = dx * inv;
        velocity.vy = dy * inv;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{BaseMoveSpeed, MoveTarget, Position, Velocity};
    use crate::simulation::{ArrivedThisTick, SimDelta};
    use bevy_ecs::prelude::{Schedule, World};

    fn run_seek_with_dt(world: &mut World, dt_ms: u32) {
        let mut schedule = Schedule::default();
        schedule.add_systems(seek_system);
        world.insert_resource(ArrivedThisTick::default());
        world.insert_resource(SimDelta::from_ms(dt_ms));
        schedule.run(world);
        world.flush();
    }

    fn run_seek(world: &mut World) {
        run_seek_with_dt(world, 16);
    }

    #[test]
    fn seek_sets_velocity_toward_target() {
        let mut world = World::new();
        world.spawn((
            Position { x: 0.0, y: 0.0 },
            MoveTarget { x: 3.0, y: 4.0 },
            BaseMoveSpeed(10.0),
            Velocity { vx: 0.0, vy: 0.0 },
        ));

        run_seek(&mut world);

        let velocity = world.query::<&Velocity>().single(&world).expect("velocity");
        assert!((velocity.vx - 6.0).abs() < 1e-5);
        assert!((velocity.vy - 8.0).abs() < 1e-5);
    }

    #[test]
    fn seek_arrival_snaps_and_removes_target() {
        let mut world = World::new();
        let entity = world
            .spawn((
                Position { x: 19.95, y: 0.0 },
                MoveTarget { x: 20.0, y: 0.0 },
                BaseMoveSpeed(2.0),
                Velocity { vx: 1.0, vy: 0.0 },
            ))
            .id();

        run_seek(&mut world);

        let position = world.get::<Position>(entity).expect("position");
        assert_eq!(position.x, 20.0);
        assert_eq!(position.y, 0.0);
        let velocity = world.get::<Velocity>(entity).expect("velocity");
        assert_eq!(velocity.vx, 0.0);
        assert_eq!(velocity.vy, 0.0);
        assert!(world.get::<MoveTarget>(entity).is_none());
    }

    #[test]
    fn seek_arrives_when_step_would_overshoot() {
        let mut world = World::new();
        let entity = world
            .spawn((
                Position { x: 0.0, y: 0.0 },
                MoveTarget { x: 1.0, y: 0.0 },
                BaseMoveSpeed(45.0),
                Velocity { vx: 0.0, vy: 0.0 },
            ))
            .id();

        // step = 45 * 0.1 = 4.5 world units, far past the target 1.0 away.
        run_seek_with_dt(&mut world, 100);

        let position = world.get::<Position>(entity).expect("position");
        assert_eq!(position.x, 1.0);
        assert_eq!(position.y, 0.0);
        let velocity = world.get::<Velocity>(entity).expect("velocity");
        assert_eq!(velocity.vx, 0.0);
        assert_eq!(velocity.vy, 0.0);
        assert!(world.get::<MoveTarget>(entity).is_none());
        assert!(world.resource::<ArrivedThisTick>().0.contains(&entity));
    }

    #[test]
    fn seek_keeps_full_speed_while_step_is_short_of_target() {
        let mut world = World::new();
        world.spawn((
            Position { x: 0.0, y: 0.0 },
            MoveTarget { x: 20.0, y: 0.0 },
            BaseMoveSpeed(45.0),
            Velocity { vx: 0.0, vy: 0.0 },
        ));

        run_seek_with_dt(&mut world, 100);

        let velocity = world.query::<&Velocity>().single(&world).expect("velocity");
        assert!((velocity.vx - 45.0).abs() < 1e-5);
        assert!((velocity.vy - 0.0).abs() < 1e-5);
    }
}
