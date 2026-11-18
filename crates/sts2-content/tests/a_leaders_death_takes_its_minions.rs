//! The last primary enemy going down takes the minions beside it with it,
//! inside its own death: nothing that runs after the blow finds one standing.

use sts2_engine::{Action, CardFingerprint, PileName, PowerType, ScenarioBuilder, Simulator};
use sts2_rng::RunStream;

const SEED: &str = "LEADERDOWN";

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str) -> CardFingerprint {
    let mut fingerprint = CardFingerprint::base(id(model_id));
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

/// A fabricator one strike from dying, a minion beside it, and a player whose
/// serpent form throws damage at a random enemy after every card.
fn fight() -> Simulator {
    ScenarioBuilder::new(SEED, id("ENCOUNTER.FABRICATOR_NORMAL"))
        .played_as(id("CHARACTER.SILENT"))
        .player_hp(70, 70)
        .energy(3, 3)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatTargets, 0)
        .enemy(1, id("MONSTER.FABRICATOR"), 3, 150, "FABRICATE_MOVE")
        .enemy(2, id("MONSTER.STABBOT"), 18, 23, "STAB_MOVE")
        .power(2, id("POWER.MINION_POWER"), 1, PowerType::Buff)
        .power(0, id("POWER.SERPENT_FORM_POWER"), 4, PowerType::Buff)
        .card(PileName::Hand, 0, card("CARD.STRIKE_SILENT"), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

#[test]
fn the_serpent_finds_no_minion_left_after_the_leader_falls() {
    let mut simulator = fight();
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::PlayCard { card, target: Some(target) }
                if card.fingerprint.model_id == id("CARD.STRIKE_SILENT")
                    && target.model_id == id("MONSTER.FABRICATOR"))
        })
        .cloned()
        .expect("the strike at the fabricator is legal");
    simulator.step(action).expect("the play applies");
    let combat = simulator
        .state()
        .combat
        .as_ref()
        .expect("a fight is standing");
    assert!(
        !combat
            .creatures
            .iter()
            .any(|creature| creature.model_id == id("MONSTER.STABBOT") && creature.alive()),
        "the minion went down with its leader"
    );
    assert_eq!(
        simulator
            .state()
            .rng
            .stream(RunStream::CombatTargets)
            .counter(),
        0,
        "and the serpent form had nothing to roll for"
    );
}
