use bevy_ecs::prelude::Component;
use serde::{Deserialize, Serialize};

/// Template or archetype name assigned when the entity was spawned from YAML.
#[derive(Component, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EntityType(pub String);

#[cfg(test)]
mod tests {
    use super::EntityType;
    use bevy_ecs::prelude::*;

    #[test]
    fn entity_type_component_round_trip() {
        let mut world = World::new();
        world.spawn(EntityType("scout".to_owned()));

        let mut query = world.query::<&EntityType>();
        let mut count = 0;
        for entity_type in query.iter(&world) {
            assert_eq!(entity_type.0, "scout");
            count += 1;
        }
        assert_eq!(count, 1);
    }
}
