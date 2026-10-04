//! Replays: the content a match started from, plus every command and the tick it applied at.
//!
//! The simulation is deterministic, so that is the whole match: [`Replay::run`] loads the content
//! into a fresh [`Api`], schedules the commands, and steps. The same replay gives the same
//! [`Api::state_hash`] on every platform. Stored as JSON.

#![deny(clippy::float_arithmetic)]

use serde::{Deserialize, Serialize};

use crate::api::Api;
use crate::commands::{Command, ScheduleError};
use crate::import::ImportError;
use crate::map::MapError;

/// The [`Replay::version`] this build reads and writes.
pub const REPLAY_VERSION: u32 = 1;

/// A recorded match.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Replay {
    /// Format version, [`REPLAY_VERSION`].
    pub version: u32,
    /// Templates file, as passed to [`Api::load_templates_yaml`].
    pub templates_yaml: String,
    /// Map file, as passed to [`Api::load_map_yaml`]; empty for a match without one.
    pub map_yaml: String,
    /// Match seed. Reserved: the RNG resource seeded from it arrives with the first system that
    /// needs randomness, and the field is here so replays recorded before that keep their shape.
    pub seed: u64,
    /// Every command, with the tick it applied at.
    pub commands: Vec<ReplayCommand>,
}

/// One command of a [`Replay`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayCommand {
    /// Tick the command applied at: it ran at the start of the step that produced this tick, so
    /// it is at least `1`.
    pub tick: u64,
    /// The command.
    pub command: Command,
}

/// Why a replay could not be read or run.
#[derive(Debug)]
pub enum ReplayError {
    /// The JSON is malformed or does not have the replay's shape.
    Json(serde_json::Error),
    /// A version this build does not know.
    UnsupportedVersion(u32),
    /// The templates did not load.
    Templates(ImportError),
    /// The map did not load.
    Map(MapError),
    /// A command's tick cannot be scheduled: tick `0` has no step to apply it in.
    Schedule {
        /// Position of the command in [`Replay::commands`].
        index: usize,
        /// Why it was refused.
        error: ScheduleError,
    },
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(err) => write!(f, "replay JSON is invalid: {err}"),
            Self::UnsupportedVersion(version) => write!(
                f,
                "replay version {version} is not supported (this build reads {REPLAY_VERSION})"
            ),
            Self::Templates(err) => write!(f, "replay templates: {err}"),
            Self::Map(err) => write!(f, "replay map: {err}"),
            Self::Schedule { index, error } => write!(f, "replay command #{index}: {error}"),
        }
    }
}

impl std::error::Error for ReplayError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Json(err) => Some(err),
            Self::Templates(err) => Some(err),
            Self::Map(err) => Some(err),
            Self::Schedule { error, .. } => Some(error),
            Self::UnsupportedVersion(_) => None,
        }
    }
}

impl Replay {
    /// Reads a replay from JSON.
    ///
    /// # Errors
    ///
    /// [`ReplayError::Json`] when the text is not a replay.
    pub fn from_json(json: &str) -> Result<Self, ReplayError> {
        serde_json::from_str(json).map_err(ReplayError::Json)
    }

    /// Writes the replay as pretty-printed JSON.
    ///
    /// # Panics
    ///
    /// Never: every field serializes.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a replay always serializes")
    }

    /// Plays the match up to `until_tick` and returns the simulation there.
    ///
    /// Commands scheduled after `until_tick` stay queued in the returned [`Api`]: stepping it
    /// further carries on with the recording. `run(0)` loads and schedules without stepping.
    ///
    /// # Errors
    ///
    /// [`ReplayError::UnsupportedVersion`], [`ReplayError::Templates`], [`ReplayError::Map`], or
    /// [`ReplayError::Schedule`] for a command at tick `0`.
    pub fn run(&self, until_tick: u64) -> Result<Api, ReplayError> {
        if self.version != REPLAY_VERSION {
            return Err(ReplayError::UnsupportedVersion(self.version));
        }

        let mut api = Api::new();
        api.load_templates_yaml(&self.templates_yaml)
            .map_err(ReplayError::Templates)?;
        if !self.map_yaml.trim().is_empty() {
            api.load_map_yaml(&self.map_yaml)
                .map_err(ReplayError::Map)?;
        }
        for (index, entry) in self.commands.iter().enumerate() {
            api.schedule(entry.tick, entry.command.clone())
                .map_err(|error| ReplayError::Schedule { index, error })?;
        }

        while api.current_tick() < until_tick {
            api.step();
        }
        Ok(api)
    }
}
