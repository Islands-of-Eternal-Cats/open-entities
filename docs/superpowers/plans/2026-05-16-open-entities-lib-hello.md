# open_entities Hello Scaffold Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Cargo workspace at the repo root and a minimal `open_entities` library crate with `hello()`, a unit test, and a runnable example.

**Architecture:** Root workspace declares shared `[workspace.package]` metadata (edition 2024, GPL-3.0-or-later). The `open-entities-lib` member crate inherits those fields and exposes a single public function. An `examples/hello.rs` target demonstrates the API without adding a binary crate.

**Tech Stack:** Rust 2024 edition, Cargo workspace resolver 2, no external dependencies.

**Spec:** `docs/superpowers/specs/2026-05-16-open-entities-lib-hello-design.md`

---

## File map

| File | Responsibility |
|------|----------------|
| `Cargo.toml` | Workspace root: members, resolver, shared package metadata |
| `open-entities-lib/Cargo.toml` | Package `open_entities`, inherits workspace fields |
| `open-entities-lib/src/lib.rs` | Public `hello()` API + unit tests |
| `open-entities-lib/examples/hello.rs` | Runnable demo printing greeting to stdout |

---

### Task 1: Root workspace manifest

**Files:**
- Create: `Cargo.toml`

- [ ] **Step 1: Create root `Cargo.toml`**

```toml
[workspace]
members = ["open-entities-lib"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "GPL-3.0-or-later"
```

- [ ] **Step 2: Verify workspace parses**

Run (from repo root):

```bash
cargo metadata --format-version 1 --no-deps 2>&1 | head -5
```

Expected: JSON metadata output (may warn that member has no manifest until Task 2; if `cargo metadata` errors on missing member, proceed to Task 2 first, then re-run).

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "chore: add Cargo workspace root manifest"
```

---

### Task 2: Crate manifest

**Files:**
- Create: `open-entities-lib/Cargo.toml`

- [ ] **Step 1: Create `open-entities-lib/Cargo.toml`**

```toml
[package]
name = "open_entities"
version.workspace = true
edition.workspace = true
license.workspace = true
```

- [ ] **Step 2: Verify workspace resolves**

Run:

```bash
cargo metadata --format-version 1 --no-deps -q | grep -o '"name":"open_entities"'
```

Expected: `"name":"open_entities"`

- [ ] **Step 3: Commit**

```bash
git add open-entities-lib/Cargo.toml
git commit -m "chore: add open_entities crate manifest"
```

---

### Task 3: Failing unit test (RED)

**Files:**
- Create: `open-entities-lib/src/lib.rs`

- [ ] **Step 1: Write `lib.rs` with test only (no `hello` yet)**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_returns_greeting() {
        assert_eq!(hello(), "Hello, world!");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test -p open_entities
```

Expected: FAIL — compile error: `cannot find function hello in this scope` (or similar).

- [ ] **Step 3: Commit**

```bash
git add open-entities-lib/src/lib.rs
git commit -m "test: add failing hello unit test"
```

---

### Task 4: Implement `hello()` (GREEN)

**Files:**
- Modify: `open-entities-lib/src/lib.rs`

- [ ] **Step 1: Add minimal implementation above the test module**

Full file content:

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

- [ ] **Step 2: Run tests**

Run:

```bash
cargo test -p open_entities
```

Expected:

```
test tests::hello_returns_greeting ... ok
test result: ok. 1 passed; 0 failed
```

- [ ] **Step 3: Build workspace**

Run:

```bash
cargo build --workspace
```

Expected: `Finished` with no errors.

- [ ] **Step 4: Commit**

```bash
git add open-entities-lib/src/lib.rs
git commit -m "feat: add open_entities::hello()"
```

---

### Task 5: Runnable example

**Files:**
- Create: `open-entities-lib/examples/hello.rs`

- [ ] **Step 1: Create example**

```rust
use open_entities::hello;

fn main() {
    println!("{}", hello());
}
```

- [ ] **Step 2: Run example**

Run:

```bash
cargo run -p open_entities --example hello
```

Expected stdout (exact line):

```
Hello, world!
```

- [ ] **Step 3: Commit**

```bash
git add open-entities-lib/examples/hello.rs
git commit -m "feat: add hello example for open_entities"
```

---

### Task 6: Final verification

**Files:** none (verification only)

- [ ] **Step 1: Run full spec verification suite**

Run from repo root:

```bash
cargo test
cargo build --workspace
cargo run -p open_entities --example hello
```

Expected:
- All tests pass
- Workspace builds cleanly
- Example prints `Hello, world!`

- [ ] **Step 2: Confirm file tree matches spec**

Run:

```bash
find open-entities-lib Cargo.toml -type f | sort
```

Expected files:

```
Cargo.toml
open-entities-lib/Cargo.toml
open-entities-lib/examples/hello.rs
open-entities-lib/src/lib.rs
```

---

## Spec coverage checklist

| Spec requirement | Task |
|------------------|------|
| Root workspace `members`, `resolver = "2"` | Task 1 |
| `[workspace.package]` edition 2024, GPL license | Task 1 |
| Crate `open_entities` inherits workspace | Task 2 |
| `pub fn hello() -> &'static str` | Task 4 |
| Unit test `assert_eq!(hello(), "Hello, world!")` | Tasks 3–4 |
| `examples/hello.rs` | Task 5 |
| `cargo test` / `cargo build --workspace` / `cargo run --example hello` | Task 6 |
| wasm-bindings, CI, rust-toolchain — out of scope | — |

## Plan self-review notes

- License matches repo `LICENSE` (GPL-3.0-or-later), not MIT.
- Edition 2024 requires `rustc >= 1.85`; if build fails on older toolchain, document locally or add `rust-toolchain.toml` in a follow-up (out of scope per spec).
- No placeholders; each code step includes full file content where applicable.
