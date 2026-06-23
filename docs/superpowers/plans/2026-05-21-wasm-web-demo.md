# WASM Web Demo (Vite + Canvas) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a browser interactive demo under `wasm-bindings/web/` (Vite + canvas + Play/Pause) wired to existing `Simulation` WASM (`bundler` target), with `make wasm-web-demo`, Playwright e2e in `make wasm-check`, and a one-time committed web fixture copy.

**Architecture:** Vite serves static `public/fixtures/` and bundles TS that imports `../pkg/open_entities_wasm.js` after `wasm-pack build --target bundler`. `main.ts` runs the fixed spawn scenario from `demo/run.mjs`, drives `tick(16)` via `requestAnimationFrame` while playing, and `render.ts` projects world JSON to canvas. Playwright polls `window.__openEntitiesDemo.getWorld()` against the **web** fixture at `wasm-bindings/web/public/fixtures/spawn_entity_templates.yaml`.

**Tech Stack:** Rust `wasm-pack` (`bundler` target), Vite 6, `vite-plugin-wasm`, `vite-plugin-top-level-await`, TypeScript, `@playwright/test`.

**Spec:** `docs/superpowers/specs/2026-05-21-wasm-web-demo-design.md`

**Prerequisites:**

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
# Node 20+ and npm
```

**Out of scope (do not implement):** Changes to `open-entities-lib/`, `wasm-bindings/src/lib.rs`, `wasm-bindings/demo/run.mjs`, repo-root `fixtures/`, Node `--target nodejs` demo, sandbox UI, CI workflows.

---

## File map

| File | Responsibility |
|------|----------------|
| `wasm-bindings/web/package.json` | npm scripts, dev deps, optional `predev` pkg check |
| `wasm-bindings/web/package-lock.json` | Locked deps for `npm ci` in Make targets |
| `wasm-bindings/web/vite.config.ts` | Vite + wasm plugins, `@wasm` alias |
| `wasm-bindings/web/tsconfig.json` | TS for `src/` |
| `wasm-bindings/web/index.html` | canvas, Play/Pause, `#status` |
| `wasm-bindings/web/src/main.ts` | WASM init, spawn, loop, test hook |
| `wasm-bindings/web/src/render.ts` | viewport + canvas draw |
| `wasm-bindings/web/src/vite-env.d.ts` | `@wasm` module types |
| `wasm-bindings/web/public/fixtures/spawn_entity_templates.yaml` | Web-owned YAML (one-time copy from repo `fixtures/`) |
| `wasm-bindings/web/playwright.config.ts` | e2e + `webServer: preview` on 4173 |
| `wasm-bindings/web/e2e/demo.spec.ts` | headless scout arrival |
| `Makefile` | `wasm-web-build`, `wasm-web-demo`, `wasm-web-check`; extend `wasm-check` |
| `.gitignore` | `web/node_modules`, `web/dist`, playwright artifacts |
| `README.md` | Web demo subsection after WASM (Node) |

**Already gitignored:** `wasm-bindings/pkg/` (both `nodejs` and `bundler` builds share this directory).

**Unchanged:** `wasm-bindings/src/lib.rs`, `wasm-bindings/demo/run.mjs`, `fixtures/spawn_entity_templates.yaml` (repo canonical).

---

### Task 1: Web fixture (one-time copy)

**Files:**
- Create: `wasm-bindings/web/public/fixtures/spawn_entity_templates.yaml`

- [ ] **Step 1: Copy canonical fixture**

```bash
mkdir -p wasm-bindings/web/public/fixtures
cp fixtures/spawn_entity_templates.yaml wasm-bindings/web/public/fixtures/
```

- [ ] **Step 2: Verify copy**

```bash
diff -u fixtures/spawn_entity_templates.yaml wasm-bindings/web/public/fixtures/spawn_entity_templates.yaml
```

Expected: no output (identical at setup). Scout template includes `move_target: { x: 20.0, y: 0.0 }` — e2e asserts against **this** file, not repo `fixtures/` after divergence.

- [ ] **Step 3: Commit**

```bash
git add wasm-bindings/web/public/fixtures/spawn_entity_templates.yaml
git commit -m "chore: add web demo fixture copy for wasm-bindings/web"
```

---

### Task 2: Vite project skeleton

**Files:**
- Create: `wasm-bindings/web/package.json`
- Create: `wasm-bindings/web/vite.config.ts`
- Create: `wasm-bindings/web/tsconfig.json`
- Create: `wasm-bindings/web/index.html`

