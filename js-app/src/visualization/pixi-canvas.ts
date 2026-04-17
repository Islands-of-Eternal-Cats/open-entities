/**
 * PixiJS canvas visualization: renders entities as circles on a 2D canvas.
 * World coordinates (from WASM) are scaled to canvas size.
 * Drag a rectangle to select multiple units; Shift adds to selection.
 * Tap empty ground with selection issues a move order; Esc / clear button / right-click clears selection.
 */
import { Application, Container, Graphics } from "pixi.js";
import type { EntitySnapshot, Pos } from "../core/types";
import {
  ENTITY_RADIUS_PX,
  WORLD_SIZE,
  centerViewOnWorld,
  clientToCanvas,
  getLogicalCanvasSize,
  screenToWorld,
  setLogicalCanvasSize,
  worldToScreen,
} from "./coords";
import { entityIdAtScreenPoint, entityIdsInScreenMarquee } from "./selection-logic";

const COLORS = [0x3498db, 0xe74c3c, 0x2ecc71, 0xf39c12, 0x9b59b6, 0x1abc9c];

const DRAG_THRESHOLD_PX = 5;

const MOVE_TARGET_MS = 2000;

const MINIMAP_W = 108;
const MINIMAP_H = 82;
const MINIMAP_PAD = 7;
const MINIMAP_MARGIN = 10;

/** Stable color index from entity id string (for consistent color per entity). */
function colorIndex(id: string): number {
  let h = 0;
  for (let i = 0; i < id.length; i++) h = ((h << 5) - h + id.charCodeAt(i)) | 0;
  return Math.abs(h) % COLORS.length;
}

function makeCircle(g: Graphics, color: number): void {
  g.clear();
  g.circle(0, 0, ENTITY_RADIUS_PX).fill(color);
}

export type PixiCanvasOptions = {
  /** Called when the temporary selection set changes (marquee or click). */
  onSelectionChange?: (selectedIds: ReadonlySet<string>) => void;
  /**
   * Tap/click on empty ground while at least one unit is selected: world-space move order.
   * Selection is not cleared; use Esc / UI to deselect.
   */
  onMoveOrder?: (world: Pos) => void | Promise<void>;
};

/**
 * Initializes PixiJS application and mounts canvas into the container.
 */
