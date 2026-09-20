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
| `loadMapYaml(yaml)` | `load_map_yaml` → array of ids |
| `mapBounds()` | `map_bounds` → `{width, height}` or `null` |
| `hello()` | `hello` |

`tick(0)`, non-integer, NaN, or non-finite `dtMs` are rejected in JavaScript before Rust runs.

Override objects use the same snake_case keys as YAML and export (`position`, `move_target`, etc.). See [`wasm-bindings/demo/run.mjs`](wasm-bindings/demo/run.mjs) for a full example.

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
