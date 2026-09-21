//! The Ironclad's own cards, both forms, pinned one behaviour at a time.
//!
//! No recording in the corpus takes most of these cards, so a scenario is the
//! evidence for each of them: what the play costs, what it deals or works up,
//! what it moves between piles, and what it reads off the state it lands in.

use sts2_engine::{Action, CardFingerprint, DecisionContext, PileName, ScenarioBuilder, Simulator};
use sts2_rng::RunStream;

const SEED: &str = "IRONCLADCARDS";

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str, upgrade_level: u8) -> CardFingerprint {
    let mut fingerprint = CardFingerprint::base(id(model_id));
    fingerprint.upgrade_level = upgrade_level;
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

/// One fight against a single beetle, with the hand the test names and enough
/// energy that a cost never gets in the way of what is being measured.
///
/// Card costs are worked out from the definition at every boundary, so the cost
/// a scenario places a card with is not what the play charges.
fn fight(hand: &[(&str, u8)]) -> Simulator {
    fight_with_draw(hand, &[])
}

fn fight_with_draw(hand: &[(&str, u8)], draw: &[(&str, u8)]) -> Simulator {
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
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
    for (model_id, upgrade) in draw {
        scenario = scenario.card(PileName::Draw, next_id, card(model_id, *upgrade), 0);
        next_id += 1;
    }
    scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

/// The same fight with two beetles, for the cards that sweep.
fn crowd(hand: &[(&str, u8)]) -> Simulator {
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .enemy(2, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE");
    for (index, (model_id, upgrade)) in hand.iter().enumerate() {
        scenario = scenario.card(
            PileName::Hand,
            u64::try_from(index).expect("small index"),
            card(model_id, *upgrade),
            0,
        );
    }
    scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

fn play_action(simulator: &Simulator, model_id: &str) -> Option<Action> {
    let wanted = id(model_id);
    simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::PlayCard { card, .. } if card.fingerprint.model_id == wanted)
        })
        .cloned()
}

fn play(simulator: &mut Simulator, model_id: &str) {
    let action = play_action(simulator, model_id).unwrap_or_else(|| {
        panic!(
            "{model_id} was not legal: {:?}",
            simulator.legal_actions().len()
        )
    });
    simulator.step(action).expect("the play applies");
}

fn choose(simulator: &mut Simulator, model_id: &str) {
    let wanted = id(model_id);
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::ChooseCards { cards, .. }
                if cards.len() == 1 && cards[0].fingerprint.model_id == wanted)
        })
        .cloned()
        .unwrap_or_else(|| panic!("{model_id} was not on the screen"));
    simulator.step(action).expect("the choice applies");
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

fn player(simulator: &Simulator) -> &sts2_engine::Creature {
    creature(simulator, 0)
}

fn pile(simulator: &Simulator, pile: PileName) -> Vec<String> {
    let piles = &simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles;
    let cards = match pile {
        PileName::Draw => &piles.draw,
        PileName::Hand => &piles.hand,
        PileName::Discard => &piles.discard,
        PileName::Exhaust => &piles.exhaust,
        PileName::Play => &piles.play,
    };
    cards
        .iter()
        .map(|card| card.fingerprint.model_id.to_string())
        .collect()
}

fn energy(simulator: &Simulator) -> i32 {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .energy
}

fn power_amount(simulator: &Simulator, combat_id: u64, power: &str) -> i32 {
    creature(simulator, combat_id).power_amount(power)
}

fn power_counter(simulator: &Simulator, combat_id: u64, power: &str) -> i32 {
    let wanted = id(power);
    creature(simulator, combat_id)
        .powers
        .iter()
        .find(|instance| instance.model_id == wanted)
        .map_or(0, |instance| instance.counter)
}

// --- attacks -------------------------------------------------------------

/// A breakthrough pays a point of its owner's own HP, which nothing blocks,
/// before it lands on everything in front of them.
#[test]
fn breakthrough_pays_one_hp_and_sweeps_the_field() {
    for (upgrade, damage) in [(0, 9), (1, 13)] {
        let mut simulator = crowd(&[("CARD.BREAKTHROUGH", upgrade)]);
        play(&mut simulator, "CARD.BREAKTHROUGH");
        assert_eq!(player(&simulator).current_hp, 79);
        assert_eq!(creature(&simulator, 1).current_hp, 39 - damage);
        assert_eq!(creature(&simulator, 2).current_hp, 39 - damage);
    }
}

/// A conflagration lands four small blows on everything, and upgrading buys a
/// fifth rather than a heavier one.
#[test]
fn conflagration_buys_another_hit_rather_than_a_bigger_one() {
    for (upgrade, hits) in [(0, 4), (1, 5)] {
        let mut simulator = crowd(&[("CARD.CONFLAGRATION", upgrade)]);
        play(&mut simulator, "CARD.CONFLAGRATION");
        assert_eq!(creature(&simulator, 1).current_hp, 39 - 2 * hits);
        assert_eq!(creature(&simulator, 2).current_hp, 39 - 2 * hits);
    }
}

/// A thunderclap sweeps and leaves everything shaken, one stack each.
#[test]
fn thunderclap_sweeps_and_shakes_everything() {
    for (upgrade, damage) in [(0, 4), (1, 7)] {
        let mut simulator = crowd(&[("CARD.THUNDERCLAP", upgrade)]);
        play(&mut simulator, "CARD.THUNDERCLAP");
        assert_eq!(creature(&simulator, 1).current_hp, 39 - damage);
        assert_eq!(creature(&simulator, 2).current_hp, 39 - damage);
        assert_eq!(power_amount(&simulator, 1, "POWER.VULNERABLE_POWER"), 1);
        assert_eq!(power_amount(&simulator, 2, "POWER.VULNERABLE_POWER"), 1);
    }
}

/// Fiend fire burns the whole hand and swings once for every card that was in
/// it — the count taken before any of it burns, and the fire itself is not part
/// of it.
#[test]
fn fiend_fire_swings_once_for_every_card_the_hand_held() {
    for (upgrade, damage) in [(0, 7), (1, 10)] {
        let mut simulator = fight(&[
            ("CARD.FIEND_FIRE", upgrade),
            ("CARD.STRIKE_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
        ]);
        play(&mut simulator, "CARD.FIEND_FIRE");
        assert_eq!(creature(&simulator, 1).current_hp, 39 - 2 * damage);
        assert_eq!(pile(&simulator, PileName::Hand), Vec::<String>::new());
        let exhaust = pile(&simulator, PileName::Exhaust);
        assert_eq!(exhaust.len(), 3, "the hand burns and so does the fire");
        assert!(exhaust.contains(&"CARD.FIEND_FIRE".to_owned()));
    }
}

/// An empty hand still plays: nothing burns and the blow lands no times at all.
#[test]
fn fiend_fire_on_an_empty_hand_swings_nothing() {
    let mut simulator = fight(&[("CARD.FIEND_FIRE", 0)]);
    play(&mut simulator, "CARD.FIEND_FIRE");
    assert_eq!(creature(&simulator, 1).current_hp, 39);
    assert_eq!(pile(&simulator, PileName::Exhaust), ["CARD.FIEND_FIRE"]);
}

/// A mangling takes the target's strength for the turn and hands it back as the
/// turn ends, which is what the temporary power is for.
#[test]
fn mangle_takes_strength_for_one_turn_and_gives_it_back() {
    let mut simulator = fight(&[("CARD.MANGLE", 0)]);
    play(&mut simulator, "CARD.MANGLE");
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 15);
    assert_eq!(power_amount(&simulator, 1, "POWER.MANGLE_POWER"), 10);
    assert_eq!(power_amount(&simulator, 1, "POWER.STRENGTH_POWER"), -10);

    end_turn(&mut simulator);
    assert_eq!(
        power_amount(&simulator, 1, "POWER.MANGLE_POWER"),
        0,
        "the mangling comes off at the end of a turn its target took part in"
    );
    assert_eq!(power_amount(&simulator, 1, "POWER.STRENGTH_POWER"), 0);
}

/// Upgrading a mangling pays five more damage and five more strength.
#[test]
fn an_upgraded_mangle_takes_five_more_of_each() {
    let mut simulator = fight(&[("CARD.MANGLE", 1)]);
    play(&mut simulator, "CARD.MANGLE");
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 20);
    assert_eq!(power_amount(&simulator, 1, "POWER.STRENGTH_POWER"), -15);
}

/// A setup strike leaves its owner stronger for the turn, and counts as a
/// Strike for whatever is counting them.
#[test]
fn setup_strike_lends_its_owner_strength_for_the_turn() {
    for (upgrade, damage, strength) in [(0, 7, 2), (1, 9, 3)] {
        let mut simulator = fight(&[("CARD.SETUP_STRIKE", upgrade)]);
        play(&mut simulator, "CARD.SETUP_STRIKE");
        assert_eq!(creature(&simulator, 1).current_hp, 39 - damage);
        assert_eq!(
            power_amount(&simulator, 0, "POWER.STRENGTH_POWER"),
            strength
        );
        end_turn(&mut simulator);
        assert_eq!(power_amount(&simulator, 0, "POWER.STRENGTH_POWER"), 0);
    }
}

