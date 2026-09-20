//! Carrying: a passenger's position belongs to the vehicle it rides.

use bevy_ecs::prelude::*;

use crate::components::{PassengerOf, Position, Velocity};

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
