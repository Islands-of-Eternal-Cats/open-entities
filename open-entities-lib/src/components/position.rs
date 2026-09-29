use bevy_ecs::prelude::Component;

/// 2D position in simulation space, in milli-units.
///
/// Serializes as map units (`{ x: 1.5, y: 0.0 }`); see [`crate::units`].
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// X coordinate, milli-units.
    pub x: i32,
    /// Y coordinate, milli-units.
    pub y: i32,
}

#[cfg(test)]
mod tests {
    use super::Position;
    use bevy_ecs::prelude::*;

    #[test]
    fn position_component_round_trip() {
        let mut world = World::new();
        world.spawn(Position { x: 1000, y: 2000 });

        let mut query = world.query::<&Position>();
        let mut count = 0;
        for position in query.iter(&world) {
            assert_eq!(position.x, 1000);
            assert_eq!(position.y, 2000);
            count += 1;
        }
        assert_eq!(count, 1);
    }
}
