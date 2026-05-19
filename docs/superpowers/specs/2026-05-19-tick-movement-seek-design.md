# Design: Simulation Tick, Seek, and Movement

**Date:** 2026-05-19  
**Status:** Approved (brainstorming)  
**Depends on:** [RTS Components and World JSON Export v2](2026-05-16-rts-components-export-v2-design.md), [WASM Spawn + YAML](2026-05-18-wasm-spawn-yaml-design.md), [Component Registry](2026-05-17-component-registry-design.md)  
**Scope:** Add `BaseMoveSpeed`, Bevy `Schedule`-driven `seek` / `movement` systems, `Api::tick(dt_ms)`, WASM `Simulation::tick`, and Node demo tick loop. Export stays schema version 3 with one new optional field.

## Summary

Implement the simulation loop deferred from the RTS components increment: entities with `MoveTarget` and `BaseMoveSpeed` are steered by `seek`; all entities with `Position` and `Velocity` are integrated by `movement` unless they arrived this frame. `Core` owns a `Schedule` registered at construction; `Api::tick(dt_ms)` validates delta time, clamps it, inserts per-tick resources, and runs the schedule.

Delivery surface: **`open-entities-lib`** + **`wasm-bindings`** + **Node demo** (`make wasm-demo`) that spawns `scout` and ticks until arrival at the fixture move target.

## Goals

- `Api::tick(dt_ms: u32) -> Result<(), TickError>` with `dt_ms == 0` → error.
- `dt_ms` clamped to `MAX_DT_MS` (100 ms); delta passed to systems as `SimDelta` resource (seconds, `f32`).
- `BaseMoveSpeed` component registered for YAML spawn, overrides, and `world_json` export.
- `seek` then `movement` via Bevy `Schedule` in `Core` (not inline in `tick`).
- Arrival: snap `Position` to target, remove `MoveTarget`, zero `Velocity`, skip `movement` for that entity in the same frame.
- WASM: `Simulation::tick(dtMs)` with JS validation; demo asserts scout reaches `(20, 0)`.

## Non-Goals

- Browser WASM target, CI workflow changes, schema version bump beyond v3 field addition.
- JSON import, new gameplay components beyond `BaseMoveSpeed`.
- Pathfinding, collision, rotation, acceleration curves, flocking.
- Fixed timestep accumulator or multiple schedule labels (single default schedule is enough).
- `tick` on entities without a loaded world change beyond running systems.
- Pretty-print export, `world_json_pretty()` in the library.

## Decisions (Brainstorming)

| Topic | Choice | Rationale |
|-------|--------|-----------|
| Delivery | Lib + WASM + Node demo | User: option C |
| System hosting | Bevy `Schedule` inside `Core` | User: option 2; idiomatic for growth |
| Speed for seek | `BaseMoveSpeed(f32)` component | User: option A; per-entity via YAML |
| Arrival | Snap position, remove `MoveTarget`, `Velocity = 0`, skip movement same frame | User: option D |
| Arrival threshold | Fixed `0.1` world units | User: option A |
| Movement without order | Integrate any `Position` + `Velocity` | User: option A (inertia) |
| `dt` type | `u32` milliseconds, unsigned | User: unsigned; `0` is error |
| `dt == 0` | `TickError::ZeroDeltaTime` | User: fail, not no-op |
| `dt > MAX_DT_MS` | Clamp to `MAX_DT_MS` | User: upper bound |
| Seek without speed | No-op for that entity | No `BaseMoveSpeed` → seek skips |
| Export schema | v3 + optional `base_move_speed` | Registry-driven flatten |

## Architecture

```text
Node demo / tests
       │ tick(dt_ms)
       ▼
┌──────────────┐
│     Api      │
│  validate dt │
│  clamp dt    │
│  SimDelta    │
│  clear       │
│  Arrived     │
└──────┬───────┘
       │ schedule.run(world)
       ▼
┌──────────────┐     ┌──────────────────┐
│     Core     │────▶│ Schedule         │
│ world        │     │ 1. seek_system   │
│ schedule     │     │ 2. movement_sys  │
└──────────────┘     └────────┬─────────┘
                              │
                              ▼
                     Position / Velocity / components updated
```

### `Core`

```rust
pub struct Core {
    world: World,
    schedule: Schedule,
}
```

- `Core::new()` creates `World`, `Schedule::default()`, registers systems:
  - `schedule.add_systems((seek_system, movement_system).chain());`
- `Core::schedule(&self) -> &Schedule` (optional; tests may use `tick` via `Api` only).
- Initial world insert (once in `Core::new` or first `tick`): `ArrivedThisTick::default()` resource type registered in world.

