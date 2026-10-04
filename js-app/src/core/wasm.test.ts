import { describe, it, expect, vi, beforeEach } from "vitest";

/** Bytes that satisfy the wasm magic-number guard in initWasm: "\0asm" + version 1. */
function wasmHeaderBytes(): ArrayBuffer {
  return new Uint8Array([0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]).buffer;
}

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
        arrayBuffer: () => Promise.resolve(wasmHeaderBytes()),
        text: () =>
          s.includes("entities.yaml") ? Promise.resolve(yaml) : Promise.resolve(""),
      } as Response);
    })
  );
}

/**
 * What the mock worker does with commands: every `submit` gets the next sequence numbers, and the
 * next `frame` reports one outcome per command submitted since the frame before, built by
 * `outcomeFor`. Tests swap `outcomeFor` and `frameEntities` to script the core.
 */
const workerScript = {
  nextSeq: 0,
  posted: [] as Array<{ type: string; [key: string]: unknown }>,
  queued: [] as Array<{ seq: number; command: Record<string, unknown> }>,
  /** Rows `[index, generation, x, y]` (milli-units) of the next frame's buffer. */
  frameRows: [] as number[][],
  /** Metadata delta JSON the next frame carries, if any. */
  frameMeta: undefined as string | undefined,
  tick: 0,
  outcomeFor: (seq: number, _command: Record<string, unknown>): unknown => ({
    seq,
    ok: true,
    applied: 1,
    skipped: 0,
  }),
};

function resetWorkerScript(): void {
  workerScript.nextSeq = 0;
  workerScript.posted = [];
  workerScript.queued = [];
  workerScript.frameRows = [];
  workerScript.frameMeta = undefined;
  workerScript.tick = 0;
  workerScript.outcomeFor = (seq) => ({ seq, ok: true, applied: 1, skipped: 0 });
}

/** Mock Worker so init runs without loading real WASM in worker. */
function installMockWorker(): void {
  class MockWorker {
    onmessage: ((e: MessageEvent) => void) | null = null;

    private reply(data: unknown): void {
      setTimeout(() => this.onmessage?.({ data } as MessageEvent), 0);
    }

    postMessage(data: { type: string; [key: string]: unknown }): void {
      workerScript.posted.push(data);
      if (data.type === "init") {
        this.reply({ type: "ready" });
        return;
      }
      if (data.type === "submit") {
        const commands = data.commands as Array<Record<string, unknown>>;
        const seqs = commands.map((command) => {
          const seq = workerScript.nextSeq++;
          workerScript.queued.push({ seq, command });
          return seq;
        });
        this.reply({ type: "submitted", seqs });
        return;
      }
      if (data.type === "frame") {
        const outcomes = workerScript.queued.map(({ seq, command }) =>
          workerScript.outcomeFor(seq, command)
        );
        workerScript.queued = [];
        workerScript.tick += 1;
        const rows = workerScript.frameRows;
        const current = new Int32Array(3 + rows.length * 4);
        current.set([workerScript.tick, 0, rows.length]);
        rows.forEach((row, i) => current.set(row, 3 + i * 4));
        this.reply({
          type: "frame",
          tick: workerScript.tick,
          alpha: 0,
          current: current.buffer,
          outcomes,
          ...(workerScript.frameMeta !== undefined ? { meta: workerScript.frameMeta } : {}),
        });
        workerScript.frameMeta = undefined;
      }
    }

    terminate(): void {
      /* jsdom Worker has no terminate; real wasm.ts cleans up on init failure */
    }
  }
  vi.stubGlobal("Worker", MockWorker);
}

/** Lets queued worker replies (setTimeout 0) and the promise chains behind them run. */
async function settle(): Promise<void> {
  for (let i = 0; i < 5; i++) await new Promise((r) => setTimeout(r, 0));
}

/** The commands the app submitted, in order. */
function submitted(): Array<Record<string, unknown>> {
  return workerScript.posted
    .filter((m) => m.type === "submit")
    .flatMap((m) => m.commands as Array<Record<string, unknown>>);
}

