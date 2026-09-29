use bevy_ecs::prelude::Component;

/// 2D velocity, in milli-units per tick.
///
/// Serializes as map units per second; see [`crate::units`].
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Velocity {
    /// X component, milli-units per tick.
    pub vx: i32,
    /// Y component, milli-units per tick.
    pub vy: i32,
}

#[cfg(test)]
mod tests {
    use super::Velocity;
    use bevy_ecs::prelude::*;

    #[test]
    fn velocity_component_round_trip() {
        let mut world = World::new();
        world.spawn(Velocity { vx: 75, vy: -100 });

        let mut query = world.query::<&Velocity>();
        let mut count = 0;
        for velocity in query.iter(&world) {
            assert_eq!(velocity.vx, 75);
            assert_eq!(velocity.vy, -100);
            count += 1;
        }
        assert_eq!(count, 1);
    }
}
