# OpenEntities

Rust workspace with the core library crate `open_entities` in `open-entities-lib/`.

The library uses [Bevy ECS](https://crates.io/crates/bevy_ecs) (`bevy_ecs` only, not the full Bevy engine) for entity simulation. Public entry points:

- [`Core`](open-entities-lib/src/core.rs) — owns the ECS [`World`](https://docs.rs/bevy_ecs/latest/bevy_ecs/world/struct.World.html)
- [`Api`](open-entities-lib/src/api.rs) — facade over `Core` (spawn, import, export)
- [`export`](open-entities-lib/src/export/mod.rs) — `Api::world_json()` serializes **every entity** in the world to JSON **schema version 3**; registered gameplay fields are omitted when absent (not `null`)
- [`EntityComponents`](open-entities-lib/src/entity_components.rs) — shared struct for YAML templates, `spawn_entity` overrides, and flattened export rows

Domain components live under `open_entities::components`: `Position`, `Velocity`, `Faction`, `MoveTarget`, `BaseMoveSpeed`, and `Health`.

## Simulation tick

`Api::tick(dt_ms)` advances the ECS schedule: `seek_system` (entities with `MoveTarget` + `BaseMoveSpeed` + `Velocity`) then `movement_system` (all `Position` + `Velocity`). Delta is unsigned milliseconds; `0` returns `TickError::ZeroDeltaTime`; values above **100 ms** are clamped.

Arrival (distance ≤ 0.1): snap to target, remove `MoveTarget`, zero `Velocity`, skip movement that frame.

```rust
api.tick(16)?; // ~60 Hz step
```

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

- Rust **1.85+** (edition 2024; `bevy_ecs 0.18` may require a newer toolchain — check `cargo build` if compile fails)

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
| `spawnEntity(name, overrides)` | `spawn_entity` → `SpawnedEntity` |
| `getWorldAsJson()` | `world_json` |
| `tick(dtMs)` | `tick` |
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
