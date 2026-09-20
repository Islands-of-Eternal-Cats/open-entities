/**
 * App-level types that combine WASM types with visualization/UI state.
 */

/** 2D position. */
export type Pos = { x: number; y: number };

/** 2D velocity. */
export type Velocity = { vx: number; vy: number };

/**
 * One entity as the visualization layer wants it.
 *
 * Built from a row of `Simulation.getWorldAsJson()`; see `entityIdToKey` for what `id` is.
 */
export interface EntitySnapshot {
  /**
   * Stable entity key, `"<index>:<generation>"` from the export's `id` pair.
   * A string so it can key a Map or a DOM attribute; `keyToEntityId` turns it back.
   */
  id: string;
  /** Template name this entity was spawned from (export field `entity_type`). */
  entityType: string;
  pos: Pos;
  /** null when the entity has no `Velocity` component — in this engine, when it cannot move. */
  velocity: Velocity | null;
  /** ECS `Faction` id when present; null if the entity has no faction component. */
  faction: number | null;
  /** Seats from the `Boardable` component; null when this entity is not a vehicle. */
  seats: number | null;
  /**
   * Key of the vehicle this unit is riding, or null when it is on its own feet.
   *
   * A passenger's position is the vehicle's, so the two rows always report the same point.
   */
  aboard: string | null;
}

/** Entity identity as the WASM side spells it. */
export interface EntityId {
  index: number;
  generation: number;
}

/** One row of the world export (schema version 4); component keys are absent, never null. */
export interface WorldExportRow {
  id: EntityId;
  entity_type?: string;
  position?: Pos;
  velocity?: Velocity;
  faction?: number;
  base_move_speed?: number;
  move_target?: Pos;
  health?: { current: number; max: number };
  boardable?: number;
}

/** Payload of `Simulation.getWorldAsJson()`. */
export interface WorldExport {
  version: number;
  entities: WorldExportRow[];
}

/** Packs an id pair into the string key the visualization uses. */
export function entityIdToKey(id: EntityId): string {
  return `${id.index}:${id.generation}`;
}

/** Unpacks a key back into the id pair the WASM calls accept. */
export function keyToEntityId(key: string): EntityId {
  const [index, generation] = key.split(":");
  return { index: Number(index), generation: Number(generation) };
}
