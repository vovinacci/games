//! Computer players. Each is a function from what a player can see ([`Observation`]) to a command; none can change
//! a battle or see its random numbers. A player that changes its choices gets a new name, so recorded results always
//! say which one played.

mod errand;

pub use errand::{ERRAND_V1, errand_v1};

use bounty_core::battle::{Action, Command, Observation, adjacent, hostile};
use bounty_core::content::Cell;

/// The name `approach_v1` goes by in suites and reports.
pub const APPROACH_V1: &str = "approach-v1";

/// The plain pilot of the regression runs (`docs/spec/04-AI-AND-BALANCE.md`, "Minimal first pilot"). In this order:
///
/// 1. strike a neighbouring enemy, the one with the lowest ID;
/// 2. shoot, the enemy with the lowest ID;
/// 3. go towards the nearest free cell beside an enemy: fly there when the stack can and the way on foot is longer
///    than one step or there is none, otherwise take the first step of the shortest way;
/// 4. pass.
///
/// It never waits and never casts. Not clever on purpose: it exists so that a change in the rules shows up as a
/// change in its results.
///
/// `None` when the battle is over.
#[must_use]
pub fn approach_v1(seen: &Observation) -> Option<Command> {
    let active = seen.active.as_ref()?;
    Some(Command {
        revision: seen.revision,
        actor: active.stack.clone(),
        action: choose(seen, &active.stack),
    })
}

fn choose(seen: &Observation, actor: &str) -> Action {
    // Legal actions come ordered by target ID within their kind.
    for wanted in [is_melee as fn(&Action) -> bool, is_shot] {
        if let Some(action) = seen.legal.iter().find(|a| wanted(a)) {
            return action.clone();
        }
    }
    let Some(me) = seen.stacks.iter().map(|s| &s.stack).find(|s| s.id == actor) else {
        return Action::Pass;
    };

    // Every free cell beside a living stack this one fights, with that stack's ID: where an attack can be made
    // from. For a stack out of control that is every other stack, its own side included.
    let mut enemies: Vec<_> = seen
        .stacks
        .iter()
        .map(|s| &s.stack)
        .filter(|s| s.alive() && hostile(s, me))
        .collect();
    enemies.sort_by(|a, b| a.id.cmp(&b.id));
    let cells = || (0..seen.height).flat_map(|y| (0..seen.width).map(move |x| Cell { x, y }));
    let mut goals = Vec::new();
    for enemy in &enemies {
        goals.extend(
            cells()
                .filter(|&c| seen.free(c) && adjacent(c, enemy.cell))
                .map(|c| (enemy.id.as_str(), c)),
        );
    }

    let ways = Ways::from(seen, me.cell);
    let on_foot = goals
        .iter()
        .filter_map(|&(enemy, cell)| {
            ways.steps(cell)
                .map(|steps| (steps, enemy, cell.y, cell.x, cell))
        })
        .min();
    let can_land = |to: Cell| seen.legal.contains(&Action::Fly { to });
    if on_foot.is_none_or(|(steps, ..)| steps > 1) {
        let landing = goals
            .iter()
            .filter(|&&(_, cell)| can_land(cell))
            .map(|&(enemy, cell)| (enemy, cell.y, cell.x, cell))
            .min();
        if let Some((.., to)) = landing {
            return Action::Fly { to };
        }
    }
    if let Some((.., goal)) = on_foot {
        let walk = Action::Walk {
            to: ways.first_step(goal),
        };
        if seen.legal.contains(&walk) {
            return walk;
        }
    }
    Action::Pass
}

fn is_melee(action: &Action) -> bool {
    matches!(action, Action::Melee { .. })
}

fn is_shot(action: &Action) -> bool {
    matches!(action, Action::Shoot { .. })
}

/// Shortest ways on foot from one cell to every cell that can be reached, under the rules' own step rule and with
/// the stacks where they stand now. Neighbours are tried row by row, which settles which of two equal ways is taken.
struct Ways {
    width: u32,
    from: Cell,
    /// For every cell: steps from the start and the cell before it on the way, when it can be reached.
    reached: Vec<Option<(u32, Cell)>>,
}

impl Ways {
    fn from(seen: &Observation, from: Cell) -> Self {
        let mut ways = Self {
            width: seen.width,
            from,
            reached: vec![None; (seen.width * seen.height) as usize],
        };
        let mut frontier = std::collections::VecDeque::from([(from, 0)]);
        while let Some((at, steps)) = frontier.pop_front() {
            for y in at.y.saturating_sub(1)..=at.y + 1 {
                for x in at.x.saturating_sub(1)..=at.x + 1 {
                    let to = Cell { x, y };
                    if to != from
                        && seen.step_allowed(at, to)
                        && ways.reached[ways.index(to)].is_none()
                    {
                        let index = ways.index(to);
                        ways.reached[index] = Some((steps + 1, at));
                        frontier.push_back((to, steps + 1));
                    }
                }
            }
        }
        ways
    }

