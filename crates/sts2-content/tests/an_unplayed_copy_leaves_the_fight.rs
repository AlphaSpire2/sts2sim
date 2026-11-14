//! A copy made to be played once leaves the fight even when it is put down
//! unplayed.
//!
//! An automatic play that cannot go ahead puts its card down where the play
//! would have put it: an unplayable card, a play a power or relic refuses, and
//! an aimed card with nothing left to aim at. A copy is the exception. A played
//! copy goes to no pile, and an unplayed one does not land in the discard pile
//! either.
//!
//! A History Course replays the last skill of the turn before as a copy once
//! the turn start has drawn the hand. A Hellraiser plays every Strike as it is
//! drawn, so a Strike drawn into a Test Subject's first form puts that form
//! down before the course replays a Taunt: the fight goes on, but there is no
//! enemy for the copy to aim at.

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

/// A History Course and a Hellraiser against a Test Subject's first form, a
/// Taunt in hand, and a Strike among the cards the next hand draws.
fn a_history_course_over_a_test_subject(subject_hp: i32) -> Simulator {
    let mut scenario = ScenarioBuilder::new("HISTORYCOPY", id("ENCOUNTER.TEST_SUBJECT_BOSS"))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatTargets, 0)
        .rng_counter(RunStream::Shuffle, 0)
        .relics(&[id("RELIC.HISTORY_COURSE")])
        .power(0, id("POWER.HELLRAISER_POWER"), 1, PowerType::Buff)
        .enemy(1, id("MONSTER.TEST_SUBJECT"), subject_hp, 100, "BITE_MOVE")
        .power(1, id("POWER.ADAPTABLE_POWER"), 1, PowerType::Buff)
        .card(PileName::Hand, 0, card("CARD.TAUNT"), 0)
        .card(PileName::Draw, 1, card("CARD.STRIKE_IRONCLAD"), 0);
    for card_id in 2..6 {
        scenario = scenario.card(PileName::Draw, card_id, card("CARD.DEFEND_IRONCLAD"), 0);
    }
    scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

/// Taunts the Test Subject and ends the turn, so the next turn start draws the
/// Strike and the course replays the Taunt.
fn taunt_and_end_the_turn(simulator: &mut Simulator) {
    let taunt = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::PlayCard { card, .. }
                if card.fingerprint.model_id == id("CARD.TAUNT"))
        })
        .cloned()
        .expect("the Taunt is playable");
    simulator.step(taunt).expect("the Taunt is played");
    let end_turn = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::EndTurn { .. }))
        .cloned()
        .expect("the turn can be ended");
    simulator.step(end_turn).expect("the turn ends");
}

fn taunts(simulator: &Simulator) -> Vec<(PileName, bool)> {
    let piles = &simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is held")
        .player
        .piles;
    [
        (PileName::Hand, &piles.hand),
        (PileName::Draw, &piles.draw),
        (PileName::Discard, &piles.discard),
        (PileName::Exhaust, &piles.exhaust),
        (PileName::Play, &piles.play),
    ]
    .into_iter()
    .flat_map(|(name, pile)| {
        pile.iter()
            .filter(|card| card.fingerprint.model_id == id("CARD.TAUNT"))
            .map(move |card| (name, card.dupe))
    })
    .collect()
}

fn player_block(simulator: &Simulator) -> i32 {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is held")
        .creatures
        .iter()
        .find(|creature| creature.id.get() == 0)
        .expect("the player is in the fight")
        .block
}

#[test]
fn a_copy_with_something_to_aim_at_is_played_and_leaves_the_fight() {
    let mut simulator = a_history_course_over_a_test_subject(100);
    taunt_and_end_the_turn(&mut simulator);
    assert_eq!(player_block(&simulator), 7, "the course's copy was played");
    assert_eq!(taunts(&simulator), [(PileName::Discard, false)]);
}

#[test]
fn a_copy_with_nothing_to_aim_at_is_put_down_out_of_the_fight() {
    let mut simulator = a_history_course_over_a_test_subject(4);
    taunt_and_end_the_turn(&mut simulator);
    let combat = simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is held");
    assert!(
        combat.in_progress,
        "the first form went down behind its Adaptable, so the fight goes on"
    );
    assert_eq!(player_block(&simulator), 0, "the copy was not played");
    assert_eq!(
        taunts(&simulator),
        [(PileName::Discard, false)],
        "only the Taunt played last turn is left in the fight"
    );
}
