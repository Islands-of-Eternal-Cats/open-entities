# Changelog

All notable changes to this project are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Nothing is published yet: the crates carry `publish = false` and the demo package is private. This
section becomes 0.1.0 on the day that changes.

One facade, one way to name an entity, and a browser demo that exercises the whole path from YAML
to a click on the canvas.

### Changed

- **Breaking. Commands.** `Api::step()` returns a `StepReport { tick, outcomes }` instead of `()`:
  it first applies the commands due at the tick it produces, then runs the systems. In JavaScript,
  `step()` returns `{ tick, outcomes }`. The demo worker sends every order as a command through
  `submit`: the `spawn_at`, `move_to`, `stop`, `create_group`, `add_to_group`, `group_move_to`,
  `board` and `unboard` worker messages are replaced by one `submit` message, and the `spawned` and
  `id` replies by `submitted`. Order replies no longer carry a snapshot; their promises
  (`moveSelectedTo`, `boardUnits`, `spawnAt`, `createGroupWith`, …) settle from the outcome in the
  frame that applied them — `spawnAt` with the new entity from that frame, the orders with the
  command's outcome, and a refusal as a rejection with the core's reason. Positions no longer jump
  for a frame when an order reply lands between frames.
- **Breaking. Ties break by `EntityId`.** Grid slots for group and mission orders, the last seat
  when several units reach a vehicle on the same tick, and the order missions are closed in follow
  `EntityId` (index, then generation) instead of query order; the replanner's tie-break compares
  the generation too. `group_members`, `passengers`, `approaching` and `mission_assignees` return
  ids in that order. `EntityId` implements `Ord`.
- **Breaking.** `ArrivedThisTick` holds a `BTreeSet<Entity>` instead of a `HashSet<Entity>`.
- **Breaking.** `Core::new()` builds the schedule at once instead of on the first step. Building
  creates a resource, and in `bevy_ecs` 0.19 resources are entities, so the lazy build took an
  entity index in the middle of a match; every entity spawned after `Api::new()` now gets an index
  one lower than before, and ids no longer depend on whether the first step has run.
- Rust is pinned to **1.99.0** in `rust-toolchain.toml`, with `rustfmt`, `clippy` and the wasm
  target; CI installs exactly that toolchain. Tests that checked a collection with
  `assert!(….is_empty())` use `assert_eq!(…, [])`, as clippy 1.99 asks.
- **Breaking. Integer simulation space.** Simulation state is integer milli-units
  (`MILLI_PER_UNIT` = 1000): `Position`, `MoveTarget` and `Mission::radius` are `i32` milli-units,
  `Velocity` and `BaseMoveSpeed` are `i32` milli-units per tick, and `BOARDING_RANGE` is `i32`.
  `Api::create_mission` takes the radius as `i32` milli-units. `ARRIVAL_THRESHOLD` (0.1) becomes
  `ARRIVAL_RADIUS` (100) and `TICK_SECS` is removed. Seek, arrival, range checks, the group grid
  and the replanner compute in integers with an integer square root, so results no longer depend
  on platform float behaviour. YAML, spawn overrides, the JSON export and the JS API keep map units
  as decimals, converted in the new `units` module; the export schema stays **5**. Speeds are
  quantised to 1 milli-unit per tick (0.02 units/s at `TICK_MS` = 50): `0.51` runs and exports as
  `0.52`. A value beyond `i32` milli-units is an import error.
- **Breaking. Fixed timestep.** The simulation advances only in whole ticks of
  `TICK_MS` = 50 ms: `Api::step()` runs one tick and `Api::current_tick()` counts them.
  `Api::tick(dt_ms)`, `MAX_DT_MS`, `TickError` and the `SimDelta` resource are removed; systems
  derive per-tick movement from `TICK_MS`, so the world no longer depends on the host's frame rate.
  The world snapshot carries `tick` and its schema version is now **5**. In JavaScript,
  `Simulation.tick(dtMs)` is replaced by `step()` and `currentTick()`, plus the module function
  `tickMs()`. The demo worker accumulates real frame time, runs at most a few ticks per frame and
  replies to every frame with the current tick, the positions of the tick before and `alpha`; the
  renderer interpolates between them.
- **Boarding is an order, not a range check.** `order_board(units, vehicle)` marks units with
  `BoardingTarget`; the new `boarding_approach_system` walks them to the vehicle — following it
  if it drives off — and boards them once within `BOARDING_RANGE`. A unit that arrives to find
  no seat stays outside with the order dropped; stop and a new move order cancel it. `board`
  itself no longer refuses on distance (`BoardError::TooFarAway` is gone): it is the primitive
  underneath. `boarding_target_of` and `approaching` read the order back, and the demo's **B**
  now sends units over instead of refusing the far ones.
- `BoardError::TooFarAway` carries the measured distance, so a refusal says whether to walk the
  unit over or to stop the vehicle first. `BoardError` is no longer `Eq`, since a distance is a
  float.
