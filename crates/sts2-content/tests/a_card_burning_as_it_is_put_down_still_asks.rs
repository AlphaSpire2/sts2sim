//! A screen reached through a played card burning as it is put down still asks.
//!
//! The build sends a played card whose result pile is the exhaust pile through
//! the same exhaust command a body would, with the player's choice context in
//! hand, so a listener on that exhaust may put a question to the player. Dark
//! Embrace answers the exhaust with a draw, Hellraiser auto-plays the Strike
//! that draw dealt, and a Seeker Strike's own body offers three of the draw
//! pile: the screen opens and the play waits on it.

use sts2_engine::{Action, CardFingerprint, DecisionContext, PileName, PowerType, ScenarioBuilder};
use sts2_rng::RunStream;

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str) -> CardFingerprint {
    let mut fingerprint = CardFingerprint::base(id(model_id));
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

#[test]
fn a_seeker_strike_auto_played_off_a_burning_cards_draw_opens_its_screen() {
    let mut simulator = ScenarioBuilder::new("BURNING", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 200, 200, "SHRINKER_MOVE")
        .power(0, id("POWER.DARK_EMBRACE_POWER"), 1, PowerType::Buff)
        .power(0, id("POWER.HELLRAISER_POWER"), 1, PowerType::Buff)
        .card(PileName::Hand, 0, card("CARD.INFERNAL_BLADE"), 0)
        .card(PileName::Draw, 1, card("CARD.SEEKER_STRIKE"), 0)
        .card(PileName::Draw, 2, card("CARD.STRIKE_IRONCLAD"), 0)
        .card(PileName::Draw, 3, card("CARD.DEFEND_IRONCLAD"), 0)
        .card(PileName::Draw, 4, card("CARD.BASH"), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    let wanted = id("CARD.INFERNAL_BLADE");
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::PlayCard { card, .. } if card.fingerprint.model_id == wanted)
        })
        .cloned()
        .expect("the infernal blade is playable");
    simulator
        .step(action)
        .expect("the play applies and leaves the seeker strike's screen standing");

    let DecisionContext::ChooseCards { candidates, .. } = simulator.decision() else {
        panic!(
            "the seeker strike's pick should be standing, found {:?}",
            simulator.decision()
        );
    };
    assert_eq!(candidates.len(), 3, "three of the draw pile are offered");
}
