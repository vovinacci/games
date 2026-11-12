//! One battle: its state, the commands that change it and the events that tell what happened.
//!
//! The only way to change a [`Battle`] is [`Battle::apply`]. A command that is refused changes nothing: not the
//! state, not the revision, not the random generator. Everything a command needs is checked before anything is
//! touched; `apply` works on a copy and keeps it only when the whole command went through.

use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::sync::Arc;

use crate::canon::Value;
use crate::content::{Cell, ContentError, Rules, Scenario, Side, Troop};
use crate::rng::Lcg32;

/// A group of creatures of one kind, acting as one piece on the board.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stack {
    /// Stable for the whole battle; commands and events name stacks by it.
    pub id: String,
    pub side: Side,
    pub slot: u32,
    pub troop_id: String,
    /// Index of the troop in this battle's rules.
    troop: usize,
    pub cell: Cell,
    /// Living creatures; 0 means the stack is dead and off the board.
    pub count: u32,
    /// Health missing from the one hurt creature, always less than one creature's health.
    pub injury: u32,
    pub shots: u32,
    /// Has struck back this round.
    pub retaliated: bool,
    /// Has waited this round.
    pub waited: bool,
}

impl Stack {
    #[must_use]
    pub const fn alive(&self) -> bool {
        self.count > 0
    }
}

/// The stack whose turn it is and what it may still do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Activation {
    pub stack: String,
    pub movement: u32,
    pub flights: u32,
    /// Has walked or flown in this activation; waiting is then no longer allowed.
    pub moved: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    PlayerWin,
    EnemyWin,
    /// The round limit passed with both sides alive. A diagnosis, not a defeat of either side.
    BudgetExceeded,
}

impl Outcome {
    /// The name used in reports and digests.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::PlayerWin => "player_win",
            Self::EnemyWin => "enemy_win",
            Self::BudgetExceeded => "budget_exceeded",
        }
    }
}

/// What the active stack is asked to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// End the activation.
    Pass,
    /// Strike a living enemy stack in one of the eight neighbouring cells. Ends the activation.
    Melee { target: String },
}

/// A request to change the battle. It names what should happen, never the result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Command {
    /// The revision the sender saw. A command made for an older state is refused, not applied to whatever
    /// stands there now.
    pub revision: u64,
    /// The stack that acts.
    pub actor: String,
    pub action: Action,
}

/// Why a command was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleError {
    /// The battle has ended.
    BattleOver,
    /// The command was made for another revision.
    Stale { command: u64, battle: u64 },
    /// The actor is not the stack whose turn it is (or no such stack).
    NotActive { actor: String },
    /// The action needs a movement point and the stack has none left.
    NoMovement,
    /// The target is not a living enemy stack.
    NotAnEnemy { target: String },
    /// The target is not in a neighbouring cell.
    NotAdjacent { target: String },
}

impl fmt::Display for RuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BattleOver => f.write_str("the battle is over"),
            Self::Stale { command, battle } => {
                write!(
                    f,
                    "command is for revision {command}, the battle is at {battle}"
                )
            }
            Self::NotActive { actor } => write!(f, "it is not the turn of {actor:?}"),
            Self::NoMovement => f.write_str("no movement points left"),
            Self::NotAnEnemy { target } => write!(f, "{target:?} is not a living enemy stack"),
            Self::NotAdjacent { target } => write!(f, "{target:?} is not in a neighbouring cell"),
        }
    }
}

impl std::error::Error for RuleError {}

/// Something that happened, in the order it happened. Holds IDs and numbers, never text for the screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    RoundStarted {
        round: u32,
    },
    ActivationStarted {
        stack: String,
    },
    ActivationEnded {
        stack: String,
    },
    /// One blow, with everything that went into it and what it left behind.
    Damage {
        attacker: String,
        target: String,
        /// The damage roll for one creature.
        roll: u32,
        /// Attacking creatures at the moment of the blow.
        attackers: u32,
        /// `roll` times `attackers`.
        damage: u64,
        /// Creatures the target lost.
        losses: u32,
        /// The target after the blow.
        count: u32,
        injury: u32,
        /// The blow was the target of an attack striking back.
        retaliation: bool,
    },
    /// The stack lost its last creature and left the board.
    StackRemoved {
        stack: String,
    },
    BattleEnded {
        outcome: Outcome,
    },
}

