# Simulation Tick, Seek, and Movement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `BaseMoveSpeed`, Bevy `Schedule`-driven `seek` / `movement` systems, `Api::tick(dt_ms)`, WASM `Simulation::tick`, and a Node demo tick loop that proves scout reaches `(20, 0)`.

**Architecture:** `Core` owns `World` + `Schedule` with chained `seek_system` → `movement_system`. `Api::tick` validates/clamps delta time, sets `SimDelta` and clears `ArrivedThisTick`, then runs the schedule. `BaseMoveSpeed` joins the component registry for YAML/spawn/export. WASM forwards `tick` with JS integer validation.

**Tech Stack:** Rust 2024, `bevy_ecs 0.18` (`Schedule`, `System`, `Resource`, `Commands`), `serde` / `serde_yaml`, `wasm-bindgen`, Node ESM.

**Spec:** `docs/superpowers/specs/2026-05-19-tick-movement-seek-design.md`

---

## File map

| File | Responsibility |
|------|----------------|
| `open-entities-lib/src/components/base_move_speed.rs` | `BaseMoveSpeed(f32)` + round-trip test |
| `open-entities-lib/src/components/mod.rs` | Re-export `BaseMoveSpeed` |
| `open-entities-lib/src/component_registry/registered.rs` | `register_component!(base_move_speed, BaseMoveSpeed)` |
| `open-entities-lib/src/simulation.rs` | `SimDelta`, `ArrivedThisTick`, `TickError`, `Api::tick` |
| `open-entities-lib/src/systems/mod.rs` | `MAX_DT_MS`, `ARRIVAL_THRESHOLD`, module wiring |
| `open-entities-lib/src/systems/seek.rs` | `seek_system` + unit tests |
| `open-entities-lib/src/systems/movement.rs` | `movement_system` + unit tests |
| `open-entities-lib/src/core.rs` | `schedule: Schedule`, `run_schedule()`, init resources |
| `open-entities-lib/src/lib.rs` | `mod simulation; mod systems;`, re-export `TickError` |
| `fixtures/spawn_entity_templates.yaml` | Scout: `base_move_speed`, no `velocity` |
| `wasm-bindings/src/lib.rs` | `Simulation::tick`, `tick_advances_scout` test |
| `wasm-bindings/demo/run.mjs` | Second `Simulation` tick loop + assertions |
| `README.md` | `tick`, `BaseMoveSpeed`, WASM `tick(16)` |

**Unchanged:** Schema version `3`, export shape (one new optional flattened key), `make wasm-check` targets.

---

### Task 1: `BaseMoveSpeed` component + registry

**Files:**
- Create: `open-entities-lib/src/components/base_move_speed.rs`
- Modify: `open-entities-lib/src/components/mod.rs`
- Modify: `open-entities-lib/src/component_registry/registered.rs`
- Modify: `open-entities-lib/src/component_registry/mod.rs` (test helper)

- [ ] **Step 1: Create component + failing-style test (new file)**

Create `open-entities-lib/src/components/base_move_speed.rs`:

```rust
use bevy_ecs::prelude::Component;
use serde::{Deserialize, Serialize};

/// Maximum travel speed used by seek (world units per second).
#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BaseMoveSpeed(pub f32);

#[cfg(test)]
mod tests {
    use super::BaseMoveSpeed;
    use bevy_ecs::prelude::*;

    #[test]
    fn base_move_speed_component_round_trip() {
        let mut world = World::new();
        world.spawn(BaseMoveSpeed(2.0));

        let mut query = world.query::<&BaseMoveSpeed>();
        let mut count = 0;
        for speed in query.iter(&world) {
            assert_eq!(speed.0, 2.0);
            count += 1;
        }
        assert_eq!(count, 1);
    }
}
```

- [ ] **Step 2: Wire `components/mod.rs`**

Add module + re-export:

```rust
pub mod base_move_speed;
// ...
pub use base_move_speed::BaseMoveSpeed;
```

