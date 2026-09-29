//! Velocity integration.

use bevy_ecs::prelude::*;

use crate::components::{PassengerOf, Position, Velocity};
use crate::simulation::{ArrivedThisTick, TICK_SECS};

/// Moves every non-passenger by its `Velocity`, skipping entities that arrived this tick.
#[allow(clippy::needless_pass_by_value)] // Bevy `Res` system parameters
pub fn movement_system(
    // Passengers are carried, not integrated: `passenger_sync_system` owns their position.
    mut query: Query<(Entity, &mut Position, &Velocity), Without<PassengerOf>>,
    arrived: Res<ArrivedThisTick>,
) {
    for (entity, mut position, velocity) in &mut query {
        if arrived.0.contains(&entity) {
            continue;
        }
        position.x += velocity.vx * TICK_SECS;
        position.y += velocity.vy * TICK_SECS;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Position, Velocity};
    use crate::simulation::ArrivedThisTick;
    use bevy_ecs::prelude::{Schedule, World};

    #[test]
    fn movement_integrates_velocity() {
        let mut world = World::new();
        world.spawn((Position { x: 0.0, y: 0.0 }, Velocity { vx: 10.0, vy: 0.0 }));
        world.insert_resource(ArrivedThisTick::default());

        let mut schedule = Schedule::default();
        schedule.add_systems(movement_system);
        schedule.run(&mut world);

        let position = world.query::<&Position>().single(&world).expect("position");
        // 10 units/s over one 50 ms tick.
        assert!((position.x - 0.5).abs() < 1e-5);
        assert!((position.y - 0.0).abs() < 1e-5);
    }
}
