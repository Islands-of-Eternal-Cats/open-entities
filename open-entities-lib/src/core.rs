use bevy_ecs::prelude::World;

/// Owns the ECS [`World`] for a simulation instance.
pub struct Core {
    world: World,
}

impl Core {
    /// Creates an empty world.
    #[must_use]
    pub fn new() -> Self {
        Self {
            world: World::new(),
        }
    }

    /// Immutable access to the underlying ECS world.
    #[must_use]
    pub const fn world(&self) -> &World {
        &self.world
    }

    /// Mutable access to the underlying ECS world.
    pub const fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }
}

impl Default for Core {
    fn default() -> Self {
        Self::new()
    }
}
