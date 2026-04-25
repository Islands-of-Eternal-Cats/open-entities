//! Minimal binary that runs the ECS once — used to measure native binary size.

use std::path::PathBuf;

use open_entities::setup_world_with_yaml;

fn main() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let yaml = manifest.join("../assets/entities.yaml");
    let (mut world, mut schedule) = setup_world_with_yaml(yaml);
    schedule.run(&mut world);
}
