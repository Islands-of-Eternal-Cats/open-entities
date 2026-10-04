//! Rarely changing per-entity data for a renderer, sent only when it changes.
//!
//! The [position frame](super::frame) carries what moves every tick. Everything else a renderer
//! shows — template name, faction, seats, who rides or walks to what, group, move target —
//! changes seldom, so [`Api::meta_delta`] reports only the entities whose metadata changed, or
//! that went away, since the previous call.
//!
//! Changes are found with `bevy_ecs` change detection and removal tracking, not by scanning the
//! world, and each candidate is compared with what was last reported, so a system that writes
//! the same value again (mission steering re-inserts its target every tick) is not a change.

use std::collections::BTreeSet;

use bevy_ecs::prelude::{Added, Changed, Entity, Or, Query, RemovedComponents, With, World};
use bevy_ecs::system::SystemState;
use serde::Serialize;

use crate::api::Api;
use crate::components::{
    Boardable, BoardingTarget, EntityType, Faction, MemberOf, MoveTarget, PassengerOf, Position,
    Velocity,
};
use crate::orders::EntityId;
use crate::state_hash::StateHasher;

/// Metadata of one entity: everything a renderer needs besides its position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EntityMeta {
    /// The entity.
    pub id: EntityId,
    /// Template name it was spawned from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    /// Faction id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faction: Option<u32>,
    /// Seats, when it is a vehicle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seats: Option<u8>,
    /// `true` when it carries a [`Velocity`]: in this engine, when it can move at all.
    pub mobile: bool,
    /// The vehicle it rides.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aboard: Option<EntityId>,
    /// The vehicle it walks to board.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boarding: Option<EntityId>,
    /// The group it belongs to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<EntityId>,
    /// Where it has been told to go; serialises in map units.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub move_target: Option<MoveTarget>,
}

/// What changed since the previous [`Api::meta_delta`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct MetaDelta {
    /// Entities that are new or whose metadata changed, with their current metadata, by id.
    pub changed: Vec<EntityMeta>,
    /// Entities reported before that are gone (despawned, or no longer positioned), by id.
    pub removed: Vec<EntityId>,
}

impl MetaDelta {
    /// `true` when nothing changed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.changed.is_empty() && self.removed.is_empty()
    }
}

/// Entities whose metadata may have changed: a positioned entity that gained a position or
/// velocity, or had a metadata component inserted or written.
type TouchedQuery<'w, 's> = Query<
    'w,
    's,
    Entity,
    (
        With<Position>,
        Or<(
            Added<Position>,
            Added<Velocity>,
            Changed<EntityType>,
            Changed<Faction>,
            Changed<Boardable>,
            Changed<PassengerOf>,
            Changed<BoardingTarget>,
            Changed<MemberOf>,
            Changed<MoveTarget>,
        )>,
    ),
>;

/// Entities that lost a component the metadata (or the frame) depends on, despawns included.
type Removals<'w, 's> = (
    RemovedComponents<'w, 's, Position>,
    RemovedComponents<'w, 's, Velocity>,
    RemovedComponents<'w, 's, EntityType>,
    RemovedComponents<'w, 's, Faction>,
    RemovedComponents<'w, 's, Boardable>,
    RemovedComponents<'w, 's, PassengerOf>,
    RemovedComponents<'w, 's, BoardingTarget>,
    RemovedComponents<'w, 's, MemberOf>,
    RemovedComponents<'w, 's, MoveTarget>,
);

/// Change tracking behind [`Api::meta_delta`]; created by its first call.
pub(crate) struct MetaTracker {
    state: SystemState<(TouchedQuery<'static, 'static>, Removals<'static, 'static>)>,
    /// Entities touched since the last delta.
    touched: BTreeSet<Entity>,
    /// Per entity index: the generation and metadata fingerprint last reported.
    reported: Vec<Option<(u32, u64)>>,
}

impl MetaTracker {
    /// Starts tracking with every positioned entity touched, so the first delta is the full set.
    fn new(world: &mut World) -> Self {
        let state = SystemState::new(world);
        let touched = world
            .query_filtered::<Entity, With<Position>>()
            .iter(world)
            .collect();
        Self {
            state,
            touched,
            reported: Vec::new(),
        }
    }

    /// Notes what changed since the previous collection.
    ///
    /// Must run before [`World::clear_trackers`] drops the removal records it reads; `Api::step`
    /// calls it right before clearing them.
    pub(crate) fn collect(&mut self, world: &mut World) {
        let (touched, removals) = self
            .state
            .get_mut(world)
            .expect("meta tracking reads only components and removals");
        self.touched.extend(touched.iter());
        let (
            mut position,
            mut velocity,
            mut entity_type,
            mut faction,
            mut boardable,
            mut passenger,
            mut boarding,
            mut member,
            mut move_target,
        ) = removals;
        self.touched.extend(position.read());
        self.touched.extend(velocity.read());
        self.touched.extend(entity_type.read());
        self.touched.extend(faction.read());
        self.touched.extend(boardable.read());
        self.touched.extend(passenger.read());
        self.touched.extend(boarding.read());
        self.touched.extend(member.read());
        self.touched.extend(move_target.read());
    }

