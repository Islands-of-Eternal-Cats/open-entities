/**
 * WASM core wrapper. Initializes ECS in a web worker and re-exports the game API.
 * Visualization layer depends only on this module and types from ./types.
 */
import type { EntityId, EntitySnapshot } from "./types";
import type {
  RawEntitySnapshot,
  WorkerInMessage,
  WorkerOutMessage,
} from "./worker-types";
import { WORLD_SIZE } from "../visualization/coords";

/** First four bytes of every wasm module: \0asm. */
const WASM_MAGIC = [0x00, 0x61, 0x73, 0x6d];

/**
 * Guard against the dev server answering with index.html instead of the module.
 *
 * `build-wasm.sh` copies the wasm-pack output into `public/`; when that step has not run, the
 * fetch still succeeds with HTML and the failure surfaces deep inside wasm-bindgen as an
 * unreadable "failed to match magic number".
 */
function assertLooksLikeWasm(buffer: ArrayBuffer): void {
  const head = new Uint8Array(buffer.slice(0, WASM_MAGIC.length));
  const ok =
    head.length === WASM_MAGIC.length &&
    WASM_MAGIC.every((byte, i) => head[i] === byte);
  if (!ok) {
    throw new Error(
      "open_entities_wasm_bg.wasm is missing or not a wasm module — the dev server answered " +
        "with something else. Run `npm run build:wasm` to build and copy it into public/."
    );
  }
}

function flushQueue(): void {
  if (!worker || pending !== null || requestQueue.length === 0) return;
  const next = requestQueue.shift()!;
  pending = next;
  worker.postMessage(next.message);
}

let worker: Worker | null = null;
let initialized = false;
/** Shared promise for init in progress; concurrent callers await this instead of creating new workers. */
let initPromise: Promise<void> | null = null;
/** Resolve/reject for the init promise; only set while waiting for worker "ready" or "error". */
let initResolve: (() => void) | null = null;
let initReject: ((reason: unknown) => void) | null = null;
type PendingRequest =
  | {
      resolve: (value: EntitySnapshot[]) => void;
      reject: (reason: unknown) => void;
      kind: "entities";
    }
  | {
      resolve: (value: EntitySnapshot) => void;
      reject: (reason: unknown) => void;
      kind: "spawned";
    }
  | {
      resolve: (value: EntityId) => void;
      reject: (reason: unknown) => void;
      kind: "id";
    };
let pending: PendingRequest | null = null;

type QueuedRequest =
  | {
      resolve: (value: EntitySnapshot[]) => void;
      reject: (reason: unknown) => void;
      message: WorkerInMessage;
      kind: "entities";
    }
  | {
      resolve: (value: EntitySnapshot) => void;
      reject: (reason: unknown) => void;
      message: WorkerInMessage;
      kind: "spawned";
    }
  | {
      resolve: (value: EntityId) => void;
      reject: (reason: unknown) => void;
      message: WorkerInMessage;
      kind: "id";
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
    seats: e.seats ?? null,
    aboard: e.aboard ?? null,
  }));
}

function rawToSnapshot(raw: RawEntitySnapshot): EntitySnapshot {
  return {
    id: raw.id,
    entityType: raw.entityType,
    pos: raw.pos,
    velocity: raw.velocity,
    faction: raw.faction ?? null,
    seats: raw.seats ?? null,
    aboard: raw.aboard ?? null,
  };
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
  if (msg.type === "entities" && pending && pending.kind === "entities") {
    pending.resolve(rawToSnapshots(msg.entities));
    pending = null;
    flushQueue();
    return;
  }
  if (msg.type === "spawned" && pending && pending.kind === "spawned") {
    pending.resolve(rawToSnapshot(msg.entity));
    pending = null;
    flushQueue();
    return;
  }
  if (msg.type === "id" && pending && pending.kind === "id") {
    pending.resolve(msg.id);
    pending = null;
    flushQueue();
  }
}

