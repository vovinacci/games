//! An expedition with a king: the hero has a class, catches villains, and is given ranks in the king's castle
//! (`docs/spec/07-RULES-REGISTER.md`, U-02). The meadow serves, with a castle beside its town and a Knight.

use super::*;
use crate::canon::Value;

const CASTLE: Cell = Cell { x: 2, y: 1 };

/// The meadow's files with a king in them: a castle on the map, a Knight for a hero, and the Knight's commission
/// for the weekly income.
fn texts() -> (String, String) {
    let object = r#"    {
      "id": "meadow-objective","#;
    let castle = r#"    {
      "id": "meadow-castle",
      "kind": "castle",
      "x": 2,
      "y": 1
    },
"#;
    assert!(MEADOW_MAP.contains(object));
    let map = MEADOW_MAP.replacen(object, &format!("{castle}{object}"), 1);
    let king = r#"  "king": {
    "castle_object_id": "meadow-castle",
    "hero_class": "knight"
  },
  "recovery": {"#;
    let expedition = MEADOW.replacen(r#"  "recovery": {"#, king, 1).replacen(
        r#""weekly_income": 500"#,
        r#""weekly_income": 1000"#,
        1,
    );
    assert!(expedition.contains("hero_class") && expedition.contains("1000"));
    (expedition, map)
}

fn with_rules(rules: &str) -> Expedition {
    let (expedition, map) = texts();
    Expedition::start(Arc::new(
        ExpeditionDef::from_texts(&expedition, &map, rules).unwrap(),
    ))
}

fn knight() -> Expedition {
    with_rules(REFERENCE)
}

/// The hero in the castle, as if he had caught this many villains: so many bands beaten under contract.
fn at_court(mut e: Expedition, villains: u32) -> Expedition {
    let bands = ["meadow-pack", "meadow-company", "meadow-swarm"];
    e.cleared = bands[..villains as usize]
        .iter()
        .map(|b| (*b).to_owned())
        .collect();
    e.standing.as_mut().unwrap().villains = villains;
    go(&mut e, 1, 0, 1);
    assert_eq!(e.hero(), CASTLE);
    e
}

#[test]
fn a_king_gives_the_hero_a_class_and_its_first_rank() {
    let e = knight();
    let king = e.def().king.as_ref().unwrap();
    assert_eq!(king.castle, CASTLE);
    assert_eq!(e.def().rules.classes[king.class].id, "knight");
    assert_eq!(
        e.standing(),
        Some(&Standing {
            rank: 0,
            leadership: 100,
            commission: 1000,
            villains: 0
        })
    );
    assert_eq!(e.rank().unwrap().id, "knight");
    assert_eq!((e.leadership(), e.weekly_income()), (100, 1000));

    // Without a king nothing of it: the meadow as it is, and the fixture.
    let plain = Expedition::start(Arc::new(
        ExpeditionDef::from_texts(MEADOW, MEADOW_MAP, REFERENCE).unwrap(),
    ));
    assert_eq!((plain.standing(), plain.rank()), (None, None));
    assert_eq!((plain.leadership(), plain.weekly_income()), (100, 500));
    assert_eq!(start().standing(), None);
}

