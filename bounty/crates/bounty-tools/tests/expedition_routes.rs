//! The routes the demo expedition's fixture carries (`content/demo/expedition.json`), played as written by the
//! tool that reports on them, with the pilot fighting where the fixture says `resolve_battle`. Each must end with
//! the numbers the fixture expects. Those numbers were worked out by its author by hand and had never been run
//! before this test. Every state on the way is also saved and read back.

use std::cell::Cell;
use std::path::Path;
use std::sync::Arc;

use bounty_core::campaign::{Expedition, ExpeditionDef, Status};
use bounty_core::save;
use bounty_tools::expedition::{load, play_route};
use serde_json::Value;

fn def() -> Arc<ExpeditionDef> {
    load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/demo/expedition.json")).unwrap()
}

thread_local! {
    /// While set, play goes on from the reloaded save after every command.
    static RELOAD: Cell<bool> = const { Cell::new(false) };
}

/// Every state a route passes through, in battle or out of it, comes back from its save as it was.
fn saved_and_read_back(expedition: &mut Expedition) {
    let saved = save::encode(expedition);
    let loaded = save::decode(&saved, Arc::clone(expedition.def_shared()))
        .unwrap_or_else(|error| panic!("the save at revision {}: {error}", expedition.revision()));
    assert_eq!(&loaded, expedition);
    assert_eq!(save::encode(&loaded), saved);
    if RELOAD.with(Cell::get) {
        *expedition = loaded;
    }
}

fn number(value: &Value) -> i64 {
    value
        .as_i64()
        .unwrap_or_else(|| panic!("{value} is a number"))
}

fn play(route: &Value) -> Expedition {
    let (end, row) = play_route(&def(), route, &mut saved_and_read_back).unwrap();
    assert_eq!((row.gold_end, row.elapsed_ticks), (end.gold(), end.ticks()));
    end
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

// AT-098: closing the game after any command, in a battle too, and going on from the save ends where the route
// ends without it.
#[test]
fn a_route_played_from_its_saves_ends_as_the_route_played_through() {
    let def = def();
    for route in &def.test_routes {
        let through = play(route);
        RELOAD.with(|reload| reload.set(true));
        let reloaded = play(route);
        RELOAD.with(|reload| reload.set(false));
        assert_eq!(reloaded, through, "{}", route["id"]);
        // AT-044: the ended expedition loads as ended, and takes nothing more.
        let again = save::decode(&save::encode(&through), Arc::clone(&def)).unwrap();
        assert_eq!(
            (again.status(), again.gold()),
            (through.status(), through.gold())
        );
    }
}
