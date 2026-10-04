/**
 * Type declarations for the open_entities WASM module (Rust → wasm-pack, `--target web`).
 * Mirrors the `Simulation` surface of `wasm-bindings/src/lib.rs`.
 */
declare module "open_entities_wasm" {
  /**
   * WASM module initialization; must be called before constructing `Simulation`.
   * Takes the fetched `.wasm` bytes as `{ module_or_path }`; without an argument wasm-pack
   * falls back to `import.meta.url`, which does not survive every bundler.
   */
  export interface InitOptions {
    module_or_path?: string | URL | Request | ArrayBuffer;
  }

  export default function init(options?: InitOptions): Promise<void>;

  /** Length of one simulation tick in milliseconds. */
  export function tickMs(): number;

  /** Stable entity identity, exactly as the world export reports each entity's `id`. */
  export interface EntityId {
    index: number;
    generation: number;
  }

  /** A point in map units. */
  export interface Point {
    x: number;
    y: number;
  }

  /**
   * One order, as data — `Command` in the Rust core. Points and radii are in map units;
   * `overrides` takes the same fields as `spawnEntity`.
   */
  export type Command =
    | { type: "spawn"; template: string; overrides?: Record<string, unknown> }
    | { type: "despawn"; ids: EntityId[] }
    | { type: "move_to"; ids: EntityId[]; target: Point }
    | { type: "stop"; ids: EntityId[] }
    | { type: "create_group"; faction: number }
    | { type: "add_to_group"; group: EntityId; unit: EntityId }
    | { type: "remove_from_group"; unit: EntityId }
    | { type: "group_move_to"; group: EntityId; target: Point }
    | { type: "clear_group_manual"; group: EntityId }
    | { type: "create_mission"; target: Point; radius: number }
    | { type: "assign_group"; mission: EntityId; group: EntityId }
    | { type: "unassign_group"; group: EntityId }
    | { type: "board"; units: EntityId[]; vehicle: EntityId }
    | { type: "unboard"; units: EntityId[] };

  /**
   * What one command did. `seq` is the number `submit`/`schedule` returned for it. A creation
   * carries the new id (`spawned`, `group` or `mission`); any other command carries how many of
   * the entities it named it `applied` to and how many it `skipped`.
   */
  export type CommandOutcome =
    | {
        seq: number;
        ok: true;
        spawned?: EntityId;
        group?: EntityId;
        mission?: EntityId;
        applied?: number;
        skipped?: number;
      }
    | { seq: number; ok: false; error: string };

  /** What one `step()` did: the tick it produced and the commands applied at its start. */
  export interface StepReport {
    tick: number;
    outcomes: CommandOutcome[];
  }

  /** World bounds from the last loaded map. */
  export interface MapBounds {
    width: number;
    height: number;
  }

  /** ECS world: load templates, spawn, order, step, export. */
  export class Simulation {
    constructor();

    /** Canonical greeting; handy as a smoke test that the module loaded. */
    hello(): string;

    /** Load entity templates (YAML with an `entities:` root). Replaces any previous set. */
    loadTemplatesYaml(yaml: string): void;

    /** Spawn one entity from a template, with optional component overrides. */
    spawnEntity(templateName: string, overrides: unknown): EntityId;

    /** Spawn a starting layout; returns the ids in file order. */
    loadMapYaml(yaml: string): EntityId[];

    /** Bounds of the last loaded map, or null when none declared them. */
    mapBounds(): MapBounds | null;

    /** Move order for a group; returns how many entities took it. */
    orderMoveTo(ids: EntityId[], x: number, y: number): number;

    /** Zero velocity and drop any move target; returns how many were stopped. */
    orderStop(ids: EntityId[]): number;

    /** Remove entities; returns how many were actually removed. */
    despawn(ids: EntityId[]): number;

    /** True while the entity behind this id is spawned. */
    isAlive(id: EntityId): boolean;

    /** Whole world as JSON (schema version 5). For debugging and saves, not for every frame. */
    getWorldAsJson(): string;

    /**
     * Positions of the current tick: header `[tick_lo, tick_hi, count]`, then per entity
     * `[index, generation, x, y]` in milli-units, sorted by `index`. A fresh array each call, so
     * its buffer can be transferred.
     */
    writeFrame(): Int32Array;

    /**
     * JSON `{ changed, removed }` of the metadata (type, faction, seats, mobile, aboard, boarding,
     * group, move target) that changed since the previous call, or `undefined` when nothing did.
     * The first call reports every positioned entity.
     */
    metaDelta(): string | undefined;

    /** A new empty group for that faction. */
    createGroup(faction: number): EntityId;

    /** Join a group, leaving whatever group the unit was in. */
    addToGroup(group: EntityId, unit: EntityId): void;

    /** Leave whatever group the unit is in; true when it was in one. */
    removeFromGroup(unit: EntityId): boolean;

    /** The unit's group, or null. */
    groupOf(unit: EntityId): EntityId | null;

    /** The group's live members. */
    groupMembers(group: EntityId): EntityId[];

    /** Manual order to a whole group; returns how many members took it. */
    orderGroupMoveTo(group: EntityId, x: number, y: number): number;

    /** True while the player is steering the group by hand. */
    isGroupManual(group: EntityId): boolean;

    /** Hand the group back to automation; true when it was manual. */
    clearGroupManual(group: EntityId): boolean;

    /** A point automation should reach, and how close counts as reached. */
    createMission(x: number, y: number, radius: number): EntityId;

    /** Send a group to a mission. */
    assignGroup(mission: EntityId, group: EntityId): void;

    /** The mission this group is working, or null. */
    missionOf(group: EntityId): EntityId | null;

    /** True once somebody reached the mission. */
    isMissionCompleted(mission: EntityId): boolean;

    /** Put a unit standing next to a vehicle inside it; throws when it cannot go. */
    board(unit: EntityId, vehicle: EntityId): void;
    /**
     * Sends units to walk to a vehicle and get in; returns how many took the order. Throws only
     * when the vehicle has no seats at all.
     */
    orderBoard(units: EntityId[], vehicle: EntityId): number;
    /** The vehicle the unit is walking to board, or null. */
    boardingTargetOf(unit: EntityId): EntityId | null;
    /** Who is on the way to board the vehicle. */
    approaching(vehicle: EntityId): EntityId[];

    /** Let a passenger off, beside the vehicle. */
    unboard(unit: EntityId): void;

    /** What the unit is riding, or null. */
    vehicleOf(unit: EntityId): EntityId | null;

    /** Who is aboard the vehicle right now. */
    passengers(vehicle: EntityId): EntityId[];

    /** Seats still empty, or null when the entity has no seats at all. */
    freeSeats(vehicle: EntityId): number | null;

    /**
     * Queue a command for the next tick; returns its sequence number. Nothing changes until the
     * `step()` that produces that tick, whose report carries the outcome. This is how a host gives
     * orders; the immediate methods above are for tools and tests.
     */
    submit(command: Command): number;

    /** Queue a command for a later tick; throws when `tick` is not after `currentTick()`. */
    schedule(tick: number, command: Command): number;

    /**
     * Advance the simulation by exactly one tick of `tickMs()` milliseconds, applying the commands
     * due first.
     */
    step(): StepReport;

    /** State hash as 16 hex digits; equal on every platform for the same state. */
    stateHash(): string;

    /** Ticks advanced since the simulation was created. */
    currentTick(): number;
  }
}
