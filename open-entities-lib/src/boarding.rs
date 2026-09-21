//! Boarding: putting units inside vehicles and letting them out again.
//!
//! A vehicle is any entity with [`Boardable`]; a passenger carries [`PassengerOf`]. While that
//! component is present the movement systems skip the unit entirely, so its position has exactly
//! one owner — the vehicle — and the two can never disagree about where it is.
//!
//! There are two ways in. [`Api::board`] is the primitive: it puts the unit inside at once, from
//! wherever it stands, and is what a scenario uses to start a truck already loaded. [`Api::order_board`]
//! is the order a player gives: it marks the unit with [`BoardingTarget`], and the boarding system
//! walks it over tick by tick and boards it once it is within [`BOARDING_RANGE`]. The core executes
//! the intent; whether to give it is the game's decision.
//!
//! See `docs/design/vehicle-seats.md`.

use bevy_ecs::prelude::{Entity, World};

use crate::api::Api;
use crate::components::{
    Boardable, BoardingTarget, MoveTarget, OrderSource, PassengerOf, Position, Velocity,
};
use crate::orders::{EntityId, OrderReport, can_take_a_move_order};

/// How close a unit has to be for the boarding system to put it aboard, in world units.
///
/// Not a limit on [`Api::board`], which takes a unit from anywhere: it is the point on the way
/// over at which an [`Api::order_board`] order turns into boarding.
pub const BOARDING_RANGE: f32 = 3.0;

/// Where a unit is put down when it leaves a vehicle, in world units from it.
const UNBOARD_OFFSET: f32 = 1.5;

/// Errors from boarding and unboarding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardError {
    /// The id does not resolve to a live entity.
    UnknownUnit(EntityId),
    /// The id does not resolve, or resolves to something without seats.
    NotBoardable(EntityId),
    /// The unit is already riding something.
    AlreadyAboard(EntityId),
    /// Every seat is taken.
    NoSeatsLeft { seats: u8 },
    /// The unit or the vehicle has no position, so it has no place in the world to board from.
    NoPosition(EntityId),
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
                "entity {}:{} has no position, so it cannot board",
                id.index, id.generation
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

/// Everyone riding `vehicle`.
pub(crate) fn passengers_of(world: &mut World, vehicle: Entity) -> Vec<Entity> {
    let mut query = world.query::<(Entity, &PassengerOf)>();
    query
        .iter(world)
        .filter(|(_, passenger_of)| passenger_of.0 == vehicle)
        .map(|(entity, _)| entity)
        .collect()
}

/// Puts `unit` inside `vehicle` right now. Shared by [`Api::board`] and the boarding system, so
/// the two cannot drift apart on what "aboard" means.
pub(crate) fn board_now(
    world: &mut World,
    unit: Entity,
    vehicle: Entity,
) -> Result<(), BoardError> {
    let unit_id = EntityId::of(unit);
    let vehicle_id = EntityId::of(vehicle);
    if !world.entities().contains_spawned(unit) {
        return Err(BoardError::UnknownUnit(unit_id));
    }
    if world.get::<PassengerOf>(unit).is_some() {
        return Err(BoardError::AlreadyAboard(unit_id));
    }
    let seats = world
        .get::<Boardable>(vehicle)
        .ok_or(BoardError::NotBoardable(vehicle_id))?
        .0;
    if world.get::<Position>(unit).is_none() {
        return Err(BoardError::NoPosition(unit_id));
    }
    if world.get::<Position>(vehicle).is_none() {
        return Err(BoardError::NoPosition(vehicle_id));
    }
    if passengers_of(world, vehicle).len() >= usize::from(seats) {
        return Err(BoardError::NoSeatsLeft { seats });
    }

    // The unit is not going anywhere under its own power any more. The move target and the
    // boarding order both go: a claim left behind would block the next order after stepping off.
    world
        .entity_mut(unit)
        .remove::<(MoveTarget, OrderSource, BoardingTarget)>()
        .insert(PassengerOf(vehicle));
    if let Some(mut velocity) = world.get_mut::<Velocity>(unit) {
        velocity.vx = 0.0;
        velocity.vy = 0.0;
    }
    Ok(())
}

