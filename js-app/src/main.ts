/**
 * Entry point: init WASM core in worker, wire UI and visualization.
 */
import "./styles.css";
import {
  initWasm,
  isWasmReady,
  moveSelectedTo,
  snapshot,
  tick,
  spawnRandomAt,
  spawnAt,
} from "./core/wasm";
import type { EntitySnapshot, Pos } from "./core/types";
import { renderEntities } from "./visualization/render";
import { initPixiCanvas } from "./visualization/pixi-canvas";
import { WORLD_SIZE } from "./visualization/coords";

const statusEl = document.getElementById("status");
const selectionDetailEl = document.getElementById("selection-detail");
const entityListEl = document.getElementById("entity-list");
const canvasContainer = document.getElementById("canvas-container");
const clearSelectionBtn = document.getElementById(
  "clear-selection"
) as HTMLButtonElement | null;
const trainButtons = Array.from(
  document.querySelectorAll<HTMLButtonElement>("[data-train-type]")
);

const ENTITY_TYPES = ["mover", "another_mover", "static_obstacle"] as const;

type PixiApi = {
  updateEntities: (entities: EntitySnapshot[]) => void;
  getSelectedIds: () => ReadonlySet<string>;
  clearSelection: () => void;
  setSelectedIds: (ids: readonly string[]) => void;
  showMoveTarget: (world: Pos) => void;
  LookAt: (entityId: string) => boolean;
};

let pixiApi: PixiApi | null = null;
let lastEntities: EntitySnapshot[] = [];
let updatePixiEntities: ((entities: EntitySnapshot[]) => void) | null = null;
let lastFrameTime: number | null = null;

function upsertSpawnedEntity(spawned: EntitySnapshot): void {
  const existingIndex = lastEntities.findIndex((entity) => entity.id === spawned.id);
  if (existingIndex === -1) {
    render([...lastEntities, spawned]);
    return;
  }
  const next = [...lastEntities];
  next[existingIndex] = spawned;
  render(next);
}

function getSelectedBaseFaction(
  selected: ReadonlySet<string>,
  entities: EntitySnapshot[]
): number | null {
  if (selected.size !== 1) return null;
  const selectedId = [...selected][0];
  const selectedEntity = entities.find((entity) => entity.id === selectedId);
  if (!selectedEntity || selectedEntity.entityType !== "base") return null;
  return selectedEntity.faction;
}

function syncTrainButtonsVisibility(
  selected: ReadonlySet<string>,
  entities: EntitySnapshot[]
): void {
  const baseFaction = getSelectedBaseFaction(selected, entities);
  for (const btn of trainButtons) {
    const trainType = btn.dataset.trainType;
    if (!trainType || trainType === "base") continue;
    const canTrainFromSelectedBase = baseFaction !== null;
    btn.hidden = !canTrainFromSelectedBase;
    btn.disabled = !canTrainFromSelectedBase;
    if (canTrainFromSelectedBase) {
      btn.dataset.trainFaction = String(baseFaction);
    } else {
      delete btn.dataset.trainFaction;
    }
  }
}

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

function syncEntityListSelectionHighlight(): void {
  if (!entityListEl || !pixiApi) return;
  const sel = pixiApi.getSelectedIds();
  for (const btn of entityListEl.querySelectorAll<HTMLButtonElement>(
    ".entity-row"
  )) {
    const id = btn.getAttribute("data-entity-id");
    const selected = id != null && sel.has(id);
    btn.classList.toggle("entity-row--selected", selected);
    if (selected) btn.setAttribute("aria-current", "true");
    else btn.removeAttribute("aria-current");
  }
}

function syncSelectionUi(): void {
  if (!pixiApi) return;
  const ids = pixiApi.getSelectedIds();
  updateSelectionPanel(ids, lastEntities);
  syncTrainButtonsVisibility(ids, lastEntities);
  if (clearSelectionBtn) {
    clearSelectionBtn.hidden = ids.size === 0;
    clearSelectionBtn.disabled = ids.size === 0;
  }
  syncEntityListSelectionHighlight();
}

function render(entities: EntitySnapshot[]): void {
  lastEntities = entities;
  if (!entityListEl) return;
  renderEntities(entities, entityListEl, pixiApi?.getSelectedIds());
  if (updatePixiEntities) updatePixiEntities(entities);
  syncSelectionUi();
}

function getInitialLookAtEntityId(entities: EntitySnapshot[]): string | null {
  const playerBase = entities.find(
    (entity) => entity.entityType === "base" && entity.faction === 1
  );
  if (playerBase) return playerBase.id;

  const playerUnit = entities.find((entity) => entity.faction === 1);
  return playerUnit?.id ?? null;
}

async function createEntity(typeName?: string): Promise<void> {
  if (!isWasmReady()) return;
  if (!pixiApi) return;
  const selectedIds = pixiApi.getSelectedIds();
  if (selectedIds.size !== 1) return;
  const selectedBaseId = [...selectedIds][0];
  const selectedBase = lastEntities.find((entity) => entity.id === selectedBaseId);
  if (!selectedBase || selectedBase.entityType !== "base") return;
  if (selectedBase.faction === null) return;
  const type =
    typeName ??
    ENTITY_TYPES[Math.floor(Math.random() * ENTITY_TYPES.length)];
  try {
    const spawnRadius = 20;
    const angle = Math.random() * Math.PI * 2;
    const distance = Math.random() * spawnRadius;
    const x = Math.max(
      0,
      Math.min(WORLD_SIZE, selectedBase.pos.x + Math.cos(angle) * distance)
    );
    const y = Math.max(
      0,
      Math.min(WORLD_SIZE, selectedBase.pos.y + Math.sin(angle) * distance)
    );
    const spawned = await spawnAt(type, x, y, selectedBase.faction);
    upsertSpawnedEntity(spawned);
  } catch (e) {
    console.error("spawnAt error:", e);
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

    for (const btn of trainButtons) {
      const trainType = btn.dataset.trainType;
      btn.addEventListener("click", () => {
        if (trainType === "base") {
          void spawnRandomAt("base", 1)
            .then((spawned) => upsertSpawnedEntity(spawned))
            .catch((e) => console.error("spawnRandomAt(base) error:", e));
          return;
        }
        if (
          trainType &&
          (ENTITY_TYPES as readonly string[]).includes(trainType)
        ) {
          void createEntity(trainType as (typeof ENTITY_TYPES)[number]);
        }
      });
    }

    // Initial state read without advancing simulation time.
    const entities = await snapshot();
    render(entities);
    const initialLookAtEntityId = getInitialLookAtEntityId(entities);
    if (initialLookAtEntityId && pixiApi) {
      pixiApi.LookAt(initialLookAtEntityId);
    }
    requestAnimationFrame(gameLoop);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (statusEl) setStatusError(statusEl, `Error loading WASM: ${message}`);
    console.error("WASM init error:", error);
  }
}

run();
