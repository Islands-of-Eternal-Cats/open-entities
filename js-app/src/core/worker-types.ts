/**
 * Message types for main thread ↔ ECS web worker.
 */
import type { EntitySnapshot } from "./types";

export type WorkerInMessage =
  | { type: "init"; wasmBuffer?: ArrayBuffer }
  | { type: "tick"; dt: number }
  | { type: "spawn"; x: number; y: number; vx: number; vy: number };

export type WorkerOutMessage =
  | { type: "ready" }
  | { type: "error"; message: string }
  | { type: "entities"; entities: EntitySnapshot[] };
