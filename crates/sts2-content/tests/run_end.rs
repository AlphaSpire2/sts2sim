//! How a run ends when it is won.
//!
//! The final boss leads to the Architect's Proceed action and then victory.
//! Cosmetic dialogue clicks do not alter gameplay or the victory boundary.

use serde_json::json;
use sts2_engine::{Action, DecisionContext, MapPointType, RunPhase, RunResult, Simulator};
use sts2_replay::{AdapterErrorCode, AdapterOutcome, Record, ReplayAdapter};

fn id(model_id: &str) -> sts2_core::ModelId {
    model_id.parse().expect("static model ID")
}

fn run_at_the_final_boss() -> Simulator {
    sts2_content::final_boss_fixture().expect("the final-boss fixture builds")
}

fn step(simulator: &mut Simulator, wanted: impl Fn(&Action) -> bool) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| wanted(action))
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "no legal action matched at {:?}: {:?}",
                simulator.decision(),
                simulator.legal_actions()
            )
        });
    simulator.step(action).expect("the action applies");
}

fn architect_choice(sequence: u64, option_id: &str) -> Record {
    Record {
        sequence,
        kind: "event.choose".into(),
        payload: json!({"data": {
            "event_id": "EVENT.THE_ARCHITECT",
            "option_id": option_id,
            "option_index": 0,
            "is_proceed": false,
        }}),
    }
}

fn advance_act(sequence: u64) -> Record {
    Record {
        sequence,
        kind: "act.next".into(),
        payload: json!({"data": {}}),
    }
}

fn replay_at_architect() -> ReplayAdapter {
    let mut simulator = run_at_the_final_boss();
    step(&mut simulator, |action| {
        matches!(action, Action::PlayCard { .. })
    });
    let mut adapter = ReplayAdapter::new(simulator).verifying_observations();
    adapter.accept(&advance_act(1)).unwrap();
    adapter
}

#[test]
fn replay_abstracts_architect_dialogue_without_changing_state_or_winning_early() {
    for pages in [0, 1, 2, 4] {
        let mut adapter = replay_at_architect();
        let before = adapter.simulator().snapshot();
        let mut sequence = 2;
        for page in 0..=pages {
            let option = if page == pages {
                "PROCEED".to_owned()
            } else {
                format!("THE_ARCHITECT.dialogue.{page}")
            };
            let mut state = sts2_replay::observation::render(adapter.simulator());
            state["choices"]["events"][0]["options"] = if page == 0 {
                json!([])
            } else {
                json!([{"index": 0, "option_id": option, "is_proceed": false}])
            };
            adapter.accept(&Record {
                sequence,
                kind: "state.observe".into(),
                payload: json!({"after": sequence - 1, "context": "action.settled", "state": state}),
            }).unwrap();
            sequence += 1;
            let outcome = adapter
                .accept(&architect_choice(sequence, &option))
                .unwrap();
            sequence += 1;
            if page < pages {
                assert!(matches!(outcome, AdapterOutcome::Ignored(_)));
                assert_eq!(adapter.simulator().snapshot(), before);
                assert_eq!(adapter.applied_actions(), 1);
            } else {
                assert!(matches!(outcome, AdapterOutcome::Applied(_)));
            }
        }
        assert_eq!(adapter.compared_observations(), pages + 1);
        assert_eq!(adapter.cleared_event_pages(), 0);
        assert!(
            adapter
                .uncovered_fields()
                .contains("/choices/events/*/options")
        );
        assert!(adapter.simulator().state().terminal.is_none());
        adapter.accept(&advance_act(sequence)).unwrap();
        assert_eq!(
            adapter.simulator().state().terminal,
            Some(RunResult::Victory)
        );
        assert_eq!(adapter.applied_actions(), 3);
    }
}

