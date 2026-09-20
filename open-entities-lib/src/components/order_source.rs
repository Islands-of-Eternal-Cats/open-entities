use bevy_ecs::prelude::Component;

/// Who set the [`MoveTarget`](super::MoveTarget) an entity is currently following.
///
/// The ranking is the point: a personal order outranks group steering, which outranks a mission.
/// Without it, automation would silently overwrite what the player just told one unit to do.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OrderSource {
    /// Automatic steering along a mission — the weakest claim.
    MissionSteering,
    /// A manual order given to the whole group.
    GroupSteering,
    /// A manual order given to this unit alone — never overwritten while it stands.
    PlayerUnit,
}

impl OrderSource {
    /// `true` when an order from `self` may replace one already set by `current`.
    #[must_use]
    pub fn may_override(self, current: Self) -> bool {
        self >= current
    }
}

#[cfg(test)]
mod tests {
    use super::OrderSource;

    #[test]
    fn a_personal_order_outranks_everything() {
        assert!(OrderSource::PlayerUnit.may_override(OrderSource::GroupSteering));
        assert!(OrderSource::PlayerUnit.may_override(OrderSource::MissionSteering));
        assert!(OrderSource::PlayerUnit.may_override(OrderSource::PlayerUnit));
    }

    #[test]
    fn automation_does_not_touch_a_personal_order() {
        assert!(!OrderSource::GroupSteering.may_override(OrderSource::PlayerUnit));
        assert!(!OrderSource::MissionSteering.may_override(OrderSource::PlayerUnit));
        assert!(!OrderSource::MissionSteering.may_override(OrderSource::GroupSteering));
    }

    #[test]
    fn a_group_order_replaces_an_older_group_order() {
        assert!(OrderSource::GroupSteering.may_override(OrderSource::GroupSteering));
    }
}
