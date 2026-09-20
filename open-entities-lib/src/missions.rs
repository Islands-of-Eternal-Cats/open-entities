//! Missions: a place the automation wants reached, and the groups sent to reach it.
//!
//! A mission is an entity carrying [`Mission`]; the groups working it carry [`AssignedTo`], so a
//! mission's roster is read the same way a group's is — by query, from one source of truth.
//!
//! See `docs/design/group-mission-contract.md`. The replanner that picks a group's next mission is
//! the last slice and is not here yet.

use bevy_ecs::prelude::Entity;

use crate::api::Api;
use crate::components::{AssignedTo, Group, Mission, MissionCompleted, MoveTarget};
use crate::orders::EntityId;

/// Errors from the mission operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissionError {
    /// The id does not resolve, or resolves to an entity that is not a mission.
    UnknownMission(EntityId),
    /// The id does not resolve, or resolves to an entity that is not a group.
    UnknownGroup(EntityId),
    /// The mission is done; nobody else is sent to it.
    AlreadyCompleted(EntityId),
}

impl std::fmt::Display for MissionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownMission(id) => {
                write!(f, "no mission with id {}:{}", id.index, id.generation)
            }
            Self::UnknownGroup(id) => {
                write!(f, "no group with id {}:{}", id.index, id.generation)
            }
            Self::AlreadyCompleted(id) => write!(
                f,
                "mission {}:{} is already completed",
                id.index, id.generation
            ),
        }
    }
}

impl std::error::Error for MissionError {}

impl Api {
    /// Creates a mission: a point to reach and how close counts as reached.
    ///
    /// A negative radius is clamped to zero, which means the unit has to land on the point.
    pub fn create_mission(&mut self, target: MoveTarget, radius: f32) -> EntityId {
        let entity = self
            .core_mut()
            .world_mut()
            .spawn(Mission {
                target,
                radius: radius.max(0.0),
            })
            .id();
        EntityId::of(entity)
    }

    /// Sends a group to a mission, taking it off whatever it was working before.
    ///
    /// Several groups can work the same mission; the first arrival finishes it for all of them.
    ///
    /// # Errors
    ///
    /// [`MissionError::UnknownMission`], [`MissionError::UnknownGroup`], or
    /// [`MissionError::AlreadyCompleted`] when the mission is done.
    pub fn assign_group(&mut self, mission: EntityId, group: EntityId) -> Result<(), MissionError> {
        let mission_entity = self.resolve_mission(mission)?;
        if self
            .core()
            .world()
            .get::<MissionCompleted>(mission_entity)
            .is_some()
        {
            return Err(MissionError::AlreadyCompleted(mission));
        }
        let group_entity = self.resolve_group_for_mission(group)?;

        self.core_mut()
            .world_mut()
            .entity_mut(group_entity)
            .insert(AssignedTo(mission_entity));
        Ok(())
    }

    /// Takes a group off its mission. `true` when it was on one.
    pub fn unassign_group(&mut self, group: EntityId) -> bool {
        let Some(entity) = group.to_entity() else {
            return false;
        };
        let world = self.core_mut().world_mut();
        if world.get::<AssignedTo>(entity).is_none() {
            return false;
        }
        world.entity_mut(entity).remove::<AssignedTo>();
        true
    }

    /// The mission this group is working, if any.
    #[must_use]
    pub fn mission_of(&self, group: EntityId) -> Option<EntityId> {
        let entity = group.to_entity()?;
        let assigned = self.core().world().get::<AssignedTo>(entity)?;
        Some(EntityId::of(assigned.0))
    }

    /// Every group currently working this mission.
    pub fn mission_assignees(&mut self, mission: EntityId) -> Vec<EntityId> {
        let Some(mission_entity) = mission.to_entity() else {
            return Vec::new();
        };
        let world = self.core_mut().world_mut();
        let mut query = world.query::<(Entity, &AssignedTo)>();
        query
            .iter(world)
            .filter(|(_, assigned)| assigned.0 == mission_entity)
            .map(|(group, _)| EntityId::of(group))
            .collect()
    }

