//! Streaming STS2PGN v1/v2 parsing and Projection v1 canonical digests.

mod adapter;
mod canonical;
mod conformance;
mod delta;
mod input;
mod json;
mod profile;

pub use profile::{ReplayOutcome, ReplayVerification, TraceProfile, VerificationLevel};

#[cfg(test)]
mod storage_tests;

pub use input::decode_input;

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io::{self, BufRead, Write};

use serde_json::Value;
use sha2::{Digest as _, Sha256};
use sts2_core::{CompatibilityManifest, ModIdentity};
use sts2_rng::canonicalize_seed;

pub use adapter::{AdapterError, AdapterErrorCode, AdapterOutcome, ReplayAdapter};
pub mod observation;

pub use conformance::{
    FieldDifference, FirstDivergence, TimelineRole, TimelineValidator, compare_projection,
    record_role,
};

/// One losslessly decoded STS2PGN header. Unknown keys remain in this list.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Header {
    pub key: String,
    pub value: String,
}

/// A numbered record with its complete, extensible JSON payload.
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub sequence: u64,
    pub kind: String,
    pub payload: Value,
}

/// A streaming body item.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Record(Record),
    Terminal(String),
}

/// Streaming parser, retaining headers, one line, and one recorded-state baseline.
/// Version-two deltas are verified and returned as complete state.observe records.
pub struct Parser<R> {
    reader: R,
    headers: Vec<Header>,
    pending: Option<(String, bool)>,
    line_number: usize,
    delta: Option<delta::Decoder>,
    failed: bool,
}

impl<R: BufRead> Parser<R> {
    /// Reads the header block and leaves the first body line pending.
    pub fn new(mut reader: R) -> Result<Self, ReplayError> {
        let mut headers = Vec::new();
        let mut pending = None;
        let mut line_number = 0;
        loop {
            let mut line = String::new();
            let count = reader.read_line(&mut line).map_err(ReplayError::io)?;
            if count == 0 {
                break;
            }
            line_number += 1;
            let complete = line.ends_with('\n');
            trim_line_ending(&mut line);
            if line.is_empty() {
                if headers.is_empty() {
                    continue;
                }
                break;
            }
            if line.starts_with('[') {
                headers.push(parse_header(&line, line_number)?);
            } else {
                pending = Some((line, complete));
                break;
            }
        }
        if headers.is_empty() {
            return Err(ReplayError::new(
                line_number,
                ReplayErrorKind::MissingHeaders,
            ));
        }
        let versions: Vec<_> = headers
            .iter()
            .filter(|header| header.key == "FormatVersion")
            .collect();
        if versions.len() > 1 {
            return Err(ReplayError::new(
                line_number,
                ReplayErrorKind::DuplicateHeader,
            ));
        }
        let delta = match versions.first().map(|header| header.value.as_str()) {
            Some("1") | None => None,
            Some("2") => Some(delta::Decoder::default()),
            _ => {
                return Err(ReplayError::new(
                    line_number,
                    ReplayErrorKind::UnsupportedFormat,
                ));
            }
        };
        Ok(Self {
            reader,
            headers,
            pending,
            line_number,
            delta,
            failed: false,
        })
    }

    #[must_use]
    pub fn headers(&self) -> &[Header] {
        &self.headers
    }

