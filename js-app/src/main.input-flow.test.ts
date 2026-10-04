import { beforeEach, describe, expect, it, vi } from "vitest";
import type { EntitySnapshot, Pos } from "./core/types";

const state = vi.hoisted(() => {
  const applied = { seq: 0, ok: true as const, applied: 1, skipped: 0 };
  const selectedIds = new Set<string>(["u1"]);
  /** A unit on foot, an immobile base and a truck: what the transport buttons read. */
  const world: EntitySnapshot[] = [
    {
      id: "u1",
      entityType: "mover",
      pos: { x: 0, y: 0 },
      // A velocity is what makes it mobile, and therefore what makes it cargo the demo will offer
      // to load. The base below deliberately has none.
      velocity: { vx: 0, vy: 0 },
      faction: 1,
      seats: null,
      aboard: null,
      boarding: null,
      group: null,
      moveTarget: null,
    },
    {
      id: "b1",
      entityType: "base",
      pos: { x: 2, y: 0 },
      velocity: null,
      faction: 1,
      seats: null,
      aboard: null,
      boarding: null,
      group: null,
      moveTarget: null,
    },
    {
      id: "v1",
      entityType: "truck",
      pos: { x: 1, y: 0 },
      velocity: { vx: 0, vy: 0 },
      faction: 1,
      seats: 4,
      aboard: null,
      boarding: null,
      group: null,
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
    orderGroupTo: vi.fn(async () => applied),
    boardUnits: vi.fn(async () => applied),
    stopSelected: vi.fn(async () => applied),
    unboardUnits: vi.fn(async () => applied),
    // Orders resolve with the command's outcome; the world comes from frames only.
    moveSelectedTo: vi.fn(async () => applied),
    renderEntities: vi.fn(),
    frame: vi.fn(async (_elapsedMs: number) => ({ tick: 0, alpha: 0, stepped: false })),
    drawWorld: vi.fn(),
    /** Animation-frame callbacks main.ts asked for, run by hand in the loop test. */
    animationFrames: [] as FrameRequestCallback[],
  };
});

vi.mock("./core/wasm", async () => ({
  // The world the HUD reads: the same three rows, through the interface main.ts uses.
  world: Object.assign(
    (await import("./test-support/fake-world")).fakeWorld(state.world),
    { extrapolate: vi.fn() }
  ),
  initWasm: vi.fn(async () => {}),
  isWasmReady: vi.fn(() => true),
  coreBuildInfo: vi.fn(() => ({ id: "deadbeef", bytes: 1024 })),
  moveSelectedTo: state.moveSelectedTo,
  createGroupWith: state.createGroupWith,
  orderGroupTo: state.orderGroupTo,
  boardUnits: state.boardUnits,
  stopSelected: state.stopSelected,
  unboardUnits: state.unboardUnits,
  frame: state.frame,
  // main.ts imports these two as well; leaving them out made run() throw on the first
  // `await snapshot()` and swallow the rest of the wiring.
  snapshot: vi.fn(async () => {}),
  spawnRandomAt: vi.fn(async () => state.world[0]),
  spawnAt: vi.fn(async () => state.world[0]),
  stress: vi.fn(async () => 0),
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
        drawWorld: state.drawWorld,
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
    state.frame.mockClear();
    state.drawWorld.mockClear();
    state.animationFrames.length = 0;
    vi.stubGlobal(
      "requestAnimationFrame",
      vi.fn((callback: FrameRequestCallback) => {
        state.animationFrames.push(callback);
        return state.animationFrames.length;
      })
    );
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
      new Error("entity 9:0 has no seats to board")
    );
    await import("./main");
    await flush();

    (document.getElementById("board-units") as HTMLButtonElement).click();
    await flush();
    await flush();

    expect(document.getElementById("transport-state")?.textContent).toContain(
      "no seats to board"
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

  it("does not offer to load a building into the truck", async () => {
    state.selectedIds.clear();
    state.selectedIds.add("b1");
    state.selectedIds.add("v1");
    await import("./main");
    await flush();
    await flush();

    // The base has no velocity, so in this engine it does not move under its own power and has no
    // business being freight. The core would allow it; the demo should not suggest it.
    const board = document.getElementById("board-units") as HTMLButtonElement;
    expect(board.hidden).toBe(true);
  });

  it("does not redraw the world from an order reply", async () => {
    await import("./main");
    await flush();
    const drawsBefore = state.renderEntities.mock.calls.length;

    await state.onMoveOrder?.({ x: 40, y: 50 });
    (document.getElementById("stop-order") as HTMLButtonElement).click();
    await flush();

    // The order applies at the next tick and shows up in the frame that runs it; a reply that
    // drew its own snapshot would make positions jump for a frame.
    expect(state.moveSelectedTo).toHaveBeenCalled();
    expect(state.stopSelected).toHaveBeenCalled();
    expect(state.renderEntities.mock.calls.length).toBe(drawsBefore);
  });

  it("lists only the selection in Forces, with the whole army's count", async () => {
    await import("./main");
    await flush();

    const calls = state.renderEntities.mock.calls;
    const [rows, , selected, total] = calls[calls.length - 1] as unknown as [
      EntitySnapshot[],
      HTMLElement,
      ReadonlySet<string>,
      number,
    ];
    expect(rows.map((row) => row.id)).toEqual(["u1"]);
    expect([...selected]).toEqual(["u1"]);
    expect(total).toBe(3);
  });

  it("keeps move-order flow active via onMoveOrder callback", async () => {
    await import("./main");
    await flush();

    expect(state.onMoveOrder).not.toBeNull();
    await state.onMoveOrder?.({ x: 40, y: 50 });

    expect(state.moveSelectedTo).toHaveBeenCalledWith(["u1"], { x: 40, y: 50 });
    expect(state.showMoveTarget).toHaveBeenCalledWith({ x: 40, y: 50 });
  });

  it("draws every animation frame without waiting for the worker, one request at a time", async () => {
    let reply: (value: { tick: number; alpha: number; stepped: boolean }) => void = () => {};
    state.frame.mockImplementation(
      () => new Promise((resolve) => (reply = resolve))
    );
    await import("./main");
    await flush();
    await flush();
    const nextFrame = (timestamp: number) => state.animationFrames.shift()!(timestamp);

    nextFrame(0);
    const drawsAfterFirst = state.drawWorld.mock.calls.length;
    nextFrame(16);
    nextFrame(32);

    // Drawn on each frame; only the first request went out, the reply is still pending.
    expect(state.drawWorld.mock.calls.length).toBe(drawsAfterFirst + 2);
    expect(state.frame).toHaveBeenCalledTimes(1);

    reply({ tick: 1, alpha: 0, stepped: true });
    await flush();
    await flush();
    nextFrame(48);

    // The time that passed while the request was out goes with the next one.
    expect(state.frame).toHaveBeenCalledTimes(2);
    expect(state.frame.mock.calls[1][0]).toBe(48);
  });
});