- [ ] **Step 3: Register component**

In `open-entities-lib/src/component_registry/registered.rs`, add import and line (order after `move_target` is fine):

```rust
use crate::components::{Faction, Health, MoveTarget, Position, Velocity, BaseMoveSpeed};

define_registered_components! {
    register_component!(position, Position);
    register_component!(velocity, Velocity);
    register_component!(faction, Faction);
    register_component!(move_target, MoveTarget);
    register_component!(base_move_speed, BaseMoveSpeed);
    register_component!(health, Health);
}
```

- [ ] **Step 4: Extend registry unit test `entity_components_has_any`**

In `open-entities-lib/src/component_registry/mod.rs`, update `entity_components_has_any` helper:

```rust
fn entity_components_has_any(doc: &EntityComponents) -> bool {
    doc.position.is_some()
        || doc.velocity.is_some()
        || doc.faction.is_some()
        || doc.move_target.is_some()
        || doc.base_move_speed.is_some()
        || doc.health.is_some()
}
```

Add test:

```rust
#[test]
fn entity_components_has_any_detects_base_move_speed() {
    let doc = EntityComponents {
        base_move_speed: Some(BaseMoveSpeed(1.5)),
        ..Default::default()
    };
    assert!(entity_components_has_any(&doc));
}
```

(Add `use crate::components::BaseMoveSpeed;` at top of test module.)

- [ ] **Step 5: Run tests**

```bash
cargo test -p open_entities base_move_speed -- --nocapture
```

Expected: `base_move_speed_component_round_trip ... ok` and registry test ok.

- [ ] **Step 6: Commit**

```bash
git add open-entities-lib/src/components/base_move_speed.rs \
  open-entities-lib/src/components/mod.rs \
  open-entities-lib/src/component_registry/registered.rs \
  open-entities-lib/src/component_registry/mod.rs
git commit -m "feat: add BaseMoveSpeed component and registry entry"
```

---

### Task 2: Simulation types and `TickError`

**Files:**
- Create: `open-entities-lib/src/simulation.rs`
- Modify: `open-entities-lib/src/lib.rs`

- [ ] **Step 1: Create `simulation.rs`**

```rust
use std::collections::HashSet;

use bevy_ecs::prelude::{Entity, Resource};

use crate::systems::{MAX_DT_MS, ARRIVAL_THRESHOLD};

/// Per-tick delta time in seconds (from clamped `dt_ms`).
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct SimDelta {
    pub dt_secs: f32,
}

impl SimDelta {
    #[must_use]
    pub const fn from_ms(ms: u32) -> Self {
        Self {
            dt_secs: ms as f32 / 1000.0,
        }
    }
}

/// Entities that arrived this tick; `movement_system` skips them.
#[derive(Resource, Debug, Default)]
pub struct ArrivedThisTick(pub HashSet<Entity>);

/// Errors from [`crate::api::Api::tick`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickError {
    ZeroDeltaTime,
}

impl std::fmt::Display for TickError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroDeltaTime => f.write_str("tick delta must be greater than zero"),
        }
    }
}

impl std::error::Error for TickError {}

// Re-export constants for tests/docs (defined in systems/mod.rs).
pub use crate::systems::{ARRIVAL_THRESHOLD, MAX_DT_MS};
```

Note: `simulation.rs` references `systems` — create a minimal `systems/mod.rs` first in Task 3 Step 1 with only constants, **or** define constants in `simulation.rs` temporarily and move them in Task 3. **Recommended order:** do Task 3 Step 1 (constants-only `systems/mod.rs`) before Task 2 Step 1, or put constants in `systems/mod.rs` in Task 2 and import from there.

**Practical order for workers:** Task 3 Step 1 (constants + empty modules) → Task 2 → Task 3 rest.

- [ ] **Step 2: Add `mod simulation` to `lib.rs`**

```rust
mod simulation;
pub mod systems;

pub use simulation::TickError;
```

