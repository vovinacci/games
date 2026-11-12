//! `bounty-cjson-v1`: one fixed way to write a tree of values as bytes, so that a state has exactly one digest
//! whatever machine or build wrote it. A cut-down JSON: no floats, object keys in byte order, no spaces. The rules
//! are in `docs/spec/SERIALIZATION.md`; `prototype/canonical_vectors.json` holds the examples the tests check.

use std::collections::BTreeMap;
use std::fmt;

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use sha2::{Digest, Sha256};

/// A value that can be written canonically.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Null,
    Bool(bool),
    /// From -2^63 to 2^64-1; anything outside is refused when written.
    Int(i128),
    Str(String),
    /// Order is kept: sort before building the value when the order carries no meaning.
    Array(Vec<Self>),
    /// Written in the byte order of the keys, which is the order the map already keeps.
    Object(BTreeMap<String, Self>),
}

/// Why a value could not be read or written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CanonError {
    /// The text is not JSON, or holds a float or the same key twice.
    Parse(String),
    /// An integer below -2^63 or above 2^64-1.
    IntOutOfRange(i128),
    /// An object key with a character outside ASCII.
    NonAsciiKey(String),
}

impl fmt::Display for CanonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(why) => write!(f, "not canonical JSON: {why}"),
            Self::IntOutOfRange(n) => write!(f, "integer {n} is outside -2^63..=2^64-1"),
            Self::NonAsciiKey(key) => write!(f, "object key {key:?} is not ASCII"),
        }
    }
}

impl std::error::Error for CanonError {}

impl Value {
    /// An object from pairs, for building states by hand.
    #[must_use]
    pub fn object<const N: usize>(pairs: [(&str, Self); N]) -> Self {
        Self::Object(
            pairs
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
        )
    }

    /// A string value.
    #[must_use]
    pub fn str(text: &str) -> Self {
        Self::Str(text.to_owned())
    }

    /// Reads JSON text strictly: a float or a repeated key is an error, not a guess.
    ///
    /// # Errors
    /// [`CanonError::Parse`] with the reader's message.
    pub fn parse(text: &str) -> Result<Self, CanonError> {
        serde_json::from_str(text).map_err(|e| CanonError::Parse(e.to_string()))
    }

    /// The canonical bytes.
    ///
    /// # Errors
    /// An integer out of range or a key that is not ASCII, anywhere in the tree.
    pub fn to_bytes(&self) -> Result<Vec<u8>, CanonError> {
        let mut out = String::new();
        self.write(&mut out)?;
        Ok(out.into_bytes())
    }

    /// Lowercase hex SHA-256 of the canonical bytes.
    ///
    /// # Errors
    /// As [`Value::to_bytes`].
    pub fn digest(&self) -> Result<String, CanonError> {
        let hash = Sha256::digest(self.to_bytes()?);
        let mut hex = String::with_capacity(64);
        for byte in hash {
            hex.push(hex_digit(byte >> 4));
            hex.push(hex_digit(byte));
        }
        Ok(hex)
    }

    fn write(&self, out: &mut String) -> Result<(), CanonError> {
        match self {
            Self::Null => out.push_str("null"),
            Self::Bool(true) => out.push_str("true"),
            Self::Bool(false) => out.push_str("false"),
            Self::Int(n) => {
                if *n < i128::from(i64::MIN) || *n > i128::from(u64::MAX) {
                    return Err(CanonError::IntOutOfRange(*n));
                }
                out.push_str(&n.to_string());
            }
            Self::Str(text) => write_string(text, out),
            Self::Array(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write(out)?;
                }
                out.push(']');
            }
            Self::Object(members) => {
                out.push('{');
                for (i, (key, value)) in members.iter().enumerate() {
                    if !key.is_ascii() {
                        return Err(CanonError::NonAsciiKey(key.clone()));
                    }
                    if i > 0 {
                        out.push(',');
                    }
                    write_string(key, out);
                    out.push(':');
                    value.write(out)?;
                }
                out.push('}');
            }
        }
        Ok(())
    }
}

fn hex_digit(nibble: u8) -> char {
    char::from(b"0123456789abcdef"[usize::from(nibble & 15)])
}

