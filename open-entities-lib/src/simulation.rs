use std::collections::HashSet;

use bevy_ecs::prelude::{Entity, Resource};

pub use crate::systems::ARRIVAL_THRESHOLD;

/// Per-tick delta time in seconds (from clamped `dt_ms`).
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct SimDelta {
    pub dt_secs: f32,
}

impl SimDelta {
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // dt_ms ≤ MAX_DT_MS (100); exact f32 representation
    pub const fn from_ms(ms: u32) -> Self {
        Self {
            dt_secs: ms as f32 / 1000.0,
        }
    }
}

/// Entities that arrived this tick; `movement_system` skips them.
#[derive(Resource, Debug, Default)]
pub struct ArrivedThisTick(pub HashSet<Entity>);

/// Errors from [`Api::tick`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickError {
    ZeroDeltaTime,
}

impl std::fmt::Display for TickError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroDeltaTime => f.write_str("tick delta must be greater than zero"),
        }
    }
}

impl std::error::Error for TickError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::Api;
    use crate::components::{BaseMoveSpeed, MoveTarget, Position, Velocity};
    use crate::entity_components::EntityComponents;

    #[test]
    fn tick_zero_delta_fails() {
        let mut api = Api::new();
        let err = api.tick(0).unwrap_err();
        assert_eq!(err, TickError::ZeroDeltaTime);
    }

    #[test]
    fn tick_clamps_large_dt() {
        let mut api = Api::new();
        let entity = api
            .core_mut()
            .world_mut()
            .spawn((
                Position { x: 0.0, y: 0.0 },
                Velocity { vx: 1.0, vy: 0.0 },
            ))
            .id();

        api.tick(500).expect("tick with clamp");
        let pos_after_500 = api.core_mut().world().get::<Position>(entity).unwrap().x;

        let mut api2 = Api::new();
        let entity2 = api2
            .core_mut()
            .world_mut()
            .spawn((
                Position { x: 0.0, y: 0.0 },
                Velocity { vx: 1.0, vy: 0.0 },
            ))
            .id();
        api2.tick(100).expect("tick at cap");
        let pos_after_100 = api2.core_mut().world().get::<Position>(entity2).unwrap().x;

        assert!((pos_after_500 - pos_after_100).abs() < 1e-5);
    }

    #[test]
    fn movement_skips_arrived_same_frame() {
        let mut api = Api::new();
        let entity = api
            .core_mut()
            .world_mut()
            .spawn((
                Position { x: 19.95, y: 0.0 },
                MoveTarget { x: 20.0, y: 0.0 },
                BaseMoveSpeed(2.0),
                Velocity { vx: 100.0, vy: 0.0 },
            ))
            .id();

        api.tick(16).expect("tick");

        let world = api.core_mut().world();
        let position = world.get::<Position>(entity).expect("position");
        assert!((position.x - 20.0).abs() < 1e-4);
        assert!((position.y - 0.0).abs() < 1e-4);
        assert!(world.get::<MoveTarget>(entity).is_none());
    }

    const FIXTURE_YAML: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/spawn_entity_templates.yaml"
    ));

    #[test]
    fn scout_reaches_move_target() {
        let mut api = Api::new();
        api.load_templates_yaml(FIXTURE_YAML)
            .expect("load fixture");
        let entity = api
            .spawn_entity("scout", EntityComponents::default())
            .expect("spawn scout");

        for _ in 0..1000 {
            api.tick(16).expect("tick");
            if api.core_mut().world().get::<MoveTarget>(entity).is_none() {
                break;
            }
        }

        let world = api.core_mut().world();
        let position = world.get::<Position>(entity).expect("position");
        assert!((position.x - 20.0).abs() < 0.01);
        assert!((position.y - 0.0).abs() < 0.01);
        assert!(world.get::<MoveTarget>(entity).is_none());
    }
}
