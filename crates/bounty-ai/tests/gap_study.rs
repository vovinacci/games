//! Measurements behind `bounty/docs/spec/10-GAMEPLAY-GAPS.md`: battles of the reference rules, the pilot on both
//! sides, 500 seeds each. They print tables and assert nothing; run them by hand:
//!
//! ```sh
//! cargo test --release -p bounty-ai --test gap_study -- --ignored --nocapture
//! ```
//!
//! `measure` prints, for the reference rules, "player wins of 100, mean rounds, share of its creatures the winner
//! keeps". `measure_the_variant` sets the reference beside each switch of the `modern` variant and beside all four.

use std::collections::BTreeMap;
use std::sync::Arc;

use bounty_ai::approach_v1;
use bounty_core::battle::{Battle, Outcome};
use bounty_core::content::{Rules, Scenario, Side, StackStart};

const SEEDS: u32 = 500;

type Army = [(&'static str, u32)];

fn rules() -> Arc<Rules> {
    Arc::new(Rules::from_json(include_str!("../../../content/reference/rules.json")).unwrap())
}

/// The player's stacks down the left edge, the enemy's down the right, as an encounter sets them up.
fn scenario(player: &Army, enemy: &Army, leadership: Option<u32>, seed: u32) -> Scenario {
    let line = |army: &Army, side: Side, x: u32| {
        let prefix = if side == Side::Player { 'p' } else { 'e' };
        let stack = move |(slot, (troop, count)): (usize, &(&str, u32))| StackStart {
            id: format!("{prefix}{slot}"),
            side,
            slot: slot as u32,
            troop_id: (*troop).to_owned(),
            count: *count,
            injury: 0,
            x,
            y: slot as u32,
        };
        army.iter().enumerate().map(stack).collect::<Vec<_>>()
    };
    let mut stacks = line(player, Side::Player, 0);
    stacks.extend(line(enemy, Side::Enemy, 5));
    Scenario {
        id: "study".to_owned(),
        rng_state: seed,
        obstacles: Vec::new(),
        hero_charges: BTreeMap::new(),
        leadership,
        stacks,
    }
}

fn play(player: &Army, enemy: &Army, leadership: Option<u32>) -> String {
    let (wins, rounds, kept) = play_under(&rules(), player, enemy, leadership);
    format!("{wins:>3} wins, {rounds:>4.1} rounds, winner keeps {kept:>2.0}%")
}

/// Player wins of 100, mean rounds, and the share of its creatures the winner keeps, in 100.
fn play_under(
    rules: &Arc<Rules>,
    player: &Army,
    enemy: &Army,
    leadership: Option<u32>,
) -> (u32, f64, f64) {
    let (mut wins, mut rounds, mut kept) = (0, 0.0, 0.0);
    for seed in 1..=SEEDS {
        let start = scenario(player, enemy, leadership, seed);
        let mut battle = Battle::start(Arc::clone(rules), &start, None).unwrap();
        while let Some(command) = approach_v1(&battle.observe()) {
            battle.apply(&command).unwrap();
        }
        let won = battle.outcome() == Some(Outcome::PlayerWin);
        let (side, army) = if won {
            (Side::Player, player)
        } else {
            (Side::Enemy, enemy)
        };
        let before: u32 = army.iter().map(|(_, count)| count).sum();
        let after: u32 = battle
            .stacks()
            .iter()
            .filter(|s| s.side == side)
            .map(|s| s.count)
            .sum();
        wins += u32::from(won);
        rounds += f64::from(battle.round());
        kept += f64::from(after) / f64::from(before);
    }
    let seeds = f64::from(SEEDS);
    (wins * 100 / SEEDS, rounds / seeds, kept * 100.0 / seeds)
}

const MODERN: &str = include_str!("../../../content/modern/rules.json");

/// Each switch as the rules files write it: its settings by name, the reference's first, under the empty name.
const SWITCHES: [&[(&str, &str)]; 4] = [
    &[
        (
            "",
            r#""count_used": "count_before_the_exchange",
    "retaliation_uses_survivors": false,"#,
        ),
        (
            "answer",
            r#""count_used": "count_at_the_blow",
    "retaliation_uses_survivors": true,"#,
        ),
        (
            "answer-mean",
            r#""count_used": "mean_of_before_and_after_the_blow",
    "retaliation_uses_survivors": false,"#,
        ),
    ],
    &[
        ("", r#""rolls": "one""#),
        ("rolls", r#""rolls": "mean_of_two""#),
    ],
    &[
        (
            "",
            r#""rule": "lowest_rank_over_own_side_self_included_chart_read_other_then_self""#,
        ),
        (
            "morale",
            r#""rule": "alone_high_else_others_read_self_then_other_any_low_is_low_all_high_is_high""#,
        ),
    ],
    &[
        ("", r#""over_leadership": "out_of_control""#),
        (
            "control",
            r#""over_leadership": "only_what_leadership_covers_fights""#,
        ),
    ],
];

/// The variant's rules with exactly the named settings; every switch not named is as the reference has it,
/// whatever the variant's own file says.
fn modern(on: &[&str]) -> Arc<Rules> {
    let mut text = MODERN.to_owned();
    for settings in SWITCHES {
        let now = settings
            .iter()
            .find(|(_, written)| text.contains(written))
            .expect("the file sets every switch");
        let wanted = settings
            .iter()
            .find(|(name, _)| on.contains(name))
            .unwrap_or(&settings[0]);
        text = text.replacen(now.1, wanted.1, 1);
    }
    Arc::new(Rules::from_json(&text).unwrap())
}

/// The settings set beside each other: the reference, each switch alone, and the variant as its file has it.
const COLUMNS: [(&str, &[&str]); 7] = [
    ("refer.", &[]),
    ("answer", &["answer"]),
    ("ans.mean", &["answer-mean"]),
    ("rolls", &["rolls"]),
    ("morale", &["morale"]),
    ("control", &["control"]),
    ("modern", &["morale", "control"]),
];

/// One case under every column: "wins/keeps" for each.
fn across(title: &str, player: &Army, enemy: &Army, leadership: Option<u32>) {
    let cell = |(_, on): &(&str, &[&str])| {
        let (wins, _, kept) = play_under(&modern(on), player, enemy, leadership);
        format!("{wins:>3}/{kept:<3.0}")
    };
    let cells: Vec<String> = COLUMNS.iter().map(cell).collect();
    println!("  {title:<44} {}", cells.join("  "));
}

#[test]
#[ignore = "a measurement to run by hand, see the top of the file"]
fn measure_the_variant() {
    println!("\nPlayer wins of 100 / share of its creatures the winner keeps, under:");
    let heads: Vec<String> = COLUMNS
        .iter()
        .map(|(head, _)| format!("{head:<7}"))
        .collect();
    println!("  {:<44} {}", "", heads.join("  "));
    println!(" What moving first is worth: the same army on both sides, no hero");
    across("20 Wolves", &[("wolves", 20)], &[("wolves", 20)], None);
    across("60 Sprites", &[("sprites", 60)], &[("sprites", 60)], None);
    across("6 Archers", &[("archers", 6)], &[("archers", 6)], None);
    let mixed: &Army = &[("wolves", 20), ("archers", 6), ("sprites", 40)];
    across("20 Wolves, 6 Archers, 40 Sprites", mixed, mixed, None);
    println!(" How often the stronger army wins: a tenth more creatures, no hero");
    across(
        "22 Wolves on 20, the stronger moves first",
        &[("wolves", 22)],
        &[("wolves", 20)],
        None,
    );
    across(
        "20 Wolves on 22, the weaker moves first",
        &[("wolves", 20)],
        &[("wolves", 22)],
        None,
    );
    across(
        "66 Sprites on 60",
        &[("sprites", 66)],
        &[("sprites", 60)],
        None,
    );
    across(
        "60 Sprites on 66",
        &[("sprites", 60)],
        &[("sprites", 66)],
        None,
    );
    println!(" Morale: the same army on both sides, the player's with a hero");
    let pair: &Army = &[("sprites", 60), ("wolves", 10)];
    across("60 Sprites and 10 Wolves", pair, pair, Some(100_000));
    across(
        "20 Wolves",
        &[("wolves", 20)],
        &[("wolves", 20)],
        Some(100_000),
    );
    across(
        "60 Sprites",
        &[("sprites", 60)],
        &[("sprites", 60)],
        Some(100_000),
    );
    println!(" Leadership: 20 Wolves (60 health), 4 Archers against 15 Wolves, 3 Archers");
    for leadership in [60, 59, 45] {
        let (player, enemy): (&Army, &Army) = (
            &[("wolves", 20), ("archers", 4)],
            &[("wolves", 15), ("archers", 3)],
        );
        across(
            &format!("leadership {leadership}"),
            player,
            enemy,
            Some(leadership),
        );
    }
    println!(" Leadership: 40 Wolves (120 health), 6 Archers, 30 Sprites against 30, 6, 30");
    for leadership in [120, 100, 61] {
        let player: &Army = &[("wolves", 40), ("archers", 6), ("sprites", 30)];
        let enemy: &Army = &[("wolves", 30), ("archers", 6), ("sprites", 30)];
        across(
            &format!("leadership {leadership}"),
            player,
            enemy,
            Some(leadership),
        );
    }
}

#[test]
#[ignore = "a measurement to run by hand, see the top of the file"]
fn measure() {
    println!("\n1. The same army on both sides, no hero: what moving first is worth");
    let armies: [&Army; 4] = [
        &[("wolves", 20)],
        &[("sprites", 60)],
        &[("archers", 6)],
        &[("wolves", 20), ("archers", 6), ("sprites", 40)],
    ];
    for army in armies {
        println!("  {army:?}: {}", play(army, army, None));
    }

    println!("\n2. The same army on both sides: without a hero, then with one (so with morale)");
    let armies: [&Army; 4] = [
        &[("sprites", 60), ("wolves", 10)],
        &[("sprites", 60)],
        &[("wolves", 20)],
        &[("sprites", 60), ("archers", 4)],
    ];
    for army in armies {
        println!(
            "  {army:?}\n    no hero: {}\n    hero:    {}",
            play(army, army, None),
            play(army, army, Some(100_000))
        );
    }

    for (title, armies) in [
        (
            "3. Armies of equal price, 3000 gold, no hero; the first named moves first",
            [("wolves", 75), ("sprites", 200), ("archers", 12)],
        ),
        (
            "4. Armies of equal health, 300, which is equal leadership; no hero",
            [("wolves", 100), ("sprites", 300), ("archers", 30)],
        ),
    ] {
        println!("\n{title}");
        for player in armies {
            for enemy in armies.into_iter().filter(|enemy| enemy.0 != player.0) {
                println!(
                    "  {player:?} against {enemy:?}: {}",
                    play(&[player], &[enemy], None)
                );
            }
        }
    }

    println!(
        "\n5. Loss of control: 20 Wolves (60 health) and 4 Archers against 15 Wolves and 3 Archers"
    );
    for leadership in [60, 59, 45] {
        let (player, enemy): (&Army, &Army) = (
            &[("wolves", 20), ("archers", 4)],
            &[("wolves", 15), ("archers", 3)],
        );
        println!(
            "  leadership {leadership}: {}",
            play(player, enemy, Some(leadership))
        );
    }
    println!(
        "\n6. Deep loss of control: 40 Wolves (120 health), 6 Archers, 30 Sprites against 30, 6, 30"
    );
    for leadership in [120, 100, 61] {
        let player: &Army = &[("wolves", 40), ("archers", 6), ("sprites", 30)];
        let enemy: &Army = &[("wolves", 30), ("archers", 6), ("sprites", 30)];
        println!(
            "  leadership {leadership}: {}",
            play(player, enemy, Some(leadership))
        );
    }
}
