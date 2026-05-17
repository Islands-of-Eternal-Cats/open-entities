# Design: README v3 Sync and `register_component!` trybuild

**Date:** 2026-05-17  
**Status:** Approved (brainstorming)  
**Depends on:** [Component Registry](2026-05-17-component-registry-design.md)  
**Scope:** Bring root `README.md` in line with current library behavior (world_json v3, import/spawn, component registry) and add a `trybuild` compile-fail test that standalone `register_component!` is rejected. No code behavior changes, no WASM, no new public API.

## Summary

The component-registry increment shipped `world_json` schema version 3, `Health`, `spawn_entity`, YAML template inheritance, and export of all entities. Root `README.md` still documents v2, an obsolete RTS-component inclusion filter, and omits import/registry topics. The component-registry spec listed a macro-misuse compile-test as optional follow-up; this increment delivers it with `trybuild`.

## Goals

- README accurately describes current public API and JSON schema v3.
- README gives a **medium-depth** overview (option B): facts + brief Import/spawn and component-registry sections; default example is `spawn_entity`.
- `cargo test -p open_entities` includes a compile-fail UI test: `register_component!` outside `define_registered_components!` fails with the existing `compile_error!` message.

## Non-Goals

- Full project overview README (option C): architecture deep-dive, links to every spec, CI/Makefile changes.
- Rustdoc on `register_component!` / `define_registered_components!` (option 3 extension).
- Additional trybuild cases (valid usage, other macros).
- Separate workspace member crate for macro tests.
- Library behavior or schema changes.

## Decisions (Brainstorming)

| Topic | Choice | Rationale |
|-------|--------|-----------|
| README depth | **B — medium** | Fix stale facts + Import/spawn + registry + examples; avoid full rewrite |
| trybuild placement | **`tests/ui/` in `open-entities-lib`** | Standard pattern; `dev-dependency` only on library crate |
| JSON in README | Hand-written v3 sample aligned with tests/export | Stable doc; not tied to example stdout churn |
| Default example | Document `make example` → `spawn_entity` | Matches current `Makefile` `EXAMPLE` default |

## README Changes

### Introduction / API surface

Update bullets for:

- **`export`**: `Api::world_json()` serializes **every entity** in the world to JSON **schema version 3**. Registered gameplay fields are omitted per entity when that component is absent (not `null`).
- **Components** (under `open_entities::components`): `Position`, `Velocity`, `Faction`, `MoveTarget`, `Health`.
- **`EntityComponents`**: shared struct for YAML templates, `spawn_entity` overrides, and flattened export rows.

Remove:

- Claim that export only includes entities with at least one of `Position`, `Velocity`, `Faction`, `MoveTarget`.
- Schema version `2` and sample JSON showing `version: 2`.

### New section: Import and spawn

Document (~10–15 lines):

1. `Api::load_templates_yaml(yaml)` — root shape `entities: { name: components, ... }`; replaces prior templates on success.
2. `Api::spawn_entity(template_name, overrides)` — requires prior successful load; `EntityComponents::default()` spawns template as resolved (including `template` inheritance at load time).
3. Override semantics: each `Some` field in `overrides` replaces the template value; `None` leaves template unchanged.
4. Point to `open-entities-lib/examples/spawn_entity.rs` for inheritance (`template`, `template: [a, b]`).

### New section: Component registry

Document (~5–8 lines):

1. Registered components are listed in `open-entities-lib/src/component_registry/registered.rs` via `define_registered_components! { register_component!(field, Type); ... }`.
2. Adding a component: implement type under `components/`, add one `register_component!` line, run tests (merge/spawn/export wiring is generated).
3. `register_component!` **must not** be used outside `define_registered_components!` — standalone use is a compile error (covered by trybuild).

### Examples section

| Example | Command | Purpose |
|---------|---------|---------|
| **Spawn from YAML** (default) | `make example` or `cargo run -p open_entities --example spawn_entity` | Templates, inheritance, overrides, prints pretty world JSON |
| Hello | `cargo run -p open_entities --example hello` | Scaffold greeting |
| World JSON | `make example EXAMPLE=world_json` | Minimal export demo |

Keep existing build/test/requirements sections; only adjust if factually wrong.

### JSON sample (v3)

Replace embedded sample with `version: 3` and fields consistent with current serde shape:

- Top-level: `version`, `entities` array.
- Per entity: `id` `{ index, generation }`, flattened registered components, optional `entity_type` (spawn template name when present).
- Show optional keys: e.g. one entity with `position` + `velocity`, one with `faction` only, one with `health` if space permits.
- State explicitly: missing components are **omitted** from JSON, not serialized as `null`.

Source of truth for field names: `EntityComponents` + `export` tests in `open-entities-lib/src/export/mod.rs`.

## trybuild

### Dependency

Add to `open-entities-lib/Cargo.toml`:

```toml
[dev-dependencies]
trybuild = "1"
```

No workspace-level `[workspace.dependencies]` entry required (single consumer).

### Files

```text
open-entities-lib/
├── tests/
│   ├── ui_tests.rs
│   └── ui/
│       └── register_component_standalone.rs
```

### `tests/ui/register_component_standalone.rs`

Minimal misuse case:

```rust
use open_entities::register_component;

register_component!(foo, open_entities::components::Position);
```

Expected: compilation fails with message containing:

`register_component! must only appear inside define_registered_components!`

### `tests/ui_tests.rs`

```rust
#[test]
fn register_component_outside_define_registered_components() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/register_component_standalone.rs");
}
```

If trybuild requires pinned stderr on the project's toolchain, add `tests/ui/register_component_standalone.stderr` with the normalized error snippet; otherwise rely on default `compile_fail` matching.

### CI / local verification

```bash
cargo test -p open_entities
cargo clippy -p open_entities -- -D warnings
```

`make test` at repo root remains sufficient (runs workspace tests).

## Testing

| Check | Asserts |
|-------|---------|
| `register_component_outside_define_registered_components` | Standalone `register_component!` does not compile |
| Existing `cargo test -p open_entities` | No regressions |
| Manual README review | Version, component list, export rule, example commands match code |

## Errors

No new runtime or library error types. README and trybuild are documentation/test-only.

## Alternatives Considered

| Approach | Why not |
|----------|---------|
| README only, no trybuild | User chose trybuild A; spec already promised optional compile-test |
| Separate `open-entities-macro-tests` crate | Extra workspace member for one case |
| README JSON copied from example stdout | Fragile when example output changes |
| Full README rewrite (C) | User chose medium depth B |
| rustdoc on macros | Scope creep for chore increment |

## Evolution

- Link README to `docs/superpowers/specs/` when more features land.
- Additional trybuild cases if new foot-gun macros are added.
- `template_names()` docs when that API exists.

## License Note

`trybuild` is MIT/Apache-2.0; compatible with project GPL-3.0-or-later for dev-dependencies only.
