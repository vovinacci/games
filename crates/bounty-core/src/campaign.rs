//! An expedition: the hero on the map, the clock, the purse, the army, and the commands that change them.
//!
//! The same discipline as a battle. The only way to change an [`Expedition`] is [`Expedition::apply`]; a command
//! that is refused changes nothing; what happened is told as events, in order, with IDs and numbers and no text for
//! the screen. One clock owns time: every command that takes time goes through `pass`, which settles each week
//! once and stops at the deadline.

mod content;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;

pub use content::{
    ArmyStart, BandStack, BattleLayout, Clock, Encounter, ExpeditionDef, King, Map, MapObject,
    ObjectKind, Recovery, Terrain, Upkeep,
};

use crate::battle::{self, Battle, Outcome};
use crate::canon::Value;
use crate::content::{Cell, Profile, Rank, Scenario, Side, StackStart, Switches, Unpaid};
use crate::save::{Reader, SaveError, invalid};

/// How an expedition stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// The hero is on the map and may act.
    Adventure,
    /// A battle is being fought; only its commands and surrender are taken.
    Battle,
    /// The objective was found.
    Victory,
    /// The deadline came first.
    Timeout,
    /// A week's upkeep could not be paid.
    Insolvent,
}

impl Status {
    /// The name used in reports, saves and digests.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Adventure => "adventure",
            Self::Battle => "battle",
            Self::Victory => "victory",
            Self::Timeout => "timeout",
            Self::Insolvent => "insolvent",
        }
    }

    /// The status of that name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        let all = [
            Self::Adventure,
            Self::Battle,
            Self::Victory,
            Self::Timeout,
            Self::Insolvent,
        ];
        all.into_iter().find(|status| status.name() == name)
    }
}

/// A stack of the hero's army between battles. Injury is healed outside battle, so there is none to keep.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArmyStack {
    /// Stable for the whole expedition.
    pub id: String,
    pub troop_id: String,
    pub count: u32,
}

/// What the hero is asked to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// One step north, south, east or west.
    Move { dx: i32, dy: i32 },
    /// Let one tick pass.
    WaitTick,
    /// Buy creatures in the town.
    Recruit { troop_id: String, quantity: u32 },
    /// Take the contract on an encounter, in the town.
    TakeContract { id: String },
    /// Search the ground where the hero stands for the objective.
    Search,
    /// A command of the battle being fought.
    Battle(battle::Command),
    /// Give the battle up.
    Surrender,
    /// Ask the king, in his castle, for the rank the villains caught have earned.
    Promote,
}

/// A request to change the expedition. It names what should happen, never the price or the result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Command {
    /// The revision the sender saw; a command made for another state is refused.
    pub revision: u64,
    pub action: Action,
}

/// Why a command was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refused {
    /// The expedition has ended.
    Over,
    /// The command was made for another revision.
    Stale { command: u64, expedition: u64 },
    /// Not one step north, south, east or west.
    NotAStep,
    /// The step leads off the map or onto ground that cannot be walked.
    CannotEnter,
    /// A battle is being fought; the map waits.
    InBattle,
    /// No battle is being fought.
    NotInBattle,
    /// The battle refused its command.
    Battle(battle::RuleError),
    /// Only possible in the town.
    NotInTown,
    /// The town does not sell this troop.
    NotSold { troop_id: String },
    /// Nothing asked for.
    NoQuantity,
    /// More than the town has left.
    OutOfStock { stock: u32 },
    /// More than the purse holds.
    CannotAfford { cost: u64, gold: u64 },
    /// The stack's whole health would exceed the hero's leadership.
    OverLeadership { health: u64, leadership: u32 },
    /// Every slot of the army holds another troop.
    ArmyFull,
    /// No such band to take a contract on, or it is already beaten.
    NoSuchContract { id: String },
    /// Only possible in the king's castle; an expedition without a king has none.
    NotAtCastle,
    /// The king has no rank to give: `needed` villains earn the next, or there is none above the hero's.
    NoPromotion { caught: u32, needed: Option<u32> },
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Over => f.write_str("the expedition is over"),
            Self::Stale {
                command,
                expedition,
            } => {
                write!(
                    f,
                    "command is for revision {command}, the expedition is at {expedition}"
                )
            }
            Self::NotAStep => f.write_str("not one step north, south, east or west"),
            Self::CannotEnter => {
                f.write_str("the step leads off the map or onto ground that cannot be walked")
            }
            Self::InBattle => f.write_str("a battle is being fought"),
            Self::NotInBattle => f.write_str("no battle is being fought"),
            Self::Battle(refused) => write!(f, "the battle refused: {refused}"),
            Self::NotInTown => f.write_str("only possible in the town"),
            Self::NotSold { troop_id } => write!(f, "the town does not sell {troop_id:?}"),
            Self::NoQuantity => f.write_str("nothing asked for"),
            Self::OutOfStock { stock } => write!(f, "the town has only {stock} left"),
            Self::CannotAfford { cost, gold } => {
                write!(f, "that costs {cost}, the purse holds {gold}")
            }
            Self::OverLeadership { health, leadership } => {
                write!(
                    f,
                    "a stack of {health} health is more than the leadership of {leadership}"
                )
            }
            Self::ArmyFull => f.write_str("every slot of the army holds another troop"),
            Self::NoSuchContract { id } => write!(f, "no contract to take on {id:?}"),
            Self::NotAtCastle => f.write_str("only possible in the king's castle"),
            Self::NoPromotion {
                caught,
                needed: Some(needed),
            } => write!(
                f,
                "the next rank asks for {needed} villains, {caught} are caught"
            ),
            Self::NoPromotion { needed: None, .. } => f.write_str("there is no higher rank"),
        }
    }
}

