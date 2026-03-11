import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("open-entities-wasm", () => ({
  default: vi.fn().mockResolvedValue(undefined),
}));

describe("wasm module", () => {
  beforeEach(() => {
    vi.resetModules();
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
