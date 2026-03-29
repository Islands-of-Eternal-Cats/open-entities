//! Загрузка описаний сущностей из YAML и создание сущностей в ECS по имени типа.
//!
//! Подвижность типа задаётся только полем **`base_move_speed`**: значение `> 0` — юнит может двигаться
//! ([`Velocity`] при спавне — нулевая, [`BaseMoveSpeed`] — из шаблона); иначе сущность статична
//! (только [`Position`], без [`Velocity`]/[`BaseMoveSpeed`]). [`EntityTemplate`] с `#[serde(default)]`:
//! допустима пустая карта `{}`.

use crate::components::{BaseMoveSpeed, Position, Velocity};
use bevy_ecs::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Шаблон одной сущности: позиция и базовая скорость движения.
/// `base_move_speed` отсутствует, ноль или `≤ 0` — тип неподвижен; `> 0` — подвижный тип.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct EntityTemplate {
    pub position: Option<Position>,
    /// Базовая скорость движения (юнит/с). `> 0` — юнит подвижен и получает [`BaseMoveSpeed`]; иначе статика.
    pub base_move_speed: Option<f32>,
}

/// `true`, если в YAML задан положительный `base_move_speed` (тип может двигаться).
pub fn is_movable(template: &EntityTemplate) -> bool {
    matches!(template.base_move_speed, Some(s) if s > 0.0)
}

/// Корневая структура YAML-файла: именованные типы сущностей.
#[derive(Debug, Clone, Deserialize)]
pub struct EntityDefinitionsFile {
    pub entities: HashMap<String, EntityTemplate>,
}

/// Загруженные определения: по имени типа — шаблон сущности.
#[derive(Debug, Clone, Default, Resource)]
pub struct EntityDefinitions {
    definitions: HashMap<String, EntityTemplate>,
}

impl EntityDefinitions {
    /// Создать пустой набор определений.
    pub fn new() -> Self {
        Self {
            definitions: HashMap::new(),
        }
    }

    /// Загрузить определения из YAML-файла.
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self, LoadError> {
        let path_ref = path.as_ref();
        let s = std::fs::read_to_string(path_ref).map_err(|e| LoadError::Io {
            op: "load_from_path",
            path: Some(path_ref.to_path_buf()),
            source: e.to_string(),
        })?;
        Self::load_from_str(&s)
    }

    /// Загрузить определения из YAML-строки.
    pub fn load_from_str(s: &str) -> Result<Self, LoadError> {
        let file: EntityDefinitionsFile = yaml_serde::from_str(s).map_err(|e| LoadError::Yaml {
            op: "load_from_str",
            source: e.to_string(),
        })?;
        Ok(Self {
            definitions: file.entities,
        })
    }

    /// Добавить или перезаписать определение типа.
    pub fn insert(&mut self, name: String, template: EntityTemplate) {
        self.definitions.insert(name, template);
    }

    /// Получить шаблон по имени типа.
    pub fn get(&self, type_name: &str) -> Option<&EntityTemplate> {
        self.definitions.get(type_name)
    }

    /// Имена всех загруженных типов.
    pub fn type_names(&self) -> impl Iterator<Item = &String> {
        self.definitions.keys()
    }
}

/// Ошибки загрузки
#[derive(Debug, Clone)]
pub enum LoadError {
    Io {
        op: &'static str,
        path: Option<PathBuf>,
        source: String,
    },
    Yaml {
        op: &'static str,
        source: String,
    },
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Io { op, path, source } => {
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
            LoadError::Yaml { op, source } => {
                write!(f, "YAML parse error during {}: {}", op, source)
            }
        }
    }
}

impl std::error::Error for LoadError {}

/// Ошибки спавна сущности по имени типа.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpawnError {
    UnknownEntityType { type_name: String },
    DefinitionsNotLoaded,
}

impl std::fmt::Display for SpawnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpawnError::UnknownEntityType { type_name } => {
                write!(f, "Unknown entity type: '{}'", type_name)
            }
            SpawnError::DefinitionsNotLoaded => {
                write!(f, "Entity definitions resource is not loaded")
            }
        }
    }
}

