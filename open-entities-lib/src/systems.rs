use crate::components::{Position, Velocity};
use crate::entity_loader::load_and_spawn_all_from_path;
use bevy_app::{App, Startup, Update};
use bevy_ecs::prelude::*;
use std::path::PathBuf;

/// System: Update position based on velocity
pub fn move_system(mut query: Query<(&mut Position, &Velocity)>) {
    for (mut pos, vel) in &mut query {
        pos.x += vel.vx;
        pos.y += vel.vy;
    }
}

/// System: Print positions of all entities
pub fn print_position_system(query: Query<&Position>) {
    for (i, position) in query.iter().enumerate() {
        println!("Entity {}: position = ({}, {})", i, position.x, position.y);
    }
}

/// Setup system: spawn some entities
fn setup_system(mut commands: Commands) {
    // Spawn first entity with position and velocity
    commands.spawn((Position { x: 0.0, y: 0.0 }, Velocity { vx: 1.0, vy: 2.0 }));

    // Spawn second entity with only position
    commands.spawn(Position { x: 10.0, y: 10.0 });
}

/// Resource: path to YAML file with entity definitions.
/// If set, a startup system will load definitions and spawn one entity per type.
#[derive(Resource, Default)]
pub struct EntityDefinitionsPath(pub Option<PathBuf>);

/// Startup system: load entity definitions from path in resource and spawn one entity per type.
pub fn load_entities_from_yaml_system(
    mut commands: Commands,
    path: Option<Res<EntityDefinitionsPath>>,
) {
    let Some(res) = path else { return };
    let Some(ref p) = res.0 else { return };
    if let Err(e) = load_and_spawn_all_from_path(&mut commands, p) {
        eprintln!("Failed to load entities from YAML: {}", e);
    }
}

/// Initialize the ECS world (hardcoded entities).
pub fn setup_app(app: &mut App) {
    app.add_systems(Startup, setup_system)
        .add_systems(Update, (move_system, print_position_system));
}

/// Initialize the ECS world with entities loaded from a YAML file.
/// Spawns one entity per type defined in the file; does not run the hardcoded setup_system.
pub fn setup_app_with_yaml(app: &mut App, path: impl Into<PathBuf>) {
    app.insert_resource(EntityDefinitionsPath(Some(path.into())))
        .add_systems(Startup, load_entities_from_yaml_system)
        .add_systems(Update, (move_system, print_position_system));
}
