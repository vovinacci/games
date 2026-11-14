//! Battles under the reference rules (`content/reference/rules.json`, `docs/spec/07-RULES-REGISTER.md`). The
//! numbers in these tests are worked out by hand from the rules as the register states them.

use std::collections::BTreeMap;

use super::*;
use crate::content::StackStart;

fn reference() -> Arc<Rules> {
    Arc::new(Rules::from_json(include_str!("../../../../../content/reference/rules.json")).unwrap())
}

pub(super) fn stack(id: &str, troop: &str, count: u32, x: u32, y: u32) -> StackStart {
    let side = if id.starts_with('p') {
        Side::Player
    } else {
        Side::Enemy
    };
    let slot = id[1..].parse().unwrap();
    StackStart {
        id: id.to_owned(),
        side,
        slot,
        troop_id: troop.to_owned(),
        count,
        injury: 0,
        x,
        y,
    }
}

fn fight(stacks: Vec<StackStart>, obstacles: Vec<Cell>) -> Battle {
    led(None, stacks, obstacles)
}

/// A battle in which the player's hero has this leadership, or no hero at all.
fn led(leadership: Option<u32>, stacks: Vec<StackStart>, obstacles: Vec<Cell>) -> Battle {
    let scenario = Scenario {
        id: "test".to_owned(),
        rng_state: 1,
        obstacles,
        hero_charges: BTreeMap::new(),
        leadership,
        stacks,
    };
    Battle::start(reference(), &scenario, None).unwrap()
}

pub(super) fn damage_events(events: &[Event]) -> Vec<(u32, u32, u64, u32, u32, u32, bool)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Damage {
                roll,
                attackers,
                damage,
                losses,
                count,
                injury,
                retaliation,
                ..
            } => Some((
                *roll,
                *attackers,
                *damage,
                *losses,
                *count,
                *injury,
                *retaliation,
            )),
            _ => None,
        })
        .collect()
}

// U-10: damage is roll x creatures x (attacker's skill + 5 - target's skill) / 10, and the answer comes from the
// number of creatures the target had before the blow.
#[test]
fn damage_is_scaled_by_skill_and_the_dead_still_strike_back() {
    // Ten Wolves (skill 2) on thirty Sprites (skill 1, one health each).
    let mut b = fight(
        vec![
            stack("p0", "wolves", 10, 1, 2),
            stack("e0", "sprites", 30, 2, 2),
        ],
        vec![],
    );
    let events = apply_forced(&mut b, melee("e0"), &[2, 1]).unwrap();
    assert_eq!(
        damage_events(&events),
        [
            // 2 x 10 = 20, times (2 + 5 - 1) = 6, over ten: 12. Twelve Sprites die.
            (2, 10, 12, 12, 18, 0, false),
            // The answer: all 30 Sprites strike, not the 18 left. 1 x 30 = 30, times (1 + 5 - 2) = 4, over ten:
            // 12, which is four Wolves of three health.
            (1, 30, 12, 4, 6, 0, true),
        ]
    );
    assert_eq!(
        (count_and_injury(&b, "p0"), count_and_injury(&b, "e0")),
        ((6, 0), (18, 0))
    );
}

#[test]
fn each_step_of_the_damage_sum_rounds_down() {
    // Seven Sprites on five Wolves: 1 x 7 = 7, times 4, over ten: 2 (from 2.8). The Wolves keep all five, two
    // health short. They answer with five: 3 x 5 = 15, times 6, over ten: 9, more than seven Sprites have.
    let mut b = fight(
        vec![
            stack("p0", "sprites", 7, 1, 2),
            stack("e0", "wolves", 5, 2, 2),
        ],
        vec![],
    );
    let events = apply_forced(&mut b, melee("e0"), &[1, 3]).unwrap();
    assert_eq!(
        damage_events(&events),
        [(1, 7, 2, 0, 5, 2, false), (3, 5, 9, 7, 0, 0, true)]
    );
    assert_eq!(b.outcome(), Some(Outcome::EnemyWin));

    // Equal skill halves: four Wolves on Archers, 3 x 4 = 12, times 5, over ten: 6.
    let mut even = fight(
        vec![
            stack("p0", "wolves", 4, 1, 2),
            stack("e0", "archers", 9, 2, 2),
        ],
        vec![],
    );
    let events = apply_forced(&mut even, melee("e0"), &[3, 1]).unwrap();
    assert_eq!(damage_events(&events)[0], (3, 4, 6, 0, 9, 6, false));
}

