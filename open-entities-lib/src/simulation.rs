//! Tick-time constants and resources for [`Api::step`](crate::Api::step).

#![deny(clippy::float_arithmetic)]

use std::collections::HashSet;

use bevy_ecs::prelude::{Entity, Resource};

pub use crate::systems::ARRIVAL_RADIUS;

/// Length of one simulation tick in milliseconds (20 Hz).
pub const TICK_MS: u32 = 50;

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
                Position { x: 19_950, y: 0 },
                MoveTarget { x: 20_000, y: 0 },
                BaseMoveSpeed(100),
                Velocity { vx: 5000, vy: 0 },
            ))
            .id();

        api.step();

        let world = api.core_mut().world();
        let position = world.get::<Position>(entity).expect("position");
        assert_eq!(*position, Position { x: 20_000, y: 0 });
        assert!(world.get::<MoveTarget>(entity).is_none());
    }

    #[test]
    fn fast_unit_arrives_without_oscillation() {
        let mut api = Api::new();
        let entity = api
            .core_mut()
            .world_mut()
            .spawn((
                Position { x: 0, y: 0 },
                MoveTarget { x: 20_000, y: 0 },
                BaseMoveSpeed(2250),
                Velocity { vx: 0, vy: 0 },
            ))
            .id();

        // Regression for overshoot: 45 units/s steps 2250 milli-units per tick, far above ARRIVAL_RADIUS,
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
        assert_eq!(*position, Position { x: 20_000, y: 0 });
        let velocity = world.get::<Velocity>(entity).expect("velocity");
        assert_eq!(*velocity, Velocity { vx: 0, vy: 0 });
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
        assert_eq!(*position, Position { x: 20_000, y: 0 });
        assert!(world.get::<MoveTarget>(entity).is_none());
    }
}