export async function initPixiCanvas(
  container: HTMLElement,
  options?: PixiCanvasOptions
): Promise<{
  updateEntities: (entities: EntitySnapshot[]) => void;
  getSelectedIds: () => ReadonlySet<string>;
  clearSelection: () => void;
  setSelectedIds: (ids: readonly string[]) => void;
  showMoveTarget: (world: Pos) => void;
}> {
  const onSelectionChange = options?.onSelectionChange;
  const onMoveOrder = options?.onMoveOrder;

  const application = new Application();
  await application.init({
    resizeTo: container,
    backgroundColor: 0x1a1a2e,
    antialias: true,
    resolution: window.devicePixelRatio ?? 1,
    autoDensity: true,
  });

  setLogicalCanvasSize(application.screen.width, application.screen.height);
  centerViewOnWorld(WORLD_SIZE / 2, WORLD_SIZE / 2);

  const canvas = application.canvas as HTMLCanvasElement;
  canvas.style.touchAction = "none";

  const entityLayer = new Container();
  const hoverGraphics = new Graphics();
  const selectionRings = new Graphics();
  const moveTargetGraphics = new Graphics();
  const marqueeGraphics = new Graphics();

  application.stage.addChild(entityLayer);
  application.stage.addChild(hoverGraphics);
  application.stage.addChild(selectionRings);
  application.stage.addChild(moveTargetGraphics);

  const minimapRoot = new Container();
  const minimapFrame = new Graphics();
  const minimapViewport = new Graphics();
  const minimapDots = new Graphics();
  minimapRoot.addChild(minimapFrame);
  minimapRoot.addChild(minimapViewport);
  minimapRoot.addChild(minimapDots);
  application.stage.addChild(minimapRoot);
  application.stage.addChild(marqueeGraphics);

  const entityGraphics = new Map<string, Graphics>();

  let lastEntities: EntitySnapshot[] = [];
  const selectedIds = new Set<string>();
  let hoveredId: string | null = null;
  let moveTargetFlash: { x: number; y: number; until: number } | null = null;

  function layoutMinimap(): void {
    minimapRoot.x = MINIMAP_MARGIN;
    minimapRoot.y = application.screen.height - MINIMAP_H - MINIMAP_MARGIN;
  }

  function canvasPointInMinimap(sx: number, sy: number): boolean {
    return (
      sx >= minimapRoot.x &&
      sy >= minimapRoot.y &&
      sx <= minimapRoot.x + MINIMAP_W &&
      sy <= minimapRoot.y + MINIMAP_H
    );
  }

  function minimapCanvasToWorld(sx: number, sy: number): Pos {
    const lx = sx - minimapRoot.x;
    const ly = sy - minimapRoot.y;
    const iw = MINIMAP_W - 2 * MINIMAP_PAD;
    const ih = MINIMAP_H - 2 * MINIMAP_PAD;
    const nx = ((lx - MINIMAP_PAD) / iw) * WORLD_SIZE;
    const ny = ((ly - MINIMAP_PAD) / ih) * WORLD_SIZE;
    return {
      x: Math.max(0, Math.min(WORLD_SIZE, nx)),
      y: Math.max(0, Math.min(WORLD_SIZE, ny)),
    };
  }

  function handleMinimapTap(sx: number, sy: number): void {
    if (!canvasPointInMinimap(sx, sy)) return;
    const world = minimapCanvasToWorld(sx, sy);
    if (selectedIds.size > 0) {
      void Promise.resolve(onMoveOrder?.(world));
    } else {
      centerViewOnWorld(world.x, world.y);
      repositionAllGraphics();
    }
  }

  function redrawMinimapViewport(): void {
    minimapViewport.clear();
    const { width: W, height: H } = getLogicalCanvasSize();
    const corners = [
      screenToWorld(0, 0),
      screenToWorld(W, 0),
      screenToWorld(W, H),
      screenToWorld(0, H),
    ];
    let minX = WORLD_SIZE;
    let maxX = 0;
    let minY = WORLD_SIZE;
    let maxY = 0;
    for (const c of corners) {
      minX = Math.min(minX, c.x);
      maxX = Math.max(maxX, c.x);
      minY = Math.min(minY, c.y);
      maxY = Math.max(maxY, c.y);
    }
    minX = Math.max(0, Math.min(WORLD_SIZE, minX));
    maxX = Math.max(0, Math.min(WORLD_SIZE, maxX));
    minY = Math.max(0, Math.min(WORLD_SIZE, minY));
    maxY = Math.max(0, Math.min(WORLD_SIZE, maxY));
    const p0 = worldToMinimap(minX, minY);
    const p1 = worldToMinimap(maxX, maxY);
    const left = Math.min(p0.x, p1.x);
    const top = Math.min(p0.y, p1.y);
    const rw = Math.abs(p1.x - p0.x);
    const rh = Math.abs(p1.y - p0.y);
    if (rw < 3 || rh < 3) return;
    minimapViewport
      .rect(left, top, rw, rh)
      .stroke({ width: 1, color: 0xffffff, alpha: 0.38 });
  }

  function worldToMinimap(wx: number, wy: number): { x: number; y: number } {
    const iw = MINIMAP_W - 2 * MINIMAP_PAD;
    const ih = MINIMAP_H - 2 * MINIMAP_PAD;
    const cx = Math.max(0, Math.min(WORLD_SIZE, wx));
    const cy = Math.max(0, Math.min(WORLD_SIZE, wy));
    return {
      x: MINIMAP_PAD + (cx / WORLD_SIZE) * iw,
      y: MINIMAP_PAD + (cy / WORLD_SIZE) * ih,
    };
  }

  function redrawMinimapFrame(): void {
    minimapFrame.clear();
    minimapFrame
      .roundRect(0, 0, MINIMAP_W, MINIMAP_H, 5)
      .fill({ color: 0x0d0f18, alpha: 0.94 });
    minimapFrame
      .roundRect(0, 0, MINIMAP_W, MINIMAP_H, 5)
      .stroke({ width: 1, color: 0x5c6578, alpha: 0.95 });
  }

  function redrawMinimap(): void {
    minimapDots.clear();
    for (const e of lastEntities) {
      const { x, y } = worldToMinimap(e.pos.x, e.pos.y);
      const sel = selectedIds.has(e.id);
      const r = sel ? 3.5 : 2.5;
      const c = sel ? 0xf1c40f : COLORS[colorIndex(e.id)];
      minimapDots.circle(x, y, r).fill({ color: c, alpha: sel ? 1 : 0.88 });
    }
    if (moveTargetFlash !== null && performance.now() < moveTargetFlash.until) {
      const { x, y } = worldToMinimap(moveTargetFlash.x, moveTargetFlash.y);
      minimapDots
        .circle(x, y, 5)
        .stroke({ width: 1.5, color: 0x2ecc71, alpha: 0.95 });
    }
    redrawMinimapViewport();
  }

  function notifySelection(): void {
    onSelectionChange?.(selectedIds);
    redrawMinimap();
  }

  function pruneSelection(validIds: Set<string>): void {
    let changed = false;
    for (const id of selectedIds) {
      if (!validIds.has(id)) {
        selectedIds.delete(id);
        changed = true;
      }
    }
    if (changed) notifySelection();
  }

  function redrawSelectionRings(): void {
    selectionRings.clear();
    for (const id of selectedIds) {
      const entity = lastEntities.find((e) => e.id === id);
      if (!entity) continue;
      const { x, y } = worldToScreen(entity.pos.x, entity.pos.y);
      selectionRings
        .circle(x, y, ENTITY_RADIUS_PX + 5)
        .stroke({ width: 2, color: 0xf1c40f, alpha: 0.45 });
      selectionRings
        .circle(x, y, ENTITY_RADIUS_PX + 2)
        .stroke({ width: 2, color: 0xf39c12, alpha: 0.98 });
    }
  }

  function redrawHoverRing(): void {
    hoverGraphics.clear();
    if (hoveredId === null) return;
    const entity = lastEntities.find((e) => e.id === hoveredId);
    if (!entity) return;
    const { x, y } = worldToScreen(entity.pos.x, entity.pos.y);
    hoverGraphics
      .circle(x, y, ENTITY_RADIUS_PX + 3)
      .stroke({ width: 1.5, color: 0xffffff, alpha: 0.8 });
  }

  function drawMoveTarget(): void {
    moveTargetGraphics.clear();
    if (moveTargetFlash !== null) {
      const now = performance.now();
      if (now >= moveTargetFlash.until) {
        moveTargetFlash = null;
      } else {
        const t = Math.min(1, (moveTargetFlash.until - now) / 450);
        const alpha = 0.35 + t * 0.55;
        const { x, y } = worldToScreen(moveTargetFlash.x, moveTargetFlash.y);
        moveTargetGraphics
          .circle(x, y, 14)
          .stroke({ width: 2, color: 0x2ecc71, alpha: alpha * 0.95 });
        moveTargetGraphics
          .circle(x, y, 5)
          .fill({ color: 0x2ecc71, alpha: alpha * 0.5 });
      }
    }
    redrawMinimap();
  }

  function repositionAllGraphics(): void {
    for (const entity of lastEntities) {
      const g = entityGraphics.get(entity.id);
      if (!g) continue;
      const { x, y } = worldToScreen(entity.pos.x, entity.pos.y);
      g.x = x;
      g.y = y;
    }
    redrawSelectionRings();
    redrawHoverRing();
    drawMoveTarget();
    layoutMinimap();
    redrawMinimap();
  }

  application.renderer.on("resize", () => {
    const prev = getLogicalCanvasSize();
    const centerBeforeResize = screenToWorld(prev.width / 2, prev.height / 2);
    setLogicalCanvasSize(application.screen.width, application.screen.height);
    centerViewOnWorld(centerBeforeResize.x, centerBeforeResize.y);
    repositionAllGraphics();
    redrawMinimapFrame();
  });

  function redrawMarquee(
    x0: number,
    y0: number,
    x1: number,
    y1: number
  ): void {
    marqueeGraphics.clear();
    const left = Math.min(x0, x1);
    const top = Math.min(y0, y1);
    const w = Math.abs(x1 - x0);
    const h = Math.abs(y1 - y0);
    if (w < 1 || h < 1) return;
    marqueeGraphics
      .rect(left, top, w, h)
      .fill({ color: 0xecf0f1, alpha: 0.12 });
    marqueeGraphics
      .rect(left, top, w, h)
      .stroke({ width: 1, color: 0xffffff, alpha: 0.55 });
  }

  let dragStart: { x: number; y: number } | null = null;
  let dragCurrent: { x: number; y: number } | null = null;
  let isDragging = false;
  let interactionStartedOnMinimap = false;

  function clearMarquee(): void {
    marqueeGraphics.clear();
    dragStart = null;
    dragCurrent = null;
    isDragging = false;
  }

  function applyMarqueeSelection(
    x0: number,
    y0: number,
    x1: number,
    y1: number,
    shiftKey: boolean
  ): void {
    const picked = entityIdsInScreenMarquee(lastEntities, x0, y0, x1, y1);
    if (shiftKey) {
      for (const id of picked) selectedIds.add(id);
    } else {
      selectedIds.clear();
      for (const id of picked) selectedIds.add(id);
    }
    notifySelection();
  }

  function applyClickSelection(
    sx: number,
    sy: number,
    shiftKey: boolean
  ): void {
    const hit = entityIdAtScreenPoint(lastEntities, sx, sy);
    if (hit === null) {
      if (selectedIds.size > 0) {
        void Promise.resolve(onMoveOrder?.(screenToWorld(sx, sy)));
      }
      return;
    }
    if (shiftKey) {
      if (selectedIds.has(hit)) selectedIds.delete(hit);
      else selectedIds.add(hit);
    } else {
      selectedIds.clear();
      selectedIds.add(hit);
    }
    notifySelection();
  }

  function updateHoverFromClient(clientX: number, clientY: number): void {
    if (isDragging) return;
    const p = clientToCanvas(canvas, clientX, clientY);
    canvas.style.cursor = canvasPointInMinimap(p.x, p.y) ? "pointer" : "";
    const hit = entityIdAtScreenPoint(lastEntities, p.x, p.y);
    if (hit !== hoveredId) {
      hoveredId = hit;
      redrawHoverRing();
    }
  }

  canvas.addEventListener("pointerdown", (ev) => {
    if (ev.button !== 0) return;
    const p = clientToCanvas(canvas, ev.clientX, ev.clientY);
    interactionStartedOnMinimap = canvasPointInMinimap(p.x, p.y);
    dragStart = p;
    dragCurrent = { ...dragStart };
    isDragging = true;
    try {
      canvas.setPointerCapture(ev.pointerId);
    } catch {
      /* ignore */
    }
  });

  canvas.addEventListener("pointermove", (ev) => {
    if (isDragging && dragStart !== null && !interactionStartedOnMinimap) {
      dragCurrent = clientToCanvas(canvas, ev.clientX, ev.clientY);
      const dx = dragCurrent.x - dragStart.x;
      const dy = dragCurrent.y - dragStart.y;
      if (Math.hypot(dx, dy) >= DRAG_THRESHOLD_PX) {
        redrawMarquee(dragStart.x, dragStart.y, dragCurrent.x, dragCurrent.y);
      }
      return;
    }
    updateHoverFromClient(ev.clientX, ev.clientY);
  });

  canvas.addEventListener("pointerup", (ev) => {
    if (ev.button === 2) {
      if (selectedIds.size > 0) {
        clearSelection();
      }
      clearMarquee();
      interactionStartedOnMinimap = false;
      try {
        canvas.releasePointerCapture(ev.pointerId);
      } catch {
        /* ignore */
      }
      return;
    }
    if (!isDragging || dragStart === null) {
      clearMarquee();
      interactionStartedOnMinimap = false;
      try {
        canvas.releasePointerCapture(ev.pointerId);
      } catch {
        /* ignore */
      }
      return;
    }
    const end = clientToCanvas(canvas, ev.clientX, ev.clientY);
    const dx = end.x - dragStart.x;
    const dy = end.y - dragStart.y;
    const dist = Math.hypot(dx, dy);

    if (interactionStartedOnMinimap) {
      if (dist < DRAG_THRESHOLD_PX) {
        const tap = canvasPointInMinimap(end.x, end.y) ? end : dragStart;
        handleMinimapTap(tap.x, tap.y);
      }
      interactionStartedOnMinimap = false;
      clearMarquee();
      redrawSelectionRings();
      try {
        canvas.releasePointerCapture(ev.pointerId);
      } catch {
        /* ignore */
      }
      return;
    }

    if (dist < DRAG_THRESHOLD_PX) {
      applyClickSelection(end.x, end.y, ev.shiftKey);
    } else {
      applyMarqueeSelection(
        dragStart.x,
        dragStart.y,
        end.x,
        end.y,
        ev.shiftKey
      );
    }

    clearMarquee();
    redrawSelectionRings();
    try {
      canvas.releasePointerCapture(ev.pointerId);
    } catch {
      /* ignore */
    }
  });

  canvas.addEventListener("pointercancel", (ev) => {
    clearMarquee();
    interactionStartedOnMinimap = false;
    try {
      canvas.releasePointerCapture(ev.pointerId);
    } catch {
      /* ignore */
    }
  });

  canvas.addEventListener("pointerleave", () => {
    canvas.style.cursor = "";
    if (hoveredId !== null) {
      hoveredId = null;
      redrawHoverRing();
    }
  });

  canvas.addEventListener("contextmenu", (ev) => {
    ev.preventDefault();
  });

  container.appendChild(canvas);

  layoutMinimap();
  redrawMinimapFrame();
  redrawMinimap();

  function updateEntities(entities: EntitySnapshot[]): void {
    lastEntities = entities;
    const ids = new Set(entities.map((e) => e.id));
    pruneSelection(ids);

    for (const [id, g] of entityGraphics.entries()) {
      if (!ids.has(id)) {
        entityLayer.removeChild(g);
        g.destroy();
        entityGraphics.delete(id);
      }
    }

    entities.forEach((entity) => {
      let g = entityGraphics.get(entity.id);
      if (!g) {
        g = new Graphics();
        const color = COLORS[colorIndex(entity.id)];
        makeCircle(g, color);
        entityGraphics.set(entity.id, g);
        entityLayer.addChild(g);
      }

      const { x, y } = worldToScreen(entity.pos.x, entity.pos.y);
      g.x = x;
      g.y = y;
    });

    redrawSelectionRings();
    redrawHoverRing();
    drawMoveTarget();
  }

  function getSelectedIds(): ReadonlySet<string> {
    return selectedIds;
  }

  function clearSelection(): void {
    selectedIds.clear();
    notifySelection();
    redrawSelectionRings();
  }

  function setSelectedIds(ids: readonly string[]): void {
    const valid = new Set(lastEntities.map((e) => e.id));
    selectedIds.clear();
    for (const id of ids) {
      if (valid.has(id)) selectedIds.add(id);
    }
    notifySelection();
    redrawSelectionRings();
  }

  function showMoveTarget(world: Pos): void {
    moveTargetFlash = {
      x: world.x,
      y: world.y,
      until: performance.now() + MOVE_TARGET_MS,
    };
    drawMoveTarget();
  }

  return {
    updateEntities,
    getSelectedIds,
    clearSelection,
    setSelectedIds,
    showMoveTarget,
  };
}