    #[must_use]
    pub fn header(&self, key: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|header| header.key == key)
            .map(|header| header.value.as_str())
    }

    fn read_event(&mut self) -> Result<Option<Event>, ReplayError> {
        loop {
            let (line, complete) = if let Some(line) = self.pending.take() {
                line
            } else {
                let mut line = String::new();
                let count = self.reader.read_line(&mut line).map_err(ReplayError::io)?;
                if count == 0 {
                    return Ok(None);
                }
                self.line_number += 1;
                let complete = line.ends_with('\n');
                trim_line_ending(&mut line);
                (line, complete)
            };
            if line.is_empty() {
                continue;
            }
            if line.starts_with('[') {
                return Err(ReplayError::new(
                    self.line_number,
                    ReplayErrorKind::HeaderAfterBody,
                ));
            }
            let mut pieces = line.splitn(3, ' ');
            let first = pieces.next().expect("split always yields one part");
            if let (Ok(sequence), Some(kind), Some(json)) =
                (first.parse::<u64>(), pieces.next(), pieces.next())
            {
                if kind.is_empty() {
                    return Err(ReplayError::new(
                        self.line_number,
                        ReplayErrorKind::InvalidRecord,
                    ));
                }
                let payload = match parse_json(json, self.line_number) {
                    Ok(payload) => payload,
                    Err(_)
                        if !complete
                            && serde_json::from_str::<&serde_json::value::RawValue>(json)
                                .is_err_and(|error| error.is_eof()) =>
                    {
                        return Ok(None);
                    }
                    Err(error) => return Err(error),
                };
                if !payload.is_object() {
                    return Err(ReplayError::new(
                        self.line_number,
                        ReplayErrorKind::RecordPayloadNotObject,
                    ));
                }
                let record = Record {
                    sequence,
                    kind: kind.to_owned(),
                    payload,
                };
                let record = if let Some(decoder) = &mut self.delta {
                    decoder.decode(record, self.line_number)?
                } else {
                    if kind == "state.delta" {
                        return Err(ReplayError::new(
                            self.line_number,
                            ReplayErrorKind::UnsupportedFormat,
                        ));
                    }
                    record
                };
                return Ok(Some(Event::Record(record)));
            }
            // `*` is the open terminal Format v1 writes until a clean
            // finalization rewrites it: an interrupted recording ends on it,
            // and a decision script always does.
            if line == "*"
                || line
                    .chars()
                    .all(|character| character.is_ascii_uppercase() || character == '_')
            {
                return Ok(Some(Event::Terminal(line)));
            }
            return Err(ReplayError::new(
                self.line_number,
                ReplayErrorKind::InvalidRecord,
            ));
        }
    }
}

impl<R: BufRead> Iterator for Parser<R> {
    type Item = Result<Event, ReplayError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.failed {
            return None;
        }
        let result = self.read_event().transpose();
        self.failed = matches!(result, Some(Err(_)));
        result
    }
}

fn trim_line_ending(line: &mut String) {
    if line.ends_with('\n') {
        line.pop();
        if line.ends_with('\r') {
            line.pop();
        }
    }
}

fn parse_header(line: &str, line_number: usize) -> Result<Header, ReplayError> {
    let body = line
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .ok_or_else(|| ReplayError::new(line_number, ReplayErrorKind::InvalidHeader))?;
    let (key, encoded) = body
        .split_once(' ')
        .ok_or_else(|| ReplayError::new(line_number, ReplayErrorKind::InvalidHeader))?;
    if key.is_empty() || key.chars().any(char::is_whitespace) {
        return Err(ReplayError::new(
            line_number,
            ReplayErrorKind::InvalidHeader,
        ));
    }
    let value: String =
        serde_json::from_str(encoded).map_err(|error| ReplayError::json(line_number, error))?;
    Ok(Header {
        key: key.to_owned(),
        value,
    })
}

fn parse_json(input: &str, line_number: usize) -> Result<Value, ReplayError> {
    json::parse(input, line_number)
}