/// A stack of `count` creatures with `hp` health each, `injury` of it missing from one of them, takes `damage`.
/// Returns the count and injury afterwards; a dead stack is `(0, 0)`.
#[must_use]
pub fn health_after(hp: u32, count: u32, injury: u32, damage: u64) -> (u32, u32) {
    let hp = u64::from(hp);
    let health = (u64::from(count) * hp).saturating_sub(u64::from(injury));
    let left = health.saturating_sub(damage);
    if left == 0 || hp == 0 {
        return (0, 0);
    }
    let count = left.div_ceil(hp);
    // `left` is at most the health before, so both numbers fit where they came from.
    (count as u32, (count * hp - left) as u32)
}

/// Where damage rolls come from: the battle's generator in play, a fixed list in tests.
type Rolls<'a> = &'a mut dyn FnMut(&mut Lcg32, u32, u32) -> u32;

/// A stack together with what its troop is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StackView {
    pub stack: Stack,
    pub troop: Troop,
}

/// What a player of either side can see: the board, the stacks, whose turn it is and what that stack may do. A
/// computer player is given this and nothing else, so it cannot look at the random generator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observation {
    pub revision: u64,
    pub round: u32,
    pub width: u32,
    pub height: u32,
    /// Every stack, dead ones included, player slots first.
    pub stacks: Vec<StackView>,
    pub obstacles: Vec<Cell>,
    pub active: Option<Activation>,
    /// As [`Battle::legal_actions`].
    pub legal: Vec<Action>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Battle {
    rules: Arc<Rules>,
    /// Goes up by one with every accepted command.
    revision: u64,
    /// The round being played, from 1.
    round: u32,
    rng: Lcg32,
    /// Player slots in rising order, then enemy slots. Dead stacks stay in the list with a count of 0.
    stacks: Vec<Stack>,
    obstacles: Vec<Cell>,
    /// Stacks still to act this round, in order.
    queue: VecDeque<String>,
    /// Stacks that waited, in the order they did.
    deferred: VecDeque<String>,
    active: Option<Activation>,
    hero_charges: BTreeMap<String, u32>,
    /// The player's hero has cast this round.
    cast_this_round: bool,
    outcome: Option<Outcome>,
}

impl Battle {
    /// Sets up the scenario and starts round 1. `rng_state` replaces the scenario's own random state, for runs
    /// over a list of seeds; the scenario itself is never changed.
    ///
    /// # Errors
    /// The scenario does not fit the rules, see [`Scenario::check`].
    pub fn start(
        rules: Arc<Rules>,
        scenario: &Scenario,
        rng_state: Option<u32>,
    ) -> Result<Self, ContentError> {
        scenario.check(&rules)?;
        let mut stacks = Vec::new();
        for start in &scenario.stacks {
            // `check` has seen every troop, so the search finds one.
            let Some(troop) = rules.troops.iter().position(|t| t.id == start.troop_id) else {
                continue;
            };
            stacks.push(Stack {
                id: start.id.clone(),
                side: start.side,
                slot: start.slot,
                troop_id: start.troop_id.clone(),
                troop,
                cell: start.cell(),
                count: start.count,
                injury: start.injury,
                shots: rules.troops[troop].shots,
                retaliated: false,
                waited: false,
            });
        }
        stacks.sort_by_key(|s| (s.side, s.slot));
        let mut obstacles = scenario.obstacles.clone();
        obstacles.sort_by_key(|c| (c.y, c.x));
        let mut battle = Self {
            rules,
            revision: 0,
            round: 1,
            rng: Lcg32::new(rng_state.unwrap_or(scenario.rng_state)),
            stacks,
            obstacles,
            queue: VecDeque::new(),
            deferred: VecDeque::new(),
            active: None,
            hero_charges: scenario.hero_charges.clone(),
            cast_this_round: false,
            outcome: None,
        };
        battle.fill_queue();
        battle.next_activation(&mut Vec::new());
        Ok(battle)
    }

