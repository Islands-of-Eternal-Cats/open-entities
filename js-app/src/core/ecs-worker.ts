/**
 * ECS web worker: loads WASM and runs simulation.
 * Listens for init, tick, spawn; posts back ready, entities, error.
 */
import initWasmModule, { JsWorld } from "open-entities-wasm";
import type { EntitySnapshot } from "./types";
import type { WorkerInMessage, WorkerOutMessage } from "./worker-types";

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
      try {
        world = new JsWorld(msg.entitiesYaml ?? null);
      } catch (e) {
        post({
          type: "error",
          message: e instanceof Error ? e.message : String(e),
        });
        return;
      }
      post({ type: "ready" });
      return;
    }

    if (!world) {
      post({ type: "error", message: "Worker not initialized" });
      return;
    }

    if (msg.type === "tick") {
      world.tick(msg.dt);
      const entities: EntitySnapshot[] = Array.from(world.get_entities());
      post({ type: "entities", entities });
      return;
    }

    if (msg.type === "spawn") {
      try {
        world.spawn(msg.typeName);
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        post({ type: "error", message });
        return;
      }
      const entities: EntitySnapshot[] = Array.from(world.get_entities());
      post({ type: "entities", entities });
      return;
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    post({ type: "error", message });
  }
};
