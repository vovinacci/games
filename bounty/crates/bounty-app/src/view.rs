//! Draws a battle. Everything shown is read from the battle as it stands; nothing drawn here feeds back into it.
//! The layout is the fixed one of `docs/spec/CONTENT-AND-ASSETS.md`: header, board, sidebar, log.

use bounty_core::battle::{Action, Battle, Observation, Stack};
use bounty_core::content::{Cell, MODERN_RULESET, Profile, Side, Troop};
use gamekit::t;
use gamekit::text::{measure, text_at, text_centered_fit, text_fit};
use macroquad::prelude::*;

use crate::control::action_at;
use crate::words::{morale_text, named, outcome_text, stack_name};

/// Side of a board cell, and where the board's top-left corner is.
pub const CELL: f32 = 96.0;
pub const BOARD: Vec2 = vec2(112.0, 96.0);

pub const SIDEBAR: Rect = Rect {
    x: 800.0,
    y: 48.0,
    w: 464.0,
    h: 576.0,
};
const LOG: Rect = Rect {
    x: 16.0,
    y: 640.0,
    w: 1248.0,
    h: 64.0,
};

pub const PANEL: Color = Color::new(0.07, 0.08, 0.12, 1.0);
pub const LINE: Color = Color::new(0.3, 0.34, 0.42, 1.0);
pub const GROUND: Color = Color::new(0.11, 0.15, 0.13, 1.0);
const ROCK: Color = Color::new(0.24, 0.24, 0.27, 1.0);
pub const PLAYER: Color = Color::new(0.45, 0.72, 1.0, 1.0);
pub const ENEMY: Color = Color::new(1.0, 0.55, 0.35, 1.0);
const REACH: Color = Color::new(0.35, 0.8, 0.5, 0.22);
pub const TARGET: Color = Color::new(1.0, 0.3, 0.3, 1.0);
pub const CURSOR: Color = Color::new(1.0, 0.9, 0.3, 1.0);

/// The corner of a cell on the virtual screen.
#[must_use]
pub fn corner(cell: Cell) -> Vec2 {
    BOARD + vec2(cell.x as f32, cell.y as f32) * CELL
}

/// The board cell under a point of the virtual screen, if it is on the board.
#[must_use]
pub fn cell_at(point: Vec2, width: u32, height: u32) -> Option<Cell> {
    let at = (point - BOARD) / CELL;
    let inside = at.x >= 0.0 && at.y >= 0.0 && at.x < width as f32 && at.y < height as f32;
    inside.then_some(Cell {
        x: at.x as u32,
        y: at.y as u32,
    })
}

/// What the screen knows besides the battle.
pub struct Shown<'a> {
    pub scenario: &'a str,
    pub seed: u32,
    pub cursor: Cell,
    /// The last lines of the log, oldest first.
    pub log: &'a [String],
    /// The battle is part of an expedition: it can be given up, and it is not fought again.
    pub in_expedition: bool,
}

pub fn draw(battle: &Battle, shown: &Shown<'_>) {
    let seen = battle.observe();
    header(battle, shown);
    board(battle, &seen, shown.cursor);
    sidebar(battle, &seen, shown);
    log(shown.log);
    if let Some(outcome) = battle.outcome() {
        banner(&outcome_text(outcome));
    }
}

fn header(battle: &Battle, shown: &Shown<'_>) {
    text_at(t!("title"), 16.0, 32.0, 28.0, WHITE);
    let battle_line = t!("header-battle", id = shown.scenario, seed = shown.seed);
    text_fit(
        &format!("{battle_line}   {}", t!("round", n = battle.round())),
        150.0,
        32.0,
        620.0,
        18.0,
        LIGHTGRAY,
    );
    // The rules in use are not the original game's; the screen says so, always, and says which they are.
    let rules = battle.rules();
    let note = match rules.profile {
        Profile::Prototype => t!("prototype-note"),
        Profile::Reference(_) if rules.id == MODERN_RULESET => t!("modern-note"),
        Profile::Reference(_) => t!("reference-note"),
    };
    text_fit(&note, SIDEBAR.x, 32.0, SIDEBAR.w, 18.0, CURSOR);
}

