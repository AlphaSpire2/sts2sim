//! The quiet step is `step` minus the chronicle and nothing else: the same
//! walk lands on the same states, the same illegal action is refused the same
//! way and leaves the state standing, and a later chronicled step still
//! carries its checkpoints.

use sts2_core::UnlockPresetManifest;
use sts2_engine::{Action, Simulator};

const SEED: &str = "NLD6VZXP94";

fn fresh_run() -> Simulator {
    let preset = UnlockPresetManifest::pinned().unwrap();
    let character = "CHARACTER.IRONCLAD".parse().unwrap();
    sts2_content::standard_run_on_preset_at(SEED, &character, &preset, 0).unwrap()
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

#[test]
fn a_quiet_walk_lands_on_the_states_a_chronicled_walk_does() {
    let mut chronicled = fresh_run();
    let mut quiet = fresh_run();
    for _ in 0..120 {
        if chronicled.state().terminal.is_some() {
            break;
        }
        let action = next_action(&chronicled);
        assert_eq!(chronicled.legal_actions(), quiet.legal_actions());
        let transition = chronicled.step(action.clone()).unwrap();
        quiet.step_quietly(&action).unwrap();
        assert_eq!(chronicled.state(), quiet.state());
        assert_eq!(transition.state_key, quiet.state_key().unwrap());
        assert_eq!(chronicled.decision(), quiet.decision());
    }
}

#[test]
fn a_refused_quiet_step_matches_the_chronicled_refusal_and_moves_nothing() {
    let mut chronicled = fresh_run();
    let mut quiet = fresh_run();
    let illegal = Action::ChooseMap {
        destination: sts2_engine::MapCoord { col: 7, row: 7 },
    };
    let before = quiet.state_key().unwrap();
    let chronicled_error = chronicled.step(illegal.clone()).unwrap_err();
    let quiet_error = quiet.step_quietly(&illegal).unwrap_err();
    assert_eq!(chronicled_error.code, quiet_error.code);
    assert_eq!(before, quiet.state_key().unwrap());
    assert_eq!(chronicled.state(), quiet.state());
}

#[test]
fn a_chronicled_step_after_a_quiet_one_still_carries_its_checkpoints() {
    let mut simulator = fresh_run();
    simulator.step_quietly(&next_action(&simulator)).unwrap();
    let transition = simulator.step(next_action(&simulator)).unwrap();
    assert!(
        !transition.checkpoints.is_empty(),
        "the quiet step must not leave checkpoint recording lowered"
    );
}
