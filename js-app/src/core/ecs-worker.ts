/**
 * ECS web worker: loads WASM and runs simulation.
 * Listens for init/tick/spawn_at/move_to; posts back ready/entities/error.
 */
import initWasmModule, { JsPosition, JsWorld } from "open-entities-wasm";
import type { WorkerInMessage, WorkerOutMessage } from "./worker-types";

function snapshotFromWorld(world: JsWorld): WorkerOutMessage {
  const entities = Array.from(world.get_entities()).map((e) => ({
    id: e.id,
    entityType: e.entityType,
    pos: e.pos,
    velocity: e.velocity,
    faction: e.faction ?? null,
  }));
  return { type: "entities", entities };
}

let world: JsWorld | null = null;

function post(msg: WorkerOutMessage): void {
  self.postMessage(msg);
}

self.onmessage = async (event: MessageEvent<WorkerInMessage>) => {
  const msg = event.data;
  try {
    if (msg.type === "init") {
      await initWasmModule(msg.wasmBuffer);
      try {
        world = new JsWorld(msg.entitiesYaml);
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
      post(snapshotFromWorld(world));
      return;
    }

    if (msg.type === "spawn_at") {
      try {
        world.spawn_at(msg.typeName, msg.x, msg.y, msg.faction);
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        post({ type: "error", message });
        return;
      }
      post(snapshotFromWorld(world));
      return;
    }

    if (msg.type === "move_to") {
      try {
        world.order_move_to(
          msg.entityIds,
          new JsPosition(msg.point.x, msg.point.y)
        );
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        post({ type: "error", message });
        return;
      }
      post(snapshotFromWorld(world));
      return;
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    post({ type: "error", message });
  }
};
