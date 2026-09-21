//! Carrying and boarding: walking ordered units to their vehicle, and moving passengers with it.

use bevy_ecs::prelude::*;

use crate::boarding::{BOARDING_RANGE, board_now};
use crate::components::{
    Boardable, BoardingTarget, MoveTarget, OrderSource, PassengerOf, Position, Velocity,
};

/// Walks every unit with a [`BoardingTarget`] toward its vehicle and boards it on arrival.
///
/// Runs before seek, so the target it sets is what the unit steers by this tick. Each tick, per
/// unit, one of three things happens:
///
/// - the vehicle is gone or has lost its seats: the order is dropped, the unit stays put;
/// - the unit is within [`BOARDING_RANGE`]: it boards; if there is no seat left the order is
///   dropped instead — the unit stands outside, and the host can see that from the snapshot;
/// - otherwise its [`MoveTarget`] is set to where the vehicle is *now*, which is what makes it
///   follow a vehicle that drives off rather than walk to where it used to be.
///
/// The target is stamped [`OrderSource::PlayerUnit`]: boarding is the unit's own order, and
/// group or mission steering must not pull it away halfway.
///
/// Exclusive access is deliberate: boarding is a multi-component transaction on two entities
/// (`board_now`), and a system that both reads the world and boards is simpler than one that
/// queues commands and reasons about what it already decided this tick.
pub fn boarding_approach_system(world: &mut World) {
    let mut query = world.query::<(Entity, &BoardingTarget)>();
    let orders: Vec<(Entity, Entity)> = query
        .iter(world)
        .map(|(unit, target)| (unit, target.0))
        .collect();

    for (unit, vehicle) in orders {
        let (Some(unit_position), Some(vehicle_position)) = (
            world.get::<Position>(unit).copied(),
            world
                .get::<Position>(vehicle)
                .copied()
                .filter(|_| world.get::<Boardable>(vehicle).is_some()),
        ) else {
            drop_order(world, unit);
            continue;
        };
        if world.get::<PassengerOf>(unit).is_some() {
            // Something else put it aboard meanwhile; the order is done.
            drop_order(world, unit);
            continue;
        }

        let dx = vehicle_position.x - unit_position.x;
        let dy = vehicle_position.y - unit_position.y;
        if dx.hypot(dy) <= BOARDING_RANGE {
            // The expected refusal is `NoSeatsLeft`; the rest were checked above, and a refusal
            // there means the world changed under us. Either way the order is over.
            if board_now(world, unit, vehicle).is_err() {
                drop_order(world, unit);
            }
            continue;
        }

        if world.get::<Velocity>(unit).is_none() {
            world.entity_mut(unit).insert(Velocity { vx: 0.0, vy: 0.0 });
        }
        world.entity_mut(unit).insert((
            MoveTarget {
                x: vehicle_position.x,
                y: vehicle_position.y,
            },
            OrderSource::PlayerUnit,
        ));
    }
}

/// Ends a boarding order without boarding: the unit stops where it is.
fn drop_order(world: &mut World, unit: Entity) {
    world
        .entity_mut(unit)
        .remove::<(BoardingTarget, MoveTarget, OrderSource)>();
    if let Some(mut velocity) = world.get_mut::<Velocity>(unit) {
        velocity.vx = 0.0;
        velocity.vy = 0.0;
    }
}

/// Copies each vehicle's position onto the units it carries.
///
/// Runs after movement, so passengers land where the vehicle actually ended the tick. A passenger
/// whose vehicle is gone is let off where it stands rather than left pointing at nothing: an
/// entity that belongs to a missing vehicle is a leak waiting to be read.
pub fn passenger_sync_system(
    mut commands: Commands,
    mut passengers: Query<(Entity, &PassengerOf, &mut Position, Option<&mut Velocity>)>,
    vehicles: Query<&Position, Without<PassengerOf>>,
) {
    for (entity, passenger_of, mut position, velocity) in &mut passengers {
        let Ok(vehicle_position) = vehicles.get(passenger_of.0) else {
            commands.entity(entity).remove::<PassengerOf>();
            continue;
        };
        position.x = vehicle_position.x;
        position.y = vehicle_position.y;
        if let Some(mut velocity) = velocity {
            // The vehicle does the travelling; a passenger with its own velocity would be
            // integrated twice the moment it steps off.
            velocity.vx = 0.0;
            velocity.vy = 0.0;
        }
    }
}
