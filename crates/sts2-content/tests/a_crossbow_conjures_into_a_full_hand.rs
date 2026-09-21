//! A card conjured into a full hand lands in the discard, and whatever the
//! conjuring did to it goes with it.
//!
//! A crossbow conjures one of its owner's attacks into the hand as every turn
//! opens, free for that turn. A hand already holding ten cards has no room,
//! so the game puts the attack in the discard instead — still free. The
//! engine once looked for the conjured card at the end of the hand, so the
//! free cost landed on whatever card happened to be there and the attack in
//! the discard kept its printed cost.

use sts2_engine::{Action, CardFingerprint, PileName, ScenarioBuilder, Simulator};

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str) -> CardFingerprint {
    let mut fingerprint = CardFingerprint::base(id(model_id));
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

/// A crossbow and a runic pyramid, five Defends in hand and five more to
/// draw: the pyramid keeps the five, the draw brings five, and the hand is
/// full as the crossbow fires.
fn a_crossbow_over_a_hand_about_to_fill() -> Simulator {
    let mut scenario = ScenarioBuilder::new("CROSSBOW", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(1, 1)
        .relics(&[id("RELIC.CROSSBOW"), id("RELIC.RUNIC_PYRAMID")])
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE");
    for index in 0..5 {
        scenario = scenario.card(PileName::Hand, index, card("CARD.DEFEND_IRONCLAD"), 1);
    }
    for index in 5..10 {
        scenario = scenario.card(PileName::Draw, index, card("CARD.DEFEND_IRONCLAD"), 1);
    }
    scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

fn end_the_turn(simulator: &mut Simulator) {
    let end_turn = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::EndTurn { .. }))
        .cloned()
        .expect("the turn can be ended");
    simulator.step(end_turn).expect("the turn ends");
}

#[test]
fn a_crossbows_attack_conjured_into_a_full_hand_is_free_in_the_discard() {
    let mut simulator = a_crossbow_over_a_hand_about_to_fill();
    end_the_turn(&mut simulator);
    let piles = &simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles;
    assert_eq!(piles.hand.len(), 10, "the hand is full");
    assert!(
        piles.hand.iter().all(|card| card.energy_cost == 1),
        "no card in the hand was made free: {:?}",
        piles
            .hand
            .iter()
            .map(|card| (card.fingerprint.model_id.to_string(), card.energy_cost))
            .collect::<Vec<_>>()
    );
    let [conjured] = piles.discard.as_slice() else {
        panic!(
            "the conjured attack alone is in the discard: {:?}",
            piles.discard
        );
    };
    assert_ne!(conjured.fingerprint.model_id, id("CARD.DEFEND_IRONCLAD"));
    assert_eq!(
        conjured.energy_cost, 0,
        "the attack is free where it landed"
    );
}
