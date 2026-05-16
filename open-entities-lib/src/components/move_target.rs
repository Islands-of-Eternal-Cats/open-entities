use bevy_ecs::prelude::Component;
use serde::Serialize;

/// World-space movement goal point.
#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize)]
pub struct MoveTarget {
    pub x: f32,
    pub y: f32,
}

#[cfg(test)]
mod tests {
    use super::MoveTarget;
    use bevy_ecs::prelude::*;

    #[test]
    fn move_target_component_round_trip() {
        let mut world = World::new();
        world.spawn(MoveTarget { x: 20.0, y: 0.0 });

        let mut query = world.query::<&MoveTarget>();
        let mut count = 0;
        for target in query.iter(&world) {
            assert_eq!(target.x, 20.0);
            assert_eq!(target.y, 0.0);
            count += 1;
        }
        assert_eq!(count, 1);
    }
}
