/**
 * Visualization layer: render game state to the DOM.
 * Depends only on core types; no direct WASM imports.
 */
import type { EntitySnapshot } from "../core/types";

function formatCoord(value: number): string {
  return value.toFixed(2);
}

/**
 * Renders the entity count and list into the given container element.
 * Count is part of the same output so it always stays in sync with the list.
 */
export function renderEntities(
  entities: EntitySnapshot[],
  container: HTMLElement
): void {
  const count = entities.length;
  const listHtml = entities
    .map(
      (e) => `<div class="entity">
        <strong>Entity ${e.id}</strong><br>
        Position: (${formatCoord(e.x)}, ${formatCoord(e.y)})<br>
        Velocity: (${formatCoord(e.vx)}, ${formatCoord(e.vy)})
      </div>`
    )
    .join("");
  container.innerHTML = `<p class="entity-count"><strong>Count: ${count}</strong></p>${listHtml}`;
}
