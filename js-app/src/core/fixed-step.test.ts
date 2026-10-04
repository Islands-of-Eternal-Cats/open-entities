import { describe, expect, it } from "vitest";
import { advanceClock, MAX_STEPS_PER_FRAME, TICK_MS } from "./fixed-step";

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
