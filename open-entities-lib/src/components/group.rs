use bevy_ecs::prelude::Component;

/// Marks an entity as a group (squad) and fixes the faction it commands for.
///
/// A group is an entity of its own, not a list held elsewhere: it survives losing every member,
/// so a name, a mission history or a HUD slot can hang off it. Membership itself lives on the
/// units, in [`MemberOf`](super::MemberOf) — one place to read, one place to change.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Group {
    pub faction: u32,
}
