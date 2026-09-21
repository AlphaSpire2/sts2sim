use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::{Record, ReplayError, ReplayErrorKind};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimelineRole {
    InitiatingAction,
    NestedChoice,
    Resolve,
    Lifecycle,
    Observation,
    Finish,
}

#[must_use]
pub fn record_role(kind: &str) -> TimelineRole {
    if kind == "state.observe" {
        TimelineRole::Observation
    } else if kind == "run.finish" {
        TimelineRole::Finish
    } else if kind.starts_with("choice.") {
        TimelineRole::NestedChoice
    } else if kind.ends_with(".resolve") {
        TimelineRole::Resolve
    } else if matches!(
        kind,
        "room.enter"
            | "room.exit"
            | "reward.settled"
            | "combat.turn.start"
            | "combat.turn.end"
            // Entering an act is announced after the act transition settles;
            // the act was already walked into by `act.next`.
            | "act.enter"
            // A run picked back up off its save file. Nobody chose it, and the
            // observations around it are the load's own stages rather than
            // states the run settles on.
            | "run.resume"
    ) {
        TimelineRole::Lifecycle
    } else {
        TimelineRole::InitiatingAction
    }
}

/// Incremental role/correlation validation above the simulator.
#[derive(Clone, Debug, Default)]
pub struct TimelineValidator {
    initiating: BTreeSet<u64>,
    resolved: BTreeSet<u64>,
    last_input: Option<u64>,
    last_observed_after: Option<u64>,
    late_observations: usize,
}

impl TimelineValidator {
    pub fn accept(&mut self, record: &Record) -> Result<TimelineRole, ReplayError> {
        let role = record_role(&record.kind);
        match role {
            TimelineRole::InitiatingAction | TimelineRole::NestedChoice => {
                self.initiating.insert(record.sequence);
                self.last_input = Some(record.sequence);
            }
            TimelineRole::Resolve => {
                let initiating = record
                    .payload
                    .pointer("/data/action_sequence")
                    .or_else(|| record.payload.get("action_sequence"))
                    .and_then(Value::as_u64)
                    .ok_or_else(|| ReplayError::new(0, ReplayErrorKind::InvalidCorrelation))?;
                if !self.initiating.contains(&initiating) || !self.resolved.insert(initiating) {
                    return Err(ReplayError::new(0, ReplayErrorKind::InvalidCorrelation));
                }
            }
            TimelineRole::Observation => {
                let after = record
                    .payload
                    .get("after")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| ReplayError::new(0, ReplayErrorKind::InvalidCorrelation))?;
                // An observation names the record it was taken after, and that
                // record is always behind it. It is not always the latest one:
                // the recorder writes observations from a queue of its own, so
                // one may be written late, naming an action two rooms back.
                // The state it carries is the state at the moment it was
                // written, so a late correlation is the recorder's ordering
                // rather than a malformed record.
                if after >= record.sequence {
                    return Err(ReplayError::new(0, ReplayErrorKind::InvalidCorrelation));
                }
                if self.last_observed_after.is_some_and(|old| after < old) {
                    self.late_observations += 1;
                } else {
                    self.last_observed_after = Some(after);
                }
            }
            TimelineRole::Lifecycle | TimelineRole::Finish => {}
        }
        Ok(role)
    }

    #[must_use]
    pub const fn last_input(&self) -> Option<u64> {
        self.last_input
    }

    /// How many observations named an action earlier than one an observation
    /// before them already named: the recorder writing one late.
    #[must_use]
    pub const fn late_observations(&self) -> usize {
        self.late_observations
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldDifference {
    pub pointer: String,
    pub expected: Option<Value>,
    pub actual: Option<Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FirstDivergence {
    pub fixture_id: String,
    pub compatibility_id: String,
    pub record_sequence: u64,
    pub initiating_action: Option<Value>,
    pub last_matching_checkpoint: Option<u64>,
    pub expected_digest: Option<String>,
    pub actual_digest: Option<String>,
    pub differences: Vec<FieldDifference>,
    pub rng_counter_deltas: BTreeMap<String, i64>,
    pub queue_context: Option<String>,
    pub choice_context: Option<String>,
    pub compatibility_warnings: Vec<String>,
}

/// Projection v1 comparison. Declared identity-keyed collections are matched
/// by key; every other array retains gameplay order.
#[must_use]
pub fn compare_projection(expected: &Value, actual: &Value) -> Vec<FieldDifference> {
    let mut differences = Vec::new();
    compare_at("", None, expected, actual, &mut differences);
    differences
}

fn compare_at(
    pointer: &str,
    field_name: Option<&str>,
    expected: &Value,
    actual: &Value,
    differences: &mut Vec<FieldDifference>,
) {
    match (expected, actual) {
        (Value::Object(expected), Value::Object(actual)) => {
            let keys: BTreeSet<_> = expected.keys().chain(actual.keys()).collect();
            for key in keys {
                let child_pointer = format!("{pointer}/{}", escape_pointer(key));
                match (expected.get(key), actual.get(key)) {
                    (Some(expected), Some(actual)) => {
                        compare_at(&child_pointer, Some(key), expected, actual, differences);
                    }
                    (expected, actual) => differences.push(FieldDifference {
                        pointer: child_pointer,
                        expected: expected.cloned(),
                        actual: actual.cloned(),
                    }),
                }
            }
        }
        (Value::Array(expected), Value::Array(actual)) => {
            if let Some(identity) = field_name.and_then(identity_field) {
                compare_keyed_arrays(pointer, identity, expected, actual, differences);
            } else {
                let length = expected.len().max(actual.len());
                for index in 0..length {
                    let child_pointer = format!("{pointer}/{index}");
                    match (expected.get(index), actual.get(index)) {
                        (Some(expected), Some(actual)) => {
                            compare_at(&child_pointer, None, expected, actual, differences);
                        }
                        (expected, actual) => differences.push(FieldDifference {
                            pointer: child_pointer,
                            expected: expected.cloned(),
                            actual: actual.cloned(),
                        }),
                    }
                }
            }
        }
        _ if expected != actual => differences.push(FieldDifference {
            pointer: pointer.to_owned(),
            expected: Some(expected.clone()),
            actual: Some(actual.clone()),
        }),
        _ => {}
    }
}

fn compare_keyed_arrays(
    pointer: &str,
    identity: &str,
    expected: &[Value],
    actual: &[Value],
    differences: &mut Vec<FieldDifference>,
) {
    let expected = keyed(expected, identity);
    let actual = keyed(actual, identity);
    let keys: BTreeSet<_> = expected.keys().chain(actual.keys()).collect();
    for key in keys {
        let child_pointer = format!("{pointer}/@{identity}={}", escape_pointer(key));
        match (expected.get(key), actual.get(key)) {
            (Some(expected), Some(actual)) => {
                compare_at(&child_pointer, None, expected, actual, differences);
            }
            (expected, actual) => differences.push(FieldDifference {
                pointer: child_pointer,
                expected: expected.copied().cloned(),
                actual: actual.copied().cloned(),
            }),
        }
    }
}

fn keyed<'a>(values: &'a [Value], identity: &str) -> BTreeMap<String, &'a Value> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let key = value
                .get(identity)
                .map_or_else(|| format!("#missing-{index}"), value_key);
            (key, value)
        })
        .collect()
}

