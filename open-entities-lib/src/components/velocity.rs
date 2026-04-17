use bevy_ecs::prelude::Component;
use serde::Deserialize;

/// Component: Velocity of an entity (also used for YAML deserialization).
#[derive(Component, Clone, Copy, Debug, Deserialize)]
pub struct Velocity {
    pub vx: f32,
    pub vy: f32,
}

impl Velocity {
    /// Constructs zero velocity.
    pub const fn zero() -> Self {
        Self { vx: 0.0, vy: 0.0 }
    }

    /// Returns position delta for the provided `dt` in seconds.
    pub fn delta_for_dt(&self, dt: f32) -> (f32, f32) {
        (self.vx * dt, self.vy * dt)
    }

    /// Returns squared step length for the provided `dt`.
    pub fn step_len_sq_for_dt(&self, dt: f32) -> f32 {
        let (dx, dy) = self.delta_for_dt(dt);
        dx * dx + dy * dy
    }
}