/// A pact's end asks the exhaust pile first: three cards in it and the sweep
/// lands, fewer and the play does nothing at all.
#[test]
fn pacts_end_only_swings_once_three_cards_have_burned() {
    let mut simulator = crowd(&[("CARD.PACTS_END", 0)]);
    play(&mut simulator, "CARD.PACTS_END");
    assert_eq!(
        creature(&simulator, 1).current_hp,
        39,
        "nothing has burned yet, so nothing lands"
    );

    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .card(PileName::Hand, 0, card("CARD.PACTS_END", 0), 0)
        .card(PileName::Exhaust, 1, card("CARD.STRIKE_IRONCLAD", 0), 0)
        .card(PileName::Exhaust, 2, card("CARD.STRIKE_IRONCLAD", 0), 0)
        .card(PileName::Exhaust, 3, card("CARD.STRIKE_IRONCLAD", 0), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.PACTS_END");
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 17);
}

// --- skills --------------------------------------------------------------

/// Armaments works up block and then upgrades: one card of the hand picked out
/// of it, and once it is itself upgraded, every upgradable card in it.
#[test]
fn armaments_upgrades_one_card_and_upgraded_it_upgrades_them_all() {
    let mut simulator = fight(&[
        ("CARD.ARMAMENTS", 0),
        ("CARD.STRIKE_IRONCLAD", 0),
        ("CARD.DEFEND_IRONCLAD", 0),
    ]);
    play(&mut simulator, "CARD.ARMAMENTS");
    assert_eq!(player(&simulator).block, 5);
    assert!(matches!(
        simulator.decision(),
        DecisionContext::ChooseCards { .. }
    ));
    choose(&mut simulator, "CARD.STRIKE_IRONCLAD");
    let hand = &simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles
        .hand;
    assert_eq!(hand[0].fingerprint.model_id, id("CARD.STRIKE_IRONCLAD"));
    assert_eq!(hand[0].fingerprint.upgrade_level, 1);
    assert_eq!(hand[1].fingerprint.upgrade_level, 0);

    let mut simulator = fight(&[
        ("CARD.ARMAMENTS", 1),
        ("CARD.STRIKE_IRONCLAD", 0),
        ("CARD.DEFEND_IRONCLAD", 0),
    ]);
    play(&mut simulator, "CARD.ARMAMENTS");
    assert_eq!(
        player(&simulator).block,
        5,
        "the upgrade buys the whole hand, not more block"
    );
    let hand = &simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles
        .hand;
    assert!(hand.iter().all(|card| card.fingerprint.upgrade_level == 1));
}

/// A blood wall costs two of its owner's own HP before the block goes up.
#[test]
fn blood_wall_pays_two_hp_for_its_block() {
    for (upgrade, block) in [(0, 16), (1, 20)] {
        let mut simulator = fight(&[("CARD.BLOOD_WALL", upgrade)]);
        play(&mut simulator, "CARD.BLOOD_WALL");
        assert_eq!(player(&simulator).current_hp, 78);
        assert_eq!(player(&simulator).block, block);
    }
}

/// Bloodletting is three HP for energy, and the upgrade buys a third point of
/// it rather than cheaper blood.
#[test]
fn bloodletting_trades_three_hp_for_energy() {
    for (upgrade, gained) in [(0, 2), (1, 3)] {
        let mut simulator = fight(&[("CARD.BLOODLETTING", upgrade)]);
        let before = energy(&simulator);
        play(&mut simulator, "CARD.BLOODLETTING");
        assert_eq!(player(&simulator).current_hp, 77);
        assert_eq!(energy(&simulator), before + gained);
    }
}

/// A brand costs a point of blood, burns a card off the hand, and pays strength
/// for it — in that order, so the strength lands after the screen is answered.
#[test]
fn brand_burns_a_chosen_card_and_pays_strength_for_it() {
    let mut simulator = fight(&[
        ("CARD.BRAND", 0),
        ("CARD.STRIKE_IRONCLAD", 0),
        ("CARD.DEFEND_IRONCLAD", 0),
    ]);
    play(&mut simulator, "CARD.BRAND");
    assert_eq!(player(&simulator).current_hp, 79);
    assert_eq!(
        power_amount(&simulator, 0, "POWER.STRENGTH_POWER"),
        0,
        "the strength waits behind the screen"
    );
    choose(&mut simulator, "CARD.DEFEND_IRONCLAD");
    assert_eq!(
        pile(&simulator, PileName::Exhaust),
        ["CARD.DEFEND_IRONCLAD"]
    );
    assert_eq!(power_amount(&simulator, 0, "POWER.STRENGTH_POWER"), 1);
}

/// With one card in hand there is nothing to choose between, so the screen does
/// not come up at all and the play settles in one step.
#[test]
fn brand_with_one_card_in_hand_burns_it_without_asking() {
    let mut simulator = fight(&[("CARD.BRAND", 1), ("CARD.STRIKE_IRONCLAD", 0)]);
    play(&mut simulator, "CARD.BRAND");
    assert_eq!(
        pile(&simulator, PileName::Exhaust),
        ["CARD.STRIKE_IRONCLAD"]
    );
    assert_eq!(power_amount(&simulator, 0, "POWER.STRENGTH_POWER"), 2);
}

/// A burning pact burns a card and draws for it, and the draw happens after the
/// burning rather than before.
#[test]
fn burning_pact_burns_first_and_draws_after() {
    for (upgrade, drawn) in [(0, 2), (1, 3)] {
        let mut simulator = fight_with_draw(
            &[("CARD.BURNING_PACT", upgrade), ("CARD.STRIKE_IRONCLAD", 0)],
            &[
                ("CARD.DEFEND_IRONCLAD", 0),
                ("CARD.DEFEND_IRONCLAD", 0),
                ("CARD.DEFEND_IRONCLAD", 0),
            ],
        );
        play(&mut simulator, "CARD.BURNING_PACT");
        assert_eq!(
            pile(&simulator, PileName::Exhaust),
            ["CARD.STRIKE_IRONCLAD"]
        );
        assert_eq!(pile(&simulator, PileName::Hand).len(), drawn);
    }
}

/// Impervious is block and nothing else, and it burns up behind itself.
#[test]
fn impervious_is_block_that_burns_up() {
    for (upgrade, block) in [(0, 30), (1, 40)] {
        let mut simulator = fight(&[("CARD.IMPERVIOUS", upgrade)]);
        play(&mut simulator, "CARD.IMPERVIOUS");
        assert_eq!(player(&simulator).block, block);
        assert_eq!(pile(&simulator, PileName::Exhaust), ["CARD.IMPERVIOUS"]);
    }
}

