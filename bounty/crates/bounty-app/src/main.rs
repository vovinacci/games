//! Bounty: for now one battle, under the synthetic prototype rules or the first rules taken from a remake of the
//! original. The player's stacks by keyboard or mouse, the enemy's by the plain pilot.
//!
//! Developer flags: `--rules prototype|reference` picks the rules, `--scenario <id>` and `--seed <n>` the battle; `--lang <id>` the language; `--steps <n>`
//! lets the pilot play that many commands for both sides first; `--shot <file.png>` saves a screenshot and quits.
//! With `--shot` the user's settings are neither read nor written and the dedication is skipped.

mod content;
mod control;
mod settings;
mod view;
mod words;

use std::sync::Arc;

use bounty_ai::approach_v1;
use bounty_core::battle::{Action, Battle, Command};
use bounty_core::content::{Cell, Rules, Scenario, Side};
use gamekit::dedication::Intro;
use gamekit::{dev, i18n, screen};
use macroquad::prelude::*;

use crate::settings::Store;

/// Seconds the pilot takes over each command of the enemy, or of a stack out of control, so that a player can
/// follow them.
const ENEMY_PACE: f32 = 0.45;

struct Game {
    rules: Arc<Rules>,
    scenario: Scenario,
    seed: u32,
    battle: Battle,
    cursor: Cell,
    log: Vec<String>,
    /// Time since the enemy's last command.
    waited: f32,
}

impl Game {
    fn new(rules: Arc<Rules>, scenario: Scenario, seed: u32) -> Self {
        // The scenario was checked when it was read, so the battle starts.
        let battle = Battle::start(Arc::clone(&rules), &scenario, Some(seed))
            .expect("a checked scenario starts");
        let cursor = battle
            .active()
            .and_then(|a| battle.stack(&a.stack))
            .map_or(Cell { x: 0, y: 0 }, |s| s.cell);
        Self {
            rules,
            scenario,
            seed,
            battle,
            cursor,
            log: Vec::new(),
            waited: 0.0,
        }
    }

    /// Applies an action of the stack whose turn it is and writes what happened into the log. Actions come from
    /// what the rules offer, so a refusal means the screen and the rules disagree: it is shown, not hidden.
    fn act(&mut self, action: Action) {
        let Some(active) = self.battle.active() else {
            return;
        };
        let command = Command {
            revision: self.battle.revision(),
            actor: active.stack.clone(),
            action,
        };
        match self.battle.apply(&command) {
            Ok(events) => {
                self.log.extend(
                    events
                        .iter()
                        .filter_map(|event| words::describe(event, &self.battle)),
                );
                self.follow_the_turn();
            }
            Err(refused) => self.log.push(refused.to_string()),
        }
        self.waited = 0.0;
    }

    /// Puts the cursor on the player's stack when its turn begins.
    fn follow_the_turn(&mut self) {
        let active = self
            .battle
            .active()
            .and_then(|a| self.battle.stack(&a.stack));
        if let Some(stack) = active.filter(|s| s.side == Side::Player && !s.out_of_control) {
            self.cursor = stack.cell;
        }
    }

    /// The pilot's command for whoever's turn it is.
    fn pilot(&mut self) {
        if let Some(command) = approach_v1(&self.battle.observe()) {
            self.act(command.action);
        }
    }

    fn update(&mut self) {
        if self.battle.outcome().is_some() {
            if is_key_pressed(KeyCode::R) {
                *self = Self::new(
                    Arc::clone(&self.rules),
                    self.scenario.clone(),
                    self.seed.wrapping_add(1),
                );
            }
            return;
        }
        if !view::player_commands(&self.battle) {
            self.waited += get_frame_time();
            if self.waited >= ENEMY_PACE {
                self.pilot();
            }
            return;
        }
        let board = &self.rules.board;
        for (keys, dx, dy) in [
            ([KeyCode::Left, KeyCode::Kp4], -1, 0),
            ([KeyCode::Right, KeyCode::Kp6], 1, 0),
            ([KeyCode::Up, KeyCode::Kp8], 0, -1),
            ([KeyCode::Down, KeyCode::Kp2], 0, 1),
        ] {
            if keys.into_iter().any(is_key_pressed) {
                self.cursor = control::moved(self.cursor, dx, dy, board.width, board.height);
            }
        }
        let mut confirm = is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter);
        let mouse = screen::mouse();
        if is_mouse_button_pressed(MouseButton::Left)
            && let Some(cell) = view::cell_at(mouse, board.width, board.height)
        {
            self.cursor = cell;
            confirm = true;
        }
        let seen = self.battle.observe();
        if confirm {
            if let Some(action) = control::action_at(&seen, self.cursor) {
                self.act(action);
            }
        } else if is_key_pressed(KeyCode::W) && seen.legal.contains(&Action::Wait) {
            self.act(Action::Wait);
        } else if is_key_pressed(KeyCode::Space) {
            self.act(Action::Pass);
        } else {
            let keys = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4];
            let spells: Vec<String> = self.battle.hero_charges().keys().cloned().collect();
            for (spell, key) in spells.iter().zip(keys) {
                if is_key_pressed(key)
                    && let Some(action) = control::cast_at(&seen, self.cursor, spell)
                {
                    self.act(action);
                    break;
                }
            }
        }
    }
}

