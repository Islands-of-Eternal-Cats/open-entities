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
/** Shared promise for init in progress; concurrent callers await this instead of creating new workers. */
let initPromise: Promise<void> | null = null;
/** Resolve/reject for the init promise; only set while waiting for worker "ready" or "error". */
let initResolve: (() => void) | null = null;
let initReject: ((reason: unknown) => void) | null = null;
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
    id: string;
    entityType: string;
    pos: { x: number; y: number };
    velocity: { vx: number; vy: number } | null;
    faction?: number | null;
  }>
): EntitySnapshot[] {
  return raw.map((e) => ({
    id: e.id,
    entityType: e.entityType,
    pos: e.pos,
    velocity: e.velocity,
    faction: e.faction ?? null,
  }));
}

function onMessage(event: MessageEvent<WorkerOutMessage>): void {
  const msg = event.data;
  if (msg.type === "ready") {
    initialized = true;
    if (initResolve) {
      initResolve();
      initResolve = null;
      initReject = null;
    }
    flushQueue();
    return;
  }
  if (msg.type === "error") {
    if (initReject) {
      initReject(new Error(msg.message));
      initResolve = null;
      initReject = null;
    }
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
  if (initPromise !== null) return initPromise;

  initPromise = (async () => {
    try {
      const workerUrl = new URL("./ecs-worker.ts", import.meta.url);
      worker = new Worker(workerUrl, { type: "module" });
      worker.onmessage = onMessage;
      worker.onerror = (e) => {
        if (initReject) {
          initReject(e);
          initResolve = null;
          initReject = null;
        }
        if (pending) {
          pending.reject(e);
          pending = null;
        }
        for (const q of requestQueue) q.reject(e);
        requestQueue.length = 0;
      };

      const origin =
        typeof window !== "undefined" && window.location
          ? window.location.origin
          : "http://localhost:5173";
      const wasmUrl = `${origin}/wasm_bindings_bg.wasm?t=${Date.now()}`;
      const [wasmRes, yamlRes] = await Promise.all([
        fetch(wasmUrl, { cache: "no-store" }),
        fetch(`${origin}/assets/entities.yaml`, { cache: "no-store" }),
      ]);
      if (!wasmRes.ok) throw new Error(`Failed to fetch WASM: ${wasmRes.status}`);
      if (!yamlRes.ok) {
        throw new Error(
          `Failed to fetch entity definitions (assets/entities.yaml): HTTP ${yamlRes.status} ${yamlRes.statusText}`
        );
      }
      const wasmBuffer = await wasmRes.arrayBuffer();
      const entitiesYaml = await yamlRes.text();

      await new Promise<void>((resolve, reject) => {
        initResolve = resolve;
        initReject = reject;
        worker!.postMessage(
          { type: "init", wasmBuffer, entitiesYaml } satisfies WorkerInMessage,
          [wasmBuffer]
        );
      });
    } catch (e) {
      initResolve = null;
      initReject = null;
      if (worker) {
        worker.terminate();
        worker = null;
      }
      throw e;
    } finally {
      initPromise = null;
    }
  })();

  return initPromise;
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
 * Spawn an entity in the worker by type name from `assets/entities.yaml`
 * using random coordinates.
 * Returns current entity snapshots.
 */
/**
 * Issue move-to-world-point for the given entity ids (snapshot id strings). Does not tick.
 */
export function moveSelectedTo(
  entityIds: string[],
  point: { x: number; y: number }
): Promise<EntitySnapshot[]> {
  if (!worker || !initialized)
    return Promise.reject(new Error("WASM not initialized"));
  if (entityIds.length === 0) {
    return Promise.reject(new Error("moveSelectedTo: no entity ids"));
  }
  return new Promise((resolve, reject) => {
    const message: WorkerInMessage = {
      type: "move_to",
      entityIds,
      point,
    };
    if (pending === null && requestQueue.length === 0) {
      pending = { resolve, reject };
      worker!.postMessage(message);
    } else {
      requestQueue.push({ resolve, reject, message });
    }
  });
}

/**
 * Spawn by type at random coordinates. Optional `faction` sets ECS `Faction` id.
 */
export function spawnRandomAt(
  typeName: string,
  faction?: number
): Promise<EntitySnapshot[]> {
  if (!worker || !initialized)
    return Promise.reject(new Error("WASM not initialized"));
  return new Promise((resolve, reject) => {
    // Spawn in random world coordinates (roughly matching visualization WORLD_SIZE).
    const x = Math.random() * 100;
    const y = Math.random() * 100;
    const message: WorkerInMessage = {
      type: "spawn_at",
      typeName,
      x,
      y,
      ...(faction !== undefined ? { faction } : {}),
    };
    if (pending === null && requestQueue.length === 0) {
      pending = { resolve, reject };
      worker!.postMessage(message);
    } else {
      requestQueue.push({ resolve, reject, message });
    }
  });
}
