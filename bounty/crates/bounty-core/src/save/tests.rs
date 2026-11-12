//! A save against the demo expedition: what was saved comes back, and anything else is refused with its reason.

use std::collections::BTreeMap;

use super::*;
use crate::battle;
use crate::campaign::{Action, Command, Status};
use crate::content::Cell;

const MAP: &str = include_str!("../../../../content/demo/map.json");
const EXPEDITION: &str = include_str!("../../../../content/demo/expedition.json");
const RULES: &str = include_str!("../../../../prototype/rules.json");

fn def() -> Arc<ExpeditionDef> {
    Arc::new(ExpeditionDef::from_texts(EXPEDITION, MAP, RULES).unwrap())
}

fn act(expedition: &mut Expedition, action: Action) {
    let command = Command {
        revision: expedition.revision(),
        action,
    };
    expedition.apply(&command).unwrap();
}

/// The battle's active stack strikes if it can and passes if not.
fn blow(expedition: &mut Expedition) {
    let battle = expedition.battle().unwrap();
    let strike = battle
        .legal_actions()
        .into_iter()
        .find(|a| matches!(a, battle::Action::Melee { .. }));
    let order = battle::Command {
        revision: battle.revision(),
        actor: battle.active().unwrap().stack.clone(),
        action: strike.unwrap_or(battle::Action::Pass),
    };
    act(expedition, Action::Battle(order));
}

/// The demo with twelve Wardens, at the start of the battle with the first band.
fn in_battle() -> Expedition {
    let mut e = Expedition::start(def());
    act(
        &mut e,
        Action::Recruit {
            troop_id: "warden".to_owned(),
            quantity: 2,
        },
    );
    for _ in 0..3 {
        act(&mut e, Action::Move { dx: 1, dy: 0 });
    }
    assert_eq!(e.status(), Status::Battle);
    e
}

type Members = BTreeMap<String, Value>;

fn members(value: &mut Value) -> &mut Members {
    let Value::Object(members) = value else {
        panic!("{value:?} is an object")
    };
    members
}

/// The save with its payload changed and the checksum made right again: what a careful forger would write.
fn resealed(text: &str, change: impl FnOnce(&mut Members)) -> String {
    let mut envelope = Value::parse(text).unwrap();
    let outer = members(&mut envelope);
    change(members(outer.get_mut("payload").unwrap()));
    let sha256 = outer["payload"].digest().unwrap();
    outer.insert("sha256".to_owned(), Value::Str(sha256));
    String::from_utf8(envelope.to_bytes().unwrap()).unwrap()
}

fn invalid_with(text: &str, change: impl FnOnce(&mut Members)) -> bool {
    matches!(
        decode(&resealed(text, change), def()),
        Err(SaveError::Invalid(_))
    )
}

#[test]
fn a_save_gives_back_the_expedition_it_was_made_of() {
    for expedition in [Expedition::start(def()), in_battle()] {
        let text = encode(&expedition);
        let back = decode(&text, def()).unwrap();
        assert_eq!(back, expedition);
        assert_eq!(back.digest(), expedition.digest());
        assert_eq!(encode(&back), text, "the same state gives the same bytes");
        assert!(text.ends_with('\n') && text.is_ascii());
    }
}

#[test]
fn the_envelope_is_the_one_of_the_schema() {
    let mut envelope = Value::parse(&encode(&in_battle())).unwrap();
    let outer = members(&mut envelope);
    let keys = |m: &Members| m.keys().cloned().collect::<Vec<_>>().join(" ");
    assert_eq!(
        keys(outer),
        "canonical_format payload save_schema_version sha256"
    );
    let payload = members(outer.get_mut("payload").unwrap());
    assert_eq!(
        keys(payload),
        "battle campaign content_sha256 policy_state rng ruleset_id ruleset_version"
    );
    assert_eq!(payload["ruleset_id"], Value::str("prototype-v0.1"));
    assert_eq!(payload["ruleset_version"], Value::str("0.1"));
    assert!(matches!(payload["battle"], Value::Object(_)));
    assert!(!members(payload.get_mut("campaign").unwrap()).contains_key("battle"));
    // Out of battle the place of the battle is there and empty.
    let mut envelope = Value::parse(&encode(&Expedition::start(def()))).unwrap();
    let payload = members(members(&mut envelope).get_mut("payload").unwrap());
    assert_eq!(payload["battle"], Value::Null);
}

