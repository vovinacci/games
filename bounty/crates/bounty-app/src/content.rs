//! The content compiled into the game: for now the synthetic prototype rules and their battles.

use bounty_core::content::{ContentError, Rules, Scenario, scenarios_from_json};

const RULES: &str = include_str!("../../../prototype/rules.json");
const BATTLES: &str = include_str!("../../../prototype/battles.json");

/// The rules. The file is part of the program and checked by the tests, so it loads.
///
/// # Panics
/// The compiled-in rules file is broken: a mistake caught by the first test run, not by a player.
#[must_use]
pub fn rules() -> Rules {
    Rules::from_json(RULES).expect("the compiled-in rules load")
}

/// The scenario with this ID.
///
/// # Errors
/// The battles file is broken, or it has no scenario of this name; the message lists the ones it has.
pub fn scenario(rules: &Rules, id: &str) -> Result<Scenario, ContentError> {
    let all = scenarios_from_json(BATTLES, rules)?;
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
}
