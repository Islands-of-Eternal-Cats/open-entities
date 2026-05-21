# Design: WASM Web Demo (Vite + Canvas)

**Date:** 2026-05-21  
**Status:** Approved (brainstorming)  
**Depends on:** [WASM Spawn + YAML (Node)](2026-05-18-wasm-spawn-yaml-design.md), [Tick / Movement / Seek](2026-05-19-tick-movement-seek-design.md)  
**Scope:** Browser interactive demo for `wasm-bindings` using Vite, canvas visualization, `make wasm-web-demo`, and Playwright e2e included in `make wasm-check`. No changes to `open-entities-lib`, `wasm-bindings/src/lib.rs`, or Node demo behavior.

## Summary

Add a **Vite-based web app** under `wasm-bindings/web/` that loads the existing `Simulation` WASM API (`--target bundler`), runs the **same fixed spawn scenario** as `demo/run.mjs`, and renders entities on a **2D canvas** with **Play / Pause** driving `tick(16)` via `requestAnimationFrame`.

Verification extends **`wasm-check`** with a **headless Playwright** test against `vite preview`, asserting scout reaches the fixture move target (same end condition as the tick block in `run.mjs`).

The Node demo (`make wasm-demo`, `--target nodejs`) remains unchanged.

## Goals

- Interactive browser demo: canvas, circles by `position`, color by `faction`, `entity_type` label.
- Fixed scenario on load: fetch fixture YAML, spawn `marker` → `heavy_tank` → `tank` → `scout` → `unit` with scout overrides identical to `demo/run.mjs`.
- `make wasm-web-demo`: `wasm-pack build --target bundler` + Vite dev server + auto-open browser (`--open`).
- `make wasm-web-check`: production build + Playwright on `vite preview`.
- `make wasm-check`: `wasm-demo` + `wasm-test` + `wasm-web-check`.

## Non-Goals

- Sandbox UI (YAML editor, spawn controls, JSON panel, reset, tick speed).
- `npm publish`, GitHub Actions (local Make targets only).
- Changes to `Simulation` Rust API or `open-entities-lib`.
- TypeScript type generation for `EntityComponents`.
- Replacing or removing the Node demo.

## Decisions (Brainstorming)

| Topic | Choice | Rationale |
|-------|--------|-----------|
| Demo type | **Interactive canvas simulation** | User: show movement, not devtools-only |
| Interaction v1 | **Fixed scenario + Play/Pause** | Same fidelity as `run.mjs`; minimal UI |
| Tooling | **Vite + `vite-plugin-wasm` + `vite-plugin-top-level-await`** | User preference; `bundler` target fits Vite |
| `make wasm-web-demo` | **Build bundler + `npm run dev -- --open`** | User: one command, browser opens |
| Rendering | **Circles + faction color + `entity_type` label** | Simple, readable RTS-style preview |
| Verification | **Playwright in `wasm-check`** | User: headless guard; poll via `window.__openEntitiesDemo` |
| Fixture in web | **One-time copy → `public/fixtures/`** (committed) | Web demo owns its YAML; may diverge from repo `fixtures/` later |
| Rust / Node | **No changes** | Existing API and `wasm-demo` sufficient |

## Section 1: Repository Layout

```text
open-entities/
├── fixtures/
│   └── spawn_entity_templates.yaml      # canonical for lib / Node / wasm tests
├── Makefile                             # + wasm-web-* targets; wasm-check extended
└── wasm-bindings/
    ├── pkg/                             # gitignored; nodejs + bundler output
    ├── src/lib.rs                       # unchanged
    ├── demo/run.mjs                     # unchanged (reads repo fixtures/)
    └── web/
        ├── package.json
        ├── vite.config.ts
        ├── index.html
        ├── public/
        │   └── fixtures/
        │       └── spawn_entity_templates.yaml   # copied once at demo setup; committed
        ├── src/
        │   ├── main.ts                  # init, loop, UI
        │   └── render.ts              # canvas projection + draw
        └── e2e/
            └── demo.spec.ts           # Playwright
```

