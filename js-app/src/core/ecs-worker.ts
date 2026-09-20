/**
 * ECS web worker: loads WASM and runs the simulation.
 * Listens for init/snapshot/tick/spawn_at/move_to; posts back ready/entities/error.
 *
 * The world crosses the boundary as JSON (`getWorldAsJson`), which this module adapts into the
 * flat `EntitySnapshot` rows the visualization layer expects.
 */
import initWasmModule, { Simulation } from "open_entities_wasm";
import type { EntitySnapshot, WorldExport, WorldExportRow } from "./types";
import { entityIdToKey, keyToEntityId } from "./types";
import type { WorkerInMessage, WorkerOutMessage } from "./worker-types";

/** Smallest tick the WASM side accepts is 1 ms; it rejects zero. */
const MIN_TICK_MS = 1;

let sim: Simulation | null = null;

function post(msg: WorkerOutMessage): void {
  self.postMessage(msg);
}

/**
 * One export row as the visualization wants it, or null when the row is not a gameplay unit.
 *
 * Rows without `entity_type` were not spawned from a template (they are raw ECS entities), and
 * rows without a position have nothing to draw.
 */
function toSnapshot(row: WorldExportRow): EntitySnapshot | null {
  if (row.entity_type === undefined || row.position === undefined) return null;
  return {
    id: entityIdToKey(row.id),
    entityType: row.entity_type,
    pos: { x: row.position.x, y: row.position.y },
    velocity: row.velocity ? { ...row.velocity } : null,
    faction: row.faction ?? null,
  };
}

function readWorld(simulation: Simulation): EntitySnapshot[] {
  const parsed = JSON.parse(simulation.getWorldAsJson()) as WorldExport;
  const snapshots: EntitySnapshot[] = [];
  for (const row of parsed.entities) {
    const snapshot = toSnapshot(row);
    if (snapshot) snapshots.push(snapshot);
  }
  return snapshots;
}

function entitiesMessage(simulation: Simulation): WorkerOutMessage {
  return { type: "entities", entities: readWorld(simulation) };
}

self.onmessage = async (event: MessageEvent<WorkerInMessage>) => {
  const msg = event.data;
  try {
    if (msg.type === "init") {
      await initWasmModule({ module_or_path: msg.wasmBuffer });
      try {
        const simulation = new Simulation();
        simulation.loadTemplatesYaml(msg.templatesYaml);
        simulation.loadMapYaml(msg.mapYaml);
        sim = simulation;
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

    if (!sim) {
      post({ type: "error", message: "Worker not initialized" });
      return;
    }

    if (msg.type === "tick") {
      const dtMs = Math.max(MIN_TICK_MS, Math.round(msg.dt * 1000));
      sim.tick(dtMs);
      post(entitiesMessage(sim));
      return;
    }

    if (msg.type === "snapshot") {
      post(entitiesMessage(sim));
      return;
    }

    if (msg.type === "spawn_at") {
      try {
        const overrides: Record<string, unknown> = {
          position: { x: msg.x, y: msg.y },
        };
        if (msg.faction !== undefined) overrides.faction = msg.faction;
        const id = sim.spawnEntity(msg.typeName, overrides);
        const key = entityIdToKey(id);
        const spawned = readWorld(sim).find((entity) => entity.id === key);
        if (!spawned) {
          throw new Error(
            `spawned ${msg.typeName} but it is missing from the world export`
          );
        }
        post({ type: "spawned", entity: spawned });
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        post({ type: "error", message });
      }
      return;
    }

    if (msg.type === "create_group") {
      try {
        post({ type: "id", id: sim.createGroup(msg.faction) });
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        post({ type: "error", message });
      }
      return;
    }

    if (msg.type === "add_to_group") {
      try {
        for (const key of msg.entityIds) {
          sim.addToGroup(msg.group, keyToEntityId(key));
        }
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        post({ type: "error", message });
        return;
      }
      post(entitiesMessage(sim));
      return;
    }

    if (msg.type === "group_move_to") {
      try {
        sim.orderGroupMoveTo(msg.group, msg.point.x, msg.point.y);
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        post({ type: "error", message });
        return;
      }
      post(entitiesMessage(sim));
      return;
    }

    if (msg.type === "move_to") {
      try {
        sim.orderMoveTo(
          msg.entityIds.map(keyToEntityId),
          msg.point.x,
          msg.point.y
        );
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        post({ type: "error", message });
        return;
      }
      post(entitiesMessage(sim));
      return;
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    post({ type: "error", message });
  }
};
