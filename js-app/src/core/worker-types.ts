/**
 * Message types for main thread ↔ ECS web worker.
 */
import type { Command, CommandOutcome, EntitySnapshot, Pos } from "./types";

/**
 * Raw snapshot row as it crosses the worker boundary.
 * `faction` may be absent on malformed payloads, so the main thread normalizes it to null.
 */
export type RawEntitySnapshot = Omit<
  EntitySnapshot,
  "faction" | "seats" | "aboard" | "boarding" | "moveTarget"
> & {
  faction?: number | null;
  seats?: number | null;
  aboard?: string | null;
  boarding?: string | null;
  moveTarget?: Pos | null;
};

export type WorkerInMessage =
  | {
      type: "init";
      wasmBuffer: ArrayBuffer;
      /** Entity templates (YAML with an `entities:` root). */
      templatesYaml: string;
      /** Starting layout (YAML with `map` and `spawns`). */
      mapYaml: string;
    }
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
  | { type: "submit"; commands: Command[] };

export type WorkerOutMessage =
  | { type: "ready" }
  | { type: "error"; message: string }
  | { type: "entities"; entities: RawEntitySnapshot[] }
  | {
      type: "frame";
      /** State at the current tick. */
      entities: RawEntitySnapshot[];
      /** Positions at the tick before, keyed by entity id. */
      previous: Record<string, Pos>;
      /** How far the frame is between `previous` and `entities`, in [0, 1). */
      alpha: number;
      tick: number;
      /** What the commands applied during this frame's steps did, in `seq` order. */
      outcomes: CommandOutcome[];
    }
  /** Sequence numbers of the commands in a `submit`, in the same order. */
  | { type: "submitted"; seqs: number[] };