impl Api {
    /// Puts a unit aboard a vehicle at once, wherever the two stand.
    ///
    /// Distance is not checked: this is the primitive, for scenarios that start a vehicle loaded
    /// and for the boarding system once a unit has walked over. The order a player gives is
    /// [`Api::order_board`].
    ///
    /// # Errors
    ///
    /// [`BoardError::NotBoardable`] when the vehicle has no seats, [`BoardError::AlreadyAboard`],
    /// [`BoardError::NoSeatsLeft`], and the `Unknown*` / `NoPosition` cases when an id does not
    /// resolve to something with a place in the world.
    pub fn board(&mut self, unit: EntityId, vehicle: EntityId) -> Result<(), BoardError> {
        let unit_entity = unit.to_entity().ok_or(BoardError::UnknownUnit(unit))?;
        let vehicle_entity = vehicle
            .to_entity()
            .ok_or(BoardError::NotBoardable(vehicle))?;
        board_now(self.core_mut().world_mut(), unit_entity, vehicle_entity)
    }

    /// Orders units to walk to a vehicle and get in.
    ///
    /// Each unit gets a [`BoardingTarget`]; from then on the boarding system steers it toward
    /// the vehicle every tick — following it if it drives off — and boards it once within
    /// [`BOARDING_RANGE`]. The order replaces whatever move order the unit had, and it is the
    /// unit's own order, so group and mission steering do not take it back.
    ///
    /// Units that cannot take a move order — unknown, immobile, without a position, already a
    /// passenger — are skipped and counted, as in [`Api::order_move_to`]. Whether a seat will
    /// still be free on arrival is not known now; a unit that finds none stays outside with its
    /// order dropped.
    ///
    /// # Errors
    ///
    /// [`BoardError::NotBoardable`] when `vehicle` is not something with seats: there is nothing
    /// to walk to, and no unit is ordered.
    pub fn order_board(
        &mut self,
        units: &[EntityId],
        vehicle: EntityId,
    ) -> Result<OrderReport, BoardError> {
        let world = self.core_mut().world_mut();
        let vehicle_entity = vehicle
            .to_entity()
            .filter(|entity| world.get::<Boardable>(*entity).is_some())
            .ok_or(BoardError::NotBoardable(vehicle))?;

        let mut ordered: Vec<Entity> = Vec::with_capacity(units.len());
        for id in units {
            let Some(entity) = id.to_entity() else {
                continue;
            };
            if ordered.contains(&entity) || entity == vehicle_entity {
                continue;
            }
            if !can_take_a_move_order(world, entity) {
                continue;
            }
            ordered.push(entity);
        }

        for entity in &ordered {
            world
                .entity_mut(*entity)
                .remove::<(MoveTarget, OrderSource)>()
                .insert(BoardingTarget(vehicle_entity));
        }

        Ok(OrderReport {
            ordered: ordered.len(),
            skipped: units.len() - ordered.len(),
        })
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

    /// The vehicle this unit is on its way to board, if it has such an order.
    #[must_use]
    pub fn boarding_target_of(&self, unit: EntityId) -> Option<EntityId> {
        let entity = unit.to_entity()?;
        let target = self.core().world().get::<BoardingTarget>(entity)?;
        Some(EntityId::of(target.0))
    }

    /// Everyone currently riding this vehicle.
    pub fn passengers(&mut self, vehicle: EntityId) -> Vec<EntityId> {
        let Some(vehicle_entity) = vehicle.to_entity() else {
            return Vec::new();
        };
        passengers_of(self.core_mut().world_mut(), vehicle_entity)
            .into_iter()
            .map(EntityId::of)
            .collect()
    }

    /// Everyone on their way to board this vehicle.
    pub fn approaching(&mut self, vehicle: EntityId) -> Vec<EntityId> {
        let Some(vehicle_entity) = vehicle.to_entity() else {
            return Vec::new();
        };
        let world = self.core_mut().world_mut();
        let mut query = world.query::<(Entity, &BoardingTarget)>();
        query
            .iter(world)
            .filter(|(_, target)| target.0 == vehicle_entity)
            .map(|(entity, _)| EntityId::of(entity))
            .collect()
    }

    /// Seats left on this vehicle, or `None` when it has none to begin with.
    pub fn free_seats(&mut self, vehicle: EntityId) -> Option<u8> {
        let vehicle_entity = vehicle.to_entity()?;
        let seats = self.core().world().get::<Boardable>(vehicle_entity)?.0;
        let world = self.core_mut().world_mut();
        let taken = u8::try_from(passengers_of(world, vehicle_entity).len()).unwrap_or(u8::MAX);
        Some(seats.saturating_sub(taken))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{BaseMoveSpeed, Faction};

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
    fn boarding_works_from_across_the_map() {
        let mut api = Api::new();
        let truck = spawn_vehicle(&mut api, 0.0, 2);
        let rider = spawn_unit(&mut api, 50.0);

        api.board(rider, truck)
            .expect("distance is the game's concern, not the core's");
        assert_eq!(api.vehicle_of(rider), Some(truck));
        api.tick(16).expect("tick");
        let world = api.core().world();
        let rider_position = world
            .get::<Position>(rider.to_entity().expect("live"))
            .expect("position");
        assert_eq!(rider_position.x, 0.0, "it is inside the truck now");
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

    fn position_of(api: &Api, id: EntityId) -> Position {
        *api.core()
            .world()
            .get::<Position>(id.to_entity().expect("live"))
            .expect("position")
    }

    #[test]
    fn an_ordered_unit_walks_over_and_gets_in() {
        let mut api = Api::new();
        let truck = spawn_vehicle(&mut api, 0.0, 2);
        let rider = spawn_unit(&mut api, 50.0);

        let report = api.order_board(&[rider], truck).expect("order");
        assert_eq!(
            report,
            OrderReport {
                ordered: 1,
                skipped: 0
            }
        );
        assert_eq!(api.boarding_target_of(rider), Some(truck));
        assert_eq!(api.approaching(truck), vec![rider]);

        // One tick: it has not teleported, it has taken a step toward the truck.
        api.tick(100).expect("tick");
        assert_eq!(api.vehicle_of(rider), None);
        let after_one = position_of(&api, rider);
        assert!(
            after_one.x < 50.0 && after_one.x > 40.0,
            "walking, at {}",
            after_one.x
        );

        for _ in 0..60 {
            api.tick(100).expect("tick");
        }
        assert_eq!(
            api.vehicle_of(rider),
            Some(truck),
            "aboard once within range"
        );
        assert_eq!(api.boarding_target_of(rider), None, "the order is spent");
        assert_eq!(api.approaching(truck), Vec::<EntityId>::new());
        assert_eq!(
            position_of(&api, rider).x,
            0.0,
            "and it is inside the truck"
        );
    }

    #[test]
    fn an_ordered_unit_follows_a_vehicle_that_drives_off() {
        let mut api = Api::new();
        let truck = spawn_vehicle(&mut api, 20.0, 2);
        let rider = spawn_unit(&mut api, 0.0);
        api.order_board(&[rider], truck).expect("order");
        // The truck (30/s) pulls away from the rider (10/s), then parks at 60.
        api.order_move_to(&[truck], MoveTarget { x: 60.0, y: 0.0 });

        for _ in 0..30 {
            api.tick(100).expect("tick");
        }
        assert_eq!(api.vehicle_of(rider), None, "still chasing");
        assert!(
            position_of(&api, rider).x > 20.0,
            "past where the truck used to be"
        );
        for _ in 0..60 {
            api.tick(100).expect("tick");
        }
        assert_eq!(
            api.vehicle_of(rider),
            Some(truck),
            "caught up at the new spot"
        );
        assert!((position_of(&api, rider).x - 60.0).abs() < 1e-3);
    }

    #[test]
    fn no_seat_on_arrival_leaves_the_unit_outside_with_no_order() {
        let mut api = Api::new();
        let truck = spawn_vehicle(&mut api, 0.0, 1);
        let first = spawn_unit(&mut api, 1.0);
        let second = spawn_unit(&mut api, 20.0);
        api.board(first, truck).expect("first aboard");
        api.order_board(&[second], truck).expect("order");

        for _ in 0..40 {
            api.tick(100).expect("tick");
        }
        assert_eq!(api.vehicle_of(second), None);
        assert_eq!(
            api.boarding_target_of(second),
            None,
            "the order is dropped, not retried"
        );
        let stood = position_of(&api, second);
        assert!(
            stood.x <= BOARDING_RANGE + 1e-3 && stood.x > 0.0,
            "stopped at the door, at {}",
            stood.x
        );
        api.tick(100).expect("tick");
        assert_eq!(position_of(&api, second).x, stood.x, "and stays there");
    }

    #[test]
    fn losing_the_vehicle_drops_the_boarding_order() {
        let mut api = Api::new();
        let truck = spawn_vehicle(&mut api, 0.0, 2);
        let rider = spawn_unit(&mut api, 30.0);
        api.order_board(&[rider], truck).expect("order");
        api.tick(100).expect("tick");

        assert_eq!(api.despawn(&[truck]), 1);
        api.tick(100).expect("tick");
        assert_eq!(api.boarding_target_of(rider), None);
        let stood = position_of(&api, rider);
        api.tick(100).expect("tick");
        assert_eq!(position_of(&api, rider).x, stood.x, "it stops where it was");
    }

    #[test]
    fn stop_and_a_new_move_order_both_cancel_boarding() {
        let mut api = Api::new();
        let truck = spawn_vehicle(&mut api, 0.0, 2);
        let a = spawn_unit(&mut api, 30.0);
        let b = spawn_unit(&mut api, 30.0);
        api.order_board(&[a, b], truck).expect("order");
        api.tick(100).expect("tick");

        api.order_stop(&[a]);
        api.order_move_to(&[b], MoveTarget { x: 60.0, y: 0.0 });
        assert_eq!(api.boarding_target_of(a), None);
        assert_eq!(api.boarding_target_of(b), None);

        for _ in 0..50 {
            api.tick(100).expect("tick");
        }
        assert_eq!(api.vehicle_of(a), None);
        assert_eq!(api.vehicle_of(b), None);
        assert!(
            (position_of(&api, b).x - 60.0).abs() < 1e-3,
            "b went where it was last told"
        );
    }

    #[test]
    fn a_boarding_order_is_the_units_own_and_automation_does_not_take_it() {
        let mut api = Api::new();
        let truck = spawn_vehicle(&mut api, 0.0, 2);
        let rider = spawn_unit(&mut api, 30.0);
        api.core_mut()
            .world_mut()
            .entity_mut(rider.to_entity().expect("live"))
            .insert(Faction(1));
        let group = api.create_group(1);
        api.add_to_group(group, rider).expect("member");
        api.order_board(&[rider], truck).expect("order");
        api.tick(100).expect("tick");

        // Group steering is weaker than a personal order and must not pull the rider off course.
        api.order_group_move_to(group, MoveTarget { x: 100.0, y: 0.0 })
            .expect("group order");
        for _ in 0..60 {
            api.tick(100).expect("tick");
        }
        assert_eq!(api.vehicle_of(rider), Some(truck));
    }

    #[test]
    fn ordering_units_into_a_rock_is_refused_up_front() {
        let mut api = Api::new();
        let rock = {
            let entity = api
                .core_mut()
                .world_mut()
                .spawn(Position { x: 0.0, y: 0.0 })
                .id();
            EntityId::of(entity)
        };
        let rider = spawn_unit(&mut api, 10.0);
        assert_eq!(
            api.order_board(&[rider], rock),
            Err(BoardError::NotBoardable(rock))
        );
        assert_eq!(api.boarding_target_of(rider), None);
    }

    #[test]
    fn a_passenger_and_the_vehicle_itself_are_skipped_by_the_order() {
        let mut api = Api::new();
        let truck = spawn_vehicle(&mut api, 0.0, 2);
        let aboard = spawn_unit(&mut api, 1.0);
        let walker = spawn_unit(&mut api, 10.0);
        api.board(aboard, truck).expect("aboard");

        // The player boxes the truck and everyone near it and presses B.
        let report = api
            .order_board(&[truck, aboard, walker], truck)
            .expect("order");
        assert_eq!(
            report,
            OrderReport {
                ordered: 1,
                skipped: 2
            }
        );
        assert_eq!(api.approaching(truck), vec![walker]);
    }
}
