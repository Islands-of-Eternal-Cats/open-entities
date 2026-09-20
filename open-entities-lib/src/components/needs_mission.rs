use bevy_ecs::prelude::Component;

/// On a group: its mission ended under it, and the planner owes it a new one.
///
/// The marker exists so the planner touches only groups that lost work, never every idle group
/// on the map. Creating a mission should not make every spare squad walk toward it.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct NeedsMission;