    #[must_use]
    pub fn rules(&self) -> &Rules {
        &self.rules
    }

    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    #[must_use]
    pub const fn round(&self) -> u32 {
        self.round
    }

    #[must_use]
    pub const fn rng_state(&self) -> u32 {
        self.rng.state()
    }

    /// Every stack, dead ones included, player slots first.
    #[must_use]
    pub fn stacks(&self) -> &[Stack] {
        &self.stacks
    }

    #[must_use]
    pub fn stack(&self, id: &str) -> Option<&Stack> {
        self.stacks.iter().find(|s| s.id == id)
    }

    /// What a stack is, from the rules.
    #[must_use]
    pub fn troop(&self, stack: &Stack) -> &Troop {
        &self.rules.troops[stack.troop]
    }

    #[must_use]
    pub fn obstacles(&self) -> &[Cell] {
        &self.obstacles
    }

    /// The stack whose turn it is; `None` once the battle is over.
    #[must_use]
    pub const fn active(&self) -> Option<&Activation> {
        self.active.as_ref()
    }

    #[must_use]
    pub const fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }

    /// Charges of each hero spell the player side has left.
    #[must_use]
    pub const fn hero_charges(&self) -> &BTreeMap<String, u32> {
        &self.hero_charges
    }

    /// The battle as a player sees it.
    #[must_use]
    pub fn observe(&self) -> Observation {
        let view = |s: &Stack| StackView {
            stack: s.clone(),
            troop: self.troop(s).clone(),
        };
        Observation {
            revision: self.revision,
            round: self.round,
            width: self.rules.board.width,
            height: self.rules.board.height,
            stacks: self.stacks.iter().map(view).collect(),
            obstacles: self.obstacles.clone(),
            active: self.active.clone(),
            legal: self.legal_actions(),
        }
    }

    /// Carries out a command, or refuses it and leaves the battle exactly as it was.
    ///
    /// # Errors
    /// [`RuleError`] saying why the command is not allowed now.
    pub fn apply(&mut self, command: &Command) -> Result<Vec<Event>, RuleError> {
        self.apply_with(command, &mut |rng, lo, hi| rng.range(lo, hi))
    }

    fn apply_with(&mut self, command: &Command, rolls: Rolls<'_>) -> Result<Vec<Event>, RuleError> {
        let mut next = self.clone();
        let mut events = Vec::new();
        next.carry_out(command, rolls, &mut events)?;
        next.revision += 1;
        *self = next;
        Ok(events)
    }

    /// What the active stack may do now, in a fixed order: attacks by target ID, then Pass. Empty once the battle
    /// is over. Every command built from one of these is accepted; the screen and the computer players take their
    /// choices from here, not from rules of their own.
    #[must_use]
    pub fn legal_actions(&self) -> Vec<Action> {
        let Some(active) = &self.active else {
            return Vec::new();
        };
        let Some(actor) = self.stack(&active.stack) else {
            return Vec::new();
        };
        let mut actions = Vec::new();
        if active.movement > 0 {
            let mut targets: Vec<&Stack> = self
                .stacks
                .iter()
                .filter(|s| s.alive() && s.side != actor.side && adjacent(s.cell, actor.cell))
                .collect();
            targets.sort_by(|a, b| a.id.cmp(&b.id));
            actions.extend(targets.into_iter().map(|s| Action::Melee {
                target: s.id.clone(),
            }));
        }
        actions.push(Action::Pass);
        actions
    }

    fn carry_out(
        &mut self,
        command: &Command,
        rolls: Rolls<'_>,
        events: &mut Vec<Event>,
    ) -> Result<(), RuleError> {
        if self.outcome.is_some() {
            return Err(RuleError::BattleOver);
        }
        if command.revision != self.revision {
            return Err(RuleError::Stale {
                command: command.revision,
                battle: self.revision,
            });
        }
        let Some(active) = self.active.clone().filter(|a| a.stack == command.actor) else {
            return Err(RuleError::NotActive {
                actor: command.actor.clone(),
            });
        };
        match &command.action {
            Action::Pass => self.end_activation(events),
            Action::Melee { target } => {
                if active.movement == 0 {
                    return Err(RuleError::NoMovement);
                }
                let (Some(attacker), Some(defender)) =
                    (self.index(&active.stack), self.index(target))
                else {
                    return Err(RuleError::NotAnEnemy {
                        target: target.clone(),
                    });
                };
                let (a, d) = (&self.stacks[attacker], &self.stacks[defender]);
                if !d.alive() || d.side == a.side {
                    return Err(RuleError::NotAnEnemy {
                        target: target.clone(),
                    });
                }
                if !adjacent(a.cell, d.cell) {
                    return Err(RuleError::NotAdjacent {
                        target: target.clone(),
                    });
                }
                self.melee(attacker, defender, rolls, events);
            }
        }
        Ok(())
    }

    fn index(&self, id: &str) -> Option<usize> {
        self.stacks.iter().position(|s| s.id == id)
    }

    /// The attacker strikes; a defender that survives and has not struck back this round does so once, with the
    /// creatures it has left. Then the attacker's activation ends.
    fn melee(
        &mut self,
        attacker: usize,
        defender: usize,
        rolls: Rolls<'_>,
        events: &mut Vec<Event>,
    ) {
        self.strike(attacker, defender, false, rolls, events);
        if self.outcome.is_none()
            && self.stacks[defender].alive()
            && !self.stacks[defender].retaliated
        {
            self.stacks[defender].retaliated = true;
            self.strike(defender, attacker, true, rolls, events);
        }
        if self.outcome.is_none() {
            self.end_activation(events);
        }
    }

    /// One melee blow: one roll from the attacker's range, times its creatures.
    fn strike(
        &mut self,
        from: usize,
        to: usize,
        retaliation: bool,
        rolls: Rolls<'_>,
        events: &mut Vec<Event>,
    ) {
        let troop = &self.rules.troops[self.stacks[from].troop];
        let (lo, hi) = (troop.melee_min, troop.melee_max);
        let roll = rolls(&mut self.rng, lo, hi);
        let damage = u64::from(roll) * u64::from(self.stacks[from].count);
        self.hurt(from, to, roll, damage, retaliation, events);
    }

    /// Takes `damage` off a stack. Removes it when it dies and ends the battle when it was its side's last.
    fn hurt(
        &mut self,
        from: usize,
        to: usize,
        roll: u32,
        damage: u64,
        retaliation: bool,
        events: &mut Vec<Event>,
    ) {
        let hp = self.rules.troops[self.stacks[to].troop].hp;
        let attackers = self.stacks[from].count;
        let attacker = self.stacks[from].id.clone();
        let target = &mut self.stacks[to];
        let before = target.count;
        (target.count, target.injury) = health_after(hp, target.count, target.injury, damage);
        let (id, count, injury, side) =
            (target.id.clone(), target.count, target.injury, target.side);
        events.push(Event::Damage {
            attacker,
            target: id.clone(),
            roll,
            attackers,
            damage,
            losses: before - count,
            count,
            injury,
            retaliation,
        });
        if count > 0 {
            return;
        }
        self.queue.retain(|queued| *queued != id);
        self.deferred.retain(|queued| *queued != id);
        events.push(Event::StackRemoved { stack: id });
        if !self.stacks.iter().any(|s| s.alive() && s.side == side) {
            let outcome = if side == Side::Enemy {
                Outcome::PlayerWin
            } else {
                Outcome::EnemyWin
            };
            self.finish(outcome, events);
        }
    }

    fn end_activation(&mut self, events: &mut Vec<Event>) {
        if let Some(activation) = self.active.take() {
            events.push(Event::ActivationEnded {
                stack: activation.stack,
            });
        }
        self.next_activation(events);
    }

    /// Puts every living stack into the queue, in order.
    fn fill_queue(&mut self) {
        self.queue = self
            .stacks
            .iter()
            .filter(|s| s.alive())
            .map(|s| s.id.clone())
            .collect();
    }

    /// Gives the turn to the next living stack: first the queue, then those that waited. When both are empty the
    /// round is over and the next begins, unless the round limit is reached.
    fn next_activation(&mut self, events: &mut Vec<Event>) {
        loop {
            while let Some(id) = self.queue.pop_front().or_else(|| self.deferred.pop_front()) {
                let Some(stack) = self.stacks.iter().find(|s| s.id == id && s.alive()) else {
                    continue;
                };
                let troop = &self.rules.troops[stack.troop];
                self.active = Some(Activation {
                    stack: id.clone(),
                    movement: troop.movement,
                    flights: troop.flights_per_activation,
                    moved: false,
                });
                events.push(Event::ActivationStarted { stack: id });
                return;
            }
            if self.round >= self.rules.round_limit {
                self.finish(Outcome::BudgetExceeded, events);
                return;
            }
            self.round += 1;
            self.cast_this_round = false;
            for stack in &mut self.stacks {
                stack.retaliated = false;
                stack.waited = false;
            }
            self.fill_queue();
            events.push(Event::RoundStarted { round: self.round });
        }
    }

    fn finish(&mut self, outcome: Outcome, events: &mut Vec<Event>) {
        self.active = None;
        self.queue.clear();
        self.deferred.clear();
        self.outcome = Some(outcome);
        events.push(Event::BattleEnded { outcome });
    }

    /// The whole state that decides what happens next, as a value with one fixed encoding. Stacks are listed by
    /// ID, since their order in memory carries no meaning; the queues keep their order, since theirs does.
    #[must_use]
    pub fn to_canon(&self) -> Value {
        let ids = |queue: &VecDeque<String>| {
            Value::Array(queue.iter().map(|id| Value::str(id)).collect())
        };
        let cell = |c: Cell| Value::object([("x", c.x.into()), ("y", c.y.into())]);
        let mut stacks: Vec<&Stack> = self.stacks.iter().collect();
        stacks.sort_by(|a, b| a.id.cmp(&b.id));
        let stacks = stacks
            .into_iter()
            .map(|s| {
                Value::object([
                    ("cell", cell(s.cell)),
                    ("count", s.count.into()),
                    ("id", Value::str(&s.id)),
                    ("injury", s.injury.into()),
                    ("retaliated", s.retaliated.into()),
                    ("shots", s.shots.into()),
                    ("side", Value::str(s.side.name())),
                    ("slot", s.slot.into()),
                    ("troop_id", Value::str(&s.troop_id)),
                    ("waited", s.waited.into()),
                ])
            })
            .collect();
        let active = self.active.as_ref().map_or(Value::Null, |a| {
            Value::object([
                ("flights", a.flights.into()),
                ("moved", a.moved.into()),
                ("movement", a.movement.into()),
                ("stack", Value::str(&a.stack)),
            ])
        });
        let charges = self
            .hero_charges
            .iter()
            .map(|(spell, n)| (spell.clone(), Value::from(*n)))
            .collect();
        Value::object([
            ("active", active),
            ("cast_this_round", self.cast_this_round.into()),
            ("deferred", ids(&self.deferred)),
            ("hero_charges", Value::Object(charges)),
            (
                "obstacles",
                Value::Array(self.obstacles.iter().map(|&c| cell(c)).collect()),
            ),
            (
                "outcome",
                self.outcome.map_or(Value::Null, |o| Value::str(o.name())),
            ),
            ("queue", ids(&self.queue)),
            ("revision", self.revision.into()),
            (
                "rng",
                Value::object([
                    ("algorithm", Value::str("prototype-lcg32-v1")),
                    ("state", self.rng.state().into()),
                ]),
            ),
            ("round", self.round.into()),
            ("ruleset_id", Value::str(&self.rules.id)),
            ("stacks", Value::Array(stacks)),
        ])
    }

    /// The digest of [`Battle::to_canon`]: two battles with the same digest go on in the same way.
    #[must_use]
    pub fn digest(&self) -> String {
        // Every key above is ASCII and every number a u32 or u64, so the encoding cannot fail.
        self.to_canon().digest().unwrap_or_default()
    }
}

/// In one of the eight cells around, not the same cell.
const fn adjacent(a: Cell, b: Cell) -> bool {
    a.x.abs_diff(b.x) <= 1 && a.y.abs_diff(b.y) <= 1 && (a.x != b.x || a.y != b.y)
}

#[cfg(test)]
mod tests;