fn board(battle: &Battle, seen: &Observation, cursor: Cell) {
    let players_turn = player_commands(battle);
    for y in 0..seen.height {
        for x in 0..seen.width {
            let cell = Cell { x, y };
            let at = corner(cell);
            let rock = seen.obstacles.contains(&cell);
            draw_rectangle(at.x, at.y, CELL, CELL, if rock { ROCK } else { GROUND });
            if rock {
                // Marked by pattern as well as colour.
                draw_line(
                    at.x + 12.0,
                    at.y + 12.0,
                    at.x + CELL - 12.0,
                    at.y + CELL - 12.0,
                    3.0,
                    LINE,
                );
                draw_line(
                    at.x + CELL - 12.0,
                    at.y + 12.0,
                    at.x + 12.0,
                    at.y + CELL - 12.0,
                    3.0,
                    LINE,
                );
            }
            if players_turn {
                match action_at(seen, cell) {
                    Some(Action::Walk { .. } | Action::Fly { .. }) => {
                        draw_rectangle(at.x, at.y, CELL, CELL, REACH);
                    }
                    Some(_) => draw_rectangle_lines(
                        at.x + 5.0,
                        at.y + 5.0,
                        CELL - 10.0,
                        CELL - 10.0,
                        3.0,
                        TARGET,
                    ),
                    None => {}
                }
            }
            draw_rectangle_lines(at.x, at.y, CELL, CELL, 1.0, LINE);
        }
    }
    let active = battle.active().map(|a| a.stack.as_str());
    for stack in battle.stacks().iter().filter(|s| s.alive()) {
        let fighting = battle.fighting(&stack.id).unwrap_or(stack.count);
        token(
            stack,
            battle.troop(stack),
            active == Some(stack.id.as_str()),
            fighting,
        );
    }
    let at = corner(cursor);
    draw_rectangle_lines(at.x + 1.0, at.y + 1.0, CELL - 2.0, CELL - 2.0, 4.0, CURSOR);
}

/// A stack on the board. The shape tells the kind (square: fights hand to hand, circle: shoots, triangle: flies),
/// and a filled shape is the player's while a hollow one is the enemy's, so neither depends on colour alone.
fn token(stack: &Stack, troop: &Troop, active: bool, fighting: u32) {
    let at = corner(stack.cell);
    let centre = at + vec2(CELL / 2.0, 28.0);
    let (color, filled) = if stack.side == Side::Player {
        (PLAYER, true)
    } else {
        (ENEMY, false)
    };
    let r = 20.0;
    if troop.flights_per_activation > 0 {
        let (a, b, c) = (
            centre + vec2(0.0, -r),
            centre + vec2(-r, r),
            centre + vec2(r, r),
        );
        if filled {
            draw_triangle(a, b, c, color);
        } else {
            draw_triangle_lines(a, b, c, 4.0, color);
        }
    } else if troop.ranged().is_some() {
        if filled {
            draw_circle(centre.x, centre.y, r, color);
        } else {
            draw_circle_lines(centre.x, centre.y, r - 2.0, 4.0, color);
        }
    } else if filled {
        draw_rectangle(centre.x - r, centre.y - r, 2.0 * r, 2.0 * r, color);
    } else {
        draw_rectangle_lines(centre.x - r, centre.y - r, 2.0 * r, 2.0 * r, 8.0, color);
    }
    if active {
        draw_rectangle_lines(at.x + 4.0, at.y + 4.0, CELL - 8.0, CELL - 8.0, 2.0, WHITE);
    }
    if stack.out_of_control {
        // A mark in the corner, not a colour: the stack obeys nobody.
        text_at("!", at.x + CELL - 20.0, at.y + 26.0, 28.0, TARGET);
        text_at("!", at.x + 10.0, at.y + 26.0, 28.0, TARGET);
    }
    // "10/11": ten of the eleven fight, the hero's leadership reaching no further.
    let number = if fighting < stack.count {
        format!("{fighting}/{}", stack.count)
    } else {
        stack.count.to_string()
    };
    let count = if stack.injury > 0 {
        format!("{number} -{}", stack.injury)
    } else {
        number
    };
    let middle = at.x + CELL / 2.0;
    text_centered_fit(&count, middle, at.y + 68.0, CELL - 8.0, 20.0, WHITE);
    // A name too long for the cell goes on two lines, split at the space nearest its middle.
    let name = named("troop", &stack.troop_id);
    let width = CELL - 6.0;
    let split = name
        .match_indices(' ')
        .map(|(i, _)| i)
        .min_by_key(|i| i.abs_diff(name.len() / 2));
    match split.filter(|_| measure(&name, 12.0).width > width) {
        Some(i) => {
            text_centered_fit(&name[..i], middle, at.y + 79.0, width, 11.0, LIGHTGRAY);
            text_centered_fit(&name[i + 1..], middle, at.y + 89.0, width, 11.0, LIGHTGRAY);
        }
        None => text_centered_fit(&name, middle, at.y + 84.0, width, 12.0, LIGHTGRAY),
    }
}

