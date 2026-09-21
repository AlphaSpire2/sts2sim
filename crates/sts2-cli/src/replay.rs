//! Differential replay of a recorded STS2PGN file against the simulator.

use std::io::{BufRead, Write};

use serde_json::{Value, json};
use sts2_replay::{
    AdapterError, AdapterErrorCode, Event, Parser, ReplayAdapter, validate_compatibility,
};

use crate::CliError;

/// How many differing fields a divergence report prints before summarizing.
const DIFFERENCE_LIMIT: usize = 20;

/// What a differential replay found. `diverged` is the only failing outcome;
/// everything else is reported even on a clean run.
#[derive(Debug)]
pub struct ReplayReport {
    pub seed: String,
    pub records: u64,
    pub applied_actions: usize,
    pub compared_observations: usize,
    /// Observations taken between two rooms, which this engine walks through
    /// inside a single step and so never settles on.
    pub dropped_room_entries: usize,
    pub uncovered_fields: Vec<String>,
    pub recorded_terminal: Option<String>,
    pub simulated_terminal: Option<String>,
    /// How many times the recording picked the run back up off its save file.
    pub resumes: usize,
    /// How many applied actions those reloads rewound: the ones taken between
    /// the build's last save and the quit, which the resumed timeline never
    /// saw.
    pub rewound_actions: usize,
    pub divergence: Option<Box<AdapterError>>,
}

impl ReplayReport {
    /// True when the recording and the simulation agreed on everything the
    /// renderer covers, and both reached the same terminal.
    #[must_use]
    pub fn matched(&self) -> bool {
        self.divergence.is_none() && self.terminals_agree()
    }

    fn terminals_agree(&self) -> bool {
        match (&self.recorded_terminal, &self.simulated_terminal) {
            // Format v1 writes `*` while a recording is open and rewrites it
            // only at a clean finalization. A trace that still says `*` is a
            // prefix and makes no assertion about what settling its last input
            // would eventually do, so only its recorded checkpoints constrain
            // replay. An abandoned trace does assert that the player walked
            // away from a still-live run.
            (Some(recorded), _) if recorded == "*" => true,
            (Some(recorded), simulated) if recorded.eq_ignore_ascii_case("ABANDONED") => {
                simulated.is_none()
            }
            (Some(recorded), Some(simulated)) => recorded.eq_ignore_ascii_case(simulated),
            (None, None) => true,
            _ => false,
        }
    }
}

/// Replays `input` against a run generated from its own `Seed` header,
/// comparing every settled recorded observation against simulated state.
pub fn replay<R: BufRead>(input: R) -> Result<ReplayReport, CliError> {
    let mut parser = Parser::new(input).map_err(|error| CliError::Replay(error.to_string()))?;
    validate_compatibility(&parser).map_err(|error| CliError::Replay(error.to_string()))?;
    let seed = parser
        .header("Seed")
        .expect("compatibility validation requires Seed")
        .to_owned();
    let recorded_terminal = parser.header("Result").map(str::to_owned);
    // The level the run was played at is a generation input the recording
    // states outright, so the run is generated at it rather than at the
    // baseline. `validate_compatibility` has already checked that the header
    // parses; a level the build itself would not start is refused by
    // `RunConfig::validate` when the run is stood up.
    let ascension: u8 = parser
        .header("Ascension")
        .expect("compatibility validation requires Ascension")
        .parse()
        .map_err(|_| {
            CliError::Replay(
                "unsupported_mode: the recorded ascension is not a level this build offers"
                    .to_owned(),
            )
        })?;
    require_ironclad(&parser)?;
    // The mode is a lobby fact the projection compares, not a generation
    // input: a custom run is a standard run whose seed was typed in.
    // `validate_compatibility` has already refused any mode the manifest
    // does not support.
    let mode = if parser
        .header("Mode")
        .expect("compatibility validation requires Mode")
        .eq_ignore_ascii_case("custom")
    {
        sts2_engine::RunMode::Custom
    } else {
        sts2_engine::RunMode::Standard
    };

    // The run is generated once the recording has named the local profile it
    // was played on, which its opening `run.start` projects and its headers do
    // not carry.
    let mut adapter: Option<ReplayAdapter> = None;
    let mut records = 0;
    let mut divergence = None;
    for event in &mut parser {
        let event = event.map_err(|error| CliError::Replay(error.to_string()))?;
        let Event::Record(record) = event else {
            continue;
        };
        records += 1;
        let adapter = match adapter.as_mut() {
            Some(adapter) => adapter,
            None => adapter.insert(
                ReplayAdapter::new(recorded_run(&seed, ascension, mode, &record)?)
                    .verifying_observations(),
            ),
        };
        if let Err(error) = adapter.accept(&record) {
            if error.code == AdapterErrorCode::StateDivergence {
                divergence = Some(Box::new(error));
                break;
            }
            return Err(CliError::Replay(error.to_string()));
        }
    }
    let mut adapter = adapter
        .ok_or_else(|| CliError::Replay("recording carries no records to replay".to_owned()))?;
    // The last observation stays pending until an action follows it, so a
    // recording that ends on one needs the final comparison forced.
    if divergence.is_none() {
        if let Err(error) = adapter.verify_pending_observation() {
            divergence = Some(Box::new(error));
        }
    }
    Ok(ReplayReport {
        seed,
        records,
        applied_actions: adapter.applied_actions(),
        compared_observations: adapter.compared_observations(),
        dropped_room_entries: adapter.dropped_room_entries(),
        uncovered_fields: adapter.uncovered_fields().iter().cloned().collect(),
        recorded_terminal,
        simulated_terminal: adapter
            .simulator()
            .state()
            .terminal
            .map(|result| format!("{result:?}").to_uppercase()),
        resumes: adapter.resumes(),
        rewound_actions: adapter.rewound_actions(),
        divergence,
    })
}

