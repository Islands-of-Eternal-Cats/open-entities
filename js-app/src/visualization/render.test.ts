import { describe, it, expect, beforeEach } from "vitest";
import { renderEntities } from "./render";
import type { EntitySnapshot } from "../core/types";

function mockEntity(
  overrides: Partial<EntitySnapshot> = {}
): EntitySnapshot {
  return { id: 0, x: 0, y: 0, vx: 0, vy: 0, ...overrides };
}

describe("renderEntities", () => {
  let container: HTMLElement;

  beforeEach(() => {
    container = document.createElement("div");
  });

  it("renders empty list as empty container", () => {
    renderEntities([], container);
    expect(container.innerHTML).toBe("");
  });

  it("renders one entity with position and velocity", () => {
    const entities = [mockEntity({ id: 0, x: 1.5, y: 2.25, vx: -0.5, vy: 1 })];
    renderEntities(entities, container);
    expect(container.innerHTML).toContain("Entity 0");
    expect(container.innerHTML).toContain("1.50");
    expect(container.innerHTML).toContain("2.25");
    expect(container.innerHTML).toContain("-0.50");
    expect(container.querySelectorAll(".entity").length).toBe(1);
  });

  it("renders multiple entities", () => {
    const entities = [
      mockEntity({ id: 0, x: 0, y: 0 }),
      mockEntity({ id: 1, x: 10, y: 20, vx: 1, vy: 2 }),
    ];
    renderEntities(entities, container);
    expect(container.querySelectorAll(".entity").length).toBe(2);
    expect(container.innerHTML).toContain("Entity 0");
    expect(container.innerHTML).toContain("Entity 1");
    expect(container.innerHTML).toContain("10.00");
    expect(container.innerHTML).toContain("20.00");
  });
});
