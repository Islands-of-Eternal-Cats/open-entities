//! World and schedule setup: create ECS world and run startup/update schedules.

use crate::components::{MoveTarget, Position, Velocity};
use crate::entity_loader::{EntityDefinitions, LoadError};
use crate::systems::{
    DeltaTime, EntityDefinitionsPath, load_entities_from_yaml_system, move_system,
    print_position_system, seek_move_target_system,
};
use bevy_ecs::prelude::{Entity, IntoScheduleConfigs, World};
use bevy_ecs::schedule::Schedule;
use std::path::PathBuf;

/// Initialize the ECS world with no entities.
/// Spawn units via [`setup_world_with_yaml`] or [`create_world_with_definitions`] so every unit type
/// is defined in YAML.
/// Returns `(World, Schedule)` — run `schedule.run(&mut world)` each tick.
pub fn setup_world() -> (World, Schedule) {
    let world = World::new();
    let mut update = Schedule::default();
    update.add_systems((seek_move_target_system, move_system, print_position_system).chain());
    (world, update)
}

/// Initialize the ECS world with entities loaded from a YAML file.
/// Spawns one entity per type defined in the file.
/// Returns `(World, Schedule)` — run `schedule.run(&mut world)` each tick.
pub fn setup_world_with_yaml(path: impl Into<PathBuf>) -> (World, Schedule) {
    let mut world = World::new();
    world.insert_resource(EntityDefinitionsPath(Some(path.into())));

    let mut startup = Schedule::default();
    startup.add_systems(load_entities_from_yaml_system);
    startup.run(&mut world);

    let mut update = Schedule::default();
    update.add_systems((seek_move_target_system, move_system, print_position_system).chain());
    (world, update)
}

/// Empty world with only the update schedule (move_system). No startup, no initial entities.
/// Use for WASM or custom game loops; call `run_tick(&mut world, &mut schedule, dt)` each frame.
pub fn create_empty_world() -> (World, Schedule) {
    let world = World::new();
    let mut update = Schedule::default();
    update.add_systems((seek_move_target_system, move_system).chain());
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
    update.add_systems((seek_move_target_system, move_system).chain());
    Ok((world, update))
}

/// Cell spacing for the move-destination grid (world units between adjacent slots).
const MOVE_GROUP_GRID_SPACING: f32 = 5.0;

/// Compute a per-unit destination around `center` so the group does not stack on one point.
/// Uses a rectangle grid (~`ceil(sqrt(n))` columns) centered on `target`, so extent from the
/// click grows about **√n** instead of linearly with `n` (as with a single ring).
fn move_target_for_group_index(center: Position, index: usize, count: usize) -> Position {
    if count <= 1 {
        return center;
    }
    let cols = (count as f32).sqrt().ceil() as usize;
    let cols = cols.max(1);
    let rows = count.div_ceil(cols);
    let row = index / cols;
    let col = index % cols;
    let ox = (col as f32 - (cols.saturating_sub(1) as f32) / 2.0) * MOVE_GROUP_GRID_SPACING;
    let oy = (row as f32 - (rows.saturating_sub(1) as f32) / 2.0) * MOVE_GROUP_GRID_SPACING;
    Position {
        x: center.x + ox,
        y: center.y + oy,
    }
}

/// Issue a move-to-world-point order for entities identified by `Entity::to_bits()` (as in snapshots).
/// Skips unknown ids, invalid bits, or entities without [`Position`].
/// Entities without [`Velocity`] receive zero velocity so seek + integration can run.
///
/// For more than one valid id, destinations are placed on a **grid** around `target` so units do not
/// share the same [`MoveTarget`] point.
pub fn order_move_entities_to(world: &mut World, id_bits: &[u64], target: Position) {
    let mut entities = Vec::new();
    for bits in id_bits {
        let Some(entity) = Entity::try_from_bits(*bits) else {
            continue;
        };
        if world.get::<Position>(entity).is_none() {
            continue;
        }
        entities.push(entity);
    }
    let count = entities.len();
    for (i, entity) in entities.into_iter().enumerate() {
        if world.get::<Velocity>(entity).is_none() {
            world
                .entity_mut(entity)
                .insert(Velocity { vx: 0.0, vy: 0.0 });
        }
        let at = move_target_for_group_index(target, i, count);
        world.entity_mut(entity).insert(MoveTarget { at });
    }
}

/// Run one simulation tick with the given delta time (seconds).
/// Inserts `DeltaTime(dt)` and runs the schedule. Use with the update schedule only.
pub fn run_tick(world: &mut World, schedule: &mut Schedule, dt: f32) {
    world.insert_resource(DeltaTime(dt));
    schedule.run(world);
}

/// Returns all entities of the world that have at least a Position component.
/// Each item is `(entity_id_bits, Position, Option<Velocity>)` for use by WASM/JS.
/// Entities with only Position (e.g. static obstacles) have `None` for velocity.
/// Entity id is from `Entity::to_bits()` so the same entity has a stable id across frames.
pub fn get_entities(world: &mut World) -> Vec<(u64, Position, Option<Velocity>)> {
    let mut query = world.query::<(Entity, &Position, Option<&Velocity>)>();
    query
        .iter(world)
        .map(|(entity, p, v)| (entity.to_bits(), *p, v.copied()))
        .collect()
}