### Vite dependencies (web/package.json)

- `vite` (dev)
- `vite-plugin-wasm`, `vite-plugin-top-level-await`
- `@playwright/test` (dev)
- TypeScript optional; plain `.ts` with minimal types is fine

### wasm-pack target

| Consumer | Target |
|----------|--------|
| `make wasm-demo` | `nodejs` (unchanged) |
| `make wasm-web-*` | `bundler` |

Both write to `wasm-bindings/pkg/`. `wasm-check` runs nodejs demo first, then bundler web build — order is intentional and safe.

### Web demo fixture (independent copy)

At **initial creation** of `wasm-bindings/web/`, copy once from the repo canonical file:

```bash
mkdir -p wasm-bindings/web/public/fixtures
cp fixtures/spawn_entity_templates.yaml wasm-bindings/web/public/fixtures/
```

- The copy is **committed** in git (`public/fixtures/spawn_entity_templates.yaml`).
- **No** `sync-fixtures` script, **no** `predev`/`prebuild` copy step.
- After setup, the web fixture **may diverge** from `fixtures/spawn_entity_templates.yaml` (e.g. demo-specific templates, move targets, extra entities). Node demo, Rust examples, and `#[wasm_bindgen_test]` keep using repo-root `fixtures/`.
- Playwright e2e assertions use values from the **web** fixture (e.g. scout `move_target` at `(20, 0)` in the initial copy); if the web YAML changes, update e2e accordingly.

## Section 2: Runtime Data Flow

### Initialization (once on load)

1. `import init, { Simulation } from '../pkg/open_entities_wasm.js'` (or Vite alias `@wasm`).
2. `await init()`.
3. `fetch('/fixtures/spawn_entity_templates.yaml')` → text.
4. `sim.loadTemplatesYaml(yaml)`.
5. For each name in `['marker', 'heavy_tank', 'tank', 'scout', 'unit']`:
   - `scout`: overrides `{ position: { x: 50, y: 25 }, health: { current: 40, max: 100 } }`
   - others: `{}`
   - `sim.spawnEntity(name, overrides)`.
6. `render()` from `getWorldAsJson()`.

### Play / Pause

| State | Behavior |
|-------|----------|
| **Pause** (default) | No `requestAnimationFrame`; world frozen after init |
| **Play** | Each animation frame: `sim.tick(16)` → `getWorldAsJson()` → `render()` |
| **Pause** (from Play) | Cancel `rAF`; simulation state preserved |

Use `tick(16)` only while playing (~60 Hz logical step, matches Node tick demo).

### Canvas rendering

| Rule | Detail |
|------|--------|
| Entities drawn | Export rows with `position` present |
| Shape | Filled circle, radius 8 (world units before scale) |
| Color | Map `faction` (1 → blue, 2 → red, 3 → green; missing → gray) |
| Label | `entity_type` string offset from circle center |
| Y axis | Invert world Y when mapping to canvas (math Y-up → screen Y-down) |
| Viewport | On first frame (and after resize): bounding box of all positioned entities + 10% padding; uniform scale |

Entities without `position` in export (e.g. `marker`, `unit`) are skipped — expected for v1.

### E2E test hook

Expose on `window` for Playwright only (no Rust changes):

```ts
window.__openEntitiesDemo = {
  getWorld: () => JSON.parse(sim.getWorldAsJson()),
  isPlaying: () => boolean,
};
```

## Section 3: Makefile, Gitignore, Errors

### Makefile targets

```makefile
.PHONY: wasm-web-build wasm-web-demo wasm-web-check wasm-check

wasm-web-build:
	wasm-pack build wasm-bindings --target bundler
	cd wasm-bindings/web && npm ci && npm run build

wasm-web-demo:
	@command -v wasm-pack >/dev/null 2>&1 || { echo "wasm-pack not found"; exit 1; }
	@command -v node >/dev/null 2>&1 || { echo "node not found"; exit 1; }
	wasm-pack build wasm-bindings --target bundler
	cd wasm-bindings/web && npm install && npm run dev -- --open

wasm-web-check:
	wasm-pack build wasm-bindings --target bundler
	cd wasm-bindings/web && npm ci && npm run build
	cd wasm-bindings/web && npx playwright test

wasm-check: wasm-demo wasm-test wasm-web-check
```

