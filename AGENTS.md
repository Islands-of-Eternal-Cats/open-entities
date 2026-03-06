# OpenEntities - Agent Guide

## Project Overview

OpenEntities is a Rust-based Entity Component System (ECS) library built on Bevy ECS, with WebAssembly bindings for JavaScript integration. It provides a minimal framework for building ECS-based applications.

**Core Stack:**
- Language: Rust (edition 2021)
- ECS Framework: `bevy_ecs`
- WASM Target: `wasm32-unknown-unknown`
- JS Bundler: Vite 5

---

## Directory Structure

```
open-entities/
├── Cargo.toml                 # Workspace root - defines members
├── Makefile                   # Build commands (all targets)
├── README.md                  # User-facing documentation
├── AGENTS.md                  # This file - agent guidance
├── assets/                    # Data files
│   └── entities.yaml         # Example entity type definitions (YAML)
├── open-entities-lib/         # Core ECS library
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs            # Main lib with tests
│       ├── components/       # Component definitions
│       │   ├── mod.rs
│       │   ├── position.rs   # Position component
│       │   └── velocity.rs  # Velocity component
│       ├── entity_loader.rs  # YAML load + spawn by type name
│       └── systems.rs        # ECS systems
├── wasm-bindings/            # WebAssembly bindings
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs           # wasm-bindgen exports
└── js-app/                   # JavaScript demo app
    ├── package.json
    ├── vite.config.js
    └── src/
        └── main.js          # HTML + JS integration
```

---

## Essential Commands

### Build Commands

```bash
# Build in debug mode (default)
make           # or: make build
cargo build

# Build in release mode
make release
cargo build --release

# Build WebAssembly
make wasm
cargo build --target wasm32-unknown-unknown --release -p wasm-bindings
```

### Testing & Quality

```bash
# Run all tests
make test
cargo test

# Run Clippy linter
make clippy
cargo clippy --all-targets --all-features -- -D warnings

# Format code
make fmt
cargo fmt --all

# Check without building
make check
cargo check --all-targets --all-features

# Generate docs
make docs
cargo doc --no-deps --open

# CI check (all quality gates)
make ci
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all
cargo test
```

### JavaScript App Commands

```bash
# Development server
cd js-app && npm run dev

# Build for production
cd js-app && npm run build

# Preview production build
cd js-app && npm run preview
```

### Pre-requisites

- Rust and Cargo (`rustc`, `cargo`)
- WASM target: `rustup target add wasm32-unknown-unknown`
- Node.js 18+ (for js-app)
- wasm-pack (for building WASM from JS app)

---

## Code Organization

### Modules

| Module | Purpose |
|--------|---------|
| `open-entities-lib/src/components/` | ECS Component definitions (Position, Velocity) |
| `open-entities-lib/src/entity_loader.rs` | Load entity definitions from YAML, spawn by type name |
| `open-entities-lib/src/systems.rs` | ECS system functions and app setup |
| `wasm-bindings/src/lib.rs` | JavaScript-compatible wrappers via wasm-bindgen |

### Component Patterns

**Position Component** (`open-entities-lib/src/components/position.rs`)
- Basic 2D position with `x` and `y` fields
- Derives: `Component`, `Clone`, `Debug`
- No validation on values (assumes caller provides valid data)

**Velocity Component** (`open-entities-lib/src/components/velocity.rs`)
- 2D velocity with `vx` and `vy` fields
- Derives: `Component`, `Clone`, `Debug`
- No validation on values

### System Patterns

**ECS Systems** (`open-entities-lib/src/systems.rs`)

| System | Schedule | Purpose |
|--------|----------|---------|
| `setup_system` | `Startup` | Spawns initial entities |
| `move_system` | `Update` | Updates position based on velocity |
| `print_position_system` | `Update` | Logs entity positions |

**Key Patterns:**
- Systems use Bevy's `Query` for component access
- `move_system` uses mutable query: `Query<(&mut Position, &Velocity)>`
- `print_position_system` uses immutable query: `Query<&Position>`
- System functions are exported from the lib for external wiring

**App Setup:**
```rust
pub fn setup_app(app: &mut App) {
    app.add_systems(Startup, setup_system)
        .add_systems(Update, (move_system, print_position_system));
}
```

