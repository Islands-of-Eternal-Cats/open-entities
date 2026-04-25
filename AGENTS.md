# OpenEntities - Agent Guide

## Project Overview

OpenEntities is a Rust-based Entity Component System (ECS) library built on **bevy_ecs only** (no bevy_app). It uses `World` and `Schedule` directly. WebAssembly bindings provide TypeScript/JavaScript integration.

**Core Stack:**
- Language: Rust (edition 2024)
- ECS Framework: `bevy_ecs` (no `bevy_app`)
- WASM Target: `wasm32-unknown-unknown`
- Frontend: **TypeScript** + Vite 5
- WASM build: автоматическая сборка при `npm run dev` и при изменении файлов Rust (плагин `watch-rust-dirs` в `vite.config.js`)

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
│   ├── examples/
│   │   └── run_ecs.rs        # Example: run ECS one tick (native)
│   └── src/
│       ├── lib.rs            # Main lib with tests
│       ├── components/       # Component definitions
│       │   ├── mod.rs
│       │   ├── position.rs
│       │   ├── velocity.rs
│       │   ├── base_move_speed.rs
│       │   └── …
│       ├── entity_loader.rs  # YAML load + spawn by type name
│       ├── systems/          # ECS systems (seek, move, …)
│       └── world.rs          # World + schedule setup
├── wasm-bindings/            # WebAssembly bindings
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs           # wasm-bindgen exports
└── js-app/                   # TypeScript demo app (WASM core + visualization)
    ├── package.json
    ├── tsconfig.json
    ├── vite.config.js
    ├── CORE-API.md          # Contract: WASM ↔ TS (see for extension)
    └── src/
        ├── main.ts          # Entry: init, UI, game loop
        ├── core/            # WASM wrapper and app types
        │   ├── wasm-types.d.ts
        │   ├── wasm.ts
        │   └── types.ts
        └── visualization/
            ├── pixi-canvas.ts   # PixiJS canvas, selection, move orders
            ├── coords.ts        # Screen ↔ world coordinates
            ├── selection-logic.ts
            └── render.ts        # DOM entity list (alongside canvas)
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

### TypeScript App Commands (js-app)

```bash
# Development server (WASM собирается автоматически при старте и при изменении .rs)
cd js-app && npm run dev

# Type-check (TypeScript)
cd js-app && npm run typecheck

# Build for production (сначала соберите WASM: npm run build:wasm, затем npm run build)
cd js-app && npm run build

# Preview production build
cd js-app && npm run preview
```

js-app написан на **TypeScript**. WASM собирается автоматически: при запуске `npm run dev` и при изменении файлов в `open-entities-lib/` и `wasm-bindings/` (плагин `watch-rust-dirs` в `vite.config.js`). Отдельно запускать `make wasm` перед разработкой не обязательно. Граница между WASM-ядром и визуализацией описана в `js-app/CORE-API.md`.

### Pre-requisites

- Rust and Cargo (`rustc`, `cargo`) — Rust 1.85+ for edition 2024
- WASM target: `rustup target add wasm32-unknown-unknown`
- Node.js 18+ (для js-app)
- wasm-pack (для сборки WASM; используется скриптом `js-app/build-wasm.sh` при dev и при `npm run build:wasm`)

---

## Code Organization

### Modules

| Module | Purpose |
|--------|---------|
| `open-entities-lib/src/components/` | ECS components: `Position`, `Velocity`, `BaseMoveSpeed`, `MoveTarget`, `Faction`, `EntityTypeName`, … |
| `open-entities-lib/src/entity_loader.rs` | Load entity definitions from YAML, spawn by type name |
| `open-entities-lib/src/systems/` | ECS systems (`seek_move_target_system`, `move_system`, …) and schedule helpers |
| `wasm-bindings/src/lib.rs` | TypeScript/JavaScript wrappers via wasm-bindgen |

### Component Patterns

