# OpenEntities

Rust workspace with the core library crate `open_entities` in `open-entities-lib/`.

The library uses [Bevy ECS](https://crates.io/crates/bevy_ecs) (`bevy_ecs` only, not the full Bevy engine) for entity simulation. Public entry points:

- [`Api`](open-entities-lib/src/api.rs) — the facade: spawn, orders, lifecycle, import, export
- [`EntityId`](open-entities-lib/src/orders.rs) — how every call names an entity: an `{index, generation}` pair
- [`Core`](open-entities-lib/src/core.rs) — owns the ECS [`World`](https://docs.rs/bevy_ecs/latest/bevy_ecs/world/struct.World.html); reachable through `Api::core()` / `core_mut()`
- [`export`](open-entities-lib/src/export/mod.rs) — `Api::world_json()` serializes **every entity** in the world to JSON **schema version 3**; registered gameplay fields are omitted when absent (not `null`)
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

`Api::tick(dt_ms)` advances the ECS schedule: `seek_system` (entities with `MoveTarget` + `BaseMoveSpeed` + `Velocity`) then `movement_system` (all `Position` + `Velocity`). Delta is unsigned milliseconds; `0` returns `TickError::ZeroDeltaTime`; values above **100 ms** are clamped.

Arrival (distance ≤ 0.1): snap to target, remove `MoveTarget`, zero `Velocity`, skip movement that frame.

```rust
api.tick(16)?; // ~60 Hz step
```

## Move orders

`Api::order_move_to(ids, target)` gives a group of entities a destination. Ids are
[`EntityId`](open-entities-lib/src/orders.rs) — the `{index, generation}` pair `world_json` reports
as each entity's `id`, so a host can feed them straight back from a snapshot.

Entities without `Position` or `BaseMoveSpeed` are skipped: immobile things cannot take a move
order. With more than one id the destinations are spread over a grid around the point, so the group
does not pile onto one spot. The returned `OrderReport` says how many took the order.

```rust
api.order_move_to(&[id], MoveTarget { x: 20.0, y: 0.0 });
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
api.order_group_move_to(group, MoveTarget { x: 50.0, y: 50.0 })?;
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
behaviour; missions and the replanner are not implemented yet.

## Missions

A mission is a point somebody should reach, and how close counts as reached. Groups are sent to
it; the first arrival finishes it for everyone.

```rust
let mission = api.create_mission(MoveTarget { x: 80.0, y: 20.0 }, 2.0);
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

## Requirements

- Rust **1.85+** (edition 2024; `bevy_ecs 0.19` may require a newer toolchain — check `cargo build` if compile fails)

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
| `getWorldAsJson()` | `world_json` |
| `tick(dtMs)` | `tick` |
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

`tick(0)`, non-integer, NaN, or non-finite `dtMs` are rejected in JavaScript before Rust runs.

Override objects use the same snake_case keys as YAML and export (`position`, `move_target`, etc.). See [`wasm-bindings/demo/run.mjs`](wasm-bindings/demo/run.mjs) for a full example.

## Browser demo (js-app)

An interactive RTS-style demo lives in [`js-app/`](js-app/): PixiJS canvas, marquee selection,
move orders for a group, minimap, pan and zoom. The simulation runs in a web worker; the main
thread only renders and handles input.

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

Minimal spawn + compact JSON export:

```bash
make example EXAMPLE=world_json
```

Or:

```bash
cargo run -p open_entities --example world_json
```

Compact JSON is available from the library API:

```rust
use open_entities::{Api, components::Position};

let mut api = Api::new();
api.core_mut().world_mut().spawn(Position { x: 1.0, y: 2.0 });
let json = api.world_json().expect("export world");
```

### Exported JSON (schema version 3)

Every entity in the world appears in `entities`. Component keys are omitted when the entity does not have that component (not `null`).

```json
{
  "version": 3,
  "entities": [
    {
      "id": { "index": 0, "generation": 0 },
      "position": { "x": 1.0, "y": 2.0 },
      "velocity": { "vx": 0.5, "vy": -0.5 },
      "base_move_speed": 2.0
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
