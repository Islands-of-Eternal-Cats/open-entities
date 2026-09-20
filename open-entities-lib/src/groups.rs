//! Groups: the layer of command between the player and a single unit.
//!
//! A group is an entity carrying [`Group`], and membership lives on the units as [`MemberOf`], so
//! there is exactly one place to read it and one place to change it. Two rules hold at all times:
//! a unit belongs to at most one group, and a group commands only units of its own faction.
//!
//! See `docs/design/group-mission-contract.md` for the behaviour this implements; missions and the
//! replanner are the next slices.

use bevy_ecs::prelude::Entity;

use crate::api::Api;
use crate::components::{
    AssignedTo, Faction, Group, ManualActive, MemberOf, MoveTarget, OrderSource,
};
use crate::orders::{EntityId, OrderReport, can_take_a_move_order, steer};

/// Errors from the group operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupError {
    /// The id does not resolve, or resolves to an entity that is not a group.
    UnknownGroup(EntityId),
    /// The id does not resolve to a live entity.
    UnknownUnit(EntityId),
    /// A unit without a [`Faction`] cannot join anything: the faction rule could not be checked.
    UnitHasNoFaction(EntityId),
    /// A group commands one faction only.
    FactionMismatch { unit: u32, group: u32 },
}

impl std::fmt::Display for GroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownGroup(id) => {
                write!(f, "no group with id {}:{}", id.index, id.generation)
            }
            Self::UnknownUnit(id) => {
                write!(f, "no entity with id {}:{}", id.index, id.generation)
            }
            Self::UnitHasNoFaction(id) => write!(
                f,
                "entity {}:{} has no faction, so it cannot join a group",
                id.index, id.generation
            ),
            Self::FactionMismatch { unit, group } => write!(
                f,
                "unit belongs to faction {unit}, group commands faction {group}"
            ),
        }
    }
}

impl std::error::Error for GroupError {}

impl Api {
    /// Creates an empty group for a faction.
    ///
    /// A group outlives its members: losing every unit leaves it in place, ready to be refilled,
    /// so whatever the host hung off it — a name, a HUD slot — survives too.
    pub fn create_group(&mut self, faction: u32) -> EntityId {
        let entity = self.core_mut().world_mut().spawn(Group { faction }).id();
        EntityId::of(entity)
    }

    /// Puts a unit in a group, taking it out of its previous one.
    ///
    /// The move is one step: after it the unit is in exactly one group, never none and never two.
    ///
    /// # Errors
    ///
    /// [`GroupError::UnknownGroup`] when `group` is not a group, [`GroupError::UnknownUnit`] when
    /// the unit is gone, [`GroupError::UnitHasNoFaction`] when it has no [`Faction`], and
    /// [`GroupError::FactionMismatch`] when the two factions differ.
    pub fn add_to_group(&mut self, group: EntityId, unit: EntityId) -> Result<(), GroupError> {
        let group_entity = self.resolve_group(group)?;
        let group_faction = self.group_faction(group_entity);

        let unit_entity = unit.to_entity().ok_or(GroupError::UnknownUnit(unit))?;
        let world = self.core_mut().world_mut();
        if !world.entities().contains_spawned(unit_entity) {
            return Err(GroupError::UnknownUnit(unit));
        }
        let unit_faction = world
            .get::<Faction>(unit_entity)
            .ok_or(GroupError::UnitHasNoFaction(unit))?
            .0;
        if unit_faction != group_faction {
            return Err(GroupError::FactionMismatch {
                unit: unit_faction,
                group: group_faction,
            });
        }

        world.entity_mut(unit_entity).insert(MemberOf(group_entity));
        Ok(())
    }

    /// Takes a unit out of whatever group it is in. `true` when it was in one.
    pub fn remove_from_group(&mut self, unit: EntityId) -> bool {
        let Some(entity) = unit.to_entity() else {
            return false;
        };
        let world = self.core_mut().world_mut();
        if world.get::<MemberOf>(entity).is_none() {
            return false;
        }
        world.entity_mut(entity).remove::<MemberOf>();
        true
    }

    /// The group this unit belongs to, if any.
    #[must_use]
    pub fn group_of(&self, unit: EntityId) -> Option<EntityId> {
        let entity = unit.to_entity()?;
        let member_of = self.core().world().get::<MemberOf>(entity)?;
        Some(EntityId::of(member_of.0))
    }

    /// Every live member of a group, in no particular order.
    pub fn group_members(&mut self, group: EntityId) -> Vec<EntityId> {
        let Some(group_entity) = group.to_entity() else {
            return Vec::new();
        };
        self.members_of(group_entity)
            .into_iter()
            .map(EntityId::of)
            .collect()
    }

