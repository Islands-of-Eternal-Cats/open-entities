/// Standalone `register_component!` is forbidden — only valid inside `define_registered_components!`.
#[macro_export]
macro_rules! register_component {
    ($field:ident, $ty:ty) => {
        compile_error!(
            "register_component! must only appear inside define_registered_components! { ... }"
        );
    };
}

/// Expands the built-in registry list into `EntityComponents`, its merge, and
/// `register_builtin_components`, which registers each built-in with the `ComponentRegistry`
/// through the same `register` path a game's components take. Built-ins get a typed field of their own; every other registered component lives in
/// `EntityComponents::extra`.
#[macro_export]
macro_rules! define_registered_components {
    (
        $(
            register_component!($field:ident, $ty:ty);
        )*
    ) => {
        /// Gameplay components shared by YAML templates, spawn overrides, and export (flattened).
        ///
        /// Built-in components have a typed field each. Components a game registered with
        /// [`Api::register_component`](crate::Api::register_component) sit in [`extra`](Self::extra)
        /// under their field name, as YAML values; they are checked against the registry when a
        /// template, an override or a map entry is loaded, and an unknown key is an error.
        #[derive(Clone, Default, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
        pub struct EntityComponents {
            $(
                #[allow(missing_docs)]
                #[serde(skip_serializing_if = "Option::is_none")]
                pub $field: Option<$ty>,
            )*
            /// Registered components without a typed field, by field name. Serializes flat, next
            /// to the built-ins.
            #[serde(flatten)]
            pub extra: std::collections::BTreeMap<String, yaml_serde::Value>,
        }

        /// Component-level merge: `child` wins when `Some`, and for each `extra` key it has.
        pub fn merge_components(
            parent: &EntityComponents,
            child: &EntityComponents,
        ) -> EntityComponents {
            merge_owned(parent.clone(), child.clone())
        }

        /// [`merge_components`] for owned documents: no clone of `child`'s extra values.
        pub(crate) fn merge_owned(
            parent: EntityComponents,
            child: EntityComponents,
        ) -> EntityComponents {
            let mut extra = parent.extra;
            extra.extend(child.extra);
            EntityComponents {
                $(
                    $field: child.$field.or(parent.$field),
                )*
                extra,
            }
        }

        /// Registers the built-ins, in list order: that order is the state hash's.
        pub(crate) fn register_builtin_components(
            registry: &mut $crate::component_registry::ComponentRegistry,
        ) {
            $(
                registry
                    .register($crate::component_registry::ComponentDescriptor {
                        field: stringify!($field).to_owned(),
                        tag: <$ty as $crate::state_hash::StateHash>::TAG,
                        spawn: |entity, doc, _| {
                            if let Some(value) = doc.$field {
                                entity.insert(value);
                            }
                        },
                        export: |entity, doc, _| {
                            doc.$field = entity.get::<$ty>().copied();
                        },
                        hash: $crate::state_hash::hash_component::<$ty>,
                        // A built-in never reaches `extra`: serde fills its typed field.
                        check: None,
                    })
                    .expect("built-in components have distinct fields and tags");
            )*
        }
    };
}
