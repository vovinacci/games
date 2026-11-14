//! Game data as read from files: the ruleset (troops, spells, board) and battle scenarios. Everything is checked
//! when it is read, so the rules never meet a troop that does not exist or a stack outside the board. Runtime state
//! lives in `battle`; nothing here changes during play.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::Deserialize;

use crate::canon::Value;

mod reference;

pub use reference::{Answer, Morale, Reference, Switches, Unpaid};

/// The synthetic ruleset the engine was built on: invented troops, for testing the engine.
pub const RULESET: &str = "prototype-v0.1";

/// The first rules taken from a reimplementation of the original game, not yet checked against the original.
pub const REFERENCE_RULESET: &str = "reference-v0.1";

/// A variant of the reference rules: the same troops and battles, with the rule changes the study of the
/// original's gaps proposes switched on. Never a claim about the original.
pub const MODERN_RULESET: &str = "modern-v0.1";

/// Which family of rules a ruleset belongs to. The rules of a battle branch on this and on nothing else, so a
/// rules file cannot mix the two.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Profile {
    Prototype,
    Reference(Reference),
}

/// Boards larger than this are refused before any memory is set aside for them.
const MAX_BOARD_SIDE: u32 = 16;

/// Why content was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContentError {
    /// Not JSON, a float, a repeated key, a missing or unknown field, or a value of the wrong type.
    Malformed(String),
    /// Well formed, but asks for a rule this engine does not implement.
    Unsupported(String),
    /// Well formed, but the values contradict each other or the ruleset.
    Invalid(String),
}

impl fmt::Display for ContentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed(why) => write!(f, "malformed content: {why}"),
            Self::Unsupported(why) => write!(f, "unsupported content: {why}"),
            Self::Invalid(why) => write!(f, "invalid content: {why}"),
        }
    }
}

impl std::error::Error for ContentError {}

fn invalid<T>(why: String) -> Result<T, ContentError> {
    Err(ContentError::Invalid(why))
}

/// Reads a file into `T`. The strict reader goes first, so a float or a repeated key is caught even where `T` would
/// have ignored it.
fn read<T: for<'de> Deserialize<'de>>(text: &str) -> Result<T, ContentError> {
    Value::parse(text).map_err(|e| ContentError::Malformed(e.to_string()))?;
    serde_json::from_str(text).map_err(|e| ContentError::Malformed(e.to_string()))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Player,
    Enemy,
}

impl Side {
    #[must_use]
    pub const fn other(self) -> Self {
        match self {
            Self::Player => Self::Enemy,
            Self::Enemy => Self::Player,
        }
    }

    /// The name used in files and digests.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Player => "player",
            Self::Enemy => "enemy",
        }
    }

    /// The side of that name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        [Self::Player, Self::Enemy]
            .into_iter()
            .find(|side| side.name() == name)
    }
}

/// A board cell: `x` to the right, `y` down, from the top-left corner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cell {
    pub x: u32,
    pub y: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Board {
    pub width: u32,
    pub height: u32,
    pub max_stacks_per_side: u32,
}

impl Board {
    #[must_use]
    pub const fn contains(&self, cell: Cell) -> bool {
        cell.x < self.width && cell.y < self.height
    }
}

/// What a troop is; how many there are and how hurt they are is in `battle::Stack`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Troop {
    pub id: String,
    pub name: String,
    /// Health of one creature.
    pub hp: u32,
    /// Movement points at the start of an activation.
    pub movement: u32,
    pub melee_min: u32,
    pub melee_max: u32,
    #[serde(default)]
    pub ranged_min: Option<u32>,
    #[serde(default)]
    pub ranged_max: Option<u32>,
    /// Shots for the whole battle; they never come back.
    pub shots: u32,
    pub flights_per_activation: u32,
    pub recruit_cost: u32,
    /// Skill level, which scales damage under the reference rules. 0 in the prototype, which has no skill; a
    /// prototype file cannot set it.
    #[serde(skip)]
    pub skill: u32,
    /// Place of the troop's morale group in the morale chart (0 is A). `None` in the prototype.
    #[serde(skip)]
    pub morale_group: Option<usize>,
}

impl Troop {
    /// The damage range of a shot, for a troop that can shoot at all.
    #[must_use]
    pub fn ranged(&self) -> Option<(u32, u32)> {
        self.ranged_min.zip(self.ranged_max)
    }
}

