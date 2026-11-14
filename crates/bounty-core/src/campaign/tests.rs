//! The expedition against its fixture, `content/demo/`: the routes the fixture carries, with the numbers it
//! expects, and the rules its README states, each with its edge.

use super::*;
use crate::battle;
use crate::content::{ContentError, Rules};

const MAP: &str = include_str!("../../../../content/demo/map.json");
const EXPEDITION: &str = include_str!("../../../../content/demo/expedition.json");
const RULES: &str = include_str!("../../../../prototype/rules.json");

fn def() -> ExpeditionDef {
    ExpeditionDef::from_texts(EXPEDITION, MAP, RULES).unwrap()
}

fn start() -> Expedition {
    Expedition::start(Arc::new(def()))
}

/// The demo with something about it changed.
fn start_with(change: impl FnOnce(&mut ExpeditionDef)) -> Expedition {
    let mut def = def();
    change(&mut def);
    Expedition::start(Arc::new(def))
}

fn act(expedition: &mut Expedition, action: Action) -> Result<Vec<Event>, Refused> {
    let command = Command {
        revision: expedition.revision(),
        action,
    };
    expedition.apply(&command)
}

/// The action is refused with `why` and nothing changes.
#[expect(
    clippy::needless_pass_by_value,
    reason = "a test reads better with the action built in place"
)]
fn refused(expedition: &mut Expedition, action: Action, why: &Refused) {
    let before = (expedition.clone(), expedition.digest());
    assert_eq!(
        act(expedition, action.clone()).as_ref(),
        Err(why),
        "{action:?}"
    );
    assert_eq!(
        (expedition.clone(), expedition.digest()),
        before,
        "refusing {action:?} changed nothing"
    );
}

fn go(expedition: &mut Expedition, dx: i32, dy: i32, steps: u32) {
    for _ in 0..steps {
        act(expedition, Action::Move { dx, dy }).unwrap();
    }
}

fn wait(expedition: &mut Expedition, ticks: u32) {
    for _ in 0..ticks {
        act(expedition, Action::WaitTick).unwrap();
    }
}

fn recruit(quantity: u32) -> Action {
    Action::Recruit {
        troop_id: "warden".to_owned(),
        quantity,
    }
}

fn wardens(expedition: &Expedition) -> u32 {
    expedition
        .army()
        .iter()
        .filter(|s| s.troop_id == "warden")
        .map(|s| s.count)
        .sum()
}

/// The fixture's route with this ID: its commands as actions, and what it expects.
fn route(id: &str) -> (Vec<Action>, serde_json::Value) {
    let def = def();
    let route = def
        .test_routes
        .iter()
        .find(|route| route["id"] == id)
        .unwrap_or_else(|| panic!("route {id}"));
    let number = |value: &serde_json::Value| value.as_i64().unwrap();
    let actions =
        route["commands"].as_array().unwrap().iter().map(|command| {
            match command["kind"].as_str().unwrap() {
                "wait_tick" => Action::WaitTick,
                "search" => Action::Search,
                "move" => Action::Move {
                    dx: number(&command["dx"]) as i32,
                    dy: number(&command["dy"]) as i32,
                },
                "take_contract" => Action::TakeContract {
                    id: command["id"].as_str().unwrap().to_owned(),
                },
                "recruit" => Action::Recruit {
                    troop_id: command["troop_id"].as_str().unwrap().to_owned(),
                    quantity: number(&command["quantity"]) as u32,
                },
                other => panic!("the route has a command this test cannot give: {other}"),
            }
        });
    (actions.collect(), route["expected"].clone())
}

#[test]
fn the_fixture_loads() {
    let def = def();
    assert_eq!(
        (def.map.width, def.map.height, def.start, def.town),
        (12, 8, Cell { x: 1, y: 1 }, Cell { x: 1, y: 1 })
    );
    assert_eq!(
        (
            def.start_gold,
            def.leadership,
            def.clock.week_ticks,
            def.clock.deadline_ticks
        ),
        (100, 120, 20, 24)
    );
    assert_eq!(def.objective(), Cell { x: 8, y: 5 });
    assert_eq!(def.encounter("demo-captain").unwrap().gold_reward, 60);
    assert_eq!(
        (
            def.recovery.count,
            def.recovery.ticks,
            def.recovery.destination
        ),
        (4, 4, def.town)
    );
    assert_eq!(def.test_routes.len(), 4);
    assert!(def.map.passable(Cell { x: 1, y: 1 }) && !def.map.passable(Cell { x: 0, y: 0 }));
    assert!(!def.map.passable(Cell { x: 12, y: 1 }), "off the map");

    let e = start();
    assert_eq!(
        (e.hero(), e.ticks(), e.gold(), wardens(&e), e.status()),
        (def.start, 0, 100, 10, Status::Adventure)
    );
    assert_eq!(
        (e.stock("warden"), e.contract(), e.objective_known()),
        (10, None, None)
    );
}

