/**
 * The main thread's copy of the world: the last two position frames and every entity's metadata.
 *
 * Frames arrive as `Int32Array`s (see `frame-buffer.ts`) and are never unpacked into objects; the
 * renderer walks them once per animation frame through `forEachDrawn`. Metadata arrives as a
 * delta, only when something changed, and is kept per entity index. Snapshot rows
 * (`EntitySnapshot`) are built on demand, for the few entities the HUD shows.
 */
import { TICK_MS } from "./fixed-step";
import {
  FRAME_HEADER_LEN,
  FRAME_STRIDE,
  MILLI_PER_UNIT,
  findRow,
  frameCount,
} from "./frame-buffer";
import type { EntityId, EntitySnapshot, Pos } from "./types";
import { entityIdToKey, keyToEntityId } from "./types";
import type { FrameMessage, SnapshotMessage } from "./worker-types";

/** One entity's metadata as `Simulation.metaDelta()` spells it; absent fields are omitted. */
export interface EntityMeta {
  id: EntityId;
  entity_type?: string;
  faction?: number;
  seats?: number;
  /** Carries a velocity: can move at all. */
  mobile: boolean;
  aboard?: EntityId;
  boarding?: EntityId;
  group?: EntityId;
  /** Map units. */
  move_target?: Pos;
}

/** Payload of `Simulation.metaDelta()`. */
export interface MetaDelta {
  changed: EntityMeta[];
  removed: EntityId[];
}

/** Called for each drawn entity with its interpolated position in map units. */
export type DrawnVisitor = (
  index: number,
  generation: number,
  x: number,
  y: number,
  /** Riding a vehicle: drawn faded, on top of it. */
  aboard: boolean
) => void;

/** Highest blend factor `extrapolate` reaches: the renderer never draws past the current tick. */
const MAX_ALPHA = 0.999;

/** `slotFlags` bits, per entity index. */
const DRAWN = 1;
const ABOARD = 2;

/** What the UI reads; `WorldView` is the implementation, tests substitute their own. */
export interface WorldReader {
  readonly tick: number;
  /** Number of drawn entities. */
  readonly size: number;
  /** Every drawn entity, interpolated, in index order. Allocates nothing per entity. */
  forEachDrawn(visit: DrawnVisitor): void;
  get(key: string): EntitySnapshot | null;
  has(key: string): boolean;
  find(predicate: (entity: EntitySnapshot) => boolean): EntitySnapshot | null;
  /** Keys of the units riding this vehicle. */
  ridersOf(vehicleKey: string): string[];
  /** Keys of the units walking to board this vehicle. */
  walkersTo(vehicleKey: string): string[];
}

/** Drawn entities are the ones spawned from a template; raw ECS entities are not units. */
function drawable(meta: EntityMeta | undefined, generation: number): meta is EntityMeta {
  return meta !== undefined && meta.id.generation === generation && meta.entity_type !== undefined;
}

function addLink(links: Map<string, Set<string>>, to: EntityId | undefined, key: string): void {
  if (!to) return;
  const target = entityIdToKey(to);
  let set = links.get(target);
  if (!set) links.set(target, (set = new Set()));
  set.add(key);
}

function dropLink(links: Map<string, Set<string>>, to: EntityId | undefined, key: string): void {
  if (!to) return;
  const target = entityIdToKey(to);
  const set = links.get(target);
  if (!set) return;
  set.delete(key);
  if (set.size === 0) links.delete(target);
}

export class WorldView implements WorldReader {
  tick = 0;
  /** Blend factor the worker reported with the last frame. */
  alpha = 0;
  /** Blend factor drawn with: `alpha`, advanced by `extrapolate` until the next reply. */
  private displayAlpha = 0;
  private previous: Int32Array | null = null;
  private current: Int32Array | null = null;
  /** By entity index; a stale generation means the slot was reused and the entry is outdated. */
  private readonly meta = new Map<number, EntityMeta>();
  private readonly riders = new Map<string, Set<string>>();
  private readonly walkers = new Map<string, Set<string>>();
  private drawn = 0;
  /**
   * Per entity index, what the per-frame loop needs: the generation the metadata is for and the
   * `DRAWN` / `ABOARD` bits. Typed arrays, so the loop over 100 000 rows does no map lookups.
   */
  private slotGeneration = new Uint32Array(0);
  private slotFlags = new Uint8Array(0);