/// Generates the run a recording was played on: its seed, plus the registered
/// unlock preset whose generation inputs the opening `run.start` projects.
///
/// Epoch and encounter history are generation inputs, so a recording played on
/// a profile no registered preset carries is refused rather than replayed
/// against the nearest one. The prior-run count is not one, past a first run,
/// so the preset lookup ignores it — but the recording's own count is what the
/// run carries and projects, since the observations compare it exactly. This
/// reads the recording's declared configuration, not its state: nothing here
/// repairs simulated state.
fn recorded_run(
    seed: &str,
    ascension: u8,
    mode: sts2_engine::RunMode,
    first: &sts2_replay::Record,
) -> Result<sts2_engine::Simulator, CliError> {
    if first.kind != "run.start" {
        return Err(CliError::Replay(format!(
            "invalid_replay: a recording opens with run.start, not {}",
            first.kind
        )));
    }
    let projected = first
        .payload
        .pointer("/data/state/players/0/unlocks")
        .ok_or_else(|| {
            CliError::Replay("invalid_replay: run.start projects no unlock state".to_owned())
        })?;
    let unlocks: sts2_core::UnlockState = serde_json::from_value(projected.clone())
        .map_err(|error| CliError::Replay(format!("invalid_replay: unlock state: {error}")))?;
    let preset = sts2_core::UnlockPresetManifest::matching(&unlocks)
        .map_err(|error| CliError::Replay(error.to_string()))?
        .ok_or_else(|| {
            CliError::Replay(format!(
                "unsupported_mode: the recorded profile ({} prior runs, {} encounters seen, \
                 {} epochs revealed) is not a registered unlock preset",
                unlocks.number_of_runs,
                unlocks.encounters_seen.len(),
                unlocks.epochs.len()
            ))
        })?;
    // The character the run was played as is what the recording's own opening
    // state names, and it is what the run is generated for.
    let character = first
        .payload
        .pointer("/data/state/players/0/character_model")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            CliError::Replay("invalid_replay: run.start projects no character".to_owned())
        })?;
    let character: sts2_core::ModelId = character
        .parse()
        .map_err(|_| CliError::Replay("invalid_replay: character is not a model ID".to_owned()))?;
    Ok(sts2_content::run_on_preset_at(
        seed, &character, &preset, ascension, mode,
    )?)
}

/// A recording of more than one player is outside the replayed slice.
fn require_ironclad<R: BufRead>(parser: &Parser<R>) -> Result<(), CliError> {
    let players: Value = serde_json::from_str(
        parser
            .header("Players")
            .expect("compatibility validation requires Players"),
    )?;
    let characters: Vec<&str> = players
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|player| player.get("character").and_then(Value::as_str))
        .collect();
    if characters.len() == 1 {
        return Ok(());
    }
    Err(CliError::Replay(format!(
        "unsupported recording: only a single-player run replays, not {}",
        characters.join(", ")
    )))
}

/// Renders a report. `uncovered` lists every unrendered field by name rather
/// than only counting them; JSON output always carries the full list.
pub fn write_report<W: Write>(
    report: &ReplayReport,
    output: &mut W,
    json_output: bool,
    uncovered: bool,
) -> Result<(), CliError> {
    if json_output {
        write_json(report, output)
    } else {
        write_text(report, output, uncovered)
    }
}

