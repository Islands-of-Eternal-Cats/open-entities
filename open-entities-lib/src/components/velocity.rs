use bevy_ecs::prelude::Component;
use serde::{Deserialize, Serialize};

/// 2D velocity in world/simulation space.
#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Velocity {
    pub vx: f32,
    pub vy: f32,
}

#[cfg(test)]
mod tests {
    use super::Velocity;
    use bevy_ecs::prelude::*;

    #[test]
    fn velocity_component_round_trip() {
        let mut world = World::new();
        world.spawn(Velocity { vx: 1.5, vy: -2.0 });

        let mut query = world.query::<&Velocity>();
        let mut count = 0;
        for velocity in query.iter(&world) {
            assert_eq!(velocity.vx, 1.5);
            assert_eq!(velocity.vy, -2.0);
            count += 1;
        }
        assert_eq!(count, 1);
    }
}
