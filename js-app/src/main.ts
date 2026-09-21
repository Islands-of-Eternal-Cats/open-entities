/**
 * Entry point: init WASM core in worker, wire UI and visualization.
 */
import "./styles.css";
import {
  boardUnits,
  coreBuildInfo,
  createGroupWith,
  initWasm,
  isWasmReady,
  moveSelectedTo,
  orderGroupTo,
  snapshot,
  stopSelected,
  tick,
  spawnRandomAt,
  spawnAt,
  unboardUnits,
} from "./core/wasm";
import type { EntityId, EntitySnapshot, Pos } from "./core/types";
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
const formGroupBtn = document.getElementById(
  "form-group"
) as HTMLButtonElement | null;
const groupModeBtn = document.getElementById(
  "group-mode"
) as HTMLButtonElement | null;
const groupStateEl = document.getElementById("group-state");
const boardBtn = document.getElementById(
  "board-units"
) as HTMLButtonElement | null;
const unboardBtn = document.getElementById(
  "unboard-units"
) as HTMLButtonElement | null;
const transportStateEl = document.getElementById("transport-state");
const stopOrderBtn = document.getElementById(
  "stop-order"
) as HTMLButtonElement | null;
const trainButtons = Array.from(
  document.querySelectorAll<HTMLButtonElement>("[data-train-type]")
);

const ENTITY_TYPES = [
  "mover",
  "another_mover",
  "truck",
  "static_obstacle",
] as const;

type PixiApi = {
  updateEntities: (entities: EntitySnapshot[]) => void;
  getSelectedIds: () => ReadonlySet<string>;
  clearSelection: () => void;
  setSelectedIds: (ids: readonly string[]) => void;
  showMoveTarget: (world: Pos) => void;
  LookAt: (entityId: string) => boolean;
};

let pixiApi: PixiApi | null = null;
/** The group the demo commands, once the player has formed one. */
let activeGroup: EntityId | null = null;
/** Ids that went into that group, for the HUD line. */
let groupMemberIds: string[] = [];
/**
 * When on, a click on empty ground is a *group* order instead of a personal one.
 *
 * The difference is the point of the demo: a personal order outranks group steering, so a unit
 * ordered by hand keeps walking its own way while the rest of the group turns.
 */
let groupOrdersOn = false;
/**
 * Why the last board or unboard did nothing, shown until the next attempt.
 *
 * The HUD is rewritten every frame, so a message that is not held somewhere flashes once and is
 * gone before it can be read.
 */
let transportNotice: string | null = null;
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
  // The build fingerprint is the point: a core that did not rebuild keeps the same one, which is
  // the difference between "the fix does not work" and "the fix is not in what you are running".
  const build = coreBuildInfo();
  const stamp = build
    ? ` · core ${build.id} · ${(build.bytes / 1024 / 1024).toFixed(2)} MB`
    : "";
  el.textContent = `Core ready (worker)${stamp}`;
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

function syncGroupUi(selectionSize: number): void {
  if (formGroupBtn) {
    formGroupBtn.hidden = selectionSize === 0;
    formGroupBtn.disabled = selectionSize === 0;
  }
  if (groupModeBtn) {
    groupModeBtn.hidden = activeGroup === null;
    groupModeBtn.disabled = activeGroup === null;
    groupModeBtn.textContent = `Group orders: ${groupOrdersOn ? "on" : "off"}`;
    groupModeBtn.setAttribute("aria-pressed", String(groupOrdersOn));
  }
  if (groupStateEl) {
    groupStateEl.textContent =
      activeGroup === null
        ? "No group"
        : `Group ${activeGroup.index}: ${groupMemberIds.length} units`;
  }
}

/** What the current selection offers the transport buttons. */
interface TransportSelection {
  /** The one selected vehicle, or null when the selection holds none or several. */
  vehicle: EntitySnapshot | null;
  /** Selected units on their own feet, which is who Board would try to load. */
  boarders: EntitySnapshot[];
  /** Passengers Unboard would let off. */
  riders: EntitySnapshot[];
}

function readTransportSelection(
  selected: ReadonlySet<string>
): TransportSelection {
  const chosen = lastEntities.filter((entity) => selected.has(entity.id));
  const vehicles = chosen.filter((entity) => entity.seats !== null);
  const vehicle = vehicles.length === 1 ? vehicles[0] : null;
  const boarders = chosen.filter(
    (entity) => entity.seats === null && entity.aboard === null
  );
  // Picking the truck is enough to unload it; picking the riders themselves works too.
  const riders = vehicle
    ? lastEntities.filter((entity) => entity.aboard === vehicle.id)
    : chosen.filter((entity) => entity.aboard !== null);
  return { vehicle, boarders, riders };
}

function describeTransport(vehicle: EntitySnapshot | null): string {
  if (vehicle === null || vehicle.seats === null) return "No vehicle selected";
  const taken = lastEntities.filter(
    (entity) => entity.aboard === vehicle.id
  ).length;
  return `${vehicle.entityType} ${vehicle.id}: ${taken}/${vehicle.seats} seats taken`;
}

