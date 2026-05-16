# Core, Api, and World JSON Export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Introduce `Core` (owns `World`), `Api` (public facade), and `export::world_json` that returns compact JSON (schema version 1) for every entity with a `Position` component.

**Architecture:** `Api` wraps `Core` which owns `bevy_ecs::World`. Export logic lives in `export/mod.rs` as `impl Api::world_json(&mut self)` because Bevy 0.18 queries need `&mut World`. Private `WorldExport` / `EntityExport` structs serialize via `serde_json`; `Position` derives `Serialize` directly (no duplicate DTO type). Entity IDs export as `{ index, generation }`.

**Tech Stack:** Rust 2024, `bevy_ecs 0.18`, `serde` / `serde_json` (workspace deps).

**Spec:** `docs/superpowers/specs/2026-05-16-core-api-world-export-design.md`

**Prerequisite:** Complete `docs/superpowers/plans/2026-05-16-bevy-ecs-position.md` first (`Position`, ECS re-exports, `hello()` unchanged). Do **not** implement RTS components or schema v2 here — that is `docs/superpowers/plans/2026-05-16-rts-components-export-v2.md`.

---

## File map

| File | Responsibility |
|------|----------------|
| `Cargo.toml` | Workspace `serde`, `serde_json` |
| `open-entities-lib/Cargo.toml` | Crate deps on workspace serde crates |
| `open-entities-lib/src/core.rs` | `Core { world: World }`, `new`, `world`, `world_mut` |
| `open-entities-lib/src/api.rs` | `Api { core: Core }`, `new`, `core_mut` |
| `open-entities-lib/src/export/mod.rs` | `ExportError`, `Api::world_json`, export unit tests |
| `open-entities-lib/src/components/position.rs` | Add `Serialize` to `Position` |
| `open-entities-lib/src/lib.rs` | Wire modules; re-export `Api`, `Core`, `ExportError` |
| `open-entities-lib/examples/world_json.rs` | Spawn positions, pretty-print JSON to stdout |
| `Makefile` | `example` → `hello`; `example-world-json` → `world_json` |

**Unchanged:** `open-entities-lib/examples/hello.rs`, `components/mod.rs` layout, existing `hello()` test.

---

### Task 1: Serde workspace dependencies

**Files:**
- Modify: `Cargo.toml`
- Modify: `open-entities-lib/Cargo.toml`

- [ ] **Step 1: Add workspace serde dependencies**

Append to repo root `Cargo.toml` under `[workspace.dependencies]` (after `bevy_ecs`):

```toml
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

Full `[workspace.dependencies]` section:

```toml
[workspace.dependencies]
bevy_ecs = "0.18"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [ ] **Step 2: Add crate dependencies**

Append to `open-entities-lib/Cargo.toml`:

```toml
serde = { workspace = true }
serde_json = { workspace = true }
```

Full `[dependencies]`:

```toml
[dependencies]
bevy_ecs = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
```

- [ ] **Step 3: Verify manifests resolve**

Run (from repo root):

```bash
cargo check -p open_entities
```

Expected: compiles (no new API yet).

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml open-entities-lib/Cargo.toml Cargo.lock
git commit -m "chore: add serde workspace deps for world JSON export"
```

---

### Task 2: `Core` — world ownership

**Files:**
- Create: `open-entities-lib/src/core.rs`

- [ ] **Step 1: Create `core.rs`**

Create `open-entities-lib/src/core.rs`:

```rust
use bevy_ecs::prelude::World;

/// Owns the ECS [`World`] for a simulation instance.
pub struct Core {
    world: World,
}

impl Core {
    /// Creates an empty world.
    #[must_use]
    pub fn new() -> Self {
        Self {
            world: World::new(),
        }
    }

    /// Immutable access to the underlying ECS world.
    #[must_use]
    pub const fn world(&self) -> &World {
        &self.world
    }

    /// Mutable access to the underlying ECS world.
    pub const fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }
}

