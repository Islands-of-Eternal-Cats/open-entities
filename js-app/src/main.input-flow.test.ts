import { beforeEach, describe, expect, it, vi } from "vitest";
import type { EntitySnapshot, Pos } from "./core/types";

const state = vi.hoisted(() => {
  const selectedIds = new Set<string>(["u1"]);
  /** A unit on foot and a truck beside it: what the transport buttons read. */
  const world: EntitySnapshot[] = [
    {
      id: "u1",
      entityType: "mover",
      pos: { x: 0, y: 0 },
      velocity: null,
      faction: 1,
      seats: null,
      aboard: null,
      moveTarget: null,
    },
    {
      id: "v1",
      entityType: "truck",
      pos: { x: 1, y: 0 },
      velocity: null,
      faction: 1,
      seats: 4,
      aboard: null,
      moveTarget: null,
    },
  ];
  return {
    selectedIds,
    world,
    onMoveOrder: null as ((world: Pos) => void | Promise<void>) | null,
    clearSelection: vi.fn(() => {
      selectedIds.clear();
    }),
    showMoveTarget: vi.fn(),
    createGroupWith: vi.fn(async () => ({ index: 7, generation: 0 })),
    orderGroupTo: vi.fn(async () => [] as EntitySnapshot[]),
    boardUnits: vi.fn(async () => [] as EntitySnapshot[]),
    stopSelected: vi.fn(async () => [] as EntitySnapshot[]),
    unboardUnits: vi.fn(async () => [] as EntitySnapshot[]),
    moveSelectedTo: vi.fn(async () => [
      {
        id: "u1",
        entityType: "mover",
        pos: { x: 12, y: 24 },
        velocity: null,
        faction: null,
        seats: null,
        aboard: null,
        moveTarget: null,
      } satisfies EntitySnapshot,
    ]),
    renderEntities: vi.fn(),
  };
});

vi.mock("./core/wasm", () => ({
  initWasm: vi.fn(async () => {}),
  isWasmReady: vi.fn(() => true),
  coreBuildInfo: vi.fn(() => ({ id: "deadbeef", bytes: 1024 })),
  moveSelectedTo: state.moveSelectedTo,
  createGroupWith: state.createGroupWith,
  orderGroupTo: state.orderGroupTo,
  boardUnits: state.boardUnits,
  stopSelected: state.stopSelected,
  unboardUnits: state.unboardUnits,
  tick: vi.fn(async () => [] as EntitySnapshot[]),
  // main.ts imports these two as well; leaving them out made run() throw on the first
  // `await snapshot()` and swallow the rest of the wiring.
  snapshot: vi.fn(async () => state.world),
  spawnRandomAt: vi.fn(async () => [] as EntitySnapshot[]),
  spawnAt: vi.fn(async () => [] as EntitySnapshot[]),
}));

vi.mock("./visualization/render", () => ({
  renderEntities: state.renderEntities,
}));

vi.mock("./visualization/pixi-canvas", () => ({
  initPixiCanvas: vi.fn(
    async (
      _container: HTMLElement,
      options?: {
        onSelectionChange?: (selectedIds: ReadonlySet<string>) => void;
        onMoveOrder?: (world: Pos) => void | Promise<void>;
      }
    ) => {
      state.onMoveOrder = options?.onMoveOrder ?? null;
      options?.onSelectionChange?.(state.selectedIds);
      return {
        updateEntities: vi.fn(),
        getSelectedIds: () => state.selectedIds,
        clearSelection: state.clearSelection,
        setSelectedIds: vi.fn((ids: readonly string[]) => {
          state.selectedIds.clear();
          for (const id of ids) state.selectedIds.add(id);
          options?.onSelectionChange?.(state.selectedIds);
        }),
        showMoveTarget: state.showMoveTarget,
        LookAt: vi.fn(() => true),
      };
    }
  ),
}));

function mountMainDom(): void {
  document.body.innerHTML = `
    <div id="status"></div>
    <div id="selection-detail"></div>
    <div id="entity-list"></div>
    <div id="canvas-container"></div>
    <button id="clear-selection" hidden disabled></button>
    <button id="form-group" hidden disabled></button>
    <button id="group-mode" hidden disabled></button>
    <p id="group-state"></p>
    <button id="stop-order" hidden disabled></button>
    <button id="board-units" hidden disabled></button>
    <button id="unboard-units" hidden disabled></button>
    <p id="transport-state"></p>
  `;
}

