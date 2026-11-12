//! Translations. Every piece of text on screen comes from a Fluent file compiled into the game, looked up by key:
//!
//! ```ignore
//! t!("menu-new-game")
//! t!("stack-count", n = stack.count)
//! ```
//!
//! A game hands its languages to [`init`] once at start-up; the first is the default and the fallback. The kit has a
//! few texts of its own (the dedication), in English and Ukrainian, found after the game's. Fluent handles plurals
//! per language (Ukrainian has three forms).
//!
//! A game keeps one short test of its own, built from [`missing_keys`] and [`keys_used_in`], so that no language
//! lacks a key and no key used in the code is missing.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::path::Path;

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use unic_langid::LanguageIdentifier;

/// One language of a game.
pub struct Language {
    /// As in the file name: `en-GB`, `uk`.
    pub id: &'static str,
    /// What the language calls itself, for the menu where it is chosen.
    pub name: &'static str,
    /// The Fluent file, usually `include_str!("../assets/i18n/<id>.ftl")`.
    pub source: &'static str,
}

/// The kit's own texts. English first: it is the fallback.
const KIT: &[Language] = &[
    Language {
        id: "en-GB",
        name: "English",
        source: include_str!("../assets/i18n/en-GB.ftl"),
    },
    // The language's own name is written in the game's table, where non-ASCII text may stand; here it is not shown.
    Language {
        id: "uk",
        name: "uk",
        source: include_str!("../assets/i18n/uk.ftl"),
    },
];

type Bundle = FluentBundle<FluentResource>;

#[derive(Default)]
struct State {
    languages: &'static [Language],
    bundles: Vec<Bundle>,
    current: usize,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::default();
    static KIT_BUNDLES: Vec<Bundle> = KIT.iter().map(bundle).collect();
}

fn bundle(language: &Language) -> Bundle {
    let id: LanguageIdentifier = language.id.parse().expect("valid language id");
    let mut bundle = FluentBundle::new(vec![id]);
    // No invisible Unicode isolation marks around arguments: the font has no glyphs for them.
    bundle.set_use_isolating(false);
    let resource = FluentResource::try_new(language.source.to_owned())
        .unwrap_or_else(|(_, errors)| panic!("bad {}.ftl: {errors:?}", language.id));
    bundle
        .add_resource(resource)
        .unwrap_or_else(|errors| panic!("duplicate keys in {}.ftl: {errors:?}", language.id));
    bundle
}

/// Sets the game's languages; the first is the default and the fallback. Call once, before any text is drawn.
///
/// # Panics
/// A file is not valid Fluent or has a key twice: a mistake to fix before the game ships, so it stops at once.
pub fn init(languages: &'static [Language]) {
    let bundles = languages.iter().map(bundle).collect();
    STATE.set(State {
        languages,
        bundles,
        current: 0,
    });
}

#[must_use]
pub fn language_count() -> usize {
    STATE.with_borrow(|state| state.languages.len())
}

/// Place of the language in use in the game's list.
#[must_use]
pub fn language() -> usize {
    STATE.with_borrow(|state| state.current)
}

/// What the language at `place` calls itself.
#[must_use]
pub fn language_name(place: usize) -> &'static str {
    STATE.with_borrow(|state| {
        state
            .languages
            .get(place)
            .map_or("", |language| language.name)
    })
}

/// Switches all text to the language at `place`; a place outside the list is the first language.
pub fn set_language(place: usize) {
    STATE.with_borrow_mut(|state| {
        state.current = if place < state.languages.len() {
            place
        } else {
            0
        }
    });
}

/// Place of the operating system's language in the game's list, if the game has it, else the first. For the first
/// run; after that the player's choice is kept.
#[must_use]
pub fn detect() -> usize {
    sys_locale::get_locale().map_or(0, |locale| place_of(&locale))
}

/// Place of the language a locale such as `uk-UA` or `en_US.UTF-8` asks for, else the first.
#[must_use]
pub fn place_of(locale: &str) -> usize {
    STATE.with_borrow(|state| {
        state
            .languages
            .iter()
            .position(|l| same_language(l.id, locale))
            .unwrap_or(0)
    })
}

/// The part before the region: `uk` of `uk-UA`, `en` of `en_GB`.
fn base(id: &str) -> String {
    id.split(['-', '_', '.'])
        .next()
        .unwrap_or(id)
        .to_lowercase()
}

