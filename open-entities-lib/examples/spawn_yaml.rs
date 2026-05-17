//! Loads RTS entity templates from YAML, spawns instances by name, and prints world JSON.

use open_entities::Api;

const TEMPLATES_YAML: &str = r"
entities:
  scout:
    position: { x: 10.0, y: 5.0 }
    velocity: { vx: 1.0, vy: 0.0 }
    faction: 1
    move_target: { x: 20.0, y: 0.0 }

  base:
    faction: 2

  marker: {}
";

fn main() {
    let mut api = Api::new();

    if let Err(err) = api.load_templates_yaml(TEMPLATES_YAML) {
        eprintln!("failed to load templates: {err}");
        return;
    }

    for name in ["scout", "scout", "base", "marker"] {
        match api.spawn_yaml(name) {
            Ok(entity) => println!("spawned {name} -> entity {:?}", entity),
            Err(err) => eprintln!("spawn {name} failed: {err}"),
        }
    }

    match api.world_json() {
        Ok(json) => match serde_json::from_str::<serde_json::Value>(&json) {
            Ok(value) => {
                let pretty = serde_json::to_string_pretty(&value)
                    .expect("pretty-print valid JSON value");
                println!("\n{pretty}");
            }
            Err(err) => eprintln!("export returned invalid JSON: {err}"),
        },
        Err(err) => eprintln!("export failed: {err}"),
    }
}