/// Validates required headers against the pinned compatibility manifest.
pub fn validate_compatibility<R: BufRead>(parser: &Parser<R>) -> Result<TraceProfile, ReplayError> {
    let mut seen = BTreeSet::new();
    for header in parser.headers() {
        if !seen.insert(&header.key) {
            return Err(ReplayError::new(0, ReplayErrorKind::DuplicateHeader));
        }
    }
    let profile = TraceProfile::from_headers(parser.headers())?;
    require_header(
        parser,
        "Format",
        "STS2PGN",
        ReplayErrorKind::UnsupportedFormat,
    )?;
    match parser.header("FormatVersion") {
        Some("1" | "2") => {}
        None => return Err(ReplayError::new(0, ReplayErrorKind::MissingRequiredHeader)),
        Some(_) => return Err(ReplayError::new(0, ReplayErrorKind::UnsupportedFormat)),
    }
    let manifest = CompatibilityManifest::pinned()
        .map_err(|_| ReplayError::new(0, ReplayErrorKind::ManifestMismatch))?;
    require_header(
        parser,
        "GameVersion",
        &manifest.game.version,
        ReplayErrorKind::UnsupportedBuild,
    )?;
    require_header(
        parser,
        "GameCommit",
        &manifest.game.commit,
        ReplayErrorKind::UnsupportedBuild,
    )?;
    require_header(
        parser,
        "ModelIdHash",
        &manifest.game.model_id_hash.to_string(),
        ReplayErrorKind::UnsupportedBuild,
    )?;
    validate_run_headers(parser, profile)?;

    let mode = parser
        .header("Mode")
        .ok_or_else(|| ReplayError::new(0, ReplayErrorKind::MissingRequiredHeader))?;
    if !manifest
        .support_policy
        .game_modes
        .iter()
        .any(|supported| supported.eq_ignore_ascii_case(mode))
    {
        return Err(ReplayError::new(0, ReplayErrorKind::UnsupportedMode));
    }
    let players: Value = serde_json::from_str(
        parser
            .header("Players")
            .ok_or_else(|| ReplayError::new(0, ReplayErrorKind::MissingRequiredHeader))?,
    )
    .map_err(|error| ReplayError::json(0, error))?;
    let count = players
        .as_array()
        .ok_or_else(|| ReplayError::new(0, ReplayErrorKind::InvalidHeader))?
        .len();
    if !players_are_valid(&players) {
        return Err(ReplayError::new(0, ReplayErrorKind::InvalidHeader));
    }
    if !manifest
        .support_policy
        .player_counts
        .contains(&(count as u32))
    {
        return Err(ReplayError::new(0, ReplayErrorKind::UnsupportedMode));
    }

    let loaded_mods: Vec<ModIdentity> = serde_json::from_str(parser.header("Mods").unwrap_or("[]"))
        .map_err(|error| ReplayError::json(0, error))?;
    // A `Mods` header states which mods were loaded, in whatever order the
    // loader enumerated them. Any mod may be loaded; ids and versions are
    // never compared. What refuses a recording is a mod that declares itself
    // gameplay-affecting — a recording is the ground truth replays are diffed
    // against, and such a mod poisons it. The declaration is the mod's own,
    // so a mod that touches gameplay without saying so is accepted and shows
    // up as a divergence rather than a refusal.
    if !manifest.support_policy.gameplay_affecting_mods
        && loaded_mods.iter().any(|loaded| loaded.affects_gameplay)
    {
        return Err(ReplayError::new(0, ReplayErrorKind::UnsupportedMod));
    }
    Ok(profile)
}

fn validate_run_headers<R: BufRead>(
    parser: &Parser<R>,
    profile: TraceProfile,
) -> Result<(), ReplayError> {
    let required = match profile {
        TraceProfile::Recording => ["RecorderVersion", "RunId", "Mods", "Result"].as_slice(),
        TraceProfile::Script => ["Producer"].as_slice(),
    };
    for key in ["Seed", "Ascension", "Players", "Mode"]
        .into_iter()
        .chain(required.iter().copied())
    {
        if parser.header(key).is_none_or(str::is_empty) {
            return Err(ReplayError::new(0, ReplayErrorKind::MissingRequiredHeader));
        }
    }
    if profile == TraceProfile::Script {
        let producer = parser
            .header("Producer")
            .expect("required header was checked");
        if !producer.split_once('/').is_some_and(|(name, version)| {
            !name.is_empty()
                && !version.is_empty()
                && name
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
                && !version.chars().any(|c| c.is_whitespace() || c == '/')
        }) {
            return Err(ReplayError::new(0, ReplayErrorKind::InvalidHeader));
        }
        require_header(parser, "Mode", "CUSTOM", ReplayErrorKind::UnsupportedMode)?;
        profile.validate_terminal(Some(parser.header("Result").unwrap_or("*")))?;
    }
    let canonical_seed = parser.header("Seed").expect("required header was checked");
    if canonicalize_seed(canonical_seed) != canonical_seed {
        return Err(ReplayError::new(0, ReplayErrorKind::InvalidHeader));
    }
    parser
        .header("Ascension")
        .expect("required header was checked")
        .parse::<u32>()
        .map_err(|_| ReplayError::new(0, ReplayErrorKind::InvalidHeader))?;
    Ok(())
}

fn players_are_valid(value: &Value) -> bool {
    value.as_array().is_some_and(|players| {
        let mut slots = BTreeSet::new();
        players.iter().all(|player| {
            let Some(player) = player.as_object() else {
                return false;
            };
            let Some(slot) = player.get("slot").and_then(Value::as_str) else {
                return false;
            };
            player
                .get("character")
                .and_then(Value::as_str)
                .is_some_and(|character| !character.is_empty() && slots.insert(slot))
        })
    })
}

