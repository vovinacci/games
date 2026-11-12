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

/// The file beside `file` whose name is that of `file` with `.suffix` after it.
fn beside(file: &Path, suffix: &str) -> PathBuf {
    let mut name = file.file_name().unwrap_or_default().to_owned();
    name.push(".");
    name.push(suffix);
    file.with_file_name(name)
}

/// Where [`write_keeping_previous`] keeps the content `file` had before the last write.
#[must_use]
pub fn previous_of(file: &Path) -> PathBuf {
    beside(file, "previous")
}

/// Writes `text` as the whole content of `file` and keeps what the file held before as its previous version
/// ([`previous_of`]), for a file whose loss would cost the player something: a saved game. `valid` says whether a
/// text is one the game can read.
///
/// The text goes into a draft, which is read back and compared before anything else is touched. Then the current
/// file becomes the previous one, and the draft the current one. Stopped at any point, this leaves a valid file
/// for [`read_newest`] to find. A current file that is not valid never replaces a valid previous one: it is moved
/// aside as `<name>.damaged`, its bytes untouched.
///
/// # Errors
/// The text is not valid, did not read back as written, or a file could not be written or moved. `Ok` is returned
/// only once `file` holds `text`.
pub fn write_keeping_previous(
    file: &Path,
    text: &str,
    valid: impl Fn(&str) -> bool,
) -> io::Result<()> {
    if let Some(folder) = file.parent() {
        fs::create_dir_all(folder)?;
    }
    let draft = beside(file, "tmp");
    fs::write(&draft, text)?;
    if !valid(text) || fs::read_to_string(&draft).ok().as_deref() != Some(text) {
        let _ = fs::remove_file(&draft);
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "the text did not read back as a valid file",
        ));
    }
    if file.exists() {
        let good = read_whole(file).is_some_and(|current| valid(&current));
        let aside = if good {
            previous_of(file)
        } else {
            beside(file, "damaged")
        };
        fs::rename(file, aside)?;
    }
    fs::rename(&draft, file)
}

/// What `read` makes of `file`, or failing that of its previous version, with whether it is the previous one.
/// `read` gives `None` for a text the game cannot use. `None` when neither file can be used; both are left as
/// they are.
#[must_use]
pub fn read_newest<T>(file: &Path, read: impl Fn(&str) -> Option<T>) -> Option<(T, bool)> {
    let from = |file: &Path| read_whole(file).and_then(|text| read(&text));
    from(file)
        .map(|value| (value, false))
        .or_else(|| from(&previous_of(file)).map(|value| (value, true)))
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

    /// A text counts as valid here when it starts with "ok".
    fn ok(text: &str) -> bool {
        text.starts_with("ok")
    }

    fn newest(file: &Path) -> Option<(String, bool)> {
        read_newest(file, |text| ok(text).then(|| text.to_owned()))
    }

    #[test]
    fn a_kept_write_leaves_the_content_before_it_as_the_previous_one() {
        let folder = scratch("keep");
        let file = folder.join("save.json");
        assert_eq!(newest(&file), None);
        write_keeping_previous(&file, "ok 1", ok).unwrap();
        assert_eq!(newest(&file), Some(("ok 1".to_owned(), false)));
        assert!(!previous_of(&file).exists());
        write_keeping_previous(&file, "ok 2", ok).unwrap();
        write_keeping_previous(&file, "ok 3", ok).unwrap();
        assert_eq!(read_whole(&file).as_deref(), Some("ok 3"));
        assert_eq!(read_whole(&previous_of(&file)).as_deref(), Some("ok 2"));
        assert!(!beside(&file, "tmp").exists());
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn a_text_that_is_not_valid_is_not_written_and_success_is_not_reported() {
        let folder = scratch("invalid");
        let file = folder.join("save.json");
        write_keeping_previous(&file, "ok 1", ok).unwrap();
        write_keeping_previous(&file, "ok 2", ok).unwrap();
        assert!(write_keeping_previous(&file, "bad", ok).is_err());
        assert_eq!(read_whole(&file).as_deref(), Some("ok 2"));
        assert_eq!(read_whole(&previous_of(&file)).as_deref(), Some("ok 1"));
        assert!(!beside(&file, "tmp").exists());
        // The draft cannot be written: an error, and both files as they were.
        fs::create_dir(beside(&file, "tmp")).unwrap();
        assert!(write_keeping_previous(&file, "ok 3", ok).is_err());
        assert_eq!(newest(&file), Some(("ok 2".to_owned(), false)));
        fs::remove_dir_all(folder).unwrap();
    }

    // The write stopped after each of its steps: a valid file is always found, and the next write succeeds.
    #[test]
    fn a_write_stopped_at_any_step_leaves_a_valid_file() {
        let folder = scratch("stopped");
        let file = folder.join("save.json");
        let (draft, previous) = (beside(&file, "tmp"), previous_of(&file));
        write_keeping_previous(&file, "ok 1", ok).unwrap();
        write_keeping_previous(&file, "ok 2", ok).unwrap();

        // Stopped half way through the draft.
        fs::write(&draft, "ok").unwrap();
        assert_eq!(newest(&file), Some(("ok 2".to_owned(), false)));
        // Stopped with the draft whole.
        fs::write(&draft, "ok 3").unwrap();
        assert_eq!(newest(&file), Some(("ok 2".to_owned(), false)));
        // Stopped after the current file became the previous one: the draft is not yet the save.
        fs::rename(&file, &previous).unwrap();
        assert_eq!(newest(&file), Some(("ok 2".to_owned(), true)));

        write_keeping_previous(&file, "ok 4", ok).unwrap();
        assert_eq!(newest(&file), Some(("ok 4".to_owned(), false)));
        assert_eq!(read_whole(&previous).as_deref(), Some("ok 2"));
        assert!(!draft.exists());
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn a_damaged_file_is_set_aside_whole_and_never_replaces_the_previous_one() {
        let folder = scratch("damaged");
        let file = folder.join("save.json");
        write_keeping_previous(&file, "ok 1", ok).unwrap();
        write_keeping_previous(&file, "ok 2", ok).unwrap();
        fs::write(&file, "torn").unwrap();
        assert_eq!(newest(&file), Some(("ok 1".to_owned(), true)));
        assert_eq!(
            read_whole(&file).as_deref(),
            Some("torn"),
            "reading changes nothing"
        );

        write_keeping_previous(&file, "ok 3", ok).unwrap();
        assert_eq!(newest(&file), Some(("ok 3".to_owned(), false)));
        assert_eq!(read_whole(&previous_of(&file)).as_deref(), Some("ok 1"));
        assert_eq!(
            read_whole(&beside(&file, "damaged")).as_deref(),
            Some("torn")
        );

        fs::write(&file, "torn").unwrap();
        fs::write(previous_of(&file), "torn too").unwrap();
        assert_eq!(newest(&file), None);
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