// U-08: the player's stacks act, then the enemy's; flags are renewed before each side's part.
#[test]
fn the_sides_take_turns_and_the_round_ends_after_the_enemy() {
    let mut b = fight(
        vec![
            stack("p0", "wolves", 5, 0, 0),
            stack("p1", "wolves", 5, 0, 4),
            stack("e0", "wolves", 5, 5, 2),
        ],
        vec![],
    );
    let started = |stack: &str| Event::ActivationStarted {
        stack: stack.to_owned(),
    };
    assert_eq!(b.active().unwrap().stack, "p0");
    assert_eq!(b.apply(&pass(&b)).unwrap()[1], started("p1"));
    let to_enemy = b.apply(&pass(&b)).unwrap();
    assert_eq!(
        to_enemy[1..],
        [Event::SideStarted { side: Side::Enemy }, started("e0")]
    );
    assert_eq!(b.round(), 1);
    let to_round_two = b.apply(&pass(&b)).unwrap();
    assert_eq!(
        to_round_two[1..],
        [Event::RoundStarted { round: 2 }, started("p0")]
    );

    // Whose part it is belongs to the state: the same position in the other part is another state.
    let Value::Object(state) = b.to_canon() else {
        panic!("an object")
    };
    assert_eq!(state["side"], Value::str("player"));
    assert_eq!(
        state["rng"],
        Value::object([
            ("algorithm", Value::str("reference-lcg64-v1")),
            ("state", 1_u32.into())
        ])
    );
}

#[test]
fn a_stack_answers_once_in_each_sides_part_of_the_round() {
    // Two stacks of the player beside one large enemy stack.
    let mut b = fight(
        vec![
            stack("p0", "wolves", 20, 1, 1),
            stack("p1", "wolves", 20, 1, 3),
            stack("e0", "wolves", 60, 2, 2),
        ],
        vec![],
    );
    assert_eq!(
        blows(&apply_forced(&mut b, melee("e0"), &[1, 1]).unwrap()),
        2,
        "the enemy answers the first blow"
    );
    assert_eq!(
        blows(&apply_forced(&mut b, melee("e0"), &[1]).unwrap()),
        1,
        "and not the second in the same part"
    );
    // The enemy's part: its flag is renewed, and so is that of the stack it strikes.
    assert_eq!(b.active().unwrap().stack, "e0");
    assert!(!b.stack("e0").unwrap().retaliated && !b.stack("p0").unwrap().retaliated);
    assert_eq!(
        blows(&apply_forced(&mut b, melee("p0"), &[1, 1]).unwrap()),
        2,
        "the struck stack answers"
    );
    // Round 2, the player's part: the enemy answers again.
    assert_eq!((b.round(), b.active().unwrap().stack.as_str()), (2, "p0"));
    assert_eq!(
        blows(&apply_forced(&mut b, melee("e0"), &[1, 1]).unwrap()),
        2
    );
}

#[test]
fn a_stack_that_waits_acts_after_its_own_side_and_before_the_other() {
    let mut b = fight(
        vec![
            stack("p0", "wolves", 5, 0, 0),
            stack("p1", "wolves", 5, 0, 4),
            stack("e0", "wolves", 5, 5, 2),
        ],
        vec![],
    );
    act(&mut b, Action::Wait).unwrap();
    assert_eq!(b.active().unwrap().stack, "p1");
    b.apply(&pass(&b)).unwrap();
    assert_eq!(
        b.active().unwrap().stack,
        "p0",
        "the stack that waited, still in the player's part"
    );
    b.apply(&pass(&b)).unwrap();
    assert_eq!(b.active().unwrap().stack, "e0");
}

// U-09: two flights, free of movement; the second ends the turn. A step ends it when the movement is gone.
#[test]
fn a_flyer_has_two_flights_that_cost_no_movement() {
    let sprites_and_wolves = || {
        vec![
            stack("p0", "sprites", 10, 0, 0),
            stack("e0", "wolves", 5, 5, 4),
        ]
    };
    let mut b = fight(sprites_and_wolves(), vec![]);
    let active = b.active().unwrap();
    assert_eq!((active.movement, active.flights), (1, 2));

    act(&mut b, fly(3, 3)).unwrap();
    let active = b.active().unwrap();
    assert_eq!(
        (active.stack.as_str(), active.movement, active.flights),
        ("p0", 1, 1),
        "still its turn"
    );
    act(&mut b, fly(4, 3)).unwrap();
    assert_eq!(
        b.active().unwrap().stack,
        "e0",
        "the second flight ends the turn"
    );
    assert_eq!(b.stack("p0").unwrap().cell, cell(4, 3));

    // Fly once, then strike.
    let mut strike = fight(sprites_and_wolves(), vec![]);
    act(&mut strike, fly(4, 3)).unwrap();
    assert_eq!(strike.legal_actions()[0], melee("e0"));

    // A step uses the one movement point, and that ends the turn whatever flights are left.
    let mut step = fight(sprites_and_wolves(), vec![]);
    act(&mut step, walk(1, 1)).unwrap();
    assert_eq!(step.active().unwrap().stack, "e0");
}

