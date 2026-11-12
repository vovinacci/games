//! An expedition as read from files: the map and everything on it that a campaign starts from. Checked when read,
//! like the rules and scenarios of a battle, so the campaign never meets an object that is not on the map or a troop
//! the rules do not know. Nothing here changes during play.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use serde::Deserialize;

use crate::canon::Value;
use crate::content::{Cell, ContentError, Rules};

/// Maps larger than this on a side are refused before any memory is set aside for them.
const MAX_MAP_SIDE: u32 = 256;

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
    pub upkeep_per_creature: u64,
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
    /// The routes a fixture file carries for the tests, as written. Nothing in play reads them.
    pub test_routes: Vec<serde_json::Value>,
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
    tests: Vec<serde_json::Value>,
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
    warden_count: u32,
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
        if band.enemy_count == 0 || !seen.insert(band.object_id.as_str()) {
            return invalid(format!(
                "encounter {:?} must be unique and not empty",
                band.object_id
            ));
        }
    }
    Ok(cell)
}

fn check_layout(layout: &BattleLayout, rules: &Rules) -> Result<(), ContentError> {
    let cells = [layout.player, layout.enemy];
    let on_board = cells
        .iter()
        .chain(&layout.obstacles)
        .all(|&cell| rules.board.contains(cell));
    if !on_board
        || layout.player == layout.enemy
        || cells.iter().any(|cell| layout.obstacles.contains(cell))
    {
        return invalid(
            "the battle layout puts something off the board or two things in one cell".to_owned(),
        );
    }
    Ok(())
}

fn recovery(file: &RecoveryFile, map: &Map, rules: &Rules) -> Result<Recovery, ContentError> {
    // The file names the troop of recovery in its field's name; the fixture has only Wardens.
    let troop_id = "warden";
    known(rules, troop_id, "recovery")?;
    let Some(destination) = map.object(&file.destination_object_id) else {
        return invalid(format!(
            "recovery leads to {:?}, which is not on the map",
            file.destination_object_id
        ));
    };
    if file.warden_count == 0 || file.ticks == 0 {
        return invalid(
            "recovery must give at least one creature and take at least one tick".to_owned(),
        );
    }
    Ok(Recovery {
        destination: destination.cell(),
        ticks: file.ticks,
        troop_id: troop_id.to_owned(),
        count: file.warden_count,
    })
}

fn known(rules: &Rules, troop: &str, what: &str) -> Result<(), ContentError> {
    match rules.troop(troop) {
        Some(_) => Ok(()),
        None => invalid(format!("{what} names the unknown troop {troop:?}")),
    }
}

impl ExpeditionDef {
    /// Reads and checks an expedition file against its map and the rules of its battles. The caller reads the
    /// three files; this crate opens none.
    ///
    /// # Errors
    /// [`ContentError`] naming the first thing wrong.
    pub fn from_json(text: &str, map: Map, rules: Arc<Rules>) -> Result<Self, ContentError> {
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

        let clock = Clock {
            week_ticks: file
                .clock
                .ticks_per_day
                .saturating_mul(file.clock.days_per_week),
            deadline_ticks: file.clock.deadline_ticks,
            movement_ticks: file.clock.movement_ticks,
            search_ticks: file.clock.search_ticks,
            weekly_income: file.clock.weekly_income,
            upkeep_per_creature: file.clock.upkeep_per_living_creature,
        };
        if clock.week_ticks == 0
            || clock.deadline_ticks == 0
            || clock.movement_ticks == 0
            || clock.search_ticks == 0
        {
            return invalid("the clock needs a week, a deadline, and a cost of at least one tick to move and to search".to_owned());
        }
        let start = check_start(&file.start, &map, &rules)?;
        let town = check_town_and_bands(&file.town, &file.encounters, &map, &rules)?;
        place(
            &map,
            &file.objective.object_id,
            ObjectKind::Objective,
            "the objective",
        )?;
        check_layout(&file.battle_layout, &rules)?;
        let recovery = recovery(&file.recovery, &map, &rules)?;

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
            test_routes: file.tests,
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