struct Options {
    scenario: String,
    seed: Option<u32>,
    language: Option<String>,
    steps: u32,
    shot: Option<String>,
}

async fn run(options: Options, rules: Arc<Rules>, scenario: Scenario) {
    // A developer's screenshot run leaves the user's files alone.
    let store = if options.shot.is_some() {
        Store(None)
    } else {
        Store::of_the_user()
    };
    let mut settings = store.load();
    i18n::init(&words::LANGUAGES);
    let wanted = options
        .language
        .clone()
        .or_else(|| settings.language.clone());
    i18n::set_language(wanted.map_or_else(i18n::detect, |id| i18n::place_of(&id)));

    let seed = options.seed.unwrap_or(scenario.rng_state);
    let mut game = Game::new(rules, scenario, seed);
    for _ in 0..options.steps {
        game.pilot();
    }
    let mut intro = (!settings.dedication_seen && options.shot.is_none()).then(Intro::default);
    let mut frame = 0_u32;
    loop {
        frame += 1;
        clear_background(BLACK);
        set_camera(&screen::camera());
        if let Some(showing) = &mut intro {
            if !showing.show() {
                intro = None;
                settings.dedication_seen = true;
                store.save(&settings);
            }
        } else {
            if is_key_pressed(KeyCode::L) {
                let next = (i18n::language() + 1) % i18n::language_count().max(1);
                i18n::set_language(next);
                settings.language = Some(words::LANGUAGES[next].id.to_owned());
                store.save(&settings);
            }
            if options.shot.is_none() {
                game.update();
            }
            let shown = view::Shown {
                scenario: &options.scenario,
                seed: game.seed,
                cursor: game.cursor,
                log: &game.log,
            };
            view::draw(&game.battle, &shown);
            if is_key_pressed(KeyCode::Escape) {
                break;
            }
        }
        if let Some(path) = &options.shot
            && frame == 30
        {
            dev::save_shot(path);
            break;
        }
        next_frame().await;
    }
}

/// Reads the command line; a message for the user when something on it cannot be used.
fn options(rules: &Rules) -> Result<Options, String> {
    let number = |flag: &str| match dev::arg_after(flag) {
        Some(text) => text
            .parse::<u32>()
            .map(Some)
            .map_err(|_| format!("{flag} {text}: a whole number is needed")),
        None => Ok(None),
    };
    let shot = dev::arg_after("--shot");
    if let Some(path) = &shot
        && let Some(problem) = dev::shot_problem(std::path::Path::new(path))
    {
        return Err(format!("--shot {path}: {problem}"));
    }
    Ok(Options {
        scenario: dev::arg_after("--scenario")
            .unwrap_or_else(|| content::first_scenario(rules).to_owned()),
        seed: number("--seed")?,
        language: dev::arg_after("--lang"),
        steps: number("--steps")?.unwrap_or(0),
        shot,
    })
}

fn main() {
    let rules = dev::arg_after("--rules").unwrap_or_else(|| "prototype".to_owned());
    let start = content::rules_named(&rules).and_then(|rules| {
        let options = options(&rules)?;
        let scenario = content::scenario(&rules, &options.scenario).map_err(|e| e.to_string())?;
        Ok((options, Arc::new(rules), scenario))
    });
    match start {
        Ok((options, rules, scenario)) => {
            macroquad::Window::from_config(screen::window("Bounty"), run(options, rules, scenario));
        }
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    }
}
