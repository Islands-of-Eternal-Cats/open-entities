# OpenEntities

A library for working with entities using the **bevy_ecs** framework, with WebAssembly bindings and a **TypeScript** demo app (Vite). WASM собирается автоматически при запуске dev-сервера и при изменении кода на Rust.

## Features

- Entity-Component-System (ECS) architecture based on `bevy_ecs`
- Components: `Position`, `Velocity`
- Systems: `move_system`, `print_position_system`
- WebAssembly support via `wasm-bindings`

## Project Structure

```
open-entities/
├── open-entities-lib/    # Core ECS library
├── wasm-bindings/        # WebAssembly bindings
├── js-app/              # TypeScript demo app (Vite, auto WASM build)
├── target/              # Build artifacts
├── Cargo.toml           # Workspace configuration
└── Makefile             # Build helpers
```

## Quick Start

### Prerequisites

- Rust and Cargo (`rustc`, `cargo`)
- `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- Для js-app: Node.js 18+, wasm-pack

### Building

```bash
# Build in debug mode
make

# Build in release mode
make release

# Build WebAssembly (вручную; для js-app при npm run dev собирается автоматически)
make wasm
```

### TypeScript app (js-app)

Фронтенд на **TypeScript** (Vite). WASM собирается автоматически при `npm run dev` и при изменении файлов в `open-entities-lib/` и `wasm-bindings/`.

```bash
cd js-app && npm run dev      # Dev-сервер с автосборкой WASM
cd js-app && npm run typecheck
cd js-app && npm run build:wasm && npm run build   # Production build
```

### Running Tests

```bash
make test
```

### Code Quality

```bash
# Run Clippy linter
make clippy

# Format code
make fmt

# Check without building
make check
```

### Documentation

```bash
# Generate and open docs
make docs
```

## Usage Example

The library uses `bevy_ecs` only (no `bevy_app`). You get a `World` and `Schedule` and run the schedule each tick:

```rust
use open_entities::setup_world;

fn main() {
    let (mut world, mut schedule) = setup_world();
    schedule.run(&mut world); // one tick
}
```

To load entities from a YAML file instead of hardcoded ones:

```rust
use open_entities::setup_world_with_yaml;

fn main() {
    let (mut world, mut schedule) = setup_world_with_yaml("assets/entities.yaml");
    schedule.run(&mut world);
}
```

### YAML Entity Definitions

YAML root key must be `entities`. Each key inside `entities` is a type name.
`position` and `velocity` are optional, so different types can have different component sets.

```yaml
entities:
  mover:
    position: { x: 0.0, y: 0.0 }
    velocity: { vx: 1.0, vy: 2.0 }
  static_obstacle:
    position: { x: 10.0, y: 10.0 }
```

### Spawn by Type Name

Rust API:

```rust
use open_entities::{EntityDefinitions, spawn_entity_by_type_in_world, SpawnError, World};

fn spawn_example(world: &mut World) -> Result<(), SpawnError> {
    // Assume EntityDefinitions resource was inserted earlier.
    let _entity = spawn_entity_by_type_in_world(world, "mover")?;
    Ok(())
}
```

WASM/TypeScript API (through `JsWorld`):

```typescript
const world = new JsWorld(entitiesYaml);
await world.spawn("mover"); // throws JS Error if type is unknown
```

### Common YAML/Spawn Errors

- `YAML parse error during load_from_str`: invalid YAML syntax or missing required `entities` root.
- `IO error during load_from_path`: bad path or read permissions issue for YAML file.
- `spawn failed: Unknown entity type: '...'`: requested type name is not in `entities`.
- `spawn failed: Entity definitions resource is not loaded`: world does not contain `EntityDefinitions` resource.

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
