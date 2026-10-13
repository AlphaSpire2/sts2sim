//! A potion can be thrown away while the card reward a won fight paid is
//! still waiting to be answered.
//!
//! Out of a fight a discard is an action on the player's queue, and the queue
//! holds an action back only behind one of the player's own that is waiting
//! for an answer. The card reward is not such an action: the reward screen
//! stands beside the queue. So the discard runs at once, the reward screen
//! stays up behind it, and answering the screen afterwards takes the card as
//! it would have without the discard.

use sts2_core::UnlockPresetManifest;
use sts2_engine::{Action, ChoiceScreen, DecisionContext, Simulator};

const SEED: &str = "NLD6VZXP94";

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

/// A fresh run with a potion that cannot be drunk out of a fight on the belt,
/// so the only thing the belt offers outside one is throwing it away.
fn fresh_run_with_a_potion() -> Simulator {
    let preset = UnlockPresetManifest::pinned().unwrap();
    let character = "CHARACTER.IRONCLAD".parse().unwrap();
    let run = sts2_content::standard_run_on_preset_at(SEED, &character, &preset, 0).unwrap();
    let mut state = run.state().clone();
    state.run_player.potions[0] = Some(id("POTION.BLOCK_POTION"));
    Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run builds")
}

/// The first legal action, preferring a non-empty card pick so a skippable
/// reward cannot re-offer itself, and never the belt.
fn next_action(simulator: &Simulator) -> Action {
    let legal = simulator.legal_actions();
    legal
        .iter()
        .find(|action| matches!(action, Action::ChooseCards { cards, .. } if !cards.is_empty()))
        .or_else(|| {
            legal.iter().find(|action| {
                !matches!(
                    action,
                    Action::UsePotion { .. } | Action::DiscardPotion { .. }
                )
            })
        })
        .cloned()
        .expect("a live run offers an action")
}

/// Walks the run into its first fight, wins it, and opens the card reward it
/// paid.
fn a_card_reward_after_a_won_fight() -> Simulator {
    let mut simulator = fresh_run_with_a_potion();
    for _ in 0..200 {
        let in_a_fight = simulator
            .state()
            .combat
            .as_ref()
            .is_some_and(|combat| combat.in_progress && !combat.has_ended);
        if in_a_fight {
            break;
        }
        simulator.step_quietly(&next_action(&simulator)).unwrap();
    }
    simulator.resolve_combat_won(40).expect("the fight is won");
    let claim = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::ClaimReward { fingerprint, .. }
                if fingerprint.reward_type == "card")
        })
        .cloned()
        .expect("a hallway fight pays a card reward");
    simulator.step(claim).expect("the card reward opens");
    assert!(
        simulator.state().run_player.potions[0].is_some(),
        "the walk left the potion on the belt"
    );
    simulator
}

fn reward_choice(simulator: &Simulator) -> sts2_core::ChoiceId {
    match simulator.decision() {
        DecisionContext::ChooseCards {
            choice_id,
            screen: ChoiceScreen::CardReward,
            ..
        } => *choice_id,
        other => panic!("expected the card reward, found {other:?}"),
    }
}

#[test]
fn a_potion_is_thrown_away_beside_the_card_reward() {
    let mut simulator = a_card_reward_after_a_won_fight();
    let choice = reward_choice(&simulator);
    let discard = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::DiscardPotion { slot: 0, .. }))
        .cloned()
        .expect("the belt is in reach beside the card reward");
    simulator
        .step(discard)
        .expect("the discard applies beside the screen");

    assert!(simulator.state().run_player.potions[0].is_none());
    assert_eq!(
        reward_choice(&simulator),
        choice,
        "the card reward the potion was thrown away beside is still standing"
    );
}

#[test]
fn the_card_reward_behind_the_discard_still_answers() {
    let mut simulator = a_card_reward_after_a_won_fight();
    let discard = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::DiscardPotion { slot: 0, .. }))
        .cloned()
        .expect("the belt is in reach beside the card reward");
    simulator.step(discard).unwrap();
    let deck_before = simulator.state().run_player.deck.len();
    let pick = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseCards { cards, .. } if !cards.is_empty()))
        .cloned()
        .expect("the card reward still offers its cards");
    simulator.step(pick).expect("the card is taken");
    assert_eq!(
        simulator.state().run_player.deck.len(),
        deck_before + 1,
        "the card went into the deck"
    );
}
