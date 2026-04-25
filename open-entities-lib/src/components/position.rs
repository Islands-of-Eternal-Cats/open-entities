use bevy_ecs::prelude::Component;
use serde::Deserialize;

/// Component: Position of an entity (also used for YAML deserialization).
#[derive(Component, Clone, Copy, Debug, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

impl Position {
    /// Returns a new position shifted by the provided delta.
    pub fn shifted(self, dx: f32, dy: f32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
        }
    }
}
