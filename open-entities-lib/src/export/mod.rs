//! State out of the simulation.
//!
//! - [`Api::world_snapshot`](crate::Api::world_snapshot): every entity with every registered
//!   component, plain data with a stable `Serialize` shape; picking the wire format (JSON for the
//!   WASM host) is left to the caller. For debugging and saves, not for every frame.
//! - [`Api::write_frame`](crate::Api::write_frame): positions only, as flat `i32`s — the per-tick
//!   render path.
//! - [`Api::meta_delta`](crate::Api::meta_delta): the rarely changing rest, only what changed.

mod frame;
mod meta;

pub use frame::{FRAME_HEADER_LEN, FRAME_STRIDE, write_frame_header};
pub(crate) use meta::MetaTracker;
pub use meta::{EntityMeta, MetaDelta};

use bevy_ecs::prelude::{EntityRef, World};
use bevy_ecs::query::Without;
use bevy_ecs::resource::IsResource;
use serde::Serialize;

use crate::api::Api;
use crate::component_registry::ComponentRegistry;
use crate::components::EntityType;
use crate::entity_components::EntityComponents;
use crate::orders::EntityId;
use crate::simulation::SimTick;

/// Schema version reported in [`WorldSnapshot::version`].
pub const SCHEMA_VERSION: u32 = 5;

/// Every entity in the world at one moment.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorldSnapshot {
    /// Snapshot schema version, [`SCHEMA_VERSION`].
    pub version: u32,
    /// Simulation tick the snapshot was taken at, [`Api::current_tick`].
    pub tick: u64,
    /// One row per entity.
    pub entities: Vec<EntitySnapshot>,
}

/// One entity of a [`WorldSnapshot`].
///
/// Serializes flat: registered components — built-in and a game's own — sit next to `id`, and
/// absent ones are omitted.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EntitySnapshot {
    /// Id to feed back into orders.
    pub id: EntityId,
    /// Registered gameplay components present on the entity.
    #[serde(flatten)]
    pub components: EntityComponents,
    /// Template name the entity was spawned from, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<EntityType>,
}

impl Api {
    /// Captures every entity in the world (schema version 5).
    #[must_use]
    pub fn world_snapshot(&mut self) -> WorldSnapshot {
        world_snapshot_from_world(self.core.world_mut(), &self.registry)
    }
}

fn world_snapshot_from_world(world: &mut World, registry: &ComponentRegistry) -> WorldSnapshot {
    let tick = world.resource::<SimTick>().0;
    let mut query = world.query_filtered::<EntityRef<'_>, Without<IsResource>>();
    let entities = query
        .iter(world)
        .map(|entity| EntitySnapshot {
            id: EntityId::of(entity.id()),
            components: registry.export(&entity),
            entity_type: entity.get::<EntityType>().cloned(),
        })
        .collect();

    WorldSnapshot {
        version: SCHEMA_VERSION,
        tick,
        entities,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{BaseMoveSpeed, EntityType, Faction, Health, Position, Velocity};

    #[test]
    fn empty_world() {
        let mut api = Api::new();
        let snapshot = api.world_snapshot();
        assert_eq!(snapshot.version, 5);
        assert_eq!(snapshot.tick, 0);
        assert_eq!(snapshot.entities, []);
    }

    #[test]
    fn includes_positioned_entities() {
        let mut api = Api::new();
        let entity = api
            .core_mut()
            .world_mut()
            .spawn(Position { x: 1000, y: 2000 })
            .id();

        let snapshot = api.world_snapshot();
        assert_eq!(snapshot.entities.len(), 1);
        let row = &snapshot.entities[0];
        assert_eq!(row.id, EntityId::of(entity));
        assert_eq!(row.components.position, Some(Position { x: 1000, y: 2000 }));
    }

    #[test]
    fn partial_components() {
        let mut api = Api::new();
        api.core_mut()
            .world_mut()
            .spawn((Position { x: 1000, y: 2000 }, Velocity { vx: 25, vy: -25 }));

        let row = &api.world_snapshot().entities[0];
        assert_eq!(row.components.velocity, Some(Velocity { vx: 25, vy: -25 }));
        assert!(row.components.faction.is_none());
        assert!(row.components.move_target.is_none());
        assert!(row.entity_type.is_none());
    }

    #[test]
    fn single_component_entities() {
        let mut api = Api::new();
        let world = api.core_mut().world_mut();
        world.spawn(Faction(2));
        world.spawn(EntityType("marker".to_owned()));
        world.spawn(Health {
            current: 80,
            max: 100,
        });
        world.spawn((Position { x: 1000, y: 2000 }, BaseMoveSpeed(125)));

        let snapshot = api.world_snapshot();
        let rows = &snapshot.entities;
        assert_eq!(rows.len(), 4);
        assert!(
            rows.iter()
                .any(|r| r.components.faction == Some(Faction(2)))
        );
        assert!(
            rows.iter()
                .any(|r| r.entity_type == Some(EntityType("marker".to_owned())))
        );
        assert!(rows.iter().any(|r| r.components.health
            == Some(Health {
                current: 80,
                max: 100
            })));
        assert!(
            rows.iter()
                .any(|r| r.components.base_move_speed == Some(BaseMoveSpeed(125)))
        );
    }

    #[test]
    fn includes_entity_with_no_components() {
        let mut api = Api::new();
        api.core_mut().world_mut().spawn_empty();

        let snapshot = api.world_snapshot();
        assert_eq!(snapshot.entities.len(), 1);
        let row = &snapshot.entities[0];
        assert_eq!(row.components, EntityComponents::default());
        assert!(row.entity_type.is_none());
    }

    /// The serialized shape is the host contract: flat rows, absent components omitted.
    #[test]
    fn serializes_flat_and_omits_absent() {
        let mut api = Api::new();
        api.core_mut().world_mut().spawn((
            Position { x: 1000, y: 2000 },
            EntityType("scout".to_owned()),
        ));
        api.core_mut().world_mut().spawn(Faction(2));

        let value = serde_json::to_value(api.world_snapshot()).expect("serialize");
        assert_eq!(value["version"], 5);
        assert_eq!(value["tick"], 0);
        let entities = value["entities"].as_array().expect("entities array");
        let scout = entities
            .iter()
            .find(|e| e["entity_type"] == "scout")
            .expect("scout row");
        assert_eq!(scout["position"]["x"], 1.0);
        assert!(scout["id"]["index"].is_number());
        assert!(scout["id"]["generation"].is_number());
        assert!(scout.get("faction").is_none());
        let other = entities
            .iter()
            .find(|e| e["faction"] == 2)
            .expect("faction row");
        assert!(other.get("position").is_none());
        assert!(other.get("entity_type").is_none());
    }
}
