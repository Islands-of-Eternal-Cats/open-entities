/**
 * Type declarations for the open-entities WASM module (Rust → wasm-pack).
 * Mirrors the public API of wasm-bindings. When wasm-pack is built,
 * pkg/open_entities_wasm.d.ts may be used instead by configuring module resolution.
 */
declare module "open-entities-wasm" {
  /**
   * WASM module initialization; must be called before using any other export.
   * Optional URL/path to the .wasm file; if omitted, wasm-pack uses import.meta.url (can fail in some bundlers).
   */
  export default function init(
    module_or_path?: string | URL | Request | ArrayBuffer
  ): Promise<void>;

  /** JavaScript wrapper for Position component (x, y). */
  export class JsPosition {
    constructor(x: number, y: number);
    x(): number;
    set_x(x: number): void;
    y(): number;
    set_y(y: number): void;
  }

  /** JavaScript wrapper for Velocity component (vx, vy). */
  export class JsVelocity {
    constructor(vx: number, vy: number);
    vx(): number;
    set_vx(vx: number): void;
    vy(): number;
    set_vy(vy: number): void;
  }

  /**
   * Advance position by velocity for one tick (legacy; prefer JsWorld.tick(dt)).
   */
  export function move_position(
    pos: JsPosition,
    vel: JsVelocity
  ): JsPosition;

  /** ECS world: run simulation with delta time and get entity snapshots. */
  export class JsWorld {
    /** Create world from required entity definitions YAML string. */
    constructor(entitiesYaml: string);
    /**
     * Spawn an entity by type name from loaded definitions (e.g. "mover", "static_obstacle").
     * Optional faction id attaches the `Faction` component.
     */
    spawn(typeName: string, faction?: number): void;
    /**
     * Spawn by type name at the given position.
     * Velocity component is not created for this spawn variant.
     * Optional faction id attaches the `Faction` component.
     */
    spawn_at(typeName: string, x: number, y: number, faction?: number): void;
    /** Load initial map instances from YAML string (e.g. assets/init_map.yaml). */
    load_map_yaml(mapYaml: string): void;
    tick(dt: number): void;
    /** Move-to order for snapshot id strings (decimal `Entity::to_bits()`). Target: same `JsPosition` as world coords. */
    order_move_to(entityIds: string[], target: JsPosition): void;
    /** Entity id is string to preserve u64 precision (JS Number safe only to 2^53-1). */
    get_entities(): Array<{
      id: string;
      entityType: string;
      pos: { x: number; y: number };
      velocity: { vx: number; vy: number } | null;
      faction: number | null;
    }>;
  }
}
