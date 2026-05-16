# Design: RTS Components and World JSON Export v2

**Date:** 2026-05-16  
**Status:** Approved (brainstorming)  
**Depends on:** [Core, Api, and World JSON Export](2026-05-16-core-api-world-export-design.md)  
**Scope:** Add RTS-oriented ECS components (`Velocity`, `Faction`, `MoveTarget`), extend `world_json` to schema version 2 with optional per-component fields. No simulation systems, tick API, or YAML loading.

## Summary

Expand the `open_entities` domain model for a top-down RTS simulation. Introduce three new components alongside existing `Position`, each with unit tests. Update `export::world_json` to schema **version 2**: one `Query` pass with `Option<&T>` per component, export only entities that have at least one of the four components, and omit JSON keys for components the entity does not have.

Systems (`seek`, `movement`), `Api::tick`, YAML map loading, and `wasm-bindings` remain out of scope for this increment.

## Goals

- RTS-ready component types consumers can attach via `spawn` / `insert`.
- JSON snapshots suitable for future browser tooling and debugging.
- Single-query export path (no merge maps, no full-world entity iteration).
- Preserve existing `Core` / `Api` / `hello()` surface; only extend components and export.

## Non-Goals (This Increment)

- `Api::tick(dt)` or any `systems/` module.
- YAML / asset-driven entity definitions.
- `Deserialize` / `from_json` import.
- Exporting entities with none of `Position`, `Velocity`, `Faction`, `MoveTarget`.
- Additional components (`Health`, `Unit`, `Vehicle`, …).
- `wasm-bindings`, CI, `rust-toolchain.toml` updates.
- Bumping example `world_json.rs` behavior beyond what tests/README document (example may keep manual spawns; tests are authoritative for v2 contract).

## Decisions

| Topic | Choice | Rationale |
|-------|--------|-----------|
| Domain | Top-down RTS | User direction; sets component shapes |
| First slice | Components + export only | Defer systems/tick to a follow-up spec |
| `Velocity` | `{ vx: f32, vy: f32 }` | Matches 2D `Position` model |
| `Faction` | `Faction(pub u32)` newtype | Numeric side id; `#[serde(transparent)]` → JSON number |
| `MoveTarget` | `{ x: f32, y: f32 }` | World-space goal point |
| Export inclusion | Entity has ≥1 of the four components | Empty/id-only entities excluded |
| Export query | One pass: `(Entity, Option<&Position>, …)` | User preference; efficient archetype walk |
| Missing components in JSON | Omit keys (`skip_serializing_if`) | Compact, explicit presence |
| Schema version | `2` | Breaking change vs v1 filter and entity shape |
| `id` in JSON | Unchanged `{ index, generation }` | Stable from v1 |
| `Position` in export struct | `Option<Position>` in `EntityExport` | Required for optional key omission (v1 had required `position`) |

## Repository Layout

```text
open-entities-lib/src/
├── components/
│   ├── mod.rs              # re-export new types
│   ├── position.rs         # unchanged shape; export no longer assumes always present
│   ├── velocity.rs         # new
│   ├── faction.rs          # new
│   └── move_target.rs      # new
└── export/
    └── mod.rs              # v2 query + EntityExport options + tests
```

## Components

### `Velocity` (`components/velocity.rs`)

```rust
#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Velocity {
    pub vx: f32,
    pub vy: f32,
}
```

Public path: `open_entities::components::Velocity`.

### `Faction` (`components/faction.rs`)

```rust
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct Faction(pub u32);
```

JSON: `"faction": 1` (not `{ "0": 1 }`).

Public path: `open_entities::components::Faction`.

### `MoveTarget` (`components/move_target.rs`)

```rust
#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize)]
pub struct MoveTarget {
    pub x: f32,
    pub y: f32,
}
```

Public path: `open_entities::components::MoveTarget`.

### `Position`

Existing type; keep `Serialize` and fields `{ x, y }`. No API change.

### Module exports (`components/mod.rs`)

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

Each new component file includes a `#[cfg(test)]` spawn + `Query` round-trip test (same pattern as `position.rs`).

## JSON Contract (Schema Version 2)

