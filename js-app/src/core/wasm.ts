/**
 * WASM core wrapper. Initializes ECS in a web worker and re-exports the game API.
 * Visualization layer depends only on this module and types from ./types.
 *
 * The world lives in `world`, a `WorldView` fed by the worker's replies: position frames as
 * transferred `Int32Array` buffers, metadata deltas when something changed. Nothing on this path
 * parses a world export.
 *
 * Orders are commands: each is submitted to the core for the next tick, and its promise settles
 * when a later `frame` reports the command's outcome. Order replies carry no snapshot; the world
 * is drawn from frames only.
 */
import type { Command, CommandOutcome, EntityId, EntitySnapshot } from "./types";
import { entityIdToKey, keyToEntityId } from "./types";
import type { WorkerInMessage, WorkerOutMessage } from "./worker-types";
import { WorldView } from "./world-view";
import { WORLD_SIZE } from "../visualization/coords";

/** The main thread's copy of the world, updated by every `snapshot()` and `frame()` reply. */
export const world = new WorldView();

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

/**
 * Short fingerprint of the bytes the browser actually loaded.
 *
 * FNV-1a over the whole module. Not a security hash — its job is to answer "is the core running
 * here the one I just built?", which cost an evening of debugging a fix that was never in the
 * build. Shown in the status line; compare two reloads, and identical means nothing rebuilt.
 */
function fingerprint(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  let hash = 0x811c9dc5;
  for (let i = 0; i < bytes.length; i++) {
    hash ^= bytes[i];
    // FNV prime, via shifts: Math.imul keeps this in 32-bit territory.
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash.toString(16).padStart(8, "0");
}

/** Fingerprint and size of the loaded module, once `initWasm` has fetched it. */
let coreBuild: { id: string; bytes: number } | null = null;

/** What the status line prints, or null before the module is loaded. */
export function coreBuildInfo(): { id: string; bytes: number } | null {
  return coreBuild;
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
      resolve: (value: void) => void;
      reject: (reason: unknown) => void;
      kind: "snapshot";
    }
  | {
      resolve: (value: number) => void;
      reject: (reason: unknown) => void;
      kind: "stressed";
    }
  | {
      resolve: (value: number[]) => void;
      reject: (reason: unknown) => void;
      kind: "submitted";
    }
  | {
      resolve: (value: FrameResult) => void;
      reject: (reason: unknown) => void;
      kind: "frame";
    };
let pending: PendingRequest | null = null;

type QueuedRequest = PendingRequest & { message: WorkerInMessage };
const requestQueue: QueuedRequest[] = [];

/**
 * A submitted command waiting for the frame that reports what it did. Called after that frame is
 * in `world`, so a waiter can read the state the command produced.
 */
type OutcomeWaiter = (outcome: CommandOutcome) => void;
/** Keyed by sequence number; a frame settles and removes the ones it reports. */
const outcomeWaiters = new Map<number, OutcomeWaiter>();

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
  if (msg.type === "snapshot" && pending && pending.kind === "snapshot") {
    world.applySnapshot(msg);
    pending.resolve();
    pending = null;
    flushQueue();
    return;
  }
  if (msg.type === "frame" && pending && pending.kind === "frame") {
    world.applyFrame(msg);
    for (const outcome of msg.outcomes ?? []) {
      const waiter = outcomeWaiters.get(outcome.seq);
      if (!waiter) continue;
      outcomeWaiters.delete(outcome.seq);
      waiter(outcome);
    }
    pending.resolve({
      tick: msg.tick,
      alpha: msg.alpha,
      stepped: msg.current !== undefined,
    });
    pending = null;
    flushQueue();
    return;
  }
  if (msg.type === "stressed" && pending && pending.kind === "stressed") {
    pending.resolve(msg.count);
    pending = null;
    flushQueue();
    return;
  }
  if (msg.type === "submitted" && pending && pending.kind === "submitted") {
    pending.resolve(msg.seqs);
    pending = null;
    flushQueue();
  }
}

