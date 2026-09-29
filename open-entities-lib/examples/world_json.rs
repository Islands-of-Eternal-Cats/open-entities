//! Spawns sample RTS entities and prints a pretty-printed world snapshot as JSON.

use open_entities::{
    Api,
    components::{Faction, MoveTarget, Position, Velocity},
};

fn main() {
    let mut api = Api::new();
    let world = api.core_mut().world_mut();

    // Components hold milli-units; velocities are milli-units per tick. The JSON shows map units.
    world.spawn((
        Position::from_units(10.0, 5.0),
        Velocity { vx: 50, vy: 0 }, // 1 unit/s
        Faction(1),
        MoveTarget::from_units(20.0, 0.0),
    ));
    world.spawn(Faction(2));
    world.spawn((Position { x: 0, y: 0 }, Velocity { vx: 13, vy: -25 })); // 0.26, -0.5 units/s

    let pretty = serde_json::to_string_pretty(&api.world_snapshot())
        .expect("world snapshot serializes to JSON");
    println!("{pretty}");
}
