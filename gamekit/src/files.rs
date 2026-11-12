//! A game's own files in the user's data folder: where they are and how they are written. No window, no sound:
//! usable from tests and tools.

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Where the game `game` keeps the file `name`, if the system names a place: `<data folder>/<game>/<name>`.
#[must_use]
pub fn data_file(game: &str, name: &str) -> Option<PathBuf> {
    data_folder(|variable| env::var_os(variable).map(PathBuf::from))
        .map(|folder| folder.join(game).join(name))
}

/// The folder for a program's own data, by the custom of the system. A variable that is empty or names no absolute
/// path counts as not set. `var` reads an environment variable; tests give their own.
fn data_folder(var: impl Fn(&str) -> Option<PathBuf>) -> Option<PathBuf> {
    let folder = |name: &str| var(name).filter(|path| path.is_absolute());
    if cfg!(target_os = "windows") {
        folder("APPDATA")
    } else if cfg!(target_os = "macos") {
        folder("HOME").map(|home| home.join("Library").join("Application Support"))
    } else {
        folder("XDG_DATA_HOME").or_else(|| folder("HOME").map(|home| home.join(".local/share")))
    }
}

/// Writes `text` as the whole content of `file`, creating its folder. The text goes into a file beside it first,
/// which is then moved over it, so that a crash half way through leaves the earlier content whole.
///
/// # Errors
/// The folder cannot be made, or the file cannot be written or moved. The earlier content is then still there.
pub fn write_whole(file: &Path, text: &str) -> io::Result<()> {
    if let Some(folder) = file.parent() {
        fs::create_dir_all(folder)?;
    }
    let draft = file.with_extension("tmp");
    fs::write(&draft, text)?;
    fs::rename(&draft, file)
}

/// The content of `file`, or `None` when there is no such file or it cannot be read as text.
#[must_use]
pub fn read_whole(file: &Path) -> Option<String> {
    fs::read_to_string(file).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of this test's own under the build's temporary folder, empty at the start.
    fn scratch(name: &str) -> PathBuf {
        let folder = env::temp_dir().join(format!("gamekit-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        folder
    }

    #[test]
    fn a_file_is_written_whole_and_read_back() {
        let folder = scratch("write");
        let file = folder.join("deep").join("settings.txt");
        assert_eq!(read_whole(&file), None);
        write_whole(&file, "one").unwrap();
        write_whole(&file, "two").unwrap();
        assert_eq!(read_whole(&file).as_deref(), Some("two"));
        assert!(
            !file.with_extension("tmp").exists(),
            "the draft was moved, not left behind"
        );
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn a_write_that_fails_says_so_and_keeps_the_old_content() {
        let folder = scratch("fail");
        let file = folder.join("settings.txt");
        write_whole(&file, "kept").unwrap();
        // A folder where the draft should go makes the write fail.
        fs::create_dir(file.with_extension("tmp")).unwrap();
        assert!(write_whole(&file, "lost").is_err());
        assert_eq!(read_whole(&file).as_deref(), Some("kept"));
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn the_data_folder_follows_the_system_and_ignores_paths_that_are_not_absolute() {
        let with = |pairs: &'static [(&'static str, &'static str)]| {
            data_folder(move |name| {
                pairs
                    .iter()
                    .find(|(key, _)| *key == name)
                    .map(|(_, value)| PathBuf::from(value))
            })
        };
        assert_eq!(with(&[]), None);
        if cfg!(target_os = "macos") {
            assert_eq!(
                with(&[("HOME", "/Users/x")]),
                Some(PathBuf::from("/Users/x/Library/Application Support"))
            );
            assert_eq!(with(&[("HOME", "relative")]), None);
        } else if cfg!(target_os = "windows") {
            assert_eq!(with(&[("APPDATA", "relative")]), None);
        } else {
            assert_eq!(
                with(&[("XDG_DATA_HOME", "/data"), ("HOME", "/home/x")]),
                Some(PathBuf::from("/data"))
            );
            assert_eq!(
                with(&[("XDG_DATA_HOME", ""), ("HOME", "/home/x")]),
                Some(PathBuf::from("/home/x/.local/share"))
            );
        }
    }
}
