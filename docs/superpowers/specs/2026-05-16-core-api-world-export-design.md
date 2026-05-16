# Design: Core, Api, and World JSON Export

**Date:** 2026-05-16  
**Status:** Approved (implemented)  
**Depends on:** [bevy_ecs Integration and Position Component](2026-05-16-bevy-ecs-position-design.md)  
**Scope:** Introduce `Core` (owns `World`), `Api` (public facade), and `export::world_json` for JSON snapshots. Native Rust only; `wasm-bindings` deferred.

## Summary

Add a layered entry point to `open_entities` so consumers do not hold a raw `World` as the only integration path. `Core` owns the ECS world; `Api` wraps `Core` for spawn, future systems, and export. The `export` module implements `Api::world_json()` — a compact JSON string describing every entity that has a `Position` component.

Serialization uses `serde` / `serde_json` in `open-entities-lib` (not in a separate DTO crate and not in `wasm-bindings` for this increment). `Position` derives `Serialize` and is embedded directly in the export payload (no duplicate `PositionExport` type). Entity IDs are exported as `{ index, generation }`, not as opaque `bevy_ecs::Entity` values.

## Goals

- Single owned simulation instance (`Core` + `World`).
- Stable, testable JSON snapshot for tooling, examples, and future JS/WASM consumers.
- Keep ECS mutation paths available via `Api::core_mut()` while export lives in a dedicated module.
- Avoid duplicating component field definitions where the JSON shape matches the domain type.

## Non-Goals (This Increment)

- `wasm-bindings` crate or `wasm32` CI.
- `world_json_pretty()` in the library (pretty printing only in `examples/world_json.rs`).
- Exporting entities without `Position` or exporting other components.
- Systems, schedules, resources, simulation tick API.
- Separate public DTO module (`WorldSnapshot` structs) — internal `WorldExport` / `EntityExport` types are private to `export`.
- `Api::world_json(&self)` — requires `&mut self` (see below).

## Decisions

| Topic | Choice | Rationale |
|-------|--------|-----------|
| World ownership | `Core { world: World }` | Clear lifetime boundary; future WASM holds one `Core` instance |
| Public facade | `Api { core: Core }` | Export and gameplay APIs share one handle |
| JSON location | `open-entities-lib` / `export` | One implementation tested with `cargo test`; WASM reuses later |
| Serde on components | `Position: Serialize` | Same shape as ECS component; no `PositionExport` duplicate |
| Entity id in JSON | `{ index: u32, generation: u32 }` | Stable, explicit; not coupled to `Entity` serde |
| Schema version | Top-level `"version": 1` | Forward-compatible contract for future consumers |
| Export filter | Entities with `Position` only | Matches current domain; empty `entities: []` for empty world |
| `world_json` receiver | `&mut self` | `World::query()` in Bevy 0.18 requires `&mut World` |
| JSON formatting | Compact string from lib | Example pretty-prints via `serde_json::to_string_pretty` after parse |
| ECS re-exports | Keep `World`, `Entity`, `Query`, `Component` | Hybrid API from prior spec; advanced users can still use ECS directly |
| `hello()` | Unchanged | Backward compatible scaffold |

## Architecture

```text
┌─────────────┐     owns      ┌──────────────┐
│    Api      │──────────────▶│    Core      │
│  (facade)   │               │  world: World│
└──────┬──────┘               └──────────────┘
       │
       │ world_json(&mut self)
       ▼
┌─────────────┐
│   export    │  Query (Entity, &Position) → serde_json
└─────────────┘
```

**Data flow (export):**

1. `Api::world_json` borrows `Core::world_mut()`.
2. Run `query::<(Entity, &Position)>`, collect `EntityExport` rows.
3. Wrap in `WorldExport { version: 1, entities }`.
4. `serde_json::to_string` → `Result<String, ExportError>`.

## Repository Layout

```text
open-entities/
├── Cargo.toml                              # + serde, serde_json workspace deps
├── Makefile                                # example, example-world-json
└── open-entities-lib/
    ├── Cargo.toml                          # serde, serde_json
    ├── examples/
    │   ├── hello.rs
    │   └── world_json.rs                   # pretty-printed stdout
    └── src/
        ├── lib.rs                          # mod api, core, export; re-exports
        ├── core.rs
        ├── api.rs
        ├── export/
        │   └── mod.rs                      # ExportError, impl Api::world_json
        └── components/
            └── position.rs                 # + Serialize on Position
```

