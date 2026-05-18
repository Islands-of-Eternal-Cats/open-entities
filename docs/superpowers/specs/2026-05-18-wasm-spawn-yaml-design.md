# Design: WASM Spawn + YAML (Node)

**Date:** 2026-05-18  
**Status:** Approved (brainstorming)  
**Depends on:** [WASM Bindings PoC](2026-05-17-wasm-bindings-poc-design.md), [Component Registry](2026-05-17-component-registry-design.md), [Spawn Entity / EntityComponents](2026-05-17-spawn-entity-shared-components-design.md)  
**Scope:** Extend `wasm-bindings` with `load_templates_yaml`, `spawn_entity` (JS overrides via `serde_wasm_bindgen`), shared workspace fixtures, full `spawn_entity` demo, and `#[wasm_bindgen_test]` + `make wasm-test`. No browser target, no GitHub Actions, no lib import/export logic changes.

## Summary

The PoC proved `open_entities::Api::world_json()` works from Node. This increment closes the loop: **load YAML templates → spawn entities with optional overrides → export `world_json`**, matching [`open-entities-lib/examples/spawn_entity.rs`](../../../open-entities-lib/examples/spawn_entity.rs).

Shared template data lives in **workspace-root `fixtures/`** so Rust examples, native tests, Node demo, and WASM unit tests use one file.

Verification uses **both** an extended Node demo (`make wasm-demo`) and **`wasm-pack test --node`** (`make wasm-test`).

## Goals

- Expose `Api::load_templates_yaml` and `Api::spawn_entity` on `Simulation` without duplicating import/export logic.
- Accept spawn overrides as a plain JS object (`EntityComponents` shape) via `serde_wasm_bindgen`.
- Return spawned entity ids as `{ index, generation }`, aligned with `world_json` entity `id` fields.
- Extract inline YAML from `spawn_entity.rs` into `fixtures/spawn_entity_templates.yaml`.
- Guard `wasm32` builds: Node full-cycle demo + `#[wasm_bindgen_test]`.

## Non-Goals

- GitHub Actions / CI workflow (local `make wasm-demo` / `make wasm-test` only).
- `wasm-pack build --target web`, browser demo, TypeScript defs, npm publish.
- `tick`, systems, simulation loop in WASM.
- Changes to YAML parsing, template inheritance, merge rules, or `world_json` schema in `open-entities-lib`.
- Re-exporting `EntityComponents` as a generated TS type (future).

## Decisions (Brainstorming)

| Topic | Choice | Rationale |
|-------|--------|-----------|
| Verification | **Node demo + `#[wasm_bindgen_test]`** | User: option C — runtime import path + Rust-side wasm tests |
| Demo fidelity | **Port `spawn_entity.rs`** | Same templates, spawn order, scout overrides, assert on exported JSON |
| Fixtures location | **Repo root `fixtures/`** | Single source for lib example, lib tests (optional), wasm tests, Node demo |
| Overrides on JS boundary | **`serde_wasm_bindgen::from_value`** | PoC spec follow-up; `EntityComponents` already `Deserialize` |
| Spawn return type | **`SpawnedEntity { index, generation }`** | Matches `world_json` `id` shape; usable from JS without parsing export |
| Bindings style | **Thin forwarders** | Same as PoC: lib owns logic, map errors to `JsValue` |
| JS method names | **camelCase via `js_name`** | `loadTemplatesYaml`, `spawnEntity`, `getWorldAsJson` (Rust stays snake_case) |
| `hello()` | **Keep on `Simulation`** | Smoke test; demo may still call it before spawn cycle (`hello` unchanged in JS) |

## Section 1: Scope and Repository Layout

### Workspace fixtures (canonical YAML)

```text
open-entities/
├── fixtures/
│   └── spawn_entity_templates.yaml    # moved from spawn_entity.rs const
├── open-entities-lib/
│   └── examples/
│       └── spawn_entity.rs            # loads fixture via include_str!
└── wasm-bindings/
    ├── src/
    │   └── lib.rs                     # Simulation API + wasm_bindgen tests
    └── demo/
        └── run.mjs                    # reads fixture via fs (ESM path)
```

**Fixture file:** `fixtures/spawn_entity_templates.yaml` — byte-for-byte equivalent of the current `TEMPLATES_YAML` in `spawn_entity.rs` (entities `unit`, `scout`, `tank`, `heavy_tank`, `marker`; template inheritance as today).

**Loading conventions (no cwd-relative Rust paths):**

| Consumer | Mechanism |
|----------|-----------|
| `open-entities-lib/examples/spawn_entity.rs` | `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/spawn_entity_templates.yaml"))` |
| `#[wasm_bindgen_test]` in `wasm-bindings` | `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/spawn_entity_templates.yaml"))` |
| `wasm-bindings/demo/run.mjs` | `readFileSync` resolved from `import.meta.url` → `../../fixtures/spawn_entity_templates.yaml` |