// AT-050: after the reload the battle offers the same actions, in the same order, and the same blows fall.
#[test]
fn a_battle_goes_on_after_the_reload_as_it_would_have() {
    let mut played = in_battle();
    let mut loaded = decode(&encode(&played), def()).unwrap();
    assert_eq!(
        loaded.battle().unwrap().legal_actions(),
        played.battle().unwrap().legal_actions()
    );
    while played.status() == Status::Battle {
        blow(&mut played);
        blow(&mut loaded);
        assert_eq!(loaded, played);
    }
    assert_eq!(loaded.gold(), played.gold());
}

#[test]
fn a_text_that_is_no_save_is_malformed() {
    let text = encode(&in_battle());
    let malformed = |text: &str| matches!(decode(text, def()), Err(SaveError::Malformed(_)));
    assert!(malformed(""));
    assert!(malformed("{}"));
    assert!(malformed("[1]"));
    assert!(malformed(&text[..text.len() / 2]), "cut short");
    assert!(malformed(&text.replacen(
        "bounty-cjson-v1",
        "bounty-cjson-v2",
        1
    )));
    assert!(malformed(&text.replacen(
        r#""sha256":""#,
        r#""sha256":1,"x":""#,
        1
    )));
}

#[test]
fn another_version_is_named_and_not_read() {
    let text = encode(&in_battle());
    let other = text.replacen(
        r#""save_schema_version":1"#,
        r#""save_schema_version":2"#,
        1,
    );
    assert_ne!(other, text);
    assert_eq!(decode(&other, def()), Err(SaveError::UnsupportedVersion(2)));
}