fn value_key(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        _ => value.to_string(),
    }
}

fn identity_field(name: &str) -> Option<&'static str> {
    match name {
        "acts" => Some("index"),
        "players" => Some("slot"),
        "rooms" => Some("stack_depth"),
        "player_choices" | "rewards" | "points" => Some("id"),
        "creatures" => Some("combat_id"),
        _ => None,
    }
}

fn escape_pointer(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn keyed_arrays_compare_by_identity_but_other_arrays_are_ordered() {
        let expected = json!({
            "players": [{"slot":"p0","hp":10},{"slot":"p1","hp":8}],
            "hand": ["a","b"]
        });
        let actual = json!({
            "players": [{"slot":"p1","hp":8},{"slot":"p0","hp":9}],
            "hand": ["b","a"]
        });
        let differences = compare_projection(&expected, &actual);
        assert_eq!(differences.len(), 3);
        assert_eq!(differences[0].pointer, "/hand/0");
        assert_eq!(differences[2].pointer, "/players/@slot=p0/hp");
    }

    #[test]
    fn missing_and_null_are_distinct() {
        let differences = compare_projection(&json!({"x":null}), &json!({}));
        assert_eq!(differences[0].expected, Some(Value::Null));
        assert_eq!(differences[0].actual, None);
    }

    #[test]
    fn resolve_must_reference_one_unresolved_input() {
        let mut validator = TimelineValidator::default();
        validator
            .accept(&Record {
                sequence: 4,
                kind: "reward.claim".to_owned(),
                payload: json!({}),
            })
            .unwrap();
        let resolve = Record {
            sequence: 5,
            kind: "reward.resolve".to_owned(),
            payload: json!({"data":{"action_sequence":4}}),
        };
        assert_eq!(validator.accept(&resolve).unwrap(), TimelineRole::Resolve);
        assert_eq!(
            validator.accept(&resolve).unwrap_err().kind,
            ReplayErrorKind::InvalidCorrelation
        );
    }
}
