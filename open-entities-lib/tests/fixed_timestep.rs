//! Roadmap step 1: the simulation advances in whole ticks of `TICK_MS`, independent of host frames.

use open_entities::{Api, EntityComponents, TICK_MS, WorldSnapshot};

const FIXTURE_YAML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/spawn_entity_templates.yaml"
));

/// Tick at which the frame-rate runs are compared.
const TARGET_TICK: u64 = 40;

fn scenario() -> Api {
    let mut api = Api::new();
    api.load_templates_yaml(FIXTURE_YAML).expect("load fixture");
    api.spawn_entity("scout", EntityComponents::default())
        .expect("spawn scout");
    api
}

/// Host loop as the demo runs it: accumulate frame time, step while a whole tick is available.
///
/// Stops right after the step that reaches [`TARGET_TICK`], not at the end of that frame, so both
/// runs compare the same tick.
fn run_host_loop(frame_ms: u32) -> WorldSnapshot {
    let mut api = scenario();
    let mut accumulator = 0;
    loop {
        accumulator += frame_ms;
        while accumulator >= TICK_MS {
            api.step();
            accumulator -= TICK_MS;
            if api.current_tick() == TARGET_TICK {
                return api.world_snapshot();
            }
        }
    }
}

#[test]
fn host_frame_rate_does_not_change_state_at_same_tick() {
    let at_60_hz = run_host_loop(16);
    let at_30_hz = run_host_loop(33);
    assert_eq!(at_60_hz, at_30_hz);
    assert_eq!(at_60_hz.tick, TARGET_TICK);
}

#[test]
fn step_advances_tick_counter_by_one() {
    let mut api = scenario();
    assert_eq!(api.current_tick(), 0);
    api.step();
    assert_eq!(api.current_tick(), 1);
    api.step();
    assert_eq!(api.current_tick(), 2);
    assert_eq!(api.world_snapshot().tick, 2);
}
