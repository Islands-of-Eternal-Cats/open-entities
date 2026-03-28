/**
 * Message types for main thread ↔ ECS web worker.
 */
import type { EntitySnapshot } from "./types";

export type WorkerInMessage =
  | { type: "init"; wasmBuffer: ArrayBuffer; entitiesYaml: string }
  | { type: "tick"; dt: number }
  | { type: "spawn_at"; typeName: string; x: number; y: number }
  | { type: "move_to"; entityIds: string[]; wx: number; wy: number };

export type WorkerOutMessage =
  | { type: "ready" }
  | { type: "error"; message: string }
  | { type: "entities"; entities: EntitySnapshot[] };
