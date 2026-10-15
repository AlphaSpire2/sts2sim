//! Block outcomes belong to the original target of a redirected blow.

use std::sync::Arc;

use sts2_engine::{
    Action, Amount, CardFingerprint, Command, CreatureCmd, CreatureSelector, HookEvent,
    ModifierCmd, ModifierHook, OstyCmd, PileName, PowerType, ScenarioBuilder, Simulator,
    ValueProps,
};

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().unwrap()
}

fn fight(block: i32, pet_hp: i32, prevent_hp_loss: bool) -> Simulator {
    let mut registry = (*sts2_content::standard_registry()).clone();
    for monster in sts2_content::monsters_at_ascension(9) {
        registry.add_monster(monster);
    }
    let mut setup = registry
        .card(&id("CARD.DEFEND_NECROBINDER"))
        .unwrap()
        .clone();
    setup.model_id = id("CARD.TEST_SETUP");
    setup.effects = vec![
        Command::Osty(OstyCmd::Summon {
            amount: Amount::Fixed(pet_hp),
            owner: CreatureSelector::Owner,
        }),
        Command::Creature(CreatureCmd::GainBlock {
            target: CreatureSelector::Owner,
            amount: Amount::Fixed(block),
            props: ValueProps::UNPOWERED,
        }),
    ];
    registry.add_card(setup);
    let mut watcher = registry.power(&id("POWER.STRENGTH_POWER")).unwrap().clone();
    watcher.model_id = id("POWER.TEST_BLOCK_WATCHER");
    watcher.hooks = vec![ModifierHook::every_turn(
        HookEvent::AfterBlockBroken,
        vec![Command::Creature(CreatureCmd::GainMaxHp {
            target: CreatureSelector::Owner,
            amount: Amount::Fixed(1),
        })],
    )];
    if prevent_hp_loss {
        watcher.hooks.push(ModifierHook::every_turn(
            HookEvent::ModifyHpLostAfterOsty,
            vec![Command::Modify(ModifierCmd::CapAt(Amount::Fixed(0)))],
        ));
    }
    registry.add_power(watcher);
    ScenarioBuilder::new("REDIRECTBLOCK", id("ENCOUNTER.BOWLBUGS_NORMAL"))
        .played_as(id("CHARACTER.NECROBINDER"))
        .player_hp(80, 80)
        .energy(3, 3)
        .enemy(1, id("MONSTER.BOWLBUG_ROCK"), 45, 45, "HEADBUTT_MOVE")
        .power(1, id("POWER.IMBALANCED_POWER"), 1, PowerType::Debuff)
        .power(0, id("POWER.TEST_BLOCK_WATCHER"), 1, PowerType::Buff)
        .card(
            PileName::Hand,
            0,
            CardFingerprint::base(id("CARD.TEST_SETUP")),
            1,
        )
        .build(Arc::new(registry))
        .unwrap()
}

fn end_turn(simulator: &mut Simulator) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::EndTurn { .. }))
        .cloned()
        .unwrap();
    simulator.step(action).unwrap();
}

#[test]
fn redirected_block_outcomes_are_announced_only_for_the_original_target() {
    for (block, pet_hp, prevented, stunned, hp_lost, pet_lost) in [
        (12, 10, false, false, 0, 4),
        (12, 2, false, false, 2, 2),
        (16, 10, false, true, 0, 0),
        (20, 10, false, true, 0, 0),
        // Full-block classification reads the HP-loss modifiers' result,
        // even when block alone did not stop all sixteen damage.
        (12, 10, true, true, 0, 0),
    ] {
        let mut simulator = fight(block, pet_hp, prevented);
        let action = simulator
            .legal_actions()
            .iter()
            .find(|action| matches!(action, Action::PlayCard { .. }))
            .cloned()
            .unwrap();
        simulator.step(action).unwrap();
        let snapshot = simulator.snapshot();
        end_turn(&mut simulator);
        let combat = simulator.state().combat.as_ref().unwrap();
        let player = combat.creatures.iter().find(|c| c.id.get() == 0).unwrap();
        let rock = combat.creatures.iter().find(|c| c.id.get() == 1).unwrap();
        let pet = combat
            .creatures
            .iter()
            .find(|c| c.pet_owner.is_some())
            .unwrap();
        let broken = i32::from(block <= 16);
        assert_eq!(
            player.max_hp,
            80 + broken,
            "one player block-break announcement"
        );
        assert_eq!(pet.max_hp, pet_hp, "no block-break announcement for Osty");
        assert_eq!(player.current_hp, 80 - hp_lost + broken);
        assert_eq!(pet.current_hp, pet_hp - pet_lost);
        assert_eq!(
            rock.move_state == "STUNNED",
            stunned,
            "block={block}, pet={pet_hp}"
        );
        let outcome = simulator.snapshot();
        simulator.restore(&snapshot).unwrap();
        end_turn(&mut simulator);
        assert_eq!(simulator.snapshot(), outcome);
    }
}
