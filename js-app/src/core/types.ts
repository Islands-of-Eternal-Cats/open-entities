/**
 * App-level types that combine WASM types with visualization/UI state.
 */

/** 2D position. */
export type Pos = { x: number; y: number };

/** 2D velocity. */
export type Velocity = { vx: number; vy: number };

/** Entity snapshot from WASM world.get_entities() for rendering. */
export interface EntitySnapshot {
  /** Stable entity id (string to preserve u64 precision; JS Number is only safe to 2^53-1). */
  id: string;
  pos: Pos;
  /** null for static entities (Position only); present for moving entities. */
  velocity: Velocity | null;
}