/// Quote and backslash are escaped, control characters become `\u00xx`, everything else stands as it is.
fn write_string(text: &str, out: &mut String) {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            ch if u32::from(ch) < 0x20 => {
                out.push_str("\\u00");
                out.push(hex_digit(ch as u8 >> 4));
                out.push(hex_digit(ch as u8 & 15));
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<u32> for Value {
    fn from(value: u32) -> Self {
        Self::Int(i128::from(value))
    }
}

impl From<u64> for Value {
    fn from(value: u64) -> Self {
        Self::Int(i128::from(value))
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Strict;

        impl<'de> Visitor<'de> for Strict {
            type Value = Value;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("null, a boolean, an integer, a string, an array or an object")
            }

            fn visit_unit<E>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
                Ok(Value::Bool(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
                Ok(Value::Int(i128::from(value)))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Value, E> {
                Ok(Value::Int(i128::from(value)))
            }

            fn visit_f64<E: de::Error>(self, value: f64) -> Result<Value, E> {
                // The reader hands "-0" over as a float; the format reads it as plain zero.
                if value == 0.0 {
                    return Ok(Value::Int(0));
                }
                Err(E::custom(format!(
                    "{value} is not an integer in -2^63..=2^64-1"
                )))
            }

            fn visit_str<E>(self, value: &str) -> Result<Value, E> {
                Ok(Value::Str(value.to_owned()))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
                let mut items = Vec::new();
                while let Some(item) = seq.next_element()? {
                    items.push(item);
                }
                Ok(Value::Array(items))
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
                let mut members = BTreeMap::new();
                while let Some((key, value)) = map.next_entry::<String, Value>()? {
                    if members.contains_key(&key) {
                        return Err(de::Error::custom(format!("key {key:?} appears twice")));
                    }
                    members.insert(key, value);
                }
                Ok(Value::Object(members))
            }
        }

        deserializer.deserialize_any(Strict)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_recorded_vectors_encode_and_hash_as_recorded() {
        let file = Value::parse(include_str!("../../../prototype/canonical_vectors.json")).unwrap();
        let Value::Object(file) = file else {
            panic!("vectors file is an object")
        };
        let Value::Array(vectors) = &file["vectors"] else {
            panic!("vectors is an array")
        };
        assert_eq!(vectors.len(), 5);
        for vector in vectors {
            let Value::Object(vector) = vector else {
                panic!("a vector is an object")
            };
            let (Value::Str(id), Value::Str(bytes), Value::Str(hash)) =
                (&vector["id"], &vector["canonical_utf8"], &vector["sha256"])
            else {
                panic!("id, canonical_utf8 and sha256 are strings")
            };
            let value = &vector["value"];
            assert_eq!(
                &String::from_utf8(value.to_bytes().unwrap()).unwrap(),
                bytes,
                "{id}"
            );
            assert_eq!(&value.digest().unwrap(), hash, "{id}");
        }
    }

    #[test]
    fn what_the_format_refuses() {
        assert!(matches!(
            Value::parse(r#"{"a":1,"a":2}"#),
            Err(CanonError::Parse(_))
        ));
        assert!(matches!(Value::parse("1.5"), Err(CanonError::Parse(_))));
        assert!(matches!(
            Value::parse("18446744073709551616"),
            Err(CanonError::Parse(_))
        ));
        assert!(matches!(
            Value::parse(r#""\ud800""#),
            Err(CanonError::Parse(_))
        ));
        let key = "\u{43a}"; // a Cyrillic letter, written as an escape to keep this file ASCII
        assert_eq!(
            Value::object([(key, Value::Null)]).to_bytes(),
            Err(CanonError::NonAsciiKey(key.to_owned()))
        );
        let too_big = i128::from(u64::MAX) + 1;
        assert_eq!(
            Value::Int(too_big).to_bytes(),
            Err(CanonError::IntOutOfRange(too_big))
        );
    }

    #[test]
    fn negative_zero_reads_as_zero() {
        assert_eq!(
            Value::parse("-0").map(|v| v.to_bytes()),
            Ok(Ok(b"0".to_vec()))
        );
    }
}