/// A heal never carries its owner past their maximum.
#[test]
fn not_yet_heals_and_stops_at_the_maximum() {
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(50, 80)
        .energy(9, 9)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .card(PileName::Hand, 0, card("CARD.NOT_YET", 0), 0)
        .card(PileName::Hand, 1, card("CARD.NOT_YET", 1), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.NOT_YET");
    assert_eq!(player(&simulator).current_hp, 60);
    assert_eq!(pile(&simulator, PileName::Exhaust), ["CARD.NOT_YET"]);

    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(75, 80)
        .energy(9, 9)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .card(PileName::Hand, 0, card("CARD.NOT_YET", 1), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.NOT_YET");
    assert_eq!(player(&simulator).current_hp, 80);
}

/// Shrugging it off is block and a card; upgrading buys block, not a card.
#[test]
fn shrug_it_off_draws_one_card_at_either_level() {
    for (upgrade, block) in [(0, 8), (1, 11)] {
        let mut simulator = fight_with_draw(
            &[("CARD.SHRUG_IT_OFF", upgrade)],
            &[("CARD.DEFEND_IRONCLAD", 0), ("CARD.DEFEND_IRONCLAD", 0)],
        );
        play(&mut simulator, "CARD.SHRUG_IT_OFF");
        assert_eq!(player(&simulator).block, block);
        assert_eq!(pile(&simulator, PileName::Hand).len(), 1);
    }
}

/// True grit burns whatever the roll picks out of the hand; upgraded, it asks
/// instead — and the roll it would have made is not read.
#[test]
fn true_grit_rolls_its_card_and_upgraded_asks_for_one() {
    let mut simulator = fight(&[
        ("CARD.TRUE_GRIT", 0),
        ("CARD.STRIKE_IRONCLAD", 0),
        ("CARD.DEFEND_IRONCLAD", 0),
    ]);
    let before = simulator
        .state()
        .rng
        .stream(RunStream::CombatCardSelection)
        .counter();
    play(&mut simulator, "CARD.TRUE_GRIT");
    assert_eq!(player(&simulator).block, 7);
    assert_eq!(pile(&simulator, PileName::Exhaust).len(), 1);
    assert_eq!(
        simulator
            .state()
            .rng
            .stream(RunStream::CombatCardSelection)
            .counter(),
        before + 1,
        "the card is rolled on the combat card-selection stream"
    );

    let mut simulator = fight(&[
        ("CARD.TRUE_GRIT", 1),
        ("CARD.STRIKE_IRONCLAD", 0),
        ("CARD.DEFEND_IRONCLAD", 0),
    ]);
    play(&mut simulator, "CARD.TRUE_GRIT");
    assert_eq!(player(&simulator).block, 9);
    assert!(matches!(
        simulator.decision(),
        DecisionContext::ChooseCards { .. }
    ));
    choose(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(
        pile(&simulator, PileName::Exhaust),
        ["CARD.STRIKE_IRONCLAD"]
    );
}

/// A beetle already wound up to stomp for thirteen, for the cards that answer
/// a blow rather than deal one.
fn stomping_beetle(hand: &[(&str, u8)], shaken: bool) -> Simulator {
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .rng_counter(RunStream::MonsterAi, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "STOMP_MOVE");
    if shaken {
        scenario = scenario.power(
            1,
            id("POWER.VULNERABLE_POWER"),
            1,
            sts2_engine::PowerType::Debuff,
        );
    }
    for (index, (model_id, upgrade)) in hand.iter().enumerate() {
        scenario = scenario.card(
            PileName::Hand,
            u64::try_from(index).expect("small index"),
            card(model_id, *upgrade),
            0,
        );
    }
    scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

/// A colossus halves what a shaken attacker gets through, and only a shaken
/// one: the power reads whoever is swinging, not its own owner.
#[test]
fn a_colossus_halves_what_a_shaken_attacker_lands() {
    let mut simulator = stomping_beetle(&[("CARD.COLOSSUS", 0)], false);
    play(&mut simulator, "CARD.COLOSSUS");
    assert_eq!(player(&simulator).block, 5);
    end_turn(&mut simulator);
    assert_eq!(
        player(&simulator).current_hp,
        80 - (13 - 5),
        "an unshaken stomp lands whole"
    );

    let mut simulator = stomping_beetle(&[("CARD.COLOSSUS", 0)], true);
    play(&mut simulator, "CARD.COLOSSUS");
    end_turn(&mut simulator);
    assert_eq!(
        player(&simulator).current_hp,
        80 - (13 / 2 - 5),
        "a shaken one is halved before the block takes any of it"
    );
    assert_eq!(
        power_amount(&simulator, 0, "POWER.COLOSSUS_POWER"),
        0,
        "one stack is spent for the enemy turn that ended"
    );
}

/// The upgrade buys block only.
#[test]
fn an_upgraded_colossus_stands_behind_more_block() {
    let mut simulator = fight(&[("CARD.COLOSSUS", 1)]);
    play(&mut simulator, "CARD.COLOSSUS");
    assert_eq!(player(&simulator).block, 8);
    assert_eq!(power_amount(&simulator, 0, "POWER.COLOSSUS_POWER"), 1);
}

/// A flame barrier burns whoever swings at its owner, for as much as the
/// barrier is worth, and burns out at the end of the other side's turn.
#[test]
fn a_flame_barrier_burns_whoever_swings_and_then_burns_out() {
    for (upgrade, block, back) in [(0, 12, 4), (1, 16, 6)] {
        let mut simulator = stomping_beetle(&[("CARD.FLAME_BARRIER", upgrade)], false);
        play(&mut simulator, "CARD.FLAME_BARRIER");
        assert_eq!(player(&simulator).block, block);
        assert_eq!(
            power_amount(&simulator, 0, "POWER.FLAME_BARRIER_POWER"),
            back
        );
        end_turn(&mut simulator);
        assert_eq!(creature(&simulator, 1).current_hp, 39 - back);
        assert_eq!(
            power_amount(&simulator, 0, "POWER.FLAME_BARRIER_POWER"),
            0,
            "the barrier is gone once the other side's turn has ended"
        );
    }
}

// --- powers --------------------------------------------------------------

/// Stone Armor applies its Plating once. Power cards are moved out of play as
/// part of resolving them, but that must not run the card's effects again.
#[test]
fn stone_armor_applies_one_stack_of_its_printed_plating() {
    for (upgrade, plating) in [(0, 4), (1, 6)] {
        let mut simulator = fight(&[("CARD.STONE_ARMOR", upgrade)]);
        play(&mut simulator, "CARD.STONE_ARMOR");
        assert_eq!(power_amount(&simulator, 0, "POWER.PLATING_POWER"), plating);
    }
}

/// A barricade keeps the block its owner is standing behind, and upgrading only
/// makes it cheaper.
#[test]
fn a_barricade_keeps_its_owners_block_through_the_turn() {
    let mut simulator = fight(&[("CARD.BARRICADE", 0), ("CARD.DEFEND_IRONCLAD", 0)]);
    play(&mut simulator, "CARD.BARRICADE");
    play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    assert_eq!(player(&simulator).block, 5);
    end_turn(&mut simulator);
    assert!(
        player(&simulator).block >= 5,
        "block is not cleared while the barricade stands"
    );

    let costs = |upgrade| {
        let simulator = fight(&[("CARD.BARRICADE", upgrade)]);
        simulator
            .state()
            .combat
            .as_ref()
            .expect("the fight is standing")
            .player
            .piles
            .hand[0]
            .energy_cost
    };
    assert_eq!(costs(0), 3);
    assert_eq!(costs(1), 2);
}

/// Cruelty carries nothing of its own: it makes every shaking its owner works
/// count for more, which Vulnerable reads off whoever is swinging.
#[test]
fn cruelty_raises_what_a_shaking_its_owner_worked_is_worth() {
    let plain = {
        let mut simulator = fight(&[("CARD.TAUNT", 0), ("CARD.STRIKE_IRONCLAD", 0)]);
        play(&mut simulator, "CARD.TAUNT");
        play(&mut simulator, "CARD.STRIKE_IRONCLAD");
        39 - creature(&simulator, 1).current_hp
    };
    assert_eq!(plain, 9, "six through a shaking is nine");

    for (upgrade, dealt) in [(0, 10), (1, 12)] {
        let mut simulator = fight(&[
            ("CARD.CRUELTY", upgrade),
            ("CARD.TAUNT", 0),
            ("CARD.STRIKE_IRONCLAD", 0),
        ]);
        play(&mut simulator, "CARD.CRUELTY");
        play(&mut simulator, "CARD.TAUNT");
        play(&mut simulator, "CARD.STRIKE_IRONCLAD");
        assert_eq!(39 - creature(&simulator, 1).current_hp, dealt);
    }
}

/// A demon form hands its owner strength as their own turn opens, and not on
/// the turn it was laid down.
#[test]
fn a_demon_form_pays_out_at_the_start_of_its_owners_turn() {
    for (upgrade, strength) in [(0, 2), (1, 3)] {
        let mut simulator = fight(&[("CARD.DEMON_FORM", upgrade)]);
        play(&mut simulator, "CARD.DEMON_FORM");
        assert_eq!(
            power_amount(&simulator, 0, "POWER.STRENGTH_POWER"),
            0,
            "the form pays nothing on the turn it is laid down"
        );
        end_turn(&mut simulator);
        assert_eq!(
            power_amount(&simulator, 0, "POWER.STRENGTH_POWER"),
            strength
        );
    }
}

/// A pyre raises how much energy its owner's turn opens with.
#[test]
fn a_pyre_raises_the_energy_a_turn_opens_with() {
    for (upgrade, extra) in [(0, 1), (1, 2)] {
        let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .player_hp(80, 80)
            .energy(3, 3)
            .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
            .card(PileName::Hand, 0, card("CARD.PYRE", upgrade), 0)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
        play(&mut simulator, "CARD.PYRE");
        assert_eq!(energy(&simulator), 1, "the pyre costs two of the three");
        end_turn(&mut simulator);
        assert_eq!(energy(&simulator), 3 + extra);
    }
}

// --- what burns, and what answers for it ---------------------------------

/// Feeling no pain is block for every card of its owner's that burns, and
/// nothing scales it.
#[test]
fn feel_no_pain_pays_block_for_every_card_that_burns() {
    for (upgrade, per_card) in [(0, 3), (1, 4)] {
        let mut simulator = fight(&[
            ("CARD.FEEL_NO_PAIN", upgrade),
            ("CARD.TRUE_GRIT", 0),
            ("CARD.STRIKE_IRONCLAD", 0),
        ]);
        play(&mut simulator, "CARD.FEEL_NO_PAIN");
        assert_eq!(player(&simulator).block, 0);
        play(&mut simulator, "CARD.TRUE_GRIT");
        assert_eq!(
            player(&simulator).block,
            7 + per_card,
            "the grit's block and the block the burn paid"
        );
    }
}

/// A card that burns for having been played pays the same way one a body
/// burned does: ending a play in the exhaust pile is a burn like any other.
#[test]
fn a_card_that_burns_as_it_is_played_pays_feel_no_pain_too() {
    let mut simulator = fight(&[("CARD.FEEL_NO_PAIN", 0), ("CARD.IMPERVIOUS", 0)]);
    play(&mut simulator, "CARD.FEEL_NO_PAIN");
    play(&mut simulator, "CARD.IMPERVIOUS");
    assert_eq!(player(&simulator).block, 30 + 3);
}

/// A dark embrace draws for every card that burns; a card that burns up in hand
/// as the turn ends is counted instead and paid for once the hand is down.
#[test]
fn a_dark_embrace_draws_for_a_burn_and_defers_an_ethereal_one() {
    let mut simulator = fight_with_draw(
        &[
            ("CARD.DARK_EMBRACE", 0),
            ("CARD.TRUE_GRIT", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
        ],
        &[
            ("CARD.STRIKE_IRONCLAD", 0),
            ("CARD.STRIKE_IRONCLAD", 0),
            ("CARD.STRIKE_IRONCLAD", 0),
        ],
    );
    play(&mut simulator, "CARD.DARK_EMBRACE");
    play(&mut simulator, "CARD.TRUE_GRIT");
    assert_eq!(
        pile(&simulator, PileName::Hand).len(),
        1,
        "the grit burned the one card left in hand and the embrace drew one back"
    );
    assert_eq!(pile(&simulator, PileName::Draw).len(), 2);
}

/// Upgrading a dark embrace makes it cheaper and nothing else.
#[test]
fn an_upgraded_dark_embrace_only_costs_less() {
    let cost = |upgrade| {
        let simulator = fight(&[("CARD.DARK_EMBRACE", upgrade)]);
        simulator
            .state()
            .combat
            .as_ref()
            .expect("the fight is standing")
            .player
            .piles
            .hand[0]
            .energy_cost
    };
    assert_eq!(cost(0), 2);
    assert_eq!(cost(1), 1);
}

/// An evil eye works its block up once, and twice over when something of its
/// owner's has already burned this turn.
#[test]
fn an_evil_eye_pays_twice_once_something_has_burned() {
    for (upgrade, block_amount) in [(0, 8), (1, 11)] {
        let mut simulator = fight(&[("CARD.EVIL_EYE", upgrade)]);
        play(&mut simulator, "CARD.EVIL_EYE");
        assert_eq!(player(&simulator).block, block_amount);

        let mut simulator = fight(&[
            ("CARD.EVIL_EYE", upgrade),
            ("CARD.IMPERVIOUS", 0),
            ("CARD.STRIKE_IRONCLAD", 0),
        ]);
        play(&mut simulator, "CARD.IMPERVIOUS");
        play(&mut simulator, "CARD.EVIL_EYE");
        assert_eq!(player(&simulator).block, 30 + 2 * block_amount);
    }
}

/// A forgotten ritual asks before it burns itself, so playing it first pays
/// nothing at all.
#[test]
fn a_forgotten_ritual_does_not_count_its_own_burn() {
    for (upgrade, gained) in [(0, 3), (1, 4)] {
        let mut simulator = fight(&[("CARD.FORGOTTEN_RITUAL", upgrade)]);
        let before = energy(&simulator);
        play(&mut simulator, "CARD.FORGOTTEN_RITUAL");
        assert_eq!(
            energy(&simulator),
            before - 1,
            "it costs its energy and pays none back"
        );
        assert_eq!(
            pile(&simulator, PileName::Exhaust),
            ["CARD.FORGOTTEN_RITUAL"]
        );

        let mut simulator = fight(&[
            ("CARD.FORGOTTEN_RITUAL", upgrade),
            ("CARD.IMPERVIOUS", 0),
            ("CARD.STRIKE_IRONCLAD", 0),
        ]);
        play(&mut simulator, "CARD.IMPERVIOUS");
        let before = energy(&simulator);
        play(&mut simulator, "CARD.FORGOTTEN_RITUAL");
        assert_eq!(energy(&simulator), before - 1 + gained);
    }
}

/// A drum of battle draws when it is played and pays energy when it burns,
/// whatever burned it — and the burning is not the playing.
#[test]
fn a_drum_of_battle_pays_its_energy_when_it_burns() {
    for (upgrade, gained) in [(0, 2), (1, 3)] {
        let mut simulator = fight_with_draw(
            &[("CARD.DRUM_OF_BATTLE", upgrade), ("CARD.TRUE_GRIT", 1)],
            &[("CARD.STRIKE_IRONCLAD", 0), ("CARD.STRIKE_IRONCLAD", 0)],
        );
        let before = energy(&simulator);
        play(&mut simulator, "CARD.DRUM_OF_BATTLE");
        assert_eq!(
            energy(&simulator),
            before - 1,
            "playing it only draws; the drum is in the discard pile, not burned"
        );
        assert_eq!(pile(&simulator, PileName::Hand).len(), 3);

        // True Grit lays out the hand; burning something else pays nothing.
        play(&mut simulator, "CARD.TRUE_GRIT");
        let before = energy(&simulator);
        choose(&mut simulator, "CARD.STRIKE_IRONCLAD");
        assert_eq!(energy(&simulator), before);

        let mut simulator = fight(&[
            ("CARD.DRUM_OF_BATTLE", upgrade),
            ("CARD.TRUE_GRIT", 1),
            ("CARD.STRIKE_IRONCLAD", 0),
        ]);
        play(&mut simulator, "CARD.TRUE_GRIT");
        let before = energy(&simulator);
        choose(&mut simulator, "CARD.DRUM_OF_BATTLE");
        assert_eq!(energy(&simulator), before + gained);
    }
}

// --- the cards whose cost is X -------------------------------------------

/// A whirlwind swallows every point of energy its owner has left and lands once
/// for each of them, over everything on the other side.
#[test]
fn a_whirlwind_lands_once_for_every_point_of_energy_it_swallowed() {
    for (upgrade, damage) in [(0, 5), (1, 8)] {
        let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .player_hp(80, 80)
            .energy(3, 3)
            .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
            .enemy(2, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
            .card(PileName::Hand, 0, card("CARD.WHIRLWIND", upgrade), 0)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
        assert_eq!(
            simulator
                .state()
                .combat
                .as_ref()
                .expect("the fight is standing")
                .player
                .piles
                .hand[0]
                .energy_cost,
            0,
            "an X-cost card's own cost is nothing at all"
        );
        play(&mut simulator, "CARD.WHIRLWIND");
        assert_eq!(energy(&simulator), 0, "it swallows every point");
        assert_eq!(creature(&simulator, 1).current_hp, 39 - 3 * damage);
        assert_eq!(creature(&simulator, 2).current_hp, 39 - 3 * damage);
    }
}

/// At no energy the play still happens and lands nothing: X is nothing, so
/// there are no hits to make.
#[test]
fn a_whirlwind_at_no_energy_is_a_play_that_lands_nothing() {
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(0, 3)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .card(PileName::Hand, 0, card("CARD.WHIRLWIND", 0), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.WHIRLWIND");
    assert_eq!(creature(&simulator, 1).current_hp, 39);
    assert_eq!(pile(&simulator, PileName::Discard), ["CARD.WHIRLWIND"]);
}

/// A whirlwind nobody chose still takes its X from the energy its owner has
/// left. A Stampede playing it out of the hand at the end of the turn spends
/// nothing, and the unspent energy is what it lands once per point of.
#[test]
fn an_auto_played_whirlwind_takes_its_x_from_the_energy_left_unspent() {
    for (left, hits) in [(2, 2), (0, 0)] {
        let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .player_hp(80, 80)
            .energy(left, 3)
            .turn(1, 1)
            .power(
                0,
                id("POWER.STAMPEDE_POWER"),
                1,
                sts2_engine::PowerType::Buff,
            )
            .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
            .card(PileName::Hand, 0, card("CARD.WHIRLWIND", 1), 0)
            .card(PileName::Hand, 1, card("CARD.DEFEND_IRONCLAD", 0), 0)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
        end_turn(&mut simulator);
        assert_eq!(
            creature(&simulator, 1).current_hp,
            39 - hits * 8,
            "the stampede's whirlwind lands once per point left unspent"
        );
    }
}

/// A cascade plays that many cards off the top of the draw pile, and one more
/// once it is upgraded. What it plays is not forced to burn.
#[test]
fn a_cascade_plays_one_card_off_the_top_for_every_point_it_swallowed() {
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(2, 3)
        .rng_counter(RunStream::CombatTargets, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .card(PileName::Hand, 0, card("CARD.CASCADE", 0), 0)
        .card(PileName::Draw, 1, card("CARD.STRIKE_IRONCLAD", 0), 0)
        .card(PileName::Draw, 2, card("CARD.STRIKE_IRONCLAD", 0), 0)
        .card(PileName::Draw, 3, card("CARD.STRIKE_IRONCLAD", 0), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.CASCADE");
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 12, "two strikes");
    assert_eq!(pile(&simulator, PileName::Draw).len(), 1);
    assert!(
        pile(&simulator, PileName::Exhaust).is_empty(),
        "a cascade does not force what it plays to burn"
    );

    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(2, 3)
        .rng_counter(RunStream::CombatTargets, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .card(PileName::Hand, 0, card("CARD.CASCADE", 1), 0)
        .card(PileName::Draw, 1, card("CARD.STRIKE_IRONCLAD", 0), 0)
        .card(PileName::Draw, 2, card("CARD.STRIKE_IRONCLAD", 0), 0)
        .card(PileName::Draw, 3, card("CARD.STRIKE_IRONCLAD", 0), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.CASCADE");
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 18, "three strikes");
}

// --- cards that lean on a listener, and cards that move piles ------------

/// A headbutt swings and then lays the discard pile out; the card picked out of
/// it goes on top of the draw pile, which is where the next draw comes from.
#[test]
fn headbutt_puts_a_discarded_card_back_on_top_of_the_draw_pile() {
    for (upgrade, damage) in [(0, 9), (1, 12)] {
        let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .player_hp(80, 80)
            .energy(9, 9)
            .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
            .card(PileName::Hand, 0, card("CARD.HEADBUTT", upgrade), 0)
            .card(PileName::Discard, 1, card("CARD.BASH", 0), 0)
            .card(PileName::Discard, 2, card("CARD.STRIKE_IRONCLAD", 0), 0)
            .card(PileName::Draw, 3, card("CARD.DEFEND_IRONCLAD", 0), 0)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
        play(&mut simulator, "CARD.HEADBUTT");
        assert_eq!(creature(&simulator, 1).current_hp, 39 - damage);
        choose(&mut simulator, "CARD.BASH");
        assert_eq!(
            pile(&simulator, PileName::Draw),
            ["CARD.BASH", "CARD.DEFEND_IRONCLAD"],
            "the pick goes on top, not on the bottom"
        );
        assert_eq!(
            pile(&simulator, PileName::Discard),
            ["CARD.STRIKE_IRONCLAD", "CARD.HEADBUTT"],
            "what was not picked stays, and the headbutt joins it"
        );
    }
}

/// A pillage keeps drawing for as long as it keeps turning up attacks.
#[test]
fn pillage_draws_while_it_keeps_turning_up_attacks() {
    let mut simulator = fight_with_draw(
        &[("CARD.PILLAGE", 0)],
        &[
            ("CARD.STRIKE_IRONCLAD", 0),
            ("CARD.STRIKE_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.STRIKE_IRONCLAD", 0),
        ],
    );
    play(&mut simulator, "CARD.PILLAGE");
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 6);
    assert_eq!(
        pile(&simulator, PileName::Hand),
        [
            "CARD.STRIKE_IRONCLAD",
            "CARD.STRIKE_IRONCLAD",
            "CARD.DEFEND_IRONCLAD"
        ],
        "the defend is drawn and stops the run"
    );
}

/// A second wind burns every card of the hand that is not an attack and works
/// up block for each of them, one at a time.
#[test]
fn second_wind_burns_the_skills_and_pays_block_for_each() {
    for (upgrade, per_card) in [(0, 5), (1, 7)] {
        let mut simulator = fight(&[
            ("CARD.SECOND_WIND", upgrade),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.SHRUG_IT_OFF", 0),
            ("CARD.STRIKE_IRONCLAD", 0),
        ]);
        play(&mut simulator, "CARD.SECOND_WIND");
        assert_eq!(player(&simulator).block, 2 * per_card);
        assert_eq!(pile(&simulator, PileName::Hand), ["CARD.STRIKE_IRONCLAD"]);
        assert_eq!(
            pile(&simulator, PileName::Exhaust),
            ["CARD.DEFEND_IRONCLAD", "CARD.SHRUG_IT_OFF"]
        );
    }
}

/// Expecting a fight pays a point for every attack in hand — the second wind's
/// own card is in the play pile by then, so it is not one of them — and then
/// stops the next gain outright.
#[test]
fn expect_a_fight_pays_per_attack_in_hand_and_then_stops_the_next_gain() {
    let mut simulator = fight(&[
        ("CARD.EXPECT_A_FIGHT", 0),
        ("CARD.STRIKE_IRONCLAD", 0),
        ("CARD.STRIKE_IRONCLAD", 0),
        ("CARD.DEFEND_IRONCLAD", 0),
        ("CARD.BLOODLETTING", 0),
    ]);
    let before = energy(&simulator);
    play(&mut simulator, "CARD.EXPECT_A_FIGHT");
    assert_eq!(energy(&simulator), before - 2 + 2);

    let before = energy(&simulator);
    play(&mut simulator, "CARD.BLOODLETTING");
    assert_eq!(
        energy(&simulator),
        before,
        "the bloodletting still costs its HP but its energy is worth nothing"
    );
    assert_eq!(player(&simulator).current_hp, 77);

    end_turn(&mut simulator);
    assert_eq!(
        power_amount(&simulator, 0, "POWER.NO_ENERGY_GAIN_POWER"),
        0,
        "the stopper comes off at the end of a turn its owner took part in"
    );
}

/// A battle trance draws and then stops the well; the next turn's own opening
/// hand still comes, because that draw is the hand draw.
#[test]
fn battle_trance_draws_and_stops_anything_else_drawing_this_turn() {
    for (upgrade, drawn) in [(0, 3), (1, 4)] {
        let mut simulator = fight_with_draw(
            &[("CARD.BATTLE_TRANCE", upgrade), ("CARD.SHRUG_IT_OFF", 0)],
            &[("CARD.DEFEND_IRONCLAD", 0); 8],
        );
        play(&mut simulator, "CARD.BATTLE_TRANCE");
        assert_eq!(pile(&simulator, PileName::Hand).len(), 1 + drawn);
        let held = pile(&simulator, PileName::Hand).len();
        play(&mut simulator, "CARD.SHRUG_IT_OFF");
        assert_eq!(player(&simulator).block, 8);
        assert_eq!(
            pile(&simulator, PileName::Hand).len(),
            held - 1,
            "the shrug's own draw is refused outright"
        );
    }
}

/// A juggernaut throws what it is worth at somebody every time its owner works
/// up block, and rolls who on the combat-targets stream.
#[test]
fn a_juggernaut_answers_every_block_its_owner_works_up() {
    for (upgrade, damage) in [(0, 6), (1, 8)] {
        let mut simulator = fight(&[
            ("CARD.JUGGERNAUT", upgrade),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
        ]);
        play(&mut simulator, "CARD.JUGGERNAUT");
        assert_eq!(creature(&simulator, 1).current_hp, 39);
        play(&mut simulator, "CARD.DEFEND_IRONCLAD");
        assert_eq!(creature(&simulator, 1).current_hp, 39 - damage);
        play(&mut simulator, "CARD.DEFEND_IRONCLAD");
        assert_eq!(creature(&simulator, 1).current_hp, 39 - 2 * damage);
    }
}

/// Viciousness draws for every shaking its owner works onto anybody.
#[test]
fn viciousness_draws_when_its_owner_shakes_somebody() {
    for (upgrade, drawn) in [(0, 1), (1, 2)] {
        let mut simulator = fight_with_draw(
            &[("CARD.VICIOUS", upgrade), ("CARD.TAUNT", 0)],
            &[("CARD.DEFEND_IRONCLAD", 0); 4],
        );
        play(&mut simulator, "CARD.VICIOUS");
        assert_eq!(pile(&simulator, PileName::Hand).len(), 1);
        play(&mut simulator, "CARD.TAUNT");
        assert_eq!(pile(&simulator, PileName::Hand).len(), drawn);
    }
}

/// A one-two punch makes the next attack play run twice, and one stack is spent
/// for each attack that does.
#[test]
fn a_one_two_punch_runs_the_next_attack_twice() {
    let mut simulator = fight(&[
        ("CARD.ONE_TWO_PUNCH", 0),
        ("CARD.STRIKE_IRONCLAD", 0),
        ("CARD.STRIKE_IRONCLAD", 0),
        ("CARD.DEFEND_IRONCLAD", 0),
    ]);
    play(&mut simulator, "CARD.ONE_TWO_PUNCH");
    play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    assert_eq!(
        power_amount(&simulator, 0, "POWER.ONE_TWO_PUNCH_POWER"),
        1,
        "a skill does not spend it"
    );
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 12);
    assert_eq!(power_amount(&simulator, 0, "POWER.ONE_TWO_PUNCH_POWER"), 0);
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 18, "and no more");
}

/// The upgraded punch is worth two attacks.
#[test]
fn an_upgraded_one_two_punch_carries_two_stacks() {
    let mut simulator = fight(&[("CARD.ONE_TWO_PUNCH", 1), ("CARD.STRIKE_IRONCLAD", 0)]);
    play(&mut simulator, "CARD.ONE_TWO_PUNCH");
    assert_eq!(power_amount(&simulator, 0, "POWER.ONE_TWO_PUNCH_POWER"), 2);
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(power_amount(&simulator, 0, "POWER.ONE_TWO_PUNCH_POWER"), 1);
}

/// Juggling hands back copies of the third attack its owner plays in a turn,
/// counting from the attacks already played when it landed.
#[test]
fn juggling_copies_the_third_attack_of_the_turn() {
    let mut simulator = fight(&[
        ("CARD.JUGGLING", 0),
        ("CARD.STRIKE_IRONCLAD", 0),
        ("CARD.STRIKE_IRONCLAD", 0),
        ("CARD.STRIKE_IRONCLAD", 0),
    ]);
    play(&mut simulator, "CARD.JUGGLING");
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(pile(&simulator, PileName::Hand).len(), 1);
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(
        pile(&simulator, PileName::Hand),
        ["CARD.STRIKE_IRONCLAD"],
        "the third one comes back"
    );
}

/// Upgrading a juggling makes it innate: it is put on top of the draw pile
/// before the opening hand, and the opening hand is dealt around it.
#[test]
fn an_upgraded_juggling_is_innate() {
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(0, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .card(PileName::Draw, 0, card("CARD.DEFEND_IRONCLAD", 0), 0)
        .card(PileName::Draw, 1, card("CARD.DEFEND_IRONCLAD", 0), 0)
        .card(PileName::Draw, 2, card("CARD.DEFEND_IRONCLAD", 0), 0)
        .card(PileName::Draw, 3, card("CARD.DEFEND_IRONCLAD", 0), 0)
        .card(PileName::Draw, 4, card("CARD.DEFEND_IRONCLAD", 0), 0)
        .card(PileName::Draw, 5, card("CARD.JUGGLING", 1), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    end_turn(&mut simulator);
    assert!(
        pile(&simulator, PileName::Hand).contains(&"CARD.JUGGLING".to_owned()),
        "an innate card is in the opening hand however deep it sat"
    );
}

/// A howl lands on everything, and a howl standing in the exhaust pile plays
/// itself again once its owner has stopped acting. One in the discard pile,
/// which is where a played howl goes, does nothing.
#[test]
fn a_howl_from_beyond_plays_itself_again_out_of_the_exhaust_pile() {
    let howling = |pile: PileName| {
        ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .player_hp(80, 80)
            .energy(9, 9)
            .rng_counter(RunStream::MonsterAi, 0)
            .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 200, 200, "SHRINKER_MOVE")
            .card(pile, 0, card("CARD.HOWL_FROM_BEYOND", 0), 0)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds")
    };

    let mut simulator = howling(PileName::Hand);
    play(&mut simulator, "CARD.HOWL_FROM_BEYOND");
    assert_eq!(creature(&simulator, 1).current_hp, 200 - 16);
    assert_eq!(
        pile(&simulator, PileName::Discard),
        ["CARD.HOWL_FROM_BEYOND"]
    );
    let after_play = creature(&simulator, 1).current_hp;
    end_turn(&mut simulator);
    assert_eq!(
        creature(&simulator, 1).current_hp,
        after_play,
        "a howl in the discard pile does nothing"
    );

    let mut simulator = howling(PileName::Exhaust);
    end_turn(&mut simulator);
    assert_eq!(
        creature(&simulator, 1).current_hp,
        200 - 16,
        "a howl in the exhaust pile plays itself once the player stops acting"
    );
}

/// A stampede plays attacks out of the hand once its owner has stopped acting,
/// as many as it is worth, each one rolled off the shuffle stream.
#[test]
fn a_stampede_plays_attacks_out_of_the_hand_at_the_end_of_the_turn() {
    for (upgrade, cost) in [(0, 2), (1, 1)] {
        let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .player_hp(80, 80)
            .energy(9, 9)
            .rng_counter(RunStream::MonsterAi, 0)
            .rng_counter(RunStream::Shuffle, 0)
            .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 200, 200, "SHRINKER_MOVE")
            .card(PileName::Hand, 0, card("CARD.STAMPEDE", upgrade), 0)
            .card(PileName::Hand, 1, card("CARD.STRIKE_IRONCLAD", 0), 0)
            .card(PileName::Hand, 2, card("CARD.DEFEND_IRONCLAD", 0), 0)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
        let before = energy(&simulator);
        play(&mut simulator, "CARD.STAMPEDE");
        assert_eq!(energy(&simulator), before - cost);
        end_turn(&mut simulator);
        assert_eq!(
            creature(&simulator, 1).current_hp,
            200 - 6,
            "the one attack in hand is played for free"
        );
    }
}

/// An automatic play asks whether the card may be played at all before it
/// rolls a target. Ringing therefore stops Stampede after the player has
/// already played their one card
/// for the turn, and the chosen attack is put down without being played.
#[test]
fn ringing_stops_stampedes_automatic_attack_after_the_turns_one_play() {
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::Shuffle, 0)
        .rng_counter(RunStream::CombatTargets, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 200, 200, "SHRINKER_MOVE")
        .power(
            0,
            id("POWER.STAMPEDE_POWER"),
            1,
            sts2_engine::PowerType::Buff,
        )
        .power(
            0,
            id("POWER.RINGING_POWER"),
            1,
            sts2_engine::PowerType::Debuff,
        )
        .card(PileName::Hand, 0, card("CARD.BATTLE_TRANCE", 0), 0)
        .card(PileName::Hand, 1, card("CARD.STRIKE_IRONCLAD", 0), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.BATTLE_TRANCE");
    end_turn(&mut simulator);
    assert_eq!(
        creature(&simulator, 1).current_hp,
        200,
        "Ringing blocks the attack Stampede selected"
    );
    assert_eq!(
        simulator
            .state()
            .rng
            .stream(RunStream::CombatTargets)
            .counter(),
        0,
        "a blocked auto-play never rolls a target"
    );
}

/// A hellraiser plays every Strike its owner draws the moment it is drawn.
#[test]
fn a_hellraiser_plays_every_strike_as_it_is_drawn() {
    let mut simulator = fight_with_draw(
        &[("CARD.HELLRAISER", 0), ("CARD.SHRUG_IT_OFF", 0)],
        &[("CARD.STRIKE_IRONCLAD", 0), ("CARD.DEFEND_IRONCLAD", 0)],
    );
    play(&mut simulator, "CARD.HELLRAISER");
    play(&mut simulator, "CARD.SHRUG_IT_OFF");
    assert_eq!(
        creature(&simulator, 1).current_hp,
        39 - 6,
        "the strug drew the strike and the strike went off"
    );
    assert!(
        pile(&simulator, PileName::Hand).is_empty(),
        "the strike is not held: it is played out of the hand as it lands"
    );
    assert_eq!(pile(&simulator, PileName::Discard).len(), 2);
}

/// A hellraiser plays nine Strikes a turn at the most once nothing its owner
/// can hit has HP that runs out. A colony reads that way for the rest of a turn
/// that has already cost it everything its hardened shell holds back, and from
/// there no blow can end the fight: the cap is what stops the play a drawn
/// Strike answers from feeding itself forever.
#[test]
fn a_hellraiser_caps_its_plays_where_nothing_it_hits_can_die() {
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SKULKING_COLONY_ELITE"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .enemy(1, id("MONSTER.SKULKING_COLONY"), 80, 80, "ZOOM_MOVE")
        .power(
            0,
            id("POWER.HELLRAISER_POWER"),
            1,
            sts2_engine::PowerType::Buff,
        )
        .power(
            1,
            id("POWER.HARDENED_SHELL_POWER"),
            20,
            sts2_engine::PowerType::Buff,
        )
        .card(PileName::Hand, 0, card("CARD.BLUDGEON", 0), 0)
        .card(PileName::Hand, 1, card("CARD.OFFERING", 1), 0)
        .card(PileName::Hand, 2, card("CARD.OFFERING", 1), 0);
    for index in 3..15u64 {
        scenario = scenario.card(PileName::Draw, index, card("CARD.STRIKE_IRONCLAD", 0), 0);
    }
    let mut simulator = scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    // The shell holds the turn to twenty, and the blow that reaches twenty is
    // what puts the colony out of reach for the rest of it.
    play(&mut simulator, "CARD.BLUDGEON");
    assert_eq!(creature(&simulator, 1).current_hp, 60);
    assert_eq!(
        creature(&simulator, 1).hp_display,
        sts2_engine::HpDisplay::InfiniteWithNumbers
    );
    // Ten Strikes are drawn against that field; the hellraiser plays nine of
    // them and leaves the tenth in the hand.
    play(&mut simulator, "CARD.OFFERING");
    play(&mut simulator, "CARD.OFFERING");
    assert_eq!(
        creature(&simulator, 1).current_hp,
        60,
        "no blow gets through"
    );
    assert_eq!(pile(&simulator, PileName::Hand), ["CARD.STRIKE_IRONCLAD"]);
    assert_eq!(
        pile(&simulator, PileName::Discard)
            .iter()
            .filter(|model_id| *model_id == "CARD.STRIKE_IRONCLAD")
            .count(),
        9,
        "nine of the ten drawn Strikes were played"
    );
    assert_eq!(power_counter(&simulator, 0, "POWER.HELLRAISER_POWER"), 10);
    // Both counts belong to the turn: the shell holds the next one to twenty
    // of its own, and the turn its owner took part in puts the hellraiser's
    // count back to nothing, so the draw that lands after the shell has filled
    // again is the first of the next nine.
    end_turn(&mut simulator);
    assert_eq!(creature(&simulator, 1).current_hp, 40);
    assert_eq!(power_counter(&simulator, 0, "POWER.HELLRAISER_POWER"), 1);
}

/// An enchantment is part of an automatic play just as it is part of a card
/// the player chose. Corrupted works its self-damage after the Strike body.
#[test]
fn a_hellraiser_runs_the_enchantment_on_an_auto_played_strike() {
    let mut corrupted = card("CARD.STRIKE_IRONCLAD", 0);
    corrupted.enchantment = Some(sts2_engine::EnchantmentFingerprint::new(
        id("ENCHANTMENT.CORRUPTED"),
        0,
    ));
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .card(PileName::Hand, 0, card("CARD.HELLRAISER", 0), 0)
        .card(PileName::Hand, 1, card("CARD.SHRUG_IT_OFF", 0), 0)
        .card(PileName::Draw, 2, corrupted, 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.HELLRAISER");
    play(&mut simulator, "CARD.SHRUG_IT_OFF");
    assert_eq!(player(&simulator).current_hp, 78);
}

/// Ending the fight cancels the tail of an automatic play, but the play still
/// finishes in combat history and reaches relics that count attacks.
#[test]
fn a_hellraiser_finishes_a_lethal_auto_played_strike() {
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .relics(&[id("RELIC.NUNCHAKU")])
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 9, 9, "SHRINKER_MOVE")
        .card(PileName::Hand, 0, card("CARD.HELLRAISER", 0), 0)
        .card(PileName::Hand, 1, card("CARD.SHRUG_IT_OFF", 0), 0)
        .card(PileName::Draw, 2, card("CARD.STRIKE_IRONCLAD", 1), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.HELLRAISER");
    play(&mut simulator, "CARD.SHRUG_IT_OFF");
    assert!(
        simulator.state().combat.as_ref().is_none_or(|combat| combat
            .creatures
            .iter()
            .filter(|creature| creature.side == sts2_engine::CombatSide::Enemy)
            .all(|creature| !creature.alive())),
        "the Strike ended the fight"
    );
    assert_eq!(
        simulator
            .state()
            .run_player
            .relics
            .iter()
            .find(|relic| relic.model_id == id("RELIC.NUNCHAKU"))
            .expect("Nunchaku is held")
            .counter,
        1
    );
}

/// Aggression takes attacks back out of the discard pile as its owner's turn
/// opens, shuffled, and upgrades each of them.
#[test]
fn aggression_takes_attacks_back_and_upgrades_them() {
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 200, 200, "SHRINKER_MOVE")
        .card(PileName::Hand, 0, card("CARD.AGGRESSION", 0), 0)
        .card(PileName::Discard, 1, card("CARD.STRIKE_IRONCLAD", 0), 0)
        .card(PileName::Discard, 2, card("CARD.DEFEND_IRONCLAD", 0), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.AGGRESSION");
    end_turn(&mut simulator);
    let hand = &simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles
        .hand;
    let taken: Vec<_> = hand
        .iter()
        .filter(|card| card.fingerprint.model_id == id("CARD.STRIKE_IRONCLAD"))
        .collect();
    assert_eq!(taken.len(), 1, "one attack came back, not the defend");
    assert_eq!(
        taken[0].fingerprint.upgrade_level, 1,
        "and it came back upgraded"
    );
    assert!(
        simulator
            .state()
            .combat
            .as_ref()
            .expect("the fight is standing")
            .player
            .piles
            .all_cards()
            .filter(|card| card.fingerprint.model_id == id("CARD.DEFEND_IRONCLAD"))
            .all(|card| card.fingerprint.upgrade_level == 0),
        "nothing but attacks is taken, and nothing else is upgraded"
    );
}

// --- the cards that read the fight, roll the pool, or grow ----------------

/// A feed grows its owner when the blow kills, and not otherwise.
#[test]
fn feed_grows_its_owner_only_when_the_blow_kills() {
    for (upgrade, damage, max_hp) in [(0, 10, 3), (1, 12, 4)] {
        let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .player_hp(60, 80)
            .energy(9, 9)
            .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
            .card(PileName::Hand, 0, card("CARD.FEED", upgrade), 0)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
        play(&mut simulator, "CARD.FEED");
        assert_eq!(creature(&simulator, 1).current_hp, 39 - damage);
        assert_eq!(player(&simulator).max_hp, 80, "nothing died, nothing grew");
        assert_eq!(pile(&simulator, PileName::Exhaust), ["CARD.FEED"]);

        let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .player_hp(60, 80)
            .energy(9, 9)
            .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 3, 39, "SHRINKER_MOVE")
            .card(PileName::Hand, 0, card("CARD.FEED", upgrade), 0)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
        play(&mut simulator, "CARD.FEED");
        assert_eq!(simulator.state().run_player.max_hp, 80 + max_hp);
    }
}

/// A feed pays nothing for a minion: the power that makes a creature a minion
/// says its death is not the kill the card asks for.
#[test]
fn feed_pays_nothing_for_a_minion() {
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.THE_KIN_BOSS"))
        .player_hp(60, 80)
        .energy(9, 9)
        .enemy(1, id("MONSTER.KIN_FOLLOWER"), 3, 59, "QUICK_SLASH_MOVE")
        .card(PileName::Hand, 0, card("CARD.FEED", 0), 0)
        .power(1, id("POWER.MINION_POWER"), 1, sts2_engine::PowerType::Buff)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.FEED");
    assert_eq!(
        simulator.state().run_player.max_hp,
        80,
        "a minion's death is not a kill"
    );
}

/// A tear asunder lands once, and once more for every blow its owner has taken
/// this fight that block did not stop whole.
#[test]
fn tear_asunder_lands_once_more_for_every_blow_that_got_through() {
    for (upgrade, damage) in [(0, 5), (1, 7)] {
        let mut simulator = stomping_beetle(&[("CARD.TEAR_ASUNDER", upgrade)], false);
        play(&mut simulator, "CARD.TEAR_ASUNDER");
        assert_eq!(
            creature(&simulator, 1).current_hp,
            39 - damage,
            "nothing has got through yet"
        );

        let mut simulator = stomping_beetle(
            &[
                ("CARD.TEAR_ASUNDER", upgrade),
                ("CARD.TEAR_ASUNDER", upgrade),
            ],
            false,
        );
        end_turn(&mut simulator);
        play(&mut simulator, "CARD.TEAR_ASUNDER");
        assert_eq!(
            creature(&simulator, 1).current_hp,
            39 - 2 * damage,
            "one stomp got through, so the blow lands twice"
        );
    }
}

/// A rampage grows its own damage every time it is played.
#[test]
fn a_rampage_grows_every_time_it_is_played() {
    for (upgrade, increase) in [(0, 5), (1, 9)] {
        let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .player_hp(80, 80)
            .energy(9, 9)
            .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 200, 200, "SHRINKER_MOVE")
            .card(PileName::Hand, 0, card("CARD.RAMPAGE", upgrade), 0)
            .card(PileName::Hand, 1, card("CARD.RAMPAGE", upgrade), 0)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
        play(&mut simulator, "CARD.RAMPAGE");
        assert_eq!(creature(&simulator, 1).current_hp, 200 - 9);
        // The second copy is its own card and has not grown.
        play(&mut simulator, "CARD.RAMPAGE");
        assert_eq!(creature(&simulator, 1).current_hp, 200 - 9 - 9);
        let grown = simulator
            .state()
            .combat
            .as_ref()
            .expect("the fight is standing")
            .player
            .piles
            .discard
            .iter()
            .map(|card| card.damage_bonus)
            .collect::<Vec<_>>();
        assert_eq!(grown, [increase, increase]);
    }
}

/// A stomp costs one less for every attack its owner has already played this
/// turn, counted as it joins the fight and again for each attack played while
/// it is held.
#[test]
fn a_stomp_costs_less_for_every_attack_already_played() {
    let mut simulator = fight(&[
        ("CARD.STOMP", 0),
        ("CARD.STRIKE_IRONCLAD", 0),
        ("CARD.STRIKE_IRONCLAD", 0),
    ]);
    let cost = |simulator: &Simulator| {
        simulator
            .state()
            .combat
            .as_ref()
            .expect("the fight is standing")
            .player
            .piles
            .hand
            .iter()
            .find(|card| card.fingerprint.model_id == id("CARD.STOMP"))
            .expect("the stomp is held")
            .energy_cost
    };
    assert_eq!(cost(&simulator), 3);
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(cost(&simulator), 2);
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(cost(&simulator), 1);
    end_turn(&mut simulator);
    assert_eq!(cost(&simulator), 3, "what a turn took off goes with it");
}

/// End-of-turn cleanup happens before an enemy move is allowed to suspend on a
/// choice. A Stomp in the draw pile must already have its full cost while the
/// Knowledge Demon's curse choice is standing.
#[test]
fn a_stomp_cost_is_restored_before_an_enemy_choice() {
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.KNOWLEDGE_DEMON_BOSS"))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(1, 1)
        .enemy(
            1,
            id("MONSTER.KNOWLEDGE_DEMON"),
            379,
            379,
            "CURSE_OF_KNOWLEDGE_MOVE",
        )
        .card(PileName::Draw, 3, card("CARD.STOMP", 0), 3);
    for card_id in 0..3 {
        scenario = scenario.card(PileName::Hand, card_id, card("CARD.STRIKE_IRONCLAD", 0), 0);
    }
    let mut simulator = scenario
        .build(sts2_content::standard_registry())
        .expect("the Knowledge Demon fight builds");

    for _ in 0..3 {
        play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    }
    end_turn(&mut simulator);
    assert!(matches!(
        simulator.decision(),
        DecisionContext::ChooseCards { .. }
    ));
    let stomp = simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles
        .draw
        .iter()
        .find(|card| card.fingerprint.model_id == id("CARD.STOMP"))
        .expect("Stomp remains in the draw pile");
    assert_eq!(stomp.energy_cost, 3);
}

/// A copy of a stomp made mid-fight is owed nothing for what the fight has
/// already seen: the listener that counts the history skips a copy.
#[test]
fn a_stomp_made_a_copy_of_reads_no_history() {
    // A music box makes an ethereal copy of the first attack of the turn, so
    // the copy joins the fight with one attack already played.
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .relics(&[id("RELIC.MUSIC_BOX")])
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .card(PileName::Hand, 0, card("CARD.STOMP", 0), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.STOMP");
    let copy = simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles
        .hand
        .iter()
        .find(|held| held.fingerprint.model_id == id("CARD.STOMP"))
        .expect("the music box left a copy in hand");
    assert!(copy.clone_of.is_some(), "the copy knows what it came from");
    // A copy carries the original's own cost modifiers over, so the copy
    // keeps the point the played stomp took off for itself. What it
    // does not get is a second reading of the turn's attacks: with the clone
    // guard gone this would be one.
    assert_eq!(copy.energy_cost, 2);
    assert!(
        !copy.from_deck(),
        "and belongs to the fight rather than to the deck, because \
         cloning clears the deck version"
    );
}

/// A rupture pays strength for HP its owner loses on their own turn, and what a
/// card costs them is paid once that card's play is over.
#[test]
fn a_rupture_pays_for_hp_lost_on_its_owners_own_turn() {
    for (upgrade, strength) in [(0, 1), (1, 2)] {
        let mut simulator = fight(&[
            ("CARD.RUPTURE", upgrade),
            ("CARD.BLOODLETTING", 0),
            ("CARD.HEMOKINESIS", 0),
        ]);
        play(&mut simulator, "CARD.RUPTURE");
        play(&mut simulator, "CARD.BLOODLETTING");
        assert_eq!(
            power_amount(&simulator, 0, "POWER.STRENGTH_POWER"),
            strength
        );

        // Hemokinesis pays two HP and then swings: the strength lands after the
        // play, so the swing is not stronger for it.
        let before = creature(&simulator, 1).current_hp;
        play(&mut simulator, "CARD.HEMOKINESIS");
        assert_eq!(
            before - creature(&simulator, 1).current_hp,
            15 + strength,
            "the swing carries the strength the bloodletting already paid"
        );
        assert_eq!(
            power_amount(&simulator, 0, "POWER.STRENGTH_POWER"),
            2 * strength
        );
    }
}

/// A Smoggy smogs every skill of its owner's once one has been played, smogs a
/// skill that joins the fight afterwards, and clears the lot at the end of the
/// turn. A smogged card cannot be played while it carries the affliction.
#[test]
fn a_smoggy_smogs_every_skill_once_one_has_been_played() {
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .power(
            0,
            id("POWER.SMOGGY_POWER"),
            1,
            sts2_engine::PowerType::Debuff,
        );
    for index in 0..3u64 {
        scenario = scenario.card(PileName::Hand, index, card("CARD.DEFEND_IRONCLAD", 0), 0);
    }
    for index in 3..9u64 {
        scenario = scenario.card(PileName::Draw, index, card("CARD.STRIKE_IRONCLAD", 0), 0);
    }
    let mut simulator = scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    assert!(play_action(&simulator, "CARD.DEFEND_IRONCLAD").is_some());
    play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    let smogged = |simulator: &Simulator| {
        simulator
            .state()
            .combat
            .as_ref()
            .expect("the fight is standing")
            .player
            .piles
            .all_cards()
            .filter(|held| held.affliction.as_ref() == Some(&id("AFFLICTION.SMOG")))
            .count()
    };
    assert_eq!(smogged(&simulator), 3, "the two held and the one played");
    assert!(
        play_action(&simulator, "CARD.DEFEND_IRONCLAD").is_none(),
        "a smogged skill cannot be played"
    );
    // A strike is not a skill and is untouched.
    end_turn(&mut simulator);
    assert_eq!(smogged(&simulator), 0, "the turn's end clears every smog");
}

/// The tally a rupture keeps is one entry per in-flight play, not one for the
/// lot: a card played inside another card's play runs up its own entry and pays
/// it out when it finishes, leaving the outer play's where it was.
///
/// The two shapes agree on every sequence the pinned build can reach, because
/// no card of it costs its owner HP and then plays another card; what this pins
/// is that a nested play settles on its own rather than on whatever is around
/// it.
#[test]
fn a_rupture_keeps_a_tally_for_each_play_that_is_running() {
    let mut simulator = fight(&[
        ("CARD.RUPTURE", 0),
        ("CARD.HEMOKINESIS", 0),
        ("CARD.BEAT_DOWN", 0),
    ]);
    play(&mut simulator, "CARD.RUPTURE");
    play(&mut simulator, "CARD.HEMOKINESIS");
    assert_eq!(power_amount(&simulator, 0, "POWER.STRENGTH_POWER"), 1);
    // A beat down plays the hemokinesis again out of the discard pile, inside
    // its own play: the HP that costs is owed to the inner play.
    play(&mut simulator, "CARD.BEAT_DOWN");
    assert_eq!(
        power_amount(&simulator, 0, "POWER.STRENGTH_POWER"),
        2,
        "the inner play paid its own point and the outer one paid nothing"
    );
}

/// An unmovable doubles the block its owner works up off a card, for as many
/// separate plays a turn as it is worth.
#[test]
fn an_unmovable_doubles_the_first_blocks_of_the_turn() {
    let mut simulator = fight(&[
        ("CARD.UNMOVABLE", 0),
        ("CARD.DEFEND_IRONCLAD", 0),
        ("CARD.DEFEND_IRONCLAD", 0),
    ]);
    play(&mut simulator, "CARD.UNMOVABLE");
    play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    assert_eq!(player(&simulator).block, 10, "the first is doubled");
    play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    assert_eq!(player(&simulator).block, 15, "the second is not");
}

/// An infernal blade rolls one attack of the character's own pool and hands it
/// over free for the turn.
#[test]
fn an_infernal_blade_hands_over_a_free_attack() {
    for (upgrade, cost) in [(0, 1), (1, 0)] {
        let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .player_hp(80, 80)
            .energy(9, 9)
            .rng_counter(RunStream::CombatCardGeneration, 0)
            .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 200, 200, "SHRINKER_MOVE")
            .card(PileName::Hand, 0, card("CARD.INFERNAL_BLADE", upgrade), 0)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
        let before = energy(&simulator);
        play(&mut simulator, "CARD.INFERNAL_BLADE");
        assert_eq!(energy(&simulator), before - cost);
        let hand = &simulator
            .state()
            .combat
            .as_ref()
            .expect("the fight is standing")
            .player
            .piles
            .hand;
        assert_eq!(hand.len(), 1);
        assert_eq!(hand[0].energy_cost, 0, "it is free for the turn");
        assert!(!hand[0].from_deck(), "it exists for this fight only");
        assert_eq!(pile(&simulator, PileName::Exhaust), ["CARD.INFERNAL_BLADE"]);
    }
}

/// A stoke burns the whole hand and rolls that many cards of the character's
/// own pool back into it, upgraded once the stoke is.
#[test]
fn a_stoke_trades_the_hand_for_rolled_cards() {
    for (upgrade, level) in [(0, 0), (1, 1)] {
        let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .player_hp(80, 80)
            .energy(9, 9)
            .rng_counter(RunStream::CombatCardGeneration, 0)
            .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 200, 200, "SHRINKER_MOVE")
            .card(PileName::Hand, 0, card("CARD.STOKE", upgrade), 0)
            .card(PileName::Hand, 1, card("CARD.STRIKE_IRONCLAD", 0), 0)
            .card(PileName::Hand, 2, card("CARD.DEFEND_IRONCLAD", 0), 0)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
        play(&mut simulator, "CARD.STOKE");
        let hand = &simulator
            .state()
            .combat
            .as_ref()
            .expect("the fight is standing")
            .player
            .piles
            .hand;
        assert_eq!(hand.len(), 2, "two burned, two came back");
        assert!(
            hand.iter()
                .all(|card| card.fingerprint.upgrade_level == level)
        );
        assert_eq!(
            pile(&simulator, PileName::Exhaust),
            ["CARD.STRIKE_IRONCLAD", "CARD.DEFEND_IRONCLAD"]
        );
    }
}

/// A thrash swings twice for what its damage var has grown to and then eats an
/// attack out of the hand, growing by what that card would have been worth.
///
/// What the eaten card is worth is read with nobody in the target's place, so
/// only the dealer's own modifiers are asked. The reader's own vulnerable is a
/// thing done to whoever takes a blow, so it has
/// no say in what a card in the hand is worth — a thrash eaten from under three
/// stacks of it grows by exactly what it grows by without them.
#[test]
fn thrash_reads_the_card_it_eats_with_nobody_taking_the_blow() {
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .power(
            0,
            id("POWER.VULNERABLE_POWER"),
            3,
            sts2_engine::PowerType::Debuff,
        );
    for (slot, model_id) in ["CARD.THRASH", "CARD.STRIKE_IRONCLAD"]
        .into_iter()
        .enumerate()
    {
        scenario = scenario.card(PileName::Hand, slot as u64, card(model_id, 0), 0);
    }
    let mut simulator = scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");

    play(&mut simulator, "CARD.THRASH");

    assert_eq!(
        pile(&simulator, PileName::Exhaust),
        vec!["CARD.STRIKE_IRONCLAD".to_owned()],
        "the only attack left in the hand is the one it ate"
    );
    let thrash = simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight stands")
        .player
        .piles
        .all_cards()
        .find(|card| card.fingerprint.model_id == id("CARD.THRASH"))
        .expect("the thrash is in a pile");
    assert_eq!(
        thrash.damage_bonus, 6,
        "a strike is worth its own six, and the vulnerable on the reader is not part of it"
    );
}

/// The reading is of the card that was picked, not of the card doing the
/// picking: the pass is handed the eaten card, so it opens with that card's
/// enchantment.
#[test]
fn thrash_grows_by_the_enchantment_on_the_card_it_ate() {
    let mut sharp = card("CARD.STRIKE_IRONCLAD", 0);
    sharp.enchantment = Some(sts2_engine::EnchantmentFingerprint::new(
        id("ENCHANTMENT.SHARP"),
        3,
    ));
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .card(PileName::Hand, 0, card("CARD.THRASH", 0), 0)
        .card(PileName::Hand, 1, sharp, 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");

    play(&mut simulator, "CARD.THRASH");

    let thrash = simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight stands")
        .player
        .piles
        .all_cards()
        .find(|card| card.fingerprint.model_id == id("CARD.THRASH"))
        .expect("the thrash is in a pile");
    assert_eq!(
        thrash.damage_bonus, 9,
        "a sharpened strike is worth its six and the three the enchantment adds"
    );
}

/// An unsettling lamp doubles the first debuff a card works and nothing else it
/// works from the same card. A Mangle is one debuff written twice: the Mangle
/// power itself, and the strength that power lends its owner as it lands.
/// Doubling both is the whole reason the lamp remembers what it has already
/// doubled — it would otherwise come out four times over.
///
/// A Mangle under the lamp is written at twice its amount, and the strength it
/// lends is that doubled amount — twenty and twenty, not twenty and forty.
#[test]
fn an_unsettling_lamp_doubles_a_mangle_once_and_not_the_strength_it_lends() {
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .relics(&[id("RELIC.UNSETTLING_LAMP")])
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .card(PileName::Hand, 0, card("CARD.MANGLE", 0), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.MANGLE");

    let powers = |model_id: &str| {
        creature(&simulator, 1)
            .powers
            .iter()
            .find(|power| power.model_id == id(model_id))
            .map(|power| power.amount)
    };
    assert_eq!(
        powers("POWER.MANGLE_POWER"),
        Some(20),
        "the lamp doubled the mangle the card wrote"
    );
    assert_eq!(
        powers("POWER.STRENGTH_POWER"),
        Some(-20),
        "and the strength that mangle lends is the doubled amount, not twice it"
    );
}