### Entity definitions (YAML)

Сущности можно загружать из YAML по имени типа.

**Формат** (`assets/entities.yaml`): корневой ключ `entities`, внутри — именованные типы; у каждого типа опционально `position` и `velocity`. Отсутствие поля = у сущности нет этого компонента.

```yaml
entities:
  mover:
    position: { x: 0.0, y: 0.0 }
    velocity: { vx: 1.0, vy: 2.0 }
  static_obstacle:
    position: { x: 10.0, y: 10.0 }
    # без velocity — только Position
```

**API:**
- `EntityDefinitions::load_from_path(path)` / `load_from_str(s)` — загрузка определений
- `spawn_entity_by_type(commands, &definitions, "mover")` — создать одну сущность по имени типа
- `load_and_spawn_all_from_path(commands, path)` — загрузить файл и заспавнить по одной сущности каждого типа
- `setup_app_with_yaml(app, path)` — инициализация приложения: при старте загружается YAML и заспавниваются все типы (без жёстко заданных сущностей в `setup_system`)

**Ресурс:** `EntityDefinitionsPath(Option<PathBuf>)` — при `Some(path)` стартовая система загружает YAML и спавнит сущности. `EntityDefinitions` можно положить в мир как ресурс и вызывать `spawn_entity_by_type` из своих систем.

---

## WASM Bindings

### Exported API (`wasm-bindings/src/lib.rs`)

| Export | Type | Description |
|--------|------|-------------|
| `wasm_init()` | Function | Initialize panic hook |
| `JsPosition` | Class | JavaScript wrapper for Position |
| `JsVelocity` | Class | JavaScript wrapper for Velocity |
| `move_position(pos, vel)` | Function | Calculate new position |

### JsPosition API
```javascript
new JsPosition(x: f32, y: f32)  // Constructor
pos.x() -> f32                   // Getter
pos.set_x(x: f32)               // Setter
pos.y() -> f32
pos.set_y(y: f32)
```

### JsVelocity API
```javascript
new JsVelocity(vx: f32, vy: f32)  // Constructor
vel.vx() -> f32                   // Getter
vel.set_vx(vx: f32)               // Setter
vel.vy() -> f32
vel.set_vy(vy: f32)
```

### JS Usage Pattern (`js-app/src/main.js`)

```javascript
import init, { JsPosition, JsVelocity, move_position } from "open-entities-wasm";

await init();  // Required before using WASM

// Create entities
const position = new JsPosition(x, y);
const velocity = new JsVelocity(vx, vy);

// Move entities
const newPos = move_position(position, velocity);
```

---

## Testing

### Library Tests (`open-entities-lib/src/lib.rs`)

Three tests provided:
1. `test_components_compile` - Verifies components instantiate
2. `test_spawn_entity_and_query` - Tests Spawner, Query, and component access
3. `test_entity_loader_from_str_and_spawn_by_type` - Loads definitions from YAML string, spawns by type name, runs move_system and checks components

### Running Tests

```bash
# All tests in workspace
cargo test

# Specific crate tests
cargo test -p open-entities-lib
cargo test -p wasm-bindings
```

### Test Patterns

- Uses `bevy_app::App` for ECS world setup
- Tests access world via `app.world_mut()` and `app.world()`
- Tests use `query.iter(&app.world()).collect()` for iteration

---

## Code Style & Conventions

### Rust Conventions

- Edition: 2021
- Indentation: 4 spaces (standard Rust fmt)
- Module structure: One file per component, `mod.rs` for module exports
- Documentation: Rustdoc comments on all public items
- Component derives: `Component`, `Clone`, `Debug` (consistent pattern)

### Naming Conventions

| Pattern | Examples |
|---------|----------|
| Components | `Position`, `Velocity` (PascalCase) |
| Systems | `move_system`, `print_position_system` (snake_case + _system) |
| JS Wrappers | `JsPosition`, `JsVelocity` (PascalCase with Js prefix) |
| JS Functions | `move_position` (snake_case) |
| Modules | `components`, `systems` (snake_case) |

### WASM-specific Patterns

- `#[wasm_bindgen]` attribute on exports
- `#[wasm_bindgen(start)]` for initialization function
- Wrapper structs hold inner Rust types
- Constructor exports as `#[wasm_bindgen(constructor)]`

