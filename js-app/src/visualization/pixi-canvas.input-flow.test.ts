import { beforeEach, describe, expect, it, vi } from "vitest";
import type { EntitySnapshot } from "../core/types";
import { fakeWorld } from "../test-support/fake-world";
import { worldToScreen } from "./coords";

/** Particle layers the mocked Pixi created, newest last. */
const pixiMock = vi.hoisted(() => ({
  layers: [] as Array<{ particleChildren: Array<{ x: number; y: number }> }>,
}));

vi.mock("pixi.js", () => {
  class Graphics {
    x = 0;
    y = 0;
    clear(): this {
      return this;
    }
    circle(): this {
      return this;
    }
    rect(): this {
      return this;
    }
    roundRect(): this {
      return this;
    }
    fill(): this {
      return this;
    }
    stroke(): this {
      return this;
    }
    destroy(): void {}
  }

  class Container {
    x = 0;
    y = 0;
    children: unknown[] = [];
    addChild(child: unknown): void {
      this.children.push(child);
    }
    removeChild(child: unknown): void {
      this.children = this.children.filter((c) => c !== child);
    }
  }

  class Particle {
    x = 0;
    y = 0;
    alpha = 1;
    constructor(public options: unknown) {}
  }

  class ParticleContainer extends Container {
    particleChildren: Particle[] = [];
    constructor(public options?: unknown) {
      super();
      pixiMock.layers.push(this);
    }
    update(): void {}
  }

  class Application {
    stage = new Container();
    screen = { width: 640, height: 480 };
    canvas = document.createElement("canvas");
    renderer = {
      on: () => {},
      generateTexture: () => ({}),
    };
    async init(): Promise<void> {
      Object.defineProperty(this.canvas, "getBoundingClientRect", {
        value: () => ({
          x: 0,
          y: 0,
          top: 0,
          left: 0,
          width: this.screen.width,
          height: this.screen.height,
          right: this.screen.width,
          bottom: this.screen.height,
          toJSON: () => ({}),
        }),
      });
    }
  }

  return { Application, Container, Graphics, Particle, ParticleContainer };
});

import { initPixiCanvas } from "./pixi-canvas";

function makeEntity(id: string, x: number, y: number): EntitySnapshot {
  return {
    id,
    entityType: "mover",
    pos: { x, y },
    velocity: null,
    faction: null,
    seats: null,
    aboard: null,
    boarding: null,
    group: null,
    moveTarget: null,
  };
}

function emitPointer(
  canvas: HTMLCanvasElement,
  type: "pointerdown" | "pointerup",
  x: number,
  y: number,
  opts?: { button?: number; shiftKey?: boolean; ctrlKey?: boolean }
): void {
  const ev = new MouseEvent(type, {
    bubbles: true,
    button: opts?.button ?? 0,
    clientX: x,
    clientY: y,
    shiftKey: opts?.shiftKey ?? false,
    ctrlKey: opts?.ctrlKey ?? false,
  });
  Object.defineProperty(ev, "pointerId", { value: 1 });
  canvas.dispatchEvent(ev);
}

describe("pixi-canvas drawing", () => {
  beforeEach(() => {
    document.body.innerHTML = "";
  });

  it("keeps one particle per unit across frames and drops the ones that are gone", async () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const pixi = await initPixiCanvas(container);
    const layer = () => pixiMock.layers[pixiMock.layers.length - 1].particleChildren;

    pixi.drawWorld(fakeWorld([makeEntity("1:0", 1, 1), makeEntity("2:0", 2, 2)]));
    const [first] = layer();
    expect(layer()).toHaveLength(2);

    pixi.drawWorld(fakeWorld([makeEntity("1:0", 5, 5)], 1));
    expect(layer()).toEqual([first]);
    const p = worldToScreen(5, 5);
    expect(first).toMatchObject({ x: p.x, y: p.y });
  });

  it("drops a selected unit from the selection once it is gone", async () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const pixi = await initPixiCanvas(container);
    pixi.drawWorld(fakeWorld([makeEntity("1:0", 1, 1), makeEntity("2:0", 2, 2)]));
    pixi.setSelectedIds(["1:0", "2:0"]);

    pixi.drawWorld(fakeWorld([makeEntity("2:0", 2, 2)], 1));
    expect([...pixi.getSelectedIds()]).toEqual(["2:0"]);
  });
});

describe("pixi-canvas input flow", () => {
  beforeEach(() => {
    document.body.innerHTML = "";
  });

  it("issues move order on plain click to empty ground with active selection", async () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const onMoveOrder = vi.fn();
    const pixi = await initPixiCanvas(container, { onMoveOrder });

    pixi.drawWorld(fakeWorld([makeEntity("1:0", 20, 20)]));
    pixi.setSelectedIds(["1:0"]);

    const canvas = container.querySelector("canvas") as HTMLCanvasElement;
    emitPointer(canvas, "pointerdown", 450, 330);
    emitPointer(canvas, "pointerup", 450, 330);

    expect(onMoveOrder).toHaveBeenCalledTimes(1);
  });

  it("does not issue move order on modified click on empty ground", async () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const onMoveOrder = vi.fn();
    const pixi = await initPixiCanvas(container, { onMoveOrder });

    pixi.drawWorld(fakeWorld([makeEntity("1:0", 20, 20)]));
    pixi.setSelectedIds(["1:0"]);

    const canvas = container.querySelector("canvas") as HTMLCanvasElement;
    emitPointer(canvas, "pointerdown", 450, 330, { shiftKey: true });
    emitPointer(canvas, "pointerup", 450, 330, { shiftKey: true });

    expect(onMoveOrder).not.toHaveBeenCalled();
  });

  it("does not issue move order from minimap when modifier key is pressed", async () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const onMoveOrder = vi.fn();
    const pixi = await initPixiCanvas(container, { onMoveOrder });

    pixi.drawWorld(fakeWorld([makeEntity("1:0", 30, 30)]));
    pixi.setSelectedIds(["1:0"]);

    const canvas = container.querySelector("canvas") as HTMLCanvasElement;
    emitPointer(canvas, "pointerdown", 20, 400, { ctrlKey: true });
    emitPointer(canvas, "pointerup", 20, 400, { ctrlKey: true });

    expect(onMoveOrder).not.toHaveBeenCalled();
  });

  it("clears selection on right click pointerup", async () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const pixi = await initPixiCanvas(container);
    pixi.drawWorld(fakeWorld([makeEntity("1:0", 20, 20)]));
    pixi.setSelectedIds(["1:0"]);

    const p = worldToScreen(20, 20);
    const canvas = container.querySelector("canvas") as HTMLCanvasElement;
    emitPointer(canvas, "pointerup", p.x, p.y, { button: 2 });

    expect([...pixi.getSelectedIds()]).toEqual([]);
  });
});