**Position Component** (`open-entities-lib/src/components/position.rs`)
- Basic 2D position with `x` and `y` fields
- Derives: `Component`, `Clone`, `Debug`
- No validation on values (assumes caller provides valid data)

**Velocity Component** (`open-entities-lib/src/components/velocity.rs`)
- 2D velocity with `vx` and `vy` fields
- Derives: `Component`, `Clone`, `Debug`
- No validation on values

**BaseMoveSpeed Component** (`open-entities-lib/src/components/base_move_speed.rs`)
- Scalar `f32`: базовая скорость движения (юнит/с), из YAML-поля `base_move_speed` > 0 для подвижных типов
- Derives: `Component`, `Clone`, `Copy`, `Debug`

### System Patterns

**ECS Systems** (`open-entities-lib/src/systems/`)

| System | When run | Purpose |
|--------|----------|---------|
| `load_entities_from_yaml_system` | Startup (only in `setup_world_with_yaml`) | Loads YAML and spawns one entity per type |
| `seek_move_target_system` | Update | Sets velocity toward `MoveTarget` using `BaseMoveSpeed` |
| `move_system` | Update | Updates position based on velocity (entities with `BaseMoveSpeed`) |
| `print_position_system` | Update | Logs entity positions |

**Key Patterns:**
- No `bevy_app::App`: the lib uses `World` and `Schedule` directly.
- Systems use Bevy's `Query` for component access.
- `seek_move_target_system`: steers toward `MoveTarget`; `move_system`: `Query<(&mut Position, &Velocity), With<BaseMoveSpeed>>`; `print_position_system`: `Query<&Position>`.
- System functions are exported for external wiring.

**World / Schedule setup:**
```rust
// Empty world (no entities); spawn via YAML-backed APIs or insert definitions + spawn by type
let (mut world, mut schedule) = setup_world();
schedule.run(&mut world);  // one tick

// One entity per type from YAML (load_entities_from_yaml_system runs at startup)
let (mut world, mut schedule) = setup_world_with_yaml("assets/entities.yaml");
schedule.run(&mut world);
```

### Entity definitions (YAML)

Сущности можно загружать из YAML по имени типа.

**Формат** (`assets/entities.yaml`): корневой ключ `entities`, внутри — именованные типы; у каждого типа опционально `position` и **`base_move_speed`**. Подвижность: только `base_move_speed` > 0 (юнит/с); иначе — статика без `Velocity`/`BaseMoveSpeed` в ECS.

```yaml
entities:
  mover:
    position: { x: 0.0, y: 0.0 }
    base_move_speed: 45.0
  static_obstacle:
    position: { x: 10.0, y: 10.0 }
```

**API:**
- `EntityDefinitions::load_from_path(path)` / `load_from_str(s)` — загрузка определений
- `spawn_entity_by_type(commands, &definitions, "mover", None)` — создать одну сущность по имени типа; последний аргумент — опциональная фракция `Option<u32>`
- `load_and_spawn_all_from_path(commands, path)` — загрузить файл и заспавнить по одной сущности каждого типа
- `setup_world_with_yaml(path)` — инициализация: возвращает `(World, Schedule)`; при первом запуске schedule стартовая система загружает YAML и спавнит по одной сущности каждого типа

**Ресурс:** `EntityDefinitionsPath(Option<PathBuf>)` — при `Some(path)` стартовая система `load_entities_from_yaml_system` загружает YAML и спавнит сущности. `EntityDefinitions` можно положить в мир как ресурс и вызывать `spawn_entity_by_type` из своих систем.

---

## WASM Bindings

### Exported API (`wasm-bindings/src/lib.rs`)

