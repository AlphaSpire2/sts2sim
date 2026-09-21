//! The Lagavulin Matriarch, the Underdocks' third boss, and the sleep it opens
//! behind.
//!
//! No recording reaches this boss, so its whole fight is driven here: the
//! plating and the sleep it stands up with, the branch that keeps it asleep
//! until the sleep has gone, the two ways the sleep ends, and the round of four
//! it walks once it is awake.

use std::sync::Arc;

use sts2_engine::{
    Action, CombatSide, ContentRegistry, Creature, PileName, ScenarioBuilder, Simulator,
};
use sts2_rng::RunStream;

const ENCOUNTER: &str = "ENCOUNTER.LAGAVULIN_MATRIARCH_BOSS";
const MATRIARCH: &str = "MONSTER.LAGAVULIN_MATRIARCH";

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().unwrap()
}

fn card(model_id: &str) -> sts2_engine::CardFingerprint {
    let mut fingerprint = sts2_engine::CardFingerprint::base(id(model_id));
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

fn content_at(ascension: u8) -> Arc<ContentRegistry> {
    let mut registry = (*sts2_content::standard_registry()).clone();
    for definition in sts2_content::monsters_at_ascension(ascension) {
        registry.add_monster(definition);
    }
    Arc::new(registry)
}

/// The matriarch standing on the move named, already carrying the sleep and the
/// plating its arrival works onto it.
fn sleeping(asleep: i32, plating: i32, ascension: u8) -> Simulator {
    matriarch("SLEEP_MOVE", asleep, plating, ascension)
}

fn matriarch(opener: &str, asleep: i32, plating: i32, ascension: u8) -> Simulator {
    let hp = if ascension >= 8 { 233 } else { 222 };
    let mut scenario = ScenarioBuilder::new("LAGATEST", id(ENCOUNTER))
        .player_hp(200, 200)
        .energy(3, 3)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .relics(&[id("RELIC.BURNING_BLOOD")])
        .enemy(1, id(MATRIARCH), hp, hp, opener);
    if asleep > 0 {
        scenario = scenario.power(
            1,
            id("POWER.ASLEEP_POWER"),
            asleep,
            sts2_engine::PowerType::Buff,
        );
    }
    if plating > 0 {
        scenario = scenario.power(
            1,
            id("POWER.PLATING_POWER"),
            plating,
            sts2_engine::PowerType::Buff,
        );
    }
    for card_id in 0..5 {
        scenario = scenario.card(PileName::Hand, card_id, card("CARD.STRIKE_IRONCLAD"), 1);
    }
    scenario.build(content_at(ascension)).unwrap()
}

fn enemy(simulator: &Simulator) -> &Creature {
    simulator
        .state()
        .combat
        .as_ref()
        .unwrap()
        .creatures
        .iter()
        .find(|creature| creature.side == CombatSide::Enemy)
        .unwrap()
}

fn player(simulator: &Simulator) -> &Creature {
    simulator
        .state()
        .combat
        .as_ref()
        .unwrap()
        .creatures
        .iter()
        .find(|creature| creature.side == CombatSide::Player)
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

fn strike(simulator: &mut Simulator) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::PlayCard { card, .. }
            if card.fingerprint.model_id == id("CARD.STRIKE_IRONCLAD"))
        })
        .cloned()
        .unwrap();
    simulator.step(action).unwrap();
}

fn damage_of_one_turn(simulator: &mut Simulator) -> i32 {
    let before = player(simulator).current_hp;
    end_turn(simulator);
    before - player(simulator).current_hp
}

/// The matriarch is one size with no range to roll in, its machine opens on the
/// sleep, and what its arrival works is the twelve plating and the three sleep.
#[test]
fn the_matriarch_stands_up_asleep_behind_twelve_plating() {
    let fight = matriarch("SLEEP_MOVE", 0, 0, 0);
    let standing = enemy(&fight);
    assert_eq!(standing.max_hp, 222, "one size, and no range to roll in");
    assert_eq!(
        standing.move_state, "SLEEP_MOVE",
        "and the machine opens on the sleep"
    );

    // What the arrival works is checked through the model rather than the
    // scenario, because a scenario places the powers itself; the room that
    // generates the fight is what witnesses them standing.
    let definition = sts2_content::standard_registry()
        .monster(&id(MATRIARCH))
        .expect("the matriarch is content")
        .clone();
    assert_eq!(
        definition.on_added.len(),
        2,
        "twelve plating and three sleep"
    );
}

