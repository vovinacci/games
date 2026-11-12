use super::*;
use crate::content::tests::{RULES, rules, scenario};

fn battle(id: &str) -> Battle {
    Battle::start(Arc::new(rules()), &scenario(id), None).unwrap()
}

fn pass(battle: &Battle) -> Command {
    let actor = battle.active().unwrap().stack.clone();
    Command {
        revision: battle.revision(),
        actor,
        action: Action::Pass,
    }
}

#[test]
fn a_battle_starts_with_the_first_player_slot() {
    let b = battle("blocked_shooter");
    assert_eq!((b.round(), b.revision(), b.outcome()), (1, 0, None));
    assert_eq!(
        b.stacks().iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
        ["p0", "p1", "e0"]
    );
    let active = b.active().unwrap();
    assert_eq!(
        (
            active.stack.as_str(),
            active.movement,
            active.flights,
            active.moved
        ),
        ("p0", 2, 0, false)
    );
    assert_eq!(b.stack("p0").unwrap().shots, 4);
    assert_eq!(b.rng_state(), 2);
}

#[test]
fn a_seed_given_at_the_start_replaces_the_scenarios_own() {
    let b = Battle::start(Arc::new(rules()), &scenario("melee_exchange"), Some(77)).unwrap();
    assert_eq!(b.rng_state(), 77);
    assert_ne!(b.digest(), battle("melee_exchange").digest());
}

// AT-002, AT-006: a refused command leaves state, revision and random numbers exactly as they were.
#[test]
fn a_refused_command_changes_nothing() {
    let mut b = battle("melee_exchange");
    let before = (b.clone(), b.digest());
    let command = |revision, actor: &str| Command {
        revision,
        actor: actor.to_owned(),
        action: Action::Pass,
    };

    assert_eq!(
        b.apply(&command(5, "p0")),
        Err(RuleError::Stale {
            command: 5,
            battle: 0
        })
    );
    assert_eq!(
        b.apply(&command(0, "e0")),
        Err(RuleError::NotActive {
            actor: "e0".to_owned()
        })
    );
    assert_eq!(
        b.apply(&command(0, "nobody")),
        Err(RuleError::NotActive {
            actor: "nobody".to_owned()
        })
    );
    assert_eq!((b.clone(), b.digest()), before);

    // An accepted command makes the one written for the old revision stale.
    let old = pass(&b);
    b.apply(&old).unwrap();
    assert_eq!(
        b.apply(&old),
        Err(RuleError::Stale {
            command: 0,
            battle: 1
        })
    );
}

#[test]
fn turns_go_player_slots_then_enemy_slots_then_a_new_round() {
    let mut b = battle("blocked_shooter");
    let ended = |stack: &str| Event::ActivationEnded {
        stack: stack.to_owned(),
    };
    let started = |stack: &str| Event::ActivationStarted {
        stack: stack.to_owned(),
    };
    assert_eq!(b.apply(&pass(&b)).unwrap(), [ended("p0"), started("p1")]);
    assert_eq!(b.apply(&pass(&b)).unwrap(), [ended("p1"), started("e0")]);
    assert_eq!(
        b.apply(&pass(&b)).unwrap(),
        [ended("e0"), Event::RoundStarted { round: 2 }, started("p0")]
    );
    assert_eq!((b.round(), b.revision()), (2, 3));
}

// AT-023 in its simplest form: nobody fights, so the round limit ends the battle as a diagnosis.
#[test]
fn the_round_limit_ends_a_battle_nobody_wins() {
    let mut b = battle("melee_exchange");
    let mut last = Vec::new();
    while b.outcome().is_none() {
        last = b.apply(&pass(&b)).unwrap();
    }
    assert_eq!(b.outcome(), Some(Outcome::BudgetExceeded));
    assert_eq!(
        last.last(),
        Some(&Event::BattleEnded {
            outcome: Outcome::BudgetExceeded
        })
    );
    assert_eq!((b.round(), b.revision()), (200, 400));
    assert!(b.active().is_none());
    let late = Command {
        revision: b.revision(),
        actor: "p0".to_owned(),
        action: Action::Pass,
    };
    assert_eq!(b.apply(&late), Err(RuleError::BattleOver));
}