| Export | Type | Description |
|--------|------|-------------|
| `wasm_init()` | Function | Panic hook (`#[wasm_bindgen(start)]`; usually automatic) |
| `JsWorld` | Class | ECS world from YAML string: `tick(dt)`, `spawn` / `spawn_at`, `order_move_to`, `get_entities` |
| `JsPosition` | Class | Wrapper for `Position` (used by `order_move_to` and legacy helpers) |
| `JsVelocity` | Class | Wrapper for `Velocity` |
| `move_position(pos, vel)` | Function | One step without `dt` (legacy; prefer `JsWorld.tick`) |

### JsWorld API (primary for games)

```javascript
const world = new JsWorld(entitiesYamlString);
world.spawn("mover", optionalFactionU32);           // position from YAML
world.spawn_at("mover", x, y, optionalFactionU32); // position override
world.tick(dtSeconds);
world.order_move_to(["123", ...], new JsPosition(tx, ty));
const rows = Array.from(world.get_entities());    // { id, entityType, pos, velocity, faction }
```

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

### TypeScript Usage Pattern

**js-app (worker):** главный поток вызывает `initWasm()` из `core/wasm.ts` (см. `js-app/CORE-API.md`); симуляция в `ecs-worker.ts` через `JsWorld`.

**Прямой импорт пакета (без worker):**

```typescript
import init, { JsWorld } from "open-entities-wasm";

await init();
const world = new JsWorld(entitiesYaml);
world.spawn_at("mover", 10, 20);
world.tick(1 / 60);
```

**Legacy helpers:** `JsPosition`, `JsVelocity`, `move_position` — как раньше, для простых примеров без полного тика.

### Selection & move (target UX)

Intended behavior for the js-app canvas (selection + orders), including touch without keyboard modifiers:

| Situation | Action |
|-----------|--------|
| A group is selected and the user taps/clicks **empty ground** (no unit under the point) | **Move order**: selected units go to that world position. |
| **Esc** or a **UI control** shown while selection is non-empty | **Clear selection** only (no move). |

Rationale: one tap on empty space cannot mean both “deselect” and “move”; with an active selection, **move is the primary action**; deselect is explicit (keyboard or HUD). Aligns with common mobile RTS patterns.

**Implementation:** `js-app` — `onMoveOrder` in `pixi-canvas.ts` → `moveSelectedTo` in `core/wasm.ts` → worker `move_to` → `JsWorld::order_move_to`. ECS: `MoveTarget` + `seek_move_target_system` (chained before `move_system`).

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

- Uses `World` and `Schedule` for ECS (no App)
- Tests spawn entities and insert resources into `World`, then run `Schedule::run(&mut world)`
- Queries use `world.query::<...>()` and `query.iter(&world)`

---

## Code Style & Conventions

### Rust Conventions

- Edition: 2024
- Indentation: 4 spaces (standard Rust fmt)
- Module structure: One file per component, `mod.rs` for module exports
- Documentation: Rustdoc comments on all public items
- Component derives: `Component`, `Clone`, `Debug` (consistent pattern)

### Naming Conventions

| Pattern | Examples |
|---------|----------|
| Components | `Position`, `Velocity` (PascalCase) |
| Systems | `move_system`, `print_position_system` (snake_case + _system) |
| JS Wrappers | `JsWorld`, `JsPosition`, `JsVelocity` (PascalCase with Js prefix) |
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

2. **WASM bindings** — вручную при необходимости:
   ```bash
   make wasm  # or: cargo build --target wasm32-unknown-unknown --release -p wasm-bindings
   ```
   Для **js-app** WASM собирается автоматически при `npm run dev` (и при изменении `.rs`), отдельный шаг не нужен.

3. **TypeScript app (production):**
   ```bash
   cd js-app && npm run build:wasm  # Собрать WASM
   npm run build                   # Собрать приложение
   ```

### Build Artifacts

- Rust debug: `target/debug/`
- Rust release: `target/release/`
- WASM: `wasm-bindings/pkg/`
- JS app: `js-app/dist/`

---

## Gotchas & Pitfalls

### Non-Obvious Patterns