impl std::error::Error for Refused {}

/// Something that happened, in the order it happened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Moved {
        from: Cell,
        to: Cell,
    },
    /// The clock went on, from one tick to another.
    TimePassed {
        from: u32,
        to: u32,
    },
    /// A week ended: income came in, then upkeep went out. `gold` is the purse afterwards.
    WeekSettled {
        week: u32,
        income: u64,
        upkeep: u64,
        gold: u64,
    },
    Recruited {
        troop_id: String,
        quantity: u32,
        cost: u64,
    },
    ContractTaken {
        id: String,
    },
    Searched {
        found: bool,
    },
    /// The hero stepped onto a band and its battle began.
    BattleStarted {
        band: String,
    },
    /// Something happened in the battle.
    Battle(battle::Event),
    /// A stack that could not be paid at the week's end left the army.
    StackLeft {
        troop_id: String,
        count: u32,
    },
    /// After a won battle some of a stack's fallen came back to it.
    FallenReturned {
        troop_id: String,
        count: u32,
    },
    /// The band is beaten; the reward is in the purse.
    BattleWon {
        band: String,
        reward: u64,
    },
    /// A clue named the place of an object.
    ClueGained {
        object: String,
    },
    /// A band was beaten while its contract was held: a villain caught. `caught` is how many there are now.
    VillainCaught {
        band: String,
        caught: u32,
    },
    /// The king gave the hero a rank, or several at once: `rank` is the one he holds now, `ranks` how many he
    /// rose, and the rest what he has with them.
    Promoted {
        rank: String,
        ranks: u32,
        leadership: u32,
        commission: u64,
    },
    /// The battle ran out of rounds with nobody winning; the hero stays where he stood and the band remains.
    BattleStalled {
        band: String,
    },
    /// A battle was lost or given up: the hero is back at the place of recovery with what recovery leaves him.
    Recovered {
        gold: u64,
    },
    Ended {
        status: Status,
    },
}

/// How a stretch of time ended.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Passed {
    /// All of it passed and the expedition goes on.
    All,
    /// The deadline was reached on its last tick: what the command was for still happens.
    AtDeadline,
    /// The deadline cut it short: what the command was for does not happen.
    CutShort,
    /// A week could not be paid.
    Insolvent,
}

/// A battle of the expedition while it is fought.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Engagement {
    battle: Battle,
    band: String,
    arrival: Cell,
}

/// Where a hero who serves a king stands with him.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Standing {
    /// Place of the hero's rank in his class.
    pub rank: usize,
    /// What the ranks reached give together.
    pub leadership: u32,
    /// Gold from the king every week, likewise.
    pub commission: u64,
    /// Bands beaten while their contract was held.
    pub villains: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expedition {
    def: Arc<ExpeditionDef>,
    /// Goes up by one with every accepted command.
    revision: u64,
    hero: Cell,
    /// Ticks elapsed since the start.
    ticks: u32,
    gold: u64,
    /// In slot order.
    army: Vec<ArmyStack>,
    /// What the town has left, by troop ID.
    stock: BTreeMap<String, u32>,
    /// The band whose contract the hero holds.
    contract: Option<String>,
    /// Objects whose place a clue has named.
    clues: BTreeSet<String>,
    /// Bands that are beaten.
    cleared: BTreeSet<String>,
    /// The battle being fought, with the band and the cell the hero arrives on if he wins.
    engagement: Option<Engagement>,
    /// Only in an expedition with a king.
    standing: Option<Standing>,
    status: Status,
}

impl Expedition {
    /// The expedition at its start.
    #[must_use]
    pub fn start(def: Arc<ExpeditionDef>) -> Self {
        let army = def.start_army.iter();
        let army = army
            .map(|s| ArmyStack {
                id: s.id.clone(),
                troop_id: s.troop_id.clone(),
                count: s.count,
            })
            .collect();
        let standing = def.king.as_ref().map(|_| Standing {
            rank: 0,
            leadership: def.leadership,
            commission: def.clock.weekly_income,
            villains: 0,
        });
        Self {
            standing,
            hero: def.start,
            gold: def.start_gold,
            stock: def.stock.clone(),
            army,
            def,
            revision: 0,
            ticks: 0,
            contract: None,
            clues: BTreeSet::new(),
            cleared: BTreeSet::new(),
            engagement: None,
            status: Status::Adventure,
        }
    }

