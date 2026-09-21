/**
 * Visualization layer: render game state to the DOM.
 * Depends only on core types; no direct WASM imports.
 */
import type { EntitySnapshot } from "../core/types";
import { setText } from "./dom";

function formatCoord(value: number): string {
  return value.toFixed(2);
}

function describeEntity(e: EntitySnapshot): string {
  const vel =
    e.velocity != null
      ? `v (${formatCoord(e.velocity.vx)}, ${formatCoord(e.velocity.vy)})`
      : "static";
  // Transport is invisible on the map — a rider sits inside its vehicle — so the row says it.
  const transport =
    e.aboard !== null
      ? ` · aboard ${e.aboard}`
      : e.boarding !== null
        ? ` · boarding ${e.boarding}`
        : e.seats !== null
          ? ` · ${e.seats} seats`
          : "";
  const order =
    e.moveTarget !== null
      ? ` · → (${formatCoord(e.moveTarget.x)}, ${formatCoord(e.moveTarget.y)})`
      : "";
  return `${e.entityType} · (${formatCoord(e.pos.x)}, ${formatCoord(e.pos.y)}) · ${vel}${transport}${order}`;
}

function createRow(e: EntitySnapshot): HTMLButtonElement {
  const row = document.createElement("button");
  row.type = "button";
  row.className = "entity entity-row";
  row.dataset.entityId = e.id;
  row.setAttribute("aria-label", `Select entity ${e.id}`);
  const title = document.createElement("strong");
  title.textContent = `Entity ${e.id}`;
  const meta = document.createElement("span");
  meta.className = "entity-meta";
  row.append(title, "\n        ", meta);
  return row;
}

function syncRow(
  row: HTMLButtonElement,
  e: EntitySnapshot,
  selected: boolean
): void {
  const meta = row.querySelector<HTMLElement>(".entity-meta");
  if (meta) setText(meta, describeEntity(e));
  if (row.classList.contains("entity-row--selected") !== selected) {
    row.classList.toggle("entity-row--selected", selected);
    if (selected) row.setAttribute("aria-current", "true");
    else row.removeAttribute("aria-current");
  }
}

/**
 * Renders the entity count and list into the given container element.
 *
 * Rows are kept between calls and keyed by entity id: a row is created when its entity first
 * appears, removed when it is gone, and otherwise only its text is touched — and only when the
 * text differs. The list is synced after every tick, and rebuilding it each time would make
 * its text impossible to select.
 */
export function renderEntities(
  entities: EntitySnapshot[],
  container: HTMLElement,
  selectedIds?: ReadonlySet<string>
): void {
  let count = container.querySelector<HTMLElement>(".entity-count");
  if (!count) {
    count = document.createElement("p");
    count.className = "entity-count";
    count.append("Forces: ", document.createElement("strong"));
    container.prepend(count);
  }
  const countValue = count.querySelector("strong");
  if (countValue) setText(countValue, String(entities.length));

  const rows = new Map<string, HTMLButtonElement>();
  for (const row of container.querySelectorAll<HTMLButtonElement>(".entity-row")) {
    const id = row.dataset.entityId;
    if (id !== undefined) rows.set(id, row);
  }

  let cursor: Element = count;
  for (const e of entities) {
    let row = rows.get(e.id);
    if (row) {
      rows.delete(e.id);
    } else {
      row = createRow(e);
    }
    syncRow(row, e, selectedIds?.has(e.id) ?? false);
    // Keep document order equal to snapshot order without rebuilding: move only what is out of place.
    if (cursor.nextElementSibling !== row) cursor.after(row);
    cursor = row;
  }
  for (const stale of rows.values()) stale.remove();
}