#[test]
fn a_changed_byte_is_damage() {
    let expedition = in_battle();
    let text = encode(&expedition);
    let gold = format!(r#""gold":{}"#, expedition.gold());
    let richer = text.replacen(&gold, r#""gold":999999"#, 1);
    assert_ne!(richer, text);
    assert_eq!(decode(&richer, def()), Err(SaveError::Damaged));
    let checksum = text.rfind(r#""sha256":""#).unwrap() + 10;
    let mut other = text.clone();
    let digit = if &text[checksum..=checksum] == "0" {
        "1"
    } else {
        "0"
    };
    other.replace_range(checksum..=checksum, digit);
    assert_eq!(decode(&other, def()), Err(SaveError::Damaged));
}

#[test]
fn a_save_of_other_content_is_refused() {
    let text = encode(&in_battle());
    let other = |r: Result<Expedition, SaveError>| matches!(r, Err(SaveError::OtherContent(_)));

    // Any of the three files changed, even by a space that changes no rule.
    let spaced =
        Arc::new(ExpeditionDef::from_texts(EXPEDITION, &format!("{MAP} "), RULES).unwrap());
    assert!(other(decode(&text, spaced)));
    assert!(other(decode(
        &resealed(&text, |p| drop(
            p.insert("ruleset_id".to_owned(), Value::str("reference-v0.1"))
        )),
        def()
    )));
    assert!(other(decode(
        &resealed(&text, |p| drop(
            p.insert("ruleset_version".to_owned(), Value::str("0.2"))
        )),
        def()
    )));
    // The texts are told apart by their lengths too: moving a byte from one to the next is another content.
    let (a, b) = (
        ExpeditionDef::from_texts(EXPEDITION, MAP, RULES).unwrap(),
        ExpeditionDef::from_texts(EXPEDITION, MAP, &format!("{RULES}\n")).unwrap(),
    );
    assert_ne!(a.content_sha256, b.content_sha256);
}

#[test]
fn a_state_the_rules_cannot_reach_is_refused_though_its_checksum_is_right() {
    let text = encode(&in_battle());
    let quiet = encode(&Expedition::start(def()));
    assert!(
        decode(&resealed(&text, |_| ()), def()).is_ok(),
        "resealing alone changes nothing"
    );
    let campaign = |p: &mut Members, key: &str, value: Value| {
        members(p.get_mut("campaign").unwrap()).insert(key.to_owned(), value);
    };
    let cell = |x: u32, y: u32| Value::object([("x", x.into()), ("y", y.into())]);
    assert!(!def().map.passable(Cell { x: 0, y: 0 }));

    assert!(
        invalid_with(&quiet, |p| campaign(p, "hero", cell(0, 0))),
        "hero in the water"
    );
    assert!(
        invalid_with(&quiet, |p| campaign(p, "hero", cell(900, 0))),
        "hero off the map"
    );
    assert!(invalid_with(&quiet, |p| campaign(
        p,
        "ticks",
        1_000_000_u32.into()
    )));
    assert!(invalid_with(&quiet, |p| campaign(
        p,
        "status",
        Value::str("battle")
    )));
    assert!(invalid_with(&quiet, |p| campaign(
        p,
        "status",
        Value::str("won")
    )));
    assert!(invalid_with(&quiet, |p| campaign(
        p,
        "expedition_id",
        Value::str("other")
    )));
    assert!(invalid_with(&quiet, |p| campaign(
        p,
        "contract",
        Value::str("nobody")
    )));
    assert!(invalid_with(&quiet, |p| campaign(
        p,
        "cleared",
        Value::Array(vec![Value::str("x")])
    )));
    assert!(invalid_with(&quiet, |p| campaign(
        p,
        "clues",
        Value::Array(vec![Value::str("x")])
    )));
    assert!(invalid_with(&quiet, |p| campaign(
        p,
        "stock",
        Value::object([])
    )));
    assert!(invalid_with(&quiet, |p| campaign(
        p,
        "army",
        Value::Array(vec![Value::object([
            ("count", 0_u32.into()),
            ("id", Value::str("p0")),
            ("troop_id", Value::str("warden")),
        ])])
    )));
    assert!(invalid_with(&quiet, |p| campaign(
        p,
        "gold",
        Value::str("much")
    )));
    assert!(invalid_with(&quiet, |p| drop(
        members(p.get_mut("campaign").unwrap()).remove("gold")
    )));

    assert!(invalid_with(&text, |p| campaign(
        p,
        "status",
        Value::str("adventure")
    )));
    assert!(invalid_with(&text, |p| campaign(
        p,
        "band",
        Value::str("nobody")
    )));
    assert!(invalid_with(&text, |p| campaign(p, "arrival", cell(0, 0))));
    assert!(invalid_with(&text, |p| drop(
        p.insert("battle".to_owned(), Value::Null)
    )));
}

#[test]
fn a_battle_the_rules_cannot_reach_is_refused_though_its_checksum_is_right() {
    let text = encode(&in_battle());

    let battle = |p: &mut Members, change: &dyn Fn(&mut Members)| {
        change(members(p.get_mut("battle").unwrap()));
    };
    let stacks = |b: &mut Members, change: &dyn Fn(&mut Vec<Value>)| {
        let Some(Value::Array(stacks)) = b.get_mut("stacks") else {
            panic!("stacks")
        };
        change(stacks);
    };
    let set = |key: &'static str, value: Value| {
        move |b: &mut Members| drop(b.insert(key.to_owned(), value.clone()))
    };
    assert!(invalid_with(&text, |p| battle(
        p,
        &set("round", 0_u32.into())
    )));
    assert!(invalid_with(&text, |p| battle(
        p,
        &set("round", 10_000_u32.into())
    )));
    assert!(invalid_with(&text, |p| battle(
        p,
        &set("ruleset_id", Value::str("reference-v0.1"))
    )));
    assert!(
        invalid_with(&text, |p| battle(p, &set("active", Value::Null))),
        "nobody's turn"
    );
    assert!(invalid_with(&text, |p| battle(
        p,
        &set("outcome", Value::str("player_win"))
    )));
    assert!(invalid_with(&text, |p| battle(
        p,
        &set("queue", Value::Array(vec![Value::str("x")]))
    )));
    assert!(invalid_with(&text, |p| battle(p, &|b| {
        let algorithm = Value::str("reference-lcg64-v1");
        members(b.get_mut("rng").unwrap()).insert("algorithm".to_owned(), algorithm);
    })));
    assert!(
        invalid_with(&text, |p| battle(p, &|b| stacks(b, &|all| {
            let first = members(&mut all[0])["cell"].clone();
            members(&mut all[1]).insert("cell".to_owned(), first);
        }))),
        "two stacks on a cell"
    );
    assert!(invalid_with(&text, |p| battle(p, &|b| stacks(b, &|all| {
        members(&mut all[0]).insert("troop_id".to_owned(), Value::str("dragon"));
    }))));
    assert!(invalid_with(&text, |p| battle(p, &|b| stacks(b, &|all| {
        members(&mut all[0]).insert("injury".to_owned(), 10_000_u32.into());
    }))));
    assert!(invalid_with(&text, |p| battle(p, &|b| stacks(b, &|all| {
        let twin = all[0].clone();
        all.push(twin);
    }))));
}
