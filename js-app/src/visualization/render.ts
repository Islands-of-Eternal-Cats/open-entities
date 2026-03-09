/**
 * Visualization layer: render game state to the DOM.
 * Depends only on core types; no direct WASM imports.
 */
import type { GameEntity } from "../core/types";

function formatCoord(value: number): string {
  return value.toFixed(2);
}

/**
 * Renders the list of entities into the given container element.
 */
export function renderEntities(
  entities: GameEntity[],
  container: HTMLElement
): void {
  container.innerHTML = entities
    .map((entity) => {
      const pos = entity.position;
      const vel = entity.velocity;
      return `<div class="entity">
        <strong>Entity ${entity.id}</strong><br>
        Position: (${formatCoord(pos.x())}, ${formatCoord(pos.y())})<br>
        Velocity: (${formatCoord(vel.vx())}, ${formatCoord(vel.vy())})
      </div>`;
    })
    .join("");
}
