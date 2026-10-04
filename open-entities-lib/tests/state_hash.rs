//! Roadmap step 3: [`Api::state_hash`] sees every piece of simulation state, and nothing else.
//!
//! For each hashed component, every field on its own must move the hash. A field the hash misses
//! is a desync two peers would never notice.

use bevy_ecs::prelude::{Bundle, Entity};
use open_entities::components::{
    AssignedTo, BaseMoveSpeed, Boardable, BoardingTarget, EntityType, Faction, Group, Health,
    ManualActive, MemberOf, Mission, MissionCompleted, MoveTarget, NeedsMission, OrderSource,
    PassengerOf, Position, Velocity,
};
use open_entities::{Api, EntityId};

/// A world with two spare entities to point relations at, and the entity under test.
struct Fixture {
    api: Api,
    subject: Entity,
    other_a: Entity,
    other_b: Entity,
}

fn fixture() -> Fixture {
    let mut api = Api::new();
    let world = api.core_mut().world_mut();
    let other_a = world.spawn_empty().id();
    let other_b = world.spawn_empty().id();
    let subject = world.spawn_empty().id();
    Fixture {
        api,
        subject,
        other_a,
        other_b,
    }
}

/// Hash with `base` on the subject, then with `changed`: the two must differ.
fn assert_field_is_hashed<B: Bundle>(name: &str, make: impl Fn(&Fixture) -> (B, B)) {
    let mut fixture = fixture();
    let (base, changed) = make(&fixture);

    fixture
        .api
        .core_mut()
        .world_mut()
        .entity_mut(fixture.subject)
        .insert(base);
    let before = fixture.api.state_hash();

    fixture
        .api
        .core_mut()
        .world_mut()
        .entity_mut(fixture.subject)
        .insert(changed);
    let after = fixture.api.state_hash();

    assert_ne!(before, after, "{name}: changing it must change the hash");
}

/// Hash without, then with, a marker component: adding it must change the hash.
fn assert_marker_is_hashed<B: Bundle>(name: &str, marker: B) {
    let mut fixture = fixture();
    let before = fixture.api.state_hash();
    fixture
        .api
        .core_mut()
        .world_mut()
        .entity_mut(fixture.subject)
        .insert(marker);
    assert_ne!(
        before,
        fixture.api.state_hash(),
        "{name}: adding it must change the hash"
    );
}

#[test]
fn position_fields_are_hashed() {
    assert_field_is_hashed("Position.x", |_| {
        (Position { x: 1, y: 2 }, Position { x: 9, y: 2 })
    });
    assert_field_is_hashed("Position.y", |_| {
        (Position { x: 1, y: 2 }, Position { x: 1, y: 9 })
    });
}

#[test]
fn velocity_fields_are_hashed() {
    assert_field_is_hashed("Velocity.vx", |_| {
        (Velocity { vx: 1, vy: 2 }, Velocity { vx: 9, vy: 2 })
    });
    assert_field_is_hashed("Velocity.vy", |_| {
        (Velocity { vx: 1, vy: 2 }, Velocity { vx: 1, vy: 9 })
    });
}

#[test]
fn move_target_fields_are_hashed() {
    assert_field_is_hashed("MoveTarget.x", |_| {
        (MoveTarget { x: 1, y: 2 }, MoveTarget { x: 9, y: 2 })
    });
    assert_field_is_hashed("MoveTarget.y", |_| {
        (MoveTarget { x: 1, y: 2 }, MoveTarget { x: 1, y: 9 })
    });
}

#[test]
fn scalar_components_are_hashed() {
    assert_field_is_hashed("Faction", |_| (Faction(1), Faction(2)));
    assert_field_is_hashed("BaseMoveSpeed", |_| (BaseMoveSpeed(25), BaseMoveSpeed(26)));
    assert_field_is_hashed("Boardable", |_| (Boardable(2), Boardable(3)));
    assert_field_is_hashed("Group.faction", |_| {
        (Group { faction: 1 }, Group { faction: 2 })
    });
    assert_field_is_hashed("EntityType", |_| {
        (
            EntityType("scout".to_owned()),
            EntityType("scouts".to_owned()),
        )
    });
}

