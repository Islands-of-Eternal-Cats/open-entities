/**
 * ECS web worker: loads WASM and runs the simulation.
 * Listens for init/snapshot/frame/submit messages; posts back ready/entities/frame/submitted/error.
 *
 * Orders reach the core only as commands (`Simulation.submit`): each applies at the start of the
 * next tick, and its outcome rides back on the `frame` that ran that tick. Nothing here changes
 * the world between steps.
 *
 * The world crosses the boundary as JSON (`getWorldAsJson`), which this module adapts into the
 * flat `EntitySnapshot` rows the visualization layer expects.
 */
import initWasmModule, { Simulation, tickMs } from "open_entities_wasm";
import { FrameClock, TICK_MS } from "./fixed-step";
import type {
  CommandOutcome,
  EntitySnapshot,
  Pos,
  WorldExport,
  WorldExportRow,
} from "./types";
import { entityIdToKey, keyToEntityId } from "./types";
import type { WorkerInMessage, WorkerOutMessage } from "./worker-types";

let sim: Simulation | null = null;
let clock: FrameClock | null = null;
/** Outcomes of the steps run since the last `frame` reply. */
let outcomes: CommandOutcome[] = [];

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

function positionsOf(simulation: Simulation): Record<string, Pos> {
  const positions: Record<string, Pos> = {};
  for (const entity of readWorld(simulation)) positions[entity.id] = entity.pos;
  return positions;
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
        if (tickMs() !== TICK_MS) {
          throw new Error(
            `core tick is ${tickMs()} ms but the host clock uses ${TICK_MS} ms`
          );
        }
        const simulation = new Simulation();
        simulation.loadTemplatesYaml(msg.templatesYaml);
        simulation.loadMapYaml(msg.mapYaml);
        sim = simulation;
        clock = new FrameClock({
          step: () => {
            outcomes.push(...simulation.step().outcomes);
          },
          positions: () => positionsOf(simulation),
        });
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

    if (!sim || !clock) {
      post({ type: "error", message: "Worker not initialized" });
      return;
    }

    if (msg.type === "frame") {
      const { previous, alpha } = clock.frame(msg.elapsedMs);
      const applied = outcomes;
      outcomes = [];
      post({
        type: "frame",
        entities: readWorld(sim),
        previous,
        alpha,
        tick: sim.currentTick(),
        outcomes: applied,
      });
      return;
    }

    if (msg.type === "snapshot") {
      post(entitiesMessage(sim));
      return;
    }

    if (msg.type === "submit") {
      const simulation = sim;
      post({
        type: "submitted",
        seqs: msg.commands.map((command) => simulation.submit(command)),
      });
      return;
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    post({ type: "error", message });
  }
};
