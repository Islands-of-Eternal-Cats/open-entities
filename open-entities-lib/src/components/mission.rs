use bevy_ecs::prelude::Component;

use super::MoveTarget;

/// A place the automation wants reached, and how close counts as reached.
///
/// A mission is an entity, like a group: it can be assigned to several groups at once, and it
/// outlives the units that walk to it.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mission {
    /// Point to reach.
    pub target: MoveTarget,
    /// Arrival radius in milli-units — the `Rg` of the contract.
    pub radius: i32,
}

/// Marks a mission nobody needs to walk to any more.
///
/// A completed mission is kept, not despawned: the host may want to show what was done, and an id
/// that suddenly resolves to nothing is worse than one that resolves to a finished thing.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissionCompleted;
