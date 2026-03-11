/**
 * Entry point: init WASM core, wire UI and visualization.
 */
import { initWasm, isWasmReady, JsWorld } from "./core/wasm";
import type { EntitySnapshot } from "./core/types";
import { renderEntities } from "./visualization/render";
import { initPixiCanvas } from "./visualization/pixi-canvas";

let world: JsWorld | null = null;
const statusEl = document.getElementById("status");
const entityListEl = document.getElementById("entity-list");
const canvasContainer = document.getElementById("canvas-container");
const addEntityBtn = document.getElementById("add-entity");

let updatePixiEntities: ((entities: EntitySnapshot[]) => void) | null = null;
let lastFrameTime: number | null = null;

function createEntity(
  x: number = Math.random() * 100,
  y: number = Math.random() * 100
): void {
  if (!isWasmReady() || !world) return;
  world.spawn(
    x,
    y,
    Math.random() * 2 - 1,
    Math.random() * 2 - 1
  );
  syncAndRender();
}

function syncAndRender(): void {
  if (!world || !entityListEl) return;
  const raw = world.get_entities();
  const entities: EntitySnapshot[] = Array.from(raw).map(
    (e: { x: number; y: number; vx: number; vy: number }, i: number) => ({
      id: i,
      x: e.x,
      y: e.y,
      vx: e.vx,
      vy: e.vy,
    })
  );
  renderEntities(entities, entityListEl);
  if (updatePixiEntities) updatePixiEntities(entities);
}

function gameLoop(timestamp: number): void {
  const dtSec =
    lastFrameTime !== null
      ? Math.min((timestamp - lastFrameTime) / 1000, 0.1)
      : 1 / 60;
  lastFrameTime = timestamp;
  if (world) {
    world.tick(dtSec);
    syncAndRender();
  }
  requestAnimationFrame(gameLoop);
}

async function run(): Promise<void> {
  if (!statusEl) return;
  try {
    await initWasm();
    world = new JsWorld();
    statusEl.textContent = "WASM loaded successfully!";
    if (canvasContainer) {
      const pixi = await initPixiCanvas(canvasContainer);
      updatePixiEntities = pixi.updateEntities;
    }
    createEntity();
    syncAndRender();
    requestAnimationFrame(gameLoop);
    addEntityBtn?.addEventListener("click", () => createEntity());
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    statusEl.textContent = `Error loading WASM: ${message}`;
    console.error("WASM init error:", error);
  }
}

run();