// AT-054: the order of the catalogue carries no meaning.
#[test]
fn reordering_the_troop_list_changes_nothing() {
    let mut file: serde_json::Value = serde_json::from_str(RULES).unwrap();
    file["troops"].as_array_mut().unwrap().reverse();
    let reordered = Rules::from_json(&file.to_string()).unwrap();
    assert_ne!(reordered.troops, rules().troops);
    for id in [
        "melee_exchange",
        "blocked_shooter",
        "flight_barrier",
        "unreachable_melee",
    ] {
        let mut a = battle(id);
        let mut b = Battle::start(Arc::new(reordered.clone()), &scenario(id), None).unwrap();
        assert_eq!(a.digest(), b.digest(), "{id} at the start");
        a.apply(&pass(&a)).unwrap();
        b.apply(&pass(&b)).unwrap();
        assert_eq!(a.digest(), b.digest(), "{id} after a command");
        assert_eq!(
            b.active().unwrap().movement,
            2,
            "{id}: the next stack is a warden in both"
        );
    }
}

#[test]
fn the_digest_follows_the_state() {
    let mut b = battle("melee_exchange");
    let start = b.digest();
    assert_eq!(start.len(), 64);
    assert_eq!(start, battle("melee_exchange").digest());
    b.apply(&pass(&b)).unwrap();
    assert_ne!(b.digest(), start);
}

/// Applies a command with the damage rolls given, each checked against the range it is asked for.
fn apply_forced(
    battle: &mut Battle,
    action: Action,
    forced: &[u32],
) -> Result<Vec<Event>, RuleError> {
    let command = Command {
        revision: battle.revision(),
        actor: battle.active().unwrap().stack.clone(),
        action,
    };
    let mut left = forced.iter();
    let events = battle.apply_with(&command, &mut |_, lo, hi| {
        let roll = *left.next().expect("a forced roll for every roll asked");
        assert!(
            (lo..=hi).contains(&roll),
            "forced roll {roll} is outside {lo}..={hi}"
        );
        roll
    });
    assert!(
        events.is_err() || left.next().is_none(),
        "every forced roll was used"
    );
    events
}

fn melee(target: &str) -> Action {
    Action::Melee {
        target: target.to_owned(),
    }
}

fn count_and_injury(battle: &Battle, id: &str) -> (u32, u32) {
    let stack = battle.stack(id).unwrap();
    (stack.count, stack.injury)
}

fn blows(events: &[Event]) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, Event::Damage { .. }))
        .count()
}

/// The cases of `expected_cases.json` whose ID starts with `prefix`.
fn expected_cases(prefix: &str) -> Vec<serde_json::Value> {
    let file: serde_json::Value =
        serde_json::from_str(include_str!("../../../../prototype/expected_cases.json")).unwrap();
    let cases = file["cases"].as_array().unwrap();
    cases
        .iter()
        .filter(|c| c["id"].as_str().unwrap().starts_with(prefix))
        .cloned()
        .collect()
}

fn number(value: &serde_json::Value) -> u32 {
    u32::try_from(value.as_u64().unwrap()).unwrap()
}