#[test]
fn architect_dialogue_still_checks_health_rng_and_event_identity() {
    for pointer in [
        "/players/0/current_hp",
        "/players/0/rng/counters/rewards",
        "/choices/events/0/event_id",
    ] {
        let mut adapter = replay_at_architect();
        let before = adapter.simulator().snapshot();
        let mut state = sts2_replay::observation::render(adapter.simulator());
        state["choices"]["events"][0]["options"] = json!([]);
        *state.pointer_mut(pointer).expect("the field is rendered") =
            if pointer.ends_with("event_id") {
                json!("EVENT.NEOW")
            } else {
                json!(999_999)
            };
        adapter
            .accept(&Record {
                sequence: 2,
                kind: "state.observe".into(),
                payload: json!({"after": 1, "context": "action.settled", "state": state}),
            })
            .unwrap();
        let error = adapter
            .accept(&architect_choice(3, "THE_ARCHITECT.dialogue.0"))
            .unwrap_err();
        assert_eq!(error.code, AdapterErrorCode::StateDivergence, "{pointer}");
        assert_eq!(adapter.simulator().snapshot(), before);
    }
}

#[test]
fn architect_dialogue_requires_the_correct_room_and_option_shape() {
    for (field, value) in [
        ("event_id", json!("EVENT.NEOW")),
        ("option_index", json!(1)),
        ("is_proceed", json!(true)),
        ("option_id", json!("THE_ARCHITECT.dialogue.")),
        ("option_id", json!("THE_ARCHITECT.dialogue.-1")),
        ("option_id", json!("UNKNOWN")),
    ] {
        let mut adapter = replay_at_architect();
        let mut record = architect_choice(2, "THE_ARCHITECT.dialogue.0");
        record.payload["data"][field] = value;
        assert_eq!(
            adapter.accept(&record).unwrap_err().code,
            AdapterErrorCode::IllegalAction
        );
    }
    let mut elsewhere = ReplayAdapter::new(run_at_the_final_boss());
    assert_eq!(
        elsewhere
            .accept(&architect_choice(1, "THE_ARCHITECT.dialogue.0"))
            .unwrap_err()
            .code,
        AdapterErrorCode::IllegalAction
    );
    let mut finished = replay_at_architect();
    finished.accept(&architect_choice(2, "PROCEED")).unwrap();
    assert_eq!(
        finished
            .accept(&architect_choice(3, "THE_ARCHITECT.dialogue.0"))
            .unwrap_err()
            .code,
        AdapterErrorCode::IllegalAction
    );
}

/// The last act's boss falls, the player walks on, and the walk is not the win:
/// it is the Architect's room, with the Proceed action that readies victory.
#[test]
fn the_last_boss_leads_to_the_architect_rather_than_straight_to_a_victory() {
    let mut simulator = run_at_the_final_boss();
    step(&mut simulator, |action| {
        matches!(action, Action::PlayCard { .. })
    });
    assert!(
        simulator.state().combat.is_some(),
        "the fight is still standing until it is walked out of"
    );
    // The room generates no rewards at all at the last act's boss, so the set
    // that goes up is empty — and an empty set is
    // complete the moment it is offered, so it closes itself. What is left is
    // the room, and at a boss walking out of it is the act change.
    assert!(matches!(simulator.decision(), DecisionContext::RoomProceed));
    assert!(
        simulator
            .state()
            .combat
            .as_ref()
            .is_some_and(|combat| combat.reward_set.is_none()),
        "the last boss pays nothing, and the screen does not wait to be dismissed"
    );
    assert!(
        simulator
            .legal_actions()
            .iter()
            .any(|action| matches!(action, Action::AdvanceAct))
    );
    assert!(simulator.state().terminal.is_none());

    let events_visited_before = simulator.state().run.as_ref().unwrap().acts[2].events_visited;
    step(&mut simulator, |action| {
        matches!(action, Action::AdvanceAct)
    });
    let run = simulator.state().run.as_ref().unwrap();
    assert!(
        simulator.state().terminal.is_none(),
        "walking on from the last boss opens the Architect's room, it does not win the run"
    );
    assert_eq!(run.phase, RunPhase::Room);
    assert_eq!(run.current_room_model, Some(id("EVENT.THE_ARCHITECT")));
    assert_eq!(run.current_room_type, Some(MapPointType::Ancient));
    assert_eq!(
        run.current_act, 2,
        "the run stays in its last act; there is no act to move to"
    );
    // Entering the room marks a room of the act's own family visited, so the
    // act counts one more event; no event is pulled off the act's list, so the
    // run does not record the Architect among the events it has seen.
    assert_eq!(run.acts[2].events_visited, events_visited_before + 1);
    assert!(!run.visited_events.contains(&id("EVENT.THE_ARCHITECT")));
    // No map point was walked onto, so the boss point's history gains nothing.
    assert!(
        run.map_history
            .iter()
            .flat_map(|act| &act.points)
            .flat_map(|point| &point.rooms)
            .all(|room| room.model_id != Some(id("EVENT.THE_ARCHITECT")))
    );

    let DecisionContext::Event { model_id, options } = simulator.decision().clone() else {
        panic!(
            "the Architect's room is an event page: {:?}",
            simulator.decision()
        );
    };
    assert_eq!(model_id, id("EVENT.THE_ARCHITECT"));
    assert_eq!(
        options
            .iter()
            .map(|option| option.option_id.clone())
            .collect::<Vec<_>>(),
        ["PROCEED"],
        "cosmetic dialogue adds no simulator decisions"
    );
}

