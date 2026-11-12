//! The dedication every game here carries. The words are in the kit's own translation files and are the same in
//! every game; a game shows them once on its first run ([`Intro`]) and in its About screen ([`draw`]).

use macroquad::prelude::*;

use crate::screen::{HEIGHT, WIDTH};
use crate::t;
use crate::text::text_centered;

/// How long the first-run screen stays, in seconds.
const INTRO_TIME: f32 = 3.5;

/// "In memory of ...", the years, and why, centred on `y` (the years line), at this opacity.
pub fn draw(y: f32, alpha: f32) {
    let cx = WIDTH / 2.0;
    let color = |c: Color| Color { a: alpha, ..c };
    text_centered(&t!("dedication-1"), cx, y - 38.0, 30.0, color(WHITE));
    text_centered(&t!("dedication-years"), cx, y, 22.0, color(GRAY));
    text_centered(&t!("dedication-2"), cx, y + 42.0, 24.0, color(LIGHTGRAY));
}

/// The first-run screen: the dedication fades in and out; any key or click skips it.
#[derive(Clone, Copy, Debug, Default)]
pub struct Intro {
    time: f32,
}

impl Intro {
    /// Draws one frame. Returns `false` once the screen is over.
    pub fn show(&mut self) -> bool {
        // The first frames after start-up can take long; don't let them eat the intro.
        self.time += get_frame_time().min(0.05);
        let skip = get_last_key_pressed().is_some() || is_mouse_button_pressed(MouseButton::Left);
        if self.time >= INTRO_TIME || (skip && self.time > 0.2) {
            return false;
        }
        let fade = (self.time / 0.6)
            .min((INTRO_TIME - self.time) / 0.6)
            .clamp(0.0, 1.0);
        draw(HEIGHT / 2.0 - 10.0, fade);
        true
    }
}
