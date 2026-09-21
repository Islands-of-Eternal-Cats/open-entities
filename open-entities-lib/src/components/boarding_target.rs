use bevy_ecs::prelude::{Component, Entity};

/// On a unit: the vehicle it has been told to get into.
///
/// This is an order in flight, not a fact about where the unit is. Each tick the boarding system
/// walks the unit toward the vehicle and, once within [`BOARDING_RANGE`](crate::BOARDING_RANGE),
/// puts it aboard; the component is removed when it boards, when it cannot (no seat), or when
/// the vehicle stops existing. A stop order removes it too.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BoardingTarget(pub Entity);
