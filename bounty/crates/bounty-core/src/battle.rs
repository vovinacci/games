//! One battle: its state, the commands that change it and the events that tell what happened.
//!
//! The only way to change a [`Battle`] is [`Battle::apply`]. A command that is refused changes nothing: not the
//! state, not the revision, not the random generator. Everything a command needs is checked before anything is
//! touched; `apply` works on a copy and keeps it only when the whole command went through.

use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::sync::Arc;

use crate::canon::Value;
use crate::content::{Cell, ContentError, Profile, Rules, Scenario, Side, SpellEffect, Troop};
use crate::rng::{Lcg32, Lcg64, Rng};

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
        /// The damage roll for one creature; for a spell, its fixed damage.
        roll: u32,
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
    attackers: u32,
    damage: u64,
    retaliation: bool,
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
        let enemies = || living.iter().filter(|s| s.side != actor.side);
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
        let near =
            |s: &Stack| s.alive() && s.side != shooter.side && adjacent(s.cell, shooter.cell);
        if self.stacks.iter().any(near) {
            return Err(RuleError::EnemyAdjacent);
        }
        self.stacks[actor].shots -= 1;
        let roll = rolls(&mut self.rng, lo, hi);
        let count = self.stacks[actor].count;
        let damage = self.damage(actor, defender, roll, count);
        let blow = Blow {
            roll,
            attackers: count,
            damage,
            retaliation: false,
        };
        self.hurt_by(Some(actor), defender, blow, events);
        if self.outcome.is_none() {
            self.end_activation(events);
        }
        Ok(())
    }

    /// The living enemy stack of `actor` with this ID.
    fn enemy_of(&self, actor: usize, target: &str) -> Result<usize, RuleError> {
        self.index(target)
            .filter(|&i| self.stacks[i].alive() && self.stacks[i].side != self.stacks[actor].side)
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
                    attackers: 1,
                    damage: u64::from(amount),
                    retaliation: false,
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
    /// number it had before the blow, so creatures killed by the blow still strike. Then the attacker's
    /// activation ends.
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
            let answering = if self.reference() {
                before
            } else {
                self.stacks[defender].count
            };
            self.strike(defender, attacker, answering, true, rolls, events);
        }
        if self.outcome.is_none() {
            self.end_activation(events);
        }
    }

    /// One melee blow by `count` creatures: one roll from the attacker's range.
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
        let (lo, hi) = (troop.melee_min, troop.melee_max);
        let roll = rolls(&mut self.rng, lo, hi);
        let damage = self.damage(from, to, roll, count);
        self.hurt_by(
            Some(from),
            to,
            Blow {
                roll,
                attackers: count,
                damage,
                retaliation,
            },
            events,
        );
    }

    /// What a blow of `count` creatures with this roll takes off the target. Prototype: roll times count.
    /// Reference: that, times (attacker's skill + 5 - target's skill), divided by ten and rounded down; a term of
    /// zero or less does no damage.
    fn damage(&self, from: usize, to: usize, roll: u32, count: u32) -> u64 {
        let base = u64::from(roll) * u64::from(count);
        if !self.reference() {
            return base;
        }
        let skill = |stack: usize| self.rules.troops[self.stacks[stack].troop].skill;
        let term = (skill(from) + 5).saturating_sub(skill(to));
        base * u64::from(term) / 10
    }

    /// Takes `damage` off a stack. Removes it when it dies and ends the battle when it was its side's last. `from`
    /// is `None` for the hero's spell, which is recorded as coming from "hero"; `attackers` is the number of
    /// creatures that struck, which under the reference rules can be more than the stack still has.
    fn hurt_by(&mut self, from: Option<usize>, to: usize, blow: Blow, events: &mut Vec<Event>) {
        let Blow {
            roll,
            attackers,
            damage,
            retaliation,
        } = blow;
        let hp = self.rules.troops[self.stacks[to].troop].hp;
        let attacker = from.map_or_else(|| "hero".to_owned(), |i| self.stacks[i].id.clone());
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

/// In one of the eight cells around, not the same cell.
#[must_use]
pub const fn adjacent(a: Cell, b: Cell) -> bool {
    a.x.abs_diff(b.x) <= 1 && a.y.abs_diff(b.y) <= 1 && (a.x != b.x || a.y != b.y)
}

#[cfg(test)]
mod tests;
