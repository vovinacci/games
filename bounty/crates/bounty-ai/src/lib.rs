//! Computer players. Each is a function from what a player can see ([`Observation`]) to a command; none can change
//! a battle or see its random numbers. A player that changes its choices gets a new name, so recorded results always
//! say which one played.

use bounty_core::battle::{Action, Command, Observation};

/// The name `approach_v1` goes by in suites and reports.
pub const APPROACH_V1: &str = "approach-v1";

/// The plain pilot of the regression runs (`docs/spec/04-AI-AND-BALANCE.md`, "Minimal first pilot"): strike a
/// neighbouring enemy if there is one, the one with the lowest ID first; otherwise pass. It never waits and never
/// casts. Not clever on purpose: it exists so that a change in the rules shows up as a change in its results.
///
/// `None` when the battle is over.
#[must_use]
pub fn approach_v1(seen: &Observation) -> Option<Command> {
    let active = seen.active.as_ref()?;
    // Legal actions come ordered: attacks by target ID first.
    let melee = seen
        .legal
        .iter()
        .find(|a| matches!(a, Action::Melee { .. }));
    Some(Command {
        revision: seen.revision,
        actor: active.stack.clone(),
        action: melee.cloned().unwrap_or(Action::Pass),
    })
}
