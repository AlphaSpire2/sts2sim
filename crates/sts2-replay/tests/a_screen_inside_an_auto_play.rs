//! A screen the build shows while a turn start is still playing cards
//! automatically, and answers inside that play, is a stage of the turn start.
//!
//! The recorder writes a snapshot as such a screen is offered, and the turn
//! start's own snapshot after it. When the turn start's snapshot comes late,
//! behind the next input, the screen's snapshot is the last state written
//! before that input. This engine never stops on that screen, so the snapshot
//! is passed over rather than compared. A screen offered once the play phase
//! has begun is compared, and so is any other snapshot of the auto-play.

use serde_json::{Value, json};
use sts2_replay::{AdapterError, AdapterErrorCode, Record, ReplayAdapter};

fn end_turn(sequence: u64, turn: u32) -> Record {
    Record {
        sequence,
        kind: "combat.end_turn".into(),
        payload: json!({"data": {"turn": turn}}),
    }
}

fn observe(sequence: u64, after: u64, context: &str, state: &Value) -> Record {
    Record {
        sequence,
        kind: "state.observe".into(),
        payload: json!({
            "after": after,
            "context": context,
            "state": state,
            "projection_version": 1,
            "digest": sts2_replay::projection_digest(state).unwrap(),
        }),
    }
}

/// The first turn ended, then the second turn's snapshot, written under
/// `context` in `phase` and differing from what this engine holds, and the
/// player ending that turn too.
fn replay_a_snapshot_then_end_the_turn(
    context: &str,
    phase: &str,
) -> (ReplayAdapter, Result<(), AdapterError>) {
    let simulator = sts2_content::shrinker_beetle_fixture().expect("the fixture builds");
    let mut adapter = ReplayAdapter::new(simulator).verifying_observations();
    adapter.accept(&end_turn(1, 1)).unwrap();
    let mut snapshot = sts2_replay::observation::render(adapter.simulator());
    assert!(
        snapshot.pointer("/combat/players/0/phase").is_some(),
        "the snapshot names the turn's phase"
    );
    snapshot["combat"]["players"][0]["phase"] = json!(phase);
    snapshot["players"][0]["gold"] = json!(4_321);
    adapter.accept(&observe(2, 1, context, &snapshot)).unwrap();
    let result = adapter.accept(&end_turn(3, 2)).map(|_| ());
    (adapter, result)
}

#[test]
fn a_screen_offered_during_the_turn_starts_auto_play_is_passed_over() {
    let (adapter, result) = replay_a_snapshot_then_end_the_turn("choice.offered", "auto_pre_play");
    result.expect("the screen's snapshot is a stage of the turn start");
    assert_eq!(adapter.compared_observations(), 0);
    assert_eq!(adapter.mid_handover_observations(), 1);
}

#[test]
fn a_screen_offered_once_the_play_phase_has_begun_is_compared() {
    let (_, result) = replay_a_snapshot_then_end_the_turn("choice.offered", "play");
    let error = result.expect_err("a screen in the play phase is compared");
    assert_eq!(error.code, AdapterErrorCode::StateDivergence);
}

#[test]
fn a_settled_snapshot_during_the_auto_play_is_compared() {
    let (_, result) = replay_a_snapshot_then_end_the_turn("action.settled", "auto_pre_play");
    let error = result.expect_err("only the screen's snapshot is passed over");
    assert_eq!(error.code, AdapterErrorCode::StateDivergence);
}
