//! `Api::step` at 100 000 entities. CI only compiles this (`cargo bench --no-run`); timings are
//! measured locally, because shared runners are too noisy to hold a budget.
//!
//! Budgets per step at 100k moving (docs/design/lockstep-roadmap.md, step 4): native ≤ 5 ms,
//! wasm in Node ≤ 15 ms (`wasm-bindings/demo/bench.mjs`).

use std::hint::black_box;
use std::time::{Duration, Instant};

use criterion::{Criterion, criterion_group, criterion_main};
use open_entities::components::{MoveTarget, Position};
use open_entities::{Api, EntityComponents, EntityId, MILLI_PER_UNIT};

const UNITS: usize = 100_000;
/// The map is 2000 × 2000 map units.
const MAP: u64 = 2000 * (MILLI_PER_UNIT.unsigned_abs() as u64);
const SEED: u64 = 0x005e_ed0f_57e9;
/// Ticks a world is stepped before it is rebuilt. Stepping one world for the whole run would let
/// the movers arrive one by one and the bench drift towards idle; at 5 units/s, 200 ticks cover 50
/// map units, so all but a fraction of a percent are still walking when the world is replaced.
const TICKS_PER_WORLD: u32 = 200;

const TEMPLATES: &str = "entities:
  mover:
    faction: 1
    velocity: { vx: 0.0, vy: 0.0 }
    base_move_speed: 5.0
";

/// SplitMix64: a fixed, seeded sequence, so every run benches the same world.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// A coordinate on the map, in milli-units.
    fn coord(&mut self) -> i32 {
        i32::try_from(self.next() % MAP).expect("the map fits i32")
    }
}

fn spawn_movers(api: &mut Api, rng: &mut Rng, with_targets: bool) -> Vec<EntityId> {
    api.load_templates_yaml(TEMPLATES).expect("templates load");
    (0..UNITS)
        .map(|_| {
            let position = Position {
                x: rng.coord(),
                y: rng.coord(),
            };
            let move_target = with_targets.then(|| MoveTarget {
                x: rng.coord(),
                y: rng.coord(),
            });
            api.spawn_entity(
                "mover",
                EntityComponents {
                    position: Some(position),
                    move_target,
                    ..Default::default()
                },
            )
            .expect("mover spawns")
        })
        .collect()
}

/// Times `step()` alone; building a fresh world every [`TICKS_PER_WORLD`] ticks is not timed.
fn bench_steps(c: &mut Criterion, name: &str, build: fn() -> Api) {
    let mut api = build();
    let mut ticks = 0;
    c.bench_function(name, |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                if ticks == TICKS_PER_WORLD {
                    api = build();
                    ticks = 0;
                }
                let start = Instant::now();
                black_box(api.step());
                total += start.elapsed();
                ticks += 1;
            }
            total
        });
    });
}

fn moving() -> Api {
    let mut api = Api::new();
    spawn_movers(&mut api, &mut Rng(SEED), true);
    api
}

fn idle() -> Api {
    let mut api = Api::new();
    spawn_movers(&mut api, &mut Rng(SEED), false);
    api
}

/// 1000 groups of 100, each group sent to its own mission: the automation path.
fn groups_of_100() -> Api {
    let mut api = Api::new();
    let mut rng = Rng(SEED);
    let units = spawn_movers(&mut api, &mut rng, false);
    for squad in units.chunks(100) {
        let group = api.create_group(1);
        for unit in squad {
            api.add_to_group(group, *unit).expect("unit joins");
        }
        let mission = api.create_mission(
            MoveTarget {
                x: rng.coord(),
                y: rng.coord(),
            },
            MILLI_PER_UNIT,
        );
        api.assign_group(mission, group).expect("group assigned");
    }
    api
}

fn step_100k_moving(c: &mut Criterion) {
    bench_steps(c, "step_100k_moving", moving);
}

fn step_100k_idle(c: &mut Criterion) {
    bench_steps(c, "step_100k_idle", idle);
}

fn step_1k_groups_of_100(c: &mut Criterion) {
    bench_steps(c, "step_1k_groups_of_100", groups_of_100);
}

criterion_group!(
    benches,
    step_100k_moving,
    step_100k_idle,
    step_1k_groups_of_100
);
criterion_main!(benches);
