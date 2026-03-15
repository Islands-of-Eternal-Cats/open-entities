/**
 * Message types for main thread ↔ ECS web worker.
 */

export type WorkerInMessage =
  | { type: "init"; wasmBuffer?: ArrayBuffer }
  | { type: "tick"; dt: number }
  | { type: "spawn"; x: number; y: number; vx: number; vy: number };

export type WorkerOutMessage =
  | { type: "ready" }
  | { type: "error"; message: string }
  | {
      type: "entities";
      entities: Array<{
        id: number;
        pos: { x: number; y: number };
        velocity: { vx: number; vy: number } | null;
      }>;
    };
