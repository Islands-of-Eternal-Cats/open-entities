//! Загрузка описаний сущностей из YAML и создание сущностей в ECS по имени типа.
//!
//! Компоненты заданы заранее (Position, Velocity); поля YAML маппятся на них.
//! У каждого типа сущности может быть свой набор компонентов.

use crate::components::{Position, Velocity};
use bevy_ecs::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Шаблон одной сущности: какие компоненты и с какими значениями.
/// Отсутствующие поля означают отсутствие компонента у этого типа.
#[derive(Debug, Clone, Deserialize)]
pub struct EntityTemplate {
    pub position: Option<Position>,
    pub velocity: Option<Velocity>,
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
        let s = std::fs::read_to_string(path.as_ref()).map_err(|e| LoadError::Io(e.to_string()))?;
        Self::load_from_str(&s)
    }

    /// Загрузить определения из YAML-строки.
    pub fn load_from_str(s: &str) -> Result<Self, LoadError> {
        let file: EntityDefinitionsFile =
            yaml_serde::from_str(s).map_err(|e| LoadError::Yaml(e.to_string()))?;
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
    Io(String),
    Yaml(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Io(s) => write!(f, "IO: {}", s),
            LoadError::Yaml(s) => write!(f, "YAML: {}", s),
        }
    }
}

impl std::error::Error for LoadError {}

/// Создать одну сущность в ECS по имени типа из загруженных определений.
/// Возвращает `Some(Entity)` если тип найден и сущность создана, иначе `None`.
pub fn spawn_entity_by_type(
    commands: &mut Commands,
    definitions: &EntityDefinitions,
    type_name: &str,
) -> Option<Entity> {
    let template = definitions.get(type_name)?;

    let mut entity = commands.spawn_empty();

    if let Some(p) = &template.position {
        entity.insert(*p);
    }
    if let Some(v) = &template.velocity {
        entity.insert(*v);
    }

    Some(entity.id())
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
        if let Some(e) = spawn_entity_by_type(commands, &definitions, name) {
            entities.push(e);
        }
    }
    Ok(entities)
}
