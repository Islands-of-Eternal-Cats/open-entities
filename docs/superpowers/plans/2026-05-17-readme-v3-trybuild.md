# README v3 Sync and `register_component!` trybuild Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Sync root `README.md` with world_json v3, import/spawn, and component registry; add a `trybuild` compile-fail test for standalone `register_component!`.

**Architecture:** Documentation-only plus dev-test harness under `open-entities-lib/tests/`. No library behavior changes. trybuild compiles a minimal misuse `.rs` file and expects failure with the existing `compile_error!` message from `register_component!`.

**Tech Stack:** Rust 2024, `trybuild` 1.x (dev-dependency only), existing `open_entities` macros.

**Spec:** `docs/superpowers/specs/2026-05-17-readme-v3-trybuild-design.md`

---

## File map

| File | Responsibility |
|------|----------------|
| `open-entities-lib/Cargo.toml` | Add `[dev-dependencies] trybuild = "1"` |
| `open-entities-lib/tests/ui/register_component_standalone.rs` | Misuse snippet (must not compile) |
| `open-entities-lib/tests/ui_tests.rs` | trybuild harness `#[test]` |
| `open-entities-lib/tests/ui/register_component_standalone.stderr` | Optional pinned stderr if trybuild requires it |
| `README.md` | v3 export, components, import/spawn, registry, examples |

**Out of scope:** `wasm-bindings`, rustdoc on macros, CI/Makefile changes, library code changes.

---

### Task 1: trybuild compile-fail for `register_component!`

**Files:**
- Modify: `open-entities-lib/Cargo.toml`
- Create: `open-entities-lib/tests/ui/register_component_standalone.rs`
- Create: `open-entities-lib/tests/ui_tests.rs`
- Create (if needed): `open-entities-lib/tests/ui/register_component_standalone.stderr`

- [ ] **Step 1: Add dev-dependency**

In `open-entities-lib/Cargo.toml`, append:

```toml
[dev-dependencies]
trybuild = "1"
```

- [ ] **Step 2: Create misuse source file**

Create `open-entities-lib/tests/ui/register_component_standalone.rs`:

```rust
use open_entities::register_component;

register_component!(foo, open_entities::components::Position);
```

- [ ] **Step 3: Create harness test**

Create `open-entities-lib/tests/ui_tests.rs`:

```rust
#[test]
fn register_component_outside_define_registered_components() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/register_component_standalone.rs");
}
```

- [ ] **Step 4: Run UI test**

Run: `cargo test -p open_entities register_component_outside_define_registered_components -- --nocapture`

Expected: PASS (trybuild confirms the file fails to compile).

If FAIL with message about stderr mismatch or `TRYBUILD=overwrite`:

1. Run: `TRYBUILD=overwrite cargo test -p open_entities register_component_outside_define_registered_components -- --nocapture`
2. Commit the generated `tests/ui/register_component_standalone.stderr` if created.
3. Re-run Step 4 without `TRYBUILD=overwrite`; expect PASS.

Pinned stderr should contain (substring is enough for manual review):

`register_component! must only appear inside define_registered_components!`

- [ ] **Step 5: Commit**

```bash
git add open-entities-lib/Cargo.toml \
  open-entities-lib/tests/ui_tests.rs \
  open-entities-lib/tests/ui/register_component_standalone.rs
# add .stderr too if generated:
# git add open-entities-lib/tests/ui/register_component_standalone.stderr
git commit -m "test: trybuild compile-fail for standalone register_component!"
```

---

### Task 2: Update README to v3 and current API

**Files:**
- Modify: `README.md` (replace entire file)

- [ ] **Step 1: Replace `README.md` with the content below**

