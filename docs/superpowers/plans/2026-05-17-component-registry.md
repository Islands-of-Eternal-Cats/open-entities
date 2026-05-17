# Component Registry and `register_component!` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Introduce a compile-time component registry (`define_registered_components!` / `register_component!`) so merge, spawn, and export wiring come from one list; add `Health` as proof; bump `world_json` to schema version 3.

**Architecture:** New `component_registry/` module holds `macro_rules!` that expand `EntityComponents`, `merge_components`, `spawn_registered_components`, export query helpers, and inclusion checks from `registered.rs`. `entity_components.rs` becomes a thin re-export for stable paths. `import` and `export` call generated helpers instead of per-component `if` lists. `EntityType` stays outside the registry (spawn-injected, export-only field).

**Tech Stack:** Rust 2024, `bevy_ecs 0.18`, `serde`, `serde_yaml 0.9`, `serde_json` (existing; no new deps).

**Spec:** `docs/superpowers/specs/2026-05-17-component-registry-design.md`

---

## File map

| File | Responsibility |
|------|----------------|
| `open-entities-lib/src/component_registry/macros.rs` | `register_component!` misuse guard + `define_registered_components!` expansion |
| `open-entities-lib/src/component_registry/registered.rs` | Single list of `register_component!(field, Type);` lines |
| `open-entities-lib/src/component_registry/mod.rs` | `#[macro_use] mod macros`, `mod registered`, re-exports |
| `open-entities-lib/src/components/health.rs` | **NEW** — `Health { current, max }` + ECS round-trip test |
| `open-entities-lib/src/components/mod.rs` | `pub mod health;` + `pub use health::Health` |
| `open-entities-lib/src/entity_components.rs` | Re-export registry types/fns; keep existing merge unit tests |
| `open-entities-lib/src/lib.rs` | `mod component_registry;` |
| `open-entities-lib/src/import/mod.rs` | `spawn_from_doc` → `spawn_registered_components` |
| `open-entities-lib/src/export/mod.rs` | Use `WorldExportQuery` + helpers; `SCHEMA_VERSION = 3`; v3 tests |

**Out of scope:** `wasm-bindings`, JSON import, `inventory` crate, README/CI, proc-macro crate.

---

### Task 1: Component registry macros (four existing components)

**Files:**
- Create: `open-entities-lib/src/component_registry/macros.rs`
- Create: `open-entities-lib/src/component_registry/registered.rs`
- Create: `open-entities-lib/src/component_registry/mod.rs`
- Modify: `open-entities-lib/src/lib.rs`

- [ ] **Step 1: Create `component_registry/macros.rs`**

```rust
/// Standalone `register_component!` is forbidden — only valid inside `define_registered_components!`.
#[macro_export]
macro_rules! register_component {
    ($field:ident, $ty:ty) => {
        compile_error!(
            "register_component! must only appear inside define_registered_components! { ... }"
        );
    };
}

/// Expands the registry list into `EntityComponents`, merge/spawn/export helpers.
#[macro_export]
macro_rules! define_registered_components {
    (
        $(
            register_component!($field:ident, $ty:ty);
        )*
    ) => {
        /// Gameplay components shared by YAML templates, spawn overrides, and export (flattened).
        #[derive(Clone, Copy, Default, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        pub struct EntityComponents {
            $(
                #[serde(skip_serializing_if = "Option::is_none")]
                pub $field: Option<$ty>,
            )*
        }

        /// Component-level merge: `child` wins when `Some`.
        pub fn merge_components(
            parent: &EntityComponents,
            child: &EntityComponents,
        ) -> EntityComponents {
            EntityComponents {
                $(
                    $field: child.$field.or(parent.$field),
                )*
            }
        }

        /// Inserts each `Some` registered component on the entity under construction.
        pub fn spawn_registered_components(
            entity: &mut bevy_ecs::prelude::EntityCommands<'_>,
            doc: &EntityComponents,
        ) {
            $(
                if let Some(value) = doc.$field {
                    entity.insert(value);
                }
            )*
        }

        /// True if any registered field is `Some`.
        pub fn entity_components_has_any(doc: &EntityComponents) -> bool {
            false $(|| doc.$field.is_some())*
        }

        /// Bevy `Query` tuple for export: registered `Option<&T>` plus `EntityType`.
        pub type WorldExportQuery<'w> = (
            bevy_ecs::prelude::Entity,
            $(
                Option<&'w $ty>,
            )*
            Option<&'w crate::components::EntityType>,
        );

        /// True if the entity has at least one registered gameplay component.
        pub fn registered_components_present(
            $($field: Option<&$ty>,)*
        ) -> bool {
            false $(|| $field.is_some())*
        }

        /// Builds export row `EntityComponents` from query `Option` references.
        pub fn entity_components_from_query(
            $($field: Option<&$ty>,)*
        ) -> EntityComponents {
            EntityComponents {
                $($field: $field.copied(),)*
            }
        }
    };
}
```

