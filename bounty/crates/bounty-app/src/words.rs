//! Words for what the rules report: names of stacks and one line of text for an event. No window is needed, so the
//! tests can read every line in every language.

use bounty_core::battle::{Battle, Event, Outcome};
use bounty_core::content::{Morale, Side};
use gamekit::i18n::{Language, tr};
use gamekit::t;

/// The game's languages; the first is the default and the fallback.
pub static LANGUAGES: [Language; 2] = [
    Language {
        id: "en-GB",
        name: "English",
        source: include_str!("../assets/i18n/en-GB.ftl"),
    },
    Language {
        id: "uk",
        name: "\u{423}\u{43a}\u{440}\u{430}\u{457}\u{43d}\u{441}\u{44c}\u{43a}\u{430}", // the language's own name, in escapes to keep this file ASCII
        source: include_str!("../assets/i18n/uk.ftl"),
    },
];

/// The name of a troop or spell, or its ID when the translation files have none: content can name a troop the
/// texts do not know yet, and it must still show.
pub fn named(prefix: &str, id: &str) -> String {
    let key = format!("{prefix}-{id}");
    let text = tr(&key, &[]);
    if text == key { id.to_owned() } else { text }
}

/// A stack as the log and the sidebar call it: its troop with its side, "your Wardens".
pub fn stack_name(battle: &Battle, id: &str) -> String {
    let Some(stack) = battle.stack(id) else {
        return id.to_owned();
    };
    let troop = named("troop", &stack.troop_id);
    match stack.side {
        Side::Player => t!("name-player", troop = troop),
        Side::Enemy => t!("name-enemy", troop = troop),
    }
}

/// A morale as the sidebar names it.
pub fn morale_text(morale: Morale) -> String {
    match morale {
        Morale::Low => t!("morale-low"),
        Morale::Normal => t!("morale-normal"),
        Morale::High => t!("morale-high"),
    }
}

pub fn outcome_text(outcome: Outcome) -> String {
    tr(&format!("outcome-{}", outcome.name()), &[])
}

