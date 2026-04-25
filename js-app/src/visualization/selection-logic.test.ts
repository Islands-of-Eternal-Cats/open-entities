import { describe, expect, it } from "vitest";
import type { EntitySnapshot } from "../core/types";
import { worldToScreen } from "./coords";
import {
  entityIdAtScreenPoint,
  entityIdsInScreenMarquee,
  shouldIssueMoveOrder,
} from "./selection-logic";

describe("selection-logic", () => {
  const a: EntitySnapshot = {
    id: "1",
    entityType: "mover",
    pos: { x: 5, y: 5 },
    velocity: null,
    faction: null,
  };
  const b: EntitySnapshot = {
    id: "2",
    entityType: "mover",
    pos: { x: 90, y: 90 },
    velocity: null,
    faction: null,
  };

  it("marquee includes only entities inside world AABB", () => {
    const p00 = worldToScreen(0, 0);
    const p50 = worldToScreen(50, 50);
    const ids = entityIdsInScreenMarquee([a, b], p00.x, p00.y, p50.x, p50.y);
    expect(ids).toContain("1");
    expect(ids).not.toContain("2");
  });

  it("entityIdAtScreenPoint hits circle around world position", () => {
    const p = worldToScreen(5, 5);
    expect(entityIdAtScreenPoint([a, b], p.x, p.y)).toBe("1");
    expect(entityIdAtScreenPoint([a, b], p.x + 40, p.y)).toBe(null);
  });

  it("issues move order only on plain click to empty ground", () => {
    expect(
      shouldIssueMoveOrder({
        hitEntityId: null,
        selectedCount: 2,
        shiftKey: false,
        ctrlKey: false,
        altKey: false,
        metaKey: false,
      })
    ).toBe(true);

    expect(
      shouldIssueMoveOrder({
        hitEntityId: "1",
        selectedCount: 2,
        shiftKey: false,
        ctrlKey: false,
        altKey: false,
        metaKey: false,
      })
    ).toBe(false);

    expect(
      shouldIssueMoveOrder({
        hitEntityId: null,
        selectedCount: 0,
        shiftKey: false,
        ctrlKey: false,
        altKey: false,
        metaKey: false,
      })
    ).toBe(false);

    expect(
      shouldIssueMoveOrder({
        hitEntityId: null,
        selectedCount: 2,
        shiftKey: true,
        ctrlKey: false,
        altKey: false,
        metaKey: false,
      })
    ).toBe(false);
  });
});