- [ ] **Step 2: Create `component_registry/registered.rs` (four components only — refactor baseline)**

```rust
use crate::components::{Faction, MoveTarget, Position, Velocity};

define_registered_components! {
    register_component!(position, Position);
    register_component!(velocity, Velocity);
    register_component!(faction, Faction);
    register_component!(move_target, MoveTarget);
}
```

- [ ] **Step 3: Create `component_registry/mod.rs`**

```rust
#[macro_use]
mod macros;

mod registered;

pub use registered::{
    entity_components_from_query, entity_components_has_any, merge_components,
    registered_components_present, spawn_registered_components, EntityComponents,
    WorldExportQuery,
};
```

- [ ] **Step 4: Wire crate module in `lib.rs`**

After `pub mod import;` add:

```rust
mod component_registry;
```

- [ ] **Step 5: Run compile check (macros only, entity_components still hand-written)**

Run: `cargo check -p open_entities 2>&1`
Expected: PASS (registry module compiles; not yet used by `entity_components.rs`)

- [ ] **Step 6: Commit**

```bash
git add open-entities-lib/src/component_registry/ open-entities-lib/src/lib.rs
git commit -m "feat: add component registry macros and four-component list"
```

---

### Task 2: Switch `entity_components` to registry-generated types

**Files:**
- Modify: `open-entities-lib/src/entity_components.rs`

- [ ] **Step 1: Replace hand-written struct and merge with re-exports**

Replace entire `entity_components.rs` with:

```rust
pub use crate::component_registry::{
    entity_components_has_any, merge_components, EntityComponents,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Faction, Velocity};

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

- [ ] **Step 2: Run merge and import tests**

Run: `cargo test -p open_entities entity_components merge spawn_entity load_templates 2>&1`
Expected: PASS (same behavior, generated struct matches prior fields)

- [ ] **Step 3: Commit**

```bash
git add open-entities-lib/src/entity_components.rs
git commit -m "refactor: generate EntityComponents from component registry"
```

---

### Task 3: Registry-driven spawn in import

**Files:**
- Modify: `open-entities-lib/src/import/mod.rs:151-167`

- [ ] **Step 1: Add import and replace manual inserts**

Add at top of `import/mod.rs` (with other `crate::` imports):

```rust
use crate::component_registry::spawn_registered_components;
```

Replace `spawn_from_doc` body with:

```rust
fn spawn_from_doc(world: &mut World, template_name: &str, doc: &EntityComponents) -> Entity {
    let mut entity = world.spawn_empty();
    spawn_registered_components(&mut entity, doc);
    entity.insert(EntityType(template_name.to_owned()));
    entity.id()
}
```

- [ ] **Step 2: Run import/spawn test suite**

Run: `cargo test -p open_entities import:: 2>&1`
Expected: PASS (all template inheritance and override tests unchanged)

- [ ] **Step 3: Commit**

```bash
git add open-entities-lib/src/import/mod.rs
git commit -m "refactor: spawn entities via registry spawn_registered_components"
```

---

### Task 4: Add `Health` component

**Files:**
- Create: `open-entities-lib/src/components/health.rs`
- Modify: `open-entities-lib/src/components/mod.rs`

- [ ] **Step 1: Write failing ECS round-trip test and type**

Create `open-entities-lib/src/components/health.rs`:

```rust
use bevy_ecs::prelude::Component;
use serde::{Deserialize, Serialize};

