use bevy_ecs::prelude::Component;
use serde::Deserialize;

/// Component: Velocity of an entity (also used for YAML deserialization).
#[derive(Component, Clone, Debug, Deserialize)]
pub struct Velocity {
    pub vx: f32,
    pub vy: f32,
}
