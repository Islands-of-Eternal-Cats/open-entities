/**
 * Normalizes raw objects from WASM `get_entities()` into {@link EntitySnapshot}.
 * Rust uses `Entity::to_bits()` serialized as decimal string so ids stay stable across ticks
 * and avoid JS Number precision loss for large u64 values.
 */
import type { EntitySnapshot } from "./types";

export const OLD_WASM_FORMAT_HINT =
  "WASM returned old format (no pos/velocity). Run: cd js-app && ./build-wasm.sh then hard-reload (Ctrl+Shift+R).";

let fallbackIdCounter = 0;

/** Resets fallback id sequence; use in tests only. */
export function resetToEntityFallbackCounterForTests(): void {
  fallbackIdCounter = 0;
}

/**
 * Parse one element from `world.get_entities()` (worker) into a typed snapshot.
 * @throws Error with {@link OLD_WASM_FORMAT_HINT} if `pos` is missing.
 */
export function toEntity(raw: unknown): EntitySnapshot {
  const o = raw as {
    id?: string | number | bigint;
    pos?: { x: number; y: number };
    velocity?: { vx: number; vy: number } | null;
  };
  if (o.pos == null) {
    throw new Error(OLD_WASM_FORMAT_HINT);
  }
  const id =
    typeof o.id === "string"
      ? o.id
      : typeof o.id === "number"
        ? String(o.id)
        : typeof o.id === "bigint"
          ? o.id.toString()
          : `fallback-${fallbackIdCounter++}`;
  const velocity =
    o.velocity != null
      ? { vx: o.velocity.vx, vy: o.velocity.vy }
      : null;
  return {
    id,
    pos: { x: o.pos.x, y: o.pos.y },
    velocity,
  };
}
