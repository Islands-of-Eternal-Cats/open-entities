//! # OpenEntities
//!
//! A library for working with entities using the **bevy_ecs** framework (no bevy_app).
//!
//! - **Components**: Data attached to entities (Position, Velocity)
//! - **Systems**: Functions that operate on components
//! - **Entities**: Unique objects in the world
//!
//! # Examples
//!
//! ```rust
//! use open_entities::setup_world;
//!
//! fn main() {
//!     let (mut world, mut schedule) = setup_world();
//!     schedule.run(&mut world); // one tick
//! }
//! ```

pub mod components;
pub mod entity_loader;
pub mod systems;
pub mod world;

pub use bevy_ecs::prelude::{Schedule, World};
pub use components::{Position, Velocity};
pub use entity_loader::{
    load_and_spawn_all_from_path, spawn_entity_by_type, EntityDefinitions, EntityDefinitionsFile,
    EntityTemplate, LoadError,
};
pub use systems::{
    load_entities_from_yaml_system, move_system, print_position_system, DeltaTime,
    EntityDefinitionsPath,
};
pub use world::{
    create_empty_world, get_entities_position_velocity, run_tick, setup_world,
    setup_world_with_yaml,
};

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::prelude::{Commands, Entity, Res, World};
    use bevy_ecs::schedule::Schedule;

    #[test]
    fn test_components_compile() {
        // Test that components can be instantiated
        let _pos = Position { x: 0.0, y: 0.0 };
        let _vel = Velocity { vx: 1.0, vy: 2.0 };
    }

    #[test]
    fn test_spawn_entity_and_query() {
        let mut world = World::new();

        // Spawn an entity with both Position and Velocity
        let entity = world
            .spawn((Position { x: 5.0, y: 5.0 }, Velocity { vx: 2.0, vy: 3.0 }))
            .id();

        // Query for entities with Velocity
        {
            let mut query = world.query::<&Velocity>();
            let velocities: Vec<_> = query.iter(&world).collect();
            assert_eq!(velocities.len(), 1);
        }

        // Query for entities with Position but without Velocity
        {
            let mut query = world.query::<(&Position, Entity)>();
            let positions: Vec<_> = query
                .iter(&world)
                .filter(|(_, entity)| world.get::<Velocity>(*entity).is_none())
                .collect();
            assert_eq!(positions.len(), 0);
        }

        // Query for specific entity by ID
        {
            let pos = world.get::<Position>(entity).unwrap();
            assert_eq!(pos.x, 5.0);
            assert_eq!(pos.y, 5.0);
        }
    }

    #[test]
    fn test_entity_loader_from_str_and_spawn_by_type() {
        let yaml = r#"
entities:
  mover:
    position: { x: 1.0, y: 2.0 }
    velocity: { vx: 0.5, vy: 0.5 }
  static:
    position: { x: 10.0, y: 10.0 }
"#;
        let definitions = EntityDefinitions::load_from_str(yaml).unwrap();
        assert!(definitions.get("mover").is_some());
        assert!(definitions.get("static").is_some());
        assert!(definitions.get("missing").is_none());

        let mut world = World::new();
        world.insert_resource(definitions);

        let mut startup = Schedule::default();
        startup.add_systems(
            |mut commands: Commands, defs: Res<EntityDefinitions>| {
                spawn_entity_by_type(&mut commands, &defs, "mover");
                spawn_entity_by_type(&mut commands, &defs, "static");
            },
        );
        startup.run(&mut world);

        let mut update = Schedule::default();
        update.add_systems(move_system);
        update.run(&mut world);

        let mut query = world.query::<(&Position, Option<&Velocity>)>();
        let entities: Vec<(&Position, Option<&Velocity>)> = query.iter(&world).collect();
        assert_eq!(entities.len(), 2);

        let with_vel: Vec<_> = entities.iter().filter(|(_, vel)| vel.is_some()).collect();
        let without_vel: Vec<_> = entities.iter().filter(|(_, vel)| vel.is_none()).collect();
        assert_eq!(with_vel.len(), 1);
        assert_eq!(without_vel.len(), 1);

        let (pos, _) = with_vel[0];
        assert_eq!(pos.x, 1.5); // 1.0 + 0.5 after one move_system tick
        assert_eq!(pos.y, 2.5);
    }

    #[test]
    fn test_load_entities_from_yaml_system_via_setup_world_with_yaml() {
        // Full pipeline: setup_world_with_yaml -> load_entities_from_yaml_system -> spawn
        use std::io::Write;
        let yaml = r#"
entities:
  mover:
    position: { x: 0.0, y: 0.0 }
    velocity: { vx: 1.0, vy: 2.0 }
  another_mover:
    position: { x: 5.0, y: 5.0 }
    velocity: { vx: -0.5, vy: 0.5 }
  static_obstacle:
    position: { x: 10.0, y: 10.0 }
"#;
        let dir = std::env::temp_dir();
        let path = dir.join("open_entities_test_entities.yaml");
        std::fs::File::create(&path)
            .unwrap()
            .write_all(yaml.as_bytes())
            .unwrap();

        let (mut world, mut update) = setup_world_with_yaml(&path);
        let mut query = world.query::<&Position>();
        let count_before = query.iter(&world).count();
        assert_eq!(count_before, 3, "YAML defines mover, another_mover, static_obstacle");

        update.run(&mut world); // one tick: movers move
        let _ = std::fs::remove_file(&path);
        let positions: Vec<_> = query.iter(&world).collect();
        assert_eq!(positions.len(), 3);
        let pairs: Vec<(f32, f32)> = positions.iter().map(|p| (p.x, p.y)).collect();
        assert!(
            pairs.contains(&(1.0, 2.0)),
            "mover should be at (1.0, 2.0) after one tick, got pairs: {:?}",
            pairs
        );
        assert!(
            pairs.contains(&(4.5, 5.5)),
            "another_mover should be at (4.5, 5.5) after one tick, got pairs: {:?}",
            pairs
        );
        assert!(
            pairs.contains(&(10.0, 10.0)),
            "static_obstacle should stay at (10.0, 10.0), got pairs: {:?}",
            pairs
        );
    }
}
