//! What the game remembers between runs: the language and whether the dedication has been shown. Kept as a few
//! `name = value` lines in the game's folder of the user's data folder. A file that is missing or cannot be read is
//! the same as a first run; a setting that cannot be saved is a convenience lost, never a reason to stop the game.

use std::path::PathBuf;

use gamekit::files;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    /// The ID of the chosen language, `None` until one is chosen: then the system's language is used.
    pub language: Option<String>,
    pub dedication_seen: bool,
}

impl Settings {
    /// Reads the lines of a settings file. Lines it does not know are skipped, so an older game reads a newer file.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let mut settings = Self::default();
        for (name, value) in text.lines().filter_map(|line| line.split_once('=')) {
            match (name.trim(), value.trim()) {
                ("language", id) if !id.is_empty() => settings.language = Some(id.to_owned()),
                ("dedication_seen", seen) => settings.dedication_seen = seen == "yes",
                _ => {}
            }
        }
        settings
    }

    #[must_use]
    pub fn to_text(&self) -> String {
        let language = self
            .language
            .as_ref()
            .map_or_else(String::new, |id| format!("language = {id}\n"));
        let seen = if self.dedication_seen { "yes" } else { "no" };
        format!("{language}dedication_seen = {seen}\n")
    }
}

/// Where the settings are kept; `None` in a developer run that must not touch the user's files.
pub struct Store(pub Option<PathBuf>);

impl Store {
    /// The user's own file.
    #[must_use]
    pub fn of_the_user() -> Self {
        Self(files::data_file("bounty", "settings.txt"))
    }

    #[must_use]
    pub fn load(&self) -> Settings {
        self.0
            .as_deref()
            .and_then(files::read_whole)
            .map_or_else(Settings::default, |text| Settings::parse(&text))
    }

    pub fn save(&self, settings: &Settings) {
        if let Some(file) = &self.0
            && let Err(error) = files::write_whole(file, &settings.to_text())
        {
            eprintln!("settings not saved to {}: {error}", file.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_survive_being_written_and_read() {
        let settings = Settings {
            language: Some("uk".to_owned()),
            dedication_seen: true,
        };
        assert_eq!(Settings::parse(&settings.to_text()), settings);
        assert_eq!(
            Settings::parse(&Settings::default().to_text()),
            Settings::default()
        );
    }

    #[test]
    fn a_file_with_unknown_or_broken_lines_still_reads() {
        let text = "volume = 7\nnonsense\nlanguage = uk\ndedication_seen = maybe\nlanguage =\n";
        assert_eq!(
            Settings::parse(text),
            Settings {
                language: Some("uk".to_owned()),
                dedication_seen: false
            }
        );
        assert_eq!(Settings::parse(""), Settings::default());
    }

    #[test]
    fn a_store_keeps_settings_in_its_file_and_nowhere_when_it_has_none() {
        let file = std::env::temp_dir()
            .join(format!("bounty-test-{}", std::process::id()))
            .join("settings.txt");
        let store = Store(Some(file.clone()));
        assert_eq!(
            store.load(),
            Settings::default(),
            "no file yet: a first run"
        );
        let settings = Settings {
            language: Some("uk".to_owned()),
            dedication_seen: true,
        };
        store.save(&settings);
        assert_eq!(store.load(), settings);
        std::fs::remove_dir_all(file.parent().unwrap()).unwrap();

        let nowhere = Store(None);
        nowhere.save(&settings);
        assert_eq!(nowhere.load(), Settings::default());
    }
}
