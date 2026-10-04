//! [`Core`]: the ECS world and the fixed system schedule behind [`Api`](crate::Api).

#![deny(clippy::float_arithmetic)]

use bevy_ecs::prelude::{Schedule, SystemSet, World};
use bevy_ecs::schedule::{IntoScheduleConfigs, ScheduleLabel};
#[cfg(debug_assertions)]
use bevy_ecs::schedule::{LogLevel, ScheduleBuildSettings};
use bevy_ecs::system::ScheduleSystem;

use crate::simulation::{ArrivedThisTick, SimSet, SimTick};
use crate::systems::{
    boarding_approach_system, mission_completion_system, mission_steering_system, movement_system,
    passenger_sync_system, replanner_system, seek_system,
};

#[derive(ScheduleLabel, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SimulationSchedule;

/// The built-in systems of one [`SimSet`].
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Builtins(SimSet);

/// The systems of one `extend::add_systems` call: the `n`th batch added to a set.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct UserBatch(SimSet, u32);

/// Owns the ECS [`World`] and gameplay [`Schedule`] for a simulation instance.
pub struct Core {
    world: World,
    schedule: Schedule,
    /// Batches added to each [`SimSet`] so far, by [`SimSet::index`].
    user_batches: [u32; SimSet::ALL.len()],
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
        // arrival is judged on where everyone ended up this tick. The sets run in a chain, and
        // within each set the built-ins run first and chained, so the order of the built-ins is
        // one line: mission steering, boarding approach, seek, movement, passenger sync, mission
        // completion, replanner.
        schedule.configure_sets(
            (
                SimSet::Commands,
                SimSet::Steering,
                SimSet::Movement,
                SimSet::PostMovement,
                SimSet::Resolve,
            )
                .chain(),
        );
        for set in SimSet::ALL {
            schedule.configure_sets(Builtins(set).in_set(set));
        }
        schedule.add_systems(
            (
                mission_steering_system,
                boarding_approach_system,
                seek_system,
            )
                .chain()
                .in_set(Builtins(SimSet::Steering)),
        );
        schedule.add_systems(movement_system.in_set(Builtins(SimSet::Movement)));
        schedule.add_systems(passenger_sync_system.in_set(Builtins(SimSet::PostMovement)));
        schedule.add_systems(
            (mission_completion_system, replanner_system)
                .chain()
                .in_set(Builtins(SimSet::Resolve)),
        );

        // Build now, not on the first step. Building creates resources, and resources are
        // entities: done lazily, it would take an entity index in the middle of the match, and an
        // entity created before the first step would get a different id from one created after.
        schedule
            .initialize(&mut world)
            .expect("the simulation schedule builds");

        Self {
            world,
            schedule,
            user_batches: [0; SimSet::ALL.len()],
        }
    }

    /// Adds a game's systems to `set`: chained, after the set's built-ins and after every batch
    /// added to the set before. Rebuilds the schedule at once, for the same reason [`Core::new`]
    /// builds it eagerly.
    ///
    /// # Errors
    ///
    /// The build error, rendered, when the schedule does not build.
    pub(crate) fn add_user_systems<M>(
        &mut self,
        set: SimSet,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> Result<(), String> {
        let count = &mut self.user_batches[set.index()];
        let batch = UserBatch(set, *count);
        if *count == 0 {
            self.schedule
                .configure_sets(batch.in_set(set).after(Builtins(set)));
        } else {
            self.schedule
                .configure_sets(batch.in_set(set).after(UserBatch(set, *count - 1)));
        }
        *count += 1;
        self.schedule.add_systems(systems.chain().in_set(batch));
        match self.schedule.initialize(&mut self.world) {
            Ok(_) => Ok(()),
            Err(err) => Err(err.to_string(self.schedule.graph(), &self.world)),
        }
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
