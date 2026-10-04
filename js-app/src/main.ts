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
  frame,
  spawnRandomAt,
  spawnAt,
  stress,
  unboardUnits,
  world,
} from "./core/wasm";
import type { EntityId, EntitySnapshot, Pos } from "./core/types";
import type { WorldReader } from "./core/world-view";
import { renderEntities } from "./visualization/render";
import { setHtml, setText } from "./visualization/dom";
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
const stressBtn = document.getElementById(
  "stress-spawn"
) as HTMLButtonElement | null;
const stressCountEl = document.getElementById(
  "stress-count"
) as HTMLInputElement | null;
const perfEl = document.getElementById("perf");
const trainButtons = Array.from(
  document.querySelectorAll<HTMLButtonElement>("[data-train-type]")
);

const ENTITY_TYPES = [
  "mover",
  "another_mover",
  "truck",
  "static_obstacle",
] as const;

/** Most rows the Forces list shows; it lists the selection, not the army. */
const MAX_LISTED_ROWS = 200;
/**
 * Largest selection the transport buttons consider. Boarding is a squad-sized order, and reading
 * every unit of a 100 000-unit selection each frame to decide whether to offer it would cost more
 * than the frame.
 */
const TRANSPORT_SELECTION_LIMIT = 200;

type PixiApi = {
  drawWorld: (world: WorldReader) => void;
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
 * The HUD is synced after every frame, so a message that is not held somewhere flashes once and
 * is gone before it can be read.
 */
let transportNotice: string | null = null;
let lastFrameTime: number | null = null;
/** Seed of the next stress crowd; each press spawns a different, reproducible one. */
let stressSeed = 1;
/** Frame-rate readout: frames counted since `perfSince`. */
let perfFrames = 0;
let perfSince: number | null = null;

function getSelectedBaseFaction(selected: ReadonlySet<string>): number | null {
  if (selected.size !== 1) return null;
  const selectedId = [...selected][0];
  const selectedEntity = world.get(selectedId);
  if (!selectedEntity || selectedEntity.entityType !== "base") return null;
  return selectedEntity.faction;
}

function syncTrainButtonsVisibility(selected: ReadonlySet<string>): void {
  const baseFaction = getSelectedBaseFaction(selected);
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

function updateSelectionPanel(selected: ReadonlySet<string>): void {
  if (!selectionDetailEl) return;
  if (selected.size === 0) {
    setHtml(selectionDetailEl, `<p class="selection-empty">Nothing selected</p>`);
    return;
  }
  if (selected.size === 1) {
    const id = [...selected][0];
    const e = world.get(id);
    if (!e) {
      setHtml(selectionDetailEl, `<p class="selection-empty">Nothing selected</p>`);
      return;
    }
    const vel =
      e.velocity != null
        ? `(${e.velocity.vx.toFixed(2)}, ${e.velocity.vy.toFixed(2)})`
        : "—";
    const safeId = e.id.replace(/&/g, "&amp;").replace(/</g, "&lt;");
    const safeType = e.entityType.replace(/&/g, "&amp;").replace(/</g, "&lt;");
    // The `<dl>` is built once per selected entity and its cells patched after that, so the
    // player can select the id while the position underneath keeps ticking.
    let list = selectionDetailEl.querySelector<HTMLElement>("dl");
    if (!list || list.dataset.entityId !== e.id) {
      setHtml(
        selectionDetailEl,
        `<dl data-entity-id="${safeId.replace(/"/g, "&quot;")}">
      <dt>ID</dt><dd>${safeId}</dd>
      <dt>Type</dt><dd>${safeType}</dd>
      <dt>Position</dt><dd data-field="position"></dd>
      <dt>Velocity</dt><dd data-field="velocity"></dd>
    </dl>`
      );
      list = selectionDetailEl.querySelector<HTMLElement>("dl");
    }
    const position = list?.querySelector('[data-field="position"]');
    const velocity = list?.querySelector('[data-field="velocity"]');
    if (position) setText(position, `(${e.pos.x.toFixed(2)}, ${e.pos.y.toFixed(2)})`);
    if (velocity) setText(velocity, vel);
    return;
  }
  setHtml(
    selectionDetailEl,
    `<p class="selection-multi"><strong>${selected.size}</strong> units selected</p>`
  );
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
    setText(groupModeBtn, `Group orders: ${groupOrdersOn ? "on" : "off"}`);
    groupModeBtn.setAttribute("aria-pressed", String(groupOrdersOn));
  }
  if (groupStateEl) {
    setText(
      groupStateEl,
      activeGroup === null
        ? "No group"
        : `Group ${activeGroup.index}: ${groupMemberIds.length} units`
    );
  }
}

/** What the current selection offers the transport buttons. */
interface TransportSelection {
  /** The one selected vehicle, or null when the selection holds none or several. */
  vehicle: EntitySnapshot | null;
  /** Selected units on their own feet that could plausibly be cargo. */
  boarders: EntitySnapshot[];
  /** Passengers Unboard would let off. */
  riders: EntitySnapshot[];
}

function readTransportSelection(
  selected: ReadonlySet<string>
): TransportSelection {
  if (selected.size > TRANSPORT_SELECTION_LIMIT) {
    return { vehicle: null, boarders: [], riders: [] };
  }
  const chosen: EntitySnapshot[] = [];
  for (const id of selected) {
    const entity = world.get(id);
    if (entity) chosen.push(entity);
  }
  const vehicles = chosen.filter((entity) => entity.seats !== null);
  const vehicle = vehicles.length === 1 ? vehicles[0] : null;
  // A velocity is what makes a thing mobile in this engine, so it is also what makes it something
  // a truck carries. The core is deliberately more permissive — it asks a passenger only for a
  // position, and whether a self-propelled gun counts as freight is a question for a game, not for
  // an ECS — but offering to load the player's base is nonsense the demo should not put on screen.
  const boarders = chosen.filter(
    (entity) =>
      entity.seats === null && entity.aboard === null && entity.velocity !== null
  );
  // Picking the truck is enough to unload it; picking the riders themselves works too.
  const riders = vehicle
    ? world
        .ridersOf(vehicle.id)
        .map((key) => world.get(key))
        .filter((entity): entity is EntitySnapshot => entity !== null)
    : chosen.filter((entity) => entity.aboard !== null);
  return { vehicle, boarders, riders };
}

function describeTransport(vehicle: EntitySnapshot | null): string {
  if (vehicle === null || vehicle.seats === null) return "No vehicle selected";
  const taken = world.ridersOf(vehicle.id).length;
  const walking = world.walkersTo(vehicle.id).length;
  const onTheWay = walking > 0 ? `, ${walking} on the way` : "";
  return `${vehicle.entityType} ${vehicle.id}: ${taken}/${vehicle.seats} seats taken${onTheWay}`;
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
    const tooMany =
      selected.size > TRANSPORT_SELECTION_LIMIT
        ? `Select at most ${TRANSPORT_SELECTION_LIMIT} units to board or unboard`
        : null;
    setText(
      transportStateEl,
      transportNotice ?? tooMany ?? describeTransport(vehicle)
    );
  }
}