1. **No bevy_app**: The library uses only `bevy_ecs`. You get `(World, Schedule)` from `setup_world()` (empty) / `setup_world_with_yaml()` / `create_world_with_definitions()`, not an `App`. Run the schedule with `schedule.run(&mut world)` each tick. Spawned units should come from YAML definitions (by type), not ad-hoc spawns in library setup.

2. **Module import in WASM**: The `wasm-bindings/src/lib.rs` imports `open_entities` (from lib), not `open_entities_lib` (crate name)

3. **Component public fields**: Position and Velocity have public fields - no encapsulation

4. **Velocity setters**: `JsVelocity` uses `set_vx()` and `set_vy()` to match getters `vx()` and `vy()`

5. **System mutability**: `move_system` takes `&mut Position` to modify; `print_position_system` takes `&Position` read-only

6. **TS app self-initializes**: `js-app/src/main.ts` calls `initWasm()` immediately on load - no explicit initialization needed from user

7. **No distance check**: The `move_system` moves every entity every frame without boundary checks

8. **YAML entity path**: `EntityDefinitions::load_from_path` and `load_and_spawn_all_from_path` resolve paths relative to the current working directory (e.g. run from repo root: `assets/entities.yaml`)

### Build Gotchas

1. **WASM requires target**: Must specify `--target wasm32-unknown-unknown` for WASM builds

2. **js-app dependency**: `js-app/package.json` references `"open-entities-wasm": "file:../wasm-bindings/pkg"`. При `npm run dev` WASM собирается автоматически; для чистого production build выполните `npm run build:wasm` перед `npm run build`

3. **Clean removes target**: `make clean` removes `target/` and `wasm-bindings/pkg/` - build artifacts are ephemeral

4. **No Windows support noted**: Makefile uses Unix shell syntax (`rm -rf`)

### Обновление воркера и WASM без перезагрузки страницы (вариант B)

Предпочтительный способ: **главный поток сам качает WASM** (с обходом кэша), затем передаёт буфер воркеру.

- Главный поток: `fetch(wasmUrl, { cache: 'no-store' })` с cache-bust в URL (`?t=...` или `?v=...`), получает `ArrayBuffer`.
- Передача воркеру: `worker.postMessage({ type: 'init', wasmBuffer, entitiesYaml }, [wasmBuffer])` (transferable только буфер).
- Воркер вызывает `init(wasmBuffer)`, затем `new JsWorld(entitiesYaml)` и шлёт `ready`.

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

1. Add system function in `open-entities-lib/src/systems/` (например `movement.rs` или новый модуль):
   ```rust
   pub fn <name>_system(mut query: Query<...>) {
       // implementation
   }
   ```

2. Register in the relevant update schedule in `world.rs` (e.g. `setup_world`, `setup_world_with_yaml`, `create_empty_world`, `create_world_with_definitions` — keep them consistent where they share the same tick):
   ```rust
   update.add_systems((move_system, print_position_system, <name>_system));
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
- Ensure components are spawned into `World` or resources are inserted with `world.insert_resource()`
- Verify Query type signature matches component types

**WASM/TS imports fail:**
- При `npm run dev` WASM должен собираться автоматически; при ошибках проверьте наличие wasm-pack и `rustup target add wasm32-unknown-unknown`
- Или соберите WASM вручную: `cd js-app && npm run build:wasm`, затем снова `npm run dev`
- Проверьте путь зависимости в `package.json`

### Logging

- Use `println!` in systems (output to console/terminal)
- WASM panics captured by `console_error_panic_hook`

---

## Project Context

- **ECS Pattern**: Data (Components) + Logic (Systems) separation
- **Target Use**: Educational example or lightweight ECS base
- **Platform**: Desktop (Rust) + Web (WASM + TypeScript)
- **License**: MIT (per README)
- **Status**: ECS prototype with YAML-driven types, move orders, WASM worker integration, PixiJS canvas