// AT-012 and the other health cases of the fixture file.
#[test]
fn health_arithmetic_matches_the_fixture() {
    let cases = expected_cases("health_");
    assert_eq!(cases.len(), 3);
    for case in cases {
        let (input, expected) = (&case["input"], &case["expected"]);
        let hp = number(&input["hp_each"]);
        let damage = u64::from(number(&input["damage"]));
        let (count, injury) = health_after(
            hp,
            number(&input["count"]),
            number(&input["injury"]),
            damage,
        );
        assert_eq!(
            (count, injury),
            (number(&expected["count"]), number(&expected["injury"])),
            "{}",
            case["id"]
        );
        assert_eq!(
            count * hp - injury,
            number(&expected["total_hp"]),
            "{}",
            case["id"]
        );
    }
    assert_eq!(health_after(10, 10, 0, 26), (8, 6));
    assert_eq!(
        health_after(10, 1, 9, 1),
        (0, 0),
        "the last point of health"
    );
    assert_eq!(health_after(10, 3, 4, 0), (3, 4), "no damage");
    assert_eq!(
        health_after(u32::MAX, u32::MAX, 0, u64::MAX),
        (0, 0),
        "no overflow"
    );
}

// AT-013: the fixture's melee exchange with forced rolls 3 then 2.
#[test]
fn a_melee_exchange_matches_the_fixture() {
    let case = &expected_cases("melee_exchange")[0];
    let forced: Vec<u32> = case["forced_damage_rolls"]
        .as_array()
        .unwrap()
        .iter()
        .map(number)
        .collect();
    let mut b = battle(case["scenario_id"].as_str().unwrap());
    let rng = b.rng_state();
    let events = apply_forced(
        &mut b,
        melee(case["command"]["target"].as_str().unwrap()),
        &forced,
    )
    .unwrap();

    let expected = &case["expected"];
    assert_eq!(
        count_and_injury(&b, "e0"),
        (
            number(&expected["e0"]["count"]),
            number(&expected["e0"]["injury"])
        )
    );
    assert_eq!(
        count_and_injury(&b, "p0"),
        (
            number(&expected["p0"]["count"]),
            number(&expected["p0"]["injury"])
        )
    );
    assert!(b.stack("e0").unwrap().retaliated);
    assert_eq!(
        b.active().unwrap().stack,
        "e0",
        "the attacker's activation ended"
    );
    assert_eq!(b.rng_state(), rng, "forced rolls draw nothing");
    assert_eq!(
        events[0],
        Event::Damage {
            attacker: "p0".to_owned(),
            target: "e0".to_owned(),
            roll: 3,
            attackers: 10,
            damage: 30,
            losses: 3,
            count: 7,
            injury: 0,
            retaliation: false,
            morale: None,
        }
    );
    assert!(matches!(
        &events[1],
        Event::Damage {
            damage: 14,
            losses: 1,
            retaliation: true,
            ..
        }
    ));
}

// AT-014: a stack strikes back once in a round, and again in the next.
#[test]
fn a_second_attack_in_the_round_is_not_struck_back() {
    let mut scenario = scenario("blocked_shooter");
    let p1 = scenario.stacks.iter_mut().find(|s| s.id == "p1").unwrap();
    (p1.x, p1.y) = (3, 3);
    let mut b = Battle::start(Arc::new(rules()), &scenario, None).unwrap();

    assert_eq!(
        blows(&apply_forced(&mut b, melee("e0"), &[1, 2]).unwrap()),
        2
    );
    let slingers = count_and_injury(&b, "p0");

    assert_eq!(blows(&apply_forced(&mut b, melee("e0"), &[2]).unwrap()), 1);
    assert_eq!(count_and_injury(&b, "p1"), (5, 0));
    assert_eq!(count_and_injury(&b, "p0"), slingers);

    // The enemy passes; in round 2 it strikes back again.
    b.apply(&pass(&b)).unwrap();
    assert_eq!(b.round(), 2);
    assert!(!b.stack("e0").unwrap().retaliated);
    assert_eq!(
        blows(&apply_forced(&mut b, melee("e0"), &[1, 2]).unwrap()),
        2
    );
}