async function flush(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
}

describe("main input wiring", () => {
  beforeEach(() => {
    vi.resetModules();
    state.selectedIds.clear();
    state.selectedIds.add("u1");
    state.onMoveOrder = null;
    state.clearSelection.mockClear();
    state.showMoveTarget.mockClear();
    state.moveSelectedTo.mockClear();
    state.createGroupWith.mockClear();
    state.orderGroupTo.mockClear();
    state.boardUnits.mockClear();
    state.stopSelected.mockClear();
    state.unboardUnits.mockClear();
    state.renderEntities.mockClear();
    for (const entity of state.world) entity.aboard = null;
    vi.stubGlobal("requestAnimationFrame", vi.fn());
    mountMainDom();
  });

  it("clears selection by Escape", async () => {
    await import("./main");
    await flush();

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    expect(state.clearSelection).toHaveBeenCalledTimes(1);
  });

  it("clears selection by clear-selection button", async () => {
    await import("./main");
    await flush();

    const clearButton = document.getElementById(
      "clear-selection"
    ) as HTMLButtonElement;
    clearButton.click();

    expect(state.clearSelection).toHaveBeenCalledTimes(1);
  });

  it("routes a click to the group once one is formed", async () => {
    await import("./main");
    await flush();

    // The snapshot mock carries a faction-1 unit, which is what forming a group reads.
    (document.getElementById("form-group") as HTMLButtonElement).click();
    await flush();
    await flush();

    await state.onMoveOrder?.({ x: 10, y: 20 });

    expect(state.orderGroupTo).toHaveBeenCalledWith(
      { index: 7, generation: 0 },
      { x: 10, y: 20 }
    );
    expect(state.moveSelectedTo).not.toHaveBeenCalled();
  });

  it("boards the selected units onto the one selected vehicle", async () => {
    state.selectedIds.add("v1");
    await import("./main");
    await flush();
    await flush();

    const board = document.getElementById("board-units") as HTMLButtonElement;
    expect(board.hidden).toBe(false);
    board.click();
    await flush();

    // The truck is the vehicle, not cargo: it must not be in the list of units boarding it.
    expect(state.boardUnits).toHaveBeenCalledWith(["u1"], "v1");
  });

  it("unboards everyone the selected vehicle carries", async () => {
    state.world[0].aboard = "v1";
    state.selectedIds.clear();
    state.selectedIds.add("v1");
    await import("./main");
    await flush();
    await flush();

    const unboard = document.getElementById(
      "unboard-units"
    ) as HTMLButtonElement;
    expect(unboard.hidden).toBe(false);
    unboard.click();
    await flush();

    expect(state.unboardUnits).toHaveBeenCalledWith(["u1"]);
  });

  it("says why boarding did nothing instead of logging it", async () => {
    state.selectedIds.add("v1");
    state.boardUnits.mockRejectedValueOnce(
      new Error("the unit is more than 3 units away")
    );
    await import("./main");
    await flush();

    (document.getElementById("board-units") as HTMLButtonElement).click();
    await flush();
    await flush();

    expect(document.getElementById("transport-state")?.textContent).toContain(
      "more than 3 units away"
    );
  });

  it("stops the selection so a vehicle does not drive off without its passengers", async () => {
    await import("./main");
    await flush();
    await flush();

    const stop = document.getElementById("stop-order") as HTMLButtonElement;
    expect(stop.hidden).toBe(false);
    stop.click();
    await flush();

    expect(state.stopSelected).toHaveBeenCalledWith(["u1"]);
  });

  it("keeps move-order flow active via onMoveOrder callback", async () => {
    await import("./main");
    await flush();

    expect(state.onMoveOrder).not.toBeNull();
    await state.onMoveOrder?.({ x: 40, y: 50 });

    expect(state.moveSelectedTo).toHaveBeenCalledWith(["u1"], { x: 40, y: 50 });
    expect(state.showMoveTarget).toHaveBeenCalledWith({ x: 40, y: 50 });
  });
});
