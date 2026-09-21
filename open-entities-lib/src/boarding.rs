//! Boarding: putting units inside vehicles and letting them out again.
//!
//! A vehicle is any entity with [`Boardable`]; a passenger carries [`PassengerOf`]. While that
//! component is present the movement systems skip the unit entirely, so its position has exactly
//! one owner — the vehicle — and the two can never disagree about where it is.
//!
//! See `docs/design/vehicle-seats.md`.

use bevy_ecs::prelude::Entity;

use crate::api::Api;
use crate::components::{Boardable, MoveTarget, OrderSource, PassengerOf, Position, Velocity};
use crate::orders::EntityId;

/// How close a unit has to be to climb aboard, in world units.
pub const BOARDING_RANGE: f32 = 3.0;

/// Where a unit is put down when it leaves a vehicle, in world units from it.
const UNBOARD_OFFSET: f32 = 1.5;

/// Errors from boarding and unboarding.
// Not `Eq`: `TooFarAway` carries the measured distance, and a distance is a float.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BoardError {
    /// The id does not resolve to a live entity.
    UnknownUnit(EntityId),
    /// The id does not resolve, or resolves to something without seats.
    NotBoardable(EntityId),
    /// The unit is already riding something.
    AlreadyAboard(EntityId),
    /// Every seat is taken.
    NoSeatsLeft { seats: u8 },
    /// One of the two has no position, so the distance between them is undefined.
    NoPosition(EntityId),
    /// The unit is not close enough to climb aboard. Carries how far it actually was, because
    /// "too far" alone does not say whether to walk the unit over or to stop the vehicle first.
    TooFarAway { distance: f32 },
    /// The unit is not riding anything.
    NotAboard(EntityId),
}

impl std::fmt::Display for BoardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownUnit(id) => write!(f, "no entity with id {}:{}", id.index, id.generation),
            Self::NotBoardable(id) => write!(
                f,
                "entity {}:{} has no seats to board",
                id.index, id.generation
            ),
            Self::AlreadyAboard(id) => write!(
                f,
                "entity {}:{} is already aboard a vehicle",
                id.index, id.generation
            ),
            Self::NoSeatsLeft { seats } => write!(f, "all {seats} seats are taken"),
            Self::NoPosition(id) => write!(
                f,
                "entity {}:{} has no position, so boarding distance is undefined",
                id.index, id.generation
            ),
            Self::TooFarAway { distance } => write!(
                f,
                "the unit is {distance:.1} units away, and boarding range is {BOARDING_RANGE}"
            ),
            Self::NotAboard(id) => write!(
                f,
                "entity {}:{} is not aboard anything",
                id.index, id.generation
            ),
        }
    }
}

impl std::error::Error for BoardError {}

