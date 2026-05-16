//! Spawns sample RTS entities and prints a pretty-printed world JSON snapshot (schema v2).

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
    world.spawn((
        Position { x: 0.0, y: 0.0 },
        Velocity { vx: 0.25, vy: -0.5 },
    ));

    match api.world_json() {
        Ok(json) => match serde_json::from_str::<serde_json::Value>(&json) {
            Ok(value) => {
                let pretty = serde_json::to_string_pretty(&value)
                    .expect("pretty-print valid JSON value");
                println!("{pretty}");
            }
            Err(err) => eprintln!("export returned invalid JSON: {err}"),
        },
        Err(err) => eprintln!("export failed: {err}"),
    }
}
