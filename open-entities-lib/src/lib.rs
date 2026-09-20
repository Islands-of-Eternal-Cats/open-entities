#![warn(clippy::pedantic)]
// Exact float comparison is the point in tests: the systems snap a value to its target, and a snap
// either happened or it did not. Epsilon comparisons there would stop testing what is being tested.
#![cfg_attr(test, allow(clippy::float_cmp))]

pub use bevy_ecs::prelude::{Component, Entity, Query, World};

pub mod api;
pub mod components;
pub mod core;
pub mod export;
pub mod import;
pub mod map;
pub mod orders;
pub mod simulation;
pub mod systems;

mod component_registry;
mod entity_components;

pub use api::Api;
pub use core::Core;
pub use entity_components::EntityComponents;
pub use export::ExportError;
pub use import::ImportError;
pub use map::{MapBounds, MapError};
pub use orders::{EntityId, OrderReport};
pub use simulation::TickError;

/// Returns the canonical hello-world greeting.
#[must_use]
pub const fn hello() -> &'static str {
    "Hello, world!"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_returns_greeting() {
        assert_eq!(hello(), "Hello, world!");
    }
}