#[test]
fn a_killing_blow_is_not_struck_back_and_can_end_the_battle() {
    let mut scenario = scenario("melee_exchange");
    scenario
        .stacks
        .iter_mut()
        .find(|s| s.id == "e0")
        .unwrap()
        .count = 2;
    let mut b = Battle::start(Arc::new(rules()), &scenario, None).unwrap();
    let events = apply_forced(&mut b, melee("e0"), &[2]).unwrap();
    assert_eq!(count_and_injury(&b, "e0"), (0, 0));
    assert_eq!(count_and_injury(&b, "p0"), (10, 0));
    assert_eq!(b.outcome(), Some(Outcome::PlayerWin));
    assert!(b.active().is_none() && b.legal_actions().is_empty());
    let removed = Event::StackRemoved {
        stack: "e0".to_owned(),
    };
    assert_eq!(
        events[1..],
        [
            removed,
            Event::BattleEnded {
                outcome: Outcome::PlayerWin
            }
        ]
    );
}

#[test]
fn an_attacker_killed_by_the_answer_loses_its_turn_and_the_battle_goes_on() {
    let mut scenario = scenario("blocked_shooter");
    scenario
        .stacks
        .iter_mut()
        .find(|s| s.id == "p0")
        .unwrap()
        .count = 1;
    let mut b = Battle::start(Arc::new(rules()), &scenario, None).unwrap();
    apply_forced(&mut b, melee("e0"), &[1, 2]).unwrap();
    assert!(!b.stack("p0").unwrap().alive());
    assert_eq!(b.outcome(), None);
    assert_eq!(b.active().unwrap().stack, "p1");
    // The dead stack never comes back into the order.
    b.apply(&pass(&b)).unwrap();
    b.apply(&pass(&b)).unwrap();
    assert_eq!((b.round(), b.active().unwrap().stack.as_str()), (2, "p1"));
}

#[test]
fn melee_is_refused_without_a_neighbouring_living_enemy() {
    let mut b = battle("blocked_shooter");
    let refuse = |b: &mut Battle, target: &str| {
        let before = b.digest();
        let error = apply_forced(b, melee(target), &[]).unwrap_err();
        assert_eq!(b.digest(), before, "refusing {target} changed nothing");
        error
    };
    assert_eq!(
        refuse(&mut b, "p1"),
        RuleError::NotAnEnemy {
            target: "p1".to_owned()
        }
    );
    assert_eq!(
        refuse(&mut b, "p0"),
        RuleError::NotAnEnemy {
            target: "p0".to_owned()
        }
    );
    assert_eq!(
        refuse(&mut b, "ghost"),
        RuleError::NotAnEnemy {
            target: "ghost".to_owned()
        }
    );
    b.apply(&pass(&b)).unwrap();
    assert_eq!(
        refuse(&mut b, "e0"),
        RuleError::NotAdjacent {
            target: "e0".to_owned()
        }
    );
}

#[test]
fn legal_actions_come_in_a_fixed_order() {
    let b = battle("blocked_shooter");
    // The slinger at (2,2) beside the enemy at (3,2): no shot, and no step past the enemy's corners.
    let expected = [
        melee("e0"),
        walk(1, 1),
        walk(2, 1),
        walk(1, 2),
        walk(1, 3),
        walk(2, 3),
        Action::Wait,
        Action::Pass,
    ];
    assert_eq!(b.legal_actions(), expected);
    assert_eq!(b.observe().legal, expected);
}

#[test]
fn real_rolls_come_from_the_battles_generator() {
    let mut a = battle("melee_exchange");
    let mut b = battle("melee_exchange");
    let command = Command {
        revision: 0,
        actor: "p0".to_owned(),
        action: melee("e0"),
    };
    assert_eq!(a.apply(&command), b.apply(&command));
    assert_eq!(a.digest(), b.digest());
    assert_ne!(a.rng_state(), 1, "rolls were drawn");
}

fn act(battle: &mut Battle, action: Action) -> Result<Vec<Event>, RuleError> {
    let command = Command {
        revision: battle.revision(),
        actor: battle.active().unwrap().stack.clone(),
        action,
    };
    battle.apply(&command)
}

