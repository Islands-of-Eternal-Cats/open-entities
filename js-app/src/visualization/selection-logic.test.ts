import { describe, expect, it } from "vitest";
import type { EntitySnapshot } from "../core/types";
import { worldToScreen } from "./coords";
import { entityIdAtScreenPoint, entityIdsInScreenMarquee } from "./selection-logic";

describe("selection-logic", () => {
  const a: EntitySnapshot = {
    id: "1",
    pos: { x: 5, y: 5 },
    velocity: null,
  };
  const b: EntitySnapshot = {
    id: "2",
    pos: { x: 90, y: 90 },
    velocity: null,
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
});