- [ ] **Step 3: Verify compile**

```bash
cargo check -p open_entities
```

Expected: compiles once `systems/mod.rs` exists (Task 3).

- [ ] **Step 4: Commit**

```bash
git add open-entities-lib/src/simulation.rs open-entities-lib/src/lib.rs
git commit -m "feat: add simulation tick resources and TickError"
```

---

### Task 3: `systems` module — constants, `seek_system`, `movement_system`

**Files:**
- Create: `open-entities-lib/src/systems/mod.rs`
- Create: `open-entities-lib/src/systems/seek.rs`
- Create: `open-entities-lib/src/systems/movement.rs`

- [ ] **Step 1: Create `systems/mod.rs` with constants**

```rust
pub mod movement;
pub mod seek;

pub use movement::movement_system;
pub use seek::seek_system;

/// Maximum allowed tick delta (milliseconds); larger values are clamped.
pub const MAX_DT_MS: u32 = 100;

/// Distance at or below which seek treats the entity as arrived (world units).
pub const ARRIVAL_THRESHOLD: f32 = 0.1;
```

- [ ] **Step 2: Implement `seek_system`**

Create `open-entities-lib/src/systems/seek.rs`:

```rust
use bevy_ecs::prelude::*;

use crate::components::{BaseMoveSpeed, MoveTarget, Position, Velocity};
use crate::simulation::ArrivedThisTick;

use super::ARRIVAL_THRESHOLD;

pub fn seek_system(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &mut Position,
        &MoveTarget,
        &BaseMoveSpeed,
        &mut Velocity,
    )>,
    mut arrived: ResMut<ArrivedThisTick>,
) {
    for (entity, mut position, target, speed, mut velocity) in &mut query {
        let dx = target.x - position.x;
        let dy = target.y - position.y;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist <= ARRIVAL_THRESHOLD {
            position.x = target.x;
            position.y = target.y;
            velocity.vx = 0.0;
            velocity.vy = 0.0;
            commands.entity(entity).remove::<MoveTarget>();
            arrived.0.insert(entity);
            continue;
        }

        if dist > 0.0 {
            let inv = speed.0 / dist;
            velocity.vx = dx * inv;
            velocity.vy = dy * inv;
        } else {
            position.x = target.x;
            position.y = target.y;
            velocity.vx = 0.0;
            velocity.vy = 0.0;
            commands.entity(entity).remove::<MoveTarget>();
            arrived.0.insert(entity);
        }
    }
}
```

- [ ] **Step 3: Implement `movement_system`**

Create `open-entities-lib/src/systems/movement.rs`:

```rust
use bevy_ecs::prelude::*;

use crate::components::{Position, Velocity};
use crate::simulation::{ArrivedThisTick, SimDelta};

pub fn movement_system(
    mut query: Query<(Entity, &mut Position, &Velocity)>,
    arrived: Res<ArrivedThisTick>,
    delta: Res<SimDelta>,
) {
    for (entity, mut position, velocity) in &mut query {
        if arrived.0.contains(&entity) {
            continue;
        }
        position.x += velocity.vx * delta.dt_secs;
        position.y += velocity.vy * delta.dt_secs;
    }
}
```

- [ ] **Step 4: Commit systems skeleton**

```bash
git add open-entities-lib/src/systems/
git commit -m "feat: add seek and movement ECS systems"
```

---

### Task 4: `seek` unit tests

**Files:**
- Modify: `open-entities-lib/src/systems/seek.rs`

- [ ] **Step 1: Add test helpers and `seek_sets_velocity_toward_target`**

