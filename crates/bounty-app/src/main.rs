//! Bounty: a small expedition (map, town, battles, a prize to find before time runs out), saved after every
//! command and continued at the next start; or, with `--rules` or `--scenario` alone, one battle by itself.
//! `--expedition demo|meadow` picks the expedition: `demo` is the test fixture, `meadow` a small one on the
//! reference's troops, played under `--rules reference` or `--rules modern`. The player's
//! stacks by keyboard or mouse, the enemy's by the plain pilot.
//!
//! Developer flags: `--rules prototype|reference|modern` and `--scenario <id>` pick one battle and `--seed <n>` its
//! seed; `--route <id>` picks a route of the expedition's file for `--steps`; `--lang <id>` the language;
//! `--steps <n>` plays that many commands first (the pilot's in a battle, the route's in the expedition);
//! `--shot <file.png>` saves a screenshot and quits. With `--shot` or `--steps` the user's files are neither read
//! nor written, and with `--shot` the dedication is skipped.

mod adventure;
mod content;
mod control;
mod input;
mod map;
mod saves;
mod settings;
mod view;
mod words;

use std::sync::Arc;

use bounty_ai::approach_v1;
use bounty_core::battle::{Action, Battle, Command};
use bounty_core::campaign::{self, Expedition, ExpeditionDef, Status};
use bounty_core::content::{Cell, Rules, Scenario, Side};
use gamekit::dedication::Intro;
use gamekit::{dev, i18n, screen};
use macroquad::prelude::*;

use crate::adventure::Adventure;
use crate::input::ENEMY_PACE;
use crate::saves::Saves;
use crate::settings::Store;

/// The version of this build, as the sidebar shows it and a release is named.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// One battle by itself.
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
        if let Some(action) = input::battle_action(&self.battle, &mut self.cursor) {
            self.act(action);
        }
    }

    fn draw(&self) {
        let shown = view::Shown {
            scenario: &self.scenario.id,
            seed: self.seed,
            cursor: self.cursor,
            log: &self.log,
            in_expedition: false,
        };
        view::draw(&self.battle, &shown);
    }
}

/// The keys of the expedition: those of its battle while one is fought, those of the map otherwise.
fn update_adventure(adventure: &mut Adventure) {
    if adventure.over() {
        if is_key_pressed(KeyCode::N) {
            adventure.restart();
        }
        return;
    }
    if let Some(battle) = adventure.expedition.battle() {
        if is_key_pressed(KeyCode::G) {
            adventure.act(campaign::Action::Surrender);
        } else if !view::player_commands(battle) {
            adventure.waited += get_frame_time();
            if adventure.waited >= ENEMY_PACE {
                adventure.pilot();
            }
        } else {
            let mut cursor = adventure.cursor;
            let action = input::battle_action(battle, &mut cursor);
            adventure.cursor = cursor;
            if let Some(action) = action {
                adventure.fight(action);
            }
        }
        return;
    }
    for (keys, dx, dy) in [
        ([KeyCode::Left, KeyCode::Kp4], -1, 0),
        ([KeyCode::Right, KeyCode::Kp6], 1, 0),
        ([KeyCode::Up, KeyCode::Kp8], 0, -1),
        ([KeyCode::Down, KeyCode::Kp2], 0, 1),
    ] {
        if keys.into_iter().any(is_key_pressed) {
            return adventure.act(campaign::Action::Move { dx, dy });
        }
    }
    if is_key_pressed(KeyCode::S) {
        adventure.act(campaign::Action::Search);
    } else if is_key_pressed(KeyCode::W) {
        adventure.act(campaign::Action::WaitTick);
    } else if is_key_pressed(KeyCode::C) {
        // The key does what the sidebar offers; where it offers nothing, nothing is asked.
        if let Some(id) = adventure.contract_offer().map(str::to_owned) {
            adventure.act(campaign::Action::TakeContract { id });
        }
    } else {
        let keys = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4];
        let sold: Vec<String> = adventure.on_sale().iter().map(|s| s.0.to_owned()).collect();
        let in_town = adventure.expedition.hero() == adventure.expedition.def().town;
        for (troop_id, key) in sold.into_iter().zip(keys) {
            if in_town && is_key_pressed(key) {
                let quantity = 1;
                return adventure.act(campaign::Action::Recruit { troop_id, quantity });
            }
        }
    }
}

