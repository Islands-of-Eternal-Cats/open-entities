# OpenEntities

Rust workspace with the core library crate `open_entities` in `open-entities-lib/`.

The library uses [Bevy ECS](https://crates.io/crates/bevy_ecs) (`bevy_ecs` only, not the full Bevy engine) for entity simulation. Public entry points:

- [`Api`](open-entities-lib/src/api.rs) — the facade: spawn, orders, lifecycle, import, export
- [`Command`](open-entities-lib/src/commands.rs) — an order as data, applied at the start of a known tick: `Api::submit` / `Api::schedule`
- [`Replay`](open-entities-lib/src/replay.rs) — content plus command log; `Api::state_hash()` proves two runs identical
- [`EntityId`](open-entities-lib/src/orders.rs) — how every call names an entity: an `{index, generation}` pair
- [`Core`](open-entities-lib/src/core.rs) — owns the ECS [`World`](https://docs.rs/bevy_ecs/latest/bevy_ecs/world/struct.World.html); reachable through `Api::core()` / `core_mut()`
- [`export`](open-entities-lib/src/export/mod.rs) — `Api::world_snapshot()` captures **every entity** in the world as a `WorldSnapshot` (**schema version 5**); it serializes flat, and registered gameplay fields are omitted when absent (not `null`). The WASM bindings turn it into JSON
- [`EntityComponents`](open-entities-lib/src/entity_components.rs) — shared struct for YAML templates, `spawn_entity` overrides, and flattened export rows

## Where bevy stops

No `bevy_ecs` type appears in the normal path: `spawn_entity`, `load_map_yaml`, the orders and the
lifecycle calls all speak `EntityId`, and the world comes out as JSON. The crate root re-exports
nothing from `bevy_ecs`, so the ECS version is not part of this library's public contract and a
consumer on a different `bevy_ecs` can still depend on it.

`Api::core()` and `Api::core_mut()` are the deliberate exception: they hand out the `World` for
anyone who wants to write systems or queries directly. That is a door, not an oversight — but step
through it and your code is tied to the `bevy_ecs` version this crate builds against.

Domain components live under `open_entities::components`: `Position`, `Velocity`, `Faction`, `MoveTarget`, `BaseMoveSpeed`, and `Health`.

## Simulation tick

The simulation advances only in whole ticks of constant length, `TICK_MS` = **50 ms** (20 Hz).
`Api::step()` runs exactly one tick; the host never passes a delta, so the same orders give the
same state at the same tick whatever the host's frame rate. `Api::current_tick()` counts the ticks
run so far, and every `WorldSnapshot` carries it as `tick`.

A step first applies the [commands](#commands) due at the tick it produces, then runs the ECS
schedule: mission steering, boarding approach, `seek_system` (entities with `MoveTarget` +
`BaseMoveSpeed` + `Velocity`), `movement_system` (all `Position` + `Velocity`), passenger sync,
mission completion and the replanner — strictly in that order. In debug builds, which is every
test, the schedule is built with ambiguity detection set to error: two systems touching the same
data with no order between them fail the build of the schedule instead of running in whatever
order the executor picks. Components hold speeds and
velocities in milli-units per tick, so movement adds `Velocity` to `Position` once per tick; see
[Units](#units).

Arrival (distance ≤ `ARRIVAL_RADIUS` = 100 milli-units, or this tick's step would reach the
target): snap to target, remove `MoveTarget`, zero `Velocity`, skip movement that tick.

A host with a variable frame rate accumulates real elapsed time and calls `step()` while at least
one tick is in the accumulator; the remainder, `accumulator / TICK_MS`, is how far to interpolate
between the last two ticks when drawing.

```rust
let mut accumulator = 0;
accumulator += frame_ms;
while accumulator >= open_entities::TICK_MS {
    api.step();
    accumulator -= open_entities::TICK_MS;
}
```

## Units

The simulation runs on integers only, so the same inputs give bit-identical state on every
platform. Its unit is the **milli-unit**: `MILLI_PER_UNIT` = 1000 per map unit.

- Positions, move targets, mission radii, `BOARDING_RANGE` and the unboarding offset are `i32`
  milli-units.
- Speeds (`BaseMoveSpeed`) and velocities (`Velocity`) are `i32` milli-units **per tick**.
- Seek and range checks work in `i64`/`u128`, divide truncating toward zero, and take lengths with
  an integer square root. No system uses `f32` or `f64`; the crate enforces this with
  `#![deny(clippy::float_arithmetic)]` in every simulation module, and CI fails if `f32`/`f64`
  appears in one (`make float-check`).

The outside world keeps **map units as decimals**: YAML templates and maps, `spawn_entity`
overrides, the JSON export and the JavaScript API all read and write `1.5`, not `1500`, with the
same shape as before. Conversion happens in one place, `open_entities::units`, at the boundary:

- In: `round(value × 1000)`, half away from zero. A value that does not fit in `i32` milli-units
  (beyond about ±2 147 483 map units) is an import error.
- Out: `milli / 1000`, which is exact — a position with up to three decimals exports as the same
  decimal it was imported as.
- Speeds in: `round(units_per_second × TICK_MS)`; out: `per_tick / TICK_MS`.

At `TICK_MS` = 50 one milli-unit per tick is **0.02 units/s**, so speeds are quantised to that
step: `base_move_speed: 0.51` is simulated as 26 milli-units per tick and exported as `0.52`. The
export shows the speed actually simulated.

Rust host code that builds components directly writes milli-units, or uses the constructors
`Position::from_units(x, y)` and `MoveTarget::from_units(x, y)`.

## Commands

Orders are data. A host does not change the world between steps; it hands the `Api` a `Command`,
and the command is applied at the start of the step that produces its tick:

```rust
use open_entities::{Command, CommandResult};

let seq = api.submit(Command::MoveTo { ids: vec![scout], target: MoveTarget::from_units(20.0, 0.0) });
api.schedule(120, Command::Stop { ids: vec![scout] })?; // a chosen future tick

let report = api.step();                 // applies `seq`, then runs the systems
assert_eq!(report.outcomes[0].seq, seq);
assert!(matches!(report.outcomes[0].result, Ok(CommandResult::Applied { applied: 1, .. })));
```

- `Api::submit(command)` targets the next tick, `current_tick() + 1`. `Api::schedule(tick,
  command)` targets any later tick and refuses one that is not after `current_tick()`
  (`ScheduleError::NotInFuture`).
- Both return a `CommandSeq`, which grows with every call. Within a tick, commands apply in
  `CommandSeq` order — the order they were handed in.
- `Api::step()` returns a `StepReport { tick, outcomes }`, one `CommandOutcome { seq, result }` per
  command applied. A result is the new id for a creation (`Spawned`, `GroupCreated`,
  `MissionCreated`), `Applied { applied, skipped }` for anything else, or a `CommandError` saying
  why the command was refused. A refused command changes nothing.

There is one command per order: `spawn`, `despawn`, `move_to`, `stop`, `create_group`,
`add_to_group`, `remove_from_group`, `group_move_to`, `clear_group_manual`, `create_mission`,
`assign_group`, `unassign_group`, `board` (the walk-over order, `order_board`) and `unboard`.
In JSON a command is tagged by `type`, with points, radii and overrides in map units like
everywhere else outside the simulation:

```json
{ "type": "move_to", "ids": [{ "index": 5, "generation": 0 }], "target": { "x": 20.0, "y": 0.0 } }
{ "type": "create_mission", "target": { "x": 60.0, "y": 10.0 }, "radius": 4.0 }
{ "type": "spawn", "template": "soldier", "overrides": { "position": { "x": 5.0, "y": 40.0 } } }
```

The immediate methods described below (`order_move_to`, `create_group`, `board`, …) are what the
commands call. They stay public for tools, scenarios and tests. **A networked host must use
`submit` and `schedule` only**: a change made between steps is invisible to the other peers and
to the replay, and the match diverges.

## Replays

A `Replay` is the content a match started from plus every command and the tick it applied at:

```json
{
  "version": 1,
  "templates_yaml": "entities: ...",
  "map_yaml": "spawns: ...",
  "seed": 20261004,
  "commands": [{ "tick": 4, "command": { "type": "assign_group", "mission": { "index": 13, "generation": 0 }, "group": { "index": 11, "generation": 0 } } }]
}
```

`Replay::from_json` / `to_json` read and write it, and `Replay::run(until_tick)` loads the content
into a fresh `Api`, schedules every command and steps to `until_tick`. Commands after
`until_tick` stay queued, so stepping the returned `Api` carries on with the recording; `run(0)`
loads without stepping. A command at tick 0 is refused — there is no step to apply it in.
`map_yaml` may be empty. `seed` is reserved: the RNG resource seeded from it arrives with the first
system that needs randomness, and replays recorded before then keep their shape.

The golden replay [`fixtures/replays/basic.json`](fixtures/replays/basic.json) plays 600 ticks of
moves, stops, groups, three missions with the replanner, boarding and a despawn. Its state hash is
pinned in [`fixtures/replays/basic.hash`](fixtures/replays/basic.hash), and CI checks it natively on
`ubuntu-24.04`, `windows-2025` and `macos-15` and under wasm32 in Node. Regenerate the hash only for
an intended change in behaviour, and say in the pull request why it changed:

```bash
UPDATE_GOLDEN=1 cargo test -p open_entities --test golden_replay
```

A hash that differs on one platform only is a determinism bug; it is never fixed by loosening the
test.

## State hash

`Api::state_hash()` is a `u64` that two runs agree on exactly when their simulation state does:
FNV-1a 64 over `current_tick`, then every entity in ascending `(index, generation)` — its id, then
each simulation component it carries as a tag byte plus its fields in little-endian, then a closing
`0` byte. All integers, so the value is the same on every platform, native and wasm32.

Every component that is simulation state is in it: the registered ones (`Position`, `Velocity`,
`Faction`, `MoveTarget`, `BaseMoveSpeed`, `Health`, `Boardable`), the template name, and the
internal relations and markers — passenger, boarding target, group membership, the group itself,
manual control, mission assignment, the mission, completion, the replanner's marker and the
order-source claim. Per-tick scratch such as `ArrivedThisTick` is not. Each component implements
the `StateHash` trait; the registry macro emits the calls for registered components.

Lockstep peers compare it to detect a desync. In JavaScript, `stateHash()` returns it as 16 hex
digits.

### What keeps it deterministic

- Integers only in simulation state (see [Units](#units)).
- `clippy.toml` disallows `std::collections::HashMap` and `HashSet` — their iteration order is
  random; use `BTreeMap` or `IndexMap`.
- Every "pick one of several" breaks ties by `EntityId` (index, then generation), never by query
  order: grid slots for group and mission orders, the last seat when several units reach a vehicle
  on the same tick, the replanner's nearest mission, the order missions are closed in. Listings
  (`group_members`, `passengers`, `approaching`, `mission_assignees`) come back in `EntityId`
  order.
- The schedule is built in `Api::new()`, not on the first step: building it creates resources,
  which in `bevy_ecs` 0.19 are entities, so a lazy build would hand out an entity index in the
  middle of a match.
- The toolchain is pinned in `rust-toolchain.toml`.

## Move orders

`Api::order_move_to(ids, target)` gives a group of entities a destination. Ids are
[`EntityId`](open-entities-lib/src/orders.rs) — the `{index, generation}` pair `world_snapshot` reports
as each entity's `id`, so a host can feed them straight back from a snapshot.

Entities without `Position` or `BaseMoveSpeed` are skipped: immobile things cannot take a move
order. With more than one id the destinations are spread over a grid around the point, so the group
does not pile onto one spot. The returned `OrderReport` says how many took the order.

```rust
api.order_move_to(&[id], MoveTarget::from_units(20.0, 0.0));
```

`Api::order_stop(ids)` is the counterpart: it zeroes `Velocity` and drops any `MoveTarget`. It does
**not** require a `BaseMoveSpeed`, because in this engine a `Velocity` alone is movement — an entity
that carries one without a move speed drifts until something stops it.

## Groups

A group is an entity of its own, created for one faction; membership lives on the units, so there
is one place to read it and one place to change it.

```rust
let group = api.create_group(1);
api.add_to_group(group, unit)?;
api.order_group_move_to(group, MoveTarget::from_units(50.0, 50.0))?;
```

A unit belongs to at most one group — joining a second one leaves the first — and a group commands
only units of its own faction. An empty group stays alive: it can be refilled and ordered again.

A group order steers every member that will take it, spread over the same grid a multi-unit order
uses, and leaves the group **manually controlled**. Automation must leave a manual group alone
until `Api::clear_group_manual(group)` hands it back; nothing releases it on its own, because a
player who cannot tell who is steering has lost the thread.

Members already following a **personal** order keep it. Orders carry an `OrderSource` —
`MissionSteering < GroupSteering < PlayerUnit` — and a weaker source never overwrites a stronger
one. The claim is released when the unit arrives or is stopped.

See [`docs/design/group-mission-contract.md`](docs/design/group-mission-contract.md) for the full
behaviour, including missions and the replanner (below).

## Missions

A mission is a point somebody should reach, and how close counts as reached. Groups are sent to
it; the first arrival finishes it for everyone.

```rust
let mission = api.create_mission(MoveTarget::from_units(80.0, 20.0), 2000); // radius in milli-units
api.assign_group(mission, group)?;
```

While a mission is assigned, its groups are steered toward it every tick. A group under manual
control is skipped, and a manual order to a group takes it off the mission — the mission is then
free for whoever can still work it. A group that has lost every member is unassigned too, since it
cannot arrive.

When any live member of any assigned group comes within the radius, the mission is marked
completed, every assignee is released, and the units that were following **mission** steering stop.
Personal and group orders are untouched: they never belonged to the mission.

A completed mission stays in the world, marked, and refuses new groups.

When a mission closes, the groups it released are handed to the **replanner**: each takes the
nearest open mission, or stands idle when there is none. Only groups whose mission ended under
them are planned for — creating a mission does not send every idle squad after it — and a group
the player is steering by hand is left alone.

## Entity lifecycle

`Api::despawn(ids)` removes entities and returns how many were actually removed — unknown, stale
and repeated ids are skipped, so the count is of entities, never of ids passed in.
`Api::is_alive(id)` says whether an id still resolves.

An id stops resolving the moment its entity is despawned, and stays dead afterwards: the index may
be handed to a new entity, but that one carries a higher `generation`, so an old id never points at
a new entity by accident.

```rust
let removed = api.despawn(&[id]);
assert!(!api.is_alive(id));
```

## Carrying units

A vehicle is an entity with seats; a unit standing next to it can climb aboard.

```yaml
entities:
  jeep:
    position: { x: 0.0, y: 0.0 }
    velocity: { vx: 0.0, vy: 0.0 }
    base_move_speed: 60.0
    boardable: 4
```

```rust
api.order_board(&[unit], jeep)?;  // walk over, follow it if it moves, climb in within range
api.board(unit, jeep)?;           // the primitive: inside at once, from anywhere
api.unboard(unit)?;               // steps off beside the vehicle
```

`order_board` is the order a player gives. It marks the unit with `BoardingTarget`, and each tick
the boarding system points it at where the vehicle is *now* and boards it once within
`BOARDING_RANGE`. The order ends when the unit boards, finds no seat left, loses the vehicle, or
is told to stop or go somewhere else. `board` is the primitive underneath — no distance check —
for scenarios that start a vehicle loaded and for the system itself.

While a unit is aboard, its position belongs to the vehicle: seek and movement skip passengers
entirely, and one system copies the vehicle's position onto them after it has moved. A move order
addressed to a passenger is **refused**, not stored — the position has exactly one owner at a
time, and a target parked on a rider would fire the moment it stepped off and walk it away from
the vehicle that just dropped it. `order_move_to` counts it as skipped, group and mission steering
pass it over.

If the vehicle is despawned, its passengers are let off where they stand: a component pointing at
an entity that no longer exists is a leak waiting to be read.

`Api::passengers(vehicle)`, `Api::vehicle_of(unit)` and `Api::free_seats(vehicle)` read the state.
All five are exposed to JavaScript as `board`, `unboard`, `passengers`, `vehicleOf` and
`freeSeats`, and the browser demo drives them with **B** and **U**.

## Map layout

`Api::load_map_yaml(yaml)` spawns a starting layout and records its bounds. An entry names a
template and carries the same override fields as `spawn_entity`, so there is nothing new to learn
beyond the `template` key:

```yaml
map:
  width: 200.0
  height: 200.0

spawns:
  - template: scout
    position: { x: 20.0, y: 10.0 }
    faction: 1
```

Template names are checked before anything is spawned, so a typo leaves the world untouched rather
than half-populated. The loader inserts only what the template and the entry ask for — an entry
without `velocity` gets no `Velocity` component, and therefore never drifts.

`map` is optional and purely informational: `Api::map_bounds()` hands it to a host that needs to
frame or clamp a view. The simulation does not enforce it — nothing stops an entity from leaving
the map.

See [`fixtures/init_map.yaml`](fixtures/init_map.yaml).

## Import and spawn

Load named entity templates from YAML, then spawn by template name with optional overrides:

1. **`Api::load_templates_yaml(yaml)`** — root must be `entities: { <name>: <components>, ... }`. Replaces any previously loaded templates on success. Template inheritance (`template`, `template: [a, b]`) is resolved at load time.
2. **`Api::spawn_entity(template_name, overrides)`** — requires a prior successful load. [`EntityComponents::default()`](open-entities-lib/src/entity_components.rs) spawns the template as resolved.
3. **Overrides** — each `Some` field in `overrides` replaces the template value; `None` leaves the template unchanged.

See [`open-entities-lib/examples/spawn_entity.rs`](open-entities-lib/examples/spawn_entity.rs) for inheritance and override examples.

## Component registry

Gameplay components are registered in one list:

[`open-entities-lib/src/component_registry/registered.rs`](open-entities-lib/src/component_registry/registered.rs)

```rust
define_registered_components! {
    register_component!(position, Position);
    // ...
}
```

To add a component: implement the type under `components/`, add one `register_component!(field, Type);` line, and run tests — merge, spawn, and export wiring are generated.

`register_component!` must only appear inside `define_registered_components!`; standalone use is a compile error (see `open-entities-lib/tests/ui/`).

## Render boundary

A renderer needs positions every tick and everything else rarely. The two travel separately, and
neither is the JSON export.

**Position frame.** `Api::write_frame(&mut Vec<i32>)` (JS: `Simulation.writeFrame()` →
`Int32Array`) writes the current tick as flat integers:

| Words | Content |
|---|---|
| `0..3` | header `[tick_lo, tick_hi, count]`: the tick split into its low and high 32 bits, then the row count |
| `3 + 4·i ..` | row `i`: `[index, generation, x, y]`, position in milli-units |

Every entity with a `Position` has a row; rows are sorted by `index`, so two frames merge in one
pass. Ids are the `EntityId` parts bit-cast to `i32` (read them back unsigned). Constants:
`FRAME_HEADER_LEN = 3`, `FRAME_STRIDE = 4`. Pass the same `Vec` every tick and its allocation is
reused; the wasm binding returns a fresh array, so a worker can transfer its buffer.

**Metadata delta.** `Api::meta_delta()` (JS: `Simulation.metaDelta()`) returns the entities whose
metadata changed, and those that went away, since the previous call:

```json
{
  "changed": [
    {
      "id": { "index": 4, "generation": 0 },
      "entity_type": "truck", "faction": 1, "seats": 4, "mobile": true,
      "aboard": { "index": 2, "generation": 0 },
      "boarding": { "index": 3, "generation": 0 },
      "group": { "index": 9, "generation": 0 },
      "move_target": { "x": 12.5, "y": 0.0 }
    }
  ],
  "removed": [{ "index": 7, "generation": 0 }]
}
```

Metadata is the template name, faction, seats, `mobile` (carries a `Velocity`), the vehicle it
rides or walks to, its group and its move target; absent fields are omitted, points are in map
units, rows and removals are sorted by id. The first call reports every positioned entity. After
that each `step()` notes what changed through `bevy_ecs` change detection and removal tracking — no
scan of the world — and each candidate is compared with what was last reported, so a system that
writes the same value again (mission steering re-inserts its target every tick) is not a change.
Until the first call nothing is tracked. `metaDelta()` in JavaScript returns `undefined` when
nothing changed, so asking every frame serialises nothing.

Reading the boundary never changes the simulation: the state hash is the same with or without it.

`Api::world_snapshot()` / `getWorldAsJson()` stay for debugging and saves; nothing calls them per
frame.

## Performance

Budgets for `step()` with 100 000 units moving, from the
[lockstep roadmap](docs/design/lockstep-roadmap.md): native ≤ 5 ms, wasm in Node ≤ 15 ms. The
systems run sequentially (no `par_iter`): wasm is single-threaded here, and a fixed order keeps
determinism simple.

Measured on an Apple M5 Pro (64 GB), Rust 1.99.0, Node v26.4.0:

| Measurement | Time per call |
|---|---|
| `step_100k_moving` native: 100 000 movers walking to seeded targets on a 2000 × 2000 map | 1.29 ms |
| `step_100k_idle` native: the same units without targets | 0.19 ms |
| `step_1k_groups_of_100` native: 1000 groups of 100, each assigned to its own mission | 217 ms |
| `step_100k_moving` wasm32 in Node | 2.42 ms (median) |
| `writeFrame()` at 100 000 units, wasm32 in Node, including the copy into JS | 2.21 ms (median) |
| Browser demo at 100 000 units (Stress), canvas 3160 × 1786 at DPR 2, 120 Hz display | 120 fps (budget 60) |

Both 100k step budgets hold, and so does the demo's. Its main thread spends about 2.3 ms per
animation frame drawing 100 000 units and 2.3 ms in PixiJS; the demo runs at the display's
120 Hz in a small window and at 1920 × 1080 alike (Chromium, the same machine). The first version
reached 30 fps: a `Map` lookup per unit per frame, and drawing only when the worker's reply
arrived, which queued behind step frames. The groups bench has no budget yet and is far from cheap: mission steering
and mission completion each scan every group member once per group or mission, so the cost grows
with groups × units. A profile puts almost all of the step there; movement and seek are a rounding
error beside it.

Run them:

```bash
cargo bench -p open_entities --bench step
make wasm-bench
```

The benches step a world for 200 ticks and then rebuild it, untimed, so the movers do not arrive
and turn the moving bench into an idle one. CI only compiles them (`cargo bench --no-run`);
shared runners are too noisy to hold a timing budget.

## Requirements

- Rust **1.99.0**, pinned in [`rust-toolchain.toml`](rust-toolchain.toml) together with `rustfmt`,
  `clippy` and the `wasm32-unknown-unknown` target; `rustup` installs it on the first `cargo`
  call. The crate itself needs 1.85+ (edition 2024, `u64::isqrt`); the pin keeps the golden replay
  and clippy's verdict from moving with each stable release.

Check your toolchain:

```bash
rustc --version
cargo --version
```

## Build

From the repository root:

```bash
cargo build --workspace
```

Build only the library crate:

```bash
cargo build -p open_entities
```

## Test

```bash
make test
```

Or directly:

```bash
cargo test
```

Includes a trybuild compile-fail test for macro misuse (`register_component!` outside the registry wrapper).

## WASM (Node)

The [`wasm-bindings/`](wasm-bindings/) crate exposes the same spawn → export cycle as the Rust library through [`wasm-bindgen`](https://github.com/rustwasm/wasm-bindgen), built for Node with [`wasm-pack`](https://rustwasm.github.io/wasm-pack/).

**Prerequisites** (one-time):

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
```

**Demo** — build WASM, run the Node script (loads [`fixtures/spawn_entity_templates.yaml`](fixtures/spawn_entity_templates.yaml), spawns entities, asserts on `world_json`):

```bash
make wasm-demo
```

**WASM unit tests** (`#[wasm_bindgen_test]` in `wasm-bindings/src/lib.rs`):

```bash
make wasm-test
```

**Both** demo and wasm tests:

```bash
make wasm-check
```

### JavaScript API

`Simulation` mirrors `Api` with camelCase method names:

| JavaScript | Rust |
|------------|------|
| `loadTemplatesYaml(yaml)` | `load_templates_yaml` |
| `spawnEntity(name, overrides)` | `spawn_entity` → id `{index, generation}` |
| `getWorldAsJson()` | `world_snapshot`, as JSON (debugging, saves) |
| `writeFrame()` | `write_frame` → `Int32Array` ([Render boundary](#render-boundary)) |
| `metaDelta()` | `meta_delta` → JSON, or `undefined` when nothing changed |
| `submit(command)` | `submit` → sequence number |
| `schedule(tick, command)` | `schedule` → sequence number |
| `step()` | `step` → `{ tick, outcomes }` |
| `currentTick()` | `current_tick` |
| `stateHash()` | `state_hash` → 16 hex digits |
| `orderMoveTo(ids, x, y)` | `order_move_to` |
| `orderStop(ids)` | `order_stop` |
| `despawn(ids)` | `despawn` → count removed |
| `isAlive(id)` | `is_alive` |
| `createGroup(faction)` | `create_group` |
| `addToGroup(groupId, unitId)` | `add_to_group` |
| `removeFromGroup(unitId)` | `remove_from_group` |
| `groupOf(unitId)` | `group_of` |
| `groupMembers(groupId)` | `group_members` |
| `orderGroupMoveTo(groupId, x, y)` | `order_group_move_to` |
| `isGroupManual(groupId)` / `clearGroupManual(groupId)` | `is_group_manual` / `clear_group_manual` |
| `createMission(x, y, radius)` | `create_mission` |
| `assignGroup(missionId, groupId)` | `assign_group` |
| `missionOf(groupId)` | `mission_of` |
| `isMissionCompleted(missionId)` | `is_mission_completed` |
| `loadMapYaml(yaml)` | `load_map_yaml` → array of ids |
| `mapBounds()` | `map_bounds` → `{width, height}` or `null` |
| `hello()` | `hello` |
| `tickMs()` (module function) | `TICK_MS` |

Override objects use the same snake_case keys as YAML and export (`position`, `move_target`, etc.). Commands are the JSON objects from [Commands](#commands). An outcome in a step report is flat: `{ seq, ok: true, spawned | group | mission }` for a creation, `{ seq, ok: true, applied, skipped }` for anything else, `{ seq, ok: false, error }` when refused. The immediate order methods in the table remain for tools and tests; a host that wants lockstep or replays uses `submit`. See [`wasm-bindings/demo/run.mjs`](wasm-bindings/demo/run.mjs) for a full example.

## Browser demo (js-app)

An interactive RTS-style demo lives in [`js-app/`](js-app/): PixiJS canvas, marquee selection,
move orders for a group, minimap, pan and zoom. The simulation runs in a web worker; the main
thread only renders and handles input. Every order the demo gives goes to the core as a command
through `submit`, applies on the next tick, and its promise settles from the frame that ran it.

The world crosses from the worker as [position frames](#render-boundary), transferred rather than
copied, plus a metadata delta when something changed; the per-frame path creates no JSON. Units
are drawn as particles in one batched PixiJS `ParticleContainer`, interpolated between the last
two ticks. The Forces list shows the selection (at most 200 rows) and the size of the whole army;
the top bar shows fps, unit count and tick.

**Stress** spawns N seeded movers (`stress_mover`, 3 units/s) walking to random points across the
map — spawn commands like any other order. Each press uses the next seed.

```bash
cd js-app
npm run build:wasm   # builds the wasm package and copies it into js-app/public/
npm install
npm run dev
```

Press **G** or hit **Form group** to make a group out of the selection, then toggle **Group
orders**: a click on empty ground becomes a group order instead of a personal one. Give one unit a
personal order first and watch it keep its own course while the rest of the group turns — that is
the priority rule, visible.

The map starts with a truck parked beside one of the movers. Select the truck together with units
standing next to it and press **B** to load them; **U** lets them off again, and **S** stops
whatever is selected — worth doing to the truck before anyone steps out, since unboarding does not
halt it and a vehicle still under orders drives away from the unit it just dropped. Riders fade into the
truck and travel with it, and the second mover is deliberately parked out of reach, so it has to be
walked over before it can board — boarding is not a move order.

WASM rebuilds automatically when a `.rs` file under `open-entities-lib/` or `wasm-bindings/`
changes. The boundary between the WASM core and the visualization — message protocol, id packing,
input semantics — is documented in [`js-app/CORE-API.md`](js-app/CORE-API.md).

The demo owns its YAML under `js-app/public/fixtures/`; the library's own fixtures in `/fixtures`
serve the Node demo and the Rust tests.

## Examples

### Spawn from YAML (default)

Loads templates (with inheritance), spawns entities, prints pretty world JSON:

```bash
make example
```

Or:

```bash
cargo run -p open_entities --example spawn_entity
```

### Hello world

Prints a greeting to stdout:

```bash
cargo run -p open_entities --example hello
```

Expected output:

```
Hello, world!
```

### World JSON export

The full export, for debugging and saves (a renderer reads the [render boundary](#render-boundary)
instead). Minimal spawn + compact JSON export:

```bash
make example EXAMPLE=world_json
```

Or:

```bash
cargo run -p open_entities --example world_json
```

The library returns the snapshot as data; serialize it with any `serde` format:

```rust
use open_entities::{Api, components::Position};

let mut api = Api::new();
api.core_mut().world_mut().spawn(Position::from_units(1.0, 2.0));
let snapshot = api.world_snapshot();
let json = serde_json::to_string(&snapshot).expect("export world");
```

### Exported JSON (schema version 5)

`tick` is the simulation tick the snapshot was taken at. Every entity in the world appears in `entities`. Component keys are omitted when the entity does not have that component (not `null`).

```json
{
  "version": 5,
  "tick": 0,
  "entities": [
    {
      "id": { "index": 0, "generation": 0 },
      "position": { "x": 1.0, "y": 2.0 },
      "velocity": { "vx": 0.5, "vy": -0.5 },
      "base_move_speed": 2.0,
      "boardable": 4
    },
    {
      "id": { "index": 1, "generation": 0 },
      "faction": 2
    },
    {
      "id": { "index": 2, "generation": 0 },
      "health": { "current": 80, "max": 100 }
    },
    {
      "id": { "index": 3, "generation": 0 },
      "entity_type": "scout"
    }
  ]
}
```

## License

Dual-licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option. This is the usual arrangement in the Rust ecosystem: the MIT half keeps the terms
short, the Apache half adds an explicit patent grant.

Unless you state otherwise, any contribution you intentionally submit for inclusion in this work
shall be dual-licensed as above, without any additional terms or conditions.
