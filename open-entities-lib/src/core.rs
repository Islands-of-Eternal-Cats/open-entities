//! [`Core`]: the ECS world and the fixed system schedule behind [`Api`](crate::Api).

#![deny(clippy::float_arithmetic)]

use bevy_ecs::prelude::{Schedule, World};
use bevy_ecs::schedule::{IntoScheduleConfigs, ScheduleLabel};
#[cfg(debug_assertions)]
use bevy_ecs::schedule::{LogLevel, ScheduleBuildSettings};

use crate::simulation::{ArrivedThisTick, SimTick};
use crate::systems::{
    boarding_approach_system, mission_completion_system, mission_steering_system, movement_system,
    passenger_sync_system, replanner_system, seek_system,
};

#[derive(ScheduleLabel, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SimulationSchedule;

/// Owns the ECS [`World`] and gameplay [`Schedule`] for a simulation instance.
pub struct Core {
    world: World,
    schedule: Schedule,
}

impl Core {
    /// Creates an empty world and builds the simulation schedule.
    ///
    /// # Panics
    ///
    /// When the schedule does not build — in debug builds, that includes two systems with
    /// conflicting access and no order between them.
    #[must_use]
    pub fn new() -> Self {
        let mut world = World::new();
        world.insert_resource(ArrivedThisTick::default());
        world.insert_resource(SimTick::default());

        let mut schedule = Schedule::new(SimulationSchedule);
        // Two systems touching the same data in no fixed order would run in whatever order the
        // executor picks. Debug builds — every test — refuse to build such a schedule.
        #[cfg(debug_assertions)]
        schedule.set_build_settings(ScheduleBuildSettings {
            ambiguity_detection: LogLevel::Error,
            ..Default::default()
        });
        // Order matters: automation proposes, steering resolves, movement integrates, and
        // arrival is judged on where everyone ended up this tick.
        schedule.add_systems(
            (
                mission_steering_system,
                boarding_approach_system,
                seek_system,
                movement_system,
                passenger_sync_system,
                mission_completion_system,
                replanner_system,
            )
                .chain(),
        );

        // Build now, not on the first step. Building creates resources, and resources are
        // entities: done lazily, it would take an entity index in the middle of the match, and an
        // entity created before the first step would get a different id from one created after.
        schedule
            .initialize(&mut world)
            .expect("the simulation schedule builds");

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::Position;
    use bevy_ecs::prelude::Query;
    use bevy_ecs::schedule::{LogLevel, ScheduleBuildSettings};

    #[test]
    fn the_simulation_schedule_builds_with_ambiguities_as_errors() {
        // `Core::new` builds the schedule and panics if it does not build; in debug builds
        // ambiguity detection is an error, so getting here means no ambiguities.
        let core = Core::new();
        assert_eq!(
            core.schedule().get_build_settings().ambiguity_detection,
            LogLevel::Error
        );
    }

    #[test]
    fn the_first_step_takes_no_entity_index() {
        let mut core = Core::new();
        let before = core.world_mut().spawn_empty().id();
        core.run_schedule();
        let after = core.world_mut().spawn_empty().id();
        assert_eq!(after.index_u32(), before.index_u32() + 1);
    }

    #[test]
    fn ambiguity_detection_catches_unordered_writers() {
        fn nudge_x(mut positions: Query<&mut Position>) {
            for mut position in &mut positions {
                position.x += 1;
            }
        }
        fn nudge_y(mut positions: Query<&mut Position>) {
            for mut position in &mut positions {
                position.y += 1;
            }
        }

        let mut world = World::new();
        let mut schedule = Schedule::new(SimulationSchedule);
        schedule.set_build_settings(ScheduleBuildSettings {
            ambiguity_detection: LogLevel::Error,
            ..Default::default()
        });
        // Not chained: both write `Position` and nothing says which runs first.
        schedule.add_systems((nudge_x, nudge_y));

        assert!(schedule.initialize(&mut world).is_err());
    }
}
