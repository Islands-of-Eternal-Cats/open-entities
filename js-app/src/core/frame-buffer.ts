/**
 * Reading the core's position frames (`Simulation.writeFrame()`).
 *
 * A frame is one `Int32Array`: header `[tick_lo, tick_hi, count]`, then `count` rows of
 * `[index, generation, x, y]`, positions in milli-units, rows sorted by `index`. Ids are unsigned
 * in the core and bit-cast into the array, so they are read back with `>>> 0`.
 *
 * Nothing here allocates per entity: the renderer walks up to 100 000 rows every frame.
 */

/** Words before the first row: `[tick_lo, tick_hi, count]`. */
export const FRAME_HEADER_LEN = 3;
/** Words per row: `[index, generation, x, y]`. */
export const FRAME_STRIDE = 4;
/** Milli-units per map unit. */
export const MILLI_PER_UNIT = 1000;

export function frameTick(frame: Int32Array): number {
  return (frame[1] >>> 0) * 2 ** 32 + (frame[0] >>> 0);
}

export function frameCount(frame: Int32Array): number {
  return frame[2] >>> 0;
}

/** Called once per drawn entity, positions in map units. */
export type RowVisitor = (
  index: number,
  generation: number,
  x: number,
  y: number
) => void;

/**
 * Visits every entity of `current`, blended towards it from `previous` by `alpha`.
 *
 * One pass over both frames, merged on `index` (both are sorted by it). An entity found in both
 * with the same generation is interpolated; one that is new at the current tick is drawn where it
 * is; one that is only in `previous` was despawned and is not drawn. A matching index with another
 * generation is a different entity that reused the slot, so it is not blended either.
 */
export function interpolateFrames(
  previous: Int32Array | null,
  current: Int32Array,
  alpha: number,
  visit: RowVisitor
): void {
  const count = frameCount(current);
  const prevEnd = previous
    ? FRAME_HEADER_LEN + frameCount(previous) * FRAME_STRIDE
    : FRAME_HEADER_LEN;
  let p = FRAME_HEADER_LEN;
  for (let row = 0; row < count; row++) {
    const at = FRAME_HEADER_LEN + row * FRAME_STRIDE;
    const index = current[at] >>> 0;
    const generation = current[at + 1] >>> 0;
    let x = current[at + 2];
    let y = current[at + 3];
    if (previous) {
      while (p < prevEnd && previous[p] >>> 0 < index) p += FRAME_STRIDE;
      if (
        p < prevEnd &&
        previous[p] >>> 0 === index &&
        previous[p + 1] >>> 0 === generation
      ) {
        x = previous[p + 2] + (x - previous[p + 2]) * alpha;
        y = previous[p + 3] + (y - previous[p + 3]) * alpha;
      }
    }
    visit(index, generation, x / MILLI_PER_UNIT, y / MILLI_PER_UNIT);
  }
}

/** Offset of the row for this entity, or -1 when the frame does not hold it. Binary search. */
export function findRow(
  frame: Int32Array,
  index: number,
  generation: number
): number {
  let lo = 0;
  let hi = frameCount(frame) - 1;
  while (lo <= hi) {
    const mid = (lo + hi) >>> 1;
    const at = FRAME_HEADER_LEN + mid * FRAME_STRIDE;
    const found = frame[at] >>> 0;
    if (found === index) {
      return frame[at + 1] >>> 0 === generation ? at : -1;
    }
    if (found < index) lo = mid + 1;
    else hi = mid - 1;
  }
  return -1;
}
