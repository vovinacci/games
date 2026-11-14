//! A battle written down as the commands that made it, so it can be played again exactly: for a bug report, and to
//! prove that a change to the code changed no outcome. The tape holds the commands that were accepted, whoever chose
//! them, so playing it back needs no computer player.

use std::fmt;
use std::sync::Arc;

use crate::battle::{Battle, Command, Event, RuleError};
use crate::content::{ContentError, Rules, Scenario};

/// The accepted commands of one battle, with the digest of the state after each.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tape {
    pub scenario_id: String,
    /// The random state the battle started from.
    pub seed: u32,
    pub commands: Vec<Command>,
    /// `digests[i]` is the state after `commands[i]`.
    pub digests: Vec<String>,
}

/// Why a tape could not be played back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplayError {
    /// The tape is for another scenario, or has a different number of commands and digests.
    WrongTape(String),
    /// The scenario does not start under these rules.
    Start(ContentError),
    /// Command `index` is refused now.
    Refused { index: usize, error: RuleError },
    /// Command `index` is accepted but leaves another state than it did when recorded: the first point where the
    /// two battles part.
    Diverged {
        index: usize,
        recorded: String,
        now: String,
    },
}

impl fmt::Display for ReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongTape(why) => write!(f, "the tape does not fit: {why}"),
            Self::Start(error) => write!(f, "the battle does not start: {error}"),
            Self::Refused { index, error } => write!(f, "command {index} is refused: {error}"),
            Self::Diverged {
                index,
                recorded,
                now,
            } => {
                write!(
                    f,
                    "after command {index} the state is {now}, recorded was {recorded}"
                )
            }
        }
    }
}

impl std::error::Error for ReplayError {}

impl Tape {
    /// An empty tape for a battle of `scenario` started from `seed`.
    #[must_use]
    pub fn new(scenario: &Scenario, seed: u32) -> Self {
        Self {
            scenario_id: scenario.id.clone(),
            seed,
            commands: Vec::new(),
            digests: Vec::new(),
        }
    }

    /// Applies the command and, when it is accepted, writes it down.
    ///
    /// # Errors
    /// As [`Battle::apply`]; a refused command is not recorded.
    pub fn apply(
        &mut self,
        battle: &mut Battle,
        command: &Command,
    ) -> Result<Vec<Event>, RuleError> {
        let events = battle.apply(command)?;
        self.commands.push(command.clone());
        self.digests.push(battle.digest());
        Ok(events)
    }

    /// Plays the tape from the start and returns the battle as it stands after the last command.
    ///
    /// # Errors
    /// [`ReplayError`] naming the first command that no longer does what it did.
    pub fn replay(&self, rules: Arc<Rules>, scenario: &Scenario) -> Result<Battle, ReplayError> {
        if scenario.id != self.scenario_id {
            return Err(ReplayError::WrongTape(format!(
                "recorded for scenario {:?}, played on {:?}",
                self.scenario_id, scenario.id
            )));
        }
        if self.commands.len() != self.digests.len() {
            return Err(ReplayError::WrongTape(format!(
                "{} commands but {} digests",
                self.commands.len(),
                self.digests.len()
            )));
        }
        let mut battle =
            Battle::start(rules, scenario, Some(self.seed)).map_err(ReplayError::Start)?;
        for (index, (command, recorded)) in self.commands.iter().zip(&self.digests).enumerate() {
            battle
                .apply(command)
                .map_err(|error| ReplayError::Refused { index, error })?;
            let now = battle.digest();
            if now != *recorded {
                return Err(ReplayError::Diverged {
                    index,
                    recorded: recorded.clone(),
                    now,
                });
            }
        }
        Ok(battle)
    }
}
