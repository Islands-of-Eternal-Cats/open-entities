/**
 * Entry point: init WASM core, wire UI and visualization.
 */
import {
  initWasm,
  isWasmReady,
  JsPosition,
  JsVelocity,
  move_position,
} from "./core/wasm";
import type { GameEntity } from "./core/types";
import { renderEntities } from "./visualization/render";
import { initPixiCanvas } from "./visualization/pixi-canvas";

const entities: GameEntity[] = [];
const statusEl = document.getElementById("status");
const entityListEl = document.getElementById("entity-list");
const canvasContainer = document.getElementById("canvas-container");
const addEntityBtn = document.getElementById("add-entity");
const moveAllBtn = document.getElementById("move-all");

let updatePixiEntities: ((entities: GameEntity[]) => void) | null = null;

function createEntity(
  x: number = Math.random() * 100,
  y: number = Math.random() * 100
): GameEntity | void {
  if (!isWasmReady()) return;
  const velocity = new JsVelocity(
    Math.random() * 2 - 1,
    Math.random() * 2 - 1
  );
  const position = new JsPosition(x, y);
  const entity: GameEntity = {
    id: entities.length,
    position,
    velocity,
  };
  entities.push(entity);
  if (entityListEl) renderEntities(entities, entityListEl);
  if (updatePixiEntities) updatePixiEntities(entities);
  return entity;
}

function moveAllEntities(): void {
  if (!isWasmReady() || !entityListEl) return;
  for (const entity of entities) {
    entity.position = move_position(entity.position, entity.velocity);
  }
  if (entityListEl) renderEntities(entities, entityListEl);
  if (updatePixiEntities) updatePixiEntities(entities);
}

async function run(): Promise<void> {
  if (!statusEl) return;
  try {
    await initWasm();
    statusEl.textContent = "WASM loaded successfully!";
    if (canvasContainer) {
      const pixi = await initPixiCanvas(canvasContainer);
      updatePixiEntities = pixi.updateEntities;
    }
    if (entityListEl) renderEntities(entities, entityListEl);
    createEntity();
    if (updatePixiEntities) updatePixiEntities(entities);
    setInterval(moveAllEntities, 1000);
    addEntityBtn?.addEventListener("click", () => createEntity());
    moveAllBtn?.addEventListener("click", () => moveAllEntities());
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    statusEl.textContent = `Error loading WASM: ${message}`;
    console.error("WASM init error:", error);
  }
}

run();
