//! Digest verification for canonical JSON emitted by the C# recorder.
//!
//! Numeric tokens in STS2PGN payloads are already canonicalized by the writer.
//! Keep them intact instead of round-tripping its decimal values through f64.

use std::io::{self, Write};

use serde_json::Value;
use sha2::{Digest as _, Sha256};

use crate::{HashWriter, Record, ReplayError, ReplayErrorKind};

pub(super) fn validate(record: &Record) -> Result<(), ReplayError> {
    if record.kind != "state.observe" {
        return Err(ReplayError::new(0, ReplayErrorKind::NotObservation));
    }
    let payload = &record.payload;
    if payload["projection_version"].as_u64() != Some(1) {
        return Err(ReplayError::new(0, ReplayErrorKind::UnsupportedProjection));
    }
    let state = payload
        .get("state")
        .filter(|state| state.is_object())
        .ok_or_else(|| ReplayError::new(0, ReplayErrorKind::InvalidObservation))?;
    let expected = payload
        .get("digest")
        .and_then(Value::as_str)
        .ok_or_else(|| ReplayError::new(0, ReplayErrorKind::InvalidObservation))?;
    if expected == digest(state)? {
        Ok(())
    } else {
        Err(ReplayError::new(0, ReplayErrorKind::DigestMismatch))
    }
}

pub(super) fn digest(value: &Value) -> Result<String, ReplayError> {
    let mut writer = HashWriter(Sha256::new());
    write(value, &mut writer).map_err(ReplayError::io)?;
    Ok(format!("sha256:{:x}", writer.0.finalize()))
}

fn write(value: &Value, writer: &mut impl Write) -> io::Result<()> {
    match value {
        Value::Null => writer.write_all(b"null"),
        Value::Bool(value) => writer.write_all(if *value { b"true" } else { b"false" }),
        Value::Number(value) => writer.write_all(value.as_str().as_bytes()),
        Value::String(value) => string(value, writer),
        Value::Array(values) => {
            writer.write_all(b"[")?;
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    writer.write_all(b",")?;
                }
                write(value, writer)?;
            }
            writer.write_all(b"]")
        }
        Value::Object(values) => {
            writer.write_all(b"{")?;
            let mut entries: Vec<_> = values.iter().collect();
            // StringComparer.Ordinal sorts UTF-16 code units, not UTF-8 bytes.
            entries.sort_unstable_by(|a, b| a.0.encode_utf16().cmp(b.0.encode_utf16()));
            for (index, (key, value)) in entries.into_iter().enumerate() {
                if index > 0 {
                    writer.write_all(b",")?;
                }
                string(key, writer)?;
                writer.write_all(b":")?;
                write(value, writer)?;
            }
            writer.write_all(b"}")
        }
    }
}

fn string(value: &str, writer: &mut impl Write) -> io::Result<()> {
    writer.write_all(b"\"")?;
    for unit in value.encode_utf16() {
        match unit {
            8 => writer.write_all(b"\\b")?,
            9 => writer.write_all(b"\\t")?,
            10 => writer.write_all(b"\\n")?,
            12 => writer.write_all(b"\\f")?,
            13 => writer.write_all(b"\\r")?,
            92 => writer.write_all(b"\\\\")?,
            32..=126 if !matches!(unit, 34 | 38 | 39 | 43 | 60 | 62 | 96) => {
                writer.write_all(&[u8::try_from(unit).expect("ASCII code unit")])?;
            }
            _ => write!(writer, "\\u{unit:04X}")?,
        }
    }
    writer.write_all(b"\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recorder_encoding_preserves_decimal_tokens_and_utf16_order() {
        let value = crate::json::parse(
            r#"{"\ue000":"é😀","😀":"\"&'+<>`\n\\","n":0.1234567890123456789012345678,"e":1E-07}"#,
            1,
        )
        .unwrap();
        let mut actual = Vec::new();
        write(&value, &mut actual).unwrap();
        // System.Text.Json / CanonicalJson.Serialize output, including G29 exponent.
        let expected = r#"{"e":1E-07,"n":0.1234567890123456789012345678,"\uD83D\uDE00":"\u0022\u0026\u0027\u002B\u003C\u003E\u0060\n\\","\uE000":"\u00E9\uD83D\uDE00"}"#;
        assert_eq!(String::from_utf8(actual).unwrap(), expected);
        assert_eq!(
            digest(&value).unwrap(),
            "sha256:67a63998b90578490c66d74f6fdca2d21c23af27320246a32e0ec363748ae2df"
        );
    }
}
