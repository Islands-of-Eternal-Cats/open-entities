#[macro_use]
mod macros;

mod registered;

#[allow(unused_imports)] // re-exports are the public registry API
pub use registered::{
    collect_world_export_rows, entity_components_from_query, merge_components,
    spawn_registered_components, EntityComponents, WorldExportQuery, WorldExportRow,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Health, Position};
    use bevy_ecs::prelude::World;

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

    #[test]
    fn collect_world_export_rows_reads_registered_components() {
        let mut world = World::new();
        world.spawn((
            Position { x: 3.0, y: 4.0 },
            Health {
                current: 7,
                max: 9,
            },
        ));

        let rows = collect_world_export_rows(&mut world);
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].components.position,
            Some(Position { x: 3.0, y: 4.0 })
        );
        assert_eq!(
            rows[0].components.health,
            Some(Health {
                current: 7,
                max: 9,
            })
        );
        assert!(rows[0].entity_type.is_none());
    }
}
