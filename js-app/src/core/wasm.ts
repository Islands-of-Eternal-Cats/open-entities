/**
 * WASM core wrapper. Initializes ECS in a web worker and re-exports the game API.
 * Visualization layer depends only on this module and types from ./types.
 */
import type { EntitySnapshot } from "./types";
import type {
  RawEntitySnapshot,
  WorkerInMessage,
  WorkerOutMessage,
} from "./worker-types";
import { WORLD_SIZE } from "../visualization/coords";

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
  raw: RawEntitySnapshot[]
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
      const [wasmRes, entitiesYamlRes, initMapYamlRes] = await Promise.all([
        fetch(wasmUrl, { cache: "no-store" }),
        fetch(`${origin}/assets/entities.yaml`, { cache: "no-store" }),
        fetch(`${origin}/assets/init_map.yaml`, { cache: "no-store" }),
      ]);
      if (!wasmRes.ok) throw new Error(`Failed to fetch WASM: ${wasmRes.status}`);
      if (!entitiesYamlRes.ok) {
        throw new Error(
          `Failed to fetch entity definitions (assets/entities.yaml): HTTP ${entitiesYamlRes.status} ${entitiesYamlRes.statusText}`
        );
      }
      if (!initMapYamlRes.ok) {
        throw new Error(
          `Failed to fetch init map (assets/init_map.yaml): HTTP ${initMapYamlRes.status} ${initMapYamlRes.statusText}`
        );
      }
      const wasmBuffer = await wasmRes.arrayBuffer();
      const entitiesYaml = await entitiesYamlRes.text();
      const initMapYaml = await initMapYamlRes.text();

      await new Promise<void>((resolve, reject) => {
        initResolve = resolve;
        initReject = reject;
        worker!.postMessage(
          {
            type: "init",
            wasmBuffer,
            entitiesYaml,
            initMapYaml,
          } satisfies WorkerInMessage,
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
    // Spawn in random world coordinates across the full logical map bounds.
    const x = Math.random() * WORLD_SIZE;
    const y = Math.random() * WORLD_SIZE;
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

/**
 * Spawn by type at explicit world coordinates. Optional `faction` sets ECS `Faction` id.
 */
export function spawnAt(
  typeName: string,
  x: number,
  y: number,
  faction?: number
): Promise<EntitySnapshot[]> {
  if (!worker || !initialized)
    return Promise.reject(new Error("WASM not initialized"));
  return new Promise((resolve, reject) => {
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
