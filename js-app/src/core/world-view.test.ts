import { describe, expect, it } from "vitest";
import { FRAME_HEADER_LEN, FRAME_STRIDE } from "./frame-buffer";
import type { EntityMeta, MetaDelta } from "./world-view";
import { WorldView } from "./world-view";

function frame(tick: number, rows: number[][]): ArrayBuffer {
  const out = new Int32Array(FRAME_HEADER_LEN + rows.length * FRAME_STRIDE);
  out.set([tick, 0, rows.length]);
  rows.forEach((row, i) => out.set(row, FRAME_HEADER_LEN + i * FRAME_STRIDE));
  return out.buffer;
}

function meta(index: number, extra: Partial<EntityMeta> = {}): EntityMeta {
  return {
    id: { index, generation: 0 },
    entity_type: "mover",
    mobile: true,
    ...extra,
  };
}

function delta(changed: EntityMeta[], removed: MetaDelta["removed"] = []): string {
  return JSON.stringify({ changed, removed });
}

type Drawn = [key: string, x: number, y: number];

function drawn(view: WorldView): Drawn[] {
  const out: Drawn[] = [];
  view.forEachDrawn((index, generation, x, y) => out.push([`${index}:${generation}`, x, y]));
  return out;
}

/** A view after the initial snapshot at tick 0: units 1 and 2 at the origin. */
function started(): WorldView {
  const view = new WorldView();
  view.applySnapshot({
    type: "snapshot",
    tick: 0,
    current: frame(0, [
      [1, 0, 0, 0],
      [2, 0, 0, 0],
    ]),
    meta: delta([meta(1), meta(2, { entity_type: "truck", seats: 4, faction: 1 })]),
  });
  return view;
}

