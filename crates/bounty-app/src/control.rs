//! From the cursor to a command. The screen never decides what is allowed: it looks the cursor's cell up among the
//! actions the rules offer, and offers nothing else.

use bounty_core::battle::{Action, Observation};
use bounty_core::content::Cell;

/// What Enter does with the cursor on `cursor`: strike the enemy there, else shoot it, else step there, else fly
/// there. `None` when the rules offer none of these for that cell.
#[must_use]
pub fn action_at(seen: &Observation, cursor: Cell) -> Option<Action> {
    let target = stack_at(seen, cursor);
    let is = |action: &Action| seen.legal.contains(action);
    let on_stack = |make: fn(String) -> Action| target.map(|id| make(id.to_owned())).filter(is);
    on_stack(|target| Action::Melee { target })
        .or_else(|| on_stack(|target| Action::Shoot { target }))
        .or_else(|| Some(Action::Walk { to: cursor }).filter(is))
        .or_else(|| Some(Action::Fly { to: cursor }).filter(is))
}

/// The cast of `spell` on the stack under the cursor, when the rules offer it.
#[must_use]
pub fn cast_at(seen: &Observation, cursor: Cell, spell: &str) -> Option<Action> {
    let target = stack_at(seen, cursor)?.to_owned();
    Some(Action::Cast {
        spell: spell.to_owned(),
        target,
    })
    .filter(|action| seen.legal.contains(action))
}

/// The ID of the living stack on a cell.
#[must_use]
pub fn stack_at(seen: &Observation, cell: Cell) -> Option<&str> {
    seen.stacks
        .iter()
        .map(|s| &s.stack)
        .find(|s| s.alive() && s.cell == cell)
        .map(|s| s.id.as_str())
}

/// The cursor one cell on, stopping at the edge of the board.
#[must_use]
pub fn moved(cursor: Cell, dx: i32, dy: i32, width: u32, height: u32) -> Cell {
    let step =
        |at: u32, by: i32, size: u32| at.saturating_add_signed(by).min(size.saturating_sub(1));
    Cell {
        x: step(cursor.x, dx, width),
        y: step(cursor.y, dy, height),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use bounty_core::battle::Battle;

    use super::*;
    use crate::content;

    fn seen(id: &str, charges: u32) -> Observation {
        let rules = Arc::new(content::rules());
        let mut scenario = content::scenario(&rules, id).unwrap();
        scenario.hero_charges.insert("blast".to_owned(), charges);
        Battle::start(rules, &scenario, None).unwrap().observe()
    }

    fn cell(x: u32, y: u32) -> Cell {
        Cell { x, y }
    }

    #[test]
    fn enter_strikes_shoots_steps_or_flies_as_the_rules_allow() {
        // The slinger at (2,2) stands beside the enemy at (3,2): it strikes, it cannot shoot.
        let beside = seen("blocked_shooter", 0);
        assert_eq!(
            action_at(&beside, cell(3, 2)),
            Some(Action::Melee {
                target: "e0".to_owned()
            })
        );
        assert_eq!(
            action_at(&beside, cell(1, 2)),
            Some(Action::Walk { to: cell(1, 2) })
        );
        assert_eq!(
            action_at(&beside, cell(1, 4)),
            None,
            "a friend stands there"
        );
        assert_eq!(action_at(&beside, cell(5, 0)), None, "too far to step");
        assert_eq!(action_at(&beside, cell(2, 2)), None, "its own cell");

        // The skyrider: a step beside it, a flight anywhere else that is free, nothing on the wall.
        let flyer = seen("flight_barrier", 0);
        assert_eq!(
            action_at(&flyer, cell(1, 2)),
            Some(Action::Walk { to: cell(1, 2) })
        );
        assert_eq!(
            action_at(&flyer, cell(4, 2)),
            Some(Action::Fly { to: cell(4, 2) })
        );
        assert_eq!(action_at(&flyer, cell(2, 2)), None, "an obstacle");
        assert_eq!(
            action_at(&flyer, cell(5, 2)),
            None,
            "the enemy is out of reach"
        );
    }

    #[test]
    fn a_shooter_away_from_the_enemy_shoots() {
        let rules = Arc::new(content::rules());
        let mut scenario = content::scenario(&rules, "blocked_shooter").unwrap();
        let slinger = scenario.stacks.iter_mut().find(|s| s.id == "p0").unwrap();
        (slinger.x, slinger.y) = (0, 0);
        let far = Battle::start(rules, &scenario, None).unwrap().observe();
        assert_eq!(
            action_at(&far, cell(3, 2)),
            Some(Action::Shoot {
                target: "e0".to_owned()
            })
        );
    }

    #[test]
    fn a_spell_needs_a_charge_and_a_stack_it_fits() {
        let with = seen("melee_exchange", 1);
        let blast = Action::Cast {
            spell: "blast".to_owned(),
            target: "e0".to_owned(),
        };
        assert_eq!(cast_at(&with, cell(2, 2), "blast"), Some(blast));
        assert_eq!(cast_at(&with, cell(1, 2), "blast"), None, "not on a friend");
        assert_eq!(cast_at(&with, cell(0, 0), "blast"), None, "nobody there");
        assert_eq!(
            cast_at(&seen("melee_exchange", 0), cell(2, 2), "blast"),
            None,
            "no charge"
        );
    }

    #[test]
    fn the_cursor_stops_at_the_edges() {
        assert_eq!(moved(cell(0, 0), -1, -1, 6, 5), cell(0, 0));
        assert_eq!(moved(cell(5, 4), 1, 1, 6, 5), cell(5, 4));
        assert_eq!(moved(cell(2, 2), 1, -1, 6, 5), cell(3, 1));
    }
}
