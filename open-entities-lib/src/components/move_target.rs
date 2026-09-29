use bevy_ecs::prelude::Component;

/// Movement goal point, in milli-units.
///
/// Serializes as map units; see [`crate::units`].
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveTarget {
    /// X coordinate, milli-units.
    pub x: i32,
    /// Y coordinate, milli-units.
    pub y: i32,
}

#[cfg(test)]
mod tests {
    use super::MoveTarget;
    use bevy_ecs::prelude::*;

    #[test]
    fn move_target_component_round_trip() {
        let mut world = World::new();
        world.spawn(MoveTarget { x: 20_000, y: 0 });

        let mut query = world.query::<&MoveTarget>();
        let mut count = 0;
        for target in query.iter(&world) {
            assert_eq!(target.x, 20_000);
            assert_eq!(target.y, 0);
            count += 1;
        }
        assert_eq!(count, 1);
    }
}
