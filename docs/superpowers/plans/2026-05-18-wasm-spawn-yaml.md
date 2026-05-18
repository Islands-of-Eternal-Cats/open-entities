# WASM Spawn + YAML (Node) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend `wasm-bindings` so Node/JS can run the full `spawn_entity` cycle (load YAML → spawn with overrides → `getWorldAsJson`), with shared root `fixtures/`, camelCase JS method names, and `make wasm-test` guarding `wasm32`.

**Architecture:** Thin `open_entities_wasm::Simulation` forwards to existing `Api::load_templates_yaml`, `spawn_entity`, and `world_json`. Overrides cross the boundary via `serde_wasm_bindgen::from_value` into `EntityComponents`. YAML lives in `fixtures/spawn_entity_templates.yaml` for the lib example, Node demo, and wasm tests.

**Tech Stack:** Rust workspace, `wasm-bindgen 0.2`, `serde-wasm-bindgen 0.6`, `wasm-bindgen-test 0.3`, `serde_json` (wasm test parsing), `wasm-pack` (Node), Node ESM (`.mjs`).

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
| `fixtures/spawn_entity_templates.yaml` | Canonical YAML (byte-for-byte from `spawn_entity.rs` `TEMPLATES_YAML`) |
| `open-entities-lib/examples/spawn_entity.rs` | Load fixture via `include_str!` |
| `wasm-bindings/Cargo.toml` | `serde-wasm-bindgen`; dev-deps `wasm-bindgen-test`, `serde_json` |
| `wasm-bindings/src/lib.rs` | `SpawnedEntity`, bindings, `js_name`, `#[wasm_bindgen_test]` |
| `wasm-bindings/demo/run.mjs` | Full spawn cycle + assertions; camelCase API |
| `Makefile` | `wasm-test`, `wasm-check` |

**Unchanged:** `open-entities-lib` import/export logic, schema v3, merge rules.

**Out of scope:** GitHub Actions, browser target, TS defs, camelCase on component keys (`move_target`, etc.).

---

### Task 1: Workspace fixture

**Files:**
- Create: `fixtures/spawn_entity_templates.yaml`
- Modify: `open-entities-lib/examples/spawn_entity.rs`

- [ ] **Step 1: Create fixture file**

Create `fixtures/spawn_entity_templates.yaml` with this exact content (leading blank line matches the raw string in `spawn_entity.rs` today):

```yaml

entities:
  unit:
    faction: 1
    health:
      current: 100
      max: 100

  scout:
    template: unit
    position: { x: 10.0, y: 5.0 }
    velocity: { vx: 2.0, vy: 0.0 }
    move_target: { x: 20.0, y: 0.0 }
    health:
      current: 80
      max: 100

  tank:
    template: unit
    faction: 2
    velocity: { vx: 0.5, vy: 0.0 }
    health:
      current: 200
      max: 200

  heavy_tank:
    template: [unit, tank]
    faction: 3
    position: { x: 0.0, y: 0.0 }
    health:
      current: 150
      max: 300

  marker: {}
```

- [ ] **Step 2: Point example at fixture**

In `open-entities-lib/examples/spawn_entity.rs`, replace lines 11–45 (`const TEMPLATES_YAML: &str = r"..."`) with:

```rust
const TEMPLATES_YAML: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/spawn_entity_templates.yaml"));
```

- [ ] **Step 3: Verify example still runs**

Run:

```bash
cargo run -p open_entities --example spawn_entity
```

Expected: prints five `spawned …` lines and pretty JSON with `"version": 3` and five entities; no load/spawn errors on stderr.

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

Replace the `[dependencies]` and add `[dev-dependencies]`:

```toml
[dependencies]
open_entities = { path = "../open-entities-lib" }
wasm-bindgen = "0.2"
serde-wasm-bindgen = "0.6"

[dev-dependencies]
wasm-bindgen-test = "0.3"
serde_json = { workspace = true }
```

- [ ] **Step 2: Verify host check compiles**

Run:

```bash
cargo check -p open_entities_wasm
```

Expected: success (bindings not added yet; only dependency graph change).

- [ ] **Step 3: Commit**

```bash
git add wasm-bindings/Cargo.toml
git commit -m "chore(wasm): add serde-wasm-bindgen and wasm-bindgen-test"
```

