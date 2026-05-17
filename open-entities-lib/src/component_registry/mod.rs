#[macro_use]
mod macros;

mod registered;

#[allow(unused_imports)] // re-exports are the public registry API
pub use registered::{
    entity_components_from_query, entity_components_has_any, merge_components,
    registered_components_present, spawn_registered_components, EntityComponents,
    WorldExportQuery,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::Health;

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
