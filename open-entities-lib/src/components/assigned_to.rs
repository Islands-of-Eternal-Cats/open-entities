use bevy_ecs::prelude::{Component, Entity};

/// On a group: the mission it is currently working.
///
/// One mission per group at a time; a mission may hold many groups, and its roster is read by
/// query, the same way a group's roster is.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssignedTo(pub Entity);
