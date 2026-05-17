# YAML Component Spawn Import Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `load_templates_yaml` + `spawn_yaml` so callers preload a YAML `entities:` map on `Api`, then spawn one ECS entity per template name with the same component field shapes as export v2.

**Architecture:** New `import/` module mirrors `export/`: `ImportError`, private `EntitySpawnYaml` + `EntityTemplates` map, parse via `serde_yaml` into `Api.templates`, spawn via `world.spawn_empty()` + per-field `insert`. Components gain `Deserialize` alongside existing `Serialize`. `impl Api` for import lives in `import/mod.rs` (same pattern as `export/mod.rs`).

**Tech Stack:** Rust 2024, `bevy_ecs 0.18`, `serde`, `serde_yaml 0.9` (new workspace dep).

**Spec:** `docs/superpowers/specs/2026-05-16-yaml-spawn-import-design.md`

---

## File map

| File | Responsibility |
|------|----------------|
| `Cargo.toml` (workspace) | `serde_yaml = "0.9"` workspace dependency |
| `open-entities-lib/Cargo.toml` | `serde_yaml = { workspace = true }` |
| `open-entities-lib/src/components/position.rs` | Add `Deserialize` |
| `open-entities-lib/src/components/velocity.rs` | Add `Deserialize` |
| `open-entities-lib/src/components/faction.rs` | Add `Deserialize` |
| `open-entities-lib/src/components/move_target.rs` | Add `Deserialize` |
| `open-entities-lib/src/import/mod.rs` | `ImportError`, YAML types, load/spawn, all import tests |
| `open-entities-lib/src/api.rs` | `templates: Option<EntityTemplates>`, `new()` sets `None` |
| `open-entities-lib/src/lib.rs` | `pub mod import;`, `pub use import::ImportError;` |

**Out of scope (no tasks):** `wasm-bindings`, examples, Makefile, README, template inheritance, disk I/O inside the library.

---

### Task 1: Add `serde_yaml` dependency

**Files:**
- Modify: `Cargo.toml`
- Modify: `open-entities-lib/Cargo.toml`

- [ ] **Step 1: Add workspace dependency**

In repo-root `Cargo.toml`, under `[workspace.dependencies]`:

```toml
serde_yaml = "0.9"
```

- [ ] **Step 2: Add crate dependency**

In `open-entities-lib/Cargo.toml`, under `[dependencies]`:

```toml
serde_yaml = { workspace = true }
```

- [ ] **Step 3: Verify dependency resolves**

Run:

```bash
cargo check -p open_entities
```

Expected: compiles successfully (no new source yet).

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml open-entities-lib/Cargo.toml
git commit -m "chore: add serde_yaml workspace dependency"
```

---

### Task 2: `Deserialize` on export components

**Files:**
- Modify: `open-entities-lib/src/components/position.rs`
- Modify: `open-entities-lib/src/components/velocity.rs`
- Modify: `open-entities-lib/src/components/faction.rs`
- Modify: `open-entities-lib/src/components/move_target.rs`

- [ ] **Step 1: Update `Position`**

Replace the serde import and derive in `position.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Position {
```

- [ ] **Step 2: Update `Velocity`**

Same pattern in `velocity.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Velocity {
```

- [ ] **Step 3: Update `Faction`**

In `faction.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Faction(pub u32);
```

- [ ] **Step 4: Update `MoveTarget`**

Same as `Position` in `move_target.rs`.

- [ ] **Step 5: Run existing component tests**

Run:

```bash
cargo test -p open_entities position_component_round_trip velocity_component_round_trip faction_component_round_trip move_target_component_round_trip -- --nocapture
```

Expected: all four tests `ok`.

- [ ] **Step 6: Commit**

```bash
git add open-entities-lib/src/components/position.rs open-entities-lib/src/components/velocity.rs open-entities-lib/src/components/faction.rs open-entities-lib/src/components/move_target.rs
git commit -m "feat: add Deserialize to RTS export components"
```

---

### Task 3: `ImportError` and import module skeleton

**Files:**
- Create: `open-entities-lib/src/import/mod.rs`
- Modify: `open-entities-lib/src/lib.rs`

- [ ] **Step 1: Create `ImportError`**

Create `open-entities-lib/src/import/mod.rs`:

```rust
/// Errors while loading YAML templates or spawning from them.
#[derive(Debug)]
pub enum ImportError {
    /// YAML syntax, type mismatch, or unknown field.
    Yaml(serde_yaml::Error),
    /// `spawn_yaml` called before a successful `load_templates_yaml`.
    TemplatesNotLoaded,
    /// No template with this name in the loaded map.
    UnknownTemplate(String),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Yaml(err) => write!(f, "YAML import failed: {err}"),
            Self::TemplatesNotLoaded => {
                f.write_str("templates not loaded; call load_templates_yaml first")
            }
            Self::UnknownTemplate(name) => {
                write!(f, "unknown template name: {name}")
            }
        }
    }
}