---

### Task 3: `SpawnedEntity` type

**Files:**
- Modify: `wasm-bindings/src/lib.rs`

- [ ] **Step 1: Add `SpawnedEntity` struct**

After the `use` lines at the top of `wasm-bindings/src/lib.rs`, extend imports and add:

```rust
use open_entities::{hello, Api, EntityComponents, ExportError, ImportError};
```

Add before `#[wasm_bindgen] pub struct Simulation`:

```rust
#[wasm_bindgen]
pub struct SpawnedEntity {
    index: u32,
    generation: u32,
}

#[wasm_bindgen]
impl SpawnedEntity {
    #[wasm_bindgen(getter)]
    pub fn index(&self) -> u32 {
        self.index
    }

    #[wasm_bindgen(getter)]
    pub fn generation(&self) -> u32 {
        self.generation
    }
}
```

- [ ] **Step 2: Build WASM**

Run:

```bash
wasm-pack build wasm-bindings --target nodejs
```

Expected: success.

- [ ] **Step 3: Commit**

```bash
git add wasm-bindings/src/lib.rs
git commit -m "feat(wasm): add SpawnedEntity return type"
```

---

### Task 4: `loadTemplatesYaml` and `spawnEntity`

**Files:**
- Modify: `wasm-bindings/src/lib.rs` (`impl Simulation`)

- [ ] **Step 1: Add `load_templates_yaml`**

Inside `impl Simulation`, add:

```rust
/// JS: `loadTemplatesYaml(yaml)`
#[wasm_bindgen(js_name = loadTemplatesYaml)]
pub fn load_templates_yaml(&mut self, yaml: &str) -> Result<(), JsValue> {
    self.api
        .load_templates_yaml(yaml)
        .map_err(|e: ImportError| JsValue::from_str(&e.to_string()))
}
```

- [ ] **Step 2: Add `spawn_entity`**

Still in `impl Simulation`, add:

```rust
/// JS: `spawnEntity(templateName, overrides)`
#[wasm_bindgen(js_name = spawnEntity)]
pub fn spawn_entity(
    &mut self,
    template_name: &str,
    overrides: JsValue,
) -> Result<SpawnedEntity, JsValue> {
    let overrides: EntityComponents = serde_wasm_bindgen::from_value(overrides)
        .map_err(|e| JsValue::from_str(&format!("invalid overrides: {e}")))?;
    let entity = self
        .api
        .spawn_entity(template_name, overrides)
        .map_err(|e: ImportError| JsValue::from_str(&e.to_string()))?;
    Ok(SpawnedEntity {
        index: entity.index_u32(),
        generation: entity.generation().to_bits(),
    })
}
```

- [ ] **Step 3: Build WASM**

Run:

```bash
wasm-pack build wasm-bindings --target nodejs
```

Expected: success.

- [ ] **Step 4: Commit**

```bash
git add wasm-bindings/src/lib.rs
git commit -m "feat(wasm): loadTemplatesYaml and spawnEntity bindings"
```

---

### Task 5: Rename JS export `getWorldAsJson`

**Files:**
- Modify: `wasm-bindings/src/lib.rs`

- [ ] **Step 1: Add `js_name` to `world_json`**

Change the existing method to:

```rust
/// JS: `getWorldAsJson()`
#[wasm_bindgen(js_name = getWorldAsJson)]
pub fn world_json(&mut self) -> Result<String, JsValue> {
    self.api
        .world_json()
        .map_err(|e: ExportError| JsValue::from_str(&e.to_string()))
}
```

- [ ] **Step 2: Build WASM**

Run:

```bash
wasm-pack build wasm-bindings --target nodejs
```

Expected: success.

- [ ] **Step 3: Commit**

```bash
git add wasm-bindings/src/lib.rs
git commit -m "feat(wasm): expose getWorldAsJson js_name for world_json"
```

---

### Task 6: `#[wasm_bindgen_test]` suite

**Files:**
- Modify: `wasm-bindings/src/lib.rs`

**Error strings to assert** (from `ImportError` `Display` in `open-entities-lib/src/import/mod.rs`):
- Not loaded: `templates not loaded; call load_templates_yaml first`
- Unknown template: `unknown template name: nope`

