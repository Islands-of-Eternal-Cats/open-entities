//! [`ComponentRegistry`]: every component the YAML loader, the export and the state hash know
//! about, in registration order.
//!
//! [`Api::new`](crate::Api::new) registers the built-ins (the list in `registered.rs`); a game adds
//! its own with [`Api::register_component`](crate::Api::register_component). Both go through
//! [`ComponentRegistry::register`], and the registry order is the order components are spawned,
//! exported and hashed in.

#[macro_use]
mod macros;

mod registered;

use bevy_ecs::prelude::{Component, EntityRef, EntityWorldMut};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::extend::RegisterError;
use crate::state_hash::{StateHash, StateHasher, hash_component};

#[allow(unused_imports)] // re-exports are the public registry API
pub use registered::{EntityComponents, merge_components};
pub(crate) use registered::{merge_owned, register_builtin_components};

/// Checks a YAML value against a component's `Deserialize` impl.
pub(crate) type CheckFn = fn(&yaml_serde::Value) -> Result<(), String>;

/// How one component gets from YAML into the world, and out of it into the export and the hash.
pub(crate) struct ComponentDescriptor {
    /// Key in YAML and in the export.
    pub(crate) field: String,
    /// [`StateHash::TAG`] of the component.
    pub(crate) tag: u8,
    /// Inserts the component when the document carries it. The `&str` is [`Self::field`].
    pub(crate) spawn: fn(&mut EntityWorldMut<'_>, &EntityComponents, &str),
    /// Copies the component, when the entity carries it, into the export row.
    pub(crate) export: fn(&EntityRef<'_>, &mut EntityComponents, &str),
    /// Writes tag and fields when the entity carries the component.
    pub(crate) hash: fn(&EntityRef<'_>, &mut StateHasher),
    /// Checks a value from [`EntityComponents::extra`]; `None` for built-ins, which have a typed
    /// field and never land there.
    pub(crate) check: Option<CheckFn>,
}

/// A component field in a template, an override or a map entry that the registry rejects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComponentError {
    /// No component is registered under this field.
    Unknown {
        /// The key as written.
        field: String,
        /// Every registered field, in registration order.
        known: Vec<String>,
    },
    /// The field is registered, but the value does not deserialize into its component.
    Invalid {
        /// The field.
        field: String,
        /// What the deserializer said.
        message: String,
    },
}

impl std::fmt::Display for ComponentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown { field, known } => write!(
                f,
                "unknown component field `{field}`; known fields: {}",
                known.join(", ")
            ),
            Self::Invalid { field, message } => {
                write!(f, "invalid value for component `{field}`: {message}")
            }
        }
    }
}

impl std::error::Error for ComponentError {}

/// The registered components, in registration order.
#[derive(Default)]
pub(crate) struct ComponentRegistry {
    descriptors: Vec<ComponentDescriptor>,
}

impl ComponentRegistry {
    /// The registry with the built-ins registered.
    pub(crate) fn with_builtins() -> Self {
        let mut registry = Self::default();
        register_builtin_components(&mut registry);
        registry
    }

    /// Appends a descriptor.
    ///
    /// # Errors
    ///
    /// [`RegisterError::DuplicateField`] or [`RegisterError::DuplicateTag`].
    pub(crate) fn register(
        &mut self,
        descriptor: ComponentDescriptor,
    ) -> Result<(), RegisterError> {
        if self.get(&descriptor.field).is_some() {
            return Err(RegisterError::DuplicateField(descriptor.field));
        }
        if let Some(existing) = self.descriptors.iter().find(|d| d.tag == descriptor.tag) {
            return Err(RegisterError::DuplicateTag {
                tag: descriptor.tag,
                field: descriptor.field,
                existing: existing.field.clone(),
            });
        }
        self.descriptors.push(descriptor);
        Ok(())
    }

    fn get(&self, field: &str) -> Option<&ComponentDescriptor> {
        self.descriptors.iter().find(|d| d.field == field)
    }

    /// Registered fields, in registration order.
    pub(crate) fn fields(&self) -> impl Iterator<Item = &str> {
        self.descriptors.iter().map(|d| d.field.as_str())
    }

