/**
 * PixiJS canvas visualization: renders entities as circles on a 2D canvas.
 * World coordinates (from WASM) are scaled to canvas size.
 * Drag a rectangle to select multiple units; Shift adds to selection.
 * Tap empty ground with selection issues a move order; Esc / clear button clears selection.
 */
import { Application, Container, Graphics } from "pixi.js";
import type { EntitySnapshot, Pos } from "../core/types";
import {
  ENTITY_RADIUS_PX,
  clientToCanvas,
  screenToWorld,
  setLogicalCanvasSize,
  worldToScreen,
} from "./coords";
import { entityIdAtScreenPoint, entityIdsInScreenMarquee } from "./selection-logic";

const COLORS = [0x3498db, 0xe74c3c, 0x2ecc71, 0xf39c12, 0x9b59b6, 0x1abc9c];

const DRAG_THRESHOLD_PX = 5;

const MOVE_TARGET_MS = 2000;

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
  application.stage.addChild(marqueeGraphics);

  const entityGraphics = new Map<string, Graphics>();

  let lastEntities: EntitySnapshot[] = [];
  const selectedIds = new Set<string>();
  let hoveredId: string | null = null;
  let moveTargetFlash: { x: number; y: number; until: number } | null = null;

  function notifySelection(): void {
    onSelectionChange?.(selectedIds);
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
    if (moveTargetFlash === null) return;
    const now = performance.now();
    if (now >= moveTargetFlash.until) {
      moveTargetFlash = null;
      return;
    }
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
  }

  application.renderer.on("resize", () => {
    setLogicalCanvasSize(application.screen.width, application.screen.height);
    repositionAllGraphics();
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
    const hit = entityIdAtScreenPoint(lastEntities, p.x, p.y);
    if (hit !== hoveredId) {
      hoveredId = hit;
      redrawHoverRing();
    }
  }

  canvas.addEventListener("pointerdown", (ev) => {
    dragStart = clientToCanvas(canvas, ev.clientX, ev.clientY);
    dragCurrent = { ...dragStart };
    isDragging = true;
    try {
      canvas.setPointerCapture(ev.pointerId);
    } catch {
      /* ignore */
    }
  });

  canvas.addEventListener("pointermove", (ev) => {
    if (isDragging && dragStart !== null) {
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
    if (!isDragging || dragStart === null) {
      clearMarquee();
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
    try {
      canvas.releasePointerCapture(ev.pointerId);
    } catch {
      /* ignore */
    }
  });

  canvas.addEventListener("pointerleave", () => {
    if (hoveredId !== null) {
      hoveredId = null;
      redrawHoverRing();
    }
  });

  container.appendChild(canvas);

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
