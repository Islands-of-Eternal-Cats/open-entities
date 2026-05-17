/// Standalone `register_component!` is forbidden — only valid inside `define_registered_components!`.
#[macro_export]
macro_rules! register_component {
    ($field:ident, $ty:ty) => {
        compile_error!(
            "register_component! must only appear inside define_registered_components! { ... }"
        );
    };
}

/// Expands the registry list into `EntityComponents`, merge/spawn/export helpers.
#[macro_export]
macro_rules! define_registered_components {
    (
        $(
            register_component!($field:ident, $ty:ty);
        )*
    ) => {
        /// Gameplay components shared by YAML templates, spawn overrides, and export (flattened).
        #[derive(Clone, Copy, Default, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        pub struct EntityComponents {
            $(
                #[serde(skip_serializing_if = "Option::is_none")]
                pub $field: Option<$ty>,
            )*
        }

        /// Component-level merge: `child` wins when `Some`.
        pub fn merge_components(
            parent: &EntityComponents,
            child: &EntityComponents,
        ) -> EntityComponents {
            EntityComponents {
                $(
                    $field: child.$field.or(parent.$field),
                )*
            }
        }

        /// Inserts each `Some` registered component on the entity under construction.
        pub fn spawn_registered_components(
            entity: &mut bevy_ecs::prelude::EntityWorldMut<'_>,
            doc: &EntityComponents,
        ) {
            $(
                if let Some(value) = doc.$field {
                    entity.insert(value);
                }
            )*
        }

        /// True if any registered field is `Some`.
        #[allow(dead_code)] // public helper for tests and downstream callers
        pub const fn entity_components_has_any(doc: &EntityComponents) -> bool {
            false $(|| doc.$field.is_some())*
        }

        /// Bevy `Query` tuple for export: registered `Option<&T>` plus `EntityType`.
        pub type WorldExportQuery<'w> = (
            bevy_ecs::prelude::Entity,
            $(
                Option<&'w $ty>,
            )*
            Option<&'w $crate::components::EntityType>,
        );

        /// True if the entity has at least one registered gameplay component.
        pub const fn registered_components_present(
            $($field: Option<&$ty>,)*
        ) -> bool {
            false $(|| $field.is_some())*
        }

        /// Builds export row `EntityComponents` from query `Option` references.
        pub const fn entity_components_from_query(
            $($field: Option<&$ty>,)*
        ) -> EntityComponents {
            EntityComponents {
                $($field: $field.copied(),)*
            }
        }
    };
}
