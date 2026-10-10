//! Omnislice's primary damage and unpowered splash belong to one attack.

use sts2_engine::{Action, CardFingerprint, PileName, PowerType, ScenarioBuilder, Simulator};

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn scenario(primary_hp: i32, upgrade: u8) -> ScenarioBuilder {
    let mut card = CardFingerprint::base(id("CARD.OMNISLICE"));
    card.upgrade_level = upgrade;
    ScenarioBuilder::new("OMNISLICE", id("ENCOUNTER.PHANTASMAL_GARDENERS_ELITE"))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(1, 1)
        .enemy(
            1,
            id("MONSTER.PHANTASMAL_GARDENER"),
            primary_hp,
            50,
            "BITE_MOVE",
        )
        .enemy(2, id("MONSTER.PHANTASMAL_GARDENER"), 50, 50, "BITE_MOVE")
        .power(1, id("POWER.SKITTISH_POWER"), 7, PowerType::Buff)
        .power(2, id("POWER.SKITTISH_POWER"), 7, PowerType::Buff)
        .power(0, id("POWER.STRENGTH_POWER"), 3, PowerType::Buff)
        .card(PileName::Hand, 0, card, 0)
}

fn play(simulator: &mut Simulator) {
    let action = simulator.legal_actions().iter().find(|action| {
        matches!(action, Action::PlayCard { target: Some(target), .. } if target.combat_id.get() == 1)
    }).cloned().expect("Omnislice can target the first gardener");
    simulator.step(action).expect("the play settles");
}

#[test]
fn omnislice_announces_both_hits_to_skittish_including_overkill() {
    for upgrade in [0, 1] {
        for primary_hp in [5, 50] {
            let mut simulator = scenario(primary_hp, upgrade)
                .build(sts2_content::standard_registry())
                .expect("the scenario builds");
            play(&mut simulator);
            let damage = if upgrade == 0 { 11 } else { 14 };
            let combat = simulator.state().combat.as_ref().unwrap();
            let other = combat
                .creatures
                .iter()
                .find(|creature| creature.id.get() == 2)
                .unwrap();
            assert_eq!(
                other.current_hp,
                50 - damage,
                "splash includes overkill and does not apply Strength twice"
            );
            assert_eq!(
                other.block, 7,
                "Skittish sees the splash as part of the attack"
            );
            if primary_hp == 50 {
                let primary = combat
                    .creatures
                    .iter()
                    .find(|creature| creature.id.get() == 1)
                    .unwrap();
                assert_eq!(primary.current_hp, 50 - damage);
                assert_eq!(primary.block, 7, "Skittish sees the primary hit too");
            }
        }
    }
}

#[test]
fn omnislice_binds_and_spends_one_attacks_worth_of_buffs() {
    let mut simulator = scenario(50, 0)
        .power(0, id("POWER.VIGOR_POWER"), 2, PowerType::Buff)
        .power(0, id("POWER.GIGANTIFICATION_POWER"), 2, PowerType::Buff)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator);
    let combat = simulator.state().combat.as_ref().unwrap();
    for creature in combat
        .creatures
        .iter()
        .filter(|creature| creature.id.get() != 0)
    {
        assert_eq!(creature.current_hp, 50 - (8 + 3 + 2) * 3);
        assert_eq!(creature.block, 7);
    }
    let player = combat
        .creatures
        .iter()
        .find(|creature| creature.id.get() == 0)
        .unwrap();
    assert!(
        !player
            .powers
            .iter()
            .any(|power| power.model_id == id("POWER.VIGOR_POWER"))
    );
    let gigantification = player
        .powers
        .iter()
        .find(|power| power.model_id == id("POWER.GIGANTIFICATION_POWER"))
        .unwrap();
    assert_eq!(
        gigantification.amount, 1,
        "both damage calls spend just one attack charge"
    );
}
