//! Commands: orders as data, applied at the start of a known tick.
//!
//! A host does not change the world directly. It hands the [`Api`] a [`Command`] with
//! [`Api::submit`] (next tick) or [`Api::schedule`] (a chosen future tick), and the command is
//! applied at the start of the step that produces that tick. That is what makes lockstep and
//! replays possible: two peers holding the same command log apply the same commands at the same
//! moments, and a [`Replay`](crate::Replay) is nothing more than that log.
//!
//! Within one tick, commands apply in [`CommandSeq`] order — the order they were handed in.
//! [`Api::step`] returns a [`StepReport`] saying what each one did.
//!
//! The immediate `Api` methods (`order_move_to`, `create_group`, …) are what the commands call.
//! They stay public for tools and tests; a networked host uses `submit` and `schedule` only.

#![deny(clippy::float_arithmetic)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::api::Api;
use crate::boarding::BoardError;
use crate::components::MoveTarget;
use crate::entity_components::EntityComponents;
use crate::groups::GroupError;
use crate::missions::MissionError;
use crate::orders::{EntityId, OrderReport};

/// Position of a command in the order it was handed to the [`Api`]: the tie-break within a tick.
///
/// Sequence numbers grow by one with every [`Api::submit`] and [`Api::schedule`] call and are
/// never reused, so a host can match a [`CommandOutcome`] to the call that produced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommandSeq(pub u64);

/// One order, as data.
///
/// Serializes as JSON tagged by `type` in `snake_case`: `{"type": "move_to", "ids": [...],
/// "target": {"x": 10.0, "y": 0.0}}`. Points, radii and override fields are in map units, as
/// everywhere else outside the simulation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    /// [`Api::spawn_entity`]: one entity from a loaded template, with optional overrides.
    Spawn {
        /// Template name.
        template: String,
        /// Fields that replace the template's; may be left out.
        #[serde(default)]
        overrides: EntityComponents,
    },
    /// [`Api::despawn`].
    Despawn {
        /// Entities to remove.
        ids: Vec<EntityId>,
    },
    /// [`Api::order_move_to`].
    MoveTo {
        /// Units to move.
        ids: Vec<EntityId>,
        /// Destination; several units spread over a grid around it.
        target: MoveTarget,
    },
    /// [`Api::order_stop`].
    Stop {
        /// Entities to stop.
        ids: Vec<EntityId>,
    },
    /// [`Api::create_group`].
    CreateGroup {
        /// The faction the group commands.
        faction: u32,
    },
    /// [`Api::add_to_group`].
    AddToGroup {
        /// Group to join.
        group: EntityId,
        /// Unit joining it.
        unit: EntityId,
    },
    /// [`Api::remove_from_group`].
    RemoveFromGroup {
        /// Unit leaving its group.
        unit: EntityId,
    },
    /// [`Api::order_group_move_to`].
    GroupMoveTo {
        /// Group to order.
        group: EntityId,
        /// Destination.
        target: MoveTarget,
    },
    /// [`Api::clear_group_manual`].
    ClearGroupManual {
        /// Group to hand back to automation.
        group: EntityId,
    },
    /// [`Api::create_mission`].
    CreateMission {
        /// Point to reach.
        target: MoveTarget,
        /// Arrival radius in milli-units; map units on the wire.
        #[serde(with = "crate::units::distance")]
        radius: i32,
    },
    /// [`Api::assign_group`].
    AssignGroup {
        /// Mission to work.
        mission: EntityId,
        /// Group sent to it.
        group: EntityId,
    },
    /// [`Api::unassign_group`].
    UnassignGroup {
        /// Group taken off its mission.
        group: EntityId,
    },
    /// [`Api::order_board`]: walk over and get in.
    Board {
        /// Units to board.
        units: Vec<EntityId>,
        /// Vehicle to board.
        vehicle: EntityId,
    },
    /// [`Api::unboard`], for each unit.
    Unboard {
        /// Passengers to let off.
        units: Vec<EntityId>,
    },
}

/// What an applied command did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandResult {
    /// [`Command::Spawn`]: the new entity.
    Spawned(EntityId),
    /// [`Command::CreateGroup`]: the new group.
    GroupCreated(EntityId),
    /// [`Command::CreateMission`]: the new mission.
    MissionCreated(EntityId),
    /// Any other command: how many of the entities it named it applied to, and how many it
    /// skipped — unknown, repeated, or not in a state the order fits. A command that names one
    /// thing (`AddToGroup`, `UnassignGroup`, …) reports `1/0` when it changed something and `0/1`
    /// when there was nothing to change.
    Applied {
        /// Entities the command applied to.
        applied: usize,
        /// Entities it skipped.
        skipped: usize,
    },
}

impl CommandResult {
    fn from_report(report: OrderReport) -> Self {
        Self::Applied {
            applied: report.ordered,
            skipped: report.skipped,
        }
    }

    fn from_flag(changed: bool) -> Self {
        Self::Applied {
            applied: usize::from(changed),
            skipped: usize::from(!changed),
        }
    }
}

/// Why a command was refused. A refused command changes nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandError {
    /// [`Command::Spawn`] before any templates were loaded.
    TemplatesNotLoaded,
    /// [`Command::Spawn`] named a template that is not loaded.
    UnknownTemplate(String),
    /// A group command was refused.
    Group(GroupError),
    /// A mission command was refused.
    Mission(MissionError),
    /// A boarding command was refused.
    Board(BoardError),
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TemplatesNotLoaded => {
                f.write_str("templates not loaded; call load_templates_yaml first")
            }
            Self::UnknownTemplate(name) => write!(f, "unknown template name: {name}"),
            Self::Group(err) => err.fmt(f),
            Self::Mission(err) => err.fmt(f),
            Self::Board(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for CommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::TemplatesNotLoaded | Self::UnknownTemplate(_) => None,
            Self::Group(err) => Some(err),
            Self::Mission(err) => Some(err),
            Self::Board(err) => Some(err),
        }
    }
}