Append to `seek.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{BaseMoveSpeed, MoveTarget, Position, Velocity};
    use crate::simulation::{ArrivedThisTick, SimDelta};
    use bevy_ecs::prelude::*;

    fn run_seek(world: &mut World) {
        let mut schedule = Schedule::default();
        schedule.add_systems(seek_system);
        world.insert_resource(ArrivedThisTick::default());
        world.insert_resource(SimDelta::from_ms(16));
        schedule.run(world);
    }

    #[test]
    fn seek_sets_velocity_toward_target() {
        let mut world = World::new();
        world.spawn((
            Position { x: 0.0, y: 0.0 },
            MoveTarget { x: 3.0, y: 4.0 },
            BaseMoveSpeed(10.0),
            Velocity { vx: 0.0, vy: 0.0 },
        ));

        run_seek(&mut world);

        let velocity = world
            .query::<&Velocity>()
            .single(&world)
            .expect("velocity");
        assert!((velocity.vx - 6.0).abs() < 1e-5);
        assert!((velocity.vy - 8.0).abs() < 1e-5);
    }
}
```

- [ ] **Step 2: Run test (expect pass after implementation)**

```bash
cargo test -p open_entities seek_sets_velocity_toward_target -- --nocapture
```

Expected: `ok`

- [ ] **Step 3: Add `seek_arrival_snaps_and_removes_target`**

```rust
    #[test]
    fn seek_arrival_snaps_and_removes_target() {
        let mut world = World::new();
        let entity = world
            .spawn((
                Position { x: 19.95, y: 0.0 },
                MoveTarget { x: 20.0, y: 0.0 },
                BaseMoveSpeed(2.0),
                Velocity { vx: 1.0, vy: 0.0 },
            ))
            .id();

        run_seek(&mut world);
        // Apply deferred command buffer so MoveTarget removal is visible.
        world.flush();

        let position = world.get::<Position>(entity).expect("position");
        assert_eq!(position.x, 20.0);
        assert_eq!(position.y, 0.0);
        let velocity = world.get::<Velocity>(entity).expect("velocity");
        assert_eq!(velocity.vx, 0.0);
        assert_eq!(velocity.vy, 0.0);
        assert!(world.get::<MoveTarget>(entity).is_none());
    }
```

- [ ] **Step 4: Run arrival test**

```bash
cargo test -p open_entities seek_arrival_snaps_and_removes_target -- --nocapture
```

Expected: `ok` (if `flush` is unavailable on `World`, use a one-system schedule that includes `ApplyDeferred` after seek, or run `schedule.run` twice — prefer `world.flush()` from `bevy_ecs::world::World`).

- [ ] **Step 5: Commit**

```bash
git add open-entities-lib/src/systems/seek.rs
git commit -m "test: seek velocity and arrival behavior"
```

---

### Task 5: `movement` unit test + `tick` clamp test

**Files:**
- Modify: `open-entities-lib/src/systems/movement.rs`
- Modify: `open-entities-lib/src/simulation.rs` (add `Api::tick` — or separate Task 6)

- [ ] **Step 1: `movement_integrates_velocity` test**

Append to `movement.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Position, Velocity};
    use crate::simulation::{ArrivedThisTick, SimDelta};
    use bevy_ecs::prelude::*;

    #[test]
    fn movement_integrates_velocity() {
        let mut world = World::new();
        world.spawn((
            Position { x: 0.0, y: 0.0 },
            Velocity { vx: 10.0, vy: 0.0 },
        ));
        world.insert_resource(SimDelta::from_ms(100));
        world.insert_resource(ArrivedThisTick::default());

        let mut schedule = Schedule::default();
        schedule.add_systems(movement_system);
        schedule.run(&mut world);

        let position = world.query::<&Position>().single(&world).expect("position");
        assert!((position.x - 1.0).abs() < 1e-5);
        assert!((position.y - 0.0).abs() < 1e-5);
    }
}
```

- [ ] **Step 2: Run test**

```bash
cargo test -p open_entities movement_integrates_velocity -- --nocapture
```

Expected: `ok`

- [ ] **Step 3: Commit**

```bash
git add open-entities-lib/src/systems/movement.rs
git commit -m "test: movement integrates velocity with SimDelta"
```

---

### Task 6: `Core` schedule + `Api::tick`