### `Api::tick`

```rust
pub fn tick(&mut self, dt_ms: u32) -> Result<(), TickError> {
    if dt_ms == 0 {
        return Err(TickError::ZeroDeltaTime);
    }
    let dt_ms = dt_ms.min(MAX_DT_MS);
    let world = self.core_mut().world_mut();
    world.insert_resource(SimDelta::from_ms(dt_ms));
    world.resource_mut::<ArrivedThisTick>().0.clear();
    self.core.schedule.run(world); // or core.run_schedule() wrapper
    Ok(())
}
```

`Core` exposes `fn run_schedule(&mut self)` that calls `self.schedule.run(&mut self.world)` so `Api` does not reach into private fields awkwardly.

### Per-tick resources

| Resource | Set by | Read by |
|----------|--------|---------|
| `SimDelta { dt_secs: f32 }` | `Api::tick` (insert/replace each tick) | `movement_system` |
| `ArrivedThisTick(HashSet<Entity>)` | cleared start of tick; `seek_system` inserts | `movement_system` |

`SimDelta::from_ms(ms: u32) -> Self` computes `dt_secs = ms as f32 / 1000.0`.

### Constants (`systems/mod.rs` or `simulation.rs`)

| Name | Value |
|------|-------|
| `MAX_DT_MS` | `100` |
| `ARRIVAL_THRESHOLD` | `0.1` |

## Systems

### `seek_system`

**Query:** `(Entity, &mut Position, &MoveTarget, &BaseMoveSpeed, &mut Velocity)`  
**Also:** `Commands` (remove `MoveTarget`), `ResMut<ArrivedThisTick>`

For each row:

1. `dx = target.x - position.x`, `dy = target.y - position.y`, `dist = hypot(dx, dy)`.
2. If `dist <= ARRIVAL_THRESHOLD`:
   - Set `position` to `(target.x, target.y)`.
   - Set `velocity` to `{ vx: 0, vy: 0 }`.
   - `commands.entity(entity).remove::<MoveTarget>()`.
   - Insert `entity` into `ArrivedThisTick`.
   - Continue to next entity (do not set velocity toward target).
3. Else if `dist > 0.0` (use epsilon only if needed for float safety):
   - `velocity = normalize(dx, dy) * base_move_speed.0`.
4. Else (`dist == 0` but above threshold branch failed): treat as arrival (same as step 2).

Entities with `MoveTarget` but **without** `BaseMoveSpeed` are not in this query and are unchanged by seek.

### `movement_system`

**Query:** `(Entity, &mut Position, &Velocity)`  
**Also:** `Res<ArrivedThisTick>`, `Res<SimDelta>`

For each row:

- If `arrived.0.contains(&entity)`, skip.
- Else: `position.x += velocity.vx * dt_secs`, `position.y += velocity.vy * dt_secs`.

Runs **after** `seek_system` via schedule chain.

## Component: `BaseMoveSpeed`

```rust
/// Maximum travel speed used by seek (world units per second).
#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BaseMoveSpeed(pub f32);
```

- File: `open-entities-lib/src/components/base_move_speed.rs`
- Public path: `open_entities::components::BaseMoveSpeed`
- Registry: `register_component!(base_move_speed, BaseMoveSpeed);`
- YAML key: `base_move_speed: 2.0`
- JSON export: `"base_move_speed": 2.0` (transparent float)

Unit test: spawn + query round-trip (same pattern as `velocity.rs`).

## Fixture and demo data

Update `fixtures/spawn_entity_templates.yaml` — `scout` entry:

```yaml
  scout:
    template: unit
    position: { x: 10.0, y: 5.0 }
    base_move_speed: 2.0
    move_target: { x: 20.0, y: 0.0 }
    health:
      current: 80
      max: 100
```

Remove `velocity` from scout template (seek sets velocity each frame while moving). Other templates unchanged unless tests require speed elsewhere.

**Node demo (`wasm-bindings/demo/run.mjs`):**

- After existing spawn/export checks, run ~120 iterations: `sim.tick(16)` (`dt_ms = 16` ≈ 60 Hz).
- Parse `getWorldAsJson()`, find scout, assert final `position` near `(20, 0)` within tolerance (e.g. `0.01`), assert `move_target` key absent.
- Log one-line progress optional (`console.log` every 30 ticks).

**WASM tests:** add `tick_advances_scout` — load fixture, spawn scout with position override `(50, 25)`, call `tick(16)` multiple times, assert position changes and eventually stabilizes.

## Public API

### Rust

