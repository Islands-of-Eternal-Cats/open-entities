use bevy_ecs::prelude::{Component, Entity};

/// On a unit: the vehicle carrying it.
///
/// While this is present the unit's position belongs to the vehicle — the movement systems skip
/// passengers entirely, so nothing else can claim it and the two cannot disagree about where the
/// unit is.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PassengerOf(pub Entity);
