//! Compares two reports of the same suite: what was run differently, and which battles ended differently.

use std::collections::BTreeMap;
use std::fmt::Write;

use crate::balance::{REPORT_SCHEMA, Report, Row};

/// The result of a comparison.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Comparison {
    /// Why the two reports cannot be compared at all. When not empty, nothing else is filled in.
    pub not_comparable: Vec<String>,
    /// Differences in what was run: revision, rules, scenarios. Expected when a rules change is being measured.
    pub identity: Vec<String>,
    /// One line for every battle that ended differently.
    pub rows: Vec<String>,
    /// Battles compared.
    pub compared: usize,
}

impl Comparison {
    /// True when every battle ended in exactly the same position.
    #[must_use]
    pub fn same_results(&self) -> bool {
        self.not_comparable.is_empty() && self.rows.is_empty()
    }

    /// The comparison as text for a person.
    #[must_use]
    pub fn text(&self) -> String {
        let mut out = String::new();
        if !self.not_comparable.is_empty() {
            out.push_str("NOT COMPARABLE\n");
            for why in &self.not_comparable {
                let _ = writeln!(out, "  {why}");
            }
            return out;
        }
        for line in &self.identity {
            let _ = writeln!(out, "run differs: {line}");
        }
        let _ = writeln!(
            out,
            "{} battles compared, {} ended differently",
            self.compared,
            self.rows.len()
        );
        for line in &self.rows {
            let _ = writeln!(out, "  {line}");
        }
        out
    }
}

fn changed<T: PartialEq + std::fmt::Debug>(what: &str, a: &T, b: &T, into: &mut Vec<String>) {
    if a != b {
        into.push(format!("{what}: {a:?} -> {b:?}"));
    }
}

fn row_difference(a: &Row, b: &Row) -> Option<String> {
    if a == b {
        return None;
    }
    let mut parts = Vec::new();
    changed("outcome", &a.outcome, &b.outcome, &mut parts);
    changed("survivors", &a.survivors, &b.survivors, &mut parts);
    changed(
        "rounds",
        &a.completed_rounds,
        &b.completed_rounds,
        &mut parts,
    );
    changed(
        "actions",
        &a.accepted_actions,
        &b.accepted_actions,
        &mut parts,
    );
    changed(
        "ammunition",
        &a.ammunition_used,
        &b.ammunition_used,
        &mut parts,
    );
    changed(
        "spell charges",
        &a.spell_charges_used,
        &b.spell_charges_used,
        &mut parts,
    );
    changed(
        "invalid commands",
        &a.invalid_commands,
        &b.invalid_commands,
        &mut parts,
    );
    changed(
        "cost",
        &a.replacement_cost_reference,
        &b.replacement_cost_reference,
        &mut parts,
    );
    if parts.is_empty() {
        parts.push("same numbers, another final position".to_owned());
    }
    Some(format!(
        "{} seed {}: {}",
        a.case_id,
        a.seed,
        parts.join("; ")
    ))
}

