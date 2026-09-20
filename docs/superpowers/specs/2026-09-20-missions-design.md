# Design: Missions

**Date:** 2026-09-20
**Status:** Implemented
**Contract:** [group-mission-contract.md](../../design/group-mission-contract.md) (v1.4/v1.5)
**Scope:** Slice 2 — missions, assignment, mission steering, G1 completion. The replanner is
slice 3.
**Depends on:** [Groups and order sources](2026-09-20-groups-design.md)

## Summary

A mission is an entity carrying `Mission { target, radius }`. Groups sent to it carry
`AssignedTo(mission)`. Two systems do the work: one steers the members of assigned groups, one
closes the mission when anybody arrives.

## Decisions

| Topic | Choice | Rationale |
|-------|--------|-----------|
| Assignment direction | `AssignedTo` on the group, roster by query | Same shape as group membership: one source of truth, nothing to keep in sync. |
| Completed missions | Kept, marked `MissionCompleted` | An id that suddenly resolves to nothing is worse than one that resolves to a finished thing; the host may want to show what was done. |
| Schedule order | steering → seek → movement → completion | Automation proposes, steering resolves, movement integrates, arrival is judged on where everyone actually ended up this tick. |
| Empty group | Unassigned by the steering system | It cannot arrive, so holding the mission only blocks others. |
| Manual order | Takes the group off its mission | Contract's partial unassign: the mission is freed for whoever can still work it. |
| Arrival | Any live member of any assignee within `radius` | Rule G1, unchanged. |

## What completion does

`MissionCompleted` goes on the mission, every assignee loses `AssignedTo`, and every unit that was
following **mission** steering loses its `MoveTarget` and `OrderSource`, so it stops. Units under a
personal or group order keep theirs — those orders never belonged to the mission.

## Not in this slice

The replanner: after a mission closes, a freed group sits idle instead of picking the next one.
`ManualActive` is already the flag it will have to respect.

## Verification

Ten tests in `missions.rs`: the walk-and-close path, first-arrival-wins with a second group still
in transit, manual group ignored by its mission, personal order outranking mission steering, empty
group releasing its mission, a completed mission refusing new groups, one mission per group, and
the id-of-the-wrong-kind refusals.
