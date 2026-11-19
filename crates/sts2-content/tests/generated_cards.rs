//! A card a fight rolls up, rather than one dealt out of the run deck.
//!
//! A generated card is made fresh, so it stands at whatever its own saved
//! values open at: a Genetic Algorithm's block at one and a Scythe's damage at
//! thirteen.
//! A rolled card that carried none of them would read every one of them as
//! nothing.

use std::sync::Arc;

use sts2_engine::{
    Action, Amount, CardFilter, CardPileCmd, CardPoolDefinition, CardPoolRef, Command,
    ContentRegistry, DrawPilePosition, PileName, ScenarioBuilder, Simulator,
};
use sts2_rng::RunStream;

const SEED: &str = "GENERATEDCARDS";
const POOL: &str = "CARD_POOL.TEST_GENERATION";
const GENERATOR: &str = "CARD.TEST_GENERATOR";

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

/// The character's registry plus two test models: a pool holding the one card
/// under test, and a free skill whose whole body rolls a card out of it into
/// the hand. The pool entry is the character pool's own, so what the roll sees
/// is the pinned card.
fn registry_rolling(character: &str, pool: &str, generated: &str) -> Arc<ContentRegistry> {
    let mut registry = (*sts2_content::registry_for(&id(character))).clone();
    let entry = registry
        .card_pool(&id(pool))
        .expect("the character pool is registered")
        .iter()
        .find(|card| card.model_id == id(generated))
        .expect("the card stands in its character pool")
        .clone();
    registry.add_card_pool(CardPoolDefinition {
        model_id: id(POOL),
        is_colorless: false,
        cards: vec![entry],
    });
    let mut generator = registry
        .card(&id("CARD.DEFEND_IRONCLAD"))
        .expect("a starter skill to build the generator out of")
        .clone();
    generator.model_id = id(GENERATOR);
    generator.energy_cost = 0;
    generator.upgrade = None;
    generator.supported_upgrade_levels = vec![0];
    generator.effects = vec![Command::CardPile(CardPileCmd::AddGeneratedFromPool {
        pool: CardPoolRef::Named(id(POOL)),
        count: Amount::Fixed(1),
        distinct: true,
        filter: CardFilter::Any,
        pile: PileName::Hand,
        position: DrawPilePosition::Bottom,
        excludes_source: false,
        upgraded: false,
        temporary_cost: None,
        ethereal: false,
        rolled_one_at_a_time: false,
    })];
    registry.add_card(generator);
    Arc::new(registry)
}

/// A fight holding nothing but the generator, standing against one enemy.
fn fight(character: &str, pool: &str, generated: &str, enemy_hp: i32) -> Simulator {
    let registry = registry_rolling(character, pool, generated);
    let cost = registry
        .card(&id(GENERATOR))
        .expect("the generator is registered")
        .energy_cost;
    ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .played_as(id(character))
        .player_hp(75, 75)
        .energy(9, 9)
        .turn(1, 1)
        .enemy(
            1,
            id("MONSTER.SHRINKER_BEETLE"),
            enemy_hp,
            enemy_hp,
            "SHRINKER_MOVE",
        )
        .rng_counter(RunStream::CombatCardGeneration, 0)
        .card(
            PileName::Hand,
            10,
            sts2_engine::CardFingerprint::base(id(GENERATOR)),
            cost,
        )
        .build(registry)
        .expect("the scenario builds")
}

fn play(simulator: &mut Simulator, model_id: &str) {
    let wanted = id(model_id);
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::PlayCard { card, .. } if card.fingerprint.model_id == wanted)
        })
        .cloned()
        .unwrap_or_else(|| panic!("{model_id} was not a legal play"));
    simulator.step(action).expect("the play applies");
}

fn creature(simulator: &Simulator, combat_id: u64) -> &sts2_engine::Creature {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("a fight is standing")
        .creatures
        .iter()
        .find(|creature| creature.id.get() == combat_id)
        .expect("the creature is in the fight")
}

fn hand_card(simulator: &Simulator, model_id: &str) -> sts2_engine::CardInstance {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("a fight is standing")
        .player
        .piles
        .hand
        .iter()
        .find(|card| card.fingerprint.model_id == id(model_id))
        .expect("the roll put the card in hand")
        .clone()
}

fn exhausted(simulator: &Simulator) -> sts2_engine::CardFingerprint {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("a fight is standing")
        .player
        .piles
        .exhaust
        .first()
        .expect("the card burns up for having been played")
        .fingerprint
        .clone()
}

/// The block a Genetic Algorithm pays is its saved `CurrentBlock`, which a
/// fresh card carries at one however the card was made.
#[test]
fn a_generated_genetic_algorithm_pays_the_block_a_fresh_one_holds() {
    let mut simulator = fight(
        "CHARACTER.DEFECT",
        "CARD_POOL.DEFECT_CARD_POOL",
        "CARD.GENETIC_ALGORITHM",
        39,
    );
    play(&mut simulator, GENERATOR);
    let rolled = hand_card(&simulator, "CARD.GENETIC_ALGORITHM");
    assert_eq!(
        rolled.fingerprint.properties.get("CurrentBlock"),
        Some(&serde_json::Value::from(1)),
        "a rolled card carries the property it was created with"
    );
    play(&mut simulator, "CARD.GENETIC_ALGORITHM");
    assert_eq!(creature(&simulator, 0).block, 1);
    // The block is one plus what the card has grown by: what a play grows the
    // card to is measured off the base it was made with.
    assert_eq!(
        exhausted(&simulator).properties.get("CurrentBlock"),
        Some(&serde_json::Value::from(4))
    );
}

/// The damage a Scythe deals is its saved `CurrentDamage`, which a fresh card
/// carries at thirteen however the card was made.
#[test]
fn a_generated_scythe_deals_the_damage_a_fresh_one_holds() {
    let mut simulator = fight(
        "CHARACTER.NECROBINDER",
        "CARD_POOL.NECROBINDER_CARD_POOL",
        "CARD.THE_SCYTHE",
        200,
    );
    play(&mut simulator, GENERATOR);
    let rolled = hand_card(&simulator, "CARD.THE_SCYTHE");
    assert_eq!(
        rolled.fingerprint.properties.get("CurrentDamage"),
        Some(&serde_json::Value::from(13))
    );
    play(&mut simulator, "CARD.THE_SCYTHE");
    assert_eq!(creature(&simulator, 1).current_hp, 200 - 13);
    // The damage is thirteen plus what the card has grown by.
    assert_eq!(
        exhausted(&simulator).properties.get("CurrentDamage"),
        Some(&serde_json::Value::from(18))
    );
}