impl Default for Core {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 2: Wire module in `lib.rs` (temporary — full re-exports in Task 6)**

Add to `open-entities-lib/src/lib.rs` after existing `pub mod components;`:

```rust
pub mod core;
```

- [ ] **Step 3: Verify compile**

Run:

```bash
cargo check -p open_entities
```

Expected: success.

- [ ] **Step 4: Commit**

```bash
git add open-entities-lib/src/core.rs open-entities-lib/src/lib.rs
git commit -m "feat: add Core owning ECS World"
```

---

### Task 3: `Api` facade

**Files:**
- Create: `open-entities-lib/src/api.rs`
- Modify: `open-entities-lib/src/lib.rs`

- [ ] **Step 1: Create `api.rs`**

Create `open-entities-lib/src/api.rs`:

```rust
use crate::core::Core;

/// Public facade over [`Core`] for simulation operations and export.
pub struct Api {
    core: Core,
}

impl Api {
    /// Creates an API backed by a new empty [`Core`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            core: Core::new(),
        }
    }

    /// Mutable access to the underlying core.
    pub const fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }
}

impl Default for Api {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 2: Wire module**

Add to `open-entities-lib/src/lib.rs`:

```rust
pub mod api;
```

- [ ] **Step 3: Verify compile**

Run:

```bash
cargo check -p open_entities
```

Expected: success.

- [ ] **Step 4: Commit**

```bash
git add open-entities-lib/src/api.rs open-entities-lib/src/lib.rs
git commit -m "feat: add Api facade over Core"
```

---

### Task 4: `Serialize` on `Position`

**Files:**
- Modify: `open-entities-lib/src/components/position.rs`

- [ ] **Step 1: Add serde import and derive**

Replace the top of `position.rs` so `Position` derives `Serialize`:

```rust
use bevy_ecs::prelude::Component;
use serde::Serialize;

/// 2D position in world/simulation space.
#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}
```

Leave the existing `position_component_round_trip` test unchanged.

- [ ] **Step 2: Run position tests**

Run:

```bash
cargo test -p open_entities position_component_round_trip
```

Expected: `test result: ok. 1 passed`

- [ ] **Step 3: Commit**

```bash
git add open-entities-lib/src/components/position.rs
git commit -m "feat: derive Serialize on Position for JSON export"
```

---

### Task 5: Export module and `world_json` (TDD)

**Files:**
- Create: `open-entities-lib/src/export/mod.rs`
- Modify: `open-entities-lib/src/lib.rs`

- [ ] **Step 1: Write failing export tests**

Create `open-entities-lib/src/export/mod.rs` with types, stub `world_json`, and tests only:

```rust
use bevy_ecs::prelude::{Entity, World};
use serde::Serialize;

use crate::api::Api;
use crate::components::Position;

const SCHEMA_VERSION: u32 = 1;

/// Errors while serializing a world snapshot to JSON.
#[derive(Debug)]
pub enum ExportError {
    /// JSON serialization failed.
    Serde(serde_json::Error),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Serde(err) => write!(f, "JSON export failed: {err}"),
        }
    }
}

impl std::error::Error for ExportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Serde(err) => Some(err),
        }
    }
}

impl From<serde_json::Error> for ExportError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serde(err)
    }
}

#[derive(Serialize)]
struct WorldExport {
    version: u32,
    entities: Vec<EntityExport>,
}

#[derive(Serialize)]
struct EntityExport {
    id: EntityIdExport,
    position: Position,
}

#[derive(Serialize)]
struct EntityIdExport {
    index: u32,
    generation: u32,
}

impl Api {
    /// Serializes every entity with a [`Position`] component to compact JSON (schema version 1).
    pub fn world_json(&mut self) -> Result<String, ExportError> {
        world_json_from_world(self.core_mut().world_mut())
    }
}

