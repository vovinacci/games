//! Plays the routes an expedition file carries and reports what each cost: time, gold in and out, creatures lost,
//! how it ended (AT-081). Two reports are compared route by route.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use bounty_ai::{APPROACH_V1, approach_v1};
use bounty_core::campaign::{Action, Command, Event, Expedition, ExpeditionDef, Status};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Version of the report format. Reports of different versions are not compared.
pub const REPORT_SCHEMA: u32 = 1;

/// The most commands one battle of a route may take before the run gives up on it.
const BATTLE_BUDGET: u32 = 100_000;

/// Creature counts by troop ID.
pub type Counts = BTreeMap<String, u64>;

/// What was run, so that two reports can be told comparable or not.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    pub engine_revision: String,
    pub expedition_id: String,
    pub ruleset_id: String,
    /// SHA-256 over the expedition, its map and its rules, as a save carries it.
    pub content_sha256: String,
    pub pilot_version: String,
}

/// One route, start to end.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Row {
    pub route_id: String,
    pub status: String,
    pub elapsed_ticks: u32,
    pub weeks_settled: u32,
    pub gold_start: u64,
    pub gold_end: u64,
    /// Gold in: the weeks' income and the rewards of won battles.
    pub income: u64,
    pub rewards: u64,
    /// Gold out: the weeks' upkeep, the price of recruits, and what recovery took.
    pub upkeep: u64,
    pub recruiting: u64,
    pub lost_in_recovery: u64,
    pub recruited: Counts,
    pub battles: u32,
    pub battles_won: u32,
    pub battles_stalled: u32,
    /// Battles lost or given up, each followed by recovery.
    pub recoveries: u32,
    /// Creatures of the hero's that did not come back from battle, a lost army counted whole.
    pub casualties: Counts,
    /// The casualties at the expedition's prices.
    pub casualties_cost: u64,
    pub army_end: Counts,
    /// Commands the expedition took, those of its battles included.
    pub accepted_commands: u32,
    /// Digest of the final state: two rows with the same digest ended in exactly the same state.
    pub final_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    pub schema_version: u32,
    pub identity: Identity,
    /// In the order of the expedition file.
    pub rows: Vec<Row>,
    /// Defects found while running. A report with any is not a valid result.
    pub problems: Vec<String>,
}

/// Reads an expedition file with the map and the rules it names, both beside it by its own paths.
///
/// # Errors
/// A file that cannot be read, or content the rules refuse.
pub fn load(expedition_path: &Path) -> Result<Arc<ExpeditionDef>, String> {
    let read =
        |path: &Path| fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()));
    let text = read(expedition_path)?;
    let file: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let dir = expedition_path.parent().unwrap_or_else(|| Path::new("."));
    let beside = |field: &str| {
        let path = file[field]
            .as_str()
            .ok_or_else(|| format!("the expedition names no {field}"))?;
        read(&dir.join(path))
    };
    let def = ExpeditionDef::from_texts(&text, &beside("map_path")?, &beside("battle_rules_path")?);
    def.map(Arc::new).map_err(|e| e.to_string())
}

fn army(expedition: &Expedition) -> Counts {
    let mut counts = Counts::new();
    for stack in expedition.army() {
        *counts.entry(stack.troop_id.clone()).or_default() += u64::from(stack.count);
    }
    counts
}

/// The hero's actions a command of a route stands for; the commands of a battle are found while it is fought.
enum Step {
    Act(Action),
    ResolveBattle,
}

fn step(command: &Value) -> Result<Step, String> {
    let text = |field: &str| {
        let value = command[field].as_str().map(str::to_owned);
        value.ok_or_else(|| format!("the command has no {field}: {command}"))
    };
    let number = |field: &str| {
        let value = command[field].as_i64().and_then(|n| i32::try_from(n).ok());
        value.ok_or_else(|| format!("the command has no {field}: {command}"))
    };
    Ok(Step::Act(match text("kind")?.as_str() {
        "take_contract" => Action::TakeContract { id: text("id")? },
        "recruit" => Action::Recruit {
            troop_id: text("troop_id")?,
            quantity: number("quantity")?.unsigned_abs(),
        },
        "move" => Action::Move {
            dx: number("dx")?,
            dy: number("dy")?,
        },
        "wait_tick" => Action::WaitTick,
        "search" => Action::Search,
        "surrender" => Action::Surrender,
        "resolve_battle" => {
            // Never a result handed in from outside: the named pilot plays both sides.
            let pilot = text("pilot")?;
            if pilot != APPROACH_V1 {
                return Err(format!(
                    "the route asks for the pilot {pilot:?}, there is {APPROACH_V1:?}"
                ));
            }
            return Ok(Step::ResolveBattle);
        }
        other => {
            return Err(format!(
                "the route has a command this tool does not know: {other}"
            ));
        }
    }))
}

