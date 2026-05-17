# Spawn Entity & Shared `EntityComponents` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract public `EntityComponents` shared by import, export, and spawn overrides; rename `spawn_yaml` → `spawn_entity` with component-level merge overrides.

**Architecture:** New `entity_components.rs` holds `EntityComponents` and `merge_components`. `import` and `export` both use it (export via `#[serde(flatten)]` on `EntityExport`). `spawn_entity` clones the loaded template, merges `overrides` with `merge_components(&base, &overrides)`, then calls existing `spawn_from_doc`. No `wasm-bindings` in this increment.

**Tech Stack:** Rust 2024, `bevy_ecs 0.18`, `serde`, `serde_yaml 0.9`, `serde_json` (existing).

**Spec:** `docs/superpowers/specs/2026-05-17-spawn-entity-shared-components-design.md`

---

## File map

| File | Responsibility |
|------|----------------|
| `open-entities-lib/src/entity_components.rs` | **NEW** — public `EntityComponents`, `merge_components`, unit tests |
| `open-entities-lib/src/lib.rs` | `mod entity_components;` + `pub use entity_components::EntityComponents` |
| `open-entities-lib/src/import/mod.rs` | Remove local `EntityComponents` / `merge_components`; `spawn_entity`; test renames + override tests |
| `open-entities-lib/src/export/mod.rs` | `EntityExport` flattens `EntityComponents`; construction uses `components: EntityComponents { ... }` |
| `open-entities-lib/examples/spawn_entity.rs` | **RENAMED** from `spawn_yaml.rs`; uses `spawn_entity` + demo overrides |
| `Makefile` | `EXAMPLE ?= spawn_entity` |

**Out of scope:** `wasm-bindings`, `spawn_yaml` deprecation alias, new components, deep merge inside component structs.

---

### Task 1: Add `entity_components` module with `EntityComponents` and `merge_components`

**Files:**
- Create: `open-entities-lib/src/entity_components.rs`
- Modify: `open-entities-lib/src/lib.rs`

- [ ] **Step 1: Create `entity_components.rs` with type, merge fn, and tests**

Create `open-entities-lib/src/entity_components.rs`:

```rust
use serde::{Deserialize, Serialize};

use crate::components::{Faction, MoveTarget, Position, Velocity};

/// Gameplay components shared by YAML templates, spawn overrides, and export (flattened).
#[derive(Clone, Default, PartialEq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntityComponents {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<Position>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub velocity: Option<Velocity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faction: Option<Faction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub move_target: Option<MoveTarget>,
}

/// Component-level merge: `child` wins when `Some`.
pub fn merge_components(parent: &EntityComponents, child: &EntityComponents) -> EntityComponents {
    EntityComponents {
        position: child.position.or(parent.position),
        velocity: child.velocity.or(parent.velocity),
        faction: child.faction.or(parent.faction),
        move_target: child.move_target.or(parent.move_target),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_child_wins_over_parent() {
        let parent = EntityComponents {
            faction: Some(Faction(1)),
            velocity: Some(Velocity { vx: 1.0, vy: 0.0 }),
            ..Default::default()
        };
        let child = EntityComponents {
            faction: Some(Faction(2)),
            ..Default::default()
        };
        let merged = merge_components(&parent, &child);
        assert_eq!(merged.faction, Some(Faction(2)));
        assert_eq!(merged.velocity, Some(Velocity { vx: 1.0, vy: 0.0 }));
    }

    #[test]
    fn merge_fills_missing_from_parent() {
        let parent = EntityComponents {
            faction: Some(Faction(1)),
            ..Default::default()
        };
        let child = EntityComponents {
            velocity: Some(Velocity { vx: 2.0, vy: 0.0 }),
            ..Default::default()
        };
        let merged = merge_components(&parent, &child);
        assert_eq!(merged.faction, Some(Faction(1)));
        assert_eq!(merged.velocity, Some(Velocity { vx: 2.0, vy: 0.0 }));
    }
}
```

- [ ] **Step 2: Wire module and public re-export in `lib.rs`**

After `pub mod import;` add:

```rust
mod entity_components;
```

After `pub use import::ImportError;` add:

```rust
pub use entity_components::EntityComponents;
```

- [ ] **Step 3: Run merge unit tests**

Run:

```bash
cargo test -p open_entities merge_child_wins_over_parent merge_fills_missing_from_parent -- --nocapture
```

Expected: both tests `ok`.

- [ ] **Step 4: Commit**

```bash
git add open-entities-lib/src/entity_components.rs open-entities-lib/src/lib.rs
git commit -m "feat: add shared EntityComponents module"
```

---

### Task 2: Refactor `import` to use shared `EntityComponents`

**Files:**
- Modify: `open-entities-lib/src/import/mod.rs`

- [ ] **Step 1: Update imports and remove local definitions**

At top of `import/mod.rs`, add:

```rust
use crate::entity_components::{merge_components, EntityComponents};
```

Delete the entire block from line `/// Shared component bundle` through `fn merge_components(...) { ... }` (the `pub(crate) struct EntityComponents`, `EntitySpawnYaml` alias stays but now points at imported type).

Keep:

```rust
pub(crate) type EntitySpawnYaml = EntityComponents;
```

Remove `#[cfg(test)] mod merge_tests { ... }` (tests now live in `entity_components.rs`).

- [ ] **Step 2: Run full import + resolve test suite**

Run:

```bash
cargo test -p open_entities -- --nocapture
```

Expected: all tests pass (still using `spawn_yaml`).

- [ ] **Step 3: Commit**

```bash
git add open-entities-lib/src/import/mod.rs
git commit -m "refactor: import uses shared EntityComponents"
```

---

### Task 3: Flatten `EntityExport` onto `EntityComponents`

**Files:**
- Modify: `open-entities-lib/src/export/mod.rs`

- [ ] **Step 1: Run export tests (baseline)**

Run:

```bash
cargo test -p open_entities world_json -- --nocapture
```

Expected: all `world_json_*` tests `ok` (record this as regression baseline).

- [ ] **Step 2: Refactor `EntityExport`**

Add import at top of `export/mod.rs`:

```rust
use crate::entity_components::EntityComponents;
```

Replace `EntityExport` struct with:

```rust
#[derive(Serialize)]
struct EntityExport {
    id: EntityIdExport,
    #[serde(flatten)]
    components: EntityComponents,
    #[serde(skip_serializing_if = "Option::is_none")]
    entity_type: Option<EntityType>,
}
```

In `world_json_from_world`, replace the `Some(EntityExport { ... })` construction with:

```rust
Some(EntityExport {
    id: EntityIdExport {
        index: entity.index_u32(),
        generation: entity.generation().to_bits(),
    },
    components: EntityComponents {
        position: position.copied(),
        velocity: velocity.copied(),
        faction: faction.copied(),
        move_target: move_target.copied(),
    },
    entity_type: entity_type.cloned(),
})
```

- [ ] **Step 3: Run export tests again**

Run:

```bash
cargo test -p open_entities world_json -- --nocapture
```

Expected: same tests `ok` (JSON shape unchanged — flattened fields omit `None` via `skip_serializing_if` on `EntityComponents`).

- [ ] **Step 4: Commit**

```bash
git add open-entities-lib/src/export/mod.rs
git commit -m "refactor: EntityExport flattens EntityComponents"
```

---

### Task 4: `spawn_entity` with overrides (TDD)

**Files:**
- Modify: `open-entities-lib/src/import/mod.rs`

- [ ] **Step 1: Write failing override tests**

In `mod tests` in `import/mod.rs`, add (after `load_fixture`):