**Files:**
- Modify: `open-entities-lib/src/core.rs`
- Modify: `open-entities-lib/src/simulation.rs`
- Modify: `open-entities-lib/src/api.rs`

- [ ] **Step 1: Update `Core`**

Replace `open-entities-lib/src/core.rs` with:

```rust
use bevy_ecs::prelude::{Schedule, World};
use bevy_ecs::schedule::ScheduleLabel;

use crate::simulation::ArrivedThisTick;
use crate::systems::{movement_system, seek_system};

#[derive(ScheduleLabel, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SimulationSchedule;

/// Owns the ECS [`World`] and gameplay [`Schedule`] for a simulation instance.
pub struct Core {
    world: World,
    schedule: Schedule,
}

impl Core {
    /// Creates an empty world and registers seek → movement systems.
    #[must_use]
    pub fn new() -> Self {
        let mut world = World::new();
        world.insert_resource(ArrivedThisTick::default());

        let mut schedule = Schedule::new(SimulationSchedule);
        schedule.add_systems((seek_system, movement_system).chain());

        Self { world, schedule }
    }

    /// Immutable access to the underlying ECS world.
    #[must_use]
    pub const fn world(&self) -> &World {
        &self.world
    }

    /// Mutable access to the underlying ECS world.
    pub const fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Immutable access to the simulation schedule.
    #[must_use]
    pub const fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    /// Runs the simulation schedule on the world.
    pub fn run_schedule(&mut self) {
        self.schedule.run(&mut self.world);
    }
}

impl Default for Core {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 2: Implement `Api::tick` in `simulation.rs`**

Add to `simulation.rs`:

```rust
use crate::api::Api;

impl Api {
    /// Advances simulation by `dt_ms` milliseconds (clamped to [`MAX_DT_MS`]).
    ///
    /// # Errors
    ///
    /// Returns [`TickError::ZeroDeltaTime`] when `dt_ms == 0`.
    pub fn tick(&mut self, dt_ms: u32) -> Result<(), TickError> {
        if dt_ms == 0 {
            return Err(TickError::ZeroDeltaTime);
        }
        let dt_ms = dt_ms.min(MAX_DT_MS);
        let core = self.core_mut();
        let world = core.world_mut();
        world.insert_resource(SimDelta::from_ms(dt_ms));
        world.resource_mut::<ArrivedThisTick>().0.clear();
        core.run_schedule();
        Ok(())
    }
}
```

- [ ] **Step 3: Add tick API tests in `simulation.rs`**

```rust
#[cfg(test)]
mod api_tests {
    use super::*;
    use crate::api::Api;
    use crate::components::{BaseMoveSpeed, MoveTarget, Position, Velocity};

    #[test]
    fn tick_zero_delta_fails() {
        let mut api = Api::new();
        let err = api.tick(0).unwrap_err();
        assert_eq!(err, TickError::ZeroDeltaTime);
    }

    #[test]
    fn tick_clamps_large_dt() {
        let mut api = Api::new();
        let entity = api
            .core_mut()
            .world_mut()
            .spawn((
                Position { x: 0.0, y: 0.0 },
                Velocity { vx: 1.0, vy: 0.0 },
            ))
            .id();

        api.tick(500).expect("tick with clamp");
        let pos_after_500 = api.core_mut().world().get::<Position>(entity).unwrap().x;

        let mut api2 = Api::new();
        let entity2 = api2
            .core_mut()
            .world_mut()
            .spawn((
                Position { x: 0.0, y: 0.0 },
                Velocity { vx: 1.0, vy: 0.0 },
            ))
            .id();
        api2.tick(100).expect("tick at cap");
        let pos_after_100 = api2.core_mut().world().get::<Position>(entity2).unwrap().x;

        assert!((pos_after_500 - pos_after_100).abs() < 1e-5);
    }
}
```

- [ ] **Step 4: Run tick tests**

```bash
cargo test -p open_entities tick_zero_delta_fails tick_clamps_large_dt -- --nocapture
```

Expected: both `ok`

- [ ] **Step 5: Commit**

```bash
git add open-entities-lib/src/core.rs open-entities-lib/src/simulation.rs
git commit -m "feat: Core schedule and Api::tick"
```

---

### Task 7: Integration tests — arrival skip + scout reaches target

**Files:**
- Create: `open-entities-lib/tests/tick_scout.rs` (integration test crate) **or** add module under `simulation.rs` / `systems` — prefer **`open-entities-lib/src/simulation.rs` `mod integration_tests`** to avoid new test harness.

Use `#[cfg(test)] mod integration_tests` in `simulation.rs`:

- [ ] **Step 1: `movement_skips_arrived_same_frame`**

```rust
    #[test]
    fn movement_skips_arrived_same_frame() {
        let mut api = Api::new();
        let entity = api
            .core_mut()
            .world_mut()
            .spawn((
                Position { x: 19.95, y: 0.0 },
                MoveTarget { x: 20.0, y: 0.0 },
                BaseMoveSpeed(2.0),
                Velocity { vx: 100.0, vy: 0.0 },
            ))
            .id();

        api.tick(16).expect("tick");

        let world = api.core_mut().world();
        let position = world.get::<Position>(entity).expect("position");
        assert!((position.x - 20.0).abs() < 1e-4);
        assert!((position.y - 0.0).abs() < 1e-4);
        assert!(world.get::<MoveTarget>(entity).is_none());
    }
```

- [ ] **Step 2: `scout_reaches_move_target`**

```rust
    const FIXTURE_YAML: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/spawn_entity_templates.yaml"
    ));

    #[test]
    fn scout_reaches_move_target() {
        let mut api = Api::new();
        api.load_templates_yaml(FIXTURE_YAML)
            .expect("load fixture");
        let entity = api
            .spawn_entity("scout", Default::default())
            .expect("spawn scout");

        for _ in 0..1000 {
            api.tick(16).expect("tick");
            if api.core_mut().world().get::<MoveTarget>(entity).is_none() {
                break;
            }
        }

        let world = api.core_mut().world();
        let position = world.get::<Position>(entity).expect("position");
        assert!((position.x - 20.0).abs() < 0.01);
        assert!((position.y - 0.0).abs() < 0.01);
        assert!(world.get::<MoveTarget>(entity).is_none());
    }
```

Add imports: `use crate::import::ImportError` only if needed; `EntityComponents::default()` via `Default`.

- [ ] **Step 3: Run integration tests**

```bash
cargo test -p open_entities movement_skips_arrived_same_frame scout_reaches_move_target -- --nocapture
```

Expected: both `ok` (requires fixture update in Task 8 — run Task 8 before Step 3 if scout test fails on missing `base_move_speed` in YAML).

- [ ] **Step 4: Commit**

```bash
git add open-entities-lib/src/simulation.rs
git commit -m "test: tick integration for arrival and scout path"
```

---

### Task 8: Fixture + export test for `base_move_speed`

**Files:**
- Modify: `fixtures/spawn_entity_templates.yaml`
- Modify: `open-entities-lib/src/export/mod.rs`

- [ ] **Step 1: Update scout template**

In `fixtures/spawn_entity_templates.yaml`, replace the `scout` block with:

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

(Remove `velocity` line.)

- [ ] **Step 2: Add export test**

In `open-entities-lib/src/export/mod.rs` tests, add:

```rust
    use crate::components::BaseMoveSpeed;

    #[test]
    fn world_json_v3_base_move_speed() {
        let mut api = Api::new();
        api.core_mut()
            .world_mut()
            .spawn((
                Position { x: 1.0, y: 2.0 },
                BaseMoveSpeed(2.5),
            ));

        let json = api.world_json().expect("serialize world");
        let value: serde_json::Value = serde_json::from_str(&json).expect("parse JSON");
        let entity = &value["entities"][0];
        assert_eq!(entity["base_move_speed"], 2.5);
    }
```