/// A route being played, with its account.
struct Run<'a, F: FnMut(&mut Expedition)> {
    expedition: Expedition,
    row: Row,
    /// The army as it went into the battle being fought.
    fielded: Counts,
    /// Called after every accepted command, for a test that wants to look at each state or replace it.
    after: &'a mut F,
}

impl<F: FnMut(&mut Expedition)> Run<'_, F> {
    fn act(&mut self, action: Action) -> Result<(), String> {
        let said = format!("{action:?}");
        let gold_before = self.expedition.gold();
        let command = Command {
            revision: self.expedition.revision(),
            action,
        };
        let events = self
            .expedition
            .apply(&command)
            .map_err(|refused| format!("{said} refused: {refused}"))?;
        self.row.accepted_commands += 1;
        for event in events {
            self.account(&event, gold_before);
        }
        (self.after)(&mut self.expedition);
        Ok(())
    }

    fn account(&mut self, event: &Event, gold_before: u64) {
        let row = &mut self.row;
        match event {
            Event::WeekSettled { income, upkeep, .. } => {
                row.weeks_settled += 1;
                row.income += income;
                row.upkeep += upkeep;
            }
            Event::Recruited {
                troop_id,
                quantity,
                cost,
            } => {
                row.recruiting += cost;
                *row.recruited.entry(troop_id.clone()).or_default() += u64::from(*quantity);
            }
            Event::BattleStarted { .. } => {
                row.battles += 1;
                self.fielded = army(&self.expedition);
            }
            Event::BattleWon { reward, .. } => {
                row.battles_won += 1;
                row.rewards += reward;
                self.count_casualties(&army(&self.expedition));
            }
            Event::BattleStalled { .. } => {
                row.battles_stalled += 1;
                self.count_casualties(&army(&self.expedition));
            }
            Event::Recovered { gold } => {
                row.recoveries += 1;
                // Recovery is the first thing of its command to touch the purse.
                row.lost_in_recovery += gold_before.saturating_sub(*gold);
                // The army is lost whole; what the hero has now was given to him.
                self.count_casualties(&Counts::new());
            }
            _ => {}
        }
    }

    fn count_casualties(&mut self, back: &Counts) {
        let prices = &self.expedition.def().prices;
        for (troop, went) in std::mem::take(&mut self.fielded) {
            let lost = went.saturating_sub(back.get(&troop).copied().unwrap_or(0));
            if lost > 0 {
                self.row.casualties_cost += lost * prices.get(&troop).copied().unwrap_or(0);
                *self.row.casualties.entry(troop).or_default() += lost;
            }
        }
    }

    fn resolve_battle(&mut self) -> Result<(), String> {
        for _ in 0..BATTLE_BUDGET {
            let Some(battle) = self.expedition.battle() else {
                return Ok(());
            };
            let Some(order) = approach_v1(&battle.observe()) else {
                return Err("the pilot has no command for a battle that is not over".to_owned());
            };
            self.act(Action::Battle(order))?;
        }
        Err(format!("a battle took more than {BATTLE_BUDGET} commands"))
    }
}

