use bevy_ecs::prelude::*;

use crate::components::{BaseMoveSpeed, MoveTarget, Position, Velocity};
use crate::simulation::ArrivedThisTick;

use super::ARRIVAL_THRESHOLD;

pub fn seek_system(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &mut Position,
        &MoveTarget,
        &BaseMoveSpeed,
        &mut Velocity,
    )>,
    mut arrived: ResMut<ArrivedThisTick>,
) {
    for (entity, mut position, target, speed, mut velocity) in &mut query {
        let dx = target.x - position.x;
        let dy = target.y - position.y;
        let dist = dx.hypot(dy);

        if dist <= ARRIVAL_THRESHOLD {
            position.x = target.x;
            position.y = target.y;
            velocity.vx = 0.0;
            velocity.vy = 0.0;
            commands.entity(entity).remove::<MoveTarget>();
            arrived.0.insert(entity);
            continue;
        }

        if dist > 0.0 {
            let inv = speed.0 / dist;
            velocity.vx = dx * inv;
            velocity.vy = dy * inv;
        } else {
            position.x = target.x;
            position.y = target.y;
            velocity.vx = 0.0;
            velocity.vy = 0.0;
            commands.entity(entity).remove::<MoveTarget>();
            arrived.0.insert(entity);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{BaseMoveSpeed, MoveTarget, Position, Velocity};
    use crate::simulation::{ArrivedThisTick, SimDelta};
    use bevy_ecs::prelude::{Schedule, World};

    fn run_seek(world: &mut World) {
        let mut schedule = Schedule::default();
        schedule.add_systems(seek_system);
        world.insert_resource(ArrivedThisTick::default());
        world.insert_resource(SimDelta::from_ms(16));
        schedule.run(world);
        world.flush();
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

        let velocity = world
            .query::<&Velocity>()
            .single(&world)
            .expect("velocity");
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
}
