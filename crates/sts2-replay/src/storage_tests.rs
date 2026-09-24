//! Synthetic storage cases, independent of any recorded game.

use std::fmt::Write as _;
use std::io::{Cursor, Read, Write};

use flate2::{Compression, write::GzEncoder};
use serde_json::{Value, json};

use crate::{Event, Parser, Record, ReplayErrorKind, canonical, decode_input};

fn observe(sequence: u64, state: &Value) -> Record {
    let mut payload = json!({"digest":canonical::digest(state).unwrap(),
        "projection_version":1, "after":sequence-1, "context":"action.settled",
        "base":"extension", "patch":{"unknown":true}});
    payload["state"] = state.clone();
    Record {
        sequence,
        kind: "state.observe".into(),
        payload,
    }
}

fn delta(record: &Record, base: u64, patch: Value) -> Record {
    let mut observation = record.payload.clone();
    observation.as_object_mut().unwrap().remove("state");
    let mut payload = json!({"base":base});
    payload["observation"] = observation;
    payload["patch"] = patch;
    Record {
        sequence: record.sequence,
        kind: "state.delta".into(),
        payload,
    }
}

fn text(version: u8, records: &[Record]) -> String {
    let mut output = format!("[Format \"STS2PGN\"]\n[FormatVersion \"{version}\"]\n\n");
    for record in records {
        writeln!(
            output,
            "{} {} {}",
            record.sequence, record.kind, record.payload
        )
        .unwrap();
    }
    output
}

fn parse(input: impl AsRef<[u8]>) -> Result<Vec<Event>, crate::ReplayError> {
    Parser::new(decode_input(input.as_ref()).unwrap())?.collect()
}

fn gzip(input: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(input).unwrap();
    encoder.finish().unwrap()
}

#[test]
fn full_delta_and_gzip_preserve_all_logical_records() {
    let before = json!({"stable":{"hp":72},"/":{"~":{"":1}},"items":[1,2],"delete":null});
    let after = json!({"stable":{"hp":72},"/":{"~":{"":2}},"items":[3],"added":null});
    let first = observe(1, &before);
    let second = observe(3, &after);
    let action = Record {
        sequence: 2,
        kind: "event.advance".into(),
        payload: json!({"data":{}}),
    };
    let patch = json!([
        ["set", ["/", "~", ""], 2],
        ["set", ["items"], [3]],
        ["remove", ["delete"]],
        ["set", ["added"], null]
    ]);
    let records = [first.clone(), action.clone(), second.clone()];
    let encoded = text(2, &[first, action, delta(&second, 1, patch)]);
    let expected: Vec<_> = records.iter().cloned().map(Event::Record).collect();
    for input in [
        text(1, &records).into_bytes(),
        text(2, &records).into_bytes(),
        encoded.as_bytes().to_vec(),
        gzip(encoded.as_bytes()),
    ] {
        assert_eq!(parse(input).unwrap(), expected);
    }
    let mut parser = Parser::new(Cursor::new(encoded)).unwrap();
    let headers = parser.headers().to_vec();
    let records = parser.by_ref().collect::<Result<Vec<_>, _>>().unwrap();
    let mut expanded = Vec::new();
    crate::write_stream(&mut expanded, &headers, records).unwrap();
    assert_eq!(parse(expanded).unwrap(), expected);
}

#[test]
fn bad_deltas_are_errors_even_at_eof_and_fuse_the_parser() {
    let first = observe(1, &json!({"object":{"keep":1},"array":[1]}));
    let valid = delta(&observe(2, &first.payload["state"]), 1, json!([]));
    let mut bad_cases = Vec::new();
    for patch in [
        json!([["set", [], 0]]),
        json!([["set", ["object", "keep", "x"], 0]]),
        json!([["set", ["missing", "x"], 0]]),
        json!([["remove", ["absent"]]]),
        json!([["set", ["array", "0"], 0]]),
        json!([["set", [1], 0]]),
        json!([["move", ["object"], 0]]),
        json!([["remove", ["object"], 0]]),
    ] {
        let mut bad = valid.clone();
        bad.payload["patch"] = patch;
        bad_cases.push((bad, ReplayErrorKind::InvalidDelta));
    }
    for base in [0, 2, 100] {
        let mut bad = valid.clone();
        bad.payload["base"] = json!(base);
        bad_cases.push((bad, ReplayErrorKind::InvalidDeltaBase));
    }
    let mut bad = valid.clone();
    bad.payload["observation"]["digest"] = json!("sha256:bad");
    bad_cases.push((bad, ReplayErrorKind::DigestMismatch));
    let mut bad = valid.clone();
    bad.payload["observation"]["state"] = json!({});
    bad_cases.push((bad, ReplayErrorKind::InvalidDelta));
    let mut bad = valid;
    bad.payload["extra"] = json!(true);
    bad_cases.push((bad, ReplayErrorKind::InvalidDelta));
    for (bad, kind) in bad_cases {
        let input = text(2, &[first.clone(), bad]);
        for input in [input.as_str(), input.trim_end()] {
            let mut parser = Parser::new(Cursor::new(input)).unwrap();
            parser.next().unwrap().unwrap();
            let error = parser.next().unwrap().unwrap_err();
            assert_eq!(error.kind, kind, "{input}");
            assert_eq!(error.line, 5);
            assert!(parser.next().is_none());
        }
    }
}

