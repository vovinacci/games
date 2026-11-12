//! Runs a suite and records one row for every case and seed.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bounty_ai::{APPROACH_V1, approach_v1};
use bounty_core::battle::{Battle, Outcome};
use bounty_core::canon::sha256_hex;
use bounty_core::content::{Rules, Scenario, Side, scenarios_from_json};
use serde::{Deserialize, Serialize};

/// The version of the report's shape; a report of another version is not compared.
pub const REPORT_SCHEMA: u32 = 2;

/// A battle that takes more accepted commands than this is stopped and reported as a problem. Far above what the
/// round limit allows a sane pilot (200 rounds, ten stacks, a few commands each).
const COMMAND_BUDGET: u32 = 100_000;

/// A suite file, `balance/suite.json`. Paths in it are relative to the file.
#[derive(Clone, Debug, Deserialize)]
pub struct Suite {
    pub schema_version: u32,
    pub suite_id: String,
    pub ruleset_id: String,
    pub rules_path: PathBuf,
    pub scenarios_path: PathBuf,
    pub metrics_path: PathBuf,
    /// The recorded report that runs are compared with. `None` for a suite whose first numbers have not been
    /// accepted yet: such a suite can be run, and there is nothing to compare it with.
    #[serde(default)]
    pub baseline_path: Option<PathBuf>,
    pub pilot: Pilot,
    pub battle_seeds: Vec<u32>,
    pub cases: Vec<Case>,
    pub planned_runs: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Pilot {
    pub id: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Case {
    pub id: String,
    /// Any other outcome is reported as a problem.
    pub allowed_outcomes: Vec<String>,
}

#[derive(Deserialize)]
struct Metrics {
    reference_prices: BTreeMap<String, u64>,
}

/// Everything a run needs, read and checked.
pub struct Loaded {
    pub suite: Suite,
    pub rules: Arc<Rules>,
    pub scenarios: Vec<Scenario>,
    /// Price of one creature by troop ID, frozen, for valuing losses the same way before and after a change.
    pub prices: BTreeMap<String, u64>,
    pub rules_hash: String,
    pub scenario_hash: String,
}

/// What was run, so that two reports can be told comparable or not.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    pub engine_revision: String,
    pub ruleset_id: String,
    /// SHA-256 of the rules file's bytes.
    pub rules_hash: String,
    /// SHA-256 of the scenario file's bytes.
    pub scenario_hash: String,
    pub pilot_version: String,
    pub seed_list: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Coverage {
    /// Runs the suite plans: cases times seeds.
    pub planned: u32,
    pub executed: u32,
    /// Cases of the suite left out of this run.
    pub cases_not_run: Vec<String>,
}

/// Creature counts by side, then by troop ID.
pub type Counts = BTreeMap<String, BTreeMap<String, u32>>;

/// One battle, start to end.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Row {
    pub case_id: String,
    pub seed: u32,
    pub outcome: String,
    pub survivors: Counts,
    /// Starting count minus survivors. Right only while nothing is resurrected or summoned.
    pub casualties: Counts,
    /// Casualties valued at the frozen reference prices, by side.
    pub replacement_cost_reference: BTreeMap<String, u64>,
    pub completed_rounds: u32,
    pub accepted_actions: u32,
    pub ammunition_used: u32,
    pub spell_charges_used: u32,
    /// Commands of the pilot the rules refused. Any is a defect.
    pub invalid_commands: u32,
    /// Digest of the final state: two rows with the same digest ended in exactly the same position.
    pub final_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseSummary {
    pub case_id: String,
    pub runs: u32,
    /// How many runs ended in each outcome.
    pub outcomes: BTreeMap<String, u32>,
    /// Creatures lost over all runs, by side.
    pub casualties: BTreeMap<String, u64>,
    /// Replacement cost over all runs, by side.
    pub replacement_cost_reference: BTreeMap<String, u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    pub schema_version: u32,
    pub suite_id: String,
    pub identity: Identity,
    pub coverage: Coverage,
    /// Ordered by case as in the suite, then by seed as in the suite.
    pub rows: Vec<Row>,
    pub summary: Vec<CaseSummary>,
    /// Defects found while running. A report with any is not a valid result.
    pub problems: Vec<String>,
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// Reads a suite file and the files it names.
///
/// # Errors
/// A message naming the file and what is wrong with it.
pub fn load(suite_path: &Path) -> Result<Loaded, String> {
    let text = read(suite_path)?;
    let suite: Suite =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", suite_path.display()))?;
    if suite.schema_version != REPORT_SCHEMA {
        return Err(format!(
            "suite schema {} is not {REPORT_SCHEMA}",
            suite.schema_version
        ));
    }
    if suite.pilot.id != APPROACH_V1 {
        return Err(format!(
            "pilot {:?} is not implemented; only {APPROACH_V1:?} is",
            suite.pilot.id
        ));
    }
    let planned = u32::try_from(suite.cases.len() * suite.battle_seeds.len()).unwrap_or(u32::MAX);
    if suite.planned_runs != planned {
        return Err(format!(
            "planned_runs is {}, cases times seeds is {planned}",
            suite.planned_runs
        ));
    }
    let dir = suite_path.parent().unwrap_or_else(|| Path::new("."));
    let rules_text = read(&dir.join(&suite.rules_path))?;
    let rules = Rules::from_json(&rules_text).map_err(|e| e.to_string())?;
    if rules.id != suite.ruleset_id {
        return Err(format!(
            "suite is for ruleset {:?}, the rules file is {:?}",
            suite.ruleset_id, rules.id
        ));
    }
    let scenario_text = read(&dir.join(&suite.scenarios_path))?;
    let scenarios = scenarios_from_json(&scenario_text, &rules).map_err(|e| e.to_string())?;
    for case in &suite.cases {
        if !scenarios.iter().any(|s| s.id == case.id) {
            return Err(format!("case {:?} has no scenario", case.id));
        }
    }
    let metrics_path = dir.join(&suite.metrics_path);
    let metrics: Metrics = serde_json::from_str(&read(&metrics_path)?)
        .map_err(|e| format!("{}: {e}", metrics_path.display()))?;
    for troop in &rules.troops {
        if !metrics.reference_prices.contains_key(&troop.id) {
            return Err(format!("no reference price for troop {:?}", troop.id));
        }
    }
    Ok(Loaded {
        suite,
        rules: Arc::new(rules),
        scenarios,
        prices: metrics.reference_prices,
        rules_hash: sha256_hex(rules_text.as_bytes()),
        scenario_hash: sha256_hex(scenario_text.as_bytes()),
    })
}

fn counts(battle: &Battle) -> Counts {
    let mut counts = Counts::new();
    for stack in battle.stacks() {
        let side = counts.entry(stack.side.name().to_owned()).or_default();
        *side.entry(stack.troop_id.clone()).or_default() += stack.count;
    }
    counts
}

/// Plays one battle to its end, the pilot on both sides.
fn play(
    loaded: &Loaded,
    scenario: &Scenario,
    seed: u32,
    problems: &mut Vec<String>,
) -> Result<Row, String> {
    let here = format!("{} seed {seed}", scenario.id);
    let mut battle = Battle::start(Arc::clone(&loaded.rules), scenario, Some(seed))
        .map_err(|e| e.to_string())?;
    let start = counts(&battle);
    let shots = |b: &Battle| b.stacks().iter().map(|s| s.shots).sum::<u32>();
    let charges = |b: &Battle| b.hero_charges().values().sum::<u32>();
    let (shots_at_start, charges_at_start) = (shots(&battle), charges(&battle));

    let mut invalid_commands = 0;
    while battle.outcome().is_none() {
        let Some(command) = approach_v1(&battle.observe()) else {
            break;
        };
        if let Err(refused) = battle.apply(&command) {
            invalid_commands += 1;
            problems.push(format!(
                "{here}: the pilot's command was refused: {refused}"
            ));
            break;
        }
        if battle.revision() >= u64::from(COMMAND_BUDGET) {
            problems.push(format!("{here}: stopped after {COMMAND_BUDGET} commands"));
            break;
        }
    }

    let outcome = battle.outcome().map_or("unfinished", Outcome::name);
    let survivors = counts(&battle);
    let mut casualties = Counts::new();
    let mut cost = BTreeMap::new();
    for side in [Side::Player, Side::Enemy] {
        let name = side.name().to_owned();
        let mut lost = BTreeMap::new();
        let mut value = 0_u64;
        for (troop, before) in start.get(&name).into_iter().flatten() {
            let after = survivors
                .get(&name)
                .and_then(|s| s.get(troop))
                .copied()
                .unwrap_or(0);
            let n = before.saturating_sub(after);
            value += u64::from(n) * loaded.prices.get(troop).copied().unwrap_or(0);
            lost.insert(troop.clone(), n);
        }
        casualties.insert(name.clone(), lost);
        cost.insert(name, value);
    }
    // A battle won in round n completed n-1 rounds; the round limit ends it after the last round was completed.
    let completed_rounds = if battle.outcome() == Some(Outcome::BudgetExceeded) {
        battle.round()
    } else {
        battle.round() - 1
    };
    Ok(Row {
        case_id: scenario.id.clone(),
        seed,
        outcome: outcome.to_owned(),
        survivors,
        casualties,
        replacement_cost_reference: cost,
        completed_rounds,
        accepted_actions: u32::try_from(battle.revision()).unwrap_or(u32::MAX),
        ammunition_used: shots_at_start - shots(&battle),
        spell_charges_used: charges_at_start - charges(&battle),
        invalid_commands,
        final_digest: battle.digest(),
    })
}

fn summarise(case_id: &str, rows: &[Row]) -> CaseSummary {
    let mut summary = CaseSummary {
        case_id: case_id.to_owned(),
        runs: 0,
        outcomes: BTreeMap::new(),
        casualties: BTreeMap::new(),
        replacement_cost_reference: BTreeMap::new(),
    };
    for row in rows.iter().filter(|r| r.case_id == case_id) {
        summary.runs += 1;
        *summary.outcomes.entry(row.outcome.clone()).or_default() += 1;
        for (side, lost) in &row.casualties {
            *summary.casualties.entry(side.clone()).or_default() +=
                lost.values().map(|&n| u64::from(n)).sum::<u64>();
        }
        for (side, value) in &row.replacement_cost_reference {
            *summary
                .replacement_cost_reference
                .entry(side.clone())
                .or_default() += value;
        }
    }
    summary
}

/// Runs the suite, or only the cases named in `only`. Every battle is played twice; the two must end in the same
/// position, or the run is not repeatable and says so.
///
/// # Errors
/// `only` names a case the suite does not have, or a scenario cannot be started.
pub fn run(loaded: &Loaded, only: &[String], engine_revision: &str) -> Result<Report, String> {
    let suite = &loaded.suite;
    if let Some(unknown) = only
        .iter()
        .find(|id| !suite.cases.iter().any(|c| c.id == **id))
    {
        return Err(format!("the suite has no case {unknown:?}"));
    }
    let chosen = |case: &&Case| only.is_empty() || only.contains(&case.id);
    let mut rows = Vec::new();
    let mut summary = Vec::new();
    let mut problems = Vec::new();
    for case in suite.cases.iter().filter(chosen) {
        let Some(scenario) = loaded.scenarios.iter().find(|s| s.id == case.id) else {
            continue;
        };
        for &seed in &suite.battle_seeds {
            let row = play(loaded, scenario, seed, &mut problems)?;
            let again = play(loaded, scenario, seed, &mut Vec::new())?;
            if row != again {
                problems.push(format!(
                    "{} seed {seed}: a second run ended differently",
                    case.id
                ));
            }
            if !case.allowed_outcomes.contains(&row.outcome) {
                problems.push(format!(
                    "{} seed {seed}: outcome {} is not one of {:?}",
                    case.id, row.outcome, case.allowed_outcomes
                ));
            }
            rows.push(row);
        }
        summary.push(summarise(&case.id, &rows));
    }
    Ok(Report {
        schema_version: REPORT_SCHEMA,
        suite_id: suite.suite_id.clone(),
        identity: Identity {
            engine_revision: engine_revision.to_owned(),
            ruleset_id: loaded.rules.id.clone(),
            rules_hash: loaded.rules_hash.clone(),
            scenario_hash: loaded.scenario_hash.clone(),
            pilot_version: suite.pilot.id.clone(),
            seed_list: suite.battle_seeds.clone(),
        },
        coverage: Coverage {
            planned: suite.planned_runs,
            executed: u32::try_from(rows.len()).unwrap_or(u32::MAX),
            cases_not_run: suite
                .cases
                .iter()
                .filter(|c| !chosen(c))
                .map(|c| c.id.clone())
                .collect(),
        },
        rows,
        summary,
        problems,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn suite_path() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../balance/suite.json")
    }

    pub(crate) fn melee_report() -> Report {
        run(
            &load(&suite_path()).unwrap(),
            &["melee_exchange".to_owned()],
            "test",
        )
        .unwrap()
    }

    #[test]
    fn the_supplied_suite_loads() {
        let loaded = load(&suite_path()).unwrap();
        assert_eq!(loaded.suite.cases.len(), 4);
        assert_eq!(loaded.suite.battle_seeds, [1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(loaded.prices["skyrider"], 50);
        assert_eq!(loaded.rules_hash.len(), 64);
    }

    // AT-070, AT-071: real rows from a finished battle, the same every time.
    #[test]
    fn a_melee_case_gives_one_valid_row_for_every_seed() {
        let report = melee_report();
        assert_eq!(report.problems, Vec::<String>::new());
        assert_eq!(report.rows.len(), 8);
        assert_eq!((report.coverage.planned, report.coverage.executed), (32, 8));
        assert_eq!(
            report.coverage.cases_not_run,
            ["blocked_shooter", "flight_barrier", "unreachable_melee"]
        );
        for row in &report.rows {
            assert!(
                ["player_win", "enemy_win"].contains(&row.outcome.as_str()),
                "{row:?}"
            );
            let loser = if row.outcome == "player_win" {
                "enemy"
            } else {
                "player"
            };
            assert_eq!(row.survivors[loser]["warden"], 0);
            assert_eq!(row.casualties[loser]["warden"], 10);
            assert_eq!(row.replacement_cost_reference[loser], 200);
            assert_eq!(row.invalid_commands, 0);
        }
        assert_eq!(report.summary[0].runs, 8);
        assert_eq!(report.summary[0].outcomes.values().sum::<u32>(), 8);
        assert_eq!(
            report,
            melee_report(),
            "a second run of the suite gives the same report"
        );
    }

    // AT-074, AT-080: the whole suite runs, and the unreachable case ends as the diagnosis it expects.
    #[test]
    fn the_whole_suite_runs_without_problems() {
        let report = run(&load(&suite_path()).unwrap(), &[], "test").unwrap();
        assert_eq!(report.problems, Vec::<String>::new());
        assert_eq!(
            (report.coverage.planned, report.coverage.executed),
            (32, 32)
        );
        assert_eq!(report.coverage.cases_not_run, Vec::<String>::new());
        let unreachable = report
            .summary
            .iter()
            .find(|c| c.case_id == "unreachable_melee")
            .unwrap();
        assert_eq!(unreachable.outcomes["budget_exceeded"], 8);
        assert_eq!(unreachable.casualties.values().sum::<u64>(), 0);
        for case in report
            .summary
            .iter()
            .filter(|c| c.case_id != "unreachable_melee")
        {
            assert!(!case.outcomes.contains_key("budget_exceeded"), "{case:?}");
        }
    }

    // B-REG-004: the code as it stands gives exactly the recorded baseline. A change that is meant to move results
    // replaces the baseline in the same pull request and says there what moved and why (bounty/balance/README.md);
    // a change that is not meant to must leave this test alone.
    #[test]
    fn each_suite_gives_its_recorded_baseline() {
        for (file, battles) in [("suite.json", 32), ("reference-suite.json", 40)] {
            let path = suite_path().with_file_name(file);
            let loaded = load(&path).unwrap();
            let baseline = path.with_file_name(loaded.suite.baseline_path.as_ref().unwrap());
            let baseline: Report =
                serde_json::from_str(&fs::read_to_string(baseline).unwrap()).unwrap();
            let comparison =
                crate::compare::compare(&baseline, &run(&loaded, &[], "test").unwrap());
            assert!(
                comparison.same_results(),
                "{file}: results moved against the baseline:\n{}",
                comparison.text()
            );
            assert_eq!(comparison.compared, battles, "{file}");
        }
    }

    // The cases for the reference rules run to an end without problems, and the one with a fixed answer gives it.
    #[test]
    fn the_reference_suite_runs_without_problems() {
        let path = suite_path().with_file_name("reference-suite.json");
        let loaded = load(&path).unwrap();
        let report = run(&loaded, &[], "test").unwrap();
        assert_eq!(report.problems, Vec::<String>::new());
        assert_eq!(
            (report.coverage.planned, report.coverage.executed),
            (40, 40)
        );
        assert_eq!(report.identity.ruleset_id, "reference-v0.1");
        let overmatch = report
            .summary
            .iter()
            .find(|c| c.case_id == "overmatch")
            .unwrap();
        assert_eq!(overmatch.outcomes["player_win"], 8);
        assert_eq!(overmatch.casualties["player"], 0);
        assert_eq!(
            report,
            run(&loaded, &[], "test").unwrap(),
            "a second run gives the same report"
        );
    }

    // AT-073, AT-074: an outcome the case does not allow is a problem, not a quiet row.
    #[test]
    fn an_outcome_the_case_does_not_allow_is_a_problem() {
        let mut loaded = load(&suite_path()).unwrap();
        loaded.suite.cases[0].allowed_outcomes = vec!["budget_exceeded".to_owned()];
        let report = run(&loaded, &["melee_exchange".to_owned()], "test").unwrap();
        assert_eq!(report.problems.len(), 8);
        assert!(report.problems[0].contains("is not one of"));
    }

    #[test]
    fn a_case_the_suite_does_not_have_is_refused() {
        let loaded = load(&suite_path()).unwrap();
        assert!(run(&loaded, &["siege".to_owned()], "test").is_err());
    }

    #[test]
    fn a_report_survives_being_written_and_read() {
        let report = melee_report();
        let text = serde_json::to_string_pretty(&report).unwrap();
        assert_eq!(serde_json::from_str::<Report>(&text).unwrap(), report);
    }
}
