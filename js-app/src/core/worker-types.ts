/**
 * Message types for main thread ↔ ECS web worker.
 */
import type { EntitySnapshot, Pos, Velocity } from "./types";

export type WorkerInMessage =
  | { type: "init"; wasmBuffer?: ArrayBuffer }
  | { type: "tick"; dt: number }
  | { type: "spawn"; pos: Pos; velocity: Velocity | null };

export type WorkerOutMessage =
  | { type: "ready" }
  | { type: "error"; message: string }
  | { type: "entities"; entities: EntitySnapshot[] };
