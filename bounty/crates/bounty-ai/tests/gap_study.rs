//! Measurements behind `bounty/docs/spec/10-GAMEPLAY-GAPS.md`: battles of the reference rules, the pilot on both
//! sides, 500 seeds each. They print tables and assert nothing; run them by hand:
//!
//! ```sh
//! cargo test --release -p bounty-ai --test gap_study -- --ignored --nocapture
//! ```
//!
//! Every line is "player wins of 100, mean rounds, share of its creatures the winner keeps".

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
    let rules = rules();
    let (mut wins, mut rounds, mut kept) = (0, 0.0, 0.0);
    for seed in 1..=SEEDS {
        let start = scenario(player, enemy, leadership, seed);
        let mut battle = Battle::start(Arc::clone(&rules), &start, None).unwrap();
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
    format!(
        "{:>3} wins, {:>4.1} rounds, winner keeps {:>2.0}%",
        wins * 100 / SEEDS,
        rounds / seeds,
        kept * 100.0 / seeds
    )
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
