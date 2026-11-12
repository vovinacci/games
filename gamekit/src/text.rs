//! Text on the virtual screen, in the one font all games share (Scorch Mono, `assets/fonts/README.md`).

use std::cell::Cell;

use macroquad::prelude::*;

use crate::screen::{WIDTH, scale};

thread_local! {
    static SHADOW: Cell<bool> = const { Cell::new(false) };
    /// Leaked on purpose: dropping it at exit would free its texture after the window's GL context is gone
    /// (segfault).
    static FONT: &'static Font = Box::leak(Box::new(
        load_ttf_font_from_bytes(include_bytes!("../assets/fonts/ScorchMono.ttf")).expect("bundled font loads"),
    ));
}

/// Draws text with its baseline at `y`. Glyphs are rasterised at the real window resolution and then scaled back,
/// so text stays sharp however the game is scaled.
pub fn text_at(text: impl AsRef<str>, x: f32, y: f32, size: f32, color: Color) {
    let text = text.as_ref();
    let s = scale();
    FONT.with(|&font| {
        let params = |color| TextParams {
            font: Some(font),
            font_size: (size * s).round() as u16,
            font_scale: 1.0 / s,
            color,
            ..Default::default()
        };
        if SHADOW.get() {
            // Dark outline, about 1/16 of the text size: readable over any background.
            let d = (size / 16.0).max(1.0 / s);
            let dark = Color::new(0.0, 0.0, 0.0, 0.8 * color.a);
            for (dx, dy) in [(-d, 0.0), (d, 0.0), (0.0, -d), (0.0, d), (d, d)] {
                draw_text_ex(text, x + dx, y + dy, params(dark));
            }
        }
        draw_text_ex(text, x, y, params(color));
    });
}

/// Draws everything in `f` with outlined text, for text that can sit on any colour.
pub fn with_shadow(f: impl FnOnce()) {
    SHADOW.set(true);
    f();
    SHADOW.set(false);
}

/// Measures text the same way [`text_at`] draws it.
#[must_use]
pub fn measure(text: impl AsRef<str>, size: f32) -> TextDimensions {
    let s = scale();
    FONT.with(|&font| {
        measure_text(
            text.as_ref(),
            Some(font),
            (size * s).round() as u16,
            1.0 / s,
        )
    })
}

/// Largest text size up to `size`, and not below 12, at which `text` fits in `width`.
fn fit(text: &str, mut size: f32, width: f32) -> (f32, TextDimensions) {
    let mut dims = measure(text, size);
    while dims.width > width && size > 12.0 {
        size -= 1.0;
        dims = measure(text, size);
    }
    (size, dims)
}

/// Draws text at `x, y`, shrinking it if needed to fit within `max_w`. Translations run longer than English: give
/// every text with a fixed place either this or [`text_wrapped`].
pub fn text_fit(text: &str, x: f32, y: f32, max_w: f32, size: f32, color: Color) {
    let (size, _) = fit(text, size, max_w);
    text_at(text, x, y, size, color);
}

/// Centred text; shrinks to stay inside the screen.
pub fn text_centered(text: &str, cx: f32, y: f32, size: f32, color: Color) {
    text_centered_fit(text, cx, y, WIDTH - 32.0, size, color);
}

/// Centred text, shrunk if needed to fit in `max_w`.
pub fn text_centered_fit(text: &str, cx: f32, y: f32, max_w: f32, size: f32, color: Color) {
    let (size, dims) = fit(text, size, max_w);
    text_at(text, cx - dims.width / 2.0, y, size, color);
}

/// Draws word-wrapped text and returns the y just below the last line.
#[expect(
    clippy::must_use_candidate,
    reason = "drawing is the point; the height is for those who stack text"
)]
pub fn text_wrapped(text: &str, x: f32, mut y: f32, max_w: f32, size: f32, color: Color) -> f32 {
    for paragraph in text.split('\n') {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if !line.is_empty() && measure(&candidate, size).width > max_w {
                text_at(&line, x, y, size, color);
                y += size * 1.25;
                word.clone_into(&mut line);
            } else {
                line = candidate;
            }
        }
        text_at(&line, x, y, size, color);
        y += size * 1.25;
    }
    y
}