/** Sends a request now when the worker is free, or queues it behind the ones already waiting. */
function enqueue(
  message: WorkerInMessage,
  request: Omit<PendingRequest, "message">
): void {
  if (pending === null && requestQueue.length === 0) {
    pending = request as PendingRequest;
    worker!.postMessage(message);
    return;
  }
  requestQueue.push({ ...request, message } as QueuedRequest);
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
      const wasmUrl = `${origin}/open_entities_wasm_bg.wasm?t=${Date.now()}`;
      const [wasmRes, entitiesYamlRes, initMapYamlRes] = await Promise.all([
        fetch(wasmUrl, { cache: "no-store" }),
        fetch(`${origin}/fixtures/entities.yaml`, { cache: "no-store" }),
        fetch(`${origin}/fixtures/init_map.yaml`, { cache: "no-store" }),
      ]);
      if (!wasmRes.ok) throw new Error(`Failed to fetch WASM: ${wasmRes.status}`);
      if (!entitiesYamlRes.ok) {
        throw new Error(
          `Failed to fetch entity templates (fixtures/entities.yaml): HTTP ${entitiesYamlRes.status} ${entitiesYamlRes.statusText}`
        );
      }
      if (!initMapYamlRes.ok) {
        throw new Error(
          `Failed to fetch init map (fixtures/init_map.yaml): HTTP ${initMapYamlRes.status} ${initMapYamlRes.statusText}`
        );
      }
      const wasmBuffer = await wasmRes.arrayBuffer();
      assertLooksLikeWasm(wasmBuffer);
      const templatesYaml = await entitiesYamlRes.text();
      const mapYaml = await initMapYamlRes.text();

      await new Promise<void>((resolve, reject) => {
        initResolve = resolve;
        initReject = reject;
        worker!.postMessage(
          {
            type: "init",
            wasmBuffer,
            templatesYaml,
            mapYaml,
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
 * Read current world state without advancing simulation time.
 * Use this for initial UI sync or on-demand refreshes.
 */
export function snapshot(): Promise<EntitySnapshot[]> {
  if (!worker || !initialized)
    return Promise.reject(new Error("WASM not initialized"));
  return new Promise((resolve, reject) => {
    const message: WorkerInMessage = { type: "snapshot" };
    if (pending === null && requestQueue.length === 0) {
      pending = { resolve, reject, kind: "entities" };
      worker!.postMessage(message);
    } else {
      requestQueue.push({ resolve, reject, message, kind: "entities" });
    }
  });
}

/**
 * Advance simulation by `dt` seconds and return updated world snapshots.
 * The worker converts to the integer milliseconds the WASM tick takes.
 */
export function tick(dt: number): Promise<EntitySnapshot[]> {
  if (!worker || !initialized)
    return Promise.reject(new Error("WASM not initialized"));
  return new Promise((resolve, reject) => {
    const message: WorkerInMessage = { type: "tick", dt };
    if (pending === null && requestQueue.length === 0) {
      pending = { resolve, reject, kind: "entities" };
      worker!.postMessage(message);
    } else {
      requestQueue.push({ resolve, reject, message, kind: "entities" });
    }
  });
}

/**
 * Queue move-to order for the given entity ids (snapshot id strings).
 * Does not advance simulation; call `tick` to apply movement over time.
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
      pending = { resolve, reject, kind: "entities" };
      worker!.postMessage(message);
    } else {
      requestQueue.push({ resolve, reject, message, kind: "entities" });
    }
  });
}

/**
 * Forms a group for a faction and puts the given units in it.
 *
 * Resolves with the group id; keep it, every group call takes it.
 */
export function createGroupWith(
  faction: number,
  entityIds: string[]
): Promise<EntityId> {
  if (!worker || !initialized)
    return Promise.reject(new Error("WASM not initialized"));
  if (entityIds.length === 0) {
    return Promise.reject(new Error("createGroupWith: no entity ids"));
  }
  return new Promise<EntityId>((resolve, reject) => {
    enqueue(
      { type: "create_group", faction },
      { resolve, reject, kind: "id" }
    );
  }).then(
    (group) =>
      new Promise<EntityId>((resolve, reject) => {
        enqueue(
          { type: "add_to_group", group, entityIds },
          {
            resolve: () => resolve(group),
            reject,
            kind: "entities",
          }
        );
      })
  );
}

/**
 * Orders a whole group to a world point.
 *
 * Unlike `moveSelectedTo` this is a *group* order: members already following a personal order
 * keep it, and the group is left under manual control.
 */
export function orderGroupTo(
  group: EntityId,
  point: { x: number; y: number }
): Promise<EntitySnapshot[]> {
  if (!worker || !initialized)
    return Promise.reject(new Error("WASM not initialized"));
  return new Promise((resolve, reject) => {
    enqueue({ type: "group_move_to", group, point }, {
      resolve,
      reject,
      kind: "entities",
    } as PendingRequest);
  });
}

/**
 * Puts units aboard a vehicle they are standing next to.
 *
 * Rejects with every refusal joined together — too far away, no seats left — after boarding the
 * units that could go. Boarding is not a move order: walk them over first.
 */
export function boardUnits(
  units: string[],
  vehicle: string
): Promise<EntitySnapshot[]> {
  if (!worker || !initialized)
    return Promise.reject(new Error("WASM not initialized"));
  if (units.length === 0) {
    return Promise.reject(new Error("boardUnits: no entity ids"));
  }
  return new Promise((resolve, reject) => {
    enqueue({ type: "board", units, vehicle }, {
      resolve,
      reject,
      kind: "entities",
    } as PendingRequest);
  });
}

/** Lets passengers off, beside whatever they were riding. */
export function unboardUnits(units: string[]): Promise<EntitySnapshot[]> {
  if (!worker || !initialized)
    return Promise.reject(new Error("WASM not initialized"));
  if (units.length === 0) {
    return Promise.reject(new Error("unboardUnits: no entity ids"));
  }
  return new Promise((resolve, reject) => {
    enqueue({ type: "unboard", units }, {
      resolve,
      reject,
      kind: "entities",
    } as PendingRequest);
  });
}

/**
 * Spawn by type at random coordinates.
 * Returns only the newly spawned entity (not a full world snapshot).
 * Optional `faction` sets ECS `Faction` id.
 */
export function spawnRandomAt(
  typeName: string,
  faction?: number
): Promise<EntitySnapshot> {
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
      pending = { resolve, reject, kind: "spawned" };
      worker!.postMessage(message);
    } else {
      requestQueue.push({ resolve, reject, message, kind: "spawned" });
    }
  });
}

/**
 * Spawn by type at explicit world coordinates.
 * Returns only the newly spawned entity (not a full world snapshot).
 * Optional `faction` sets ECS `Faction` id.
 */
export function spawnAt(
  typeName: string,
  x: number,
  y: number,
  faction?: number
): Promise<EntitySnapshot> {
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
      pending = { resolve, reject, kind: "spawned" };
      worker!.postMessage(message);
    } else {
      requestQueue.push({ resolve, reject, message, kind: "spawned" });
    }
  });
}
