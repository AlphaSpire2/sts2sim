//! `resolve_combat_won` settles a live fight through the ordinary victory
//! path: rewards roll and open, nothing on the enemy side is left standing,
//! the player's hit points land where the caller set them, and the run walks
//! on. Everything else — no combat, a settled fight, hit points outside the
//! accepted range — is refused without moving the state.

use sts2_core::UnlockPresetManifest;
use sts2_engine::{Action, ErrorCode, Simulator};

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

fn in_live_combat(simulator: &Simulator) -> bool {
    simulator
        .state()
        .combat
        .as_ref()
        .is_some_and(|combat| combat.in_progress && !combat.has_ended && !combat.rewards_open)
}

/// Walks a fresh run to the first decision inside a live fight.
fn run_at_a_fight() -> Simulator {
    let mut simulator = fresh_run();
    for _ in 0..200 {
        if in_live_combat(&simulator) {
            return simulator;
        }
        simulator.step_quietly(&next_action(&simulator)).unwrap();
    }
    panic!("the seeded walk never entered a fight");
}

#[test]
fn a_forced_win_settles_the_fight_and_the_run_walks_on() {
    let mut simulator = run_at_a_fight();
    let player_hp = 10;
    simulator.resolve_combat_won(player_hp).unwrap();

    let combat = simulator.state().combat.as_ref().expect("rewards stand");
    assert!(combat.has_ended && combat.rewards_open);
    assert!(
        combat
            .creatures
            .iter()
            .all(|creature| creature.id == combat.player.creature_id || !creature.alive()),
        "nothing on the enemy side is left standing"
    );
    let set = combat
        .reward_set
        .as_ref()
        .expect("the reward set is rolled");
    assert!(!set.rewards.is_empty(), "a hallway fight pays something");
    // The victory hooks run before the run reads the creature back, so the
    // starter relic's heal may sit on top of the injected value.
    let landed = simulator.state().run_player.current_hp;
    assert!(
        landed >= player_hp && landed <= simulator.state().run_player.max_hp,
        "the run carries the injected hit points (a victory heal may add): {landed}"
    );

    // The run proceeds through the rewards to the next map choice.
    for _ in 0..40 {
        if simulator.state().combat.is_none() {
            break;
        }
        simulator.step_quietly(&next_action(&simulator)).unwrap();
    }
    assert!(simulator.state().combat.is_none(), "the rewards close");
    assert!(simulator.state().terminal.is_none());
    assert!(!simulator.legal_actions().is_empty());
}

#[test]
fn hit_points_outside_the_accepted_range_are_refused_and_move_nothing() {
    let mut simulator = run_at_a_fight();
    let key = simulator.state_key().unwrap();
    let current = simulator
        .state()
        .combat
        .as_ref()
        .and_then(|combat| {
            combat
                .creatures
                .iter()
                .find(|creature| creature.id == combat.player.creature_id)
        })
        .map(|creature| creature.current_hp)
        .expect("the player stands in the fight");
    for hp in [0, -3, current + 1] {
        let error = simulator.resolve_combat_won(hp).unwrap_err();
        assert_eq!(error.code, ErrorCode::IllegalAction);
    }
    assert_eq!(key, simulator.state_key().unwrap(), "nothing moved");
}

#[test]
fn a_run_outside_a_live_fight_is_refused() {
    // At the run's opening decision there is no combat at all.
    let mut fresh = fresh_run();
    let error = fresh.resolve_combat_won(1).unwrap_err();
    assert_eq!(error.code, ErrorCode::WrongDecisionContext);

    // A settled fight — rewards open — is no longer live.
    let mut settled = run_at_a_fight();
    settled.resolve_combat_won(5).unwrap();
    let error = settled.resolve_combat_won(5).unwrap_err();
    assert_eq!(error.code, ErrorCode::WrongDecisionContext);
}
