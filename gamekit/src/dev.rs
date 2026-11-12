//! Aids for developers: reading the command line and saving a screenshot of the canvas, so that a screen can be
//! checked in every language without a person looking at a window.

use std::path::Path;

use macroquad::prelude::get_screen_data;

use crate::screen::canvas_of;

/// The command line value after `flag`, as in `--scenario melee_exchange`.
#[must_use]
pub fn arg_after(flag: &str) -> Option<String> {
    value_after(&std::env::args().collect::<Vec<_>>(), flag)
}

/// Whether `flag` is on the command line.
#[must_use]
pub fn has_flag(flag: &str) -> bool {
    std::env::args().any(|arg| arg == flag)
}

fn value_after(args: &[String], flag: &str) -> Option<String> {
    let at = args.iter().position(|arg| arg == flag)?;
    args.get(at + 1).cloned()
}

/// Why a screenshot can't be saved to `path`, if it can't: the name must end in `.png` and its folder must exist.
/// Asked before the window opens, since a failure later would only show as a crash.
#[must_use]
pub fn shot_problem(path: &Path) -> Option<&'static str> {
    let png = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("png"));
    let folder = path.parent().filter(|p| !p.as_os_str().is_empty());
    if !png {
        Some("the file needs a name that ends in .png, like shot.png")
    } else if folder.is_some_and(|folder| !folder.is_dir()) {
        Some("the folder doesn't exist")
    } else {
        None
    }
}

/// Saves what is on screen now, cut down to the canvas.
pub fn save_shot(path: &str) {
    canvas_of(&get_screen_data()).export_png(path);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_value_is_what_follows_its_flag() {
        let args: Vec<String> = ["game", "--seed", "7", "--shot"].map(String::from).to_vec();
        assert_eq!(value_after(&args, "--seed").as_deref(), Some("7"));
        assert_eq!(
            value_after(&args, "--shot"),
            None,
            "a flag at the end has no value"
        );
        assert_eq!(value_after(&args, "--lang"), None);
    }

    #[test]
    fn a_screenshot_needs_a_png_name_in_a_folder_that_exists() {
        assert_eq!(shot_problem(Path::new("shot.png")), None);
        assert_eq!(shot_problem(Path::new("SHOT.PNG")), None);
        assert!(
            shot_problem(Path::new("shot.jpg"))
                .unwrap()
                .contains(".png")
        );
        assert!(
            shot_problem(Path::new("no-such-folder-here/shot.png"))
                .unwrap()
                .contains("folder")
        );
    }
}