    fn index(&self, cell: Cell) -> usize {
        (cell.y * self.width + cell.x) as usize
    }

    fn steps(&self, to: Cell) -> Option<u32> {
        self.reached
            .get(self.index(to))
            .copied()
            .flatten()
            .map(|(steps, _)| steps)
    }

    /// The first cell on the way to `to`, which must be reachable.
    fn first_step(&self, to: Cell) -> Cell {
        let mut cell = to;
        while let Some((_, before)) = self.reached[self.index(cell)] {
            if before == self.from {
                break;
            }
            cell = before;
        }
        cell
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use bounty_core::battle::{Battle, Outcome};
    use bounty_core::content::{Rules, Scenario, scenarios_from_json};
    use bounty_core::replay::{ReplayError, Tape};

    use super::*;

    const CASES: [&str; 4] = [
        "melee_exchange",
        "blocked_shooter",
        "flight_barrier",
        "unreachable_melee",
    ];

    fn rules() -> Arc<Rules> {
        Arc::new(Rules::from_json(include_str!("../../../prototype/rules.json")).unwrap())
    }

    fn scenario(id: &str) -> Scenario {
        let all =
            scenarios_from_json(include_str!("../../../prototype/battles.json"), &rules()).unwrap();
        all.into_iter().find(|s| s.id == id).unwrap()
    }

    fn battle(id: &str) -> Battle {
        Battle::start(rules(), &scenario(id), None).unwrap()
    }

    fn choice(battle: &Battle) -> Action {
        approach_v1(&battle.observe()).unwrap().action
    }

    /// Plays the battle to its end with the pilot on both sides, recording it.
    fn play(id: &str, seed: u32) -> (Battle, Tape) {
        let scenario = scenario(id);
        let mut battle = Battle::start(rules(), &scenario, Some(seed)).unwrap();
        let mut tape = Tape::new(&scenario, seed);
        while let Some(command) = approach_v1(&battle.observe()) {
            // AT-060: every command of the pilot is legal.
            tape.apply(&mut battle, &command)
                .unwrap_or_else(|e| panic!("{id} seed {seed}: {command:?} refused: {e}"));
        }
        (battle, tape)
    }

    #[test]
    fn it_strikes_a_neighbour_before_anything_else() {
        assert_eq!(
            choice(&battle("melee_exchange")),
            Action::Melee {
                target: "e0".to_owned()
            }
        );
        assert_eq!(
            choice(&battle("blocked_shooter")),
            Action::Melee {
                target: "e0".to_owned()
            }
        );
    }

    #[test]
    fn it_shoots_when_no_enemy_is_near() {
        let mut scenario = scenario("blocked_shooter");
        let slinger = scenario.stacks.iter_mut().find(|s| s.id == "p0").unwrap();
        (slinger.x, slinger.y) = (0, 0);
        let battle = Battle::start(rules(), &scenario, None).unwrap();
        assert_eq!(
            choice(&battle),
            Action::Shoot {
                target: "e0".to_owned()
            }
        );
    }

    #[test]
    fn it_walks_the_shortest_way_and_strikes_on_arrival() {
        let mut b = battle("blocked_shooter");
        b.apply(&approach_v1(&b.observe()).unwrap()).unwrap();
        // The warden at (1,4); the enemy at (3,2) has a free cell at (2,3), one step away.
        assert_eq!(b.active().unwrap().stack, "p1");
        assert_eq!(
            choice(&b),
            Action::Walk {
                to: Cell { x: 2, y: 3 }
            }
        );
        b.apply(&approach_v1(&b.observe()).unwrap()).unwrap();
        assert_eq!(
            choice(&b),
            Action::Melee {
                target: "e0".to_owned()
            }
        );
    }

    #[test]
    fn it_flies_over_what_it_cannot_walk_round() {
        let mut b = battle("flight_barrier");
        // Cells beside the enemy at (5,2), row by row: (4,1) comes first.
        assert_eq!(
            choice(&b),
            Action::Fly {
                to: Cell { x: 4, y: 1 }
            }
        );
        b.apply(&approach_v1(&b.observe()).unwrap()).unwrap();
        assert_eq!(
            choice(&b),
            Action::Melee {
                target: "e0".to_owned()
            }
        );
    }

    // AT-023: with no way to the enemy it passes, and the battle ends as a diagnosis, not a defeat.
    #[test]
    fn it_passes_when_the_enemy_cannot_be_reached() {
        assert_eq!(choice(&battle("unreachable_melee")), Action::Pass);
        let (end, tape) = play("unreachable_melee", 4);
        assert_eq!(end.outcome(), Some(Outcome::BudgetExceeded));
        assert_eq!(tape.commands.len(), 400);
        assert!(end.stacks().iter().all(|s| s.count == 8));
    }

    // AT-060: over all fixtures and many seeds the pilot is never refused and every battle ends.
    #[test]
    fn every_fixture_ends_and_no_command_is_refused() {
        for id in CASES {
            for seed in 0..40 {
                let (end, _) = play(id, seed);
                let expected_stall = id == "unreachable_melee";
                assert_eq!(
                    end.outcome() == Some(Outcome::BudgetExceeded),
                    expected_stall,
                    "{id} seed {seed}"
                );
            }
        }
    }

    // AT-061: the pilot's choice depends on what it sees, not on the random numbers to come.
    #[test]
    fn the_choice_does_not_depend_on_the_seed() {
        for id in CASES {
            let scenario = scenario(id);
            let choices: Vec<Action> = (0..5)
                .map(|seed| choice(&Battle::start(rules(), &scenario, Some(seed)).unwrap()))
                .collect();
            assert!(choices.iter().all(|c| *c == choices[0]), "{id}");
        }
    }

    // AT-003, AT-055: the recorded commands alone give the same battle again, at every step.
    #[test]
    fn a_recorded_battle_plays_back_to_the_same_state() {
        for id in CASES {
            let (end, tape) = play(id, 3);
            let again = tape.replay(rules(), &scenario(id)).unwrap();
            assert_eq!(again, end, "{id}");
            assert_eq!(again.digest(), *tape.digests.last().unwrap(), "{id}");
        }
    }

    #[test]
    fn a_tape_that_no_longer_fits_names_the_first_command_that_differs() {
        let (_, tape) = play("melee_exchange", 3);
        let scenario = scenario("melee_exchange");

        let mut other_seed = tape.clone();
        other_seed.seed = 4;
        assert!(matches!(
            other_seed.replay(rules(), &scenario),
            Err(ReplayError::Diverged { index: 0, .. })
        ));

        let mut changed = tape.clone();
        changed.commands[1].action = Action::Pass;
        assert!(matches!(
            changed.replay(rules(), &scenario),
            Err(ReplayError::Diverged { index: 1, .. })
        ));

        let mut stale = tape.clone();
        stale.commands[1].revision = 0;
        assert!(matches!(
            stale.replay(rules(), &scenario),
            Err(ReplayError::Refused { index: 1, .. })
        ));

        let wrong = self::scenario("flight_barrier");
        assert!(matches!(
            tape.replay(rules(), &wrong),
            Err(ReplayError::WrongTape(_))
        ));
    }

    /// A battle of the three reference troops on each side, with a rock in the middle.
    fn reference_fight(seed: u32) -> (Arc<Rules>, Scenario) {
        use bounty_core::content::{Side, StackStart};
        let rules = Arc::new(
            Rules::from_json(include_str!("../../../content/reference/rules.json")).unwrap(),
        );
        let stack = |id: &str, side, slot, troop: &str, count, x, y| StackStart {
            id: id.to_owned(),
            side,
            slot,
            troop_id: troop.to_owned(),
            count,
            injury: 0,
            x,
            y,
        };
        let scenario = Scenario {
            id: "reference_fight".to_owned(),
            rng_state: seed,
            obstacles: vec![Cell { x: 2, y: 2 }],
            hero_charges: std::collections::BTreeMap::new(),
            leadership: None,
            stacks: vec![
                stack("p0", Side::Player, 0, "wolves", 20, 0, 0),
                stack("p1", Side::Player, 1, "archers", 6, 0, 2),
                stack("p2", Side::Player, 2, "sprites", 40, 0, 4),
                stack("e0", Side::Enemy, 0, "wolves", 20, 5, 0),
                stack("e1", Side::Enemy, 1, "archers", 6, 5, 2),
                stack("e2", Side::Enemy, 2, "sprites", 40, 5, 4),
            ],
        };
        (rules, scenario)
    }

    // AT-060, AT-003 under the reference rules: the pilot is never refused, every battle ends with a winner, and
    // the recorded commands give the same battle again.
    #[test]
    fn the_pilot_plays_the_reference_rules_and_the_battle_replays() {
        let mut winners = std::collections::BTreeSet::new();
        for seed in 0..40 {
            let (rules, scenario) = reference_fight(seed);
            let mut battle = Battle::start(Arc::clone(&rules), &scenario, None).unwrap();
            let mut tape = Tape::new(&scenario, seed);
            while let Some(command) = approach_v1(&battle.observe()) {
                tape.apply(&mut battle, &command)
                    .unwrap_or_else(|e| panic!("seed {seed}: {command:?} refused: {e}"));
            }
            let outcome = battle.outcome().unwrap();
            assert_ne!(outcome, Outcome::BudgetExceeded, "seed {seed}");
            winners.insert(outcome.name());
            assert_eq!(
                tape.replay(rules, &scenario).unwrap(),
                battle,
                "seed {seed}"
            );
        }
        assert_eq!(
            winners.len(),
            2,
            "over forty seeds both sides win some: {winners:?}"
        );
    }

    // Every troop of the reference rules takes the field, each beside its neighbour in the table and against the
    // same pair, so that a troop the rules or the pilot cannot handle shows here and not in a later expedition.
    #[test]
    fn every_reference_troop_fights_a_battle_to_its_end() {
        let (rules, template) = reference_fight(7);
        let ids: Vec<String> = rules.troops.iter().map(|t| t.id.clone()).collect();
        for (place, first) in ids.iter().enumerate() {
            let second = &ids[(place + 1) % ids.len()];
            let mut scenario = template.clone();
            scenario.stacks.retain(|s| s.slot < 2);
            for stack in &mut scenario.stacks {
                let troop = if stack.slot == 0 { first } else { second };
                stack.troop_id.clone_from(troop);
                // About the same health on the field whatever the troop.
                stack.count = (200 / rules.troop(troop).unwrap().hp).max(1);
            }
            let mut battle = Battle::start(Arc::clone(&rules), &scenario, None).unwrap();
            let mut tape = Tape::new(&scenario, 7);
            while let Some(command) = approach_v1(&battle.observe()) {
                tape.apply(&mut battle, &command)
                    .unwrap_or_else(|e| panic!("{first} and {second}: {command:?} refused: {e}"));
            }
            let outcome = battle.outcome().unwrap();
            assert_ne!(outcome, Outcome::BudgetExceeded, "{first} and {second}");
            assert_eq!(
                tape.replay(Arc::clone(&rules), &scenario).unwrap(),
                battle,
                "{first} and {second}"
            );
        }
    }

    // With a hero whose leadership the twenty Wolves exceed (60 health against 45), they start out of control. The
    // pilot moves them like any stack: never refused, every battle ends, and the tape replays.
    #[test]
    fn the_pilot_plays_a_stack_out_of_control() {
        let mut regained = 0;
        for seed in 0..40 {
            let (rules, mut scenario) = reference_fight(seed);
            scenario.leadership = Some(45);
            let mut battle = Battle::start(Arc::clone(&rules), &scenario, None).unwrap();
            assert!(battle.stack("p0").unwrap().out_of_control);
            let mut tape = Tape::new(&scenario, seed);
            while let Some(command) = approach_v1(&battle.observe()) {
                tape.apply(&mut battle, &command)
                    .unwrap_or_else(|e| panic!("seed {seed}: {command:?} refused: {e}"));
            }
            assert_ne!(
                battle.outcome().unwrap(),
                Outcome::BudgetExceeded,
                "seed {seed}"
            );
            regained += usize::from(!battle.stack("p0").unwrap().out_of_control);
            assert_eq!(
                tape.replay(rules, &scenario).unwrap(),
                battle,
                "seed {seed}"
            );
        }
        assert!(
            regained > 0,
            "in some battles losses bring the Wolves back under control"
        );
    }

    // The `modern` variant, every switch on, with a hero whose leadership the Wolves exceed: the pilot is never
    // refused, no stack is ever out of control, every battle ends and replays.
    #[test]
    fn the_pilot_plays_the_modern_variant() {
        let modern =
            Arc::new(Rules::from_json(include_str!("../../../content/modern/rules.json")).unwrap());
        for seed in 0..40 {
            let (_, mut scenario) = reference_fight(seed);
            scenario.leadership = Some(45);
            let mut battle = Battle::start(Arc::clone(&modern), &scenario, None).unwrap();
            let mut tape = Tape::new(&scenario, seed);
            while let Some(command) = approach_v1(&battle.observe()) {
                tape.apply(&mut battle, &command)
                    .unwrap_or_else(|e| panic!("seed {seed}: {command:?} refused: {e}"));
                assert!(
                    battle.stacks().iter().all(|s| !s.out_of_control),
                    "seed {seed}"
                );
            }
            assert_ne!(
                battle.outcome().unwrap(),
                Outcome::BudgetExceeded,
                "seed {seed}"
            );
            assert_eq!(
                tape.replay(Arc::clone(&modern), &scenario).unwrap(),
                battle,
                "seed {seed}"
            );
        }
    }
}
