# bevy_ecs Position Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Integrate `bevy_ecs` into `open_entities`, add `components::Position { x: f32, y: f32 }`, re-export core ECS types, and verify spawn/query with a unit test while keeping `hello()` unchanged.

**Architecture:** Pin `bevy_ecs = "0.18"` at the workspace level. Domain components live under `src/components/`; `lib.rs` is the public facade (re-exports + `hello()`). No full `bevy` crate, no examples, no README changes in this plan.

**Tech Stack:** Rust 2024, Cargo workspace, `bevy_ecs 0.18` (MSRV **1.89+**).

**Spec:** `docs/superpowers/specs/2026-05-16-bevy-ecs-position-design.md`

---

## File map

| File | Responsibility |
|------|----------------|
| `Cargo.toml` | `[workspace.dependencies] bevy_ecs = "0.18"` |
| `open-entities-lib/Cargo.toml` | Crate dependency on workspace `bevy_ecs` |
| `open-entities-lib/src/lib.rs` | Re-export `World`, `Entity`, `Component`, `Query`; `pub mod components`; keep `hello()` + existing test |
| `open-entities-lib/src/components/mod.rs` | `pub mod position;` + `pub use position::Position;` |
| `open-entities-lib/src/components/position.rs` | `Position` component + `position_component_round_trip` test |

---

### Task 1: Workspace dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add workspace dependency**

Append to repo root `Cargo.toml`:

```toml
[workspace.dependencies]
bevy_ecs = "0.18"
```

Full file should look like:

```toml
[workspace]
members = ["open-entities-lib"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "GPL-3.0-or-later"

[workspace.dependencies]
bevy_ecs = "0.18"
```

- [ ] **Step 2: Verify manifest parses**

Run (from repo root):

```bash
cargo metadata --format-version 1 --no-deps 2>&1 | head -3
```

Expected: JSON metadata (no parse error).

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "chore: add bevy_ecs workspace dependency"
```

---

### Task 2: Crate dependency

**Files:**
- Modify: `open-entities-lib/Cargo.toml`

- [ ] **Step 1: Add `bevy_ecs` to the library crate**

Full `open-entities-lib/Cargo.toml`:

```toml
[package]
name = "open_entities"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
bevy_ecs = { workspace = true }
```

- [ ] **Step 2: Fetch and resolve dependencies**

Run:

```bash
cargo fetch
```

Expected: completes without error (requires network; Rust **1.89+**).

If rustc is too old:

```
error: package `bevy_ecs v0.18.x` requires rustc 1.89 or newer
```

Upgrade toolchain: `rustup update stable` and verify `rustc --version` ≥ 1.89.

- [ ] **Step 3: Commit**

```bash
git add open-entities-lib/Cargo.toml Cargo.lock
git commit -m "chore: wire open_entities to bevy_ecs"
```

---

### Task 3: Failing ECS round-trip test

**Files:**
- Create: `open-entities-lib/src/components/mod.rs`
- Create: `open-entities-lib/src/components/position.rs`
- Modify: `open-entities-lib/src/lib.rs`

- [ ] **Step 1: Create `components/mod.rs`**

```rust
pub mod position;

pub use position::Position;
```

- [ ] **Step 2: Create `position.rs` with failing test (no `Position` struct yet)**

```rust
#[cfg(test)]
mod tests {
    use super::Position;
    use bevy_ecs::prelude::*;

    #[test]
    fn position_component_round_trip() {
        let mut world = World::new();
        world.spawn(Position { x: 1.0, y: 2.0 });

        let mut query = world.query::<&Position>();
        let mut count = 0;
        for position in query.iter(&world) {
            assert_eq!(position.x, 1.0);
            assert_eq!(position.y, 2.0);
            count += 1;
        }
        assert_eq!(count, 1);
    }
}
```

- [ ] **Step 3: Register module in `lib.rs` (keep `hello()` and its test)**

Add after the crate attributes, before `hello()`:

```rust
pub mod components;
```

`lib.rs` should still contain:

```rust
/// Returns the canonical hello-world greeting.
pub fn hello() -> &'static str {
    "Hello, world!"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_returns_greeting() {
        assert_eq!(hello(), "Hello, world!");
    }
}
```

- [ ] **Step 4: Run test to verify it fails**

Run:

```bash
cargo test -p open_entities position_component_round_trip -- --nocapture
```

Expected: **compile error** — `Position` not found in `position.rs` (or similar). This is the RED step.

- [ ] **Step 5: Commit**

```bash
git add open-entities-lib/src/components/mod.rs \
        open-entities-lib/src/components/position.rs \
        open-entities-lib/src/lib.rs