#[test]
fn lifecycle_boundaries_require_a_new_full_checkpoint() {
    for kind in ["run.start", "run.resume", "room.enter", "act.enter"] {
        let first = observe(1, &json!({"run":{"reloads":0}}));
        let boundary = Record {
            sequence: 2,
            kind: kind.into(),
            payload: json!({}),
        };
        let second = observe(3, &json!({"run":{"reloads":1}}));
        let bad = delta(&second, 1, json!([["set", ["run", "reloads"], 1]]));
        assert_eq!(
            parse(text(2, &[first.clone(), boundary.clone(), bad]))
                .unwrap_err()
                .kind,
            ReplayErrorKind::InvalidDeltaBase
        );
        let third = delta(&observe(4, &second.payload["state"]), 3, json!([]));
        assert_eq!(
            parse(text(2, &[first, boundary, second, third]))
                .unwrap()
                .len(),
            4
        );
        let mut checkpoint = observe(2, &json!({}));
        checkpoint.payload["context"] = json!(kind);
        assert_eq!(
            parse(text(
                2,
                &[observe(1, &json!({})), delta(&checkpoint, 1, json!([]))]
            ))
            .unwrap_err()
            .kind,
            ReplayErrorKind::InvalidDelta
        );
    }
}

#[test]
fn chain_bases_are_previous_observations_and_are_not_limited_by_writer_cadence() {
    let state = json!({"unchanged":true});
    let mut records = vec![observe(1, &state)];
    for sequence in 2..=70 {
        records.push(delta(&observe(sequence, &state), sequence - 1, json!([])));
    }
    assert_eq!(parse(text(2, &records)).unwrap().len(), 70);
    records[69].payload["base"] = json!(1);
    assert_eq!(
        parse(text(2, &records)).unwrap_err().kind,
        ReplayErrorKind::InvalidDeltaBase
    );
}

#[test]
fn lossless_numbers_unknown_keys_and_unicode_survive_a_delta_and_rewrite() {
    let state = crate::json::parse(r#"{"n":0.1234567890123456789012345678,"e":1E-07,"big":18446744073709551616,"$serde_json::private::Number":"ordinary","text":"é\"😀"}"#,1).unwrap();
    let full = observe(1, &state);
    let mut changed = state.clone();
    changed["e"] = crate::json::parse("1E-08", 1).unwrap();
    changed["n"] = crate::json::parse("0.1234567890123456789012345679", 1).unwrap();
    let patch = crate::json::parse(
        r#"[["set",["e"],1E-08],["set",["n"],0.1234567890123456789012345679]]"#,
        1,
    )
    .unwrap();
    let input = text(2, &[full, delta(&observe(2, &changed), 1, patch)]);
    let mut parser = Parser::new(Cursor::new(input)).unwrap();
    let headers = parser.headers().to_vec();
    let events = parser.by_ref().collect::<Result<Vec<_>, _>>().unwrap();
    let Event::Record(record) = &events[1] else {
        panic!("observation")
    };
    assert_eq!(record.payload["state"], changed);
    crate::validate_observation(record).unwrap();
    let mut output = Vec::new();
    crate::write_stream(&mut output, &headers, events.clone()).unwrap();
    assert_eq!(parse(output).unwrap(), events);
}

#[test]
fn version_gate_and_full_checkpoint_digest_are_enforced() {
    assert_eq!(
        parse("[FormatVersion \"3\"]\n\n").unwrap_err().kind,
        ReplayErrorKind::UnsupportedFormat
    );
    assert_eq!(
        parse("[FormatVersion \"1\"]\n[FormatVersion \"2\"]\n\n")
            .unwrap_err()
            .kind,
        ReplayErrorKind::DuplicateHeader
    );
    let mut first = observe(1, &json!({}));
    assert_eq!(
        parse(text(1, &[delta(&first, 0, json!([]))]))
            .unwrap_err()
            .kind,
        ReplayErrorKind::UnsupportedFormat
    );
    assert_eq!(
        parse(text(2, &[delta(&first, 0, json!([]))]))
            .unwrap_err()
            .kind,
        ReplayErrorKind::InvalidDeltaBase
    );
    first.payload["state"] = json!({"corrupt":true});
    assert_eq!(
        parse(text(2, &[first])).unwrap_err().kind,
        ReplayErrorKind::DigestMismatch
    );
}

#[test]
fn interrupted_json_tail_is_ignored_but_complete_corruption_is_rejected() {
    let prefix = text(2, &[observe(1, &json!({}))]);
    for tail in ["2 state.delta {", "2 state.delta {\"base\":1,\"patch\":["] {
        assert_eq!(parse(format!("{prefix}{tail}")).unwrap().len(), 1);
        assert_eq!(
            parse(format!("{prefix}{tail}\n")).unwrap_err().kind,
            ReplayErrorKind::InvalidJson
        );
    }
    assert_eq!(
        parse(format!("{prefix}2 state.delta {{}}"))
            .unwrap_err()
            .kind,
        ReplayErrorKind::InvalidDelta
    );
}

#[test]
fn gzip_magic_short_reads_concatenated_members_and_corruption() {
    struct Short<R>(R);
    impl<R: Read> Read for Short<R> {
        fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
            let count = output.len().min(1);
            self.0.read(&mut output[..count])
        }
    }
    let input = text(2, &[observe(1, &json!({}))]);
    let split = input.len() / 2;
    let mut compressed = gzip(&input.as_bytes()[..split]);
    compressed.extend(gzip(&input.as_bytes()[split..]));
    let decoded = decode_input(Short(Cursor::new(compressed.clone()))).unwrap();
    assert_eq!(Parser::new(decoded).unwrap().count(), 1);
    compressed.pop();
    assert_eq!(parse(compressed).unwrap_err().kind, ReplayErrorKind::Io);
    let mut corrupt = gzip(input.as_bytes());
    let crc = corrupt.len() - 8;
    corrupt[crc] ^= 1;
    assert_eq!(parse(corrupt).unwrap_err().kind, ReplayErrorKind::Io);
}
