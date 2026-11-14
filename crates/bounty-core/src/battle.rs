//! One battle: its state, the commands that change it and the events that tell what happened.
//!
//! The only way to change a [`Battle`] is [`Battle::apply`]. A command that is refused changes nothing: not the
//! state, not the revision, not the random generator. Everything a command needs is checked before anything is
//! touched; `apply` works on a copy and keeps it only when the whole command went through.

use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::sync::Arc;

use crate::canon::Value;
use crate::content::{
    Ability, Answer, Cell, ContentError, Morale, Profile, Rules, Scenario, Side, SpellEffect,
    Switches, Troop,
};
use crate::rng::{Lcg32, Lcg64, Rng};
use crate::save::{Reader, SaveError, invalid};

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
    /// Reference rules: the stack is too large for the hero's leadership. It keeps its turns but is an enemy to
    /// both sides, its own included, until its losses bring it back under.
    pub out_of_control: bool,
    /// For a troop that regains creatures by leeching: the number it began the battle with, beyond which it
    /// regains none. `None` for every other troop, which has no use for it.
    pub began_with: Option<u32>,
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
    /// The outcome of that name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        [Self::PlayerWin, Self::EnemyWin, Self::BudgetExceeded]
            .into_iter()
            .find(|outcome| outcome.name() == name)
    }
}

/// What the active stack is asked to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// End the activation.
    Pass,
    /// Strike a living enemy stack in one of the eight neighbouring cells. Ends the activation.
    Melee { target: String },
    /// Shoot a living enemy stack anywhere on the board; not allowed with an enemy in a neighbouring cell. Costs
    /// one shot and ends the activation. Never struck back.
    Shoot { target: String },
    /// Step into a free neighbouring cell, for one movement point. A diagonal step needs both cells it passes
    /// between to be free as well.
    Walk { to: Cell },
    /// Land on any free cell, whatever lies between, for one movement point and one flight.
    Fly { to: Cell },
    /// Before moving: go to the end of the round's order. A stack that has waited this round passes instead.
    Wait,
    /// The player's hero casts a spell, once a round, during the turn of one of the player's stacks. It does not
    /// use up that stack's turn.
    Cast { spell: String, target: String },
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
    /// The stack has no shots (left).
    NoShots,
    /// An enemy stands in a neighbouring cell, so the stack cannot shoot.
    EnemyAdjacent,
    /// The cell is outside the board, an obstacle, taken, the stack's own, too far for a step, or a diagonal
    /// step between cells that are not both free.
    CannotEnter { to: Cell },
    /// The stack cannot fly (any more in this activation).
    NoFlights,
    /// Waiting is only allowed before the stack has moved.
    AlreadyMoved,
    /// Only the player side has a hero.
    NoHero,
    /// The hero has cast this round.
    AlreadyCast,
    /// No such spell, or no charge of it left.
    NoCharge { spell: String },
    /// The spell cannot be cast on this stack.
    WrongSpellTarget { target: String },
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
            Self::NoShots => f.write_str("no shots left"),
            Self::EnemyAdjacent => f.write_str("an enemy in a neighbouring cell prevents shooting"),
            Self::CannotEnter { to } => write!(f, "cell {},{} cannot be entered", to.x, to.y),
            Self::NoFlights => f.write_str("no flight left in this activation"),
            Self::AlreadyMoved => f.write_str("waiting is only allowed before moving"),
            Self::NoHero => f.write_str("this side has no hero to cast spells"),
            Self::AlreadyCast => f.write_str("the hero has cast this round"),
            Self::NoCharge { spell } => write!(f, "no charge of {spell:?}"),
            Self::WrongSpellTarget { target } => {
                write!(f, "the spell cannot be cast on {target:?}")
            }
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
    /// Under the reference rules the sides take turns: the enemy's part of the round begins. The player's part
    /// begins with the round.
    SideStarted {
        side: Side,
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
        /// The damage roll for one creature, or the sum of the rolls where a blow takes more than one; for a
        /// spell, its fixed damage.
        roll: u32,
        /// How many rolls `roll` is the sum of: 1, or 2 under the variant rule that takes their mean.
        rolls: u32,
        /// Attacking creatures at the moment of the blow; for a spell, 1.
        attackers: u32,
        /// What the blow takes off the target: `roll` times `attackers`, scaled as the ruleset says.
        damage: u64,
        /// Creatures the target lost.
        losses: u32,
        /// The target after the blow.
        count: u32,
        injury: u32,
        /// The blow was the target of an attack striking back.
        retaliation: bool,
        /// The attacker's morale, where the rules have morale and it applied: it has already scaled `damage`.
        morale: Option<Morale>,
    },
    /// Reference rules: losses brought the stack back within the hero's leadership.
    ControlRegained {
        stack: String,
    },
    /// The stack lost its last creature and left the board.
    StackRemoved {
        stack: String,
    },
    Moved {
        stack: String,
        from: Cell,
        to: Cell,
        flew: bool,
    },
    /// The stack went to the end of the round's order.
    Waited {
        stack: String,
    },
    SpellCast {
        spell: String,
        target: String,
    },
    /// Injury taken off a stack; `injury` is what is left.
    Healed {
        target: String,
        removed: u32,
        injury: u32,
    },
    /// The scythe fell: on top of the blow that follows, `creatures` of the target die, half of it rounded up.
    Scythed {
        attacker: String,
        target: String,
        creatures: u32,
    },
    /// A magic shot at a troop that magic does nothing to: the shot is spent and nothing else happens.
    Fizzled {
        attacker: String,
        target: String,
    },
    /// Creatures joined a stack after its blow: those it killed (absorb) or as many as the health it took
    /// restores (leech). `count` and `injury` are the stack's afterwards.
    Grew {
        stack: String,
        gained: u32,
        count: u32,
        injury: u32,
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

/// A blow as it lands: what was rolled, how many struck, what it takes off the target.
#[derive(Clone, Copy)]
struct Blow {
    roll: u32,
    rolls: u32,
    attackers: u32,
    damage: u64,
    retaliation: bool,
    morale: Option<Morale>,
    /// Creatures of the target the scythe takes on top; their health is already in `damage`.
    scythed: u32,
}

/// Whether two stacks fight each other: stacks of different sides do, and a stack out of control fights everyone.
#[must_use]
pub fn hostile(a: &Stack, b: &Stack) -> bool {
    a.id != b.id && (a.side != b.side || a.out_of_control || b.out_of_control)
}

/// Where damage rolls come from: the battle's generator in play, a fixed list in tests.
type Rolls<'a> = &'a mut dyn FnMut(&mut Rng, u32, u32) -> u32;

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
    /// Whether a diagonal step needs both cells beside it free: the ruleset's choice.
    pub diagonal_needs_clear_corners: bool,
}

