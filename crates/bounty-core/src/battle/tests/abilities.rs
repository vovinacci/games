//! What some troops of the reference rules do that the others do not (`docs/spec/07-RULES-REGISTER.md`, U-12).
//! The numbers are worked out by hand from the rules as the register states them: damage is roll x creatures x
//! (attacker's skill + 5 - target's skill) / 10.

use std::collections::BTreeMap;

use super::reference::{damage_events, stack};
use super::*;
use crate::content::{Ability, StackStart};

fn rules() -> Arc<Rules> {
    Arc::new(Rules::from_json(include_str!("../../../../../content/reference/rules.json")).unwrap())
}

fn fight(stacks: Vec<StackStart>) -> Battle {
    let scenario = Scenario {
        id: "test".to_owned(),
        rng_state: 1,
        obstacles: vec![],
        hero_charges: BTreeMap::new(),
        leadership: None,
        stacks,
    };
    Battle::start(rules(), &scenario, None).unwrap()
}

fn grown(events: &[Event]) -> Vec<(&str, u32, u32, u32)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Grew {
                stack,
                gained,
                count,
                injury,
            } => Some((stack.as_str(), *gained, *count, *injury)),
            _ => None,
        })
        .collect()
}

#[test]
fn the_troops_carry_their_abilities() {
    let rules = rules();
    let of = |id: &str| rules.troop(id).unwrap().abilities.clone();
    assert_eq!(of("trolls"), [Ability::Regeneration]);
    assert_eq!(of("ghosts"), [Ability::Absorb, Ability::Undead]);
    assert_eq!(of("vampires"), [Ability::Leech, Ability::Undead]);
    assert_eq!(of("skeletons"), [Ability::Undead]);
    assert_eq!(of("demons"), [Ability::Scythe]);
    assert_eq!(of("druids"), [Ability::MagicShot]);
    assert_eq!(of("dragons"), [Ability::Immune]);
    assert_eq!(of("wolves"), []);
}

// Ghosts (skill 4, ten health) on Militia (skill 2, two health): the creatures a blow kills join the Ghosts, with
// no limit, and they are there for the next blow.
#[test]
fn ghosts_grow_by_what_they_kill() {
    let mut b = fight(vec![
        stack("p0", "ghosts", 10, 1, 2),
        stack("e0", "militia", 30, 2, 2),
    ]);
    // 3 x 10 = 30, times (4 + 5 - 2) = 7, over ten: 21. Ten Militia die and one is hurt by 1.
    // The answer: all 30 strike. 1 x 30, times (2 + 5 - 4) = 3, over ten: 9, an injury.
    let events = apply_forced(&mut b, melee("e0"), &[3, 1]).unwrap();
    assert_eq!(grown(&events), [("p0", 10, 20, 0)]);
    assert_eq!(
        (count_and_injury(&b, "p0"), count_and_injury(&b, "e0")),
        ((20, 9), (20, 1))
    );
    let blow = events
        .iter()
        .position(|e| matches!(e, Event::Damage { .. }));
    let grew = events.iter().position(|e| matches!(e, Event::Grew { .. }));
    assert!(blow < grew, "the stack grows after its blow");

    // The Militia's turn: 20 strike, 1 x 20 x 3 / 10 = 6, with the 9 from before 15: one Ghost dies. The answer
    // comes from the 20 Ghosts there were: 3 x 20 x 7 / 10 = 42, with the 1 from before 43: 21 Militia would die
    // of 20, so all do, and all twenty join.
    let events = apply_forced(&mut b, melee("p0"), &[1, 3]).unwrap();
    assert_eq!(grown(&events), [("p0", 20, 39, 5)]);
    assert_eq!(b.outcome(), Some(Outcome::PlayerWin));
    let grew = events.iter().position(|e| matches!(e, Event::Grew { .. }));
    let ended = events
        .iter()
        .position(|e| matches!(e, Event::BattleEnded { .. }));
    assert!(grew < ended, "and before the battle is called");
}

