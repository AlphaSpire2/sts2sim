//! Proceeding past a rewards screen is not leaving the room.
//!
//! Proceeding from a terminal rewards screen opens the map and does nothing
//! else. The set stays on the run's own stack of reward sets, claimable, and
//! only leaving the room — the next point walked onto — retires it. The
//! recorder reads that stack, so a recording carries the set through every
//! observation between the two.
//!
//! Anything that makes an observation in that window sees the set still
//! there. A potion discarded on the map is exactly that, and it is what the
//! fixture does between Proceed and the next point.

use sts2_engine::{Action, Simulator};

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

#[test]
fn a_skipped_card_reward_can_be_reopened_after_proceeding_to_the_map() {
    let mut simulator = walk_until(fresh_run(), |simulator| {
        simulator.legal_actions().iter().any(|action| {
            matches!(action, Action::ClaimReward { fingerprint, .. } if !fingerprint.offered_cards.is_empty())
        })
    });
    let claim = simulator.legal_actions().iter().find(|action| {
        matches!(action, Action::ClaimReward { fingerprint, .. } if !fingerprint.offered_cards.is_empty())
    }).unwrap().clone();
    simulator.step(claim.clone()).unwrap();
    take(
        &mut simulator,
        |action| matches!(action, Action::ChooseCards { cards, .. } if cards.is_empty()),
        "skip",
    );
    take(
        &mut simulator,
        |action| matches!(action, Action::Proceed),
        "proceed",
    );
    let deck_size = simulator.state().run_player.deck.len();
    assert!(simulator.legal_actions().contains(&claim));
    simulator.step(claim.clone()).unwrap();
    take(
        &mut simulator,
        |action| matches!(action, Action::ChooseCards { cards, .. } if cards.is_empty()),
        "skip again",
    );
    assert!(
        simulator.legal_actions().contains(&claim),
        "skipping still leaves the reward available"
    );
    simulator.step(claim.clone()).unwrap();
    take(
        &mut simulator,
        |action| matches!(action, Action::ChooseCards { cards, .. } if !cards.is_empty()),
        "take a card",
    );
    assert_eq!(simulator.state().run_player.deck.len(), deck_size + 1);
    assert!(
        !simulator.legal_actions().contains(&claim),
        "a reward cannot be taken twice"
    );
    while let Some(action) = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ClaimReward { .. }))
        .cloned()
    {
        simulator.step(action).unwrap();
    }
    assert!(
        simulator
            .state()
            .run
            .as_ref()
            .unwrap()
            .dismissed_reward_set
            .is_none()
    );
    take(
        &mut simulator,
        |action| matches!(action, Action::Proceed),
        "proceed after the last reward",
    );
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
