/**
 * ECS web worker: loads WASM and runs simulation.
 * Listens for init/snapshot/tick/spawn_at/move_to; posts back ready/entities/error.
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

function spawnedFromWorld(
  world: JsWorld,
  beforeIds: ReadonlySet<string>
): WorkerOutMessage {
  const spawned = Array.from(world.get_entities()).find((entity) => {
    return !beforeIds.has(entity.id);
  });
  if (!spawned) {
    throw new Error("spawn_at succeeded but spawned entity was not found");
  }
  return {
    type: "spawned",
    entity: {
      id: spawned.id,
      entityType: spawned.entityType,
      pos: spawned.pos,
      velocity: spawned.velocity,
      faction: spawned.faction ?? null,
    },
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
      await initWasmModule(msg.wasmBuffer);
      try {
        world = new JsWorld(msg.entitiesYaml);
        world.load_map_yaml(msg.initMapYaml);
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

    if (msg.type === "snapshot") {
      post(snapshotFromWorld(world));
      return;
    }

    if (msg.type === "spawn_at") {
      try {
        const beforeIds = new Set(
          Array.from(world.get_entities()).map((entity) => entity.id)
        );
        world.spawn_at(msg.typeName, msg.x, msg.y, msg.faction);
        post(spawnedFromWorld(world, beforeIds));
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        post({ type: "error", message });
      }
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
