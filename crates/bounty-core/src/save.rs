//! Saving an expedition and reading it back.
//!
//! A save is an envelope (`docs/spec/SERIALIZATION.md`): the whole state as one canonical value, the payload, with
//! its SHA-256 beside it, the version of the save's shape, and what rules and content it was played with. Reading
//! is strict and never guesses: a file that is damaged, of another version, for other content, or that holds a
//! state the rules could not have produced is refused with the reason, and nothing is half loaded.
//!
//! This module turns states into text and back. It opens no file; where a save lives and how it replaces the one
//! before is the application's business (`gamekit::files`).

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use crate::campaign::{Expedition, ExpeditionDef};
use crate::canon::Value;

/// The version of the envelope's shape that this code writes and reads.
pub const SCHEMA_VERSION: u64 = 1;

/// The encoding of the payload.
pub const FORMAT: &str = "bounty-cjson-v1";

/// Why a save was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveError {
    /// Not a save at all: not the JSON a save is, or a part of the envelope is missing.
    Malformed(String),
    /// A save of another version of the shape. Newer saves are not guessed at, older ones not silently upgraded.
    UnsupportedVersion(u64),
    /// The payload is not what its checksum says: the file was damaged or changed by hand.
    Damaged,
    /// A sound save, of other rules or other content than the game now has.
    OtherContent(String),
    /// A sound save for this content whose state the rules could not have produced.
    Invalid(String),
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed(why) => write!(f, "not a save: {why}"),
            Self::UnsupportedVersion(version) => {
                write!(
                    f,
                    "a save of version {version}; this game reads version {SCHEMA_VERSION}"
                )
            }
            Self::Damaged => {
                f.write_str("the save is damaged: its content does not match its checksum")
            }
            Self::OtherContent(why) => write!(f, "the save is for other content: {why}"),
            Self::Invalid(why) => write!(f, "the save holds a state that cannot be: {why}"),
        }
    }
}

impl std::error::Error for SaveError {}

pub(crate) fn invalid<T>(why: impl Into<String>) -> Result<T, SaveError> {
    Err(SaveError::Invalid(why.into()))
}

/// Reads the members of an object by name, each as the kind it must be. A member that is missing or of another
/// kind is an invalid state, named with the object it belongs to.
pub(crate) struct Reader<'a> {
    members: &'a BTreeMap<String, Value>,
    /// What the object is, for messages.
    what: &'a str,
}

impl<'a> Reader<'a> {
    pub(crate) fn of(value: &'a Value, what: &'a str) -> Result<Self, SaveError> {
        match value {
            Value::Object(members) => Ok(Self { members, what }),
            _ => invalid(format!("{what} is not an object")),
        }
    }

    fn get(&self, key: &str) -> Result<&'a Value, SaveError> {
        self.members
            .get(key)
            .map_or_else(|| invalid(format!("{} has no {key}", self.what)), Ok)
    }

    fn wrong<T>(&self, key: &str, kind: &str) -> Result<T, SaveError> {
        invalid(format!("{}.{key} is not {kind}", self.what))
    }

    /// The member, or `None` where it is absent or null.
    pub(crate) fn opt(&self, key: &str) -> Option<&'a Value> {
        self.members.get(key).filter(|value| **value != Value::Null)
    }

    pub(crate) fn u64(&self, key: &str) -> Result<u64, SaveError> {
        match self.get(key)? {
            Value::Int(n) => {
                u64::try_from(*n).or_else(|_| self.wrong(key, "a whole number from 0"))
            }
            _ => self.wrong(key, "a number"),
        }
    }

    pub(crate) fn u32(&self, key: &str) -> Result<u32, SaveError> {
        u32::try_from(self.u64(key)?).or_else(|_| self.wrong(key, "a number that small"))
    }

    pub(crate) fn text(&self, key: &str) -> Result<&'a str, SaveError> {
        match self.get(key)? {
            Value::Str(text) => Ok(text),
            _ => self.wrong(key, "text"),
        }
    }

    pub(crate) fn flag(&self, key: &str) -> Result<bool, SaveError> {
        match self.get(key)? {
            Value::Bool(flag) => Ok(*flag),
            _ => self.wrong(key, "yes or no"),
        }
    }

    pub(crate) fn list(&self, key: &str) -> Result<&'a [Value], SaveError> {
        match self.get(key)? {
            Value::Array(items) => Ok(items),
            _ => self.wrong(key, "a list"),
        }
    }

    /// A list of texts, such as IDs.
    pub(crate) fn texts(&self, key: &str) -> Result<Vec<String>, SaveError> {
        let text = |item: &Value| match item {
            Value::Str(text) => Ok(text.clone()),
            _ => self.wrong(key, "a list of texts"),
        };
        self.list(key)?.iter().map(text).collect()
    }

    pub(crate) fn object(&self, key: &'a str) -> Result<Self, SaveError> {
        Self::of(self.get(key)?, key)
    }

    /// The members of an object that maps names to numbers.
    pub(crate) fn numbers(&self, key: &str) -> Result<BTreeMap<String, u32>, SaveError> {
        let Value::Object(members) = self.get(key)? else {
            return self.wrong(key, "an object");
        };
        let number = |(name, value): (&String, &Value)| match value {
            Value::Int(n) => u32::try_from(*n)
                .map(|n| (name.clone(), n))
                .or_else(|_| self.wrong(key, "small numbers")),
            _ => self.wrong(key, "numbers"),
        };
        members.iter().map(number).collect()
    }
}

