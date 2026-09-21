# Changelog

All notable changes to this project are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Nothing is published yet: the crates carry `publish = false` and the demo package is private. This
section becomes 0.1.0 on the day that changes.

One facade, one way to name an entity, and a browser demo that exercises the whole path from YAML
to a click on the canvas.

### Added

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

### Changed

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