    #[must_use]
    pub fn def(&self) -> &ExpeditionDef {
        &self.def
    }

    /// The definition as the expedition holds it, for another expedition of the same: a loaded save.
    #[must_use]
    pub const fn def_shared(&self) -> &Arc<ExpeditionDef> {
        &self.def
    }

    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    #[must_use]
    pub const fn hero(&self) -> Cell {
        self.hero
    }

    #[must_use]
    pub const fn ticks(&self) -> u32 {
        self.ticks
    }

    #[must_use]
    pub const fn gold(&self) -> u64 {
        self.gold
    }

    /// The hero's standing with the king, in an expedition that has one.
    #[must_use]
    pub const fn standing(&self) -> Option<&Standing> {
        self.standing.as_ref()
    }

    /// The hero's rank, where there is a king to give one.
    #[must_use]
    pub fn rank(&self) -> Option<&Rank> {
        let (king, standing) = (self.def.king.as_ref()?, self.standing.as_ref()?);
        self.def.rules.classes[king.class].ranks.get(standing.rank)
    }

    /// The hero's leadership now: what his ranks give, or what the expedition starts with.
    #[must_use]
    pub fn leadership(&self) -> u32 {
        self.standing
            .as_ref()
            .map_or(self.def.leadership, |s| s.leadership)
    }

    /// The gold a week brings now.
    #[must_use]
    pub fn weekly_income(&self) -> u64 {
        self.standing
            .as_ref()
            .map_or(self.def.clock.weekly_income, |s| s.commission)
    }

    #[must_use]
    pub fn army(&self) -> &[ArmyStack] {
        &self.army
    }

    /// What the town has left of a troop.
    #[must_use]
    pub fn stock(&self, troop_id: &str) -> u32 {
        self.stock.get(troop_id).copied().unwrap_or(0)
    }

    #[must_use]
    pub fn contract(&self) -> Option<&str> {
        self.contract.as_deref()
    }

    #[must_use]
    pub fn is_cleared(&self, object_id: &str) -> bool {
        self.cleared.contains(object_id)
    }

    /// The battle being fought, if one is.
    #[must_use]
    pub fn battle(&self) -> Option<&Battle> {
        self.engagement
            .as_ref()
            .map(|engagement| &engagement.battle)
    }

    /// The band being fought, if one is.
    #[must_use]
    pub fn engaged_with(&self) -> Option<&str> {
        self.engagement
            .as_ref()
            .map(|engagement| engagement.band.as_str())
    }

    #[must_use]
    pub const fn status(&self) -> Status {
        self.status
    }

    /// Where the objective lies, once a clue has named the place. Without a clue the player is not told, though a
    /// search there finds it all the same.
    #[must_use]
    pub fn objective_known(&self) -> Option<Cell> {
        self.clues
            .contains(&self.def.objective_id)
            .then(|| self.def.objective())
    }

    /// What the army as it stands costs at the end of a week.
    #[must_use]
    pub fn upkeep(&self) -> u64 {
        self.army.iter().map(|stack| self.upkeep_of(stack)).sum()
    }

    fn upkeep_of(&self, stack: &ArmyStack) -> u64 {
        let each = match self.def.clock.upkeep {
            Upkeep::PerCreature(each) => each,
            Upkeep::TenthOfCost => {
                let troop = self.def.rules.troop(&stack.troop_id);
                troop.map_or(0, |troop| u64::from(troop.recruit_cost / 10))
            }
        };
        each.saturating_mul(u64::from(stack.count))
    }

    /// The switches of the rules the expedition is played under; all off under rules that have none.
    fn switches(&self) -> Switches {
        match &self.def.rules.profile {
            Profile::Reference(reference) => reference.switches,
            Profile::Prototype => Switches::default(),
        }
    }

    /// Takes a week's upkeep from the purse and gives what was paid, or `None` when the week cannot be paid and
    /// the rules end the expedition for it. Under the reference rules a purse too small is emptied and every
    /// stack stays; under the variant's switch the stacks are paid in slot order, each in full or not at all,
    /// and a stack not paid leaves.
    fn pay_upkeep(&mut self, events: &mut Vec<Event>) -> Option<u64> {
        let due = self.upkeep();
        if self.switches().unpaid == Unpaid::Leave {
            let mut paid = 0;
            let army = std::mem::take(&mut self.army);
            for stack in army {
                let cost = self.upkeep_of(&stack);
                if cost <= self.gold {
                    self.gold -= cost;
                    paid += cost;
                    self.army.push(stack);
                } else {
                    events.push(Event::StackLeft {
                        troop_id: stack.troop_id,
                        count: stack.count,
                    });
                }
            }
            return Some(paid);
        }
        if self.gold >= due {
            self.gold -= due;
            return Some(due);
        }
        match self.def.rules.profile {
            Profile::Prototype => None,
            Profile::Reference(_) => Some(std::mem::take(&mut self.gold)),
        }
    }

