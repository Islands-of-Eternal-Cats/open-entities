# Design: Boarding

**Date:** 2026-09-20
**Status:** Implemented
**Contract:** [vehicle-seats.md](../../design/vehicle-seats.md)
**Scope:** Carrying units in vehicles: `Boardable`, `PassengerOf`, the sync system, and the
board/unboard API.

## Summary

A vehicle is an entity with `Boardable(seats)`; a passenger carries `PassengerOf(vehicle)`. The
movement systems skip passengers, and one system copies the vehicle's position onto them after it
has moved.

## Decisions

| Topic | Choice | Rationale |
|-------|--------|-----------|
| Who owns a passenger's position | The vehicle, exclusively | The design document's first invariant. Enforced by `Without<PassengerOf>` on seek and movement rather than by discipline, so a stray order on a passenger does nothing instead of fighting the sync. |
| Where sync runs | After movement, before mission completion | Passengers land where the vehicle actually ended the tick, and arrival is judged on the final positions. |
| Boarding commands | Direct `Api` calls, not queued events | Every other order in this library is a direct call; a queue would be a second idiom for no gain at this size. |
| Boarding range | A constant, 3 world units | The document leaves it open. A constant is honest and easy to move to a component when a vehicle needs its own. |
| Losing the vehicle | The passenger is let off where it stands | A component pointing at a despawned entity is a leak waiting to be read. |
| Seat offsets | Not implemented | `SeatIndex` is listed as optional in the document; passengers share the vehicle's exact position until something needs otherwise. |
| YAML | `boardable: 4` through the registry | One line in the registry, and seats become a template field like any other — which is what the registry is for. |

## Schema

Registering `Boardable` adds a `boardable` field to exported entities, so `world_json` is now
**schema version 4**. The Node demo and the TypeScript row type were updated with it.

## Verification

Ten tests in `boarding.rs`: a passenger rides along, a passenger ignores its own orders, boarding
from across the map is refused, seats run out, a unit rides one vehicle at a time, a rock cannot be
boarded, unboarding puts the unit beside the vehicle, a unit that is not aboard cannot step off,
losing the vehicle lets the passenger go, and a unit moves again after stepping off.