fn require_header<R: BufRead>(
    parser: &Parser<R>,
    key: &str,
    expected: &str,
    mismatch: ReplayErrorKind,
) -> Result<(), ReplayError> {
    match parser.header(key) {
        Some(actual) if actual == expected => Ok(()),
        Some(_) => Err(ReplayError::new(0, mismatch)),
        None => Err(ReplayError::new(0, ReplayErrorKind::MissingRequiredHeader)),
    }
}

/// Incremental structural validation statistics.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ValidationSummary {
    pub record_count: u64,
    pub observation_count: u64,
    pub terminal: Option<String>,
}

/// Consumes and validates a stream without retaining record payloads.
pub fn validate_stream<R: BufRead>(
    parser: &mut Parser<R>,
) -> Result<ValidationSummary, ReplayError> {
    let profile = validate_compatibility(parser)?;
    let expected_terminal = parser.header("Result").unwrap_or("*").to_owned();
    let version_two = parser.header("FormatVersion") == Some("2");
    let mut summary = ValidationSummary::default();
    let mut expected_sequence = 1;
    let mut terminal_seen = false;
    for event in parser {
        match event? {
            Event::Record(record) => {
                profile.validate_record(&record)?;
                if terminal_seen {
                    return Err(ReplayError::new(0, ReplayErrorKind::DataAfterTerminal));
                }
                if record.sequence != expected_sequence {
                    return Err(ReplayError::new(0, ReplayErrorKind::NonContiguousSequence));
                }
                expected_sequence += 1;
                summary.record_count += 1;
                if record.kind == "state.observe" {
                    if !version_two {
                        validate_observation(&record)?;
                    }
                    summary.observation_count += 1;
                }
            }
            Event::Terminal(terminal) => {
                if terminal_seen {
                    return Err(ReplayError::new(0, ReplayErrorKind::DuplicateTerminal));
                }
                terminal_seen = true;
                summary.terminal = Some(terminal);
            }
        }
    }
    if summary.terminal.is_none() {
        return Err(ReplayError::new(0, ReplayErrorKind::MissingTerminal));
    }
    profile.validate_terminal(summary.terminal.as_deref())?;
    if summary.terminal.as_deref() != Some(expected_terminal.as_str()) {
        return Err(ReplayError::new(0, ReplayErrorKind::TerminalMismatch));
    }
    Ok(summary)
}

/// Validates a Projection v1 observation's embedded digest, accepting both the
/// legacy Rust encoding and the C# recorder's encoding of canonical wire values.
pub fn validate_observation(record: &Record) -> Result<(), ReplayError> {
    if record.kind != "state.observe" {
        return Err(ReplayError::new(0, ReplayErrorKind::NotObservation));
    }
    let object = record
        .payload
        .as_object()
        .ok_or_else(|| ReplayError::new(0, ReplayErrorKind::RecordPayloadNotObject))?;
    if object.get("projection_version").and_then(Value::as_u64) != Some(1) {
        return Err(ReplayError::new(0, ReplayErrorKind::UnsupportedProjection));
    }
    let state = object
        .get("state")
        .ok_or_else(|| ReplayError::new(0, ReplayErrorKind::InvalidObservation))?;
    let expected = object
        .get("digest")
        .and_then(Value::as_str)
        .ok_or_else(|| ReplayError::new(0, ReplayErrorKind::InvalidObservation))?;
    let actual = projection_digest(state)?;
    if expected == actual || canonical::validate(record).is_ok() {
        Ok(())
    } else {
        Err(ReplayError::new(0, ReplayErrorKind::DigestMismatch))
    }
}