#[test]
fn health_fields_are_hashed() {
    assert_field_is_hashed("Health.current", |_| {
        (Health { current: 1, max: 5 }, Health { current: 2, max: 5 })
    });
    assert_field_is_hashed("Health.max", |_| {
        (Health { current: 1, max: 5 }, Health { current: 1, max: 6 })
    });
}

#[test]
fn mission_fields_are_hashed() {
    let mission = |x, y, radius| Mission {
        target: MoveTarget { x, y },
        radius,
    };
    assert_field_is_hashed("Mission.target.x", |_| (mission(1, 2, 3), mission(9, 2, 3)));
    assert_field_is_hashed("Mission.target.y", |_| (mission(1, 2, 3), mission(1, 9, 3)));
    assert_field_is_hashed("Mission.radius", |_| (mission(1, 2, 3), mission(1, 2, 9)));
}

#[test]
fn order_source_is_hashed() {
    assert_field_is_hashed("OrderSource", |_| {
        (OrderSource::MissionSteering, OrderSource::GroupSteering)
    });
    assert_field_is_hashed("OrderSource", |_| {
        (OrderSource::GroupSteering, OrderSource::PlayerUnit)
    });
}

#[test]
fn relations_are_hashed_by_the_entity_they_point_at() {
    assert_field_is_hashed("PassengerOf", |f| {
        (PassengerOf(f.other_a), PassengerOf(f.other_b))
    });
    assert_field_is_hashed("BoardingTarget", |f| {
        (BoardingTarget(f.other_a), BoardingTarget(f.other_b))
    });
    assert_field_is_hashed("MemberOf", |f| (MemberOf(f.other_a), MemberOf(f.other_b)));
    assert_field_is_hashed("AssignedTo", |f| {
        (AssignedTo(f.other_a), AssignedTo(f.other_b))
    });
}

#[test]
fn markers_are_hashed() {
    assert_marker_is_hashed("ManualActive", ManualActive);
    assert_marker_is_hashed("MissionCompleted", MissionCompleted);
    assert_marker_is_hashed("NeedsMission", NeedsMission);
}

#[test]
fn the_tick_is_hashed() {
    let mut api = Api::new();
    let before = api.state_hash();
    api.step();
    assert_ne!(before, api.state_hash());
}

#[test]
fn entity_ids_are_hashed() {
    // Two worlds with the same components on different ids are different states: every later
    // command names entities by id.
    let mut first = Api::new();
    first.core_mut().world_mut().spawn(Faction(1));

    let mut second = Api::new();
    let gone = second.core_mut().world_mut().spawn_empty().id();
    second.core_mut().world_mut().despawn(gone);
    second.core_mut().world_mut().spawn(Faction(1));

    assert_ne!(first.state_hash(), second.state_hash());
}

#[test]
fn the_same_state_hashes_the_same() {
    let build = || {
        let mut api = Api::new();
        let world = api.core_mut().world_mut();
        let unit = world
            .spawn((Position { x: 5, y: 6 }, Faction(1), OrderSource::PlayerUnit))
            .id();
        world.spawn((PassengerOf(unit), Health { current: 1, max: 2 }));
        api
    };
    assert_eq!(build().state_hash(), build().state_hash());
}

#[test]
fn per_tick_scratch_is_not_hashed() {
    // `ArrivedThisTick` is filled during a step and means nothing between steps; two worlds that
    // differ only in it are the same state.
    let mut api = Api::new();
    let entity = api.core_mut().world_mut().spawn(Faction(1)).id();
    let before = api.state_hash();
    api.core_mut()
        .world_mut()
        .resource_mut::<open_entities::simulation::ArrivedThisTick>()
        .0
        .insert(entity);
    assert_eq!(before, api.state_hash());
}

#[test]
fn the_hash_does_not_depend_on_component_insertion_order() {
    let mut first = Api::new();
    let a = first.core_mut().world_mut().spawn(Faction(1)).id();
    first
        .core_mut()
        .world_mut()
        .entity_mut(a)
        .insert(Position { x: 1, y: 1 });

    let mut second = Api::new();
    let b = second
        .core_mut()
        .world_mut()
        .spawn(Position { x: 1, y: 1 })
        .id();
    second
        .core_mut()
        .world_mut()
        .entity_mut(b)
        .insert(Faction(1));

    assert_eq!(EntityId::of(a), EntityId::of(b));
    assert_eq!(first.state_hash(), second.state_hash());
}