    /// Every living creature of the army.
    #[must_use]
    pub fn creatures(&self) -> u64 {
        self.army.iter().map(|stack| u64::from(stack.count)).sum()
    }

    /// Why this action would be refused now, or `None` when it would be carried out. Nothing changes either way:
    /// a screen asks this to offer only what the rules allow.
    #[must_use]
    pub fn refusal(&self, action: &Action) -> Option<Refused> {
        let command = Command {
            revision: self.revision,
            action: action.clone(),
        };
        self.clone().apply(&command).err()
    }

    /// Carries out a command, or refuses it and leaves the expedition exactly as it was.
    ///
    /// # Errors
    /// [`Refused`] saying why the command is not allowed now.
    pub fn apply(&mut self, command: &Command) -> Result<Vec<Event>, Refused> {
        let mut next = self.clone();
        let mut events = Vec::new();
        next.carry_out(command, &mut events)?;
        next.revision += 1;
        *self = next;
        Ok(events)
    }

    fn carry_out(&mut self, command: &Command, events: &mut Vec<Event>) -> Result<(), Refused> {
        if !matches!(self.status, Status::Adventure | Status::Battle) {
            return Err(Refused::Over);
        }
        if command.revision != self.revision {
            return Err(Refused::Stale {
                command: command.revision,
                expedition: self.revision,
            });
        }
        let fighting = self.status == Status::Battle;
        match &command.action {
            Action::Battle(order) if fighting => self.fight(order, events),
            Action::Surrender if fighting => {
                self.recover(events);
                Ok(())
            }
            Action::Battle(_) | Action::Surrender => Err(Refused::NotInBattle),
            _ if fighting => Err(Refused::InBattle),
            Action::Move { dx, dy } => self.step(*dx, *dy, events),
            Action::WaitTick => {
                self.pass(1, events);
                self.end_if_due(events);
                Ok(())
            }
            Action::Recruit { troop_id, quantity } => self.recruit(troop_id, *quantity, events),
            Action::TakeContract { id } => self.take_contract(id, events),
            Action::Search => {
                self.search(events);
                Ok(())
            }
            Action::Promote => self.promote(events),
        }
    }

    /// The king's audience: every rank the villains caught have earned is given at once, each adding its
    /// leadership and commission to what the hero has. It takes no time.
    fn promote(&mut self, events: &mut Vec<Event>) -> Result<(), Refused> {
        let def = Arc::clone(&self.def);
        let (Some(king), Some(standing)) = (&def.king, &mut self.standing) else {
            return Err(Refused::NotAtCastle);
        };
        if self.hero != king.castle {
            return Err(Refused::NotAtCastle);
        }
        let ranks = &def.rules.classes[king.class].ranks;
        let from = standing.rank;
        while let Some(next) = ranks.get(standing.rank + 1)
            && next.villains_needed <= standing.villains
        {
            standing.rank += 1;
            standing.leadership = standing.leadership.saturating_add(next.leadership);
            standing.commission = standing
                .commission
                .saturating_add(u64::from(next.commission));
        }
        if standing.rank == from {
            return Err(Refused::NoPromotion {
                caught: standing.villains,
                needed: ranks.get(from + 1).map(|next| next.villains_needed),
            });
        }
        events.push(Event::Promoted {
            rank: ranks[standing.rank].id.clone(),
            ranks: u32::try_from(standing.rank - from).unwrap_or(u32::MAX),
            leadership: standing.leadership,
            commission: standing.commission,
        });
        Ok(())
    }

    fn step(&mut self, dx: i32, dy: i32, events: &mut Vec<Event>) -> Result<(), Refused> {
        if dx.abs() + dy.abs() != 1 {
            return Err(Refused::NotAStep);
        }
        let to = self
            .hero
            .x
            .checked_add_signed(dx)
            .zip(self.hero.y.checked_add_signed(dy));
        let Some(to) = to
            .map(|(x, y)| Cell { x, y })
            .filter(|&to| self.def.map.passable(to))
        else {
            return Err(Refused::CannotEnter);
        };
        let band = self
            .def
            .map
            .object_at(to)
            .filter(|o| o.kind == ObjectKind::Encounter && !self.is_cleared(&o.id));
        let band = band.map(|object| object.id.clone());
        // The step takes its time whatever stands there, and happens unless the deadline cuts that time short.
        let passed = self.pass(self.def.clock.movement_ticks, events);
        match band {
            // A band bars the way: the step opens its battle, and the cell is where the hero arrives if he wins.
            // With time up at the same tick there is no battle.
            Some(band) if passed == Passed::All => self.engage(band, to, events),
            None if matches!(passed, Passed::All | Passed::AtDeadline) => {
                let from = std::mem::replace(&mut self.hero, to);
                events.push(Event::Moved { from, to });
            }
            _ => {}
        }
        self.end_if_due(events);
        Ok(())
    }

