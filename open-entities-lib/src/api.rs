use crate::core::Core;
use crate::import::EntityTemplates;
use crate::simulation::{ArrivedThisTick, SimDelta, TickError};
use crate::systems::MAX_DT_MS;

/// Public facade over [`Core`] for simulation operations, export, and import.
pub struct Api {
    core: Core,
    pub(crate) templates: Option<EntityTemplates>,
}

impl Api {
    /// Creates an API backed by a new empty [`Core`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            core: Core::new(),
            templates: None,
        }
    }

    /// Mutable access to the underlying core.
    pub const fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    /// Advances simulation by `dt_ms` milliseconds (clamped to [`MAX_DT_MS`]).
    ///
    /// # Errors
    ///
    /// Returns [`TickError::ZeroDeltaTime`] when `dt_ms == 0`.
    pub fn tick(&mut self, dt_ms: u32) -> Result<(), TickError> {
        if dt_ms == 0 {
            return Err(TickError::ZeroDeltaTime);
        }
        let dt_ms = dt_ms.min(MAX_DT_MS);
        let core = self.core_mut();
        let world = core.world_mut();
        world.insert_resource(SimDelta::from_ms(dt_ms));
        world.resource_mut::<ArrivedThisTick>().0.clear();
        core.run_schedule();
        Ok(())
    }
}

impl Default for Api {
    fn default() -> Self {
        Self::new()
    }
}
