//! A fake merchant turns on the player once.
//!
//! A foul potion thrown at him buys the rug, whatever is still stocked on his
//! shelf, and the fight he starts without leaving his page. The page is handed
//! off to that fight and is not handed back — the room is left by the map once
//! the fight is done — so there is no second merchant on it to throw a second
//! potion at, however many foul potions the belt still holds. The fixture
//! walks in with two.

use sts2_core::UnlockPresetManifest;
use sts2_engine::{Action, ActiveRoom, DecisionContext, Simulator};

const SEED: &str = "NLD6VZXP94";
const FOUL: &str = "POTION.FOUL_POTION";

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn fresh_run() -> Simulator {
    let preset = UnlockPresetManifest::pinned().expect("pinned unlock preset");
    let character = "CHARACTER.IRONCLAD".parse().expect("static model ID");
    sts2_content::standard_run_on_preset_at(SEED, &character, &preset, 0).expect("the run builds")
}

fn next_action(simulator: &Simulator) -> Action {
    simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseCards { cards, .. } if !cards.is_empty()))
        .or_else(|| simulator.legal_actions().first())
        .cloned()
        .expect("a live run offers an action")
}

/// The run standing on an event's page, whichever page the seed reaches first.
fn run_at_an_event() -> Simulator {
    let mut simulator = fresh_run();
    for _ in 0..300 {
        if matches!(simulator.decision(), DecisionContext::Event { .. })
            && simulator
                .state()
                .run
                .as_ref()
                .is_some_and(|run| matches!(run.active_room, Some(ActiveRoom::Event { .. })))
        {
            return simulator;
        }
        simulator
            .step_quietly(&next_action(&simulator))
            .expect("the walk applies");
    }
    panic!("the seed reaches an event inside three hundred decisions");
}

/// That page rewritten as the fake merchant's, with two foul potions on the
/// belt — one for the throw, one to try it again with.
fn a_fake_merchant_with_two_foul_potions() -> Simulator {
    let simulator = run_at_an_event();
    let mut state = simulator.state().clone();
    let run = state.run.as_mut().expect("the run stands");
    let Some(ActiveRoom::Event {
        model_id, options, ..
    }) = run.active_room.as_mut()
    else {
        panic!("the walk stopped on an event page");
    };
    *model_id = id("EVENT.FAKE_MERCHANT");
    // The fake merchant's page carries no options of its own: the shelf and
    // the throw are all there is to answer it with.
    options.clear();
    state.run_player.potions.fill(None);
    state.run_player.potions[0] = Some(id(FOUL));
    state.run_player.potions[1] = Some(id(FOUL));
    Simulator::from_scenario(state, sts2_content::standard_registry()).expect("the state rebuilds")
}

fn the_throw(simulator: &Simulator) -> Option<Action> {
    simulator
        .legal_actions()
        .iter()
        .find(
            |action| matches!(action, Action::UsePotion { model_id, .. } if *model_id == id(FOUL)),
        )
        .cloned()
}

/// The throw is on the belt while the merchant is still standing.
#[test]
fn the_throw_is_offered_while_the_merchant_stands() {
    let simulator = a_fake_merchant_with_two_foul_potions();
    assert!(
        the_throw(&simulator).is_some(),
        "a merchant to throw it at makes the throw legal: {:?}",
        simulator.legal_actions()
    );
}

/// Taking it starts his fight, and the page is behind that fight from then on.
#[test]
fn the_throw_starts_the_fight_it_pays_for() {
    let mut simulator = a_fake_merchant_with_two_foul_potions();
    let throw = the_throw(&simulator).expect("the merchant is there");
    simulator.step(throw).expect("the throw applies");

    assert!(
        matches!(simulator.decision(), DecisionContext::CombatPriority { .. }),
        "the merchant turned: {:?}",
        simulator.decision()
    );
    let run = simulator.state().run.as_ref().expect("the run stands");
    assert!(
        run.pushed_room.is_some(),
        "and the page handed off to the fight"
    );
}

/// With the fight won and its rewards on the table, the second foul potion
/// has nothing to be thrown at.
#[test]
fn the_second_throw_is_not_offered_over_the_won_fights_rewards() {
    let mut simulator = a_fake_merchant_with_two_foul_potions();
    let throw = the_throw(&simulator).expect("the merchant is there");
    simulator.step(throw).expect("the throw applies");
    simulator
        .resolve_combat_won(40)
        .expect("the fight settles as won");

    assert!(
        matches!(simulator.decision(), DecisionContext::Rewards { .. }),
        "the fight paid out: {:?}",
        simulator.decision()
    );
    assert!(
        simulator
            .state()
            .run_player
            .potions
            .iter()
            .flatten()
            .count()
            == 1,
        "the second foul potion is still held"
    );
    assert_eq!(
        the_throw(&simulator),
        None,
        "but there is no merchant left to throw it at: {:?}",
        simulator.legal_actions()
    );
}
