# Design: WASM Bindings PoC (Node)

**Date:** 2026-05-17  
**Status:** Approved (brainstorming)  
**Depends on:** [Core, Api, and World JSON Export](2026-05-16-core-api-world-export-design.md), [Component Registry](2026-05-17-component-registry-design.md)  
**Scope:** Add `wasm-bindings/` workspace crate, build with `wasm-pack` for Node, expose `hello` and `world_json` via `#[wasm_bindgen]`, and run a minimal Node demo. No spawn/YAML, browser demo, or CI in this increment.

## Summary

`open_entities` already exposes `hello()`, `Api`, and `Api::world_json()` (JSON schema version 3) for native Rust. This increment proves the same library compiles to `wasm32-unknown-unknown` and is callable from Node through a thin `wasm-bindgen` layer.

Deliverables:

1. Workspace member **`wasm-bindings/`** (`open_entities_wasm` package) depending on `open_entities`.
2. **`Simulation`** — `#[wasm_bindgen]` type owning `Api`, with `hello()` and `world_json()`.
3. **`wasm-pack build --target nodejs`** producing `wasm-bindings/pkg/`.
4. **`wasm-bindings/demo/run.mjs`** — `init()` → print greeting and empty-world JSON.
5. **`make wasm-demo`** — build + run (clear error if `wasm-pack` is missing).

Serialization and export logic stay in `open-entities-lib`; bindings only forward calls and map errors to `JsValue`.

## Goals

- Verify `bevy_ecs` + `open_entities` link successfully for `wasm32-unknown-unknown`.
- Document the standard build path (`wasm-pack`, Node target) for future browser/game work.
- Provide a copy-pasteable Node script showing how to load and call the module.
- Keep bindings thin so later methods (`load_templates_yaml`, `spawn_entity`) add without redesign.

## Non-Goals

- Browser demo (`--target web`, static `index.html`).
- `load_templates_yaml`, `spawn_entity`, `serde_wasm_bindgen`, or `EntityComponents` on the JS boundary.
- Simulation `tick`, systems, or game loop in WASM.
- TypeScript definitions, npm publish, or `package.json` workspace for consumers.
- `wasm32` CI job or `rust-toolchain.toml` changes.
- Changes to `world_json` schema or export logic in `open-entities-lib`.
- README expansion beyond optional one-liner for `make wasm-demo` (optional follow-up).

## Decisions (Brainstorming)

| Topic | Choice | Rationale |
|-------|--------|-----------|
| Consumer | Minimal PoC (not full browser game yet) | User choice: prove toolchain before game APIs |
| Demo runtime | **Node only** | Simpler than browser; easy local/CI-style check |
| Build tool | **`wasm-pack`** + `wasm-bindgen` | One command; standard glue JS; vs manual `wasm-bindgen-cli` |
| Wrapper type | `Simulation` owns `Api` | `world_json` needs `&mut Api`; single instance in demo |
| `hello` | Instance method on `Simulation` | One object in demo; can add free function later |
| Errors | `Result<String, JsValue>` with `from_str` | No custom JS error types in PoC |
| JSON location | `open_entities` only | No duplicate export in bindings |
| Output dir | `wasm-bindings/pkg/` | `wasm-pack` default; **gitignored** |
| Workspace | Add `"wasm-bindings"` to root `members` | Matches existing layout notes in hello spec |

## Architecture

```text
Node (demo/run.mjs)
    │ import ../pkg/open_entities_wasm.js
    ▼
open_entities_wasm (#[wasm_bindgen] Simulation)
    │ owns Api
    ▼
open_entities::Api::world_json()  →  schema v3 JSON string
open_entities::hello()            →  "Hello, world!"
```

**Data flow (`world_json`):**

1. JS calls `simulation.world_json()` (mutable borrow on WASM side).
2. `Simulation` forwards to `api.world_json()`.
3. Lib runs export query + `serde_json` (unchanged).
4. Ok → JSON string to JS; Err → `JsValue` with display message.

## Repository Layout

