use bevy_ecs::prelude::Component;
use serde::{Deserialize, Serialize};

/// Numeric faction / side identifier.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Faction(pub u32);

#[cfg(test)]
mod tests {
    use super::Faction;
    use bevy_ecs::prelude::*;

    #[test]
    fn faction_component_round_trip() {
        let mut world = World::new();
        world.spawn(Faction(42));

        let mut query = world.query::<&Faction>();
        let mut count = 0;
        for faction in query.iter(&world) {
            assert_eq!(faction.0, 42);
            count += 1;
        }
        assert_eq!(count, 1);
    }
}