## Dependencies

Root `Cargo.toml`:

```toml
[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

`open-entities-lib/Cargo.toml`:

```toml
serde = { workspace = true }
serde_json = { workspace = true }
```

## Public API

### `Core` (`core.rs`)

- `Core::new()` / `Default`
- `world(&self) -> &World`
- `world_mut(&mut self) -> &mut World`

### `Api` (`api.rs`)

- `Api::new()` / `Default`
- `core_mut(&mut self) -> &mut Core`
- `world_json(&mut self) -> Result<String, ExportError>` — via `impl Api` in `export/mod.rs`

### Re-exports (`lib.rs`)

```rust
pub use api::Api;
pub use core::Core;
pub use export::ExportError;
```

### `Position` (`components/position.rs`)

```rust
#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}
```

## JSON Contract (Schema Version 1)

Compact serialization (field order not guaranteed stable across serde versions):

```json
{
  "version": 1,
  "entities": [
    {
      "id": { "index": 0, "generation": 0 },
      "position": { "x": 1.0, "y": 2.0 }
    }
  ]
}
```

| Field | Type | Notes |
|-------|------|-------|
| `version` | `u32` | Always `1` for this schema |
| `entities` | array | Only entities with `Position` |
| `entities[].id.index` | `u32` | `Entity::index_u32()` |
| `entities[].id.generation` | `u32` | `Entity::generation().to_bits()` |
| `entities[].position` | object | Same fields as `Position` |

## Error Handling

| API | Error type | Cases |
|-----|------------|-------|
| `Api::world_json` | `ExportError::Serde` | `serde_json::to_string` failure (unexpected for current types) |

ECS misuse (invalid spawn/query) still panics via `bevy_ecs`, consistent with direct `World` usage.

## Testing

| Test | Location | Assertion |
|------|----------|-----------|
| `world_json_empty_world` | `export/mod.rs` | `version == 1`, `entities == []` |
| `world_json_includes_positioned_entities` | `export/mod.rs` | One entity, position `x/y`, numeric id fields |
| Existing tests | `lib.rs`, `position.rs` | Unchanged |

Verification:

```bash
cargo test -p open_entities
cargo clippy -p open_entities -- -D warnings
make example-world-json
```

## Examples & Makefile

| Target | Command |
|--------|---------|
| Default example | `make example` → `hello` |
| World JSON | `make example-world-json` or `make example EXAMPLE=world_json` |

## Alternatives Considered

| Approach | Why not now |
|----------|-------------|
| JSON only in `wasm-bindings` | Cannot query `World` without exposing it; duplicates export logic when WASM lands |
| Public `WorldSnapshot` DTO module | Extra types for a single consumer; revisit if multiple export formats appear |
| `world_json(&self)` with cached `QueryState` | `RefCell` + stored query adds complexity; `&mut self` is idiomatic for Bevy |
| Separate `PositionExport` | Duplicates `Position`; removed in favor of `Serialize` on component |
| `feature = "serde"` on lib | Possible later if native consumers must avoid serde dependency |

## Out of Scope

- `wasm-bindings` and TypeScript types generated from JSON schema.
- Import / deserialization (`from_json`, `Deserialize` on `Position`).
- Delta snapshots, binary formats, per-frame export optimization.
- Additional components in export (`Velocity`, faction, YAML metadata, …).
- Spawn/helpers on `Api` (consumers use `core_mut().world_mut().spawn(...)`).

## Future Extensions

- `wasm-bindings`: thin `#[wasm_bindgen]` wrapper calling `Api::world_json()` (or `snapshot` + serde in wasm if lib serde is feature-gated).
- `world_json_pretty()` or `ExportFormat::Pretty` in lib.
- Extend export query as new components stabilize; bump `version` when JSON shape breaks.
- Optional `feature = "export-serde"` if dependency-free native builds matter.
- `Api` methods for spawn/load map instead of raw `world_mut()`.

## License Note

`serde` / `serde_json` are licensed MIT OR Apache-2.0. Compatible with `open_entities` (GPL-3.0-or-later).
