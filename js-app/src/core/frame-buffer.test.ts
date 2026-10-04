import { describe, expect, it } from "vitest";
import {
  FRAME_HEADER_LEN,
  FRAME_STRIDE,
  findRow,
  frameCount,
  frameTick,
  interpolateFrames,
} from "./frame-buffer";

/** Builds a frame: rows of `[index, generation, x, y]` in milli-units. */
function frame(tick: number, rows: number[][]): Int32Array {
  const out = new Int32Array(FRAME_HEADER_LEN + rows.length * FRAME_STRIDE);
  out[0] = tick % 2 ** 32;
  out[1] = Math.floor(tick / 2 ** 32);
  out[2] = rows.length;
  rows.forEach((row, i) => out.set(row, FRAME_HEADER_LEN + i * FRAME_STRIDE));
  return out;
}

type Drawn = [index: number, generation: number, x: number, y: number];

function drawn(
  previous: Int32Array | null,
  current: Int32Array,
  alpha: number
): Drawn[] {
  const out: Drawn[] = [];
  interpolateFrames(previous, current, alpha, (index, generation, x, y) => {
    out.push([index, generation, x, y]);
  });
  return out;
}

describe("frame header", () => {
  it("reads the tick from two words and the row count", () => {
    const f = frame(2 ** 32 + 5, [[1, 0, 0, 0]]);
    expect(frameTick(f)).toBe(2 ** 32 + 5);
    expect(frameCount(f)).toBe(1);
  });

  it("reads a low word with the top bit set as unsigned", () => {
    const f = frame(0, []);
    f[0] = -1;
    expect(frameTick(f)).toBe(2 ** 32 - 1);
  });
});

describe("interpolateFrames (merge-join on index)", () => {
  it("blends an entity present in both ticks and converts milli-units to map units", () => {
    const previous = frame(1, [[3, 0, 10_000, 0]]);
    const current = frame(2, [[3, 0, 20_000, 4_000]]);
    expect(drawn(previous, current, 0.25)).toEqual([[3, 0, 12.5, 1]]);
  });

  it("draws an entity spawned between the ticks where it is now", () => {
    const previous = frame(1, [[1, 0, 0, 0]]);
    const current = frame(2, [
      [1, 0, 1000, 0],
      [2, 0, 7000, 7000],
    ]);
    expect(drawn(previous, current, 0.5)).toEqual([
      [1, 0, 0.5, 0],
      [2, 0, 7, 7],
    ]);
  });

  it("does not draw an entity despawned between the ticks", () => {
    const previous = frame(1, [
      [1, 0, 0, 0],
      [2, 0, 5000, 0],
      [4, 0, 0, 0],
    ]);
    const current = frame(2, [
      [1, 0, 2000, 0],
      [4, 0, 0, 2000],
    ]);
    expect(drawn(previous, current, 0.5)).toEqual([
      [1, 0, 1, 0],
      [4, 0, 0, 1],
    ]);
  });

  it("does not blend across a reused index: another generation is another entity", () => {
    const previous = frame(1, [[5, 0, 0, 0]]);
    const current = frame(2, [[5, 1, 9000, 0]]);
    expect(drawn(previous, current, 0.5)).toEqual([[5, 1, 9, 0]]);
  });

  it("handles spawn and despawn on both sides of the join at once", () => {
    const previous = frame(1, [
      [0, 0, 0, 0],
      [2, 0, 0, 0],
      [6, 0, 0, 0],
    ]);
    const current = frame(2, [
      [1, 0, 1000, 0],
      [2, 0, 2000, 0],
      [7, 0, 3000, 0],
    ]);
    expect(drawn(previous, current, 0.5)).toEqual([
      [1, 0, 1, 0],
      [2, 0, 1, 0],
      [7, 0, 3, 0],
    ]);
  });

  it("draws the current tick as is without a previous frame", () => {
    const current = frame(0, [[1, 0, 1500, -2500]]);
    expect(drawn(null, current, 0.7)).toEqual([[1, 0, 1.5, -2.5]]);
  });
});

describe("findRow", () => {
  const f = frame(1, [
    [1, 0, 0, 0],
    [4, 2, 0, 0],
    [9, 0, 0, 0],
  ]);

  it("finds a row by index and generation", () => {
    expect(findRow(f, 4, 2)).toBe(FRAME_HEADER_LEN + FRAME_STRIDE);
    expect(findRow(f, 9, 0)).toBe(FRAME_HEADER_LEN + 2 * FRAME_STRIDE);
  });

  it("misses an absent index or a stale generation", () => {
    expect(findRow(f, 5, 0)).toBe(-1);
    expect(findRow(f, 4, 1)).toBe(-1);
  });
});
