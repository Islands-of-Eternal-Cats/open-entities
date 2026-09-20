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

- **Seek overshoot.** A unit whose step (`speed * dt`) passed the target flew by, turned around and
  oscillated forever without dropping its `MoveTarget` — at any speed above 12.5 units/s at a 16 ms
  tick, or 2 units/s at the 100 ms cap. A step that reaches the target now counts as arrival.
- Browser demo: `npm run dev` builds and copies the wasm the dev server serves, and `initWasm`
  reports a missing module instead of letting an unreadable validation error out of the loader.
- Browser demo: the HUD syncs once the canvas api is in hand, so the clear-selection button is no
  longer stuck hidden and disabled after startup.

