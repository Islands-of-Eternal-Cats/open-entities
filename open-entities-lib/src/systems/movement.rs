use bevy_ecs::prelude::*;

use crate::components::{Position, Velocity};
use crate::simulation::{ArrivedThisTick, SimDelta};

#[allow(clippy::needless_pass_by_value)] // Bevy `Res` system parameters
pub fn movement_system(
    mut query: Query<(Entity, &mut Position, &Velocity)>,
    arrived: Res<ArrivedThisTick>,
    delta: Res<SimDelta>,
) {
    for (entity, mut position, velocity) in &mut query {
        if arrived.0.contains(&entity) {
            continue;
        }
        position.x += velocity.vx * delta.dt_secs;
        position.y += velocity.vy * delta.dt_secs;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Position, Velocity};
    use crate::simulation::{ArrivedThisTick, SimDelta};
    use bevy_ecs::prelude::{Schedule, World};

    #[test]
    fn movement_integrates_velocity() {
        let mut world = World::new();
        world.spawn((
            Position { x: 0.0, y: 0.0 },
            Velocity { vx: 10.0, vy: 0.0 },
        ));
        world.insert_resource(SimDelta::from_ms(100));
        world.insert_resource(ArrivedThisTick::default());

        let mut schedule = Schedule::default();
        schedule.add_systems(movement_system);
        schedule.run(&mut world);

        let position = world.query::<&Position>().single(&world).expect("position");
        assert!((position.x - 1.0).abs() < 1e-5);
        assert!((position.y - 0.0).abs() < 1e-5);
    }
}