/// The action is refused with `error` and nothing changes.
#[expect(
    clippy::needless_pass_by_value,
    reason = "a test reads better with the action built in place"
)]
fn refused(battle: &mut Battle, action: Action, error: &RuleError) {
    let before = battle.digest();
    assert_eq!(
        act(battle, action.clone()).as_ref(),
        Err(error),
        "{action:?}"
    );
    assert_eq!(
        battle.digest(),
        before,
        "refusing {action:?} changed nothing"
    );
    assert!(
        !battle.legal_actions().contains(&action),
        "{action:?} is not offered"
    );
}

fn cell(x: u32, y: u32) -> Cell {
    Cell { x, y }
}

fn walk(x: u32, y: u32) -> Action {
    Action::Walk { to: cell(x, y) }
}

fn fly(x: u32, y: u32) -> Action {
    Action::Fly { to: cell(x, y) }
}

fn shoot(target: &str) -> Action {
    Action::Shoot {
        target: target.to_owned(),
    }
}

fn cast(spell: &str, target: &str) -> Action {
    Action::Cast {
        spell: spell.to_owned(),
        target: target.to_owned(),
    }
}

/// A scenario with one stack moved, or its count changed, before the battle starts.
fn battle_with(id: &str, change: impl FnOnce(&mut Scenario)) -> Battle {
    let mut scenario = scenario(id);
    change(&mut scenario);
    Battle::start(Arc::new(rules()), &scenario, None).unwrap()
}

fn place(scenario: &mut Scenario, id: &str, x: u32, y: u32) {
    let stack = scenario.stacks.iter_mut().find(|s| s.id == id).unwrap();
    (stack.x, stack.y) = (x, y);
}

// AT-010: the board has edges.
#[test]
fn a_step_off_the_board_or_further_than_a_neighbour_is_refused() {
    let mut b = battle_with("melee_exchange", |s| place(s, "p0", 0, 0));
    refused(
        &mut b,
        walk(6, 0),
        &RuleError::CannotEnter { to: cell(6, 0) },
    );
    refused(
        &mut b,
        walk(0, 5),
        &RuleError::CannotEnter { to: cell(0, 5) },
    );
    refused(
        &mut b,
        walk(2, 0),
        &RuleError::CannotEnter { to: cell(2, 0) },
    );
    refused(
        &mut b,
        walk(0, 0),
        &RuleError::CannotEnter { to: cell(0, 0) },
    );
    let steps: Vec<Action> = b
        .legal_actions()
        .into_iter()
        .filter(|a| matches!(a, Action::Walk { .. }))
        .collect();
    assert_eq!(steps, [walk(1, 0), walk(0, 1), walk(1, 1)], "row by row");
}

// AT-011: a diagonal step needs both cells it passes between.
#[test]
fn a_diagonal_step_past_a_blocked_corner_is_refused() {
    let mut b = battle_with("melee_exchange", |s| {
        place(s, "p0", 1, 1);
        place(s, "e0", 2, 1);
        s.obstacles.push(cell(1, 2));
    });
    // (2,2) lies between the enemy at (2,1) and the obstacle at (1,2).
    refused(
        &mut b,
        walk(2, 2),
        &RuleError::CannotEnter { to: cell(2, 2) },
    );
    refused(
        &mut b,
        walk(2, 1),
        &RuleError::CannotEnter { to: cell(2, 1) },
    );
    refused(
        &mut b,
        walk(1, 2),
        &RuleError::CannotEnter { to: cell(1, 2) },
    );
    // (2,0) passes the enemy's cell: one blocked corner is enough.
    refused(
        &mut b,
        walk(2, 0),
        &RuleError::CannotEnter { to: cell(2, 0) },
    );
    // (0,2) passes the obstacle.
    refused(
        &mut b,
        walk(0, 2),
        &RuleError::CannotEnter { to: cell(0, 2) },
    );
    assert!(
        act(&mut b, walk(0, 0)).is_ok(),
        "a diagonal between free cells"
    );
}

