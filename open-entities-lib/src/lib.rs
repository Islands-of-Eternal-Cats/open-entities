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
pub use components::{MoveTarget, Position, Velocity};
pub use entity_loader::{
    EntityDefinitions, EntityDefinitionsFile, EntityTemplate, LoadError, SpawnError,
    load_and_spawn_all_from_path, spawn_entity_by_type, spawn_entity_by_type_at_in_world,
    spawn_entity_by_type_in_world,
};
pub use systems::{
    DeltaTime, EntityDefinitionsPath, load_entities_from_yaml_system, move_system,
    print_position_system,
};
pub use world::{
    create_empty_world, create_world_with_definitions, get_entities, order_move_entities_to,
    run_tick, setup_world, setup_world_with_yaml,
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
        startup.add_systems(|mut commands: Commands, defs: Res<EntityDefinitions>| {
            let _ = spawn_entity_by_type(&mut commands, &defs, "mover");
            let _ = spawn_entity_by_type(&mut commands, &defs, "static");
        });
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
        assert_eq!(
            count_before, 3,
            "YAML defines mover, another_mover, static_obstacle"
        );

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

    #[test]
    fn test_invalid_yaml_returns_structured_parse_error() {
        let broken = "entities:\n  mover: [";
        let err = EntityDefinitions::load_from_str(broken).unwrap_err();
        match err {
            LoadError::Yaml { op, source } => {
                assert_eq!(op, "load_from_str");
                assert!(!source.is_empty());
            }
            other => panic!("expected YAML error, got: {:?}", other),
        }
    }

    #[test]
    fn test_missing_entities_root_returns_parse_error() {
        let yaml = r#"
not_entities:
  mover:
    position: { x: 1.0, y: 2.0 }
"#;
        let err = EntityDefinitions::load_from_str(yaml).unwrap_err();
        assert!(matches!(err, LoadError::Yaml { .. }));
    }

    #[test]
    fn test_empty_entities_map_is_valid_and_empty() {
        let defs = EntityDefinitions::load_from_str("entities: {}").unwrap();
        assert_eq!(defs.type_names().count(), 0);
    }

    #[test]
    fn test_spawn_unknown_type_returns_explicit_error() {
        let defs = EntityDefinitions::load_from_str("entities: {}").unwrap();
        let mut world = World::new();
        world.insert_resource(defs);

        let mut startup = Schedule::default();
        startup.add_systems(|mut commands: Commands, defs: Res<EntityDefinitions>| {
            let err = spawn_entity_by_type(&mut commands, &defs, "missing").unwrap_err();
            assert_eq!(
                err,
                SpawnError::UnknownEntityType {
                    type_name: "missing".to_string()
                }
            );
        });
        startup.run(&mut world);
    }

    #[test]
    fn test_spawn_in_world_reports_missing_defs_and_unknown_type() {
        let mut world = World::new();
        let missing_defs_err = spawn_entity_by_type_in_world(&mut world, "mover").unwrap_err();
        assert_eq!(missing_defs_err, SpawnError::DefinitionsNotLoaded);

        let defs = EntityDefinitions::load_from_str("entities: {}").unwrap();
        world.insert_resource(defs);
        let unknown_type_err = spawn_entity_by_type_in_world(&mut world, "ghost").unwrap_err();
        assert_eq!(
            unknown_type_err,
            SpawnError::UnknownEntityType {
                type_name: "ghost".to_string()
            }
        );
    }

    #[test]
    fn test_position_only_entity_does_not_move_after_tick() {
        let yaml = r#"
entities:
  static_only:
    position: { x: 7.0, y: 9.0 }
"#;
        let (mut world, mut schedule) = create_world_with_definitions(yaml).unwrap();
        let spawned = spawn_entity_by_type_in_world(&mut world, "static_only").unwrap();

        run_tick(&mut world, &mut schedule, 0.016);
        let pos = world
            .get::<Position>(spawned)
            .expect("spawned static_only should have Position");
        assert_eq!(pos.x, 7.0);
        assert_eq!(pos.y, 9.0);
    }

    #[test]
    fn test_order_move_entities_to_adds_velocity_and_seeks_target() {
        let yaml = r#"
entities:
  static_only:
    position: { x: 0.0, y: 0.0 }
"#;
        let (mut world, mut schedule) = create_world_with_definitions(yaml).unwrap();
        let spawned = spawn_entity_by_type_in_world(&mut world, "static_only").unwrap();
        let bits = spawned.to_bits();
        order_move_entities_to(&mut world, &[bits], Position { x: 100.0, y: 0.0 });
        assert!(world.get::<Velocity>(spawned).is_some());
        assert!(world.get::<MoveTarget>(spawned).is_some());
        run_tick(&mut world, &mut schedule, 0.05);
        let pos = world.get::<Position>(spawned).unwrap();
        assert!(pos.x > 0.0, "expected movement toward x=100, got {:?}", pos);
    }

    #[test]
    fn test_spawn_at_overrides_position_and_does_not_create_velocity() {
        let yaml = r#"
entities:
  mover:
    position: { x: 1.0, y: 2.0 }
    velocity: { vx: 0.5, vy: 0.5 }
"#;
        let (mut world, mut schedule) = create_world_with_definitions(yaml).unwrap();
        let spawned = spawn_entity_by_type_at_in_world(&mut world, "mover", 123.0, 456.0).unwrap();

        let pos = world
            .get::<Position>(spawned)
            .expect("spawned mover should have Position");
        assert_eq!(pos.x, 123.0);
        assert_eq!(pos.y, 456.0);
        assert!(
            world.get::<Velocity>(spawned).is_none(),
            "spawn_at must not create Velocity"
        );

        run_tick(&mut world, &mut schedule, 0.016);
        let pos_after = world
            .get::<Position>(spawned)
            .expect("spawned mover should still have Position after tick");
        assert_eq!(pos_after.x, 123.0);
        assert_eq!(pos_after.y, 456.0);
    }
}
