/**
 * PixiJS canvas visualization: renders entities as circles on a 2D canvas.
 * World coordinates (from WASM) are scaled to canvas size.
 */
import { Application, Graphics } from "pixi.js";
import type { EntitySnapshot } from "../core/types";

const CANVAS_WIDTH = 640;
const CANVAS_HEIGHT = 480;
/** World space used for scaling (entities typically live in ~0..100) */
const WORLD_SIZE = 120;
const ENTITY_RADIUS = 8;
const COLORS = [0x3498db, 0xe74c3c, 0x2ecc71, 0xf39c12, 0x9b59b6, 0x1abc9c];

let app: Application | null = null;
const entityGraphics = new Map<number, Graphics>();

function worldToScreen(x: number, y: number): { x: number; y: number } {
  const scale = Math.min(CANVAS_WIDTH, CANVAS_HEIGHT) / WORLD_SIZE;
  const offsetX = (CANVAS_WIDTH - WORLD_SIZE * scale) / 2;
  const offsetY = (CANVAS_HEIGHT - WORLD_SIZE * scale) / 2;
  return {
    x: offsetX + x * scale,
    y: offsetY + y * scale,
  };
}

function makeCircle(g: Graphics, color: number): void {
  g.clear();
  g.circle(0, 0, ENTITY_RADIUS).fill(color);
}

/**
 * Initializes PixiJS application and mounts canvas into the container.
 * Returns an object with updateEntities to sync display with game state.
 */
export async function initPixiCanvas(
  container: HTMLElement
): Promise<{ updateEntities: (entities: EntitySnapshot[]) => void }> {
  const application = new Application();
  await application.init({
    width: CANVAS_WIDTH,
    height: CANVAS_HEIGHT,
    backgroundColor: 0x1a1a2e,
    antialias: true,
    resolution: window.devicePixelRatio ?? 1,
    autoDensity: true,
  });

  app = application;
  container.appendChild(application.canvas as HTMLCanvasElement);

  function updateEntities(entities: EntitySnapshot[]): void {
    if (!app) return;

    const stage = app.stage;
    const ids = new Set(entities.map((e) => e.id));

    // Remove display objects for entities that no longer exist
    for (const [id, g] of entityGraphics.entries()) {
      if (!ids.has(id)) {
        stage.removeChild(g);
        g.destroy();
        entityGraphics.delete(id);
      }
    }

    // Add or update (color by entity id for consistent visual per entity)
    entities.forEach((entity) => {
      let g = entityGraphics.get(entity.id);
      if (!g) {
        g = new Graphics();
        const color = COLORS[entity.id % COLORS.length];
        makeCircle(g, color);
        entityGraphics.set(entity.id, g);
        stage.addChild(g);
      }

      const { x, y } = worldToScreen(entity.pos.x, entity.pos.y);
      g.x = x;
      g.y = y;
    });
  }

  return { updateEntities };
}
