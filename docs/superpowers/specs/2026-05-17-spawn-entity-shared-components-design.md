# Design: `spawn_entity` and Shared `EntityComponents`

**Date:** 2026-05-17  
**Status:** Draft (brainstorming)  
**Depends on:** [YAML Component Spawn Import](2026-05-16-yaml-spawn-import-design.md), [YAML Template Inheritance](2026-05-17-yaml-template-inheritance-design.md)  
**Scope:** Extract public `EntityComponents` shared by import, export, and spawn overrides; rename `spawn_yaml` → `spawn_entity` with optional per-spawn component overrides. No `wasm-bindings` in this increment.

## Summary

Today `EntityComponents` lives privately in `import/mod.rs` and duplicates the gameplay field set embedded in export’s private `EntityExport` (which also adds `id` and `entity_type`). Spawn is `spawn_yaml(name)` with no runtime overrides.

This increment:

1. Moves **`EntityComponents`** to a dedicated module, public, with **`Serialize` + `Deserialize`** and the same four optional fields as export v2 / YAML templates.
2. Refactors **`EntityExport`** to `#[serde(flatten)]` the shared bundle (no behavioral change to `world_json` JSON shape).
3. Renames **`spawn_yaml` → `spawn_entity`**, taking `overrides: EntityComponents`.
4. Applies overrides with existing **component-level merge** semantics: each `Some` field in `overrides` replaces the template value; `None` leaves the template unchanged.

`entity_type` remains spawn-injected from the template name, not part of `EntityComponents`. `EntityExport::id` remains export-only.

## Goals

- One struct to extend when adding importable/exportable components.
- Spawn-time placement/faction overrides without editing YAML.
- Clear Rust API for future WASM (`serde_wasm_bindgen::from_value` → `EntityComponents`).
- Symmetric JSON field shapes with export v2 (`position`, `velocity`, `faction`, `move_target`).

## Non-Goals

- `wasm-bindings` crate or JS API in this PR.
- `SpawnEntityRequest` with `type` inside the same object (WASM can add later via `#[serde(flatten)]`).
- Flat `posX` / `posY` in core (use nested `position: { x, y }` — matches export YAML/JSON).
- Override of `entity_type` at spawn.
- `spawn_yaml` deprecation alias.
- New components beyond the existing four.
- Deep merge inside a component struct.

## Decisions (Brainstorming)

| Topic | Choice | Rationale |
|-------|--------|-----------|
| Shared type | Public `EntityComponents` | Single source of truth for YAML, spawn overrides, export gameplay fields |
| Not `EntityExport` | Export row includes `id` + `entity_type` | Wrong semantics for spawn input; confuses WASM clients |
| Spawn API name | `spawn_entity` | Spawns ECS entity from loaded template, not “raw YAML” |
| Override param | `overrides: EntityComponents` | Reuses merge; `Default::default()` = old `spawn_yaml(name)` behavior |
| Override semantics | `merge_components(template, &overrides)` — override wins when `Some` | User choice: spawn args always replace template when provided |
| `entity_type` | Set from `template_name` after merge; never from overrides | Unchanged from spawn-import |
| Module location | `src/entity_components.rs` | Avoid `export` ↔ `import` cycle |
| Re-export | `pub use entity_components::EntityComponents` in `lib.rs` | Ergonomic `open_entities::EntityComponents` |
| Field visibility | `pub` on all four `Option<…>` fields | Rust callers build overrides inline |
| Serde | `Serialize`, `Deserialize`, `deny_unknown_fields`, `Default` | Strict YAML + future WASM JSON |
| Export refactor | `EntityExport { id, #[serde(flatten)] components, entity_type }` | DRY; JSON output unchanged |
| Breaking change | Hard rename; update tests, example, `Makefile` | Early project; no alias |
| `load_templates_yaml` | Unchanged name and behavior | Still loads YAML only |

## Types

### `entity_components.rs`

```rust
/// Gameplay components shared by YAML templates, spawn overrides, and export (flattened).
#[derive(Clone, Default, PartialEq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntityComponents {
    pub position: Option<Position>,
    pub velocity: Option<Velocity>,
    pub faction: Option<Faction>,
    pub move_target: Option<MoveTarget>,
}

/// Component-level merge: `child` wins when `Some`.
pub fn merge_components(parent: &EntityComponents, child: &EntityComponents) -> EntityComponents;
```

`merge_components` moves from `import/mod.rs` to this module (or stays `pub(crate)` here, used by import resolve + spawn).

