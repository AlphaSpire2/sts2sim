//! Proceeding past a rewards screen is not leaving the room.
//!
//! Proceeding from a terminal rewards screen opens the map and does nothing
//! else. The set stays on the run's own stack for replay observations, and
//! only leaving the room — the next point walked onto — retires it. The
//! recorder reads that stack, so a recording carries the set through every
//! observation between the two.
//!
//! Reward decisions are final at Proceed. Map travel cannot reopen rewards
//! or repeatedly proceed; replay alone can consume a redundant map-open record.

use serde_json::json;
use sts2_engine::{Action, DecisionContext, ErrorCode, Simulator};
use sts2_replay::{AdapterErrorCode, AdapterOutcome, Record, ReplayAdapter};

const WALK_SEED: &str = "NLD6VZXP94";

fn fresh_run() -> Simulator {
    let preset = sts2_core::UnlockPresetManifest::pinned().unwrap();
    let character = "CHARACTER.IRONCLAD".parse().unwrap();
    sts2_content::standard_run_on_preset_at(WALK_SEED, &character, &preset, 0).unwrap()
}

/// The first legal action, preferring a non-empty card pick so a skippable
/// reward cannot re-offer itself.
fn next_action(simulator: &Simulator) -> Action {
    simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseCards { cards, .. } if !cards.is_empty()))
        .or_else(|| simulator.legal_actions().first())
        .cloned()
        .expect("a live run offers an action")
}

fn walk_until(mut simulator: Simulator, stop: impl Fn(&Simulator) -> bool) -> Simulator {
    for _ in 0..300 {
        if stop(&simulator) {
            return simulator;
        }
        let action = next_action(&simulator);
        simulator.step_quietly(&action).unwrap();
    }
    panic!("the seed reaches the stop inside three hundred decisions");
}

fn take(simulator: &mut Simulator, wanted: impl Fn(&Action) -> bool, name: &str) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| wanted(action))
        .cloned()
        .unwrap_or_else(|| panic!("{name} was not legal at {:?}", simulator.decision()));
    simulator.step_quietly(&action).unwrap();
}

fn rewards(simulator: &Simulator) -> serde_json::Value {
    sts2_replay::observation::render(simulator)["choices"]["rewards"].clone()
}

#[test]
fn a_reward_set_survives_the_proceed_and_dies_on_the_next_point() {
    let mut simulator = walk_until(fresh_run(), |simulator| {
        simulator
            .state()
            .combat
            .as_ref()
            .is_some_and(|combat| combat.reward_set.is_some())
    });
    let offered = rewards(&simulator);
    assert_eq!(
        offered.as_array().map(Vec::len),
        Some(1),
        "the won fight's set is on the screen"
    );

    take(
        &mut simulator,
        |action| matches!(action, Action::Proceed),
        "proceed",
    );
    assert!(
        simulator.state().combat.is_none(),
        "the fight itself is torn down on the way out, as it always was"
    );
    assert_eq!(
        rewards(&simulator),
        offered,
        "proceeding opens the map; the set is still on the build's rewards stack, unchanged"
    );

    take(
        &mut simulator,
        |action| matches!(action, Action::ChooseMap { .. }),
        "a map point",
    );
    assert_eq!(
        rewards(&simulator).as_array().map(Vec::len),
        Some(0),
        "leaving the room retires it"
    );
}

fn offered_card_reward() -> (Simulator, Action) {
    let simulator = walk_until(fresh_run(), |simulator| {
        simulator.legal_actions().iter().any(|action| {
            matches!(action, Action::ClaimReward { fingerprint, .. } if !fingerprint.offered_cards.is_empty())
        })
    });
    let claim = simulator.legal_actions().iter().find(|action| {
        matches!(action, Action::ClaimReward { fingerprint, .. } if !fingerprint.offered_cards.is_empty())
    }).unwrap().clone();
    (simulator, claim)
}

fn collect_rewards(simulator: &mut Simulator) {
    while let Some(claim) = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ClaimReward { .. }))
        .cloned()
    {
        simulator.step_quietly(&claim).unwrap();
        if matches!(simulator.decision(), DecisionContext::ChooseCards { .. }) {
            take(
                simulator,
                |action| matches!(action, Action::ChooseCards { cards, .. } if !cards.is_empty()),
                "take a card",
            );
        }
    }
}

fn proceed_record(sequence: u64) -> Record {
    Record {
        sequence,
        kind: "room.proceed".into(),
        payload: json!({"data": {}}),
    }
}

fn claim_record(sequence: u64, claim: &Action) -> Record {
    let Action::ClaimReward {
        set_id,
        reward_index,
        fingerprint,
    } = claim
    else {
        panic!("expected a reward claim");
    };
    Record {
        sequence,
        kind: "reward.claim".into(),
        payload: json!({"data": {
            "set_id": set_id.get(), "reward_index": reward_index, "reward": fingerprint
        }}),
    }
}

