/**
 * Message types for main thread ↔ ECS web worker.
 */
import type { Command, CommandOutcome } from "./types";

export type WorkerInMessage =
  | {
      type: "init";
      wasmBuffer: ArrayBuffer;
      /** Entity templates (YAML with an `entities:` root). */
      templatesYaml: string;
      /** Starting layout (YAML with `map` and `spawns`). */
      mapYaml: string;
    }
  /** The current tick's frame and the full metadata, without advancing time; replies `snapshot`. */
  | { type: "snapshot" }
  /**
   * Real time since the previous frame, in milliseconds. The worker accumulates it and runs as
   * many fixed ticks as fit; the reply is a `frame` message.
   */
  | { type: "frame"; elapsedMs: number }
  /**
   * Orders, as commands. The worker hands each to `Simulation.submit`, so it applies at the start
   * of the next tick, and replies `submitted` with the sequence numbers; the outcomes arrive in a
   * later `frame`.
   */
  | { type: "submit"; commands: Command[] }
  /**
   * Spawn `count` seeded movers with random targets inside `[0, worldSize]²`, as `spawn` commands
   * for the next tick. Replies `stressed`; the outcomes are not reported one by one.
   */
  | { type: "stress"; count: number; seed: number; worldSize: number };

/**
 * One frame's reply.
 *
 * Without a step it carries only `tick` and `alpha`: nothing changed, the renderer keeps blending
 * the two frames it holds. With steps it carries `current`, the `Simulation.writeFrame()` buffer of
 * the last tick (transferred, not copied), and after two or more steps `previous`, the buffer of the
 * tick before it; after one step the main thread's old `current` is that tick. `meta` is the JSON
 * from `Simulation.metaDelta()`, present only when something changed. `outcomes` are what the
 * commands applied during these steps did, in `seq` order.
 */
export interface FrameMessage {
  type: "frame";
  tick: number;
  /** How far the frame is between the previous and the current tick, in [0, 1). */
  alpha: number;
  current?: ArrayBuffer;
  previous?: ArrayBuffer;
  meta?: string;
  outcomes?: CommandOutcome[];
}

/** The current tick's frame and, on the first call, every entity's metadata. */
export interface SnapshotMessage {
  type: "snapshot";
  tick: number;
  current: ArrayBuffer;
  meta?: string;
}

export type WorkerOutMessage =
  | { type: "ready" }
  | { type: "error"; message: string }
  | FrameMessage
  | SnapshotMessage
  /** Sequence numbers of the commands in a `submit`, in the same order. */
  | { type: "submitted"; seqs: number[] }
  /** How many spawn commands a `stress` request queued. */
  | { type: "stressed"; count: number };