/// Proceeding past the Architect readies the player for an act change, and the
/// act change is what ends the run: winning at the Architect only sets the
/// player ready, the act change comes round again, and this time the current
/// room is the victory room.
///
/// It settles the way any won run settles. The fight is long gone, nothing
/// more is written down because the room is not a combat room, and the tidy-up
/// kills the player's *creature* — which is not the run's own health, because
/// the Architect's room stands no fight up for the player. The recorded
/// victory ends with the player still standing.
#[test]
fn proceeding_past_the_architect_wins_the_run_and_leaves_the_player_standing() {
    let mut simulator = run_at_the_final_boss();
    step(&mut simulator, |action| {
        matches!(action, Action::PlayCard { .. })
    });
    step(&mut simulator, |action| {
        matches!(action, Action::AdvanceAct)
    });
    let DecisionContext::Event { options, .. } = simulator.decision().clone() else {
        panic!("the Architect exposes Proceed before the run can finish");
    };
    assert_eq!(
        options
            .iter()
            .map(|option| option.option_id.clone())
            .collect::<Vec<_>>(),
        ["PROCEED"]
    );
    assert!(simulator.state().terminal.is_none());

    step(
        &mut simulator,
        |action| matches!(action, Action::ChooseEvent { option_id, .. } if option_id == "PROCEED"),
    );
    assert!(
        simulator.state().terminal.is_none(),
        "the proceed only readies the act change"
    );
    assert_eq!(
        simulator.state().run.as_ref().unwrap().phase,
        RunPhase::ActTransition
    );
    let health = simulator.state().run_player.current_hp;
    step(&mut simulator, |action| {
        matches!(action, Action::AdvanceAct)
    });

    let state = simulator.state();
    assert_eq!(state.terminal, Some(RunResult::Victory));
    assert_eq!(
        *simulator.decision(),
        DecisionContext::Terminal {
            result: RunResult::Victory
        }
    );
    assert!(
        simulator.legal_actions().is_empty(),
        "a run that has ended has nothing left to decide"
    );
    assert_eq!(
        state.run_player.current_hp, health,
        "the kill reaches a creature the Architect's room never stood up"
    );
    assert!(state.combat.is_none());
    let run = state.run.as_ref().expect("the won run is still described");
    assert!(run.pending.is_empty(), "nothing is left owed");
    assert!(run.suspended_choice.is_none());
    assert!(run.reward_set.is_none());
    // The run ends standing in the Architect's room rather than walking out of
    // it: the projected room stack is his.
    assert_eq!(run.current_room_model, Some(id("EVENT.THE_ARCHITECT")));
    assert!(matches!(
        run.active_room,
        Some(sts2_engine::ActiveRoom::Event { .. })
    ));
}
