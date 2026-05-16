# Design: open-entities-lib Hello World Scaffold

**Date:** 2026-05-16  
**Status:** Approved (brainstorming)  
**Scope:** Minimal Rust library scaffold with workspace layout, public API, example, and unit test.

## Summary

Add a Cargo workspace at the repository root with `open-entities-lib` as the first member crate. The crate `open_entities` exposes a single `hello()` function, includes one unit test, and ships an example binary target for local demonstration.

## Decisions

| Topic | Choice |
|-------|--------|
| Workspace layout | Root `Cargo.toml` with `members = ["open-entities-lib"]` |
| Workspace metadata | `[workspace.package]` shared fields; crates inherit via `{ workspace = true }` |
| Crate name | `open_entities` (package in `open-entities-lib/`) |
| Rust edition | `2024` (in `[workspace.package]`) |
| License | `GPL-3.0-or-later` (matches repository `LICENSE`) |
| Hello surface | `pub fn hello() -> &'static str` |
| Runnable demo | `examples/hello.rs` (not `src/main.rs`) |
| API style | Plain function (no domain types yet) |

## Repository Layout

```
open-entities/
├── Cargo.toml
├── open-entities-lib/
│   ├── Cargo.toml
│   ├── src/
│   │   └── lib.rs
│   └── examples/
│       └── hello.rs
```

## Root `Cargo.toml`

```toml
[workspace]
members = ["open-entities-lib"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "GPL-3.0-or-later"
```

## `open-entities-lib/Cargo.toml`

- `name = "open_entities"`
- Inherit `version`, `edition`, `license` from workspace: `{ workspace = true }`
- Default library target (crate name `open_entities`)

## Public API

`src/lib.rs`:

```rust
/// Returns the canonical hello-world greeting.
pub fn hello() -> &'static str {
    "Hello, world!"
}
```

## Example

`examples/hello.rs` imports `open_entities::hello()`, prints with `println!`.

Run from repository root:

```bash
cargo run -p open_entities --example hello
```

Expected stdout:

```
Hello, world!
```

## Testing

Unit test in `lib.rs` under `#[cfg(test)]`:

```rust
assert_eq!(hello(), "Hello, world!");
```

Verification commands (from repo root):

```bash
cargo test
cargo build --workspace
cargo run -p open_entities --example hello
```

## Error Handling

Not applicable for this scaffold: `hello()` returns a constant string with no fallible operations.

## Out of Scope

- `wasm-bindings/` crate (workspace is structured to add members later)
- CI configuration
- `rust-toolchain.toml` / MSRV pinning (recommend `rustc >= 1.85` when added, for edition 2024)
- README expansion beyond optional one-liner for build/test commands
- Additional modules, features, or dependencies

## Future Extensions

When `wasm-bindings` is added:

1. Add path to `workspace.members`.
2. Reuse `[workspace.package]` for shared `edition`, `version`, `license`.
3. Depend on `open_entities` via path dependency if needed.
