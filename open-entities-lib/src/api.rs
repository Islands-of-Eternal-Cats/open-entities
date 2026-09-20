use crate::core::Core;
use crate::import::EntityTemplates;
use crate::map::MapBounds;
use crate::orders::EntityId;
use crate::simulation::{ArrivedThisTick, SimDelta, TickError};
use crate::systems::MAX_DT_MS;

/// Public facade over [`Core`] for simulation operations, export, and import.
pub struct Api {
    core: Core,
    pub(crate) templates: Option<EntityTemplates>,
    pub(crate) map_bounds: Option<MapBounds>,
}

impl Api {
    /// Creates an API backed by a new empty [`Core`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            core: Core::new(),
            templates: None,
            map_bounds: None,
        }
    }

    /// Read-only access to the underlying core.
    #[must_use]
    pub const fn core(&self) -> &Core {
        &self.core
    }

    /// Mutable access to the underlying core.
    pub const fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    /// `true` while the entity behind this id is spawned.
    ///
    /// An id stops resolving once its entity is despawned, even after the same index is handed to
    /// a new entity: the generation differs.
    #[must_use]
    pub fn is_alive(&self, id: EntityId) -> bool {
        id.to_entity()
            .is_some_and(|entity| self.core().world().entities().contains_spawned(entity))
    }

    /// Despawns the entities behind these ids, returning how many were removed.
    ///
    /// Unknown, stale and repeated ids are skipped, so the count is of entities actually removed,
    /// never of ids passed in.
    pub fn despawn(&mut self, ids: &[EntityId]) -> usize {
        let world = self.core_mut().world_mut();
        let mut removed = 0;
        for id in ids {
            let Some(entity) = id.to_entity() else {
                continue;
            };
            if world.entities().contains_spawned(entity) {
                let _ = world.despawn(entity);
                removed += 1;
            }
        }
        removed
    }

    /// Advances simulation by `dt_ms` milliseconds (clamped to [`MAX_DT_MS`]).
    ///
    /// # Errors
    ///
    /// Returns [`TickError::ZeroDeltaTime`] when `dt_ms == 0`.
    pub fn tick(&mut self, dt_ms: u32) -> Result<(), TickError> {
        if dt_ms == 0 {
            return Err(TickError::ZeroDeltaTime);
        }
        let dt_ms = dt_ms.min(MAX_DT_MS);
        let core = self.core_mut();
        let world = core.world_mut();
        world.insert_resource(SimDelta::from_ms(dt_ms));
        world.resource_mut::<ArrivedThisTick>().0.clear();
        core.run_schedule();
        Ok(())
    }
}

impl Default for Api {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{EntityType, Position};

    fn spawn_marker(api: &mut Api, x: f32) -> EntityId {
        let entity = api
            .core_mut()
            .world_mut()
            .spawn((Position { x, y: 0.0 }, EntityType("marker".to_owned())))
            .id();
        EntityId::of(entity)
    }

    #[test]
    fn despawn_removes_entities_and_counts_them() {
        let mut api = Api::new();
        let first = spawn_marker(&mut api, 1.0);
        let second = spawn_marker(&mut api, 2.0);

        assert_eq!(api.despawn(&[first]), 1);

        assert!(!api.is_alive(first));
        assert!(api.is_alive(second));
        let json = api.world_json().expect("export");
        let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert_eq!(value["entities"].as_array().map(Vec::len), Some(1));
    }

    #[test]
    fn despawn_counts_entities_not_ids() {
        let mut api = Api::new();
        let id = spawn_marker(&mut api, 1.0);
        let stale = EntityId::new(id.index, id.generation.wrapping_add(3));

        // The same id twice plus one that never resolved: one entity goes away.
        assert_eq!(api.despawn(&[id, id, stale]), 1);
        assert_eq!(api.despawn(&[id]), 0);
    }

    #[test]
    fn a_reused_index_does_not_revive_an_old_id() {
        let mut api = Api::new();
        let old = spawn_marker(&mut api, 1.0);
        api.despawn(&[old]);

        let fresh = spawn_marker(&mut api, 2.0);

        assert!(api.is_alive(fresh));
        assert!(
            !api.is_alive(old),
            "an id must not resolve again once its entity is gone"
        );
    }

    #[test]
    fn is_alive_rejects_an_unspawned_id() {
        let api = Api::new();
        assert!(!api.is_alive(EntityId::new(7, 0)));
    }
}