- [ ] **Step 1: Create `package.json`**

```json
{
  "name": "open-entities-web-demo",
  "private": true,
  "type": "module",
  "scripts": {
    "predev": "node -e \"const fs=require('fs'); if(!fs.existsSync('../pkg/open_entities_wasm.js')) { console.error('Missing ../pkg/. Run: wasm-pack build wasm-bindings --target bundler'); process.exit(1); }\"",
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview",
    "e2e": "playwright test"
  },
  "devDependencies": {
    "@playwright/test": "^1.52.0",
    "typescript": "^5.8.0",
    "vite": "^6.3.0",
    "vite-plugin-top-level-await": "^1.5.0",
    "vite-plugin-wasm": "^3.4.0"
  }
}
```

- [ ] **Step 2: Create `vite.config.ts`**

```ts
import { defineConfig } from "vite";
import wasm from "vite-plugin-wasm";
import topLevelAwait from "vite-plugin-top-level-await";
import { fileURLToPath } from "node:url";
import path from "node:path";

const dir = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  root: dir,
  plugins: [wasm(), topLevelAwait()],
  resolve: {
    alias: {
      "@wasm": path.resolve(dir, "../pkg/open_entities_wasm.js"),
    },
  },
  server: { port: 5173 },
  preview: { port: 4173 },
});
```

