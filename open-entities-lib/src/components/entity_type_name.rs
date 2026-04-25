use bevy_ecs::prelude::Component;

/// Имя типа сущности из YAML (ключ под `entities:`), задаётся при спавне из [`crate::entity_loader`].
#[derive(Component, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EntityTypeName(pub String);