fn same_language(a: &str, b: &str) -> bool {
    base(a) == base(b)
}

/// A value passed into a translation: text or a number (numbers pick plurals).
pub enum Arg {
    Text(String),
    Number(f64),
}

impl From<&str> for Arg {
    fn from(v: &str) -> Self {
        Self::Text(v.to_owned())
    }
}

impl From<String> for Arg {
    fn from(v: String) -> Self {
        Self::Text(v)
    }
}

impl From<&String> for Arg {
    fn from(v: &String) -> Self {
        Self::Text(v.clone())
    }
}

macro_rules! number_arg {
    ($($t:ty),+ ; $($wide:ty),+) => {
        $(impl From<$t> for Arg {
            fn from(v: $t) -> Self {
                Self::Number(f64::from(v))
            }
        })+
        // Counts and money, far below 2^53: exact as f64 too.
        $(impl From<$wide> for Arg {
            fn from(v: $wide) -> Self {
                Self::Number(v as f64)
            }
        })+
    };
}
number_arg!(i32, u32, f32, f64; u64, usize);

/// Looks up `key`: in the game's current language, then the game's first, then the kit's texts in the current
/// language, then the kit's English, then the key itself, so that a missing text shows as its key and not as a
/// crash.
#[must_use]
pub fn tr(key: &str, args: &[(&str, Arg)]) -> String {
    let mut fluent_args = FluentArgs::new();
    for (name, value) in args {
        let value = match value {
            Arg::Text(s) => FluentValue::from(s.clone()),
            Arg::Number(n) => FluentValue::from(*n),
        };
        fluent_args.set(*name, value);
    }
    let args = (!args.is_empty()).then_some(&fluent_args);
    let format = |bundle: &Bundle| {
        let pattern = bundle.get_message(key)?.value()?;
        let mut errors = Vec::new();
        Some(
            bundle
                .format_pattern(pattern, args, &mut errors)
                .into_owned(),
        )
    };
    let (from_game, current_id) = STATE.with_borrow(|state| {
        let found = state
            .bundles
            .get(state.current)
            .and_then(format)
            .or_else(|| state.bundles.first().and_then(format));
        (
            found,
            state
                .languages
                .get(state.current)
                .map_or(KIT[0].id, |language| language.id),
        )
    });
    from_game
        .or_else(|| {
            KIT_BUNDLES.with(|kit| {
                let place = KIT
                    .iter()
                    .position(|l| same_language(l.id, current_id))
                    .unwrap_or(0);
                kit.get(place)
                    .and_then(format)
                    .or_else(|| kit.first().and_then(format))
            })
        })
        .unwrap_or_else(|| key.to_owned())
}

/// Translated text: `t!(key)` or `t!(key, name = value, ...)`.
#[macro_export]
macro_rules! t {
    ($key:expr) => {
        $crate::i18n::tr($key, &[])
    };
    ($key:expr, $($name:ident = $value:expr),+ $(,)?) => {
        $crate::i18n::tr($key, &[$((stringify!($name), $crate::i18n::Arg::from($value))),+])
    };
}

/// The keys of a Fluent file.
#[must_use]
pub fn keys(source: &str) -> BTreeSet<String> {
    source
        .lines()
        .filter(|line| line.chars().next().is_some_and(|c| c.is_ascii_alphabetic()))
        .filter_map(|line| line.split_once(" =").map(|(key, _)| key.trim().to_owned()))
        .collect()
}

/// For a game's test: every key that one language has and another lacks, as `"<id> lacks <key>"`. Empty when all
/// languages have the same keys.
#[must_use]
pub fn missing_keys(languages: &[Language]) -> Vec<String> {
    let all: BTreeSet<String> = languages.iter().flat_map(|l| keys(l.source)).collect();
    let mut missing = Vec::new();
    for language in languages {
        let has = keys(language.source);
        missing.extend(
            all.difference(&has)
                .map(|key| format!("{} lacks {key}", language.id)),
        );
    }
    missing
}

