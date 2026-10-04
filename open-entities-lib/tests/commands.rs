//! Roadmap step 3: orders are data. A [`Command`] is applied at the start of a known tick, and
//! [`Api::step`] reports what each one did.

use open_entities::components::{MoveTarget, Position};
use open_entities::{
    Api, Command, CommandError, CommandResult, CommandSeq, EntityComponents, EntityId, GroupError,
    ScheduleError, StepReport,
};

const TEMPLATES_YAML: &str = r"
entities:
  walker:
    faction: 1
    position: { x: 0.0, y: 0.0 }
    velocity: { vx: 0.0, vy: 0.0 }
    base_move_speed: 10.0
  truck:
    faction: 1
    position: { x: 0.0, y: 0.0 }
    velocity: { vx: 0.0, vy: 0.0 }
    base_move_speed: 10.0
    boardable: 2
";

fn api() -> Api {
    let mut api = Api::new();
    api.load_templates_yaml(TEMPLATES_YAML)
        .expect("load templates");
    api
}

fn spawn_at(api: &mut Api, template: &str, x: f64) -> EntityId {
    api.spawn_entity(
        template,
        EntityComponents {
            position: Some(Position::from_units(x, 0.0)),
            ..Default::default()
        },
    )
    .expect("spawn")
}

fn position(api: &Api, id: EntityId) -> Position {
    *api.core()
        .world()
        .get::<Position>(id.to_entity().expect("entity"))
        .expect("position")
}

/// The only outcome of a report that carried exactly one command.
fn only_result(report: &StepReport) -> &Result<CommandResult, CommandError> {
    assert_eq!(report.outcomes.len(), 1, "one command, one outcome");
    &report.outcomes[0].result
}

#[test]
fn a_submitted_command_waits_for_the_next_step() {
    let mut api = api();
    let walker = spawn_at(&mut api, "walker", 0.0);

    api.submit(Command::MoveTo {
        ids: vec![walker],
        target: MoveTarget::from_units(100.0, 0.0),
    });

    // Nothing happens on submit: the world is the same until the step that applies it.
    assert!(
        api.core()
            .world()
            .get::<MoveTarget>(walker.to_entity().unwrap())
            .is_none()
    );

    let report = api.step();

    assert_eq!(report.tick, 1);
    assert_eq!(
        only_result(&report),
        &Ok(CommandResult::Applied {
            applied: 1,
            skipped: 0
        })
    );
    // Applied at the start of the tick, so the unit already moved during it: 10 units/s is 500
    // milli-units per tick.
    assert_eq!(position(&api, walker), Position { x: 500, y: 0 });
}

#[test]
fn submit_targets_the_tick_after_the_current_one() {
    let mut api = api();
    api.step();
    api.step();

    let seq = api.submit(Command::CreateGroup { faction: 1 });
    let report = api.step();

    assert_eq!(report.tick, 3);
    assert_eq!(report.outcomes.len(), 1);
    assert_eq!(report.outcomes[0].seq, seq);
}

#[test]
fn a_scheduled_command_applies_on_its_tick_and_not_before() {
    let mut api = api();
    let walker = spawn_at(&mut api, "walker", 0.0);

    api.schedule(
        3,
        Command::MoveTo {
            ids: vec![walker],
            target: MoveTarget::from_units(100.0, 0.0),
        },
    )
    .expect("tick 3 is in the future");

    assert!(api.step().outcomes.is_empty());
    assert!(api.step().outcomes.is_empty());
    assert_eq!(position(&api, walker), Position { x: 0, y: 0 });

    let report = api.step();
    assert_eq!(report.tick, 3);
    assert_eq!(report.outcomes.len(), 1);
    assert_eq!(position(&api, walker), Position { x: 500, y: 0 });
}

#[test]
fn schedule_refuses_a_tick_that_is_not_in_the_future() {
    let mut api = api();
    api.step();
    api.step();

    for tick in [0, 1, 2] {
        assert_eq!(
            api.schedule(tick, Command::CreateGroup { faction: 1 }),
            Err(ScheduleError::NotInFuture { tick, current: 2 }),
        );
    }
    assert!(api.schedule(3, Command::CreateGroup { faction: 1 }).is_ok());
}

#[test]
fn commands_of_one_tick_apply_in_sequence_order() {
    let mut api = api();
    let walker = spawn_at(&mut api, "walker", 0.0);

    // Scheduled out of order on purpose: the later submission targets the same tick first.
    let first = api
        .schedule(
            2,
            Command::MoveTo {
                ids: vec![walker],
                target: MoveTarget::from_units(-100.0, 0.0),
            },
        )
        .expect("schedule");
    let second = api.submit(Command::Stop { ids: vec![walker] });
    let third = api
        .schedule(
            2,
            Command::MoveTo {
                ids: vec![walker],
                target: MoveTarget::from_units(100.0, 0.0),
            },
        )
        .expect("schedule");
    assert!(
        first < second && second < third,
        "seqs grow with every call"
    );

    let tick_one = api.step();
    assert_eq!(tick_one.outcomes.len(), 1);
    assert_eq!(tick_one.outcomes[0].seq, second);

    let tick_two = api.step();
    let seqs: Vec<CommandSeq> = tick_two.outcomes.iter().map(|o| o.seq).collect();
    assert_eq!(seqs, vec![first, third]);
    // The last command of the tick wins: the unit heads for +100.
    assert_eq!(position(&api, walker), Position { x: 500, y: 0 });
}

