//! The second expedition, `content/meadow/`, on the reference's troops: the rules of the campaign that the demo
//! does not reach (upkeep by a troop's cost, a week that cannot be paid, the fallen who return, a band of several
//! stacks), the expedition pilot, and saving, under the reference rules and the variant.

use std::path::Path;
use std::sync::Arc;

use bounty_ai::errand_v1;
use bounty_core::campaign::{Action, Command, Event, Expedition, ExpeditionDef, Status};
use bounty_core::content::{Profile, Rules, Unpaid};
use bounty_core::save;
use bounty_tools::expedition::load_under;

fn meadow(rules: &str) -> ExpeditionDef {
    let content = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let rules = content.join(rules).join("rules.json");
    let def = load_under(&content.join("meadow/expedition.json"), Some(&rules)).unwrap();
    (*def).clone()
}

/// The expedition with the campaign switches of its rules set by hand.
fn switched(mut def: ExpeditionDef, leave: bool, returning: u32) -> ExpeditionDef {
    let mut rules: Rules = (*def.rules).clone();
    let Profile::Reference(reference) = &mut rules.profile else {
        panic!("the meadow is played under the reference rules or their variant");
    };
    reference.switches.unpaid = if leave { Unpaid::Leave } else { Unpaid::Stay };
    reference.switches.fallen_return_percent = returning;
    def.rules = Arc::new(rules);
    def
}

fn act(expedition: &mut Expedition, action: Action) -> Vec<Event> {
    let said = format!("{action:?}");
    let command = Command {
        revision: expedition.revision(),
        action,
    };
    expedition
        .apply(&command)
        .unwrap_or_else(|refused| panic!("{said} refused: {refused}"))
}

/// Plays on with the pilot until the expedition is over; every event, in order. No command of the pilot is ever
/// refused, and every state on the way comes back from its save as it was.
fn play_out(expedition: &mut Expedition) -> Vec<Event> {
    let mut events = Vec::new();
    for _ in 0..100_000 {
        let Some(action) = errand_v1(expedition) else {
            return events;
        };
        events.extend(act(expedition, action));
        let saved = save::encode(expedition);
        let loaded = save::decode(&saved, Arc::clone(expedition.def_shared())).unwrap();
        assert_eq!(&loaded, expedition);
    }
    panic!("the pilot does not come to an end");
}

fn count(expedition: &Expedition, troop: &str) -> u32 {
    let stacks = expedition.army().iter().filter(|s| s.troop_id == troop);
    stacks.map(|s| s.count).sum()
}

#[test]
fn the_variant_file_sets_the_campaign_switches_and_the_reference_has_none() {
    let switches = |def: &ExpeditionDef| match &def.rules.profile {
        Profile::Reference(reference) => reference.switches,
        Profile::Prototype => panic!("not the prototype"),
    };
    let (reference, modern) = (switches(&meadow("reference")), switches(&meadow("modern")));
    assert_eq!(
        (reference.unpaid, reference.fallen_return_percent),
        (Unpaid::Stay, 0)
    );
    assert_eq!(
        (modern.unpaid, modern.fallen_return_percent),
        (Unpaid::Stay, 25)
    );
}

// The reference's rule: a tenth of the recruiting cost a creature, rounded down, each week.
#[test]
fn upkeep_is_a_tenth_of_each_creatures_cost() {
    let mut e = Expedition::start(Arc::new(meadow("reference")));
    assert_eq!(e.upkeep(), 20, "twenty Sprites at 15: one each");
    act(
        &mut e,
        Action::Recruit {
            troop_id: "archers".to_owned(),
            quantity: 2,
        },
    );
    act(
        &mut e,
        Action::Recruit {
            troop_id: "wolves".to_owned(),
            quantity: 3,
        },
    );
    assert_eq!(e.upkeep(), 20 + 2 * 25 + 3 * 4);
    let gold = e.gold();
    let mut settled = Vec::new();
    for _ in 0..20 {
        settled.extend(act(&mut e, Action::WaitTick));
    }
    let week = Event::WeekSettled {
        week: 1,
        income: 500,
        upkeep: 82,
        gold: gold + 500 - 82,
    };
    assert!(settled.contains(&week), "{settled:?}");
}

/// The meadow with an army it cannot pay: twenty gold in the purse and this much income, against an upkeep of
/// 20 for the Sprites, 250 for the Archers and 48 for the Wolves.
fn broke(def: ExpeditionDef, income: u64) -> Expedition {
    let mut def = def;
    def.clock.weekly_income = income;
    let mut e = Expedition::start(Arc::new(def));
    act(
        &mut e,
        Action::Recruit {
            troop_id: "archers".to_owned(),
            quantity: 10,
        },
    );
    act(
        &mut e,
        Action::Recruit {
            troop_id: "wolves".to_owned(),
            quantity: 12,
        },
    );
    assert_eq!((e.gold(), e.upkeep()), (20, 20 + 250 + 48));
    e
}

fn a_week(expedition: &mut Expedition) -> Vec<Event> {
    (0..20)
        .flat_map(|_| act(expedition, Action::WaitTick))
        .collect()
}

#[test]
fn a_week_that_cannot_be_paid_empties_the_purse_and_everyone_stays() {
    let mut e = broke(switched(meadow("reference"), false, 0), 0);
    let events = a_week(&mut e);
    assert!(events.contains(&Event::WeekSettled {
        week: 1,
        income: 0,
        upkeep: 20,
        gold: 0
    }));
    assert_eq!((e.gold(), e.status()), (0, Status::Adventure));
    assert_eq!(
        (
            count(&e, "sprites"),
            count(&e, "archers"),
            count(&e, "wolves")
        ),
        (20, 10, 12)
    );
}

