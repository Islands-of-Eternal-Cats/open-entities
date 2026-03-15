/**
 * App-level types that combine WASM types with visualization/UI state.
 */

/** Entity snapshot from WASM world.get_entities() for rendering. */
export interface EntitySnapshot {
  /** Stable entity id (string to preserve u64 precision; JS Number is only safe to 2^53-1). */
  id: string;
  pos: { x: number; y: number };
  /** null for static entities (Position only); present for moving entities. */
  velocity: { vx: number; vy: number } | null;
}
