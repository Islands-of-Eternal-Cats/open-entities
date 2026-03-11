/**
 * App-level types that combine WASM types with visualization/UI state.
 */

/** Entity snapshot from WASM world.get_entities() for rendering. */
export interface EntitySnapshot {
  id: number;
  x: number;
  y: number;
  vx: number;
  vy: number;
}
