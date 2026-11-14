//! A battle under each set of rules comes back from its saved form as it was, at every command, and goes on from
//! there to the same end: queues, flags, control and the random state included.

use std::sync::Arc;

use bounty_ai::approach_v1;
use bounty_core::battle::Battle;
use bounty_core::content::{Rules, scenarios_from_json};

const PROTOTYPE: &str = include_str!("../../../prototype/rules.json");
const PROTOTYPE_BATTLES: &str = include_str!("../../../prototype/battles.json");
const REFERENCE: &str = include_str!("../../../content/reference/rules.json");
const MODERN: &str = include_str!("../../../content/modern/rules.json");
const REFERENCE_BATTLES: &str = include_str!("../../../content/reference/battles.json");

/// Plays every battle of the file with the pilot, twice: straight through, and going on from the reloaded battle
/// after every command. Gives the number of commands checked.
fn round_trips(rules: &str, battles: &str) -> usize {
    let rules = Arc::new(Rules::from_json(rules).unwrap());
    let mut commands = 0;
    for scenario in scenarios_from_json(battles, &rules).unwrap() {
        for seed in [None, Some(7), Some(1990)] {
            let mut through = Battle::start(Arc::clone(&rules), &scenario, seed).unwrap();
            let mut reloaded = through.clone();
            while let Some(command) = approach_v1(&through.observe()) {
                assert_eq!(approach_v1(&reloaded.observe()).as_ref(), Some(&command));
                through.apply(&command).unwrap();
                reloaded.apply(&command).unwrap();
                reloaded = Battle::from_canon(Arc::clone(&rules), &reloaded.to_canon())
                    .unwrap_or_else(|error| panic!("{} after {command:?}: {error}", scenario.id));
                assert_eq!(reloaded, through, "{} after {command:?}", scenario.id);
                assert_eq!(reloaded.legal_actions(), through.legal_actions());
                commands += 1;
            }
            assert_eq!(reloaded.digest(), through.digest());
        }
    }
    commands
}

#[test]
fn prototype_battles_round_trip() {
    assert!(round_trips(PROTOTYPE, PROTOTYPE_BATTLES) > 100);
}

#[test]
fn reference_battles_round_trip() {
    assert!(round_trips(REFERENCE, REFERENCE_BATTLES) > 100);
}

#[test]
fn modern_battles_round_trip() {
    assert!(round_trips(MODERN, REFERENCE_BATTLES) > 100);
}
