#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

pub use bevy_ecs::prelude::{Component, Entity, Query, World};

pub mod api;
pub mod components;
pub mod core;
pub mod export;
pub mod import;

pub use api::Api;
pub use core::Core;
pub use export::ExportError;
pub use import::ImportError;

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
