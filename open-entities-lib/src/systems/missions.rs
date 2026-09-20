//! Mission automation: steer the assigned groups, and close a mission when one of them arrives.

use bevy_ecs::prelude::*;

use crate::components::{
    AssignedTo, BaseMoveSpeed, Group, ManualActive, MemberOf, Mission, MissionCompleted,
    MoveTarget, OrderSource, Position, Velocity,
};
use crate::orders::group_slot;

/// Steers the members of every assigned group toward their mission.
///
/// Three kinds of group are left alone: one under [`ManualActive`], because the player is driving
/// it; one whose mission is already [`MissionCompleted`]; and one with no live members, which is
/// unassigned here so it stops holding a mission nobody is walking to.
///
/// Within a group, a member already following a stronger order keeps it. The ladder lives in
/// [`OrderSource`], and mission steering sits at the bottom of it.
pub fn mission_steering_system(
    mut commands: Commands,
    groups: Query<(Entity, &AssignedTo), (With<Group>, Without<ManualActive>)>,
    missions: Query<&Mission, Without<MissionCompleted>>,
    members: Query<(Entity, &MemberOf)>,
    steerable: Query<
        (Option<&OrderSource>, Option<&Velocity>),
        (With<Position>, With<BaseMoveSpeed>),
    >,
) {
    for (group, assigned) in &groups {
        let Ok(mission) = missions.get(assigned.0) else {
            continue;
        };

        let roster: Vec<Entity> = members
            .iter()
            .filter(|(_, member_of)| member_of.0 == group)
            .map(|(entity, _)| entity)
            .collect();

        if roster.is_empty() {
            // An empty group must not keep a mission reserved for itself.
            commands.entity(group).remove::<AssignedTo>();
            continue;
        }

        let targets: Vec<Entity> = roster
            .into_iter()
            .filter(|entity| steerable.contains(*entity))
            .collect();
        let count = targets.len();

        for (slot, entity) in targets.into_iter().enumerate() {
            let Ok((source, velocity)) = steerable.get(entity) else {
                continue;
            };
            if let Some(current) = source
                && !OrderSource::MissionSteering.may_override(*current)
            {
                continue;
            }
            if velocity.is_none() {
                commands
                    .entity(entity)
                    .insert(Velocity { vx: 0.0, vy: 0.0 });
            }
            commands.entity(entity).insert((
                group_slot(mission.target, slot, count),
                OrderSource::MissionSteering,
            ));
        }
    }
}

/// Closes a mission as soon as one live unit of one assigned group is inside its radius.
///
/// That is rule G1: the first arrival finishes the mission for everyone, including the groups
/// still walking. Every assignee is released and the mission's steering claims are dropped, so
/// those units stop where they are. Orders the player gave by hand are left alone — they never
/// belonged to the mission.
pub fn mission_completion_system(
    mut commands: Commands,
    missions: Query<(Entity, &Mission), Without<MissionCompleted>>,
    groups: Query<(Entity, &AssignedTo), With<Group>>,
    members: Query<(Entity, &MemberOf)>,
    positions: Query<&Position>,
    sources: Query<&OrderSource>,
) {
    for (mission_entity, mission) in &missions {
        let assignees: Vec<Entity> = groups
            .iter()
            .filter(|(_, assigned)| assigned.0 == mission_entity)
            .map(|(group, _)| group)
            .collect();
        if assignees.is_empty() {
            continue;
        }

        let roster: Vec<Entity> = members
            .iter()
            .filter(|(_, member_of)| assignees.contains(&member_of.0))
            .map(|(entity, _)| entity)
            .collect();

        let arrived = roster.iter().any(|entity| {
            positions.get(*entity).is_ok_and(|position| {
                let dx = mission.target.x - position.x;
                let dy = mission.target.y - position.y;
                dx.hypot(dy) <= mission.radius
            })
        });
        if !arrived {
            continue;
        }

        commands.entity(mission_entity).insert(MissionCompleted);
        for group in assignees {
            commands.entity(group).remove::<AssignedTo>();
        }
        for entity in roster {
            if let Ok(source) = sources.get(entity)
                && *source == OrderSource::MissionSteering
            {
                commands
                    .entity(entity)
                    .remove::<(MoveTarget, OrderSource)>();
            }
        }
    }
}
