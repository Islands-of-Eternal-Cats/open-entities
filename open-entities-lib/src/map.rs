//! Initial map layout: world bounds plus the entities a world starts with.
//!
//! A map entry names a template and carries the same override fields as
//! [`Api::spawn_entity`](crate::Api::spawn_entity), so the format adds nothing to learn beyond the
//! `template` key. The loader inserts **only** what the template and the entry ask for: an entity
//! that names no `velocity` gets no [`Velocity`](crate::components::Velocity) component.

use serde::{Deserialize, Serialize};

use crate::api::Api;
use crate::entity_components::EntityComponents;
use crate::import::ImportError;
use crate::orders::EntityId;

/// Extent of the playable area in world units, for hosts that need to frame or clamp a view.
///
/// The simulation does not enforce these bounds; nothing stops an entity from leaving them.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapBounds {
    pub width: f32,
    pub height: f32,
}

/// Errors while loading a map layout.
#[derive(Debug)]
pub enum MapError {
    /// YAML syntax, type mismatch, or unknown field.
    Yaml(yaml_serde::Error),
    /// `load_map_yaml` called before a successful `load_templates_yaml`.
    TemplatesNotLoaded,
    /// An entry names a template that was not loaded; nothing was spawned.
    UnknownTemplate { index: usize, name: String },
    /// A spawn failed after validation passed.
    Spawn(ImportError),
}

impl std::fmt::Display for MapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Yaml(err) => write!(f, "map load failed: {err}"),
            Self::TemplatesNotLoaded => {
                f.write_str("templates not loaded; call load_templates_yaml first")
            }
            Self::UnknownTemplate { index, name } => {
                write!(f, "spawn #{index} names unknown template: {name}")
            }
            Self::Spawn(err) => write!(f, "map spawn failed: {err}"),
        }
    }
}

impl std::error::Error for MapError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Yaml(err) => Some(err),
            Self::Spawn(err) => Some(err),
            Self::TemplatesNotLoaded | Self::UnknownTemplate { .. } => None,
        }
    }
}

/// One entry under `spawns`: a template name plus override fields.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MapSpawn {
    template: String,
    #[serde(flatten)]
    overrides: EntityComponents,
}

/// Root of a map file: optional `map` bounds and a list of `spawns`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MapFile {
    #[serde(default)]
    map: Option<MapBounds>,
    #[serde(default)]
    spawns: Vec<MapSpawn>,
}

impl Api {
    /// Spawns a starting layout from YAML and records its bounds.
    ///
    /// Every entry's template name is checked before anything is spawned, so a typo leaves the
    /// world untouched instead of half-populated. Returns the spawned entities in file order.
    ///
    /// # Errors
    ///
    /// [`MapError::Yaml`] for invalid YAML, [`MapError::TemplatesNotLoaded`] when no templates are
    /// loaded, [`MapError::UnknownTemplate`] when an entry names a template that does not exist.
    pub fn load_map_yaml(&mut self, yaml: &str) -> Result<Vec<EntityId>, MapError> {
        let file: MapFile = yaml_serde::from_str(yaml).map_err(MapError::Yaml)?;

        {
            let templates = self
                .templates
                .as_ref()
                .ok_or(MapError::TemplatesNotLoaded)?;
            for (index, spawn) in file.spawns.iter().enumerate() {
                if !templates.contains_key(&spawn.template) {
                    return Err(MapError::UnknownTemplate {
                        index,
                        name: spawn.template.clone(),
                    });
                }
            }
        }

        let mut spawned = Vec::with_capacity(file.spawns.len());
        for spawn in &file.spawns {
            let entity = self
                .spawn_entity(&spawn.template, spawn.overrides)
                .map_err(MapError::Spawn)?;
            spawned.push(entity);
        }

        self.map_bounds = file.map;
        Ok(spawned)
    }