- [ ] **Step 1: Add test module at bottom of `lib.rs`**

```rust
#[cfg(test)]
mod wasm_tests {
    use super::*;
    use open_entities::components::{Health, Position};
    use open_entities::EntityComponents;
    use wasm_bindgen_test::*;

    const FIXTURE_YAML: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/spawn_entity_templates.yaml"
    ));

    fn empty_overrides() -> JsValue {
        serde_wasm_bindgen::to_value(&EntityComponents::default()).expect("empty overrides")
    }

    fn scout_overrides() -> JsValue {
        serde_wasm_bindgen::to_value(&EntityComponents {
            position: Some(Position { x: 50.0, y: 25.0 }),
            health: Some(Health {
                current: 40,
                max: 100,
            }),
            ..Default::default()
        })
        .expect("scout overrides")
    }

    fn err_string(result: Result<SpawnedEntity, JsValue>) -> String {
        result
            .unwrap_err()
            .as_string()
            .expect("JsValue error should be a string")
    }

    #[wasm_bindgen_test]
    fn load_and_spawn_from_fixture() {
        let mut sim = Simulation::new();
        sim.load_templates_yaml(FIXTURE_YAML)
            .expect("load fixture");
        sim.spawn_entity("marker", empty_overrides())
            .expect("spawn marker");
        let json = sim.world_json().expect("export world");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(value["version"], 3);
        let entities = value["entities"]
            .as_array()
            .expect("entities array");
        assert!(!entities.is_empty());
    }

    #[wasm_bindgen_test]
    fn spawn_scout_with_overrides() {
        let mut sim = Simulation::new();
        sim.load_templates_yaml(FIXTURE_YAML)
            .expect("load fixture");
        sim.spawn_entity("scout", scout_overrides())
            .expect("spawn scout");
        let json = sim.world_json().expect("export world");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("valid JSON");
        let entities = value["entities"]
            .as_array()
            .expect("entities array");
        let scout = entities
            .iter()
            .find(|e| e["entity_type"] == "scout")
            .expect("scout row in export");
        assert_eq!(scout["position"]["x"], 50.0);
        assert_eq!(scout["position"]["y"], 25.0);
        assert_eq!(scout["health"]["current"], 40);
        assert_eq!(scout["health"]["max"], 100);
    }

    #[wasm_bindgen_test]
    fn spawn_without_load_fails() {
        let mut sim = Simulation::new();
        let msg = err_string(sim.spawn_entity("marker", empty_overrides()));
        assert!(
            msg.contains("templates not loaded"),
            "expected TemplatesNotLoaded message, got: {msg}"
        );
    }

    #[wasm_bindgen_test]
    fn unknown_template_fails() {
        let mut sim = Simulation::new();
        sim.load_templates_yaml(FIXTURE_YAML)
            .expect("load fixture");
        let msg = err_string(sim.spawn_entity("nope", empty_overrides()));
        assert!(
            msg.contains("unknown template name: nope"),
            "expected UnknownTemplate message, got: {msg}"
        );
    }
}
```

- [ ] **Step 2: Run wasm tests**

Run:

```bash
wasm-pack test wasm-bindings --node
```

Expected: 4 tests pass (`load_and_spawn_from_fixture`, `spawn_scout_with_overrides`, `spawn_without_load_fails`, `unknown_template_fails`).

- [ ] **Step 3: Commit**

```bash
git add wasm-bindings/src/lib.rs wasm-bindings/Cargo.toml
git commit -m "test(wasm): spawn and YAML fixture coverage"
```

---

### Task 7: Node demo full cycle

**Files:**
- Modify: `wasm-bindings/demo/run.mjs`

- [ ] **Step 1: Replace demo with full spawn cycle**

Replace entire `wasm-bindings/demo/run.mjs` with:

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

const sim = new Simulation();
console.log(sim.hello());

sim.loadTemplatesYaml(yaml);

const names = ["marker", "heavy_tank", "tank", "scout", "unit"];
const spawnedIds = [];