- [ ] **Step 3: Verify example + tests**

```bash
cargo run -p open_entities --example spawn_entity
cargo test -p open_entities scout_reaches_move_target world_json_v3_base_move_speed -- --nocapture
```

Expected: example prints five spawns; tests pass.

- [ ] **Step 4: Commit**

```bash
git add fixtures/spawn_entity_templates.yaml open-entities-lib/src/export/mod.rs
git commit -m "chore: scout fixture uses base_move_speed; export test"
```

---

### Task 9: WASM `Simulation::tick` + tests

**Files:**
- Modify: `wasm-bindings/src/lib.rs`

- [ ] **Step 1: Add `tick` binding**

Inside `impl Simulation`, add:

```rust
    /// JS: `tick(dtMs)` — positive integer milliseconds only.
    #[wasm_bindgen(js_name = tick)]
    pub fn tick(&mut self, dt_ms: f64) -> Result<(), JsValue> {
        if !dt_ms.is_finite() || dt_ms <= 0.0 || dt_ms.fract() != 0.0 {
            return Err(JsValue::from_str(
                "tick(dtMs) requires a positive finite integer",
            ));
        }
        if dt_ms > f64::from(u32::MAX) {
            return Err(JsValue::from_str("tick(dtMs) exceeds u32::MAX"));
        }
        let dt_ms = dt_ms as u32;
        self.api
            .tick(dt_ms)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
```

Add at top of file if needed: `use open_entities::TickError;` (only if used; mapping uses `Display`).

- [ ] **Step 2: Add `tick_advances_scout` wasm test**

In `wasm_tests` module:

```rust
    #[wasm_bindgen_test]
    fn tick_advances_scout() {
        let mut sim = Simulation::new();
        sim.load_templates_yaml(FIXTURE_YAML)
            .expect("load fixture");
        sim.spawn_entity("scout", scout_overrides())
            .expect("spawn scout");

        let before = sim.world_json().expect("export before");
        let before_val: serde_json::Value =
            serde_json::from_str(&before).expect("parse JSON");
        let scout_before = before_val["entities"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["entity_type"] == "scout")
            .expect("scout row");
        let x0 = scout_before["position"]["x"].as_f64().unwrap();

        for _ in 0..60 {
            sim.tick(16.0).expect("tick");
        }

        let after = sim.world_json().expect("export after");
        let after_val: serde_json::Value =
            serde_json::from_str(&after).expect("parse JSON");
        let scout_after = after_val["entities"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["entity_type"] == "scout")
            .expect("scout row");
        let x1 = scout_after["position"]["x"].as_f64().unwrap();

        assert_ne!(x0, x1, "position should change after ticks");
    }

    #[wasm_bindgen_test]
    fn tick_zero_rejected() {
        let mut sim = Simulation::new();
        let err = sim.tick(0.0).unwrap_err();
        let msg = err.as_string().expect("string error");
        assert!(msg.contains("greater than zero"), "got: {msg}");
    }
```

- [ ] **Step 3: Run wasm tests**

```bash
make wasm-test
```

Expected: all wasm tests pass including new ones.

- [ ] **Step 4: Commit**

```bash
git add wasm-bindings/src/lib.rs
git commit -m "feat(wasm): expose Simulation::tick with JS validation"
```

---

### Task 10: Node demo tick loop

**Files:**
- Modify: `wasm-bindings/demo/run.mjs`

- [ ] **Step 1: Append tick demo block**

After existing spawn/export assertions, add:

