//! The content compiled into the game: the synthetic prototype rules with their battles, and the first rules
//! taken from a reimplementation of the original with theirs.

use bounty_core::content::{ContentError, Profile, Rules, Scenario, scenarios_from_json};

const RULES: &str = include_str!("../../../prototype/rules.json");
const BATTLES: &str = include_str!("../../../prototype/battles.json");
const REFERENCE_RULES: &str = include_str!("../../../content/reference/rules.json");
const MODERN_RULES: &str = include_str!("../../../content/modern/rules.json");
const REFERENCE_BATTLES: &str = include_str!("../../../content/reference/battles.json");

/// The prototype rules. The file is part of the program and checked by the tests, so it loads.
///
/// # Panics
/// The compiled-in rules file is broken: a mistake caught by the first test run, not by a player.
#[must_use]
pub fn rules() -> Rules {
    Rules::from_json(RULES).expect("the compiled-in rules load")
}

/// The rules called `name` on the command line: `prototype`, `reference`, or `modern`, the variant of the
/// reference rules, which plays the reference's battles.
///
/// # Errors
/// Another name; the message lists the two.
///
/// # Panics
/// As [`rules`].
pub fn rules_named(name: &str) -> Result<Rules, String> {
    match name {
        "prototype" => Ok(rules()),
        "reference" => {
            Ok(Rules::from_json(REFERENCE_RULES).expect("the compiled-in reference rules load"))
        }
        "modern" => Ok(Rules::from_json(MODERN_RULES).expect("the compiled-in variant rules load")),
        other => Err(format!(
            "no rules {other:?}; there are: prototype, reference, modern"
        )),
    }
}

/// The battle a ruleset starts with when none is named.
#[must_use]
pub fn first_scenario(rules: &Rules) -> &'static str {
    match rules.profile {
        Profile::Prototype => "blocked_shooter",
        Profile::Reference(_) => "even_fight",
    }
}

/// The scenario with this ID, among the battles of these rules.
///
/// # Errors
/// The battles file is broken, or it has no scenario of this name; the message lists the ones it has.
pub fn scenario(rules: &Rules, id: &str) -> Result<Scenario, ContentError> {
    let battles = match rules.profile {
        Profile::Prototype => BATTLES,
        Profile::Reference(_) => REFERENCE_BATTLES,
    };
    let all = scenarios_from_json(battles, rules)?;
    let names: Vec<String> = all.iter().map(|s| s.id.clone()).collect();
    all.into_iter().find(|s| s.id == id).ok_or_else(|| {
        ContentError::Invalid(format!("no battle {id:?}; there are: {}", names.join(", ")))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_compiled_in_content_loads_and_names_what_it_lacks() {
        let rules = rules();
        assert!(scenario(&rules, "melee_exchange").is_ok());
        let error = scenario(&rules, "siege").unwrap_err().to_string();
        assert!(error.contains("flight_barrier"), "{error}");
    }

    #[test]
    fn both_rulesets_load_with_their_own_battles() {
        let reference = rules_named("reference").unwrap();
        assert_eq!(first_scenario(&reference), "even_fight");
        for id in [
            "even_fight",
            "morale_sprites_wolves",
            "loss_of_control",
            "overmatch",
        ] {
            assert!(scenario(&reference, id).is_ok(), "{id}");
        }
        assert!(
            scenario(&reference, "blocked_shooter").is_err(),
            "a prototype battle is not a reference battle"
        );
        assert!(scenario(&rules_named("prototype").unwrap(), first_scenario(&rules())).is_ok());
        assert!(
            rules_named("classic")
                .unwrap_err()
                .contains("prototype, reference, modern")
        );
        let modern = rules_named("modern").unwrap();
        assert_eq!(
            (modern.id.as_str(), first_scenario(&modern)),
            ("modern-v0.1", "even_fight")
        );
        assert!(
            scenario(&modern, "loss_of_control_deep").is_ok(),
            "the variant plays the reference's battles"
        );
    }
}
