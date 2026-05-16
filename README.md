# OpenEntities

Rust workspace with the core library crate `open_entities` in `open-entities-lib/`.

The library uses [Bevy ECS](https://crates.io/crates/bevy_ecs) (`bevy_ecs` only, not the full Bevy engine) for entity simulation. Public entry points:

- [`Core`](open-entities-lib/src/core.rs) — owns the ECS [`World`](https://docs.rs/bevy_ecs/latest/bevy_ecs/world/struct.World.html)
- [`Api`](open-entities-lib/src/api.rs) — facade over `Core` (spawn, systems, export)
- [`export`](open-entities-lib/src/export/mod.rs) — `Api::world_json()` serializes entities with a [`Position`](open-entities-lib/src/components/position.rs) component to JSON

Domain components live under `open_entities::components` (currently `Position { x, y }`).

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

## Examples

### Hello world

Prints a greeting to stdout:

```bash
make example
```

Or:

```bash
cargo run -p open_entities --example hello
```

Expected output:

```
Hello, world!
```

### World JSON export

Spawns sample entities and prints a pretty-printed JSON snapshot of the world:

```bash
make example-world-json
```

Or:

```bash
make example EXAMPLE=world_json
cargo run -p open_entities --example world_json
```

Compact JSON is available from the library API:

```rust
use open_entities::{Api, components::Position};

let mut api = Api::new();
api.core_mut().world_mut().spawn(Position { x: 1.0, y: 2.0 });
let json = api.world_json().expect("export world");
```

Exported shape (schema version `1`):

```json
{
  "version": 1,
  "entities": [
    {
      "id": { "index": 0, "generation": 0 },
      "position": { "x": 1.0, "y": 2.0 }
    }
  ]
}
```

Only entities that have a `Position` component are included.
