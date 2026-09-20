/**
 * Message types for main thread ↔ ECS web worker.
 */
import type { EntityId, EntitySnapshot } from "./types";

/**
 * Raw snapshot row as it crosses the worker boundary.
 * `faction` may be absent on malformed payloads, so the main thread normalizes it to null.
 */
export type RawEntitySnapshot = Omit<
  EntitySnapshot,
  "faction" | "seats" | "aboard"
> & {
  faction?: number | null;
  seats?: number | null;
  aboard?: string | null;
};

export type WorkerInMessage =
  | {
      type: "init";
      wasmBuffer: ArrayBuffer;
      /** Entity templates (YAML with an `entities:` root). */
      templatesYaml: string;
      /** Starting layout (YAML with `map` and `spawns`). */
      mapYaml: string;
    }
  | { type: "snapshot" }
  /** Delta in **seconds**; the worker converts to the integer milliseconds WASM takes. */
  | { type: "tick"; dt: number }
  | {
      type: "spawn_at";
      typeName: string;
      x: number;
      y: number;
      /** When set, the ECS `Faction` component is attached with this id. */
      faction?: number;
    }
  | { type: "move_to"; entityIds: string[]; point: { x: number; y: number } }
  | { type: "create_group"; faction: number }
  | { type: "add_to_group"; group: EntityId; entityIds: string[] }
  | { type: "group_move_to"; group: EntityId; point: { x: number; y: number } }
  /** Units climb aboard one vehicle; ones that are too far or find no seat are reported. */
  | { type: "board"; units: string[]; vehicle: string }
  | { type: "unboard"; units: string[] };

export type WorkerOutMessage =
  | { type: "ready" }
  | { type: "error"; message: string }
  | { type: "entities"; entities: RawEntitySnapshot[] }
  | { type: "spawned"; entity: RawEntitySnapshot }
  | { type: "id"; id: EntityId };