```text
open-entities/
├── Cargo.toml                          # + "wasm-bindings" in [workspace.members]
├── Makefile                            # + wasm-demo target
├── .gitignore                          # + wasm-bindings/pkg/
└── wasm-bindings/
    ├── Cargo.toml                      # package open_entities_wasm, cdylib + rlib
    ├── src/
    │   └── lib.rs                      # Simulation + wasm_bindgen exports
    ├── demo/
    │   └── run.mjs                     # node demo/run.mjs (cwd: wasm-bindings/)
    └── pkg/                            # generated; not committed
```

## Crate: `open_entities_wasm`

### `Cargo.toml`

```toml
[package]
name = "open_entities_wasm"
version.workspace = true
edition.workspace = true
license.workspace = true

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
open_entities = { path = "../open-entities-lib" }
wasm-bindgen = "0.2"
```

No `serde_wasm_bindgen` in this increment.

### Public WASM API (`src/lib.rs`)

```rust
use open_entities::{hello, Api, ExportError};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Simulation {
    api: Api,
}

#[wasm_bindgen]
impl Simulation {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { api: Api::new() }
    }

    /// Returns the canonical greeting from `open_entities::hello()`.
    pub fn hello(&self) -> String {
        hello().to_owned()
    }

    /// Serializes the ECS world to JSON (schema version 3).
    pub fn world_json(&mut self) -> Result<String, JsValue> {
        self.api
            .world_json()
            .map_err(|e: ExportError| JsValue::from_str(&e.to_string()))
    }
}
```

### Expected demo output

On an empty world:

```text
Hello, world!
{"version":3,"entities":[]}
```

(Exact whitespace of JSON follows lib compact serialization; demo may `JSON.parse` for readability but must not require pretty printing.)

## Build and Run

**Prerequisites (document in Makefile comment or spec-only):**

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack   # if not present
```

**Build:**

```bash
wasm-pack build wasm-bindings --target nodejs
```

Run from repository root or `wasm-bindings/`; `Makefile` should use a fixed path.

**Demo (`wasm-bindings/demo/run.mjs`):**

```javascript
import init, { Simulation } from "../pkg/open_entities_wasm.js";

await init();
const sim = new Simulation();
console.log(sim.hello());
console.log(sim.world_json());
```

Execute with `node demo/run.mjs` and **current working directory `wasm-bindings/`** so the relative import to `pkg/` resolves.

**Makefile target:**

```makefile
wasm-demo:
	wasm-pack build wasm-bindings --target nodejs
	cd wasm-bindings && node demo/run.mjs
```

If `wasm-pack` is missing, fail with a one-line hint to install it.

## Error Handling

| Layer | Behavior |
|-------|----------|
| `ExportError::Serde` | `JsValue::from_str` with `Display` message |
| Panics in lib | Allowed to abort WASM instance (no `console_error_panic_hook` required in PoC) |
| Missing `pkg/` | Demo fails at import; `make wasm-demo` always builds first |

## Testing and Verification

| Check | Command |
|-------|---------|
| Native lib unchanged | `cargo test` |
| WASM compiles | `wasm-pack build wasm-bindings --target nodejs` |
| Node runtime | `make wasm-demo` prints greeting + valid JSON with `"version":3` and `"entities":[]` |

Optional follow-up (not required for PoC approval): `#[wasm_bindgen_test]` in `open_entities_wasm` via `wasm-pack test --node`.

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| `bevy_ecs` or transitive deps fail on `wasm32` | PoC blocked until build succeeds; fix with feature flags or dependency pins in bindings crate only if needed |
| `wasm-pack` not installed | Makefile error message; document in spec |
| Large `.wasm` binary | Acceptable for PoC; size optimization later |

## Future Extensions (Out of Scope)

1. `load_templates_yaml(yaml: &str)` and `spawn_entity(name, overrides: JsValue)` using `serde_wasm_bindgen`.
2. `wasm-pack build --target web` + static browser page.
3. CI job: install `wasm-pack`, run `make wasm-demo`.
4. Re-export `Entity` id from spawn as `{ index, generation }` object for JS tracking.
5. `tick(dt: f32)` when systems land in `open_entities`.

## Alternatives Considered

| Approach | Why not chosen for v1 |
|----------|-------------------------|
| `cargo build --target wasm32` + manual `wasm-bindgen` CLI | More Makefile steps; worse ergonomics |
| `#[wasm_bindgen_test]` only, no `run.mjs` | Does not demonstrate consumer JS import path |
| Browser-first demo | User chose Node for first PoC |