/// Writes canonical UTF-8 JSON with recursively sorted object keys.
pub fn write_canonical_json<W: Write>(value: &Value, writer: &mut W) -> Result<(), ReplayError> {
    match value {
        Value::Null => writer.write_all(b"null").map_err(ReplayError::io),
        Value::Bool(value) => writer
            .write_all(if *value { b"true" } else { b"false" })
            .map_err(ReplayError::io),
        Value::Number(value) => writer
            .write_all(value.to_string().as_bytes())
            .map_err(ReplayError::io),
        Value::String(value) => {
            serde_json::to_writer(writer, value).map_err(|error| ReplayError::json(0, error))
        }
        Value::Array(values) => {
            writer.write_all(b"[").map_err(ReplayError::io)?;
            for (index, value) in values.iter().enumerate() {
                if index != 0 {
                    writer.write_all(b",").map_err(ReplayError::io)?;
                }
                write_canonical_json(value, writer)?;
            }
            writer.write_all(b"]").map_err(ReplayError::io)
        }
        Value::Object(values) => {
            writer.write_all(b"{").map_err(ReplayError::io)?;
            let mut entries: Vec<_> = values.iter().collect();
            entries.sort_unstable_by(|left, right| left.0.cmp(right.0));
            for (index, (key, value)) in entries.into_iter().enumerate() {
                if index != 0 {
                    writer.write_all(b",").map_err(ReplayError::io)?;
                }
                serde_json::to_writer(&mut *writer, key)
                    .map_err(|error| ReplayError::json(0, error))?;
                writer.write_all(b":").map_err(ReplayError::io)?;
                write_canonical_json(value, writer)?;
            }
            writer.write_all(b"}").map_err(ReplayError::io)
        }
    }
}

#[must_use = "digest computation can fail while canonicalizing"]
pub fn projection_digest(value: &Value) -> Result<String, ReplayError> {
    let mut writer = HashWriter(Sha256::new());
    write_canonical_json(value, &mut writer)?;
    Ok(format!("sha256:{:x}", writer.0.finalize()))
}

struct HashWriter(Sha256);

impl Write for HashWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Re-emits headers and events while preserving all unknown semantic fields.
pub fn write_stream<W, I>(writer: &mut W, headers: &[Header], events: I) -> Result<(), ReplayError>
where
    W: Write,
    I: IntoIterator<Item = Event>,
{
    let profile = TraceProfile::from_headers(headers)?;
    for header in headers {
        writeln!(
            writer,
            "[{} {}]",
            header.key,
            serde_json::to_string(&header.value).map_err(|error| ReplayError::json(0, error))?
        )
        .map_err(ReplayError::io)?;
    }
    if !headers.iter().any(|header| header.key == "Profile") {
        writeln!(writer, "[Profile \"{}\"]", profile.as_str()).map_err(ReplayError::io)?;
    }
    writer.write_all(b"\n").map_err(ReplayError::io)?;
    for event in events {
        match event {
            Event::Record(record) => {
                write!(writer, "{} {} ", record.sequence, record.kind).map_err(ReplayError::io)?;
                write_canonical_json(&record.payload, writer)?;
                writer.write_all(b"\n").map_err(ReplayError::io)?;
            }
            Event::Terminal(terminal) => {
                writeln!(writer, "{terminal}").map_err(ReplayError::io)?;
            }
        }
    }
    Ok(())
}

#[derive(Debug)]
pub struct ReplayError {
    pub line: usize,
    pub kind: ReplayErrorKind,
    source: Option<Box<dyn Error + Send + Sync>>,
}

impl ReplayError {
    fn new(line: usize, kind: ReplayErrorKind) -> Self {
        Self {
            line,
            kind,
            source: None,
        }
    }

    fn json(line: usize, error: serde_json::Error) -> Self {
        Self {
            line,
            kind: ReplayErrorKind::InvalidJson,
            source: Some(Box::new(error)),
        }
    }

    fn io(error: io::Error) -> Self {
        Self {
            line: 0,
            kind: ReplayErrorKind::Io,
            source: Some(Box::new(error)),
        }
    }
}

impl Display for ReplayError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(formatter, "{}", self.kind)
        } else {
            write!(formatter, "line {}: {}", self.line, self.kind)
        }
    }
}

