use bevy_ecs::prelude::Component;
use serde::Deserialize;

/// Component: Position of an entity (also used for YAML deserialization).
#[derive(Component, Clone, Copy, Debug, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}
