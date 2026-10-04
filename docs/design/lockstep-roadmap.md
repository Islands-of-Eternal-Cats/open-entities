# Lockstep roadmap

Goal: a deterministic simulation of 100 000 units, suitable for lockstep multiplayer and replays.

| Step | Topic | Status |
| ---- | ----- | ------ |
| 1 | Fixed timestep | done, PR #22 |
| 2 | Integer simulation space | done, PR #23 |
| 3 | Commands, replay, state hash | done, PR #24 |
| 4 | Scale: benchmark and binary render boundary | in progress, PR #25 |
| 5 | Extension API | |

## How to work through this file

- One step = one branch `step-N-<slug>` = one PR, in the order above.
- The decisions listed under each step are made. Do not reopen them; the only reasons to stop
  are listed in "When to stop and ask".
- Write the tests from the step's "Done when" first, then implement.
- In the same PR: README.md, CHANGELOG.md (one heading per change type under `[Unreleased]`,
  breaking entries prefixed **Breaking.**), `js-app/CORE-API.md` whenever the JS boundary changes,
  and CLAUDE.md — remove the `(target)` marker from the invariant the step fulfils.
- PR description: the step's "Done when" as a checklist, plus the verification commands run.
- Before pushing, check the commit author with `git log -1 --format='%an <%ae>'`.

## When to stop and ask

Proceed without asking unless one of these happens:

1. A "Done when" item cannot be met as written.
2. A decision in this file contradicts what the code actually does, and following it would mean
   a larger change than the step describes.
3. The golden state hash differs between platforms (step 3). Never "fix" this by loosening the test.
4. A benchmark misses its budget by more than 2× (step 4). Report the profile before optimising.

## Definitions (apply from step 2 on)

- `current_tick()` is the number of completed steps. A snapshot taken at tick N is the state after
  N steps.
- A command scheduled for tick T is applied at the start of the step that produces tick T
  (that is, while `current_tick() == T - 1`). `submit` targets `current_tick() + 1`.
- Simulation modules: `components/`, `systems/`, `simulation.rs`, `orders.rs`, `groups.rs`,
  `missions.rs`, `boarding.rs`, `core.rs`, and from step 3 `commands.rs`, `replay.rs`,
  `state_hash.rs`. Boundary modules: `import/`, `export/`,
  `entity_components.rs`, `map.rs` and the new `units.rs`. Only boundary modules may use floats.

---

## Step 1 — Fixed timestep (done, PR #22)

`TICK_MS` = 50, `Api::step()`, `Api::current_tick()`, `SimTick`, `WorldSnapshot.tick`
(schema 5), worker `FrameClock` with interpolation.

Carried forward:

- The worker re-reads the whole world as JSON on every frame, including frames without a step.
  Fixed in step 4.
- Replies to orders carry an uninterpolated snapshot. Goes away in step 3.

---

## Step 2 — Integer simulation space (done, PR #23)

**Why.** Positions are `f32`, arrival is judged at 0.1, unboarding places a unit 1.5 units from the
vehicle. Float results diverge across platforms and compilers.

**Decisions.**

- Simulation unit: the milli-unit. `pub const MILLI_PER_UNIT: i32 = 1000;`. Positions, move
  targets, radii, ranges and offsets are `i32` milli-units.
- Speeds and velocities are stored as `i32` milli-units **per tick**. Conversion at the boundary:
  `per_tick = round(units_per_second × TICK_MS)`, back: `units_per_second = per_tick / TICK_MS`.
  At `TICK_MS` = 50 this quantises speeds to 0.02 units/s (1 milli-unit per tick). This is
  intended: the export shows the speed actually simulated. Document it in README.
- All intermediate arithmetic in `i64`. Division truncates toward zero. Lengths via
  `u64::isqrt` on the squared length (stable since Rust 1.84; the crate requires 1.85).
- The outside world keeps map units as decimals: YAML, `spawn_entity` overrides, the JSON export
  and the JS API do not change shape or units. `units.rs` converts: in with
  `(v * 1000.0).round()` in `f64` (round half away from zero), out with `milli as f64 / 1000.0`.
  Values beyond `i32` range are an import error. The out conversion is exact: any `i32` divided by
  1000 has at most 13 significant digits, so the shortest round-trip form serde prints is the
  original decimal.
- Export shape is unchanged, so the schema version stays **5**.
- Rust public API is breaking: component fields become `i32`. Add `from_units(f64, f64)`
  constructors on `Position` and `MoveTarget` in `units.rs` for ergonomic host code.
- Seek, in integers: `d = target − position` (`i64`), `dist = isqrt(dx² + dy²)`. If
  `dist ≤ max(ARRIVAL_RADIUS, speed_per_tick)`: snap to target, remove `MoveTarget`, zero
  `Velocity`. Otherwise `velocity = d × speed_per_tick / dist`.