/// The part of a ruleset's ID after "-v", which is its version: "0.1" of "prototype-v0.1".
fn ruleset_version(id: &str) -> &str {
    id.rsplit_once("-v").map_or("", |(_, version)| version)
}

/// The text of a save of this expedition. The battle being fought, if one is, is saved with it, so a game can be
/// closed in the middle of a battle.
#[must_use]
pub fn encode(expedition: &Expedition) -> String {
    let def = expedition.def();
    // The battle sits in the payload beside the campaign, as the envelope's schema has it.
    let mut campaign = expedition.to_canon();
    let battle = match &mut campaign {
        Value::Object(members) => members.remove("battle").unwrap_or(Value::Null),
        _ => Value::Null,
    };
    let payload = Value::object([
        ("battle", battle),
        ("campaign", campaign),
        ("content_sha256", Value::str(&def.content_sha256)),
        // The battle's random state is in the battle; the campaign draws nothing yet, and no computer player
        // keeps anything between commands. Both are here so that a later version has a place for them.
        ("policy_state", Value::object([])),
        ("rng", Value::object([])),
        ("ruleset_id", Value::str(&def.rules.id)),
        (
            "ruleset_version",
            Value::str(ruleset_version(&def.rules.id)),
        ),
    ]);
    // Every key is ASCII and every number in range, so neither encoding can fail.
    let sha256 = payload.digest().unwrap_or_default();
    let envelope = Value::object([
        ("canonical_format", Value::str(FORMAT)),
        ("payload", payload),
        ("save_schema_version", SCHEMA_VERSION.into()),
        ("sha256", Value::Str(sha256)),
    ]);
    let mut text = String::from_utf8(envelope.to_bytes().unwrap_or_default()).unwrap_or_default();
    text.push('\n');
    text
}

/// Reads a save of an expedition of `def`.
///
/// # Errors
/// [`SaveError`] saying why the text is not a save this game can go on from. Nothing is loaded in that case.
pub fn decode(text: &str, def: Arc<ExpeditionDef>) -> Result<Expedition, SaveError> {
    let envelope = Value::parse(text).map_err(|e| SaveError::Malformed(e.to_string()))?;
    let malformed = |error: SaveError| match error {
        SaveError::Invalid(why) => SaveError::Malformed(why),
        other => other,
    };
    let outer = Reader::of(&envelope, "the save").map_err(malformed)?;
    let version = outer.u64("save_schema_version").map_err(malformed)?;
    if version != SCHEMA_VERSION {
        return Err(SaveError::UnsupportedVersion(version));
    }
    let format = outer.text("canonical_format").map_err(malformed)?;
    if format != FORMAT {
        return Err(SaveError::Malformed(format!(
            "its payload is encoded as {format:?}, not {FORMAT:?}"
        )));
    }
    let payload = outer.get("payload").map_err(malformed)?;
    let checksum = outer.text("sha256").map_err(malformed)?;
    if payload.digest().ok().as_deref() != Some(checksum) {
        return Err(SaveError::Damaged);
    }

    // From here on the payload is what was written; what remains is whether it is for this game.
    let inner = Reader::of(payload, "the payload").map_err(malformed)?;
    let ruleset = inner.text("ruleset_id").map_err(malformed)?;
    let content = inner.text("content_sha256").map_err(malformed)?;
    if ruleset != def.rules.id {
        return Err(SaveError::OtherContent(format!(
            "it was played under {ruleset:?}, the game has {:?}",
            def.rules.id
        )));
    }
    if inner.text("ruleset_version").map_err(malformed)? != ruleset_version(&def.rules.id) {
        return Err(SaveError::OtherContent(
            "another version of the rules".to_owned(),
        ));
    }
    if content != def.content_sha256 {
        return Err(SaveError::OtherContent(
            "the expedition, its map or its rules have changed since".to_owned(),
        ));
    }
    let campaign = inner.get("campaign").map_err(malformed)?;
    Expedition::from_canon(def, campaign, inner.opt("battle"))
}

#[cfg(test)]
mod tests;