---

## Build Workflow

### Complete Build Process

1. **Compile Rust workspace:**
   ```bash
   make build  # or: cargo build
   ```

2. **Build WASM bindings:**
   ```bash
   make wasm  # or: cargo build --target wasm32-unknown-unknown --release -p wasm-bindings
   ```

3. **Build JavaScript app:**
   ```bash
   cd js-app && npm run build:wasm  # Builds WASM first
   npm run build
   ```

### Build Artifacts

- Rust debug: `target/debug/`
- Rust release: `target/release/`
- WASM: `wasm-bindings/pkg/`
- JS app: `js-app/dist/`

---

## Gotchas & Pitfalls

### Non-Obvious Patterns

1. **Module import in WASM**: The `wasm-bindings/src/lib.rs` imports `open_entities` (from lib), not `open_entities_lib` (crate name)

2. **Component public fields**: Position and Velocity have public fields - no encapsulation

3. **Velocity setters**: `JsVelocity` uses `set_vx()` and `set_vy()` to match getters `vx()` and `vy()`

4. **System mutability**: `move_system` takes `&mut Position` to modify; `print_position_system` takes `&Position` read-only

5. **JS app self-initializes**: `js-app/src/main.js` calls `initWasm()` immediately on load - no explicit initialization needed from user

6. **No distance check**: The `move_system` moves every entity every frame without boundary checks

7. **YAML entity path**: `EntityDefinitions::load_from_path` and `load_and_spawn_all_from_path` resolve paths relative to the current working directory (e.g. run from repo root: `assets/entities.yaml`)

### Build Gotchas

1. **WASM requires target**: Must specify `--target wasm32-unknown-unknown` for WASM builds

2. **JS app dependency**: `js-app/package.json` references `"open-entities-wasm": "file:../wasm-bindings/pkg"` - requires WASM build first

3. **Clean removes target**: `make clean` removes `target/` and `wasm-bindings/pkg/` - build artifacts are ephemeral

4. **No Windows support noted**: Makefile uses Unix shell syntax (`rm -rf`)

---

## Adding New Components

1. Create `open-entities-lib/src/components/<name>.rs`:
   ```rust
   use bevy_ecs::prelude::Component;
   
   #[derive(Component, Clone, Debug)]
   pub struct <ComponentName> {
       pub field: Type,
   }
   ```

2. Export in `open-entities-lib/src/components/mod.rs`:
   ```rust
   pub mod <name>;
   pub use <name>::<ComponentName>;
   ```

3. Update `open-entities-lib/src/lib.rs` exports:
   ```rust
   pub use components::<ComponentName>;
   ```

### Adding New Systems

1. Add system function in `open-entities-lib/src/systems.rs`:
   ```rust
   pub fn <name>_system(mut query: Query<...>) {
       // implementation
   }
   ```

2. Add to `setup_app()` in same file:
   ```rust
   app.add_systems(Update, <name>_system)
   ```

---

## Related Files

| File | Key Content |
|------|-------------|
| `open-entities-lib/Cargo.toml` | bevy_ecs dependency |
| `wasm-bindings/Cargo.toml` | wasm-bindgen, wasm-bindgen-test |
| `js-app/vite.config.js` | Vite config (check contents) |
| `js-app/index.html` | HTML entry point |

---

## Debugging

### Common Issues

**WASM not loading:**
- Check `rustup target add wasm32-unknown-unknown`
- Verify `make wasm` completes without errors
- Check browser console for WASM fetch errors

**Query errors:**
- Ensure components are added to App via `App::init_resource()` or spawned
- Verify Query type signature matches component types

**JS imports fail:**
- Run `make wasm` or `npm run build:wasm` before `npm run dev`
- Check `package.json` dependency path is correct

### Logging

- Use `println!` in systems (output to console/terminal)
- WASM panics captured by `console_error_panic_hook`

---

## Project Context

- **ECS Pattern**: Data (Components) + Logic (Systems) separation
- **Target Use**: Educational example or lightweight ECS base
- **Platform**: Desktop (Rust) + Web (WASM + JS)
- **License**: MIT (per README)
- **Status**: Minimal prototype with Position/Velocity components
