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
    /// Stats from the last decoded observation, including observations after
    /// divergence. These describe the recording, never simulated state.
    pub recorded_end: Option<Value>,
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
    let input = sts2_replay::decode_input(input)?;
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
    // Standard mode applies boss discovery; custom mode keeps the rolled boss.
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
    let mut recorded_end = None;
    for event in &mut parser {
        let event = event.map_err(|error| CliError::Replay(error.to_string()))?;
        let Event::Record(record) = event else {
            continue;
        };
        records += 1;
        if record.kind == "state.observe" {
            recorded_end = observation_end(&record.payload);
        }
        // Keep decoding and validating the recording to summarize its actual
        // end, but preserve the simulator and counters at the first mismatch.
        if divergence.is_some() {
            continue;
        }
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
                continue;
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
        recorded_end,
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

fn observation_end(payload: &Value) -> Option<Value> {
    let state = payload.get("state")?;
    let player = state.get("players")?.get(0)?;
    Some(json!({
        "floor": state.get("run")?.get("total_floor")?.as_u64()?,
        "hp": player.get("current_hp")?.as_i64()?,
        "max_hp": player.get("max_hp")?.as_i64()?,
        "deck": player.get("deck")?.as_array()?.len(),
        "relics": player.get("relics")?.as_array()?.len(),
    }))
}

/// Generates from the recorded unlocks and run-start profile inputs. Legacy
/// recordings omit act discovery; those assume every act was discovered.
/// These are generation inputs, never a repair of simulated state.
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
    let profile = match first.payload.pointer("/data/profile") {
        Some(profile) => serde_json::from_value(profile.clone()).map_err(|error| {
            CliError::Replay(format!("invalid_replay: profile inputs: {error}"))
        })?,
        None => sts2_core::ProfileInputs {
            discovered_acts: sts2_content::implemented_act_ids(),
        },
    };
    let preset = sts2_core::UnlockPresetManifest {
        schema_version: 1,
        preset_id: "recorded_profile".to_owned(),
        compatibility_id: sts2_core::PINNED_COMPATIBILITY_ID.to_owned(),
        unlocks,
        profile,
    };
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
        "recorded_end": report.recorded_end,
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
    use std::fmt::Write as _;

    fn profile_start() -> sts2_replay::Record {
        let preset = sts2_core::UnlockPresetManifest::pinned().unwrap();
        sts2_replay::Record {
            sequence: 1,
            kind: "run.start".to_owned(),
            payload: json!({"data":{"state":{"players":[{
                "character_model":"CHARACTER.IRONCLAD", "unlocks":preset.unlocks
            }]}}}),
        }
    }

    #[test]
    fn recorded_encounter_history_generates_an_unregistered_profile() {
        let mut start = profile_start();
        start.payload["data"]["state"]["players"][0]["unlocks"]["encounters_seen"] =
            json!(["ENCOUNTER.WATERFALL_GIANT_BOSS"]);
        let sim = recorded_run("X7JFABM2QS", 10, sts2_engine::RunMode::Standard, &start).unwrap();
        let run = sim.state().run.as_ref().unwrap();
        assert_eq!(run.acts[0].boss.to_string(), "ENCOUNTER.SOUL_FYSH_BOSS");
        assert_eq!(
            run.acts[1].boss.to_string(),
            "ENCOUNTER.THE_INSATIABLE_BOSS"
        );
        assert_eq!(run.acts[2].boss.to_string(), "ENCOUNTER.QUEEN_BOSS");
        assert_ne!(run.acts[2].second_boss.as_ref(), Some(&run.acts[2].boss));
        assert_eq!(
            run.unlocks.encounters_seen,
            ["ENCOUNTER.WATERFALL_GIANT_BOSS"]
        );
        let restored: sts2_engine::RunConfig =
            serde_json::from_value(serde_json::to_value(&run.config).unwrap()).unwrap();
        restored.validate().unwrap();
        assert_eq!(restored, run.config);
        assert_eq!(restored.unlock_profile.unwrap().unlocks, run.unlocks);
    }

    #[test]
    fn explicit_empty_discovery_is_distinct_from_legacy_all_discovered() {
        let mut start = profile_start();
        let legacy = recorded_run("N3Z9EE2CXB", 0, sts2_engine::RunMode::Standard, &start).unwrap();
        assert_eq!(
            legacy.state().run.as_ref().unwrap().acts[0]
                .act_model
                .to_string(),
            "ACT.OVERGROWTH"
        );
        start.payload["data"]["profile"] = json!({"discovered_acts":[]});
        let undiscovered =
            recorded_run("N3Z9EE2CXB", 0, sts2_engine::RunMode::Standard, &start).unwrap();
        assert_eq!(
            undiscovered.state().run.as_ref().unwrap().acts[0]
                .act_model
                .to_string(),
            "ACT.UNDERDOCKS"
        );
        start.payload["data"]["profile"] = json!({"discovered_acts":["ACT.UNDERDOCKS"]});
        let discovered =
            recorded_run("N3Z9EE2CXB", 0, sts2_engine::RunMode::Standard, &start).unwrap();
        assert_eq!(
            discovered.state().run.as_ref().unwrap().acts,
            legacy.state().run.as_ref().unwrap().acts
        );
    }

    #[test]
    fn custom_runs_keep_the_rolled_bosses_despite_unseen_history() {
        let mut start = profile_start();
        let veteran = recorded_run("ABC", 10, sts2_engine::RunMode::Custom, &start).unwrap();
        start.payload["data"]["state"]["players"][0]["unlocks"]["encounters_seen"] = json!([]);
        let unseen = recorded_run("ABC", 10, sts2_engine::RunMode::Custom, &start).unwrap();
        assert_eq!(
            veteran.state().run.as_ref().unwrap().acts,
            unseen.state().run.as_ref().unwrap().acts
        );
    }

    #[test]
    fn malformed_profile_inputs_are_not_replaced_with_the_legacy_assumption() {
        for profile in [
            json!(null),
            json!({}),
            json!({"discovered_acts":null}),
            json!({"discovered_acts":["not an id"]}),
        ] {
            let mut start = profile_start();
            start.payload["data"]["profile"] = profile;
            let error = recorded_run("ABC", 0, sts2_engine::RunMode::Standard, &start)
                .err()
                .unwrap();
            assert!(error.to_string().contains("invalid_replay: profile inputs"));
        }
    }

    #[test]
    fn unsupported_content_profiles_fail_explicitly() {
        let mut start = profile_start();
        start.payload["data"]["state"]["players"][0]["unlocks"]["number_of_runs"] = json!(0);
        let error = recorded_run("ABC", 0, sts2_engine::RunMode::Standard, &start)
            .err()
            .unwrap();
        assert!(error.to_string().contains("first-run generation"));
        let mut start = profile_start();
        start.payload["data"]["state"]["players"][0]["unlocks"]["epochs"] = json!([]);
        let error = recorded_run("ABC", 0, sts2_engine::RunMode::Standard, &start)
            .err()
            .unwrap();
        assert!(error.to_string().contains("all pinned epochs"));
    }

    fn opening(version: u8) -> (String, Value) {
        let mut sim = sts2_content::standard_ironclad_run_at("ABC", 0).unwrap();
        let start = sts2_replay::observation::render(&sim);
        let mut text = format!(
            "[Format \"STS2PGN\"]\n[FormatVersion \"{version}\"]\n\
             [RecorderVersion \"test\"]\n[RunId \"synthetic\"]\n\
             [GameVersion \"v0.107.1\"]\n[GameCommit \"59260271\"]\n\
             [ModelIdHash \"3954186980\"]\n[Seed \"ABC\"]\n\
             [Mode \"STANDARD\"]\n[Ascension \"0\"]\n\
             [Players \"[{{\\\"slot\\\":\\\"p0\\\",\\\"character\\\":\\\"IRONCLAD\\\"}}]\"]\n\
             [Mods \"[]\"]\n[Result \"*\"]\n\n"
        );
        writeln!(text, "1 run.start {}", json!({"data":{"state":start}})).unwrap();
        let action = sim
            .legal_actions()
            .iter()
            .find(|action| matches!(action, sts2_engine::Action::ChooseEvent { .. }))
            .unwrap()
            .clone();
        let sts2_engine::Action::ChooseEvent {
            index,
            ref option_id,
        } = action
        else {
            unreachable!()
        };
        writeln!(
            text,
            "2 event.choose {}",
            json!({"data":{"option_index":index,"option_id":option_id}})
        )
        .unwrap();
        sim.step(action).unwrap();
        (text, sts2_replay::observation::render(&sim))
    }

    fn observe(text: &mut String, sequence: u64, state: &Value) {
        writeln!(
            text,
            "{sequence} state.observe {}",
            json!({
                "after":sequence-1,"context":"action.settled","projection_version":1,
                "digest":sts2_replay::projection_digest(state).unwrap(),"state":state
            })
        )
        .unwrap();
    }

    fn diverging_run(version: u8) -> String {
        let (mut text, mut state) = opening(version);
        state["players"][0]["current_hp"] = json!(1);
        observe(&mut text, 3, &state);
        // This input settles observation 3 and stops the simulator. Later
        // actions must not run, but the resumed recording must still decode.
        text.push_str("4 event.advance {}\n5 combat.play_card {}\n6 run.resume {}\n");
        state["players"][0]["current_hp"] = json!(11);
        observe(&mut text, 7, &state);
        state["players"][0]["current_hp"] = json!(7);
        state["run"]["total_floor"] = json!(20);
        if version == 1 {
            observe(&mut text, 8, &state);
        } else {
            writeln!(
                text,
                "8 state.delta {}",
                json!({
                    "base":7,
                    "patch":[["set",["players"],state["players"]],["set",["run","total_floor"],20]],
                    "observation":{"after":7,"context":"action.settled","projection_version":1,
                        "digest":sts2_replay::projection_digest(&state).unwrap()}
                })
            )
            .unwrap();
        }
        text
    }

    #[test]
    fn final_stats_follow_deltas_and_reload_boundaries_after_first_divergence() {
        for version in [1, 2] {
            let report = replay(diverging_run(version).as_bytes()).unwrap();
            assert!(!report.matched());
            assert_eq!(report.divergence.as_ref().unwrap().record_sequence, 3);
            assert_eq!(report.applied_actions, 1);
            assert_eq!(report.compared_observations, 0);
            assert_eq!(report.resumes, 0);
            assert_eq!(report.records, 8);
            let end = report.recorded_end.as_ref().unwrap();
            assert_eq!(end["floor"], 20);
            assert_eq!(end["hp"], 7);
            let mut json = Vec::new();
            write_report(&report, &mut json, true, false).unwrap();
            let json: Value = serde_json::from_slice(&json).unwrap();
            assert_eq!(&json["recorded_end"], end);
        }
    }

    #[test]
    fn invalid_delta_after_divergence_is_still_an_error() {
        let text = diverging_run(2).replace("\"base\":7", "\"base\":999");
        assert!(
            replay(text.as_bytes())
                .unwrap_err()
                .to_string()
                .contains("invalid_delta_base")
        );
    }

    #[test]
    fn an_unusable_final_observation_does_not_reuse_earlier_stats() {
        let mut text = diverging_run(2);
        observe(&mut text, 9, &json!({"scope":"unknown"}));
        let report = replay(text.as_bytes()).unwrap();
        assert!(report.divergence.is_some());
        assert!(report.recorded_end.is_none());
    }

    #[test]
    fn matched_and_observation_free_runs_report_their_recorded_end() {
        for version in [1, 2] {
            let (mut text, state) = opening(version);
            let report = replay(text.as_bytes()).unwrap();
            assert!(report.matched());
            assert!(report.recorded_end.is_none());
            observe(&mut text, 3, &state);
            let report = replay(text.as_bytes()).unwrap();
            assert!(report.matched());
            assert_eq!(
                report.recorded_end,
                observation_end(&json!({"state":state}))
            );
        }
    }

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
            recorded_end: None,
            simulated_terminal: Some("DEFEAT".to_owned()),
            resumes: 0,
            rewound_actions: 0,
            divergence: None,
        };
        assert!(report.matched());
    }
}
