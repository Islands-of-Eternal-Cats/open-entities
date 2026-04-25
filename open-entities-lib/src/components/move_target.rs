use super::position::Position;
use bevy_ecs::prelude::Component;

/// World-space position the entity should move toward. Removed when close enough.
#[derive(Component, Clone, Copy, Debug)]
pub struct MoveTarget {
    pub at: Position,
}

impl MoveTarget {
    /// Returns remaining vector from current position to target.
    pub fn remaining_delta_from(&self, current: Position) -> (f32, f32) {
        (self.at.x - current.x, self.at.y - current.y)
    }

    /// Returns squared distance from current position to target.
    pub fn remaining_dist_sq_from(&self, current: Position) -> f32 {
        let (dx, dy) = self.remaining_delta_from(current);
        dx * dx + dy * dy
    }
}
