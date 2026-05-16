use bevy_ecs::prelude::{Entity, World};
use serde::Serialize;

use crate::api::Api;
use crate::components::Position;

const SCHEMA_VERSION: u32 = 1;

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
    position: Position,
}

#[derive(Serialize)]
struct EntityIdExport {
    index: u32,
    generation: u32,
}

impl Api {
    /// Serializes all entities with a [`Position`] component to JSON.
    ///
    /// # Errors
    ///
    /// Returns [`ExportError::Serde`] if JSON encoding fails.
    pub fn world_json(&mut self) -> Result<String, ExportError> {
        world_json_from_world(self.core_mut().world_mut())
    }
}

fn world_json_from_world(world: &mut World) -> Result<String, ExportError> {
    let mut query = world.query::<(Entity, &Position)>();
    let entities = query
        .iter(world)
        .map(|(entity, position)| EntityExport {
            id: EntityIdExport {
                index: entity.index_u32(),
                generation: entity.generation().to_bits(),
            },
            position: *position,
        })
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
    use crate::components::Position;

    #[test]
    fn world_json_empty_world() {
        let mut api = Api::new();
        let json = api.world_json().expect("serialize empty world");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("exported JSON should parse");
        assert_eq!(value["version"], 1);
        assert_eq!(value["entities"].as_array().map(Vec::len), Some(0));
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

        assert_eq!(value["version"], 1);
        let entities = value["entities"].as_array().expect("entities array");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0]["position"]["x"], 1.0);
        assert_eq!(entities[0]["position"]["y"], 2.0);
        assert!(entities[0]["id"]["index"].is_number());
        assert!(entities[0]["id"]["generation"].is_number());
    }
}
