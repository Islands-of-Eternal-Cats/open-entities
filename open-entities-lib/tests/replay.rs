//! Roadmap step 3: a [`Replay`] is the content plus the command log, and running it again gives
//! the same state, hash for hash.

use open_entities::components::MoveTarget;
use open_entities::{Api, Command, EntityId, Replay, ReplayCommand, ReplayError};

const TEMPLATES_YAML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/spawn_entity_templates.yaml"
));
const MAP_YAML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/init_map.yaml"
));

/// The ids the map spawns, in file order: marker, scout, unit.
fn map_ids() -> Vec<EntityId> {
    let mut api = Api::new();
    api.load_templates_yaml(TEMPLATES_YAML).expect("templates");
    api.load_map_yaml(MAP_YAML).expect("map")
}

fn replay(commands: Vec<ReplayCommand>) -> Replay {
    Replay {
        version: 1,
        templates_yaml: TEMPLATES_YAML.to_owned(),
        map_yaml: MAP_YAML.to_owned(),
        seed: 7,
        commands,
    }
}

fn move_scout(tick: u64, x: f64) -> ReplayCommand {
    let scout = map_ids()[1];
    ReplayCommand {
        tick,
        command: Command::MoveTo {
            ids: vec![scout],
            target: MoveTarget::from_units(x, 0.0),
        },
    }
}

#[test]
fn a_replay_rebuilds_the_run_it_was_recorded_from() {
    // The live run: a host submitting as it goes.
    let mut live = Api::new();
    live.load_templates_yaml(TEMPLATES_YAML).expect("templates");
    live.load_map_yaml(MAP_YAML).expect("map");
    let scout = map_ids()[1];
    for _ in 0..10 {
        live.step();
    }
    live.submit(Command::MoveTo {
        ids: vec![scout],
        target: MoveTarget::from_units(60.0, 30.0),
    });
    for _ in 0..40 {
        live.step();
    }

    // The same thing as data: submitted at tick 10, so it targets tick 11.
    let recorded = replay(vec![ReplayCommand {
        tick: 11,
        command: Command::MoveTo {
            ids: vec![scout],
            target: MoveTarget::from_units(60.0, 30.0),
        },
    }]);
    let mut replayed = recorded.run(50).expect("replay runs");

    assert_eq!(replayed.current_tick(), 50);
    assert_eq!(replayed.state_hash(), live.state_hash());
    assert_eq!(replayed.world_snapshot(), live.world_snapshot());
}

#[test]
fn a_replay_survives_json() {
    let original = replay(vec![move_scout(3, 40.0), move_scout(20, -5.5)]);

    let json = original.to_json();
    let parsed = Replay::from_json(&json).expect("parse");

    assert_eq!(parsed, original);
    assert_eq!(
        parsed.run(60).expect("run").state_hash(),
        original.run(60).expect("run").state_hash()
    );
}

#[test]
fn running_twice_gives_the_same_hash() {
    let log = replay(vec![move_scout(1, 80.0), move_scout(30, 0.0)]);
    let first = log.run(100).expect("run").state_hash();
    let second = log.run(100).expect("run").state_hash();
    assert_eq!(first, second);
}

#[test]
fn commands_past_the_end_are_not_applied() {
    let log = replay(vec![move_scout(50, 80.0)]);
    let short = log.run(49).expect("run");
    let without = replay(vec![]).run(49).expect("run");
    assert_eq!(short.state_hash(), without.state_hash());
}

#[test]
fn an_unknown_version_is_refused() {
    let mut log = replay(vec![]);
    log.version = 2;
    assert!(matches!(
        log.run(1),
        Err(ReplayError::UnsupportedVersion(2))
    ));
}

#[test]
fn a_command_at_tick_zero_is_refused() {
    // Tick 0 is the state before any step; nothing can be applied "at the start of" it.
    let log = replay(vec![move_scout(0, 1.0)]);
    assert!(matches!(
        log.run(1),
        Err(ReplayError::Schedule { index: 0, .. })
    ));
}

#[test]
fn broken_content_is_reported() {
    let mut log = replay(vec![]);
    log.templates_yaml = "entities: [".to_owned();
    assert!(matches!(log.run(1), Err(ReplayError::Templates(_))));

    let mut log = replay(vec![]);
    log.map_yaml = "spawns: [{ template: nope }]".to_owned();
    assert!(matches!(log.run(1), Err(ReplayError::Map(_))));
}

#[test]
fn a_replay_without_a_map_starts_empty() {
    let mut log = replay(vec![]);
    log.map_yaml = String::new();
    let mut api = log.run(5).expect("run");
    assert!(api.world_snapshot().entities.is_empty());
}

#[test]
fn malformed_json_is_reported() {
    assert!(matches!(
        Replay::from_json("{ \"version\": 1 }"),
        Err(ReplayError::Json(_))
    ));
}