```rust
#[test]
fn spawn_entity_overrides_faction() {
    let mut api = Api::new();
    load_fixture(&mut api);
    let entity = api
        .spawn_entity(
            "scout",
            EntityComponents {
                faction: Some(Faction(99)),
                ..Default::default()
            },
        )
        .expect("spawn with faction override");
    let world = api.core_mut().world_mut();
    let faction = world.get::<Faction>(entity).expect("faction");
    assert_eq!(faction.0, 99);
    // velocity unchanged from template
    let velocity = world.get::<Velocity>(entity).expect("velocity");
    assert_eq!(velocity.vx, 2.0);
    assert_eq!(velocity.vy, 0.0);
}

#[test]
fn spawn_entity_overrides_position() {
    let mut api = Api::new();
    load_fixture(&mut api);
    let entity = api
        .spawn_entity(
            "scout",
            EntityComponents {
                position: Some(Position { x: 100.0, y: 200.0 }),
                ..Default::default()
            },
        )
        .expect("spawn with position override");
    let world = api.core_mut().world_mut();
    let position = world.get::<Position>(entity).expect("position");
    assert_eq!(position.x, 100.0);
    assert_eq!(position.y, 200.0);
    let faction = world.get::<Faction>(entity).expect("faction still from template");
    assert_eq!(faction.0, 1);
}

#[test]
fn spawn_entity_no_overrides_matches_template() {
    let mut api = Api::new();
    load_fixture(&mut api);
    let entity = api
        .spawn_entity("scout", EntityComponents::default())
        .expect("spawn scout");
    let world = api.core_mut().world_mut();
    let position = world.get::<Position>(entity).expect("position");
    assert_eq!(position.x, 0.0);
    assert_eq!(position.y, 0.0);
    let velocity = world.get::<Velocity>(entity).expect("velocity");
    assert_eq!(velocity.vx, 2.0);
    assert_eq!(velocity.vy, 0.0);
    let faction = world.get::<Faction>(entity).expect("faction");
    assert_eq!(faction.0, 1);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test -p open_entities spawn_entity_overrides spawn_entity_no_overrides -- --nocapture
```

Expected: compile error — `spawn_entity` not found on `Api`.

- [ ] **Step 3: Replace `spawn_yaml` with `spawn_entity`**

In `ImportError` doc comment, change `spawn_yaml` → `spawn_entity`.

Replace `spawn_yaml` method with:

```rust
/// Spawns one entity from a previously loaded template, applying optional component overrides.
///
/// Each `Some` field in `overrides` replaces the template value; `None` fields leave the
/// template unchanged. `EntityComponents::default()` spawns the template as loaded.
///
/// # Errors
///
/// Returns [`ImportError::TemplatesNotLoaded`] if no successful load yet.
/// Returns [`ImportError::UnknownTemplate`] if `template_name` is missing.
pub fn spawn_entity(
    &mut self,
    template_name: &str,
    overrides: EntityComponents,
) -> Result<Entity, ImportError> {
    let templates = self
        .templates
        .as_ref()
        .ok_or(ImportError::TemplatesNotLoaded)?;
    let base = templates
        .get(template_name)
        .ok_or_else(|| ImportError::UnknownTemplate(template_name.to_owned()))?
        .clone();
    let doc = merge_components(&base, &overrides);
    Ok(spawn_from_doc(
        self.core_mut().world_mut(),
        template_name,
        &doc,
    ))
}
```

Delete the old `spawn_yaml` method entirely (no deprecation alias).

- [ ] **Step 4: Run new spawn tests**

Run:

```bash
cargo test -p open_entities spawn_entity_overrides spawn_entity_no_overrides -- --nocapture
```

Expected: all three new tests `ok`.

- [ ] **Step 5: Commit**

```bash
git add open-entities-lib/src/import/mod.rs
git commit -m "feat: spawn_entity with component overrides"
```

---

### Task 5: Rename spawn tests and fix compile errors

**Files:**
- Modify: `open-entities-lib/src/import/mod.rs`

- [ ] **Step 1: Rename test functions and update calls**

Apply these renames in `import/mod.rs` `mod tests`:

| Old test name | New test name |
|---------------|---------------|
| `spawn_yaml_without_load` | `spawn_entity_without_load` |
| `spawn_yaml_unknown_template` | `spawn_entity_unknown_template` |
| `spawn_yaml_scout` | `spawn_entity_scout` |
| `spawn_yaml_base` | `spawn_entity_base` |
| `spawn_yaml_marker` | `spawn_entity_marker` |
| `spawn_yaml_twice_same_name` | `spawn_entity_twice_same_name` |
| `spawn_yaml_exports_entity_type_in_world_json` | `spawn_entity_exports_entity_type_in_world_json` |

Replace every `api.spawn_yaml("name")` call. Each becomes:

```rust
api.spawn_entity("scout", EntityComponents::default())
```

Also update the two calls inside `load_templates_yaml_invalid`:

```rust
api.spawn_entity("scout", EntityComponents::default()).unwrap_err(),
```

And `load_templates_yaml_failed_replaces_keeps_previous` / inheritance tests — all `spawn_yaml` occurrences in this file.

