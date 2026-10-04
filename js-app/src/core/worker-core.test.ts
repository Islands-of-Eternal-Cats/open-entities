import { afterEach, describe, expect, it, vi } from "vitest";
import { TICK_MS } from "./fixed-step";
import { FRAME_HEADER_LEN, frameTick } from "./frame-buffer";
import type { Command, CommandOutcome } from "./types";
import { WorkerCore, type FrameSimulation } from "./worker-core";

/**
 * A fake core: one unit whose x is the tick number in map units. Submitted commands apply on the
 * next step and come back as outcomes; `pendingMeta` is what the next `metaDelta()` returns.
 */
function fakeSim() {
  let tick = 0;
  let seq = 0;
  const queued: number[] = [];
  const sim = {
    pendingMeta: undefined as string | undefined,
    getWorldAsJson: vi.fn(() => "{}"),
    currentTick: () => tick,
    submit: (_command: Command) => {
      queued.push(seq);
      return seq++;
    },
    step: () => {
      tick += 1;
      const outcomes: CommandOutcome[] = queued
        .splice(0)
        .map((s) => ({ seq: s, ok: true, applied: 1, skipped: 0 }));
      return { tick, outcomes };
    },
    writeFrame: () => {
      const frame = new Int32Array(FRAME_HEADER_LEN + 4);
      frame.set([tick, 0, 1, 7, 0, tick * 1000, 0]);
      return frame;
    },
    metaDelta: () => {
      const meta = sim.pendingMeta;
      sim.pendingMeta = undefined;
      return meta;
    },
  };
  return sim satisfies FrameSimulation & Record<string, unknown>;
}

function ticksOf(buffer: ArrayBuffer | undefined): number | undefined {
  return buffer ? frameTick(new Int32Array(buffer)) : undefined;
}

describe("WorkerCore.frame", () => {
  afterEach(() => vi.restoreAllMocks());

  it("posts only tick and alpha on a frame without a step", () => {
    const core = new WorkerCore(fakeSim());
    const { message, transfer } = core.frame(TICK_MS / 5);
    expect(message).toEqual({ type: "frame", tick: 0, alpha: 0.2 });
    expect(transfer).toEqual([]);
  });

  it("posts the buffer of the last tick as a transferable after one step, with the outcomes", () => {
    const sim = fakeSim();
    const core = new WorkerCore(sim);
    sim.submit({ type: "stop", ids: [] });
    const { message, transfer } = core.frame(TICK_MS);
    if (message.type !== "frame") throw new Error("expected a frame");
    expect(message.tick).toBe(1);
    expect(ticksOf(message.current)).toBe(1);
    expect(message.previous).toBeUndefined();
    expect(message.outcomes).toEqual([{ seq: 0, ok: true, applied: 1, skipped: 0 }]);
    expect(transfer).toEqual([message.current]);
  });

  it("also posts the buffer of tick t−1 after two or more steps", () => {
    const core = new WorkerCore(fakeSim());
    const { message, transfer } = core.frame(TICK_MS * 3);
    if (message.type !== "frame") throw new Error("expected a frame");
    expect(message.tick).toBe(3);
    expect(ticksOf(message.current)).toBe(3);
    expect(ticksOf(message.previous)).toBe(2);
    expect(transfer).toEqual([message.current, message.previous]);
  });

  it("collects the outcomes of every step in the frame", () => {
    const sim = fakeSim();
    const core = new WorkerCore(sim);
    sim.submit({ type: "stop", ids: [] });
    core.frame(0);
    const { message } = core.frame(TICK_MS * 2);
    if (message.type !== "frame") throw new Error("expected a frame");
    expect(message.outcomes?.map((o) => o.seq)).toEqual([0]);
  });

  it("posts metadata only when the core reports a change", () => {
    const sim = fakeSim();
    const core = new WorkerCore(sim);
    expect(core.frame(TICK_MS).message).not.toHaveProperty("meta");
    const meta = '{"changed":[],"removed":[{"index":7,"generation":0}]}';
    sim.pendingMeta = meta;
    expect(core.frame(TICK_MS).message).toHaveProperty("meta", meta);
  });

  it("creates no JSON on the per-frame path, with or without steps", () => {
    const sim = fakeSim();
    const core = new WorkerCore(sim);
    const stringify = vi.spyOn(JSON, "stringify");
    const parse = vi.spyOn(JSON, "parse");
    for (const elapsed of [0, TICK_MS / 3, TICK_MS, TICK_MS * 4, 7]) {
      core.frame(elapsed);
    }
    expect(stringify).not.toHaveBeenCalled();
    expect(parse).not.toHaveBeenCalled();
    expect(sim.getWorldAsJson).not.toHaveBeenCalled();
  });
});

describe("WorkerCore.snapshot", () => {
  it("posts the current frame and the full metadata without stepping", () => {
    const sim = fakeSim();
    sim.pendingMeta = '{"changed":[],"removed":[]}';
    const core = new WorkerCore(sim);
    const { message, transfer } = core.snapshot();
    if (message.type !== "snapshot") throw new Error("expected a snapshot");
    expect(message.tick).toBe(0);
    expect(ticksOf(message.current)).toBe(0);
    expect(message.meta).toBe('{"changed":[],"removed":[]}');
    expect(transfer).toEqual([message.current]);
  });
});

describe("WorkerCore.stress", () => {
  it("submits one seeded spawn command per unit, the same for the same seed", () => {
    const submitted: Command[][] = [[], []];
    for (const run of submitted) {
      const sim = fakeSim();
      sim.submit = (command: Command) => {
        run.push(command);
        return run.length;
      };
      const { message } = new WorkerCore(sim).stress(3, 42, 100);
      expect(message).toEqual({ type: "stressed", count: 3 });
    }
    expect(submitted[0]).toHaveLength(3);
    expect(submitted[0]).toEqual(submitted[1]);
    const [first] = submitted[0];
    if (first.type !== "spawn") throw new Error("expected spawn");
    const overrides = first.overrides as {
      position: { x: number; y: number };
      move_target: { x: number; y: number };
    };
    for (const v of [
      overrides.position.x,
      overrides.position.y,
      overrides.move_target.x,
      overrides.move_target.y,
    ]) {
      expect(v).toBeGreaterThanOrEqual(0);
      expect(v).toBeLessThanOrEqual(100);
    }
  });
});