describe("wasm module", () => {
  it("frame() loads the posted buffer and metadata into the shared world", async () => {
    vi.resetModules();
    resetWorkerScript();
    installMockFetch();
    installMockWorker();
    const { initWasm, frame, world } = await import("./wasm");
    await initWasm();
    workerScript.frameRows = [[4, 0, 1500, 2500]];
    workerScript.frameMeta = JSON.stringify({
      changed: [{ id: { index: 4, generation: 0 }, entity_type: "truck", seats: 4, mobile: true }],
      removed: [],
    });

    const result = await frame(50);

    expect(result).toEqual({ tick: 1, alpha: 0, stepped: true });
    expect(world.size).toBe(1);
    expect(world.get("4:0")).toMatchObject({ entityType: "truck", seats: 4, pos: { x: 1.5, y: 2.5 } });
  });

  beforeEach(() => {
    vi.resetModules();
    resetWorkerScript();
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

  it("fingerprints the module it loaded, so a stale build is visible", async () => {
    const { initWasm, coreBuildInfo } = await import("./wasm");
    expect(coreBuildInfo()).toBeNull();

    await initWasm();

    const build = coreBuildInfo();
    // The mock fetch answers with the 8-byte wasm header, whose FNV-1a is fixed.
    expect(build?.bytes).toBe(8);
    expect(build?.id).toMatch(/^[0-9a-f]{8}$/);
  });

  it("spawnRandomAt submits a spawn across WORLD_SIZE and resolves from the next frame", async () => {
    const randomSpy = vi
      .spyOn(Math, "random")
      .mockReturnValueOnce(0.5)
      .mockReturnValueOnce(0.25);
    const { initWasm, spawnRandomAt, frame } = await import("./wasm");
    const { WORLD_SIZE } = await import("../visualization/coords");
    await initWasm();
    workerScript.outcomeFor = (seq) => ({
      seq,
      ok: true,
      spawned: { index: 9, generation: 2 },
    });
    workerScript.frameRows = [[9, 2, 500 * WORLD_SIZE, 250 * WORLD_SIZE]];
    workerScript.frameMeta = JSON.stringify({
      changed: [{ id: { index: 9, generation: 2 }, entity_type: "mover", mobile: true }],
      removed: [],
    });

    let spawned: { id: string; pos: { x: number; y: number } } | null = null;
    void spawnRandomAt("mover").then((entity) => (spawned = entity));
    await settle();

    expect(submitted()).toEqual([
      {
        type: "spawn",
        template: "mover",
        overrides: { position: { x: 0.5 * WORLD_SIZE, y: 0.25 * WORLD_SIZE } },
      },
    ]);
    expect(spawned).toBeNull(); // nothing is applied until a step runs

    await frame(50);
    await settle();

    expect(spawned).not.toBeNull();
    expect(spawned!.id).toBe("9:2");
    expect(spawned!.pos.x).toBeCloseTo(0.5 * WORLD_SIZE);
    expect(randomSpy).toHaveBeenCalledTimes(2);
    randomSpy.mockRestore();
  });

  it("an order resolves with its outcome, not with a snapshot", async () => {
    const { initWasm, moveSelectedTo, frame } = await import("./wasm");
    await initWasm();

    const order = moveSelectedTo(["3:0", "4:1"], { x: 10, y: 20 });
    await settle();
    expect(submitted()).toEqual([
      {
        type: "move_to",
        ids: [
          { index: 3, generation: 0 },
          { index: 4, generation: 1 },
        ],
        target: { x: 10, y: 20 },
      },
    ]);

    await frame(50);
    await expect(order).resolves.toEqual({ seq: 0, ok: true, applied: 1, skipped: 0 });
  });

  it("a refused command rejects with the core's reason", async () => {
    const { initWasm, boardUnits, frame } = await import("./wasm");
    await initWasm();
    workerScript.outcomeFor = (seq) => ({
      seq,
      ok: false,
      error: "entity 9:0 has no seats to board",
    });

    const order = boardUnits(["1:0"], "9:0");
    const rejected = expect(order).rejects.toThrow("no seats to board");
    await settle();
    await frame(50);
    await rejected;
    expect(submitted()).toEqual([
      { type: "board", units: [{ index: 1, generation: 0 }], vehicle: { index: 9, generation: 0 } },
    ]);
  });

  it("createGroupWith adds the units once the core has named the group", async () => {
    const { initWasm, createGroupWith, frame } = await import("./wasm");
    await initWasm();
    workerScript.outcomeFor = (seq, command) =>
      command.type === "create_group"
        ? { seq, ok: true, group: { index: 12, generation: 0 } }
        : { seq, ok: true, applied: 1, skipped: 0 };

    const group = createGroupWith(1, ["5:0", "6:0"]);
    await settle();
    expect(submitted()).toEqual([{ type: "create_group", faction: 1 }]);

    await frame(50);
    await settle();
    expect(submitted().slice(1)).toEqual([
      { type: "add_to_group", group: { index: 12, generation: 0 }, unit: { index: 5, generation: 0 } },
      { type: "add_to_group", group: { index: 12, generation: 0 }, unit: { index: 6, generation: 0 } },
    ]);

    await frame(50);
    await expect(group).resolves.toEqual({ index: 12, generation: 0 });
  });
});
