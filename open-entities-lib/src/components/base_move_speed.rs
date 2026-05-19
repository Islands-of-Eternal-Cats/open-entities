use bevy_ecs::prelude::Component;
use serde::{Deserialize, Serialize};

/// Maximum travel speed used by seek (world units per second).
#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BaseMoveSpeed(pub f32);

#[cfg(test)]
mod tests {
    use super::BaseMoveSpeed;
    use bevy_ecs::prelude::*;

    #[test]
    fn base_move_speed_component_round_trip() {
        let mut world = World::new();
        world.spawn(BaseMoveSpeed(2.0));

        let mut query = world.query::<&BaseMoveSpeed>();
        let mut count = 0;
        for speed in query.iter(&world) {
            assert_eq!(speed.0, 2.0);
            count += 1;
        }
        assert_eq!(count, 1);
    }
}