/// Plays one route of `def` from the start. `after` sees the expedition after every accepted command and may
/// replace it, which is how a test goes on from a reloaded save.
///
/// # Errors
/// What stopped the route: a command it cannot read, or one the expedition refused.
pub fn play_route(
    def: &Arc<ExpeditionDef>,
    route: &Value,
    after: &mut impl FnMut(&mut Expedition),
) -> Result<(Expedition, Row), String> {
    let expedition = Expedition::start(Arc::clone(def));
    let row = Row {
        route_id: route["id"].as_str().unwrap_or_default().to_owned(),
        gold_start: expedition.gold(),
        ..Row::default()
    };
    let mut run = Run {
        expedition,
        row,
        fielded: Counts::new(),
        after,
    };
    for command in route["commands"].as_array().map_or(&[][..], Vec::as_slice) {
        match step(command)? {
            Step::Act(action) => run.act(action)?,
            Step::ResolveBattle => run.resolve_battle()?,
        }
    }
    let Run {
        expedition,
        mut row,
        ..
    } = run;
    row.status.push_str(expedition.status().name());
    row.elapsed_ticks = expedition.ticks();
    row.gold_end = expedition.gold();
    row.army_end = army(&expedition);
    row.final_digest = expedition.digest();
    Ok((expedition, row))
}

/// What is wrong with a row by itself: a route left in the middle of a battle, or an account that does not add up.
fn defects(row: &Row) -> Vec<String> {
    let mut found = Vec::new();
    if row.status == Status::Battle.name() {
        found.push(format!(
            "{}: the route ends in the middle of a battle",
            row.route_id
        ));
    }
    let gold_in = row.gold_start + row.income + row.rewards;
    let gold_out = row.upkeep + row.recruiting + row.lost_in_recovery;
    if gold_in.checked_sub(gold_out) != Some(row.gold_end) {
        found.push(format!(
            "{}: {gold_in} gold in and {gold_out} out do not leave the {} in the purse",
            row.route_id, row.gold_end
        ));
    }
    found
}

/// Plays every route of the expedition, each twice: a route that does not repeat is a defect.
#[must_use]
pub fn run(def: &Arc<ExpeditionDef>, engine_revision: &str) -> Report {
    let mut report = Report {
        schema_version: REPORT_SCHEMA,
        identity: Identity {
            engine_revision: engine_revision.to_owned(),
            expedition_id: def.id.clone(),
            ruleset_id: def.rules.id.clone(),
            content_sha256: def.content_sha256.clone(),
            pilot_version: APPROACH_V1.to_owned(),
        },
        rows: Vec::new(),
        problems: Vec::new(),
    };
    if def.test_routes.is_empty() {
        report
            .problems
            .push("the expedition carries no route".to_owned());
    }
    for route in &def.test_routes {
        let id = route["id"].as_str().unwrap_or("a route without ID");
        let play = || play_route(def, route, &mut |_| ()).map(|(_, row)| row);
        match (play(), play()) {
            (Ok(row), Ok(again)) => {
                if row != again {
                    report
                        .problems
                        .push(format!("{id}: played twice, it ended differently"));
                }
                report.problems.extend(defects(&row));
                report.rows.push(row);
            }
            (Err(why), _) | (_, Err(why)) => report.problems.push(format!("{id}: {why}")),
        }
    }
    report
}