#[test]
fn a_king_who_cannot_be_is_refused() {
    let (expedition, map) = texts();
    let load = |expedition: &str, map: &str, rules: &str| {
        ExpeditionDef::from_texts(expedition, map, rules).map(|_| ())
    };
    assert_eq!(load(&expedition, &map, REFERENCE), Ok(()));
    let refused = |old: &str, new: &str, why: &str| {
        assert!(expedition.contains(old), "{old}");
        let error = load(&expedition.replacen(old, new, 1), &map, REFERENCE).unwrap_err();
        assert!(error.to_string().contains(why), "{error}");
    };
    refused(
        r#""hero_class": "knight""#,
        r#""hero_class": "druid""#,
        "class",
    );
    refused(
        r#""castle_object_id": "meadow-castle""#,
        r#""castle_object_id": "meadow-town""#,
        "castle",
    );
    // What the expedition starts with is what the class's first rank gives.
    refused(
        r#""weekly_income": 1000"#,
        r#""weekly_income": 500"#,
        "sets out",
    );
    refused(r#""leadership": 100"#, r#""leadership": 120"#, "sets out");
    // The prototype has no classes, so no king either.
    let (demo, demo_map) = (
        EXPEDITION.replacen(
            r#"  "recovery": {"#,
            r#"  "king": {"castle_object_id": "x", "hero_class": "knight"},
  "recovery": {"#,
            1,
        ),
        MAP,
    );
    assert!(demo.contains("hero_class"));
    assert!(load(&demo, demo_map, RULES).is_err());
}

#[test]
fn the_king_gives_a_rank_only_in_his_castle_and_only_when_it_is_earned() {
    let mut e = knight();
    refused(&mut e, Action::Promote, &Refused::NotAtCastle);
    let mut e = at_court(knight(), 0);
    let none = Refused::NoPromotion {
        caught: 0,
        needed: Some(2),
    };
    refused(&mut e, Action::Promote, &none);
    let mut e = at_court(knight(), 1);
    let one = Refused::NoPromotion {
        caught: 1,
        needed: Some(2),
    };
    refused(&mut e, Action::Promote, &one);
    // An expedition without a king has no castle to ask in.
    refused(&mut start(), Action::Promote, &Refused::NotAtCastle);
}

// A General: the Knight's 100 leadership and 1000 a week, and the General's 100 and 1000 on top.
#[test]
fn a_rank_adds_its_leadership_and_commission_once() {
    let mut e = at_court(knight(), 2);
    let (ticks, gold) = (e.ticks(), e.gold());
    let events = act(&mut e, Action::Promote).unwrap();
    assert_eq!(
        events,
        [Event::Promoted {
            rank: "general".to_owned(),
            ranks: 1,
            leadership: 200,
            commission: 2000
        }]
    );
    assert_eq!(e.rank().unwrap().id, "general");
    assert_eq!((e.leadership(), e.weekly_income()), (200, 2000));
    assert_eq!(
        (e.ticks(), e.gold()),
        (ticks, gold),
        "an audience costs nothing"
    );
    let again = Refused::NoPromotion {
        caught: 2,
        needed: Some(8),
    };
    refused(&mut e, Action::Promote, &again);

    // The leadership is the hero's from then on: thirty-four Wolves of three health, two over what a Knight
    // holds, are now within it.
    go(&mut e, -1, 0, 1);
    let wolves = Action::Recruit {
        troop_id: "wolves".to_owned(),
        quantity: 34,
    };
    let over = Refused::OverLeadership {
        health: 102,
        leadership: 100,
    };
    refused(&mut knight(), wolves.clone(), &over);
    act(&mut e, wolves).unwrap();
    // And the commission is the week's income.
    let week = e.def().clock.week_ticks;
    let mut settled = Vec::new();
    while e.ticks() < week {
        settled.extend(act(&mut e, Action::WaitTick).unwrap());
    }
    let income = settled.iter().find_map(|event| match event {
        Event::WeekSettled { income, .. } => Some(*income),
        _ => None,
    });
    assert_eq!(income, Some(2000));
}

// With the Marshal's eight villains made three, three villains earn two ranks at one audience; and with the
// Lord's fourteen made three as well, all three, after which there is no rank left to give.
#[test]
fn enough_villains_earn_several_ranks_at_once() {
    let marshal_at_three =
        REFERENCE.replacen(r#""villains_needed": 8"#, r#""villains_needed": 3"#, 1);
    assert_ne!(marshal_at_three, REFERENCE);
    let mut e = at_court(with_rules(&marshal_at_three), 3);
    let events = act(&mut e, Action::Promote).unwrap();
    assert_eq!(
        events,
        [Event::Promoted {
            rank: "marshal".to_owned(),
            ranks: 2,
            leadership: 500,
            commission: 4000
        }]
    );
    let top = Refused::NoPromotion {
        caught: 3,
        needed: Some(14),
    };
    refused(&mut e, Action::Promote, &top);
}

/// Fights the battle to its end with both sides walking up and striking: a blow where one can be struck, else the
/// step that leaves the least distance to an enemy.
fn close_and_fight(expedition: &mut Expedition) -> Vec<Event> {
    let mut events = Vec::new();
    while let Some(battle) = expedition.battle() {
        let me = battle.stack(&battle.active().unwrap().stack).unwrap();
        let far = |cell: Cell| {
            let others = battle.stacks().iter();
            let enemies = others.filter(|s| s.alive() && s.side != me.side);
            let reach =
                |s: &battle::Stack| cell.x.abs_diff(s.cell.x).max(cell.y.abs_diff(s.cell.y));
            enemies.map(reach).min().unwrap_or(0)
        };
        let legal = battle.legal_actions();
        let strike = legal
            .iter()
            .find(|a| matches!(a, battle::Action::Melee { .. }));
        let steps = legal.iter().filter_map(|a| match a {
            battle::Action::Walk { to } => Some((far(*to), a)),
            _ => None,
        });
        let step = steps.min_by_key(|(left, _)| *left).map(|(_, a)| a);
        let action = strike.or(step).cloned().unwrap_or(battle::Action::Pass);
        let order = Action::Battle(battle::Command {
            revision: battle.revision(),
            actor: me.id.clone(),
            action,
        });
        events.extend(act(expedition, order).unwrap());
    }
    events
}

// A band beaten while its contract is held is a villain caught; beaten without it, it is only beaten.
#[test]
fn a_band_beaten_under_contract_is_a_villain_caught() {
    let caught_after = |contract: bool| {
        let (expedition, map) = texts();
        // Thirty-three Wolves, all the Knight's leadership holds, against the twelve of the pack.
        let strong = expedition
            .replacen(r#""troop_id": "sprites""#, r#""troop_id": "wolves""#, 1)
            .replacen(r#""count": 20,"#, r#""count": 33,"#, 1);
        assert!(strong.contains(r#""count": 33,"#) && strong.contains(r#""troop_id": "wolves""#));
        let def = ExpeditionDef::from_texts(&strong, &map, REFERENCE).unwrap();
        let mut e = Expedition::start(Arc::new(def));
        if contract {
            let id = "meadow-pack".to_owned();
            act(&mut e, Action::TakeContract { id }).unwrap();
        }
        go(&mut e, 1, 0, 2);
        go(&mut e, 0, 1, 2);
        let events = close_and_fight(&mut e);
        assert!(e.is_cleared("meadow-pack"), "the pack is beaten");
        let told = events
            .iter()
            .filter(|event| matches!(event, Event::VillainCaught { caught: 1, .. }))
            .count();
        (e.standing().unwrap().villains, told)
    };
    assert_eq!(caught_after(true), (1, 1));
    assert_eq!(caught_after(false), (0, 0));
}

// The standing is part of the state, for an expedition with a king and for no other.
#[test]
fn the_standing_is_saved_and_one_that_cannot_be_is_refused() {
    let mut e = at_court(knight(), 2);
    act(&mut e, Action::Promote).unwrap();
    let def = Arc::clone(e.def_shared());
    let saved = e.to_canon();
    assert_eq!(
        Expedition::from_canon(Arc::clone(&def), &saved, None).unwrap(),
        e
    );

    let text = String::from_utf8(saved.to_bytes().unwrap()).unwrap();
    let changed = |old: &str, new: &str| {
        assert!(text.contains(old), "{old} in {text}");
        let value = Value::parse(&text.replacen(old, new, 1)).unwrap();
        Expedition::from_canon(Arc::clone(&def), &value, None).map(|_| ())
    };
    assert!(changed(r#""leadership":200"#, r#""leadership":300"#).is_err());
    assert!(changed(r#""commission":2000"#, r#""commission":9000"#).is_err());
    assert!(
        changed(r#""rank":1"#, r#""rank":2"#).is_err(),
        "a rank not earned"
    );
    assert!(
        changed(r#""rank":1"#, r#""rank":9"#).is_err(),
        "a rank that is not"
    );
    assert!(
        changed(r#""villains":2"#, r#""villains":3"#).is_err(),
        "more than the bands beaten"
    );

    let plain = String::from_utf8(start().to_canon().to_bytes().unwrap()).unwrap();
    assert!(
        !plain.contains("standing"),
        "an expedition without a king is written as before"
    );
}