fn write_json<W: Write>(report: &ReplayReport, output: &mut W) -> Result<(), CliError> {
    let divergence = report.divergence.as_ref().map(|error| {
        let differences = error.divergence.as_ref().map(|divergence| {
            divergence
                .differences
                .iter()
                .map(|difference| {
                    json!({
                        "field": difference.pointer,
                        "expected": difference.expected,
                        "simulated": difference.actual,
                    })
                })
                .collect::<Vec<_>>()
        });
        json!({
            "record": error.record_sequence,
            "kind": error.record_kind,
            "detail": error.detail,
            "last_matching_checkpoint": error
                .divergence
                .as_ref()
                .and_then(|divergence| divergence.last_matching_checkpoint),
            "differences": differences,
        })
    });
    let value = json!({
        "seed": report.seed,
        "records": report.records,
        "applied_actions": report.applied_actions,
        "compared_observations": report.compared_observations,
        "dropped_room_entries": report.dropped_room_entries,
        "uncovered_fields": report.uncovered_fields,
        "recorded_terminal": report.recorded_terminal,
        "simulated_terminal": report.simulated_terminal,
        "resumes": report.resumes,
        "rewound_actions": report.rewound_actions,
        "matched": report.matched(),
        "divergence": divergence,
    });
    serde_json::to_writer(&mut *output, &value)?;
    writeln!(output)?;
    Ok(())
}

fn write_text<W: Write>(
    report: &ReplayReport,
    output: &mut W,
    uncovered: bool,
) -> Result<(), CliError> {
    writeln!(output, "seed          {}", report.seed)?;
    writeln!(
        output,
        "records       {} read, {} actions applied",
        report.records, report.applied_actions
    )?;
    writeln!(
        output,
        "observations  {} compared, {}",
        report.compared_observations,
        if report.divergence.is_some() {
            "diverged"
        } else {
            "no differences"
        }
    )?;
    writeln!(
        output,
        "terminal      recorded {}, simulated {}",
        report.recorded_terminal.as_deref().unwrap_or("none"),
        report.simulated_terminal.as_deref().unwrap_or("none")
    )?;
    if report.resumes > 0 {
        writeln!(
            output,
            "resumed       {} reload{}, {} action{} between the last save and the quit rewound",
            report.resumes,
            if report.resumes == 1 { "" } else { "s" },
            report.rewound_actions,
            if report.rewound_actions == 1 { "" } else { "s" },
        )?;
    }
    if report.dropped_room_entries > 0 {
        writeln!(
            output,
            "between rooms {} recorded observations taken mid-walk, not compared",
            report.dropped_room_entries
        )?;
    }
    writeln!(
        output,
        "uncovered     {} recorded fields the renderer does not emit{}",
        report.uncovered_fields.len(),
        if uncovered {
            ""
        } else {
            " (--uncovered lists them)"
        }
    )?;
    if uncovered {
        for field in &report.uncovered_fields {
            writeln!(output, "                {field}")?;
        }
    }
    if let Some(error) = &report.divergence {
        write_divergence(error, output)?;
    } else if !report.matched() {
        writeln!(
            output,
            "\nMISMATCH: the recording and the simulation ended differently"
        )?;
    } else {
        writeln!(output, "\nMATCH: every compared observation agreed")?;
    }
    Ok(())
}

fn write_divergence<W: Write>(error: &AdapterError, output: &mut W) -> Result<(), CliError> {
    writeln!(
        output,
        "\nDIVERGENCE at record {} ({})",
        error.record_sequence, error.record_kind
    )?;
    let Some(divergence) = &error.divergence else {
        writeln!(output, "  {}", error.detail)?;
        return Ok(());
    };
    match divergence.last_matching_checkpoint {
        Some(sequence) => writeln!(output, "  last matching checkpoint: record {sequence}")?,
        None => writeln!(output, "  no observation matched before this one")?,
    }
    let count = divergence.differences.len();
    writeln!(
        output,
        "  {count} differing field{}",
        if count == 1 { "" } else { "s" }
    )?;
    for difference in divergence.differences.iter().take(DIFFERENCE_LIMIT) {
        writeln!(output, "    {}", difference.pointer)?;
        writeln!(
            output,
            "      recorded  {}",
            render_side(difference.expected.as_ref())
        )?;
        writeln!(
            output,
            "      simulated {}",
            render_side(difference.actual.as_ref())
        )?;
    }
    if let Some(rest) = divergence
        .differences
        .len()
        .checked_sub(DIFFERENCE_LIMIT)
        .filter(|rest| *rest > 0)
    {
        writeln!(output, "    ... and {rest} more (use --json for all)")?;
    }
    Ok(())
}

fn render_side(value: Option<&Value>) -> String {
    value.map_or_else(|| "nothing".to_owned(), ToString::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_incomplete_prefix_makes_no_terminal_claim() {
        let report = ReplayReport {
            seed: "PREFIX".to_owned(),
            records: 1,
            applied_actions: 1,
            compared_observations: 1,
            dropped_room_entries: 0,
            uncovered_fields: Vec::new(),
            recorded_terminal: Some("*".to_owned()),
            simulated_terminal: Some("DEFEAT".to_owned()),
            resumes: 0,
            rewound_actions: 0,
            divergence: None,
        };
        assert!(report.matched());
    }
}