    fn delta(&mut self, world: &World) -> MetaDelta {
        let mut delta = MetaDelta::default();
        for entity in std::mem::take(&mut self.touched) {
            let slot = usize::try_from(entity.index_u32()).expect("entity index fits usize");
            if self.reported.len() <= slot {
                self.reported.resize(slot + 1, None);
            }
            let id = EntityId::of(entity);
            let last = self.reported[slot];
            match read_meta(world, entity) {
                Some(meta) => {
                    let fingerprint = fingerprint(&meta);
                    if let Some((generation, _)) = last
                        && generation != id.generation
                    {
                        // The slot was reused: whoever held it before is gone.
                        delta.removed.push(EntityId::new(id.index, generation));
                    }
                    if last != Some((id.generation, fingerprint)) {
                        self.reported[slot] = Some((id.generation, fingerprint));
                        delta.changed.push(meta);
                    }
                }
                None => {
                    if let Some((generation, _)) = last
                        && generation == id.generation
                    {
                        self.reported[slot] = None;
                        delta.removed.push(id);
                    }
                }
            }
        }
        delta.changed.sort_unstable_by_key(|meta| meta.id);
        delta.removed.sort_unstable();
        delta.removed.dedup();
        delta
    }
}

/// Current metadata of a live, positioned entity; `None` for anything else.
fn read_meta(world: &World, entity: Entity) -> Option<EntityMeta> {
    let entity_ref = world.get_entity(entity).ok()?;
    entity_ref.get::<Position>()?;
    Some(EntityMeta {
        id: EntityId::of(entity),
        entity_type: entity_ref.get::<EntityType>().map(|t| t.0.clone()),
        faction: entity_ref.get::<Faction>().map(|f| f.0),
        seats: entity_ref.get::<Boardable>().map(|b| b.0),
        mobile: entity_ref.contains::<Velocity>(),
        aboard: entity_ref.get::<PassengerOf>().map(|p| EntityId::of(p.0)),
        boarding: entity_ref
            .get::<BoardingTarget>()
            .map(|b| EntityId::of(b.0)),
        group: entity_ref.get::<MemberOf>().map(|m| EntityId::of(m.0)),
        move_target: entity_ref.get::<MoveTarget>().copied(),
    })
}

/// FNV-1a over the metadata, so the tracker keeps eight bytes per entity rather than a copy.
fn fingerprint(meta: &EntityMeta) -> u64 {
    fn id(hasher: &mut StateHasher, id: Option<EntityId>) {
        match id {
            Some(id) => {
                hasher.write_u8(1);
                hasher.write_u32(id.index);
                hasher.write_u32(id.generation);
            }
            None => hasher.write_u8(0),
        }
    }
    let mut hasher = StateHasher::new();
    match &meta.entity_type {
        Some(name) => {
            hasher.write_u8(1);
            hasher.write_str(name);
        }
        None => hasher.write_u8(0),
    }
    match meta.faction {
        Some(faction) => {
            hasher.write_u8(1);
            hasher.write_u32(faction);
        }
        None => hasher.write_u8(0),
    }
    hasher.write_u32(meta.seats.map_or(0, |seats| u32::from(seats) + 1));
    hasher.write_u8(u8::from(meta.mobile));
    id(&mut hasher, meta.aboard);
    id(&mut hasher, meta.boarding);
    id(&mut hasher, meta.group);
    match meta.move_target {
        Some(target) => {
            hasher.write_u8(1);
            hasher.write_i32(target.x);
            hasher.write_i32(target.y);
        }
        None => hasher.write_u8(0),
    }
    hasher.finish()
}

impl Api {
    /// Metadata of the entities that changed or went away since the previous call.
    ///
    /// The first call reports every positioned entity and starts the tracking; from then on each
    /// [`step`](Api::step) notes what changed, and a call collects it. A renderer calls it after
    /// its steps and sends the result only when it is not [empty](MetaDelta::is_empty). Rows and
    /// removals are sorted by [`EntityId`].
    ///
    /// Until the first call nothing is tracked, so a host that never asks pays nothing.
    pub fn meta_delta(&mut self) -> MetaDelta {
        let tracker = self.meta.take();
        let world = self.core_mut().world_mut();
        let mut tracker = tracker.unwrap_or_else(|| MetaTracker::new(world));
        tracker.collect(world);
        let delta = tracker.delta(world);
        self.meta = Some(tracker);
        delta
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::entity::EntityGeneration;

    /// A reused slot cannot be produced on demand through the public API (the allocator hands out
    /// fresh indices first), so this plants an older generation in the tracker's record.
    #[test]
    fn a_reused_slot_reports_the_previous_holder_removed() {
        let mut api = Api::new();
        let entity = api
            .core_mut()
            .world_mut()
            .spawn(Position { x: 0, y: 0 })
            .id();
        let world = api.core_mut().world_mut();
        let mut tracker = MetaTracker::new(world);
        tracker.collect(world);
        let first = tracker.delta(world);
        assert_eq!(first.changed.len(), 1);

        let slot = usize::try_from(entity.index_u32()).expect("fits");
        let previous = EntityGeneration::from_bits(entity.generation().to_bits() + 7);
        tracker.reported[slot] = Some((previous.to_bits(), 0));
        tracker.touched.insert(entity);

        let delta = tracker.delta(world);
        assert_eq!(
            delta.removed,
            vec![EntityId::new(entity.index_u32(), previous.to_bits())]
        );
        assert_eq!(delta.changed.len(), 1);
        assert_eq!(delta.changed[0].id, EntityId::of(entity));
    }
}
