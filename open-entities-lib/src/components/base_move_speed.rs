use bevy_ecs::prelude::Component;

/// Базовая скорость движения к [`super::MoveTarget`]. Для типов из YAML задаётся полем `base_move_speed` > 0.
/// Без этого компонента сущность не двигается (seek и интеграция позиции не применяются).
#[derive(Component, Clone, Copy, Debug)]
pub struct BaseMoveSpeed(pub f32);
