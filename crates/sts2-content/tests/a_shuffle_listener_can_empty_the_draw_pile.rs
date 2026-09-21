//! A draw whose shuffle hands every shuffled card to a listener draws nothing.
//!
//! The build's draw loop asks whether a draw is still possible after the
//! shuffle it just ran, because a listener on the shuffle may have taken the
//! cards the shuffle put down. A Stratagem with as many stacks as the discard
//! pile holds is one: the shuffle fills the draw pile and the power empties it
//! again, and the draw ends where it stands.

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

fn pile(simulator: &Simulator, pile: PileName) -> Vec<String> {
    let piles = &simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight stands")
        .player
        .piles;
    let cards = match pile {
        PileName::Hand => &piles.hand,
        PileName::Draw => &piles.draw,
        PileName::Discard => &piles.discard,
        PileName::Exhaust => &piles.exhaust,
        PileName::Play => &piles.play,
    };
    cards
        .iter()
        .map(|card| card.fingerprint.model_id.to_string())
        .collect()
}

#[test]
fn a_draw_ends_when_the_shuffle_listener_took_every_card() {
    // The draw pile is empty and the discard pile holds one card, so the
    // Pommel Strike's draw shuffles first. The one-stack Stratagem answers the
    // shuffle by handing that one card to the hand: the draw pile is empty
    // again and the draw has nothing left to deal.
    let mut simulator = ScenarioBuilder::new("EMPTIED", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 60, 60, "SHRINKER_MOVE")
        .power(0, id("POWER.STRATAGEM_POWER"), 1, PowerType::Buff)
        .card(PileName::Hand, 0, card("CARD.POMMEL_STRIKE"), 0)
        .card(PileName::Discard, 1, card("CARD.STRIKE_IRONCLAD"), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    let wanted = id("CARD.POMMEL_STRIKE");
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::PlayCard { card, .. } if card.fingerprint.model_id == wanted)
        })
        .cloned()
        .expect("the pommel strike is playable");
    simulator.step(action).expect("the play applies");

    assert_eq!(
        pile(&simulator, PileName::Hand),
        vec!["CARD.STRIKE_IRONCLAD".to_owned()],
        "the stratagem handed the shuffled strike to the hand and the draw dealt nothing"
    );
    assert!(
        pile(&simulator, PileName::Draw).is_empty(),
        "nothing is left to draw"
    );
    assert_eq!(
        pile(&simulator, PileName::Discard),
        vec!["CARD.POMMEL_STRIKE".to_owned()],
        "the pommel strike was put down"
    );
}