- Multi-unit order grid: `ceil(sqrt(n))` computed with `isqrt`, spacing in milli-units.
- `BoardError` carries the distance as `i32` milli-units and derives `Eq` again.
- Mechanical gates: `#![deny(clippy::float_arithmetic)]` at the top of every simulation module,
  and a CI step that fails if `f32` or `f64` appears in a simulation module.
- Housekeeping in the same PR: merge the duplicate `### Changed` headings in CHANGELOG.

**Done when.**

- The tank test is exact: 0.5 units/s is 25 milli-units per tick, and one `step()` moves it by
  exactly 25 (`assert_eq!`, no tolerance).
- Round trip: a YAML position with up to three decimals imports and exports as the same decimal.
- Quantisation test: 0.51 units/s is simulated and exported as 0.52 units/s.
- All existing behaviour tests pass on the unchanged YAML fixtures.
- clippy with the float lint and the CI float check are green.
- README has a "Units" section: milli-units, per-tick speeds, quantisation, where conversion happens.
- CLAUDE.md: the integer invariant loses `(target)`.

---

## Step 3 — Commands, replay and state hash (done, PR #24)

**Why.** Orders are method calls that mutate the world immediately. Lockstep and replays need
orders as data, applied at a known tick, and a way to prove two runs are identical.

**Decisions.**

- `enum Command`, serde with `#[serde(tag = "type", rename_all = "snake_case")]`, one variant per
  order: `Spawn { template, overrides }`, `Despawn { ids }`, `MoveTo { ids, target }`,
  `Stop { ids }`, `CreateGroup { faction }`, `AddToGroup { group, unit }`,
  `RemoveFromGroup { unit }`, `GroupMoveTo { group, target }`, `ClearGroupManual { group }`,
  `CreateMission { target, radius }`, `AssignGroup { mission, group }`,
  `UnassignGroup { group }`, `Board { units, vehicle }`, `Unboard { units }`.
- `Api::submit(command) -> CommandSeq` targets the next tick.
  `Api::schedule(tick, command) -> Result<CommandSeq, ScheduleError>` requires
  `tick > current_tick()`. Queue: `BTreeMap<tick, Vec<(CommandSeq, Command)>>`; within a tick,
  commands apply in `CommandSeq` order.
- `step()` returns `StepReport { tick, outcomes: Vec<CommandOutcome { seq, result }> }`. Results
  carry created ids (`Spawned(EntityId)`, `GroupCreated`, `MissionCreated`), counts of skipped
  units, or a `CommandError`.
- The existing immediate `Api` methods stay: they are what commands call, and tools and tests may
  use them. README states that networked hosts must use `submit`/`schedule` only.
- `Replay { version: 1, templates_yaml, map_yaml, seed: u64, commands: Vec<{ tick, command }> }`,
  stored as JSON; `Replay::run(until_tick) -> Result<Api, ReplayError>`. `seed` is reserved: the
  RNG resource (ChaCha8 seeded from it) is added when the first system needs randomness.
- `Api::state_hash() -> u64`: FNV-1a 64 over `current_tick`, then every entity in ascending
  `(index, generation)`; per entity its id, then each simulation component in a fixed order as a
  tag byte plus little-endian fields. Includes internal relation components (passenger, boarding
  target, group membership, mission assignment, order-source claims). Excludes per-tick scratch
  resources such as `ArrivedThisTick`. Implemented through a `StateHash` trait; the registry macro
  emits the calls for registered components.
- Determinism audit:
  - `clippy.toml` `disallowed-types` for `std::collections::HashMap` and `HashSet`, with the
    reason "iteration order is random; use BTreeMap or IndexMap".
  - Every "pick one of several" decision breaks ties by `EntityId`: replanner's nearest mission,
    mission completion, seat assignment.
  - Tests build the schedule with ambiguity detection set to error.
- Golden replay: `fixtures/replays/basic.json`, about 600 ticks covering move, stop, groups, a
  mission with the replanner, boarding and despawn. Its hash is in `fixtures/replays/basic.hash`.
  Regenerate only with `UPDATE_GOLDEN=1`, and the PR must say why the hash changed.
- CI: the golden test runs on pinned runners `ubuntu-24.04`, `windows-2025`, `macos-15` and under
  wasm in Node. Add `rust-toolchain.toml` pinning the stable version current at the time.
- JS: the worker sends orders with `submit`; promises such as `spawnAt` resolve from the matching
  outcome in the next frame. Order replies no longer carry snapshots.

**Done when.**

