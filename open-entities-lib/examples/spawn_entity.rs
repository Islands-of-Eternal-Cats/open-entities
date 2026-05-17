//! Loads RTS entity templates from YAML (with template inheritance), spawns by name
//! with optional component overrides, and prints world JSON.
//!
//! Inheritance is resolved at load time:
//! - `template: unit` — single parent
//! - `template: [unit, tank]` — multiple parents (later entries win on conflict)

use open_entities::components::Position;
use open_entities::{Api, EntityComponents};

const TEMPLATES_YAML: &str = r"
entities:
  unit:
    faction: 1

  scout:
    template: unit
    position: { x: 10.0, y: 5.0 }
    velocity: { vx: 2.0, vy: 0.0 }
    move_target: { x: 20.0, y: 0.0 }

  tank:
    template: unit
    faction: 2
    velocity: { vx: 0.5, vy: 0.0 }

  heavy_tank:
    template: [unit, tank]
    faction: 3
    position: { x: 0.0, y: 0.0 }

  marker: {}
";

fn main() {
    let mut api = Api::new();

    if let Err(err) = api.load_templates_yaml(TEMPLATES_YAML) {
        eprintln!("failed to load templates: {err}");
        return;
    }

    for name in ["marker", "heavy_tank", "tank", "scout", "unit"] {
        let overrides = if name == "scout" {
            EntityComponents {
                position: Some(Position { x: 50.0, y: 25.0 }),
                ..Default::default()
            }
        } else {
            EntityComponents::default()
        };
        match api.spawn_entity(name, overrides) {
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