impl Observation {
    /// Whether a stack standing on `from` could step to `to` with the stacks where they are now, by the ruleset's
    /// own rule. For planning a way of several steps; what the active stack may do now is in `legal`.
    #[must_use]
    pub fn step_allowed(&self, from: Cell, to: Cell) -> bool {
        step_allowed(from, to, self.diagonal_needs_clear_corners, |c| {
            self.free(c)
        })
    }

    /// On the board, not an obstacle, and no living stack on it.
    #[must_use]
    pub fn free(&self, cell: Cell) -> bool {
        cell.x < self.width
            && cell.y < self.height
            && !self.obstacles.contains(&cell)
            && !self
                .stacks
                .iter()
                .any(|s| s.stack.alive() && s.stack.cell == cell)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Battle {
    rules: Arc<Rules>,
    /// Goes up by one with every accepted command.
    revision: u64,
    /// The round being played, from 1.
    round: u32,
    rng: Rng,
    /// Under the reference rules, the side whose part of the round it is. Always the player in the prototype,
    /// where both sides share one order.
    side: Side,
    /// Player slots in rising order, then enemy slots. Dead stacks stay in the list with a count of 0.
    stacks: Vec<Stack>,
    obstacles: Vec<Cell>,
    /// Stacks still to act this round, in order.
    queue: VecDeque<String>,
    /// Stacks that waited, in the order they did.
    deferred: VecDeque<String>,
    active: Option<Activation>,
    hero_charges: BTreeMap<String, u32>,
    /// The leadership of the player's hero, where the scenario has a hero. Decides loss of control and, under the
    /// reference rules, whether morale applies.
    leadership: Option<u32>,
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
                began_with: rules.troops[troop]
                    .has(Ability::Leech)
                    .then_some(start.count),
                out_of_control: start.side == Side::Player
                    && matches!(&rules.profile, Profile::Reference(r) if !r.switches.leadership_caps_fighters)
                    && scenario
                        .leadership
                        .is_some_and(|lead| !within(lead, rules.troops[troop].hp, start.count)),
            });
        }
        stacks.sort_by_key(|s| (s.side, s.slot));
        let mut obstacles = scenario.obstacles.clone();
        obstacles.sort_by_key(|c| (c.y, c.x));
        let seed = rng_state.unwrap_or(scenario.rng_state);
        let rng = match rules.profile {
            Profile::Prototype => Rng::Prototype(Lcg32::new(seed)),
            Profile::Reference(_) => Rng::Reference(Lcg64::new(u64::from(seed))),
        };
        let mut battle = Self {
            rules,
            revision: 0,
            round: 1,
            rng,
            side: Side::Player,
            stacks,
            obstacles,
            queue: VecDeque::new(),
            deferred: VecDeque::new(),
            active: None,
            hero_charges: scenario.hero_charges.clone(),
            leadership: scenario.leadership,
            cast_this_round: false,
            outcome: None,
        };
        battle.fill_queue();
        battle.next_activation(&mut Vec::new());
        Ok(battle)
    }

    /// Whether the battle is fought under the reference rules. The rules below branch on this and nothing else.
    fn reference(&self) -> bool {
        matches!(self.rules.profile, Profile::Reference(_))
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
    pub fn rng_state(&self) -> u64 {
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

    /// The leadership of the player's hero, when the scenario has a hero.
    #[must_use]
    pub const fn leadership(&self) -> Option<u32> {
        self.leadership
    }

    /// The morale of a stack, where the rules give it one: see `docs/spec/07-RULES-REGISTER.md`. `None` in the
    /// prototype, for the enemy's stacks, without a hero, and for a stack out of control.
    #[must_use]
    pub fn morale(&self, id: &str) -> Option<Morale> {
        self.index(id)
            .filter(|&i| self.stacks[i].alive())
            .and_then(|i| self.morale_at(i))
    }

    /// The stacks that bring a stack's morale down to Low, by ID; empty when its morale is not Low. For telling
    /// the player why.
    #[must_use]
    pub fn morale_lowered_by(&self, id: &str) -> Vec<String> {
        let Some(stack) = self.index(id).filter(|&i| self.stacks[i].alive()) else {
            return Vec::new();
        };
        if self.morale_at(stack) != Some(Morale::Low) {
            return Vec::new();
        }
        let low = self
            .pairings(stack)
            .unwrap_or_default()
            .into_iter()
            .filter(|&(_, morale)| morale == Morale::Low);
        low.map(|(other, _)| self.stacks[other].id.clone())
            .collect()
    }

    /// How many creatures of a living stack would fight now: all of them, or fewer where the rules let the
    /// hero's leadership cap them.
    #[must_use]
    pub fn fighting(&self, id: &str) -> Option<u32> {
        let stack = self.index(id).filter(|&i| self.stacks[i].alive())?;
        Some(self.fighters(stack, self.stacks[stack].count))
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
            diagonal_needs_clear_corners: !self.reference(),
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

    /// What the active stack may do now, in a fixed order: melee and shots by target ID, steps and landings row by
    /// row, spells by spell then target, Wait, Pass. Empty once the battle is over. Every command built from one
    /// of these is accepted; the screen and the computer players take their choices from here, not from rules of
    /// their own.
    #[must_use]
    pub fn legal_actions(&self) -> Vec<Action> {
        let Some(active) = &self.active else {
            return Vec::new();
        };
        let Some(actor) = self.stack(&active.stack) else {
            return Vec::new();
        };
        let mut living: Vec<&Stack> = self.stacks.iter().filter(|s| s.alive()).collect();
        living.sort_by(|a, b| a.id.cmp(&b.id));
        let enemies = || living.iter().filter(|s| hostile(s, actor));
        let board = &self.rules.board;
        let cells = || (0..board.height).flat_map(|y| (0..board.width).map(move |x| Cell { x, y }));
        let mut actions = Vec::new();
        if active.movement > 0 {
            let near = |s: &&&Stack| adjacent(s.cell, actor.cell);
            actions.extend(enemies().filter(near).map(|s| Action::Melee {
                target: s.id.clone(),
            }));
            if actor.shots > 0 && enemies().filter(near).count() == 0 {
                actions.extend(enemies().map(|s| Action::Shoot {
                    target: s.id.clone(),
                }));
            }
            let corners = !self.reference();
            let step = |to: &Cell| step_allowed(actor.cell, *to, corners, |c| self.free(c));
            actions.extend(cells().filter(step).map(|to| Action::Walk { to }));
            if active.flights > 0 {
                actions.extend(
                    cells()
                        .filter(|&to| self.free(to))
                        .map(|to| Action::Fly { to }),
                );
            }
        }
        if actor.side == Side::Player && !self.cast_this_round {
            for (spell, _) in self
                .hero_charges
                .iter()
                .filter(|(_, charges)| **charges > 0)
            {
                let Some(effect) = self.rules.spell(spell).map(|s| s.effect) else {
                    continue;
                };
                let fits = |s: &&&Stack| match effect {
                    SpellEffect::Damage(_) => s.side != actor.side,
                    SpellEffect::RemoveInjury(_) => s.side == actor.side && s.injury > 0,
                };
                actions.extend(living.iter().filter(fits).map(|s| Action::Cast {
                    spell: spell.clone(),
                    target: s.id.clone(),
                }));
            }
        }
        if !active.moved {
            actions.push(Action::Wait);
        }
        actions.push(Action::Pass);
        actions
    }

    /// On the board, not an obstacle, and no living stack on it.
    #[must_use]
    pub fn free(&self, cell: Cell) -> bool {
        self.rules.board.contains(cell)
            && !self.obstacles.contains(&cell)
            && !self.stacks.iter().any(|s| s.alive() && s.cell == cell)
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
        let actor = self
            .index(&active.stack)
            .ok_or_else(|| RuleError::NotActive {
                actor: command.actor.clone(),
            })?;
        let needs_movement = |left: u32| {
            if left == 0 {
                Err(RuleError::NoMovement)
            } else {
                Ok(())
            }
        };
        match &command.action {
            Action::Pass => self.end_activation(events),
            Action::Melee { target } => {
                needs_movement(active.movement)?;
                let defender = self.enemy_of(actor, target)?;
                if !adjacent(self.stacks[actor].cell, self.stacks[defender].cell) {
                    return Err(RuleError::NotAdjacent {
                        target: target.clone(),
                    });
                }
                self.melee(actor, defender, rolls, events);
            }
            Action::Shoot { target } => {
                needs_movement(active.movement)?;
                let defender = self.enemy_of(actor, target)?;
                self.shoot(actor, defender, rolls, events)?;
            }
            Action::Walk { to } => {
                needs_movement(active.movement)?;
                if !step_allowed(self.stacks[actor].cell, *to, !self.reference(), |c| {
                    self.free(c)
                }) {
                    return Err(RuleError::CannotEnter { to: *to });
                }
                self.move_to(actor, *to, false, events);
            }
            Action::Fly { to } => {
                needs_movement(active.movement)?;
                if active.flights == 0 {
                    return Err(RuleError::NoFlights);
                }
                if !self.free(*to) {
                    return Err(RuleError::CannotEnter { to: *to });
                }
                self.move_to(actor, *to, true, events);
            }
            Action::Wait => {
                if active.moved {
                    return Err(RuleError::AlreadyMoved);
                }
                if self.stacks[actor].waited {
                    self.end_activation(events);
                } else {
                    self.stacks[actor].waited = true;
                    self.deferred.push_back(active.stack.clone());
                    events.push(Event::Waited {
                        stack: active.stack.clone(),
                    });
                    self.end_activation(events);
                }
            }
            Action::Cast { spell, target } => self.cast(actor, spell, target, events)?,
        }
        Ok(())
    }

    /// A shot at a living enemy anywhere on the board: not with an enemy beside the shooter; one shot spent; never
    /// answered; ends the activation.
    fn shoot(
        &mut self,
        actor: usize,
        defender: usize,
        rolls: Rolls<'_>,
        events: &mut Vec<Event>,
    ) -> Result<(), RuleError> {
        let shooter = &self.stacks[actor];
        let Some((lo, hi)) = self.rules.troops[shooter.troop]
            .ranged()
            .filter(|_| shooter.shots > 0)
        else {
            return Err(RuleError::NoShots);
        };
        let near = |s: &Stack| s.alive() && hostile(s, shooter) && adjacent(s.cell, shooter.cell);
        if self.stacks.iter().any(near) {
            return Err(RuleError::EnemyAdjacent);
        }
        self.stacks[actor].shots -= 1;
        let has = |stack: usize, ability| self.rules.troops[self.stacks[stack].troop].has(ability);
        if has(actor, Ability::MagicShot) && has(defender, Ability::Immune) {
            events.push(Event::Fizzled {
                attacker: self.stacks[actor].id.clone(),
                target: self.stacks[defender].id.clone(),
            });
            self.end_activation(events);
            return Ok(());
        }
        let count = self.stacks[actor].count;
        let blow = self.blow(actor, defender, count, false, (lo, hi), rolls);
        self.hurt_by(Some(actor), defender, blow, events);
        if self.outcome.is_none() {
            self.end_activation(events);
        }
        Ok(())
    }

    /// The living stack with this ID that `actor` may attack: of the other side, or out of control, or any other
    /// stack when `actor` itself is out of control.
    fn enemy_of(&self, actor: usize, target: &str) -> Result<usize, RuleError> {
        self.index(target)
            .filter(|&i| self.stacks[i].alive() && hostile(&self.stacks[i], &self.stacks[actor]))
            .ok_or_else(|| RuleError::NotAnEnemy {
                target: target.to_owned(),
            })
    }

    /// A step or a landing. A step costs a movement point. A flight costs the flight, and in the prototype a
    /// movement point as well. The activation ends when the movement points are gone, and under the reference
    /// rules also with the last flight.
    fn move_to(&mut self, actor: usize, to: Cell, flew: bool, events: &mut Vec<Event>) {
        let from = std::mem::replace(&mut self.stacks[actor].cell, to);
        events.push(Event::Moved {
            stack: self.stacks[actor].id.clone(),
            from,
            to,
            flew,
        });
        let reference = self.reference();
        let mut done = false;
        if let Some(active) = &mut self.active {
            active.moved = true;
            if flew {
                active.flights -= 1;
            }
            if !(flew && reference) {
                active.movement -= 1;
            }
            done = active.movement == 0 || (reference && flew && active.flights == 0);
        }
        if done {
            self.end_activation(events);
        }
    }

    /// The hero's spell. Everything is checked before the charge is spent, so a spell that cannot be cast costs
    /// nothing.
    fn cast(
        &mut self,
        actor: usize,
        spell: &str,
        target: &str,
        events: &mut Vec<Event>,
    ) -> Result<(), RuleError> {
        let side = self.stacks[actor].side;
        if side != Side::Player {
            return Err(RuleError::NoHero);
        }
        if self.cast_this_round {
            return Err(RuleError::AlreadyCast);
        }
        let no_charge = || RuleError::NoCharge {
            spell: spell.to_owned(),
        };
        let effect = self
            .rules
            .spell(spell)
            .map(|s| s.effect)
            .ok_or_else(no_charge)?;
        if self.hero_charges.get(spell).copied().unwrap_or(0) == 0 {
            return Err(no_charge());
        }
        let wrong = || RuleError::WrongSpellTarget {
            target: target.to_owned(),
        };
        let to = self
            .index(target)
            .filter(|&i| self.stacks[i].alive())
            .ok_or_else(wrong)?;
        let fits = match effect {
            SpellEffect::Damage(_) => self.stacks[to].side != side,
            SpellEffect::RemoveInjury(_) => {
                self.stacks[to].side == side && self.stacks[to].injury > 0
            }
        };
        if !fits {
            return Err(wrong());
        }
        if let Some(charges) = self.hero_charges.get_mut(spell) {
            *charges -= 1;
        }
        self.cast_this_round = true;
        events.push(Event::SpellCast {
            spell: spell.to_owned(),
            target: target.to_owned(),
        });
        match effect {
            SpellEffect::Damage(amount) => {
                let blow = Blow {
                    roll: amount,
                    rolls: 1,
                    attackers: 1,
                    damage: u64::from(amount),
                    retaliation: false,
                    morale: None,
                    scythed: 0,
                };
                self.hurt_by(None, to, blow, events);
            }
            SpellEffect::RemoveInjury(amount) => {
                let stack = &mut self.stacks[to];
                let removed = amount.min(stack.injury);
                stack.injury -= removed;
                events.push(Event::Healed {
                    target: target.to_owned(),
                    removed,
                    injury: stack.injury,
                });
            }
        }
        Ok(())
    }

    fn index(&self, id: &str) -> Option<usize> {
        self.stacks.iter().position(|s| s.id == id)
    }

    /// The attacker strikes; a defender that survives and has not struck back since its flag was last renewed does
    /// so once. In the prototype it answers with the creatures it has left; under the reference rules with the
    /// number it had before the blow, so creatures killed by the blow still strike; a variant may switch that back
    /// to the survivors. Then the attacker's activation ends.
    fn melee(
        &mut self,
        attacker: usize,
        defender: usize,
        rolls: Rolls<'_>,
        events: &mut Vec<Event>,
    ) {
        let before = self.stacks[defender].count;
        let attackers = self.stacks[attacker].count;
        self.strike(attacker, defender, attackers, false, rolls, events);
        if self.outcome.is_none()
            && self.stacks[defender].alive()
            && !self.stacks[defender].retaliated
        {
            self.stacks[defender].retaliated = true;
            let after = self.stacks[defender].count;
            let answering = match self.switches().answer {
                Answer::Before if self.reference() => before,
                Answer::Mean => u32::midpoint(before, after),
                Answer::Before | Answer::Survivors => after,
            };
            self.strike(defender, attacker, answering, true, rolls, events);
        }
        if self.outcome.is_none() {
            self.end_activation(events);
        }
    }

    /// One melee blow by `count` creatures.
    fn strike(
        &mut self,
        from: usize,
        to: usize,
        count: u32,
        retaliation: bool,
        rolls: Rolls<'_>,
        events: &mut Vec<Event>,
    ) {
        let troop = &self.rules.troops[self.stacks[from].troop];
        let range = (troop.melee_min, troop.melee_max);
        let blow = self.blow(from, to, count, retaliation, range, rolls);
        self.hurt_by(Some(from), to, blow, events);
    }

    /// Rolls for a blow of `count` creatures and works out what it takes off the target.
    ///
    /// The roll: one from the range, or under the variant switch two, whose mean counts. The fighters: `count`,
    /// or under the variant switch at most as many as the hero's leadership covers. Prototype: roll times
    /// fighters. Reference: that, times (attacker's skill + 5 - target's skill), over ten; then Low morale halves
    /// it and High adds half. Every division rounds down; a skill term of zero or less does no damage.
    fn blow(
        &mut self,
        from: usize,
        to: usize,
        count: u32,
        retaliation: bool,
        (lo, hi): (u32, u32),
        rolls: Rolls<'_>,
    ) -> Blow {
        // The scythe is rolled for before the blow, as the reference does it: above 89 of 100 it takes half the
        // target, rounded up, as the target stands now.
        let has_scythe = self.rules.troops[self.stacks[from].troop].has(Ability::Scythe);
        let scythed = if has_scythe && rolls(&mut self.rng, 1, 100) > 89 {
            self.stacks[to].count.div_ceil(2)
        } else {
            0
        };
        let taken_whole =
            u64::from(scythed) * u64::from(self.rules.troops[self.stacks[to].troop].hp);
        let dice = if self.switches().two_rolls { 2 } else { 1 };
        let roll = (0..dice).map(|_| rolls(&mut self.rng, lo, hi)).sum::<u32>();
        let attackers = self.fighters(from, count);
        let base = u64::from(roll) * u64::from(attackers);
        if !self.reference() {
            return Blow {
                roll,
                rolls: dice,
                attackers,
                damage: base + taken_whole,
                retaliation,
                morale: None,
                scythed,
            };
        }
        let skill = |stack: usize| self.rules.troops[self.stacks[stack].troop].skill;
        let term = (skill(from) + 5).saturating_sub(skill(to));
        // The mean of the rolls is taken here, in the one division, so nothing is rounded twice.
        let scaled = base * u64::from(term) / (10 * u64::from(dice));
        let morale = self.morale_at(from);
        let damage = match morale {
            Some(Morale::Low) => scaled / 2,
            Some(Morale::High) => scaled + scaled / 2,
            Some(Morale::Normal) | None => scaled,
        };
        Blow {
            roll,
            rolls: dice,
            attackers,
            damage: damage + taken_whole,
            retaliation,
            morale,
            scythed,
        }
    }

    /// The switches of a variant of the reference rules; all off in the reference and in the prototype.
    fn switches(&self) -> Switches {
        match &self.rules.profile {
            Profile::Reference(reference) => reference.switches,
            Profile::Prototype => Switches::default(),
        }
    }

    /// How many of `count` creatures of a stack fight. All of them, except under the variant switch that lets the
    /// hero's leadership cap them: then a stack of the player's fights with as many as the leadership covers.
    fn fighters(&self, stack: usize, count: u32) -> u32 {
        let stack = &self.stacks[stack];
        match self.leadership {
            Some(leadership)
                if self.switches().leadership_caps_fighters && stack.side == Side::Player =>
            {
                count.min(leadership / self.rules.troops[stack.troop].hp)
            }
            _ => count,
        }
    }

    /// The morale of a stack, where it has one: under the reference rules, on the side of a hero, and in control.
    ///
    /// The reference's rule, which it attributes to the original: every living stack of the side, the stack
    /// itself included, is looked up in the chart as (other, self); the results are ranked Normal, Low, High and
    /// the lowest counts. So one Normal pairing makes the stack Normal, and Low needs every pairing to be Low or
    /// High.
    ///
    /// The variant's rule, the one the army screen shows: a stack alone is High; every other living stack is
    /// looked up as (self, other); any Low gives Low, all High gives High, else Normal.
    fn morale_at(&self, stack: usize) -> Option<Morale> {
        let Profile::Reference(reference) = &self.rules.profile else {
            return None;
        };
        let found: Vec<Morale> = self
            .pairings(stack)?
            .into_iter()
            .map(|(_, morale)| morale)
            .collect();
        Some(if reference.switches.army_view_morale {
            if found.contains(&Morale::Low) {
                Morale::Low
            } else if found.iter().all(|&morale| morale == Morale::High) {
                Morale::High
            } else {
                Morale::Normal
            }
        } else {
            let rank = |morale: &Morale| match morale {
                Morale::Normal => 0,
                Morale::Low => 1,
                Morale::High => 2,
            };
            found.into_iter().min_by_key(rank).unwrap_or(Morale::High)
        })
    }

    /// The pairings a stack's morale is made of: each stack of its side the rule looks at, with what the chart
    /// gives for the pair, read the way the rule reads it. `None` where the stack has no morale.
    fn pairings(&self, stack: usize) -> Option<Vec<(usize, Morale)>> {
        let Profile::Reference(reference) = &self.rules.profile else {
            return None;
        };
        let me = &self.stacks[stack];
        if me.side != Side::Player || self.leadership.is_none() || me.out_of_control {
            return None;
        }
        let group = |s: &Stack| self.rules.troops[s.troop].morale_group;
        let mine = group(me)?;
        let army_view = reference.switches.army_view_morale;
        let side = self
            .stacks
            .iter()
            .enumerate()
            .filter(|(_, s)| s.alive() && s.side == me.side);
        let pairs = side
            .filter(|&(i, _)| !(army_view && i == stack))
            .filter_map(|(i, other)| {
                let theirs = group(other)?;
                let (row, column) = if army_view {
                    (mine, theirs)
                } else {
                    (theirs, mine)
                };
                Some((i, reference.morale_chart[row][column]))
            });
        Some(pairs.collect())
    }

    /// Brings back under control every stack whose losses have taken it within the hero's leadership. Looked at
    /// before every activation, so it holds from the next move on.
    fn renew_control(&mut self, events: &mut Vec<Event>) {
        let Some(leadership) = self.leadership else {
            return;
        };
        for stack in &mut self.stacks {
            if stack.out_of_control
                && stack.alive()
                && within(leadership, self.rules.troops[stack.troop].hp, stack.count)
            {
                stack.out_of_control = false;
                events.push(Event::ControlRegained {
                    stack: stack.id.clone(),
                });
            }
        }
    }

    /// Takes `damage` off a stack. Removes it when it dies and ends the battle when it was its side's last. `from`
    /// is `None` for the hero's spell, which is recorded as coming from "hero"; `attackers` is the number of
    /// creatures that struck, which under the reference rules can be more than the stack still has.
    fn hurt_by(&mut self, from: Option<usize>, to: usize, blow: Blow, events: &mut Vec<Event>) {
        let Blow {
            roll,
            rolls,
            attackers,
            damage,
            retaliation,
            morale,
            scythed,
        } = blow;
        let hp = self.rules.troops[self.stacks[to].troop].hp;
        let attacker = from.map_or_else(|| "hero".to_owned(), |i| self.stacks[i].id.clone());
        if scythed > 0 {
            events.push(Event::Scythed {
                attacker: attacker.clone(),
                target: self.stacks[to].id.clone(),
                creatures: scythed,
            });
        }
        let target = &mut self.stacks[to];
        let before = target.count;
        (target.count, target.injury) = health_after(hp, target.count, target.injury, damage);
        let (id, count, injury, side) =
            (target.id.clone(), target.count, target.injury, target.side);
        let losses = before - count;
        events.push(Event::Damage {
            attacker,
            target: id.clone(),
            roll,
            rolls,
            attackers,
            damage,
            losses,
            count,
            injury,
            retaliation,
            morale,
        });
        if let Some(from) = from {
            // What the blow took, counted as the reference counts it: the creatures killed, at their full
            // health, and the injury now on the one left hurt.
            let taken = u64::from(losses) * u64::from(hp) + u64::from(injury);
            self.grow_by_the_blow(from, losses, taken, events);
        }
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

    /// What a blow gives back to the stack that struck it, struck back included. Absorb: the creatures it killed
    /// join the stack, without limit. Leech: one creature for each whole health of its own in what it took; past
    /// the number it began the battle with it stops there and its injury is gone, as the reference has it.
    fn grow_by_the_blow(&mut self, from: usize, killed: u32, taken: u64, events: &mut Vec<Event>) {
        let troop = &self.rules.troops[self.stacks[from].troop];
        let (absorbs, leeches, hp) = (
            troop.has(Ability::Absorb),
            troop.has(Ability::Leech),
            troop.hp,
        );
        let stack = &mut self.stacks[from];
        if !stack.alive() {
            return;
        }
        let before = stack.count;
        if absorbs {
            stack.count = stack.count.saturating_add(killed);
        }
        if let (true, Some(most)) = (leeches, stack.began_with) {
            let regained = u32::try_from(taken / u64::from(hp)).unwrap_or(u32::MAX);
            let grown = stack.count.saturating_add(regained);
            if grown > most {
                // Never fewer than it has: a stack that also absorbed may stand above the number it began with.
                stack.count = stack.count.max(most);
                stack.injury = 0;
            } else {
                stack.count = grown;
            }
        }
        if stack.count > before {
            events.push(Event::Grew {
                stack: stack.id.clone(),
                gained: stack.count - before,
                count: stack.count,
                injury: stack.injury,
            });
        }
    }

    /// Reference rules: when a round begins, a troop that regenerates has no injury left, on either side.
    fn regenerate(&mut self, events: &mut Vec<Event>) {
        for stack in &mut self.stacks {
            let regenerates = self.rules.troops[stack.troop].has(Ability::Regeneration);
            if regenerates && stack.alive() && stack.injury > 0 {
                events.push(Event::Healed {
                    target: stack.id.clone(),
                    removed: stack.injury,
                    injury: 0,
                });
                stack.injury = 0;
            }
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

    /// Puts the living stacks that act next into the queue, in order: all of them in the prototype, those of the
    /// side whose part of the round it is under the reference rules.
    fn fill_queue(&mut self) {
        let (reference, side) = (self.reference(), self.side);
        let acts = |s: &&Stack| s.alive() && (!reference || s.side == side);
        self.queue = self
            .stacks
            .iter()
            .filter(acts)
            .map(|s| s.id.clone())
            .collect();
    }

    /// Gives the turn to the next living stack: first the queue, then those that waited. When both are empty the
    /// round is over and the next begins, unless the round limit is reached.
    fn next_activation(&mut self, events: &mut Vec<Event>) {
        loop {
            self.renew_control(events);
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
            // Nobody is left to act. Under the reference rules that ends one side's part of the round: flags are
            // renewed, and after the player's part comes the enemy's.
            if self.reference() {
                for stack in &mut self.stacks {
                    stack.retaliated = false;
                    stack.waited = false;
                }
                if self.side == Side::Player {
                    self.side = Side::Enemy;
                    self.fill_queue();
                    events.push(Event::SideStarted { side: Side::Enemy });
                    continue;
                }
            }
            if self.round >= self.rules.round_limit {
                self.finish(Outcome::BudgetExceeded, events);
                return;
            }
            self.round += 1;
            self.side = Side::Player;
            self.cast_this_round = false;
            for stack in &mut self.stacks {
                stack.retaliated = false;
                stack.waited = false;
            }
            self.fill_queue();
            events.push(Event::RoundStarted { round: self.round });
            self.regenerate(events);
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
        let reference = self.reference();
        let mut stacks: Vec<&Stack> = self.stacks.iter().collect();
        stacks.sort_by(|a, b| a.id.cmp(&b.id));
        let stacks = stacks
            .into_iter()
            .map(|s| {
                let mut stack = Value::object([
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
                ]);
                if let (true, Value::Object(members)) = (reference, &mut stack) {
                    members.insert("out_of_control".to_owned(), s.out_of_control.into());
                }
                // Written only for the troops that have one, so every other battle is encoded as before.
                if let (Some(began), Value::Object(members)) = (s.began_with, &mut stack) {
                    members.insert("began_with".to_owned(), began.into());
                }
                stack
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
        let mut value = Value::object([
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
                    ("algorithm", Value::str(self.rng.algorithm())),
                    ("state", self.rng.state().into()),
                ]),
            ),
            ("round", self.round.into()),
            ("ruleset_id", Value::str(&self.rules.id)),
            ("stacks", Value::Array(stacks)),
        ]);
        // Whose part of the round it is belongs to the state only where the sides take turns. The prototype's
        // digests stay as they were recorded.
        if let (true, Value::Object(members)) = (self.reference(), &mut value) {
            members.insert("side".to_owned(), Value::str(self.side.name()));
            members.insert(
                "leadership".to_owned(),
                self.leadership.map_or(Value::Null, Value::from),
            );
        }
        value
    }

    /// The digest of [`Battle::to_canon`]: two battles with the same digest go on in the same way.
    #[must_use]
    pub fn digest(&self) -> String {
        // Every key above is ASCII and every number a u32 or u64, so the encoding cannot fail.
        self.to_canon().digest().unwrap_or_default()
    }
}

fn saved_cell(from: &Reader<'_>) -> Result<Cell, SaveError> {
    Ok(Cell {
        x: from.u32("x")?,
        y: from.u32("y")?,
    })
}

/// The stacks of a saved battle, in the order the battle keeps them.
fn saved_stacks(
    rules: &Rules,
    state: &Reader<'_>,
    reference: bool,
) -> Result<Vec<Stack>, SaveError> {
    let mut stacks = Vec::new();
    for item in state.list("stacks")? {
        let from = Reader::of(item, "a stack")?;
        let troop_id = from.text("troop_id")?;
        let Some(troop) = rules.troops.iter().position(|troop| troop.id == troop_id) else {
            return invalid(format!("a stack is of the unknown troop {troop_id:?}"));
        };
        let Some(side) = Side::from_name(from.text("side")?) else {
            return invalid(format!("a stack is on the side {:?}", from.text("side")?));
        };
        stacks.push(Stack {
            id: from.text("id")?.to_owned(),
            side,
            slot: from.u32("slot")?,
            troop_id: troop_id.to_owned(),
            troop,
            cell: saved_cell(&from.object("cell")?)?,
            count: from.u32("count")?,
            injury: from.u32("injury")?,
            shots: from.u32("shots")?,
            retaliated: from.flag("retaliated")?,
            waited: from.flag("waited")?,
            // Only the rules that have control write the mark, and they always do.
            out_of_control: if reference {
                from.flag("out_of_control")?
            } else {
                false
            },
            // Only a troop that leeches has the number, and it always has.
            began_with: if rules.troops[troop].has(Ability::Leech) {
                Some(from.u32("began_with")?)
            } else {
                None
            },
        });
    }
    stacks.sort_by_key(|stack| (stack.side, stack.slot));
    Ok(stacks)
}

impl Battle {
    /// The battle a value written by [`Battle::to_canon`] describes, under these rules. Everything is checked:
    /// a value that names an unknown troop, puts two stacks on one cell, or gives the turn to a dead stack is
    /// refused, since the rules could not have produced it.
    ///
    /// # Errors
    /// [`SaveError::Invalid`] naming the first thing that cannot be.
    pub fn from_canon(rules: Arc<Rules>, value: &Value) -> Result<Self, SaveError> {
        let state = Reader::of(value, "the battle")?;
        if state.text("ruleset_id")? != rules.id {
            return invalid(format!(
                "the battle was fought under {:?}",
                state.text("ruleset_id")?
            ));
        }
        let reference = matches!(rules.profile, Profile::Reference(_));
        let generator = state.object("rng")?;
        let seed = generator.u64("state")?;
        let rng = match (reference, generator.text("algorithm")?, u32::try_from(seed)) {
            (false, "prototype-lcg32-v1", Ok(seed)) => Rng::Prototype(Lcg32::new(seed)),
            (true, "reference-lcg64-v1", _) => Rng::Reference(Lcg64::new(seed)),
            (_, other, _) => {
                return invalid(format!(
                    "the generator {other:?} is not the one of these rules"
                ));
            }
        };

        let stacks = saved_stacks(&rules, &state, reference)?;

        let obstacles = state.list("obstacles")?.iter();
        let obstacles = obstacles
            .map(|item| saved_cell(&Reader::of(item, "an obstacle")?))
            .collect::<Result<Vec<_>, _>>()?;
        let active = match state.opt("active") {
            None => None,
            Some(value) => {
                let from = Reader::of(value, "the activation")?;
                Some(Activation {
                    stack: from.text("stack")?.to_owned(),
                    movement: from.u32("movement")?,
                    flights: from.u32("flights")?,
                    moved: from.flag("moved")?,
                })
            }
        };
        let outcome = match state.opt("outcome") {
            None => None,
            Some(Value::Str(name)) => match Outcome::from_name(name) {
                Some(outcome) => Some(outcome),
                None => return invalid(format!("the battle ended as {name:?}")),
            },
            Some(_) => return invalid("the battle's outcome is not text"),
        };
        let side = if reference {
            Side::from_name(state.text("side")?)
        } else {
            Some(Side::Player)
        };
        let Some(side) = side else {
            return invalid("the battle has no side whose part of the round it is");
        };
        let leadership = match state.opt("leadership") {
            Some(Value::Int(n)) if reference => {
                Some(u32::try_from(*n).or_else(|_| invalid("the leadership is out of range"))?)
            }
            None => None,
            Some(_) => return invalid("these rules have no leadership, or it is not a number"),
        };
        let battle = Self {
            rules,
            revision: state.u64("revision")?,
            round: state.u32("round")?,
            rng,
            side,
            stacks,
            obstacles,
            queue: state.texts("queue")?.into(),
            deferred: state.texts("deferred")?.into(),
            active,
            hero_charges: state.numbers("hero_charges")?,
            leadership,
            cast_this_round: state.flag("cast_this_round")?,
            outcome,
        };
        battle.check()?;
        Ok(battle)
    }

    /// What must hold of any battle the rules produce. A saved battle that fails this is refused.
    fn check(&self) -> Result<(), SaveError> {
        let board = &self.rules.board;
        if self.round == 0 || self.round > self.rules.round_limit {
            return invalid(format!(
                "round {} of at most {}",
                self.round, self.rules.round_limit
            ));
        }
        for (i, stack) in self.stacks.iter().enumerate() {
            let troop = &self.rules.troops[stack.troop];
            let id = &stack.id;
            let twin = |other: &Stack| {
                other.id == *id || (other.side == stack.side && other.slot == stack.slot)
            };
            if self.stacks[..i].iter().any(twin) || stack.slot >= board.max_stacks_per_side {
                return invalid(format!(
                    "stack {id:?} shares its ID or its slot, or its slot is none"
                ));
            }
            if stack.shots > troop.shots {
                return invalid(format!(
                    "stack {id:?} has more shots than its troop starts with"
                ));
            }
            match stack.began_with {
                Some(began) if troop.has(Ability::Leech) && (began > 0 || !stack.alive()) => {}
                None if !troop.has(Ability::Leech) => {}
                _ => {
                    return invalid(format!(
                        "stack {id:?} has the number it began with, or lacks it, against its troop"
                    ));
                }
            }
            if !stack.alive() {
                if stack.injury != 0 {
                    return invalid(format!("the dead stack {id:?} is hurt"));
                }
                continue;
            }
            let shared = self.stacks[..i]
                .iter()
                .any(|other| other.alive() && other.cell == stack.cell);
            if stack.injury >= troop.hp
                || !board.contains(stack.cell)
                || self.obstacles.contains(&stack.cell)
                || shared
            {
                return invalid(format!(
                    "stack {id:?} is hurt beyond one creature, or stands where it cannot"
                ));
            }
        }
        if self.obstacles.iter().any(|&cell| !board.contains(cell)) {
            return invalid("an obstacle is off the board");
        }
        let living = |id: &String| {
            self.stacks
                .iter()
                .any(|stack| stack.id == *id && stack.alive())
        };
        if let Some(lost) = self
            .queue
            .iter()
            .chain(&self.deferred)
            .find(|id| self.stack(id).is_none())
        {
            return invalid(format!("{lost:?} waits its turn and is no stack"));
        }
        if let Some(unknown) = self
            .hero_charges
            .keys()
            .find(|spell| self.rules.spell(spell).is_none())
        {
            return invalid(format!("charges of the unknown spell {unknown:?}"));
        }
        match (&self.active, self.outcome) {
            (Some(active), None) => {
                let Some(stack) = self.stack(&active.stack).filter(|_| living(&active.stack))
                else {
                    return invalid(format!(
                        "it is the turn of {:?}, which is not alive",
                        active.stack
                    ));
                };
                let troop = self.troop(stack);
                if active.movement > troop.movement || active.flights > troop.flights_per_activation
                {
                    return invalid("the active stack has more movement or flights than its troop");
                }
                Ok(())
            }
            (None, Some(_)) => Ok(()),
            _ => invalid("a battle is either someone's turn or over, never both or neither"),
        }
    }
}

/// The rule for one step on the ground, the same for the battle and for a computer player planning a route: `to`
/// is one of the eight cells around `from` and free. With `corners` (the prototype), a diagonal step also needs the
/// two cells it passes between to be free. `free` says whether a cell can be stood on.
#[must_use]
pub fn step_allowed(from: Cell, to: Cell, corners: bool, free: impl Fn(Cell) -> bool) -> bool {
    let straight = from.x == to.x || from.y == to.y;
    adjacent(from, to)
        && free(to)
        && (straight
            || !corners
            || (free(Cell { x: to.x, y: from.y }) && free(Cell { x: from.x, y: to.y })))
}

/// Whether a stack of `count` creatures with `hp` health each is within a leadership of `leadership`: its whole
/// health is not more. Equal is within.
fn within(leadership: u32, hp: u32, count: u32) -> bool {
    u64::from(hp) * u64::from(count) <= u64::from(leadership)
}

/// In one of the eight cells around, not the same cell.
#[must_use]
pub const fn adjacent(a: Cell, b: Cell) -> bool {
    a.x.abs_diff(b.x) <= 1 && a.y.abs_diff(b.y) <= 1 && (a.x != b.x || a.y != b.y)
}

#[cfg(test)]
mod tests;