```rust
// open-entities-lib/src/api.rs or simulation.rs
pub enum TickError {
    ZeroDeltaTime,
}

impl Api {
    pub fn tick(&mut self, dt_ms: u32) -> Result<(), TickError>;
}
```

Re-export `TickError` from crate root if other modules need it in tests.

### WASM

```rust
#[wasm_bindgen(js_name = tick)]
pub fn tick(&mut self, dt_ms: u32) -> Result<(), JsValue> {
    self.api
        .tick(dt_ms)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}
```

**JS boundary:** accept `number`. Reject if `!Number.isFinite(dtMs) || dtMs <= 0 || !Number.isInteger(dtMs)` before casting to `u32`; reject if `dtMs > u32::MAX`. Map `TickError` to string `JsValue`.

| Rust | JavaScript |
|------|------------|
| `tick` | `tick(dtMs)` |

## Repository layout

```text
open-entities-lib/src/
├── api.rs                    # tick() forwards to Core
├── core.rs                   # world + schedule, run_schedule()
├── systems/
│   ├── mod.rs                # ARRIVAL_THRESHOLD, MAX_DT_MS, re-exports
│   ├── seek.rs               # seek_system
│   └── movement.rs           # movement_system
├── simulation.rs             # SimDelta, ArrivedThisTick, TickError (optional split)
└── components/
    └── base_move_speed.rs

wasm-bindings/src/lib.rs       # Simulation::tick
wasm-bindings/demo/run.mjs     # tick loop + assertions
fixtures/spawn_entity_templates.yaml
```

`lib.rs` adds `pub mod systems;` and re-exports as needed.

## Error handling

| API | Condition | Result |
|-----|-----------|--------|
| `Api::tick(0)` | zero delta | `Err(TickError::ZeroDeltaTime)` |
| `Api::tick(n)` | `n > MAX_DT_MS` | clamp, `Ok(())` |
| `Simulation::tick` (JS) | non-integer, NaN, `<= 0` | `Err(JsValue)` before Rust |
| ECS / schedule | misconfiguration | panic in dev (same as rest of crate) |

`TickError` implements `Display` + `Error`; message for zero delta: `"tick delta must be greater than zero"`.

## Testing

| Test | Location | Assertion |
|------|----------|-----------|
| `base_move_speed_component_round_trip` | `base_move_speed.rs` | Component insert/query |
| `tick_zero_delta_fails` | `api` or `systems` tests | `tick(0)` → `ZeroDeltaTime` |
| `tick_clamps_large_dt` | systems tests | `tick(500)` same displacement as `tick(100)` for one step |
| `seek_sets_velocity_toward_target` | `seek.rs` | One tick, direction normalized, magnitude = speed |
| `seek_arrival_snaps_and_removes_target` | `seek.rs` | Entity at threshold: position = target, no `MoveTarget`, velocity zero |
| `movement_skips_arrived_same_frame` | integration | After arrival tick, position exactly target (no overshoot) |
| `scout_reaches_move_target` | integration | Spawn scout components, many `tick(16)`, distance to `(20,0)` < epsilon |
| `tick_advances_scout` | `wasm-bindings` wasm test | WASM path smoke |
| Node demo | `run.mjs` | End-to-end tick + JSON assert |

Verification (repo root):

```bash
cargo test -p open_entities
cargo clippy -p open_entities -- -D warnings
make wasm-check
```

## README

- Document `Api::tick(dt_ms)`, `TickError`, `BaseMoveSpeed`, and system order.
- WASM section: `sim.tick(16)`, error on `tick(0)`.
- Update component list and JSON example with optional `base_move_speed`.

## Alternatives considered

| Approach | Why not |
|----------|---------|
| Procedural `fn seek(world)` without Schedule | User chose Schedule in `Core` |
| `f32` seconds `tick(dt)` with no-op on `<= 0` | User chose unsigned ms + error on zero |
| Preserve template `velocity` for scout | Seek overwrites each frame; redundant |
| `ArrivalRadius` component | User chose fixed threshold |
| Movement only when `MoveTarget` present | User chose inertia for any `Velocity` |
| Schema v4 for one field | Registry flatten keeps v3 |

## Out of scope (follow-ups)

- Multiple schedules (`FixedUpdate`, `Gameplay`, …).
- `Commands`-based spawn during tick, events, networking.
- Replacing `HashSet` arrival tracking with a marker component.
- Rust example `tick_scout.rs` (optional; demo + tests are sufficient unless plan adds it).

## License note

No new crates. `bevy_ecs` already MIT/Apache-2.0; compatible with GPL-3.0-or-later.
