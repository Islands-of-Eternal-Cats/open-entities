# Vehicle Seats Design (OpenEntities)

## Goal

Enable units to board vehicles and move with them while staying within the current ECS architecture:
- `seek_move_target_system` computes velocity
- `move_system` integrates movement by `dt`
- WASM (`JsWorld`) exposes gameplay commands
- TS worker/UI consume entity snapshots

The implementation should avoid special-case spaghetti and keep movement ownership unambiguous.

## Core Idea

Model "unit in vehicle" as an entity relationship, not as a custom movement mode.

- Vehicle remains a normal movable entity.
- Passenger is a normal entity plus a relation component pointing to its vehicle.
- Passenger movement is driven by a dedicated sync system while boarded.

This keeps systems composable and testable.

## Components

### `Boardable`

Attach to vehicle entities that can carry units.

- `seats: u8` - max passengers

### `PassengerOf`

Attach to unit entities while boarded.

- `vehicle: Entity` - vehicle entity id

### Optional extensions

- `SeatIndex(u8)` - stable seat assignment for offsets/animation
- `DriverOf(Entity)` - explicit driver role if needed for control rules
- `CanBoardTags` - restrictions by unit/vehicle class

## Invariants (Most Important)

1. A passenger must not be moved by regular unit movement systems while boarded.
2. Boarded passenger position is owned by vehicle-sync logic.
3. Boarding/unboarding operations must preserve consistency:
   - no over-capacity
   - no invalid references
   - clean component transitions

## Systems

## 1) `board_system`

Consumes board commands and validates:
- unit exists and is not already boarded
- vehicle exists and has `Boardable`
- faction/ownership rules (optional)
- boarding distance threshold
- available seat count

On success:
- insert `PassengerOf(vehicle)`
- remove `MoveTarget` for that unit
- zero unit `Velocity` (if present)

## 2) `passenger_sync_system`

Runs every tick after vehicle movement.

For each `(passenger, PassengerOf(vehicle))`:
- if vehicle is missing, unboard safely (or mark invalid and cleanup)
- set passenger `Position` from vehicle `Position`
- optional seat offset by `SeatIndex`

## 3) `unboard_system`

Consumes unboard commands:
- remove `PassengerOf`
- place unit near vehicle (`at_pos` or computed nearby slot)
- optional: restore `Velocity` to zero

## Schedule Order

Keep deterministic order in update chain:

1. process board/unboard commands
2. seek (`MoveTarget` -> `Velocity`) for movable non-passengers
3. move integration for movable non-passengers
4. passenger sync (copy vehicle transform to passengers)

If board/unboard uses events, process them before seek/move to avoid one-frame glitches.

## Query Filtering Changes

To avoid dual ownership of position while boarded:

- `seek_move_target_system` should exclude passengers (`Without<PassengerOf>`)
- `move_system` should exclude passengers (`Without<PassengerOf>`)

This single rule prevents most subtle bugs.

## YAML and Spawning

Current YAML already describes movement with `base_move_speed`.
For vehicles, extend templates minimally:

```yaml
entities:
  jeep:
    position: { x: 0.0, y: 0.0 }
    base_move_speed: 60.0
    seats: 4
```

Mapping suggestion in loader:
- `seats > 0` => insert `Boardable { seats }`
- no seats field => regular entity behavior remains unchanged

Backward compatibility is preserved for existing YAML.

## WASM API Additions (`JsWorld`)

Add command-style methods:

- `order_board(entity_ids: Vec<String>, vehicle_id: String) -> Result<(), JsValue>`
- `order_unboard(entity_ids: Vec<String>, at: Option<JsPosition>) -> Result<(), JsValue>`

Maintain existing style:
- parse snapshot ids (`Entity::to_bits()` as decimal strings)
- perform validation in Rust world layer
- return readable errors for UI

## Worker Protocol Additions (TS)

Extend `WorkerInMessage`:

- `{ type: "board"; entityIds: string[]; vehicleId: string }`
- `{ type: "unboard"; entityIds: string[]; point?: { x: number; y: number } }`

Worker applies command, then returns fresh `entities` snapshot (same as other commands).

## Snapshot Shape Additions

Expose boarding state for UI:

- `passengerOf: string | null` (vehicle id string)
- optional `seats` / `occupiedSeats` for vehicle HUD

UI can then:
- hide boarded units from world list, or
- render them with "inside vehicle" marker.

## MVP Path (Smallest Safe Increment)

1. Add `PassengerOf` component.
2. Exclude passengers from seek/move queries.
3. Add `order_board` and `order_unboard` world APIs.
4. Add `passenger_sync_system` after movement.
5. Return `passengerOf` in `get_entities()` snapshots.
6. Wire board/unboard messages through worker and TS wrapper.

This delivers usable boarding quickly without refactoring core movement.

## File-by-File Implementation TODO

Use this sequence to avoid broad refactors and keep each step testable.

### 1) Components

Files:
- `open-entities-lib/src/components/passenger_of.rs` (new)
- `open-entities-lib/src/components/boardable.rs` (new)
- `open-entities-lib/src/components/mod.rs`
- `open-entities-lib/src/lib.rs`

TODO:
- Add `PassengerOf(pub Entity)` component.
- Add `Boardable { seats: u8 }` component.
- Export both from `components/mod.rs` and crate root.

Done when:
- Components compile and can be inserted/queried in tests.

### 2) Entity Loader and YAML

Files:
- `open-entities-lib/src/entity_loader.rs`
- `assets/entities.yaml` (example data only)

