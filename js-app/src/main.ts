/**
 * Entry point: init WASM core in worker, wire UI and visualization.
 */
import {
  initWasm,
  isWasmReady,
  moveSelectedTo,
  tick,
  spawnRandomAt,
} from "./core/wasm";
import type { EntitySnapshot } from "./core/types";
import { renderEntities } from "./visualization/render";
import { initPixiCanvas } from "./visualization/pixi-canvas";

const statusEl = document.getElementById("status");
const selectionStatusEl = document.getElementById("selection-status");
const entityListEl = document.getElementById("entity-list");
const canvasContainer = document.getElementById("canvas-container");
const addEntityBtn = document.getElementById("add-entity");
const clearSelectionBtn = document.getElementById(
  "clear-selection"
) as HTMLButtonElement | null;
const entityTypeSelect = document.getElementById("entity-type") as HTMLSelectElement | null;

let updatePixiEntities: ((entities: EntitySnapshot[]) => void) | null = null;
let lastFrameTime: number | null = null;

function render(entities: EntitySnapshot[]): void {
  if (!entityListEl) return;
  renderEntities(entities, entityListEl);
  if (updatePixiEntities) updatePixiEntities(entities);
}

const ENTITY_TYPES = ["mover", "another_mover", "static_obstacle"] as const;

async function createEntity(typeName?: string): Promise<void> {
  if (!isWasmReady()) return;
  const type =
    typeName ??
    ENTITY_TYPES[Math.floor(Math.random() * ENTITY_TYPES.length)];
  try {
    const entities = await spawnRandomAt(type);
    render(entities);
  } catch (e) {
    console.error("spawnRandomAt error:", e);
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
      const pixi = await initPixiCanvas(canvasContainer, {
        onSelectionChange: (ids) => {
          if (selectionStatusEl) {
            selectionStatusEl.textContent =
              ids.size === 0
                ? "Selection: none"
                : `Selection: ${ids.size} unit(s)`;
          }
          if (clearSelectionBtn) {
            clearSelectionBtn.hidden = ids.size === 0;
            clearSelectionBtn.disabled = ids.size === 0;
          }
        },
        onMoveOrder: async (world) => {
          if (!isWasmReady()) return;
          const ids = [...pixi.getSelectedIds()];
          if (ids.length === 0) return;
          try {
            const entities = await moveSelectedTo(ids, world);
            render(entities);
          } catch (e) {
            console.error("moveSelectedTo error:", e);
          }
        },
      });
      updatePixiEntities = pixi.updateEntities;

      const clearSelection = (): void => {
        pixi.clearSelection();
      };
      window.addEventListener("keydown", (ev) => {
        if (ev.key === "Escape") clearSelection();
      });
      clearSelectionBtn?.addEventListener("click", clearSelection);
    }
    await createEntity();
    const entities = await tick(0);
    render(entities);
    requestAnimationFrame(gameLoop);
    addEntityBtn?.addEventListener("click", () => {
      const type =
        entityTypeSelect?.value && ENTITY_TYPES.includes(entityTypeSelect.value as (typeof ENTITY_TYPES)[number])
          ? entityTypeSelect.value
          : undefined;
      createEntity(type);
    });
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    statusEl.textContent = `Error loading WASM: ${message}`;
    console.error("WASM init error:", error);
  }
}

run();
