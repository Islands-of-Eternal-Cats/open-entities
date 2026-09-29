/**
 * Fixed-timestep host clock: turns variable frame times into whole simulation ticks.
 *
 * The core advances only in ticks of `TICK_MS`; the host accumulates real elapsed time and calls
 * `step()` once per whole tick. What is left over becomes `alpha`, how far the renderer is between
 * the last two ticks.
 */
import type { EntitySnapshot, Pos } from "./types";

/** Length of one simulation tick in milliseconds; must match the core's `tickMs()`. */
export const TICK_MS = 50;

/**
 * Most ticks run in one frame. After a long stall (tab in background, debugger) the excess time
 * is dropped instead of being caught up, which would make each frame slower still.
 */
export const MAX_STEPS_PER_FRAME = 5;

/** Result of feeding one frame's elapsed time into the accumulator. */
export interface ClockAdvance {
  /** Ticks to run this frame. */
  steps: number;
  /** Milliseconds carried to the next frame, always below `TICK_MS`. */
  accumulator: number;
  /** `accumulator / TICK_MS`, in [0, 1). */
  alpha: number;
}

export function advanceClock(
  accumulator: number,
  elapsedMs: number
): ClockAdvance {
  let acc = accumulator + Math.max(0, elapsedMs);
  let steps = Math.floor(acc / TICK_MS);
  if (steps > MAX_STEPS_PER_FRAME) {
    steps = MAX_STEPS_PER_FRAME;
    acc = acc % TICK_MS;
  } else {
    acc -= steps * TICK_MS;
  }
  return { steps, accumulator: acc, alpha: acc / TICK_MS };
}

/** What `FrameClock` drives: the simulation's step and a read of its positions. */
export interface SteppedSimulation {
  step(): void;
  positions(): Record<string, Pos>;
}

/** One frame's interpolation input. */
export interface FrameTiming {
  /** Positions at tick t−1, where t is the current tick. */
  previous: Record<string, Pos>;
  alpha: number;
}

/**
 * Accumulator plus the positions of the tick before the current one.
 *
 * When a frame runs several steps, `previous` is read just before the last of them, so it is
 * always tick t−1 and never the state at the start of the frame.
 */
export class FrameClock {
  private accumulator = 0;
  private previous: Record<string, Pos> = {};

  constructor(private readonly sim: SteppedSimulation) {}

  frame(elapsedMs: number): FrameTiming {
    const advance = advanceClock(this.accumulator, elapsedMs);
    this.accumulator = advance.accumulator;
    for (let i = 0; i < advance.steps; i++) {
      if (i === advance.steps - 1) this.previous = this.sim.positions();
      this.sim.step();
    }
    return { previous: this.previous, alpha: advance.alpha };
  }
}

/** Positions blended between tick t−1 and tick t; entities new at tick t are drawn where they are. */
export function interpolate(
  previous: Record<string, Pos>,
  current: EntitySnapshot[],
  alpha: number
): EntitySnapshot[] {
  return current.map((entity) => {
    const from = previous[entity.id];
    if (!from) return entity;
    return {
      ...entity,
      pos: {
        x: from.x + (entity.pos.x - from.x) * alpha,
        y: from.y + (entity.pos.y - from.y) * alpha,
      },
    };
  });
}
