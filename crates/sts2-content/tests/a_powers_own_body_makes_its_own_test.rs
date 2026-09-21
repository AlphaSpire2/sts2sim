//! `AfterApplied` reaches a power's own body directly rather than as a pass
//! over the listener list, and a body reached that way still opens with
//! whatever test it
//! opens with. A ritual is where that matters: the skip it arms as it lands is
//! armed only for an enemy that put the ritual on itself, so a ritual that
//! lands on the player pays out at the end of the very turn it landed on.

use sts2_engine::{Action, PowerType, ScenarioBuilder, Simulator};
use sts2_rng::RunStream;

const SEED: &str = "RITUALPAYOUT";

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn creature(simulator: &Simulator, combat_id: u64) -> &sts2_engine::Creature {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .creatures
        .iter()
        .find(|creature| creature.id.get() == combat_id)
        .expect("the creature is in the fight")
}

/// How much of a power a creature is standing with, and `None` where it carries
/// none of it.
fn power(simulator: &Simulator, combat_id: u64, model_id: &str) -> Option<i32> {
    let wanted = id(model_id);
    creature(simulator, combat_id)
        .powers
        .iter()
        .find(|power| power.model_id == wanted)
        .map(|power| power.amount)
}

fn end_turn(simulator: &mut Simulator) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::EndTurn { .. }))
        .cloned()
        .expect("ending the turn is always legal");
    simulator.step(action).expect("the turn ends");
}

/// A ritual the player is handed mid-fight. The gift is what hands it over in
/// the run this was found in, and the belt is the only way to make a ritual
/// land on the player partway through a turn.
#[test]
fn a_ritual_that_lands_on_the_player_pays_out_the_turn_it_landed_on() {
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .played_as(id("CHARACTER.SILENT"))
        .player_hp(70, 70)
        .energy(9, 9)
        .turn(1, 1)
        .potions(vec![Some(id("POTION.MAZALETHS_GIFT")), None, None])
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatTargets, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");

    let drink = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::UsePotion { model_id, .. }
                if *model_id == id("POTION.MAZALETHS_GIFT"))
        })
        .cloned()
        .expect("the gift is drinkable");
    simulator.step(drink).expect("the potion applies");

    assert_eq!(power(&simulator, 0, "POWER.RITUAL_POWER"), Some(1));
    // The skip is the power's own counter, and it is not armed for a ritual
    // standing on the player. That is the cost the payout below turns on.
    assert_eq!(
        creature(&simulator, 0)
            .powers
            .iter()
            .find(|power| power.model_id == id("POWER.RITUAL_POWER"))
            .expect("the ritual is standing")
            .counter,
        0,
    );

    end_turn(&mut simulator);
    assert_eq!(power(&simulator, 0, "POWER.STRENGTH_POWER"), Some(1));
}

/// The other side of it: a cultist's incantation puts a ritual on the cultist,
/// and that one holds its payout back until the turn after the one it landed
/// on.
#[test]
fn a_ritual_an_enemy_puts_on_itself_holds_back_the_turn_it_landed_on() {
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .played_as(id("CHARACTER.SILENT"))
        .player_hp(70, 70)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatTargets, 0)
        .enemy(
            1,
            id("MONSTER.CALCIFIED_CULTIST"),
            42,
            42,
            "INCANTATION_MOVE",
        )
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");

    // The incantation runs on the enemy's turn, so the ritual lands and that
    // same turn ends behind it. Nothing is owed for it yet.
    end_turn(&mut simulator);
    assert_eq!(power(&simulator, 1, "POWER.RITUAL_POWER"), Some(2));
    assert_eq!(power(&simulator, 1, "POWER.STRENGTH_POWER"), None);

    // The turn after, the same ritual pays what it is worth.
    end_turn(&mut simulator);
    assert_eq!(power(&simulator, 1, "POWER.STRENGTH_POWER"), Some(2));
}

/// A ritual standing at the opening rather than landing mid-fight pays out at
/// the end of the first turn either way: nothing armed the skip.
#[test]
fn a_ritual_already_standing_owes_the_first_turn_too() {
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .played_as(id("CHARACTER.SILENT"))
        .player_hp(70, 70)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatTargets, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .power(0, id("POWER.RITUAL_POWER"), 1, PowerType::Buff)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");

    end_turn(&mut simulator);
    assert_eq!(power(&simulator, 0, "POWER.STRENGTH_POWER"), Some(1));
}