    /// Sets up the battle against a band: the army down the player's column of the layout, in slot order, the
    /// band on the enemy's cell.
    fn engage(&mut self, band: String, arrival: Cell, events: &mut Vec<Event>) {
        let def = Arc::clone(&self.def);
        let Some(encounter) = def.encounter(&band) else {
            return;
        };
        let layout = &def.layout;
        let height = def.rules.board.height;
        let mut stacks: Vec<StackStart> = self
            .army
            .iter()
            .zip(0..)
            .map(|(stack, slot)| StackStart {
                id: stack.id.clone(),
                side: Side::Player,
                slot,
                troop_id: stack.troop_id.clone(),
                count: stack.count,
                injury: 0,
                x: layout.player.x,
                y: (layout.player.y + slot) % height,
            })
            .collect();
        stacks.push(StackStart {
            id: "e0".to_owned(),
            side: Side::Enemy,
            slot: 0,
            troop_id: encounter.enemy_troop.clone(),
            count: encounter.enemy_count,
            injury: 0,
            x: layout.enemy.x,
            y: layout.enemy.y,
        });
        for (stack, slot) in encounter.also.iter().zip(1..) {
            stacks.push(StackStart {
                id: format!("e{slot}"),
                side: Side::Enemy,
                slot,
                troop_id: stack.troop.clone(),
                count: stack.count,
                injury: 0,
                x: layout.enemy.x,
                y: (layout.enemy.y + slot) % height,
            });
        }
        let scenario = Scenario {
            id: band.clone(),
            rng_state: encounter.battle_rng_state,
            obstacles: layout.obstacles.clone(),
            hero_charges: BTreeMap::new(),
            // Only the rules that know leadership are told the hero's.
            leadership: matches!(def.rules.profile, Profile::Reference(_))
                .then_some(self.leadership()),
            stacks,
        };
        // The loader has checked the layout and every troop, so the battle starts; if it did not, the hero would
        // simply stand before the band, a step's time spent.
        if let Ok(battle) = Battle::start(Arc::clone(&def.rules), &scenario, None) {
            events.push(Event::BattleStarted { band: band.clone() });
            self.engagement = Some(Engagement {
                battle,
                band,
                arrival,
            });
            self.status = Status::Battle;
        }
    }

    /// Passes a command to the battle and, when that ends it, writes the result back, once.
    fn fight(&mut self, order: &battle::Command, events: &mut Vec<Event>) -> Result<(), Refused> {
        let Some(engagement) = &mut self.engagement else {
            return Err(Refused::NotInBattle);
        };
        let happened = engagement.battle.apply(order).map_err(Refused::Battle)?;
        events.extend(happened.into_iter().map(Event::Battle));
        match engagement.battle.outcome() {
            None => {}
            Some(Outcome::PlayerWin) => self.win(events),
            Some(Outcome::EnemyWin) => self.recover(events),
            Some(Outcome::BudgetExceeded) => {
                // Nobody won. The army is what the battle left of it; the band stands as before.
                self.keep_survivors(0, events);
                if let Some(engagement) = self.engagement.take() {
                    events.push(Event::BattleStalled {
                        band: engagement.band,
                    });
                }
                self.status = Status::Adventure;
            }
        }
        Ok(())
    }

    /// The army as the battle left it: each stack with its survivors and, where the rules have it so, `returning`
    /// in a hundred of its fallen; the stacks left empty gone. Injury is healed.
    fn keep_survivors(&mut self, returning: u32, events: &mut Vec<Event>) {
        let Some(engagement) = &self.engagement else {
            return;
        };
        for stack in &mut self.army {
            let fought = engagement.battle.stack(&stack.id);
            // More than went in, for a stack that grew by what it killed: it keeps them.
            let survivors = fought.map_or(0, |fought| fought.count);
            let fallen = u64::from(stack.count.saturating_sub(survivors));
            // At most the fallen, so it fits.
            let back = u32::try_from(fallen * u64::from(returning) / 100).unwrap_or(0);
            stack.count = survivors + back;
            if back > 0 {
                events.push(Event::FallenReturned {
                    troop_id: stack.troop_id.clone(),
                    count: back,
                });
            }
        }
        self.army.retain(|stack| stack.count > 0);
    }

    /// The band is beaten: survivors kept, the hero on the band's cell, the reward paid, and the clue given if
    /// the hero held the band's contract. Battles take no time of the campaign.
    fn win(&mut self, events: &mut Vec<Event>) {
        self.keep_survivors(self.switches().fallen_return_percent, events);
        let Some(Engagement { band, arrival, .. }) = self.engagement.take() else {
            return;
        };
        self.status = Status::Adventure;
        let def = Arc::clone(&self.def);
        let Some(encounter) = def.encounter(&band) else {
            return;
        };
        self.gold = self.gold.saturating_add(encounter.gold_reward);
        self.cleared.insert(band.clone());
        events.push(Event::BattleWon {
            band: band.clone(),
            reward: encounter.gold_reward,
        });
        if self.contract.as_deref() == Some(band.as_str()) {
            self.contract = None;
            if let Some(standing) = &mut self.standing {
                standing.villains += 1;
                events.push(Event::VillainCaught {
                    band: band.clone(),
                    caught: standing.villains,
                });
            }
            if encounter.grants_clue && self.clues.insert(def.objective_id.clone()) {
                events.push(Event::ClueGained {
                    object: def.objective_id.clone(),
                });
            }
        }
        let from = std::mem::replace(&mut self.hero, arrival);
        events.push(Event::Moved { from, to: arrival });
    }

