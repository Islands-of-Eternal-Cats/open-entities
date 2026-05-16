//! Spawns entities and prints a JSON snapshot of the ECS world.

use open_entities::{Api, components::Position};

fn main() {
    let mut api = Api::new();

    api.core_mut().world_mut().spawn(Position { x: 0.0, y: 0.0 });
    api.core_mut().world_mut().spawn(Position { x: 10.5, y: -3.25 });

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
