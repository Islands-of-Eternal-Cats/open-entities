import { describe, it, expect, vi, beforeEach } from "vitest";

/** Mock fetch so initWasm gets wasm + yaml without hitting the network. */
function installMockFetch(): void {
  vi.stubGlobal(
    "fetch",
    vi.fn((url: string | URL) => {
      const s = String(url);
      const yaml = "entities: {}";
      return Promise.resolve({
        ok: true,
        status: 200,
        statusText: "OK",
        arrayBuffer: () => Promise.resolve(new ArrayBuffer(8)),
        text: () =>
          s.includes("entities.yaml") ? Promise.resolve(yaml) : Promise.resolve(""),
      } as Response);
    })
  );
}

/** Mock Worker so init runs without loading real WASM in worker. */
function installMockWorker(): void {
  class MockWorker {
    onmessage: ((e: MessageEvent) => void) | null = null;
    private listeners: Array<(e: MessageEvent) => void> = [];

    addEventListener(_: string, fn: (e: MessageEvent) => void): void {
      this.listeners.push(fn);
    }

    removeEventListener(_: string, fn: (e: MessageEvent) => void): void {
      this.listeners = this.listeners.filter((l) => l !== fn);
    }

    postMessage(data: unknown): void {
      if (
        data &&
        typeof data === "object" &&
        "type" in data &&
        (data as { type: string }).type === "init"
      ) {
        setTimeout(() => {
          const e = { data: { type: "ready" } } as MessageEvent;
          this.onmessage?.(e);
          this.listeners.forEach((fn) => fn(e));
        }, 0);
        return;
      }

      if (
        data &&
        typeof data === "object" &&
        "type" in data &&
        (data as { type: string }).type === "spawn_at"
      ) {
        const d = data as {
          type: "spawn_at";
          typeName: string;
          x: number;
          y: number;
          faction?: number;
        };
        setTimeout(() => {
          const e = {
            data: {
              type: "entities",
              entities: [
                {
                  id: "1",
                  entityType: d.typeName,
                  pos: { x: d.x, y: d.y },
                  velocity: null,
                  faction: d.faction ?? null,
                },
              ],
            },
          } as MessageEvent;
          this.onmessage?.(e);
          this.listeners.forEach((fn) => fn(e));
        }, 0);
      }
    }

    terminate(): void {
      /* jsdom Worker has no terminate; real wasm.ts cleans up on init failure */
    }
  }
  vi.stubGlobal("Worker", MockWorker);
}

describe("wasm module", () => {
  beforeEach(() => {
    vi.resetModules();
    installMockFetch();
    installMockWorker();
  });

  it("isWasmReady returns false before init", async () => {
    const { isWasmReady } = await import("./wasm");
    expect(isWasmReady()).toBe(false);
  });

  it("isWasmReady returns true after initWasm", async () => {
    const { initWasm, isWasmReady } = await import("./wasm");
    expect(isWasmReady()).toBe(false);
    await initWasm();
    expect(isWasmReady()).toBe(true);
  });

  it("spawnRandomAt uses WORLD_SIZE for random coordinate range", async () => {
    const randomSpy = vi
      .spyOn(Math, "random")
      .mockReturnValueOnce(0.5)
      .mockReturnValueOnce(0.25);
    const { initWasm, spawnRandomAt } = await import("./wasm");
    const { WORLD_SIZE } = await import("../visualization/coords");

    await initWasm();
    const entities = await spawnRandomAt("mover");

    expect(entities).toHaveLength(1);
    expect(entities[0]?.pos.x).toBeCloseTo(0.5 * WORLD_SIZE);
    expect(entities[0]?.pos.y).toBeCloseTo(0.25 * WORLD_SIZE);
    expect(randomSpy).toHaveBeenCalledTimes(2);
    randomSpy.mockRestore();
  });
});