/// Hit points for a unit or structure.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Health {
    pub current: u32,
    pub max: u32,
}

#[cfg(test)]
mod tests {
    use super::Health;
    use bevy_ecs::prelude::*;

    #[test]
    fn health_component_round_trip() {
        let mut world = World::new();
        world.spawn(Health {
            current: 80,
            max: 100,
        });

        let mut query = world.query::<&Health>();
        let mut count = 0;
        for health in query.iter(&world) {
            assert_eq!(health.current, 80);
            assert_eq!(health.max, 100);
            count += 1;
        }
        assert_eq!(count, 1);
    }
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test -p open_entities health_component_round_trip -- --nocapture`
Expected: PASS

- [ ] **Step 3: Export `Health` from `components/mod.rs`**

```rust
pub mod health;
```

After existing `pub use` lines add:

```rust
pub use health::Health;
```

- [ ] **Step 4: Commit**

```bash
git add open-entities-lib/src/components/health.rs open-entities-lib/src/components/mod.rs
git commit -m "feat: add Health component with ECS round-trip test"
```

---

### Task 5: Register `Health` and add merge test

**Files:**
- Modify: `open-entities-lib/src/component_registry/registered.rs`
- Modify: `open-entities-lib/src/entity_components.rs` (tests section)

- [ ] **Step 1: Add failing merge test for health**

Append to `entity_components.rs` `mod tests`:

```rust
    use crate::components::Health;

