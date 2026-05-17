use bevy_ecs::prelude::Component;
use serde::{Deserialize, Serialize};

/// 2D position in world/simulation space.
#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[cfg(test)]
mod tests {
    use super::Position;
    use bevy_ecs::prelude::*;

    #[test]
    fn position_component_round_trip() {
        let mut world = World::new();
        world.spawn(Position { x: 1.0, y: 2.0 });

        let mut query = world.query::<&Position>();
        let mut count = 0;
        for position in query.iter(&world) {
            assert_eq!(position.x, 1.0);
            assert_eq!(position.y, 2.0);
            count += 1;
        }
        assert_eq!(count, 1);
    }
}
