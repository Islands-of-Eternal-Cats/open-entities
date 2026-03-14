import { describe, it, expect, vi, beforeEach } from "vitest";

/** Mock fetch so initWasm gets a wasm buffer without hitting the network. */
function installMockFetch(): void {
  vi.stubGlobal(
    "fetch",
    vi.fn(() =>
      Promise.resolve({
        ok: true,
        arrayBuffer: () => Promise.resolve(new ArrayBuffer(0)),
      } as Response)
    )
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
      }
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
});
