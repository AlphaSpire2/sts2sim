//! A fight won at the end of an enemy turn still switches back to the player.
//!
//! The end of an enemy turn asks the win condition and then guards the switch
//! behind it, and the guard is not the guard it looks like: a fight only counts
//! as ending while it is also still in progress, and the check that just ran is
//! what took it out of progress — so on a fight that has only this moment been
//! won the guard opens rather than closes. The side comes back to the player,
//! the round winds on, and the player's turn number winds on with it, all over
//! a combat that is already over. Only the turn start itself declines, because
//! it asks whether the fight is still in progress.
//!
//! Nothing in the fight reads those numbers again, but the recorder writes them
//! down: a fight that ended this way carries the round after the one that was
//! played.
//!
//! A demise stands this up: it costs its owner its whole amount as their
//! side's turn ends, which is a pass the end of the enemy turn makes.

use sts2_engine::{
    Action, CardFingerprint, CombatSide, PileName, PowerType, ScenarioBuilder, Simulator,
};

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str) -> CardFingerprint {
    let mut fingerprint = CardFingerprint::base(id(model_id));
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

/// A beetle with seven health left and nine points of demise on it. It takes
/// its turn, and then the end of that turn kills it.
fn a_beetle_that_dies_as_its_turn_ends() -> Simulator {
    ScenarioBuilder::new("DEMI", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(1, 1)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 7, 39, "SHRINKER_MOVE")
        .power(1, id("POWER.DEMISE_POWER"), 9, PowerType::Debuff)
        .card(PileName::Hand, 0, card("CARD.DEFEND_IRONCLAD"), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

#[test]
fn a_fight_the_enemy_turns_own_end_won_still_winds_the_round_on() {
    let mut simulator = a_beetle_that_dies_as_its_turn_ends();
    let end_turn = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::EndTurn { .. }))
        .cloned()
        .expect("the turn can be ended");
    simulator.step(end_turn).expect("the turn ends");

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
        "the demise took the beetle as its own turn ended"
    );
    assert!(
        combat.has_ended && combat.rewards_open,
        "and the fight is over with what it owes on the table"
    );
    assert_eq!(combat.side, CombatSide::Player, "the side came back anyway");
    assert_eq!(combat.round, 2, "and the round wound on with it");
    assert_eq!(combat.player.turn, 2, "and so did the player's turn number");
}