fn world_json_from_world(_world: &mut World) -> Result<String, ExportError> {
    todo!("implement in Step 3")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::Position;

    #[test]
    fn world_json_empty_world() {
        let mut api = Api::new();
        let json = api.world_json().expect("serialize empty world");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("exported JSON should parse");
        assert_eq!(value["version"], 1);
        assert_eq!(value["entities"].as_array().map(Vec::len), Some(0));
    }

    #[test]
    fn world_json_includes_positioned_entities() {
        let mut api = Api::new();
        api.core_mut()
            .world_mut()
            .spawn(Position { x: 1.0, y: 2.0 });

        let json = api.world_json().expect("serialize world");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("exported JSON should parse");

        assert_eq!(value["version"], 1);
        let entities = value["entities"].as_array().expect("entities array");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0]["position"]["x"], 1.0);
        assert_eq!(entities[0]["position"]["y"], 2.0);
        assert!(entities[0]["id"]["index"].is_number());
        assert!(entities[0]["id"]["generation"].is_number());
    }
}
```

Add to `open-entities-lib/src/lib.rs`:

```rust
pub mod export;
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
cargo test -p open_entities world_json
```

Expected: FAIL (`todo!()` panics or compile error if `todo!` removed without impl).

- [ ] **Step 3: Implement `world_json_from_world`**

Replace `world_json_from_world` in `export/mod.rs`:

```rust
fn world_json_from_world(world: &mut World) -> Result<String, ExportError> {
    let mut query = world.query::<(Entity, &Position)>();
    let entities = query
        .iter(world)
        .map(|(entity, position)| EntityExport {
            id: EntityIdExport {
                index: entity.index_u32(),
                generation: entity.generation().to_bits(),
            },
            position: *position,
        })
        .collect();

    let payload = WorldExport {
        version: SCHEMA_VERSION,
        entities,
    };

    Ok(serde_json::to_string(&payload)?)
}
```

- [ ] **Step 4: Run tests to verify pass**

Run:

```bash
cargo test -p open_entities world_json
```

Expected: both `world_json_*` tests PASS.

- [ ] **Step 5: Commit**

```bash
git add open-entities-lib/src/export/mod.rs open-entities-lib/src/lib.rs
git commit -m "feat: add world_json export with schema version 1"
```

---

### Task 6: Public re-exports

**Files:**
- Modify: `open-entities-lib/src/lib.rs`

- [ ] **Step 1: Add public re-exports**

Ensure `open-entities-lib/src/lib.rs` contains (order may vary; keep existing `hello` and ECS re-exports):

```rust
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

pub use bevy_ecs::prelude::{Component, Entity, Query, World};

pub mod api;
pub mod components;
pub mod core;
pub mod export;

pub use api::Api;
pub use core::Core;
pub use export::ExportError;

/// Returns the canonical hello-world greeting.
#[must_use]
pub const fn hello() -> &'static str {
    "Hello, world!"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_returns_greeting() {
        assert_eq!(hello(), "Hello, world!");
    }
}
```

- [ ] **Step 2: Run full crate tests**

Run:

```bash
cargo test -p open_entities
```

Expected: all tests pass (hello + position + export).

- [ ] **Step 3: Run clippy**

Run:

```bash
cargo clippy -p open_entities -- -D warnings
```

Expected: no warnings.

- [ ] **Step 4: Commit**

```bash
git add open-entities-lib/src/lib.rs
git commit -m "feat: re-export Api, Core, and ExportError from crate root"
```

---

### Task 7: `world_json` example

**Files:**
- Create: `open-entities-lib/examples/world_json.rs`

- [ ] **Step 1: Create example**

Create `open-entities-lib/examples/world_json.rs`:

```rust
//! Spawns sample entities and prints a pretty-printed world JSON snapshot.

use open_entities::{Api, components::Position};

