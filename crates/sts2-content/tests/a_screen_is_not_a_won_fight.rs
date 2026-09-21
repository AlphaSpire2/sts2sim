//! A loop whose body puts a screen up is not a loop that ended the fight.
//!
//! Every command answers one question on the way out: did the fight end doing
//! this? The loops — `Repeat`, `RepeatWithLast`, `RepeatPerKill`, `ForEachOrb`,
//! `ForEachCard`, and the orb passes — have two reasons to stop early, and
//! they answer them differently. A body that opened a selection screen is
//! reported as a pause, not as a body that won the fight.
//!
//! The distinction is the queue: a won fight clears what it still owed, and
//! the continuation the suspension has *just* put on that queue would go with
//! it. The settling would then finish a combat that is still standing, out
//! from under a screen the player has not answered.
//!
//! Catastrophe stands this up, because it is a `Repeat` whose body plays a
//! card off the draw pile: give it a card that opens a screen and the loop's
//! two reasons to stop meet on the first pass.

use sts2_engine::{Action, CardFingerprint, PileName, ScenarioBuilder, Simulator};

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str) -> CardFingerprint {
    let mut fingerprint = CardFingerprint::base(id(model_id));
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

/// A catastrophe in hand and a Neow's fury waiting in the draw pile for it to
/// find. The fury swings and then offers the discard pile back, so the card the
/// catastrophe plays for its first pass is one that puts a screen up.
///
/// The beetle is healthy: nothing here ends the fight.
fn a_catastrophe_over_a_screen() -> Simulator {
    ScenarioBuilder::new("CATA", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .card(PileName::Hand, 0, card("CARD.CATASTROPHE"), 0)
        .card(PileName::Draw, 1, card("CARD.NEOWS_FURY"), 0)
        .card(PileName::Discard, 2, card("CARD.STRIKE_IRONCLAD"), 0)
        .card(PileName::Discard, 3, card("CARD.DEFEND_IRONCLAD"), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

/// The loop reports the screen as a pause: the fight is still standing, the
/// continuation the suspension wrote onto the queue is still there, and the
/// screen is the fury's pick, answerable.
#[test]
fn a_catastrophe_that_plays_a_card_with_a_screen_has_not_won_the_fight() {
    let mut simulator = a_catastrophe_over_a_screen();
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(
                action,
                Action::PlayCard { card, .. }
                    if card.fingerprint.model_id == id("CARD.CATASTROPHE")
            )
        })
        .cloned()
        .expect("the catastrophe is offered");
    simulator.step(action).expect("the catastrophe applies");

    let combat = simulator.state().combat.as_ref().expect("the fight stands");
    assert!(
        combat.suspended_choice.is_some(),
        "the card the catastrophe played put a screen up"
    );
    assert!(
        combat.in_progress && !combat.has_ended,
        "a screen going up is not the fight ending"
    );
    assert!(
        !combat.rewards_open && combat.reward_set.is_none(),
        "nothing was won, so nothing is owed"
    );
    let beetle = combat
        .creatures
        .iter()
        .find(|creature| creature.id == sts2_core::CreatureCombatId::from(1))
        .expect("the beetle stands");
    assert!(beetle.alive(), "the beetle is still standing");
}

/// The rounds the screen deferred are owed, not dropped: a Catastrophe plays
/// two cards, and the screen the first one opened must not swallow the second.
///
/// In the build each round of the loop waits for the play before it to finish,
/// so the second round runs when the screen's answer lets the first play
/// finish. The strike
/// waiting in the draw pile is the witness: it stays put while the question
/// stands, and it has been played by the time the answer settles.
#[test]
fn a_catastrophe_still_owes_its_second_play_after_the_screen() {
    let mut simulator = ScenarioBuilder::new("CATA", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .card(PileName::Hand, 0, card("CARD.CATASTROPHE"), 0)
        .card(PileName::Draw, 1, card("CARD.NEOWS_FURY"), 0)
        .card(PileName::Draw, 4, card("CARD.STRIKE_IRONCLAD"), 0)
        .card(PileName::Discard, 2, card("CARD.STRIKE_IRONCLAD"), 0)
        .card(PileName::Discard, 3, card("CARD.DEFEND_IRONCLAD"), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(
                action,
                Action::PlayCard { card, .. }
                    if card.fingerprint.model_id == id("CARD.CATASTROPHE")
            )
        })
        .cloned()
        .expect("the catastrophe is offered");
    simulator.step(action).expect("the catastrophe applies");

    let combat = simulator.state().combat.as_ref().expect("the fight stands");
    assert!(
        combat.suspended_choice.is_some(),
        "the fury went first and its screen is up"
    );
    assert_eq!(
        combat.player.piles.draw.len(),
        1,
        "the strike waits in the draw pile while the question stands"
    );

    let answer = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseCards { cards, .. } if cards.len() == 1))
        .cloned()
        .expect("the screen offers a card back");
    simulator.step(answer).expect("the answer applies");

    let combat = simulator.state().combat.as_ref().expect("the fight stands");
    assert!(
        combat.player.piles.draw.is_empty(),
        "the second round ran once the answer let the first play finish"
    );
    assert!(
        combat
            .player
            .piles
            .discard
            .iter()
            .any(
                |held| held.fingerprint.model_id == id("CARD.STRIKE_IRONCLAD")
                    && held.id == sts2_core::CardInstanceId::from(4)
            ),
        "and the strike it played is down in the discard pile"
    );
}
