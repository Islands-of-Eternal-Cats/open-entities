//! Roadmap step 3, determinism audit: whenever the simulation picks one of several, the lowest
//! [`EntityId`] wins — never whichever entity a query happened to visit first.
//!
//! Query order follows the ECS storage, which shifts whenever an entity gains or loses a
//! component. Each test below shuffles that order on purpose and checks the outcome did not move.

use bevy_ecs::prelude::{Component, Entity};
use bevy_ecs::schedule::LogLevel;
use open_entities::components::{
    BaseMoveSpeed, Boardable, BoardingTarget, Faction, MoveTarget, Position, Velocity,
};
use open_entities::{Api, EntityId};

/// Throwaway component used only to move an entity to the end of its storage table.
#[derive(Component)]
struct Shuffle;

/// Moves `entity` behind every other entity of its archetype in query order.
fn send_to_back(api: &mut Api, entity: Entity) {
    let world = api.core_mut().world_mut();
    world.entity_mut(entity).insert(Shuffle);
    world.entity_mut(entity).remove::<Shuffle>();
}

fn walker(api: &mut Api, x: i32) -> Entity {
    api.core_mut()
        .world_mut()
        .spawn((
            Position { x, y: 0 },
            BaseMoveSpeed(500),
            Velocity { vx: 0, vy: 0 },
            Faction(1),
        ))
        .id()
}

#[test]
fn the_last_seat_goes_to_the_lowest_id() {
    let mut api = Api::new();
    let truck = api
        .core_mut()
        .world_mut()
        .spawn((Position { x: 0, y: 0 }, Boardable(1)))
        .id();
    let low = walker(&mut api, 1000);
    let high = walker(&mut api, -1000);
    let truck_id = EntityId::of(truck);
    api.order_board(&[EntityId::of(low), EntityId::of(high)], truck_id)
        .expect("order");
    // Both are in range on the first tick and there is one seat.
    send_to_back(&mut api, low);

    api.step();

    assert_eq!(api.vehicle_of(EntityId::of(low)), Some(truck_id));
    assert_eq!(api.vehicle_of(EntityId::of(high)), None);
    assert!(
        api.core().world().get::<BoardingTarget>(high).is_none(),
        "the one left outside has its order dropped"
    );
}

#[test]
fn group_slots_follow_entity_ids() {
    let mut api = Api::new();
    let group = api.create_group(1);
    let low = walker(&mut api, 0);
    let high = walker(&mut api, 0);
    api.add_to_group(group, EntityId::of(low)).expect("join");
    api.add_to_group(group, EntityId::of(high)).expect("join");
    send_to_back(&mut api, low);

    api.order_group_move_to(group, MoveTarget { x: 50_000, y: 0 })
        .expect("order");

    // Two members: one row of two columns, 5 units apart; slot 0 is the left one.
    let world = api.core().world();
    assert_eq!(
        world.get::<MoveTarget>(low),
        Some(&MoveTarget { x: 47_500, y: 0 })
    );
    assert_eq!(
        world.get::<MoveTarget>(high),
        Some(&MoveTarget { x: 52_500, y: 0 })
    );
}

#[test]
fn mission_slots_follow_entity_ids() {
    let mut api = Api::new();
    let group = api.create_group(1);
    let low = walker(&mut api, 0);
    let high = walker(&mut api, 0);
    api.add_to_group(group, EntityId::of(low)).expect("join");
    api.add_to_group(group, EntityId::of(high)).expect("join");
    let mission = api.create_mission(MoveTarget { x: 50_000, y: 0 }, 1000);
    api.assign_group(mission, group).expect("assign");
    send_to_back(&mut api, low);

    api.step();

    let world = api.core().world();
    assert_eq!(
        world.get::<MoveTarget>(low),
        Some(&MoveTarget { x: 47_500, y: 0 })
    );
    assert_eq!(
        world.get::<MoveTarget>(high),
        Some(&MoveTarget { x: 52_500, y: 0 })
    );
}

#[test]
fn listings_come_in_id_order() {
    let mut api = Api::new();
    let group = api.create_group(1);
    let truck = api
        .core_mut()
        .world_mut()
        .spawn((Position { x: 0, y: 0 }, Boardable(4)))
        .id();
    let units: Vec<Entity> = (0..3).map(|_| walker(&mut api, 0)).collect();
    let ids: Vec<EntityId> = units.iter().copied().map(EntityId::of).collect();
    for id in &ids {
        api.add_to_group(group, *id).expect("join");
        api.board(*id, EntityId::of(truck)).expect("board");
    }
    send_to_back(&mut api, units[0]);

    assert_eq!(api.group_members(group), ids);
    assert_eq!(api.passengers(EntityId::of(truck)), ids);
}

#[test]
fn ambiguity_detection_is_an_error_in_tests() {
    let api = Api::new();
    assert_eq!(
        api.core()
            .schedule()
            .get_build_settings()
            .ambiguity_detection,
        LogLevel::Error
    );
}
