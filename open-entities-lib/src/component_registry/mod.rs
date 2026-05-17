#[macro_use]
mod macros;

mod registered;

#[allow(unused_imports)] // re-exports are the public registry API
pub use registered::{
    entity_components_from_query, merge_components, spawn_registered_components,
    EntityComponents, WorldExportQuery,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::Health;

    fn entity_components_has_any(doc: &EntityComponents) -> bool {
        doc.position.is_some()
            || doc.velocity.is_some()
            || doc.faction.is_some()
            || doc.move_target.is_some()
            || doc.health.is_some()
    }

    #[test]
    fn entity_components_has_any_detects_health() {
        let doc = EntityComponents {
            health: Some(Health {
                current: 1,
                max: 1,
            }),
            ..Default::default()
        };
        assert!(entity_components_has_any(&doc));
        assert!(!entity_components_has_any(&EntityComponents::default()));
    }
}
