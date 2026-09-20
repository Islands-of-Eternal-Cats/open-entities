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

  /** Stable entity identity, exactly as the world export reports each entity's `id`. */
  export interface EntityId {
    index: number;
    generation: number;
  }

  /** World bounds from the last loaded map. */
  export interface MapBounds {
    width: number;
    height: number;
  }

  /** ECS world: load templates, spawn, order, tick, export. */
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

    /** Whole world as JSON (schema version 3). */
    getWorldAsJson(): string;

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

    /** Advance the simulation; delta is a positive integer number of milliseconds. */
    tick(dtMs: number): void;
  }
}