#[test]
fn walking_costs_a_point_a_step_and_the_turn_ends_with_the_last_point() {
    let mut b = battle_with("melee_exchange", |s| place(s, "p0", 0, 0));
    let events = act(&mut b, walk(1, 0)).unwrap();
    assert_eq!(
        events,
        [Event::Moved {
            stack: "p0".to_owned(),
            from: cell(0, 0),
            to: cell(1, 0),
            flew: false
        }]
    );
    let active = b.active().unwrap();
    assert_eq!(
        (active.stack.as_str(), active.movement, active.moved),
        ("p0", 1, true)
    );
    refused(&mut b, Action::Wait, &RuleError::AlreadyMoved);
    act(&mut b, walk(1, 1)).unwrap();
    assert_eq!(b.stack("p0").unwrap().cell, cell(1, 1));
    assert_eq!(
        b.active().unwrap().stack,
        "e0",
        "no points left, so the turn passed on"
    );
}

// AT-015: the fixture's blocked shooter.
#[test]
fn a_shooter_next_to_an_enemy_cannot_shoot() {
    let case = &expected_cases("blocked_shooter")[0];
    assert_eq!(case["expected"]["legal"], false);
    let mut b = battle(case["scenario_id"].as_str().unwrap());
    let rng = b.rng_state();
    refused(
        &mut b,
        shoot(case["command"]["target"].as_str().unwrap()),
        &RuleError::EnemyAdjacent,
    );
    assert_eq!(b.rng_state(), rng);
}

// AT-016: shots are for the whole battle.
#[test]
fn shots_are_spent_and_never_come_back() {
    let mut b = battle_with("blocked_shooter", |s| {
        place(s, "p0", 0, 0);
        s.stacks.retain(|stack| stack.id != "p1");
        s.stacks
            .iter_mut()
            .find(|stack| stack.id == "e0")
            .unwrap()
            .count = 200;
    });
    for shots_left in (0..4).rev() {
        let events = apply_forced(&mut b, shoot("e0"), &[2]).unwrap();
        assert_eq!(blows(&events), 1, "a shot is never struck back");
        assert!(matches!(
            &events[0],
            Event::Damage {
                damage: 16,
                retaliation: false,
                ..
            }
        ));
        assert_eq!(b.stack("p0").unwrap().shots, shots_left);
        assert_eq!(b.active().unwrap().stack, "e0", "shooting ends the turn");
        b.apply(&pass(&b)).unwrap();
    }
    assert_eq!(b.round(), 5);
    refused(&mut b, shoot("e0"), &RuleError::NoShots);
    assert_eq!(b.stack("p0").unwrap().shots, 0);
    // A troop that never had shots.
    b.apply(&pass(&b)).unwrap();
    refused(&mut b, shoot("p0"), &RuleError::NoShots);
    refused(
        &mut b,
        shoot("e0"),
        &RuleError::NotAnEnemy {
            target: "e0".to_owned(),
        },
    );
}

