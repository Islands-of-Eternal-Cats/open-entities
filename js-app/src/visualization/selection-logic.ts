/**
 * Pure helpers for temporary selection groups (marquee + point hit).
 */
import type { Pos } from "../core/types";
import {
  ENTITY_RADIUS_PX,
  screenRectToWorldAabb,
  screenToWorld,
  worldPosInAabb,
  worldToScreenTransform,
} from "./coords";

/** An entity as hit testing sees it: a key and a world position. */
export interface PlacedEntity {
  readonly id: string;
  readonly pos: Pos;
}

/** Entity centers whose world position lies inside the world AABB from a screen marquee. */
export function entityIdsInScreenMarquee(
  entities: Iterable<PlacedEntity>,
  sx0: number,
  sy0: number,
  sx1: number,
  sy1: number
): string[] {
  const aabb = screenRectToWorldAabb(sx0, sy0, sx1, sy1);
  const ids: string[] = [];
  for (const e of entities) {
    if (worldPosInAabb(e.pos.x, e.pos.y, aabb)) ids.push(e.id);
  }
  return ids;
}

/**
 * Last entity under the point (later entries win if circles overlap).
 *
 * Compared in world space — the point is converted once — so a hover over 100 000 units does not
 * convert 100 000 positions.
 */
export function entityIdAtScreenPoint(
  entities: Iterable<PlacedEntity>,
  sx: number,
  sy: number
): string | null {
  let hit: string | null = null;
  const point = screenToWorld(sx, sy);
  const radius = ENTITY_RADIUS_PX / worldToScreenTransform().scale;
  const r2 = radius * radius;
  for (const e of entities) {
    const dx = point.x - e.pos.x;
    const dy = point.y - e.pos.y;
    if (dx * dx + dy * dy <= r2) hit = e.id;
  }
  return hit;
}

type MoveOrderIntentArgs = {
  hitEntityId: string | null;
  selectedCount: number;
  shiftKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  metaKey: boolean;
};

/**
 * Move order should fire only on plain click/tap on empty ground
 * while there is an active selection.
 */
export function shouldIssueMoveOrder(args: MoveOrderIntentArgs): boolean {
  if (args.hitEntityId !== null) return false;
  if (args.selectedCount <= 0) return false;
  if (args.shiftKey || args.ctrlKey || args.altKey || args.metaKey) return false;
  return true;
}
