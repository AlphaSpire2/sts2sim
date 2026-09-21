//! A fight the listeners on a play won is over when the play is.
//!
//! Every command reports whether the fight ended doing it, and the report is
//! handed back up the call that ran it. A hook dispatch is the one caller that
//! cannot hand it back: a hook pass walks its listeners and answers with
//! nothing, because the build defers the teardown of a fight a listener ended
//! to the next safe point rather than stopping the walk. That safe point is the
//! end of the action being run, where the win condition is asked again,
//! whoever landed the blow.
//!
//! That ask is what ends a fight whose killing blow was a listener's rather
//! than the card's own: the enemy is gone, and the rewards follow.
//!
//! A kusarigama stands this up. It cuts every third attack of the turn, off
//! `AfterCardPlayed`, so the third strike's own damage is not what kills.

use sts2_engine::{Action, CardFingerprint, PileName, ScenarioBuilder, Simulator};

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str) -> CardFingerprint {
    let mut fingerprint = CardFingerprint::base(id(model_id));
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

/// Three strikes against a beetle with twenty health: eighteen of it goes to
/// the strikes, and the two that are left go to the relic the third one woke.
fn a_kusarigama_over_three_strikes() -> Simulator {
    let mut scenario = ScenarioBuilder::new("KUSA", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .relics(&[id("RELIC.KUSARIGAMA")])
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 20, 39, "SHRINKER_MOVE");
    for slot in 0..3 {
        scenario = scenario.card(PileName::Hand, slot, card("CARD.STRIKE_IRONCLAD"), 0);
    }
    scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

fn play_a_strike(simulator: &mut Simulator) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(
                action,
                Action::PlayCard { card, .. }
                    if card.fingerprint.model_id == id("CARD.STRIKE_IRONCLAD")
            )
        })
        .cloned()
        .expect("a strike is offered");
    simulator.step(action).expect("the strike applies");
}

#[test]
fn a_kill_landed_by_a_listener_ends_the_fight_with_the_play() {
    let mut simulator = a_kusarigama_over_three_strikes();
    for _ in 0..2 {
        play_a_strike(&mut simulator);
    }
    let combat = simulator.state().combat.as_ref().expect("the fight stands");
    assert_eq!(
        combat
            .creatures
            .iter()
            .find(|creature| creature.id == sts2_core::CreatureCombatId::from(1))
            .expect("the beetle stands")
            .current_hp,
        8,
        "two strikes and no cut yet"
    );

    play_a_strike(&mut simulator);

    let combat = simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is held");
    assert!(
        !combat
            .creatures
            .iter()
            .any(|creature| creature.id == sts2_core::CreatureCombatId::from(1)),
        "the third strike left two health and the relic's cut took them"
    );
    assert!(
        combat.has_ended && !combat.in_progress,
        "the fight the relic won is over when the play it answered is"
    );
    assert!(combat.rewards_open, "and what it owes is on the table");
}
