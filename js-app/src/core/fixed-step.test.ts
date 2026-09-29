import { describe, expect, it } from "vitest";
import {
  advanceClock,
  FrameClock,
  interpolate,
  MAX_STEPS_PER_FRAME,
  TICK_MS,
} from "./fixed-step";
import type { EntitySnapshot, Pos } from "./types";

function runFrames(frameMs: number, frames: number) {
  let accumulator = 0;
  let steps = 0;
  for (let i = 0; i < frames; i++) {
    const r = advanceClock(accumulator, frameMs);
    accumulator = r.accumulator;
    steps += r.steps;
  }
  return { accumulator, steps };
}

describe("advanceClock", () => {
  it("steps once per whole tick and keeps the remainder", () => {
    expect(advanceClock(0, TICK_MS * 2 + 7)).toEqual({
      steps: 2,
      accumulator: 7,
      alpha: 7 / TICK_MS,
    });
  });

  it("does not step on a frame shorter than a tick", () => {
    const r = advanceClock(0, 16);
    expect(r.steps).toBe(0);
    expect(r.alpha).toBeCloseTo(16 / TICK_MS);
  });

  it("runs the same number of ticks per wall-clock second at 60 Hz and 144 Hz", () => {
    // Ten seconds of frames; float frame lengths may leave the last tick in the accumulator.
    const expected = 10_000 / TICK_MS;
    const at60 = runFrames(1000 / 60, 600);
    const at144 = runFrames(1000 / 144, 1440);
    expect(Math.abs(at60.steps - expected)).toBeLessThanOrEqual(1);
    expect(Math.abs(at144.steps - expected)).toBeLessThanOrEqual(1);
  });

  it("caps steps per frame and drops the excess time", () => {
    const r = advanceClock(0, TICK_MS * (MAX_STEPS_PER_FRAME + 10));
    expect(r.steps).toBe(MAX_STEPS_PER_FRAME);
    expect(r.accumulator).toBeLessThan(TICK_MS);
  });

  it("keeps alpha in [0, 1)", () => {
    for (const dt of [0, 1, 16.6, 49.9, 50, 123]) {
      const { alpha } = advanceClock(0, dt);
      expect(alpha).toBeGreaterThanOrEqual(0);
      expect(alpha).toBeLessThan(1);
    }
  });
});

/** A fake simulation: one unit that moves +1 x per step, so x equals the tick number. */
function fakeSim() {
  let tick = 0;
  return {
    get tick() {
      return tick;
    },
    step: () => {
      tick += 1;
    },
    positions: (): Record<string, Pos> => ({ u: { x: tick, y: 0 } }),
  };
}

describe("FrameClock", () => {
  it("keeps the previous tick, not the state at the start of the frame, after several steps", () => {
    const sim = fakeSim();
    const clock = new FrameClock(sim);
    const frame = clock.frame(TICK_MS * 3);
    expect(sim.tick).toBe(3);
    expect(frame.previous).toEqual({ u: { x: 2, y: 0 } });
    expect(frame.alpha).toBe(0);
  });

  it("reports alpha on frames with no step and keeps the last two ticks", () => {
    const sim = fakeSim();
    const clock = new FrameClock(sim);
    clock.frame(TICK_MS);
    const a = clock.frame(TICK_MS / 5);
    const b = clock.frame(TICK_MS / 5);
    expect(sim.tick).toBe(1);
    expect(a.previous).toEqual({ u: { x: 0, y: 0 } });
    expect(b.previous).toEqual({ u: { x: 0, y: 0 } });
    expect(a.alpha).toBeCloseTo(0.2);
    expect(b.alpha).toBeCloseTo(0.4);
  });
});

describe("interpolate", () => {
  const unit = (id: string, x: number): EntitySnapshot => ({
    id,
    entityType: "unit",
    pos: { x, y: 0 },
    velocity: null,
    faction: null,
    seats: null,
    aboard: null,
    boarding: null,
    moveTarget: null,
  });

  it("blends previous and current positions by alpha", () => {
    const [e] = interpolate({ a: { x: 10, y: 0 } }, [unit("a", 20)], 0.25);
    expect(e.pos).toEqual({ x: 12.5, y: 0 });
  });

  it("draws an entity with no previous position where it is now", () => {
    const [e] = interpolate({}, [unit("new", 7)], 0.5);
    expect(e.pos).toEqual({ x: 7, y: 0 });
  });
});