#[test]
fn creation_commands_report_the_new_ids() {
    let mut api = api();

    api.submit(Command::Spawn {
        template: "walker".to_owned(),
        overrides: EntityComponents {
            position: Some(Position::from_units(3.0, 4.0)),
            ..Default::default()
        },
    });
    api.submit(Command::CreateGroup { faction: 1 });
    api.submit(Command::CreateMission {
        target: MoveTarget::from_units(50.0, 0.0),
        radius: 1000,
    });
    let report = api.step();

    let [spawned, group, mission] = [0, 1, 2].map(|i| report.outcomes[i].result.clone());
    let Ok(CommandResult::Spawned(unit)) = spawned else {
        panic!("expected Spawned, got {spawned:?}");
    };
    let Ok(CommandResult::GroupCreated(group)) = group else {
        panic!("expected GroupCreated, got {group:?}");
    };
    let Ok(CommandResult::MissionCreated(mission)) = mission else {
        panic!("expected MissionCreated, got {mission:?}");
    };

    assert!(api.is_alive(unit));
    assert!(api.group_members(group).is_empty());
    assert!(!api.is_mission_completed(mission));
}

#[test]
fn a_refused_command_reports_why_and_changes_nothing() {
    let mut api = api();
    let walker = spawn_at(&mut api, "walker", 0.0);

    api.submit(Command::GroupMoveTo {
        group: walker,
        target: MoveTarget::from_units(1.0, 1.0),
    });
    api.submit(Command::Spawn {
        template: "ghost".to_owned(),
        overrides: EntityComponents::default(),
    });
    let before = api.world_snapshot().entities;
    let report = api.step();

    assert_eq!(
        report.outcomes[0].result,
        Err(CommandError::Group(GroupError::UnknownGroup(walker)))
    );
    assert_eq!(
        report.outcomes[1].result,
        Err(CommandError::UnknownTemplate("ghost".to_owned()))
    );
    assert_eq!(api.world_snapshot().entities, before);
}

#[test]
fn every_order_has_a_command() {
    let mut api = api();
    let truck = spawn_at(&mut api, "truck", 0.0);
    let rider = spawn_at(&mut api, "walker", 1.0);
    let group = api.create_group(1);
    let mission = api.create_mission(MoveTarget::from_units(500.0, 0.0), 1000);

    let applied = |n| {
        Ok(CommandResult::Applied {
            applied: n,
            skipped: 0,
        })
    };
    let cases: Vec<(Command, Result<CommandResult, CommandError>)> = vec![
        (Command::AddToGroup { group, unit: rider }, applied(1)),
        (
            Command::GroupMoveTo {
                group,
                target: MoveTarget::from_units(20.0, 0.0),
            },
            applied(1),
        ),
        // Halted half a unit further on, still within boarding range of the parked truck.
        (Command::Stop { ids: vec![rider] }, applied(1)),
        (Command::ClearGroupManual { group }, applied(1)),
        (Command::AssignGroup { mission, group }, applied(1)),
        (Command::UnassignGroup { group }, applied(1)),
        (Command::RemoveFromGroup { unit: rider }, applied(1)),
        (
            Command::Board {
                units: vec![rider],
                vehicle: truck,
            },
            applied(1),
        ),
    ];
    for (command, expected) in cases {
        api.submit(command.clone());
        let report = api.step();
        assert_eq!(only_result(&report), &expected, "{command:?}");
    }

    // The rider stood within range, so the boarding order put it in on the tick it was given.
    assert_eq!(api.vehicle_of(rider), Some(truck));
    api.submit(Command::Unboard { units: vec![rider] });
    assert_eq!(only_result(&api.step()), &applied(1));
    assert_eq!(api.vehicle_of(rider), None);

    api.submit(Command::Despawn {
        ids: vec![rider, rider],
    });
    assert_eq!(
        only_result(&api.step()),
        &Ok(CommandResult::Applied {
            applied: 1,
            skipped: 1
        })
    );
    assert!(!api.is_alive(rider));
}

#[test]
fn commands_are_tagged_json() {
    let command: Command = serde_json::from_str(
        r#"{ "type": "move_to", "ids": [{ "index": 4, "generation": 1 }], "target": { "x": 1.5, "y": -2 } }"#,
    )
    .expect("parse");
    assert_eq!(
        command,
        Command::MoveTo {
            ids: vec![EntityId::new(4, 1)],
            target: MoveTarget { x: 1500, y: -2000 },
        }
    );

    // A mission radius is a distance, so on the wire it is in map units like the target.
    let mission = Command::CreateMission {
        target: MoveTarget { x: 1000, y: 0 },
        radius: 2500,
    };
    let json = serde_json::to_value(&mission).expect("serialize");
    assert_eq!(
        json,
        serde_json::json!({ "type": "create_mission", "target": { "x": 1.0, "y": 0.0 }, "radius": 2.5 })
    );
    let back: Command = serde_json::from_value(json).expect("round trip");
    assert_eq!(back, mission);

    // Overrides may be left out: the template is spawned as loaded.
    let spawn: Command =
        serde_json::from_str(r#"{ "type": "spawn", "template": "walker" }"#).expect("parse");
    assert_eq!(
        spawn,
        Command::Spawn {
            template: "walker".to_owned(),
            overrides: EntityComponents::default(),
        }
    );

    assert!(
        serde_json::from_str::<Command>(r#"{ "type": "stop", "ids": [], "extra": 1 }"#).is_err(),
        "unknown fields are refused"
    );
}
