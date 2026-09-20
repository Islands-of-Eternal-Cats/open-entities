//! Gameplay commands issued from outside the simulation.
//!
//! Orders address entities by [`EntityId`] — the same `{index, generation}` pair that
//! [`Api::world_json`](crate::Api::world_json) exports as each entity's `id`, so a host can feed
//! ids straight back from a snapshot.

use bevy_ecs::entity::{Entity, EntityGeneration, EntityIndex};
use bevy_ecs::prelude::World;
use serde::{Deserialize, Serialize};

use crate::api::Api;
use crate::components::{BaseMoveSpeed, MoveTarget, OrderSource, PassengerOf, Position, Velocity};

/// World units between adjacent slots of a group move destination.
const MOVE_GROUP_GRID_SPACING: f32 = 5.0;

/// Stable external identity of an entity, as exported in `world_json`.
///
/// An id stays valid while the entity lives. After it is despawned the same `index` may be reused
/// with a higher `generation`, and the old id no longer resolves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntityId {
    /// Transiently unique slot; reused after despawn.
    pub index: u32,
    /// Counter incremented each time the index is reused.
    pub generation: u32,
}

impl EntityId {
    /// Builds an id from its two parts.
    #[must_use]
    pub const fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    /// Reads the id of a live ECS entity.
    #[must_use]
    pub fn of(entity: Entity) -> Self {
        Self {
            index: entity.index_u32(),
            generation: entity.generation().to_bits(),
        }
    }

    /// Rebuilds the ECS entity this id points at, or `None` when `index` is not a valid slot.
    ///
    /// A well-formed id whose entity is gone still rebuilds here; component lookups then miss.
    #[must_use]
    pub fn to_entity(self) -> Option<Entity> {
        let index = EntityIndex::from_raw_u32(self.index)?;
        Some(Entity::from_index_and_generation(
            index,
            EntityGeneration::from_bits(self.generation),
        ))
    }
}

/// What [`Api::order_move_to`] did with the ids it was given.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OrderReport {
    /// Entities that received a [`MoveTarget`].
    pub ordered: usize,
    /// Ids that were unknown, duplicated, immobile, or without a [`Position`].
    pub skipped: usize,
}

/// Destination for one member of a group, on a grid centred on `target`.
///
/// Columns are `ceil(sqrt(count))`, so the extent around the clicked point grows with about `√n`
/// instead of linearly as it would on a single ring.
pub(crate) fn group_slot(target: MoveTarget, index: usize, count: usize) -> MoveTarget {
    if count <= 1 {
        return target;
    }

    #[allow(clippy::cast_precision_loss, clippy::cast_sign_loss)]
    #[allow(clippy::cast_possible_truncation)]
    let cols = ((count as f32).sqrt().ceil() as usize).max(1);
    let rows = count.div_ceil(cols);
    let row = index / cols;
    let col = index % cols;

    #[allow(clippy::cast_precision_loss)]
    let offset_x = (col as f32 - (cols.saturating_sub(1) as f32) / 2.0) * MOVE_GROUP_GRID_SPACING;
    #[allow(clippy::cast_precision_loss)]
    let offset_y = (row as f32 - (rows.saturating_sub(1) as f32) / 2.0) * MOVE_GROUP_GRID_SPACING;

    MoveTarget {
        x: target.x + offset_x,
        y: target.y + offset_y,
    }
}

/// `true` when the entity is something a move order can reach: it exists, it has a place in the
/// world, it has a speed to travel at, and it is not riding in something else.
///
/// A passenger is refused rather than quietly pointed somewhere. Its position belongs to its
/// vehicle, so the target would do nothing while it rides — and then everything the moment it
/// steps off, walking the unit away from the vehicle that just dropped it.
pub(crate) fn can_take_a_move_order(world: &World, entity: Entity) -> bool {
    world.get::<Position>(entity).is_some()
        && world.get::<BaseMoveSpeed>(entity).is_some()
        && world.get::<PassengerOf>(entity).is_none()
}

/// Points entities at `target`, spread over a grid, and records who gave the order.
///
/// An entity already following a stronger order keeps it: see [`OrderSource::may_override`]. That
/// is the whole priority rule, in one place, so group steering and mission steering cannot quietly
/// undo what the player told a single unit to do.
///
/// Returns how many entities took the order.
pub(crate) fn steer(
    world: &mut World,
    entities: &[Entity],
    target: MoveTarget,
    source: OrderSource,
) -> usize {
    let count = entities.len();
    let mut ordered = 0;

    for (slot, entity) in entities.iter().copied().enumerate() {
        if let Some(current) = world.get::<OrderSource>(entity).copied()
            && !source.may_override(current)
        {
            continue;
        }
        if world.get::<Velocity>(entity).is_none() {
            world
                .entity_mut(entity)
                .insert(Velocity { vx: 0.0, vy: 0.0 });
        }
        world
            .entity_mut(entity)
            .insert((group_slot(target, slot, count), source));
        ordered += 1;
    }

    ordered
}

