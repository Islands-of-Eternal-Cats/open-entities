//! Загрузка карты и спавн объектов в ECS по типам из `EntityDefinitions`.

use crate::components::Position;
use crate::entity_loader::{SpawnError, spawn_entity_by_type_at_in_world};
use bevy_ecs::prelude::World;
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Метаданные карты: размеры игрового мира.
#[derive(Debug, Clone, Deserialize)]
pub struct MapMeta {
    pub width: f32,
    pub height: f32,
}

/// Точка спавна на карте.
#[derive(Debug, Clone, Deserialize)]
pub struct MapSpawn {
    #[serde(rename = "type")]
    pub entity_type: String,
    pub position: Position,
    pub faction: Option<u32>,
}

/// Корневая структура YAML файла карты.
#[derive(Debug, Clone, Deserialize)]
pub struct InitMapFile {
    pub map: MapMeta,
    pub spawns: Vec<MapSpawn>,
}

/// Ошибки загрузки и применения карты к миру.
#[derive(Debug, Clone)]
pub enum MapLoadError {
    Io {
        op: &'static str,
        path: Option<PathBuf>,
        source: String,
    },
    Yaml {
        op: &'static str,
        source: String,
    },
    Spawn {
        type_name: String,
        source: SpawnError,
    },
}

impl std::fmt::Display for MapLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MapLoadError::Io { op, path, source } => {
                if let Some(path) = path {
                    write!(
                        f,
                        "IO error during {} for '{}': {}",
                        op,
                        path.display(),
                        source
                    )
                } else {
                    write!(f, "IO error during {}: {}", op, source)
                }
            }
            MapLoadError::Yaml { op, source } => {
                write!(f, "YAML parse error during {}: {}", op, source)
            }
            MapLoadError::Spawn { type_name, source } => {
                write!(
                    f,
                    "Map spawn failed for entity type '{}': {}",
                    type_name, source
                )
            }
        }
    }
}

impl std::error::Error for MapLoadError {}

/// Загрузить карту из YAML-строки и заспавнить все объекты в существующем мире.
/// Для каждого `spawns[].type` используется `spawn_entity_by_type_at_in_world`.
pub fn load_map_from_str(world: &mut World, s: &str) -> Result<(), MapLoadError> {
    let file: InitMapFile = yaml_serde::from_str(s).map_err(|e| MapLoadError::Yaml {
        op: "load_map_from_str",
        source: e.to_string(),
    })?;

    for spawn in file.spawns {
        let type_name = spawn.entity_type;
        spawn_entity_by_type_at_in_world(
            world,
            &type_name,
            spawn.position.x,
            spawn.position.y,
            spawn.faction,
        )
        .map_err(|source| MapLoadError::Spawn { type_name, source })?;
    }
    Ok(())
}

/// Загрузить карту из YAML-файла и заспавнить все объекты в существующем мире.
pub fn load_map_from_path<P: AsRef<Path>>(world: &mut World, path: P) -> Result<(), MapLoadError> {
    let path_ref = path.as_ref();
    let s = std::fs::read_to_string(path_ref).map_err(|e| MapLoadError::Io {
        op: "load_map_from_path",
        path: Some(path_ref.to_path_buf()),
        source: e.to_string(),
    })?;
    load_map_from_str(world, &s)
}