function syncTransportUi(selected: ReadonlySet<string>): void {
  const { vehicle, boarders, riders } = readTransportSelection(selected);
  const canBoard = vehicle !== null && boarders.length > 0;
  if (boardBtn) {
    boardBtn.hidden = !canBoard;
    boardBtn.disabled = !canBoard;
  }
  if (unboardBtn) {
    unboardBtn.hidden = riders.length === 0;
    unboardBtn.disabled = riders.length === 0;
  }
  if (transportStateEl) {
    transportStateEl.textContent = transportNotice ?? describeTransport(vehicle);
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
  if (stopOrderBtn) {
    stopOrderBtn.hidden = ids.size === 0;
    stopOrderBtn.disabled = ids.size === 0;
  }
  syncGroupUi(ids.size);
  syncTransportUi(ids);
  syncEntityListSelectionHighlight();
}

/** Halts the selection where it stands — mainly so a truck stops before anyone steps off. */
async function stopSelection(): Promise<void> {
  if (!isWasmReady() || !pixiApi) return;
  const ids = [...pixiApi.getSelectedIds()];
  if (ids.length === 0) return;
  try {
    render(await stopSelected(ids));
  } catch (e) {
    console.error("stop order error:", e);
  }
}

/**
 * Loads the selected units onto the selected vehicle.
 *
 * Boarding is not a move order — the core refuses anyone standing further off than its boarding
 * range — so a unit across the map has to be walked over first. That refusal is the interesting
 * half of the feature, which is why it is shown rather than logged.
 */
async function boardSelection(): Promise<void> {
  if (!isWasmReady() || !pixiApi) return;
  const { vehicle, boarders } = readTransportSelection(pixiApi.getSelectedIds());
  transportNotice = null;
  if (vehicle === null) {
    transportNotice = "Select exactly one vehicle along with the units to load";
    syncSelectionUi();
    return;
  }
  if (boarders.length === 0) {
    transportNotice = "Select the units to put aboard as well";
    syncSelectionUi();
    return;
  }
  try {
    render(
      await boardUnits(
        boarders.map((entity) => entity.id),
        vehicle.id
      )
    );
  } catch (e) {
    transportNotice = `Could not board: ${e instanceof Error ? e.message : String(e)}`;
    syncSelectionUi();
  }
}

/** Lets the selected passengers off — or, with the vehicle selected, everyone it carries. */
async function unboardSelection(): Promise<void> {
  if (!isWasmReady() || !pixiApi) return;
  const { riders } = readTransportSelection(pixiApi.getSelectedIds());
  transportNotice = null;
  if (riders.length === 0) {
    transportNotice = "Nobody selected is aboard anything";
    syncSelectionUi();
    return;
  }
  try {
    render(await unboardUnits(riders.map((entity) => entity.id)));
  } catch (e) {
    transportNotice = `Could not unboard: ${e instanceof Error ? e.message : String(e)}`;
    syncSelectionUi();
  }
}

/**
 * Forms a group out of whatever is selected.
 *
 * The faction comes from the first selected unit; a group commands one faction, so units of
 * another are refused by the core and the call fails loudly rather than half-forming a group.
 */
/** Says why nothing happened, where the player is already looking. */
function reportGroupProblem(message: string): void {
  if (groupStateEl) groupStateEl.textContent = message;
  console.warn(message);
}

async function formGroupFromSelection(): Promise<void> {
  if (!isWasmReady() || !pixiApi) return;
  const ids = [...pixiApi.getSelectedIds()];
  if (ids.length === 0) {
    reportGroupProblem("Select some units first");
    return;
  }
  const first = lastEntities.find((entity) => entity.id === ids[0]);
  if (!first) {
    reportGroupProblem("The selected unit is gone");
    return;
  }
  if (first.faction === null) {
    reportGroupProblem(
      `${first.entityType} has no faction, so it cannot lead a group`
    );
    return;
  }

  try {
    activeGroup = await createGroupWith(first.faction, ids);
    groupMemberIds = ids;
    groupOrdersOn = true;
    syncSelectionUi();
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e);
    reportGroupProblem(`Could not form a group: ${message}`);
  }
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
          try {
            if (groupOrdersOn && activeGroup !== null) {
              try {
                const entities = await orderGroupTo(activeGroup, world);
                pixi.showMoveTarget(world);
                render(entities);
              } catch (e) {
                const message = e instanceof Error ? e.message : String(e);
                reportGroupProblem(`Group order failed: ${message}`);
              }
              return;
            }
            const ids = [...pixi.getSelectedIds()];
            if (ids.length === 0) return;
            const entities = await moveSelectedTo(ids, world);
            pixi.showMoveTarget(world);
            render(entities);
          } catch (e) {
            console.error("move order error:", e);
          }
        },
      });
      pixiApi = pixi;
      updatePixiEntities = pixi.updateEntities;
      // The canvas can report a selection while initPixiCanvas is still being awaited — at that
      // point pixiApi is null and syncSelectionUi bails out, leaving the HUD hidden and its
      // clear button disabled. Sync once now that the api is in hand.
      syncSelectionUi();

      const clearSelection = (): void => {
        pixi.clearSelection();
      };
      window.addEventListener("keydown", (ev) => {
        if (ev.key === "Escape") clearSelection();
        if (ev.key === "g" || ev.key === "G") void formGroupFromSelection();
        if (ev.key === "s" || ev.key === "S") void stopSelection();
        if (ev.key === "b" || ev.key === "B") void boardSelection();
        if (ev.key === "u" || ev.key === "U") void unboardSelection();
      });
      clearSelectionBtn?.addEventListener("click", clearSelection);
      stopOrderBtn?.addEventListener("click", () => {
        void stopSelection();
      });
      formGroupBtn?.addEventListener("click", () => {
        void formGroupFromSelection();
      });
      boardBtn?.addEventListener("click", () => {
        void boardSelection();
      });
      unboardBtn?.addEventListener("click", () => {
        void unboardSelection();
      });
      groupModeBtn?.addEventListener("click", () => {
        groupOrdersOn = !groupOrdersOn;
        syncSelectionUi();
      });
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