**Note:** Root `fixtures/` are workspace assets, not packaged inside the `open_entities` crate on `cargo publish` unless explicitly added later.

### Out of scope for fixtures in v1

- Additional fixture files beyond `spawn_entity_templates.yaml`.
- Rust integration tests that today use inline YAML strings (may adopt fixture in a follow-up; not required for this increment).

---

## Section 2: WASM API, Types, and Data Flow

### Architecture

```text
Node demo / JS caller
    │  loadTemplatesYaml(yamlStr)
    │  spawnEntity(name, overridesObj) → SpawnedEntity
    │  getWorldAsJson() → JSON string
    ▼
open_entities_wasm::Simulation (owns Api)
    │  forward + error map  (Rust: load_templates_yaml, spawn_entity, world_json)
    ▼
open_entities::Api
    load_templates_yaml → resolve templates (unchanged)
    spawn_entity(name, EntityComponents) → Entity
    world_json → schema v3 JSON
```

### JS export names (`#[wasm_bindgen(js_name = …)]`)

Rust methods keep **snake_case**; generated glue exposes **camelCase** to JS:

| Rust (`Simulation`) | JavaScript |
|---------------------|------------|
| `load_templates_yaml` | `loadTemplatesYaml` |
| `spawn_entity` | `spawnEntity` |
| `world_json` | `getWorldAsJson` |
| `hello` | `hello` (unchanged) |
| `new` / constructor | `new Simulation()` (unchanged) |

Override/component keys in JS objects remain **snake_case** (`move_target`, etc.) — same as YAML and `world_json` export; only **methods** use camelCase.

### New dependencies (`wasm-bindings/Cargo.toml`)

```toml
serde-wasm-bindgen = "0.6"
```

(`EntityComponents` deserialization uses existing `serde` derives in `open_entities`; no new lib deps.)

### `SpawnedEntity`

`#[wasm_bindgen]` struct returned from `spawn_entity`:

```rust
#[wasm_bindgen]
pub struct SpawnedEntity {
    index: u32,
    generation: u32,
}

#[wasm_bindgen]
impl SpawnedEntity {
    #[wasm_bindgen(getter)]
    pub fn index(&self) -> u32 { self.index }

    #[wasm_bindgen(getter)]
    pub fn generation(&self) -> u32 { self.generation }
}
```

**Mapping from `bevy_ecs::Entity`:**

- `index` ← `entity.index_u32()`
- `generation` ← `entity.generation().to_bits()`

Same encoding as [`EntityIdExport`](../../../open-entities-lib/src/export/mod.rs) in `world_json` (`id.index`, `id.generation`).

### `Simulation` methods (additions + PoC rename in JS)

```rust
/// JS: `loadTemplatesYaml(yaml)`
#[wasm_bindgen(js_name = loadTemplatesYaml)]
pub fn load_templates_yaml(&mut self, yaml: &str) -> Result<(), JsValue>;

/// JS: `spawnEntity(templateName, overrides)`
#[wasm_bindgen(js_name = spawnEntity)]
pub fn spawn_entity(
    &mut self,
    template_name: &str,
    overrides: JsValue,
) -> Result<SpawnedEntity, JsValue>;

/// JS: `getWorldAsJson()` — update existing PoC binding with `js_name`
#[wasm_bindgen(js_name = getWorldAsJson)]
pub fn world_json(&mut self) -> Result<String, JsValue>;
```

**`load_templates_yaml` implementation sketch:**

```rust
self.api
    .load_templates_yaml(yaml)
    .map_err(|e: ImportError| JsValue::from_str(&e.to_string()))
```

**`spawn_entity` implementation sketch:**

```rust
let overrides: EntityComponents = serde_wasm_bindgen::from_value(overrides)
    .map_err(|e| JsValue::from_str(&format!("invalid overrides: {e}")))?;
let entity = self.api
    .spawn_entity(template_name, overrides)
    .map_err(|e: ImportError| JsValue::from_str(&e.to_string()))?;
Ok(SpawnedEntity {
    index: entity.index_u32(),
    generation: entity.generation().to_bits(),
})
```

Existing methods: `new()`, `hello()` (no `js_name` change). **`world_json`** gains `js_name = getWorldAsJson` (breaking rename in JS vs PoC demo that called `world_json()`).

### JS overrides shape

Mirrors Rust / YAML component keys (serde field names on `EntityComponents`):

```javascript
// No overrides
sim.spawnEntity("unit", {});

// Scout overrides (same as spawn_entity.rs)
sim.spawnEntity("scout", {
  position: { x: 50.0, y: 25.0 },
  health: { current: 40, max: 100 },
});
```

