/**
 * Entry point: init WASM core in worker, wire UI and visualization.
 */
import "./styles.css";
import {
  initWasm,
  isWasmReady,
  moveSelectedTo,
  tick,
  spawnRandomAt,
} from "./core/wasm";
import type { EntitySnapshot, Pos } from "./core/types";
import { renderEntities } from "./visualization/render";
import { initPixiCanvas } from "./visualization/pixi-canvas";

const statusEl = document.getElementById("status");
const selectionDetailEl = document.getElementById("selection-detail");
const entityListEl = document.getElementById("entity-list");
const canvasContainer = document.getElementById("canvas-container");
const clearSelectionBtn = document.getElementById(
  "clear-selection"
) as HTMLButtonElement | null;

const ENTITY_TYPES = ["mover", "another_mover", "static_obstacle"] as const;

type PixiApi = {
  updateEntities: (entities: EntitySnapshot[]) => void;
  getSelectedIds: () => ReadonlySet<string>;
  clearSelection: () => void;
  setSelectedIds: (ids: readonly string[]) => void;
  showMoveTarget: (world: Pos) => void;
};

let pixiApi: PixiApi | null = null;
let lastEntities: EntitySnapshot[] = [];
let updatePixiEntities: ((entities: EntitySnapshot[]) => void) | null = null;
let lastFrameTime: number | null = null;

function setStatusReady(el: HTMLElement): void {
  el.textContent = "Core ready (worker)";
  el.classList.remove("rts-status--loading", "rts-status--error");
  el.classList.add("rts-status--ready");
}

function setStatusError(el: HTMLElement, message: string): void {
  el.textContent = message;
  el.classList.remove("rts-status--loading", "rts-status--ready");
  el.classList.add("rts-status--error");
}

function updateSelectionPanel(
  selected: ReadonlySet<string>,
  entities: EntitySnapshot[]
): void {
  if (!selectionDetailEl) return;
  if (selected.size === 0) {
    selectionDetailEl.innerHTML = `<p class="selection-empty">Nothing selected</p>`;
    return;
  }
  if (selected.size === 1) {
    const id = [...selected][0];
    const e = entities.find((x) => x.id === id);
    if (!e) {
      selectionDetailEl.innerHTML = `<p class="selection-empty">Nothing selected</p>`;
      return;
    }
    const vel =
      e.velocity != null
        ? `(${e.velocity.vx.toFixed(2)}, ${e.velocity.vy.toFixed(2)})`
        : "—";
    const safeId = e.id.replace(/&/g, "&amp;").replace(/</g, "&lt;");
    const safeType = e.entityType.replace(/&/g, "&amp;").replace(/</g, "&lt;");
    selectionDetailEl.innerHTML = `<dl>
      <dt>ID</dt><dd>${safeId}</dd>
      <dt>Type</dt><dd>${safeType}</dd>
      <dt>Position</dt><dd>(${e.pos.x.toFixed(2)}, ${e.pos.y.toFixed(2)})</dd>
      <dt>Velocity</dt><dd>${vel}</dd>
    </dl>`;
    return;
  }
  selectionDetailEl.innerHTML = `<p class="selection-multi"><strong>${selected.size}</strong> units selected</p>`;
}

function syncSelectionUi(): void {
  if (!pixiApi) return;
  updateSelectionPanel(pixiApi.getSelectedIds(), lastEntities);
  const ids = pixiApi.getSelectedIds();
  if (clearSelectionBtn) {
    clearSelectionBtn.hidden = ids.size === 0;
    clearSelectionBtn.disabled = ids.size === 0;
  }
}

function render(entities: EntitySnapshot[]): void {
  lastEntities = entities;
  if (!entityListEl) return;
  renderEntities(entities, entityListEl);
  if (updatePixiEntities) updatePixiEntities(entities);
  syncSelectionUi();
}

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
    setStatusReady(statusEl);

    if (canvasContainer) {
      const pixi = await initPixiCanvas(canvasContainer, {
        onSelectionChange: () => {
          syncSelectionUi();
        },
        onMoveOrder: async (world) => {
          if (!isWasmReady()) return;
          const ids = [...pixi.getSelectedIds()];
          if (ids.length === 0) return;
          try {
            const entities = await moveSelectedTo(ids, world);
            pixi.showMoveTarget(world);
            render(entities);
          } catch (e) {
            console.error("moveSelectedTo error:", e);
          }
        },
      });
      pixiApi = pixi;
      updatePixiEntities = pixi.updateEntities;

      const clearSelection = (): void => {
        pixi.clearSelection();
      };
      window.addEventListener("keydown", (ev) => {
        if (ev.key === "Escape") clearSelection();
      });
      clearSelectionBtn?.addEventListener("click", clearSelection);
    }

    entityListEl?.addEventListener("click", (ev) => {
      const t = (ev.target as HTMLElement).closest("[data-entity-id]");
      if (!t || !pixiApi) return;
      const id = t.getAttribute("data-entity-id");
      if (id) pixiApi.setSelectedIds([id]);
    });

    for (const btn of document.querySelectorAll<HTMLButtonElement>(
      "[data-train-type]"
    )) {
      const trainType = btn.dataset.trainType;
      btn.addEventListener("click", () => {
        if (
          trainType &&
          (ENTITY_TYPES as readonly string[]).includes(trainType)
        ) {
          void createEntity(trainType as (typeof ENTITY_TYPES)[number]);
        }
      });
    }

    await createEntity();
    const entities = await tick(0);
    render(entities);
    requestAnimationFrame(gameLoop);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (statusEl) setStatusError(statusEl, `Error loading WASM: ${message}`);
    console.error("WASM init error:", error);
  }
}

run();
