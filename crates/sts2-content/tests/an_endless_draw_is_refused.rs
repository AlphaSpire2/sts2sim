//! A step whose draws never stop is refused rather than run forever.
//!
//! A pillage keeps drawing while each card it draws is an attack and the hand
//! has room. A hellraiser plays every strike the moment it is drawn, and a
//! play that is not allowed sends the strike straight to the discard pile. With
//! a sloth already spent on the pillage itself, no strike can be played, the
//! hand never fills, and the same strikes are drawn, refused and reshuffled
//! without end. The step is refused instead, and the state before it stands.

use sts2_engine::{Action, CardFingerprint, ErrorCode, PileName, ScenarioBuilder, Simulator};
use sts2_rng::RunStream;

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str) -> CardFingerprint {
    let mut fingerprint = CardFingerprint::base(id(model_id));
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

fn endless_pillage() -> Simulator {
    ScenarioBuilder::new("ENDLESSDRAW", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .rng_counter(RunStream::CombatTargets, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .power(
            0,
            id("POWER.HELLRAISER_POWER"),
            1,
            sts2_engine::PowerType::Buff,
        )
        .power(
            0,
            id("POWER.SLOTH_POWER"),
            1,
            sts2_engine::PowerType::Debuff,
        )
        .card(PileName::Hand, 0, card("CARD.PILLAGE"), 0)
        .card(PileName::Draw, 1, card("CARD.STRIKE_IRONCLAD"), 0)
        .card(PileName::Draw, 2, card("CARD.STRIKE_IRONCLAD"), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

fn pillage(simulator: &Simulator) -> Action {
    simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::PlayCard { card, .. }
                if card.fingerprint.model_id == id("CARD.PILLAGE"))
        })
        .cloned()
        .expect("the pillage is playable")
}

fn hand(simulator: &Simulator) -> Vec<String> {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("a fight")
        .player
        .piles
        .hand
        .iter()
        .map(|card| card.fingerprint.model_id.to_string())
        .collect()
}

#[test]
fn a_pillage_drawing_strikes_nothing_can_play_is_refused() {
    let mut simulator = endless_pillage();
    let action = pillage(&simulator);
    let error = simulator
        .step(action)
        .expect_err("the endless draw is refused");
    assert_eq!(error.code, ErrorCode::InternalInvariant);
    assert!(
        error.details.contains_key("endless_loop"),
        "the refusal names the endless loop: {error:?}"
    );
    assert_eq!(
        hand(&simulator),
        ["CARD.PILLAGE"],
        "the state before the step stands"
    );
}

#[test]
fn a_quiet_step_refuses_it_too() {
    let mut simulator = endless_pillage();
    let action = pillage(&simulator);
    let error = simulator
        .step_quietly(&action)
        .expect_err("the endless draw is refused");
    assert!(
        error.details.contains_key("endless_loop"),
        "the refusal names the endless loop: {error:?}"
    );
}
