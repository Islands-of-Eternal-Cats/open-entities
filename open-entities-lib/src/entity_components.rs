use serde::{Deserialize, Serialize};

use crate::components::{Faction, MoveTarget, Position, Velocity};

/// Gameplay components shared by YAML templates, spawn overrides, and export (flattened).
#[derive(Clone, Default, PartialEq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntityComponents {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<Position>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub velocity: Option<Velocity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faction: Option<Faction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub move_target: Option<MoveTarget>,
}

/// Component-level merge: `child` wins when `Some`.
pub fn merge_components(parent: &EntityComponents, child: &EntityComponents) -> EntityComponents {
    EntityComponents {
        position: child.position.or(parent.position),
        velocity: child.velocity.or(parent.velocity),
        faction: child.faction.or(parent.faction),
        move_target: child.move_target.or(parent.move_target),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_child_wins_over_parent() {
        let parent = EntityComponents {
            faction: Some(Faction(1)),
            velocity: Some(Velocity { vx: 1.0, vy: 0.0 }),
            ..Default::default()
        };
        let child = EntityComponents {
            faction: Some(Faction(2)),
            ..Default::default()
        };
        let merged = merge_components(&parent, &child);
        assert_eq!(merged.faction, Some(Faction(2)));
        assert_eq!(merged.velocity, Some(Velocity { vx: 1.0, vy: 0.0 }));
    }

    #[test]
    fn merge_fills_missing_from_parent() {
        let parent = EntityComponents {
            faction: Some(Faction(1)),
            ..Default::default()
        };
        let child = EntityComponents {
            velocity: Some(Velocity { vx: 2.0, vy: 0.0 }),
            ..Default::default()
        };
        let merged = merge_components(&parent, &child);
        assert_eq!(merged.faction, Some(Faction(1)));
        assert_eq!(merged.velocity, Some(Velocity { vx: 2.0, vy: 0.0 }));
    }
}
