use bevy_ecs::prelude::Component;

/// Запасная скорость seek, если у сущности нет компонента [`MaxSpeed`] (например приказ до спавна с типом без `max_speed`).
pub const DEFAULT_MAX_SPEED: f32 = 45.0;

/// Лимит скорости при движении к [`super::MoveTarget`]. Для типов из YAML задаётся полем `max_speed` > 0.
#[derive(Component, Clone, Copy, Debug)]
pub struct MaxSpeed(pub f32);
