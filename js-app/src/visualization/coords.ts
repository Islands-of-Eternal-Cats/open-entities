/**
 * Shared world ↔ canvas mapping for Pixi visualization.
 * Logical size is synced from the Pixi view via `setLogicalCanvasSize` when using `resizeTo`.
 */

export const WORLD_SIZE = 120;
export const ENTITY_RADIUS_PX = 8;

let logicalWidth = 640;
let logicalHeight = 480;

/** Pixel pan of the tactical view (applied after base fit of world into canvas). */
let viewPanX = 0;
let viewPanY = 0;

export function setLogicalCanvasSize(width: number, height: number): void {
  logicalWidth = Math.max(1, Math.floor(width));
  logicalHeight = Math.max(1, Math.floor(height));
}

export function getLogicalCanvasSize(): { width: number; height: number } {
  return { width: logicalWidth, height: logicalHeight };
}

export function getScaleAndOffset(): {
  scale: number;
  offsetX: number;
  offsetY: number;
} {
  const scale = Math.min(logicalWidth, logicalHeight) / WORLD_SIZE;
  const offsetX = (logicalWidth - WORLD_SIZE * scale) / 2;
  const offsetY = (logicalHeight - WORLD_SIZE * scale) / 2;
  return { scale, offsetX, offsetY };
}

export function worldToScreen(
  x: number,
  y: number
): { x: number; y: number } {
  const { scale, offsetX, offsetY } = getScaleAndOffset();
  return {
    x: offsetX + x * scale + viewPanX,
    y: offsetY + y * scale + viewPanY,
  };
}

export function screenToWorld(
  sx: number,
  sy: number
): { x: number; y: number } {
  const { scale, offsetX, offsetY } = getScaleAndOffset();
  return {
    x: (sx - offsetX - viewPanX) / scale,
    y: (sy - offsetY - viewPanY) / scale,
  };
}

/** Place the given world point at the center of the logical canvas. */
export function centerViewOnWorld(wx: number, wy: number): void {
  const { scale, offsetX, offsetY } = getScaleAndOffset();
  const cx = logicalWidth / 2;
  const cy = logicalHeight / 2;
  viewPanX = cx - offsetX - wx * scale;
  viewPanY = cy - offsetY - wy * scale;
}

/** Reset tactical view pan (full map centered as before). */
export function resetViewPan(): void {
  viewPanX = 0;
  viewPanY = 0;
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
 * logical size. Map via the visible rect normalized to logical dimensions.
 */
export function clientToCanvas(
  canvas: HTMLCanvasElement,
  clientX: number,
  clientY: number
): { x: number; y: number } {
  const rect = canvas.getBoundingClientRect();
  const { width, height } = getLogicalCanvasSize();
  return {
    x: ((clientX - rect.left) / rect.width) * width,
    y: ((clientY - rect.top) / rect.height) * height,
  };
}
