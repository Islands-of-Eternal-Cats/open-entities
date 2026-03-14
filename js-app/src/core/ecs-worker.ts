/**
 * ECS web worker: loads WASM and runs simulation.
 * Listens for init, tick, spawn; posts back ready, entities, error.
 */
import initWasmModule, { JsWorld } from "open-entities-wasm";
import type { WorkerInMessage, WorkerOutMessage } from "./worker-types";

const WASM_URL = "/wasm_bindings_bg.wasm";

let world: JsWorld | null = null;

function post(msg: WorkerOutMessage): void {
  self.postMessage(msg);
}

self.onmessage = async (event: MessageEvent<WorkerInMessage>) => {
  const msg = event.data;
  try {
    if (msg.type === "init") {
      await initWasmModule(WASM_URL);
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
      const entities = Array.from(raw).map(
        (e: { x: number; y: number; vx: number; vy: number }) => ({
          x: e.x,
          y: e.y,
          vx: e.vx,
          vy: e.vy,
        })
      );
      post({ type: "entities", entities });
      return;
    }

    if (msg.type === "spawn") {
      world.spawn(msg.x, msg.y, msg.vx, msg.vy);
      const raw = world.get_entities();
      const entities = Array.from(raw).map(
        (e: { x: number; y: number; vx: number; vy: number }) => ({
          x: e.x,
          y: e.y,
          vx: e.vx,
          vy: e.vy,
        })
      );
      post({ type: "entities", entities });
      return;
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    post({ type: "error", message });
  }
};