/// Whether the stack whose turn it is takes the player's commands: one of the player's, and under control. A stack
/// out of control is moved by the pilot, like the enemy's.
pub fn player_commands(battle: &Battle) -> bool {
    let active = battle.active().and_then(|a| battle.stack(&a.stack));
    active.is_some_and(|s| s.side == Side::Player && !s.out_of_control)
}

fn sidebar(battle: &Battle, seen: &Observation, shown: &Shown<'_>) {
    let cursor = shown.cursor;
    draw_rectangle(SIDEBAR.x, SIDEBAR.y, SIDEBAR.w, SIDEBAR.h, PANEL);
    draw_rectangle_lines(SIDEBAR.x, SIDEBAR.y, SIDEBAR.w, SIDEBAR.h, 2.0, LINE);
    let (x, width) = (SIDEBAR.x + 16.0, SIDEBAR.w - 32.0);
    let mut y = SIDEBAR.y + 34.0;
    let mut line = |text: &str, size: f32, color: Color| {
        text_fit(text, x, y, width, size, color);
        y += size * 1.45;
    };
    let Some(active) = battle.active() else {
        return keys_after_the_battle(&mut line);
    };
    let Some(stack) = battle.stack(&active.stack) else {
        return;
    };
    let troop = battle.troop(stack);

    line(
        &t!("turn-of", name = stack_name(battle, &stack.id)),
        22.0,
        WHITE,
    );
    line(&t!("stat-count", n = stack.count), 18.0, LIGHTGRAY);
    line(
        &t!("stat-health", hp = troop.hp, injury = stack.injury),
        18.0,
        LIGHTGRAY,
    );
    line(&t!("stat-movement", n = active.movement), 18.0, LIGHTGRAY);
    if troop.ranged().is_some() {
        line(&t!("stat-shots", n = stack.shots), 18.0, LIGHTGRAY);
    }
    if troop.flights_per_activation > 0 {
        line(&t!("stat-flights", n = active.flights), 18.0, LIGHTGRAY);
    }
    // How many of the stack fight, where the hero's leadership does not reach all of them.
    if let Some(fighting) = battle.fighting(&stack.id).filter(|&n| n < stack.count) {
        line(
            &t!("stat-fighting", n = fighting, count = stack.count),
            18.0,
            TARGET,
        );
    }
    if let Some(morale) = battle.morale(&stack.id) {
        line(
            &t!("stat-morale", morale = morale_text(morale)),
            18.0,
            LIGHTGRAY,
        );
        // Low morale comes from the company the stack keeps: say whose.
        let lowered_by = battle.morale_lowered_by(&stack.id);
        if !lowered_by.is_empty() {
            let names: Vec<String> = lowered_by.iter().map(|id| stack_name(battle, id)).collect();
            line(
                &t!("morale-because", names = names.join(", ")),
                18.0,
                LIGHTGRAY,
            );
        }
    }
    if let Some(leadership) = battle.leadership() {
        line(&t!("stat-leadership", n = leadership), 18.0, LIGHTGRAY);
    }
    line("", 10.0, WHITE);
    if stack.out_of_control {
        return line(&t!("out-of-control"), 18.0, TARGET);
    }
    if stack.side == Side::Enemy {
        return line(&t!("enemy-turn"), 20.0, ENEMY);
    }

    line(&t!("actions"), 20.0, WHITE);
    line(&t!("key-move"), 18.0, LIGHTGRAY);
    let what = match action_at(seen, cursor) {
        Some(Action::Melee { target }) => t!("act-melee", name = stack_name(battle, &target)),
        Some(Action::Shoot { target }) => t!("act-shoot", name = stack_name(battle, &target)),
        Some(Action::Walk { .. }) => t!("act-walk"),
        Some(Action::Fly { .. }) => t!("act-fly"),
        _ => t!("act-none"),
    };
    line(&t!("key-act", what = what), 18.0, CURSOR);
    if seen.legal.contains(&Action::Wait) {
        line(&t!("key-wait"), 18.0, LIGHTGRAY);
    }
    line(&t!("key-pass"), 18.0, LIGHTGRAY);
    // Spells in the order of their IDs, on keys 1, 2, ...: the same order `spell_keys` in main uses.
    for (place, (spell, charges)) in battle.hero_charges().iter().enumerate() {
        let can = seen
            .legal
            .iter()
            .any(|a| matches!(a, Action::Cast { spell: s, .. } if s == spell));
        let text = t!(
            "key-cast",
            key = (place + 1).to_string(),
            spell = named("spell", spell),
            n = *charges
        );
        line(&text, 18.0, if can { LIGHTGRAY } else { DARKGRAY });
    }
    if shown.in_expedition {
        line(&t!("key-surrender"), 18.0, LIGHTGRAY);
    }
    line("", 10.0, WHITE);
    foot(&mut line);
}

