use crate::core::Core;

/// Public facade over [`Core`] for simulation operations and export.
pub struct Api {
    core: Core,
}

impl Api {
    /// Creates an API backed by a new empty [`Core`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            core: Core::new(),
        }
    }

    /// Mutable access to the underlying core.
    pub const fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }
}

impl Default for Api {
    fn default() -> Self {
        Self::new()
    }
}
