//! Tick-time constants and resources for [`Api::step`](crate::Api::step).

use std::collections::HashSet;

use bevy_ecs::prelude::{Entity, Resource};

pub use crate::systems::ARRIVAL_THRESHOLD;

/// Length of one simulation tick in milliseconds (20 Hz).
pub const TICK_MS: u32 = 50;

/// Length of one simulation tick in seconds, for per-second speeds.
#[allow(clippy::cast_precision_loss)] // TICK_MS is small; exact conversion
pub const TICK_SECS: f32 = TICK_MS as f32 / 1000.0;

/// Number of ticks the simulation has advanced; incremented once per [`Api::step`](crate::Api::step).
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SimTick(pub u64);

/// Entities that arrived this tick; `movement_system` skips them.
#[derive(Resource, Debug, Default)]
pub struct ArrivedThisTick(pub HashSet<Entity>);

#[cfg(test)]
mod tests {
    use crate::api::Api;
    use crate::components::{BaseMoveSpeed, MoveTarget, Position, Velocity};
    use crate::entity_components::EntityComponents;

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

        api.step();

        let world = api.core_mut().world();
        let position = world.get::<Position>(entity).expect("position");
        assert!((position.x - 20.0).abs() < 1e-4);
        assert!((position.y - 0.0).abs() < 1e-4);
        assert!(world.get::<MoveTarget>(entity).is_none());
    }

    #[test]
    fn fast_unit_arrives_without_oscillation() {
        let mut api = Api::new();
        let entity = api
            .core_mut()
            .world_mut()
            .spawn((
                Position { x: 0.0, y: 0.0 },
                MoveTarget { x: 20.0, y: 0.0 },
                BaseMoveSpeed(45.0),
                Velocity { vx: 0.0, vy: 0.0 },
            ))
            .id();

        // Regression for overshoot: 45 units/s steps 2.25 per tick, far above ARRIVAL_THRESHOLD,
        // so without the step check the unit would jump past the target and oscillate.
        // 8 full steps cover 18 units; the 9th reaches the target.
        let mut ticks = 0;
        for _ in 0..100 {
            api.step();
            ticks += 1;
            if api.core_mut().world().get::<MoveTarget>(entity).is_none() {
                break;
            }
        }

        assert_eq!(ticks, 9, "expected arrival in a few ticks, took {ticks}");

        let world = api.core_mut().world();
        let position = world.get::<Position>(entity).expect("position");
        assert!((position.x - 20.0).abs() < 1e-4);
        assert!((position.y - 0.0).abs() < 1e-4);
        let velocity = world.get::<Velocity>(entity).expect("velocity");
        assert_eq!(velocity.vx, 0.0);
        assert_eq!(velocity.vy, 0.0);
    }

    const FIXTURE_YAML: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/spawn_entity_templates.yaml"
    ));

    #[test]
    fn scout_reaches_move_target() {
        let mut api = Api::new();
        api.load_templates_yaml(FIXTURE_YAML).expect("load fixture");
        let entity = api
            .spawn_entity("scout", EntityComponents::default())
            .expect("spawn scout")
            .to_entity()
            .expect("live entity");

        for _ in 0..1000 {
            api.step();
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