`wasm-web-check` starts `vite preview` via Playwright `webServer` config (port **4173**, consistent with Vite default).

### Gitignore additions

```
wasm-bindings/pkg/
wasm-bindings/web/node_modules/
wasm-bindings/web/dist/
wasm-bindings/web/test-results/
wasm-bindings/web/playwright-report/
```

### Error handling (UI)

| Failure | UI |
|---------|-----|
| WASM `init` fails | `#status` error text; disable Play |
| `fetch` fixture fails | Same |
| `loadTemplatesYaml` / `spawnEntity` error | Show `JsValue` string; do not enable Play |
| `tick` / `getWorldAsJson` during Play | `console.error`; auto-switch to Pause |

### README addition

New subsection **Web demo**:

- Prerequisites: Rust wasm32 target, `wasm-pack`, Node 20+, `npm`
- First-time e2e: `cd wasm-bindings/web && npx playwright install chromium`
- Run: `make wasm-web-demo`
- Full WASM gate: `make wasm-check`

## Section 4: Playwright E2E

**File:** `wasm-bindings/web/e2e/demo.spec.ts`

**webServer** (playwright.config): `npm run preview` after build, port 4173.

**Test steps:**

1. `page.goto('/')` — wait for `canvas` visible.
2. `page.waitForFunction(() => window.__openEntitiesDemo?.getWorld()?.entities?.some(e => e.entity_type === 'scout' && e.position?.x === 50))`.
3. Click button `#play` (or `getByRole('button', { name: 'Play' })`).
4. Poll up to **30s**: scout entity has `move_target === undefined` and `position` within **0.1** of `(20, 0)` (target from **web** `public/fixtures/spawn_entity_templates.yaml`, not repo `fixtures/`).

This mirrors the assertion block at the end of `wasm-bindings/demo/run.mjs` (tick loop until arrival).

## Section 5: Vite Config Sketch

```ts
import { defineConfig } from 'vite';
import wasm from 'vite-plugin-wasm';
import topLevelAwait from 'vite-plugin-top-level-await';

export default defineConfig({
  root: '.',
  plugins: [wasm(), topLevelAwait()],
  resolve: {
    alias: { '@wasm': new URL('../pkg/open_entities_wasm.js', import.meta.url).pathname },
  },
  server: { port: 5173 },
  preview: { port: 4173 },
});
```

`package.json` scripts:

| Script | Command |
|--------|---------|
| `dev` | `vite` |
| `build` | `vite build` |
| `preview` | `vite preview` |

Optional `predev`: fail-fast if `../pkg/` is missing (hint to run `wasm-pack build --target bundler`). No fixture sync hooks.

## Success Criteria

- `make wasm-web-demo` opens browser; canvas shows positioned entities; Play animates scout toward (20, 0); Pause stops animation.
- `make wasm-web-check` passes without manual interaction.
- `make wasm-check` still passes Node demo and `#[wasm_bindgen_test]` after web target rebuild.
- `make wasm-demo` unchanged and passing in isolation.

## References

- Node demo: [`wasm-bindings/demo/run.mjs`](../../../wasm-bindings/demo/run.mjs)
- WASM API: [`wasm-bindings/src/lib.rs`](../../../wasm-bindings/src/lib.rs)
- Repo fixture (Node / lib / tests): [`fixtures/spawn_entity_templates.yaml`](../../../fixtures/spawn_entity_templates.yaml)
- Web demo fixture (independent after setup): [`wasm-bindings/web/public/fixtures/spawn_entity_templates.yaml`](../../../wasm-bindings/web/public/fixtures/spawn_entity_templates.yaml)
- Prior scope exclusion: [`2026-05-18-wasm-spawn-yaml-design.md`](2026-05-18-wasm-spawn-yaml-design.md) (browser was non-goal; superseded for web demo only)
