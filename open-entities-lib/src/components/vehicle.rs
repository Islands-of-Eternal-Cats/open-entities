use bevy_ecs::prelude::Component;
use serde::Deserialize;

/// Marker component: Vehicle
/// Used to identify entities as vehicles in the ECS system
#[derive(Component, Clone, Debug, Deserialize)]
pub struct Vehicle;
