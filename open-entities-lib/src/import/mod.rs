use std::collections::BTreeMap;

use bevy_ecs::prelude::{Entity, World};
use serde::Deserialize;

use crate::api::Api;
use crate::components::EntityType;
#[cfg(test)]
use crate::components::{Faction, MoveTarget, Position, Velocity};
use crate::entity_components::{merge_components, EntityComponents};

/// Errors while loading YAML templates or spawning from them.
#[derive(Debug)]
pub enum ImportError {
    /// YAML syntax, type mismatch, or unknown field.
    Yaml(serde_yaml::Error),
    /// `spawn_entity` called before a successful `load_templates_yaml`.
    TemplatesNotLoaded,
    /// No template with this name in the loaded map.
    UnknownTemplate(String),
    /// Referenced parent name missing from `entities` map.
    UnknownTemplateParent { child: String, parent: String },
    /// Circular `template` chain detected during load.
    TemplateCycle { chain: Vec<String> },
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
            Self::UnknownTemplateParent { child, parent } => {
                write!(
                    f,
                    r#"template "{parent}" not found (referenced from "{child}")"#
                )
            }
            Self::TemplateCycle { chain } => {
                write!(f, "template inheritance cycle: {}", chain.join(" -> "))
            }
        }
    }
}

impl std::error::Error for ImportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Yaml(err) => Some(err),
            Self::TemplatesNotLoaded
            | Self::UnknownTemplate(_)
            | Self::UnknownTemplateParent { .. }
            | Self::TemplateCycle { .. } => None,
        }
    }
}

/// In-memory map of template name → component bundle (private).
pub(crate) type EntityTemplates = BTreeMap<String, EntitySpawnYaml>;

/// Flattened template stored on Api and used at spawn.
pub(crate) type EntitySpawnYaml = EntityComponents;

/// One parent name or an ordered list (serde untagged).
#[derive(Deserialize, Clone, Default, PartialEq, Debug)]
#[serde(untagged)]
enum TemplateParents {
    #[default]
    None,
    One(String),
    Many(Vec<String>),
}

impl TemplateParents {
    fn into_vec(self) -> Vec<String> {
        match self {
            Self::None => Vec::new(),
            Self::One(name) => vec![name],
            Self::Many(names) => names,
        }
    }
}

/// Parsed template entry (load only); `template` is stripped after resolve.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct EntityTemplateRaw {
    #[serde(default)]
    template: TemplateParents,
    #[serde(flatten)]
    components: EntityComponents,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TemplatesFileRoot {
    entities: BTreeMap<String, EntityTemplateRaw>,
}

fn resolve_template(
    name: &str,
    child: &str,
    raw: &BTreeMap<String, EntityTemplateRaw>,
    stack: &mut Vec<String>,
    memo: &mut BTreeMap<String, EntityComponents>,
) -> Result<EntityComponents, ImportError> {
    if let Some(resolved) = memo.get(name) {
        return Ok(*resolved);
    }
    if stack.iter().any(|s| s == name) {
        let mut chain = stack.clone();
        chain.push(name.to_owned());
        return Err(ImportError::TemplateCycle { chain });
    }

    let entry = raw.get(name).ok_or_else(|| ImportError::UnknownTemplateParent {
        child: child.to_owned(),
        parent: name.to_owned(),
    })?;

    stack.push(name.to_owned());

    let mut base = EntityComponents::default();
    let parent_names = entry.template.clone().into_vec();
    for parent_name in parent_names {
        let parent_doc = resolve_template(&parent_name, child, raw, stack, memo)?;
        base = merge_components(&base, &parent_doc);
    }

    let merged = merge_components(&base, &entry.components);
    memo.insert(name.to_owned(), merged);
    stack.pop();

    Ok(merged)
}

fn resolve_all_templates(
    raw: &BTreeMap<String, EntityTemplateRaw>,
) -> Result<EntityTemplates, ImportError> {
    let mut memo = BTreeMap::new();
    for name in raw.keys() {
        resolve_template(name, name, raw, &mut Vec::new(), &mut memo)?;
    }
    Ok(memo)
}

