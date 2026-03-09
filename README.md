# OpenEntities

A library for working with entities using the **bevy_ecs** framework, with WebAssembly bindings.

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
├── js-app/              # JavaScript application (if any)
├── target/              # Build artifacts
├── Cargo.toml           # Workspace configuration
└── Makefile             # Build helpers
```

## Quick Start

### Prerequisites

- Rust and Cargo (`rustc`, `cargo`)
- `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`

### Building

```bash
# Build in debug mode
make

# Build in release mode
make release

# Build WebAssembly
make wasm
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

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
