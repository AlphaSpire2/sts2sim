use sts2_engine::{Action, DecisionContext, ScenarioBuilder};

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

#[test]
fn a_lantern_does_not_light_the_enemys_first_turn() {
    let mut simulator = ScenarioBuilder::new("LANTERN", id("ENCOUNTER.KNOWLEDGE_DEMON_BOSS"))
        .player_hp(80, 80)
        .energy(0, 3)
        .turn(1, 1)
        .relics(&[id("RELIC.LANTERN")])
        .enemy(
            1,
            id("MONSTER.KNOWLEDGE_DEMON"),
            379,
            379,
            "CURSE_OF_KNOWLEDGE_MOVE",
        )
        .build(sts2_content::standard_registry())
        .expect("the fight builds");
    let end_turn = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::EndTurn { .. }))
        .cloned()
        .expect("the turn can end");
    simulator
        .step(end_turn)
        .expect("the enemy offers its curse choice");
    assert!(matches!(
        simulator.decision(),
        DecisionContext::ChooseCards { .. }
    ));
    assert_eq!(
        simulator.state().combat.as_ref().unwrap().player.energy,
        0,
        "Lantern only grants energy when its owner's turn starts"
    );
}