fn spawn_from_doc(world: &mut World, template_name: &str, doc: &EntityComponents) -> Entity {
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
        let flattened = resolve_all_templates(&parsed.entities)?;
        self.templates = Some(flattened);
        Ok(())
    }

    /// Spawns one entity from a previously loaded template, applying optional component overrides.
    ///
    /// Each `Some` field in `overrides` replaces the template value; `None` fields leave the
    /// template unchanged. [`EntityComponents::default()`] spawns the template as loaded.
    ///
    /// # Errors
    ///
    /// Returns [`ImportError::TemplatesNotLoaded`] if no successful load yet.
    /// Returns [`ImportError::UnknownTemplate`] if `template_name` is missing.
    pub fn spawn_entity(
        &mut self,
        template_name: &str,
        overrides: EntityComponents,
    ) -> Result<Entity, ImportError> {
        let templates = self
            .templates
            .as_ref()
            .ok_or(ImportError::TemplatesNotLoaded)?;
        let base = *templates
            .get(template_name)
            .ok_or_else(|| ImportError::UnknownTemplate(template_name.to_owned()))?;
        let doc = merge_components(&base, &overrides);
        Ok(spawn_from_doc(
            self.core_mut().world_mut(),
            template_name,
            &doc,
        ))
    }
}

#[cfg(test)]
mod resolve_tests {
    use super::*;

