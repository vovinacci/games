//! The kit on its own: the dedication on the virtual screen, with a frame around the canvas to show the scaling.
//!
//! `cargo run -p gamekit --example dedication -- [--lang uk] [--shot file.png]`

use gamekit::i18n::{self, Language};
use gamekit::{dedication, dev, screen, text};
use macroquad::prelude::*;

/// A game's own languages; this one has no text of its own, so the files are empty.
static LANGUAGES: [Language; 2] = [
    Language {
        id: "en-GB",
        name: "English",
        source: "",
    },
    Language {
        id: "uk",
        name: "uk",
        source: "",
    },
];

async fn run(shot: Option<String>) {
    i18n::init(&LANGUAGES);
    i18n::set_language(
        dev::arg_after("--lang").map_or_else(i18n::detect, |lang| i18n::place_of(&lang)),
    );
    let mut frame = 0_u32;
    loop {
        frame += 1;
        clear_background(BLACK);
        set_camera(&screen::camera());
        draw_rectangle_lines(
            1.0,
            1.0,
            screen::WIDTH - 2.0,
            screen::HEIGHT - 2.0,
            2.0,
            DARKGRAY,
        );
        dedication::draw(screen::HEIGHT / 2.0 - 10.0, 1.0);
        let size = format!("{} x {}", screen::WIDTH, screen::HEIGHT);
        text::text_at(&size, 16.0, screen::HEIGHT - 16.0, 16.0, DARKGRAY);
        if let Some(path) = &shot
            && frame == 30
        {
            dev::save_shot(path);
            break;
        }
        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        next_frame().await;
    }
}

fn main() {
    let shot = dev::arg_after("--shot");
    if let Some(path) = &shot
        && let Some(problem) = dev::shot_problem(std::path::Path::new(path))
    {
        eprintln!("--shot {path}: {problem}");
        std::process::exit(2);
    }
    macroquad::Window::from_config(screen::window("Gamekit"), run(shot));
}
