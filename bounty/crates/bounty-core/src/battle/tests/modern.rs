//! The switches of the `modern` variant (`content/modern/rules.json`), each tried alone so that what it changes is
//! plain. The numbers are worked out by hand and set beside the reference's for the same blow.

use std::collections::BTreeMap;

use super::reference::{damage_events, stack};
use super::*;
use crate::content::{MODERN_RULESET, REFERENCE_RULESET, StackStart, scenarios_from_json};

const MODERN: &str = include_str!("../../../../../content/modern/rules.json");
const REFERENCE: &str = include_str!("../../../../../content/reference/rules.json");

/// Each switch as the rules files write it: its settings by name, the reference's first, under the empty name.
const SWITCHES: [&[(&str, &str)]; 4] = [
    &[
        (
            "",
            r#""count_used": "count_before_the_exchange",
    "retaliation_uses_survivors": false,"#,
        ),
        (
            "answer",
            r#""count_used": "count_at_the_blow",
    "retaliation_uses_survivors": true,"#,
        ),
        (
            "answer-mean",
            r#""count_used": "mean_of_before_and_after_the_blow",
    "retaliation_uses_survivors": false,"#,
        ),
    ],
    &[
        ("", r#""rolls": "one""#),
        ("rolls", r#""rolls": "mean_of_two""#),
    ],
    &[
        (
            "",
            r#""rule": "lowest_rank_over_own_side_self_included_chart_read_other_then_self""#,
        ),
        (
            "morale",
            r#""rule": "alone_high_else_others_read_self_then_other_any_low_is_low_all_high_is_high""#,
        ),
    ],
    &[
        ("", r#""over_leadership": "out_of_control""#),
        (
            "control",
            r#""over_leadership": "only_what_leadership_covers_fights""#,
        ),
    ],
];

/// The variant's rules with exactly the named settings; every switch not named is as the reference has it,
/// whatever the variant's own file says.
fn modern(on: &[&str]) -> Arc<Rules> {
    let mut text = MODERN.to_owned();
    for settings in SWITCHES {
        let now = settings
            .iter()
            .find(|(_, written)| text.contains(written))
            .expect("the file sets every switch");
        let wanted = settings
            .iter()
            .find(|(name, _)| on.contains(name))
            .unwrap_or(&settings[0]);
        text = text.replacen(now.1, wanted.1, 1);
    }
    Arc::new(Rules::from_json(&text).unwrap())
}

fn fight(rules: Arc<Rules>, leadership: Option<u32>, stacks: Vec<StackStart>) -> Battle {
    let scenario = Scenario {
        id: "test".to_owned(),
        rng_state: 1,
        obstacles: Vec::new(),
        hero_charges: BTreeMap::new(),
        leadership,
        stacks,
    };
    Battle::start(rules, &scenario, None).unwrap()
}

fn wolves_on_sprites() -> Vec<StackStart> {
    vec![
        stack("p0", "wolves", 10, 1, 2),
        stack("e0", "sprites", 30, 2, 2),
    ]
}

#[test]
fn the_variant_is_the_reference_with_some_switches_set() {
    let (variant, reference) = (
        Rules::from_json(MODERN).unwrap(),
        Rules::from_json(REFERENCE).unwrap(),
    );
    assert_eq!(
        (variant.id.as_str(), variant.battles_id.as_str()),
        (MODERN_RULESET, REFERENCE_RULESET)
    );
    // The same troops, board and chart: a variant changes rules, not content.
    assert_eq!(
        (&variant.troops, &variant.board, variant.round_limit),
        (&reference.troops, &reference.board, 200)
    );
    let (Profile::Reference(ours), Profile::Reference(theirs)) =
        (&variant.profile, &reference.profile)
    else {
        panic!("both of the reference family")
    };
    assert_eq!(ours.morale_chart, theirs.morale_chart);
    assert_eq!(theirs.switches, Switches::default());
    // What the variant has on is what the measurements bore out (`docs/spec/10-GAMEPLAY-GAPS.md`).
    let kept = Switches {
        army_view_morale: true,
        leadership_caps_fighters: true,
        ..Switches::default()
    };
    assert_eq!(ours.switches, kept);

    // It plays the reference's battles, and the prototype's not.
    let battles = include_str!("../../../../../content/reference/battles.json");
    assert_eq!(scenarios_from_json(battles, &variant).unwrap().len(), 5);
    assert!(scenarios_from_json(crate::content::tests::BATTLES, &variant).is_err());

    // With every switch at the reference's setting the two fight the same battle, under their own names.
    let mut ours = fight(modern(&[]), None, wolves_on_sprites());
    let mut theirs = fight(Arc::new(reference), None, wolves_on_sprites());
    assert_eq!(
        act(&mut ours, melee("e0")).unwrap(),
        act(&mut theirs, melee("e0")).unwrap()
    );
    assert_ne!(
        ours.digest(),
        theirs.digest(),
        "the ruleset is part of the state"
    );
}

#[test]
fn the_reference_cannot_take_a_variants_setting() {
    // Each setting of the variant, written into the reference's file, is refused. The reference file has no line
    // for the rolls or for what happens over the leadership; those two are tried by adding the line.
    for settings in SWITCHES {
        let reference = settings[0].1;
        for (name, written) in &settings[1..] {
            let changed = if REFERENCE.contains(reference) {
                REFERENCE.replacen(reference, written, 1)
            } else if *name == "rolls" {
                let line = r#""injury_persists_during_battle""#;
                REFERENCE.replacen(line, &format!("{written},\n    {line}"), 1)
            } else {
                let line = r#""checked": "before_every_activation""#;
                REFERENCE.replacen(line, &format!("{line},\n    {written}"), 1)
            };
            assert_ne!(changed, REFERENCE, "{name}");
            assert!(
                matches!(
                    Rules::from_json(&changed),
                    Err(ContentError::Unsupported(_))
                ),
                "{name}"
            );
        }
    }
    let unknown = MODERN.replacen(r#""rolls": "one""#, r#""rolls": "best_of_three""#, 1);
    assert!(matches!(
        Rules::from_json(&unknown),
        Err(ContentError::Unsupported(_))
    ));
    let other = r#""plays_battles_of": "prototype-v0.1""#;
    let other_battles = MODERN.replacen(r#""plays_battles_of": "reference-v0.1""#, other, 1);
    assert!(matches!(
        Rules::from_json(&other_battles),
        Err(ContentError::Invalid(_))
    ));
    // The yes-or-no line of the answer must agree with the setting.
    let disagreeing = MODERN.replacen(
        r#""retaliation_uses_survivors": false"#,
        r#""retaliation_uses_survivors": true"#,
        1,
    );
    assert!(matches!(
        Rules::from_json(&disagreeing),
        Err(ContentError::Unsupported(_))
    ));
}

// B4. The answer comes from the creatures left after the blow.
#[test]
fn with_the_answer_from_survivors_the_dead_do_not_strike_back() {
    let mut b = fight(modern(&["answer"]), None, wolves_on_sprites());
    let events = apply_forced(&mut b, melee("e0"), &[2, 1]).unwrap();
    assert_eq!(
        damage_events(&events),
        [
            // As under the reference: 2 x 10 = 20, times 6, over ten: 12 Sprites die, 18 are left.
            (2, 10, 12, 12, 18, 0, false),
            // The 18 answer, not the 30 of before: 1 x 18 = 18, times 4, over ten: 7. Two Wolves and one health.
            // Under the reference the answer was 12 and killed four.
            (1, 18, 7, 2, 8, 1, true),
        ]
    );
}

// B4, second try. The answer comes from the mean of the count before and after the blow.
#[test]
fn with_the_mean_answer_half_of_the_dead_strike_back() {
    let mut b = fight(modern(&["answer-mean"]), None, wolves_on_sprites());
    let events = apply_forced(&mut b, melee("e0"), &[2, 1]).unwrap();
    assert_eq!(
        damage_events(&events),
        [
            (2, 10, 12, 12, 18, 0, false),
            // 30 before and 18 after: 24 answer. 1 x 24 = 24, times 4, over ten: 9, three Wolves.
            // The reference's answer was 12 from all 30; the survivors' 7 from 18.
            (1, 24, 9, 3, 7, 0, true),
        ]
    );
    // An odd sum rounds down: 30 before, 21 after, 25 answer.
    let mut odd = fight(
        modern(&["answer-mean"]),
        None,
        vec![
            stack("p0", "wolves", 8, 1, 2),
            stack("e0", "sprites", 30, 2, 2),
        ],
    );
    let events = apply_forced(&mut odd, melee("e0"), &[2, 1]).unwrap();
    // 2 x 8 = 16, times 6, over ten: 9 Sprites die.
    assert_eq!(
        (damage_events(&events)[0].3, damage_events(&events)[1].1),
        (9, 25)
    );
}

// B5. A blow takes the mean of two rolls.
#[test]
fn with_two_rolls_a_blow_takes_their_mean() {
    let mut b = fight(modern(&["rolls"]), None, wolves_on_sprites());
    let events = apply_forced(&mut b, melee("e0"), &[1, 3, 2, 2]).unwrap();
    assert_eq!(
        damage_events(&events),
        [
            // Rolls 1 and 3, mean 2: (1 + 3) x 10 = 40, times 6, over twenty: 12.
            (4, 10, 12, 12, 18, 0, false),
            // The answer, from the 30 of before as under the reference: (2 + 2) x 30 = 120, times 4, over
            // twenty: 24, eight Wolves.
            (4, 30, 24, 8, 2, 0, true),
        ]
    );
    assert!(matches!(&events[0], Event::Damage { rolls: 2, .. }));

    // A mean that is not whole is not rounded before the rest of the sum: rolls 1 and 2 give
    // (1 + 2) x 10 = 30, times 6, over twenty: 9, where rounding the mean down first would give 6.
    let mut odd = fight(modern(&["rolls"]), None, wolves_on_sprites());
    let events = apply_forced(&mut odd, melee("e0"), &[1, 2, 1, 1]).unwrap();
    assert_eq!(damage_events(&events)[0], (3, 10, 9, 9, 21, 0, false));

    // Each blow draws twice from the generator.
    let (mut one, mut two) = (
        fight(modern(&[]), None, wolves_on_sprites()),
        fight(modern(&["rolls"]), None, wolves_on_sprites()),
    );
    act(&mut one, Action::Pass).unwrap();
    act(&mut two, Action::Pass).unwrap();
    assert_eq!(one.rng_state(), two.rng_state(), "no blow yet, no draw");
    act(&mut one, melee("p0")).unwrap();
    act(&mut two, melee("p0")).unwrap();
    assert_ne!(one.rng_state(), two.rng_state());
}

// B1. Morale by the rule the army screen shows.
#[test]
fn with_the_army_view_rule_the_wolves_are_low_and_not_the_sprites() {
    let army = || {
        vec![
            stack("p0", "sprites", 20, 1, 2),
            stack("p1", "wolves", 10, 1, 3),
            stack("e0", "wolves", 50, 2, 2),
        ]
    };
    let variant = fight(modern(&["morale"]), Some(1000), army());
    // (Sprites, Wolves) is Normal; (Wolves, Sprites) is Low. The reference has it the other way round.
    assert_eq!(
        (variant.morale("p0"), variant.morale("p1")),
        (Some(Morale::Normal), Some(Morale::Low))
    );
    let reference = fight(modern(&[]), Some(1000), army());
    assert_eq!(
        (reference.morale("p0"), reference.morale("p1")),
        (Some(Morale::Low), Some(Morale::Normal))
    );

    // The Sprites' blow: 2 x 20 = 40, times 4, over ten: 16, whole under the variant, halved under the reference.
    let blow = |mut battle: Battle| {
        damage_events(&apply_forced(&mut battle, melee("e0"), &[2, 1]).unwrap())[0].2
    };
    assert_eq!((blow(variant), blow(reference)), (16, 8));

    // Alone a stack is High; two stacks that get on are High; no hero, no morale.
    let alone = fight(
        modern(&["morale"]),
        Some(1000),
        vec![
            stack("p0", "sprites", 20, 1, 2),
            stack("e0", "wolves", 5, 5, 4),
        ],
    );
    assert_eq!(alone.morale("p0"), Some(Morale::High));
    let pack = vec![
        stack("p0", "wolves", 5, 0, 0),
        stack("p1", "wolves", 5, 0, 1),
        stack("e0", "wolves", 5, 5, 4),
    ];
    assert_eq!(
        fight(modern(&["morale"]), Some(1000), pack).morale("p0"),
        Some(Morale::High)
    );
    assert_eq!(fight(modern(&["morale"]), None, army()).morale("p1"), None);
}

// B2. A stack too large for the leadership does not turn on its side; the leadership caps how many fight.
#[test]
fn with_the_cap_a_stack_over_the_leadership_fights_with_what_the_leadership_covers() {
    let army = || {
        vec![
            stack("p0", "wolves", 11, 1, 2),
            stack("p1", "archers", 2, 1, 3),
            stack("e0", "wolves", 20, 2, 2),
        ]
    };
    let mut b = fight(modern(&["control"]), Some(30), army());
    assert!(!b.stack("p0").unwrap().out_of_control, "no mutiny");
    assert_eq!(
        b.legal_actions()[0],
        melee("e0"),
        "its own side is no target"
    );
    // Eleven Wolves are 33 health; 30 leadership covers ten. 2 x 10 = 20, times 5, over ten: 10.
    // The enemy is not capped: all 20 answer. 1 x 20 = 20, times 5, over ten: 10, three Wolves and one health.
    let events = apply_forced(&mut b, melee("e0"), &[2, 1]).unwrap();
    assert_eq!(
        damage_events(&events),
        [(2, 10, 10, 3, 17, 1, false), (1, 20, 10, 3, 8, 1, true)]
    );

    // Its answer is capped too: struck by the enemy, the eleven answer as ten.
    let mut struck = fight(modern(&["control"]), Some(30), army());
    act(&mut struck, Action::Pass).unwrap();
    act(&mut struck, Action::Pass).unwrap();
    let events = apply_forced(&mut struck, melee("p0"), &[1, 3]).unwrap();
    assert_eq!(damage_events(&events)[1].1, 10);

    // Once it is within the leadership every creature fights: the Archers pass, the enemy strikes the eight Wolves
    // that are left, and all eight answer.
    act(&mut b, Action::Pass).unwrap();
    let events = apply_forced(&mut b, melee("p0"), &[1, 1]).unwrap();
    assert_eq!(damage_events(&events)[1].1, 8);

    // Without a hero nothing is capped.
    let mut free = fight(modern(&["control"]), None, army());
    assert_eq!(
        damage_events(&apply_forced(&mut free, melee("e0"), &[2, 1]).unwrap())[0].1,
        11
    );

    // The reference's rule for the same army: the eleven start out of control.
    assert!(
        fight(modern(&[]), Some(30), army())
            .stack("p0")
            .unwrap()
            .out_of_control
    );
}
