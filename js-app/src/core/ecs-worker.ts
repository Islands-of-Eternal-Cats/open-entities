/**
 * ECS web worker: loads WASM and runs the simulation.
 * Listens for init/snapshot/tick/spawn_at/move_to/group and boarding messages; posts back
 * ready/entities/spawned/id/error.
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
    seats: row.boardable ?? null,
    aboard: null,
    boarding: null,
    moveTarget: row.move_target ? { ...row.move_target } : null,
  };
}

/**
 * Fills in who is riding what, and who is on the way to.
 *
 * `PassengerOf` and `BoardingTarget` are not exported components — they hold an entity, which
 * no YAML file has any business writing — so the links are read back per vehicle through
 * `passengers()` and `approaching()`. Vehicles are few, and this saves the main thread a round
 * trip per frame.
 */
function markPassengers(
  simulation: Simulation,
  snapshots: EntitySnapshot[]
): void {
  const byKey = new Map<string, EntitySnapshot>();
  for (const entity of snapshots) byKey.set(entity.id, entity);
  for (const vehicle of snapshots) {
    if (vehicle.seats === null) continue;
    const vehicleId = keyToEntityId(vehicle.id);
    for (const passenger of simulation.passengers(vehicleId)) {
      const rider = byKey.get(entityIdToKey(passenger));
      if (rider) rider.aboard = vehicle.id;
    }
    for (const walker of simulation.approaching(vehicleId)) {
      const unit = byKey.get(entityIdToKey(walker));
      if (unit) unit.boarding = vehicle.id;
    }
  }
}

function readWorld(simulation: Simulation): EntitySnapshot[] {
  const parsed = JSON.parse(simulation.getWorldAsJson()) as WorldExport;
  const snapshots: EntitySnapshot[] = [];
  for (const row of parsed.entities) {
    const snapshot = toSnapshot(row);
    if (snapshot) snapshots.push(snapshot);
  }
  markPassengers(simulation, snapshots);
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

    if (msg.type === "board") {
      try {
        sim.orderBoard(msg.units.map(keyToEntityId), keyToEntityId(msg.vehicle));
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        post({ type: "error", message });
        return;
      }
      post(entitiesMessage(sim));
      return;
    }

    if (msg.type === "unboard") {
      const refused: string[] = [];
      for (const key of msg.units) {
        try {
          sim.unboard(keyToEntityId(key));
        } catch (err) {
          refused.push(err instanceof Error ? err.message : String(err));
        }
      }
      // Whoever could step off did; the refusals say why the rest did not.
      if (refused.length > 0) {
        post({ type: "error", message: refused.join("; ") });
        return;
      }
      post(entitiesMessage(sim));
      return;
    }

    if (msg.type === "stop") {
      try {
        sim.orderStop(msg.entityIds.map(keyToEntityId));
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
