use bevy_ecs::prelude::{Schedule, World};
use bevy_ecs::schedule::{IntoScheduleConfigs, ScheduleLabel};

use crate::simulation::ArrivedThisTick;
use crate::systems::{movement_system, seek_system};

#[derive(ScheduleLabel, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SimulationSchedule;

/// Owns the ECS [`World`] and gameplay [`Schedule`] for a simulation instance.
pub struct Core {
    world: World,
    schedule: Schedule,
}

impl Core {
    /// Creates an empty world and registers seek → movement systems.
    #[must_use]
    pub fn new() -> Self {
        let mut world = World::new();
        world.insert_resource(ArrivedThisTick::default());

        let mut schedule = Schedule::new(SimulationSchedule);
        schedule.add_systems((seek_system, movement_system).chain());

        Self { world, schedule }
    }

    /// Immutable access to the underlying ECS world.
    #[must_use]
    pub const fn world(&self) -> &World {
        &self.world
    }

    /// Mutable access to the underlying ECS world.
    pub const fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Immutable access to the simulation schedule.
    #[must_use]
    pub const fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    /// Runs the simulation schedule on the world.
    pub fn run_schedule(&mut self) {
        self.schedule.run(&mut self.world);
    }
}

impl Default for Core {
    fn default() -> Self {
        Self::new()
    }
}