    /// A battle lost or given up: the hero is taken to the place of recovery with half his gold and the troops
    /// recovery gives; the band stands as before; contract, clues and beaten bands are kept. Then recovery takes
    /// its time, with the weeks and the deadline as for any other time.
    fn recover(&mut self, events: &mut Vec<Event>) {
        self.engagement = None;
        self.status = Status::Adventure;
        let recovery = self.def.recovery.clone();
        self.hero = recovery.destination;
        self.gold /= 2;
        self.army = vec![ArmyStack {
            id: "p0".to_owned(),
            troop_id: recovery.troop_id,
            count: recovery.count,
        }];
        events.push(Event::Recovered { gold: self.gold });
        self.pass(recovery.ticks, events);
        self.end_if_due(events);
    }

    fn recruit(
        &mut self,
        troop_id: &str,
        quantity: u32,
        events: &mut Vec<Event>,
    ) -> Result<(), Refused> {
        if self.hero != self.def.town {
            return Err(Refused::NotInTown);
        }
        let (Some(&price), Some(troop)) = (
            self.def.prices.get(troop_id),
            self.def.rules.troop(troop_id),
        ) else {
            return Err(Refused::NotSold {
                troop_id: troop_id.to_owned(),
            });
        };
        if quantity == 0 {
            return Err(Refused::NoQuantity);
        }
        let stock = self.stock(troop_id);
        if quantity > stock {
            return Err(Refused::OutOfStock { stock });
        }
        // Exactly what the purse holds is enough.
        let cost = price.saturating_mul(u64::from(quantity));
        if cost > self.gold {
            return Err(Refused::CannotAfford {
                cost,
                gold: self.gold,
            });
        }
        let slot = self
            .army
            .iter()
            .position(|stack| stack.troop_id == troop_id);
        let already = slot.map_or(0, |i| self.army[i].count);
        // The whole stack, old and new, must be within the leadership; exactly the leadership is within it.
        let health = (u64::from(already) + u64::from(quantity)) * u64::from(troop.hp);
        if health > u64::from(self.leadership()) {
            return Err(Refused::OverLeadership {
                health,
                leadership: self.leadership(),
            });
        }
        if slot.is_none() && self.army.len() >= self.def.rules.board.max_stacks_per_side as usize {
            return Err(Refused::ArmyFull);
        }

        self.gold -= cost;
        self.stock.insert(troop_id.to_owned(), stock - quantity);
        if let Some(i) = slot {
            self.army[i].count += quantity;
        } else {
            // The lowest "p<n>" no stack has: among as many names as there are stacks plus one, one is free.
            let free = (0..=self.army.len())
                .map(|n| format!("p{n}"))
                .find(|id| self.army.iter().all(|s| s.id != *id));
            self.army.push(ArmyStack {
                id: free.unwrap_or_default(),
                troop_id: troop_id.to_owned(),
                count: quantity,
            });
        }
        events.push(Event::Recruited {
            troop_id: troop_id.to_owned(),
            quantity,
            cost,
        });
        Ok(())
    }

    fn take_contract(&mut self, id: &str, events: &mut Vec<Event>) -> Result<(), Refused> {
        if self.hero != self.def.town {
            return Err(Refused::NotInTown);
        }
        if self.def.encounter(id).is_none() || self.is_cleared(id) {
            return Err(Refused::NoSuchContract { id: id.to_owned() });
        }
        self.contract = Some(id.to_owned());
        events.push(Event::ContractTaken { id: id.to_owned() });
        Ok(())
    }

    /// A search takes its time whatever it finds. It finds the objective only if all of that time passes; a
    /// search that ends exactly on the deadline still counts.
    fn search(&mut self, events: &mut Vec<Event>) {
        let passed = self.pass(self.def.clock.search_ticks, events);
        if matches!(passed, Passed::All | Passed::AtDeadline) {
            let found = self.hero == self.def.objective();
            events.push(Event::Searched { found });
            if found {
                self.end(Status::Victory, events);
            }
        }
        self.end_if_due(events);
    }