impl Error for ReplayError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source
            .as_deref()
            .map(|source| source as &(dyn Error + 'static))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplayErrorKind {
    Io,
    InvalidJson,
    MissingHeaders,
    InvalidHeader,
    InvalidProfile,
    ConflictingProfile,
    InvalidScriptRecord,
    InvalidScriptResult,
    DuplicateHeader,
    HeaderAfterBody,
    MissingRequiredHeader,
    UnsupportedFormat,
    UnsupportedBuild,
    UnsupportedMode,
    UnsupportedMod,
    ManifestMismatch,
    InvalidRecord,
    RecordPayloadNotObject,
    NonContiguousSequence,
    DuplicateTerminal,
    MissingTerminal,
    TerminalMismatch,
    DataAfterTerminal,
    NotObservation,
    InvalidObservation,
    UnsupportedProjection,
    DigestMismatch,
    InvalidCorrelation,
    InvalidDelta,
    InvalidDeltaBase,
}

impl Display for ReplayErrorKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let code = match self {
            Self::Io => "io",
            Self::InvalidJson => "invalid_json",
            Self::MissingHeaders => "missing_headers",
            Self::InvalidHeader => "invalid_header",
            Self::InvalidProfile => "invalid_profile",
            Self::ConflictingProfile => "conflicting_profile",
            Self::InvalidScriptRecord => "invalid_script_record",
            Self::InvalidScriptResult => "invalid_script_result",
            Self::DuplicateHeader => "duplicate_header",
            Self::HeaderAfterBody => "header_after_body",
            Self::MissingRequiredHeader => "missing_required_header",
            Self::UnsupportedFormat => "unsupported_format",
            Self::UnsupportedBuild => "unsupported_build",
            Self::UnsupportedMode => "unsupported_mode",
            Self::UnsupportedMod => "unsupported_mod",
            Self::ManifestMismatch => "manifest_mismatch",
            Self::InvalidRecord => "invalid_record",
            Self::RecordPayloadNotObject => "record_payload_not_object",
            Self::NonContiguousSequence => "non_contiguous_sequence",
            Self::DuplicateTerminal => "duplicate_terminal",
            Self::MissingTerminal => "missing_terminal",
            Self::TerminalMismatch => "terminal_mismatch",
            Self::DataAfterTerminal => "data_after_terminal",
            Self::NotObservation => "not_observation",
            Self::InvalidObservation => "invalid_observation",
            Self::UnsupportedProjection => "unsupported_projection",
            Self::DigestMismatch => "digest_mismatch",
            Self::InvalidCorrelation => "invalid_correlation",
            Self::InvalidDelta => "invalid_delta",
            Self::InvalidDeltaBase => "invalid_delta_base",
        };
        formatter.write_str(code)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    fn minimal(extra_header: &str, payload: &str) -> String {
        format!(
            "[Format \"STS2PGN\"]\n\
             [FormatVersion \"1\"]\n\
             [RecorderVersion \"test\"]\n\
             [RunId \"run\"]\n\
             [GameVersion \"v0.107.1\"]\n\
             [GameCommit \"59260271\"]\n\
             [ModelIdHash \"3954186980\"]\n\
             [Seed \"ABC\"]\n\
             [Mode \"STANDARD\"]\n\
             [Ascension \"0\"]\n\
             [Players \"[{{\\\"slot\\\":\\\"p0\\\",\\\"character\\\":\\\"IRONCLAD\\\"}}]\"]\n\
             [Mods \"[]\"]\n\
             [Result \"DEFEAT\"]\n\
             {extra_header}\n\
             1 custom.record {payload}\n\
             DEFEAT\n"
        )
    }

    /// The recorder stack the reference recordings declare, in an order the
    /// loader enumerated differently from one session to the next.
    const REFERENCE_STACK: &str = "[Mods \"[{\\\"id\\\":\\\"STS2_PGN\\\",\\\"version\\\":\\\"0.6.0\\\",\\\"affects_gameplay\\\":false},\
         {\\\"id\\\":\\\"STS2_MCP\\\",\\\"version\\\":\\\"0.4.0\\\",\\\"affects_gameplay\\\":false},\
         {\\\"id\\\":\\\"BaseLib\\\",\\\"version\\\":\\\"v3.4.5\\\",\\\"affects_gameplay\\\":false}]\"]";

    #[test]
    fn the_reference_stack_is_accepted_in_the_order_the_loader_enumerated_it() {
        let input = minimal("", r#"{"a":1}"#).replace("[Mods \"[]\"]", REFERENCE_STACK);
        let parser = Parser::new(Cursor::new(input)).unwrap();
        validate_compatibility(&parser).unwrap();
    }

    #[test]
    fn a_mod_outside_the_reference_stack_is_accepted_when_it_does_not_affect_gameplay() {
        let extended = REFERENCE_STACK.replace(
            "]\"]",
            ",{\\\"id\\\":\\\"QualityOfLife\\\",\\\"version\\\":\\\"1.0\\\",\\\"affects_gameplay\\\":false}]\"]",
        );
        let input = minimal("", r#"{"a":1}"#).replace("[Mods \"[]\"]", &extended);
        let parser = Parser::new(Cursor::new(input)).unwrap();
        validate_compatibility(&parser).unwrap();
    }

    #[test]
    fn a_mod_is_accepted_at_any_version() {
        let input = minimal("", r#"{"a":1}"#)
            .replace("[Mods \"[]\"]", &REFERENCE_STACK.replace("0.6.0", "0.8.0"));
        let parser = Parser::new(Cursor::new(input)).unwrap();
        validate_compatibility(&parser).unwrap();
    }

    #[test]
    fn format_two_uses_the_same_build_and_mode_compatibility_checks() {
        let input = minimal("", "{}").replace("[FormatVersion \"1\"]", "[FormatVersion \"2\"]");
        validate_compatibility(&Parser::new(Cursor::new(&input)).unwrap()).unwrap();
        let unsupported = input.replace("59260271", "unsupported");
        assert_eq!(
            validate_compatibility(&Parser::new(Cursor::new(unsupported)).unwrap())
                .unwrap_err()
                .kind,
            ReplayErrorKind::UnsupportedBuild
        );
    }

    #[test]
    fn a_mod_that_declares_itself_gameplay_affecting_is_refused() {
        let affecting = REFERENCE_STACK.replace(
            "\\\"STS2_PGN\\\",\\\"version\\\":\\\"0.6.0\\\",\\\"affects_gameplay\\\":false",
            "\\\"STS2_PGN\\\",\\\"version\\\":\\\"0.6.0\\\",\\\"affects_gameplay\\\":true",
        );
        let input = minimal("", r#"{"a":1}"#).replace("[Mods \"[]\"]", &affecting);
        let parser = Parser::new(Cursor::new(input)).unwrap();
        assert_eq!(
            validate_compatibility(&parser).unwrap_err().kind,
            ReplayErrorKind::UnsupportedMod
        );
    }

    #[test]
    fn unknown_fields_and_headers_survive_round_trip() {
        let input = minimal(
            "[FutureHeader \"future\"]",
            r#"{"z":2,"extension":{"new":true},"a":1}"#,
        );
        let mut parser = Parser::new(Cursor::new(input)).unwrap();
        validate_compatibility(&parser).unwrap();
        let headers = parser.headers().to_vec();
        let events: Vec<_> = parser.by_ref().collect::<Result<_, _>>().unwrap();
        let mut output = Vec::new();
        write_stream(&mut output, &headers, events).unwrap();
        let mut reparsed = Parser::new(Cursor::new(output)).unwrap();
        assert_eq!(reparsed.header("FutureHeader"), Some("future"));
        let Event::Record(record) = reparsed.next().unwrap().unwrap() else {
            panic!("expected record");
        };
        assert_eq!(record.payload["extension"]["new"], true);
        assert_eq!(record.payload["z"], 2);
    }

    #[test]
    fn canonical_json_is_recursive_and_distinguishes_null() {
        let value: Value =
            serde_json::from_str(r#"{"z":null,"a":[{"y":2,"x":1}],"n":1.0}"#).unwrap();
        let mut bytes = Vec::new();
        write_canonical_json(&value, &mut bytes).unwrap();
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            r#"{"a":[{"x":1,"y":2}],"n":1.0,"z":null}"#
        );
        assert_eq!(
            projection_digest(&value).unwrap(),
            "sha256:4a425fe5a4cd413271eca6d8a72efafd9133210bf710cd1cf8398a3fb389540e"
        );
    }

    #[test]
    fn duplicate_json_keys_are_rejected() {
        let input = minimal("", r#"{"a":1,"a":2}"#);
        let mut parser = Parser::new(Cursor::new(input)).unwrap();
        assert_eq!(
            parser.next().unwrap().unwrap_err().kind,
            ReplayErrorKind::InvalidJson
        );
    }

    #[test]
    fn validation_rejects_gaps_without_buffering_records() {
        let input = minimal("", r#"{"unknown":true}"#).replace("1 custom", "2 custom");
        let mut parser = Parser::new(Cursor::new(input)).unwrap();
        assert_eq!(
            validate_stream(&mut parser).unwrap_err().kind,
            ReplayErrorKind::NonContiguousSequence
        );
    }
}
