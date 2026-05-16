use crate::core::Core;
use crate::import::EntityTemplates;

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
}

impl Default for Api {
    fn default() -> Self {
        Self::new()
    }
}
