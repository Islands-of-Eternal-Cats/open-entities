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
let lastFrameTime: number | null = null;

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

/**
 * Advances all entities by dt seconds (velocity * dt).
 * @param dt - time step in seconds (e.g. from requestAnimationFrame delta)
 */
function moveAllEntities(dt: number): void {
  if (!isWasmReady()) return;
  for (const entity of entities) {
    const vel = entity.velocity;
    const scaledVel = new JsVelocity(vel.vx() * dt, vel.vy() * dt);
    entity.position = move_position(entity.position, scaledVel);
  }
  if (entityListEl) renderEntities(entities, entityListEl);
  if (updatePixiEntities) updatePixiEntities(entities);
}

function gameLoop(timestamp: number): void {
  const dtSec =
    lastFrameTime !== null
      ? Math.min((timestamp - lastFrameTime) / 1000, 0.1)
      : 1 / 60;
  lastFrameTime = timestamp;
  moveAllEntities(dtSec);
  requestAnimationFrame(gameLoop);
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
    requestAnimationFrame(gameLoop);
    addEntityBtn?.addEventListener("click", () => createEntity());
    moveAllBtn?.addEventListener("click", () => moveAllEntities(1 / 60));
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    statusEl.textContent = `Error loading WASM: ${message}`;
    console.error("WASM init error:", error);
  }
}

run();
