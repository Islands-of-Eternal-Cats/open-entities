use bevy_ecs::prelude::Component;

/// On a group: the player is steering it by hand, so automation must keep its hands off.
///
/// Set by a manual order to the group, cleared only by an explicit call — never by a mission
/// finishing or by a new order. That is the v1.5 rule: if automation could take a group back on
/// its own, the player would lose track of who is driving.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManualActive;
