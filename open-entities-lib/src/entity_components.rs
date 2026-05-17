pub use crate::component_registry::{
    entity_components_has_any, merge_components, EntityComponents,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Faction, Velocity};

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
