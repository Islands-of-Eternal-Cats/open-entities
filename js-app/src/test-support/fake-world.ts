/**
 * A `WorldReader` over a plain list of snapshot rows, for tests of the UI layer.
 *
 * The real `WorldView` reads position frames; tests that are about input and HUD wiring only need
 * the same answers from a list they can write by hand.
 */
import type { EntitySnapshot } from "../core/types";
import { keyToEntityId } from "../core/types";
import type { WorldReader } from "../core/world-view";

export function fakeWorld(entities: EntitySnapshot[], tick = 0): WorldReader {
  const byKey = () => new Map(entities.map((e) => [e.id, e]));
  return {
    tick,
    get size() {
      return entities.length;
    },
    forEachDrawn(visit) {
      for (const e of entities) {
        const { index, generation } = keyToEntityId(e.id);
        visit(index, generation, e.pos.x, e.pos.y, e.aboard !== null);
      }
    },
    get: (key) => byKey().get(key) ?? null,
    has: (key) => byKey().has(key),
    find: (predicate) => entities.find(predicate) ?? null,
    ridersOf: (vehicle) => entities.filter((e) => e.aboard === vehicle).map((e) => e.id),
    walkersTo: (vehicle) => entities.filter((e) => e.boarding === vehicle).map((e) => e.id),
  };
}
