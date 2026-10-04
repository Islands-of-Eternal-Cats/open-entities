/**
 * App-level types that combine WASM types with visualization/UI state.
 */

export type { Command, CommandOutcome, StepReport } from "open_entities_wasm";

/** 2D position. */
export type Pos = { x: number; y: number };

/** 2D velocity. */
export type Velocity = { vx: number; vy: number };

/**
 * One entity as the HUD wants it.
 *
 * Built on demand by `WorldView.get` from the position frames and the metadata, for the few
 * entities the HUD shows; the canvas reads the frames directly. See `entityIdToKey` for what `id`
 * is.
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
  /**
   * How far it moved over the last tick, in map units per tick; null when the entity has no
   * `Velocity` component — in this engine, when it cannot move.
   */
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
  /**
   * Key of the vehicle this unit is walking to board, or null when it has no such order.
   *
   * A standing order, not a place: the unit follows the vehicle if it drives off and climbs in
   * once within range. Cleared when it boards, finds no seat, or is told to do something else.
   */
  boarding: string | null;
  /** Key of the group it belongs to, or null. */
  group: string | null;
  /**
   * Where this entity has been told to go, or null when it holds no order.
   *
   * Straight from the metadata's `move_target`. Visible in the HUD on purpose: a target is the one
   * piece of unit state with no appearance on the map, so a unit standing on a stale one looks
   * identical to a unit standing still — until it walks off.
   */
  moveTarget: Pos | null;
}

/** Entity identity as the WASM side spells it. */
export interface EntityId {
  index: number;
  generation: number;
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