    /// Lets `ticks` pass, one by one: each week that ends is settled once, income first and then upkeep, and the
    /// clock stops at the deadline. Settlement comes before anything else that falls on the same tick.
    fn pass(&mut self, ticks: u32, events: &mut Vec<Event>) -> Passed {
        let clock = &self.def.clock;
        let (week_ticks, deadline) = (clock.week_ticks, clock.deadline_ticks);
        let income = self.weekly_income();
        let from = self.ticks;
        let mut passed = Passed::All;
        for done in 1..=ticks {
            self.ticks += 1;
            if self.ticks.is_multiple_of(week_ticks) {
                self.gold = self.gold.saturating_add(income);
                let Some(upkeep) = self.pay_upkeep(events) else {
                    passed = Passed::Insolvent;
                    break;
                };
                events.push(Event::WeekSettled {
                    week: self.ticks / week_ticks,
                    income,
                    upkeep,
                    gold: self.gold,
                });
            }
            if self.ticks >= deadline {
                passed = if done == ticks {
                    Passed::AtDeadline
                } else {
                    Passed::CutShort
                };
                break;
            }
        }
        events.push(Event::TimePassed {
            from,
            to: self.ticks,
        });
        if passed == Passed::Insolvent {
            self.end(Status::Insolvent, events);
        }
        passed
    }

    /// Ends the expedition as timed out when the deadline has come and nothing else has ended it.
    fn end_if_due(&mut self, events: &mut Vec<Event>) {
        if self.status == Status::Adventure && self.ticks >= self.def.clock.deadline_ticks {
            self.end(Status::Timeout, events);
        }
    }

    fn end(&mut self, status: Status, events: &mut Vec<Event>) {
        self.status = status;
        events.push(Event::Ended { status });
    }

    /// The whole state that decides what happens next, as a value with one fixed encoding.
    #[must_use]
    pub fn to_canon(&self) -> Value {
        let ids =
            |set: &BTreeSet<String>| Value::Array(set.iter().map(|id| Value::str(id)).collect());
        let army = self.army.iter().map(|stack| {
            Value::object([
                ("count", stack.count.into()),
                ("id", Value::str(&stack.id)),
                ("troop_id", Value::str(&stack.troop_id)),
            ])
        });
        let stock = self
            .stock
            .iter()
            .map(|(troop, left)| (troop.clone(), Value::from(*left)))
            .collect();
        let mut value = Value::object([
            ("army", Value::Array(army.collect())),
            ("cleared", ids(&self.cleared)),
            ("clues", ids(&self.clues)),
            (
                "contract",
                self.contract.as_deref().map_or(Value::Null, Value::str),
            ),
            ("expedition_id", Value::str(&self.def.id)),
            ("gold", self.gold.into()),
            (
                "hero",
                Value::object([("x", self.hero.x.into()), ("y", self.hero.y.into())]),
            ),
            ("revision", self.revision.into()),
            ("status", Value::str(self.status.name())),
            ("stock", Value::Object(stock)),
            ("ticks", self.ticks.into()),
        ]);
        // Written only where there is a king, so every other expedition is encoded as before.
        if let (Some(standing), Value::Object(members)) = (&self.standing, &mut value) {
            members.insert(
                "standing".to_owned(),
                Value::object([
                    ("commission", standing.commission.into()),
                    ("leadership", standing.leadership.into()),
                    ("rank", (standing.rank as u64).into()),
                    ("villains", standing.villains.into()),
                ]),
            );
        }
        // The battle being fought is part of the state, with where it leads.
        if let (Some(engagement), Value::Object(members)) = (&self.engagement, &mut value) {
            members.insert("battle".to_owned(), engagement.battle.to_canon());
            members.insert("band".to_owned(), Value::str(&engagement.band));
            let arrival = engagement.arrival;
            members.insert(
                "arrival".to_owned(),
                Value::object([("x", arrival.x.into()), ("y", arrival.y.into())]),
            );
        }
        value
    }