Partial objects are valid: only `Some` fields in Rust terms are applied; `None`/missing keys keep template values after merge.

Invalid types or unknown top-level keys → `from_value` or YAML errors surface as `JsValue` string messages (no custom error enum in v1).

### Error handling

| Source | WASM behavior |
|--------|----------------|
| `ImportError` (YAML, unknown template, cycle, not loaded, etc.) | `Err(JsValue::from_str(&display))` |
| `ExportError` (`world_json`) | unchanged from PoC |
| `serde_wasm_bindgen::Error` (bad overrides) | `Err(JsValue::from_str("invalid overrides: …"))` |
| Panics in lib | abort WASM instance (no `console_error_panic_hook` required) |

### Data flow (full cycle)

1. **Load:** `loadTemplatesYaml(yaml)` → `Api` stores flattened templates.
2. **Spawn (×N):** For each name, `from_value` → `EntityComponents` → `merge_components` inside lib → entity in `World`; return `SpawnedEntity`.
3. **Export:** `getWorldAsJson()` → JSON with `version: 3`, `entities[]` each with `id: { index, generation }` and flattened components.

Demo spawn order (same as example): `marker`, `heavy_tank`, `tank`, `scout` (with overrides), `unit`.

---

## Section 3: Node Demo

`wasm-bindings/demo/run.mjs` will:

1. Import `Simulation` from generated pkg (unchanged entry).
2. Read `fixtures/spawn_entity_templates.yaml` from disk.
3. `loadTemplatesYaml(yaml)` — throw on error.
4. Loop spawn names with `spawnEntity` and scout overrides object.
5. Log each `SpawnedEntity.index` / `.generation` (or destructured getters).
6. Parse `getWorldAsJson()` and assert:
   - `version === 3`
   - `entities.length === 5`
   - scout row has overridden `position` / `health` values
   - optional: spawned ids appear in export `entities[].id`

`make wasm-demo`: `wasm-pack build` then `cd wasm-bindings && node demo/run.mjs` (unchanged cwd contract).

---

## Section 4: WASM Tests and Makefile

### `wasm-bindings` test deps

```toml
[dev-dependencies]
wasm-bindgen-test = "0.3"
```

### `#[wasm_bindgen_test]` (in `wasm-bindings/src/lib.rs`)

Use `wasm_bindgen_test::wasm_bindgen_test` + fixture `include_str!`. Tests call Rust methods directly (`load_templates_yaml`, `world_json`); JS names are not exercised here.

Suggested cases:

| Test | Assert |
|------|--------|
| `load_and_spawn_from_fixture` | load fixture YAML; spawn `marker`; `world_json` parses; ≥1 entity |
| `spawn_scout_with_overrides` | overrides position/health; export contains expected values |
| `spawn_without_load_fails` | `TemplatesNotLoaded` message in `Err` |
| `unknown_template_fails` | `UnknownTemplate` after load |

Run: `wasm-pack test wasm-bindings --node`

### Makefile targets

```makefile
wasm-test:
	@command -v wasm-pack >/dev/null 2>&1 || { echo "…"; exit 1; }
	wasm-pack test wasm-bindings --node

wasm-check: wasm-demo wasm-test   # optional convenience aggregate
```

---

## Testing and Verification

| Check | Command |
|-------|---------|
| Native lib unchanged | `cargo test` |
| WASM builds | `wasm-pack build wasm-bindings --target nodejs` |
| Node full cycle | `make wasm-demo` |
| WASM unit tests | `make wasm-test` |
| Both | `make wasm-check` (if added) |

---

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| `serde_wasm_bindgen` version vs `wasm-bindgen 0.2` | Pin `0.6` (common pairing); verify in `wasm-pack build` + test |
| Fixture path breaks if demo moved | Resolve via `import.meta.url` only |
| `Entity` generation encoding | Use same `to_bits()` as export (documented in Section 2) |
| Demo assertions too brittle | Assert on scout overrides + entity count, not full pretty JSON |

## Future Extensions (Out of Scope)

- GitHub Actions running `make wasm-check`.
- Browser target and `init()` in demo.
- TypeScript types generated from `EntityComponents` JSON schema.
- `tick(dt)` when systems land.

## Alternatives Considered

| Approach | Why not chosen |
|----------|----------------|
| `open-entities-lib/fixtures/` only | Works, but wasm demo path couples to lib crate layout; root `fixtures/` is neutral shared workspace data |
| Overrides as JSON string | Worse JS ergonomics; duplicates parsing |
| `spawn_entity` returns void | User chose explicit `SpawnedEntity` for JS tracking |
| Demo-only verification (no wasm tests) | User chose C: demo + `wasm_bindgen_test` |
| snake_case method names in JS | User chose camelCase via `js_name` for idiomatic JS API |