fn main() {
    let mut api = Api::new();
    let world = api.core_mut().world_mut();

    world.spawn(Position { x: 10.0, y: 5.0 });
    world.spawn(Position { x: 0.0, y: 0.0 });

    match api.world_json() {
        Ok(json) => match serde_json::from_str::<serde_json::Value>(&json) {
            Ok(value) => {
                let pretty = serde_json::to_string_pretty(&value)
                    .expect("pretty-print valid JSON value");
                println!("{pretty}");
            }
            Err(err) => eprintln!("export returned invalid JSON: {err}"),
        },
        Err(err) => eprintln!("export failed: {err}"),
    }
}
```

- [ ] **Step 2: Run example**

Run:

```bash
cargo run -p open_entities --example world_json
```

Expected: stdout contains pretty JSON with `"version": 1`, two entities, each with `id` and `position` objects.

- [ ] **Step 3: Commit**

```bash
git add open-entities-lib/examples/world_json.rs
git commit -m "feat: add world_json example with pretty-printed output"
```

---

### Task 8: Makefile targets

**Files:**
- Modify: `Makefile`

- [ ] **Step 1: Update Makefile**

Replace `Makefile` with:

```makefile
.PHONY: test example example-world-json

test:
	cargo test

example:
	cargo run -p open_entities --example hello

example-world-json:
	cargo run -p open_entities --example world_json
```

- [ ] **Step 2: Verify both targets**

Run:

```bash
make example
```

Expected stdout:

```
Hello, world!
```

Run:

```bash
make example-world-json
```

Expected: pretty JSON with `"version": 1` and two entities.

- [ ] **Step 3: Commit**

```bash
git add Makefile
git commit -m "chore: add example-world-json Makefile target"
```

---

### Task 9: Final verification

**Files:** (none — verify only)

- [ ] **Step 1: Full test suite**

Run:

```bash
cargo test -p open_entities
```

Expected: all tests pass.

- [ ] **Step 2: Clippy**

Run:

```bash
cargo clippy -p open_entities -- -D warnings
```

Expected: clean.

- [ ] **Step 3: Examples via Makefile**

Run:

```bash
make example
make example-world-json
```

Expected: hello greeting; v1 world JSON snapshot.

---

## Spec coverage (self-review)

| Spec requirement | Plan task |
|------------------|-----------|
| `Core` owns `World`, `new` / `world` / `world_mut` | Task 2 |
| `Api` wraps `Core`, `new` / `core_mut` | Task 3 |
| `serde` / `serde_json` workspace + crate deps | Task 1 |
| `Position: Serialize`, no `PositionExport` duplicate | Task 4 |
| `export` module, private `WorldExport` / `EntityExport` | Task 5 |
| `Api::world_json(&mut self) -> Result<String, ExportError>` | Task 5 |
| Schema `version: 1`, entities with `Position` only | Task 5 |
| Entity id `{ index, generation }` | Task 5 |
| `ExportError::Serde` + `Display` / `Error` | Task 5 |
| Re-export `Api`, `Core`, `ExportError`; keep ECS + `hello()` | Task 6 |
| Tests: empty world, one positioned entity | Task 5 |
| `examples/world_json.rs` pretty-prints via parse + `to_string_pretty` | Task 7 |
| `make example` → hello; `make example-world-json` | Task 8 |
| No WASM, no `world_json_pretty` in lib, no other components | Out of scope |
| Verification commands from spec | Task 9 |

**Follow-on (not this plan):** RTS components and schema v2 — `docs/superpowers/plans/2026-05-16-rts-components-export-v2.md`.

No placeholders. Types and paths are consistent across tasks.

---

## Execution handoff

Plan complete and saved to `docs/superpowers/plans/2026-05-16-core-api-world-export.md`.

**Note:** On branches that already include schema v2 / RTS components, most tasks are already done; use this plan for greenfield implementation or to verify v1 behavior before applying the v2 plan.

**Two execution options:**

1. **Subagent-Driven (recommended)** — dispatch a fresh subagent per task, review between tasks, fast iteration
2. **Inline Execution** — implement in this session with checkpoints (`executing-plans`)

Which approach?