```json
{
  "version": 2,
  "entities": [
    {
      "id": { "index": 0, "generation": 0 },
      "position": { "x": 10.0, "y": 5.0 },
      "velocity": { "vx": 1.0, "vy": 0.0 },
      "faction": 1,
      "move_target": { "x": 20.0, "y": 0.0 }
    },
    {
      "id": { "index": 1, "generation": 0 },
      "faction": 2
    }
  ]
}
```

| Field | Type | Notes |
|-------|------|-------|
| `version` | `u32` | Always `2` for this schema |
| `entities` | array | One row per exported entity |
| `entities[].id` | object | Required; same as v1 |
| `entities[].position` | object | Present only if component exists |
| `entities[].velocity` | object | `vx`, `vy` |
| `entities[].faction` | number | Transparent `Faction(u32)` |
| `entities[].move_target` | object | `x`, `y` |

### Export algorithm

1. `let mut query = world.query::<(Entity, Option<&Position>, Option<&Velocity>, Option<&Faction>, Option<&MoveTarget>)>();`
2. For each row from `query.iter(world)`:
   - If all four `Option`s are `None`, **skip** (defensive; should not occur for valid query results).
   - Else build `EntityExport` with `id` and `Some` only for present components.
3. Wrap in `WorldExport { version: 2, entities }`.
4. `serde_json::to_string`.

### `EntityExport` (internal)

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

Copy component values (`*position`, `*faction`, etc.) when building the export row.

### Breaking changes vs v1

| v1 | v2 |
|----|-----|
| `version: 1` | `version: 2` |
| Only entities with `Position` | Entities with any of four components |
| `position` always present on exported rows | `position` optional |

## Public API

Unchanged entry points:

- `Api::new()`, `core_mut()`, `world_json() -> Result<String, ExportError>`
- `Core`, `ExportError`, `hello()`
- Re-exported ECS types

New public types via `open_entities::components::{Velocity, Faction, MoveTarget}`.

Update `Api::world_json` doc comment: describes v2 inclusion rule and optional fields.

## Error Handling

| API | Error | Cases |
|-----|-------|-------|
| `Api::world_json` | `ExportError::Serde` | JSON encode failure only |

ECS misuse continues to panic via `bevy_ecs`.

## Testing

| Test | Location | Assertion |
|------|----------|-----------|
| `velocity_component_round_trip` | `velocity.rs` | Single entity, `vx`/`vy` |
| `faction_component_round_trip` | `faction.rs` | `Faction(42)` |
| `move_target_component_round_trip` | `move_target.rs` | `x`/`y` |
| `position_component_round_trip` | `position.rs` | Unchanged |
| `world_json_empty_world` | `export/mod.rs` | `version == 2`, `entities == []` |
| `world_json_includes_positioned_entities` | `export/mod.rs` | Update to `version == 2` |
| `world_json_faction_only_entity` | `export/mod.rs` | `faction` present, no `position` key |
| `world_json_partial_components` | `export/mod.rs` | e.g. `Position` + `Velocity`, no `faction`/`move_target` keys |

Verification (repo root):

```bash
cargo test -p open_entities
cargo clippy -p open_entities -- -D warnings
```

## README

Update the exported JSON example and schema description:

- `version: 2`
- Inclusion rule: entities with at least one RTS export component
- Optional keys per component
- Brief mention of `Velocity`, `Faction`, `MoveTarget`

## Alternatives Considered

| Approach | Why not |
|----------|---------|
| Export all entities in `World` | User rejected id-only rows |
| Multiple queries + `HashMap` merge | User chose single `Option<&T>` query |
| `version: 1` with extended rows | Ambiguous for consumers; v1 always had `position` |
| `null` for missing components | User chose omitted keys |
| `Faction` as enum or string | User chose numeric `u32` id |
| Systems + `tick` in same increment | Deferred to keep scope reviewable |

## Out of Scope

- Simulation systems and `Api::tick(dt)`.
- YAML loading (`assets/entities.yaml`, map init).
- JSON import.
- Entities outside the four-component export set.
- `world_json_pretty()` in the library.

## Future Extensions

- **Next increment:** `systems/` (`seek`, `movement`), `Api::tick(dt)`, optional `BaseMoveSpeed` component.
- YAML-driven spawn from `assets/`.
- Extend export as new components stabilize; bump `version` when shape breaks.
- `wasm-bindings` consuming v2 JSON.

## License Note

No new dependencies. `serde` remains MIT OR Apache-2.0; compatible with GPL-3.0-or-later.