git commit -m "test: add failing Position ECS round-trip test"
```

---

### Task 4: Implement `Position`

**Files:**
- Modify: `open-entities-lib/src/components/position.rs`

- [ ] **Step 1: Add `Position` above the test module**

Full `open-entities-lib/src/components/position.rs`:

```rust
use bevy_ecs::prelude::Component;

/// 2D position in world/simulation space.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[cfg(test)]
mod tests {
    use super::Position;
    use bevy_ecs::prelude::*;

    #[test]
    fn position_component_round_trip() {
        let mut world = World::new();
        world.spawn(Position { x: 1.0, y: 2.0 });

        let mut query = world.query::<&Position>();
        let mut count = 0;
        for position in query.iter(&world) {
            assert_eq!(position.x, 1.0);
            assert_eq!(position.y, 2.0);
            count += 1;
        }
        assert_eq!(count, 1);
    }
}
```

- [ ] **Step 2: Run test to verify it passes**

Run:

```bash
cargo test -p open_entities position_component_round_trip -- --nocapture
```

Expected: `test result: ok. 1 passed`

- [ ] **Step 3: Commit**

```bash
git add open-entities-lib/src/components/position.rs
git commit -m "feat: add Position component"
```

---

### Task 5: Public re-exports

**Files:**
- Modify: `open-entities-lib/src/lib.rs`

- [ ] **Step 1: Add re-exports at top of `lib.rs`**

Full `open-entities-lib/src/lib.rs`:

```rust
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

pub use bevy_ecs::{Component, Entity, Query, World};

pub mod components;

/// Returns the canonical hello-world greeting.
pub fn hello() -> &'static str {
    "Hello, world!"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_returns_greeting() {
        assert_eq!(hello(), "Hello, world!");
    }
}
```

- [ ] **Step 2: Run full crate tests**

Run:

```bash
cargo test -p open_entities
```

Expected: all tests pass (`hello_returns_greeting`, `position_component_round_trip`).

- [ ] **Step 3: Run Clippy on the library**

Run:

```bash
cargo clippy -p open_entities -- -D warnings
```

Expected: no warnings (fix any new lints if they appear).

- [ ] **Step 4: Commit**

```bash
git add open-entities-lib/src/lib.rs
git commit -m "feat: re-export bevy_ecs core types from open_entities"
```

---

### Task 6: Workspace verification

**Files:** (none — verification only)

- [ ] **Step 1: Build workspace**

Run:

```bash
cargo build --workspace
```

Expected: `Finished dev [unoptimized + debuginfo] target(s)`

- [ ] **Step 2: Run all workspace tests**

Run:

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 3: Confirm hello example still runs**

Run:

```bash
cargo run -p open_entities --example hello
```

Expected stdout:

```
Hello, world!
```

- [ ] **Step 4: Commit (only if lockfile or incidental changes remain)**

```bash
git status
```

If `Cargo.lock` changed and was not committed in Task 2:

```bash
git add Cargo.lock
git commit -m "chore: update lockfile for bevy_ecs"
```

---

## Spec coverage (self-review)

| Spec requirement | Plan task |
|------------------|-----------|
| `bevy_ecs` only, not full `bevy` | Tasks 1–2 (dependency name) |
| Version `0.18`, workspace-pinned | Task 1 |
| `Position { x: f32, y: f32 }` + `Component` | Task 4 |
| Re-export `World`, `Entity`, `Component`, `Query` | Task 5 |
| `components/` module layout | Tasks 3–4 |
| Unit test spawn + query | Task 3–4 |
| Keep `hello()` + existing test | Tasks 3, 5, 6 |
| No examples/README/CI | Out of plan |
| MSRV 1.89+ | Task 2 note |

No placeholders. Types and paths are consistent across tasks.

---

## Execution handoff

Plan saved to `docs/superpowers/plans/2026-05-16-bevy-ecs-position.md`.

**Two execution options:**

1. **Subagent-Driven (recommended)** — fresh subagent per task, review between tasks, fast iteration  
2. **Inline Execution** — implement in this session with checkpoints (`executing-plans`)

Which approach do you want?
