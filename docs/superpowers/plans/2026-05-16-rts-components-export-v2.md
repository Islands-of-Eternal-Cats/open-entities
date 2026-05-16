# RTS Components and World JSON Export v2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add RTS components (`Velocity`, `Faction`, `MoveTarget`) and upgrade `Api::world_json()` to schema version 2 with optional per-component JSON fields and a single multi-component query.

**Architecture:** Three new `#[derive(Component)]` types under `components/`, each with a spawn + `Query` round-trip unit test matching `position.rs`. `export::world_json_from_world` uses one `Query<(Entity, Option<&T>, …)>` over all four export components, skips rows with no components, and serializes via `EntityExport` with `skip_serializing_if` on each optional field. Schema constant bumps from `1` to `2`.

**Tech Stack:** Rust 2024, `bevy_ecs 0.18`, `serde` / `serde_json` (workspace deps, no new crates).

**Spec:** `docs/superpowers/specs/2026-05-16-rts-components-export-v2-design.md`

---

## File map

| File | Responsibility |
|------|----------------|
| `open-entities-lib/src/components/velocity.rs` | `Velocity { vx, vy }` + round-trip test |
| `open-entities-lib/src/components/faction.rs` | `Faction(u32)` newtype + round-trip test |
| `open-entities-lib/src/components/move_target.rs` | `MoveTarget { x, y }` + round-trip test |
| `open-entities-lib/src/components/mod.rs` | Re-export all four component types |
| `open-entities-lib/src/components/position.rs` | Unchanged shape; existing test stays |
| `open-entities-lib/src/export/mod.rs` | v2 query, `EntityExport` options, export tests |
| `README.md` | v2 JSON example, inclusion rule, new components |

**Unchanged (verify only):** `api.rs`, `core.rs`, `lib.rs`, `examples/world_json.rs` (example may keep `Position`-only spawns; tests define the v2 contract).

---

### Task 1: `Velocity` component

**Files:**
- Create: `open-entities-lib/src/components/velocity.rs`
- Modify: `open-entities-lib/src/components/mod.rs`

- [ ] **Step 1: Write the failing test**

Create `open-entities-lib/src/components/velocity.rs`:

```rust
use bevy_ecs::prelude::Component;
use serde::Serialize;

/// 2D velocity in world/simulation space.
#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Velocity {
    pub vx: f32,
    pub vy: f32,
}

#[cfg(test)]
mod tests {
    use super::Velocity;
    use bevy_ecs::prelude::*;

    #[test]
    fn velocity_component_round_trip() {
        let mut world = World::new();
        world.spawn(Velocity { vx: 1.5, vy: -2.0 });

        let mut query = world.query::<&Velocity>();
        let mut count = 0;
        for velocity in query.iter(&world) {
            assert_eq!(velocity.vx, 1.5);
            assert_eq!(velocity.vy, -2.0);
            count += 1;
        }
        assert_eq!(count, 1);
    }
}
```

Update `open-entities-lib/src/components/mod.rs`:

```rust
pub mod position;
pub mod velocity;

pub use position::Position;
pub use velocity::Velocity;
```

- [ ] **Step 2: Run test to verify it compiles and passes**

Run:

```bash
cargo test -p open_entities velocity_component_round_trip -- --nocapture
```

Expected: `test velocity_component_round_trip ... ok`

- [ ] **Step 3: Commit**

```bash
git add open-entities-lib/src/components/velocity.rs open-entities-lib/src/components/mod.rs
git commit -m "feat: add Velocity component with round-trip test"
```

---

### Task 2: `Faction` component

**Files:**
- Create: `open-entities-lib/src/components/faction.rs`
- Modify: `open-entities-lib/src/components/mod.rs`

- [ ] **Step 1: Write the component and test**

Create `open-entities-lib/src/components/faction.rs`:

```rust
use bevy_ecs::prelude::Component;
use serde::Serialize;

/// Numeric faction / side identifier.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct Faction(pub u32);

#[cfg(test)]
mod tests {
    use super::Faction;
    use bevy_ecs::prelude::*;

    #[test]
    fn faction_component_round_trip() {
        let mut world = World::new();
        world.spawn(Faction(42));

        let mut query = world.query::<&Faction>();
        let mut count = 0;
        for faction in query.iter(&world) {
            assert_eq!(faction.0, 42);
            count += 1;
        }
        assert_eq!(count, 1);
    }
}
```

Update `open-entities-lib/src/components/mod.rs`:

```rust
pub mod faction;
pub mod position;
pub mod velocity;

pub use faction::Faction;
pub use position::Position;
pub use velocity::Velocity;
```

- [ ] **Step 2: Run test**

Run:

```bash
cargo test -p open_entities faction_component_round_trip -- --nocapture
```

Expected: `test faction_component_round_trip ... ok`

- [ ] **Step 3: Commit**

```bash
git add open-entities-lib/src/components/faction.rs open-entities-lib/src/components/mod.rs
git commit -m "feat: add Faction component with round-trip test"
```

---

### Task 3: `MoveTarget` component