impl Api {
    /// Orders the given entities to move to a world point.
    ///
    /// Ids without a [`Position`] or a [`BaseMoveSpeed`] are skipped: an immobile entity cannot
    /// take a move order. Entities that pass but carry no [`Velocity`] get a zero one so seek and
    /// integration can run. With more than one valid id the destinations are spread over a grid
    /// around `target` so the group does not pile onto a single point.
    ///
    /// Repeated ids in one call count once; the extras are reported as skipped.
    pub fn order_move_to(&mut self, ids: &[EntityId], target: MoveTarget) -> OrderReport {
        let world = self.core_mut().world_mut();

        let mut movable: Vec<Entity> = Vec::with_capacity(ids.len());
        for id in ids {
            let Some(entity) = id.to_entity() else {
                continue;
            };
            if movable.contains(&entity) {
                continue;
            }
            if !can_take_a_move_order(world, entity) {
                continue;
            }
            movable.push(entity);
        }

        let ordered = steer(world, &movable, target, OrderSource::PlayerUnit);

        OrderReport {
            ordered,
            skipped: ids.len() - ordered,
        }
    }

    /// Stops the given entities: zeroes their [`Velocity`] and drops any [`MoveTarget`].
    ///
    /// Unlike [`Api::order_move_to`] this does not require a [`BaseMoveSpeed`]. In this engine a
    /// `Velocity` is movement, so an entity that carries one without a move speed drifts until
    /// something stops it — and this is that something.
    ///
    /// The [`OrderSource`] is dropped along with the target, so a later order from automation
    /// is free to take the entity.
    ///
    /// Ids without a `Velocity`, and repeats within one call, are reported as skipped.
    pub fn order_stop(&mut self, ids: &[EntityId]) -> OrderReport {
        let world = self.core_mut().world_mut();

        let mut stopped: Vec<Entity> = Vec::with_capacity(ids.len());
        for id in ids {
            let Some(entity) = id.to_entity() else {
                continue;
            };
            if stopped.contains(&entity) {
                continue;
            }
            {
                let Some(mut velocity) = world.get_mut::<Velocity>(entity) else {
                    continue;
                };
                velocity.vx = 0.0;
                velocity.vy = 0.0;
            }
            // The source goes with the target: a leftover PlayerUnit claim on an entity that
            // is no longer going anywhere would block every later group or mission order.
            world
                .entity_mut(entity)
                .remove::<(MoveTarget, OrderSource)>();
            stopped.push(entity);
        }

        OrderReport {
            ordered: stopped.len(),
            skipped: ids.len() - stopped.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{BaseMoveSpeed, MoveTarget, Position, Velocity};

    fn spawn_mover(api: &mut Api, x: f32, y: f32) -> EntityId {
        let entity = api
            .core_mut()
            .world_mut()
            .spawn((
                Position { x, y },
                BaseMoveSpeed(10.0),
                Velocity { vx: 0.0, vy: 0.0 },
            ))
            .id();
        EntityId::of(entity)
    }

    #[test]
    fn single_unit_targets_the_exact_point() {
        let mut api = Api::new();
        let id = spawn_mover(&mut api, 0.0, 0.0);

        let report = api.order_move_to(&[id], MoveTarget { x: 7.0, y: -3.0 });

        assert_eq!(report.ordered, 1);
        assert_eq!(report.skipped, 0);
        let target = api
            .core_mut()
            .world()
            .get::<MoveTarget>(id.to_entity().expect("entity"))
            .expect("move target");
        assert_eq!(target.x, 7.0);
        assert_eq!(target.y, -3.0);
    }

    #[test]
    fn group_members_get_distinct_slots_around_the_point() {
        let mut api = Api::new();
        let ids: Vec<EntityId> = (0..4_u8)
            .map(|i| spawn_mover(&mut api, f32::from(i), 0.0))
            .collect();

        let report = api.order_move_to(&ids, MoveTarget { x: 50.0, y: 50.0 });
        assert_eq!(report.ordered, 4);

        let world = api.core_mut().world();
        let targets: Vec<MoveTarget> = ids
            .iter()
            .map(|id| {
                *world
                    .get::<MoveTarget>(id.to_entity().expect("entity"))
                    .expect("move target")
            })
            .collect();

        for (i, a) in targets.iter().enumerate() {
            for b in targets.iter().skip(i + 1) {
                assert!(a != b, "group members share a destination: {a:?}");
            }
            assert!((a.x - 50.0).abs() <= MOVE_GROUP_GRID_SPACING);
            assert!((a.y - 50.0).abs() <= MOVE_GROUP_GRID_SPACING);
        }
    }

    #[test]
    fn immobile_and_unknown_ids_are_skipped() {
        let mut api = Api::new();
        let statue = api
            .core_mut()
            .world_mut()
            .spawn(Position { x: 1.0, y: 1.0 })
            .id();
        let statue_id = EntityId::of(statue);
        let stale = EntityId::new(statue_id.index, statue_id.generation.wrapping_add(7));
        let mover = spawn_mover(&mut api, 0.0, 0.0);

        let report = api.order_move_to(&[statue_id, stale, mover], MoveTarget { x: 5.0, y: 5.0 });

        assert_eq!(report.ordered, 1);
        assert_eq!(report.skipped, 2);
        let world = api.core_mut().world();
        assert!(world.get::<MoveTarget>(statue).is_none());
        assert!(
            world
                .get::<MoveTarget>(mover.to_entity().expect("entity"))
                .is_some()
        );
    }

    #[test]
    fn repeated_ids_count_once() {
        let mut api = Api::new();
        let id = spawn_mover(&mut api, 0.0, 0.0);

        let report = api.order_move_to(&[id, id, id], MoveTarget { x: 2.0, y: 2.0 });

        assert_eq!(report.ordered, 1);
        assert_eq!(report.skipped, 2);
        let target = api
            .core_mut()
            .world()
            .get::<MoveTarget>(id.to_entity().expect("entity"))
            .expect("move target");
        assert_eq!(target.x, 2.0);
        assert_eq!(target.y, 2.0);
    }

    #[test]
    fn movable_entity_without_velocity_gets_a_zero_one() {
        let mut api = Api::new();
        let entity = api
            .core_mut()
            .world_mut()
            .spawn((Position { x: 0.0, y: 0.0 }, BaseMoveSpeed(4.0)))
            .id();

        let report = api.order_move_to(&[EntityId::of(entity)], MoveTarget { x: 9.0, y: 0.0 });

        assert_eq!(report.ordered, 1);
        let velocity = api
            .core_mut()
            .world()
            .get::<Velocity>(entity)
            .expect("velocity inserted");
        assert_eq!(velocity.vx, 0.0);
        assert_eq!(velocity.vy, 0.0);
    }

    #[test]
    fn stop_zeroes_velocity_and_drops_the_target() {
        let mut api = Api::new();
        let id = spawn_mover(&mut api, 0.0, 0.0);
        api.order_move_to(&[id], MoveTarget { x: 30.0, y: 0.0 });
        api.tick(16).expect("tick");

        let report = api.order_stop(&[id]);

        assert_eq!(report.ordered, 1);
        let entity = id.to_entity().expect("entity");
        let world = api.core_mut().world();
        let velocity = world.get::<Velocity>(entity).expect("velocity");
        assert_eq!(velocity.vx, 0.0);
        assert_eq!(velocity.vy, 0.0);
        assert!(world.get::<MoveTarget>(entity).is_none());
    }

    #[test]
    fn stop_catches_a_drifting_entity_without_base_move_speed() {
        let mut api = Api::new();
        // No BaseMoveSpeed: order_move_to cannot reach it, and nothing else would ever stop it.
        let entity = api
            .core_mut()
            .world_mut()
            .spawn((Position { x: 0.0, y: 0.0 }, Velocity { vx: 0.5, vy: 0.0 }))
            .id();
        let id = EntityId::of(entity);

        api.tick(100).expect("tick");
        let drifted = api
            .core_mut()
            .world()
            .get::<Position>(entity)
            .expect("position")
            .x;
        assert!(drifted > 0.0, "entity should have drifted");

        assert_eq!(
            api.order_move_to(&[id], MoveTarget { x: 0.0, y: 0.0 })
                .ordered,
            0
        );
        assert_eq!(api.order_stop(&[id]).ordered, 1);

        api.tick(100).expect("tick");
        let after = api
            .core_mut()
            .world()
            .get::<Position>(entity)
            .expect("position")
            .x;
        assert!(
            (after - drifted).abs() < 1e-6,
            "entity should stay put after stop"
        );
    }

    #[test]
    fn stop_skips_entities_without_velocity() {
        let mut api = Api::new();
        let statue = api
            .core_mut()
            .world_mut()
            .spawn(Position { x: 1.0, y: 1.0 })
            .id();

        let report = api.order_stop(&[EntityId::of(statue)]);

        assert_eq!(report.ordered, 0);
        assert_eq!(report.skipped, 1);
    }

    #[test]
    fn ordered_unit_reaches_its_destination() {
        let mut api = Api::new();
        let id = spawn_mover(&mut api, 0.0, 0.0);
        api.order_move_to(&[id], MoveTarget { x: 12.0, y: 0.0 });

        let entity = id.to_entity().expect("entity");
        for _ in 0..200 {
            api.tick(100).expect("tick");
            if api.core_mut().world().get::<MoveTarget>(entity).is_none() {
                break;
            }
        }

        let position = api
            .core_mut()
            .world()
            .get::<Position>(entity)
            .expect("position");
        assert!((position.x - 12.0).abs() < 1e-4);
        assert!((position.y - 0.0).abs() < 1e-4);
    }
}
