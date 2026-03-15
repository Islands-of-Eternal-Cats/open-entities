/**
 * WASM core wrapper. Initializes ECS in a web worker and re-exports the game API.
 * Visualization layer depends only on this module and types from ./types.
 */
import type { EntitySnapshot } from "./types";
import type { WorkerInMessage, WorkerOutMessage } from "./worker-types";

function flushQueue(): void {
  if (!worker || pending !== null || requestQueue.length === 0) return;
  const next = requestQueue.shift()!;
  pending = { resolve: next.resolve, reject: next.reject };
  worker.postMessage(next.message);
}

let worker: Worker | null = null;
let initialized = false;
let pending:
  | {
      resolve: (value: EntitySnapshot[]) => void;
      reject: (reason: unknown) => void;
    }
  | null = null;

type QueuedRequest = {
  resolve: (value: EntitySnapshot[]) => void;
  reject: (reason: unknown) => void;
  message: WorkerInMessage;
};
const requestQueue: QueuedRequest[] = [];

function rawToSnapshots(
  raw: Array<{
    id: number;
    pos: { x: number; y: number };
    velocity: { vx: number; vy: number };
  }>
): EntitySnapshot[] {
  return raw.map((e) => ({
    id: e.id,
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
      flushQueue();
    }
    return;
  }
  if (msg.type === "entities" && pending) {
    pending.resolve(rawToSnapshots(msg.entities));
    pending = null;
    flushQueue();
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

  const origin =
    typeof window !== "undefined" && window.location
      ? window.location.origin
      : "http://localhost:5173";
  const wasmUrl = `${origin}/wasm_bindings_bg.wasm?t=${Date.now()}`;
  const res = await fetch(wasmUrl, { cache: "no-store" });
  if (!res.ok) throw new Error(`Failed to fetch WASM: ${res.status}`);
  const wasmBuffer = await res.arrayBuffer();

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
    worker!.postMessage({ type: "init", wasmBuffer } satisfies WorkerInMessage, [
      wasmBuffer,
    ]);
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
    const message: WorkerInMessage = { type: "tick", dt };
    if (pending === null && requestQueue.length === 0) {
      pending = { resolve, reject };
      worker!.postMessage(message);
    } else {
      requestQueue.push({ resolve, reject, message });
    }
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
    const message: WorkerInMessage = {
      type: "spawn",
      x,
      y,
      vx,
      vy,
    };
    if (pending === null && requestQueue.length === 0) {
      pending = { resolve, reject };
      worker!.postMessage(message);
    } else {
      requestQueue.push({ resolve, reject, message });
    }
  });
}