    /// The expedition of `def` that a value written by [`Expedition::to_canon`] describes, with its battle given
    /// apart, as a save keeps it. A state the rules could not have reached is refused.
    ///
    /// # Errors
    /// [`SaveError::Invalid`] naming the first thing that cannot be.
    pub fn from_canon(
        def: Arc<ExpeditionDef>,
        value: &Value,
        battle: Option<&Value>,
    ) -> Result<Self, SaveError> {
        let state = Reader::of(value, "the campaign")?;
        if state.text("expedition_id")? != def.id {
            return invalid(format!(
                "it is the expedition {:?}",
                state.text("expedition_id")?
            ));
        }
        let cell = |key| -> Result<Cell, SaveError> {
            let from = state.object(key)?;
            Ok(Cell {
                x: from.u32("x")?,
                y: from.u32("y")?,
            })
        };
        let Some(status) = Status::from_name(state.text("status")?) else {
            return invalid(format!(
                "the expedition stands as {:?}",
                state.text("status")?
            ));
        };
        let mut army = Vec::new();
        for item in state.list("army")? {
            let from = Reader::of(item, "a stack of the army")?;
            army.push(ArmyStack {
                id: from.text("id")?.to_owned(),
                troop_id: from.text("troop_id")?.to_owned(),
                count: from.u32("count")?,
            });
        }
        let engagement = match battle {
            None => None,
            Some(battle) => Some(Engagement {
                battle: Battle::from_canon(Arc::clone(&def.rules), battle)?,
                band: state.text("band")?.to_owned(),
                arrival: cell("arrival")?,
            }),
        };
        // Only an expedition with a king has a standing, and it always has.
        let standing = if def.king.is_some() {
            let from = state.object("standing")?;
            Some(Standing {
                rank: usize::try_from(from.u64("rank")?).unwrap_or(usize::MAX),
                leadership: from.u32("leadership")?,
                commission: from.u64("commission")?,
                villains: from.u32("villains")?,
            })
        } else {
            None
        };
        let expedition = Self {
            standing,
            revision: state.u64("revision")?,
            hero: cell("hero")?,
            ticks: state.u32("ticks")?,
            gold: state.u64("gold")?,
            army,
            stock: state.numbers("stock")?,
            contract: match state.opt("contract") {
                None => None,
                Some(Value::Str(band)) => Some(band.clone()),
                Some(_) => return invalid("the contract is not text"),
            },
            clues: state.texts("clues")?.into_iter().collect(),
            cleared: state.texts("cleared")?.into_iter().collect(),
            engagement,
            status,
            def,
        };
        expedition.check()?;
        Ok(expedition)
    }

    /// What must hold of any expedition the rules produce. A saved one that fails this is refused.
    fn check(&self) -> Result<(), SaveError> {
        let def = &self.def;
        if !def.map.passable(self.hero) {
            return invalid("the hero stands where nobody can");
        }
        if self.ticks > def.clock.deadline_ticks {
            return invalid("the clock is past the deadline");
        }
        for (i, stack) in self.army.iter().enumerate() {
            let known = def
                .rules
                .troops
                .iter()
                .any(|troop| troop.id == stack.troop_id);
            if !known || stack.count == 0 || self.army[..i].iter().any(|other| other.id == stack.id)
            {
                return invalid(format!(
                    "the army's stack {:?} is empty, a twin, or of no troop",
                    stack.id
                ));
            }
        }
        if self.army.len() > def.rules.board.max_stacks_per_side as usize {
            return invalid("the army has more stacks than a side may field");
        }
        let stocked =
            |(troop, left): (&String, &u32)| def.stock.get(troop).is_some_and(|all| left <= all);
        if self.stock.len() != def.stock.len() || !self.stock.iter().all(stocked) {
            return invalid("the town's stock is not of this expedition");
        }
        let band = |id: &String| def.encounter(id).is_some();
        if !self.cleared.iter().all(band) || !self.contract.iter().all(band) {
            return invalid("a beaten band or the contract names no band");
        }
        self.check_standing()?;
        if let Some(unknown) = self.clues.iter().find(|id| def.map.object(id).is_none()) {
            return invalid(format!("a clue names {unknown:?}, which is not on the map"));
        }
        match (&self.engagement, self.status) {
            (Some(engagement), Status::Battle) => {
                if !band(&engagement.band) || self.cleared.contains(&engagement.band) {
                    return invalid("the battle is against no band, or one already beaten");
                }
                if !def.map.passable(engagement.arrival) {
                    return invalid("the battle leads where nobody can stand");
                }
                if engagement.battle.outcome().is_some() {
                    return invalid("the battle is over and its result not written back");
                }
                Ok(())
            }
            (None, Status::Battle) | (Some(_), _) => {
                invalid("an expedition is in battle exactly when it has one")
            }
            (None, _) => Ok(()),
        }
    }

    /// A standing is what its rank makes it: the leadership and commission of the ranks reached, and at least
    /// the villains the rank asks for, none of them more than the bands beaten.
    fn check_standing(&self) -> Result<(), SaveError> {
        let (Some(king), Some(standing)) = (&self.def.king, &self.standing) else {
            return if self.def.king.is_none() && self.standing.is_none() {
                Ok(())
            } else {
                invalid("a hero stands with a king exactly when the expedition has one")
            };
        };
        let ranks = &self.def.rules.classes[king.class].ranks;
        let Some(reached) = ranks.get(..=standing.rank) else {
            return invalid("the hero holds a rank his class does not have");
        };
        let leadership: u64 = reached.iter().map(|r| u64::from(r.leadership)).sum();
        let commission: u64 = reached.iter().map(|r| u64::from(r.commission)).sum();
        let earned = reached
            .last()
            .is_some_and(|r| r.villains_needed <= standing.villains);
        if u64::from(standing.leadership) != leadership
            || standing.commission != commission
            || !earned
            || standing.villains as usize > self.cleared.len()
        {
            return invalid("the hero's standing is not what his rank and his villains make it");
        }
        Ok(())
    }

    /// The digest of [`Expedition::to_canon`].
    #[must_use]
    pub fn digest(&self) -> String {
        // Every key above is ASCII and every number unsigned, so the encoding cannot fail.
        self.to_canon().digest().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests;
