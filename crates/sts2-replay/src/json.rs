//! Lossless JSON values, including numeric tokens and unknown object keys.

use std::collections::BTreeSet;
use std::fmt::{self, Formatter};

use serde::de::{Deserialize, Deserializer, MapAccess, Visitor};
use serde_json::value::RawValue;
use serde_json::{Map, Number, Value};

use crate::{ReplayError, ReplayErrorKind};

pub(super) fn parse(input: &str, line: usize) -> Result<Value, ReplayError> {
    parse_at(input, line, 0)
}

fn parse_at(input: &str, line: usize, depth: usize) -> Result<Value, ReplayError> {
    if depth >= 256 {
        return Err(ReplayError::new(line, ReplayErrorKind::InvalidJson));
    }
    let raw: &RawValue =
        serde_json::from_str(input).map_err(|error| ReplayError::json(line, error))?;
    let raw = raw.get();
    match raw.as_bytes()[0] {
        b'{' => {
            let members: Members =
                serde_json::from_str(raw).map_err(|error| ReplayError::json(line, error))?;
            let mut values = Map::new();
            for (key, value) in members.0 {
                values.insert(key, parse_at(value.get(), line, depth + 1)?);
            }
            Ok(Value::Object(values))
        }
        b'[' => {
            let items: Vec<Box<RawValue>> =
                serde_json::from_str(raw).map_err(|error| ReplayError::json(line, error))?;
            items
                .iter()
                .map(|item| parse_at(item.get(), line, depth + 1))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array)
        }
        b'-' | b'0'..=b'9' => {
            // RawValue has validated the token's syntax. Preserve it exactly:
            // f64 loses decimal precision; Number::from_str rewrites exponents.
            Ok(Value::Number(Number::from_string_unchecked(raw.to_owned())))
        }
        _ => serde_json::from_str(raw).map_err(|error| ReplayError::json(line, error)),
    }
}

struct Members(Vec<(String, Box<RawValue>)>);

impl<'de> Deserialize<'de> for Members {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(MembersVisitor)
    }
}

struct MembersVisitor;

impl<'de> Visitor<'de> for MembersVisitor {
    type Value = Members;

    fn expecting(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("an object without duplicate keys")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut object: A) -> Result<Members, A::Error> {
        let mut seen = BTreeSet::new();
        let mut members = Vec::new();
        while let Some((key, value)) = object.next_entry::<String, Box<RawValue>>()? {
            if !seen.insert(key.clone()) {
                return Err(serde::de::Error::custom(format!(
                    "duplicate JSON key {key:?}"
                )));
            }
            members.push((key, value));
        }
        Ok(Members(members))
    }
}
