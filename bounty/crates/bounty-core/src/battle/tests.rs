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
