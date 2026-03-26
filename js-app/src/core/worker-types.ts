/**
 * Message types for main thread ↔ ECS web worker.
 */
import type { EntitySnapshot } from "./types";

export type WorkerInMessage =
  | { type: "init"; wasmBuffer: ArrayBuffer; entitiesYaml: string }
  | { type: "tick"; dt: number }
  | { type: "spawn"; typeName: string }
  | { type: "spawn_at"; typeName: string; x: number; y: number };

export type WorkerOutMessage =
  | { type: "ready" }
  | { type: "error"; message: string }
  | { type: "entities"; entities: EntitySnapshot[] };
