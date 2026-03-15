/**
 * ECS web worker: loads WASM and runs simulation.
 * Listens for init, tick, spawn; posts back ready, entities, error.
 */
import initWasmModule, { JsWorld } from "open-entities-wasm";
import type { WorkerInMessage, WorkerOutMessage } from "./worker-types";

const OLD_WASM_HINT =
  "WASM returned old format (no pos/velocity). Run: cd js-app && ./build-wasm.sh then hard-reload (Ctrl+Shift+R).";

function toEntity(e: unknown): {
  id: string;
  pos: { x: number; y: number };
  velocity: { vx: number; vy: number } | null;
} {
  const o = e as {
    id?: string | number;
    pos?: { x: number; y: number };
    velocity?: { vx: number; vy: number } | null;
  };
  if (o.pos == null) {
    throw new Error(OLD_WASM_HINT);
  }
  const id =
    typeof o.id === "string" ? o.id : typeof o.id === "number" ? String(o.id) : "0";
  const velocity =
    o.velocity != null
      ? { vx: o.velocity.vx, vy: o.velocity.vy }
      : null;
  return {
    id,
    pos: { x: o.pos.x, y: o.pos.y },
    velocity,
  };
}

let world: JsWorld | null = null;

function post(msg: WorkerOutMessage): void {
  self.postMessage(msg);
}

self.onmessage = async (event: MessageEvent<WorkerInMessage>) => {
  const msg = event.data;
  try {
    if (msg.type === "init") {
      if (msg.wasmBuffer == null) {
        post({
          type: "error",
          message: "init requires wasmBuffer from main thread",
        });
        return;
      }
      await initWasmModule(msg.wasmBuffer);
      world = new JsWorld();
      post({ type: "ready" });
      return;
    }

    if (!world) {
      post({ type: "error", message: "Worker not initialized" });
      return;
    }

    if (msg.type === "tick") {
      world.tick(msg.dt);
      const raw = world.get_entities();
      const entities = Array.from(raw).map(toEntity);
      post({ type: "entities", entities });
      return;
    }

    if (msg.type === "spawn") {
      world.spawn(msg.x, msg.y, msg.vx, msg.vy);
      const raw = world.get_entities();
      const entities = Array.from(raw).map(toEntity);
      post({ type: "entities", entities });
      return;
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    post({ type: "error", message });
  }
};
