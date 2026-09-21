//! How a run ends when it is won.
//!
//! The corpus has one victory and it walks this whole path, so what is here is
//! the same path in a scenario: the last act's boss goes down, the player walks
//! on, meets the Architect, clicks through his line, and proceeds past him.

use sts2_engine::{Action, DecisionContext, MapPointType, RunPhase, RunResult, Simulator};

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

/// The last act's boss falls, the player walks on, and the walk is not the win:
/// it is the Architect's room, with his dialogue in front of the option that
/// ends the run.
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
        ["THE_ARCHITECT.dialogue.0"],
        "a first win rolls the one-line dialogue, which is one click before the proceed"
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
    step(
        &mut simulator,
        |action| matches!(action, Action::ChooseEvent { option_id, .. } if option_id == "THE_ARCHITECT.dialogue.0"),
    );

    let DecisionContext::Event { options, .. } = simulator.decision().clone() else {
        panic!("the dialogue puts up the next page rather than leaving the room");
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
