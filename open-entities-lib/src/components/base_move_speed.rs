use bevy_ecs::prelude::Component;

/// Maximum travel speed used by seek, in milli-units per tick.
///
/// Serializes as map units per second; see [`crate::units`].
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BaseMoveSpeed(pub i32);

#[cfg(test)]
mod tests {
    use super::BaseMoveSpeed;
    use bevy_ecs::prelude::*;

    #[test]
    fn base_move_speed_component_round_trip() {
        let mut world = World::new();
        world.spawn(BaseMoveSpeed(100));

        let mut query = world.query::<&BaseMoveSpeed>();
        let mut count = 0;
        for speed in query.iter(&world) {
            assert_eq!(speed.0, 100);
            count += 1;
        }
        assert_eq!(count, 1);
    }
}
