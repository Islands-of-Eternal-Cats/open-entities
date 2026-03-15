/**
 * Entry point: init WASM core in worker, wire UI and visualization.
 */
import { initWasm, isWasmReady, tick, spawn } from "./core/wasm";
import type { EntitySnapshot } from "./core/types";
import { renderEntities } from "./visualization/render";
import { initPixiCanvas } from "./visualization/pixi-canvas";

const statusEl = document.getElementById("status");
const entityListEl = document.getElementById("entity-list");
const canvasContainer = document.getElementById("canvas-container");
const addEntityBtn = document.getElementById("add-entity");

let updatePixiEntities: ((entities: EntitySnapshot[]) => void) | null = null;
let lastFrameTime: number | null = null;

function render(entities: EntitySnapshot[]): void {
  if (!entityListEl) return;
  renderEntities(entities, entityListEl);
  if (updatePixiEntities) updatePixiEntities(entities);
}

async function createEntity(
  x: number = Math.random() * 100,
  y: number = Math.random() * 100
): Promise<void> {
  if (!isWasmReady()) return;
  try {
    const entities = await spawn(
      { x, y },
      { vx: Math.random() * 2 - 1, vy: Math.random() * 2 - 1 }
    );
    render(entities);
  } catch (e) {
    console.error("spawn error:", e);
  }
}

function gameLoop(timestamp: number): void {
  const dtSec =
    lastFrameTime !== null
      ? Math.min((timestamp - lastFrameTime) / 1000, 0.1)
      : 1 / 60;
  lastFrameTime = timestamp;

  if (isWasmReady()) {
    tick(dtSec)
      .then((entities) => render(entities))
      .catch((e) => console.error("tick error:", e));
  }
  requestAnimationFrame(gameLoop);
}

async function run(): Promise<void> {
  if (!statusEl) return;
  try {
    await initWasm();
    statusEl.textContent = "WASM loaded (worker)!";
    if (canvasContainer) {
      const pixi = await initPixiCanvas(canvasContainer);
      updatePixiEntities = pixi.updateEntities;
    }
    await createEntity();
    const entities = await tick(0);
    render(entities);
    requestAnimationFrame(gameLoop);
    addEntityBtn?.addEventListener("click", () => createEntity());
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    statusEl.textContent = `Error loading WASM: ${message}`;
    console.error("WASM init error:", error);
  }
}

run();
