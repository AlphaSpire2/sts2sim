//! A fight that is over or ending draws nothing, and rolls nothing to draw it.
//!
//! A draw opens by asking whether the fight is over or ending, and asks the
//! same question again inside its own loop, once per card.
//! The second test is the one that matters: a listener on a card already drawn
//! can be what ends the fight, and everything the call still owed goes with it —
//! including the reshuffle the next card would have needed.
//!
//! A hellraiser is the case that shows it: it plays every Strike its owner
//! draws the moment it is drawn, so the opening hand draw of a turn can be the
//! thing that
//! wins the fight, partway through itself.
//!
//! The cost of getting it wrong is not the hand, which the end of the fight
//! clears either way. It is the shuffle stream: cards drawn past the end of a
//! fight empty the draw pile, the reshuffle rolls, and every roll the run makes
//! after that is off by the width of a deck.

use sts2_engine::{Action, CardFingerprint, PileName, PowerType, ScenarioBuilder, Simulator};
use sts2_rng::RunStream;

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str) -> CardFingerprint {
    let mut fingerprint = CardFingerprint::base(id(model_id));
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

/// A beetle with four health, a hellraiser, and a draw pile too short for a
/// hand: the strike on top of it is the first card of the turn's draw, and the
/// three cards behind it cannot be reached without a reshuffle.
fn a_hellraiser_over_a_short_draw_pile() -> Simulator {
    let mut scenario = ScenarioBuilder::new("HELL", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(1, 1)
        .rng_counter(RunStream::Shuffle, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 4, 39, "SHRINKER_MOVE")
        .power(0, id("POWER.HELLRAISER_POWER"), 1, PowerType::Buff)
        .card(PileName::Draw, 0, card("CARD.STRIKE_IRONCLAD"), 0);
    for slot in 1..8 {
        scenario = scenario.card(PileName::Discard, slot, card("CARD.DEFEND_IRONCLAD"), 0);
    }
    scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

#[test]
fn a_hand_draw_that_wins_the_fight_stops_where_it_won_it() {
    let mut simulator = a_hellraiser_over_a_short_draw_pile();
    let end_turn = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::EndTurn { .. }))
        .cloned()
        .expect("the turn can be ended");
    simulator.step(end_turn).expect("the turn ends");

    let combat = simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is held");
    assert!(
        combat.has_ended && combat.rewards_open,
        "the strike the hellraiser played as the hand was drawn won the fight"
    );
    assert_eq!(
        simulator.state().rng.stream(RunStream::Shuffle).counter(),
        0,
        "and the four cards it still owed were never drawn, so nothing reshuffled"
    );
}
