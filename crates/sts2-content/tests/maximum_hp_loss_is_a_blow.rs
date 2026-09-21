//! Maximum HP taken off a creature standing above its new maximum is a blow,
//! not a clamp.
//!
//! The current HP left over the new maximum comes off through the damage
//! pipeline — nobody deals it, no card is behind it, block does not stop it and
//! nothing scales it — and only then does the maximum come down. The difference
//! is invisible in the HP numbers and visible in everything that watches for
//! damage: the turn's record of having been hurt, and the listeners that answer
//! to a blow landing.
//!
//! A creature already standing at or below its new maximum loses no HP, so
//! there is no blow at all, and nothing that watches for one sees anything.

use sts2_engine::{Action, CardFingerprint, PileName, PowerType, ScenarioBuilder, Simulator};
use sts2_rng::RunStream;

const SEED: &str = "MAXHPLOSS";

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str, upgrade_level: u8) -> CardFingerprint {
    let mut fingerprint = CardFingerprint::base(id(model_id));
    fingerprint.upgrade_level = upgrade_level;
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

/// One fight against a single beetle, with the player's HP where the test wants
/// it and enough energy that a cost never gets in the way of what is measured.
///
/// The draw pile is stocked because the brightest flame draws, and a draw that
/// had to reshuffle would be a second thing happening in the same play.
fn fight(current_hp: i32, hand: &[(&str, u8)], powers: &[(&str, i32)]) -> Simulator {
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(current_hp, 80)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE");
    let mut next_id = 0;
    for (model_id, upgrade) in hand {
        scenario = scenario.card(PileName::Hand, next_id, card(model_id, *upgrade), 0);
        next_id += 1;
    }
    for _ in 0..3 {
        scenario = scenario.card(PileName::Draw, next_id, card("CARD.STRIKE_IRONCLAD", 0), 0);
        next_id += 1;
    }
    for (model_id, amount) in powers {
        scenario = scenario.power(0, id(model_id), *amount, PowerType::Buff);
    }
    scenario
        .build(sts2_content::standard_registry())
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
        .unwrap_or_else(|| panic!("{model_id} was not legal"));
    simulator.step(action).expect("the play applies");
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

/// A point of maximum HP taken off a creature at full health takes a point of
/// current HP with it, and the turn remembers having been hurt: a spite thrown
/// afterwards lands its whole run of hits.
#[test]
fn a_maximum_hp_loss_at_full_health_is_a_blow_the_turn_remembers() {
    let mut simulator = fight(80, &[("CARD.BRIGHTEST_FLAME", 0), ("CARD.SPITE", 0)], &[]);
    play(&mut simulator, "CARD.BRIGHTEST_FLAME");
    assert_eq!(creature(&simulator, 0).max_hp, 79);
    assert_eq!(creature(&simulator, 0).current_hp, 79);

    // Two hits of five rather than one, which is what reading the blow out of
    // the turn's damage record buys.
    play(&mut simulator, "CARD.SPITE");
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 10);
}

/// The same point of maximum HP off a creature already standing below its new
/// maximum costs no current HP, so nothing was hurt and a spite thrown
/// afterwards swings once.
#[test]
fn a_maximum_hp_loss_that_costs_no_current_hp_is_no_blow_at_all() {
    let mut simulator = fight(50, &[("CARD.BRIGHTEST_FLAME", 0), ("CARD.SPITE", 0)], &[]);
    play(&mut simulator, "CARD.BRIGHTEST_FLAME");
    assert_eq!(creature(&simulator, 0).max_hp, 79);
    assert_eq!(creature(&simulator, 0).current_hp, 50);

    play(&mut simulator, "CARD.SPITE");
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 5);
}

/// The blow reaches the listeners that watch for damage. A rupture pays its
/// strength out on the spot rather than holding it against the card being
/// played, because the blow a maximum-HP loss deals names no card as its
/// source.
#[test]
fn a_maximum_hp_loss_reaches_the_listeners_that_watch_for_damage() {
    let mut simulator = fight(
        80,
        &[("CARD.BRIGHTEST_FLAME", 0)],
        &[("POWER.RUPTURE_POWER", 1)],
    );
    play(&mut simulator, "CARD.BRIGHTEST_FLAME");
    assert_eq!(
        creature(&simulator, 0).power_amount("POWER.STRENGTH_POWER"),
        1
    );
}

/// A creature already at its new maximum wakes no listener, because no HP came
/// off it.
#[test]
fn a_maximum_hp_loss_that_costs_nothing_wakes_no_listener() {
    let mut simulator = fight(
        50,
        &[("CARD.BRIGHTEST_FLAME", 0)],
        &[("POWER.RUPTURE_POWER", 1)],
    );
    play(&mut simulator, "CARD.BRIGHTEST_FLAME");
    assert_eq!(
        creature(&simulator, 0).power_amount("POWER.STRENGTH_POWER"),
        0
    );
}