- **Breaking.** The export is **schema version 4**: registering `Boardable` adds a `boardable`
  field to entities that have seats.
- **Breaking.** `spawn_entity` and `load_map_yaml` return `EntityId` instead of a bevy `Entity`,
  and the crate root no longer re-exports `Component`, `Entity`, `Query` and `World`. No
  `bevy_ecs` type appears in the normal path, so the ECS version is not part of this library's
  public contract. `Api::core()` / `core_mut()` remain the documented escape hatch.
- **Breaking.** In JavaScript, `spawnEntity` returns a plain `{index, generation}` object — the
  same shape the export reports and every other call accepts. The `SpawnedEntity` class is gone.
- Licensed as **MIT OR Apache-2.0** (was GPL-3.0-or-later), so the library can be used from
  closed-source projects, including the author's own.
- `clippy::nursery` dropped from the crate lints; `pedantic` stays and CI denies warnings.

### Added

- **Commands.** `Command` is every order as data — `Spawn`, `Despawn`, `MoveTo`, `Stop`,
  `CreateGroup`, `AddToGroup`, `RemoveFromGroup`, `GroupMoveTo`, `ClearGroupManual`,
  `CreateMission`, `AssignGroup`, `UnassignGroup`, `Board`, `Unboard` — serialized as JSON tagged
  by `type` in `snake_case`, with points and radii in map units. `Api::submit(command)` queues it
  for the next tick and `Api::schedule(tick, command)` for any later one (`ScheduleError` for a
  tick that is not in the future); both return a `CommandSeq`, and commands of one tick apply in
  that order. Each applied command reports a `CommandOutcome`: the new id (`Spawned`,
  `GroupCreated`, `MissionCreated`), `Applied { applied, skipped }`, or a `CommandError`. The
  immediate `Api` methods stay for tools and tests. JavaScript: `submit`, `schedule`.
- **Replays.** `Replay { version: 1, templates_yaml, map_yaml, seed, commands }`, stored as JSON
  (`from_json`, `to_json`); `Replay::run(until_tick)` rebuilds the match in a fresh `Api`. `seed`
  is reserved for the RNG resource that arrives with the first system needing randomness.
- **State hash.** `Api::state_hash()`: FNV-1a 64 over the tick and every entity in id order, each
  simulation component as a tag byte plus little-endian fields, internal relations and markers
  included, per-tick scratch excluded. Components implement the new `StateHash` trait; the
  registry macro emits the calls for registered ones. JavaScript: `stateHash()`, 16 hex digits.
- **Golden replay.** `fixtures/replays/basic.json` — 600 ticks of moves, stops, groups, missions
  with the replanner, boarding and a despawn — with its hash in `fixtures/replays/basic.hash`.
  CI runs it on `ubuntu-24.04`, `windows-2025` and `macos-15`, and under wasm32 in Node as part of
  `make wasm-check`. `UPDATE_GOLDEN=1` rewrites the hash.
- **Determinism gates.** `clippy.toml` disallows `std::collections::HashMap` and `HashSet`; debug
  builds — every test — build the simulation schedule with ambiguity detection set to error.
  `commands.rs`, `replay.rs` and `state_hash.rs` join the float check.
- `units::distance`, the serde form of a bare distance: map units outside, milli-units inside.

- **`units` module.** `to_milli`, `from_milli`, `speed_to_per_tick`, `speed_from_per_tick` and
  the constructors `Position::from_units` / `MoveTarget::from_units` for host code.
- **Float gates.** `#![deny(clippy::float_arithmetic)]` in every simulation module, and
  `scripts/check-no-floats.sh` (`make float-check`, run in CI) fails if `f32` or `f64` appears in
  one.
- **Transport in the browser demo.** `board`, `unboard`, `vehicleOf`, `passengers` and `freeSeats`
  are exposed through `Simulation`, the snapshot carries `seats` and `aboard`, and the demo starts
  with a truck parked beside one mover and out of reach of the other. **B** loads the selected
  units, **U** lets them off, and a refusal — too far away, no seats left — is shown in the HUD
  instead of the console, because that refusal is the rule worth seeing.
- **Core build fingerprint.** The status line prints an FNV-1a of the wasm bytes the browser
  actually loaded, plus its size. Same fingerprint after a rebuild means nothing rebuilt — which
  is the difference between a fix that does not work and a fix that is not in what you are
  running, and that difference cost an evening.
- **The demo only offers mobile things as cargo.** Board reads the selection for units with a
  velocity, which in this engine is what being mobile means. The core stays permissive — it asks a
  passenger only for a position, and whether a self-propelled gun is freight is a question for a
  game rather than for an ECS — but the demo no longer offers to load the player's base.
- **Move targets in the HUD.** Each row in Forces prints `→ (x, y)` when the entity holds an order.
  A target is the one piece of unit state with no appearance on the map, so a unit standing on a
  stale one looked exactly like a unit standing still.
