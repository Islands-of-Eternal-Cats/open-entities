# Design: Component Registry and `register_component!` Macro

**Date:** 2026-05-17  
**Status:** Approved (brainstorming)  
**Depends on:** [YAML Template Inheritance](2026-05-17-yaml-template-inheritance-design.md), [spawn_entity and Shared EntityComponents](2026-05-17-spawn-entity-shared-components-design.md), [RTS Components and World JSON Export v2](2026-05-16-rts-components-export-v2-design.md)  
**Scope:** Introduce a compile-time component registry driven by `register_component!` inside `define_registered_components!`, refactor merge/spawn/export to use it, and add `Health` as proof. Bump `world_json` to schema version 3. No WASM, JSON import, or alternate merge rules (inventory).

## Summary

Today each gameplay component is wired manually in four places: `EntityComponents` fields, `merge_components`, `spawn_from_doc` inserts, and export query/row assembly. Adding a component requires touching all of them.

This increment:

1. Adds **`component_registry/`** with `macro_rules!` macros that generate `EntityComponents`, `merge_components`, spawn helpers, and export query/assembly from a single list in `registered.rs`.
2. Keeps **`EntityComponents`** as the public, serde-backed bundle (YAML, spawn overrides, export flatten).
3. Adds **`Health { current, max }`** via one `register_component!(health, Health)` line plus `components/health.rs`.
4. Bumps **`world_json`** to **schema version 3**, extending the export inclusion rule to registered components (including health-only entities).

`EntityType` stays outside the registry (spawn-injected from template name; export-only field on `EntityExport`).

## Goals

- Add new importable/exportable components by editing **one registry list** (+ component type file), not four manual code paths.
- Preserve existing YAML/spawn semantics: component-level merge (`child.field.or(parent.field)`), template inheritance, `spawn_entity` overrides.
- Prove the registry with **`Health`** and **world_json v3**.
- Keep typed public API (`EntityComponents`, `open_entities::components::Health`) for Rust and future WASM.

## Non-Goals

- Proc-macro crate (`macro_rules!` only, in `open-entities-lib`).
- Per-file `register_component!` with `inventory` / link-time collection.
- `HashMap<String, serde_json::Value>` bundles (loses `deny_unknown_fields` and type safety).
- Inventory, equipment, clothing, or deep merge inside component structs.
- Different merge rules per component (all use whole-component `child.or(parent)` for this increment).
- `wasm-bindings`, `world_from_json`, `spawn_yaml` alias.
- Cross-file YAML `template` references.
- README / CI updates (optional follow-up chore).

## Decisions (Brainstorming)

| Topic | Choice | Rationale |
|-------|--------|-----------|
| Motivation | Refactor + easier growth | User choice A; not dynamic mods yet |
| Proof component | `Health` in same PR | User choice B |
| `Health` shape | `{ current, max }` both `u32` | User choice B |
| Export schema | **`version: 3`** | User choice B; explicit breaking bump |
| Registry mechanism | `register_component!` inside `define_registered_components!` | User choice; one list, generated wiring |
| `EntityComponents` | Remains public struct | YAML, overrides, export flatten unchanged at API level |
| YAML/JSON keys | Field ident = key (`health`, `position`, …) | Matches existing v2 names |
| `Health` merge | Whole struct: `child.health.or(parent.health)` | Same as `position`; no partial `current`/`max` |
| Export inclusion | Entity has ≥1 **registered** gameplay component **or** `EntityType` | Extends v2; health-only entities now exported |
| `EntityType` | Not in registry | Spawn/import semantics unchanged |
| Serde on bundle | `deny_unknown_fields` on `EntityComponents` | Unknown YAML keys still fail at load |

## Repository Layout

```text
open-entities-lib/src/
├── component_registry/
│   ├── mod.rs              # spawn_registered, inclusion helpers, re-exports
│   ├── macros.rs           # define_registered_components!, register_component!
│   └── registered.rs       # single list of register_component!(…) lines
├── components/
│   ├── health.rs           # NEW
│   └── mod.rs              # pub use Health
├── entity_components.rs    # thin re-export or generated types (see below)
├── import/mod.rs           # spawn_from_doc → registry spawn
└── export/mod.rs           # v3 query + registry row build
```

## Macros

### Usage (only valid inside `define_registered_components!`)

```rust
// component_registry/registered.rs
define_registered_components! {
    register_component!(position, Position);
    register_component!(velocity, Velocity);
    register_component!(faction, Faction);
    register_component!(move_target, MoveTarget);
    register_component!(health, Health);
}
```

`register_component!` invoked outside `define_registered_components!` must fail with a clear `compile_error!`.

### Generated artifacts

`define_registered_components! { … }` expands to:

1. **`EntityComponents`** — `pub field: Option<Type>` per entry, with `#[serde(skip_serializing_if = "Option::is_none")]`, plus `Clone`, `Copy`, `Default`, `PartialEq`, `Debug`, `Serialize`, `Deserialize`, `deny_unknown_fields`.
2. **`merge_components`** — per field: `child.field.or(parent.field)`.
3. **`spawn_registered_components`** — `EntityWorldWriter`: for each `Some(v)`, `entity.insert(v)`.
4. **`entity_components_has_any`** — true if any registered field is `Some` (used for tests/helpers).
5. **Export support** — either generated query tuple + row mapping, or functions called from `export/mod.rs` that the macro documents (implementation may keep query in `export/mod.rs` but row fill and inclusion check must not duplicate per-component `if` lists).

