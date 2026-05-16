# Design: bevy_ecs Integration and Position Component

**Date:** 2026-05-16  
**Status:** Approved (brainstorming)  
**Scope:** Add `bevy_ecs` to `open_entities`, define `Position { x, y }` as `f32`, re-export core ECS types, and verify integration with a unit test.

## Summary

Integrate Bevy's standalone ECS crate (`bevy_ecs`) into the `open_entities` library. Expose a hybrid public API: re-export essential ECS types (`World`, `Entity`, `Component`, `Query`) and a domain `Position` component in `components`. Keep the existing `hello()` API unchanged. Confirm ECS wiring with a unit test that spawns an entity and reads it back via `Query`.

**Full `bevy` (engine, render, windowing, plugins) is explicitly out of scope.**

## Decisions

| Topic | Choice |
|-------|--------|
| ECS dependency | `bevy_ecs` only (not `bevy`) |
| Version | `0.18` (workspace-pinned via `[workspace.dependencies]`) |
| MSRV | Rust **1.89+** (required by `bevy_ecs 0.18`; README update deferred) |
| Position fields | `x: f32`, `y: f32` (2D, Bevy-aligned) |
| Public API style | Hybrid: re-export core ECS types + `components::Position` |
| Module layout | `components/position.rs` (approach 2 — modular, not flat `lib.rs`) |
| Demo | Unit test: `World` → spawn → `Query<&Position>` (no `examples/`) |
| `hello()` | Unchanged (backward compatible scaffold) |
| Prelude module | Not in this increment (YAGNI) |

## Repository Layout

```
open-entities/
├── Cargo.toml                          # + [workspace.dependencies] bevy_ecs
└── open-entities-lib/
    ├── Cargo.toml                      # bevy_ecs = { workspace = true }
    └── src/
        ├── lib.rs                      # re-exports + pub mod components
        └── components/
            ├── mod.rs
            └── position.rs             # Position + unit test
```

## Root `Cargo.toml` Changes

Add workspace dependency:

```toml
[workspace.dependencies]
bevy_ecs = "0.18"
```

## `open-entities-lib/Cargo.toml` Changes

```toml
[dependencies]
bevy_ecs = { workspace = true }
```

## Public API

### Re-exports (`lib.rs`)

```rust
pub use bevy_ecs::{Component, Entity, Query, World};
pub mod components;
```

### `Position` (`components/position.rs`)

```rust
use bevy_ecs::prelude::Component;

/// 2D position in world/simulation space.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}
```

Public path: `open_entities::components::Position`.

Typical consumer import:

```rust
use open_entities::{Query, World, components::Position};
```

No constructors (`new`, `from_xy`) or `Default` impl in this increment.

### Unchanged API

`pub fn hello() -> &'static str` remains as-is.

## Data Flow (Unit Test)

1. Create `World::new()`.
2. `world.spawn(Position { x: 1.0, y: 2.0 })`.
3. Iterate `Query<&Position>` — expect exactly one entity with matching coordinates.

## Error Handling

No fallible public API in this increment. ECS invariant violations surface as panics from `bevy_ecs` (standard for direct `World` usage in tests).

## Testing

| Test | Location | Assertion |
|------|----------|-----------|
| `hello_returns_greeting` | `lib.rs` | Existing scaffold test — keep |
| `position_component_round_trip` | `components/position.rs` (preferred) or `lib.rs` | Single entity, `x == 1.0`, `y == 2.0` |

Verification (from repo root):

```bash
cargo test
cargo build --workspace
```

## Out of Scope

- Full **`bevy`** crate (render, audio, window, App/plugins ecosystem)
- `examples/` binary for ECS
- README / CI / `rust-toolchain.toml` updates
- `wasm-bindings` crate and `wasm32` CI targets
- Systems, schedules, resources, event buses
- Additional components (`Velocity`, `Health`, …)
- `prelude` module
- 3D coordinate `z`, generic `Position<T>`, helper methods on `Position`

## Future Extensions

- More components under `components/`
- `prelude` when import ergonomics matter
- `wasm-bindings` crate depending on `open_entities` + `bevy_ecs` WASM path
- Optional `examples/ecs_demo.rs` and README section
- Bump or align MSRV documentation when `rust-toolchain.toml` is added

## License Note

`bevy_ecs` is licensed MIT OR Apache-2.0. `open_entities` remains GPL-3.0-or-later; using `bevy_ecs` as a dependency is compatible.