describe("WorldView", () => {
  it("draws the snapshot where it stands", () => {
    const view = started();
    expect(view.tick).toBe(0);
    expect(view.size).toBe(2);
    expect(drawn(view)).toEqual([
      ["1:0", 0, 0],
      ["2:0", 0, 0],
    ]);
  });

  it("keeps blending the same two frames on a frame without a step", () => {
    const view = started();
    view.applyFrame({
      type: "frame",
      tick: 1,
      alpha: 0,
      current: frame(1, [
        [1, 0, 2000, 0],
        [2, 0, 0, 0],
      ]),
      outcomes: [],
    });
    view.applyFrame({ type: "frame", tick: 1, alpha: 0.5 });
    expect(view.alpha).toBe(0.5);
    expect(drawn(view)[0]).toEqual(["1:0", 1, 0]);
  });

  it("shifts current to previous after one step", () => {
    const view = started();
    const tick = (t: number, x: number) =>
      view.applyFrame({
        type: "frame",
        tick: t,
        alpha: 0.25,
        current: frame(t, [[1, 0, x, 0], [2, 0, 0, 0]]),
        outcomes: [],
      });
    tick(1, 1000);
    tick(2, 3000);
    // previous is tick 1 (x = 1), current tick 2 (x = 3)
    expect(drawn(view)[0]).toEqual(["1:0", 1.5, 0]);
  });

  it("takes the previous buffer from the message after several steps", () => {
    const view = started();
    view.applyFrame({
      type: "frame",
      tick: 3,
      alpha: 0.5,
      previous: frame(2, [[1, 0, 2000, 0], [2, 0, 0, 0]]),
      current: frame(3, [[1, 0, 4000, 0], [2, 0, 0, 0]]),
      outcomes: [],
    });
    expect(drawn(view)[0]).toEqual(["1:0", 3, 0]);
  });

  it("draws a unit spawned this tick once its metadata arrives, and drops a despawned one", () => {
    const view = started();
    view.applyFrame({
      type: "frame",
      tick: 1,
      alpha: 0.5,
      current: frame(1, [[2, 0, 0, 0], [5, 0, 9000, 9000]]),
      meta: delta([meta(5)], [{ index: 1, generation: 0 }]),
      outcomes: [],
    });
    expect(drawn(view)).toEqual([
      ["2:0", 0, 0],
      ["5:0", 9, 9],
    ]);
    expect(view.get("1:0")).toBeNull();
    expect(view.size).toBe(2);
  });

  it("does not draw a positioned entity that has no template name", () => {
    const view = new WorldView();
    view.applySnapshot({
      type: "snapshot",
      tick: 0,
      current: frame(0, [[1, 0, 0, 0]]),
      meta: delta([{ id: { index: 1, generation: 0 }, mobile: false }]),
    });
    expect(drawn(view)).toEqual([]);
    expect(view.size).toBe(0);
  });

  it("builds a snapshot row on demand, with velocity from the last two ticks", () => {
    const view = started();
    view.applyFrame({
      type: "frame",
      tick: 1,
      alpha: 0,
      current: frame(1, [[1, 0, 500, -250], [2, 0, 0, 0]]),
      meta: delta([meta(1, { move_target: { x: 4, y: 0 }, group: { index: 9, generation: 0 } })]),
      outcomes: [],
    });
    expect(view.get("1:0")).toEqual({
      id: "1:0",
      entityType: "mover",
      pos: { x: 0, y: 0 },
      velocity: { vx: 0.5, vy: -0.25 },
      faction: null,
      seats: null,
      aboard: null,
      boarding: null,
      group: "9:0",
      moveTarget: { x: 4, y: 0 },
    });
    expect(view.get("2:0")?.seats).toBe(4);
  });

  it("answers who rides and who walks to a vehicle without scanning the world", () => {
    const view = started();
    view.applyFrame({
      type: "frame",
      tick: 1,
      alpha: 0,
      current: frame(1, [[1, 0, 0, 0], [2, 0, 0, 0], [3, 0, 0, 0]]),
      meta: delta([
        meta(1, { aboard: { index: 2, generation: 0 } }),
        meta(3, { boarding: { index: 2, generation: 0 } }),
      ]),
      outcomes: [],
    });
    expect(view.ridersOf("2:0")).toEqual(["1:0"]);
    expect(view.walkersTo("2:0")).toEqual(["3:0"]);

    view.applyFrame({
      type: "frame",
      tick: 2,
      alpha: 0,
      current: frame(2, [[1, 0, 0, 0], [2, 0, 0, 0]]),
      meta: delta([meta(1)], [{ index: 3, generation: 0 }]),
      outcomes: [],
    });
    expect(view.ridersOf("2:0")).toEqual([]);
    expect(view.walkersTo("2:0")).toEqual([]);
  });

  it("moves the blend on between replies, but never past the current tick", () => {
    const view = started();
    view.applyFrame({
      type: "frame",
      tick: 1,
      alpha: 0.2,
      current: frame(1, [[1, 0, 10_000, 0], [2, 0, 0, 0]]),
      outcomes: [],
    });
    view.extrapolate(10); // 0.2 + 10 / 50
    expect(drawn(view)[0][1]).toBeCloseTo(4);
    view.extrapolate(500);
    expect(drawn(view)[0][1]).toBeLessThan(10);
    expect(drawn(view)[0][1]).toBeGreaterThan(9.9);

    // A reply resets it to the worker's value.
    view.applyFrame({ type: "frame", tick: 1, alpha: 0.5 });
    expect(drawn(view)[0][1]).toBeCloseTo(5);
  });

  it("reports whether a drawn unit is riding", () => {
    const view = started();
    view.applyFrame({
      type: "frame",
      tick: 1,
      alpha: 0,
      current: frame(1, [[1, 0, 0, 0], [2, 0, 0, 0]]),
      meta: delta([meta(1, { aboard: { index: 2, generation: 0 } })]),
      outcomes: [],
    });
    const riding: boolean[] = [];
    view.forEachDrawn((_i, _g, _x, _y, aboard) => riding.push(aboard));
    expect(riding).toEqual([true, false]);
  });

  it("finds the first entity matching a predicate", () => {
    const view = started();
    expect(view.find((e) => e.entityType === "truck")?.id).toBe("2:0");
    expect(view.find((e) => e.entityType === "base")).toBeNull();
  });
});
