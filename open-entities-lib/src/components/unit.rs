use bevy_ecs::prelude::Component;
use serde::Deserialize;

/// Marker component: Unit
/// Used to identify entities as units in the ECS system
#[derive(Component, Clone, Debug, Deserialize)]
pub struct Unit;
