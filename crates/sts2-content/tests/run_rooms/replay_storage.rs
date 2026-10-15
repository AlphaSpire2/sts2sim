//! Storage variants of synthetic room scenarios; no game recordings are used.

use std::fmt::Write as _;
use std::io::{Cursor, Write};

use flate2::{Compression, write::GzEncoder};
use serde_json::{Value, json};
use sts2_replay::{AdapterError, Event, Parser, Record, ReplayAdapter, decode_input, observation};

fn record(sequence: u64, kind: &str, payload: Value) -> Record {
    Record {
        sequence,
        kind: kind.into(),
        payload,
    }
}

fn observe(sequence: u64, after: u64, context: &str, state: &Value) -> Record {
    record(
        sequence,
        "state.observe",
        json!({"after":after,"context":context,"state":state,
        "projection_version":1,"digest":sts2_replay::projection_digest(state).unwrap()}),
    )
}

fn encode(records: &[Record], version: u8, deltas: bool, gzip: bool) -> Vec<u8> {
    let mut output =
        format!("[Format \"STS2PGN\"]\n[FormatVersion \"{version}\"]\n[Result \"*\"]\n\n");
    let mut baseline: Option<(u64, Value)> = None;
    for record in records {
        let mut wire = record.clone();
        if matches!(
            record.kind.as_str(),
            "run.start" | "run.resume" | "room.enter" | "act.enter"
        ) {
            baseline = None;
        }
        if record.kind == "state.observe" {
            let state = &record.payload["state"];
            if let Some((base, previous)) = &baseline {
                if deltas {
                    // Replace changed top-level fields; unchanged fields stay only in the base.
                    let mut patch = Vec::new();
                    for (key, value) in state.as_object().unwrap() {
                        if previous.get(key) != Some(value) {
                            patch.push(json!(["set", [key], value]));
                        }
                    }
                    for key in previous.as_object().unwrap().keys() {
                        if state.get(key).is_none() {
                            patch.push(json!(["remove", [key]]));
                        }
                    }
                    let mut envelope = record.payload.clone();
                    envelope.as_object_mut().unwrap().remove("state");
                    wire.kind = "state.delta".into();
                    wire.payload = json!({"base":base,"observation":envelope,"patch":patch});
                }
            }
            baseline = Some((record.sequence, state.clone()));
        }
        writeln!(output, "{} {} {}", wire.sequence, wire.kind, wire.payload).unwrap();
    }
    if gzip {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(output.as_bytes()).unwrap();
        encoder.finish().unwrap()
    } else {
        output.into_bytes()
    }
}

fn replay(input: &[u8]) -> (ReplayAdapter, Option<AdapterError>) {
    let mut adapter = ReplayAdapter::new(super::generated_run()).verifying_observations();
    for event in Parser::new(decode_input(Cursor::new(input)).unwrap()).unwrap() {
        if let Event::Record(record) = event.unwrap() {
            if let Err(error) = adapter.accept(&record) {
                return (adapter, Some(error));
            }
        }
    }
    let error = adapter.verify_pending_observation().err();
    (adapter, error)
}

#[test]
fn repeated_resumes_rewind_the_same_actions_with_full_delta_and_gzip_input() {
    let mut source = ReplayAdapter::new(super::generated_run());
    let mut records = vec![
        record(1, "run.start", json!({})),
        record(2, "map.choose", json!({"data":{"col":4,"row":1}})),
    ];
    for record in &records {
        source.accept(record).unwrap();
    }
    records.push(observe(
        3,
        2,
        "action.settled",
        &observation::render(source.simulator()),
    ));
    for (action_sequence, resume_sequence) in [(4, 6), (8, 10)] {
        let action = record(action_sequence, "combat.end_turn", json!({"data":{}}));
        source.accept(&action).unwrap();
        records.push(action);
        records.push(observe(
            action_sequence + 1,
            action_sequence,
            "action.settled",
            &observation::render(source.simulator()),
        ));
        let resume = record(resume_sequence, "run.resume", resume_data(false));
        source.accept(&resume).unwrap();
        records.push(resume);
        records.push(observe(
            resume_sequence + 1,
            resume_sequence,
            "run.resume",
            &observation::render(source.simulator()),
        ));
    }
    let expected = observation::render(source.simulator());
    let (reference, error) = replay(&encode(&records, 1, false, false));
    assert!(error.is_none(), "{error:?}");
    assert_eq!(reference.resumes(), 2);
    assert_eq!(reference.rewound_actions(), 2);
    assert_eq!(reference.applied_actions(), 3);
    assert!(reference.compared_observations() > 0);
    assert_eq!(observation::render(reference.simulator()), expected);
    for (version, deltas, gzip) in [
        (1, false, true),
        (2, false, false),
        (2, true, false),
        (2, true, true),
    ] {
        let (actual, error) = replay(&encode(&records, version, deltas, gzip));
        assert!(error.is_none(), "{error:?}");
        assert_eq!(actual.resumes(), reference.resumes());
        assert_eq!(actual.rewound_actions(), reference.rewound_actions());
        assert_eq!(actual.applied_actions(), reference.applied_actions());
        assert_eq!(
            actual.compared_observations(),
            reference.compared_observations()
        );
        assert_eq!(
            actual.dropped_room_entries(),
            reference.dropped_room_entries()
        );
        assert_eq!(actual.uncovered_fields(), reference.uncovered_fields());
        assert_eq!(observation::render(actual.simulator()), expected);
        assert_eq!(
            actual.simulator().state().terminal,
            reference.simulator().state().terminal
        );
    }
}

