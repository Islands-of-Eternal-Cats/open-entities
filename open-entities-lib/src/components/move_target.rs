use super::position::Position;
use bevy_ecs::prelude::Component;

/// World-space position the entity should move toward. Removed when close enough.
#[derive(Component, Clone, Copy, Debug)]
pub struct MoveTarget {
    pub at: Position,
}