/// What differs between two reports, a line for each; empty when every route went the same way. The engine
/// revision is expected to differ and is not listed.
#[must_use]
pub fn compare(baseline: &Report, candidate: &Report) -> Vec<String> {
    let mut lines = Vec::new();
    if baseline.schema_version != candidate.schema_version {
        return vec![format!(
            "report formats {} and {} are not compared",
            baseline.schema_version, candidate.schema_version
        )];
    }
    let identity = |report: &Report| Identity {
        engine_revision: String::new(),
        ..report.identity.clone()
    };
    if identity(baseline) != identity(candidate) {
        lines.push(format!(
            "run differs: {:?} -> {:?}",
            identity(baseline),
            identity(candidate)
        ));
    }
    for problem in baseline.problems.iter().chain(&candidate.problems) {
        lines.push(format!("a report has a problem: {problem}"));
    }
    let fields = |row: &Row| match serde_json::to_value(row) {
        Ok(Value::Object(fields)) => fields,
        _ => serde_json::Map::new(),
    };
    for before in &baseline.rows {
        let Some(after) = candidate
            .rows
            .iter()
            .find(|row| row.route_id == before.route_id)
        else {
            lines.push(format!("{}: missing from the candidate", before.route_id));
            continue;
        };
        let after = fields(after);
        for (field, old) in fields(before) {
            let new = after.get(&field).unwrap_or(&Value::Null);
            if *new != old {
                lines.push(format!("{}: {field}: {old} -> {new}", before.route_id));
            }
        }
    }
    for row in &candidate.rows {
        if !baseline
            .rows
            .iter()
            .any(|before| before.route_id == row.route_id)
        {
            lines.push(format!("{}: new in the candidate", row.route_id));
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo() -> Arc<ExpeditionDef> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/demo/expedition.json");
        load(&path).unwrap()
    }

    fn baseline() -> Report {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../balance/baselines/expedition-demo-0001.json");
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
    }

    // AT-081: each route reports its time, its gold in and out, its losses and how it ended.
    #[test]
    fn the_demo_routes_report_what_they_cost() {
        let report = run(&demo(), "test");
        assert_eq!(report.problems, Vec::<String>::new());
        let row = |id: &str| report.rows.iter().find(|row| row.route_id == id).unwrap();
        assert_eq!(report.rows.len(), 4);

        let complete = row("complete-route");
        assert_eq!(
            (complete.status.as_str(), complete.elapsed_ticks),
            ("victory", 15)
        );
        assert_eq!((complete.gold_start, complete.gold_end), (100, 150));
        assert_eq!(
            (complete.battles, complete.battles_won, complete.recoveries),
            (2, 2, 0)
        );
        assert_eq!(complete.recruited["warden"], 2);
        assert_eq!(
            complete.army_end["warden"],
            12 - complete.casualties.get("warden").copied().unwrap_or(0)
        );

        let surrender = row("surrender-first-encounter");
        assert_eq!((surrender.battles, surrender.recoveries), (1, 1));
        assert_eq!(
            surrender.casualties["warden"], 12,
            "an army given up is lost whole"
        );
        assert_eq!(surrender.army_end["warden"], 4);
        assert!(surrender.lost_in_recovery > 0);

        let timeout = row("timeout-at-town");
        assert_eq!(
            (timeout.status.as_str(), timeout.elapsed_ticks),
            ("timeout", 24)
        );
        assert_eq!(
            (timeout.battles, timeout.recruiting, timeout.casualties_cost),
            (0, 0, 0)
        );
        assert!(timeout.weeks_settled > 0 && timeout.income > 0);
    }

    // The code as it stands gives exactly the recorded baseline; see bounty/balance/README.md for replacing it.
    #[test]
    fn the_demo_expedition_gives_its_recorded_baseline() {
        let difference = compare(&baseline(), &run(&demo(), "test"));
        assert_eq!(difference, Vec::<String>::new());
    }

    #[test]
    fn a_route_that_moved_is_named_with_what_moved() {
        let baseline = baseline();
        let mut candidate = baseline.clone();
        candidate.identity.engine_revision = "other".to_owned();
        assert_eq!(compare(&baseline, &candidate), Vec::<String>::new());

        candidate.rows[0].gold_end += 5;
        candidate.rows.remove(1);
        let lines = compare(&baseline, &candidate);
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(
            lines[0].contains("complete-route: gold_end: 150 -> 155"),
            "{lines:?}"
        );
        assert!(lines[1].contains("missing from the candidate"), "{lines:?}");

        candidate.schema_version += 1;
        assert_eq!(compare(&baseline, &candidate).len(), 1);
    }

    #[test]
    fn a_row_that_does_not_add_up_is_a_defect() {
        let mut row = run(&demo(), "test").rows.remove(0);
        assert_eq!(defects(&row), Vec::<String>::new());
        row.rewards += 1;
        assert_eq!(defects(&row).len(), 1);
        row.rewards -= 1;
        row.status = "battle".to_owned();
        assert_eq!(defects(&row).len(), 1);
    }

    #[test]
    fn a_route_the_tool_cannot_play_is_a_problem_not_a_row() {
        let mut def = (*demo()).clone();
        def.test_routes[0]["commands"][0]["kind"] = "fly".into();
        def.test_routes[1]["commands"][0]["quantity"] = 4000.into();
        let report = run(&Arc::new(def), "test");
        assert_eq!(
            (report.rows.len(), report.problems.len()),
            (2, 2),
            "{:?}",
            report.problems
        );
    }
}
