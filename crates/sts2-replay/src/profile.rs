//! Shared trace identity and the strength of a replay's evidence.

use serde::Serialize;

use crate::{AdapterErrorCode, Header, Record, ReplayError, ReplayErrorKind};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceProfile {
    Recording,
    Script,
}

impl TraceProfile {
    /// Resolve the explicit header, or the legacy producer convention. Required
    /// producer tags are checked separately by compatibility validation.
    pub fn from_headers(headers: &[Header]) -> Result<Self, ReplayError> {
        let mut profiles = headers.iter().filter(|header| header.key == "Profile");
        let declared = profiles.next().map(|header| header.value.as_str());
        if profiles.next().is_some() {
            return Err(ReplayError::new(0, ReplayErrorKind::DuplicateHeader));
        }
        let producer = headers.iter().any(|header| header.key == "Producer");
        let recorder = headers.iter().any(|header| header.key == "RecorderVersion");
        let profile = match declared {
            Some("script") => Self::Script,
            None if producer => Self::Script,
            Some("recording") | None => Self::Recording,
            Some(_) => return Err(ReplayError::new(0, ReplayErrorKind::InvalidProfile)),
        };
        if (profile == Self::Recording && producer) || (profile == Self::Script && recorder) {
            return Err(ReplayError::new(0, ReplayErrorKind::ConflictingProfile));
        }
        Ok(profile)
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Recording => "recording",
            Self::Script => "script",
        }
    }

    #[must_use]
    pub const fn verification_level(self) -> VerificationLevel {
        match self {
            Self::Recording => VerificationLevel::RecordedState,
            Self::Script => VerificationLevel::ActionConsistency,
        }
    }

    pub fn validate_record(self, record: &Record) -> Result<(), ReplayError> {
        if self == Self::Script && matches!(record.kind.as_str(), "run.resume" | "run.finish") {
            return Err(ReplayError::new(0, ReplayErrorKind::InvalidScriptRecord));
        }
        Ok(())
    }

    /// Scripts end in '*'. Recordings may be an open crash-recoverable prefix.
    pub fn validate_terminal(self, terminal: Option<&str>) -> Result<(), ReplayError> {
        if self == Self::Script && terminal != Some("*") {
            return Err(ReplayError::new(0, ReplayErrorKind::InvalidScriptResult));
        }
        Ok(())
    }

    /// Assess only the evidence actually compared. A script's terminal is not
    /// an assertion about the simulator's result, even when it finishes a run.
    #[must_use]
    pub fn assess(
        self,
        compared_observations: usize,
        failure: Option<AdapterErrorCode>,
        recorded_terminal: Option<&str>,
        simulated_terminal: Option<&str>,
    ) -> ReplayVerification {
        let outcome = match failure {
            Some(AdapterErrorCode::MalformedRecord) => ReplayOutcome::InvalidInput,
            Some(AdapterErrorCode::UnsupportedContent) => ReplayOutcome::UnsupportedContent,
            Some(AdapterErrorCode::IllegalAction | AdapterErrorCode::StateDivergence) => {
                ReplayOutcome::Diverged
            }
            None if self == Self::Recording
                && !terminals_agree(recorded_terminal, simulated_terminal) =>
            {
                ReplayOutcome::TerminalMismatch
            }
            None if self == Self::Recording && compared_observations > 0 => {
                ReplayOutcome::ObservationsMatched
            }
            None => ReplayOutcome::ActionsAccepted,
        };
        let warning = if self == Self::Script {
            Some(
                "Action-script replay checks actions and their identities. It does not verify recorded game state.",
            )
        } else if compared_observations == 0 {
            Some(
                "No authoritative observations were compared; this is not a state conformance result.",
            )
        } else {
            None
        };
        ReplayVerification {
            profile: self,
            level: self.verification_level(),
            compared_observations,
            outcome,
            failure_kind: failure,
            warning,
        }
    }
}