  get size(): number {
    return this.drawn;
  }

  /** Initial state: the current tick drawn as is, and the full metadata. */
  applySnapshot(message: SnapshotMessage): void {
    this.tick = message.tick;
    this.alpha = 0;
    this.displayAlpha = 0;
    this.current = new Int32Array(message.current);
    this.previous = this.current;
    if (message.meta !== undefined) this.applyMeta(JSON.parse(message.meta) as MetaDelta);
  }

  /**
   * One frame reply. Without buffers only `tick` and `alpha` move. With them, `current` becomes
   * the new tick and `previous` is either the buffer that came along (after two or more steps) or
   * the old `current` (after one).
   */
  applyFrame(message: FrameMessage): void {
    this.tick = message.tick;
    this.alpha = message.alpha;
    this.displayAlpha = message.alpha;
    if (message.current) {
      this.previous = message.previous
        ? new Int32Array(message.previous)
        : this.current;
      this.current = new Int32Array(message.current);
    }
    if (message.meta !== undefined) this.applyMeta(JSON.parse(message.meta) as MetaDelta);
  }

  /**
   * Moves the blend factor on by `ms` of real time since the last reply, up to just below 1.
   *
   * The renderer draws every animation frame, and the worker's reply for this frame may still be
   * on its way; without this the units would stand still until it lands, then jump.
   */
  extrapolate(ms: number): void {
    this.displayAlpha = Math.min(this.alpha + Math.max(0, ms) / TICK_MS, MAX_ALPHA);
  }

  private setSlot(index: number, generation: number, flags: number): void {
    if (index >= this.slotFlags.length) {
      const size = Math.max(index + 1, this.slotFlags.length * 2, 1024);
      const generations = new Uint32Array(size);
      generations.set(this.slotGeneration);
      const bits = new Uint8Array(size);
      bits.set(this.slotFlags);
      this.slotGeneration = generations;
      this.slotFlags = bits;
    }
    this.slotGeneration[index] = generation;
    this.slotFlags[index] = flags;
  }

  private applyMeta(delta: MetaDelta): void {
    for (const id of delta.removed) {
      const old = this.meta.get(id.index);
      if (old && old.id.generation === id.generation) this.forget(id.index, old);
    }
    for (const entity of delta.changed) {
      const old = this.meta.get(entity.id.index);
      if (old) this.forget(entity.id.index, old);
      this.meta.set(entity.id.index, entity);
      const key = entityIdToKey(entity.id);
      addLink(this.riders, entity.aboard, key);
      addLink(this.walkers, entity.boarding, key);
      if (entity.entity_type !== undefined) this.drawn++;
      this.setSlot(
        entity.id.index,
        entity.id.generation,
        (entity.entity_type !== undefined ? DRAWN : 0) |
          (entity.aboard !== undefined ? ABOARD : 0)
      );
    }
  }

  private forget(index: number, old: EntityMeta): void {
    const key = entityIdToKey(old.id);
    dropLink(this.riders, old.aboard, key);
    dropLink(this.walkers, old.boarding, key);
    if (old.entity_type !== undefined) this.drawn--;
    this.meta.delete(index);
    this.setSlot(index, 0, 0);
  }