impl std::error::Error for SpawnError {}

fn spawn_from_template_in_world(
    world: &mut World,
    template: EntityTemplate,
    position_override: Option<Position>,
    include_initial_velocity: bool,
) -> Entity {
    let mut entity = world.spawn_empty();

    // Position: override wins; otherwise use the template.
    if let Some(p) = position_override.or(template.position) {
        entity.insert(p);
    }

    if is_movable(&template) {
        let cap = template
            .base_move_speed
            .expect("is_movable implies positive base_move_speed");
        entity.insert(BaseMoveSpeed(cap));
        if include_initial_velocity {
            entity.insert(Velocity { vx: 0.0, vy: 0.0 });
        }
    }

    entity.id()
}

/// Создать одну сущность в ECS по имени типа из загруженных определений.
/// Возвращает `Ok(Entity)` если тип найден и сущность создана, иначе ошибку.
pub fn spawn_entity_by_type(
    commands: &mut Commands,
    definitions: &EntityDefinitions,
    type_name: &str,
) -> Result<Entity, SpawnError> {
    let template = definitions
        .get(type_name)
        .ok_or_else(|| SpawnError::UnknownEntityType {
            type_name: type_name.to_string(),
        })?;

    let mut entity = commands.spawn_empty();

    if let Some(p) = &template.position {
        entity.insert(*p);
    }
    if is_movable(template) {
        let cap = template
            .base_move_speed
            .expect("is_movable implies positive base_move_speed");
        entity.insert(BaseMoveSpeed(cap));
        entity.insert(Velocity { vx: 0.0, vy: 0.0 });
    }

    Ok(entity.id())
}

/// Создать одну сущность в ECS по имени типа, используя ресурс `EntityDefinitions` в мире.
/// Возвращает `Ok(Entity)` если тип найден и сущность создана, иначе ошибку.
pub fn spawn_entity_by_type_in_world(
    world: &mut World,
    type_name: &str,
) -> Result<Entity, SpawnError> {
    let template = world
        .get_resource::<EntityDefinitions>()
        .ok_or(SpawnError::DefinitionsNotLoaded)?
        .get(type_name)
        .cloned()
        .ok_or_else(|| SpawnError::UnknownEntityType {
            type_name: type_name.to_string(),
        })?;

    Ok(spawn_from_template_in_world(world, template, None, true))
}

/// Create one entity by type name at the given position, using `EntityDefinitions` resource in the world.
///
/// Host-controlled spawn (e.g. WASM/JS). Подвижные типы (`base_move_speed` > 0) получают [`BaseMoveSpeed`],
/// но без начальной [`Velocity`] — она появится при приказе движения.
pub fn spawn_entity_by_type_at_in_world(
    world: &mut World,
    type_name: &str,
    x: f32,
    y: f32,
) -> Result<Entity, SpawnError> {
    let defs = world
        .get_resource::<EntityDefinitions>()
        .ok_or(SpawnError::DefinitionsNotLoaded)?;

    let template = defs
        .get(type_name)
        .cloned()
        .ok_or_else(|| SpawnError::UnknownEntityType {
            type_name: type_name.to_string(),
        })?;

    Ok(spawn_from_template_in_world(
        world,
        template,
        Some(Position { x, y }),
        false,
    ))
}

/// Загрузить определения из файла и создать по одной сущности каждого типа.
/// Путь к YAML — относительно текущей рабочей директории.
pub fn load_and_spawn_all_from_path(
    commands: &mut Commands,
    path: &Path,
) -> Result<Vec<Entity>, LoadError> {
    let definitions = EntityDefinitions::load_from_path(path)?;
    let names: Vec<String> = definitions.type_names().cloned().collect();
    let mut entities = Vec::with_capacity(names.len());
    for name in &names {
        // Names are collected from definitions; unknown-type error is unreachable here.
        let e =
            spawn_entity_by_type(commands, &definitions, name).map_err(|err| LoadError::Yaml {
                op: "load_and_spawn_all_from_path",
                source: err.to_string(),
            })?;
        entities.push(e);
    }
    Ok(entities)
}
