//! Spawns sample RTS entities and prints a pretty-printed world snapshot as JSON.

use open_entities::{
    Api,
    components::{Faction, MoveTarget, Position, Velocity},
};

fn main() {
    let mut api = Api::new();
    let world = api.core_mut().world_mut();

    world.spawn((
        Position { x: 10.0, y: 5.0 },
        Velocity { vx: 1.0, vy: 0.0 },
        Faction(1),
        MoveTarget { x: 20.0, y: 0.0 },
    ));
    world.spawn(Faction(2));
    world.spawn((Position { x: 0.0, y: 0.0 }, Velocity { vx: 0.25, vy: -0.5 }));

    let pretty = serde_json::to_string_pretty(&api.world_snapshot())
        .expect("world snapshot serializes to JSON");
    println!("{pretty}");
}
