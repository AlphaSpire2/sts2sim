//! A listener pass the build defers can still put a question to the player.
//!
//! The turn-cycle dispatch sites pause per listener and join later: each
//! listener's body is started, the site waits only for that body to pause, and
//! every paused body is joined at the site's own end. A body that asks parks
//! its screen on the action queue and the pass runs on.
//!
//! The pass banks the screen and the site's join opens it, so the player is
//! asked exactly where the build asks. Two shapes stand that up: the retain
//! pick Well-Laid Plans owes at `BeforeFlushLate`, and a screen nested inside
//! a deferred body — a Stratagem's draw-pile pick reached through a shuffle
//! the deferred body's own draw caused.

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

/// Well-Laid Plans' end-turn retain pick: a screen put up directly by a
/// deferred body. The build asks on `BeforeFlushLate` — the late pass, so the
/// player sees every other before-flush effect first — and the join opens
/// the pick before the hand goes down.
#[test]
fn well_laid_plans_asks_its_retain_pick_at_end_of_turn() {
    let mut simulator = ScenarioBuilder::new("PLANS", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .power(0, id("POWER.WELL_LAID_PLANS_POWER"), 1, PowerType::Buff)
        .card(PileName::Hand, 0, card("CARD.STRIKE_IRONCLAD"), 0)
        .card(PileName::Hand, 1, card("CARD.DEFEND_IRONCLAD"), 0)
        .card(PileName::Hand, 2, card("CARD.STRIKE_IRONCLAD"), 0)
        // A draw pile deep enough that the next turn's hand comes off it: a
        // flushed card stays down, so only the retain can keep the defend up.
        .card(PileName::Draw, 3, card("CARD.STRIKE_IRONCLAD"), 0)
        .card(PileName::Draw, 4, card("CARD.STRIKE_IRONCLAD"), 0)
        .card(PileName::Draw, 5, card("CARD.STRIKE_IRONCLAD"), 0)
        .card(PileName::Draw, 6, card("CARD.STRIKE_IRONCLAD"), 0)
        .card(PileName::Draw, 7, card("CARD.STRIKE_IRONCLAD"), 0)
        .card(PileName::Draw, 8, card("CARD.STRIKE_IRONCLAD"), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    end_turn(&mut simulator);

    let (choice_id, minimum, maximum, candidates) = the_screen(&simulator);
    assert_eq!((minimum, maximum), (0, 1), "keep up to one card");
    assert_eq!(candidates.len(), 3, "the whole unretained hand is offered");
    let defend = candidates
        .iter()
        .find(|candidate| candidate.fingerprint.model_id == id("CARD.DEFEND_IRONCLAD"))
        .cloned()
        .expect("the defend is on the screen");
    let keep = Action::ChooseCards {
        choice_id,
        cards: vec![defend.clone()],
    };
    assert_clone_resumes_identically(&simulator, keep.clone());
    simulator.step(keep).expect("the pick applies");

    let combat = simulator.state().combat.as_ref().expect("the fight stands");
    assert_eq!(
        combat.player.turn, 2,
        "the handover ran out behind the answer"
    );
    assert!(
        combat
            .player
            .piles
            .hand
            .iter()
            .any(|held| held.id == defend.card_id),
        "the kept card survived the flush into the next turn"
    );
    assert!(
        combat
            .player
            .piles
            .hand
            .iter()
            .all(|held| held.id.get() != 0 && held.id.get() != 2),
        "and the strikes that were not kept went down with the flush"
    );
}

/// Answering with nothing keeps nothing: the same screen, backed out of, and
/// the whole hand goes down with the flush.
#[test]
fn well_laid_plans_can_be_answered_with_nothing() {
    let mut simulator = ScenarioBuilder::new("PLANS", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .power(0, id("POWER.WELL_LAID_PLANS_POWER"), 1, PowerType::Buff)
        .card(PileName::Hand, 0, card("CARD.STRIKE_IRONCLAD"), 0)
        .card(PileName::Hand, 1, card("CARD.DEFEND_IRONCLAD"), 0)
        .card(PileName::Draw, 2, card("CARD.STRIKE_IRONCLAD"), 0)
        .card(PileName::Draw, 3, card("CARD.STRIKE_IRONCLAD"), 0)
        .card(PileName::Draw, 4, card("CARD.STRIKE_IRONCLAD"), 0)
        .card(PileName::Draw, 5, card("CARD.STRIKE_IRONCLAD"), 0)
        .card(PileName::Draw, 6, card("CARD.STRIKE_IRONCLAD"), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    end_turn(&mut simulator);

    let (choice_id, ..) = the_screen(&simulator);
    simulator
        .step(Action::ChooseCards {
            choice_id,
            cards: Vec::new(),
        })
        .expect("nothing is an answer");
    let combat = simulator.state().combat.as_ref().expect("the fight stands");
    assert_eq!(combat.player.turn, 2);
    let hand_ids: Vec<u64> = combat
        .player
        .piles
        .hand
        .iter()
        .map(|held| held.id.get())
        .collect();
    assert!(
        !hand_ids.contains(&0) && !hand_ids.contains(&1),
        "nothing was kept back from the flush: {hand_ids:?}"
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