impl std::error::Error for ImportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Yaml(err) => Some(err),
            Self::TemplatesNotLoaded | Self::UnknownTemplate(_) => None,
        }
    }
}
```

- [ ] **Step 2: Wire `lib.rs`**

In `open-entities-lib/src/lib.rs`, after `pub mod export;`:

```rust
pub mod import;
```

After `pub use export::ExportError;`:

```rust
pub use import::ImportError;
```

- [ ] **Step 3: Verify module compiles**

Run:

```bash
cargo check -p open_entities
```

Expected: success.

- [ ] **Step 4: Commit**

```bash
git add open-entities-lib/src/import/mod.rs open-entities-lib/src/lib.rs
git commit -m "feat: add import module with ImportError"
```

---

### Task 4: `Api.templates` storage field

**Files:**
- Modify: `open-entities-lib/src/api.rs`
- Modify: `open-entities-lib/src/import/mod.rs`

- [ ] **Step 1: Expose `EntityTemplates` type alias to `api`**

At the top of `import/mod.rs` (after `ImportError` impls), add:

```rust
use std::collections::BTreeMap;

use bevy_ecs::prelude::{Entity, World};
use serde::Deserialize;

use crate::api::Api;
use crate::components::{Faction, MoveTarget, Position, Velocity};

/// In-memory map of template name → component bundle (private).
pub(crate) type EntityTemplates = BTreeMap<String, EntitySpawnYaml>;

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct EntitySpawnYaml {
    position: Option<Position>,
    velocity: Option<Velocity>,
    faction: Option<Faction>,
    move_target: Option<MoveTarget>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TemplatesFileRoot {
    entities: EntityTemplates,
}
```

- [ ] **Step 2: Add `templates` field on `Api`**

Replace `open-entities-lib/src/api.rs` with:

```rust
use crate::core::Core;
use crate::import::EntityTemplates;

/// Public facade over [`Core`] for simulation operations, export, and import.
pub struct Api {
    core: Core,
    pub(crate) templates: Option<EntityTemplates>,
}

impl Api {
    /// Creates an API backed by a new empty [`Core`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            core: Core::new(),
            templates: None,
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

- [ ] **Step 3: Verify compile**

Run:

```bash
cargo check -p open_entities
```

Expected: success.

- [ ] **Step 4: Commit**

```bash
git add open-entities-lib/src/api.rs open-entities-lib/src/import/mod.rs
git commit -m "feat: add templates storage on Api"
```

---

### Task 5: `load_templates_yaml` and load tests

**Files:**
- Modify: `open-entities-lib/src/import/mod.rs`

- [ ] **Step 1: Write failing load tests**

Append to `import/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::Api;

    #[test]
    fn spawn_yaml_without_load() {
        let mut api = Api::new();
        let err = api.spawn_yaml("scout").unwrap_err();
        assert!(matches!(err, ImportError::TemplatesNotLoaded));
    }

    #[test]
    fn load_templates_yaml_invalid() {
        let mut api = Api::new();
        let err = api
            .load_templates_yaml("not: [valid: yaml: structure")
            .unwrap_err();
        assert!(matches!(err, ImportError::Yaml(_)));
        assert!(api.templates.is_none());
        assert!(matches!(
            api.spawn_yaml("scout").unwrap_err(),
            ImportError::TemplatesNotLoaded
        ));
    }

    #[test]
    fn load_templates_yaml_unknown_root() {
        let mut api = Api::new();
        let err = api.load_templates_yaml("foo: 1").unwrap_err();
        assert!(matches!(err, ImportError::Yaml(_)));
        assert!(api.templates.is_none());
    }

    #[test]
    fn load_templates_yaml_invalid_nested() {
        let mut api = Api::new();
        let yaml = r"
entities:
  bad:
    position: not-an-object
";
        let err = api.load_templates_yaml(yaml).unwrap_err();
        assert!(matches!(err, ImportError::Yaml(_)));
        assert!(api.templates.is_none());
    }

    #[test]
    fn load_templates_yaml_unknown_component_key() {
        let mut api = Api::new();
        let err = api.load_templates_yaml("health: 1").unwrap_err();
        assert!(matches!(err, ImportError::Yaml(_)));
        assert!(api.templates.is_none());
    }

    #[test]
    fn load_templates_yaml_failed_replaces_keeps_previous() {
        let mut api = Api::new();
        api.load_templates_yaml("entities:\n  a:\n    faction: 1\n")
            .expect("first load");
        let err = api
            .load_templates_yaml("entities:\n  bad:\n    health: 1\n")
            .unwrap_err();
        assert!(matches!(err, ImportError::Yaml(_)));
        let templates = api.templates.as_ref().expect("map still loaded");
        assert!(templates.contains_key("a"));
        assert!(!templates.contains_key("bad"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test -p open_entities spawn_yaml_without_load load_templates_yaml -- --nocapture
```

Expected: compile errors — `load_templates_yaml` / `spawn_yaml` not found on `Api`.

- [ ] **Step 3: Implement `load_templates_yaml`**

Add to `import/mod.rs` (outside `#[cfg(test)]`):

```rust
impl Api {
    /// Parses a YAML templates file and stores it for later spawns.
    ///
    /// Root must be `entities: { <name>: <components>, ... }`.
    /// Replaces any previously loaded templates on success.
    /// On error, leaves any previously loaded templates unchanged.
    ///
    /// # Errors
    ///
    /// Returns [`ImportError::Yaml`] for invalid YAML or unknown fields.
    pub fn load_templates_yaml(&mut self, yaml: &str) -> Result<(), ImportError> {
        let parsed: TemplatesFileRoot =
            serde_yaml::from_str(yaml).map_err(ImportError::Yaml)?;
        self.templates = Some(parsed.entities);
        Ok(())
    }
}
```

- [ ] **Step 4: Add minimal `spawn_yaml` for lifecycle test only**

Still in `impl Api` (full spawn logic added in Task 6):

```rust
    /// Spawns one entity from a previously loaded template by name.
    pub fn spawn_yaml(&mut self, template_name: &str) -> Result<Entity, ImportError> {
        if self.templates.is_none() {
            return Err(ImportError::TemplatesNotLoaded);
        }
        let _ = template_name;
        Err(ImportError::UnknownTemplate(template_name.to_owned()))
    }
```

This is enough for `spawn_yaml_without_load` (`templates` is `None`). Spawn behavior tests wait until Task 6.

- [ ] **Step 5: Run load tests**

Run:

```bash
cargo test -p open_entities load_templates_yaml spawn_yaml_without_load -- --nocapture
```

Expected: all listed tests `ok`.

- [ ] **Step 6: Commit**

```bash
git add open-entities-lib/src/import/mod.rs
git commit -m "feat: load_templates_yaml with strict YAML validation"
```

---

### Task 6: `spawn_yaml`, `spawn_from_doc`, and spawn tests

**Files:**
- Modify: `open-entities-lib/src/import/mod.rs`

**Test fixture** (matches spec; note `vy` not `y`):

```rust
const FIXTURE_YAML: &str = r"
entities:
  scout:
    position: { x: 0, y: 0 }
    velocity: { vx: 2, vy: 0 }
    faction: 1
  base:
    faction: 2
  marker: {}
";
```

- [ ] **Step 1: Write failing spawn tests**

Add inside `tests` module in `import/mod.rs`:

```rust
    fn load_fixture(api: &mut Api) {
        api.load_templates_yaml(FIXTURE_YAML)
            .expect("fixture YAML should load");
    }

    #[test]
    fn spawn_yaml_unknown_template() {
        let mut api = Api::new();
        load_fixture(&mut api);
        let err = api.spawn_yaml("nope").unwrap_err();
        assert!(matches!(err, ImportError::UnknownTemplate(name) if name == "nope"));
    }

    #[test]
    fn spawn_yaml_scout() {
        let mut api = Api::new();
        load_fixture(&mut api);
        let entity = api.spawn_yaml("scout").expect("spawn scout");
        let world = api.core_mut().world_mut();
        let position = world.get::<Position>(entity).expect("position");
        assert_eq!(position.x, 0.0);
        assert_eq!(position.y, 0.0);
        let velocity = world.get::<Velocity>(entity).expect("velocity");
        assert_eq!(velocity.vx, 2.0);
        assert_eq!(velocity.vy, 0.0);
        let faction = world.get::<Faction>(entity).expect("faction");
        assert_eq!(faction.0, 1);
        assert!(world.get::<MoveTarget>(entity).is_err());
    }

    #[test]
    fn spawn_yaml_base() {
        let mut api = Api::new();
        load_fixture(&mut api);
        let entity = api.spawn_yaml("base").expect("spawn base");
        let world = api.core_mut().world_mut();
        assert!(world.get::<Position>(entity).is_err());
        assert!(world.get::<Velocity>(entity).is_err());
        let faction = world.get::<Faction>(entity).expect("faction");
        assert_eq!(faction.0, 2);
    }

    #[test]
    fn spawn_yaml_marker() {
        let mut api = Api::new();
        load_fixture(&mut api);
        let entity = api.spawn_yaml("marker").expect("spawn marker");
        let world = api.core_mut().world_mut();
        assert!(world.get::<Position>(entity).is_err());
        assert!(world.get::<Velocity>(entity).is_err());
        assert!(world.get::<Faction>(entity).is_err());
        assert!(world.get::<MoveTarget>(entity).is_err());
    }

    #[test]
    fn spawn_yaml_twice_same_name() {
        let mut api = Api::new();
        load_fixture(&mut api);
        let e1 = api.spawn_yaml("scout").expect("first scout");
        let e2 = api.spawn_yaml("scout").expect("second scout");
        assert_ne!(e1, e2);
        let world = api.core_mut().world_mut();
        assert!(world.get::<Position>(e1).is_ok());
        assert!(world.get::<Position>(e2).is_ok());
    }

    #[test]
    fn load_templates_yaml_replaces() {
        let mut api = Api::new();
        api.load_templates_yaml("entities:\n  a:\n    faction: 1\n")
            .expect("load A");
        api.load_templates_yaml("entities:\n  b:\n    faction: 2\n")
            .expect("load B");
        assert!(api.spawn_yaml("a").is_err());
        let entity = api.spawn_yaml("b").expect("only B remains");
        let world = api.core_mut().world_mut();
        assert_eq!(world.get::<Faction>(entity).map(|f| f.0), Ok(2));
    }
```

Add `FIXTURE_YAML` const at the top of the `tests` module.

- [ ] **Step 2: Run spawn tests to verify they fail**

Run:

```bash
cargo test -p open_entities spawn_yaml -- --nocapture
```

Expected: failures — stub `spawn_yaml` always returns `TemplatesNotLoaded`, or spawn does not insert components.

- [ ] **Step 3: Implement `spawn_from_doc` and real `spawn_yaml`**

Replace the stub `spawn_yaml` and add helpers in `import/mod.rs`:

```rust
fn spawn_from_doc(world: &mut World, doc: EntitySpawnYaml) -> Entity {
    let mut entity = world.spawn_empty();
    if let Some(position) = doc.position {
        entity.insert(position);
    }
    if let Some(velocity) = doc.velocity {
        entity.insert(velocity);
    }
    if let Some(faction) = doc.faction {
        entity.insert(faction);
    }
    if let Some(move_target) = doc.move_target {
        entity.insert(move_target);
    }
    entity.id()
}

impl Api {
    // load_templates_yaml unchanged from Task 5

    /// Spawns one entity from a previously loaded template by name.
    ///
    /// # Errors
    ///
    /// Returns [`ImportError::TemplatesNotLoaded`] if no successful load yet.
    /// Returns [`ImportError::UnknownTemplate`] if `template_name` is missing.
    pub fn spawn_yaml(&mut self, template_name: &str) -> Result<Entity, ImportError> {
        let templates = self
            .templates
            .as_ref()
            .ok_or(ImportError::TemplatesNotLoaded)?;
        let doc = templates
            .get(template_name)
            .ok_or_else(|| ImportError::UnknownTemplate(template_name.to_owned()))?
            .clone();
        Ok(spawn_from_doc(self.core_mut().world_mut(), doc))
    }
}
```

- [ ] **Step 4: Run all import tests**

Run:

```bash
cargo test -p open_entities import::tests -- --nocapture
```

Expected: all tests in `import::tests` `ok`.

- [ ] **Step 5: Commit**

```bash
git add open-entities-lib/src/import/mod.rs
git commit -m "feat: spawn_yaml instantiates entities from loaded templates"
```

---

### Task 7: Final verification

**Files:** (none — verification only)

- [ ] **Step 1: Run full test suite**

Run from repo root:

```bash
cargo test -p open_entities
```

Expected: all tests pass, 0 failures (includes export + component + import tests).

- [ ] **Step 2: Run clippy with warnings denied**

Run:

```bash
cargo clippy -p open_entities -- -D warnings
```

Expected: clean build, no warnings.

- [ ] **Step 3: Optional — confirm export still works**

Run:

```bash
cargo test -p open_entities world_json -- --nocapture
```

Expected: all `world_json_*` tests `ok` (no regressions from `Deserialize` on components).

---

## Spec coverage checklist

| Spec requirement | Task |
|------------------|------|
| `serde_yaml` dependency | Task 1 |
| `Deserialize` on all four components | Task 2 |
| `import/mod.rs` with `ImportError` | Task 3 |
| `EntitySpawnYaml` + `deny_unknown_fields` | Task 4 |
| `Api.templates: Option<EntityTemplates>`, `new()` → `None` | Task 4 |
| `load_templates_yaml` parses `entities:` root | Task 5 |
| Atomic load (failed load keeps previous map) | Task 5 (`load_templates_yaml_failed_replaces_keeps_previous`) |
| Reload replaces entire map | Task 6 (`load_templates_yaml_replaces`) |
| `spawn_yaml` → `TemplatesNotLoaded` / `UnknownTemplate` | Tasks 5–6 |
| `spawn_empty` + `insert` per component | Task 6 |
| Empty template `{}` valid | Task 6 (`spawn_yaml_marker`) |
| Unknown component key fails load | Task 5 |
| `pub mod import`, `pub use ImportError` | Task 3 |
| All nine spec test rows | Tasks 5–6 |
| `cargo test` + `clippy -D warnings` | Task 7 |
| No wasm / examples / README / inheritance | Out of scope |

## Placeholder scan

No TBD, "implement later", or "similar to Task N" steps. All test and implementation code is inlined above.