    /// Checks every key of `doc.extra` against the registry.
    ///
    /// # Errors
    ///
    /// The first key, in key order, that is not registered or does not deserialize.
    pub(crate) fn resolve(&self, doc: &EntityComponents) -> Result<(), ComponentError> {
        for (field, value) in &doc.extra {
            let Some(descriptor) = self.get(field) else {
                return Err(ComponentError::Unknown {
                    field: field.clone(),
                    known: self.fields().map(str::to_owned).collect(),
                });
            };
            let Some(check) = descriptor.check else {
                return Err(ComponentError::Invalid {
                    field: field.clone(),
                    message: "a built-in component goes in its typed field, not in `extra`"
                        .to_owned(),
                });
            };
            check(value).map_err(|message| ComponentError::Invalid {
                field: field.clone(),
                message,
            })?;
        }
        Ok(())
    }

    /// Inserts every component `doc` carries, in registry order. `doc` must have passed
    /// [`Self::resolve`].
    pub(crate) fn spawn(&self, entity: &mut EntityWorldMut<'_>, doc: &EntityComponents) {
        for descriptor in &self.descriptors {
            (descriptor.spawn)(entity, doc, &descriptor.field);
        }
    }

    /// The registered components `entity` carries.
    pub(crate) fn export(&self, entity: &EntityRef<'_>) -> EntityComponents {
        let mut doc = EntityComponents::default();
        for descriptor in &self.descriptors {
            (descriptor.export)(entity, &mut doc, &descriptor.field);
        }
        doc
    }

    /// Feeds the registered components `entity` carries into the hash, in registry order.
    pub(crate) fn hash(&self, entity: &EntityRef<'_>, hasher: &mut StateHasher) {
        for descriptor in &self.descriptors {
            (descriptor.hash)(entity, hasher);
        }
    }
}

/// The descriptor of a game's component: it lives in [`EntityComponents::extra`] as a YAML value.
pub(crate) fn custom_descriptor<T>(field: &str) -> ComponentDescriptor
where
    T: Component + Serialize + DeserializeOwned + StateHash,
{
    ComponentDescriptor {
        field: field.to_owned(),
        tag: T::TAG,
        spawn: |entity, doc, field| {
            if let Some(value) = doc.extra.get(field) {
                let component: T = yaml_serde::from_value(value.clone())
                    .expect("the document was resolved against the registry before spawning");
                entity.insert(component);
            }
        },
        export: |entity, doc, field| {
            if let Some(component) = entity.get::<T>() {
                let value = yaml_serde::to_value(component)
                    .expect("registration checked that the component serializes");
                doc.extra.insert(field.to_owned(), value);
            }
        },
        hash: hash_component::<T>,
        check: Some(|value| {
            yaml_serde::from_value::<T>(value.clone())
                .map(drop)
                .map_err(|err| err.to_string())
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Health, Position};
    use bevy_ecs::prelude::World;

    #[test]
    fn builtins_register_in_list_order() {
        let registry = ComponentRegistry::with_builtins();
        assert_eq!(
            registry.fields().collect::<Vec<_>>(),
            [
                "position",
                "velocity",
                "faction",
                "move_target",
                "base_move_speed",
                "health",
                "boardable"
            ]
        );
    }

    #[test]
    fn export_reads_registered_components() {
        let registry = ComponentRegistry::with_builtins();
        let mut world = World::new();
        let entity = world
            .spawn((Position { x: 3000, y: 4000 }, Health { current: 7, max: 9 }))
            .id();

        let doc = registry.export(&world.entity(entity));
        assert_eq!(doc.position, Some(Position { x: 3000, y: 4000 }));
        assert_eq!(doc.health, Some(Health { current: 7, max: 9 }));
        assert_eq!(doc.velocity, None);
    }

    #[test]
    fn spawn_inserts_what_the_document_carries() {
        let registry = ComponentRegistry::with_builtins();
        let mut world = World::new();
        let doc = EntityComponents {
            health: Some(Health { current: 1, max: 2 }),
            ..Default::default()
        };
        let mut entity = world.spawn_empty();
        registry.spawn(&mut entity, &doc);
        let entity = entity.id();
        assert_eq!(
            world.get::<Health>(entity),
            Some(&Health { current: 1, max: 2 })
        );
        assert!(world.get::<Position>(entity).is_none());
    }

    #[test]
    fn a_builtin_field_in_extra_is_refused() {
        let registry = ComponentRegistry::with_builtins();
        let mut doc = EntityComponents::default();
        doc.extra
            .insert("position".to_owned(), yaml_serde::Value::from(1_u32));
        assert!(matches!(
            registry.resolve(&doc),
            Err(ComponentError::Invalid { field, .. }) if field == "position"
        ));
    }
}
