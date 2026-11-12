//! The virtual screen. A game lays everything out on a fixed canvas of [`WIDTH`] by [`HEIGHT`]; [`camera`] scales it
//! to the largest area of the real window with the same shape, with black bands where the window is wider or taller.

use macroquad::prelude::*;

pub const WIDTH: f32 = 1280.0;
pub const HEIGHT: f32 = 720.0;

/// The window a game opens: the size of the virtual screen, resizable.
#[must_use]
pub fn window(title: &str) -> Conf {
    Conf {
        window_title: title.to_owned(),
        window_width: WIDTH as i32,
        window_height: HEIGHT as i32,
        window_resizable: true,
        ..Default::default()
    }
}

/// Camera that maps the virtual screen onto the largest centred area of the window with the same aspect ratio.
#[must_use]
pub fn camera() -> Camera2D {
    let scale = scale();
    let (w, h) = (WIDTH * scale, HEIGHT * scale);
    Camera2D {
        target: vec2(WIDTH / 2.0, HEIGHT / 2.0),
        // Positive y zoom = y grows downward, like screen coordinates.
        zoom: vec2(2.0 / WIDTH, 2.0 / HEIGHT),
        viewport: Some((
            ((screen_width() - w) / 2.0) as i32,
            ((screen_height() - h) / 2.0) as i32,
            w as i32,
            h as i32,
        )),
        ..Default::default()
    }
}

/// How many real pixels one virtual pixel takes.
#[must_use]
pub fn scale() -> f32 {
    (screen_width() / WIDTH).min(screen_height() / HEIGHT)
}

/// Mouse position in virtual screen coordinates. In the bands around the canvas it lies outside 0..WIDTH or
/// 0..HEIGHT; [`on_canvas`] tells.
#[must_use]
pub fn mouse() -> Vec2 {
    let (x, y) = mouse_position();
    camera().screen_to_world(vec2(x, y))
}

/// Whether a point in virtual coordinates is on the canvas, not in the bands around it.
#[must_use]
pub fn on_canvas(point: Vec2) -> bool {
    (0.0..WIDTH).contains(&point.x) && (0.0..HEIGHT).contains(&point.y)
}

/// The canvas part of a screenshot, without the bands around it, so shots look the same in any window.
#[must_use]
pub fn canvas_of(shot: &Image) -> Image {
    let (w, h) = (f32::from(shot.width), f32::from(shot.height));
    let scale = (w / WIDTH).min(h / HEIGHT);
    let (cw, ch) = ((WIDTH * scale).floor(), (HEIGHT * scale).floor());
    shot.sub_image(Rect::new(
        ((w - cw) / 2.0).floor(),
        ((h - ch) / 2.0).floor(),
        cw,
        ch,
    ))
}
