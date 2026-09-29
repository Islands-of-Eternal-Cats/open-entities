/**
 * Message types for main thread ↔ ECS web worker.
 */
import type { EntityId, EntitySnapshot, Pos } from "./types";

/**
 * Raw snapshot row as it crosses the worker boundary.
 * `faction` may be absent on malformed payloads, so the main thread normalizes it to null.
 */
export type RawEntitySnapshot = Omit<
  EntitySnapshot,
  "faction" | "seats" | "aboard" | "boarding" | "moveTarget"
> & {
  faction?: number | null;
  seats?: number | null;
  aboard?: string | null;
  boarding?: string | null;
  moveTarget?: Pos | null;
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
  /**
   * Real time since the previous frame, in milliseconds. The worker accumulates it and runs as
   * many fixed ticks as fit; the reply is a `frame` message.
   */
  | { type: "frame"; elapsedMs: number }
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
  /**
   * Units are sent to walk to one vehicle and get in. This is an order, not an instant board: the
   * reply is the snapshot with `boarding` set on whoever took it.
   */
  | { type: "board"; units: string[]; vehicle: string }
  | { type: "unboard"; units: string[] }
  /** Zero velocity and drop any move target — works on anything that carries a velocity. */
  | { type: "stop"; entityIds: string[] };

export type WorkerOutMessage =
  | { type: "ready" }
  | { type: "error"; message: string }
  | { type: "entities"; entities: RawEntitySnapshot[] }
  | {
      type: "frame";
      /** State at the current tick. */
      entities: RawEntitySnapshot[];
      /** Positions at the tick before, keyed by entity id. */
      previous: Record<string, Pos>;
      /** How far the frame is between `previous` and `entities`, in [0, 1). */
      alpha: number;
      tick: number;
    }
  | { type: "spawned"; entity: RawEntitySnapshot }
  | { type: "id"; id: EntityId };