```javascript
// --- Tick demo: scout walks to move_target (fixture pose, no override) ---
const tickSim = new Simulation();
tickSim.loadTemplatesYaml(yaml);
tickSim.spawnEntity("scout", {});

const maxTicks = 1000;
for (let i = 0; i < maxTicks; i++) {
  tickSim.tick(16);
  if (i > 0 && i % 30 === 0) {
    console.log(`tick ${i}`);
  }
}

const tickJson = JSON.parse(tickSim.getWorldAsJson());
const tickScout = tickJson.entities.find((e) => e.entity_type === "scout");
if (!tickScout) {
  throw new Error("tick demo: scout missing");
}
if (tickScout.move_target !== undefined) {
  throw new Error(
    `tick demo: scout still has move_target after ${maxTicks} ticks`,
  );
}
const tx = 20.0;
const ty = 0.0;
const px = tickScout.position?.x;
const py = tickScout.position?.y;
if (Math.abs(px - tx) > 0.01 || Math.abs(py - ty) > 0.01) {
  throw new Error(
    `tick demo: expected position near (${tx}, ${ty}), got (${px}, ${py})`,
  );
}

console.log("wasm tick demo ok");
```

- [ ] **Step 2: Run demo**

```bash
make wasm-demo
```

Expected: `wasm spawn demo ok`, tick progress logs, `wasm tick demo ok`.

- [ ] **Step 3: Commit**

```bash
git add wasm-bindings/demo/run.mjs
git commit -m "demo: tick scout to move_target in Node"
```

---

### Task 11: README

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Document components and tick API**

- In the domain components sentence, add `` `BaseMoveSpeed` ``.
- Add a **Simulation tick** subsection after Import/spawn:

```markdown
## Simulation tick

`Api::tick(dt_ms)` advances the ECS schedule: `seek_system` (entities with `MoveTarget` + `BaseMoveSpeed`) then `movement_system` (all `Position` + `Velocity`). Delta is unsigned milliseconds; `0` returns `TickError::ZeroDeltaTime`; values above **100 ms** are clamped.

Arrival (distance ≤ 0.1): snap to target, remove `MoveTarget`, zero `Velocity`, skip movement that frame.

```rust
api.tick(16)?; // ~60 Hz step
```
```

- In WASM API table, add row: `` `tick(dtMs)` `` | `tick` |
- Note: `tick(0)` and non-integer/NaN `dtMs` error on the JS side before Rust.

- [ ] **Step 2: Update JSON example** — add optional `"base_move_speed": 2.0` on one entity row.

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: tick API, BaseMoveSpeed, and WASM tick"
```

---

### Task 12: Final verification

- [ ] **Step 1: Full lib test + clippy**

```bash
cargo test -p open_entities
cargo clippy -p open_entities -- -D warnings
```

Expected: all tests pass, no clippy warnings.

- [ ] **Step 2: WASM check**

```bash
make wasm-check
```

Expected: demo + wasm tests pass.

- [ ] **Step 3: Commit (only if verification fixes were needed)**

```bash
git add -A
git commit -m "fix: address tick/movement review findings"
```

(Skip empty commit if nothing changed.)

---

## Self-review (spec coverage)

| Spec requirement | Task |
|------------------|------|
| `Api::tick(dt_ms)`, zero error, clamp 100 ms | Task 6 |
| `BaseMoveSpeed` YAML/export/registry | Tasks 1, 8 |
| `Schedule` seek → movement in `Core` | Tasks 3, 6 |
| Arrival snap / remove target / skip movement | Tasks 3, 4, 7 |
| `SimDelta`, `ArrivedThisTick` | Tasks 2, 6 |
| Fixture scout without velocity | Task 8 |
| WASM `tick` + JS validation | Task 9 |
| Node demo tick to `(20, 0)` | Task 10 |
| README | Task 11 |
| Verification commands | Task 12 |

**Note on tick count:** From fixture pose `(10, 5)` to `(20, 0)` at speed `2.0` u/s needs ~350× `tick(16)` (~5.6 s). The demo uses up to **1000** ticks with an arrival check; do not use ~120 ticks alone — insufficient for the assertion.

**Placeholder scan:** None — all steps include concrete code and commands.

**Type consistency:** `BaseMoveSpeed`, `SimDelta`, `ArrivedThisTick`, `TickError`, `seek_system`, `movement_system` names match across tasks.