// Vampires (skill 5, thirty health) on Militia: for every thirty health a blow takes, one Vampire is back, but
// never more than began the battle; at that number the stack is whole again, its injury gone.
#[test]
fn vampires_regain_creatures_up_to_the_number_they_began_with() {
    let mut b = fight(vec![
        stack("p0", "vampires", 20, 1, 2),
        stack("e0", "militia", 60, 2, 2),
    ]);
    assert_eq!(b.stack("p0").unwrap().began_with, Some(20));
    assert_eq!(b.stack("e0").unwrap().began_with, None);
    // 3 x 20 x (5 + 5 - 2) / 10 = 48: 24 Militia die. That is one Vampire's health and more, but all twenty stand.
    // The answer, from 60: 2 x 60 x (2 + 5 - 5) / 10 = 24, an injury.
    let events = apply_forced(&mut b, melee("e0"), &[3, 2]).unwrap();
    assert_eq!(grown(&events), []);
    assert_eq!(
        (count_and_injury(&b, "p0"), count_and_injury(&b, "e0")),
        ((20, 24), (36, 0))
    );
    // The Militia's turn: 2 x 36 x 2 / 10 = 14, with the 24 from before 38: one Vampire dies, the next is hurt
    // by 8. The answer, from 20: 6 x 20 x 8 / 10 = 96, which kills all 36 Militia. What it took counts as their
    // 72 health: two Vampires' worth, one more than the stack may have. So it is twenty again, and unhurt.
    let events = apply_forced(&mut b, melee("p0"), &[2, 6]).unwrap();
    assert_eq!(grown(&events), [("p0", 1, 20, 0)]);
    assert_eq!(count_and_injury(&b, "p0"), (20, 0));
}

// Below the number it began with, a stack that leeches keeps its injury.
#[test]
fn a_leech_that_stays_below_its_number_keeps_its_injury() {
    let mut b = fight(vec![
        stack("p0", "vampires", 20, 1, 2),
        stack("e0", "militia", 400, 2, 2),
    ]);
    // 48 damage, 24 Militia die. The answer, from 400: 2 x 400 x 2 / 10 = 160: five Vampires die, one is hurt
    // by 10.
    apply_forced(&mut b, melee("e0"), &[3, 2]).unwrap();
    assert_eq!(count_and_injury(&b, "p0"), (15, 10));
    // The Militia's turn: 2 x 376 x 2 / 10 = 150, with the 10 from before 160: five more die, hurt by 10 again.
    // The answer, from 15: 6 x 15 x 8 / 10 = 72, which is 36 Militia and two Vampires' health: twelve stand.
    let events = apply_forced(&mut b, melee("p0"), &[2, 6]).unwrap();
    assert_eq!(grown(&events), [("p0", 2, 12, 10)]);
}

// Trolls (skill 4, fifty health): whatever injury a Troll stack has is gone when a round begins, on both sides.
#[test]
fn trolls_have_no_injury_when_a_round_begins() {
    let mut b = fight(vec![
        stack("p0", "trolls", 10, 1, 2),
        stack("e0", "trolls", 10, 2, 2),
    ]);
    // Each blow: 2 x 10 x (4 + 5 - 4) / 10 = 10.
    apply_forced(&mut b, melee("e0"), &[2, 2]).unwrap();
    assert_eq!(
        (count_and_injury(&b, "p0"), count_and_injury(&b, "e0")),
        ((10, 10), (10, 10)),
        "nothing heals when the enemy's part of the round begins"
    );
    let events = apply_forced(&mut b, melee("p0"), &[2, 2]).unwrap();
    let healed: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            Event::Healed {
                target,
                removed,
                injury,
            } => Some((target.as_str(), *removed, *injury)),
            _ => None,
        })
        .collect();
    assert_eq!(healed, [("p0", 20, 0), ("e0", 20, 0)]);
    assert_eq!(b.round(), 2);
    assert_eq!(
        (count_and_injury(&b, "p0"), count_and_injury(&b, "e0")),
        ((10, 0), (10, 0))
    );
}

// The number a leech began with is part of the state: a battle saved and read again is the same battle, and a
// save that lacks the number, or has it for a troop without the ability, is refused.
#[test]
fn the_number_a_leech_began_with_is_saved() {
    let mut b = fight(vec![
        stack("p0", "vampires", 20, 1, 2),
        stack("e0", "militia", 400, 2, 2),
    ]);
    apply_forced(&mut b, melee("e0"), &[3, 2]).unwrap();
    let saved = b.to_canon();
    assert_eq!(Battle::from_canon(rules(), &saved).unwrap(), b);

    let text = String::from_utf8(saved.to_bytes().unwrap()).unwrap();
    assert_eq!(text.matches("began_with").count(), 1, "only the Vampires'");
    let without = Value::parse(&text.replace(r#""began_with":20,"#, "")).unwrap();
    assert!(Battle::from_canon(rules(), &without).is_err());
}

fn scythed(events: &[Event]) -> Vec<(&str, u32)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Scythed {
                target, creatures, ..
            } => Some((target.as_str(), *creatures)),
            _ => None,
        })
        .collect()
}

