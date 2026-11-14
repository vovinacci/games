//! Keys and mouse of the battle screen, the same whether the battle stands alone or is part of an expedition.

use bounty_core::battle::{Action, Battle};
use bounty_core::content::Cell;
use gamekit::screen;
use macroquad::prelude::*;

use crate::{control, view};

/// Seconds the pilot takes over each command of the enemy, or of a stack out of control, so that a player can
/// follow them.
pub const ENEMY_PACE: f32 = 0.45;

/// Moves the cursor by the arrows or the mouse and gives the action the player asked for this frame, if any, for
/// the stack whose turn it is. Only actions the rules offer are given.
pub fn battle_action(battle: &Battle, cursor: &mut Cell) -> Option<Action> {
    let board = &battle.rules().board;
    for (keys, dx, dy) in [
        ([KeyCode::Left, KeyCode::Kp4], -1, 0),
        ([KeyCode::Right, KeyCode::Kp6], 1, 0),
        ([KeyCode::Up, KeyCode::Kp8], 0, -1),
        ([KeyCode::Down, KeyCode::Kp2], 0, 1),
    ] {
        if keys.into_iter().any(is_key_pressed) {
            *cursor = control::moved(*cursor, dx, dy, board.width, board.height);
        }
    }
    let mut confirm = is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter);
    if is_mouse_button_pressed(MouseButton::Left)
        && let Some(cell) = view::cell_at(screen::mouse(), board.width, board.height)
    {
        *cursor = cell;
        confirm = true;
    }
    let seen = battle.observe();
    if confirm {
        return control::action_at(&seen, *cursor);
    }
    if is_key_pressed(KeyCode::W) && seen.legal.contains(&Action::Wait) {
        return Some(Action::Wait);
    }
    if is_key_pressed(KeyCode::Space) {
        return Some(Action::Pass);
    }
    // Spells in the order of their IDs, on keys 1, 2, ...: the order the sidebar lists them in.
    let keys = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4];
    let spells = battle.hero_charges().keys();
    spells
        .zip(keys)
        .filter(|(_, key)| is_key_pressed(*key))
        .find_map(|(spell, _)| control::cast_at(&seen, *cursor, spell))
}
