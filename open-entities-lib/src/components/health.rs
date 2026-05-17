use bevy_ecs::prelude::Component;
use serde::{Deserialize, Serialize};

/// Hit points for a unit or structure.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Health {
    pub current: u32,
    pub max: u32,
}

#[cfg(test)]
mod tests {
    use super::Health;
    use bevy_ecs::prelude::*;

    #[test]
    fn health_component_round_trip() {
        let mut world = World::new();
        world.spawn(Health {
            current: 80,
            max: 100,
        });

        let mut query = world.query::<&Health>();
        let mut count = 0;
        for health in query.iter(&world) {
            assert_eq!(health.current, 80);
            assert_eq!(health.max, 100);
            count += 1;
        }
        assert_eq!(count, 1);
    }
}