    #[test]
    fn merge_health_child_wins() {
        let parent = EntityComponents {
            health: Some(Health {
                current: 50,
                max: 100,
            }),
            ..Default::default()
        };
        let child = EntityComponents {
            health: Some(Health {
                current: 10,
                max: 10,
            }),
            ..Default::default()
        };
        let merged = merge_components(&parent, &child);
        assert_eq!(
            merged.health,
            Some(Health {
                current: 10,
                max: 10,
            })
        );
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p open_entities merge_health_child_wins -- --nocapture`
Expected: FAIL — `health` field missing on `EntityComponents`

- [ ] **Step 3: Register `Health` in `registered.rs`**

Add import and registry line:

```rust
use crate::components::{Faction, Health, MoveTarget, Position, Velocity};

define_registered_components! {
    register_component!(position, Position);
    register_component!(velocity, Velocity);
    register_component!(faction, Faction);
    register_component!(move_target, MoveTarget);
    register_component!(health, Health);
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p open_entities merge_health_child_wins -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add open-entities-lib/src/component_registry/registered.rs open-entities-lib/src/entity_components.rs
git commit -m "feat: register Health in component registry"
```

---

### Task 6: Health YAML spawn and template inheritance tests

**Files:**
- Modify: `open-entities-lib/src/import/mod.rs` (test module)

- [ ] **Step 1: Write failing `spawn_entity_overrides_health` test**

Add to `import/mod.rs` `#[cfg(test)] mod tests` (after existing override tests):

```rust
    use crate::components::Health;

    #[test]
    fn spawn_entity_overrides_health() {
        let mut api = Api::new();
        api.load_templates_yaml(
            r#"
entities:
  grunt:
    health:
      current: 50
      max: 100
"#,
        )
        .expect("load templates");
        let entity = api
            .spawn_entity(
                "grunt",
                EntityComponents {
                    health: Some(Health {
                        current: 10,
                        max: 10,
                    }),
                    ..Default::default()
                },
            )
            .expect("spawn with health override");
        let world = api.core_mut().world_mut();
        let health = world.get::<Health>(entity).expect("health");
        assert_eq!(health.current, 10);
        assert_eq!(health.max, 10);
    }
```

- [ ] **Step 2: Run test to verify it fails then passes after registry already has health**

Run: `cargo test -p open_entities spawn_entity_overrides_health -- --nocapture`
Expected: PASS (if Task 5 done; otherwise fail until `health` field exists)

- [ ] **Step 3: Write failing `inherit_health_via_template` test**

```rust
    #[test]
    fn inherit_health_via_template() {
        let mut api = Api::new();
        api.load_templates_yaml(
            r#"
entities:
  base_unit:
    health:
      current: 100
      max: 100
  elite:
    template: base_unit
    health:
      current: 80
      max: 100
"#,
        )
        .expect("load templates");
        let entity = api
            .spawn_entity("elite", EntityComponents::default())
            .expect("spawn elite");
        let world = api.core_mut().world_mut();
        let health = world.get::<Health>(entity).expect("health");
        assert_eq!(health.current, 80);
        assert_eq!(health.max, 100);
    }
```

- [ ] **Step 4: Run inheritance test**

Run: `cargo test -p open_entities inherit_health_via_template -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add open-entities-lib/src/import/mod.rs
git commit -m "test: health spawn override and template inheritance"
```

---

### Task 7: `world_json` schema v3 via registry export helpers

**Files:**
- Modify: `open-entities-lib/src/export/mod.rs`

- [ ] **Step 1: Update imports and schema version**

Replace component imports and add registry helpers:

```rust
use crate::components::EntityType;
use crate::component_registry::{
    entity_components_from_query, registered_components_present, WorldExportQuery,
};
```

Change:

```rust
const SCHEMA_VERSION: u32 = 3;
```

Update `Api::world_json` doc comment to mention schema version 3 and registered components (including `Health`).

- [ ] **Step 2: Replace `world_json_from_world` query/filter/row build**

Replace the function body with:

```rust
fn world_json_from_world(world: &mut World) -> Result<String, ExportError> {
    let mut query = world.query::<WorldExportQuery<'_>>();
    let entities = query
        .iter(world)
        .filter_map(
            |(entity, position, velocity, faction, move_target, health, entity_type)| {
                if !registered_components_present(
                    position,
                    velocity,
                    faction,
                    move_target,
                    health,
                ) && entity_type.is_none()
                {
                    return None;
                }
                Some(EntityExport {
                    id: EntityIdExport {
                        index: entity.index_u32(),
                        generation: entity.generation().to_bits(),
                    },
                    components: entity_components_from_query(
                        position,
                        velocity,
                        faction,
                        move_target,
                        health,
                    ),
                    entity_type: entity_type.cloned(),
                })
            },
        )
        .collect();

    let payload = WorldExport {
        version: SCHEMA_VERSION,
        entities,
    };

    Ok(serde_json::to_string(&payload)?)
}
```

**Note:** The destructuring tuple order must match `WorldExportQuery` field order from `registered.rs` (position, velocity, faction, move_target, health, then entity_type in the type alias is last — the macro emits registered fields first, then `EntityType` in the type; the `filter_map` closure receives `Entity` first, then each registered `Option`, then `entity_type`).

- [ ] **Step 3: Update existing export tests — change `version` from 2 to 3**

In `export/mod.rs` tests, replace every:

```rust
assert_eq!(value["version"], 2);
```

with:

```rust
assert_eq!(value["version"], 3);
```

(Six occurrences in current file.)

- [ ] **Step 4: Run export tests (expect pass except new v3 health tests not yet added)**

Run: `cargo test -p open_entities export:: -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add open-entities-lib/src/export/mod.rs
git commit -m "feat: world_json schema v3 with registry export helpers"
```

---

### Task 8: `world_json` v3 health-specific export tests

**Files:**
- Modify: `open-entities-lib/src/export/mod.rs` (test module)

- [ ] **Step 1: Add `world_json_v3_version` test**

```rust
    use crate::components::Health;

    #[test]
    fn world_json_v3_version() {
        let mut api = Api::new();
        let json = api.world_json().expect("serialize empty world");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("exported JSON should parse");
        assert_eq!(value["version"], 3);
    }
```

- [ ] **Step 2: Add `world_json_v3_health_only_entity` test**

```rust
    #[test]
    fn world_json_v3_health_only_entity() {
        let mut api = Api::new();
        api.core_mut().world_mut().spawn(Health {
            current: 80,
            max: 100,
        });

        let json = api.world_json().expect("serialize world");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("exported JSON should parse");

        assert_eq!(value["version"], 3);
        let entities = value["entities"].as_array().expect("entities array");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0]["health"]["current"], 80);
        assert_eq!(entities[0]["health"]["max"], 100);
        assert!(entities[0].get("position").is_none());
    }
```

- [ ] **Step 3: Add `world_json_v3_optional_keys` test**

```rust
    #[test]
    fn world_json_v3_optional_keys() {
        let mut api = Api::new();
        api.core_mut()
            .world_mut()
            .spawn(Position { x: 1.0, y: 2.0 });

        let json = api.world_json().expect("serialize world");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("exported JSON should parse");

        let entities = value["entities"].as_array().expect("entities array");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0]["position"]["x"], 1.0);
        assert!(entities[0].get("health").is_none());
    }
```

- [ ] **Step 4: Run new tests**

Run: `cargo test -p open_entities world_json_v3 -- --nocapture`
Expected: PASS

- [ ] **Step 5: Run full export regression**

Run: `cargo test -p open_entities export:: -- --nocapture`
Expected: PASS (position/faction/velocity keys unchanged aside from `version: 3`)

- [ ] **Step 6: Commit**

```bash
git add open-entities-lib/src/export/mod.rs
git commit -m "test: world_json v3 health export and optional keys"
```

---

### Task 9: Final verification

**Files:** (none — verification only)

- [ ] **Step 1: Full test suite**

Run: `cargo test -p open_entities`
Expected: all tests PASS

- [ ] **Step 2: Clippy with warnings denied**

Run: `cargo clippy -p open_entities -- -D warnings`
Expected: no warnings

- [ ] **Step 3: Commit (only if verification fixes were needed)**

```bash
git add -A
git commit -m "chore: fix clippy after component registry"
```

---

## Spec coverage (self-review)

| Spec requirement | Task |
|------------------|------|
| `component_registry/` with macros + `registered.rs` | 1 |
| `register_component!` outside wrapper → `compile_error!` | 1 (`macros.rs`) |
| Generated `EntityComponents`, `merge_components`, spawn, export helpers | 1–2, 7 |
| `entity_components.rs` thin re-export | 2 |
| `spawn_from_doc` uses registry | 3 |
| `Health { current, max }` + public `components::Health` | 4–5 |
| One-line `register_component!(health, Health)` | 5 |
| YAML/spawn merge semantics unchanged | 2–3, 6 |
| `world_json` version 3 | 7–8 |
| Health-only export inclusion | 7–8 |
| Existing four-component merge/spawn/export tests pass | 2–3, 7–9 |
| `EntityType` not in registry | 3, 7 |
| No new errors / deps | all tasks |
| Optional macro misuse compile-test | skipped (no `trybuild` in repo; optional follow-up) |

## Placeholder scan

No TBD/TODO/similar-to-task placeholders. Each code step includes full snippets or exact file replacements.

## Type consistency

- Field idents: `position`, `velocity`, `faction`, `move_target`, `health` — consistent across macro, YAML keys, tests, export destructuring.
- `WorldExportQuery` field order matches `entity_components_from_query` and `registered_components_present` argument order.
- `EntityComponents` remains `Copy` (all registered types including `Health` are `Copy`).