### Export (`export/mod.rs`)

```rust
struct EntityExport {
    id: EntityIdExport,
    #[serde(flatten)]
    components: EntityComponents,
    #[serde(skip_serializing_if = "Option::is_none")]
    entity_type: Option<EntityType>,
}
```

Construction when exporting: fill `components` from query `Option<&T>` fields; set `entity_type` separately.

### Import (`import/mod.rs`)

- Remove local `EntityComponents` definition; `use crate::entity_components::{EntityComponents, merge_components}`.
- `EntitySpawnYaml` remains `type EntitySpawnYaml = EntityComponents`.
- `EntityTemplateRaw` still `#[serde(flatten)] components: EntityComponents`.

## Public API

```rust
impl Api {
  // unchanged
  pub fn load_templates_yaml(&mut self, yaml: &str) -> Result<(), ImportError>;

  /// Spawns one entity from a loaded template, applying optional component overrides.
  pub fn spawn_entity(
      &mut self,
      template_name: &str,
      overrides: EntityComponents,
  ) -> Result<Entity, ImportError>;
}
```

### Spawn algorithm

1. `templates = self.templates.as_ref().ok_or(TemplatesNotLoaded)?`
2. `base = templates.get(template_name).ok_or(UnknownTemplate)?.clone()`
3. `doc = merge_components(&base, &overrides)`
4. `spawn_from_doc(world, template_name, &doc)` — unchanged inserts + `EntityType(template_name)`

### Rust usage

```rust
use open_entities::{Api, EntityComponents};
use open_entities::components::{Faction, Position};

api.spawn_entity("scout", EntityComponents::default())?;

api.spawn_entity(
    "scout",
    EntityComponents {
        faction: Some(Faction(99)),
        position: Some(Position { x: 100.0, y: 200.0 }),
        ..Default::default()
    },
)?;
```

## Repository layout

```text
open-entities-lib/src/
├── entity_components.rs   # NEW — EntityComponents, merge_components
├── lib.rs                 # mod entity_components; pub use EntityComponents
├── import/mod.rs          # spawn_entity; uses shared types
├── export/mod.rs          # EntityExport flattens EntityComponents
└── examples/
    spawn_entity.rs        # renamed from spawn_yaml.rs
```

Root `Makefile`: `EXAMPLE ?= spawn_entity`.

## Errors

Unchanged `ImportError` variants. No new error for “partial position” — `Position` is all-or-nothing in overrides (both `x` and `y` via struct).

## Testing

| Test | Asserts |
|------|---------|
| `spawn_entity_without_load` | `TemplatesNotLoaded` (rename from `spawn_yaml_without_load`) |
| `spawn_entity_unknown_template` | `UnknownTemplate` |
| `spawn_entity_no_overrides` | Same components as template (`EntityComponents::default()`) |
| `spawn_entity_overrides_faction` | Template `faction: 1`, override `Some(Faction(99))` → world has 99 |
| `spawn_entity_overrides_position` | Template position replaced |
| `spawn_entity_marker_empty` | `{}` template still works |
| `spawn_entity_twice_same_name` | Two entities |
| Existing inheritance / load tests | Rename `spawn_yaml` calls → `spawn_entity(..., Default::default())` |
| Export regression | `world_json` tests unchanged (JSON byte-for-byte equivalent) |
| `merge_components` unit tests | Move with type or stay in import `merge_tests` module |

## Future: WASM

```rust
// wasm-bindings (later)
let overrides: EntityComponents = serde_wasm_bindgen::from_value(opts)?;
api.spawn_entity(&template_name, overrides)?;
```

JS shape matches export: `{ faction: 2, position: { x: 10, y: 5 } }`. Template name remains a separate argument (or a future `SpawnEntityRequest` with `#[serde(flatten)]`).

## Rejected alternatives

| Alternative | Why not |
|-------------|---------|
| `EntityExport` for overrides | Includes `id` / `entity_type`; export-only row |
| `SpawnEntityOptions` duplicate struct | Extra type to keep in sync |
| Flat `faction` + `pos_x`/`pos_y` in core | Breaks symmetry with export v2; serde only in WASM if needed later |
| Keep `spawn_yaml` alias | User chose hard rename |

## Evolution

- `SpawnEntityRequest { #[serde(rename = "type")] template_name, #[serde(flatten)] overrides }` for one JS object.
- `wasm-bindings` crate.
- More components: add field once on `EntityComponents`, then import merge, export flatten, spawn insert.
