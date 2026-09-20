# Design: Replanner

**Date:** 2026-09-20
**Status:** Implemented
**Contract:** [group-mission-contract.md](../../design/group-mission-contract.md) (rule C, v1.5)
**Scope:** Slice 3, the last — what a group does when its mission ends under it.
**Depends on:** [Missions](2026-09-20-missions-design.md)

## Summary

When a mission completes, every released group is marked `NeedsMission`. The replanner, running
right after completion in the same tick, gives each marked group the nearest open mission, or
clears the marker and leaves it idle.

## Decisions

| Topic | Choice | Rationale |
|-------|--------|-----------|
| Who gets planned for | Only groups marked `NeedsMission` | A planner that grabbed every idle group would turn creating a mission into a general mobilisation. The contract asks for a pick after a mission ends, not for standing conscription. |
| When | Same tick, right after completion | The contract allows this tick or the next, and asks for determinism. Same tick means a group is never idle for a frame it did not have to be. |
| Which mission | Nearest to the group's centre of mass; ties by entity index | Distance is what a player expects. The tie-break makes the same world plan the same way every run. |
| Manual groups | Marker cleared, nothing assigned | Rule from v1.5: automation does not take a group the player is driving. |
| Empty groups | Marker cleared, nothing assigned | A group with nobody left cannot arrive, so assigning it only blocks the mission. |

## Why a marker and not a query for idle groups

Both work for the case in the contract. The difference shows up the first time a player creates a
mission while three squads stand around: with a marker, nothing happens until a mission ends under
someone; with a query, all three walk off. The second behaviour is a scheduler, not a replanner,
and it is not what this contract describes.

## Verification

Five tests in `missions.rs`: a freed group picks up the next mission, it picks the nearest of
several, it stands idle when nothing is open, a manual group is left alone, and a group with
nobody left is skipped — each also asserting the marker does not linger.