function syncSelectionUi(): void {
  if (!pixiApi) return;
  const ids = pixiApi.getSelectedIds();
  updateSelectionPanel(ids);
  syncTrainButtonsVisibility(ids);
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
    // Applies on the next tick; the frames show it.
    await stopSelected(ids);
  } catch (e) {
    console.error("stop order error:", e);
  }
}

/**
 * Sends the selected units to board the selected vehicle.
 *
 * An order, not a teleport: the core walks each unit over and puts it in once it is close enough,
 * following the truck if it drives off meanwhile. The HUD shows who is still on the way.
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
    await boardUnits(
      boarders.map((entity) => entity.id),
      vehicle.id
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
    await unboardUnits(riders.map((entity) => entity.id));
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
  const first = world.get(ids[0]);
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

/** The Forces list: the total, and a row per selected unit up to `MAX_LISTED_ROWS`. */
function renderForces(): void {
  if (!entityListEl) return;
  const selected = pixiApi?.getSelectedIds();
  const rows: EntitySnapshot[] = [];
  for (const id of selected ?? []) {
    if (rows.length >= MAX_LISTED_ROWS) break;
    const entity = world.get(id);
    if (entity) rows.push(entity);
  }
  renderEntities(rows, entityListEl, selected, world.size);
}

/** Draws `world` as it stands: canvas, Forces list, HUD. Runs once per animation frame. */
function render(): void {
  pixiApi?.drawWorld(world);
  renderForces();
  syncSelectionUi();
}

