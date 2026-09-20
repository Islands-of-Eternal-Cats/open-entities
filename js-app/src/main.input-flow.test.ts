import { beforeEach, describe, expect, it, vi } from "vitest";
import type { EntitySnapshot, Pos } from "./core/types";

const state = vi.hoisted(() => {
  const selectedIds = new Set<string>(["u1"]);
  return {
    selectedIds,
    onMoveOrder: null as ((world: Pos) => void | Promise<void>) | null,
    clearSelection: vi.fn(() => {
      selectedIds.clear();
    }),
    showMoveTarget: vi.fn(),
    moveSelectedTo: vi.fn(async () => [
      {
        id: "u1",
        entityType: "mover",
        pos: { x: 12, y: 24 },
        velocity: null,
        faction: null,
      } satisfies EntitySnapshot,
    ]),
    renderEntities: vi.fn(),
  };
});

vi.mock("./core/wasm", () => ({
  initWasm: vi.fn(async () => {}),
  isWasmReady: vi.fn(() => true),
  moveSelectedTo: state.moveSelectedTo,
  tick: vi.fn(async () => [] as EntitySnapshot[]),
  spawnRandomAt: vi.fn(async () => [] as EntitySnapshot[]),
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
    state.renderEntities.mockClear();
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

  it("keeps move-order flow active via onMoveOrder callback", async () => {
    await import("./main");
    await flush();

    expect(state.onMoveOrder).not.toBeNull();
    await state.onMoveOrder?.({ x: 40, y: 50 });

    expect(state.moveSelectedTo).toHaveBeenCalledWith(["u1"], { x: 40, y: 50 });
    expect(state.showMoveTarget).toHaveBeenCalledWith({ x: 40, y: 50 });
  });
});
