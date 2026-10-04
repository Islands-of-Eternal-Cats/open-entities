/**
 * What the ECS worker does with each message, apart from `postMessage` itself.
 *
 * Kept free of `self` and of the wasm module so tests can drive it with a fake simulation; the
 * worker (`ecs-worker.ts`) posts whatever these methods return.
 *
 * The per-frame path creates no JSON: positions travel as the `Int32Array` from `writeFrame()`,
 * transferred rather than copied, and metadata only when `metaDelta()` reports a change.
 */
import { advanceClock } from "./fixed-step";
import type { Command, CommandOutcome } from "./types";
import type { FrameMessage, WorkerOutMessage } from "./worker-types";

/** The part of `Simulation` the worker uses. */
export interface FrameSimulation {
  step(): { outcomes: CommandOutcome[] };
  writeFrame(): Int32Array;
  metaDelta(): string | undefined;
  currentTick(): number;
  submit(command: Command): number;
}

/** A message to post and the buffers to transfer with it. */
export interface Outgoing {
  message: WorkerOutMessage;
  transfer: Transferable[];
}

/** Template the stress test spawns; `public/fixtures/entities.yaml` defines it. */
export const STRESS_TEMPLATE = "stress_mover";

/** mulberry32: a small seeded generator, so the same seed spawns the same crowd. */
function seeded(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 2 ** 32;
  };
}

export class WorkerCore {
  private accumulator = 0;

  constructor(private readonly sim: FrameSimulation) {}

  /**
   * Feeds one frame's elapsed time into the fixed-step clock and runs the ticks that fit.
   *
   * When a frame runs several steps, the buffer of tick t−1 is written just before the last of
   * them, so the renderer always blends the last two ticks and never the state at the start of
   * the frame.
   */
  frame(elapsedMs: number): Outgoing {
    const advance = advanceClock(this.accumulator, elapsedMs);
    this.accumulator = advance.accumulator;
    if (advance.steps === 0) {
      return {
        message: { type: "frame", tick: this.sim.currentTick(), alpha: advance.alpha },
        transfer: [],
      };
    }

    let previous: Int32Array | undefined;
    const outcomes: CommandOutcome[] = [];
    for (let i = 0; i < advance.steps; i++) {
      if (i === advance.steps - 1 && advance.steps >= 2) {
        previous = this.sim.writeFrame();
      }
      for (const outcome of this.sim.step().outcomes) outcomes.push(outcome);
    }
    const current = this.sim.writeFrame();
    const message: FrameMessage = {
      type: "frame",
      tick: this.sim.currentTick(),
      alpha: advance.alpha,
      current: current.buffer as ArrayBuffer,
      outcomes,
    };
    const transfer: Transferable[] = [message.current!];
    if (previous) {
      message.previous = previous.buffer as ArrayBuffer;
      transfer.push(message.previous);
    }
    const meta = this.sim.metaDelta();
    if (meta !== undefined) message.meta = meta;
    return { message, transfer };
  }

  /** The current tick without advancing time; the first call carries every entity's metadata. */
  snapshot(): Outgoing {
    const current = this.sim.writeFrame().buffer as ArrayBuffer;
    const meta = this.sim.metaDelta();
    return {
      message: {
        type: "snapshot",
        tick: this.sim.currentTick(),
        current,
        ...(meta !== undefined ? { meta } : {}),
      },
      transfer: [current],
    };
  }

  submit(commands: Command[]): Outgoing {
    return {
      message: {
        type: "submitted",
        seqs: commands.map((command) => this.sim.submit(command)),
      },
      transfer: [],
    };
  }

  /**
   * Queues `count` spawn commands for seeded movers, each with a random target in the world.
   *
   * Commands like any other order: they apply at the next tick, so a stress crowd is as
   * reproducible as the rest of the match.
   */
  stress(count: number, seed: number, worldSize: number): Outgoing {
    const random = seeded(seed);
    const point = () => ({ x: random() * worldSize, y: random() * worldSize });
    for (let i = 0; i < count; i++) {
      this.sim.submit({
        type: "spawn",
        template: STRESS_TEMPLATE,
        overrides: { position: point(), move_target: point() },
      });
    }
    return { message: { type: "stressed", count }, transfer: [] };
  }
}
