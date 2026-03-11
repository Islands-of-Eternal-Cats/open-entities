import { describe, it, expect, beforeEach } from "vitest";
import { renderEntities } from "./render";
import type { GameEntity } from "../core/types";

function mockEntity(overrides: Partial<{
  id: number;
  x: number;
  y: number;
  vx: number;
  vy: number;
}> = {}): GameEntity {
  const { id = 0, x = 0, y = 0, vx = 0, vy = 0 } = overrides;
  return {
    id,
    position: { x: () => x, y: () => y },
    velocity: { vx: () => vx, vy: () => vy },
  } as unknown as GameEntity;
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
