use std::collections::BTreeMap;

use bevy_ecs::prelude::{Entity, World};
use serde::Deserialize;

use crate::api::Api;
use crate::components::{EntityType, Faction, MoveTarget, Position, Velocity};

/// Errors while loading YAML templates or spawning from them.
#[derive(Debug)]
pub enum ImportError {
    /// YAML syntax, type mismatch, or unknown field.
    Yaml(serde_yaml::Error),
    /// `spawn_yaml` called before a successful `load_templates_yaml`.
    TemplatesNotLoaded,
    /// No template with this name in the loaded map.
    UnknownTemplate(String),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Yaml(err) => write!(f, "YAML import failed: {err}"),
            Self::TemplatesNotLoaded => {
                f.write_str("templates not loaded; call load_templates_yaml first")
            }
            Self::UnknownTemplate(name) => {
                write!(f, "unknown template name: {name}")
            }
        }
    }
}

impl std::error::Error for ImportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Yaml(err) => Some(err),
            Self::TemplatesNotLoaded | Self::UnknownTemplate(_) => None,
        }
    }
}

/// In-memory map of template name → component bundle (private).
pub(crate) type EntityTemplates = BTreeMap<String, EntitySpawnYaml>;

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(crate) struct EntitySpawnYaml {
    position: Option<Position>,
    velocity: Option<Velocity>,
    faction: Option<Faction>,
    move_target: Option<MoveTarget>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TemplatesFileRoot {
    entities: EntityTemplates,
}

fn spawn_from_doc(world: &mut World, template_name: &str, doc: &EntitySpawnYaml) -> Entity {
    let mut entity = world.spawn_empty();
    if let Some(position) = doc.position {
        entity.insert(position);
    }
    if let Some(velocity) = doc.velocity {
        entity.insert(velocity);
    }
    if let Some(faction) = doc.faction {
        entity.insert(faction);
    }
    if let Some(move_target) = doc.move_target {
        entity.insert(move_target);
    }
    entity.insert(EntityType(template_name.to_owned()));
    entity.id()
}

impl Api {
    /// Parses a YAML templates file and stores it for later spawns.
    ///
    /// Root must be `entities: { <name>: <components>, ... }`.
    /// Replaces any previously loaded templates on success.
    /// On error, leaves any previously loaded templates unchanged.
    ///
    /// # Errors
    ///
    /// Returns [`ImportError::Yaml`] for invalid YAML or unknown fields.
    pub fn load_templates_yaml(&mut self, yaml: &str) -> Result<(), ImportError> {
        let parsed: TemplatesFileRoot =
            serde_yaml::from_str(yaml).map_err(ImportError::Yaml)?;
        self.templates = Some(parsed.entities);
        Ok(())
    }

    /// Spawns one entity from a previously loaded template by name.
    ///
    /// # Errors
    ///
    /// Returns [`ImportError::TemplatesNotLoaded`] if no successful load yet.
    /// Returns [`ImportError::UnknownTemplate`] if `template_name` is missing.
    pub fn spawn_yaml(&mut self, template_name: &str) -> Result<Entity, ImportError> {
        let templates = self
            .templates
            .as_ref()
            .ok_or(ImportError::TemplatesNotLoaded)?;
        let doc = templates
            .get(template_name)
            .ok_or_else(|| ImportError::UnknownTemplate(template_name.to_owned()))?
            .clone();
        Ok(spawn_from_doc(
            self.core_mut().world_mut(),
            template_name,
            &doc,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::Api;

    const FIXTURE_YAML: &str = r"
entities:
  scout:
    position: { x: 0, y: 0 }
    velocity: { vx: 2, vy: 0 }
    faction: 1
  base:
    faction: 2
  marker: {}
";

    fn load_fixture(api: &mut Api) {
        api.load_templates_yaml(FIXTURE_YAML)
            .expect("fixture YAML should load");
    }

    #[test]
    fn spawn_yaml_without_load() {
        let mut api = Api::new();
        let err = api.spawn_yaml("scout").unwrap_err();
        assert!(matches!(err, ImportError::TemplatesNotLoaded));
    }

    #[test]
    fn load_templates_yaml_invalid() {
        let mut api = Api::new();
        let err = api
            .load_templates_yaml("not: [valid: yaml: structure")
            .unwrap_err();
        assert!(matches!(err, ImportError::Yaml(_)));
        assert!(api.templates.is_none());
        assert!(matches!(
            api.spawn_yaml("scout").unwrap_err(),
            ImportError::TemplatesNotLoaded
        ));
    }

    #[test]
    fn load_templates_yaml_unknown_root() {
        let mut api = Api::new();
        let err = api.load_templates_yaml("foo: 1").unwrap_err();
        assert!(matches!(err, ImportError::Yaml(_)));
        assert!(api.templates.is_none());
    }

    #[test]
    fn load_templates_yaml_invalid_nested() {
        let mut api = Api::new();
        let yaml = r"
entities:
  bad:
    position: not-an-object
";
        let err = api.load_templates_yaml(yaml).unwrap_err();
        assert!(matches!(err, ImportError::Yaml(_)));
        assert!(api.templates.is_none());
    }

    #[test]
    fn load_templates_yaml_unknown_component_key() {
        let mut api = Api::new();
        let err = api.load_templates_yaml("health: 1").unwrap_err();
        assert!(matches!(err, ImportError::Yaml(_)));
        assert!(api.templates.is_none());
    }

    #[test]
    fn load_templates_yaml_failed_replaces_keeps_previous() {
        let mut api = Api::new();
        api.load_templates_yaml("entities:\n  a:\n    faction: 1\n")
            .expect("first load");
        let err = api
            .load_templates_yaml("entities:\n  bad:\n    health: 1\n")
            .unwrap_err();
        assert!(matches!(err, ImportError::Yaml(_)));
        let templates = api.templates.as_ref().expect("map still loaded");
        assert!(templates.contains_key("a"));
        assert!(!templates.contains_key("bad"));
    }

    #[test]
    fn spawn_yaml_unknown_template() {
        let mut api = Api::new();
        load_fixture(&mut api);
        let err = api.spawn_yaml("nope").unwrap_err();
        assert!(matches!(err, ImportError::UnknownTemplate(name) if name == "nope"));
    }

    #[test]
    fn spawn_yaml_scout() {
        let mut api = Api::new();
        load_fixture(&mut api);
        let entity = api.spawn_yaml("scout").expect("spawn scout");
        let world = api.core_mut().world_mut();
        let position = world.get::<Position>(entity).expect("position");
        assert_eq!(position.x, 0.0);
        assert_eq!(position.y, 0.0);
        let velocity = world.get::<Velocity>(entity).expect("velocity");
        assert_eq!(velocity.vx, 2.0);
        assert_eq!(velocity.vy, 0.0);
        let faction = world.get::<Faction>(entity).expect("faction");
        assert_eq!(faction.0, 1);
        assert!(world.get::<MoveTarget>(entity).is_none());
        let entity_type = world.get::<EntityType>(entity).expect("entity_type");
        assert_eq!(entity_type.0, "scout");
    }

    #[test]
    fn spawn_yaml_base() {
        let mut api = Api::new();
        load_fixture(&mut api);
        let entity = api.spawn_yaml("base").expect("spawn base");
        let world = api.core_mut().world_mut();
        assert!(world.get::<Position>(entity).is_none());
        assert!(world.get::<Velocity>(entity).is_none());
        let faction = world.get::<Faction>(entity).expect("faction");
        assert_eq!(faction.0, 2);
        let entity_type = world.get::<EntityType>(entity).expect("entity_type");
        assert_eq!(entity_type.0, "base");
    }

    #[test]
    fn spawn_yaml_marker() {
        let mut api = Api::new();
        load_fixture(&mut api);
        let entity = api.spawn_yaml("marker").expect("spawn marker");
        let world = api.core_mut().world_mut();
        assert!(world.get::<Position>(entity).is_none());
        assert!(world.get::<Velocity>(entity).is_none());
        assert!(world.get::<Faction>(entity).is_none());
        assert!(world.get::<MoveTarget>(entity).is_none());
        let entity_type = world.get::<EntityType>(entity).expect("entity_type");
        assert_eq!(entity_type.0, "marker");
    }

    #[test]
    fn load_templates_yaml_rejects_entity_type_in_yaml() {
        let mut api = Api::new();
        let yaml = r"
entities:
  scout:
    entity_type: scout
";
        let err = api.load_templates_yaml(yaml).unwrap_err();
        assert!(matches!(err, ImportError::Yaml(_)));
    }

    #[test]
    fn spawn_yaml_twice_same_name() {
        let mut api = Api::new();
        load_fixture(&mut api);
        let e1 = api.spawn_yaml("scout").expect("first scout");
        let e2 = api.spawn_yaml("scout").expect("second scout");
        assert_ne!(e1, e2);
        let world = api.core_mut().world_mut();
        assert!(world.get::<Position>(e1).is_some());
        assert!(world.get::<Position>(e2).is_some());
        assert_eq!(world.get::<EntityType>(e1).map(|t| t.0.as_str()), Some("scout"));
        assert_eq!(world.get::<EntityType>(e2).map(|t| t.0.as_str()), Some("scout"));
    }

    #[test]
    fn load_templates_yaml_replaces() {
        let mut api = Api::new();
        api.load_templates_yaml("entities:\n  a:\n    faction: 1\n")
            .expect("load A");
        api.load_templates_yaml("entities:\n  b:\n    faction: 2\n")
            .expect("load B");
        assert!(api.spawn_yaml("a").is_err());
        let entity = api.spawn_yaml("b").expect("only B remains");
        let world = api.core_mut().world_mut();
        assert_eq!(world.get::<Faction>(entity).map(|f| f.0), Some(2));
        assert_eq!(world.get::<EntityType>(entity).map(|t| t.0.as_str()), Some("b"));
    }

    #[test]
    fn spawn_yaml_exports_entity_type_in_world_json() {
        let mut api = Api::new();
        load_fixture(&mut api);
        api.spawn_yaml("scout").expect("spawn scout");
        api.spawn_yaml("marker").expect("spawn marker");

        let json = api.world_json().expect("serialize world");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("exported JSON should parse");

        let entities = value["entities"].as_array().expect("entities array");
        assert_eq!(entities.len(), 2);

        let scout = entities
            .iter()
            .find(|row| row["entity_type"] == "scout")
            .expect("scout row");
        assert_eq!(scout["position"]["x"], 0.0);
        assert_eq!(scout["faction"], 1);

        let marker = entities
            .iter()
            .find(|row| row["entity_type"] == "marker")
            .expect("marker row");
        assert!(marker.get("position").is_none());
        assert!(marker.get("faction").is_none());
    }
}