/// For a game's test: every key written as `t!("key"...)` in the `.rs` files under `dir`, subfolders included. Keys
/// built at run time are not seen; a game lists those in its own test.
///
/// # Panics
/// `dir` or a file in it cannot be read: the test that calls this should fail then.
#[must_use]
pub fn keys_used_in(dir: &Path) -> BTreeSet<String> {
    let mut used = BTreeSet::new();
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let path = entry
            .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
            .path();
        if path.is_dir() {
            used.extend(keys_used_in(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            let code = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            // A key right after `t!(`, but not after `format!(` or `print!(`.
            for (at, _) in code.match_indices("t!(\"") {
                if !code[..at].ends_with(|c: char| c.is_alphanumeric() || c == '_') {
                    used.extend(code[at + 4..].split('"').next().map(str::to_owned));
                }
            }
        }
    }
    used
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A game's languages for these tests. Ukrainian plural forms are marked x1, x2, x5 to keep this file ASCII.
    static GAME: [Language; 2] = [
        Language {
            id: "en-GB",
            name: "English",
            source: "hello = Hello, { $name }\nonly-english = English only\nwins = { $n ->\n    [one] { $n } win\n   *[other] { $n } wins\n}\n",
        },
        Language {
            id: "uk",
            name: "Ukrainian",
            source: "hello = Pryvit, { $name }\nwins = { $n ->\n    [one] { $n } x1\n    [few] { $n } x2\n   *[many] { $n } x5\n}\n",
        },
    ];

    #[test]
    fn text_comes_from_the_current_language_then_the_first_then_the_kit_then_the_key() {
        init(&GAME);
        assert_eq!(
            (language_count(), language(), language_name(1)),
            (2, 0, "Ukrainian")
        );
        assert_eq!(t!("hello", name = "Ada"), "Hello, Ada");
        set_language(1);
        assert_eq!(t!("hello", name = "Ada"), "Pryvit, Ada");
        assert_eq!(
            t!("only-english"),
            "English only",
            "falls back to the first language"
        );
        assert_eq!(t!("no-such-key"), "no-such-key");
        let english = {
            set_language(0);
            t!("dedication-1")
        };
        assert_eq!(english, "In memory of Joseph Rivkind");
        set_language(1);
        assert_ne!(
            t!("dedication-1"),
            english,
            "the kit's text follows the game's language"
        );
        assert_eq!(t!("dedication-years"), "1952\u{2013}2026");
        set_language(7);
        assert_eq!(
            language(),
            0,
            "a place outside the list is the first language"
        );
    }

    #[test]
    fn numbers_pick_the_plural_form_of_the_language() {
        init(&GAME);
        let forms = |place| {
            set_language(place);
            [1, 3, 5, 21].map(|n| t!("wins", n = n))
        };
        assert_eq!(forms(0), ["1 win", "3 wins", "5 wins", "21 wins"]);
        assert_eq!(forms(1), ["1 x1", "3 x2", "5 x5", "21 x1"]);
    }

    #[test]
    fn the_kit_alone_answers_in_english() {
        // No init on this thread: a tool or test that never set up a game.
        assert_eq!(t!("dedication-1"), "In memory of Joseph Rivkind");
        assert_eq!((language_count(), language(), detect()), (0, 0, 0));
    }

    #[test]
    fn a_locale_finds_its_language_whatever_the_region() {
        init(&GAME);
        assert_eq!(place_of("uk-UA"), 1);
        assert_eq!(place_of("uk_UA.UTF-8"), 1);
        assert_eq!(place_of("en-US"), 0);
        assert_eq!(
            place_of("de-DE"),
            0,
            "a language the game lacks is the first"
        );
    }

    #[test]
    fn the_kits_own_files_have_the_same_keys_and_cover_what_the_kit_uses() {
        assert_eq!(missing_keys(KIT), Vec::<String>::new());
        KIT_BUNDLES.with(|bundles| assert_eq!(bundles.len(), 2));
        let english = keys(KIT[0].source);
        let used = keys_used_in(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src")));
        // Keys of this file's doc example and of the test languages above: used in the kit's code, not kit text.
        let examples = [
            "menu-new-game",
            "stack-count",
            "hello",
            "wins",
            "only-english",
            "no-such-key",
            "key",
        ];
        let known: BTreeSet<String> = english
            .iter()
            .cloned()
            .chain(examples.map(String::from))
            .collect();
        let unknown: Vec<&String> = used.difference(&known).collect();
        assert!(
            unknown.is_empty(),
            "keys used in the kit but not in its en-GB.ftl: {unknown:?}"
        );
        assert!(used.contains("dedication-1"));
    }

    #[test]
    fn a_language_that_lacks_a_key_is_named() {
        assert_eq!(missing_keys(&GAME), ["uk lacks only-english"]);
    }
}
