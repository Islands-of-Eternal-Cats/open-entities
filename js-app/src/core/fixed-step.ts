/**
 * Fixed-timestep host clock: turns variable frame times into whole simulation ticks.
 *
 * The core advances only in ticks of `TICK_MS`; the host accumulates real elapsed time and calls
 * `step()` once per whole tick. What is left over becomes `alpha`, how far the renderer is between
 * the last two ticks. The worker drives it (`worker-core.ts`); interpolation itself happens on the
 * main thread, over the position frames (`frame-buffer.ts`).
 */
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
