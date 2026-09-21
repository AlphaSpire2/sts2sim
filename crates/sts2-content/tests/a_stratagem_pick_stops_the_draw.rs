//! A draw waits on the screen its shuffle opened.
//!
//! The build's draw loop shuffles before each card it needs and waits for the
//! shuffle's listeners before it takes the card. A Stratagem is one: it asks
//! which of the shuffled cards to take back, and the draw stands where it is
//! until the pick is answered — the cards it still owes come off the pile
//! behind the pick, not before it.

use sts2_engine::{
    Action, CardFingerprint, DecisionContext, PileName, PowerType, ScenarioBuilder, Simulator,
};
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

fn end_turn(simulator: &mut Simulator) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::EndTurn { .. }))
        .cloned()
        .expect("the turn can end");
    simulator.step(action).expect("ending the turn applies");
}

#[test]
fn a_stratagem_pick_stops_the_hand_draw_where_the_shuffle_stands() {
    // One card in the draw pile and six in the discard: the turn's hand draw
    // takes the one, shuffles for the second, and the shuffle wakes the
    // Stratagem.
    let mut simulator = ScenarioBuilder::new("STRATAGEM", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 60, 60, "SHRINKER_MOVE")
        .power(0, id("POWER.STRATAGEM_POWER"), 1, PowerType::Buff)
        .card(PileName::Draw, 0, card("CARD.DEFEND_IRONCLAD"), 0)
        .card(PileName::Discard, 1, card("CARD.STRIKE_IRONCLAD"), 0)
        .card(PileName::Discard, 2, card("CARD.STRIKE_IRONCLAD"), 0)
        .card(PileName::Discard, 3, card("CARD.STRIKE_IRONCLAD"), 0)
        .card(PileName::Discard, 4, card("CARD.STRIKE_IRONCLAD"), 0)
        .card(PileName::Discard, 5, card("CARD.STRIKE_IRONCLAD"), 0)
        .card(PileName::Discard, 6, card("CARD.STRIKE_IRONCLAD"), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    end_turn(&mut simulator);

    // The pick stands with the draw paused behind it: one card drawn, and the
    // whole shuffled pile on offer.
    let DecisionContext::ChooseCards {
        choice_id,
        candidates,
        ..
    } = simulator.decision().clone()
    else {
        panic!("the stratagem asks: {:?}", simulator.decision());
    };
    assert_eq!(
        pile(&simulator, PileName::Hand),
        vec!["CARD.DEFEND_IRONCLAD".to_owned()],
        "the first card was drawn before the shuffle, and nothing after it"
    );
    assert_eq!(
        pile(&simulator, PileName::Draw).len(),
        6,
        "the shuffled pile is whole"
    );
    assert_eq!(
        candidates.len(),
        6,
        "the pick is over the whole shuffled pile"
    );
    let taken = candidates[0].clone();
    simulator
        .step(Action::ChooseCards {
            choice_id,
            cards: vec![taken],
        })
        .expect("the pick applies");

    // The picked card joins the hand, then the draw finishes its four, and
    // the turn is the player's.
    assert_eq!(
        pile(&simulator, PileName::Hand).len(),
        6,
        "one drawn, one picked, four more drawn"
    );
    assert_eq!(
        pile(&simulator, PileName::Draw).len(),
        1,
        "six, less the pick and four draws"
    );
    assert!(
        simulator
            .legal_actions()
            .iter()
            .any(|action| matches!(action, Action::EndTurn { .. })),
        "the turn is the player's: {:?}",
        simulator.decision()
    );
    assert_eq!(
        simulator
            .state()
            .combat
            .as_ref()
            .expect("the fight stands")
            .player
            .turn,
        2
    );
}
