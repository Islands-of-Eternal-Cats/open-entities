# Design: Groups and order sources

**Date:** 2026-09-20
**Status:** Implemented
**Contract:** [group-mission-contract.md](../../design/group-mission-contract.md) (v1.5)
**Scope:** Slice 1 of that contract — groups, membership, group steering, `ManualActive`, and the
order priority rule. Missions and the replanner are slices 2 and 3.

## Summary

A group is an ECS entity carrying `Group { faction }`. Membership lives on the units as
`MemberOf(group)`. A group order steers every member that will take it and marks the group
`ManualActive`; a unit already following a personal order keeps it.

## Decisions

| Topic | Choice | Rationale |
|-------|--------|-----------|
| Group identity | An entity, addressed by the same `EntityId` as everything else | The contract wants groups that outlive their members and can carry state; a `Vec` on the side cannot. One id type keeps the API honest. |
| Where membership lives | `MemberOf` on the unit, never a member list on the group | One source of truth. A list plus a back-pointer is two, and they drift. |
| Group roster | Computed by query on demand | Cheap at this scale, and it cannot go stale when a unit is despawned. |
| Order priority | An `OrderSource` component beside `MoveTarget`, ordered `MissionSteering < GroupSteering < PlayerUnit` | The rule lives in one comparison (`may_override`) instead of being re-derived at every call site. |
| Releasing a claim | Arrival and `order_stop` remove `OrderSource` with the `MoveTarget` | A leftover `PlayerUnit` claim on a standing unit would silently block every later group or mission order. |
| Clearing `ManualActive` | Only `clear_group_manual` | v1.5: an automatic release means the player cannot tell who is steering. |
| Export | None of these components are registered | A renderer does not need them, and the export schema stays at version 3. |

## Invariants

1. A unit is in at most one group — `add_to_group` replaces, never adds.
2. `unit.faction == group.faction`, checked on join; a unit with no faction cannot join at all.
3. An empty group stays alive and can still be ordered.
4. A stronger `OrderSource` is never overwritten by a weaker one.

## API

```rust
let group = api.create_group(1);
api.add_to_group(group, unit)?;
api.order_group_move_to(group, MoveTarget { x: 50.0, y: 50.0 })?; // sets ManualActive
api.clear_group_manual(group)?;                                   // the explicit "resume AI"
```

Reads: `group_of(unit)`, `group_members(group)`, `is_group_manual(group)`.

## Not in this slice

Missions, `MissionSteering`, G1 arrival, global mission close, auto-unassign of empty groups, the
replanner and its conflict with `ManualActive`. Nothing here forecloses them: the priority ladder
already has a rung for mission steering, and `ManualActive` is the flag the replanner will read.

## Verification

Ten tests in `groups.rs` and three in `order_source.rs`, covering each invariant above plus the
case the whole slice exists for: a unit under a personal order ignores its group's advance.