#[test]
fn proceeding_finalizes_skipped_and_collected_rewards_before_map_travel() {
    for collect_all in [false, true] {
        let (mut simulator, claim) = offered_card_reward();
        let deck_size = simulator.state().run_player.deck.len();
        if collect_all {
            collect_rewards(&mut simulator);
            assert_eq!(simulator.state().run_player.deck.len(), deck_size + 1);
            assert!(
                simulator
                    .state()
                    .combat
                    .as_ref()
                    .unwrap()
                    .reward_set
                    .is_none()
            );
        } else {
            simulator.step(claim.clone()).unwrap();
            take(
                &mut simulator,
                |action| matches!(action, Action::ChooseCards { cards, .. } if cards.is_empty()),
                "skip the card",
            );
            assert_eq!(simulator.state().run_player.deck.len(), deck_size);
        }
        take(
            &mut simulator,
            |action| matches!(action, Action::Proceed),
            "leave rewards",
        );
        assert!(matches!(
            simulator.decision(),
            DecisionContext::MapNavigation { .. }
        ));
        assert!(
            !simulator
                .legal_actions()
                .iter()
                .any(|action| { matches!(action, Action::Proceed | Action::ClaimReward { .. }) })
        );
        let before = simulator.snapshot();
        for action in [Action::Proceed, claim] {
            assert_eq!(
                simulator.step(action.clone()).unwrap_err().code,
                ErrorCode::WrongDecisionContext
            );
            assert_eq!(
                simulator.step_quietly(&action).unwrap_err().code,
                ErrorCode::WrongDecisionContext
            );
            assert_eq!(
                simulator.snapshot(),
                before,
                "rejected actions change neither state nor RNG"
            );
        }
        take(
            &mut simulator,
            |action| matches!(action, Action::ChooseMap { .. }),
            "next room",
        );
        assert!(
            simulator
                .state()
                .run
                .as_ref()
                .unwrap()
                .dismissed_reward_set
                .is_none()
        );
    }
}

#[test]
fn replay_reopening_the_map_preserves_state_rng_and_observation_checks() {
    for collect_all in [false, true] {
        let (mut simulator, _) = offered_card_reward();
        if collect_all {
            collect_rewards(&mut simulator);
        }
        let mut adapter = ReplayAdapter::new(simulator).verifying_observations();
        assert!(matches!(
            adapter.accept(&proceed_record(1)).unwrap(),
            AdapterOutcome::Applied(_)
        ));
        let before = adapter.simulator().snapshot();
        let actions_since_save = adapter.actions_since_save();
        for sequence in 2..=3 {
            assert!(matches!(
                adapter.accept(&proceed_record(sequence)).unwrap(),
                AdapterOutcome::Ignored(_)
            ));
        }
        assert_eq!(adapter.simulator().snapshot(), before);
        assert_eq!(adapter.applied_actions(), 1);
        assert_eq!(adapter.actions_since_save(), actions_since_save);

        let state = sts2_replay::observation::render(adapter.simulator());
        adapter
            .accept(&Record {
                sequence: 4,
                kind: "state.observe".into(),
                payload: json!({"after": 3, "context": "action.settled", "state": state}),
            })
            .unwrap();
        adapter.accept(&proceed_record(5)).unwrap();
        assert_eq!(adapter.compared_observations(), 1);

        let mut incorrect = state;
        incorrect["players"][0]["gold"] = json!(999_999);
        adapter
            .accept(&Record {
                sequence: 6,
                kind: "state.observe".into(),
                payload: json!({"after": 5, "context": "action.settled", "state": incorrect}),
            })
            .unwrap();
        assert_eq!(
            adapter.accept(&proceed_record(7)).unwrap_err().code,
            AdapterErrorCode::StateDivergence
        );
        assert_eq!(adapter.simulator().snapshot(), before);
    }
}

#[test]
fn replay_cannot_collect_rewards_after_proceeding() {
    let (simulator, claim) = offered_card_reward();
    let mut normal = ReplayAdapter::new(simulator.clone());
    assert!(matches!(
        normal.accept(&claim_record(1, &claim)).unwrap(),
        AdapterOutcome::Applied(_)
    ));

    let mut adapter = ReplayAdapter::new(simulator);
    adapter.accept(&proceed_record(1)).unwrap();
    let before = adapter.simulator().snapshot();
    let error = adapter.accept(&claim_record(2, &claim)).unwrap_err();
    assert_eq!(error.code, AdapterErrorCode::UnsupportedContent);
    assert!(error.detail.contains("reward decisions are final"));
    assert_eq!(adapter.simulator().snapshot(), before);
    assert_eq!(adapter.applied_actions(), 1);
}

#[test]
fn replay_does_not_ignore_proceed_on_an_unrelated_map_or_during_combat() {
    let map = walk_until(fresh_run(), |simulator| {
        matches!(simulator.decision(), DecisionContext::MapNavigation { .. })
    });
    assert!(
        map.state()
            .run
            .as_ref()
            .unwrap()
            .dismissed_reward_set
            .is_none()
    );
    let combat = walk_until(map.clone(), |simulator| {
        simulator
            .state()
            .combat
            .as_ref()
            .is_some_and(|combat| !combat.has_ended)
    });
    for simulator in [map, combat] {
        let before = simulator.snapshot();
        let mut adapter = ReplayAdapter::new(simulator);
        assert_eq!(
            adapter.accept(&proceed_record(1)).unwrap_err().code,
            AdapterErrorCode::IllegalAction
        );
        assert_eq!(adapter.simulator().snapshot(), before);
        assert_eq!(adapter.applied_actions(), 0);
    }
}