/** Frames per second, units and tick, refreshed twice a second: what a stress run is judged by. */
function updatePerf(timestamp: number): void {
  if (!perfEl) return;
  perfFrames++;
  if (perfSince === null) {
    perfSince = timestamp;
    return;
  }
  const elapsed = timestamp - perfSince;
  if (elapsed < 500) return;
  const fps = (perfFrames * 1000) / elapsed;
  setText(perfEl, `${fps.toFixed(0)} fps · ${world.size} units · tick ${world.tick}`);
  perfFrames = 0;
  perfSince = timestamp;
}

function getInitialLookAtEntityId(): string | null {
  const playerBase = world.find(
    (entity) => entity.entityType === "base" && entity.faction === 1
  );
  if (playerBase) return playerBase.id;

  const playerUnit = world.find((entity) => entity.faction === 1);
  return playerUnit?.id ?? null;
}

async function createEntity(typeName?: string): Promise<void> {
  if (!isWasmReady()) return;
  if (!pixiApi) return;
  const selectedIds = pixiApi.getSelectedIds();
  if (selectedIds.size !== 1) return;
  const selectedBaseId = [...selectedIds][0];
  const selectedBase = world.get(selectedBaseId);
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
    // Resolves once the frame that spawned it is in `world`; the next animation frame draws it.
    await spawnAt(type, x, y, selectedBase.faction);
  } catch (e) {
    console.error("spawnAt error:", e);
  }
}

function gameLoop(timestamp: number): void {
  // Real elapsed time goes to the worker's fixed-step clock; it decides how many ticks that is.
  const elapsedMs = lastFrameTime !== null ? timestamp - lastFrameTime : 0;
  lastFrameTime = timestamp;

  if (isWasmReady()) {
    frame(elapsedMs)
      .then(() => render())
      .catch((e) => console.error("frame error:", e));
  }
  updatePerf(timestamp);
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
                await orderGroupTo(activeGroup, world);
                pixi.showMoveTarget(world);
              } catch (e) {
                const message = e instanceof Error ? e.message : String(e);
                reportGroupProblem(`Group order failed: ${message}`);
              }
              return;
            }
            const ids = [...pixi.getSelectedIds()];
            if (ids.length === 0) return;
            await moveSelectedTo(ids, world);
            pixi.showMoveTarget(world);
          } catch (e) {
            console.error("move order error:", e);
          }
        },
      });
      pixiApi = pixi;
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
          void spawnRandomAt("base", 1).catch((e) =>
            console.error("spawnRandomAt(base) error:", e)
          );
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

    stressBtn?.addEventListener("click", () => {
      const count = Math.max(0, Math.floor(Number(stressCountEl?.value ?? 0)));
      if (count === 0) return;
      const seed = stressSeed++;
      void stress(count, seed).catch((e) => console.error("stress error:", e));
    });

    // Initial state read without advancing simulation time.
    await snapshot();
    render();
    const initialLookAtEntityId = getInitialLookAtEntityId();
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