- [ ] **Step 2: Run full crate tests**

Run:

```bash
cargo test -p open_entities -- --nocapture
```

Expected: all tests `ok`.

- [ ] **Step 3: Commit**

```bash
git add open-entities-lib/src/import/mod.rs
git commit -m "test: rename spawn_yaml tests to spawn_entity"
```

---

### Task 6: Rename example and update Makefile

**Files:**
- Rename: `open-entities-lib/examples/spawn_yaml.rs` → `open-entities-lib/examples/spawn_entity.rs`
- Modify: `Makefile`

- [ ] **Step 1: Rename example file**

```bash
git mv open-entities-lib/examples/spawn_yaml.rs open-entities-lib/examples/spawn_entity.rs
```

- [ ] **Step 2: Update example source**

In `spawn_entity.rs`, update the module doc comment to mention overrides. Replace spawn loop body:

```rust
use open_entities::EntityComponents;
use open_entities::components::{Faction, Position};
```

```rust
    for name in ["marker", "heavy_tank", "tank", "scout", "unit"] {
        let overrides = if name == "scout" {
            EntityComponents {
                position: Some(Position { x: 50.0, y: 25.0 }),
                ..Default::default()
            }
        } else {
            EntityComponents::default()
        };
        match api.spawn_entity(name, overrides) {
            Ok(entity) => println!("spawned {name} -> entity {:?}", entity),
            Err(err) => eprintln!("spawn {name} failed: {err}"),
        }
    }
```

- [ ] **Step 3: Update Makefile**

Change line 6 from `EXAMPLE ?= spawn_yaml` to `EXAMPLE ?= spawn_entity`.

- [ ] **Step 4: Run example**

Run:

```bash
make example
```

Expected: compiles and prints spawned entities + pretty JSON (scout position `50, 25` in export output).

- [ ] **Step 5: Commit**

```bash
git add open-entities-lib/examples/spawn_entity.rs Makefile
git commit -m "chore: rename spawn_yaml example to spawn_entity"
```

---

### Task 7: Final verification

**Files:** (none — verification only)

- [ ] **Step 1: Full test suite + clippy**

Run:

```bash
cargo test -p open_entities
cargo clippy -p open_entities -- -D warnings
```

Expected: all tests pass, no clippy warnings.

- [ ] **Step 2: Confirm public API surface**

Run:

```bash
cargo doc -p open_entities --no-deps 2>&1 | head -5
```

Manually confirm `open_entities::EntityComponents` and `Api::spawn_entity` appear in docs. Confirm `spawn_yaml` does not exist (grep):

```bash
rg 'spawn_yaml' open-entities-lib/src open-entities-lib/examples Makefile
```

Expected: no matches in those paths.

- [ ] **Step 3: Commit (only if verification fixes were needed)**

If Step 1–2 required fixes, commit them; otherwise skip empty commit.

---

## Spec coverage checklist

| Spec requirement | Task |
|------------------|------|
| Public `EntityComponents` in `entity_components.rs` | Task 1 |
| `Serialize` + `Deserialize`, `deny_unknown_fields`, `Default` | Task 1 |
| `pub` fields on all four `Option<…>` | Task 1 |
| `merge_components` in shared module | Task 1 |
| Re-export `EntityComponents` from `lib.rs` | Task 1 |
| Import uses shared type; `EntitySpawnYaml` alias kept | Task 2 |
| `EntityExport` `#[serde(flatten)]` — JSON unchanged | Task 3 |
| `spawn_entity(name, overrides)` with merge at spawn | Task 4 |
| Hard rename, no `spawn_yaml` alias | Task 4–6 |
| Override tests (faction, position, no overrides) | Task 4 |
| Rename existing spawn tests | Task 5 |
| Example + Makefile | Task 6 |
| Export regression (`world_json` tests) | Task 3, 7 |
| No `wasm-bindings` | Out of scope |

## Type consistency notes

- `EntityComponents` field names: `position`, `velocity`, `faction`, `move_target` — used identically in import YAML, export JSON, and spawn overrides.
- `entity_type` stays on `EntityExport` only and is injected in `spawn_from_doc` from `template_name`; never part of `EntityComponents`.
- `merge_components(parent, child)` — spawn passes `parent = template`, `child = overrides` so override `Some` wins.