    /// Orders a whole group to a world point and marks it as manually steered.
    ///
    /// Members already following a personal order keep it — that is the priority rule, and it is
    /// why a player can pull one unit out of a group's advance and have it stay pulled out.
    /// Destinations are spread over the same grid a multi-unit order uses.
    ///
    /// The group is left [`ManualActive`], which automation must respect until it is cleared
    /// explicitly through [`Api::clear_group_manual`], and it is taken off any mission it was
    /// working.
    ///
    /// # Errors
    ///
    /// [`GroupError::UnknownGroup`] when `group` is not a group.
    pub fn order_group_move_to(
        &mut self,
        group: EntityId,
        target: MoveTarget,
    ) -> Result<OrderReport, GroupError> {
        let group_entity = self.resolve_group(group)?;
        let members = self.members_of(group_entity);

        let world = self.core_mut().world_mut();
        let movable: Vec<Entity> = members
            .iter()
            .copied()
            .filter(|entity| can_take_a_move_order(world, *entity))
            .collect();

        let ordered = steer(world, &movable, target, OrderSource::GroupSteering);
        // Taking a group by hand takes it off its mission: the mission is freed for whoever else
        // can still work it, instead of waiting on a group that is now going somewhere else.
        world
            .entity_mut(group_entity)
            .remove::<AssignedTo>()
            .insert(ManualActive);

        Ok(OrderReport {
            ordered,
            skipped: members.len() - ordered,
        })
    }

    /// `true` while the group is under manual control.
    #[must_use]
    pub fn is_group_manual(&self, group: EntityId) -> bool {
        group
            .to_entity()
            .is_some_and(|entity| self.core().world().get::<ManualActive>(entity).is_some())
    }

    /// Hands a group back to automation. `true` when it was manual.
    ///
    /// Nothing else clears the flag: not a finished mission, not a new order. An automatic release
    /// would mean the player cannot tell who is steering without watching the units move.
    ///
    /// # Errors
    ///
    /// [`GroupError::UnknownGroup`] when `group` is not a group.
    pub fn clear_group_manual(&mut self, group: EntityId) -> Result<bool, GroupError> {
        let group_entity = self.resolve_group(group)?;
        let world = self.core_mut().world_mut();
        if world.get::<ManualActive>(group_entity).is_none() {
            return Ok(false);
        }
        world.entity_mut(group_entity).remove::<ManualActive>();
        Ok(true)
    }

    /// Resolves an id that must be a group.
    fn resolve_group(&self, group: EntityId) -> Result<Entity, GroupError> {
        let entity = group.to_entity().ok_or(GroupError::UnknownGroup(group))?;
        if self.core().world().get::<Group>(entity).is_none() {
            return Err(GroupError::UnknownGroup(group));
        }
        Ok(entity)
    }

    /// The faction a resolved group commands.
    fn group_faction(&self, group: Entity) -> u32 {
        self.core()
            .world()
            .get::<Group>(group)
            .map_or(0, |group| group.faction)
    }

