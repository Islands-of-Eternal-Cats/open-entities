use crate::components::{Position, Velocity};
use crate::entity_loader::load_and_spawn_all_from_path;
use bevy_ecs::prelude::*;
use std::path::PathBuf;

pub mod movement;
pub mod seek;
pub mod time;
pub use movement::move_system;
pub use seek::seek_move_target_system;
pub use time::DeltaTime;

/// System: Print positions of all entities
pub fn print_position_system(query: Query<&Position>) {
    for (i, position) in query.iter().enumerate() {
        println!("Entity {}: position = ({}, {})", i, position.x, position.y);
    }
}
/// Setup system: spawn some entities
pub(crate) fn setup_system(mut commands: Commands) {
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
        eprintln!("Failed to load entities from YAML '{}': {}", p.display(), e);
    }
}
