/**
 * Visualization layer: render game state to the DOM.
 * Depends only on core types; no direct WASM imports.
 */
import type { EntitySnapshot } from "../core/types";

function formatCoord(value: number): string {
  return value.toFixed(2);
}

function escapeAttr(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/"/g, "&quot;")
    .replace(/</g, "&lt;");
}

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

/**
 * Renders the entity count and list into the given container element.
 */
export function renderEntities(
  entities: EntitySnapshot[],
  container: HTMLElement
): void {
  const count = entities.length;
  const listHtml = entities
    .map((e) => {
      const vel =
        e.velocity != null
          ? `v (${formatCoord(e.velocity.vx)}, ${formatCoord(e.velocity.vy)})`
          : "static";
      const idAttr = escapeAttr(e.id);
      const idHtml = escapeHtml(e.id);
      const typeHtml = escapeHtml(e.entityType);
      return `<button type="button" class="entity entity-row" data-entity-id="${idAttr}" aria-label="Select entity ${idAttr}">
        <strong>Entity ${idHtml}</strong>
        <span class="entity-meta">${typeHtml} · (${formatCoord(e.pos.x)}, ${formatCoord(e.pos.y)}) · ${vel}</span>
      </button>`;
    })
    .join("");
  container.innerHTML = `<p class="entity-count">Forces: <strong>${count}</strong></p>${listHtml}`;
}