/// What a hero spell does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellEffect {
    /// Damage to one living enemy stack.
    Damage(u32),
    /// Removes up to this much injury from one living friendly stack that has some.
    RemoveInjury(u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spell {
    pub id: String,
    pub effect: SpellEffect,
}

/// A checked ruleset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rules {
    pub id: String,
    /// The ruleset whose battle files these rules play: their own, or for a variant the reference's.
    pub battles_id: String,
    pub profile: Profile,
    pub board: Board,
    /// A battle with no winner after this many completed rounds ends as "budget exceeded".
    pub round_limit: u32,
    /// In file order; `troop` looks one up by ID, so the order carries no meaning.
    pub troops: Vec<Troop>,
    pub spells: Vec<Spell>,
}

mod raw {
    //! The rules file as written. Every switch in it is read and must name the one behaviour implemented.

    use serde::Deserialize;

    use super::{Board, Troop};

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Rules {
        pub schema_version: u32,
        pub id: String,
        #[serde(rename = "status")]
        pub _status: String,
        pub board: Board,
        pub rng: Rng,
        pub activation_order: String,
        pub movement: Movement,
        pub damage: Damage,
        pub round_limit: u32,
        pub round_limit_outcome: String,
        pub hero_magic: HeroMagic,
        pub troops: Vec<Troop>,
        pub spells: Vec<Spell>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Rng {
        pub algorithm: String,
        pub multiplier: u64,
        pub increment: u64,
        pub bits: u32,
        pub sampling: String,
        pub constant_range_consumes_draw: bool,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Movement {
        pub neighbours: u32,
        pub step_cost: u32,
        pub diagonal_requires_clear_orthogonals: bool,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Damage {
        pub method: String,
        pub retaliation_uses_survivors: bool,
        pub injury_persists_during_battle: bool,
        pub injury_heals_after_battle: bool,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct HeroMagic {
        pub player_enabled: bool,
        pub enemy_enabled: bool,
        pub casts_per_round: u32,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Spell {
        pub id: String,
        pub target: String,
        pub effect: String,
        pub amount: u32,
        #[serde(default)]
        pub resurrects: Option<bool>,
    }
}

/// Checks that a switch in the rules file names the behaviour this engine has.
fn only<T: PartialEq + fmt::Debug>(
    what: &str,
    found: &T,
    implemented: &T,
) -> Result<(), ContentError> {
    if found == implemented {
        Ok(())
    } else {
        Err(ContentError::Unsupported(format!(
            "{what} is {found:?}; only {implemented:?} is implemented"
        )))
    }
}

fn unique<'a>(what: &str, ids: impl Iterator<Item = &'a str>) -> Result<(), ContentError> {
    let mut seen = BTreeSet::new();
    for id in ids {
        if !seen.insert(id) {
            return invalid(format!("{what} {id:?} appears twice"));
        }
    }
    Ok(())
}

/// Every switch in the rules file must name the behaviour this engine has.
fn check_switches(raw: &raw::Rules) -> Result<(), ContentError> {
    only("schema_version", &raw.schema_version, &1)?;
    only("id", &raw.id.as_str(), &RULESET)?;
    only(
        "rng.algorithm",
        &raw.rng.algorithm.as_str(),
        &"prototype-lcg32-v1",
    )?;
    only("rng.multiplier", &raw.rng.multiplier, &1_664_525)?;
    only("rng.increment", &raw.rng.increment, &1_013_904_223)?;
    only("rng.bits", &raw.rng.bits, &32)?;
    only(
        "rng.sampling",
        &raw.rng.sampling.as_str(),
        &"inclusive_rejection",
    )?;
    only(
        "rng.constant_range_consumes_draw",
        &raw.rng.constant_range_consumes_draw,
        &false,
    )?;
    only(
        "activation_order",
        &raw.activation_order.as_str(),
        &"player_slots_then_enemy_slots_then_deferred_insertion_order",
    )?;
    only("movement.neighbours", &raw.movement.neighbours, &8)?;
    only("movement.step_cost", &raw.movement.step_cost, &1)?;
    only(
        "movement.diagonal_requires_clear_orthogonals",
        &raw.movement.diagonal_requires_clear_orthogonals,
        &true,
    )?;
    only(
        "damage.method",
        &raw.damage.method.as_str(),
        &"one_inclusive_roll_times_current_count",
    )?;
    only(
        "damage.retaliation_uses_survivors",
        &raw.damage.retaliation_uses_survivors,
        &true,
    )?;
    only(
        "damage.injury_persists_during_battle",
        &raw.damage.injury_persists_during_battle,
        &true,
    )?;
    only(
        "damage.injury_heals_after_battle",
        &raw.damage.injury_heals_after_battle,
        &true,
    )?;
    only(
        "round_limit_outcome",
        &raw.round_limit_outcome.as_str(),
        &"budget_exceeded",
    )?;
    only(
        "hero_magic.player_enabled",
        &raw.hero_magic.player_enabled,
        &true,
    )?;
    only(
        "hero_magic.enemy_enabled",
        &raw.hero_magic.enemy_enabled,
        &false,
    )?;
    only(
        "hero_magic.casts_per_round",
        &raw.hero_magic.casts_per_round,
        &1,
    )?;
    Ok(())
}

fn check_troops(troops: &[Troop]) -> Result<(), ContentError> {
    unique("troop", troops.iter().map(|t| t.id.as_str()))?;
    for troop in troops {
        let id = &troop.id;
        if troop.hp == 0 {
            return invalid(format!("troop {id:?} has no health"));
        }
        if troop.melee_min > troop.melee_max {
            return invalid(format!("troop {id:?} has melee_min above melee_max"));
        }
        match (troop.ranged_min, troop.ranged_max) {
            (None, None) if troop.shots == 0 => {}
            (Some(min), Some(max)) if min <= max && troop.shots > 0 => {}
            _ => {
                return invalid(format!(
                    "troop {id:?}: ranged_min, ranged_max and shots do not fit together"
                ));
            }
        }
    }
    Ok(())
}

fn read_spells(raw: Vec<raw::Spell>) -> Result<Vec<Spell>, ContentError> {
    unique("spell", raw.iter().map(|s| s.id.as_str()))?;
    let mut spells = Vec::new();
    for spell in raw {
        let effect = match (spell.target.as_str(), spell.effect.as_str()) {
            ("living_enemy", "damage") => SpellEffect::Damage(spell.amount),
            ("living_friendly_with_injury", "remove_injury") => {
                only("spell.resurrects", &spell.resurrects, &Some(false))?;
                SpellEffect::RemoveInjury(spell.amount)
            }
            (target, effect) => {
                return Err(ContentError::Unsupported(format!(
                    "spell {:?}: effect {effect:?} on {target:?} is not implemented",
                    spell.id
                )));
            }
        };
        spells.push(Spell {
            id: spell.id,
            effect,
        });
    }
    Ok(spells)
}

/// A board small enough to set memory aside for, room for the stacks, and a round limit.
fn check_board(board: &Board, round_limit: u32) -> Result<(), ContentError> {
    if board.width == 0
        || board.height == 0
        || board.width > MAX_BOARD_SIDE
        || board.height > MAX_BOARD_SIDE
    {
        return invalid(format!(
            "board is {} by {}; each side must be 1..={MAX_BOARD_SIDE}",
            board.width, board.height
        ));
    }
    if board.max_stacks_per_side == 0 || board.max_stacks_per_side > board.width * board.height {
        return invalid(format!(
            "max_stacks_per_side is {}",
            board.max_stacks_per_side
        ));
    }
    if round_limit == 0 {
        return invalid("round_limit is 0".to_owned());
    }
    Ok(())
}

impl Rules {
    /// Reads and checks a rules file.
    ///
    /// # Errors
    /// [`ContentError`] naming the first thing wrong.
    pub fn from_json(text: &str) -> Result<Self, ContentError> {
        /// Enough of a rules file to tell which ruleset it is.
        #[derive(Deserialize)]
        struct Head {
            id: String,
        }
        let head: Head = read(text)?;
        if head.id == REFERENCE_RULESET || head.id == MODERN_RULESET {
            return reference::load(text);
        }
        let raw: raw::Rules = read(text)?;
        check_switches(&raw)?;
        let board = raw.board;
        check_board(&board, raw.round_limit)?;

        check_troops(&raw.troops)?;
        let spells = read_spells(raw.spells)?;

        Ok(Self {
            battles_id: raw.id.clone(),
            id: raw.id,
            profile: Profile::Prototype,
            board,
            round_limit: raw.round_limit,
            troops: raw.troops,
            spells,
        })
    }

    /// The troop with this ID.
    #[must_use]
    pub fn troop(&self, id: &str) -> Option<&Troop> {
        self.troops.iter().find(|t| t.id == id)
    }

    /// The spell with this ID.
    #[must_use]
    pub fn spell(&self, id: &str) -> Option<&Spell> {
        self.spells.iter().find(|s| s.id == id)
    }
}

/// A stack as a scenario places it.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StackStart {
    pub id: String,
    pub side: Side,
    pub slot: u32,
    pub troop_id: String,
    pub count: u32,
    pub injury: u32,
    pub x: u32,
    pub y: u32,
}

impl StackStart {
    #[must_use]
    pub const fn cell(&self) -> Cell {
        Cell {
            x: self.x,
            y: self.y,
        }
    }
}

/// Where a battle starts from. Never changed by the battle: a benchmark that wants another seed passes it to
/// `Battle::start`, it does not edit this.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub id: String,
    pub rng_state: u32,
    pub obstacles: Vec<Cell>,
    /// Charges of each hero spell the player side starts with, by spell ID.
    pub hero_charges: BTreeMap<String, u32>,
    /// The leadership of the player's hero. Only the reference rules have it; without it the player's side has
    /// no hero in the battle: no morale, and no stack is ever out of control.
    #[serde(default)]
    pub leadership: Option<u32>,
    pub stacks: Vec<StackStart>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ScenarioFile {
    schema_version: u32,
    ruleset_id: String,
    scenarios: Vec<Scenario>,
}

/// Reads a scenario file and checks every scenario against the rules.
///
/// # Errors
/// [`ContentError`] naming the first thing wrong.
pub fn scenarios_from_json(text: &str, rules: &Rules) -> Result<Vec<Scenario>, ContentError> {
    let file: ScenarioFile = read(text)?;
    only("schema_version", &file.schema_version, &1)?;
    only(
        "ruleset_id",
        &file.ruleset_id.as_str(),
        &rules.battles_id.as_str(),
    )?;
    unique("scenario", file.scenarios.iter().map(|s| s.id.as_str()))?;
    for scenario in &file.scenarios {
        scenario.check(rules)?;
    }
    Ok(file.scenarios)
}

impl Scenario {
    /// Checks the scenario against the rules: known troops and spells, everything on the board, one thing per
    /// cell, both sides present.
    ///
    /// # Errors
    /// [`ContentError::Invalid`] naming the scenario and the first thing wrong.
    pub fn check(&self, rules: &Rules) -> Result<(), ContentError> {
        let here = |why: String| invalid(format!("scenario {:?}: {why}", self.id));
        if self.leadership.is_some() && rules.profile == Profile::Prototype {
            return here("the prototype rules have no leadership".to_owned());
        }
        let mut taken = BTreeSet::new();
        for &cell in &self.obstacles {
            if !rules.board.contains(cell) {
                return here(format!(
                    "obstacle at {},{} is outside the board",
                    cell.x, cell.y
                ));
            }
            if !taken.insert(cell) {
                return here(format!("two obstacles at {},{}", cell.x, cell.y));
            }
        }
        for spell in self.hero_charges.keys() {
            if rules.spell(spell).is_none() {
                return here(format!("unknown spell {spell:?}"));
            }
        }
        unique("stack", self.stacks.iter().map(|s| s.id.as_str()))
            .or_else(|e| here(e.to_string()))?;
        let mut slots = BTreeSet::new();
        for stack in &self.stacks {
            let id = &stack.id;
            let Some(troop) = rules.troop(&stack.troop_id) else {
                return here(format!(
                    "stack {id:?} has unknown troop {:?}",
                    stack.troop_id
                ));
            };
            if stack.slot >= rules.board.max_stacks_per_side {
                return here(format!("stack {id:?} is in slot {}", stack.slot));
            }
            if !slots.insert((stack.side, stack.slot)) {
                return here(format!(
                    "two stacks in {} slot {}",
                    stack.side.name(),
                    stack.slot
                ));
            }
            if stack.count == 0 || stack.injury >= troop.hp {
                return here(format!(
                    "stack {id:?} has count {} and injury {}",
                    stack.count, stack.injury
                ));
            }
            if !rules.board.contains(stack.cell()) {
                return here(format!("stack {id:?} is outside the board"));
            }
            if !taken.insert(stack.cell()) {
                return here(format!(
                    "stack {id:?} stands on an obstacle or another stack"
                ));
            }
        }
        for side in [Side::Player, Side::Enemy] {
            if !self.stacks.iter().any(|s| s.side == side) {
                return here(format!("the {} side has no stacks", side.name()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) const RULES: &str = include_str!("../../../prototype/rules.json");
    pub(crate) const BATTLES: &str = include_str!("../../../prototype/battles.json");

    pub(crate) fn rules() -> Rules {
        Rules::from_json(RULES).unwrap()
    }

    pub(crate) fn scenario(id: &str) -> Scenario {
        let all = scenarios_from_json(BATTLES, &rules()).unwrap();
        all.into_iter().find(|s| s.id == id).unwrap()
    }

    /// The rules file with one piece of text replaced, to break it in one place.
    fn rules_with(old: &str, new: &str) -> Result<Rules, ContentError> {
        assert!(RULES.contains(old), "{old:?} is in the rules file");
        Rules::from_json(&RULES.replacen(old, new, 1))
    }

    fn battles_with(old: &str, new: &str) -> Result<Vec<Scenario>, ContentError> {
        assert!(BATTLES.contains(old), "{old:?} is in the battles file");
        scenarios_from_json(&BATTLES.replacen(old, new, 1), &rules())
    }

    #[test]
    fn the_supplied_files_load() {
        let rules = rules();
        assert_eq!((rules.board.width, rules.board.height), (6, 5));
        assert_eq!(rules.troops.len(), 3);
        assert_eq!(rules.troop("slinger").unwrap().ranged(), Some((2, 3)));
        assert_eq!(
            rules.spell("blast").unwrap().effect,
            SpellEffect::Damage(18)
        );
        assert_eq!(scenarios_from_json(BATTLES, &rules).unwrap().len(), 4);
    }

    // AT-053: bad content is refused with a message, before any battle begins.
    #[test]
    fn broken_rules_are_refused() {
        let malformed = |r| matches!(r, Err(ContentError::Malformed(_)));
        let unsupported = |r| matches!(r, Err(ContentError::Unsupported(_)));
        let is_invalid = |r| matches!(r, Err(ContentError::Invalid(_)));
        assert!(
            is_invalid(rules_with(r#""id": "slinger""#, r#""id": "warden""#)),
            "duplicate troop"
        );
        assert!(
            is_invalid(rules_with(r#""width": 6"#, r#""width": 4000"#)),
            "oversize board"
        );
        assert!(
            is_invalid(rules_with(r#""hp": 10"#, r#""hp": 0"#)),
            "no health"
        );
        assert!(
            is_invalid(rules_with(r#""melee_min": 2,"#, r#""melee_min": 9,"#)),
            "damage range upside down"
        );
        assert!(
            is_invalid(rules_with(r#""shots": 4"#, r#""shots": 0"#)),
            "shooter without shots"
        );
        assert!(
            unsupported(rules_with(r#""effect": "damage""#, r#""effect": "charm""#)),
            "unknown ability"
        );
        assert!(
            unsupported(rules_with(r#""neighbours": 8"#, r#""neighbours": 4"#)),
            "other movement rule"
        );
        assert!(
            unsupported(rules_with(
                r#""enemy_enabled": false"#,
                r#""enemy_enabled": true"#
            )),
            "enemy hero"
        );
        assert!(
            malformed(rules_with(r#""hp": 10"#, r#""hp": 10.5"#)),
            "float"
        );
        assert!(
            malformed(rules_with(r#""hp": 10,"#, r#""hp": 10, "hp": 11,"#)),
            "repeated key"
        );
        assert!(
            malformed(rules_with(r#""hp": 10,"#, r#""hp": 10, "armour": 1,"#)),
            "unknown field"
        );
        assert!(
            malformed(rules_with(r#""round_limit": 200,"#, "")),
            "missing field"
        );
    }

    #[test]
    fn broken_scenarios_are_refused() {
        let is_invalid = |r| matches!(r, Err(ContentError::Invalid(_)));
        assert!(
            is_invalid(battles_with(
                r#""troop_id": "slinger""#,
                r#""troop_id": "dragon""#
            )),
            "missing troop"
        );
        assert!(
            is_invalid(battles_with(
                r#""id": "blocked_shooter""#,
                r#""id": "melee_exchange""#
            )),
            "duplicate"
        );
        assert!(
            is_invalid(battles_with(r#""count": 10"#, r#""count": 0"#)),
            "empty stack"
        );
        assert!(
            is_invalid(battles_with(r#""injury": 0"#, r#""injury": 10"#)),
            "injury of a whole creature"
        );
        assert!(
            is_invalid(battles_with(r#""x": 1"#, r#""x": 6"#)),
            "outside the board"
        );
        assert!(
            is_invalid(battles_with(r#""x": 1"#, r#""x": 2"#)),
            "two stacks in one cell"
        );
        assert!(
            is_invalid(battles_with(r#""side": "enemy""#, r#""side": "player""#)),
            "slot taken twice"
        );
        assert!(
            is_invalid(battles_with(
                r#""hero_charges": {}"#,
                r#""hero_charges": {"storm": 1}"#
            )),
            "spell"
        );
    }
}
