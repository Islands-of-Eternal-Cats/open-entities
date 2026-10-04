//! Roadmap step 5: the extension API. A game registers its own components and systems through
//! `Api::register_component` and `open_entities::extend`, without editing the crate.
//!
//! The fuel tests drive the same component and system as `examples/fuel.rs`.

#[path = "../examples/fuel.rs"]
#[allow(dead_code)] // the example's `main`
mod fuel;

use std::sync::{Arc, Mutex};

use bevy_ecs::prelude::{Component, Query, Res, Resource};
use bevy_ecs::schedule::LogLevel;
use open_entities::components::{MoveTarget, Position, Velocity};
use open_entities::extend::{self, RegisterError, SimSet};
use open_entities::{
    Api, Command, CommandError, ComponentError, EntityComponents, EntityId, ImportError, MapError,
    impl_state_hash_via_serde,
};
use serde::{Deserialize, Serialize};

use fuel::{Fuel, burn_fuel, install};

/// 1 map unit per second is 50 milli-units per tick.
const TANKER: &str = "entities:
  tanker:
    position: { x: 0, y: 0 }
    velocity: { vx: 0, vy: 0 }
    base_move_speed: 1.0
    fuel: 100
  rock:
    position: { x: 5, y: 5 }
";

/// [`TANKER`] without the `fuel` key, for an `Api` that has not registered it.
fn templates_without_fuel() -> String {
    TANKER.replace("    fuel: 100\n", "")
}

fn tanker_api() -> (Api, EntityId) {
    let mut api = Api::new();
    install(&mut api).expect("fuel installs");
    api.load_templates_yaml(TANKER).expect("templates load");
    let tanker = api
        .spawn_entity("tanker", EntityComponents::default())
        .expect("tanker spawns");
    (api, tanker)
}

fn fuel_of(api: &Api, id: EntityId) -> Option<Fuel> {
    api.core()
        .world()
        .get::<Fuel>(id.to_entity().expect("live id"))
        .copied()
}

fn position_of(api: &Api, id: EntityId) -> Position {
    *api.core()
        .world()
        .get::<Position>(id.to_entity().expect("live id"))
        .expect("position")
}

#[test]
fn fuel_from_yaml_burns_one_unit_per_tick_while_moving() {
    let (mut api, tanker) = tanker_api();
    assert_eq!(fuel_of(&api, tanker), Some(Fuel(100)));

    // Not moving yet: nothing burns.
    api.step();
    assert_eq!(fuel_of(&api, tanker), Some(Fuel(100)));

    api.submit(Command::MoveTo {
        ids: vec![tanker],
        target: MoveTarget::from_units(1000.0, 0.0),
    });
    for _ in 0..10 {
        api.step();
    }
    assert_eq!(fuel_of(&api, tanker), Some(Fuel(90)));
    assert_eq!(position_of(&api, tanker), Position { x: 500, y: 0 });
}

#[test]
fn a_unit_stops_when_its_fuel_runs_out() {
    let (mut api, tanker) = tanker_api();
    api.submit(Command::MoveTo {
        ids: vec![tanker],
        target: MoveTarget::from_units(1000.0, 0.0),
    });

    // The order applies on tick 1 and the tanker moves on every tick from then on: 100 ticks of
    // fuel, 50 milli-units per tick.
    for _ in 0..100 {
        api.step();
    }
    assert_eq!(fuel_of(&api, tanker), Some(Fuel(0)));
    assert_eq!(position_of(&api, tanker), Position { x: 5000, y: 0 });

    for _ in 0..20 {
        api.step();
    }
    assert_eq!(position_of(&api, tanker), Position { x: 5000, y: 0 });
    let world = api.core().world();
    let entity = tanker.to_entity().expect("live id");
    assert_eq!(
        world.get::<Velocity>(entity),
        Some(&Velocity { vx: 0, vy: 0 })
    );
    assert!(world.get::<MoveTarget>(entity).is_none());
}

#[test]
fn units_without_fuel_are_left_alone() {
    let (mut api, _) = tanker_api();
    let rock = api
        .spawn_entity("rock", EntityComponents::default())
        .expect("rock spawns");
    api.step();
    assert_eq!(fuel_of(&api, rock), None);
}