// U-09: a diagonal step is not stopped by what stands beside it.
#[test]
fn a_diagonal_step_passes_between_blocked_cells() {
    let stacks = vec![
        stack("p0", "wolves", 5, 1, 1),
        stack("e0", "wolves", 5, 5, 4),
    ];
    let mut b = fight(stacks, vec![cell(2, 1), cell(1, 2)]);
    assert!(b.legal_actions().contains(&walk(2, 2)));
    assert!(b.observe().step_allowed(cell(1, 1), cell(2, 2)));
    act(&mut b, walk(2, 2)).unwrap();
    refused(
        &mut b,
        walk(2, 1),
        &RuleError::CannotEnter { to: cell(2, 1) },
    );
}

// U-12: a shot is scaled like a blow, is never answered, and uses a shot.
#[test]
fn a_shot_is_scaled_by_skill_and_not_answered() {
    let mut b = fight(
        vec![
            stack("p0", "archers", 10, 0, 0),
            stack("e0", "wolves", 9, 5, 4),
        ],
        vec![],
    );
    let events = apply_forced(&mut b, shoot("e0"), &[2]).unwrap();
    // 2 x 10 = 20, times (2 + 5 - 2) = 5, over ten: 10: three Wolves and one health of the fourth.
    assert_eq!(damage_events(&events), [(2, 10, 10, 3, 6, 1, false)]);
    assert_eq!(b.stack("p0").unwrap().shots, 11);
    assert_eq!(b.active().unwrap().stack, "e0");

    let mut beside = fight(
        vec![
            stack("p0", "archers", 10, 1, 1),
            stack("e0", "wolves", 9, 2, 2),
        ],
        vec![],
    );
    refused(&mut beside, shoot("e0"), &RuleError::EnemyAdjacent);
}

#[test]
fn rolls_come_from_the_reference_generator() {
    let mut b = fight(
        vec![
            stack("p0", "wolves", 10, 1, 2),
            stack("e0", "wolves", 10, 2, 2),
        ],
        vec![],
    );
    let events = act(&mut b, melee("e0")).unwrap();
    // From state 1 the first draw from 1..=3 is 3 (see the generator's own test).
    assert_eq!(damage_events(&events)[0].0, 3);
    assert_ne!(b.rng_state(), 1);
}

#[test]
fn the_round_limit_holds_under_the_reference_rules_too() {
    let mut b = fight(
        vec![
            stack("p0", "wolves", 5, 0, 0),
            stack("e0", "wolves", 5, 5, 4),
        ],
        vec![],
    );
    while b.outcome().is_none() {
        b.apply(&pass(&b)).unwrap();
    }
    assert_eq!(
        (b.outcome(), b.round(), b.revision()),
        (Some(Outcome::BudgetExceeded), 200, 400)
    );
}

#[test]
fn every_legal_action_is_accepted_under_the_reference_rules() {
    let b = fight(
        vec![
            stack("p0", "sprites", 10, 1, 1),
            stack("p1", "archers", 5, 0, 4),
            stack("e0", "wolves", 5, 2, 2),
        ],
        vec![cell(3, 0)],
    );
    let actions = b.legal_actions();
    assert!(
        actions.iter().any(|a| matches!(a, Action::Fly { .. })) && actions.contains(&melee("e0"))
    );
    for action in actions {
        let mut copy = b.clone();
        assert!(act(&mut copy, action.clone()).is_ok(), "{action:?}");
    }
}

