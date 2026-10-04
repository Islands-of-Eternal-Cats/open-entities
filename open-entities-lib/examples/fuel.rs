//! A game-side component and system, added through the public API and `open_entities::extend`
//! only — no fork of the crate.
//!
//! `fuel: 100` in a template gives a unit a tank. `burn_fuel` runs in [`SimSet::PostMovement`],
//! after the unit has moved this tick: it burns one unit of fuel per tick while the unit is moving
//! and stops the unit when the tank runs dry. `fuel` is part of the state hash and shows up in the
//! world export like any built-in component.
//!
//! A dry unit given a new order moves for one tick before `burn_fuel` stops it again; a game that
//! minds would also refuse the order in [`SimSet::Steering`].
//!
//! Run: `cargo run -p open_entities --example fuel`

use bevy_ecs::prelude::{Commands, Component, Entity, Query};
use open_entities::components::{BoardingTarget, MoveTarget, OrderSource, Velocity};
use open_entities::extend::{self, RegisterError, SimSet};
use open_entities::{Api, impl_state_hash_via_serde};
use serde::{Deserialize, Serialize};

/// Fuel left in the tank. In YAML a template writes `fuel: 100`.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Fuel(pub u32);

// Tags below 64 belong to the engine.
impl_state_hash_via_serde!(Fuel, 64);

/// Burns one unit of fuel per tick for every unit that is moving, and stops a unit whose tank is
/// empty the way `Api::order_stop` does: no velocity, no target, no order claim.
pub fn burn_fuel(mut commands: Commands, mut units: Query<(Entity, &mut Fuel, &mut Velocity)>) {
    for (entity, mut fuel, mut velocity) in &mut units {
        if velocity.vx == 0 && velocity.vy == 0 {
            continue;
        }
        fuel.0 = fuel.0.saturating_sub(1);
        if fuel.0 == 0 {
            velocity.vx = 0;
            velocity.vy = 0;
            commands
                .entity(entity)
                .remove::<(MoveTarget, OrderSource, BoardingTarget)>();
        }
    }
}

/// Registers [`Fuel`] as the `fuel` field and adds [`burn_fuel`]. Call it on a fresh [`Api`],
/// before loading templates.
///
/// # Errors
///
/// When `fuel` is already registered, or templates are loaded or the match has started.
pub fn install(api: &mut Api) -> Result<(), RegisterError> {
    api.register_component::<Fuel>("fuel")?;
    extend::add_systems(api, SimSet::PostMovement, burn_fuel)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut api = Api::new();
    install(&mut api)?;
    api.load_templates_yaml(
        "entities:
  tanker:
    position: { x: 0, y: 0 }
    velocity: { vx: 0, vy: 0 }
    base_move_speed: 2.0
    fuel: 100
",
    )?;

    let tanker = api.spawn_entity("tanker", open_entities::EntityComponents::default())?;
    api.submit(open_entities::Command::MoveTo {
        ids: vec![tanker],
        target: MoveTarget::from_units(1000.0, 0.0),
    });

    for _ in 0..150 {
        api.step();
    }

    // 100 ticks of fuel at 0.1 map units per tick: the tanker stops at x = 10.
    let snapshot = api.world_snapshot();
    println!("{}", serde_json::to_string_pretty(&snapshot)?);
    println!(
        "state hash at tick {}: {:016x}",
        api.current_tick(),
        api.state_hash()
    );
    Ok(())
}
