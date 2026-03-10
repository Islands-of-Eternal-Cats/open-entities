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
  export default function init(module_or_path?: string | URL | Request): Promise<void>;

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
   * Advance position by velocity for one tick (delta = 1 time unit).
   * Returns a new JsPosition; does not mutate inputs.
   */
  export function move_position(
    pos: JsPosition,
    vel: JsVelocity
  ): JsPosition;
}
