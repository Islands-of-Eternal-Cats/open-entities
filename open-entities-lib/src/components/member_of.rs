use bevy_ecs::prelude::{Component, Entity};

/// The group a unit belongs to. A unit carries at most one, which is the invariant.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemberOf(pub Entity);
