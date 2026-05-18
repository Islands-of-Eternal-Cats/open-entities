# WASM Spawn + YAML (Node) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend `wasm-bindings` so Node/JS can run the full `spawn_entity` cycle (load YAML → spawn with overrides → `getWorldAsJson`), with shared root `fixtures/`, camelCase JS method names, and `make wasm-test` guarding `wasm32`.

**Architecture:** Thin `open_entities_wasm` forwards to existing `Api::load_templates_yaml`, `spawn_entity`, and `world_json`. Overrides cross the boundary via `serde_wasm_bindgen::from_value` into `EntityComponents`. YAML lives in `fixtures/spawn_entity_templates.yaml` for example, demo, and wasm tests.

**Tech Stack:** Rust workspace, `wasm-bindgen 0.2`, `serde-wasm-bindgen 0.6`, `wasm-bindgen-test 0.3`, `wasm-pack` (Node), Node ESM (`.mjs`).

**Spec:** `docs/superpowers/specs/2026-05-18-wasm-spawn-yaml-design.md`

**Prerequisites:**

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
```

---

## File map

| File | Responsibility |
|------|----------------|
| `fixtures/spawn_entity_templates.yaml` | Canonical YAML (from `spawn_entity.rs` const) |
| `open-entities-lib/examples/spawn_entity.rs` | Load fixture via `include_str!` |
| `wasm-bindings/Cargo.toml` | Add `serde-wasm-bindgen`, `wasm-bindgen-test` dev-dep |
| `wasm-bindings/src/lib.rs` | `SpawnedEntity`, new methods, `js_name`, `#[wasm_bindgen_test]` |
| `wasm-bindings/demo/run.mjs` | Full spawn cycle + assertions; camelCase API |
| `Makefile` | `wasm-test`, optional `wasm-check` |

**Unchanged:** `open-entities-lib` import/export logic, schema v3, merge rules.

**Out of scope:** GitHub Actions, browser target, TS defs, camelCase on component keys.

---

### Task 1: Workspace fixture

**Files:**
- Create: `fixtures/spawn_entity_templates.yaml`
- Modify: `open-entities-lib/examples/spawn_entity.rs`

- [ ] **Step 1: Create fixture file**

Copy the exact YAML body from `TEMPLATES_YAML` in `spawn_entity.rs` (lines 11–45, no leading `r"` wrapper) into `fixtures/spawn_entity_templates.yaml`.

- [ ] **Step 2: Update example to use fixture**

Replace `const TEMPLATES_YAML: &str = r"..."` with:

```rust
const TEMPLATES_YAML: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/spawn_entity_templates.yaml"));
```

- [ ] **Step 3: Verify example still runs**

```bash
cargo run -p open_entities --example spawn_entity
```

Expected: spawns five entities, prints pretty JSON (no load errors).

- [ ] **Step 4: Commit**

```bash
git add fixtures/spawn_entity_templates.yaml open-entities-lib/examples/spawn_entity.rs
git commit -m "chore: extract spawn entity YAML to workspace fixtures"
```

---

### Task 2: WASM crate dependencies

**Files:**
- Modify: `wasm-bindings/Cargo.toml`

- [ ] **Step 1: Add dependencies**

```toml
[dependencies]
open_entities = { path = "../open-entities-lib" }
wasm-bindgen = "0.2"
serde-wasm-bindgen = "0.6"

[dev-dependencies]
wasm-bindgen-test = "0.3"
```

- [ ] **Step 2: Verify wasm32 check compiles (may fail until Task 3)**

```bash
cargo check -p open_entities_wasm --target wasm32-unknown-unknown
```

- [ ] **Step 3: Commit**

```bash
git add wasm-bindings/Cargo.toml
git commit -m "chore(wasm): add serde-wasm-bindgen and wasm-bindgen-test"
```

---

### Task 3: `SpawnedEntity` and import/spawn bindings

**Files:**
- Modify: `wasm-bindings/src/lib.rs`

- [ ] **Step 1: Add imports and `SpawnedEntity`**

```rust
use open_entities::{hello, Api, EntityComponents, ExportError, ImportError};
```

Add `SpawnedEntity` struct with private fields and `#[wasm_bindgen(getter)]` for `index` and `generation` per spec.

- [ ] **Step 2: Add `load_templates_yaml` and `spawn_entity` with `js_name`**

```rust
#[wasm_bindgen(js_name = loadTemplatesYaml)]
pub fn load_templates_yaml(&mut self, yaml: &str) -> Result<(), JsValue> { ... }

#[wasm_bindgen(js_name = spawnEntity)]
pub fn spawn_entity(
    &mut self,
    template_name: &str,
    overrides: JsValue,
) -> Result<SpawnedEntity, JsValue> { ... }
```

Map errors per spec (`ImportError` display, `invalid overrides: …` for serde).

- [ ] **Step 3: Add `js_name` to existing `world_json`**

```rust
#[wasm_bindgen(js_name = getWorldAsJson)]
pub fn world_json(&mut self) -> Result<String, JsValue> { ... }
```