#[test]
fn fuel_appears_in_the_json_export() {
    let (mut api, tanker) = tanker_api();
    api.submit(Command::MoveTo {
        ids: vec![tanker],
        target: MoveTarget::from_units(1000.0, 0.0),
    });
    api.step();
    api.step();

    let json = serde_json::to_value(api.world_snapshot()).expect("snapshot serializes");
    let row = json["entities"]
        .as_array()
        .expect("entities")
        .iter()
        .find(|row| row["entity_type"] == "tanker")
        .expect("tanker row");
    assert_eq!(row["fuel"], 98);
    assert_eq!(row["position"]["x"], 0.1);
}

#[test]
fn fuel_is_part_of_the_state_hash() {
    let hash_with = |fuel: u32| {
        let mut api = Api::new();
        install(&mut api).expect("fuel installs");
        api.load_templates_yaml(TANKER).expect("templates load");
        let mut overrides = EntityComponents::default();
        overrides
            .extra
            .insert("fuel".to_owned(), yaml_serde::Value::from(fuel));
        api.spawn_entity("tanker", overrides).expect("spawns");
        api.state_hash()
    };
    assert_eq!(hash_with(100), hash_with(100));
    assert_ne!(hash_with(100), hash_with(99));

    // Carrying the component at all is state: the same rock with and without a tank.
    let without_fuel = {
        let mut api = Api::new();
        install(&mut api).expect("fuel installs");
        api.load_templates_yaml(TANKER).expect("templates load");
        api.spawn_entity("rock", EntityComponents::default())
            .expect("spawns");
        api.state_hash()
    };
    let with_fuel = {
        let mut api = Api::new();
        install(&mut api).expect("fuel installs");
        api.load_templates_yaml(TANKER).expect("templates load");
        let mut overrides = EntityComponents::default();
        overrides
            .extra
            .insert("fuel".to_owned(), yaml_serde::Value::from(5_u32));
        api.spawn_entity("rock", overrides).expect("spawns");
        api.state_hash()
    };
    assert_ne!(without_fuel, with_fuel);
}

#[test]
fn custom_overrides_travel_in_commands() {
    let (mut api, _) = tanker_api();
    let command: Command = serde_json::from_str(
        r#"{"type": "spawn", "template": "tanker", "overrides": {"fuel": 7}}"#,
    )
    .expect("command parses");
    api.submit(command);
    let report = api.step();
    let Ok(open_entities::CommandResult::Spawned(id)) = &report.outcomes[0].result else {
        panic!("spawn applied: {report:?}");
    };
    assert_eq!(fuel_of(&api, *id), Some(Fuel(7)));
}

// --- Registration rules ---------------------------------------------------------------------

/// A component that keeps a float: simulation state must be integers.
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
struct Heat {
    celsius: f32,
}
impl_state_hash_via_serde!(Heat, 70);

/// The float hides inside an option, inside a list, inside an enum's second variant.
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
enum Engine {
    Off,
    Running { readings: Vec<Option<f64>> },
}
impl_state_hash_via_serde!(Engine, 71);

