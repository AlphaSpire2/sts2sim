//! A listener pass the build defers can still put a question to the player.
//!
//! The turn-cycle dispatch sites pause per listener and join later: each
//! listener's body is started, the site waits only for that body to pause, and
//! every paused body is joined at the site's own end. A body that asks parks
//! its screen on the action queue and the pass runs on.
//!
//! The pass banks the screen and the site's join opens it, so the player is
//! asked exactly where the build asks. A screen nested inside a deferred body
//! stands that up: a Stratagem's draw-pile pick reached through a shuffle the
//! deferred body's own draw caused.

use sts2_engine::{
    Action, CardFingerprint, DecisionContext, PileName, PowerType, ScenarioBuilder, Simulator,
};

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str) -> CardFingerprint {
    let mut fingerprint = CardFingerprint::base(id(model_id));
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
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

/// The standing card screen, or a panic naming what stood instead.
fn the_screen(
    simulator: &Simulator,
) -> (
    sts2_core::ChoiceId,
    usize,
    usize,
    Vec<sts2_engine::CardHandle>,
) {
    let DecisionContext::ChooseCards {
        choice_id,
        minimum,
        maximum,
        candidates,
        ..
    } = simulator.decision()
    else {
        panic!("no screen is standing: {:?}", simulator.decision());
    };
    (*choice_id, *minimum, *maximum, candidates.clone())
}

/// A simulator cloned at the suspended boundary and one stepped from the
/// original must walk to the same place, which pins the banked screens and
/// their owed continuations as honest state rather than live borrows.
fn assert_clone_resumes_identically(simulator: &Simulator, action: Action) {
    let mut original = simulator.clone();
    let mut copy = simulator.clone();
    assert_eq!(original.legal_actions(), copy.legal_actions());
    original.step(action.clone()).expect("the answer applies");
    copy.step(action).expect("the answer applies to the clone");
    assert_eq!(
        original.state_key().unwrap(),
        copy.state_key().unwrap(),
        "the two copies settle on the same state"
    );
}

/// The `FromCombatPile` shape: a screen nested inside a deferred body. A
/// Dazed burns up as the turn ends, so Dark Embrace owes a draw at
/// `AfterSideTurnEnd` — a deferred pass — and that draw's shuffle wakes a
/// Stratagem, whose draw-pile pick opens under the deferred pass. The pick
/// is banked with everything the interrupted draw still owes, and opens at
/// the site's join.
#[test]
fn a_screen_nested_inside_a_deferred_body_still_asks() {
    let mut simulator = ScenarioBuilder::new("EMBRACE", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .power(0, id("POWER.DARK_EMBRACE_POWER"), 1, PowerType::Buff)
        .power(0, id("POWER.STRATAGEM_POWER"), 1, PowerType::Buff)
        .card(PileName::Hand, 0, card("CARD.DAZED"), 0)
        .card(PileName::Discard, 1, card("CARD.STRIKE_IRONCLAD"), 0)
        .card(PileName::Discard, 2, card("CARD.STRIKE_IRONCLAD"), 0)
        .card(PileName::Discard, 3, card("CARD.STRIKE_IRONCLAD"), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    end_turn(&mut simulator);

    let (choice_id, minimum, maximum, candidates) = the_screen(&simulator);
    assert_eq!(
        (minimum, maximum),
        (1, 1),
        "the stratagem takes exactly one"
    );
    // All three shuffled cards: the draw pauses while the shuffle's listener
    // asks, so the pick is over the pile as the shuffle left it, and the
    // draw's own card comes off the pile only behind the answer. What this
    // test pins beyond that is the deferred-pass discipline around it: the
    // screen is banked, opened at the join, and everything the body still
    // owes — the rest of the draw included — runs behind the answer.
    assert_eq!(
        candidates.len(),
        3,
        "the whole shuffled pile, with the draw waiting"
    );
    let taken = candidates[1].clone();
    let answer = Action::ChooseCards {
        choice_id,
        cards: vec![taken.clone()],
    };
    assert_clone_resumes_identically(&simulator, answer.clone());
    simulator.step(answer).expect("the pick applies");

    let combat = simulator.state().combat.as_ref().expect("the fight stands");
    assert_eq!(
        combat.player.turn, 2,
        "the interrupted draw, the pass, and the handover all ran out"
    );
    assert!(
        combat
            .player
            .piles
            .hand
            .iter()
            .any(|held| held.id == taken.card_id),
        "the stratagem handed the picked card back to the hand"
    );
    let embrace_counter = combat
        .creatures
        .iter()
        .find(|creature| creature.id == combat.player.creature_id)
        .expect("the player stands")
        .powers
        .iter()
        .find(|power| power.model_id == id("POWER.DARK_EMBRACE_POWER"))
        .expect("the embrace stands")
        .counter;
    assert_eq!(
        embrace_counter, 0,
        "the rest of the embrace's body — the counter reset behind the \
         screen — ran once the answer came back"
    );
}