- [ ] **Step 4: Build WASM**

```bash
wasm-pack build wasm-bindings --target nodejs
```

Expected: success.

- [ ] **Step 5: Commit**

```bash
git add wasm-bindings/src/lib.rs
git commit -m "feat(wasm): loadTemplatesYaml, spawnEntity, getWorldAsJson"
```

---

### Task 4: `#[wasm_bindgen_test]`

**Files:**
- Modify: `wasm-bindings/src/lib.rs`

- [ ] **Step 1: Add fixture constant in test module**

```rust
const FIXTURE_YAML: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/spawn_entity_templates.yaml"));
```

- [ ] **Step 2: Write tests (minimum set from spec)**

| Test | Behavior |
|------|----------|
| `load_and_spawn_from_fixture` | `load_templates_yaml(FIXTURE_YAML)`; `spawn_entity("marker", default)`; `world_json()` ok; parsed JSON has `version == 3` and `entities.len() >= 1` |
| `spawn_scout_with_overrides` | load fixture; spawn scout with `EntityComponents { position: Some(...), health: Some(...) }`; export contains `x: 50`, `current: 40` |
| `spawn_without_load_fails` | `spawn_entity` before load → `Err` containing `TemplatesNotLoaded` |
| `unknown_template_fails` | load fixture; spawn `"nope"` → `Err` containing `unknown template` |

Use `wasm_bindgen_test::wasm_bindgen_test` attribute; construct `Simulation::new()` in each test.

- [ ] **Step 3: Run wasm tests**

```bash
wasm-pack test wasm-bindings --node
```

Expected: all pass.

- [ ] **Step 4: Commit**

```bash
git add wasm-bindings/src/lib.rs
git commit -m "test(wasm): spawn and YAML fixture coverage"
```

---

### Task 5: Node demo full cycle

**Files:**
- Modify: `wasm-bindings/demo/run.mjs`

- [ ] **Step 1: Load fixture from disk**

```javascript
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { Simulation } from "../pkg/open_entities_wasm.js";

const fixturePath = join(
  dirname(fileURLToPath(import.meta.url)),
  "../../fixtures/spawn_entity_templates.yaml",
);
const yaml = readFileSync(fixturePath, "utf8");
```

- [ ] **Step 2: Implement spawn loop (match `spawn_entity.rs`)**

```javascript
const sim = new Simulation();
const names = ["marker", "heavy_tank", "tank", "scout", "unit"];
sim.loadTemplatesYaml(yaml);
for (const name of names) {
  const overrides =
    name === "scout"
      ? { position: { x: 50.0, y: 25.0 }, health: { current: 40, max: 100 } }
      : {};
  const spawned = sim.spawnEntity(name, overrides);
  console.log(`spawned ${name}`, spawned.index, spawned.generation);
}
```

- [ ] **Step 3: Assert on `getWorldAsJson()`**

Parse JSON; assert `version === 3`, `entities.length === 5`; find scout entity with `position.x === 50` and `health.current === 40` (search by `entity_type` or position if present in export).

Throw `Error` with message on mismatch (non-zero exit).

- [ ] **Step 4: Run demo**

```bash
make wasm-demo
```

Expected: build + run succeeds.

- [ ] **Step 5: Commit**

```bash
git add wasm-bindings/demo/run.mjs
git commit -m "feat(wasm): demo loadTemplatesYaml spawn cycle"
```

---

### Task 6: Makefile targets

**Files:**
- Modify: `Makefile`

- [ ] **Step 1: Add `wasm-test`**

```makefile
wasm-test:
	@command -v wasm-pack >/dev/null 2>&1 || { echo "wasm-pack not found. Install with: cargo install wasm-pack"; exit 1; }
	wasm-pack test wasm-bindings --node
```

- [ ] **Step 2: Add `wasm-check` (optional aggregate)**

```makefile
wasm-check: wasm-demo wasm-test
```

Update `.PHONY` line.

- [ ] **Step 3: Verify**

```bash
make wasm-check
```

- [ ] **Step 4: Commit**

```bash
git add Makefile
git commit -m "chore: add make wasm-test and wasm-check"
```

---

### Task 7: Final verification

- [ ] **Step 1: Native tests**

```bash
cargo test
```

Expected: all pass.

- [ ] **Step 2: WASM gate**

```bash
make wasm-check
```

Expected: demo prints spawns + assertions pass; wasm tests pass.

---

## Spec compliance checklist

| Spec requirement | Task |
|------------------|------|
| Root `fixtures/spawn_entity_templates.yaml` | 1 |
| `loadTemplatesYaml` / `spawnEntity` / `getWorldAsJson` | 3 |
| `SpawnedEntity { index, generation }` | 3 |
| `serde_wasm_bindgen` overrides | 3 |
| `#[wasm_bindgen_test]` + fixture | 4 |
| `make wasm-demo` full cycle | 5 |
| `make wasm-test` | 6 |
| No lib logic changes | — |