impl Api {
    /// Puts a unit aboard a vehicle standing next to it.
    ///
    /// The unit's move target and order are dropped: it is not going anywhere under its own power
    /// any more, and a claim left behind would block the next order it receives after stepping off.
    ///
    /// # Errors
    ///
    /// [`BoardError::NotBoardable`] when the vehicle has no seats, [`BoardError::AlreadyAboard`],
    /// [`BoardError::NoSeatsLeft`], [`BoardError::TooFarAway`] beyond [`BOARDING_RANGE`], and the
    /// `Unknown*` / `NoPosition` cases when an id does not resolve to something with a place in
    /// the world.
    pub fn board(&mut self, unit: EntityId, vehicle: EntityId) -> Result<(), BoardError> {
        let unit_entity = unit.to_entity().ok_or(BoardError::UnknownUnit(unit))?;
        let vehicle_entity = vehicle
            .to_entity()
            .ok_or(BoardError::NotBoardable(vehicle))?;

        let seats = {
            let world = self.core().world();
            if !world.entities().contains_spawned(unit_entity) {
                return Err(BoardError::UnknownUnit(unit));
            }
            if world.get::<PassengerOf>(unit_entity).is_some() {
                return Err(BoardError::AlreadyAboard(unit));
            }
            let seats = world
                .get::<Boardable>(vehicle_entity)
                .ok_or(BoardError::NotBoardable(vehicle))?
                .0;

            let unit_position = world
                .get::<Position>(unit_entity)
                .copied()
                .ok_or(BoardError::NoPosition(unit))?;
            let vehicle_position = world
                .get::<Position>(vehicle_entity)
                .copied()
                .ok_or(BoardError::NoPosition(vehicle))?;
            let dx = vehicle_position.x - unit_position.x;
            let dy = vehicle_position.y - unit_position.y;
            let distance = dx.hypot(dy);
            if distance > BOARDING_RANGE {
                return Err(BoardError::TooFarAway { distance });
            }
            seats
        };

        if self.passenger_entities(vehicle_entity).len() >= usize::from(seats) {
            return Err(BoardError::NoSeatsLeft { seats });
        }

        let world = self.core_mut().world_mut();
        world
            .entity_mut(unit_entity)
            .remove::<(MoveTarget, OrderSource)>()
            .insert(PassengerOf(vehicle_entity));
        if let Some(mut velocity) = world.get_mut::<Velocity>(unit_entity) {
            velocity.vx = 0.0;
            velocity.vy = 0.0;
        }
        Ok(())
    }

    /// Lets a unit out, beside the vehicle it was riding.
    ///
    /// # Errors
    ///
    /// [`BoardError::NotAboard`] when the unit is not riding anything.
    pub fn unboard(&mut self, unit: EntityId) -> Result<(), BoardError> {
        let unit_entity = unit.to_entity().ok_or(BoardError::UnknownUnit(unit))?;
        let world = self.core_mut().world_mut();
        let Some(passenger_of) = world.get::<PassengerOf>(unit_entity).copied() else {
            return Err(BoardError::NotAboard(unit));
        };

        let beside = world
            .get::<Position>(passenger_of.0)
            .map(|position| Position {
                x: position.x + UNBOARD_OFFSET,
                y: position.y,
            });

        world.entity_mut(unit_entity).remove::<PassengerOf>();
        if let Some(beside) = beside
            && let Some(mut position) = world.get_mut::<Position>(unit_entity)
        {
            position.x = beside.x;
            position.y = beside.y;
        }
        Ok(())
    }

    /// The vehicle this unit is riding, if any.
    #[must_use]
    pub fn vehicle_of(&self, unit: EntityId) -> Option<EntityId> {
        let entity = unit.to_entity()?;
        let passenger_of = self.core().world().get::<PassengerOf>(entity)?;
        Some(EntityId::of(passenger_of.0))
    }

    /// Everyone currently riding this vehicle.
    pub fn passengers(&mut self, vehicle: EntityId) -> Vec<EntityId> {
        let Some(vehicle_entity) = vehicle.to_entity() else {
            return Vec::new();
        };
        self.passenger_entities(vehicle_entity)
            .into_iter()
            .map(EntityId::of)
            .collect()
    }

    /// Seats left on this vehicle, or `None` when it has none to begin with.
    pub fn free_seats(&mut self, vehicle: EntityId) -> Option<u8> {
        let vehicle_entity = vehicle.to_entity()?;
        let seats = self.core().world().get::<Boardable>(vehicle_entity)?.0;
        let taken = u8::try_from(self.passenger_entities(vehicle_entity).len()).unwrap_or(u8::MAX);
        Some(seats.saturating_sub(taken))
    }

