use bevy_ecs::prelude::World;
use serde::Serialize;

use crate::api::Api;
use crate::component_registry::{
    entity_components_from_query, registered_components_present, WorldExportQuery,
};
use crate::components::EntityType;
use crate::entity_components::EntityComponents;

const SCHEMA_VERSION: u32 = 3;

/// Errors while serializing a world snapshot to JSON.
#[derive(Debug)]
pub enum ExportError {
    /// JSON serialization failed.
    Serde(serde_json::Error),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Serde(err) => write!(f, "JSON export failed: {err}"),
        }
    }
}

impl std::error::Error for ExportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Serde(err) => Some(err),
        }
    }
}

impl From<serde_json::Error> for ExportError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serde(err)
    }
}

#[derive(Serialize)]
struct WorldExport {
    version: u32,
    entities: Vec<EntityExport>,
}

#[derive(Serialize)]
struct EntityExport {
    id: EntityIdExport,
    #[serde(flatten)]
    components: EntityComponents,
    #[serde(skip_serializing_if = "Option::is_none")]
    entity_type: Option<EntityType>,
}

#[derive(Serialize)]
struct EntityIdExport {
    index: u32,
    generation: u32,
}

impl Api {
    /// Serializes entities that have at least one registered gameplay component or
    /// [`EntityType`] to JSON (schema version 3).
    ///
    /// Component fields are omitted from each entity row when that component is
    /// not present on the entity.
    ///
    /// # Errors
    ///
    /// Returns [`ExportError::Serde`] if JSON encoding fails.
    pub fn world_json(&mut self) -> Result<String, ExportError> {
        world_json_from_world(self.core_mut().world_mut())
    }
}

fn world_json_from_world(world: &mut World) -> Result<String, ExportError> {
    let mut query = world.query::<WorldExportQuery<'_>>();
    let entities = query
        .iter(world)
        .filter_map(
            |(entity, position, velocity, faction, move_target, health, entity_type)| {
                if !registered_components_present(
                    position,
                    velocity,
                    faction,
                    move_target,
                    health,
                ) && entity_type.is_none()
                {
                    return None;
                }
                Some(EntityExport {
                    id: EntityIdExport {
                        index: entity.index_u32(),
                        generation: entity.generation().to_bits(),
                    },
                    components: entity_components_from_query(
                        position,
                        velocity,
                        faction,
                        move_target,
                        health,
                    ),
                    entity_type: entity_type.cloned(),
                })
            },
        )
        .collect();

    let payload = WorldExport {
        version: SCHEMA_VERSION,
        entities,
    };

    Ok(serde_json::to_string(&payload)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{EntityType, Faction, Health, Position, Velocity};

    #[test]
    fn world_json_empty_world() {
        let mut api = Api::new();
        let json = api.world_json().expect("serialize empty world");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("exported JSON should parse");
        assert_eq!(value["version"], 3);
        assert_eq!(value["entities"].as_array().map(Vec::len), Some(0));
    }

    #[test]
    fn world_json_v3_version() {
        let mut api = Api::new();
        let json = api.world_json().expect("serialize empty world");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("exported JSON should parse");
        assert_eq!(value["version"], 3);
    }

    #[test]
    fn world_json_includes_positioned_entities() {
        let mut api = Api::new();
        api.core_mut()
            .world_mut()
            .spawn(Position { x: 1.0, y: 2.0 });

        let json = api.world_json().expect("serialize world");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("exported JSON should parse");

        assert_eq!(value["version"], 3);
        let entities = value["entities"].as_array().expect("entities array");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0]["position"]["x"], 1.0);
        assert_eq!(entities[0]["position"]["y"], 2.0);
        assert!(entities[0]["id"]["index"].is_number());
        assert!(entities[0]["id"]["generation"].is_number());
    }

    #[test]
    fn world_json_faction_only_entity() {
        let mut api = Api::new();
        api.core_mut().world_mut().spawn(Faction(2));

        let json = api.world_json().expect("serialize world");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("exported JSON should parse");

        assert_eq!(value["version"], 3);
        let entities = value["entities"].as_array().expect("entities array");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0]["faction"], 2);
        assert!(entities[0].get("position").is_none());
    }

    #[test]
    fn world_json_partial_components() {
        let mut api = Api::new();
        api.core_mut().world_mut().spawn((
            Position { x: 1.0, y: 2.0 },
            Velocity { vx: 0.5, vy: -0.5 },
        ));

        let json = api.world_json().expect("serialize world");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("exported JSON should parse");

        assert_eq!(value["version"], 3);
        let entities = value["entities"].as_array().expect("entities array");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0]["position"]["x"], 1.0);
        assert_eq!(entities[0]["velocity"]["vx"], 0.5);
        assert!(entities[0].get("faction").is_none());
        assert!(entities[0].get("move_target").is_none());
    }

    #[test]
    fn world_json_entity_type_only_entity() {
        let mut api = Api::new();
        api.core_mut()
            .world_mut()
            .spawn(EntityType("marker".to_owned()));

        let json = api.world_json().expect("serialize world");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("exported JSON should parse");

        assert_eq!(value["version"], 3);
        let entities = value["entities"].as_array().expect("entities array");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0]["entity_type"], "marker");
    }

    #[test]
    fn world_json_omits_entity_type_when_absent() {
        let mut api = Api::new();
        api.core_mut()
            .world_mut()
            .spawn(Position { x: 1.0, y: 2.0 });

        let json = api.world_json().expect("serialize world");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("exported JSON should parse");

        let entities = value["entities"].as_array().expect("entities array");
        assert_eq!(entities.len(), 1);
        assert!(entities[0].get("entity_type").is_none());
    }

    #[test]
    fn world_json_v3_health_only_entity() {
        let mut api = Api::new();
        api.core_mut().world_mut().spawn(Health {
            current: 80,
            max: 100,
        });

        let json = api.world_json().expect("serialize world");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("exported JSON should parse");

        assert_eq!(value["version"], 3);
        let entities = value["entities"].as_array().expect("entities array");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0]["health"]["current"], 80);
        assert_eq!(entities[0]["health"]["max"], 100);
        assert!(entities[0].get("position").is_none());
    }

    #[test]
    fn world_json_v3_optional_keys() {
        let mut api = Api::new();
        api.core_mut()
            .world_mut()
            .spawn(Position { x: 1.0, y: 2.0 });

        let json = api.world_json().expect("serialize world");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("exported JSON should parse");

        let entities = value["entities"].as_array().expect("entities array");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0]["position"]["x"], 1.0);
        assert!(entities[0].get("health").is_none());
    }
}