// AT-017: the fixture's flight over the barrier.
#[test]
fn a_flyer_crosses_obstacles_but_cannot_land_on_one() {
    let case = &expected_cases("flight_to_clear_cell")[0];
    let (x, y) = (number(&case["command"]["x"]), number(&case["command"]["y"]));
    let mut b = battle(case["scenario_id"].as_str().unwrap());
    refused(
        &mut b,
        fly(2, 2),
        &RuleError::CannotEnter { to: cell(2, 2) },
    );
    refused(
        &mut b,
        fly(5, 2),
        &RuleError::CannotEnter { to: cell(5, 2) },
    );
    refused(
        &mut b,
        fly(0, 2),
        &RuleError::CannotEnter { to: cell(0, 2) },
    );
    refused(
        &mut b,
        fly(6, 2),
        &RuleError::CannotEnter { to: cell(6, 2) },
    );

    let events = act(&mut b, fly(x, y)).unwrap();
    assert_eq!(
        events,
        [Event::Moved {
            stack: "p0".to_owned(),
            from: cell(0, 2),
            to: cell(x, y),
            flew: true
        }]
    );
    let active = b.active().unwrap();
    assert_eq!(
        active.movement,
        number(&case["expected"]["movement_remaining"])
    );
    assert_eq!(
        active.flights,
        number(&case["expected"]["flights_remaining"])
    );
    refused(&mut b, fly(4, 3), &RuleError::NoFlights);
    assert_eq!(
        b.legal_actions()[0],
        melee("e0"),
        "it may still strike after landing"
    );

    // A troop that cannot fly at all.
    let mut ground = battle("melee_exchange");
    refused(&mut ground, fly(0, 0), &RuleError::NoFlights);
}

// AT-018: waiting moves a stack to the end of the round, once.
#[test]
fn a_stack_that_waits_acts_once_at_the_end_of_the_round() {
    let mut b = battle("blocked_shooter");
    let events = act(&mut b, Action::Wait).unwrap();
    assert_eq!(
        events[0],
        Event::Waited {
            stack: "p0".to_owned()
        }
    );
    assert_eq!(b.active().unwrap().stack, "p1");
    act(&mut b, Action::Wait).unwrap();
    b.apply(&pass(&b)).unwrap(); // the enemy
    // The two that waited, in the order they waited, with all their movement.
    let active = b.active().unwrap().clone();
    assert_eq!(
        (active.stack.as_str(), active.movement, b.round()),
        ("p0", 2, 1)
    );
    // Waiting again is a pass: the stack does not go round a second time.
    let events = act(&mut b, Action::Wait).unwrap();
    assert_eq!(
        events[0],
        Event::ActivationEnded {
            stack: "p0".to_owned()
        }
    );
    assert_eq!(b.active().unwrap().stack, "p1");
    b.apply(&pass(&b)).unwrap();
    assert_eq!((b.round(), b.active().unwrap().stack.as_str()), (2, "p0"));
    assert!(!b.stack("p0").unwrap().waited, "a new round, a new wait");
}

#[test]
fn a_stack_killed_while_waiting_does_not_act() {
    let mut b = battle_with("blocked_shooter", |s| {
        s.stacks
            .iter_mut()
            .find(|stack| stack.id == "p0")
            .unwrap()
            .count = 1;
    });
    act(&mut b, Action::Wait).unwrap();
    b.apply(&pass(&b)).unwrap(); // p1
    apply_forced(&mut b, melee("p0"), &[2]).unwrap(); // the enemy kills the waiting slinger
    assert!(!b.stack("p0").unwrap().alive());
    assert_eq!((b.round(), b.active().unwrap().stack.as_str()), (2, "p1"));
}

fn with_spells(id: &str, blast: u32, mend: u32) -> Battle {
    battle_with(id, |s| {
        s.hero_charges.insert("blast".to_owned(), blast);
        s.hero_charges.insert("mend".to_owned(), mend);
    })
}

// AT-019: Mend takes injury off; it brings nobody back.
#[test]
fn mend_removes_injury_and_raises_no_creature() {
    let mut b = with_spells("melee_exchange", 0, 2);
    refused(
        &mut b,
        cast("mend", "p0"),
        &RuleError::WrongSpellTarget {
            target: "p0".to_owned(),
        },
    );
    apply_forced(&mut b, melee("e0"), &[2, 2]).unwrap(); // 8 enemies strike back for 16: 9 left, 6 injury
    assert_eq!(count_and_injury(&b, "p0"), (9, 6));
    b.apply(&pass(&b)).unwrap();
    assert_eq!(b.round(), 2);

    let events = act(&mut b, cast("mend", "p0")).unwrap();
    assert_eq!(
        events[1],
        Event::Healed {
            target: "p0".to_owned(),
            removed: 6,
            injury: 0
        }
    );
    assert_eq!(count_and_injury(&b, "p0"), (9, 0));
    assert_eq!(b.hero_charges()["mend"], 1);
    assert_eq!(
        b.active().unwrap().stack,
        "p0",
        "casting does not use up the turn"
    );
    assert_eq!(b.active().unwrap().movement, 2);
}