    fn passenger_entities(&mut self, vehicle: Entity) -> Vec<Entity> {
        let world = self.core_mut().world_mut();
        let mut query = world.query::<(Entity, &PassengerOf)>();
        query
            .iter(world)
            .filter(|(_, passenger_of)| passenger_of.0 == vehicle)
            .map(|(entity, _)| entity)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::BaseMoveSpeed;

    fn spawn_unit(api: &mut Api, x: f32) -> EntityId {
        let entity = api
            .core_mut()
            .world_mut()
            .spawn((
                Position { x, y: 0.0 },
                BaseMoveSpeed(10.0),
                Velocity { vx: 0.0, vy: 0.0 },
            ))
            .id();
        EntityId::of(entity)
    }

    fn spawn_vehicle(api: &mut Api, x: f32, seats: u8) -> EntityId {
        let entity = api
            .core_mut()
            .world_mut()
            .spawn((
                Position { x, y: 0.0 },
                BaseMoveSpeed(30.0),
                Velocity { vx: 0.0, vy: 0.0 },
                Boardable(seats),
            ))
            .id();
        EntityId::of(entity)
    }

    #[test]
    fn a_passenger_rides_along() {
        let mut api = Api::new();
        let truck = spawn_vehicle(&mut api, 0.0, 2);
        let rider = spawn_unit(&mut api, 1.0);
        api.board(rider, truck).expect("board");

        api.order_move_to(&[truck], MoveTarget { x: 40.0, y: 0.0 });
        for _ in 0..50 {
            api.tick(100).expect("tick");
        }

        let world = api.core().world();
        let truck_position = world
            .get::<Position>(truck.to_entity().expect("live"))
            .expect("position");
        let rider_position = world
            .get::<Position>(rider.to_entity().expect("live"))
            .expect("position");
        assert!((truck_position.x - 40.0).abs() < 1e-3, "the truck arrives");
        assert_eq!(rider_position.x, truck_position.x, "and carries the rider");
        assert_eq!(rider_position.y, truck_position.y);
    }

    #[test]
    fn a_passenger_ignores_its_own_orders() {
        let mut api = Api::new();
        let truck = spawn_vehicle(&mut api, 0.0, 2);
        let rider = spawn_unit(&mut api, 1.0);
        api.board(rider, truck).expect("board");

        // Nothing stops a host from *trying* to order a passenger about. The order is refused
        // at the door — see `can_take_a_move_order` — so nothing is left to act on later.
        api.order_move_to(&[rider], MoveTarget { x: 100.0, y: 0.0 });
        for _ in 0..20 {
            api.tick(100).expect("tick");
        }

        let world = api.core().world();
        let rider_position = world
            .get::<Position>(rider.to_entity().expect("live"))
            .expect("position");
        assert_eq!(rider_position.x, 0.0, "it stays with the parked truck");
    }

    #[test]
    fn a_move_order_does_not_stick_to_a_passenger() {
        let mut api = Api::new();
        let truck = spawn_vehicle(&mut api, 0.0, 2);
        let rider = spawn_unit(&mut api, 1.0);
        api.board(rider, truck).expect("board");

        // The player boxes the truck and its passenger and clicks the ground: the order reaches
        // both ids, and the passenger must not take it.
        let report = api.order_move_to(&[truck, rider], MoveTarget { x: 40.0, y: 0.0 });
        assert_eq!(report.ordered, 1, "only the vehicle can take a move order");
        assert_eq!(report.skipped, 1);

        api.unboard(rider).expect("step off");
        let dropped = api
            .core()
            .world()
            .get::<Position>(rider.to_entity().expect("live"))
            .copied()
            .expect("position");
        for _ in 0..30 {
            api.tick(16).expect("tick");
        }
        let after = api
            .core_mut()
            .world_mut()
            .get::<Position>(rider.to_entity().expect("live"))
            .copied()
            .expect("position");
        assert_eq!(
            (after.x, after.y),
            (dropped.x, dropped.y),
            "a unit that just stepped off stays where it was put"
        );
    }

    #[test]
    fn boarding_is_refused_from_across_the_map() {
        let mut api = Api::new();
        let truck = spawn_vehicle(&mut api, 0.0, 2);
        let rider = spawn_unit(&mut api, 50.0);

        assert_eq!(
            api.board(rider, truck),
            Err(BoardError::TooFarAway { distance: 50.0 }),
            "the refusal says how far, so a caller can tell 'walk over' from 'the truck drove off'"
        );
        assert_eq!(api.vehicle_of(rider), None);
    }

    #[test]
    fn seats_run_out() {
        let mut api = Api::new();
        let truck = spawn_vehicle(&mut api, 0.0, 1);
        let first = spawn_unit(&mut api, 1.0);
        let second = spawn_unit(&mut api, 1.0);

        api.board(first, truck).expect("first aboard");
        assert_eq!(api.free_seats(truck), Some(0));
        assert_eq!(
            api.board(second, truck),
            Err(BoardError::NoSeatsLeft { seats: 1 })
        );
        assert_eq!(api.passengers(truck), vec![first]);
    }

    #[test]
    fn a_unit_rides_one_vehicle_at_a_time() {
        let mut api = Api::new();
        let first = spawn_vehicle(&mut api, 0.0, 2);
        let second = spawn_vehicle(&mut api, 1.0, 2);
        let rider = spawn_unit(&mut api, 0.5);

        api.board(rider, first).expect("board");
        assert_eq!(
            api.board(rider, second),
            Err(BoardError::AlreadyAboard(rider))
        );
    }

    #[test]
    fn a_rock_cannot_be_boarded() {
        let mut api = Api::new();
        let rock = {
            let entity = api
                .core_mut()
                .world_mut()
                .spawn(Position { x: 0.0, y: 0.0 })
                .id();
            EntityId::of(entity)
        };
        let rider = spawn_unit(&mut api, 1.0);

        assert_eq!(api.board(rider, rock), Err(BoardError::NotBoardable(rock)));
        assert_eq!(api.free_seats(rock), None);
    }

    #[test]
    fn unboarding_puts_the_unit_beside_the_vehicle() {
        let mut api = Api::new();
        let truck = spawn_vehicle(&mut api, 0.0, 2);
        let rider = spawn_unit(&mut api, 1.0);
        api.board(rider, truck).expect("board");
        api.order_move_to(&[truck], MoveTarget { x: 20.0, y: 0.0 });
        for _ in 0..50 {
            api.tick(100).expect("tick");
        }

        api.unboard(rider).expect("unboard");

        assert_eq!(api.vehicle_of(rider), None);
        let world = api.core().world();
        let rider_position = world
            .get::<Position>(rider.to_entity().expect("live"))
            .expect("position");
        assert!(
            (rider_position.x - (20.0 + UNBOARD_OFFSET)).abs() < 1e-3,
            "it steps off next to where the truck stopped"
        );
    }

    #[test]
    fn a_unit_that_is_not_aboard_cannot_step_off() {
        let mut api = Api::new();
        let rider = spawn_unit(&mut api, 0.0);
        assert_eq!(api.unboard(rider), Err(BoardError::NotAboard(rider)));
    }

    #[test]
    fn losing_the_vehicle_lets_the_passenger_go() {
        let mut api = Api::new();
        let truck = spawn_vehicle(&mut api, 0.0, 2);
        let rider = spawn_unit(&mut api, 1.0);
        api.board(rider, truck).expect("board");

        assert_eq!(api.despawn(&[truck]), 1);
        api.tick(16).expect("tick");

        assert_eq!(api.vehicle_of(rider), None, "no dangling passenger");
    }

    #[test]
    fn a_boarded_unit_moves_again_after_stepping_off() {
        let mut api = Api::new();
        let truck = spawn_vehicle(&mut api, 0.0, 2);
        let rider = spawn_unit(&mut api, 1.0);
        api.board(rider, truck).expect("board");
        api.unboard(rider).expect("unboard");

        api.order_move_to(&[rider], MoveTarget { x: 10.0, y: 0.0 });
        for _ in 0..50 {
            api.tick(100).expect("tick");
        }

        let world = api.core().world();
        let rider_position = world
            .get::<Position>(rider.to_entity().expect("live"))
            .expect("position");
        assert!((rider_position.x - 10.0).abs() < 1e-3);
    }
}
