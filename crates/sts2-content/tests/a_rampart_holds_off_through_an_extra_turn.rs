//! A rampart arms the gunner as the player's turn opens, but not as an extra
//! turn opens: the enemy side has not acted since the last grant.

use sts2_engine::{Action, CardFingerprint, PileName, PowerType, ScenarioBuilder, Simulator};
use sts2_rng::RunStream;

const SEED: &str = "RAMPARTEXTRA";

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str) -> CardFingerprint {
    let mut fingerprint = CardFingerprint::base(id(model_id));
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

/// The living shield and its gunner, against a player whose eye grants an
/// extra turn after a turn in which no card was played.
fn fight() -> Simulator {
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.TURRET_OPERATOR_WEAK"))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .enemy(1, id("MONSTER.LIVING_SHIELD"), 55, 55, "SHIELD_SLAM_MOVE")
        .enemy(2, id("MONSTER.TURRET_OPERATOR"), 41, 41, "UNLOAD_MOVE")
        .power(1, id("POWER.RAMPART_POWER"), 25, PowerType::Buff)
        .relics(&[id("RELIC.PAELS_EYE")]);
    for next_id in 0..5 {
        scenario = scenario.card(PileName::Hand, next_id, card("CARD.STRIKE_IRONCLAD"), 0);
    }
    for next_id in 5..10 {
        scenario = scenario.card(PileName::Draw, next_id, card("CARD.STRIKE_IRONCLAD"), 0);
    }
    scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

fn gunner_block(simulator: &Simulator) -> i32 {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("a fight is standing")
        .creatures
        .iter()
        .find(|creature| creature.model_id == id("MONSTER.TURRET_OPERATOR"))
        .expect("the gunner stands")
        .block
}

#[test]
fn an_extra_turn_does_not_arm_the_gunner_again() {
    let mut simulator = fight();
    let before = gunner_block(&simulator);
    simulator
        .step(Action::EndTurn { turn: 1 })
        .expect("ending the turn applies");
    assert!(
        simulator
            .state()
            .combat
            .as_ref()
            .expect("a fight is standing")
            .taking_extra_turn,
        "a turn with no card played earns the eye's extra turn"
    );
    assert_eq!(gunner_block(&simulator), before);
}
