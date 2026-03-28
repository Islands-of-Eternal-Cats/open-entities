use bevy_ecs::prelude::Component;

/// World-space point the entity should move toward. Removed when close enough.
#[derive(Component, Clone, Copy, Debug)]
pub struct MoveTarget {
    pub x: f32,
    pub y: f32,
}
