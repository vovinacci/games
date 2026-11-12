//! The routes the demo expedition's fixture carries (`content/demo/expedition.json`), played as written, with the
//! pilot fighting where the fixture says `resolve_battle`. Each must end with the numbers the fixture expects.
//! Those numbers were worked out by its author by hand and had never been run before this test.

use std::sync::Arc;

use bounty_ai::approach_v1;
use bounty_core::campaign::{Action, Command, Expedition, ExpeditionDef, Map, Status};
use bounty_core::content::Rules;
use serde_json::Value;

fn def() -> ExpeditionDef {
    let rules = Rules::from_json(include_str!("../../../prototype/rules.json")).unwrap();
    let map = Map::from_json(include_str!("../../../content/demo/map.json")).unwrap();
    ExpeditionDef::from_json(
        include_str!("../../../content/demo/expedition.json"),
        map,
        Arc::new(rules),
    )
    .unwrap()
}

fn act(expedition: &mut Expedition, action: Action) {
    let said = format!("{action:?}");
    let command = Command {
        revision: expedition.revision(),
        action,
    };
    expedition
        .apply(&command)
        .unwrap_or_else(|refused| panic!("{said} refused: {refused}"));
}

/// Plays the battle being fought to its end, the named pilot giving the commands of both sides. Never a result
/// handed in from outside: the fixture forbids that, and the expedition has no command for it.
fn resolve_battle(expedition: &mut Expedition, pilot: &str) {
    assert_eq!(pilot, bounty_ai::APPROACH_V1);
    while expedition.status() == Status::Battle {
        let seen = expedition.battle().unwrap().observe();
        act(expedition, Action::Battle(approach_v1(&seen).unwrap()));
    }
}

fn number(value: &Value) -> i64 {
    value
        .as_i64()
        .unwrap_or_else(|| panic!("{value} is a number"))
}

fn play(route: &Value) -> Expedition {
    let mut expedition = Expedition::start(Arc::new(def()));
    for command in route["commands"].as_array().unwrap() {
        let text = |field: &str| command[field].as_str().unwrap().to_owned();
        match command["kind"].as_str().unwrap() {
            "take_contract" => act(&mut expedition, Action::TakeContract { id: text("id") }),
            "recruit" => {
                let quantity = number(&command["quantity"]) as u32;
                act(
                    &mut expedition,
                    Action::Recruit {
                        troop_id: text("troop_id"),
                        quantity,
                    },
                );
            }
            "move" => {
                let (dx, dy) = (number(&command["dx"]) as i32, number(&command["dy"]) as i32);
                act(&mut expedition, Action::Move { dx, dy });
            }
            "wait_tick" => act(&mut expedition, Action::WaitTick),
            "search" => act(&mut expedition, Action::Search),
            "surrender" => act(&mut expedition, Action::Surrender),
            "resolve_battle" => resolve_battle(&mut expedition, &text("pilot")),
            other => panic!("the route has a command this test does not know: {other}"),
        }
    }
    expedition
}

#[test]
fn every_route_of_the_fixture_ends_as_the_fixture_says() {
    let def = def();
    assert_eq!(def.test_routes.len(), 4);
    for route in &def.test_routes {
        let id = route["id"].as_str().unwrap();
        let end = play(route);
        let wardens: u32 = end
            .army()
            .iter()
            .filter(|stack| stack.troop_id == "warden")
            .map(|stack| stack.count)
            .sum();
        let mut checked = 0;
        for (field, expected) in route["expected"].as_object().unwrap() {
            checked += 1;
            match field.as_str() {
                "status" => assert_eq!(
                    end.status().name(),
                    expected.as_str().unwrap(),
                    "{id}: status"
                ),
                "elapsed_ticks" => {
                    assert_eq!(i64::from(end.ticks()), number(expected), "{id}: ticks");
                }
                "gold" => assert_eq!(end.gold(), expected.as_u64().unwrap(), "{id}: gold"),
                "warden_count" => assert_eq!(i64::from(wardens), number(expected), "{id}: Wardens"),
                "x" => assert_eq!(i64::from(end.hero().x), number(expected), "{id}: x"),
                "y" => assert_eq!(i64::from(end.hero().y), number(expected), "{id}: y"),
                "guard_cleared" => assert_eq!(
                    end.is_cleared("demo-guard"),
                    expected.as_bool().unwrap(),
                    "{id}"
                ),
                "clues" => {
                    // The one clue there is names the place of the objective.
                    assert_eq!(expected, &serde_json::json!(["demo-objective"]), "{id}");
                    assert_eq!(
                        end.objective_known(),
                        Some(def.objective()),
                        "{id}: the clue"
                    );
                }
                other => panic!("{id} expects {other}, which this test does not check"),
            }
        }
        assert!(checked >= 4, "{id}");
    }
}

#[test]
fn the_same_route_gives_the_same_expedition_again() {
    let def = def();
    let route = def
        .test_routes
        .iter()
        .find(|route| route["id"] == "complete-route")
        .unwrap();
    let (first, second) = (play(route), play(route));
    assert_eq!(first.digest(), second.digest());
    assert_eq!(
        (first.status(), first.ticks(), first.gold()),
        (Status::Victory, 15, 150)
    );
}
