# Lockstep roadmap

Goal: a deterministic simulation of 100 000 units, suitable for lockstep multiplayer and replays.

Nothing is published yet (`publish = false`), so breaking changes are cheap now and expensive later.
Each step is one PR (or one OpenSpec change), in this order. Every step starts with failing tests
from its "Done when" list; README.md and CHANGELOG.md change in the same PR.

## 1. Fixed timestep

**Why.** `Api::tick(dt_ms)` takes the delta from the host, and the demo derives it from
`requestAnimationFrame`. The world therefore depends on frame rate: two clients at 60 Hz and 144 Hz
diverge with identical orders, and a replay cannot be reproduced. The seek-overshoot bug in the
changelog came from the same root — behaviour depended on step size.

**Change.**

- `pub const TICK_MS: u32 = 50;` (20 Hz, tune later). `Api::step()` advances exactly one tick.
- Remove `tick(dt_ms)`, `MAX_DT_MS` and `TickError::ZeroDeltaTime` (breaking).
- Replace the `SimDelta` resource with the constant; add a `SimTick(u64)` resource incremented once
  per step and `Api::current_tick()`.
- Speeds stay "units per second" in YAML; systems use the per-tick value derived from `TICK_MS`.
- JS worker: accumulate elapsed milliseconds and call `step()` while the accumulator holds at least
  one tick, capped at a few steps per frame (drop the excess to avoid a spiral of death). The
  snapshot carries `tick`; the renderer interpolates between the previous and current positions by
  `alpha = accumulator / TICK_MS`.

**Done when.**

- A test drives the same scenario with a host loop at 16 ms frames and at 33 ms frames and gets
  identical snapshots at the same tick.
- No system reads a host-provided delta.
- The demo moves smoothly at both 60 Hz and 144 Hz.

## 2. Integer simulation space

**Why.** Positions are `f32`, arrival is judged at a distance of 0.1, unboarding places a unit
1.5 units beside the vehicle. Float results diverge across platforms and compilers.

**Change.**

- Store positions, velocities, targets, speeds, ranges and radii as `i32` milli-units
  (1 map unit = 1000).
- YAML keeps human decimal values (`x: 20.5`); the loader converts once, rounding to the nearest
  milli-unit. Decide whether the export carries exact integers or decimals; bump the export schema
  version either way.
- Compare squared distances in `i64`. Where a length is unavoidable (normalising a direction), use
  `i64::isqrt` (stable since Rust 1.84; the crate already requires 1.85).
- Deny floats mechanically: `#![deny(clippy::float_arithmetic)]` on the simulation, systems and
  components modules.

**Done when.**

- clippy passes with the float lint enabled on simulation modules.
- All existing behaviour tests pass on integer fixtures.
- The export schema version is bumped and documented in README.

## 3. Commands, replay and state hash

**Why.** Orders are method calls that mutate the world immediately. Lockstep and replays need orders
as data, applied at a known point in a known tick.

**Change.**

- `enum Command` (serde) covering every order: move, stop, group operations, missions,
  board/unboard, spawn, despawn.
- `Api::submit(command)` queues for the next step; `step()` applies the queue first, in submission
  order. Existing order methods become thin wrappers over it.
- Replay = initial YAML (templates and map) + seed + `Vec<(tick, Command)>`; `Api::run_replay`.
- `Api::state_hash() -> u64`: FNV-1a over every simulation component, walking entities in
  `EntityId` order. Do not use `DefaultHasher`: its algorithm is not guaranteed stable across Rust
  releases.
- Commit a fixture replay with its golden hash.

**Done when.**

- The golden-hash test passes natively and under wasm (Node).
- CI runs it on ubuntu, windows and macos.

## 4. Scale: benchmark and binary render boundary

**Why.** The core hands the whole world to the worker as one JSON document every tick, and the
worker parses it into rows. At 100 000 units that is megabytes of serialisation, parsing and
structured clone per frame — the bottleneck long before the ECS.

**Change.**

- criterion bench `step_100k`: 100 000 movers with targets, measuring `step()`. CI only checks that
  it builds and runs (`cargo bench -- --test`); timings are tracked locally because CI runners are noisy.
- `Simulation` writes per-entity `[index, generation, x, y]` into an `Int32Array` (or exposes a view
  into wasm memory); the worker posts the buffer as a transferable `ArrayBuffer`.
- Rarely changing data (type, faction, seats, group) is sent only when it changes. The JSON export
  stays for debugging and saves.
- Renderer: batched sprites for large unit counts.

**Done when.**

- README records measured numbers: step time at 100k natively and in wasm, demo fps at 100k.
- The per-frame path allocates no JSON.

## 5. Extension API

**Why.** Adding a component means editing `registered.rs` inside the crate, and the schedule is a
private field of `Core`. A game cannot add its own components to YAML and export, or its own systems
to the tick, without forking the library.

**Change.**

- Runtime component registry: `Api::register_component::<T>("field")` for
  `T: Component + Serialize + DeserializeOwned`. Built-in components go through the same path; the
  macro becomes sugar over it.
- Named, chained system sets, for example `Commands → Steering → Movement → PostMovement → Resolve`,
  and an `extend` module with `add_systems(set, systems)`. Like `core_mut`, it is a documented door
  that ties the caller to the `bevy_ecs` version.
- User systems follow the same determinism invariants as built-in ones.

**Done when.**

- An example that uses only the public API adds a `Fuel` component through YAML and a system that
  consumes it, and `fuel` appears in the export.
