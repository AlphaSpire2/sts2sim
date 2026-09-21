//! The five Ruby Raiders, and both sides of every number they branch on.
//!
//! No recording in the corpus walks the raiders' room, so each machine is
//! driven here: the order its states come round in, what each move does, and
//! the size it rolls. The ascended half of each branch is driven the same way,
//! against a registry standing the raiders up as a run at
//! ascension nine would.

use std::sync::Arc;

use sts2_engine::{
    Action, CombatSide, ContentRegistry, Creature, PileName, ScenarioBuilder, Simulator,
};
use sts2_rng::RunStream;

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().unwrap()
}

fn card(model_id: &str) -> sts2_engine::CardFingerprint {
    let mut fingerprint = sts2_engine::CardFingerprint::base(id(model_id));
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

/// The content a run at `ascension` would play against, which for everything
/// but the raiders is the baseline content unchanged.
fn content_at(ascension: u8) -> Arc<ContentRegistry> {
    let mut registry = (*sts2_content::standard_registry()).clone();
    for definition in sts2_content::monsters_at_ascension(ascension) {
        registry.add_monster(definition);
    }
    Arc::new(registry)
}

/// One raider alone, standing on the move named, with a hand of Strikes.
fn raider(monster: &str, hp: i32, opener: &str, ascension: u8) -> Simulator {
    let mut scenario = ScenarioBuilder::new("RAIDERSTEST", id("ENCOUNTER.RUBY_RAIDERS_NORMAL"))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .relics(&[id("RELIC.BURNING_BLOOD")])
        .enemy(1, id(monster), hp, hp, opener);
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

/// What the player lost over one enemy turn, which is the whole of a raider
/// move that only hits.
fn damage_of_one_turn(simulator: &mut Simulator) -> i32 {
    let before = player(simulator).current_hp;
    end_turn(simulator);
    before - player(simulator).current_hp
}

fn power_amount(creature: &Creature, power: &str) -> i32 {
    creature.power_amount(power)
}

/// The assassin has one move and follows it up into itself, so a killshot is
/// all it ever does.
#[test]
fn the_assassin_only_ever_takes_the_killshot() {
    let mut fight = raider("MONSTER.ASSASSIN_RUBY_RAIDER", 23, "KILLSHOT_MOVE", 0);
    for _ in 0..3 {
        assert_eq!(enemy(&fight).move_state, "KILLSHOT_MOVE");
        assert_eq!(damage_of_one_turn(&mut fight), 10, "ten a turn, every turn");
    }
}

/// The axe raider swings twice behind its own block and then swings once
/// without it, and goes back to the first swing.
#[test]
fn the_axe_raider_guards_its_first_two_swings_and_not_the_third() {
    let mut fight = raider("MONSTER.AXE_RUBY_RAIDER", 22, "SWING_1", 0);
    assert_eq!(damage_of_one_turn(&mut fight), 5, "the first swing is five");
    assert_eq!(enemy(&fight).block, 5, "and it guards behind five");
    assert_eq!(enemy(&fight).move_state, "SWING_2");

    assert_eq!(damage_of_one_turn(&mut fight), 5, "the second swing too");
    // Block is cleared as the enemy side's turn begins, so what is standing
    // afterwards is only what this move put up.
    assert_eq!(enemy(&fight).block, 5);
    assert_eq!(enemy(&fight).move_state, "BIG_SWING");

    assert_eq!(
        damage_of_one_turn(&mut fight),
        12,
        "the big swing is twelve"
    );
    assert_eq!(enemy(&fight).block, 0, "and it guards behind nothing");
    assert_eq!(enemy(&fight).move_state, "SWING_1", "and round it goes");
}

/// The brute alternates a beating with a roar that leaves it three stronger, so
/// every beating is heavier than the last.
#[test]
fn the_brute_roars_itself_three_stronger_between_beatings() {
    let mut fight = raider("MONSTER.BRUTE_RUBY_RAIDER", 33, "BEAT_MOVE", 0);
    assert_eq!(damage_of_one_turn(&mut fight), 7, "the beating is seven");
    assert_eq!(enemy(&fight).move_state, "ROAR_MOVE");

    assert_eq!(damage_of_one_turn(&mut fight), 0, "the roar hits nobody");
    assert_eq!(
        power_amount(enemy(&fight), "POWER.STRENGTH_POWER"),
        3,
        "and leaves three strength standing"
    );
    assert_eq!(damage_of_one_turn(&mut fight), 10, "seven and three");
    assert_eq!(damage_of_one_turn(&mut fight), 0);
    assert_eq!(
        power_amount(enemy(&fight), "POWER.STRENGTH_POWER"),
        6,
        "the roars stack"
    );
}

/// The crossbow raider's machine opens on the reload, not on the shot.
#[test]
fn the_crossbow_raider_loads_before_it_fires() {
    let mut fight = raider("MONSTER.CROSSBOW_RUBY_RAIDER", 21, "RELOAD_MOVE", 0);
    assert_eq!(damage_of_one_turn(&mut fight), 0, "loading hits nobody");
    assert_eq!(enemy(&fight).block, 3, "and puts three block up");
    assert_eq!(enemy(&fight).move_state, "FIRE_MOVE");

    assert_eq!(damage_of_one_turn(&mut fight), 14, "the shot is fourteen");
    assert_eq!(
        enemy(&fight).move_state,
        "RELOAD_MOVE",
        "then it loads again"
    );
}

/// The tracker marks once and then never stops: the hounds follow themselves
/// up, so the track never comes round again.
#[test]
fn the_tracker_marks_once_and_then_sets_the_hounds_on_forever() {
    let mut fight = raider("MONSTER.TRACKER_RUBY_RAIDER", 25, "TRACK_MOVE", 0);
    assert_eq!(damage_of_one_turn(&mut fight), 0, "the track hits nobody");
    assert_eq!(
        power_amount(player(&fight), "POWER.FRAIL_POWER"),
        2,
        "it leaves two frail"
    );
    assert_eq!(enemy(&fight).move_state, "HOUNDS_MOVE");

    assert_eq!(
        damage_of_one_turn(&mut fight),
        8,
        "eight hounds of one each"
    );
    assert_eq!(
        enemy(&fight).move_state,
        "HOUNDS_MOVE",
        "and the hounds follow themselves up"
    );
    assert_eq!(damage_of_one_turn(&mut fight), 8);
}

/// The intent a raider shows is the move it is about to make, hits and all.
#[test]
fn a_raiders_intent_advertises_the_move_it_stands_on() {
    let fight = raider("MONSTER.TRACKER_RUBY_RAIDER", 25, "HOUNDS_MOVE", 0);
    let intent = enemy(&fight).intent.clone().unwrap();
    assert_eq!(intent.move_id, "HOUNDS_MOVE");
    assert_eq!(intent.intents[0].damage, Some(1));
    assert_eq!(intent.intents[0].repeats, Some(8));

    let axe = raider("MONSTER.AXE_RUBY_RAIDER", 22, "SWING_1", 0);
    let intent = enemy(&axe).intent.clone().unwrap();
    assert_eq!(intent.intents[0].damage, Some(5));
    assert_eq!(
        intent.intents[1].intent_type, "defend",
        "a swing advertises the block behind it as well"
    );
}

/// The far side of every ascension branch the raiders carry: the sizes at
/// Tough Enemies and the damage at Deadly Enemies.
#[test]
fn the_raiders_carry_their_ascension_branches() {
    let sizes = [
        ("MONSTER.ASSASSIN_RUBY_RAIDER", 19, 24),
        ("MONSTER.AXE_RUBY_RAIDER", 21, 23),
        ("MONSTER.BRUTE_RUBY_RAIDER", 31, 34),
        ("MONSTER.CROSSBOW_RUBY_RAIDER", 19, 22),
        ("MONSTER.TRACKER_RUBY_RAIDER", 22, 26),
    ];
    let ascended = content_at(9);
    for (model, minimum, maximum) in sizes {
        let definition = ascended.monster(&id(model)).expect("the raider is content");
        assert_eq!(definition.min_initial_hp, minimum, "{model} at ascension 8");
        assert_eq!(definition.max_initial_hp, maximum, "{model} at ascension 8");
    }
    // And at the baseline the same models carry the other side.
    let baseline = sts2_content::standard_registry();
    assert_eq!(
        baseline
            .monster(&id("MONSTER.BRUTE_RUBY_RAIDER"))
            .unwrap()
            .min_initial_hp,
        30
    );

    // A killshot at Deadly Enemies is eleven rather than ten.
    let mut fight = raider("MONSTER.ASSASSIN_RUBY_RAIDER", 24, "KILLSHOT_MOVE", 9);
    assert_eq!(damage_of_one_turn(&mut fight), 11);

    // The axe raider swings for six and guards behind six, and its big swing is
    // thirteen.
    let mut fight = raider("MONSTER.AXE_RUBY_RAIDER", 23, "SWING_1", 9);
    assert_eq!(damage_of_one_turn(&mut fight), 6);
    assert_eq!(enemy(&fight).block, 6);
    let _ = damage_of_one_turn(&mut fight);
    assert_eq!(damage_of_one_turn(&mut fight), 13);

    // The brute beats for eight, and still roars for three.
    let mut fight = raider("MONSTER.BRUTE_RUBY_RAIDER", 34, "BEAT_MOVE", 9);
    assert_eq!(damage_of_one_turn(&mut fight), 8);
    let _ = damage_of_one_turn(&mut fight);
    assert_eq!(power_amount(enemy(&fight), "POWER.STRENGTH_POWER"), 3);

    // The crossbow raider fires for sixteen, and still loads behind three.
    let mut fight = raider("MONSTER.CROSSBOW_RUBY_RAIDER", 22, "RELOAD_MOVE", 9);
    assert_eq!(damage_of_one_turn(&mut fight), 0);
    assert_eq!(enemy(&fight).block, 3);
    assert_eq!(damage_of_one_turn(&mut fight), 16);

    // And the tracker sets nine hounds on rather than eight, each still worth
    // one.
    let mut fight = raider("MONSTER.TRACKER_RUBY_RAIDER", 26, "HOUNDS_MOVE", 9);
    assert_eq!(damage_of_one_turn(&mut fight), 9);
    let intent = enemy(&fight).intent.clone().unwrap();
    assert_eq!(intent.intents[0].repeats, Some(9));
}
