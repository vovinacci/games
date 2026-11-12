//! Measurements for `docs/spec/10-GAMEPLAY-GAPS.md`: the second expedition played by the expedition pilot over
//! many seeds, under the reference rules, the variant, and the variant with each campaign switch. Not a test of
//! anything: run it for its table.
//!
//! `cargo test --release -p bounty-tools --test expedition_study -- --ignored --nocapture`

use std::path::Path;
use std::sync::Arc;

use bounty_core::campaign::ExpeditionDef;
use bounty_core::content::{Profile, Rules, Unpaid};
use bounty_tools::expedition::{Row, load_under, play_route};

const SEEDS: u32 = 500;

fn meadow(rules: &str) -> ExpeditionDef {
    let content = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let rules = content.join(rules).join("rules.json");
    let def = load_under(&content.join("meadow/expedition.json"), Some(&rules)).unwrap();
    (*def).clone()
}

/// The expedition with its rules' campaign switches set, and every battle started from another random state.
fn with(base: &ExpeditionDef, leave: bool, returning: u32, seed: u32) -> Arc<ExpeditionDef> {
    let mut def = base.clone();
    let mut rules: Rules = (*def.rules).clone();
    if let Profile::Reference(reference) = &mut rules.profile {
        reference.switches.unpaid = if leave { Unpaid::Leave } else { Unpaid::Stay };
        reference.switches.fallen_return_percent = returning;
    }
    def.rules = Arc::new(rules);
    for (band, place) in def.encounters.iter_mut().zip(1..) {
        band.battle_rng_state = seed.wrapping_mul(7919).wrapping_add(place);
    }
    Arc::new(def)
}

fn rows(base: &ExpeditionDef, leave: bool, returning: u32) -> Vec<Row> {
    let play = |seed| {
        let def = with(base, leave, returning, seed);
        play_route(&def, &def.test_routes[0], &mut |_| ())
            .unwrap()
            .1
    };
    (0..SEEDS).map(play).collect()
}

#[test]
#[ignore = "a measurement, run by hand"]
fn measure() {
    let (reference, modern) = (meadow("reference"), meadow("modern"));
    let columns: [(&str, &ExpeditionDef, bool, u32); 7] = [
        ("reference", &reference, false, 0),
        ("modern", &modern, false, 0),
        ("+return 25", &modern, false, 25),
        ("+return 50", &modern, false, 50),
        ("+leave", &modern, true, 0),
        ("+leave, 25", &modern, true, 25),
        ("+leave, 50", &modern, true, 50),
    ];
    println!(
        "{:<12} {:>5} {:>5} {:>6} {:>6} {:>7} {:>7} {:>6} {:>6} {:>6}",
        "", "won", "late", "ticks", "fights", "recov.", "lost", "gold", "army", "left"
    );
    for (name, base, leave, returning) in columns {
        let rows = rows(base, leave, returning);
        let count = |f: &dyn Fn(&Row) -> bool| rows.iter().filter(|row| f(row)).count();
        let mean = |f: &dyn Fn(&Row) -> u64| rows.iter().map(f).sum::<u64>() / u64::from(SEEDS);
        let won: Vec<&Row> = rows.iter().filter(|row| row.status == "victory").collect();
        let ticks = won
            .iter()
            .map(|row| u64::from(row.elapsed_ticks))
            .sum::<u64>();
        println!(
            "{name:<12} {:>5} {:>5} {:>6} {:>6} {:>7} {:>7} {:>6} {:>6} {:>6}",
            won.len(),
            count(&|row| row.status == "timeout"),
            ticks / (won.len() as u64).max(1),
            mean(&|row| u64::from(row.battles) * 100),
            mean(&|row| u64::from(row.recoveries) * 100),
            mean(&|row| row.casualties_cost),
            mean(&|row| row.gold_end),
            mean(&|row| row.army_end.values().sum()),
            count(&|row| !row.left_unpaid.is_empty()),
        );
    }
    println!(
        "won, late: runs of {SEEDS}; ticks: mean of the won runs; fights, recov.: per 100 runs;"
    );
    println!(
        "lost: mean casualties at the town's prices; gold, army: mean at the end; left: runs in which a stack left unpaid"
    );
}