    /// Bounds from the last loaded map, or `None` when no map declared them.
    #[must_use]
    pub const fn map_bounds(&self) -> Option<MapBounds> {
        self.map_bounds
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{EntityType, Faction, Position, Velocity};

    const TEMPLATES_YAML: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/spawn_entity_templates.yaml"
    ));

    const MAP_YAML: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/init_map.yaml"
    ));

    fn api_with_templates() -> Api {
        let mut api = Api::new();
        api.load_templates_yaml(TEMPLATES_YAML)
            .expect("load templates");
        api
    }

    fn spawned_count(api: &mut Api) -> usize {
        let world = api.core_mut().world_mut();
        let mut query = world.query::<&EntityType>();
        query.iter(world).count()
    }

    #[test]
    fn map_spawns_entities_with_overrides() {
        let mut api = api_with_templates();

        let spawned = api
            .load_map_yaml(
                r"
map:
  width: 200.0
  height: 120.0

spawns:
  - template: marker
    position: { x: 20.0, y: 20.0 }
    faction: 1
  - template: scout
    position: { x: 30.0, y: 20.0 }
",
            )
            .expect("load map");

        assert_eq!(spawned.len(), 2);
        assert_eq!(
            api.map_bounds(),
            Some(MapBounds {
                width: 200.0,
                height: 120.0
            })
        );

        let world = api.core_mut().world();
        let marker = spawned[0].to_entity().expect("live entity");
        let position = world.get::<Position>(marker).expect("position override");
        assert_eq!(position.x, 20.0);
        assert_eq!(position.y, 20.0);
        assert_eq!(world.get::<Faction>(marker), Some(&Faction(1)));

        let scout = spawned[1].to_entity().expect("live entity");
        let position = world.get::<Position>(scout).expect("position override");
        assert_eq!(position.x, 30.0);
        // Not overridden, so the template's own faction survives.
        assert_eq!(world.get::<Faction>(scout), Some(&Faction(1)));
    }

    #[test]
    fn loader_does_not_invent_velocity() {
        let mut api = api_with_templates();

        let spawned = api
            .load_map_yaml(
                r"
spawns:
  - template: marker
    position: { x: 1.0, y: 1.0 }
",
            )
            .expect("load map");

        // `marker` declares no velocity, so it must not receive one: in this engine a Velocity is
        // movement, and an entity that never asked to move would drift forever.
        let marker = spawned[0].to_entity().expect("live entity");
        assert!(api.core_mut().world().get::<Velocity>(marker).is_none());
    }

    #[test]
    fn unknown_template_spawns_nothing() {
        let mut api = api_with_templates();

        let err = api
            .load_map_yaml(
                r"
spawns:
  - template: marker
    position: { x: 1.0, y: 1.0 }
  - template: ghost
    position: { x: 2.0, y: 2.0 }
",
            )
            .expect_err("unknown template");

        match err {
            MapError::UnknownTemplate { index, ref name } => {
                assert_eq!(index, 1);
                assert_eq!(name, "ghost");
            }
            other => panic!("expected UnknownTemplate, got {other:?}"),
        }
        assert!(err.to_string().contains("ghost"));
        assert_eq!(spawned_count(&mut api), 0, "map must not half-populate");
    }

    #[test]
    fn templates_must_be_loaded_first() {
        let mut api = Api::new();
        let err = api
            .load_map_yaml("spawns: []")
            .expect_err("no templates loaded");
        assert!(matches!(err, MapError::TemplatesNotLoaded));
    }

    #[test]
    fn bounds_are_optional() {
        let mut api = api_with_templates();
        let spawned = api
            .load_map_yaml("spawns:\n  - template: marker\n")
            .expect("load map without bounds");
        assert_eq!(spawned.len(), 1);
        assert_eq!(api.map_bounds(), None);
    }

    #[test]
    fn invalid_yaml_is_reported() {
        let mut api = api_with_templates();
        let err = api
            .load_map_yaml("spawns:\n  - template: marker\n    nope: 1\n")
            .expect_err("unknown field");
        assert!(matches!(err, MapError::Yaml(_)));
    }

    #[test]
    fn fixture_map_loads() {
        let mut api = api_with_templates();
        let spawned = api.load_map_yaml(MAP_YAML).expect("load fixture map");
        assert_eq!(spawned.len(), 3);
        assert_eq!(
            api.map_bounds(),
            Some(MapBounds {
                width: 200.0,
                height: 200.0
            })
        );
    }
}
