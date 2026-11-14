//! An expedition as read from files: the map and everything on it that a campaign starts from. Checked when read,
//! like the rules and scenarios of a battle, so the campaign never meets an object that is not on the map or a troop
//! the rules do not know. Nothing here changes during play.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use serde::Deserialize;

use crate::canon::{Value, sha256_hex};
use crate::content::{Cell, ContentError, Rules};

/// Maps larger than this on a side are refused before any memory is set aside for them.
const MAX_MAP_SIDE: u32 = 256;

/// No length of time in an expedition is longer than this many ticks.
const MAX_TICKS: u32 = 1_000_000;

fn invalid<T>(why: String) -> Result<T, ContentError> {
    Err(ContentError::Invalid(why))
}

fn unsupported<T>(why: String) -> Result<T, ContentError> {
    Err(ContentError::Unsupported(why))
}

/// Reads a file into `T`, the strict reader first, so a float or a repeated key is caught wherever it stands.
fn read<T: for<'de> Deserialize<'de>>(text: &str) -> Result<T, ContentError> {
    Value::parse(text).map_err(|e| ContentError::Malformed(e.to_string()))?;
    serde_json::from_str(text).map_err(|e| ContentError::Malformed(e.to_string()))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ObjectKind {
    Town,
    /// The king's castle, where the hero is given his ranks.
    Castle,
    Encounter,
    Objective,
}

/// Something that stands on a map cell for the whole campaign. What has happened to it is campaign state.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapObject {
    pub id: String,
    pub kind: ObjectKind,
    pub x: u32,
    pub y: u32,
}

impl MapObject {
    #[must_use]
    pub const fn cell(&self) -> Cell {
        Cell {
            x: self.x,
            y: self.y,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Terrain {
    pub id: String,
    pub passable: bool,
}

/// A map: terrain in rows from the top, `x` to the right and `y` down, and the objects on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Map {
    pub id: String,
    pub width: u32,
    pub height: u32,
    /// One terrain for every cell, row by row.
    cells: Vec<Terrain>,
    pub objects: Vec<MapObject>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MapFile {
    schema_version: u32,
    id: String,
    width: u32,
    height: u32,
    terrain_legend: BTreeMap<String, Terrain>,
    rows: Vec<String>,
    objects: Vec<MapObject>,
}

impl Map {
    /// Reads and checks a map file.
    ///
    /// # Errors
    /// [`ContentError`] naming the first thing wrong.
    pub fn from_json(text: &str) -> Result<Self, ContentError> {
        let file: MapFile = read(text)?;
        if file.schema_version != 1 {
            return unsupported(format!("map schema {} is not 1", file.schema_version));
        }
        let (width, height) = (file.width, file.height);
        if width == 0 || height == 0 || width > MAX_MAP_SIDE || height > MAX_MAP_SIDE {
            return invalid(format!(
                "map is {width} by {height}; each side must be 1..={MAX_MAP_SIDE}"
            ));
        }
        if file.rows.len() != height as usize {
            return invalid(format!(
                "map has {} rows, its height is {height}",
                file.rows.len()
            ));
        }
        let mut cells = Vec::with_capacity((width * height) as usize);
        for row in &file.rows {
            if row.chars().count() != width as usize {
                return invalid(format!("map row {row:?} is not {width} cells wide"));
            }
            for symbol in row.chars() {
                let Some(terrain) = file.terrain_legend.get(symbol.to_string().as_str()) else {
                    return invalid(format!("map symbol {symbol:?} is not in the legend"));
                };
                cells.push(terrain.clone());
            }
        }
        let map = Self {
            id: file.id,
            width,
            height,
            cells,
            objects: file.objects,
        };
        let mut ids = BTreeSet::new();
        let mut taken = BTreeSet::new();
        for object in &map.objects {
            if !ids.insert(object.id.as_str()) {
                return invalid(format!("map object {:?} appears twice", object.id));
            }
            if !map.passable(object.cell()) {
                return invalid(format!(
                    "map object {:?} is off the map or on ground that cannot be walked",
                    object.id
                ));
            }
            if !taken.insert(object.cell()) {
                return invalid(format!(
                    "map object {:?} shares its cell with another",
                    object.id
                ));
            }
        }
        Ok(map)
    }

    /// The terrain of a cell, or `None` outside the map.
    #[must_use]
    pub fn terrain(&self, cell: Cell) -> Option<&Terrain> {
        (cell.x < self.width && cell.y < self.height)
            .then(|| &self.cells[(cell.y * self.width + cell.x) as usize])
    }

    /// On the map and ground that can be walked.
    #[must_use]
    pub fn passable(&self, cell: Cell) -> bool {
        self.terrain(cell).is_some_and(|terrain| terrain.passable)
    }

    #[must_use]
    pub fn object(&self, id: &str) -> Option<&MapObject> {
        self.objects.iter().find(|object| object.id == id)
    }

    #[must_use]
    pub fn object_at(&self, cell: Cell) -> Option<&MapObject> {
        self.objects.iter().find(|object| object.cell() == cell)
    }
}

/// A stack of the army as the expedition starts with it.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArmyStart {
    pub id: String,
    pub troop_id: String,
    pub count: u32,
    /// Always 0: injury is healed outside battle. Kept in the file to say so.
    pub injury: u32,
}

/// How time runs: everything a command costs and what the weeks and the deadline do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Clock {
    /// Ticks in a week; at every such tick the week is settled.
    pub week_ticks: u32,
    /// The tick at which the expedition is over.
    pub deadline_ticks: u32,
    pub movement_ticks: u32,
    pub search_ticks: u32,
    pub weekly_income: u64,
    pub upkeep: Upkeep,
}

/// What a creature costs to keep for a week.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Upkeep {
    /// The same for every creature.
    PerCreature(u64),
    /// A tenth of the troop's recruiting cost in the rules, rounded down: the reference's rule.
    TenthOfCost,
}