fn resume_data(phase_two: bool) -> Value {
    json!({"data":{"combat_runtime":{"ending_player_turn_phase_two":phase_two}}})
}

#[test]
fn a_resume_restores_the_same_state_whatever_runtime_block_the_recorder_wrote() {
    let payloads = [
        json!({}),
        json!({"data":{}}),
        resume_data(false),
        resume_data(true),
        json!({"data":{"combat_runtime":null}}),
        json!({"data":{"combat_runtime":{"ending_player_turn_phase_two":"false"}}}),
    ];
    let mut reference = None;
    for context in ["action.settled", "combat.enemy_turn.start"] {
        for payload in &payloads {
            let mut adapter = ReplayAdapter::new(super::generated_run()).verifying_observations();
            adapter
                .accept(&record(1, "map.choose", json!({"data":{"col":4,"row":1}})))
                .unwrap();
            let entrance = adapter.simulator().state().clone();
            adapter
                .accept(&record(2, "combat.end_turn", json!({})))
                .unwrap();
            let state = observation::render(adapter.simulator());
            adapter.accept(&observe(3, 2, context, &state)).unwrap();
            adapter
                .accept(&record(4, "run.resume", payload.clone()))
                .unwrap();

            let restored = adapter.simulator().state();
            assert_eq!(restored.run.as_ref().unwrap().reloads, 1);
            assert_eq!(restored.rng, entrance.rng, "resume does not consume RNG");
            assert_eq!(restored.player_rng, entrance.player_rng);
            let combat = restored.combat.as_ref().unwrap();
            assert_eq!(combat.round, 1);
            assert_eq!(
                combat.player.energy,
                entrance.combat.as_ref().unwrap().player.energy
            );
            assert_eq!(
                restored.run_player.current_hp,
                entrance.run_player.current_hp
            );
            assert_eq!(adapter.resumes(), 1);
            assert_eq!(adapter.rewound_actions(), 1);
            assert_eq!(adapter.actions_since_save(), 0);
            let snapshot = adapter.simulator().snapshot();
            assert_eq!(reference.get_or_insert_with(|| snapshot.clone()), &snapshot);
        }
    }
}

#[test]
fn recorded_turn_handover_flags_are_never_compared() {
    let mut adapter = ReplayAdapter::new(super::generated_run()).verifying_observations();
    adapter
        .accept(&record(1, "map.choose", json!({"data":{"col":4,"row":1}})))
        .unwrap();
    let mut state = observation::render(adapter.simulator());
    state["combat"]["ending_player_turn_phase_one"] = json!(true);
    state["combat"]["ending_player_turn_phase_two"] = json!(true);
    adapter
        .accept(&observe(2, 1, "combat.player_turn.start", &state))
        .unwrap();
    adapter.verify_pending_observation().unwrap();
    assert_eq!(adapter.compared_observations(), 1);
    for flag in ["phase_one", "phase_two"] {
        assert!(
            adapter
                .uncovered_fields()
                .contains(&format!("/combat/ending_player_turn_{flag}")),
            "a recorded flag is listed as uncovered rather than passed"
        );
    }
}

#[test]
fn an_earlier_state_divergence_is_reported_before_a_resume() {
    let mut adapter = ReplayAdapter::new(super::generated_run()).verifying_observations();
    adapter
        .accept(&record(1, "map.choose", json!({"data":{"col":4,"row":1}})))
        .unwrap();
    let mut wrong = observation::render(adapter.simulator());
    wrong["players"][0]["gold"] = json!(123_456);
    adapter
        .accept(&observe(2, 1, "action.settled", &wrong))
        .unwrap();
    let error = adapter
        .accept(&record(3, "run.resume", json!({})))
        .unwrap_err();
    assert_eq!(error.code, sts2_replay::AdapterErrorCode::StateDivergence);
    assert_eq!(error.record_sequence, 2);
    assert_eq!(adapter.resumes(), 0);
}

#[test]
fn an_unchanged_field_in_a_delta_still_produces_the_same_first_divergence() {
    let mut source = super::generated_run();
    super::choose_map(&mut source, 4);
    let mut state = observation::render(&source);
    state["players"][0]["gold"] = json!(123_456);
    let records = [
        record(1, "map.choose", json!({"data":{"col":4,"row":1}})),
        observe(2, 1, "run.start", &state),
        observe(3, 1, "action.settled", &state),
        record(4, "event.advance", json!({})),
    ];
    let (_, error) = replay(&encode(&records, 1, false, false));
    let expected = error.unwrap();
    assert_eq!(expected.record_sequence, 3);
    assert!(
        expected
            .divergence
            .as_ref()
            .unwrap()
            .differences
            .iter()
            .any(|difference| difference.pointer.ends_with("/gold"))
    );
    for (deltas, gzip) in [(false, false), (true, false), (true, true)] {
        let (_, error) = replay(&encode(&records, 2, deltas, gzip));
        let actual = error.unwrap();
        assert_eq!(actual.record_sequence, expected.record_sequence);
        assert_eq!(actual.code, expected.code);
        assert_eq!(actual.divergence, expected.divergence);
    }
}