TODO:
- Extend `EntityTemplate` with optional `seats: Option<u8>`.
- On spawn, if `seats > 0`, insert `Boardable { seats }`.
- Keep behavior unchanged for entities without `seats`.
- Add at least one vehicle example in YAML (e.g. `jeep`).

Done when:
- Existing YAML still loads unchanged.
- Vehicle type gets `Boardable`; non-vehicle types do not.

### 3) Boarding Commands in World Layer

Files:
- `open-entities-lib/src/world.rs`

TODO:
- Add world-level APIs:
  - `order_board_entities(world, unit_ids_bits, vehicle_id_bits)`
  - `order_unboard_entities(world, unit_ids_bits, at: Option<Position>)`
- Validate ids, distance, seat availability, and entity roles.
- On board success:
  - insert `PassengerOf(vehicle)`
  - remove `MoveTarget`
  - zero `Velocity` if present
- On unboard:
  - remove `PassengerOf`
  - place unit near vehicle/target point

Done when:
- APIs are safe on invalid ids (skip or return explicit error consistently).
- No panics on missing entities/components.

### 4) Movement/System Integration

Files:
- `open-entities-lib/src/systems/seek.rs`
- `open-entities-lib/src/systems/movement.rs`
- `open-entities-lib/src/systems/mod.rs`
- `open-entities-lib/src/systems/vehicle.rs` (new; optional naming)
- `open-entities-lib/src/world.rs` (schedule wiring)

TODO:
- Exclude boarded units from regular movement queries (`Without<PassengerOf>`).
- Add `passenger_sync_system` that copies vehicle position to passengers each tick.
- Ensure update schedule order:
  1. board/unboard apply
  2. seek
  3. move
  4. passenger sync

Done when:
- Boarded passengers do not self-move.
- Passenger position tracks vehicle every tick.

### 5) Snapshot Contract (Rust -> JS)

Files:
- `open-entities-lib/src/world.rs` (`EntitySnapshotRow`, `get_entities`)
- `wasm-bindings/src/lib.rs` (`JsWorld::get_entities`)
- `js-app/src/core/wasm-types.d.ts`
- `js-app/src/core/types.ts`

TODO:
- Extend snapshot with `passengerOf` (string id or null).
- Keep id representation as decimal string for `u64` safety.
- Maintain backward-compatible field meanings for existing UI.

Done when:
- Worker receives consistent snapshots with `passengerOf`.
- TypeScript compiles with updated types.

### 6) WASM Command Surface

Files:
- `wasm-bindings/src/lib.rs`

TODO:
- Add:
  - `order_board(entity_ids: Vec<String>, vehicle_id: String)`
  - `order_unboard(entity_ids: Vec<String>, target: Option<JsPosition>)`
- Parse ids using existing error style (`invalid entity id`-like messages).
- Bridge to new world-layer APIs.

Done when:
- JS can issue board/unboard without touching internals.

### 7) Worker Protocol + Main Thread API

Files:
- `js-app/src/core/worker-types.ts`
- `js-app/src/core/ecs-worker.ts`
- `js-app/src/core/wasm.ts`

TODO:
- Add worker messages:
  - `{ type: "board"; entityIds; vehicleId }`
  - `{ type: "unboard"; entityIds; point? }`
- Handle new messages in worker and return fresh `entities`.
- Expose wrappers in `wasm.ts` (e.g. `boardSelectedToVehicle`, `unboardSelected`).

Done when:
- Frontend can issue commands through existing async queue pattern.

### 8) UI Integration (Optional for MVP backend completion)

Files:
- `js-app/src/visualization/pixi-canvas.ts`
- `js-app/src/main.ts`
- `js-app/src/visualization/render.ts`

TODO:
- Add interaction flow for boarding/unboarding commands.
- Prevent regular move command on boarded units.
- Show boarded state in list/HUD (via `passengerOf`).

Done when:
- Player can board/unboard through UI and see resulting state.

### 9) Tests and Regression Net

Files:
- `open-entities-lib/src/lib.rs` (or dedicated tests module)
- `js-app/src/core/wasm.test.ts`

TODO:
- Add ECS tests for board/unboard, seat cap, sync, and invalid refs.
- Add WASM-side tests for id parsing and error reporting.
- Verify existing move-order behavior still works for non-boarded units.

Done when:
- `cargo test` and JS tests pass with new features enabled.

## Recommended Delivery Slices

Slice A (backend core):
- Steps 1-4 + Rust tests.

Slice B (interop):
- Steps 5-7 + worker/TS type checks.

Slice C (UX):
- Step 8 + interaction polish.

This keeps each PR focused and easy to review.

## Testing Checklist

Rust ECS tests:
- board success when near vehicle and seat available
- board fails when full or too far
- boarded passenger does not move via own `MoveTarget`
- passenger follows vehicle across multiple ticks
- unboard places passenger near vehicle and removes relation
- vehicle deletion cleans orphaned `PassengerOf`

WASM/worker tests:
- board/unboard command validation and error messages
- snapshot includes `passengerOf`
- ids remain string-safe for large `u64`

## Future Extensions

- seat classes (driver/gunner/passenger)
- disallow move orders to boarded units from UI
- embark/disembark animations
- formation unboard slots
- combat modifiers while embarked

## Summary

The best fit for current OpenEntities architecture is a relation-based model:
`PassengerOf` + `Boardable` + command systems, with strict movement exclusion for passengers.
It aligns with existing ECS design, preserves API style, and scales cleanly from MVP to advanced vehicle gameplay.
