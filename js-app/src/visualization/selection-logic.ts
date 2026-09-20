/**
 * Pure helpers for temporary selection groups (marquee + point hit).
 */
import type { EntitySnapshot } from "../core/types";
import {
  ENTITY_RADIUS_PX,
  screenRectToWorldAabb,
  worldPosInAabb,
  worldToScreen,
} from "./coords";

/** Entity centers whose world position lies inside the world AABB from a screen marquee. */
export function entityIdsInScreenMarquee(
  entities: EntitySnapshot[],
  sx0: number,
  sy0: number,
  sx1: number,
  sy1: number
): string[] {
  const aabb = screenRectToWorldAabb(sx0, sy0, sx1, sy1);
  return entities
    .filter((e) => worldPosInAabb(e.pos.x, e.pos.y, aabb))
    .map((e) => e.id);
}

/** Last entity in array under point (later entries win if circles overlap). */
export function entityIdAtScreenPoint(
  entities: EntitySnapshot[],
  sx: number,
  sy: number
): string | null {
  let hit: string | null = null;
  const r2 = ENTITY_RADIUS_PX * ENTITY_RADIUS_PX;
  for (const e of entities) {
    const p = worldToScreen(e.pos.x, e.pos.y);
    const dx = sx - p.x;
    const dy = sy - p.y;
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
