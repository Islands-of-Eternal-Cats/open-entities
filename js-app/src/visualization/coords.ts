/**
 * Shared world ↔ canvas mapping for Pixi visualization.
 * Must stay in sync with canvas dimensions used by the Pixi Application.
 */

export const CANVAS_WIDTH = 640;
export const CANVAS_HEIGHT = 480;
/** World extent used for scaling (entities typically live in ~0..100). */
export const WORLD_SIZE = 120;
export const ENTITY_RADIUS_PX = 8;

export function getScaleAndOffset(): {
  scale: number;
  offsetX: number;
  offsetY: number;
} {
  const scale = Math.min(CANVAS_WIDTH, CANVAS_HEIGHT) / WORLD_SIZE;
  const offsetX = (CANVAS_WIDTH - WORLD_SIZE * scale) / 2;
  const offsetY = (CANVAS_HEIGHT - WORLD_SIZE * scale) / 2;
  return { scale, offsetX, offsetY };
}

export function worldToScreen(
  x: number,
  y: number
): { x: number; y: number } {
  const { scale, offsetX, offsetY } = getScaleAndOffset();
  return {
    x: offsetX + x * scale,
    y: offsetY + y * scale,
  };
}

export function screenToWorld(
  sx: number,
  sy: number
): { x: number; y: number } {
  const { scale, offsetX, offsetY } = getScaleAndOffset();
  return {
    x: (sx - offsetX) / scale,
    y: (sy - offsetY) / scale,
  };
}

/** Axis-aligned world bounds from two canvas-space corners of a selection rectangle. */
export function screenRectToWorldAabb(
  sx0: number,
  sy0: number,
  sx1: number,
  sy1: number
): { minX: number; maxX: number; minY: number; maxY: number } {
  const w0 = screenToWorld(sx0, sy0);
  const w1 = screenToWorld(sx1, sy1);
  return {
    minX: Math.min(w0.x, w1.x),
    maxX: Math.max(w0.x, w1.x),
    minY: Math.min(w0.y, w1.y),
    maxY: Math.max(w0.y, w1.y),
  };
}

export function worldPosInAabb(
  wx: number,
  wy: number,
  aabb: { minX: number; maxX: number; minY: number; maxY: number }
): boolean {
  return wx >= aabb.minX && wx <= aabb.maxX && wy >= aabb.minY && wy <= aabb.maxY;
}

/**
 * DOM client coordinates → Pixi **screen** space (same as `worldToScreen` / DisplayObject x,y).
 *
 * With `resolution` + `autoDensity`, `canvas.width` is buffer pixels (× DPR), but the stage uses
 * logical size `CANVAS_WIDTH` × `CANVAS_HEIGHT`. Mapping via buffer pixels would be wrong; use the
 * visible rect normalized to logical dimensions.
 */
export function clientToCanvas(
  canvas: HTMLCanvasElement,
  clientX: number,
  clientY: number
): { x: number; y: number } {
  const rect = canvas.getBoundingClientRect();
  return {
    x: ((clientX - rect.left) / rect.width) * CANVAS_WIDTH,
    y: ((clientY - rect.top) / rect.height) * CANVAS_HEIGHT,
  };
}
