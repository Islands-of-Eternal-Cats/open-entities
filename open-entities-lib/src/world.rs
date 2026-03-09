//! World and schedule setup: create ECS world and run startup/update schedules.

use crate::systems::{
    load_entities_from_yaml_system, move_system, print_position_system, setup_system,
    EntityDefinitionsPath,
};
use bevy_ecs::prelude::World;
use bevy_ecs::schedule::Schedule;
use std::path::PathBuf;

/// Initialize the ECS world (hardcoded entities).
/// Returns `(World, Schedule)` — run `schedule.run(&mut world)` each tick.
pub fn setup_world() -> (World, Schedule) {
    let mut world = World::new();
    let mut startup = Schedule::default();
    startup.add_systems(setup_system);
    startup.run(&mut world);

    let mut update = Schedule::default();
    update.add_systems((move_system, print_position_system));
    (world, update)
}

/// Initialize the ECS world with entities loaded from a YAML file.
/// Spawns one entity per type defined in the file; does not run the hardcoded setup_system.
/// Returns `(World, Schedule)` — run `schedule.run(&mut world)` each tick.
pub fn setup_world_with_yaml(path: impl Into<PathBuf>) -> (World, Schedule) {
    let mut world = World::new();
    world.insert_resource(EntityDefinitionsPath(Some(path.into())));

    let mut startup = Schedule::default();
    startup.add_systems(load_entities_from_yaml_system);
    startup.run(&mut world);

    let mut update = Schedule::default();
    update.add_systems((move_system, print_position_system));
    (world, update)
}