- [ ] **Step 3: Create `tsconfig.json`**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "noEmit": true,
    "skipLibCheck": true
  },
  "include": ["src"]
}
```

- [ ] **Step 4: Create `index.html`**

```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>OpenEntities Web Demo</title>
    <style>
      body { font-family: system-ui, sans-serif; margin: 1rem; }
      #status { min-height: 1.25rem; color: #b00020; }
      canvas { border: 1px solid #ccc; display: block; margin-top: 0.5rem; }
      button { margin-right: 0.5rem; }
    </style>
  </head>
  <body>
    <h1>OpenEntities Web Demo</h1>
    <p id="status"></p>
    <button type="button" id="play" disabled>Play</button>
    <button type="button" id="pause" disabled>Pause</button>
    <canvas id="world" width="800" height="500"></canvas>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

- [ ] **Step 5: Install and lock dependencies**

```bash
cd wasm-bindings/web && npm install
```

Expected: `node_modules/` and `package-lock.json` created.

- [ ] **Step 6: Commit**

```bash
git add wasm-bindings/web/package.json wasm-bindings/web/package-lock.json \
  wasm-bindings/web/vite.config.ts wasm-bindings/web/tsconfig.json wasm-bindings/web/index.html
git commit -m "chore: scaffold Vite web demo project"
```

---

### Task 3: Canvas renderer

**Files:**
- Create: `wasm-bindings/web/src/render.ts`

- [ ] **Step 1: Implement `render.ts`**

```ts
export type WorldEntity = {
  entity_type?: string;
  faction?: number;
  position?: { x: number; y: number };
  move_target?: { x: number; y: number };
};

export type WorldJson = {
  version: number;
  entities: WorldEntity[];
};

const FACTION_COLORS: Record<number, string> = {
  1: "#3366cc",
  2: "#cc3333",
  3: "#33aa55",
};

const DEFAULT_COLOR = "#888888";
const RADIUS = 8;

export type Viewport = {
  minX: number;
  minY: number;
  scale: number;
  width: number;
  height: number;
};

export function computeViewport(
  entities: WorldEntity[],
  canvasWidth: number,
  canvasHeight: number,
): Viewport | null {
  const positioned = entities.filter((e) => e.position != null);
  if (positioned.length === 0) return null;

  let minX = Infinity;
  let maxX = -Infinity;
  let minY = Infinity;
  let maxY = -Infinity;
  for (const e of positioned) {
    const { x, y } = e.position!;
    minX = Math.min(minX, x);
    maxX = Math.max(maxX, x);
    minY = Math.min(minY, y);
    maxY = Math.max(maxY, y);
  }

  const padX = (maxX - minX) * 0.1 || 10;
  const padY = (maxY - minY) * 0.1 || 10;
  minX -= padX;
  maxX += padX;
  minY -= padY;
  maxY += padY;

  const worldW = maxX - minX;
  const worldH = maxY - minY;
  const scale = Math.min(
    (canvasWidth * 0.9) / worldW,
    (canvasHeight * 0.9) / worldH,
  );

  return { minX, minY, scale, width: canvasWidth, height: canvasHeight };
}

function worldToScreen(
  x: number,
  y: number,
  vp: Viewport,
): { sx: number; sy: number } {
  const sx = (x - vp.minX) * vp.scale + vp.width * 0.05;
  const sy = vp.height * 0.95 - (y - vp.minY) * vp.scale;
  return { sx, sy };
}

export function drawWorld(
  ctx: CanvasRenderingContext2D,
  world: WorldJson,
  vp: Viewport,
): void {
  ctx.clearRect(0, 0, vp.width, vp.height);
  for (const e of world.entities) {
    if (!e.position) continue;
    const { sx, sy } = worldToScreen(e.position.x, e.position.y, vp);
    const color =
      e.faction != null
        ? (FACTION_COLORS[e.faction] ?? DEFAULT_COLOR)
        : DEFAULT_COLOR;
    ctx.beginPath();
    ctx.arc(sx, sy, RADIUS, 0, Math.PI * 2);
    ctx.fillStyle = color;
    ctx.fill();
    if (e.entity_type) {
      ctx.fillStyle = "#111";
      ctx.font = "12px system-ui";
      ctx.fillText(e.entity_type, sx + RADIUS + 4, sy + 4);
    }
  }
}
```

- [ ] **Step 2: Commit**

```bash
git add wasm-bindings/web/src/render.ts
git commit -m "feat(web): add canvas renderer for world JSON"
```

---

### Task 4: Playwright config and failing e2e (RED)

**Files:**
- Create: `wasm-bindings/web/playwright.config.ts`
- Create: `wasm-bindings/web/e2e/demo.spec.ts`

- [ ] **Step 1: Install Playwright browser (one-time per machine)**

```bash
cd wasm-bindings/web && npx playwright install chromium
```

- [ ] **Step 2: Create `playwright.config.ts`**

```ts
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  timeout: 60_000,
  use: { headless: true },
  webServer: {
    command: "npm run preview",
    port: 4173,
    reuseExistingServer: false,
  },
});
```

- [ ] **Step 3: Create `e2e/demo.spec.ts`**

```ts
import { test, expect } from "@playwright/test";

type DemoWindow = {
  __openEntitiesDemo?: {
    getWorld: () => {
      entities: {
        entity_type?: string;
        position?: { x: number; y: number };
        move_target?: unknown;
      }[];
    };
    isPlaying: () => boolean;
  };
};

test("scout reaches move target after play", async ({ page }) => {
  await page.goto("/");
  await expect(page.locator("#world")).toBeVisible();

  await page.waitForFunction(() => {
    const w = (window as unknown as DemoWindow).__openEntitiesDemo;
    return w
      ?.getWorld()
      ?.entities?.some(
        (e) => e.entity_type === "scout" && e.position?.x === 50,
      );
  });

  await page.getByRole("button", { name: "Play" }).click();

  await page.waitForFunction(
    () => {
      const w = (window as unknown as DemoWindow).__openEntitiesDemo;
      const scout = w?.getWorld()?.entities?.find((e) => e.entity_type === "scout");
      if (!scout) return false;
      return (
        scout.move_target === undefined &&
        Math.abs((scout.position?.x ?? 999) - 20) < 0.1 &&
        Math.abs((scout.position?.y ?? 999) - 0) < 0.1
      );
    },
    { timeout: 30_000 },
  );
});
```

- [ ] **Step 4: Build bundler WASM + production bundle, run e2e (expect FAIL)**

```bash
wasm-pack build wasm-bindings --target bundler
cd wasm-bindings/web && npm run build && npx playwright test
```

Expected: FAIL — app not functional yet (`main.ts` missing or hook absent). Confirms RED before Task 5.

- [ ] **Step 5: Commit**

```bash
git add wasm-bindings/web/playwright.config.ts wasm-bindings/web/e2e/demo.spec.ts
git commit -m "test(web): add Playwright e2e for scout arrival (red)"
```

---

### Task 5: Main app — WASM init, spawn, Play/Pause (GREEN)

**Files:**
- Create: `wasm-bindings/web/src/vite-env.d.ts`
- Create: `wasm-bindings/web/src/main.ts`

- [ ] **Step 1: Add WASM import types**

`wasm-bindings/web/src/vite-env.d.ts`:

```ts
declare module "@wasm" {
  export default function init(): Promise<void>;
  export class Simulation {
    constructor();
    loadTemplatesYaml(yaml: string): void;
    spawnEntity(name: string, overrides: object): { index: number; generation: number };
    getWorldAsJson(): string;
    tick(dtMs: number): void;
  }
}
```

Note: Rust methods return `Result<_, JsValue>`; wasm-bindgen surfaces failures as **thrown** exceptions in JS (same as `demo/run.mjs`).

- [ ] **Step 2: Implement `main.ts`**

```ts
import init, { Simulation } from "@wasm";
import {
  computeViewport,
  drawWorld,
  type Viewport,
  type WorldJson,
} from "./render";

const SPAWN_ORDER = ["marker", "heavy_tank", "tank", "scout", "unit"] as const;

const statusEl = document.getElementById("status")!;
const playBtn = document.getElementById("play") as HTMLButtonElement;
const pauseBtn = document.getElementById("pause") as HTMLButtonElement;
const canvas = document.getElementById("world") as HTMLCanvasElement;
const ctx = canvas.getContext("2d")!;

let sim: Simulation;
let playing = false;
let rafId = 0;
let viewport: Viewport | null = null;

function setStatus(msg: string): void {
  statusEl.textContent = msg;
}

function scoutOverrides(): object {
  return {
    position: { x: 50.0, y: 25.0 },
    health: { current: 40, max: 100 },
  };
}

function paint(): void {
  const json = sim.getWorldAsJson();
  const world = JSON.parse(json) as WorldJson;
  if (!viewport) {
    viewport = computeViewport(world.entities, canvas.width, canvas.height);
  }
  if (viewport) drawWorld(ctx, world, viewport);
}

async function bootstrap(): Promise<void> {
  await init();
  sim = new Simulation();

  const res = await fetch("/fixtures/spawn_entity_templates.yaml");
  if (!res.ok) throw new Error(`fixture fetch failed: ${res.status}`);
  const yaml = await res.text();
  sim.loadTemplatesYaml(yaml);

  for (const name of SPAWN_ORDER) {
    const overrides = name === "scout" ? scoutOverrides() : {};
    sim.spawnEntity(name, overrides);
  }

  paint();
  playBtn.disabled = false;
  pauseBtn.disabled = false;
}

function play(): void {
  if (playing) return;
  playing = true;
  const loop = () => {
    if (!playing) return;
    try {
      sim.tick(16);
      paint();
    } catch (e) {
      console.error(e);
      pause();
      setStatus(String(e));
      return;
    }
    rafId = requestAnimationFrame(loop);
  };
  rafId = requestAnimationFrame(loop);
}

function pause(): void {
  playing = false;
  cancelAnimationFrame(rafId);
}

playBtn.addEventListener("click", () => {
  setStatus("");
  play();
});
pauseBtn.addEventListener("click", pause);

window.addEventListener("resize", () => {
  viewport = null;
  paint();
});

declare global {
  interface Window {
    __openEntitiesDemo?: {
      getWorld: () => WorldJson;
      isPlaying: () => boolean;
    };
  }
}

bootstrap()
  .then(() => {
    window.__openEntitiesDemo = {
      getWorld: () => JSON.parse(sim.getWorldAsJson()) as WorldJson,
      isPlaying: () => playing,
    };
  })
  .catch((e) => {
    setStatus(String(e));
    console.error(e);
    playBtn.disabled = true;
    pauseBtn.disabled = true;
  });
```

- [ ] **Step 3: Run e2e (expect PASS)**

```bash
wasm-pack build wasm-bindings --target bundler
cd wasm-bindings/web && npm run build && npx playwright test
```

Expected: 1 passed.

- [ ] **Step 4: Manual smoke (optional)**

```bash
make wasm-web-demo
```

Expected: browser opens; canvas shows entities; Play animates scout toward (20, 0); Pause freezes. Ctrl+C to stop dev server.

- [ ] **Step 5: Commit**

```bash
git add wasm-bindings/web/src/main.ts wasm-bindings/web/src/vite-env.d.ts
git commit -m "feat(web): WASM init, spawn scenario, Play/Pause loop"
```

---

### Task 6: Makefile and `.gitignore`

**Files:**
- Modify: `Makefile`
- Modify: `.gitignore`

- [ ] **Step 1: Append to `.gitignore`**

```
wasm-bindings/web/node_modules/
wasm-bindings/web/dist/
wasm-bindings/web/test-results/
wasm-bindings/web/playwright-report/
```

(Do not duplicate `wasm-bindings/pkg/` — already ignored.)

- [ ] **Step 2: Update `Makefile`**

Change line 1 `.PHONY` to include `wasm-web-build wasm-web-demo wasm-web-check`.

Replace line 21:

```makefile
wasm-check: wasm-demo wasm-test
```

with:

```makefile
wasm-web-build:
	@command -v wasm-pack >/dev/null 2>&1 || { echo "wasm-pack not found. Install with: cargo install wasm-pack"; exit 1; }
	wasm-pack build wasm-bindings --target bundler
	cd wasm-bindings/web && npm ci && npm run build

wasm-web-demo:
	@command -v wasm-pack >/dev/null 2>&1 || { echo "wasm-pack not found. Install with: cargo install wasm-pack"; exit 1; }
	@command -v node >/dev/null 2>&1 || { echo "node not found"; exit 1; }
	wasm-pack build wasm-bindings --target bundler
	cd wasm-bindings/web && npm install && npm run dev -- --open

wasm-web-check:
	@command -v wasm-pack >/dev/null 2>&1 || { echo "wasm-pack not found. Install with: cargo install wasm-pack"; exit 1; }
	wasm-pack build wasm-bindings --target bundler
	cd wasm-bindings/web && npm ci && npm run build
	cd wasm-bindings/web && npx playwright test

wasm-check: wasm-demo wasm-test wasm-web-check
```

`wasm-check` order: Node demo (`nodejs` target) → wasm tests → web e2e (`bundler` rebuild). Both targets write to `wasm-bindings/pkg/`; order is intentional.

- [ ] **Step 3: Verify Make targets**

```bash
make wasm-web-build
make wasm-web-check
```

Expected: `dist/` created; Playwright passes.

- [ ] **Step 4: Commit**

```bash
git add Makefile .gitignore
git commit -m "chore: add wasm-web-demo Make targets and web gitignore"
```

---

### Task 7: README

**Files:**
- Modify: `README.md` (after **WASM (Node)** section, before **Examples**)

- [ ] **Step 1: Insert Web demo subsection**

```markdown
## Web demo (browser)

Interactive canvas demo under [`wasm-bindings/web/`](wasm-bindings/web/). Uses its own fixture copy at `wasm-bindings/web/public/fixtures/` (may diverge from repo [`fixtures/`](fixtures/) over time).

**Prerequisites:** wasm32 target, `wasm-pack`, Node 20+, npm. For e2e (first time on a machine): `cd wasm-bindings/web && npx playwright install chromium`.

```bash
make wasm-web-demo
```

Headless check (included in full WASM gate):

```bash
make wasm-web-check
make wasm-check
```
```

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: add web demo section to README"
```

---

### Task 8: Full verification

- [ ] **Step 1: Run full WASM gate**

```bash
make wasm-check
```

Expected: `wasm-demo` ok (Node spawn + tick assertions in `run.mjs`), `wasm-test` ok (`#[wasm_bindgen_test]`), Playwright e2e ok.

- [ ] **Step 2: Confirm Node demo unchanged in isolation**

```bash
make wasm-demo
```

Expected: `wasm spawn demo ok` and `wasm tick demo ok`.

---

## Spec coverage checklist

| Spec requirement | Task |
|------------------|------|
| Vite + wasm plugins | Task 2 |
| One-time committed web fixture | Task 1 |
| Fixed spawn + scout overrides (match `run.mjs`) | Task 5 |
| Canvas circles / faction / labels / Y invert | Task 3 |
| Play/Pause + `tick(16)` rAF | Task 5 |
| `window.__openEntitiesDemo` | Task 5 |
| `make wasm-web-demo` (`--open`) | Task 6 |
| `make wasm-web-build` | Task 6 |
| `make wasm-web-check` | Task 4, 6 |
| `wasm-check` extended | Task 6 |
| Playwright port 4173 + preview | Task 4 |
| Error UI (`#status`, disable Play) | Task 5 |
| No Rust/Node demo changes | — |
| README | Task 7 |
| Optional `predev` pkg check | Task 2 |

## Success criteria (from spec)

- [ ] `make wasm-web-demo` opens browser; scout animates toward (20, 0)
- [ ] `make wasm-web-check` passes headless
- [ ] `make wasm-check` passes (nodejs demo + wasm tests + web e2e)
- [ ] `make wasm-demo` unchanged and passing in isolation