#[test]
fn registering_a_component_with_a_float_field_fails() {
    let mut api = Api::new();
    let err = api.register_component::<Heat>("heat").unwrap_err();
    let RegisterError::NotHashable { field, reason } = &err else {
        panic!("expected NotHashable, got {err:?}");
    };
    assert_eq!(field, "heat");
    assert!(reason.contains("celsius"), "{reason}");
    assert!(reason.contains("f32"), "{reason}");

    let err = api.register_component::<Engine>("engine").unwrap_err();
    assert!(
        matches!(&err, RegisterError::NotHashable { reason, .. } if reason.contains("f64")),
        "{err:?}"
    );

    // Nothing was registered: the fields are still free.
    assert!(!api.component_fields().contains(&"heat"));
    install(&mut api).expect("fuel still installs");
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
struct Armor(u32);
impl_state_hash_via_serde!(Armor, 64);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
struct Engine2(u8);
impl_state_hash_via_serde!(Engine2, 3);

#[test]
fn registration_errors() {
    let mut api = Api::new();
    install(&mut api).expect("fuel installs");

    assert_eq!(
        api.register_component::<Fuel>("fuel"),
        Err(RegisterError::DuplicateField("fuel".to_owned()))
    );
    assert_eq!(
        api.register_component::<Armor>("position"),
        Err(RegisterError::DuplicateField("position".to_owned()))
    );
    for reserved in ["template", "id", "entity_type", ""] {
        assert_eq!(
            api.register_component::<Armor>(reserved),
            Err(RegisterError::ReservedField(reserved.to_owned()))
        );
    }
    assert_eq!(
        api.register_component::<Armor>("armor"),
        Err(RegisterError::DuplicateTag {
            tag: 64,
            field: "armor".to_owned(),
            existing: "fuel".to_owned(),
        })
    );
    assert_eq!(
        api.register_component::<Engine2>("engine"),
        Err(RegisterError::ReservedTag(3))
    );
}

#[test]
fn registration_must_come_before_templates() {
    let mut api = Api::new();
    api.load_templates_yaml(&templates_without_fuel())
        .expect("templates load");
    assert_eq!(
        api.register_component::<Fuel>("fuel"),
        Err(RegisterError::TemplatesLoaded)
    );
}

#[test]
fn registration_after_the_first_step_is_an_error() {
    let mut api = Api::new();
    api.step();
    assert_eq!(
        api.register_component::<Fuel>("fuel"),
        Err(RegisterError::AfterFirstStep { tick: 1 })
    );
    assert_eq!(
        extend::add_systems(&mut api, SimSet::PostMovement, burn_fuel),
        Err(RegisterError::AfterFirstStep { tick: 1 })
    );
}

// --- YAML resolution ------------------------------------------------------------------------

#[test]
fn an_unknown_yaml_key_is_an_error_that_lists_the_known_fields() {
    let mut api = Api::new();
    install(&mut api).expect("fuel installs");
    let err = api
        .load_templates_yaml("entities:\n  tanker:\n    fual: 100\n")
        .unwrap_err();
    let ImportError::Component { template, error } = &err else {
        panic!("expected a component error, got {err:?}");
    };
    assert_eq!(template, "tanker");
    let ComponentError::Unknown { field, known } = error else {
        panic!("expected an unknown field, got {error:?}");
    };
    assert_eq!(field, "fual");
    assert_eq!(
        known,
        &[
            "position",
            "velocity",
            "faction",
            "move_target",
            "base_move_speed",
            "health",
            "boardable",
            "fuel",
        ]
    );
    let message = err.to_string();
    assert!(
        message.contains("fual") && message.contains("fuel"),
        "{message}"
    );
}

#[test]
fn a_custom_field_of_the_wrong_shape_is_an_error() {
    let mut api = Api::new();
    install(&mut api).expect("fuel installs");
    let err = api
        .load_templates_yaml("entities:\n  tanker:\n    fuel: -1\n")
        .unwrap_err();
    assert!(
        matches!(
            &err,
            ImportError::Component { error: ComponentError::Invalid { field, .. }, .. }
            if field == "fuel"
        ),
        "{err:?}"
    );
}

#[test]
fn custom_fields_inherit_and_override_like_built_ins() {
    let mut api = Api::new();
    install(&mut api).expect("fuel installs");
    api.load_templates_yaml(
        "entities:
  base:
    fuel: 100
  scout:
    template: base
    position: { x: 0, y: 0 }
  light:
    template: base
    fuel: 20
",
    )
    .expect("templates load");
    let scout = api
        .spawn_entity("scout", EntityComponents::default())
        .expect("scout");
    let light = api
        .spawn_entity("light", EntityComponents::default())
        .expect("light");
    assert_eq!(fuel_of(&api, scout), Some(Fuel(100)));
    assert_eq!(fuel_of(&api, light), Some(Fuel(20)));
}

#[test]
fn spawn_overrides_and_map_entries_are_resolved_too() {
    let (mut api, _) = tanker_api();

    let mut overrides = EntityComponents::default();
    overrides
        .extra
        .insert("fual".to_owned(), yaml_serde::Value::from(1_u32));
    let err = api.spawn_entity("tanker", overrides.clone()).unwrap_err();
    assert!(
        matches!(
            &err,
            ImportError::Component {
                error: ComponentError::Unknown { .. },
                ..
            }
        ),
        "{err:?}"
    );

    api.submit(Command::Spawn {
        template: "tanker".to_owned(),
        overrides,
    });
    let report = api.step();
    assert!(
        matches!(
            &report.outcomes[0].result,
            Err(CommandError::Component(ComponentError::Unknown { .. }))
        ),
        "{report:?}"
    );

    let before = api.world_snapshot().entities.len();
    let err = api
        .load_map_yaml(
            "spawns:
  - template: tanker
    fuel: 5
  - template: tanker
    fual: 5
",
        )
        .unwrap_err();
    assert!(
        matches!(&err, MapError::Component { index: 1, .. }),
        "{err:?}"
    );
    assert_eq!(
        api.world_snapshot().entities.len(),
        before,
        "a bad entry leaves the world untouched"
    );
}

// --- Systems --------------------------------------------------------------------------------

#[test]
fn ambiguity_detection_passes_with_user_systems_added() {
    // Each of these writes what a built-in in the same set touches; without an order between
    // them the schedule would not build in a debug build.
    // Ambiguity is judged on declared access, so the bodies can stay empty.
    fn nudge_velocity(_: Query<&mut Velocity>) {}
    fn nudge_position(_: Query<&mut Position>) {}
    fn nudge_target(_: Query<&mut MoveTarget>) {}

    let (mut api, _) = {
        let mut api = Api::new();
        install(&mut api).expect("fuel installs");
        for set in [
            SimSet::Commands,
            SimSet::Steering,
            SimSet::Movement,
            SimSet::PostMovement,
            SimSet::Resolve,
        ] {
            extend::add_systems(
                &mut api,
                set,
                (nudge_velocity, nudge_position, nudge_target),
            )
            .expect("user systems build without ambiguities");
        }
        api.load_templates_yaml(TANKER).expect("templates load");
        let tanker = api
            .spawn_entity("tanker", EntityComponents::default())
            .expect("spawns");
        (api, tanker)
    };
    assert_eq!(
        api.core()
            .schedule()
            .get_build_settings()
            .ambiguity_detection,
        if cfg!(debug_assertions) {
            LogLevel::Error
        } else {
            LogLevel::Ignore
        }
    );
    api.step();
}

#[derive(Resource, Clone, Default)]
struct Log(Arc<Mutex<Vec<String>>>);

impl Log {
    fn push(&self, line: String) {
        self.0.lock().expect("log lock").push(line);
    }
}

#[test]
fn user_systems_run_after_the_built_ins_of_their_set_in_registration_order() {
    fn after_seek(log: Res<Log>, units: Query<&Velocity>) {
        let moving = units.iter().any(|v| v.vx != 0);
        log.push(format!("steering: moving={moving}"));
    }
    fn after_movement(log: Res<Log>, units: Query<&Position>) {
        let x = units.iter().map(|p| p.x).max().unwrap_or_default();
        log.push(format!("movement: x={x}"));
    }
    fn first(log: Res<Log>) {
        log.push("post: first".to_owned());
    }
    fn second(log: Res<Log>) {
        log.push("post: second".to_owned());
    }
    fn third(log: Res<Log>) {
        log.push("post: third".to_owned());
    }

    let log = Log::default();
    let mut api = Api::new();
    api.core_mut().world_mut().insert_resource(log.clone());
    // Registered out of set order; within PostMovement, `first` before `(second, third)`.
    extend::add_systems(&mut api, SimSet::PostMovement, first).expect("adds");
    extend::add_systems(&mut api, SimSet::Movement, after_movement).expect("adds");
    extend::add_systems(&mut api, SimSet::PostMovement, (second, third)).expect("adds");
    extend::add_systems(&mut api, SimSet::Steering, after_seek).expect("adds");

    api.load_templates_yaml(&templates_without_fuel())
        .expect("templates load");
    let tanker = api
        .spawn_entity("tanker", EntityComponents::default())
        .expect("spawns");
    api.order_move_to(&[tanker], MoveTarget::from_units(10.0, 0.0));
    api.step();

    assert_eq!(
        *log.0.lock().expect("log lock"),
        [
            "steering: moving=true", // seek ran before it
            "movement: x=50",        // movement ran before it
            "post: first",
            "post: second",
            "post: third",
        ]
    );
}

#[test]
fn user_systems_do_not_change_entity_ids() {
    let id_after_setup = |extend_first: bool| {
        let mut api = Api::new();
        if extend_first {
            install(&mut api).expect("fuel installs");
        }
        api.load_templates_yaml(&templates_without_fuel())
            .expect("templates load");
        api.spawn_entity("rock", EntityComponents::default())
            .expect("spawns")
    };
    assert_eq!(id_after_setup(false), id_after_setup(true));
}