    fn raw_map(pairs: &[(&str, EntityTemplateRaw)]) -> BTreeMap<String, EntityTemplateRaw> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), v.clone()))
            .collect()
    }

    fn entry(template: TemplateParents, components: EntityComponents) -> EntityTemplateRaw {
        EntityTemplateRaw {
            template,
            components,
        }
    }

    #[test]
    fn resolve_single_parent() {
        let raw = raw_map(&[
            (
                "unit",
                entry(
                    TemplateParents::None,
                    EntityComponents {
                        faction: Some(Faction(1)),
                        ..Default::default()
                    },
                ),
            ),
            (
                "scout",
                entry(
                    TemplateParents::One("unit".to_owned()),
                    EntityComponents {
                        velocity: Some(Velocity { vx: 2.0, vy: 0.0 }),
                        ..Default::default()
                    },
                ),
            ),
        ]);
        let resolved = resolve_all_templates(&raw).expect("resolve");
        let scout = resolved.get("scout").expect("scout");
        assert_eq!(scout.faction, Some(Faction(1)));
        assert_eq!(scout.velocity, Some(Velocity { vx: 2.0, vy: 0.0 }));
    }

    #[test]
    fn resolve_unknown_parent() {
        let raw = raw_map(&[(
            "scout",
            entry(
                TemplateParents::One("ghost".to_owned()),
                EntityComponents::default(),
            ),
        )]);
        let err = resolve_all_templates(&raw).unwrap_err();
        assert!(matches!(
            err,
            ImportError::UnknownTemplateParent { child, parent }
            if child == "scout" && parent == "ghost"
        ));
    }

    #[test]
    fn resolve_cycle() {
        let raw = raw_map(&[
            (
                "scout",
                entry(
                    TemplateParents::One("unit".to_owned()),
                    EntityComponents::default(),
                ),
            ),
            (
                "unit",
                entry(
                    TemplateParents::One("scout".to_owned()),
                    EntityComponents::default(),
                ),
            ),
        ]);
        let err = resolve_all_templates(&raw).unwrap_err();
        assert!(matches!(
            err,
            ImportError::TemplateCycle { chain }
            if chain == ["scout", "unit", "scout"]
        ));
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
    fn spawn_entity_overrides_faction() {
        let mut api = Api::new();
        load_fixture(&mut api);
        let entity = api
            .spawn_entity(
                "scout",
                EntityComponents {
                    faction: Some(Faction(99)),
                    ..Default::default()
                },
            )
            .expect("spawn with faction override");
        let world = api.core_mut().world_mut();
        let faction = world.get::<Faction>(entity).expect("faction");
        assert_eq!(faction.0, 99);
        let velocity = world.get::<Velocity>(entity).expect("velocity");
        assert_eq!(velocity.vx, 2.0);
        assert_eq!(velocity.vy, 0.0);
    }

    #[test]
    fn spawn_entity_overrides_position() {
        let mut api = Api::new();
        load_fixture(&mut api);
        let entity = api
            .spawn_entity(
                "scout",
                EntityComponents {
                    position: Some(Position { x: 100.0, y: 200.0 }),
                    ..Default::default()
                },
            )
            .expect("spawn with position override");
        let world = api.core_mut().world_mut();
        let position = world.get::<Position>(entity).expect("position");
        assert_eq!(position.x, 100.0);
        assert_eq!(position.y, 200.0);
        let faction = world.get::<Faction>(entity).expect("faction still from template");
        assert_eq!(faction.0, 1);
    }

    #[test]
    fn spawn_entity_no_overrides_matches_template() {
        let mut api = Api::new();
        load_fixture(&mut api);
        let entity = api
            .spawn_entity("scout", EntityComponents::default())
            .expect("spawn scout");
        let world = api.core_mut().world_mut();
        let position = world.get::<Position>(entity).expect("position");
        assert_eq!(position.x, 0.0);
        assert_eq!(position.y, 0.0);
        let velocity = world.get::<Velocity>(entity).expect("velocity");
        assert_eq!(velocity.vx, 2.0);
        assert_eq!(velocity.vy, 0.0);
        let faction = world.get::<Faction>(entity).expect("faction");
        assert_eq!(faction.0, 1);
    }

    #[test]
    fn import_error_unknown_template_parent_display() {
        let err = ImportError::UnknownTemplateParent {
            child: "scout".to_owned(),
            parent: "ghost".to_owned(),
        };
        assert_eq!(
            err.to_string(),
            r#"template "ghost" not found (referenced from "scout")"#
        );
    }

    #[test]
    fn import_error_template_cycle_display() {
        let err = ImportError::TemplateCycle {
            chain: vec![
                "scout".to_owned(),
                "unit".to_owned(),
                "scout".to_owned(),
            ],
        };
        assert_eq!(
            err.to_string(),
            "template inheritance cycle: scout -> unit -> scout"
        );
    }

    #[test]
    fn spawn_entity_without_load() {
        let mut api = Api::new();
        let err = api.spawn_entity("scout", EntityComponents::default()).unwrap_err();
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
            api.spawn_entity("scout", EntityComponents::default()).unwrap_err(),
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
    fn spawn_entity_unknown_template() {
        let mut api = Api::new();
        load_fixture(&mut api);
        let err = api.spawn_entity("nope", EntityComponents::default()).unwrap_err();
        assert!(matches!(err, ImportError::UnknownTemplate(name) if name == "nope"));
    }

    #[test]
    fn spawn_entity_scout() {
        let mut api = Api::new();
        load_fixture(&mut api);
        let entity = api.spawn_entity("scout", EntityComponents::default()).expect("spawn scout");
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
    fn spawn_entity_base() {
        let mut api = Api::new();
        load_fixture(&mut api);
        let entity = api.spawn_entity("base", EntityComponents::default()).expect("spawn base");
        let world = api.core_mut().world_mut();
        assert!(world.get::<Position>(entity).is_none());
        assert!(world.get::<Velocity>(entity).is_none());
        let faction = world.get::<Faction>(entity).expect("faction");
        assert_eq!(faction.0, 2);
        let entity_type = world.get::<EntityType>(entity).expect("entity_type");
        assert_eq!(entity_type.0, "base");
    }

    #[test]
    fn spawn_entity_marker() {
        let mut api = Api::new();
        load_fixture(&mut api);
        let entity = api.spawn_entity("marker", EntityComponents::default()).expect("spawn marker");
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
    fn spawn_entity_twice_same_name() {
        let mut api = Api::new();
        load_fixture(&mut api);
        let e1 = api.spawn_entity("scout", EntityComponents::default()).expect("first scout");
        let e2 = api.spawn_entity("scout", EntityComponents::default()).expect("second scout");
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
        assert!(api.spawn_entity("a", EntityComponents::default()).is_err());
        let entity = api.spawn_entity("b", EntityComponents::default()).expect("only B remains");
        let world = api.core_mut().world_mut();
        assert_eq!(world.get::<Faction>(entity).map(|f| f.0), Some(2));
        assert_eq!(world.get::<EntityType>(entity).map(|t| t.0.as_str()), Some("b"));
    }

    #[test]
    fn spawn_entity_exports_entity_type_in_world_json() {
        let mut api = Api::new();
        load_fixture(&mut api);
        api.spawn_entity("scout", EntityComponents::default()).expect("spawn scout");
        api.spawn_entity("marker", EntityComponents::default()).expect("spawn marker");

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

    #[test]
    fn inherit_single_level() {
        let yaml = r"
entities:
  unit:
    faction: 1
  scout:
    template: unit
    velocity: { vx: 2, vy: 0 }
";
        let mut api = Api::new();
        api.load_templates_yaml(yaml).expect("load");

        let entity = api.spawn_entity("scout", EntityComponents::default()).expect("spawn scout");
        let world = api.core_mut().world_mut();
        assert_eq!(world.get::<Faction>(entity).map(|f| f.0), Some(1));
        let velocity = world.get::<Velocity>(entity).expect("velocity");
        assert_eq!(velocity.vx, 2.0);
        assert_eq!(velocity.vy, 0.0);
    }

    #[test]
    fn inherit_chain() {
        let yaml = r"
entities:
  a:
    faction: 1
  b:
    template: a
    velocity: { vx: 1, vy: 0 }
  c:
    template: b
    position: { x: 0, y: 0 }
";
        let mut api = Api::new();
        api.load_templates_yaml(yaml).expect("load");
        let entity = api.spawn_entity("c", EntityComponents::default()).expect("spawn c");
        let world = api.core_mut().world_mut();
        assert_eq!(world.get::<Faction>(entity).map(|f| f.0), Some(1));
        let velocity = world.get::<Velocity>(entity).expect("velocity");
        assert_eq!(velocity.vx, 1.0);
        let position = world.get::<Position>(entity).expect("position");
        assert_eq!(position.x, 0.0);
        assert_eq!(position.y, 0.0);
    }

    #[test]
    fn inherit_override_component() {
        let yaml = r"
entities:
  unit:
    position: { x: 1, y: 1 }
  scout:
    template: unit
    position: { x: 9, y: 9 }
";
        let mut api = Api::new();
        api.load_templates_yaml(yaml).expect("load");
        let entity = api.spawn_entity("scout", EntityComponents::default()).expect("spawn scout");
        let world = api.core_mut().world_mut();
        let position = world.get::<Position>(entity).expect("position");
        assert_eq!(position.x, 9.0);
        assert_eq!(position.y, 9.0);
    }

    #[test]
    fn inherit_child_only_template() {
        let yaml = r"
entities:
  unit:
    faction: 1
    velocity: { vx: 0.5, vy: 0 }
  clone:
    template: unit
";
        let mut api = Api::new();
        api.load_templates_yaml(yaml).expect("load");

        let unit = api.spawn_entity("unit", EntityComponents::default()).expect("spawn unit");
        let clone = api.spawn_entity("clone", EntityComponents::default()).expect("spawn clone");
        let world = api.core_mut().world_mut();
        assert_eq!(world.get::<Faction>(unit).map(|f| f.0), Some(1));
        assert_eq!(world.get::<Faction>(clone).map(|f| f.0), Some(1));
        assert!(world.get::<Velocity>(clone).is_some());
    }

    #[test]
    fn inherit_multiple_templates() {
        let yaml = r"
entities:
  unit:
    faction: 1
  tank:
    template: unit
    velocity: { vx: 0.5, vy: 0 }
  heavy_tank:
    template: [unit, tank]
    faction: 3
";
        let mut api = Api::new();
        api.load_templates_yaml(yaml).expect("load");
        let entity = api.spawn_entity("heavy_tank", EntityComponents::default()).expect("spawn");
        let world = api.core_mut().world_mut();
        assert_eq!(world.get::<Faction>(entity).map(|f| f.0), Some(3));
        let velocity = world.get::<Velocity>(entity).expect("velocity from tank");
        assert_eq!(velocity.vx, 0.5);
        assert_eq!(velocity.vy, 0.0);
    }

    #[test]
    fn inherit_multiple_string_equivalent() {
        let yaml_one = r"
entities:
  unit:
    faction: 1
  scout:
    template: unit
    velocity: { vx: 2, vy: 0 }
";
        let yaml_many = r"
entities:
  unit:
    faction: 1
  scout:
    template: [unit]
    velocity: { vx: 2, vy: 0 }
";
        let mut api_one = Api::new();
        api_one.load_templates_yaml(yaml_one).expect("load one");
        let mut api_many = Api::new();
        api_many.load_templates_yaml(yaml_many).expect("load many");

        let e1 = api_one.spawn_entity("scout", EntityComponents::default()).expect("spawn one");
        let e2 = api_many.spawn_entity("scout", EntityComponents::default()).expect("spawn many");
        let w1 = api_one.core_mut().world_mut();
        let w2 = api_many.core_mut().world_mut();
        assert_eq!(w1.get::<Faction>(e1).map(|f| f.0), w2.get::<Faction>(e2).map(|f| f.0));
        assert_eq!(
            w1.get::<Velocity>(e1).map(|v| (v.vx, v.vy)),
            w2.get::<Velocity>(e2).map(|v| (v.vx, v.vy))
        );
    }

    #[test]
    fn inherit_multiple_then_child_override() {
        let yaml = r"
entities:
  unit:
    faction: 1
  tank:
    template: unit
    faction: 2
  hybrid:
    template: [unit, tank]
    faction: 9
";
        let mut api = Api::new();
        api.load_templates_yaml(yaml).expect("load");
        let entity = api.spawn_entity("hybrid", EntityComponents::default()).expect("spawn");
        let world = api.core_mut().world_mut();
        assert_eq!(world.get::<Faction>(entity).map(|f| f.0), Some(9));
    }

    #[test]
    fn inherit_empty_template_list() {
        let yaml_with = r"
entities:
  unit:
    faction: 1
  bare:
    template: []
";
        let yaml_without = r"
entities:
  unit:
    faction: 1
  bare: {}
";
        let mut api_with = Api::new();
        api_with.load_templates_yaml(yaml_with).expect("load with");
        let mut api_without = Api::new();
        api_without
            .load_templates_yaml(yaml_without)
            .expect("load without");

        let e1 = api_with.spawn_entity("bare", EntityComponents::default()).expect("spawn with");
        let e2 = api_without.spawn_entity("bare", EntityComponents::default()).expect("spawn without");
        let w1 = api_with.core_mut().world_mut();
        let w2 = api_without.core_mut().world_mut();
        assert!(w1.get::<Faction>(e1).is_none());
        assert!(w2.get::<Faction>(e2).is_none());
    }

    #[test]
    fn load_unknown_template_parent() {
        let mut api = Api::new();
        let yaml = r"
entities:
  scout:
    template: ghost
";
        let err = api.load_templates_yaml(yaml).unwrap_err();
        assert!(matches!(
            err,
            ImportError::UnknownTemplateParent { child, parent }
            if child == "scout" && parent == "ghost"
        ));
        assert!(api.templates.is_none());
    }

    #[test]
    fn load_template_cycle() {
        let mut api = Api::new();
        let yaml = r"
entities:
  scout:
    template: unit
  unit:
    template: scout
";
        let err = api.load_templates_yaml(yaml).unwrap_err();
        assert!(matches!(
            err,
            ImportError::TemplateCycle { chain }
            if chain == ["scout", "unit", "scout"]
        ));
        assert!(api.templates.is_none());
    }

    #[test]
    fn load_failed_resolve_keeps_previous() {
        let mut api = Api::new();
        api.load_templates_yaml("entities:\n  a:\n    faction: 1\n")
            .expect("first load");
        let err = api
            .load_templates_yaml("entities:\n  bad:\n    template: missing\n")
            .unwrap_err();
        assert!(matches!(
            err,
            ImportError::UnknownTemplateParent { .. }
        ));
        let templates = api.templates.as_ref().expect("first map kept");
        assert!(templates.contains_key("a"));
        assert!(!templates.contains_key("bad"));
    }

    #[test]
    fn spawn_base_template() {
        let yaml = r"
entities:
  unit:
    faction: 1
  scout:
    template: unit
    velocity: { vx: 2, vy: 0 }
";
        let mut api = Api::new();
        api.load_templates_yaml(yaml).expect("load");
        let entity = api.spawn_entity("unit", EntityComponents::default()).expect("spawn base");
        let world = api.core_mut().world_mut();
        assert_eq!(world.get::<Faction>(entity).map(|f| f.0), Some(1));
        assert_eq!(
            world.get::<EntityType>(entity).map(|t| t.0.as_str()),
            Some("unit")
        );
    }
}