  /**
   * Merge-join of the two frames on `index`, as `interpolateFrames` does, written out so the
   * 100 000-row loop reads typed arrays only: no map lookup and no inner callback per row.
   */
  forEachDrawn(visit: DrawnVisitor): void {
    const cur = this.current;
    if (!cur) return;
    const prev = this.previous;
    const alpha = this.displayAlpha;
    const generations = this.slotGeneration;
    const flags = this.slotFlags;
    const slots = flags.length;
    const count = frameCount(cur);
    const prevEnd = prev ? FRAME_HEADER_LEN + frameCount(prev) * FRAME_STRIDE : 0;
    let p = FRAME_HEADER_LEN;
    for (let row = 0; row < count; row++) {
      const at = FRAME_HEADER_LEN + row * FRAME_STRIDE;
      const index = cur[at] >>> 0;
      const generation = cur[at + 1] >>> 0;
      if (index >= slots) continue;
      const bits = flags[index];
      if ((bits & DRAWN) === 0 || generations[index] !== generation) continue;
      let x = cur[at + 2];
      let y = cur[at + 3];
      if (prev) {
        while (p < prevEnd && prev[p] >>> 0 < index) p += FRAME_STRIDE;
        if (p < prevEnd && prev[p] >>> 0 === index && prev[p + 1] >>> 0 === generation) {
          x = prev[p + 2] + (x - prev[p + 2]) * alpha;
          y = prev[p + 3] + (y - prev[p + 3]) * alpha;
        }
      }
      visit(index, generation, x / MILLI_PER_UNIT, y / MILLI_PER_UNIT, (bits & ABOARD) !== 0);
    }
  }

  has(key: string): boolean {
    const { index, generation } = keyToEntityId(key);
    return (
      drawable(this.meta.get(index), generation) &&
      this.current !== null &&
      findRow(this.current, index, generation) >= 0
    );
  }

  get(key: string): EntitySnapshot | null {
    const { index, generation } = keyToEntityId(key);
    const meta = this.meta.get(index);
    if (!drawable(meta, generation) || !this.current) return null;
    const at = findRow(this.current, index, generation);
    if (at < 0) return null;
    const cur = this.current;
    let x = cur[at + 2];
    let y = cur[at + 3];
    let vx = 0;
    let vy = 0;
    const prevAt = this.previous ? findRow(this.previous, index, generation) : -1;
    if (this.previous && prevAt >= 0) {
      const px = this.previous[prevAt + 2];
      const py = this.previous[prevAt + 3];
      // Displacement over the last tick: what the Velocity component moved it by.
      vx = x - px;
      vy = y - py;
      x = px + vx * this.displayAlpha;
      y = py + vy * this.displayAlpha;
    }
    const link = (id: EntityId | undefined) => (id ? entityIdToKey(id) : null);
    return {
      id: key,
      entityType: meta.entity_type!,
      pos: { x: x / MILLI_PER_UNIT, y: y / MILLI_PER_UNIT },
      velocity: meta.mobile
        ? { vx: vx / MILLI_PER_UNIT, vy: vy / MILLI_PER_UNIT }
        : null,
      faction: meta.faction ?? null,
      seats: meta.seats ?? null,
      aboard: link(meta.aboard),
      boarding: link(meta.boarding),
      group: link(meta.group),
      moveTarget: meta.move_target ? { ...meta.move_target } : null,
    };
  }

  find(predicate: (entity: EntitySnapshot) => boolean): EntitySnapshot | null {
    if (!this.current) return null;
    const rows = frameCount(this.current);
    for (let row = 0; row < rows; row++) {
      const at = FRAME_HEADER_LEN + row * FRAME_STRIDE;
      const meta = this.meta.get(this.current[at] >>> 0);
      if (!drawable(meta, this.current[at + 1] >>> 0)) continue;
      const entity = this.get(entityIdToKey(meta.id));
      if (entity && predicate(entity)) return entity;
    }
    return null;
  }

  ridersOf(vehicleKey: string): string[] {
    return [...(this.riders.get(vehicleKey) ?? [])].filter((key) => this.has(key));
  }

  walkersTo(vehicleKey: string): string[] {
    return [...(this.walkers.get(vehicleKey) ?? [])].filter((key) => this.has(key));
  }
}
