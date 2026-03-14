/**
 * WASM core wrapper. Initializes ECS in a web worker and re-exports the game API.
 * Visualization layer depends only on this module and types from ./types.
 */
import type { EntitySnapshot } from "./types";
import type { WorkerInMessage, WorkerOutMessage } from "./worker-types";

let worker: Worker | null = null;
let initialized = false;
let pending:
  | {
      resolve: (value: EntitySnapshot[]) => void;
      reject: (reason: unknown) => void;
    }
  | null = null;

function rawToSnapshots(
  raw: Array<{
    pos: { x: number; y: number };
    velocity: { vx: number; vy: number };
  }>
): EntitySnapshot[] {
  return raw.map((e, i) => ({
    id: i,
    pos: e.pos,
    velocity: e.velocity,
  }));
}

function onMessage(event: MessageEvent<WorkerOutMessage>): void {
  const msg = event.data;
  if (msg.type === "ready") {
    initialized = true;
    return;
  }
  if (msg.type === "error") {
    if (pending) {
      pending.reject(new Error(msg.message));
      pending = null;
    }
    return;
  }
  if (msg.type === "entities" && pending) {
    pending.resolve(rawToSnapshots(msg.entities));
    pending = null;
  }
}

export async function initWasm(): Promise<void> {
  if (initialized) return;
  const workerUrl = new URL("./ecs-worker.ts", import.meta.url);
  worker = new Worker(workerUrl, { type: "module" });
  worker.onmessage = onMessage;
  worker.onerror = (e) => {
    if (pending) {
      pending.reject(e);
      pending = null;
    }
  };

  return new Promise<void>((resolve, reject) => {
    const onReady = () => {
      worker!.removeEventListener("message", handler);
      resolve();
    };
    const handler = (event: MessageEvent<WorkerOutMessage>) => {
      if (event.data.type === "ready") onReady();
      if (event.data.type === "error")
        reject(new Error((event.data as { message: string }).message));
    };
    worker!.addEventListener("message", handler);
    worker!.postMessage({ type: "init" } satisfies WorkerInMessage);
  });
}

export function isWasmReady(): boolean {
  return initialized && worker !== null;
}

/**
 * One simulation tick in the worker. Returns current entity snapshots.
 */
export function tick(dt: number): Promise<EntitySnapshot[]> {
  if (!worker || !initialized)
    return Promise.reject(new Error("WASM not initialized"));
  return new Promise((resolve, reject) => {
    if (pending) pending.reject(new Error("Concurrent request"));
    pending = { resolve, reject };
    worker!.postMessage({ type: "tick", dt } satisfies WorkerInMessage);
  });
}

/**
 * Spawn an entity in the worker. Returns current entity snapshots.
 */
export function spawn(
  x: number,
  y: number,
  vx: number,
  vy: number
): Promise<EntitySnapshot[]> {
  if (!worker || !initialized)
    return Promise.reject(new Error("WASM not initialized"));
  return new Promise((resolve, reject) => {
    if (pending) pending.reject(new Error("Concurrent request"));
    pending = { resolve, reject };
    worker!.postMessage({
      type: "spawn",
      x,
      y,
      vx,
      vy,
    } satisfies WorkerInMessage);
  });
}