for (const name of names) {
  const overrides =
    name === "scout"
      ? { position: { x: 50.0, y: 25.0 }, health: { current: 40, max: 100 } }
      : {};
  const spawned = sim.spawnEntity(name, overrides);
  console.log(`spawned ${name}`, spawned.index, spawned.generation);
  spawnedIds.push({ index: spawned.index, generation: spawned.generation });
}

const json = sim.getWorldAsJson();
const parsed = JSON.parse(json);

if (parsed.version !== 3) {
  throw new Error(`expected version 3, got ${parsed.version}`);
}
if (!Array.isArray(parsed.entities) || parsed.entities.length !== 5) {
  throw new Error(`expected 5 entities, got ${parsed.entities?.length}`);
}

const scout = parsed.entities.find((e) => e.entity_type === "scout");
if (!scout) {
  throw new Error("scout entity missing from export");
}
if (scout.position?.x !== 50.0 || scout.position?.y !== 25.0) {
  throw new Error(`scout position override missing: ${JSON.stringify(scout.position)}`);
}
if (scout.health?.current !== 40 || scout.health?.max !== 100) {
  throw new Error(`scout health override missing: ${JSON.stringify(scout.health)}`);
}

for (const { index, generation } of spawnedIds) {
  const found = parsed.entities.some(
    (e) => e.id?.index === index && e.id?.generation === generation,
  );
  if (!found) {
    throw new Error(`spawned id ${index}/${generation} not found in export`);
  }
}

console.log("wasm spawn demo ok");
```

- [ ] **Step 2: Run demo**

Run:

```bash
make wasm-demo
```

Expected: `wasm-pack build` succeeds; demo prints `hello`, five `spawned …` lines, and `wasm spawn demo ok`; exit code 0.

- [ ] **Step 3: Commit**

```bash
git add wasm-bindings/demo/run.mjs
git commit -m "feat(wasm): demo loadTemplatesYaml spawn cycle"
```

---

### Task 8: Makefile targets

**Files:**
- Modify: `Makefile`

- [ ] **Step 1: Extend `.PHONY` and add targets**

Change line 1 to:

```makefile
.PHONY: test example example-world-json wasm-demo wasm-test wasm-check
```

Append after `wasm-demo` target:

```makefile
wasm-test:
	@command -v wasm-pack >/dev/null 2>&1 || { echo "wasm-pack not found. Install with: cargo install wasm-pack"; exit 1; }
	wasm-pack test wasm-bindings --node

wasm-check: wasm-demo wasm-test
```

- [ ] **Step 2: Verify aggregate target**

Run:

```bash
make wasm-check
```

Expected: demo and all four wasm tests pass.

- [ ] **Step 3: Commit**

```bash
git add Makefile
git commit -m "chore: add make wasm-test and wasm-check"
```

---

### Task 9: Final verification

- [ ] **Step 1: Native tests**

Run:

```bash
cargo test
```

Expected: all workspace tests pass (lib unchanged except example path).

- [ ] **Step 2: WASM gate**

Run:

```bash
make wasm-check
```

Expected: demo + wasm tests pass.

- [ ] **Step 3: Optional single-target smoke**

Run:

```bash
wasm-pack build wasm-bindings --target nodejs
```

Expected: success (confirms `serde-wasm-bindgen` + `wasm-bindgen 0.2` pairing).

---

## Spec compliance (self-review)

| Spec requirement | Task |
|------------------|------|
| Root `fixtures/spawn_entity_templates.yaml` | 1 |
| Example uses `include_str!` | 1 |
| `serde-wasm-bindgen = "0.6"` | 2 |
| `SpawnedEntity { index, generation }` + getters | 3 |
| `loadTemplatesYaml` / `spawnEntity` | 4 |
| `getWorldAsJson` `js_name` on `world_json` | 5 |
| Four `#[wasm_bindgen_test]` cases | 6 |
| Demo: fixture via `import.meta.url`, spawn order, assertions | 7 |
| `make wasm-test`, `make wasm-check` | 8 |
| `cargo test` unchanged lib logic | 9 |
| `hello()` unchanged | — (no edit) |
| No CI / browser / lib logic changes | — |

**Gaps:** None.

**Placeholder scan:** None.

**Type consistency:** `SpawnedEntity` fields match `EntityIdExport` (`index`, `generation` via `index_u32()` / `generation().to_bits()`). JS method names match spec table.
