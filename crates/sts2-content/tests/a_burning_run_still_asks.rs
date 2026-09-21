//! A screen reached through a card a body burns still asks, and the rest of
//! the run burns behind the answer.
//!
//! The build walks the cards a body exhausts on one stack and announces each
//! burn with the player's choice context. Dark Embrace answers a burn with a
//! draw, Hellraiser auto-plays the Strike that draw dealt, and a Seeker
//! Strike's own body offers three of the draw pile: the screen opens where
//! the build opens it, and the cards the run had not burned yet burn once
//! it is answered.

use sts2_engine::{
    Action, CardFingerprint, CardHandle, DecisionContext, PileName, PowerType, ScenarioBuilder,
    Simulator,
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

fn fight() -> ScenarioBuilder {
    ScenarioBuilder::new("BURNRUN", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 200, 200, "SHRINKER_MOVE")
        .power(0, id("POWER.DARK_EMBRACE_POWER"), 1, PowerType::Buff)
        .power(0, id("POWER.HELLRAISER_POWER"), 1, PowerType::Buff)
}

fn play(simulator: &mut Simulator, model_id: &str) {
    let wanted = id(model_id);
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::PlayCard { card, .. } if card.fingerprint.model_id == wanted)
        })
        .cloned()
        .unwrap_or_else(|| panic!("{model_id} is playable"));
    simulator.step(action).expect("the play applies");
}

/// The standing card screen, or a panic naming what stood instead.
fn the_screen(simulator: &Simulator) -> (sts2_core::ChoiceId, Vec<CardHandle>) {
    let DecisionContext::ChooseCards {
        choice_id,
        candidates,
        ..
    } = simulator.decision()
    else {
        panic!(
            "a card screen should be standing, found {:?}",
            simulator.decision()
        );
    };
    (*choice_id, candidates.clone())
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

/// A True Grit burns the one other card in hand; the Seeker Strike that burn
/// drew is auto-played, and its pick stands.
#[test]
fn a_seeker_strike_auto_played_off_a_bodys_burn_opens_its_screen() {
    let mut simulator = fight()
        .card(PileName::Hand, 0, card("CARD.TRUE_GRIT"), 0)
        .card(PileName::Hand, 1, card("CARD.STRIKE_IRONCLAD"), 0)
        .card(PileName::Draw, 2, card("CARD.SEEKER_STRIKE"), 0)
        .card(PileName::Draw, 3, card("CARD.BASH"), 0)
        .card(PileName::Draw, 4, card("CARD.DEFEND_IRONCLAD"), 0)
        .card(PileName::Draw, 5, card("CARD.IRON_WAVE"), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.TRUE_GRIT");

    let (choice_id, candidates) = the_screen(&simulator);
    assert_eq!(candidates.len(), 3, "three of the draw pile are offered");
    let taken = candidates[0].clone();
    simulator
        .step(Action::ChooseCards {
            choice_id,
            cards: vec![taken.clone()],
        })
        .expect("the pick applies");

    assert!(
        simulator
            .state()
            .combat
            .as_ref()
            .expect("the fight stands")
            .player
            .piles
            .hand
            .iter()
            .any(|held| held.id == taken.card_id),
        "the pick landed in the hand"
    );
    assert_eq!(
        pile(&simulator, PileName::Exhaust),
        vec!["CARD.STRIKE_IRONCLAD".to_owned()],
        "the strike burned"
    );
    assert!(
        pile(&simulator, PileName::Discard).contains(&"CARD.TRUE_GRIT".to_owned()),
        "the true grit was put down"
    );
    assert!(
        pile(&simulator, PileName::Discard).contains(&"CARD.SEEKER_STRIKE".to_owned()),
        "the auto-played seeker strike was put down"
    );
}

/// A Purity burns two chosen cards. The first burn opens the Seeker Strike's
/// screen; the second waits for the answer and burns after it, and Dark
/// Embrace draws for it too — and once more for the Purity itself, which
/// burns as it is put down.
#[test]
fn the_rest_of_a_burning_run_burns_behind_the_answer() {
    let mut simulator = fight()
        .card(PileName::Hand, 0, card("CARD.PURITY"), 0)
        .card(PileName::Hand, 1, card("CARD.STRIKE_IRONCLAD"), 0)
        .card(PileName::Hand, 2, card("CARD.DEFEND_IRONCLAD"), 0)
        .card(PileName::Draw, 3, card("CARD.SEEKER_STRIKE"), 0)
        .card(PileName::Draw, 4, card("CARD.BASH"), 0)
        .card(PileName::Draw, 5, card("CARD.IRON_WAVE"), 0)
        .card(PileName::Draw, 6, card("CARD.THUNDERCLAP"), 0)
        .card(PileName::Draw, 7, card("CARD.ANGER"), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.PURITY");

    // Purity's own screen: both other cards of the hand, to burn.
    let (choice_id, candidates) = the_screen(&simulator);
    assert_eq!(
        candidates.len(),
        2,
        "the purity offers the rest of the hand"
    );
    simulator
        .step(Action::ChooseCards {
            choice_id,
            cards: candidates,
        })
        .expect("the burn choice applies and leaves the seeker strike's screen standing");

    // The first burn drew the Seeker Strike, which was played, and its pick
    // stands while the second card has not burned yet.
    let (choice_id, candidates) = the_screen(&simulator);
    assert_eq!(candidates.len(), 3, "three of the draw pile are offered");
    let exhausted_so_far = pile(&simulator, PileName::Exhaust);
    assert_eq!(
        exhausted_so_far.len(),
        1,
        "one card burned before the screen, found {exhausted_so_far:?}"
    );
    let taken = candidates[0].clone();
    simulator
        .step(Action::ChooseCards {
            choice_id,
            cards: vec![taken.clone()],
        })
        .expect("the pick applies");

    let mut exhausted = pile(&simulator, PileName::Exhaust);
    exhausted.sort();
    assert_eq!(
        exhausted,
        vec![
            "CARD.DEFEND_IRONCLAD".to_owned(),
            "CARD.PURITY".to_owned(),
            "CARD.STRIKE_IRONCLAD".to_owned()
        ],
        "both chosen cards burned, and the purity itself"
    );
    let hand = pile(&simulator, PileName::Hand);
    assert_eq!(
        hand.len(),
        3,
        "the pick, the second burn's draw, and the purity's own burn's draw \
         are held, found {hand:?}"
    );
    assert_eq!(
        pile(&simulator, PileName::Draw).len(),
        1,
        "five less the seeker strike, the pick, and two more draws"
    );
    assert_eq!(
        simulator
            .state()
            .combat
            .as_ref()
            .expect("the fight stands")
            .player
            .turn,
        1,
        "nothing else ran out"
    );
}