/// The sleep counts down at the end of every turn the matriarch stood through,
/// and the branch after the sleeping move keeps it asleep while it lasts. The
/// enemies roll their next move as the player's turn opens, so the branch reads
/// the count after that turn's end has taken one off it.
#[test]
fn the_sleep_runs_out_and_the_matriarch_wakes_into_the_slash() {
    let mut fight = sleeping(3, 12, 0);
    assert_eq!(damage_of_one_turn(&mut fight), 0, "sleeping hits nobody");
    assert_eq!(enemy(&fight).power_amount("POWER.ASLEEP_POWER"), 2);
    assert_eq!(enemy(&fight).move_state, "SLEEP_MOVE", "and on it sleeps");

    assert_eq!(damage_of_one_turn(&mut fight), 0);
    assert_eq!(enemy(&fight).power_amount("POWER.ASLEEP_POWER"), 1);
    assert_eq!(enemy(&fight).move_state, "SLEEP_MOVE");

    // The last count is where the shell comes off: `BeforeSideTurnEndVeryEarly`
    // takes the plating before the plating's own early pass would put its block
    // up again, and the count going leaves nothing for the branch to read.
    assert_eq!(damage_of_one_turn(&mut fight), 0);
    assert_eq!(enemy(&fight).power_amount("POWER.ASLEEP_POWER"), 0);
    assert_eq!(
        enemy(&fight).power_amount("POWER.PLATING_POWER"),
        0,
        "the shell comes off with the last count"
    );
    assert_eq!(enemy(&fight).block, 0, "and no block goes up behind it");
    assert_eq!(
        enemy(&fight).move_state,
        "SLASH_MOVE",
        "three turns asleep, and the fourth is the slash"
    );

    assert_eq!(damage_of_one_turn(&mut fight), 19, "the slash is nineteen");
}

/// A blow that gets past the shell ends the sleep where it lands: the plating
/// goes, the matriarch is stunned, and it picks its machine up on the slash.
#[test]
fn a_blow_that_gets_through_wakes_the_matriarch_into_a_stun() {
    let mut fight = sleeping(3, 12, 0);
    strike(&mut fight);
    let woken = enemy(&fight);
    assert_eq!(
        woken.power_amount("POWER.ASLEEP_POWER"),
        0,
        "the sleep is spent by the blow, whatever it was worth"
    );
    assert_eq!(
        woken.power_amount("POWER.PLATING_POWER"),
        0,
        "and the shell comes off with it"
    );
    assert_eq!(woken.move_state, "STUNNED", "the matriarch is stunned");
    assert_eq!(
        woken.stunned_follow_up.as_deref(),
        Some("SLASH_MOVE"),
        "and picks its machine back up on the slash"
    );

    // The stunned turn performs nothing at all: waking is sound and a flag.
    assert_eq!(damage_of_one_turn(&mut fight), 0);
    assert_eq!(enemy(&fight).move_state, "SLASH_MOVE");
    assert_eq!(damage_of_one_turn(&mut fight), 19);
}

/// Awake, it walks a round of four: the slash, a twofold disembowelling, a
/// second slash it guards behind, and the siphon.
#[test]
fn the_waking_matriarch_walks_its_round_of_four() {
    let mut fight = matriarch("SLASH_MOVE", 0, 0, 0);
    assert_eq!(damage_of_one_turn(&mut fight), 19);
    assert_eq!(enemy(&fight).move_state, "DISEMBOWEL_MOVE");

    assert_eq!(damage_of_one_turn(&mut fight), 18, "two of nine");
    assert_eq!(enemy(&fight).move_state, "SLASH2_MOVE");

    assert_eq!(damage_of_one_turn(&mut fight), 12);
    assert_eq!(enemy(&fight).block, 12, "and twelve block behind it");
    assert_eq!(enemy(&fight).move_state, "SOUL_SIPHON_MOVE");

    assert_eq!(damage_of_one_turn(&mut fight), 0, "the siphon hits nobody");
    assert_eq!(
        player(&fight).power_amount("POWER.STRENGTH_POWER"),
        -2,
        "two strength off the player"
    );
    assert_eq!(
        player(&fight).power_amount("POWER.DEXTERITY_POWER"),
        -2,
        "and two dexterity"
    );
    assert_eq!(
        enemy(&fight).power_amount("POWER.STRENGTH_POWER"),
        2,
        "and two onto itself"
    );
    assert_eq!(enemy(&fight).move_state, "SLASH_MOVE", "and round it goes");

    // The slash is heavier now for the strength it siphoned, and lighter for
    // nothing: the player's own strength is not what a monster's blow reads.
    assert_eq!(damage_of_one_turn(&mut fight), 21);
}

/// Its ascension branches: a bigger matriarch, a twenty-one slash, a
/// disembowelling of tens, and a second slash of fourteen behind fourteen.
#[test]
fn the_matriarch_carries_its_ascension_branches() {
    let ascended = content_at(9);
    let definition = ascended
        .monster(&id(MATRIARCH))
        .expect("the matriarch is content");
    assert_eq!(definition.min_initial_hp, 233);
    assert_eq!(definition.max_initial_hp, 233);

    let mut fight = matriarch("SLASH_MOVE", 0, 0, 9);
    assert_eq!(damage_of_one_turn(&mut fight), 21);
    assert_eq!(damage_of_one_turn(&mut fight), 20, "two of ten");
    assert_eq!(damage_of_one_turn(&mut fight), 14);
    assert_eq!(enemy(&fight).block, 14, "and fourteen block behind it");
}
