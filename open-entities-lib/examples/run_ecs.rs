//! Minimal binary that runs the ECS once — used to measure native binary size.

use open_entities::setup_world;

fn main() {
    let (mut world, mut schedule) = setup_world();
    schedule.run(&mut world);
}