/// Twenty Sprites of the player strike enemy Wolves with a roll of 2: before morale that is 2 x 20 = 40, times
/// (1 + 5 - 2) = 4, over ten: 16. Returns the damage and the morale the event reports.
fn sprites_strike(leadership: Option<u32>, beside: Option<&str>) -> (u64, Option<Morale>) {
    let mut stacks = vec![
        stack("p0", "sprites", 20, 1, 2),
        stack("e0", "wolves", 50, 2, 2),
    ];
    stacks.extend(beside.map(|troop| stack("p1", troop, 3, 0, 0)));
    let mut b = led(leadership, stacks, vec![]);
    let events = apply_forced(&mut b, melee("e0"), &[2, 1]).unwrap();
    let Event::Damage { damage, morale, .. } = &events[0] else {
        panic!("a blow first")
    };
    (*damage, *morale)
}

// U-10, U-18: morale by the rule the reference attributes to the original.
#[test]
fn morale_scales_the_damage_of_the_heros_stacks() {
    // Alone: Sprites with Sprites is High in the chart. 16 and half again.
    assert_eq!(sprites_strike(Some(1000), None), (24, Some(Morale::High)));
    // With Wolves: (Wolves, Sprites) is Low and (Sprites, Sprites) High; the lowest in the order Normal, Low,
    // High is Low. Halved.
    assert_eq!(
        sprites_strike(Some(1000), Some("wolves")),
        (8, Some(Morale::Low))
    );
    // With Archers: (Archers, Sprites) is Normal, and one Normal is enough.
    assert_eq!(
        sprites_strike(Some(1000), Some("archers")),
        (16, Some(Morale::Normal))
    );
    // No hero, no morale.
    assert_eq!(sprites_strike(None, Some("wolves")), (16, None));
}

#[test]
fn the_two_morale_rules_part_on_sprites_with_wolves() {
    // The rule in use makes the Sprites Low and leaves the Wolves Normal: (Sprites, Wolves) is Normal. The other
    // rule the reference documents would have it the other way round (see the rules register).
    let b = led(
        Some(1000),
        vec![
            stack("p0", "sprites", 20, 0, 0),
            stack("p1", "wolves", 3, 0, 2),
            stack("e0", "wolves", 5, 5, 4),
        ],
        vec![],
    );
    assert_eq!(
        (b.morale("p0"), b.morale("p1")),
        (Some(Morale::Low), Some(Morale::Normal))
    );
    assert_eq!(b.morale("e0"), None, "the enemy has no hero");
    assert_eq!(b.morale("ghost"), None);
}

#[test]
fn morale_follows_the_army_as_it_stands_and_applies_to_the_answer_too() {
    // The enemy's Wolves strike the player's Sprites, who stand with three Wolves: the Sprites answer at Low.
    let mut b = led(
        Some(1000),
        vec![
            stack("p0", "sprites", 40, 1, 2),
            stack("p1", "wolves", 3, 1, 3),
            stack("e0", "wolves", 9, 2, 2),
        ],
        vec![],
    );
    b.apply(&pass(&b)).unwrap();
    b.apply(&pass(&b)).unwrap();
    assert_eq!(b.active().unwrap().stack, "e0");
    let events = apply_forced(&mut b, melee("p0"), &[1, 2]).unwrap();
    // The enemy's blow has no morale: 1 x 9 = 9, times 6, over ten: 5.
    // The answer: 2 x 40 = 80, times 4, over ten: 32, halved by Low morale: 16.
    assert_eq!(
        damage_events(&events),
        [(1, 9, 5, 5, 35, 0, false), (2, 40, 16, 5, 4, 1, true)]
    );
    assert!(matches!(
        &events[1],
        Event::Damage {
            morale: Some(Morale::Low),
            ..
        }
    ));

    // Once the player's Wolves are gone the Sprites are alone, and High.
    let mut alone = led(
        Some(1000),
        vec![
            stack("p0", "sprites", 40, 0, 0),
            stack("p1", "wolves", 1, 4, 3),
            stack("e0", "wolves", 30, 5, 4),
        ],
        vec![],
    );
    assert_eq!(alone.morale("p0"), Some(Morale::Low));
    alone.apply(&pass(&alone)).unwrap();
    apply_forced(&mut alone, melee("e0"), &[1, 3]).unwrap(); // the one Wolf strikes and dies of the answer
    assert!(!alone.stack("p1").unwrap().alive());
    assert_eq!(alone.morale("p0"), Some(Morale::High));
}