    /// Members of a resolved group as ECS entities.
    fn members_of(&mut self, group: Entity) -> Vec<Entity> {
        let world = self.core_mut().world_mut();
        let mut query = world.query::<(Entity, &MemberOf)>();
        query
            .iter(world)
            .filter(|(_, member_of)| member_of.0 == group)
            .map(|(entity, _)| entity)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{BaseMoveSpeed, Position, Velocity};

    fn spawn_unit(api: &mut Api, faction: u32, x: f32) -> EntityId {
        let entity = api
            .core_mut()
            .world_mut()
            .spawn((
                Position { x, y: 0.0 },
                BaseMoveSpeed(10.0),
                Velocity { vx: 0.0, vy: 0.0 },
                Faction(faction),
            ))
            .id();
        EntityId::of(entity)
    }

    #[test]
    fn a_new_group_is_empty_and_automatic() {
        let mut api = Api::new();
        let group = api.create_group(1);

        assert!(api.group_members(group).is_empty());
        assert!(!api.is_group_manual(group));
    }

    #[test]
    fn a_unit_belongs_to_one_group_at_a_time() {
        let mut api = Api::new();
        let first = api.create_group(1);
        let second = api.create_group(1);
        let unit = spawn_unit(&mut api, 1, 0.0);

        api.add_to_group(first, unit).expect("join first");
        api.add_to_group(second, unit).expect("move to second");

        assert_eq!(api.group_of(unit), Some(second));
        assert!(api.group_members(first).is_empty());
        assert_eq!(api.group_members(second), vec![unit]);
    }

    #[test]
    fn a_group_commands_its_own_faction_only() {
        let mut api = Api::new();
        let group = api.create_group(1);
        let enemy = spawn_unit(&mut api, 2, 0.0);

        let err = api
            .add_to_group(group, enemy)
            .expect_err("faction mismatch");

        assert_eq!(err, GroupError::FactionMismatch { unit: 2, group: 1 });
        assert_eq!(api.group_of(enemy), None);
    }

    #[test]
    fn a_unit_without_a_faction_cannot_join() {
        let mut api = Api::new();
        let group = api.create_group(1);
        let entity = api
            .core_mut()
            .world_mut()
            .spawn(Position { x: 0.0, y: 0.0 })
            .id();
        let stray = EntityId::of(entity);

        assert_eq!(
            api.add_to_group(group, stray),
            Err(GroupError::UnitHasNoFaction(stray))
        );
    }

    #[test]
    fn a_unit_id_is_not_a_group_id() {
        let mut api = Api::new();
        let unit = spawn_unit(&mut api, 1, 0.0);

        assert_eq!(
            api.order_group_move_to(unit, MoveTarget { x: 1.0, y: 1.0 }),
            Err(GroupError::UnknownGroup(unit))
        );
    }

    #[test]
    fn a_group_order_steers_members_and_turns_manual_on() {
        let mut api = Api::new();
        let group = api.create_group(1);
        let first = spawn_unit(&mut api, 1, 0.0);
        let second = spawn_unit(&mut api, 1, 1.0);
        api.add_to_group(group, first).expect("join");
        api.add_to_group(group, second).expect("join");

        let report = api
            .order_group_move_to(group, MoveTarget { x: 50.0, y: 50.0 })
            .expect("group order");

        assert_eq!(report.ordered, 2);
        assert!(api.is_group_manual(group));

        let world = api.core().world();
        for unit in [first, second] {
            let entity = unit.to_entity().expect("live entity");
            let target = world.get::<MoveTarget>(entity).expect("move target");
            assert!((target.x - 50.0).abs() <= 5.0);
            assert_eq!(
                world.get::<OrderSource>(entity),
                Some(&OrderSource::GroupSteering)
            );
        }
    }

    #[test]
    fn a_personal_order_survives_a_group_order() {
        let mut api = Api::new();
        let group = api.create_group(1);
        let loyal = spawn_unit(&mut api, 1, 0.0);
        let detached = spawn_unit(&mut api, 1, 1.0);
        api.add_to_group(group, loyal).expect("join");
        api.add_to_group(group, detached).expect("join");

        api.order_move_to(&[detached], MoveTarget { x: 5.0, y: 0.0 });
        let report = api
            .order_group_move_to(group, MoveTarget { x: 80.0, y: 80.0 })
            .expect("group order");

        assert_eq!(report.ordered, 1, "only the unit without a personal order");
        assert_eq!(report.skipped, 1);

        let world = api.core().world();
        let detached_entity = detached.to_entity().expect("live entity");
        let target = world.get::<MoveTarget>(detached_entity).expect("target");
        assert_eq!(target.x, 5.0);
        assert_eq!(
            world.get::<OrderSource>(detached_entity),
            Some(&OrderSource::PlayerUnit)
        );
    }

    #[test]
    fn a_group_survives_losing_every_member() {
        let mut api = Api::new();
        let group = api.create_group(1);
        let unit = spawn_unit(&mut api, 1, 0.0);
        api.add_to_group(group, unit).expect("join");

        assert_eq!(api.despawn(&[unit]), 1);

        assert!(api.group_members(group).is_empty());
        assert!(
            api.order_group_move_to(group, MoveTarget { x: 1.0, y: 1.0 })
                .is_ok(),
            "an empty group is still a group"
        );
    }

    #[test]
    fn manual_control_is_released_only_on_request() {
        let mut api = Api::new();
        let group = api.create_group(1);
        let unit = spawn_unit(&mut api, 1, 0.0);
        api.add_to_group(group, unit).expect("join");
        api.order_group_move_to(group, MoveTarget { x: 2.0, y: 0.0 })
            .expect("group order");

        // Arriving does not hand the group back to automation.
        for _ in 0..50 {
            api.tick(100).expect("tick");
        }
        assert!(api.is_group_manual(group));

        assert_eq!(api.clear_group_manual(group), Ok(true));
        assert!(!api.is_group_manual(group));
        assert_eq!(api.clear_group_manual(group), Ok(false));
    }

    #[test]
    fn leaving_a_group_is_reported() {
        let mut api = Api::new();
        let group = api.create_group(1);
        let unit = spawn_unit(&mut api, 1, 0.0);
        api.add_to_group(group, unit).expect("join");

        assert!(api.remove_from_group(unit));
        assert!(!api.remove_from_group(unit));
        assert_eq!(api.group_of(unit), None);
    }
}
