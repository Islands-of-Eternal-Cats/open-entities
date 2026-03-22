import { describe, it, expect, beforeEach } from "vitest";
import {
  OLD_WASM_FORMAT_HINT,
  resetToEntityFallbackCounterForTests,
  toEntity,
} from "./to-entity";

describe("toEntity", () => {
  beforeEach(() => {
    resetToEntityFallbackCounterForTests();
  });

  it("maps full snapshot with string id (stable bits from WASM)", () => {
    expect(
      toEntity({
        id: "18446744073709551615",
        pos: { x: 1, y: 2 },
        velocity: { vx: 3, vy: 4 },
      })
    ).toEqual({
      id: "18446744073709551615",
      pos: { x: 1, y: 2 },
      velocity: { vx: 3, vy: 4 },
    });
  });

  it("coerces numeric id to string", () => {
    expect(toEntity({ id: 42, pos: { x: 0, y: 0 }, velocity: null })).toEqual({
      id: "42",
      pos: { x: 0, y: 0 },
      velocity: null,
    });
  });

  it("coerces bigint id to string", () => {
    expect(
      toEntity({
        id: 9007199254740993n,
        pos: { x: 0, y: 0 },
        velocity: null,
      })
    ).toEqual({
      id: "9007199254740993",
      pos: { x: 0, y: 0 },
      velocity: null,
    });
  });

  it("uses unique fallback ids when id is missing", () => {
    const a = toEntity({ pos: { x: 0, y: 0 }, velocity: null });
    const b = toEntity({ pos: { x: 1, y: 1 }, velocity: null });
    expect(a.id).toBe("fallback-0");
    expect(b.id).toBe("fallback-1");
  });

  it("throws when pos is missing", () => {
    expect(() => toEntity({ id: "1", velocity: null })).toThrow(
      OLD_WASM_FORMAT_HINT
    );
  });
});