fn terminals_agree(recorded: Option<&str>, simulated: Option<&str>) -> bool {
    match (recorded, simulated) {
        (Some("*"), _) | (None | Some("ABANDONED"), None) => true,
        (Some(recorded), Some(simulated)) => recorded.eq_ignore_ascii_case(simulated),
        _ => false,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationLevel {
    ActionConsistency,
    RecordedState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplayOutcome {
    ActionsAccepted,
    ObservationsMatched,
    Diverged,
    TerminalMismatch,
    InvalidInput,
    UnsupportedContent,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReplayVerification {
    pub profile: TraceProfile,
    pub level: VerificationLevel,
    pub compared_observations: usize,
    pub outcome: ReplayOutcome,
    pub failure_kind: Option<AdapterErrorCode>,
    pub warning: Option<&'static str>,
}

impl ReplayVerification {
    #[must_use]
    pub const fn succeeded(&self) -> bool {
        matches!(
            self.outcome,
            ReplayOutcome::ActionsAccepted | ReplayOutcome::ObservationsMatched
        )
    }

    /// Means the compared fields agreed, not that uncovered fields were checked.
    #[must_use]
    pub fn verified(&self) -> bool {
        self.outcome == ReplayOutcome::ObservationsMatched
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Parser, validate_compatibility, validate_stream};
    use std::fmt::Write as _;

    fn document(profile: TraceProfile, version: u8) -> String {
        let manifest = sts2_core::CompatibilityManifest::pinned().unwrap();
        let mut tags = vec![
            ("Format", "STS2PGN".to_owned()),
            ("FormatVersion", version.to_string()),
            ("GameVersion", manifest.game.version),
            ("GameCommit", manifest.game.commit),
            ("ModelIdHash", manifest.game.model_id_hash.to_string()),
            ("Seed", "ABC".to_owned()),
            ("Ascension", "0".to_owned()),
            ("Mode", "CUSTOM".to_owned()),
            (
                "Players",
                r#"[{"slot":"p0","character":"IRONCLAD"}]"#.to_owned(),
            ),
        ];
        match profile {
            TraceProfile::Recording => tags.extend([
                ("RecorderVersion", "test".to_owned()),
                ("RunId", "test".to_owned()),
                ("Mods", "[]".to_owned()),
                ("Result", "*".to_owned()),
            ]),
            TraceProfile::Script => tags.push(("Producer", "test/1".to_owned())),
        }
        let mut text = String::new();
        for (key, value) in tags {
            writeln!(text, "[{key} {}]", serde_json::to_string(&value).unwrap()).unwrap();
        }
        text.push_str("\n1 run.start {}\n*\n");
        text
    }

    #[test]
    fn both_versions_accept_explicit_and_legacy_profiles_and_emit_explicit_identity() {
        for profile in [TraceProfile::Script, TraceProfile::Recording] {
            for version in [1, 2] {
                for explicit in [false, true] {
                    let mut input = document(profile, version);
                    if explicit {
                        input = format!("[Profile \"{}\"]\n{input}", profile.as_str());
                    }
                    let mut parser = Parser::new(input.as_bytes()).unwrap();
                    assert_eq!(validate_compatibility(&parser).unwrap(), profile);
                    let headers = parser.headers().to_vec();
                    let events = parser.by_ref().collect::<Result<Vec<_>, _>>().unwrap();
                    let mut output = Vec::new();
                    crate::write_stream(&mut output, &headers, events).unwrap();
                    let mut parser = Parser::new(output.as_slice()).unwrap();
                    assert_eq!(parser.header("Profile"), Some(profile.as_str()));
                    assert_eq!(
                        validate_stream(&mut parser).unwrap().terminal.as_deref(),
                        Some("*")
                    );
                }
            }
        }
    }

    #[test]
    fn conflicting_unknown_and_duplicate_profiles_fail() {
        for profile in [TraceProfile::Script, TraceProfile::Recording] {
            let other = if profile == TraceProfile::Script {
                "recording"
            } else {
                "script"
            };
            for (prefix, expected) in [
                (
                    format!("[Profile \"{other}\"]\n"),
                    ReplayErrorKind::ConflictingProfile,
                ),
                (
                    "[Profile \"observation\"]\n".to_owned(),
                    ReplayErrorKind::InvalidProfile,
                ),
                (
                    "[Profile \"Script\"]\n".to_owned(),
                    ReplayErrorKind::InvalidProfile,
                ),
                (
                    "[Profile \"script\"]\n[Profile \"script\"]\n".to_owned(),
                    ReplayErrorKind::DuplicateHeader,
                ),
            ] {
                let input = prefix + &document(profile, 1);
                let parser = Parser::new(input.as_bytes()).unwrap();
                assert_eq!(validate_compatibility(&parser).unwrap_err().kind, expected);
            }
        }
    }

    #[test]
    fn scripts_keep_build_and_run_validation_without_recording_only_requirements() {
        let input = document(TraceProfile::Script, 2);
        let manifest = sts2_core::CompatibilityManifest::pinned().unwrap();
        for (old, new, expected) in [
            (
                manifest.game.commit.as_str(),
                "unsupported",
                ReplayErrorKind::UnsupportedBuild,
            ),
            (
                "[Mode \"CUSTOM\"]",
                "[Mode \"STANDARD\"]",
                ReplayErrorKind::UnsupportedMode,
            ),
            (
                "[Producer \"test/1\"]",
                "[Producer \"test\"]",
                ReplayErrorKind::InvalidHeader,
            ),
            (
                "[Producer \"test/1\"]\n",
                "",
                ReplayErrorKind::MissingRequiredHeader,
            ),
            (
                "[Ascension \"0\"]",
                "[Ascension \"bad\"]",
                ReplayErrorKind::InvalidHeader,
            ),
        ] {
            let invalid = format!("[Profile \"script\"]\n{}", input.replace(old, new));
            let parser = Parser::new(invalid.as_bytes()).unwrap();
            assert_eq!(validate_compatibility(&parser).unwrap_err().kind, expected);
        }
    }

    #[test]
    fn scripts_reject_recording_only_records_and_completed_results() {
        for replacement in ["run.resume", "run.finish"] {
            let text = document(TraceProfile::Script, 1).replace("run.start", replacement);
            let mut parser = Parser::new(text.as_bytes()).unwrap();
            assert_eq!(
                validate_stream(&mut parser).unwrap_err().kind,
                ReplayErrorKind::InvalidScriptRecord
            );
        }
        let text = document(TraceProfile::Script, 1).replace("*\n", "VICTORY\n");
        assert_eq!(
            validate_stream(&mut Parser::new(text.as_bytes()).unwrap())
                .unwrap_err()
                .kind,
            ReplayErrorKind::InvalidScriptResult
        );
    }

    #[test]
    fn evidence_and_failure_types_are_separate_from_success() {
        let script = TraceProfile::Script.assess(0, None, Some("*"), Some("VICTORY"));
        assert!(script.succeeded());
        assert!(!script.verified());
        assert!(script.warning.is_some());
        let recording = TraceProfile::Recording.assess(0, None, Some("*"), None);
        assert!(recording.succeeded());
        assert!(!recording.verified());
        assert!(recording.warning.is_some());
        assert!(
            TraceProfile::Recording
                .assess(1, None, Some("*"), None)
                .verified()
        );
        assert_eq!(
            TraceProfile::Recording
                .assess(1, None, Some("VICTORY"), Some("DEFEAT"))
                .outcome,
            ReplayOutcome::TerminalMismatch
        );
        for (failure, outcome) in [
            (AdapterErrorCode::IllegalAction, ReplayOutcome::Diverged),
            (AdapterErrorCode::StateDivergence, ReplayOutcome::Diverged),
            (
                AdapterErrorCode::UnsupportedContent,
                ReplayOutcome::UnsupportedContent,
            ),
            (
                AdapterErrorCode::MalformedRecord,
                ReplayOutcome::InvalidInput,
            ),
        ] {
            let evidence = TraceProfile::Script.assess(0, Some(failure), Some("*"), None);
            assert_eq!(evidence.outcome, outcome);
            assert!(!evidence.succeeded());
        }
    }
}
