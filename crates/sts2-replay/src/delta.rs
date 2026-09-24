//! Format v2 storage decoding. The baseline is recorded state, never simulator state.

use serde_json::{Map, Value};

use crate::{Record, ReplayError, ReplayErrorKind};

#[derive(Default)]
pub(super) struct Decoder {
    baseline: Option<(u64, Value)>,
}

impl Decoder {
    pub(super) fn decode(
        &mut self,
        mut record: Record,
        line: usize,
    ) -> Result<Record, ReplayError> {
        if matches!(
            record.kind.as_str(),
            "run.start" | "run.resume" | "room.enter" | "act.enter"
        ) {
            self.baseline = None;
        }
        if record.kind == "state.delta" {
            let bad = || ReplayError::new(line, ReplayErrorKind::InvalidDelta);
            let payload = record.payload.as_object().ok_or_else(bad)?;
            let base = payload
                .get("base")
                .and_then(Value::as_u64)
                .ok_or_else(bad)?;
            let (_, previous) = self
                .baseline
                .as_ref()
                .filter(|(sequence, _)| *sequence == base && base < record.sequence)
                .ok_or_else(|| ReplayError::new(line, ReplayErrorKind::InvalidDeltaBase))?;
            let observation = payload
                .get("observation")
                .and_then(Value::as_object)
                .ok_or_else(bad)?;
            let patch = payload
                .get("patch")
                .and_then(Value::as_array)
                .ok_or_else(bad)?;
            if payload.len() != 3 || observation.contains_key("state") || is_checkpoint(observation)
            {
                return Err(bad());
            }
            let mut state = previous.clone();
            for operation in patch {
                apply(&mut state, operation).map_err(|()| bad())?;
            }
            let mut expanded = observation.clone();
            expanded.insert("state".to_owned(), state);
            "state.observe".clone_into(&mut record.kind);
            record.payload = Value::Object(expanded);
        }
        if record.kind == "state.observe" {
            crate::canonical::validate(&record).map_err(|mut error| {
                error.line = line;
                error
            })?;
            self.baseline = Some((record.sequence, record.payload["state"].clone()));
        }
        Ok(record)
    }
}

fn is_checkpoint(observation: &Map<String, Value>) -> bool {
    matches!(
        observation.get("context").and_then(Value::as_str),
        Some("run.start" | "run.resume" | "room.enter" | "act.enter")
    )
}

fn apply(state: &mut Value, operation: &Value) -> Result<(), ()> {
    let operation = operation.as_array().ok_or(())?;
    let kind = operation.first().and_then(Value::as_str).ok_or(())?;
    if !matches!((kind, operation.len()), ("set", 3) | ("remove", 2)) {
        return Err(());
    }
    let path = operation[1]
        .as_array()
        .filter(|path| !path.is_empty())
        .ok_or(())?;
    let mut parent = state;
    for part in &path[..path.len() - 1] {
        parent = parent
            .as_object_mut()
            .ok_or(())?
            .get_mut(part.as_str().ok_or(())?)
            .ok_or(())?;
    }
    let parent = parent.as_object_mut().ok_or(())?;
    let key = path.last().and_then(Value::as_str).ok_or(())?;
    if kind == "set" {
        parent.insert(key.to_owned(), operation[2].clone());
    } else if parent.remove(key).is_none() {
        return Err(());
    }
    Ok(())
}