**Files:**
- Create: `open-entities-lib/src/components/move_target.rs`
- Modify: `open-entities-lib/src/components/mod.rs`

- [ ] **Step 1: Write the component and test**

Create `open-entities-lib/src/components/move_target.rs`:

```rust
use bevy_ecs::prelude::Component;
use serde::Serialize;

/// World-space movement goal point.
#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize)]
pub struct MoveTarget {
    pub x: f32,
    pub y: f32,
}

#[cfg(test)]
mod tests {
    use super::MoveTarget;
    use bevy_ecs::prelude::*;

    #[test]
    fn move_target_component_round_trip() {
        let mut world = World::new();
        world.spawn(MoveTarget { x: 20.0, y: 0.0 });

        let mut query = world.query::<&MoveTarget>();
        let mut count = 0;
        for target in query.iter(&world) {
            assert_eq!(target.x, 20.0);
            assert_eq!(target.y, 0.0);
            count += 1;
        }
        assert_eq!(count, 1);
    }
}
```

Update `open-entities-lib/src/components/mod.rs` to final exports:

```rust
pub mod faction;
pub mod move_target;
pub mod position;
pub mod velocity;

pub use faction::Faction;
pub use move_target::MoveTarget;
pub use position::Position;
pub use velocity::Velocity;
```

- [ ] **Step 2: Run test**

Run:

```bash
cargo test -p open_entities move_target_component_round_trip -- --nocapture
```

Expected: `test move_target_component_round_trip ... ok`

- [ ] **Step 3: Commit**

```bash
git add open-entities-lib/src/components/move_target.rs open-entities-lib/src/components/mod.rs
git commit -m "feat: add MoveTarget component with round-trip test"
```

---

### Task 4: Export schema v2 — update existing tests (RED)

**Files:**
- Modify: `open-entities-lib/src/export/mod.rs`

- [ ] **Step 1: Change assertions to expect version 2 (export still v1 — tests should fail)**

In `open-entities-lib/src/export/mod.rs`, inside `mod tests`:

Change `world_json_empty_world`:

```rust
assert_eq!(value["version"], 2);
```

Change `world_json_includes_positioned_entities`:

```rust
assert_eq!(value["version"], 2);
```

Add imports at top of `mod tests`:

```rust
use crate::components::{Faction, MoveTarget, Position, Velocity};
```

Add two new failing tests:

```rust
#[test]
fn world_json_faction_only_entity() {
    let mut api = Api::new();
    api.core_mut().world_mut().spawn(Faction(2));

    let json = api.world_json().expect("serialize world");
    let value: serde_json::Value =
        serde_json::from_str(&json).expect("exported JSON should parse");

    assert_eq!(value["version"], 2);
    let entities = value["entities"].as_array().expect("entities array");
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0]["faction"], 2);
    assert!(entities[0].get("position").is_none());
}

#[test]
fn world_json_partial_components() {
    let mut api = Api::new();
    api.core_mut().world_mut().spawn((
        Position { x: 1.0, y: 2.0 },
        Velocity { vx: 0.5, vy: -0.5 },
    ));

    let json = api.world_json().expect("serialize world");
    let value: serde_json::Value =
        serde_json::from_str(&json).expect("exported JSON should parse");

    assert_eq!(value["version"], 2);
    let entities = value["entities"].as_array().expect("entities array");
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0]["position"]["x"], 1.0);
    assert_eq!(entities[0]["velocity"]["vx"], 0.5);
    assert!(entities[0].get("faction").is_none());
    assert!(entities[0].get("move_target").is_none());
}
```

- [ ] **Step 2: Run export tests to verify failures**

Run:

```bash
cargo test -p open_entities world_json -- --nocapture
```

Expected: failures — `version` is `1` not `2`; `world_json_faction_only_entity` likely exports zero entities (v1 only queries `Position`).

- [ ] **Step 3: Commit (RED)**

```bash
git add open-entities-lib/src/export/mod.rs
git commit -m "test: expect world_json schema v2 and new export cases"
```

---

### Task 5: Export schema v2 — implementation (GREEN)

**Files:**
- Modify: `open-entities-lib/src/export/mod.rs`

- [ ] **Step 1: Implement v2 export**

Replace the top of `open-entities-lib/src/export/mod.rs` imports and constants:

```rust
use bevy_ecs::prelude::{Entity, World};
use serde::Serialize;

use crate::api::Api;
use crate::components::{Faction, MoveTarget, Position, Velocity};

const SCHEMA_VERSION: u32 = 2;
```

Replace `EntityExport`:

```rust
#[derive(Serialize)]
struct EntityExport {
    id: EntityIdExport,
    #[serde(skip_serializing_if = "Option::is_none")]
    position: Option<Position>,
    #[serde(skip_serializing_if = "Option::is_none")]
    velocity: Option<Velocity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    faction: Option<Faction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    move_target: Option<MoveTarget>,
}
```

Update `Api::world_json` doc comment:

```rust
/// Serializes entities that have at least one of [`Position`], [`Velocity`],
/// [`Faction`], or [`MoveTarget`] to JSON (schema version 2).
///
/// Component fields are omitted from each entity row when that component is
/// not present on the entity.
///
/// # Errors
///
/// Returns [`ExportError::Serde`] if JSON encoding fails.
```

Replace `world_json_from_world`:

```rust
fn world_json_from_world(world: &mut World) -> Result<String, ExportError> {
    let mut query = world.query::<(
        Entity,
        Option<&Position>,
        Option<&Velocity>,
        Option<&Faction>,
        Option<&MoveTarget>,
    )>();
    let entities = query
        .iter(world)
        .filter_map(|(entity, position, velocity, faction, move_target)| {
            if position.is_none()
                && velocity.is_none()
                && faction.is_none()
                && move_target.is_none()
            {
                return None;
            }
            Some(EntityExport {
                id: EntityIdExport {
                    index: entity.index_u32(),
                    generation: entity.generation().to_bits(),
                },
                position: position.copied(),
                velocity: velocity.copied(),
                faction: faction.copied(),
                move_target: move_target.copied(),
            })
        })
        .collect();

    let payload = WorldExport {
        version: SCHEMA_VERSION,
        entities,
    };

    Ok(serde_json::to_string(&payload)?)
}
```

- [ ] **Step 2: Run all export tests**

Run:

```bash
cargo test -p open_entities world_json -- --nocapture
```

Expected: all `world_json_*` tests pass.

- [ ] **Step 3: Run full crate tests**

Run:

```bash
cargo test -p open_entities
```

Expected: all tests pass (component round-trips + export).

- [ ] **Step 4: Commit**

```bash
git add open-entities-lib/src/export/mod.rs
git commit -m "feat: world_json schema v2 with optional RTS components"
```

---

### Task 6: README documentation

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update library description**

Replace the export bullet (around line 9) with:

```markdown
- [`export`](open-entities-lib/src/export/mod.rs) — `Api::world_json()` serializes entities that have at least one RTS export component (`Position`, `Velocity`, `Faction`, `MoveTarget`) to JSON schema version 2
```

Replace the domain components line (around line 11) with:

```markdown
Domain components live under `open_entities::components`: `Position`, `Velocity`, `Faction`, and `MoveTarget`.
```

- [ ] **Step 2: Update JSON example and inclusion rule**

Replace the "Exported shape" section (from `Exported shape (schema version` through `Only entities that have a Position`) with:

```markdown
Exported shape (schema version `2`):

```json
{
  "version": 2,
  "entities": [
    {
      "id": { "index": 0, "generation": 0 },
      "position": { "x": 1.0, "y": 2.0 },
      "velocity": { "vx": 0.5, "vy": -0.5 },
      "faction": 1
    },
    {
      "id": { "index": 1, "generation": 0 },
      "faction": 2
    }
  ]
}
```

Entities are included if they have at least one of `Position`, `Velocity`, `Faction`, or `MoveTarget`. Keys for components the entity does not have are omitted (not `null`).
```

- [ ] **Step 3: Verify README renders (optional skim)**

No command required; confirm fenced JSON block is closed and nested markdown is valid.

- [ ] **Step 4: Commit**

```bash
git add README.md
git commit -m "docs: document world_json schema v2 and RTS components"
```

---

### Task 7: Final verification

**Files:** (none — verification only)

- [ ] **Step 1: Run full test suite**

Run from repo root:

```bash
cargo test -p open_entities
```

Expected: all tests pass, 0 failures.

- [ ] **Step 2: Run clippy with warnings denied**

Run:

```bash
cargo clippy -p open_entities -- -D warnings
```

Expected: clean build, no warnings.

- [ ] **Step 3: Optional smoke — world_json example**

Run:

```bash
cargo run -p open_entities --example world_json
```

Expected: pretty-printed JSON with `"version": 2` and `position` entries for the two spawned entities (example unchanged; still valid v2 output).

---

## Spec coverage checklist

| Spec requirement | Task |
|------------------|------|
| `Velocity { vx, vy }` + test | Task 1 |
| `Faction(u32)` transparent serde + test | Task 2 |
| `MoveTarget { x, y }` + test | Task 3 |
| `components/mod.rs` re-exports | Tasks 1–3 |
| `Position` unchanged | — (no edits) |
| Schema `version: 2` | Task 5 |
| Single `Option<&T>` query | Task 5 |
| Skip entities with no export components | Task 5 |
| Omit missing component keys | Task 5 (`skip_serializing_if`) |
| `world_json_empty_world` v2 | Task 4–5 |
| `world_json_includes_positioned_entities` v2 | Task 4–5 |
| `world_json_faction_only_entity` | Task 4–5 |
| `world_json_partial_components` | Task 4–5 |
| `Api::world_json` doc comment v2 | Task 5 |
| README v2 example + inclusion rule | Task 6 |
| `cargo test` + `cargo clippy -D warnings` | Task 7 |
| No systems / tick / YAML / wasm | Out of scope (no tasks) |

## Placeholder scan

No TBD, "implement later", or "similar to Task N" steps. All test and implementation code is inlined above.