fn draw_adventure(adventure: &Adventure) {
    let expedition = &adventure.expedition;
    let (Some(battle), Some(band)) = (expedition.battle(), expedition.engaged_with()) else {
        return map::draw(adventure);
    };
    let seed = expedition
        .def()
        .encounter(band)
        .map_or(0, |e| e.battle_rng_state);
    let shown = view::Shown {
        scenario: &words::place_name(band),
        seed,
        cursor: adventure.cursor,
        log: &adventure.log,
        in_expedition: true,
    };
    view::draw(battle, &shown);
}

/// What is played: one battle, or the expedition.
enum Play {
    Battle(Box<Game>),
    Expedition(Box<Adventure>),
}

struct Options {
    language: Option<String>,
    shot: Option<String>,
    /// Commands were played before the first frame: a developer's run, which leaves the user's files alone.
    stepped: bool,
}

async fn run(options: Options, mut play: Play) {
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
    if let Play::Expedition(adventure) = &mut play
        && options.shot.is_none()
        && !options.stepped
    {
        // Only now, with the languages set, can a continued game say that it is one.
        let def = Arc::clone(adventure.expedition.def_shared());
        let saves = Saves::of_the_user(&def);
        **adventure = Adventure::new(def, saves);
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
            match &mut play {
                Play::Battle(game) => {
                    if options.shot.is_none() {
                        game.update();
                    }
                    game.draw();
                }
                Play::Expedition(adventure) => {
                    if options.shot.is_none() {
                        update_adventure(adventure);
                    }
                    draw_adventure(adventure);
                }
            }
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

/// The expedition after the first `steps` commands of a route its file carries, battles' commands counted too;
/// past the route's end, the state it ends in.
fn expedition_after(
    def: &Arc<ExpeditionDef>,
    route: &str,
    steps: u32,
) -> Result<Expedition, String> {
    let ids = || def.test_routes.iter().filter_map(|r| r["id"].as_str());
    let Some(found) = def.test_routes.iter().find(|r| r["id"] == route) else {
        let names: Vec<&str> = ids().collect();
        return Err(format!(
            "no route {route:?}; there are: {}",
            names.join(", ")
        ));
    };
    if steps == 0 {
        return Ok(Expedition::start(Arc::clone(def)));
    }
    let (mut taken, mut at) = (0, None);
    let mut watch = |expedition: &mut Expedition| {
        taken += 1;
        if taken == steps {
            at = Some(expedition.clone());
        }
    };
    let (end, _) = bounty_tools::expedition::play_route(def, found, &mut watch)?;
    Ok(at.unwrap_or(end))
}

/// Reads the command line; a message for the user when something on it cannot be used.
fn start() -> Result<(Options, Play), String> {
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
    let steps = number("--steps")?.unwrap_or(0);
    let options = Options {
        language: dev::arg_after("--lang"),
        shot,
        stepped: steps > 0,
    };
    let (rules, scenario) = (dev::arg_after("--rules"), dev::arg_after("--scenario"));
    let expedition = dev::arg_after("--expedition");
    // An expedition when one is named or nothing is; one battle when only rules or a battle are named.
    let play = if expedition.is_none() && (rules.is_some() || scenario.is_some()) {
        let rules = content::rules_named(&rules.unwrap_or_else(|| "prototype".to_owned()))?;
        let id = scenario.unwrap_or_else(|| content::first_scenario(&rules).to_owned());
        let scenario = content::scenario(&rules, &id).map_err(|e| e.to_string())?;
        let seed = number("--seed")?.unwrap_or(scenario.rng_state);
        let mut game = Game::new(Arc::new(rules), scenario, seed);
        for _ in 0..steps {
            game.pilot();
        }
        Play::Battle(Box::new(game))
    } else {
        let name = expedition.unwrap_or_else(|| "demo".to_owned());
        let def = content::expedition_named(&name, rules.as_deref())?;
        // The first route the expedition's file carries, unless one is named.
        let first = def
            .test_routes
            .first()
            .and_then(|route| route["id"].as_str());
        let route =
            dev::arg_after("--route").unwrap_or_else(|| first.unwrap_or_default().to_owned());
        let expedition = expedition_after(&def, &route, steps)?;
        debug_assert!(expedition.status() != Status::Battle || expedition.battle().is_some());
        Play::Expedition(Box::new(Adventure::at(expedition, Saves(None))))
    };
    Ok((options, play))
}

fn main() {
    match start() {
        Ok((options, play)) => {
            macroquad::Window::from_config(screen::window("Bounty"), run(options, play));
        }
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    }
}
