//! # YAML Entity Parser
//!
//! This module provides functionality to parse entity descriptions from YAML format
//! and spawn them in the ECS world.

use crate::components::{Position, Unit, Vehicle, Velocity};
use bevy_ecs::prelude::{Commands, Entity};
use serde::Deserialize;
use yaml_serde as serde_yaml;

/// Represents a single component parsed from YAML
#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum YamlComponent {
    /// Position component with x, y coordinates
    Position { x: f32, y: f32 },
    /// Velocity component with vx, vy components
    Velocity { vx: f32, vy: f32 },
    /// Unit marker component
    Unit,
    /// Vehicle marker component
    Vehicle,
}

/// Represents an entity description parsed from YAML
#[derive(Debug, Deserialize)]
pub struct YamlEntity {
    /// Optional name for the entity
    pub name: Option<String>,
    /// List of components for this entity
    pub components: Vec<YamlComponent>,
}

/// Represents a list of entities parsed from YAML
#[derive(Debug, Deserialize)]
pub struct YamlEntityList {
    /// List of entities
    pub entities: Vec<YamlEntity>,
}

/// Spawn entities from YAML string
/// 
/// # Arguments
/// * `commands` - Bevy Commands to spawn entities
/// * `yaml` - YAML string containing entity descriptions
/// 
/// # Returns
/// * `Vec<Entity>` - List of spawned entity IDs
/// 
/// # Example
/// 
/// ```yaml
/// entities:
///   - name: "player"
///     components:
///       - type: Position
///         data:
///           x: 10.0
///           y: 20.0
///       - type: Velocity
///         data:
///           vx: 1.0
///           vy: 0.0
///       - type: Unit
/// ```
pub fn spawn_from_yaml(commands: &mut Commands, yaml: &str) -> Result<Vec<Entity>, serde_yaml::Error> {
    let entity_list: YamlEntityList = serde_yaml::from_str(yaml)?;
    let mut entities = Vec::new();
    
    for yaml_entity in entity_list.entities {
        let entity_id = spawn_single_entity(commands, &yaml_entity);
        entities.push(entity_id);
    }
    
    Ok(entities)
}

/// Spawn a single entity from YamlEntity
fn spawn_single_entity(commands: &mut Commands, yaml_entity: &YamlEntity) -> Entity {
    let mut entity_commands = commands.spawn(());
    
    for component in &yaml_entity.components {
        match component {
            YamlComponent::Position { x, y } => {
                entity_commands.insert(Position { x: *x, y: *y });
            }
            YamlComponent::Velocity { vx, vy } => {
                entity_commands.insert(Velocity { vx: *vx, vy: *vy });
            }
            YamlComponent::Unit => {
                entity_commands.insert(Unit);
            }
            YamlComponent::Vehicle => {
                entity_commands.insert(Vehicle);
            }
        }
    }
    
    entity_commands.id()
}

/// Spawn entities from YAML and return the Commands for further chaining
/// 
/// This is a simpler version that directly spawns entities
pub fn spawn_yaml_entities(commands: &mut Commands, yaml: &str) -> Result<(), serde_yaml::Error> {
    let entity_list: YamlEntityList = serde_yaml::from_str(yaml)?;
    
    for yaml_entity in entity_list.entities {
        spawn_single_entity(commands, &yaml_entity);
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_entity() {
        let yaml = r#"
entities:
  - name: "test_entity"
    components:
      - type: Position
        data:
          x: 10.0
          y: 20.0
      - type: Velocity
        data:
          vx: 1.5
          vy: 2.5
"#;
        let result: YamlEntityList = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(result.entities.len(), 1);
        assert_eq!(result.entities[0].name, Some("test_entity".to_string()));
        assert_eq!(result.entities[0].components.len(), 2);
    }

    #[test]
    fn test_parse_marker_components() {
        let yaml = r#"
entities:
  - components:
      - type: Unit
      - type: Vehicle
"#;
        let result: YamlEntityList = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(result.entities.len(), 1);
        assert_eq!(result.entities[0].components.len(), 2);
    }

    #[test]
    fn test_parse_multiple_entities() {
        let yaml = r#"
entities:
  - name: "entity1"
    components:
      - type: Position
        data:
          x: 0.0
          y: 0.0
  - name: "entity2"
    components:
      - type: Position
        data:
          x: 5.0
          y: 5.0
      - type: Unit
"#;
        let result: YamlEntityList = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(result.entities.len(), 2);
    }
}