/// The last lines of every sidebar: the keys that always work, and which build this is.
pub fn foot(line: &mut impl FnMut(&str, f32, Color)) {
    line(&t!("key-language"), 16.0, GRAY);
    line(&t!("key-quit"), 16.0, GRAY);
    line(&t!("version", v = crate::VERSION), 16.0, DARKGRAY);
}

fn keys_after_the_battle(line: &mut impl FnMut(&str, f32, Color)) {
    line(&t!("key-again"), 20.0, WHITE);
    foot(line);
}

pub fn log(lines: &[String]) {
    draw_rectangle(LOG.x, LOG.y, LOG.w, LOG.h, PANEL);
    draw_rectangle_lines(LOG.x, LOG.y, LOG.w, LOG.h, 2.0, LINE);
    let shown = &lines[lines.len().saturating_sub(3)..];
    for (row, text) in shown.iter().enumerate() {
        let newest = row + 1 == shown.len();
        text_fit(
            text,
            LOG.x + 12.0,
            LOG.y + 19.0 + 19.0 * row as f32,
            LOG.w - 24.0,
            16.0,
            if newest { WHITE } else { GRAY },
        );
    }
}

/// A result, at the foot of the sidebar, where it covers nothing of the board or the map.
pub fn banner(text: &str) {
    let band = Rect {
        x: SIDEBAR.x + 16.0,
        y: SIDEBAR.y + SIDEBAR.h - 100.0,
        w: SIDEBAR.w - 32.0,
        h: 84.0,
    };
    draw_rectangle(band.x, band.y, band.w, band.h, BLACK);
    draw_rectangle_lines(band.x, band.y, band.w, band.h, 3.0, CURSOR);
    text_centered_fit(
        text,
        band.x + band.w / 2.0,
        band.y + 54.0,
        band.w - 24.0,
        36.0,
        WHITE,
    );
}