/// Compares a baseline with a candidate. They must be of the same suite, seeds, pilot and cases, and both free of
/// problems; a different revision or rules file is reported and compared all the same, since that is what a change
/// looks like.
#[must_use]
pub fn compare(baseline: &Report, candidate: &Report) -> Comparison {
    let mut not_comparable = Vec::new();
    for (name, report) in [("baseline", baseline), ("candidate", candidate)] {
        if report.schema_version != REPORT_SCHEMA {
            not_comparable.push(format!(
                "{name} has schema {}, not {REPORT_SCHEMA}",
                report.schema_version
            ));
        }
        if !report.problems.is_empty() {
            not_comparable.push(format!(
                "{name} has {} problems; first: {}",
                report.problems.len(),
                report.problems[0]
            ));
        }
    }
    changed(
        "suite",
        &baseline.suite_id,
        &candidate.suite_id,
        &mut not_comparable,
    );
    changed(
        "seeds",
        &baseline.identity.seed_list,
        &candidate.identity.seed_list,
        &mut not_comparable,
    );
    changed(
        "pilot",
        &baseline.identity.pilot_version,
        &candidate.identity.pilot_version,
        &mut not_comparable,
    );
    let keys = |r: &Report| {
        r.rows
            .iter()
            .map(|row| (row.case_id.clone(), row.seed))
            .collect::<Vec<_>>()
    };
    if keys(baseline) != keys(candidate) {
        not_comparable.push(format!(
            "the runs differ: {} battles in the baseline, {} in the candidate, or other cases",
            baseline.rows.len(),
            candidate.rows.len()
        ));
    }
    if !not_comparable.is_empty() {
        return Comparison {
            not_comparable,
            identity: Vec::new(),
            rows: Vec::new(),
            compared: 0,
        };
    }

    let mut identity = Vec::new();
    let (a, b) = (&baseline.identity, &candidate.identity);
    changed(
        "engine revision",
        &a.engine_revision,
        &b.engine_revision,
        &mut identity,
    );
    changed("ruleset", &a.ruleset_id, &b.ruleset_id, &mut identity);
    changed("rules file", &a.rules_hash, &b.rules_hash, &mut identity);
    changed(
        "scenario file",
        &a.scenario_hash,
        &b.scenario_hash,
        &mut identity,
    );

    let theirs: BTreeMap<(&str, u32), &Row> = candidate
        .rows
        .iter()
        .map(|r| ((r.case_id.as_str(), r.seed), r))
        .collect();
    let rows = baseline
        .rows
        .iter()
        .filter_map(|row| row_difference(row, theirs[&(row.case_id.as_str(), row.seed)]))
        .collect();
    Comparison {
        not_comparable,
        identity,
        rows,
        compared: baseline.rows.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::balance::tests::melee_report;

    #[test]
    fn a_report_equals_itself() {
        let report = melee_report();
        let comparison = compare(&report, &report);
        assert!(comparison.same_results());
        assert_eq!(comparison.compared, 8);
        assert!(
            comparison
                .text()
                .contains("8 battles compared, 0 ended differently")
        );
    }

    // AT-079: a changed result is listed by case and seed with what moved.
    #[test]
    fn a_changed_battle_is_listed_with_what_changed() {
        let baseline = melee_report();
        let mut candidate = baseline.clone();
        candidate.identity.engine_revision = "other".to_owned();
        candidate.identity.rules_hash = "0".repeat(64);
        candidate.rows[2].outcome = "budget_exceeded".to_owned();
        candidate.rows[2].completed_rounds = 200;
        let comparison = compare(&baseline, &candidate);
        assert!(!comparison.same_results());
        assert_eq!(comparison.identity.len(), 2);
        assert_eq!(comparison.rows.len(), 1);
        assert!(
            comparison.rows[0].starts_with("melee_exchange seed 3: outcome: "),
            "{}",
            comparison.rows[0]
        );
        assert!(comparison.rows[0].contains("rounds: "));
    }

    // AT-072, AT-073, AT-076: what must not be compared is refused, not passed.
    #[test]
    fn reports_that_do_not_belong_together_are_not_compared() {
        let baseline = melee_report();
        let refused = |change: fn(&mut Report)| {
            let mut candidate = melee_report();
            change(&mut candidate);
            let comparison = compare(&baseline, &candidate);
            assert!(!comparison.same_results());
            assert!(comparison.text().starts_with("NOT COMPARABLE"));
            comparison.not_comparable.len()
        };
        assert_eq!(refused(|r| r.identity.seed_list.push(9)), 1);
        assert_eq!(
            refused(|r| r.identity.pilot_version = "approach-v2".to_owned()),
            1
        );
        assert_eq!(refused(|r| r.suite_id = "other".to_owned()), 1);
        assert_eq!(refused(|r| drop(r.rows.pop())), 1);
        assert_eq!(refused(|r| r.rows[0].seed = 99), 1);
        assert_eq!(
            refused(|r| r.problems.push("a refused command".to_owned())),
            1
        );
        assert_eq!(refused(|r| r.schema_version = 1), 1);
    }
}
