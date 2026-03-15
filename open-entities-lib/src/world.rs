//! World and schedule setup: create ECS world and run startup/update schedules.

use crate::components::{Position, Velocity};
use crate::entity_loader::{EntityDefinitions, LoadError};
use crate::systems::{
    load_entities_from_yaml_system, move_system, print_position_system, setup_system,
    DeltaTime, EntityDefinitionsPath,
};
use bevy_ecs::prelude::{Entity, World};
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

/// Empty world with only the update schedule (move_system). No startup, no initial entities.
/// Use for WASM or custom game loops; call `run_tick(&mut world, &mut schedule, dt)` each frame.
pub fn create_empty_world() -> (World, Schedule) {
    let world = World::new();
    let mut update = Schedule::default();
    update.add_systems(move_system);
    (world, update)
}

/// Empty world with entity definitions loaded from a YAML string (e.g. from assets).
/// Inserts `EntityDefinitions` as a resource so `spawn_entity_by_type_in_world` can be used.
/// Use from WASM when the host fetches `entities.yaml` and passes its content.
pub fn create_world_with_definitions(yaml: &str) -> Result<(World, Schedule), LoadError> {
    let definitions = EntityDefinitions::load_from_str(yaml)?;
    let mut world = World::new();
    world.insert_resource(definitions);
    let mut update = Schedule::default();
    update.add_systems(move_system);
    Ok((world, update))
}

/// Run one simulation tick with the given delta time (seconds).
/// Inserts `DeltaTime(dt)` and runs the schedule. Use with the update schedule only.
pub fn run_tick(world: &mut World, schedule: &mut Schedule, dt: f32) {
    world.insert_resource(DeltaTime(dt));
    schedule.run(world);
}

/// Snapshot of all entities that have at least a Position component.
/// Returns `(entity_id_bits, Position, Option<Velocity>)` per entity for use by WASM/JS.
/// Entities with only Position (e.g. static obstacles) have `None` for velocity.
/// Entity id is from `Entity::to_bits()` so the same entity has a stable id across frames.
pub fn get_entities_position_velocity(world: &mut World) -> Vec<(u64, Position, Option<Velocity>)> {
    let mut query = world.query::<(Entity, &Position, Option<&Velocity>)>();
    query
        .iter(world)
        .map(|(entity, p, v)| (entity.to_bits(), *p, v.copied()))
        .collect()
}