    /// `true` once somebody reached the mission.
    #[must_use]
    pub fn is_mission_completed(&self, mission: EntityId) -> bool {
        mission.to_entity().is_some_and(|entity| {
            self.core()
                .world()
                .get::<MissionCompleted>(entity)
                .is_some()
        })
    }

    fn resolve_mission(&self, mission: EntityId) -> Result<Entity, MissionError> {
        let entity = mission
            .to_entity()
            .ok_or(MissionError::UnknownMission(mission))?;
        if self.core().world().get::<Mission>(entity).is_none() {
            return Err(MissionError::UnknownMission(mission));
        }
        Ok(entity)
    }

    fn resolve_group_for_mission(&self, group: EntityId) -> Result<Entity, MissionError> {
        let entity = group.to_entity().ok_or(MissionError::UnknownGroup(group))?;
        if self.core().world().get::<Group>(entity).is_none() {
            return Err(MissionError::UnknownGroup(group));
        }
        Ok(entity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{BaseMoveSpeed, Faction, OrderSource, Position, Velocity};

    fn spawn_unit(api: &mut Api, faction: u32, x: f32, y: f32) -> EntityId {
        let entity = api
            .core_mut()
            .world_mut()
            .spawn((
                Position { x, y },
                BaseMoveSpeed(20.0),
                Velocity { vx: 0.0, vy: 0.0 },
                Faction(faction),
            ))
            .id();
        EntityId::of(entity)
    }

    /// A group of one, standing at the origin.
    fn group_with_one_unit(api: &mut Api) -> (EntityId, EntityId) {
        let group = api.create_group(1);
        let unit = spawn_unit(api, 1, 0.0, 0.0);
        api.add_to_group(group, unit).expect("join");
        (group, unit)
    }

    #[test]
    fn an_assigned_group_walks_to_the_mission_and_closes_it() {
        let mut api = Api::new();
        let (group, unit) = group_with_one_unit(&mut api);
        let mission = api.create_mission(MoveTarget { x: 30.0, y: 0.0 }, 1.0);
        api.assign_group(mission, group).expect("assign");

        for _ in 0..100 {
            api.tick(100).expect("tick");
            if api.is_mission_completed(mission) {
                break;
            }
        }

        assert!(api.is_mission_completed(mission));
        assert_eq!(api.mission_of(group), None, "assignees are released");
        assert!(api.mission_assignees(mission).is_empty());

        let entity = unit.to_entity().expect("live entity");
        let world = api.core().world();
        assert!(world.get::<MoveTarget>(entity).is_none());
        assert!(world.get::<OrderSource>(entity).is_none());
    }

    #[test]
    fn the_first_arrival_finishes_the_mission_for_everyone() {
        let mut api = Api::new();
        let near = api.create_group(1);
        let far = api.create_group(1);
        let near_unit = spawn_unit(&mut api, 1, 28.0, 0.0);
        let far_unit = spawn_unit(&mut api, 1, -200.0, 0.0);
        api.add_to_group(near, near_unit).expect("join");
        api.add_to_group(far, far_unit).expect("join");
        let mission = api.create_mission(MoveTarget { x: 30.0, y: 0.0 }, 1.0);
        api.assign_group(mission, near).expect("assign");
        api.assign_group(mission, far).expect("assign");

        for _ in 0..100 {
            api.tick(100).expect("tick");
            if api.is_mission_completed(mission) {
                break;
            }
        }

        assert!(api.is_mission_completed(mission));
        assert_eq!(
            api.mission_of(far),
            None,
            "the group still walking is freed"
        );

        let far_entity = far_unit.to_entity().expect("live entity");
        assert!(
            api.core().world().get::<MoveTarget>(far_entity).is_none(),
            "and it stops where it is"
        );
    }

    #[test]
    fn a_manual_group_is_not_steered_by_its_mission() {
        let mut api = Api::new();
        let (group, unit) = group_with_one_unit(&mut api);
        let mission = api.create_mission(MoveTarget { x: 500.0, y: 0.0 }, 1.0);
        api.assign_group(mission, group).expect("assign");

        api.order_group_move_to(group, MoveTarget { x: -10.0, y: 0.0 })
            .expect("manual order");
        api.tick(16).expect("tick");

        let entity = unit.to_entity().expect("live entity");
        let world = api.core().world();
        let target = world.get::<MoveTarget>(entity).expect("target");
        assert_eq!(target.x, -10.0, "the manual order stands");
        assert_eq!(
            world.get::<OrderSource>(entity),
            Some(&OrderSource::GroupSteering)
        );
    }

    #[test]
    fn a_personal_order_outranks_mission_steering() {
        let mut api = Api::new();
        let (group, unit) = group_with_one_unit(&mut api);
        let mission = api.create_mission(MoveTarget { x: 500.0, y: 0.0 }, 1.0);
        api.assign_group(mission, group).expect("assign");
        api.order_move_to(&[unit], MoveTarget { x: 3.0, y: 0.0 });

        api.tick(16).expect("tick");

        let entity = unit.to_entity().expect("live entity");
        assert_eq!(
            api.core().world().get::<OrderSource>(entity),
            Some(&OrderSource::PlayerUnit)
        );
    }

    #[test]
    fn a_manual_order_takes_the_group_off_its_mission() {
        let mut api = Api::new();
        let (group, _) = group_with_one_unit(&mut api);
        let mission = api.create_mission(MoveTarget { x: 300.0, y: 0.0 }, 1.0);
        api.assign_group(mission, group).expect("assign");

        api.order_group_move_to(group, MoveTarget { x: -5.0, y: 0.0 })
            .expect("manual order");

        assert_eq!(api.mission_of(group), None);
        assert!(
            api.mission_assignees(mission).is_empty(),
            "the mission is free for another group"
        );
    }

    #[test]
    fn an_empty_group_releases_its_mission() {
        let mut api = Api::new();
        let (group, unit) = group_with_one_unit(&mut api);
        let mission = api.create_mission(MoveTarget { x: 300.0, y: 0.0 }, 1.0);
        api.assign_group(mission, group).expect("assign");

        assert_eq!(api.despawn(&[unit]), 1);
        api.tick(16).expect("tick");

        assert_eq!(api.mission_of(group), None);
        assert!(!api.is_mission_completed(mission), "nobody arrived");
    }

    #[test]
    fn a_completed_mission_takes_no_more_groups() {
        let mut api = Api::new();
        let (group, _) = group_with_one_unit(&mut api);
        let mission = api.create_mission(MoveTarget { x: 0.0, y: 0.0 }, 1.0);
        api.assign_group(mission, group).expect("assign");
        api.tick(16).expect("tick"); // the unit is already standing on the target

        assert!(api.is_mission_completed(mission));
        let second = api.create_group(1);
        assert_eq!(
            api.assign_group(mission, second),
            Err(MissionError::AlreadyCompleted(mission))
        );
    }

    #[test]
    fn a_group_works_one_mission_at_a_time() {
        let mut api = Api::new();
        let (group, _) = group_with_one_unit(&mut api);
        let first = api.create_mission(MoveTarget { x: 100.0, y: 0.0 }, 1.0);
        let second = api.create_mission(MoveTarget { x: 200.0, y: 0.0 }, 1.0);

        api.assign_group(first, group).expect("assign first");
        api.assign_group(second, group).expect("assign second");

        assert_eq!(api.mission_of(group), Some(second));
        assert!(api.mission_assignees(first).is_empty());
    }

    #[test]
    fn ids_of_the_wrong_kind_are_refused() {
        let mut api = Api::new();
        let (group, unit) = group_with_one_unit(&mut api);
        let mission = api.create_mission(MoveTarget { x: 1.0, y: 0.0 }, 1.0);

        assert_eq!(
            api.assign_group(group, group),
            Err(MissionError::UnknownMission(group))
        );
        assert_eq!(
            api.assign_group(mission, unit),
            Err(MissionError::UnknownGroup(unit))
        );
    }

    #[test]
    fn unassigning_is_reported() {
        let mut api = Api::new();
        let (group, _) = group_with_one_unit(&mut api);
        let mission = api.create_mission(MoveTarget { x: 100.0, y: 0.0 }, 1.0);
        api.assign_group(mission, group).expect("assign");

        assert!(api.unassign_group(group));
        assert!(!api.unassign_group(group));
        assert_eq!(api.mission_of(group), None);
    }
}