// U-06: a stack whose whole health is more than the hero's leadership is out of control.
#[test]
fn a_stack_too_large_for_the_leadership_is_out_of_control() {
    let army = |wolves| {
        vec![
            stack("p0", "wolves", wolves, 1, 2),
            stack("p1", "archers", 2, 1, 3),
            stack("e0", "wolves", 9, 5, 0),
        ]
    };
    // Ten Wolves are 30 health: equal to the leadership is within it.
    let within = led(Some(30), army(10), vec![]);
    assert!(!within.stack("p0").unwrap().out_of_control);
    // Eleven are 33.
    let over = led(Some(30), army(11), vec![]);
    assert!(over.stack("p0").unwrap().out_of_control);
    assert!(!over.stack("p1").unwrap().out_of_control && !over.stack("e0").unwrap().out_of_control);
    assert_eq!(over.morale("p0"), None, "no morale without control");
    // It may strike the stack beside it, of its own side.
    assert_eq!(over.legal_actions()[0], melee("p1"));
    // Without a hero nothing is out of control, whatever its size.
    assert!(!fight(army(500), vec![]).stack("p0").unwrap().out_of_control);
}

#[test]
fn a_stack_out_of_control_fights_its_own_side_and_is_fought_by_it() {
    let mut b = led(
        Some(30),
        vec![
            stack("p0", "wolves", 11, 1, 2),
            stack("p1", "archers", 2, 1, 3),
            stack("e0", "wolves", 9, 5, 0),
        ],
        vec![],
    );
    // 2 x 11 = 22, times (2 + 5 - 2) = 5, over ten: 11, with no morale. The two Archers have 20 health.
    let events = apply_forced(&mut b, melee("p1"), &[2, 1]).unwrap();
    assert_eq!(damage_events(&events)[0], (2, 11, 11, 1, 1, 1, false));
    assert!(matches!(&events[0], Event::Damage { morale: None, .. }));
    // The Archers' turn: the Wolves beside them are an enemy, so they cannot shoot, but may strike them.
    assert_eq!(b.active().unwrap().stack, "p1");
    refused(&mut b, shoot("e0"), &RuleError::EnemyAdjacent);
    assert!(b.legal_actions().contains(&melee("p0")));
}

#[test]
fn losses_bring_a_stack_back_under_control_before_the_next_turn() {
    let mut b = led(
        Some(30),
        vec![
            stack("p0", "wolves", 11, 1, 2),
            stack("p1", "wolves", 2, 0, 0),
            stack("e0", "wolves", 20, 2, 2),
        ],
        vec![],
    );
    assert!(b.stack("p0").unwrap().out_of_control);
    // The stack out of control strikes the enemy and loses creatures to the answer:
    // 1 x 20 = 20, times 5, over ten: 10, three Wolves and one health. Eight are left: 24 health, within 30.
    let events = apply_forced(&mut b, melee("e0"), &[1, 1]).unwrap();
    assert_eq!(count_and_injury(&b, "p0"), (8, 1));
    assert!(events.contains(&Event::ControlRegained {
        stack: "p0".to_owned()
    }));
    assert!(!b.stack("p0").unwrap().out_of_control);
    assert_eq!(b.active().unwrap().stack, "p1");
    assert_eq!(
        b.legal_actions()
            .iter()
            .filter(|a| matches!(a, Action::Melee { .. }))
            .count(),
        0,
        "a friend again"
    );
    assert_eq!(b.morale("p0"), Some(Morale::High), "Wolves among Wolves");
}

#[test]
fn leadership_and_control_are_part_of_the_state_and_the_prototype_has_neither() {
    let b = led(
        Some(30),
        vec![
            stack("p0", "wolves", 11, 1, 2),
            stack("e0", "wolves", 9, 5, 0),
        ],
        vec![],
    );
    let Value::Object(state) = b.to_canon() else {
        panic!("an object")
    };
    assert_eq!(state["leadership"], Value::from(30_u32));
    let Value::Array(stacks) = &state["stacks"] else {
        panic!("an array")
    };
    let Value::Object(first) = &stacks[1] else {
        panic!("an object")
    };
    assert_eq!(
        (&first["id"], &first["out_of_control"]),
        (&Value::str("p0"), &Value::Bool(true))
    );

    // A prototype scenario cannot ask for leadership, and its state has no such fields.
    let mut scenario = scenario("melee_exchange");
    scenario.leadership = Some(30);
    assert!(matches!(
        Battle::start(Arc::new(rules()), &scenario, None),
        Err(ContentError::Invalid(_))
    ));
    let Value::Object(prototype) = battle("melee_exchange").to_canon() else {
        panic!("an object")
    };
    assert!(!prototype.contains_key("leadership") && !prototype.contains_key("side"));
}
