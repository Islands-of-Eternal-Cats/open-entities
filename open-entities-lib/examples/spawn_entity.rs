//! Loads RTS entity templates from YAML (with template inheritance), spawns by name
//! with optional component overrides, and prints world JSON.
//!
//! Inheritance is resolved at load time:
//! - `template: unit` — single parent
//! - `template: [unit, tank]` — multiple parents (later entries win on conflict)

use open_entities::components::{Health, Position};
use open_entities::{Api, EntityComponents};

const TEMPLATES_YAML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/spawn_entity_templates.yaml"
));

fn main() {
    let mut api = Api::new();

    if let Err(err) = api.load_templates_yaml(TEMPLATES_YAML) {
        eprintln!("failed to load templates: {err}");
        return;
    }

    for name in ["marker", "heavy_tank", "tank", "scout", "unit"] {
        let overrides = if name == "scout" {
            EntityComponents {
                position: Some(Position::from_units(50.0, 25.0)),
                health: Some(Health {
                    current: 40,
                    max: 100,
                }),
                ..Default::default()
            }
        } else {
            EntityComponents::default()
        };
        match api.spawn_entity(name, overrides) {
            Ok(id) => println!("spawned {name} -> {id:?}"),
            Err(err) => eprintln!("spawn {name} failed: {err}"),
        }
    }

    let pretty = serde_json::to_string_pretty(&api.world_snapshot())
        .expect("world snapshot serializes to JSON");
    println!("\n{pretty}");
}