- **Stop order in the demo.** **S** (or the Stop button) halts the selection through the
  `orderStop` that was already in the core. Unboarding does not stop a vehicle, so without this
  a truck under orders drives away from the units it just dropped and they cannot climb back in.
- **Carrying units.** `Api::board`, `unboard`, `passengers`, `vehicle_of`, `free_seats`, plus a
  `boardable: <seats>` template field. A passenger's position belongs to its vehicle: the movement
  systems skip passengers and a sync system copies the vehicle's position onto them after it moves,
  so an order given to a passenger does nothing instead of fighting for where the unit is. Losing
  the vehicle lets its passengers go.
- **Groups and missions in JavaScript.** The whole group and mission surface is exposed through
  `Simulation`, and the browser demo can form a group from the selection (**G**) and switch its
  clicks between personal and group orders, which makes the priority rule visible on screen.
- **Replanner.** When a mission closes, the groups it released take the nearest open mission, or
  stand idle when there is none. Only groups whose mission ended under them are planned for, and a
  group under manual control or with no members left is skipped.
- **Missions.** `Api::create_mission`, `assign_group`, `unassign_group`, `mission_of`,
  `mission_assignees`, `is_mission_completed`, plus two systems in the tick. Assigned groups are
  steered toward the mission; the first live member of any assignee to reach the radius closes it
  for everyone, releases every assignee and stops the units that were following mission steering.
  A manual order or an empty roster takes a group off its mission.
- **Groups.** `Api::create_group`, `add_to_group`, `remove_from_group`, `group_of`,
  `group_members`, `order_group_move_to`, `is_group_manual`, `clear_group_manual`. A unit belongs
  to at most one group, a group commands one faction, and an empty group stays alive. A group
  order marks the group manually controlled, and only an explicit call hands it back to
  automation.
- **Order priority.** Orders carry an `OrderSource` — `MissionSteering < GroupSteering <
  PlayerUnit` — and a weaker source never overwrites a stronger one, so a unit pulled out of its
  group's advance by hand stays pulled out. The claim is released on arrival or on stop.
- **Move orders.** `Api::order_move_to(ids, target)` sends a group to a world point, spreading
  destinations over a `ceil(sqrt(n))` grid so the group does not pile onto one spot.
  `Api::order_stop(ids)` is the counterpart: it zeroes velocity and drops the target, and does not
  require a `BaseMoveSpeed`, so a drifting entity can always be stopped.
- **Entity lifecycle.** `Api::despawn(ids)` returns how many entities were actually removed;
  `Api::is_alive(id)` says whether an id still resolves. A reused index never revives an old id.
- **Map loading.** `Api::load_map_yaml(yaml)` spawns a starting layout and records its bounds
  (`Api::map_bounds()`). Template names are validated before anything spawns, so a typo leaves the
  world untouched rather than half-populated.
- **Browser demo** under `js-app/`: PixiJS canvas, marquee selection, group move orders, minimap,
  pan and zoom, with the simulation in a web worker.
- **CI**: `cargo test`, `fmt + clippy`, `wasm-check`, and js-app typecheck plus vitest on every
  push and pull request.

### Fixed

- **A move order stuck to a passenger.** Ordering a unit that was riding a vehicle stored a target
  the movement systems could not see; stepping off handed it back to `seek_system`, and the unit
  walked off to a destination given while it was cargo. Selecting a truck with its passengers and
  clicking the ground was enough — the rider got a grid slot 5 units from the truck's and walked
  there the moment it was let out, too far to climb back in. Passengers are now refused a move
  order outright, in `can_take_a_move_order` and in mission steering.
- **CI ran on a deprecated runtime, and installed whatever npm felt like.**
  `actions/checkout` and `actions/setup-node` moved to `v5`, the first major of each on node24,
  and `jetli/wasm-pack-action` — pinned to node16 and unmaintained — gave way to
  `taiki-e/install-action`, which is composite and brings no node runtime at all.
  `Swatinem/rust-cache@v2` already resolves to a node24 build. The js-app job also installs with
  `npm ci` now, from the committed lockfile, instead of re-resolving every `^` range per run.
- **The dev server ignored patches.** `watch-rust-dirs` rebuilt the wasm on `change` only. Git
  writes a temp file and renames it over the target, so `git am` and branch switches arrive as
  `unlink` + `add` and never triggered a rebuild: the browser kept running the previous core while
  the sources on disk said otherwise. It now listens for all three events.
- **Seek overshoot.** A unit whose step (`speed * dt`) passed the target flew by, turned around and
  oscillated forever without dropping its `MoveTarget` — at any speed above 12.5 units/s at a 16 ms
  tick, or 2 units/s at the 100 ms cap. A step that reaches the target now counts as arrival.
- Browser demo: `npm run dev` builds and copies the wasm the dev server serves, and `initWasm`
  reports a missing module instead of letting an unreadable validation error out of the loader.
- Browser demo: the HUD syncs once the canvas api is in hand, so the clear-selection button is no
  longer stuck hidden and disabled after startup.