// Demons (skill 6): before each of their blows a number from 1 to 100 is rolled, and above 89 the scythe takes
// half the target, rounded up, on top of the blow. Struck back, they roll for it as well.
#[test]
fn the_scythe_takes_half_the_target_on_a_roll_above_89() {
    let demons_on_militia = || {
        fight(vec![
            stack("p0", "demons", 2, 1, 2),
            stack("e0", "militia", 21, 2, 2),
        ])
    };
    // 89 is not enough: the blow alone, 5 x 2 x (6 + 5 - 2) / 10 = 9, four Militia and an injury of 1.
    // The answer, from 21: 1 x 21 x (2 + 5 - 6) / 10 = 2.
    let mut b = demons_on_militia();
    let events = apply_forced(&mut b, melee("e0"), &[89, 5, 1]).unwrap();
    assert_eq!(scythed(&events), []);
    assert_eq!(
        (count_and_injury(&b, "p0"), count_and_injury(&b, "e0")),
        ((2, 2), (17, 1))
    );
    // The Militia's turn: 1 x 17 x 1 / 10 = 1, an injury of 3 now. The Demons answer and roll 100: half of 17 is
    // 9, eighteen health, with the blow's 9 and the 1 from before 28: fourteen Militia die.
    let events = apply_forced(&mut b, melee("p0"), &[1, 100, 5]).unwrap();
    assert_eq!(scythed(&events), [("e0", 9)]);
    assert_eq!(
        (count_and_injury(&b, "p0"), count_and_injury(&b, "e0")),
        ((2, 3), (3, 0))
    );

    // 90 is: half of 21 is 11, twenty-two health, with the blow's 9 thirty-one: fifteen die, one is hurt by 1.
    let mut b = demons_on_militia();
    let events = apply_forced(&mut b, melee("e0"), &[90, 5, 1]).unwrap();
    assert_eq!(scythed(&events), [("e0", 11)]);
    assert_eq!(
        damage_events(&events)[0],
        (5, 2, 31, 15, 6, 1, false),
        "the blow's damage has the scythe's in it"
    );
    let fell = events
        .iter()
        .position(|e| matches!(e, Event::Scythed { .. }));
    let blow = events
        .iter()
        .position(|e| matches!(e, Event::Damage { .. }));
    assert!(fell < blow, "the scythe is told before the blow");
}

// Druids (skill 5): a magic shot is one number, with nothing rolled: 10 x 3 x (5 + 5 - 2) / 10 = 24, twelve Militia.
#[test]
fn a_magic_shot_is_a_fixed_number() {
    let mut b = fight(vec![
        stack("p0", "druids", 3, 0, 0),
        stack("e0", "militia", 30, 5, 4),
    ]);
    let drawn = b.rng_state();
    let events = act(&mut b, shoot("e0")).unwrap();
    assert_eq!(damage_events(&events), [(10, 3, 24, 12, 18, 0, false)]);
    assert_eq!(b.rng_state(), drawn, "nothing was rolled");
    assert_eq!(b.stack("p0").unwrap().shots, 2);
}

// Dragons: magic does nothing to them. The shot is spent all the same and the turn is over. An arrow is no magic.
#[test]
fn a_magic_shot_does_nothing_to_the_immune_and_an_arrow_does() {
    let mut b = fight(vec![
        stack("p0", "druids", 3, 0, 0),
        stack("p1", "archers", 10, 0, 2),
        stack("e0", "dragons", 2, 5, 4),
    ]);
    let events = act(&mut b, shoot("e0")).unwrap();
    assert!(
        events.contains(&Event::Fizzled {
            attacker: "p0".to_owned(),
            target: "e0".to_owned(),
        }),
        "{events:?}"
    );
    assert_eq!(damage_events(&events), []);
    assert_eq!(b.stack("p0").unwrap().shots, 2, "the shot is spent");
    assert_eq!(count_and_injury(&b, "e0"), (2, 0));
    assert_eq!(b.active().unwrap().stack, "p1", "and the turn is over");
    // Ten Archers (skill 2): 3 x 10 x (2 + 5 - 6) / 10 = 3.
    let events = apply_forced(&mut b, shoot("e0"), &[3]).unwrap();
    assert_eq!(damage_events(&events), [(3, 10, 3, 0, 2, 3, false)]);
}