// The switch: stacks are paid in slot order, each in full or not at all; one that is not paid leaves, and those
// after it are still paid if they can be.
#[test]
fn with_the_switch_the_stacks_that_cannot_be_paid_leave() {
    let mut e = broke(switched(meadow("reference"), true, 0), 0);
    let events = a_week(&mut e);
    // Sprites cost 20 and are paid; nothing is left for the Archers (250) or the Wolves (48).
    assert!(events.contains(&Event::StackLeft {
        troop_id: "archers".to_owned(),
        count: 10
    }));
    assert!(events.contains(&Event::StackLeft {
        troop_id: "wolves".to_owned(),
        count: 12
    }));
    assert!(events.contains(&Event::WeekSettled {
        week: 1,
        income: 0,
        upkeep: 20,
        gold: 0
    }));
    assert_eq!(
        (
            count(&e, "sprites"),
            count(&e, "archers"),
            count(&e, "wolves")
        ),
        (20, 0, 0)
    );
    // With a little more the Wolves, who come after the Archers, are paid and stay.
    let mut e = broke(switched(meadow("reference"), true, 0), 50);
    a_week(&mut e);
    assert_eq!(
        (
            count(&e, "sprites"),
            count(&e, "archers"),
            count(&e, "wolves")
        ),
        (20, 0, 12)
    );
    assert_eq!(e.gold(), 20 + 50 - 20 - 48);
}

// After a won battle a share of each stack's fallen returns, rounded down; who won is untouched.
#[test]
fn a_share_of_the_fallen_of_a_won_battle_returns() {
    let play = |returning: u32| {
        let mut e = Expedition::start(Arc::new(switched(meadow("modern"), false, returning)));
        let mut events = Vec::new();
        while !events
            .iter()
            .any(|event| matches!(event, Event::BattleWon { .. }))
        {
            let action = errand_v1(&e).expect("the first battle is won before the expedition ends");
            events.extend(act(&mut e, action));
        }
        (e, events)
    };
    let (none, plain) = play(0);
    let (half, with_half) = play(50);
    assert!(
        !plain
            .iter()
            .any(|event| matches!(event, Event::FallenReturned { .. }))
    );
    let mut back = 0;
    for event in &with_half {
        if let Event::FallenReturned { count, .. } = event {
            back += count;
        }
    }
    assert!(back > 0, "the first battle costs creatures: {with_half:?}");
    // The same battle was fought: the same army went in, and half of what fell, stack by stack, came back.
    let creatures = |e: &Expedition| e.army().iter().map(|s| s.count).sum::<u32>();
    assert_eq!(creatures(&half), creatures(&none) + back);
    assert_eq!(none.gold(), half.gold());
    assert_eq!(none.ticks(), half.ticks());
}

#[test]
fn a_band_of_two_stacks_stands_down_the_enemys_column() {
    let def = Arc::new(meadow("reference"));
    let mut e = Expedition::start(Arc::clone(&def));
    while e.engaged_with() != Some("meadow-company") {
        let action = errand_v1(&e).expect("the pilot reaches the second band");
        act(&mut e, action);
    }
    let battle = e.battle().unwrap();
    let enemy = |id: &str| {
        let stack = battle.stack(id).unwrap();
        (
            stack.troop_id.as_str(),
            stack.count,
            stack.cell.x,
            stack.cell.y,
        )
    };
    assert_eq!(enemy("e0"), ("archers", 6, 5, 1));
    assert_eq!(enemy("e1"), ("sprites", 30, 5, 2));
    assert_eq!(
        battle.leadership(),
        Some(100),
        "the hero's leadership goes into the battle"
    );
}

// Under both sets of rules the pilot plays the meadow to an end, the same way every time, with what the report
// needs along the way.
#[test]
fn the_pilot_plays_the_meadow_to_an_end_under_both_rules() {
    for rules in ["reference", "modern"] {
        let def = Arc::new(meadow(rules));
        let mut first = Expedition::start(Arc::clone(&def));
        let events = play_out(&mut first);
        let mut second = Expedition::start(Arc::clone(&def));
        play_out(&mut second);
        assert_eq!(first.digest(), second.digest(), "{rules}");
        assert_eq!(first.status(), Status::Victory, "{rules}");
        let said = |wanted: &dyn Fn(&Event) -> bool| events.iter().any(wanted);
        assert!(said(
            &|e| matches!(e, Event::ContractTaken { id } if id == "meadow-company")
        ));
        assert!(said(&|e| matches!(e, Event::ClueGained { .. })));
        assert!(said(&|e| matches!(e, Event::Searched { found: true })));
        let returned = said(&|e| matches!(e, Event::FallenReturned { .. }));
        assert_eq!(returned, rules == "modern", "{rules}");
    }
}

// The pilot sees what a player sees: with the prize buried elsewhere it does exactly the same until the clue
// tells it where.
#[test]
fn the_pilot_does_not_know_where_the_prize_lies_before_the_clue() {
    let def = meadow("reference");
    let mut moved = def.clone();
    let prize = moved
        .map
        .objects
        .iter_mut()
        .find(|o| o.id == "meadow-objective")
        .unwrap();
    (prize.x, prize.y) = (12, 7);
    let (mut here, mut there) = (
        Expedition::start(Arc::new(def)),
        Expedition::start(Arc::new(moved)),
    );
    let mut compared = 0;
    while here.objective_known().is_none() {
        let action = errand_v1(&here).unwrap();
        assert_eq!(errand_v1(&there).as_ref(), Some(&action));
        act(&mut here, action.clone());
        act(&mut there, action);
        compared += 1;
    }
    assert!(compared > 20);
    play_out(&mut there);
    assert_eq!((there.status(), there.hero().x), (Status::Victory, 12));
}