#[test]
fn broken_content_is_refused() {
    let map = |old: &str, new: &str| {
        assert!(MAP.contains(old));
        Map::from_json(&MAP.replacen(old, new, 1))
    };
    let invalid = |r: Result<Map, ContentError>| matches!(r, Err(ContentError::Invalid(_)));
    assert!(
        invalid(map(r#""~..........~","#, r#""~.........~","#)),
        "a short row"
    );
    assert!(
        invalid(map(r#""~..........~","#, r#""~....#.....~","#)),
        "a symbol not in the legend"
    );
    assert!(
        invalid(map(r#""height": 8"#, r#""height": 7"#)),
        "fewer rows than the height says"
    );
    assert!(
        invalid(map(r#""x": 4"#, r#""x": 0"#)),
        "an object in the water"
    );
    assert!(
        invalid(map(r#""id": "demo-guard""#, r#""id": "demo-town""#)),
        "an ID twice"
    );
    assert!(
        invalid(map(r#""width": 12"#, r#""width": 4000"#)),
        "too large"
    );

    let expedition = |old: &str, new: &str| {
        assert!(EXPEDITION.contains(old), "{old}");
        ExpeditionDef::from_texts(&EXPEDITION.replacen(old, new, 1), MAP, RULES)
    };
    let invalid =
        |r: Result<ExpeditionDef, ContentError>| matches!(r, Err(ContentError::Invalid(_)));
    let unsupported =
        |r: Result<ExpeditionDef, ContentError>| matches!(r, Err(ContentError::Unsupported(_)));
    assert!(
        invalid(expedition(
            r#""enemy_troop": "warden""#,
            r#""enemy_troop": "dragon""#
        )),
        "an unknown troop"
    );
    assert!(
        invalid(expedition(
            r#""object_id": "demo-guard""#,
            r#""object_id": "demo-town""#
        )),
        "a band on the town"
    );
    assert!(
        invalid(expedition(
            r#""deadline_ticks": 24"#,
            r#""deadline_ticks": 0"#
        )),
        "no deadline"
    );
    assert!(
        invalid(expedition(r#""enemy_count": 1"#, r#""enemy_count": 0"#)),
        "an empty band"
    );
    let deadline = r#""deadline_ticks": 24"#;
    assert!(
        invalid(expedition(deadline, r#""deadline_ticks": 4000000000"#)),
        "a deadline too far"
    );
    let day = r#""ticks_per_day": 4"#;
    assert!(
        invalid(expedition(day, r#""ticks_per_day": 4000000000"#)),
        "a week that overflows"
    );
    assert!(
        invalid(expedition(r#""search_ticks": 4"#, r#""search_ticks": 0"#)),
        "a search that takes no time"
    );
    assert!(unsupported(expedition(
        r#""search_requires_clue": false"#,
        r#""search_requires_clue": true"#
    )));
    assert!(unsupported(expedition(
        r#""success_on_exact_deadline": "victory""#,
        r#""success_on_exact_deadline": "timeout""#
    )));
    assert!(
        unsupported(expedition(
            r#""map_id": "demo-meadow""#,
            r#""map_id": "other""#
        )),
        "another map"
    );
}

// The fixture's route `timeout-at-town`: twenty-four ticks of waiting.
#[test]
fn waiting_out_the_clock_gives_the_fixtures_numbers() {
    let (actions, expected) = route("timeout-at-town");
    let mut e = start();
    for action in actions {
        act(&mut e, action).unwrap();
    }
    assert_eq!(e.status().name(), expected["status"]);
    assert_eq!(
        u64::from(e.ticks()),
        expected["elapsed_ticks"].as_u64().unwrap()
    );
    assert_eq!(e.gold(), expected["gold"].as_u64().unwrap());
    assert_eq!(
        u64::from(wardens(&e)),
        expected["warden_count"].as_u64().unwrap()
    );
    refused(&mut e, Action::WaitTick, &Refused::Over);
}

// AT-032: a week is settled once, income first and then upkeep, at the tick that ends it.
#[test]
fn each_week_is_settled_once_at_its_last_tick() {
    let mut e = start_with(|def| def.clock.deadline_ticks = 100);
    wait(&mut e, 19);
    assert_eq!(e.gold(), 100, "nothing before the week ends");
    let events = act(&mut e, Action::WaitTick).unwrap();
    // Twenty in, one for each of the ten Wardens out.
    assert_eq!(
        events,
        [
            Event::WeekSettled {
                week: 1,
                income: 20,
                upkeep: 10,
                gold: 110
            },
            Event::TimePassed { from: 19, to: 20 }
        ]
    );
    wait(&mut e, 19);
    assert_eq!(e.gold(), 110);
    wait(&mut e, 1);
    assert_eq!((e.gold(), e.ticks()), (120, 40));
}

#[test]
fn a_week_that_cannot_be_paid_ends_the_expedition() {
    let mut e = start_with(|def| {
        def.start_gold = 3;
        def.clock.weekly_income = 5;
    });
    wait(&mut e, 19);
    // Five in makes eight; ten are owed.
    let events = act(&mut e, Action::WaitTick).unwrap();
    assert_eq!(
        events,
        [
            Event::TimePassed { from: 19, to: 20 },
            Event::Ended {
                status: Status::Insolvent
            }
        ]
    );
    assert_eq!((e.status(), e.gold()), (Status::Insolvent, 8));
    // Exactly enough is enough.
    let mut exact = start_with(|def| {
        def.start_gold = 5;
        def.clock.weekly_income = 5;
    });
    wait(&mut exact, 20);
    assert_eq!((exact.status(), exact.gold()), (Status::Adventure, 0));
}

#[test]
fn steps_go_north_south_east_or_west_over_ground_and_cost_a_tick() {
    let mut e = start();
    refused(&mut e, Action::Move { dx: 1, dy: 1 }, &Refused::NotAStep);
    refused(&mut e, Action::Move { dx: 0, dy: 0 }, &Refused::NotAStep);
    refused(&mut e, Action::Move { dx: 2, dy: 0 }, &Refused::NotAStep);
    refused(
        &mut e,
        Action::Move { dx: -1, dy: 0 },
        &Refused::CannotEnter,
    );
    refused(
        &mut e,
        Action::Move { dx: 0, dy: -1 },
        &Refused::CannotEnter,
    );
    let events = act(&mut e, Action::Move { dx: 0, dy: 1 }).unwrap();
    let (from, to) = (Cell { x: 1, y: 1 }, Cell { x: 1, y: 2 });
    assert_eq!(
        events,
        [
            Event::TimePassed { from: 0, to: 1 },
            Event::Moved { from, to }
        ]
    );
    assert_eq!((e.hero(), e.ticks(), e.revision()), (to, 1, 1));
    // A command made for the state before is stale.
    let old = Command {
        revision: 0,
        action: Action::WaitTick,
    };
    assert_eq!(
        e.apply(&old),
        Err(Refused::Stale {
            command: 0,
            expedition: 1
        })
    );
    // The edge of the map, from its far corner.
    go(&mut e, 0, 1, 4);
    refused(&mut e, Action::Move { dx: 0, dy: 1 }, &Refused::CannotEnter);
}

// AT-033, AT-034: buying is all or nothing, and exactly enough is enough.
#[test]
fn recruiting_checks_stock_purse_and_leadership_before_it_changes_anything() {
    let mut e = start();
    refused(&mut e, recruit(0), &Refused::NoQuantity);
    refused(
        &mut e,
        Action::Recruit {
            troop_id: "slinger".to_owned(),
            quantity: 1,
        },
        &Refused::NotSold {
            troop_id: "slinger".to_owned(),
        },
    );
    // Ten Wardens of ten health against a leadership of 120: two more fit exactly, three do not.
    refused(
        &mut e,
        recruit(3),
        &Refused::OverLeadership {
            health: 130,
            leadership: 120,
        },
    );
    let events = act(&mut e, recruit(2)).unwrap();
    assert_eq!(
        events,
        [Event::Recruited {
            troop_id: "warden".to_owned(),
            quantity: 2,
            cost: 40
        }]
    );
    assert_eq!(
        (e.gold(), e.stock("warden"), wardens(&e), e.ticks()),
        (60, 8, 12, 0),
        "and it took no time"
    );
    assert_eq!(e.army().len(), 1, "the same troop joins its stack");

    // Exactly what the purse holds: five at twenty with a hundred.
    let mut rich = start_with(|def| def.leadership = 1000);
    refused(
        &mut rich,
        recruit(6),
        &Refused::CannotAfford {
            cost: 120,
            gold: 100,
        },
    );
    act(&mut rich, recruit(5)).unwrap();
    assert_eq!((rich.gold(), wardens(&rich)), (0, 15));

    // No more than the town has.
    let mut wealthy = start_with(|def| {
        def.leadership = 1000;
        def.start_gold = 1000;
    });
    refused(
        &mut wealthy,
        recruit(11),
        &Refused::OutOfStock { stock: 10 },
    );
    act(&mut wealthy, recruit(10)).unwrap();
    refused(&mut wealthy, recruit(1), &Refused::OutOfStock { stock: 0 });

    // Only in the town.
    go(&mut e, 0, 1, 1);
    refused(&mut e, recruit(1), &Refused::NotInTown);
}

#[test]
fn a_new_troop_takes_a_free_slot_and_a_full_army_takes_none() {
    let sells_slingers = |def: &mut ExpeditionDef| {
        def.stock.insert("slinger".to_owned(), 5);
        def.prices.insert("slinger".to_owned(), 10);
    };
    let mut e = start_with(sells_slingers);
    act(
        &mut e,
        Action::Recruit {
            troop_id: "slinger".to_owned(),
            quantity: 2,
        },
    )
    .unwrap();
    let ids: Vec<(&str, &str, u32)> = e
        .army()
        .iter()
        .map(|s| (s.id.as_str(), s.troop_id.as_str(), s.count))
        .collect();
    assert_eq!(ids, [("p0", "warden", 10), ("p1", "slinger", 2)]);

    let mut full = start_with(|def| {
        sells_slingers(def);
        let stack = |n: u32| ArmyStart {
            id: format!("p{n}"),
            troop_id: "warden".to_owned(),
            count: 1,
            injury: 0,
        };
        def.start_army = (0..5).map(stack).collect();
    });
    refused(
        &mut full,
        Action::Recruit {
            troop_id: "slinger".to_owned(),
            quantity: 1,
        },
        &Refused::ArmyFull,
    );
}

#[test]
fn a_contract_is_taken_in_the_town_on_a_band_that_stands() {
    let mut e = start();
    refused(
        &mut e,
        Action::TakeContract {
            id: "demo-town".to_owned(),
        },
        &Refused::NoSuchContract {
            id: "demo-town".to_owned(),
        },
    );
    let events = act(
        &mut e,
        Action::TakeContract {
            id: "demo-captain".to_owned(),
        },
    )
    .unwrap();
    assert_eq!(
        events,
        [Event::ContractTaken {
            id: "demo-captain".to_owned()
        }]
    );
    assert_eq!((e.contract(), e.ticks()), (Some("demo-captain"), 0));
    go(&mut e, 0, 1, 1);
    refused(
        &mut e,
        Action::TakeContract {
            id: "demo-guard".to_owned(),
        },
        &Refused::NotInTown,
    );
}

// AT-040: the objective is found by searching its tile, with or without a clue.
#[test]
fn a_search_on_the_objective_wins_without_a_clue() {
    let mut e = start();
    go(&mut e, 0, 1, 4);
    go(&mut e, 1, 0, 6);
    // One tile short: a search costs its four ticks and finds nothing.
    let events = act(&mut e, Action::Search).unwrap();
    assert_eq!(
        events,
        [
            Event::TimePassed { from: 10, to: 14 },
            Event::Searched { found: false }
        ]
    );
    go(&mut e, 1, 0, 1);
    assert_eq!(
        (e.hero(), e.objective_known()),
        (Cell { x: 8, y: 5 }, None),
        "standing on it, still not told"
    );
    let events = act(&mut e, Action::Search).unwrap();
    let found = [
        Event::TimePassed { from: 15, to: 19 },
        Event::Searched { found: true },
        Event::Ended {
            status: Status::Victory,
        },
    ];
    assert_eq!(events, found);
    assert_eq!(
        (e.status(), e.ticks(), e.gold()),
        (Status::Victory, 19, 100)
    );
    refused(&mut e, Action::Search, &Refused::Over);
}

// AT-041: a search that ends exactly on the deadline counts; one begun too late does not, and the week that ends
// on the way is settled first.
#[test]
fn the_deadline_lets_a_search_end_on_it_and_cuts_a_later_one_short() {
    let at_the_objective = || {
        let mut e = start();
        go(&mut e, 0, 1, 4);
        go(&mut e, 1, 0, 7);
        e
    };
    let mut on_time = at_the_objective();
    wait(&mut on_time, 9);
    assert_eq!(
        (on_time.ticks(), on_time.gold()),
        (20, 110),
        "the week was settled on the way"
    );
    act(&mut on_time, Action::Search).unwrap();
    assert_eq!((on_time.status(), on_time.ticks()), (Status::Victory, 24));

    let mut late = at_the_objective();
    wait(&mut late, 10);
    let events = act(&mut late, Action::Search).unwrap();
    // Three of its four ticks pass; the search finds nothing because it never ends.
    assert_eq!(
        events,
        [
            Event::TimePassed { from: 21, to: 24 },
            Event::Ended {
                status: Status::Timeout
            }
        ]
    );
    assert_eq!((late.status(), late.ticks()), (Status::Timeout, 24));

    // A step that ends on the deadline is taken, and then time is up.
    let mut walker = start();
    wait(&mut walker, 23);
    let events = act(&mut walker, Action::Move { dx: 0, dy: 1 }).unwrap();
    assert_eq!(
        events.last(),
        Some(&Event::Ended {
            status: Status::Timeout
        })
    );
    assert_eq!((walker.hero(), walker.ticks()), (Cell { x: 1, y: 2 }, 24));
}

/// The command of the battle's active stack that strikes if it can and passes if not.
fn blow(expedition: &Expedition) -> Action {
    let battle = expedition.battle().expect("a battle is being fought");
    let strike = battle
        .legal_actions()
        .into_iter()
        .find(|a| matches!(a, battle::Action::Melee { .. }));
    Action::Battle(battle::Command {
        revision: battle.revision(),
        actor: battle.active().unwrap().stack.clone(),
        action: strike.unwrap_or(battle::Action::Pass),
    })
}

/// Fights the battle to its end, both sides striking whenever they can.
fn fight_out(expedition: &mut Expedition) -> Vec<Event> {
    let mut events = Vec::new();
    while expedition.status() == Status::Battle {
        let strike = blow(expedition);
        events.extend(act(expedition, strike).unwrap());
    }
    events
}

/// The demo with twelve Wardens, standing before the first band.
fn before_the_guard(contract: Option<&str>) -> Expedition {
    let mut e = start();
    if let Some(id) = contract {
        act(&mut e, Action::TakeContract { id: id.to_owned() }).unwrap();
    }
    act(&mut e, recruit(2)).unwrap();
    go(&mut e, 1, 0, 2);
    e
}

#[test]
fn stepping_onto_a_band_costs_the_step_and_opens_its_battle() {
    let mut e = before_the_guard(None);
    let events = act(&mut e, Action::Move { dx: 1, dy: 0 }).unwrap();
    assert_eq!(
        events,
        [
            Event::TimePassed { from: 2, to: 3 },
            Event::BattleStarted {
                band: "demo-guard".to_owned()
            }
        ]
    );
    assert_eq!(
        (e.status(), e.hero(), e.engaged_with()),
        (Status::Battle, Cell { x: 3, y: 1 }, Some("demo-guard"))
    );
    // The army and the band as the expedition has them, where the layout puts them.
    let battle = e.battle().unwrap();
    let placed: Vec<(&str, u32, Cell)> = battle
        .stacks()
        .iter()
        .map(|s| (s.id.as_str(), s.count, s.cell))
        .collect();
    assert_eq!(
        placed,
        [
            ("p0", 12, Cell { x: 1, y: 2 }),
            ("e0", 1, Cell { x: 2, y: 2 })
        ]
    );

    // The map waits while the battle is fought.
    refused(&mut e, Action::WaitTick, &Refused::InBattle);
    refused(&mut e, Action::Move { dx: -1, dy: 0 }, &Refused::InBattle);
    refused(&mut e, Action::Search, &Refused::InBattle);
    // A command the battle refuses changes nothing, the battle included.
    let stale = battle::Command {
        revision: 9,
        actor: "p0".to_owned(),
        action: battle::Action::Pass,
    };
    let why = Refused::Battle(battle::RuleError::Stale {
        command: 9,
        battle: 0,
    });
    refused(&mut e, Action::Battle(stale), &why);

    // And without a battle its commands are refused.
    let mut idle = start();
    let pass = battle::Command {
        revision: 0,
        actor: "p0".to_owned(),
        action: battle::Action::Pass,
    };
    refused(&mut idle, Action::Battle(pass), &Refused::NotInBattle);
    refused(&mut idle, Action::Surrender, &Refused::NotInBattle);
}

// AT-022: the result of a battle is written back once.
#[test]
fn a_won_battle_pays_once_clears_the_band_and_takes_no_time() {
    let mut e = before_the_guard(None);
    act(&mut e, Action::Move { dx: 1, dy: 0 }).unwrap();
    let events = fight_out(&mut e);
    let won = Event::BattleWon {
        band: "demo-guard".to_owned(),
        reward: 30,
    };
    let arrived = Event::Moved {
        from: Cell { x: 3, y: 1 },
        to: Cell { x: 4, y: 1 },
    };
    assert_eq!(events[events.len() - 2..], [won, arrived]);
    assert_eq!(
        (e.status(), e.hero(), e.gold(), e.ticks(), wardens(&e)),
        (Status::Adventure, Cell { x: 4, y: 1 }, 90, 3, 12)
    );
    assert!(e.is_cleared("demo-guard") && e.battle().is_none());
    // No clue without the band's contract, and this band has none to give.
    assert_eq!(e.objective_known(), None);
    // The beaten band's cell is ordinary ground now: walking back over it starts nothing and pays nothing.
    go(&mut e, -1, 0, 1);
    go(&mut e, 1, 0, 1);
    assert_eq!((e.status(), e.gold()), (Status::Adventure, 90));
}

// AT-038: the clue goes to the hero who holds the band's contract.
#[test]
fn the_captain_gives_the_clue_only_to_the_holder_of_his_contract() {
    let beat_both = |contract: Option<&str>| {
        let mut e = before_the_guard(contract);
        act(&mut e, Action::Move { dx: 1, dy: 0 }).unwrap();
        fight_out(&mut e);
        go(&mut e, 1, 0, 3);
        act(&mut e, Action::Move { dx: 1, dy: 0 }).unwrap();
        (fight_out(&mut e), e)
    };
    let (events, with) = beat_both(Some("demo-captain"));
    assert!(events.contains(&Event::ClueGained {
        object: "demo-objective".to_owned()
    }));
    assert_eq!(
        (with.objective_known(), with.contract(), with.gold()),
        (Some(Cell { x: 8, y: 5 }), None, 150)
    );

    let (events, without) = beat_both(None);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::ClueGained { .. }))
    );
    assert_eq!(
        (without.objective_known(), without.gold()),
        (None, 150),
        "the gold all the same"
    );

    // The wrong contract is no contract.
    let (_, wrong) = beat_both(Some("demo-guard"));
    assert_eq!(
        (wrong.objective_known(), wrong.contract()),
        (None, None),
        "the guard's contract was fulfilled"
    );
}

// AT-042, and the fixture's route `surrender-first-encounter`.
#[test]
fn surrender_sends_the_hero_home_with_half_his_gold_and_four_wardens() {
    let mut e = before_the_guard(None);
    act(&mut e, Action::Move { dx: 1, dy: 0 }).unwrap();
    let events = act(&mut e, Action::Surrender).unwrap();
    assert_eq!(
        events,
        [
            Event::Recovered { gold: 30 },
            Event::TimePassed { from: 3, to: 7 }
        ]
    );
    assert_eq!(
        (e.status(), e.hero(), e.gold(), e.ticks(), wardens(&e)),
        (Status::Adventure, Cell { x: 1, y: 1 }, 30, 7, 4)
    );
    assert!(!e.is_cleared("demo-guard") && e.battle().is_none());
    assert_eq!(e.stock("warden"), 8, "what was bought stays bought");
    // The band stands as before, at its full strength.
    go(&mut e, 1, 0, 3);
    assert_eq!(e.battle().unwrap().stack("e0").unwrap().count, 1);
}

#[test]
fn a_lost_battle_recovers_like_a_surrender_and_keeps_what_was_won_before() {
    // The captain with fifty Wardens is more than twelve can beat.
    let mut e = start_with(|def| def.encounters[1].enemy_count = 50);
    act(
        &mut e,
        Action::TakeContract {
            id: "demo-captain".to_owned(),
        },
    )
    .unwrap();
    act(&mut e, recruit(2)).unwrap();
    go(&mut e, 1, 0, 3);
    fight_out(&mut e);
    go(&mut e, 1, 0, 4);
    let events = fight_out(&mut e);
    assert!(
        events.contains(&Event::Recovered { gold: 45 }),
        "half of the 90 he had"
    );
    assert_eq!(
        (e.status(), e.hero(), e.ticks(), wardens(&e)),
        (Status::Adventure, Cell { x: 1, y: 1 }, 11, 4)
    );
    assert!(e.is_cleared("demo-guard") && !e.is_cleared("demo-captain"));
    assert_eq!(e.contract(), Some("demo-captain"), "the contract is kept");
}

#[test]
fn recovery_takes_its_time_through_the_week_and_up_to_the_deadline() {
    // Surrender at tick 17: recovery ends at 21, across the week's end, which is settled for the four Wardens.
    let mut e = before_the_guard(None);
    wait(&mut e, 14);
    act(&mut e, Action::Move { dx: 1, dy: 0 }).unwrap();
    let events = act(&mut e, Action::Surrender).unwrap();
    let settled = Event::WeekSettled {
        week: 1,
        income: 20,
        upkeep: 4,
        gold: 46,
    };
    assert_eq!(
        events,
        [
            Event::Recovered { gold: 30 },
            settled,
            Event::TimePassed { from: 17, to: 21 }
        ]
    );

    // Surrender at tick 22: the deadline cuts recovery short, and time is up.
    let mut late = before_the_guard(None);
    wait(&mut late, 19);
    act(&mut late, Action::Move { dx: 1, dy: 0 }).unwrap();
    let events = act(&mut late, Action::Surrender).unwrap();
    assert_eq!(
        events.last(),
        Some(&Event::Ended {
            status: Status::Timeout
        })
    );
    assert_eq!((late.status(), late.ticks()), (Status::Timeout, 24));
}

#[test]
fn a_step_onto_a_band_as_time_runs_out_starts_no_battle() {
    let mut e = before_the_guard(None);
    wait(&mut e, 21);
    let events = act(&mut e, Action::Move { dx: 1, dy: 0 }).unwrap();
    assert_eq!(
        events,
        [
            Event::TimePassed { from: 23, to: 24 },
            Event::Ended {
                status: Status::Timeout
            }
        ]
    );
    assert_eq!(
        (e.status(), e.hero()),
        (Status::Timeout, Cell { x: 3, y: 1 })
    );
    assert!(e.battle().is_none());
}

// AT-023 in the campaign: a battle nobody wins leaves the hero and the band where they were.
#[test]
fn a_battle_that_runs_out_of_rounds_awards_nothing() {
    let mut e = before_the_guard(None);
    act(&mut e, Action::Move { dx: 1, dy: 0 }).unwrap();
    let mut last = Vec::new();
    while e.status() == Status::Battle {
        let battle = e.battle().unwrap();
        let pass = battle::Command {
            revision: battle.revision(),
            actor: battle.active().unwrap().stack.clone(),
            action: battle::Action::Pass,
        };
        last = act(&mut e, Action::Battle(pass)).unwrap();
    }
    assert_eq!(
        last.last(),
        Some(&Event::BattleStalled {
            band: "demo-guard".to_owned()
        })
    );
    assert_eq!(
        (e.status(), e.hero(), e.gold(), wardens(&e)),
        (Status::Adventure, Cell { x: 3, y: 1 }, 60, 12)
    );
    assert!(!e.is_cleared("demo-guard"));
}

#[test]
fn the_battle_is_part_of_the_state() {
    let mut e = before_the_guard(None);
    let before = e.digest();
    act(&mut e, Action::Move { dx: 1, dy: 0 }).unwrap();
    let Value::Object(state) = e.to_canon() else {
        panic!("an object")
    };
    assert_eq!(
        (&state["status"], &state["band"]),
        (&Value::str("battle"), &Value::str("demo-guard"))
    );
    assert!(state.contains_key("battle") && state.contains_key("arrival"));
    let engaged = e.digest();
    let strike = blow(&e);
    act(&mut e, strike).unwrap();
    assert!(before != engaged && engaged != e.digest());
}

#[test]
fn the_digest_follows_the_state() {
    let mut e = start();
    let before = e.digest();
    assert_eq!(before, start().digest());
    act(&mut e, recruit(1)).unwrap();
    assert_ne!(e.digest(), before);
    let Value::Object(state) = e.to_canon() else {
        panic!("an object")
    };
    assert_eq!(
        (&state["gold"], &state["status"]),
        (&Value::from(80_u32), &Value::str("adventure"))
    );
}

mod king;

const MEADOW: &str = include_str!("../../../../content/meadow/expedition.json");
const MEADOW_MAP: &str = include_str!("../../../../content/meadow/map.json");
const REFERENCE: &str = include_str!("../../../../content/reference/rules.json");
const MODERN: &str = include_str!("../../../../content/modern/rules.json");

// The second expedition's file says more than the fixture's: upkeep by rule, the troops of recovery by name, a
// band of several stacks. Each is read, and each mistake in it refused.
#[test]
fn the_meadow_loads_and_its_mistakes_are_refused() {
    let def = ExpeditionDef::from_texts(MEADOW, MEADOW_MAP, REFERENCE).unwrap();
    assert_eq!(def.clock.upkeep, Upkeep::TenthOfCost);
    assert_eq!(
        (def.recovery.troop_id.as_str(), def.recovery.count),
        ("sprites", 20)
    );
    assert_eq!(def.encounter("meadow-company").unwrap().also.len(), 1);
    assert!(ExpeditionDef::from_texts(MEADOW, MEADOW_MAP, MODERN).is_ok());
    assert_eq!(
        self::def().clock.upkeep,
        Upkeep::PerCreature(1),
        "the fixture keeps its flat upkeep"
    );

    let changed = |old: &str, new: &str| {
        assert!(MEADOW.contains(old), "{old}");
        ExpeditionDef::from_texts(&MEADOW.replacen(old, new, 1), MEADOW_MAP, REFERENCE)
    };
    let refused = |old: &str, new: &str| changed(old, new).is_err();
    assert!(refused(
        r#""upkeep_rule": "tenth_of_recruit_cost""#,
        r#""upkeep_rule": "a_fifth""#
    ));
    assert!(
        refused(
            r#""upkeep_per_living_creature": 0"#,
            r#""upkeep_per_living_creature": 2"#
        ),
        "a flat upkeep and a rule together"
    );
    assert!(
        refused(r#""troop": "sprites""#, r#""troop": "griffins""#),
        "an unknown troop in a band"
    );
    assert!(
        refused(r#""count": 30"#, r#""count": 0"#),
        "an empty stack in a band"
    );
    assert!(
        refused(
            r#""count": 20,
    "retains"#,
            r#""warden_count": 4, "count": 20,
    "retains"#
        ),
        "the troops of recovery named both ways"
    );
    // The enemy's column must be free as far down as the largest band reaches.
    assert!(
        refused(
            r#""x": 3,
        "y": 2"#,
            r#""x": 5,
        "y": 2"#
        ),
        "a rock where the band's second stack stands"
    );

    // The campaign's switches belong to the variant: the reference may not set them, and no share is over all.
    let rules = |text: &str, old: &str, new: &str| {
        assert!(text.contains(old), "{old}");
        Rules::from_json(&text.replacen(old, new, 1))
    };
    let section = r#""round_limit": 200,"#;
    let with = |unpaid: &str, share: u32| {
        format!(
            r#""campaign": {{"unpaid_week": "{unpaid}", "fallen_of_a_won_battle_return_percent": {share}}}, {section}"#
        )
    };
    let stay = "purse_emptied_every_stack_stays";
    let leave = "stacks_paid_in_slot_order_unpaid_leave";
    assert!(
        rules(REFERENCE, section, &with(stay, 0)).is_ok(),
        "the reference's own values, spelled out"
    );
    assert!(rules(REFERENCE, section, &with(leave, 0)).is_err());
    assert!(rules(REFERENCE, section, &with(stay, 25)).is_err());
    assert!(
        rules(
            MODERN,
            "\"fallen_of_a_won_battle_return_percent\": 25",
            "\"fallen_of_a_won_battle_return_percent\": 101"
        )
        .is_err()
    );
    assert!(rules(MODERN, stay, leave).is_ok());
    assert!(rules(MODERN, stay, "everyone_is_paid_anyway").is_err());
}