### Requirements on registered types

Each `$ty` must implement:

- `bevy_ecs::prelude::Component`
- `Clone + Copy`
- `serde::Serialize + serde::Deserialize` (for YAML templates and overrides)
- Component-specific serde attributes live on the type (e.g. `#[serde(transparent)]` on `Faction`), not in the macro.

## Components

### `Health` (`components/health.rs`)

```rust
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Health {
    pub current: u32,
    pub max: u32,
}
```

Public path: `open_entities::components::Health`.

YAML / spawn overrides / export:

```yaml
health:
  current: 80
  max: 100
```

```json
"health": { "current": 80, "max": 100 }
```

### Existing four components

No shape changes. Registry replaces manual duplication only.

## Import and Spawn

- `EntityTemplateRaw` continues `#[serde(flatten)] components: EntityComponents`.
- Template resolution (`resolve_template`, `merge_components`) unchanged in behavior; `merge_components` body is macro-generated.
- `spawn_from_doc` calls `spawn_registered_components` then `entity.insert(EntityType(template_name))`.
- `spawn_entity(template, overrides)` unchanged algorithm: `merge_components(&base, &overrides)` then spawn.

## Export (`world_json` v3)

### Schema

```json
{
  "version": 3,
  "entities": [
    {
      "id": { "index": 0, "generation": 0 },
      "position": { "x": 1.0, "y": 2.0 },
      "health": { "current": 80, "max": 100 },
      "entity_type": "scout"
    }
  ]
}
```

### Rules

| Rule | v2 | v3 |
|------|----|----|
| `version` field | `2` | `3` |
| Inclusion | ≥1 of position, velocity, faction, move_target, or entity_type | ≥1 of **any registered** component (five gameplay types) **or** entity_type |
| Missing component keys | Omitted | Omitted |
| `id` shape | `{ index, generation }` | Unchanged |

### Inclusion examples

| Entity components | v2 export? | v3 export? |
|-------------------|------------|------------|
| Only `Health` | No | **Yes** |
| `Position` only | Yes | Yes |
| Empty (no components) | No | No |
| Only `EntityType` | Yes | Yes |

### Implementation notes

- `SCHEMA_VERSION = 3`.
- Prefer one `Query` pass with `(Entity, Option<&T>, …, Option<&EntityType>)` for all registered types; macro may generate the tuple type list to avoid drift.
- `EntityExport` keeps `#[serde(flatten)] components: EntityComponents` and separate `entity_type`.

## Public API

Unchanged names:

```rust
impl Api {
    pub fn load_templates_yaml(&mut self, yaml: &str) -> Result<(), ImportError>;
    pub fn spawn_entity(
        &mut self,
        template_name: &str,
        overrides: EntityComponents,
    ) -> Result<Entity, ImportError>;
    pub fn world_json(&mut self) -> Result<String, ExportError>;
}
```

New public type: `open_entities::components::Health`.

`EntityComponents` gains `pub health: Option<Health>`.

## Errors

No new `ImportError` or `ExportError` variants. Invalid YAML keys still surface as `ImportError::Yaml` via serde `deny_unknown_fields`.

## Testing

| Test | Asserts |
|------|-----------|
| Existing merge tests (4 components) | Same results after macro refactor |
| Existing spawn/inheritance/import tests | Pass with generated `merge_components` / spawn |
| `health_component_round_trip` | ECS insert/query in `components/health.rs` |
| `merge_health_child_wins` | Whole-struct override |
| `spawn_entity_overrides_health` | Override replaces template health |
| `inherit_health_via_template` | YAML `template` + `health` on child |
| `world_json_v3_version` | `version == 3` |
| `world_json_v3_health_only_entity` | Single-component health row in `entities` |
| `world_json_v3_optional_keys` | Entity with position only omits `health` key |
| Export regression (position, faction, velocity) | Keys and values unchanged aside from `version` |
| Macro misuse | `register_component!` outside wrapper fails compile (optional compile-test) |

```bash
cargo test -p open_entities
cargo clippy -p open_entities -- -D warnings
```

Update tests that assert `version == 2` to expect `3`.

## Alternatives Considered

| Approach | Why not (this increment) |
|----------|---------------------------|
| Manual registry + hand-written `EntityComponents` fields | User chose macro for single-list ergonomics |
| `version: 2` + additive `health` key | User chose explicit v3 bump |
| v3 without health-only inclusion (option C) | User chose full inclusion extension with v3 |
| Dynamic `HashMap` registry | Type safety, WASM, serde strictness |
| `inventory` crate for scattered `register_component!` | Extra dependency; one file list is enough at current scale |

## Evolution

When merge rules diverge (inventory slots, equipment):

- Add separate YAML sections and `import/inventory.rs` (or similar), not new `register_component!` entries with different merge in the same macro.
- Registry remains for homogeneous RTS components with `child.or(parent)` at top level.
- Consider a **component import registry** trait map only if macro-generated `EntityComponents` becomes insufficient.

Future increments:

- `wasm-bindings` using `EntityComponents` + `spawn_entity`.
- `template_names()` iterator.
- README example for v3 JSON.

## License Note

No new dependencies. `macro_rules!` only.