// AT-020: a cast that is refused costs nothing; one cast a round.
#[test]
fn a_refused_cast_spends_nothing_and_the_hero_casts_once_a_round() {
    let mut b = with_spells("melee_exchange", 2, 1);
    refused(
        &mut b,
        cast("blast", "p0"),
        &RuleError::WrongSpellTarget {
            target: "p0".to_owned(),
        },
    );
    refused(
        &mut b,
        cast("blast", "ghost"),
        &RuleError::WrongSpellTarget {
            target: "ghost".to_owned(),
        },
    );
    refused(
        &mut b,
        cast("mend", "e0"),
        &RuleError::WrongSpellTarget {
            target: "e0".to_owned(),
        },
    );
    refused(
        &mut b,
        cast("storm", "e0"),
        &RuleError::NoCharge {
            spell: "storm".to_owned(),
        },
    );
    assert_eq!(
        (b.hero_charges()["blast"], b.hero_charges()["mend"]),
        (2, 1)
    );

    let rng = b.rng_state();
    let events = act(&mut b, cast("blast", "e0")).unwrap();
    assert_eq!(
        events[0],
        Event::SpellCast {
            spell: "blast".to_owned(),
            target: "e0".to_owned()
        }
    );
    assert!(matches!(
        &events[1],
        Event::Damage {
            damage: 18,
            losses: 1,
            count: 9,
            injury: 8,
            ..
        }
    ));
    assert_eq!(b.rng_state(), rng, "a spell rolls nothing");
    assert_eq!(b.hero_charges()["blast"], 1);
    refused(&mut b, cast("blast", "e0"), &RuleError::AlreadyCast);

    // Out of charges.
    let mut empty = with_spells("melee_exchange", 0, 0);
    refused(
        &mut empty,
        cast("blast", "e0"),
        &RuleError::NoCharge {
            spell: "blast".to_owned(),
        },
    );
}

// AT-021: only the player side has a hero.
#[test]
fn the_enemy_cannot_cast() {
    let mut b = with_spells("melee_exchange", 2, 2);
    b.apply(&pass(&b)).unwrap();
    assert_eq!(b.active().unwrap().stack, "e0");
    refused(&mut b, cast("blast", "p0"), &RuleError::NoHero);
    assert_eq!(b.hero_charges()["blast"], 2);
}

#[test]
fn a_spell_can_end_the_battle() {
    let mut b = battle_with("melee_exchange", |s| {
        s.hero_charges.insert("blast".to_owned(), 1);
        s.stacks
            .iter_mut()
            .find(|stack| stack.id == "e0")
            .unwrap()
            .count = 1;
    });
    let events = act(&mut b, cast("blast", "e0")).unwrap();
    assert_eq!(b.outcome(), Some(Outcome::PlayerWin));
    assert_eq!(
        events.last(),
        Some(&Event::BattleEnded {
            outcome: Outcome::PlayerWin
        })
    );
}

#[test]
fn every_legal_action_is_accepted_in_every_fixture() {
    for id in [
        "melee_exchange",
        "blocked_shooter",
        "flight_barrier",
        "unreachable_melee",
    ] {
        let b = with_spells(id, 1, 1);
        let actions = b.legal_actions();
        assert_eq!(actions.last(), Some(&Action::Pass), "{id}");
        for action in actions {
            let mut copy = b.clone();
            assert!(act(&mut copy, action.clone()).is_ok(), "{id}: {action:?}");
        }
    }
}

mod reference;