/// One line for the log, or `None` for an event the log leaves out (whose turn it is shows in the sidebar; the
/// end of the battle shows as a banner).
pub fn describe(event: &Event, battle: &Battle) -> Option<String> {
    let name = |id: &str| stack_name(battle, id);
    Some(match event {
        Event::RoundStarted { round } => t!("log-round", n = *round),
        Event::Damage {
            attacker,
            target,
            roll,
            attackers,
            damage,
            losses,
            retaliation,
            morale,
            ..
        } => {
            if battle.stack(attacker).is_none() {
                // No stack struck: the hero's spell. Which spell is in the line before; here its damage.
                t!(
                    "log-spell-damage",
                    spell = named("spell", "blast"),
                    target = name(target),
                    damage = *damage,
                    losses = *losses
                )
            } else {
                // The damage is given as the rules made it; roll and number are what went in. No sum is shown,
                // since skill and morale stand between them and the result.
                let key = if *retaliation {
                    "log-retaliation"
                } else {
                    "log-damage"
                };
                let morale = match morale {
                    Some(Morale::Low) => t!("log-morale-low"),
                    Some(Morale::High) => t!("log-morale-high"),
                    Some(Morale::Normal) | None => String::new(),
                };
                tr(
                    key,
                    &[
                        ("attacker", name(attacker).into()),
                        ("target", name(target).into()),
                        ("roll", (*roll).into()),
                        ("attackers", (*attackers).into()),
                        ("damage", (*damage).into()),
                        ("losses", (*losses).into()),
                        ("morale", morale.into()),
                    ],
                )
            }
        }
        Event::StackRemoved { stack } => t!("log-removed", name = name(stack)),
        Event::Moved { stack, flew, .. } => tr(
            if *flew { "log-flew" } else { "log-moved" },
            &[("name", name(stack).into())],
        ),
        Event::Waited { stack } => t!("log-waited", name = name(stack)),
        Event::Healed {
            target, removed, ..
        } => t!("log-healed", name = name(target), removed = *removed),
        Event::ControlRegained { stack } => t!("log-control", name = name(stack)),
        Event::SideStarted { .. }
        | Event::ActivationStarted { .. }
        | Event::ActivationEnded { .. }
        | Event::SpellCast { .. }
        | Event::BattleEnded { .. } => {
            return None;
        }
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;
    use std::sync::Arc;

    use bounty_core::battle::{Action, Command};
    use bounty_core::content::Cell;
    use gamekit::i18n;

    use super::*;
    use crate::content;

    #[test]
    fn both_languages_have_the_same_keys() {
        assert_eq!(i18n::missing_keys(&LANGUAGES), Vec::<String>::new());
    }

    #[test]
    fn every_key_used_in_the_code_exists() {
        let known = i18n::keys(LANGUAGES[0].source);
        let used = i18n::keys_used_in(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src")));
        let unknown: Vec<&String> = used.difference(&known).collect();
        assert!(unknown.is_empty(), "keys not in en-GB.ftl: {unknown:?}");
    }

    /// Keys put together at run time, which the scan of the code cannot see.
    #[test]
    fn keys_built_at_run_time_exist() {
        let rules = content::rules();
        let mut needed: Vec<String> = Vec::new();
        needed.extend(rules.troops.iter().map(|t| format!("troop-{}", t.id)));
        needed.extend(rules.spells.iter().map(|s| format!("spell-{}", s.id)));
        needed.extend(
            [
                Outcome::PlayerWin,
                Outcome::EnemyWin,
                Outcome::BudgetExceeded,
            ]
            .map(|o| format!("outcome-{}", o.name())),
        );
        needed.extend(["log-damage", "log-retaliation", "log-flew", "log-moved"].map(String::from));
        let known = i18n::keys(LANGUAGES[0].source);
        let missing: Vec<&String> = needed.iter().filter(|key| !known.contains(*key)).collect();
        assert!(missing.is_empty(), "keys not in en-GB.ftl: {missing:?}");
    }

    /// A battle in which everything the log can say happens once.
    fn eventful() -> (Battle, Vec<Event>) {
        let rules = Arc::new(content::rules());
        let mut scenario = content::scenario(&rules, "flight_barrier").unwrap();
        scenario.hero_charges.insert("blast".to_owned(), 1);
        scenario.hero_charges.insert("mend".to_owned(), 1);
        let mut battle = Battle::start(rules, &scenario, None).unwrap();
        let mut events = Vec::new();
        let mut act = |battle: &mut Battle, action: Action| {
            let command = Command {
                revision: battle.revision(),
                actor: battle.active().unwrap().stack.clone(),
                action,
            };
            events.extend(battle.apply(&command).unwrap());
        };
        act(
            &mut battle,
            Action::Cast {
                spell: "blast".to_owned(),
                target: "e0".to_owned(),
            },
        );
        act(
            &mut battle,
            Action::Fly {
                to: Cell { x: 4, y: 2 },
            },
        );
        act(
            &mut battle,
            Action::Melee {
                target: "e0".to_owned(),
            },
        );
        act(&mut battle, Action::Wait);
        act(&mut battle, Action::Pass);
        act(
            &mut battle,
            Action::Cast {
                spell: "mend".to_owned(),
                target: "p0".to_owned(),
            },
        );
        act(
            &mut battle,
            Action::Walk {
                to: Cell { x: 4, y: 3 },
            },
        );
        (battle, events)
    }

    #[test]
    fn every_line_of_the_log_is_real_text_in_every_language() {
        i18n::init(&LANGUAGES);
        let (battle, events) = eventful();
        let mut kinds = BTreeSet::new();
        for place in 0..i18n::language_count() {
            i18n::set_language(place);
            for event in &events {
                let Some(line) = describe(event, &battle) else {
                    continue;
                };
                kinds.insert(line.split([':', ' ']).next().unwrap_or_default().to_owned());
                assert!(
                    !line.contains("log-") && !line.contains('{') && !line.contains("troop-"),
                    "{line}"
                );
            }
            assert_eq!(stack_name(&battle, "ghost"), "ghost");
            assert_eq!(
                named("troop", "dragon"),
                "dragon",
                "a troop the texts do not know shows by its ID"
            );
            assert!(!outcome_text(Outcome::BudgetExceeded).starts_with("outcome-"));
        }
        // Damage, strike back, spell damage, flight, step, wait, healing, a new round: eight kinds in each language.
        assert!(kinds.len() >= 14, "{kinds:?}");
        i18n::set_language(0);
        assert_eq!(stack_name(&battle, "p0"), "your Skyriders");
        assert_eq!(stack_name(&battle, "e0"), "enemy Wardens");
    }

    // Under the reference rules the log also tells of morale and of control coming back.
    #[test]
    fn the_log_tells_of_morale_and_of_control() {
        i18n::init(&LANGUAGES);
        let rules = Arc::new(content::rules_named("reference").unwrap());
        let scenario = content::scenario(&rules, "morale_sprites_wolves").unwrap();
        let mut battle = Battle::start(rules, &scenario, None).unwrap();
        // The player's Sprites fly beside the enemy's and strike them, at Low morale.
        let act = |battle: &mut Battle, action: Action| {
            let command = Command {
                revision: battle.revision(),
                actor: battle.active().unwrap().stack.clone(),
                action,
            };
            battle.apply(&command).unwrap()
        };
        act(
            &mut battle,
            Action::Fly {
                to: Cell { x: 4, y: 0 },
            },
        );
        let events = act(
            &mut battle,
            Action::Melee {
                target: "e0".to_owned(),
            },
        );
        let control = Event::ControlRegained {
            stack: "p0".to_owned(),
        };
        for place in 0..i18n::language_count() {
            i18n::set_language(place);
            let blow = describe(&events[0], &battle).unwrap();
            assert!(blow.contains(&t!("log-morale-low")), "{blow}");
            assert!(!blow.contains('{') && !blow.contains("log-"), "{blow}");
            let back = describe(&control, &battle).unwrap();
            assert!(
                back.contains(&stack_name(&battle, "p0")) && !back.contains("log-"),
                "{back}"
            );
            for morale in [Morale::Low, Morale::Normal, Morale::High] {
                assert!(!morale_text(morale).starts_with("morale-"));
            }
        }
        i18n::set_language(0);
    }
}
