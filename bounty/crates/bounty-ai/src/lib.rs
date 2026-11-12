//! Computer players. Each is a function from what a player can see ([`Observation`]) to a command; none can change
//! a battle or see its random numbers. A player that changes its choices gets a new name, so recorded results always
//! say which one played.

use bounty_core::battle::{Action, Command, Observation, adjacent, step_allowed};
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

    // Every free cell beside a living enemy, with the ID of that enemy: where an attack can be made from.
    let mut enemies: Vec<_> = seen
        .stacks
        .iter()
        .map(|s| &s.stack)
        .filter(|s| s.alive() && s.side != me.side)
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
                        && step_allowed(at, to, |c| seen.free(c))
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
}