/// A further stack of a band, beside its first.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BandStack {
    pub troop: String,
    pub count: u32,
}

/// An enemy band standing on an encounter object.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Encounter {
    pub object_id: String,
    pub enemy_troop: String,
    pub enemy_count: u32,
    /// The random state its battle starts from: the same battle every time it is fought.
    pub battle_rng_state: u32,
    pub gold_reward: u64,
    /// Beaten while its contract is held, it gives the clue to the objective.
    pub grants_clue: bool,
    /// The band's further stacks, down the enemy's column of the layout below its first.
    #[serde(default)]
    pub also: Vec<BandStack>,
}

/// Where the two sides stand when a battle of the expedition begins.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleLayout {
    pub player: Cell,
    pub enemy: Cell,
    pub obstacles: Vec<Cell>,
}

/// What a lost or surrendered battle leaves the hero with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recovery {
    /// The object the hero is taken back to.
    pub destination: Cell,
    pub ticks: u32,
    /// The troop and number the hero is given.
    pub troop_id: String,
    pub count: u32,
}

/// The king the hero serves: where he holds court, and the class whose ranks he gives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct King {
    pub castle: Cell,
    /// Place of the hero's class in the rules.
    pub class: usize,
}

/// An expedition: a map, where it starts, and its rules of time, trade, fighting and recovery.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpeditionDef {
    pub id: String,
    pub map: Map,
    /// The rules its battles are fought under.
    pub rules: Arc<Rules>,
    pub start: Cell,
    pub start_gold: u64,
    pub leadership: u32,
    pub start_army: Vec<ArmyStart>,
    pub clock: Clock,
    /// The cell of the town, and what it sells: stock and price by troop ID.
    pub town: Cell,
    pub stock: BTreeMap<String, u32>,
    pub prices: BTreeMap<String, u64>,
    pub encounters: Vec<Encounter>,
    pub layout: BattleLayout,
    /// The object searched for. Its place is hidden until a clue names it; a search there wins with or without.
    pub objective_id: String,
    pub recovery: Recovery,
    /// With a king the hero has a class and rises through its ranks; without one, leadership and income stay
    /// what the expedition starts with.
    pub king: Option<King>,
    /// The routes a fixture file carries for the tests, as written. Nothing in play reads them.
    pub test_routes: Vec<serde_json::Value>,
    /// SHA-256 over the three files the expedition was read from: its own, its map's and its rules'. A save
    /// carries it, and is refused when any of the three has changed since.
    pub content_sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    schema_version: u32,
    id: String,
    #[serde(rename = "status")]
    _status: String,
    #[serde(rename = "map_path")]
    _map_path: String,
    #[serde(rename = "battle_rules_path")]
    _battle_rules_path: String,
    start: Start,
    clock: ClockFile,
    town: Town,
    encounters: Vec<Encounter>,
    battle_layout: BattleLayout,
    objective: Objective,
    recovery: RecoveryFile,
    #[serde(default)]
    king: Option<KingFile>,
    #[serde(default)]
    tests: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KingFile {
    castle_object_id: String,
    hero_class: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Start {
    map_id: String,
    x: u32,
    y: u32,
    gold: u64,
    leadership: u32,
    elapsed_ticks: u32,
    army: Vec<ArmyStart>,
    contract: Option<String>,
    clues: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ClockFile {
    ticks_per_day: u32,
    days_per_week: u32,
    deadline_ticks: u32,
    movement_ticks: u32,
    search_ticks: u32,
    success_on_exact_deadline: String,
    weekly_income: u64,
    upkeep_per_living_creature: u64,
    /// Absent, upkeep is `upkeep_per_living_creature`. Present, that field must be 0.
    #[serde(default)]
    upkeep_rule: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Town {
    object_id: String,
    stock: BTreeMap<String, u32>,
    price_per_creature: BTreeMap<String, u64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Objective {
    object_id: String,
    hidden_until_clue: bool,
    search_requires_clue: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryFile {
    destination_object_id: String,
    gold: String,
    ticks: u32,
    /// The fixture's way of naming the troops of recovery: so many Wardens.
    #[serde(default)]
    warden_count: Option<u32>,
    /// The general way: a troop and a number. One of the two ways is used.
    #[serde(default)]
    troop_id: Option<String>,
    #[serde(default)]
    count: Option<u32>,
    retains_contract_and_clues: bool,
}

/// Fails unless a switch of the file names the behaviour the campaign has.
fn only<T: PartialEq + std::fmt::Debug>(
    what: &str,
    found: &T,
    implemented: &T,
) -> Result<(), ContentError> {
    if found == implemented {
        Ok(())
    } else {
        unsupported(format!(
            "{what} is {found:?}; only {implemented:?} is implemented"
        ))
    }
}

/// The cell of the object with this ID, which must be of this kind.
fn place(map: &Map, id: &str, kind: ObjectKind, what: &str) -> Result<Cell, ContentError> {
    match map.object(id) {
        Some(object) if object.kind == kind => Ok(object.cell()),
        Some(_) => invalid(format!("{what} {id:?} is not a {kind:?} on the map")),
        None => invalid(format!("{what} {id:?} is not on the map")),
    }
}

/// The clock of a file, with every length of time between one tick and [`MAX_TICKS`]. The bound keeps the sums
/// of ticks far from overflowing, and no expedition is that long.
fn clock(file: &ClockFile) -> Result<Clock, ContentError> {
    // A product too large for the type saturates, and is then refused below as too long.
    let week_ticks = file.ticks_per_day.saturating_mul(file.days_per_week);
    let lengths = [
        week_ticks,
        file.deadline_ticks,
        file.movement_ticks,
        file.search_ticks,
    ];
    if lengths.iter().any(|&ticks| ticks == 0 || ticks > MAX_TICKS) {
        return invalid(format!(
            "the clock's week, deadline, step and search must each be 1..={MAX_TICKS} ticks; they are {lengths:?}"
        ));
    }
    let upkeep = match (file.upkeep_rule.as_deref(), file.upkeep_per_living_creature) {
        (None, each) => Upkeep::PerCreature(each),
        (Some("tenth_of_recruit_cost"), 0) => Upkeep::TenthOfCost,
        (Some(rule), each) => {
            return unsupported(format!(
                "clock.upkeep_rule is {rule:?} with {each} a creature; only \"tenth_of_recruit_cost\" with 0 is implemented"
            ));
        }
    };
    Ok(Clock {
        week_ticks,
        deadline_ticks: file.deadline_ticks,
        movement_ticks: file.movement_ticks,
        search_ticks: file.search_ticks,
        weekly_income: file.weekly_income,
        upkeep,
    })
}

/// Checks where and with what the expedition starts.
fn check_start(start: &Start, map: &Map, rules: &Rules) -> Result<Cell, ContentError> {
    let cell = Cell {
        x: start.x,
        y: start.y,
    };
    if !map.passable(cell) {
        return invalid(
            "the expedition starts off the map or on ground that cannot be walked".to_owned(),
        );
    }
    let mut stacks = BTreeSet::new();
    for stack in &start.army {
        known(rules, &stack.troop_id, "the starting army")?;
        if stack.count == 0 || stack.injury != 0 || !stacks.insert(stack.id.as_str()) {
            return invalid(format!(
                "starting stack {:?} must be unique, not empty and unhurt",
                stack.id
            ));
        }
    }
    if start.army.is_empty() || start.army.len() > rules.board.max_stacks_per_side as usize {
        return invalid(format!("the starting army has {} stacks", start.army.len()));
    }
    Ok(cell)
}

/// Checks the town's wares and the bands, and returns the town's cell.
fn check_town_and_bands(
    town: &Town,
    bands: &[Encounter],
    map: &Map,
    rules: &Rules,
) -> Result<Cell, ContentError> {
    let cell = place(map, &town.object_id, ObjectKind::Town, "the town")?;
    for troop in town.stock.keys().chain(town.price_per_creature.keys()) {
        known(rules, troop, "the town")?;
    }
    if let Some(unpriced) = town
        .stock
        .keys()
        .find(|troop| !town.price_per_creature.contains_key(*troop))
    {
        return invalid(format!("the town stocks {unpriced:?} without a price"));
    }
    let mut seen = BTreeSet::new();
    for band in bands {
        place(map, &band.object_id, ObjectKind::Encounter, "the encounter")?;
        known(rules, &band.enemy_troop, "an encounter")?;
        for stack in &band.also {
            known(rules, &stack.troop, "an encounter")?;
        }
        let stacks = 1 + band.also.len();
        let empty = band.enemy_count == 0 || band.also.iter().any(|stack| stack.count == 0);
        if stacks > rules.board.max_stacks_per_side as usize {
            return invalid(format!(
                "encounter {:?} has {stacks} stacks",
                band.object_id
            ));
        }
        if empty || !seen.insert(band.object_id.as_str()) {
            return invalid(format!(
                "encounter {:?} must be unique and not empty",
                band.object_id
            ));
        }
    }
    Ok(cell)
}

/// The army stands down the player's column of the layout, one stack a row from the player's cell, coming round
/// at the foot of the board; the band down the enemy's column in the same way, as many rows as the largest band
/// has stacks. Every cell that can be used must be on the board and free.
fn check_layout(
    layout: &BattleLayout,
    bands: &[Encounter],
    rules: &Rules,
) -> Result<(), ContentError> {
    let board = &rules.board;
    let column = |top: Cell, stacks: u32| {
        (0..stacks.min(board.height)).map(move |slot| Cell {
            x: top.x,
            y: (top.y + slot) % board.height.max(1),
        })
    };
    let largest = bands.iter().map(|band| 1 + band.also.len()).max();
    let largest = u32::try_from(largest.unwrap_or(1)).unwrap_or(u32::MAX);
    let mut used: Vec<Cell> = column(layout.player, board.max_stacks_per_side).collect();
    used.extend(column(layout.enemy, largest));
    let on_board = used
        .iter()
        .chain(&layout.obstacles)
        .all(|&cell| board.contains(cell));
    let clash = used
        .iter()
        .enumerate()
        .any(|(i, cell)| used[..i].contains(cell) || layout.obstacles.contains(cell));
    if !on_board || clash || board.max_stacks_per_side > board.height {
        return invalid(
            "the battle layout puts something off the board or two things in one cell".to_owned(),
        );
    }
    Ok(())
}

fn recovery(file: &RecoveryFile, map: &Map, rules: &Rules) -> Result<Recovery, ContentError> {
    let (troop_id, count) = match (file.warden_count, file.troop_id.as_deref(), file.count) {
        (Some(count), None, None) => ("warden", count),
        (None, Some(troop_id), Some(count)) => (troop_id, count),
        _ => {
            return invalid(
                "recovery names its troops either as warden_count or as troop_id with count"
                    .to_owned(),
            );
        }
    };
    known(rules, troop_id, "recovery")?;
    let Some(destination) = map.object(&file.destination_object_id) else {
        return invalid(format!(
            "recovery leads to {:?}, which is not on the map",
            file.destination_object_id
        ));
    };
    if count == 0 || file.ticks == 0 {
        return invalid(
            "recovery must give at least one creature and take at least one tick".to_owned(),
        );
    }
    Ok(Recovery {
        destination: destination.cell(),
        ticks: file.ticks,
        troop_id: troop_id.to_owned(),
        count,
    })
}

/// The king's castle is on the map and the hero's class in the rules. What the expedition starts with must be
/// what the class's first rank gives, so that the file says one thing: the ranks add to it from there.
fn check_king(
    king: &KingFile,
    file: &File,
    map: &Map,
    rules: &Rules,
) -> Result<King, ContentError> {
    let castle = place(
        map,
        &king.castle_object_id,
        ObjectKind::Castle,
        "the king's castle",
    )?;
    let Some(class) = rules.classes.iter().position(|c| c.id == king.hero_class) else {
        return invalid(format!(
            "the hero's class {:?} is not in the rules",
            king.hero_class
        ));
    };
    let first = &rules.classes[class].ranks[0];
    if file.start.leadership != first.leadership
        || file.clock.weekly_income != u64::from(first.commission)
    {
        return invalid(format!(
            "a {} sets out with leadership {} and a weekly income of {}",
            rules.classes[class].name, first.leadership, first.commission
        ));
    }
    Ok(King { castle, class })
}

fn known(rules: &Rules, troop: &str, what: &str) -> Result<(), ContentError> {
    match rules.troop(troop) {
        Some(_) => Ok(()),
        None => invalid(format!("{what} names the unknown troop {troop:?}")),
    }
}

impl ExpeditionDef {
    /// Reads and checks an expedition from the texts of its three files: its own, its map's and the rules of its
    /// battles. The caller reads the files; this crate opens none.
    ///
    /// # Errors
    /// [`ContentError`] naming the first thing wrong.
    pub fn from_texts(text: &str, map: &str, rules: &str) -> Result<Self, ContentError> {
        // The lengths go into the hash with the texts, so that no two triples of files run together alike.
        let mut all = format!("{} {} {}\n", text.len(), map.len(), rules.len());
        all.extend([text, map, rules]);
        let content_sha256 = sha256_hex(all.as_bytes());
        let (map, rules) = (Map::from_json(map)?, Arc::new(Rules::from_json(rules)?));
        let file: File = read(text)?;
        only("schema_version", &file.schema_version, &1)?;
        only(
            "start.map_id",
            &file.start.map_id.as_str(),
            &map.id.as_str(),
        )?;
        only("start.elapsed_ticks", &file.start.elapsed_ticks, &0)?;
        only("start.contract", &file.start.contract, &None)?;
        only("start.clues", &file.start.clues.len(), &0)?;
        only(
            "clock.success_on_exact_deadline",
            &file.clock.success_on_exact_deadline.as_str(),
            &"victory",
        )?;
        only(
            "objective.hidden_until_clue",
            &file.objective.hidden_until_clue,
            &true,
        )?;
        only(
            "objective.search_requires_clue",
            &file.objective.search_requires_clue,
            &false,
        )?;
        only(
            "recovery.gold",
            &file.recovery.gold.as_str(),
            &"floor(current_gold/2)",
        )?;
        only(
            "recovery.retains_contract_and_clues",
            &file.recovery.retains_contract_and_clues,
            &true,
        )?;

        let clock = clock(&file.clock)?;
        let start = check_start(&file.start, &map, &rules)?;
        let town = check_town_and_bands(&file.town, &file.encounters, &map, &rules)?;
        place(
            &map,
            &file.objective.object_id,
            ObjectKind::Objective,
            "the objective",
        )?;
        check_layout(&file.battle_layout, &file.encounters, &rules)?;
        let recovery = recovery(&file.recovery, &map, &rules)?;
        let king = match &file.king {
            Some(king) => Some(check_king(king, &file, &map, &rules)?),
            None => None,
        };

        Ok(Self {
            id: file.id,
            map,
            rules,
            start,
            start_gold: file.start.gold,
            leadership: file.start.leadership,
            start_army: file.start.army,
            clock,
            town,
            stock: file.town.stock,
            prices: file.town.price_per_creature,
            encounters: file.encounters,
            layout: file.battle_layout,
            objective_id: file.objective.object_id,
            recovery,
            king,
            test_routes: file.tests,
            content_sha256,
        })
    }

    /// The cell of the objective. Hidden from the player until a clue names it.
    #[must_use]
    pub fn objective(&self) -> Cell {
        // `from_json` has seen the object, so it is there.
        self.map
            .object(&self.objective_id)
            .map_or(self.start, MapObject::cell)
    }

    #[must_use]
    pub fn encounter(&self, object_id: &str) -> Option<&Encounter> {
        self.encounters
            .iter()
            .find(|encounter| encounter.object_id == object_id)
    }
}