```markdown
# OpenEntities

Rust workspace with the core library crate `open_entities` in `open-entities-lib/`.

The library uses [Bevy ECS](https://crates.io/crates/bevy_ecs) (`bevy_ecs` only, not the full Bevy engine) for entity simulation. Public entry points:

- [`Core`](open-entities-lib/src/core.rs) — owns the ECS [`World`](https://docs.rs/bevy_ecs/latest/bevy_ecs/world/struct.World.html)
- [`Api`](open-entities-lib/src/api.rs) — facade over `Core` (spawn, import, export)
- [`export`](open-entities-lib/src/export/mod.rs) — `Api::world_json()` serializes **every entity** in the world to JSON **schema version 3**; registered gameplay fields are omitted when absent (not `null`)
- [`EntityComponents`](open-entities-lib/src/entity_components.rs) — shared struct for YAML templates, `spawn_entity` overrides, and flattened export rows

Domain components live under `open_entities::components`: `Position`, `Velocity`, `Faction`, `MoveTarget`, and `Health`.

## Import and spawn

Load named entity templates from YAML, then spawn by template name with optional overrides:

1. **`Api::load_templates_yaml(yaml)`** — root must be `entities: { <name>: <components>, ... }`. Replaces any previously loaded templates on success. Template inheritance (`template`, `template: [a, b]`) is resolved at load time.
2. **`Api::spawn_entity(template_name, overrides)`** — requires a prior successful load. [`EntityComponents::default()`](open-entities-lib/src/entity_components.rs) spawns the template as resolved.
3. **Overrides** — each `Some` field in `overrides` replaces the template value; `None` leaves the template unchanged.

See [`open-entities-lib/examples/spawn_entity.rs`](open-entities-lib/examples/spawn_entity.rs) for inheritance and override examples.

## Component registry

Gameplay components are registered in one list:

[`open-entities-lib/src/component_registry/registered.rs`](open-entities-lib/src/component_registry/registered.rs)

```rust
define_registered_components! {
    register_component!(position, Position);
    // ...
}
```

To add a component: implement the type under `components/`, add one `register_component!(field, Type);` line, and run tests — merge, spawn, and export wiring are generated.

`register_component!` must only appear inside `define_registered_components!`; standalone use is a compile error (see `open-entities-lib/tests/ui/`).

## Requirements

- Rust **1.85+** (edition 2024; `bevy_ecs 0.18` may require a newer toolchain — check `cargo build` if compile fails)

Check your toolchain:

```bash
rustc --version
cargo --version
```

## Build

From the repository root:

```bash
cargo build --workspace
```

Build only the library crate:

```bash
cargo build -p open_entities
```

## Test

```bash
make test
```

Or directly:

```bash
cargo test
```

Includes a trybuild compile-fail test for macro misuse (`register_component!` outside the registry wrapper).

## Examples

### Spawn from YAML (default)

Loads templates (with inheritance), spawns entities, prints pretty world JSON:

```bash
make example
```

Or:

```bash
cargo run -p open_entities --example spawn_entity
```

### Hello world

Prints a greeting to stdout:

```bash
cargo run -p open_entities --example hello
```

Expected output:

```
Hello, world!
```

### World JSON export

Minimal spawn + compact JSON export:

```bash
make example EXAMPLE=world_json
```

Or:

```bash
cargo run -p open_entities --example world_json
```

Compact JSON is available from the library API:

```rust
use open_entities::{Api, components::Position};

let mut api = Api::new();
api.core_mut().world_mut().spawn(Position { x: 1.0, y: 2.0 });
let json = api.world_json().expect("export world");
```

### Exported JSON (schema version 3)

Every entity in the world appears in `entities`. Component keys are omitted when the entity does not have that component (not `null`).

```json
{
  "version": 3,
  "entities": [
    {
      "id": { "index": 0, "generation": 0 },
      "position": { "x": 1.0, "y": 2.0 },
      "velocity": { "vx": 0.5, "vy": -0.5 }
    },
    {
      "id": { "index": 1, "generation": 0 },
      "faction": 2
    },
    {
      "id": { "index": 2, "generation": 0 },
      "health": { "current": 80, "max": 100 }
    },
    {
      "id": { "index": 3, "generation": 0 },
      "entity_type": "scout"
    }
  ]
}
```
```

- [ ] **Step 2: Manual spot-check**

Confirm against code:

| README claim | Source |
|--------------|--------|
| `version: 3` | `open-entities-lib/src/export/mod.rs` `SCHEMA_VERSION` |
| All entities exported | `collect_world_export_rows` + `world_json_includes_entity_with_no_components` test |
| Component list | `open-entities-lib/src/components/mod.rs` |
| `make example` default | `Makefile` `EXAMPLE ?= spawn_entity` |
| `entity_type` key | spawn injects `EntityType` from template name |

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: sync README with world_json v3, import, and registry"
```

---

### Task 3: Final verification

**Files:** (none — verification only)

- [ ] **Step 1: Full test suite**

Run: `cargo test -p open_entities`

Expected: all tests PASS (including `register_component_outside_define_registered_components`).

- [ ] **Step 2: Clippy**

Run: `cargo clippy -p open_entities -- -D warnings`

Expected: no warnings.

- [ ] **Step 3: Commit (only if verification fixes were needed)**

```bash
git add -A
git commit -m "chore: fix issues after README and trybuild"
```

---

## Spec coverage (self-review)

| Spec requirement | Task |
|------------------|------|
| README v3, all entities, full component list | Task 2 |
| `EntityComponents` in intro | Task 2 |
| Import/spawn section | Task 2 |
| Component registry section | Task 2 |
| Examples: spawn_entity default, hello, world_json | Task 2 |
| v3 JSON sample, omitted keys not null | Task 2 |
| Remove v2 / RTS inclusion filter | Task 2 |
| `trybuild` dev-dependency | Task 1 |
| `tests/ui/register_component_standalone.rs` | Task 1 |
| `tests/ui_tests.rs` harness | Task 1 |
| Optional `.stderr` pin | Task 1 Step 4 |
| `cargo test` + clippy | Task 3 |
| No library behavior changes | No tasks touch `src/` except none |

## Placeholder scan

No TBD/TODO/similar-to-task placeholders. README and test file contents are complete in-task.

## Type consistency

- Macro path: `open_entities::register_component` (re-exported via `#[macro_export]` from crate root).
- Misuse test type: `open_entities::components::Position` (public).
- JSON field names match serde on `EntityComponents` and `Health`.
