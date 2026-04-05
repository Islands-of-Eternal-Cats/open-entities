import { describe, it, expect, beforeEach } from "vitest";
import { renderEntities } from "./render";
import type { EntitySnapshot } from "../core/types";

function mockEntity(
  overrides: Partial<EntitySnapshot> = {}
): EntitySnapshot {
  return {
    id: "0",
    entityType: "mover",
    pos: { x: 0, y: 0 },
    velocity: { vx: 0, vy: 0 },
    faction: null,
    ...overrides,
  };
}

describe("renderEntities", () => {
  let container: HTMLElement;

  beforeEach(() => {
    container = document.createElement("div");
  });

  it("renders empty list with count 0", () => {
    renderEntities([], container);
    expect(container.innerHTML).toContain("Forces:");
    expect(container.innerHTML).toContain(">0<");
    expect(container.querySelectorAll(".entity").length).toBe(0);
  });

  it("renders one entity with position and velocity", () => {
    const entities = [
      mockEntity({
        id: "0",
        pos: { x: 1.5, y: 2.25 },
        velocity: { vx: -0.5, vy: 1 },
      }),
    ];
    renderEntities(entities, container);
    expect(container.innerHTML).toContain("Entity 0");
    expect(container.innerHTML).toContain("1.50");
    expect(container.innerHTML).toContain("2.25");
    expect(container.innerHTML).toContain("-0.50");
    expect(container.innerHTML).toContain("mover ·");
    expect(container.querySelectorAll(".entity").length).toBe(1);
  });

  it("renders multiple entities", () => {
    const entities = [
      mockEntity({ id: "0", pos: { x: 0, y: 0 } }),
      mockEntity({
        id: "1",
        pos: { x: 10, y: 20 },
        velocity: { vx: 1, vy: 2 },
      }),
    ];
    renderEntities(entities, container);
    expect(container.querySelectorAll(".entity").length).toBe(2);
    expect(container.innerHTML).toContain("Entity 0");
    expect(container.innerHTML).toContain("Entity 1");
    expect(container.innerHTML).toContain("10.00");
    expect(container.innerHTML).toContain("20.00");
  });
});
