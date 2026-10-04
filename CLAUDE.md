# open-entities

Deterministic RTS simulation core in Rust (`bevy_ecs` only, not full Bevy).
Target: 100 000 units, lockstep multiplayer, replays.

## Layout

- `open-entities-lib/` — core library, crate `open_entities`
- `wasm-bindings/` — wasm-bindgen bindings (`Simulation` mirrors `Api`)
- `js-app/` — PixiJS demo; the simulation runs in a web worker, see `js-app/CORE-API.md`
- `docs/design/` — design contracts; `docs/design/lockstep-roadmap.md` is the current migration plan

## Commands

- `make test` (or `cargo test`)
- `make float-check` (no `f32`/`f64` in simulation modules)
- `cargo test -p open_entities --test golden_replay` — the golden replay hash; it must match on
  every platform. Never edit `fixtures/replays/basic.hash` to make a platform pass;
  `UPDATE_GOLDEN=1` only for an intended behaviour change, with the reason in the PR
- `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings`
- `make wasm-check`
- js-app: typecheck and vitest, as run in CI (`.github/workflows/`)

## Simulation invariants

Items marked (target) are being migrated per the roadmap. Do not write new code that
depends on the old behaviour they replace.

1. Fixed timestep: the simulation advances only in whole ticks of constant length.
   No system reads wall-clock time or a host-supplied delta.
2. Determinism: the same initial state and the same command log produce bit-identical state
   on every platform, native and wasm32.
   - Simulation state uses integers, never `f32`/`f64`.
   - No iteration over `std` `HashMap`/`HashSet` in simulation code; use `BTreeMap`,
     `IndexMap` or a sorted `Vec`.
   - Randomness comes only from the match-seeded RNG resource.
   - Ties are broken by `EntityId`, never by query iteration order.
   - Systems in the simulation schedule are strictly ordered (`.chain()`).
3. Host input enters the simulation only as `Command` values applied at the start of a tick
   (`Api::submit` / `Api::schedule`). The immediate `Api` order methods are what commands call;
   hosts, the wasm worker included, do not call them between steps.
4. Host and renderer code read simulation state and never write to it.
5. Public API: no `bevy_ecs` types in the normal path; entities are named by
   `EntityId { index, generation }`. `Api::core()` / `core_mut()` stay the one documented escape hatch.

## Working agreements

- Every behaviour change starts with a failing test.
- README.md describes current behaviour only; update it and CHANGELOG.md (Keep a Changelog)
  in the same change as the behaviour.
- Breaking changes are acceptable until 0.1.0 (crates are `publish = false`); record them
  under **Changed** with a **Breaking.** prefix.
- clippy `pedantic` stays clean; CI denies warnings.
- Work on one roadmap step at a time; do not start the next step unasked.