/// What happened to one command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutcome {
    /// The sequence number [`Api::submit`] or [`Api::schedule`] returned for it.
    pub seq: CommandSeq,
    /// What it did, or why it was refused.
    pub result: Result<CommandResult, CommandError>,
}

/// What one [`Api::step`] did with the commands due.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StepReport {
    /// The tick this step produced: [`Api::current_tick`] after it.
    pub tick: u64,
    /// One outcome per command applied, in [`CommandSeq`] order.
    pub outcomes: Vec<CommandOutcome>,
}

/// Why [`Api::schedule`] refused a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleError {
    /// The tick is not after the current one: it has already been produced.
    NotInFuture {
        /// The tick asked for.
        tick: u64,
        /// [`Api::current_tick`] at the time.
        current: u64,
    },
}

impl std::fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotInFuture { tick, current } => write!(
                f,
                "cannot schedule for tick {tick}: the simulation is at tick {current}, so the \
                 earliest is {}",
                current.saturating_add(1)
            ),
        }
    }
}

impl std::error::Error for ScheduleError {}

/// Commands waiting for their tick.
#[derive(Debug, Default)]
pub(crate) struct CommandQueue {
    next_seq: u64,
    /// Per tick, in [`CommandSeq`] order: sequence numbers only grow, so pushing keeps each
    /// list sorted.
    pending: BTreeMap<u64, Vec<(CommandSeq, Command)>>,
}

impl CommandQueue {
    fn push(&mut self, tick: u64, command: Command) -> CommandSeq {
        let seq = CommandSeq(self.next_seq);
        self.next_seq += 1;
        self.pending.entry(tick).or_default().push((seq, command));
        seq
    }

    /// Removes and returns the commands due at `tick`.
    pub(crate) fn take(&mut self, tick: u64) -> Vec<(CommandSeq, Command)> {
        self.pending.remove(&tick).unwrap_or_default()
    }
}

impl Api {
    /// Queues a command for the next tick, `current_tick() + 1`.
    ///
    /// Nothing changes until the [`Api::step`] that produces that tick; its [`StepReport`] carries
    /// the outcome under the returned sequence number.
    pub fn submit(&mut self, command: Command) -> CommandSeq {
        let tick = self.current_tick() + 1;
        self.commands.push(tick, command)
    }

    /// Queues a command for a chosen future tick.
    ///
    /// # Errors
    ///
    /// [`ScheduleError::NotInFuture`] when `tick` is not after [`Api::current_tick`].
    pub fn schedule(&mut self, tick: u64, command: Command) -> Result<CommandSeq, ScheduleError> {
        let current = self.current_tick();
        if tick <= current {
            return Err(ScheduleError::NotInFuture { tick, current });
        }
        Ok(self.commands.push(tick, command))
    }

    /// Applies one command now, through the immediate methods.
    pub(crate) fn apply_command(
        &mut self,
        command: Command,
    ) -> Result<CommandResult, CommandError> {
        Ok(match command {
            Command::Spawn {
                template,
                overrides,
            } => {
                let templates = self
                    .templates
                    .as_ref()
                    .ok_or(CommandError::TemplatesNotLoaded)?;
                if !templates.contains_key(&template) {
                    return Err(CommandError::UnknownTemplate(template));
                }
                let id = self
                    .spawn_entity(&template, overrides)
                    .expect("templates are loaded and the name was checked");
                CommandResult::Spawned(id)
            }
            Command::Despawn { ids } => {
                let removed = self.despawn(&ids);
                CommandResult::Applied {
                    applied: removed,
                    skipped: ids.len() - removed,
                }
            }
            Command::MoveTo { ids, target } => {
                CommandResult::from_report(self.order_move_to(&ids, target))
            }
            Command::Stop { ids } => CommandResult::from_report(self.order_stop(&ids)),
            Command::CreateGroup { faction } => {
                CommandResult::GroupCreated(self.create_group(faction))
            }
            Command::AddToGroup { group, unit } => {
                self.add_to_group(group, unit)
                    .map_err(CommandError::Group)?;
                CommandResult::from_flag(true)
            }
            Command::RemoveFromGroup { unit } => {
                CommandResult::from_flag(self.remove_from_group(unit))
            }
            Command::GroupMoveTo { group, target } => CommandResult::from_report(
                self.order_group_move_to(group, target)
                    .map_err(CommandError::Group)?,
            ),
            Command::ClearGroupManual { group } => CommandResult::from_flag(
                self.clear_group_manual(group)
                    .map_err(CommandError::Group)?,
            ),
            Command::CreateMission { target, radius } => {
                CommandResult::MissionCreated(self.create_mission(target, radius))
            }
            Command::AssignGroup { mission, group } => {
                self.assign_group(mission, group)
                    .map_err(CommandError::Mission)?;
                CommandResult::from_flag(true)
            }
            Command::UnassignGroup { group } => {
                CommandResult::from_flag(self.unassign_group(group))
            }
            Command::Board { units, vehicle } => CommandResult::from_report(
                self.order_board(&units, vehicle)
                    .map_err(CommandError::Board)?,
            ),
            Command::Unboard { units } => {
                let applied = units
                    .iter()
                    .filter(|unit| self.unboard(**unit).is_ok())
                    .count();
                CommandResult::Applied {
                    applied,
                    skipped: units.len() - applied,
                }
            }
        })
    }
}
