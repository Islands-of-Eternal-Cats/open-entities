//! Roadmap step 3: the golden replay. One recorded match, one expected state hash, the same on
//! every platform CI runs — native on Linux, Windows and macOS, and wasm32 under Node.
//!
//! If this fails on one platform only, the simulation is not deterministic there. Do not touch
//! the expected hash to make it pass; find the divergence.
//!
//! Regenerate the hash only for an intended change in behaviour, with `UPDATE_GOLDEN=1`, and say
//! in the pull request why it changed.

use open_entities::{CommandResult, Replay};

const REPLAY_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/replays/basic.json"
));
const HASH_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/replays/basic.hash"
);

/// Length of the golden run in ticks: 30 seconds of play.
const GOLDEN_TICKS: u64 = 600;

fn format_hash(hash: u64) -> String {
    format!("{hash:016x}")
}

#[test]
fn golden_replay_hash_matches() {
    let replay = Replay::from_json(REPLAY_JSON).expect("golden replay parses");
    let actual = format_hash(replay.run(GOLDEN_TICKS).expect("replay runs").state_hash());

    if std::env::var_os("UPDATE_GOLDEN").is_some_and(|v| v == "1") {
        std::fs::write(HASH_PATH, format!("{actual}\n")).expect("write golden hash");
        return;
    }

    let expected = std::fs::read_to_string(HASH_PATH).expect("golden hash file");
    assert_eq!(
        actual,
        expected.trim(),
        "the golden replay diverged; see the module docs before touching the expected hash"
    );
}

/// The golden replay is only worth something if it exercises what it claims to: every command
/// in it must apply, and the run must reach each kind of event.
#[test]
fn golden_replay_covers_the_simulation() {
    let replay = Replay::from_json(REPLAY_JSON).expect("golden replay parses");
    let mut api = replay.run(0).expect("replay loads");

    let mut spawned = Vec::new();
    let mut groups = Vec::new();
    let mut missions = Vec::new();
    let mut applied = 0;
    let mut boarded = false;
    for _ in 0..GOLDEN_TICKS {
        let report = api.step();
        for outcome in report.outcomes {
            match outcome.result {
                Ok(CommandResult::Spawned(id)) => spawned.push(id),
                Ok(CommandResult::GroupCreated(id)) => groups.push(id),
                Ok(CommandResult::MissionCreated(id)) => missions.push(id),
                Ok(CommandResult::Applied { applied: n, .. }) => applied += n,
                Err(err) => panic!(
                    "command {:?} at tick {} failed: {err}",
                    outcome.seq, report.tick
                ),
            }
        }
        let entities = api.world_snapshot().entities;
        boarded |= entities.iter().any(|row| api.vehicle_of(row.id).is_some());
    }

    assert_eq!(api.current_tick(), GOLDEN_TICKS);
    assert!(!spawned.is_empty(), "spawns something");
    assert!(
        spawned.iter().any(|id| !api.is_alive(*id)),
        "despawns something"
    );
    assert!(!groups.is_empty(), "forms groups");
    assert!(applied > 0, "applies orders");
    assert!(boarded, "someone rides a vehicle");
    assert!(
        missions
            .iter()
            .filter(|m| api.is_mission_completed(**m))
            .count()
            >= 2,
        "completes a mission, and the replanner sends a group on to another"
    );
    let replay_hash = replay.run(GOLDEN_TICKS).expect("replay runs").state_hash();
    assert_eq!(
        api.state_hash(),
        replay_hash,
        "stepping by hand is the same run"
    );
}
