/**
 * App-level types that combine WASM types with visualization/UI state.
 */

/** Entity snapshot from WASM world.get_entities() for rendering. */
export interface EntitySnapshot {
  id: number;
  pos: { x: number; y: number };
  /** null for static entities (Position only); present for moving entities. */
  velocity: { vx: number; vy: number } | null;
}
