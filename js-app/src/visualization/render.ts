/**
 * Visualization layer: render game state to the DOM.
 * Depends only on core types; no direct WASM imports.
 */
import type { EntitySnapshot } from "../core/types";

function formatCoord(value: number): string {
  return value.toFixed(2);
}

/**
 * Renders the list of entities into the given container element.
 */
export function renderEntities(
  entities: EntitySnapshot[],
  container: HTMLElement
): void {
  container.innerHTML = entities
    .map(
      (e) => `<div class="entity">
        <strong>Entity ${e.id}</strong><br>
        Position: (${formatCoord(e.x)}, ${formatCoord(e.y)})<br>
        Velocity: (${formatCoord(e.vx)}, ${formatCoord(e.vy)})
      </div>`
    )
    .join("");
}