- The golden-hash test passes natively on all three runners and under wasm.
- For every hashed component, a test shows that changing one field changes the hash.
- Ambiguity detection is on in tests and passes.
- The "known limitation" about order replies is removed from `js-app/CORE-API.md`.
- CLAUDE.md: the command invariant loses `(target)`.
- README has "Commands", "Replays" and "State hash" sections.

---

## Step 4 — Scale: benchmark and binary render boundary

**Why.** The world crosses to the renderer as JSON, and the worker re-reads it every frame, even
without a step. At 100 000 units that is megabytes of serialisation, parsing and structured clone
per frame, long before the ECS becomes the bottleneck.

**Decisions.**

- criterion benches in `open-entities-lib/benches/step.rs`: `step_100k_moving` (100 000 movers,
  seeded targets on a 2000 × 2000 map), `step_100k_idle`, `step_1k_groups_of_100`. CI only
  compiles them (`cargo bench --no-run`); timings are measured locally.
- Initial budgets per `step()` at 100k moving: native ≤ 5 ms, wasm in Node ≤ 15 ms. Demo: 60 fps
  at 100k on the author's machine.
- No `par_iter` for now: wasm is single-threaded here and sequential systems keep determinism simple.
- Frame buffer: `Simulation.writeFrame()` returns an `Int32Array`: header `[tick_lo, tick_hi,
  count]`, then per entity `[index, generation, x, y]` in milli-units, sorted by `index`.
- Protocol: a frame without a step posts only `{ tick, alpha }`. A frame with steps posts the
  buffer of the last tick as a transferable; after two or more steps it also posts the buffer of
  tick t−1. The main thread keeps `previous` and `current` and shifts them.
- The renderer interpolates by merge-joining `previous` and `current` on `index`; a generation
  mismatch means a different entity and is not interpolated.
- Metadata (type, faction, seats, aboard, boarding, group, move target) is tracked with bevy change
  detection and removals. `Simulation.metaDelta()` returns JSON with the entities changed or
  removed since the last call; the worker posts it only when non-empty.
- `getWorldAsJson()` stays for debugging and saves and is not called per frame.
- Demo: a "Stress" control spawning N seeded movers with random targets; the Forces list shows
  only the selection, at most 200 rows; batched sprite rendering suitable for the installed PixiJS
  version.

**Done when.**

- README records measured step times (native and wasm) and demo fps at 100k.
- Tests: a frame without a step posts only `tick` and `alpha`; a frame with steps posts buffers;
  merge-join interpolation handles spawn and despawn between ticks.
- The per-frame path creates no JSON.
- Budgets met, or stopped and reported per "When to stop and ask".

---

## Step 5 — Extension API

**Why.** Adding a component means editing `registered.rs` inside the crate, and the schedule is a
private field of `Core`. A game cannot add its own components or systems without forking.

**Decisions.**

- `ComponentRegistry` resource: descriptors in registration order, each with the field name and
  functions to insert from YAML, export and hash. `Api::new()` registers the built-ins through the
  same path; `define_registered_components!` becomes sugar over it.
- `Api::register_component::<T>(field)` for
  `T: Component + Serialize + DeserializeOwned + StateHash`. It must be called before
  `load_templates_yaml`; a duplicate field or registration after the first `step()` is an error.
- `EntityComponents` gains `#[serde(flatten)] extra: BTreeMap<String, serde_yaml::Value>`,
  resolved against the registry. An unknown key is an error that lists the known fields.
- `impl_state_hash_via_serde!` helper for custom components: a small serializer that feeds FNV
  and returns an error on `f32`/`f64`, so a float field fails loudly at registration time.
- `SimSet` system sets, chained: `Commands → Steering → Movement → PostMovement → Resolve`.
  Built-ins: command application in `Commands`; mission steering, boarding approach, seek in
  `Steering`; movement in `Movement`; passenger sync in `PostMovement`; mission completion and
  replanner in `Resolve`.
- `open_entities::extend` module with `add_systems(set, systems)`. Within a set, user systems run
  after the built-ins, chained in registration order. Like `core_mut`, it is a documented door that
  ties the caller to the `bevy_ecs` version.
- Scope: extension is for Rust games. JS cannot register Rust components, but custom components
  still appear in the JSON export.

**Done when.**

- `examples/fuel.rs`, using only the public API and `extend`: `fuel: 100` in YAML, a `burn_fuel`
  system in `PostMovement` that burns one unit per tick while moving and stops the unit at zero.
  `fuel` appears in the export and changes the state hash. A test exercises the same logic.
- A test shows that registering a component with a float field fails.
- Ambiguity detection passes with user systems added.
- README has an "Extending the engine" section.

---

## After step 5

- Decide on 0.1.0: `publish = true`, and turn `[Unreleased]` into the release section.
- Next topics, outside this roadmap: network transport, spatial grid, flow-field pathfinding,
  fog of war.
