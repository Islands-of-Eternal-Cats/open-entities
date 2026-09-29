//! Roadmap step 2: simulation state is integer milli-units; the outside world keeps decimals.

use open_entities::{Api, EntityComponents};

const FIXTURE_YAML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/spawn_entity_templates.yaml"
));

fn api_with_fixture() -> Api {
    let mut api = Api::new();
    api.load_templates_yaml(FIXTURE_YAML).expect("load fixture");
    api
}

#[test]
fn tank_moves_exactly_25_milli_units_per_tick() {
    let mut api = api_with_fixture();
    let overrides = EntityComponents {
        position: Some(open_entities::components::Position { x: 0, y: 0 }),
        ..EntityComponents::default()
    };
    api.spawn_entity("tank", overrides).expect("spawn tank");

    let snapshot = api.world_snapshot();
    let velocity = snapshot.entities[0].components.velocity.expect("velocity");
    // 0.5 units/s at TICK_MS = 50 is 25 milli-units per tick.
    assert_eq!(velocity.vx, 25);
    assert_eq!(velocity.vy, 0);

    api.step();

    let position = api.world_snapshot().entities[0]
        .components
        .position
        .expect("position");
    assert_eq!(position.x, 25);
    assert_eq!(position.y, 0);
}

#[test]
fn yaml_position_with_three_decimals_round_trips_through_export() {
    let mut api = api_with_fixture();
    api.load_map_yaml(
        "spawns:\n  - template: marker\n    position: { x: 12.345, y: -0.001 }\n  - template: marker\n    position: { x: 7, y: 1999999.999 }\n",
    )
    .expect("load map");

    let json = serde_json::to_value(api.world_snapshot()).expect("serialize");
    let entities = json["entities"].as_array().expect("entities");
    assert_eq!(entities[0]["position"]["x"].to_string(), "12.345");
    assert_eq!(entities[0]["position"]["y"].to_string(), "-0.001");
    assert_eq!(entities[1]["position"]["x"].to_string(), "7.0");
    assert_eq!(entities[1]["position"]["y"].to_string(), "1999999.999");
}

#[test]
fn speed_is_quantised_to_whole_milli_units_per_tick() {
    let mut api = Api::new();
    api.load_templates_yaml(
        "entities:\n  crawler:\n    base_move_speed: 0.51\n    velocity: { vx: 0.51, vy: 0.0 }\n",
    )
    .expect("load templates");
    api.spawn_entity("crawler", EntityComponents::default())
        .expect("spawn crawler");

    let row = &api.world_snapshot().entities[0];
    // 0.51 × 50 = 25.5 → 26 milli-units per tick → 0.52 units/s.
    assert_eq!(row.components.base_move_speed.expect("speed").0, 26);
    assert_eq!(row.components.velocity.expect("velocity").vx, 26);

    let json = serde_json::to_value(api.world_snapshot()).expect("serialize");
    let crawler = &json["entities"][0];
    assert_eq!(crawler["base_move_speed"].to_string(), "0.52");
    assert_eq!(crawler["velocity"]["vx"].to_string(), "0.52");
}

#[test]
fn value_beyond_i32_range_is_an_import_error() {
    let mut api = Api::new();
    let err = api
        .load_templates_yaml("entities:\n  far:\n    position: { x: 3000000.0, y: 0.0 }\n")
        .expect_err("x × 1000 does not fit in i32");
    assert!(err.to_string().contains("out of range"), "{err}");
}