/** Sends a request now when the worker is free, or queues it behind the ones already waiting. */
function enqueue(message: WorkerInMessage, request: PendingRequest): void {
  if (pending === null && requestQueue.length === 0) {
    pending = request;
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
        outcomeWaiters.clear();
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
      // Before the buffer is transferred to the worker, which empties it on this side.
      coreBuild = { id: fingerprint(wasmBuffer), bytes: wasmBuffer.byteLength };
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
 * Loads the current tick and every entity's metadata into `world`, without advancing time.
 * Call once at start, before the first `frame`.
 */
export function snapshot(): Promise<void> {
  if (!worker || !initialized)
    return Promise.reject(new Error("WASM not initialized"));
  return new Promise((resolve, reject) => {
    enqueue({ type: "snapshot" }, { resolve, reject, kind: "snapshot" });
  });
}

/** What one frame did; the state itself is in `world`. */
export interface FrameResult {
  tick: number;
  /** Blend factor between the last two ticks, in [0, 1). */
  alpha: number;
  /** True when the frame ran at least one step and brought new buffers. */
  stepped: boolean;
}

/**
 * Report `elapsedMs` of real time. The worker runs as many fixed ticks as fit (possibly none);
 * the reply updates `world` before the promise resolves.
 */
export function frame(elapsedMs: number): Promise<FrameResult> {
  if (!worker || !initialized)
    return Promise.reject(new Error("WASM not initialized"));
  return new Promise((resolve, reject) => {
    enqueue({ type: "frame", elapsedMs }, { resolve, reject, kind: "frame" });
  });
}

/** What a command did once it applied; `world` holds the frame it applied in. */
interface Applied {
  outcome: CommandOutcome & { ok: true };
}

/**
 * Submits commands for the next tick. Each promise settles from the `frame` whose step applied
 * that command: resolved with the outcome, or rejected with the core's reason when it was refused.
 */
function submitAll(commands: Command[]): Promise<Applied>[] {
  if (!worker || !initialized) {
    const error = Promise.reject(new Error("WASM not initialized"));
    return commands.map(() => error);
  }
  const seqs = new Promise<number[]>((resolve, reject) => {
    enqueue({ type: "submit", commands }, { resolve, reject, kind: "submitted" });
  });
  return commands.map(
    (_, i) =>
      new Promise<Applied>((resolve, reject) => {
        seqs.then((all) => {
          outcomeWaiters.set(all[i], (outcome) => {
            if (outcome.ok) resolve({ outcome });
            else reject(new Error(outcome.error));
          });
        }, reject);
      })
  );
}

function submit(command: Command): Promise<Applied> {
  return submitAll([command])[0];
}

/** Resolves with the outcome alone: what an order reply carries now that it has no snapshot. */
async function order(command: Command): Promise<CommandOutcome> {
  return (await submit(command)).outcome;
}

/**
 * Move order for the given entity ids (snapshot id strings).
 *
 * Applies at the start of the next tick; resolves with the outcome (`applied`/`skipped`) once a
 * frame has run that tick. Movement shows up in frames, as always.
 */
export function moveSelectedTo(
  entityIds: string[],
  point: { x: number; y: number }
): Promise<CommandOutcome> {
  if (entityIds.length === 0) {
    return Promise.reject(new Error("moveSelectedTo: no entity ids"));
  }
  return order({
    type: "move_to",
    ids: entityIds.map(keyToEntityId),
    target: { x: point.x, y: point.y },
  });
}

/**
 * Forms a group for a faction and puts the given units in it.
 *
 * Two rounds through the core: the group has no id until its `create_group` applies, and only
 * then can the units be added. Resolves with the group id; keep it, every group call takes it.
 * Rejects if any unit is refused (another faction, say).
 */
export async function createGroupWith(
  faction: number,
  entityIds: string[]
): Promise<EntityId> {
  if (entityIds.length === 0) {
    throw new Error("createGroupWith: no entity ids");
  }
  const created = await submit({ type: "create_group", faction });
  const group = created.outcome.group;
  if (!group) throw new Error("create_group reported no group id");
  await Promise.all(
    submitAll(
      entityIds.map((key) => ({
        type: "add_to_group" as const,
        group,
        unit: keyToEntityId(key),
      }))
    )
  );
  return group;
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
): Promise<CommandOutcome> {
  return order({
    type: "group_move_to",
    group,
    target: { x: point.x, y: point.y },
  });
}

/**
 * Stops the given entities where they are: velocity to zero, move target dropped.
 *
 * Needed as its own order because a vehicle that keeps driving after its passengers step off
 * leaves them behind, and nothing else releases a move target early.
 */
export function stopSelected(entityIds: string[]): Promise<CommandOutcome> {
  if (entityIds.length === 0) {
    return Promise.reject(new Error("stopSelected: no entity ids"));
  }
  return order({ type: "stop", ids: entityIds.map(keyToEntityId) });
}

/**
 * Sends units to walk to a vehicle and get in.
 *
 * An order, not an instant board: the core walks each unit over tick by tick, following the
 * vehicle if it moves, and boards it once within range; frames show `boarding` set on the units
 * that took it. Rejects only when the vehicle has no seats at all.
 */
export function boardUnits(
  units: string[],
  vehicle: string
): Promise<CommandOutcome> {
  if (units.length === 0) {
    return Promise.reject(new Error("boardUnits: no entity ids"));
  }
  return order({
    type: "board",
    units: units.map(keyToEntityId),
    vehicle: keyToEntityId(vehicle),
  });
}

/**
 * Lets passengers off, beside whatever they were riding. `skipped` counts the units that were not
 * aboard anything.
 */
export function unboardUnits(units: string[]): Promise<CommandOutcome> {
  if (units.length === 0) {
    return Promise.reject(new Error("unboardUnits: no entity ids"));
  }
  return order({ type: "unboard", units: units.map(keyToEntityId) });
}

/**
 * Spawn by type at random coordinates.
 * Resolves with the new entity as the frame that spawned it reports it.
 * Optional `faction` sets ECS `Faction` id.
 */
export function spawnRandomAt(
  typeName: string,
  faction?: number
): Promise<EntitySnapshot> {
  // Spawn in random world coordinates across the full logical map bounds.
  const x = Math.random() * WORLD_SIZE;
  const y = Math.random() * WORLD_SIZE;
  return spawnAt(typeName, x, y, faction);
}

/**
 * Spawn by type at explicit world coordinates.
 * Resolves with the new entity as the frame that spawned it reports it.
 * Optional `faction` sets ECS `Faction` id.
 */
export async function spawnAt(
  typeName: string,
  x: number,
  y: number,
  faction?: number
): Promise<EntitySnapshot> {
  const overrides: Record<string, unknown> = { position: { x, y } };
  if (faction !== undefined) overrides.faction = faction;
  const { outcome } = await submit({
    type: "spawn",
    template: typeName,
    overrides,
  });
  const spawned = outcome.spawned ? world.get(entityIdToKey(outcome.spawned)) : null;
  if (!spawned) {
    throw new Error(`spawned ${typeName} but it is missing from the frame`);
  }
  return spawned;
}

/**
 * Stress test: spawns `count` seeded movers (template `stress_mover`) with random targets across
 * the map, as spawn commands generated in the worker. Resolves with how many were queued; they
 * appear in the frame that runs the next tick.
 */
export function stress(count: number, seed = 1): Promise<number> {
  if (!worker || !initialized)
    return Promise.reject(new Error("WASM not initialized"));
  return new Promise((resolve, reject) => {
    enqueue(
      { type: "stress", count, seed, worldSize: WORLD_SIZE },
      { resolve, reject, kind: "stressed" }
    );
  });
}
