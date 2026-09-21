//! The Silent's whole pool, both forms of each card, pinned one behaviour at a
//! time.
//!
//! One recording plays twenty-one of these eighty-eight; the rest have no
//! recorded evidence at all, so a scenario is what stands for each of them.
//! Where a family of cards is one shape with different numbers — a plain swing,
//! a plain block, a power laid down — the family is one table-driven test, and
//! each card of it is a row.

use sts2_engine::{Action, CardFingerprint, DecisionContext, PileName, ScenarioBuilder, Simulator};
use sts2_rng::RunStream;

const SEED: &str = "SILENTCARDS";

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str, upgrade_level: u8) -> CardFingerprint {
    let mut fingerprint = CardFingerprint::base(id(model_id));
    fingerprint.upgrade_level = upgrade_level;
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

/// One fight against a single beetle, played as the Silent, with the hand and
/// the draw pile the test names and enough energy that a cost never gets in the
/// way of what is being measured.
fn fight_with_draw(hand: &[(&str, u8)], draw: &[(&str, u8)]) -> Simulator {
    scenario(1, hand, draw)
}

fn fight(hand: &[(&str, u8)]) -> Simulator {
    scenario(1, hand, &[])
}

/// The same fight with two beetles, for the cards that sweep.
fn crowd(hand: &[(&str, u8)]) -> Simulator {
    scenario(2, hand, &[])
}

fn scenario(enemies: u64, hand: &[(&str, u8)], draw: &[(&str, u8)]) -> Simulator {
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .played_as(id("CHARACTER.SILENT"))
        .player_hp(70, 70)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatTargets, 0)
        .rng_counter(RunStream::CombatCardSelection, 0);
    for slot in 1..=enemies {
        scenario = scenario.enemy(slot, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE");
    }
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

fn is_playable(simulator: &Simulator, model_id: &str) -> bool {
    let wanted = id(model_id);
    simulator
        .legal_actions()
        .iter()
        .any(|action| matches!(action, Action::PlayCard { card, .. } if card.fingerprint.model_id == wanted))
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

fn choosing(simulator: &Simulator) -> bool {
    matches!(simulator.decision(), DecisionContext::ChooseCards { .. })
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

/// What a creature has left, and nothing at all once it has gone down and been
/// taken out of the fight.
fn hp_of(simulator: &Simulator, combat_id: u64) -> Option<i32> {
    simulator
        .state()
        .combat
        .as_ref()
        .and_then(|combat| {
            combat
                .creatures
                .iter()
                .find(|creature| creature.id.get() == combat_id)
        })
        .map(|creature| creature.current_hp)
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

// --- the plain shapes ----------------------------------------------------

/// The swings that do nothing but swing, at both levels.
#[test]
fn the_plain_attacks_deal_what_their_damage_var_says() {
    for (model_id, base, upgraded) in [
        ("CARD.BACKSTAB", 11, 15),
        ("CARD.PINPOINT", 15, 19),
        ("CARD.SLICE", 6, 9),
        ("CARD.STRIKE_SILENT", 6, 9),
    ] {
        for (upgrade, damage) in [(0, base), (1, upgraded)] {
            let mut simulator = fight(&[(model_id, upgrade)]);
            play(&mut simulator, model_id);
            assert_eq!(
                creature(&simulator, 1).current_hp,
                39 - damage,
                "{model_id} at level {upgrade}"
            );
        }
    }
}

/// The swings that leave something behind on what they hit.
#[test]
fn the_attacks_that_leave_a_power_behind_leave_the_right_one() {
    for (model_id, power, base, upgraded) in [
        (
            "CARD.ASSASSINATE",
            "POWER.VULNERABLE_POWER",
            (10, 1),
            (13, 2),
        ),
        ("CARD.NEUTRALIZE", "POWER.WEAK_POWER", (3, 1), (4, 2)),
        ("CARD.POISONED_STAB", "POWER.POISON_POWER", (6, 3), (8, 4)),
        ("CARD.STRANGLE", "POWER.STRANGLE_POWER", (8, 2), (10, 3)),
        ("CARD.SUCKER_PUNCH", "POWER.WEAK_POWER", (8, 1), (10, 2)),
        ("CARD.SUPPRESS", "POWER.WEAK_POWER", (11, 3), (17, 5)),
    ] {
        for (upgrade, (damage, stacks)) in [(0, base), (1, upgraded)] {
            let mut simulator = fight(&[(model_id, upgrade)]);
            play(&mut simulator, model_id);
            assert_eq!(
                creature(&simulator, 1).current_hp,
                39 - damage,
                "{model_id} at level {upgrade}"
            );
            assert_eq!(
                power_amount(&simulator, 1, power),
                stacks,
                "{model_id} at level {upgrade}"
            );
        }
    }
}

/// The skills whose whole body is block for their owner.
#[test]
fn the_plain_blocks_work_up_what_their_block_var_says() {
    for (model_id, base, upgraded) in [
        ("CARD.DEFEND_SILENT", 5, 8),
        ("CARD.DEFLECT", 4, 7),
        ("CARD.UNTOUCHABLE", 6, 9),
    ] {
        for (upgrade, block) in [(0, base), (1, upgraded)] {
            let mut simulator = fight(&[(model_id, upgrade)]);
            play(&mut simulator, model_id);
            assert_eq!(player(&simulator).block, block, "{model_id} at {upgrade}");
        }
    }
}

/// The powers whose whole body is laying one power down on their owner.
#[test]
fn the_plain_powers_lay_down_what_they_say() {
    for (model_id, power, base, upgraded) in [
        ("CARD.ACCELERANT", "POWER.ACCELERANT_POWER", 1, 2),
        ("CARD.ACCURACY", "POWER.ACCURACY_POWER", 4, 6),
        ("CARD.AFTERIMAGE", "POWER.AFTERIMAGE_POWER", 1, 1),
        ("CARD.ANTICIPATE", "POWER.ANTICIPATE_POWER", 2, 3),
        ("CARD.BURST", "POWER.BURST_POWER", 1, 2),
        ("CARD.CORROSIVE_WAVE", "POWER.CORROSIVE_WAVE_POWER", 2, 3),
        ("CARD.ENVENOM", "POWER.ENVENOM_POWER", 1, 2),
        ("CARD.FOOTWORK", "POWER.DEXTERITY_POWER", 2, 3),
        ("CARD.INFINITE_BLADES", "POWER.INFINITE_BLADES_POWER", 1, 1),
        ("CARD.MASTER_PLANNER", "POWER.MASTER_PLANNER_POWER", 1, 1),
        ("CARD.NOXIOUS_FUMES", "POWER.NOXIOUS_FUMES_POWER", 2, 3),
        ("CARD.OUTBREAK", "POWER.OUTBREAK_POWER", 11, 15),
        ("CARD.PHANTOM_BLADES", "POWER.PHANTOM_BLADES_POWER", 9, 12),
        ("CARD.SERPENT_FORM", "POWER.SERPENT_FORM_POWER", 4, 6),
        ("CARD.SHADOWMELD", "POWER.SHADOWMELD_POWER", 1, 1),
        ("CARD.SNEAKY", "POWER.SNEAKY_POWER", 1, 2),
        ("CARD.SPEEDSTER", "POWER.SPEEDSTER_POWER", 2, 2),
        (
            "CARD.TOOLS_OF_THE_TRADE",
            "POWER.TOOLS_OF_THE_TRADE_POWER",
            1,
            1,
        ),
        ("CARD.WELL_LAID_PLANS", "POWER.WELL_LAID_PLANS_POWER", 1, 2),
    ] {
        for (upgrade, stacks) in [(0, base), (1, upgraded)] {
            let mut simulator = fight(&[(model_id, upgrade)]);
            play(&mut simulator, model_id);
            assert_eq!(
                power_amount(&simulator, 0, power),
                stacks,
                "{model_id} at level {upgrade}"
            );
        }
    }
}

/// The cards that work a power onto whatever they are aimed at and nothing
/// else.
#[test]
fn the_plain_debuffs_land_where_they_are_aimed() {
    for (model_id, power, base, upgraded) in [
        ("CARD.DEADLY_POISON", "POWER.POISON_POWER", 5, 7),
        ("CARD.SNAKEBITE", "POWER.POISON_POWER", 7, 10),
        ("CARD.FLANKING", "POWER.FLANKING_POWER", 2, 2),
    ] {
        for (upgrade, stacks) in [(0, base), (1, upgraded)] {
            let mut simulator = fight(&[(model_id, upgrade)]);
            play(&mut simulator, model_id);
            assert_eq!(
                power_amount(&simulator, 1, power),
                stacks,
                "{model_id} at level {upgrade}"
            );
        }
    }
}

/// The cards that sweep the whole enemy side.
#[test]
fn the_sweeps_land_on_everything() {
    for (model_id, base, upgraded) in [("CARD.FLICK_FLACK", 6, 8), ("CARD.DAGGER_SPRAY", 8, 12)] {
        for (upgrade, damage) in [(0, base), (1, upgraded)] {
            let mut simulator = crowd(&[(model_id, upgrade)]);
            play(&mut simulator, model_id);
            assert_eq!(creature(&simulator, 1).current_hp, 39 - damage);
            assert_eq!(creature(&simulator, 2).current_hp, 39 - damage);
        }
    }
}

/// The cards that work a power onto everything on the other side.
#[test]
fn the_sweeping_powers_land_on_everything() {
    for (model_id, power, base, upgraded) in [
        ("CARD.HAZE", "POWER.POISON_POWER", 4, 6),
        ("CARD.PIERCING_WAIL", "POWER.PIERCING_WAIL_POWER", 6, 8),
        ("CARD.SCARE", "POWER.WEAK_POWER", 1, 1),
    ] {
        for (upgrade, stacks) in [(0, base), (1, upgraded)] {
            let mut simulator = crowd(&[(model_id, upgrade)]);
            play(&mut simulator, model_id);
            assert_eq!(
                power_amount(&simulator, 1, power),
                stacks,
                "{model_id} at level {upgrade}"
            );
            assert_eq!(power_amount(&simulator, 2, power), stacks);
        }
    }
}

/// Upgrading a scare takes the Exhaust keyword off it; upgrading a calculated
/// gamble puts Retain on.
#[test]
fn two_upgrades_move_a_keyword_rather_than_a_number() {
    let mut simulator = crowd(&[("CARD.SCARE", 0)]);
    play(&mut simulator, "CARD.SCARE");
    assert_eq!(pile(&simulator, PileName::Exhaust), ["CARD.SCARE"]);

    let mut simulator = crowd(&[("CARD.SCARE", 1)]);
    play(&mut simulator, "CARD.SCARE");
    assert_eq!(pile(&simulator, PileName::Exhaust), Vec::<String>::new());
    assert_eq!(pile(&simulator, PileName::Discard), ["CARD.SCARE"]);

    let mut simulator = fight_with_draw(
        &[("CARD.CALCULATED_GAMBLE", 1), ("CARD.SLICE", 0)],
        &[("CARD.DEFLECT", 0)],
    );
    end_turn(&mut simulator);
    assert!(
        pile(&simulator, PileName::Hand).contains(&"CARD.CALCULATED_GAMBLE".to_owned()),
        "an upgraded gamble keeps itself through the flush"
    );
}

// --- draws, discards, and the hand ---------------------------------------

/// Acrobatics draws and then asks for one card back.
#[test]
fn acrobatics_draws_and_takes_one_back() {
    for (upgrade, drawn) in [(0, 3), (1, 4)] {
        let mut simulator = fight_with_draw(
            &[("CARD.ACROBATICS", upgrade)],
            &[
                ("CARD.SLICE", 0),
                ("CARD.SLICE", 0),
                ("CARD.SLICE", 0),
                ("CARD.SLICE", 0),
            ],
        );
        play(&mut simulator, "CARD.ACROBATICS");
        assert!(choosing(&simulator), "the discard screen is up");
        choose(&mut simulator, "CARD.SLICE");
        assert_eq!(pile(&simulator, PileName::Hand).len(), drawn - 1);
        assert!(pile(&simulator, PileName::Discard).contains(&"CARD.SLICE".to_owned()));
    }
}

/// Adrenaline pays energy and draws; the upgrade pays one more energy and the
/// same cards.
#[test]
fn adrenaline_pays_energy_and_draws() {
    for (upgrade, gained) in [(0, 1), (1, 2)] {
        let mut simulator = fight_with_draw(
            &[("CARD.ADRENALINE", upgrade)],
            &[("CARD.SLICE", 0), ("CARD.SLICE", 0)],
        );
        let before = energy(&simulator);
        play(&mut simulator, "CARD.ADRENALINE");
        assert_eq!(energy(&simulator), before + gained);
        assert_eq!(pile(&simulator, PileName::Hand).len(), 2);
        assert_eq!(pile(&simulator, PileName::Exhaust), ["CARD.ADRENALINE"]);
    }
}

/// A backflip works up block and draws two.
#[test]
fn backflip_blocks_and_draws() {
    for (upgrade, block) in [(0, 5), (1, 8)] {
        let mut simulator = fight_with_draw(
            &[("CARD.BACKFLIP", upgrade)],
            &[("CARD.SLICE", 0), ("CARD.SLICE", 0)],
        );
        play(&mut simulator, "CARD.BACKFLIP");
        assert_eq!(player(&simulator).block, block);
        assert_eq!(pile(&simulator, PileName::Hand).len(), 2);
    }
}

/// A calculated gamble puts the whole hand down and draws that many back. The
/// count is taken before anything moves, and the gamble is not part of it.
#[test]
fn calculated_gamble_redraws_the_hand_it_found() {
    let mut simulator = fight_with_draw(
        &[
            ("CARD.CALCULATED_GAMBLE", 0),
            ("CARD.SLICE", 0),
            ("CARD.DEFLECT", 0),
        ],
        &[("CARD.STRIKE_SILENT", 0), ("CARD.STRIKE_SILENT", 0)],
    );
    play(&mut simulator, "CARD.CALCULATED_GAMBLE");
    assert_eq!(
        pile(&simulator, PileName::Hand),
        ["CARD.STRIKE_SILENT", "CARD.STRIKE_SILENT"]
    );
    assert!(pile(&simulator, PileName::Discard).contains(&"CARD.SLICE".to_owned()));
    assert_eq!(
        pile(&simulator, PileName::Exhaust),
        ["CARD.CALCULATED_GAMBLE"]
    );
}

/// A dagger throw swings, draws one, and asks for one back.
#[test]
fn dagger_throw_swings_draws_and_takes_one_back() {
    for (upgrade, damage) in [(0, 9), (1, 12)] {
        let mut simulator =
            fight_with_draw(&[("CARD.DAGGER_THROW", upgrade)], &[("CARD.SLICE", 0)]);
        play(&mut simulator, "CARD.DAGGER_THROW");
        assert_eq!(creature(&simulator, 1).current_hp, 39 - damage);
        assert_eq!(pile(&simulator, PileName::Hand), Vec::<String>::new());
        assert!(pile(&simulator, PileName::Discard).contains(&"CARD.SLICE".to_owned()));
    }
}

/// An escape plan pays block only where the card it drew turned out to be a
/// skill.
#[test]
fn escape_plan_pays_block_only_for_a_skill() {
    for (upgrade, block) in [(0, 3), (1, 5)] {
        let mut simulator =
            fight_with_draw(&[("CARD.ESCAPE_PLAN", upgrade)], &[("CARD.DEFLECT", 0)]);
        play(&mut simulator, "CARD.ESCAPE_PLAN");
        assert_eq!(player(&simulator).block, block);

        let mut simulator = fight_with_draw(&[("CARD.ESCAPE_PLAN", upgrade)], &[("CARD.SLICE", 0)]);
        play(&mut simulator, "CARD.ESCAPE_PLAN");
        assert_eq!(player(&simulator).block, 0, "an attack pays nothing");
    }
}

/// Expertise draws the hand back up to its number, and nothing at all when the
/// hand already holds that many.
#[test]
fn expertise_fills_the_hand_and_no_further() {
    let mut simulator = fight_with_draw(
        &[("CARD.EXPERTISE", 0), ("CARD.SLICE", 0)],
        &[
            ("CARD.SLICE", 0),
            ("CARD.SLICE", 0),
            ("CARD.SLICE", 0),
            ("CARD.SLICE", 0),
            ("CARD.SLICE", 0),
            ("CARD.SLICE", 0),
            ("CARD.SLICE", 0),
        ],
    );
    play(&mut simulator, "CARD.EXPERTISE");
    // The hand held one card once the expertise left it, so five come up.
    assert_eq!(pile(&simulator, PileName::Hand).len(), 6);

    let hand: Vec<(&str, u8)> = std::iter::once(("CARD.EXPERTISE", 0))
        .chain(std::iter::repeat_n(("CARD.SLICE", 0), 7))
        .collect();
    let mut simulator = fight_with_draw(&hand, &[("CARD.DEFLECT", 0)]);
    play(&mut simulator, "CARD.EXPERTISE");
    assert_eq!(
        pile(&simulator, PileName::Hand).len(),
        7,
        "nothing is drawn"
    );
}

/// Prepared draws and puts back as many as it drew.
#[test]
fn prepared_draws_and_discards_the_same_number() {
    for (upgrade, count) in [(0, 1), (1, 2)] {
        let draw: Vec<(&str, u8)> = std::iter::repeat_n(("CARD.SLICE", 0), count).collect();
        let mut simulator = fight_with_draw(&[("CARD.PREPARED", upgrade)], &draw);
        play(&mut simulator, "CARD.PREPARED");
        assert_eq!(pile(&simulator, PileName::Hand), Vec::<String>::new());
        assert_eq!(pile(&simulator, PileName::Discard).len(), count + 1);
    }
}

/// A reflex is a plain draw, and is Sly: discarding it plays it.
#[test]
fn reflex_draws_and_plays_itself_out_of_a_discard() {
    for (upgrade, drawn) in [(0, 2), (1, 3)] {
        let draw: Vec<(&str, u8)> = std::iter::repeat_n(("CARD.SLICE", 0), drawn).collect();
        let mut simulator = fight_with_draw(&[("CARD.REFLEX", upgrade)], &draw);
        play(&mut simulator, "CARD.REFLEX");
        assert_eq!(pile(&simulator, PileName::Hand).len(), drawn);
    }

    // A shadow step puts the whole hand down, and a Sly card of that hand plays
    // itself once the batch is finished.
    let mut simulator = fight_with_draw(
        &[("CARD.SHADOW_STEP", 0), ("CARD.REFLEX", 0)],
        &[("CARD.SLICE", 0), ("CARD.SLICE", 0)],
    );
    play(&mut simulator, "CARD.SHADOW_STEP");
    assert_eq!(
        pile(&simulator, PileName::Hand).len(),
        2,
        "the reflex was discarded, played itself, and drew two"
    );
    assert!(pile(&simulator, PileName::Discard).contains(&"CARD.REFLEX".to_owned()));
}

/// A survivor works up block and asks for a card back.
#[test]
fn survivor_blocks_and_takes_a_card_back() {
    for (upgrade, block) in [(0, 8), (1, 11)] {
        let mut simulator = fight(&[("CARD.SURVIVOR", upgrade), ("CARD.SLICE", 0)]);
        play(&mut simulator, "CARD.SURVIVOR");
        assert_eq!(player(&simulator).block, block);
        assert_eq!(pile(&simulator, PileName::Discard).len(), 2);
    }
}

/// A tactician is energy, and nothing else.
#[test]
fn tactician_pays_energy() {
    for (upgrade, gained) in [(0, 1), (1, 2)] {
        let mut simulator = fight(&[("CARD.TACTICIAN", upgrade)]);
        let before = energy(&simulator);
        play(&mut simulator, "CARD.TACTICIAN");
        assert_eq!(energy(&simulator), before - 3 + gained);
    }
}

/// A shadow step puts the whole hand down and doubles what the next turn swings
/// off a card.
#[test]
fn shadow_step_empties_the_hand_and_pays_for_next_turn() {
    let mut simulator = fight(&[("CARD.SHADOW_STEP", 0), ("CARD.SLICE", 0)]);
    play(&mut simulator, "CARD.SHADOW_STEP");
    assert_eq!(pile(&simulator, PileName::Hand), Vec::<String>::new());
    assert_eq!(power_amount(&simulator, 0, "POWER.SHADOW_STEP_POWER"), 1);
}

// --- the Shivs -----------------------------------------------------------

/// The cards that hand their owner Shivs, and how many.
#[test]
fn the_shiv_makers_deal_the_right_number() {
    for (model_id, base, upgraded) in [
        ("CARD.BLADE_DANCE", 3, 4),
        ("CARD.BLADE_OF_INK", 2, 3),
        ("CARD.CLOAK_AND_DAGGER", 1, 2),
        ("CARD.UP_MY_SLEEVE", 3, 4),
    ] {
        for (upgrade, shivs) in [(0, base), (1, upgraded)] {
            let mut simulator = fight(&[(model_id, upgrade)]);
            play(&mut simulator, model_id);
            let hand = pile(&simulator, PileName::Hand);
            assert_eq!(
                hand.iter().filter(|card| *card == "CARD.SHIV").count(),
                shivs,
                "{model_id} at level {upgrade}"
            );
        }
    }
}

/// A blade of ink inks every Shiv it makes.
#[test]
fn blade_of_ink_inks_what_it_makes() {
    let mut simulator = fight(&[("CARD.BLADE_OF_INK", 0)]);
    play(&mut simulator, "CARD.BLADE_OF_INK");
    let combat = simulator.state().combat.as_ref().expect("a fight");
    for shiv in &combat.player.piles.hand {
        let enchantment = shiv
            .fingerprint
            .enchantment
            .as_ref()
            .expect("every Shiv it made is inked");
        assert_eq!(enchantment.model_id.to_string(), "ENCHANTMENT.INKY");
        assert_eq!(enchantment.amount, 1);
    }
}

/// A leading strike swings and then deals two Shivs.
#[test]
fn leading_strike_swings_and_deals_two_shivs() {
    for (upgrade, damage) in [(0, 3), (1, 6)] {
        let mut simulator = fight(&[("CARD.LEADING_STRIKE", upgrade)]);
        play(&mut simulator, "CARD.LEADING_STRIKE");
        assert_eq!(creature(&simulator, 1).current_hp, 39 - damage);
        assert_eq!(pile(&simulator, PileName::Hand), ["CARD.SHIV", "CARD.SHIV"]);
    }
}

/// Up my sleeve costs one less for the rest of the fight every time it is
/// played.
#[test]
fn up_my_sleeve_gets_cheaper_every_time() {
    let mut simulator = fight(&[("CARD.UP_MY_SLEEVE", 0)]);
    let before = energy(&simulator);
    play(&mut simulator, "CARD.UP_MY_SLEEVE");
    assert_eq!(energy(&simulator), before - 2);
    // The discount is written on the copy that was played, so it is
    // that copy — now in the discard pile — that is a point cheaper.
    let combat = simulator.state().combat.as_ref().expect("a fight");
    let played = combat
        .player
        .piles
        .discard
        .iter()
        .find(|card| card.fingerprint.model_id.to_string() == "CARD.UP_MY_SLEEVE")
        .expect("the sleeve was put down");
    assert_eq!(played.cost_this_combat, -1);
    assert_eq!(played.energy_cost, 1);
}

/// Hidden daggers pay two cards of the hand for two Shivs, and the upgraded
/// card upgrades them.
#[test]
fn hidden_daggers_pay_the_hand_for_upgraded_shivs() {
    for upgrade in [0, 1] {
        let mut simulator = fight(&[
            ("CARD.HIDDEN_DAGGERS", upgrade),
            ("CARD.SLICE", 0),
            ("CARD.SLICE", 0),
        ]);
        play(&mut simulator, "CARD.HIDDEN_DAGGERS");
        assert_eq!(pile(&simulator, PileName::Hand), ["CARD.SHIV", "CARD.SHIV"]);
        let combat = simulator.state().combat.as_ref().expect("a fight");
        for shiv in &combat.player.piles.hand {
            assert_eq!(shiv.fingerprint.upgrade_level, upgrade);
        }
    }
}

/// A storm of steel counts the hand it found, puts all of it down, and hands
/// back that many Shivs.
#[test]
fn storm_of_steel_trades_the_whole_hand_for_shivs() {
    for upgrade in [0, 1] {
        let mut simulator = fight(&[
            ("CARD.STORM_OF_STEEL", upgrade),
            ("CARD.SLICE", 0),
            ("CARD.DEFLECT", 0),
        ]);
        play(&mut simulator, "CARD.STORM_OF_STEEL");
        assert_eq!(pile(&simulator, PileName::Hand), ["CARD.SHIV", "CARD.SHIV"]);
        let combat = simulator.state().combat.as_ref().expect("a fight");
        for shiv in &combat.player.piles.hand {
            assert_eq!(shiv.fingerprint.upgrade_level, upgrade);
        }
    }
}

/// A fan of knives deals its Shivs and turns every Shiv into a sweep.
#[test]
fn fan_of_knives_points_every_shiv_at_everything() {
    for (upgrade, shivs) in [(0, 4), (1, 5)] {
        let mut simulator = crowd(&[("CARD.FAN_OF_KNIVES", upgrade)]);
        play(&mut simulator, "CARD.FAN_OF_KNIVES");
        assert_eq!(power_amount(&simulator, 0, "POWER.FAN_OF_KNIVES_POWER"), 1);
        assert_eq!(pile(&simulator, PileName::Hand).len(), shivs);
        play(&mut simulator, "CARD.SHIV");
        assert_eq!(creature(&simulator, 1).current_hp, 39 - 4);
        assert_eq!(creature(&simulator, 2).current_hp, 39 - 4);
    }
}

/// A knife trap plays every Shiv that has burned up, at whatever the trap was
/// pointed at, and the upgraded trap upgrades each of them first.
#[test]
fn knife_trap_throws_back_every_burned_shiv() {
    for (upgrade, damage) in [(0, 4), (1, 6)] {
        let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .played_as(id("CHARACTER.SILENT"))
            .player_hp(70, 70)
            .energy(9, 9)
            .turn(1, 1)
            .rng_counter(RunStream::MonsterAi, 0)
            .rng_counter(RunStream::CombatTargets, 0)
            .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
            .card(PileName::Hand, 0, card("CARD.KNIFE_TRAP", upgrade), 0)
            .card(PileName::Exhaust, 1, card("CARD.SHIV", 0), 0)
            .card(PileName::Exhaust, 2, card("CARD.SHIV", 0), 0)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
        play(&mut simulator, "CARD.KNIFE_TRAP");
        assert_eq!(creature(&simulator, 1).current_hp, 39 - 2 * damage);
    }
}

// --- the cards that read the state they land in --------------------------

/// A finisher swings once for every attack its owner has already finished
/// playing this turn, and not at all before the first one.
#[test]
fn finisher_swings_once_per_attack_already_played() {
    let mut simulator = fight(&[("CARD.FINISHER", 0)]);
    play(&mut simulator, "CARD.FINISHER");
    assert_eq!(
        creature(&simulator, 1).current_hp,
        39,
        "nothing to count yet"
    );

    let mut simulator = fight(&[("CARD.SLICE", 0), ("CARD.SLICE", 0), ("CARD.FINISHER", 0)]);
    play(&mut simulator, "CARD.SLICE");
    play(&mut simulator, "CARD.SLICE");
    play(&mut simulator, "CARD.FINISHER");
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 12 - 2 * 6);
}

/// Flechettes swing once for every skill in the hand as they land.
#[test]
fn flechettes_swing_once_per_skill_in_hand() {
    for (upgrade, damage) in [(0, 5), (1, 7)] {
        let mut simulator = fight(&[
            ("CARD.FLECHETTES", upgrade),
            ("CARD.DEFLECT", 0),
            ("CARD.DEFLECT", 0),
            ("CARD.SLICE", 0),
        ]);
        play(&mut simulator, "CARD.FLECHETTES");
        assert_eq!(creature(&simulator, 1).current_hp, 39 - 2 * damage);
    }
}

/// A memento mori is worth more for every card put down this turn.
#[test]
fn memento_mori_counts_what_has_been_discarded() {
    for (upgrade, base, per) in [(0, 9, 4), (1, 11, 5)] {
        let mut simulator = fight(&[("CARD.MEMENTO_MORI", upgrade)]);
        play(&mut simulator, "CARD.MEMENTO_MORI");
        assert_eq!(creature(&simulator, 1).current_hp, 39 - base);

        let mut simulator = fight(&[
            ("CARD.SURVIVOR", 0),
            ("CARD.SLICE", 0),
            ("CARD.MEMENTO_MORI", upgrade),
        ]);
        play(&mut simulator, "CARD.SURVIVOR");
        choose(&mut simulator, "CARD.SLICE");
        play(&mut simulator, "CARD.MEMENTO_MORI");
        assert_eq!(creature(&simulator, 1).current_hp, 39 - (base + per));
    }
}

/// A murder is worth one more for every card drawn this fight.
#[test]
fn murder_counts_every_card_drawn() {
    let mut simulator = fight_with_draw(
        &[("CARD.MURDER", 0), ("CARD.PREPARED", 0)],
        &[("CARD.SLICE", 0)],
    );
    play(&mut simulator, "CARD.PREPARED");
    choose(&mut simulator, "CARD.SLICE");
    play(&mut simulator, "CARD.MURDER");
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 2);
}

/// A precise cut is worth two less for every other card in the hand.
#[test]
fn precise_cut_is_worth_less_for_a_full_hand() {
    for (upgrade, base) in [(0, 13), (1, 16)] {
        let mut simulator = fight(&[("CARD.PRECISE_CUT", upgrade)]);
        play(&mut simulator, "CARD.PRECISE_CUT");
        assert_eq!(creature(&simulator, 1).current_hp, 39 - base);

        let mut simulator = fight(&[
            ("CARD.PRECISE_CUT", upgrade),
            ("CARD.SLICE", 0),
            ("CARD.SLICE", 0),
        ]);
        play(&mut simulator, "CARD.PRECISE_CUT");
        assert_eq!(creature(&simulator, 1).current_hp, 39 - (base - 4));
    }
}

/// A mirage is worth every point of poison the other side is carrying.
#[test]
fn mirage_is_worth_the_poison_on_the_field() {
    let mut simulator = crowd(&[
        ("CARD.DEADLY_POISON", 0),
        ("CARD.DEADLY_POISON", 0),
        ("CARD.MIRAGE", 0),
    ]);
    play(&mut simulator, "CARD.DEADLY_POISON");
    play(&mut simulator, "CARD.DEADLY_POISON");
    play(&mut simulator, "CARD.MIRAGE");
    assert_eq!(player(&simulator).block, 10);
    assert_eq!(pile(&simulator, PileName::Exhaust), ["CARD.MIRAGE"]);
}

/// A grand finale can only be played on an empty draw pile.
#[test]
fn grand_finale_waits_for_an_empty_draw_pile() {
    let simulator = fight_with_draw(&[("CARD.GRAND_FINALE", 0)], &[("CARD.SLICE", 0)]);
    assert!(!is_playable(&simulator, "CARD.GRAND_FINALE"));

    let mut simulator = crowd(&[("CARD.GRAND_FINALE", 0)]);
    play(&mut simulator, "CARD.GRAND_FINALE");
    assert_eq!(hp_of(&simulator, 1), None, "sixty is more than enough");
    assert_eq!(hp_of(&simulator, 2), None);
}

/// A bubble does nothing at all to a target carrying no poison, and stacks on
/// one that is.
#[test]
fn bubble_bubble_only_adds_to_poison_already_there() {
    for (upgrade, amount) in [(0, 9), (1, 12)] {
        let mut simulator = fight(&[("CARD.BUBBLE_BUBBLE", upgrade)]);
        play(&mut simulator, "CARD.BUBBLE_BUBBLE");
        assert_eq!(power_amount(&simulator, 1, "POWER.POISON_POWER"), 0);

        let mut simulator = fight(&[("CARD.DEADLY_POISON", 0), ("CARD.BUBBLE_BUBBLE", upgrade)]);
        play(&mut simulator, "CARD.DEADLY_POISON");
        play(&mut simulator, "CARD.BUBBLE_BUBBLE");
        assert_eq!(
            power_amount(&simulator, 1, "POWER.POISON_POWER"),
            5 + amount
        );
    }
}

/// Tracking stacks two the first time and one every time after it.
#[test]
fn tracking_stacks_two_and_then_one() {
    let mut simulator = fight(&[("CARD.TRACKING", 0), ("CARD.TRACKING", 0)]);
    play(&mut simulator, "CARD.TRACKING");
    assert_eq!(power_amount(&simulator, 0, "POWER.TRACKING_POWER"), 2);
    play(&mut simulator, "CARD.TRACKING");
    assert_eq!(power_amount(&simulator, 0, "POWER.TRACKING_POWER"), 3);
}

/// A dodge and roll owes next turn exactly what its block came to.
#[test]
fn dodge_and_roll_owes_what_the_block_came_to() {
    for (upgrade, block) in [(0, 4), (1, 6)] {
        let mut simulator = fight(&[("CARD.DODGE_AND_ROLL", upgrade)]);
        play(&mut simulator, "CARD.DODGE_AND_ROLL");
        assert_eq!(player(&simulator).block, block);
        assert_eq!(
            power_amount(&simulator, 0, "POWER.BLOCK_NEXT_TURN_POWER"),
            block
        );
    }
}

/// A dash works its block up before it swings.
#[test]
fn dash_blocks_and_then_swings() {
    for (upgrade, amount) in [(0, 10), (1, 13)] {
        let mut simulator = fight(&[("CARD.DASH", upgrade)]);
        play(&mut simulator, "CARD.DASH");
        assert_eq!(player(&simulator).block, amount);
        assert_eq!(creature(&simulator, 1).current_hp, 39 - amount);
    }
}

/// A leg sweep works up block and shakes what it is aimed at.
#[test]
fn leg_sweep_blocks_and_weakens() {
    for (upgrade, block, weak) in [(0, 11, 2), (1, 14, 3)] {
        let mut simulator = fight(&[("CARD.LEG_SWEEP", upgrade)]);
        play(&mut simulator, "CARD.LEG_SWEEP");
        assert_eq!(player(&simulator).block, block);
        assert_eq!(power_amount(&simulator, 1, "POWER.WEAK_POWER"), weak);
    }
}

/// An abrasive works dexterity on first and thorns after it, and only the
/// thorns grow.
#[test]
fn abrasive_pays_dexterity_and_thorns() {
    for (upgrade, thorns) in [(0, 4), (1, 6)] {
        let mut simulator = fight(&[("CARD.ABRASIVE", upgrade)]);
        play(&mut simulator, "CARD.ABRASIVE");
        assert_eq!(power_amount(&simulator, 0, "POWER.DEXTERITY_POWER"), 1);
        assert_eq!(power_amount(&simulator, 0, "POWER.THORNS_POWER"), thorns);
    }
}

/// A blur works up block and keeps it standing through the next turn's clear.
#[test]
fn blur_keeps_the_block_it_worked_up() {
    for (upgrade, block) in [(0, 5), (1, 8)] {
        let mut simulator = fight(&[("CARD.BLUR", upgrade)]);
        play(&mut simulator, "CARD.BLUR");
        assert_eq!(player(&simulator).block, block);
        assert_eq!(power_amount(&simulator, 0, "POWER.BLUR_POWER"), 1);
    }
}

/// A pounce swings and leaves its owner owed a free skill.
#[test]
fn pounce_swings_and_pays_a_free_skill() {
    for (upgrade, damage) in [(0, 14), (1, 20)] {
        let mut simulator = fight(&[("CARD.POUNCE", upgrade), ("CARD.DEFLECT", 0)]);
        let before = energy(&simulator);
        play(&mut simulator, "CARD.POUNCE");
        assert_eq!(creature(&simulator, 1).current_hp, 39 - damage);
        assert_eq!(power_amount(&simulator, 0, "POWER.FREE_SKILL_POWER"), 1);
        let after_pounce = energy(&simulator);
        play(&mut simulator, "CARD.DEFLECT");
        assert_eq!(energy(&simulator), after_pounce, "the skill cost nothing");
        assert_eq!(before - after_pounce, 2);
    }
}

/// A predator swings and owes its owner two cards next turn.
#[test]
fn predator_owes_two_cards_next_turn() {
    for (upgrade, damage) in [(0, 15), (1, 20)] {
        let mut simulator = fight(&[("CARD.PREDATOR", upgrade)]);
        play(&mut simulator, "CARD.PREDATOR");
        assert_eq!(creature(&simulator, 1).current_hp, 39 - damage);
        assert_eq!(
            power_amount(&simulator, 0, "POWER.DRAW_CARDS_NEXT_TURN_POWER"),
            2
        );
    }
}

/// Expose takes the block off what it is aimed at, takes its artifact, and
/// leaves it open.
#[test]
fn expose_strips_block_and_artifact() {
    for (upgrade, vulnerable) in [(0, 2), (1, 3)] {
        let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .played_as(id("CHARACTER.SILENT"))
            .player_hp(70, 70)
            .energy(9, 9)
            .turn(1, 1)
            .rng_counter(RunStream::MonsterAi, 0)
            .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
            .power(
                1,
                id("POWER.ARTIFACT_POWER"),
                1,
                sts2_engine::PowerType::Buff,
            )
            .card(PileName::Hand, 0, card("CARD.EXPOSE", upgrade), 0)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
        play(&mut simulator, "CARD.EXPOSE");
        assert_eq!(power_amount(&simulator, 1, "POWER.ARTIFACT_POWER"), 0);
        assert_eq!(
            power_amount(&simulator, 1, "POWER.VULNERABLE_POWER"),
            vulnerable
        );
        assert_eq!(pile(&simulator, PileName::Exhaust), ["CARD.EXPOSE"]);
    }
}

/// A hand trick makes one skill of the hand Sly for the turn, and a Sly card
/// plays itself when it is discarded.
#[test]
fn hand_trick_makes_a_skill_sly_for_the_turn() {
    for (upgrade, block) in [(0, 7), (1, 10)] {
        let mut simulator = fight(&[("CARD.HAND_TRICK", upgrade), ("CARD.DEFLECT", 0)]);
        play(&mut simulator, "CARD.HAND_TRICK");
        assert_eq!(player(&simulator).block, block);
        let combat = simulator.state().combat.as_ref().expect("a fight");
        assert!(
            combat
                .player
                .piles
                .hand
                .iter()
                .any(|card| card.sly_this_turn),
            "one skill of the hand is Sly for the turn"
        );
    }
}

/// A nightmare holds the card it was pointed at and deals three copies of it
/// into the next hand.
#[test]
fn nightmare_deals_three_copies_of_what_it_held() {
    let mut simulator = fight_with_draw(
        &[("CARD.NIGHTMARE", 0), ("CARD.SLICE", 0)],
        &[
            ("CARD.DEFLECT", 0),
            ("CARD.DEFLECT", 0),
            ("CARD.DEFLECT", 0),
            ("CARD.DEFLECT", 0),
            ("CARD.DEFLECT", 0),
        ],
    );
    play(&mut simulator, "CARD.NIGHTMARE");
    assert!(
        !choosing(&simulator),
        "a hand with one card in it is taken without asking"
    );
    assert_eq!(power_amount(&simulator, 0, "POWER.NIGHTMARE_POWER"), 3);
    end_turn(&mut simulator);
    let hand = pile(&simulator, PileName::Hand);
    assert_eq!(
        hand.iter().filter(|card| *card == "CARD.SLICE").count(),
        3,
        "three copies come up with the hand"
    );
    assert_eq!(power_amount(&simulator, 0, "POWER.NIGHTMARE_POWER"), 0);
}

/// A malaise costs X and takes that much strength away and leaves that much
/// weakness; the upgraded one is worth one more of each.
#[test]
fn malaise_spends_everything_it_can() {
    for (upgrade, bonus) in [(0, 0), (1, 1)] {
        let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .played_as(id("CHARACTER.SILENT"))
            .player_hp(70, 70)
            .energy(3, 3)
            .turn(1, 1)
            .rng_counter(RunStream::MonsterAi, 0)
            .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
            .card(PileName::Hand, 0, card("CARD.MALAISE", upgrade), 0)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
        play(&mut simulator, "CARD.MALAISE");
        assert_eq!(energy(&simulator), 0);
        assert_eq!(
            power_amount(&simulator, 1, "POWER.STRENGTH_POWER"),
            -(3 + bonus)
        );
        assert_eq!(power_amount(&simulator, 1, "POWER.WEAK_POWER"), 3 + bonus);
        assert_eq!(pile(&simulator, PileName::Exhaust), ["CARD.MALAISE"]);
    }
}

/// A skewer costs X and swings that many times.
#[test]
fn skewer_swings_once_for_every_point_of_energy() {
    for (upgrade, damage) in [(0, 8), (1, 11)] {
        let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .played_as(id("CHARACTER.SILENT"))
            .player_hp(70, 70)
            .energy(2, 3)
            .turn(1, 1)
            .rng_counter(RunStream::MonsterAi, 0)
            .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 200, 200, "SHRINKER_MOVE")
            .card(PileName::Hand, 0, card("CARD.SKEWER", upgrade), 0)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
        play(&mut simulator, "CARD.SKEWER");
        assert_eq!(creature(&simulator, 1).current_hp, 200 - 2 * damage);
    }
}

/// Bullet time makes the hand free for the turn and stops the well.
#[test]
fn bullet_time_makes_the_hand_free() {
    for upgrade in [0, 1] {
        let mut simulator = fight(&[("CARD.BULLET_TIME", upgrade), ("CARD.PINPOINT", 0)]);
        play(&mut simulator, "CARD.BULLET_TIME");
        assert_eq!(power_amount(&simulator, 0, "POWER.NO_DRAW_POWER"), 1);
        let before = energy(&simulator);
        play(&mut simulator, "CARD.PINPOINT");
        assert_eq!(energy(&simulator), before, "the hand was made free");
    }
}

/// A bouncing flask throws poison three times, rolling its target each throw.
#[test]
fn bouncing_flask_throws_poison_over_and_over() {
    for (upgrade, throws) in [(0, 3), (1, 4)] {
        let mut simulator = fight(&[("CARD.BOUNCING_FLASK", upgrade)]);
        play(&mut simulator, "CARD.BOUNCING_FLASK");
        assert_eq!(
            power_amount(&simulator, 1, "POWER.POISON_POWER"),
            3 * throws,
            "one enemy takes every throw"
        );
    }
}

/// A ricochet swings at a rolled target for each of its hits.
#[test]
fn ricochet_swings_four_times() {
    for (upgrade, hits) in [(0, 4), (1, 5)] {
        let mut simulator = fight(&[("CARD.RICOCHET", upgrade)]);
        play(&mut simulator, "CARD.RICOCHET");
        assert_eq!(creature(&simulator, 1).current_hp, 39 - 3 * hits);
    }
}

/// An echoing slash lands on everything and buys another pass for every kill.
#[test]
fn echoing_slash_buys_a_pass_for_every_kill() {
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .played_as(id("CHARACTER.SILENT"))
        .player_hp(70, 70)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 5, 39, "SHRINKER_MOVE")
        .enemy(2, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .card(PileName::Hand, 0, card("CARD.ECHOING_SLASH", 0), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.ECHOING_SLASH");
    assert_eq!(hp_of(&simulator, 1), None, "the first blow killed it");
    assert_eq!(
        hp_of(&simulator, 2),
        Some(39 - 20),
        "the kill bought a second pass"
    );
}

/// The hunt pays a card reward for a kill, and nothing at all for a swing that
/// leaves the target standing. Both halves need a fight standing inside a room,
/// because the whole body is guarded on one.
#[test]
fn the_hunt_only_pays_for_a_kill() {
    let mut simulator = fight_in_a_room(&[("CARD.THE_HUNT", 0)], 39);
    play(&mut simulator, "CARD.THE_HUNT");
    assert_eq!(hp_of(&simulator, 1), Some(39 - 10));
    assert_eq!(power_amount(&simulator, 0, "POWER.THE_HUNT_POWER"), 0);

    let mut simulator = fight_in_a_room(&[("CARD.THE_HUNT", 1)], 15);
    play(&mut simulator, "CARD.THE_HUNT");
    assert_eq!(hp_of(&simulator, 1), None, "fifteen is what it had left");
    assert_eq!(power_amount(&simulator, 0, "POWER.THE_HUNT_POWER"), 1);
    let rewards = simulator
        .state()
        .combat
        .as_ref()
        .expect("a fight")
        .victory_rewards
        .len();
    assert_eq!(rewards, 1, "the kill added a card offer to the room's own");
}

/// A fight standing inside a run's own combat room, for the one card whose body
/// asks what room it is being played in. The run is the seeded one the final
/// boss fixture stands up; only the fight in front of the player is replaced.
fn fight_in_a_room(hand: &[(&str, u8)], enemy_hp: i32) -> Simulator {
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .played_as(id("CHARACTER.SILENT"))
        .player_hp(70, 70)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatCardGeneration, 0)
        .enemy(
            1,
            id("MONSTER.SHRINKER_BEETLE"),
            enemy_hp,
            39,
            "SHRINKER_MOVE",
        )
        // A second beetle keeps the fight standing when the first goes down, so
        // that what the kill paid can be read off the room.
        .enemy(2, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE");
    for (index, (model_id, upgrade)) in hand.iter().enumerate() {
        scenario = scenario.card(
            PileName::Hand,
            u64::try_from(index).expect("small index"),
            card(model_id, *upgrade),
            0,
        );
    }
    let combat = scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
        .state()
        .combat
        .clone();
    let mut state = sts2_content::final_boss_fixture()
        .expect("the fixture builds")
        .state()
        .clone();
    state.combat = combat;
    Simulator::from_scenario(state, sts2_content::standard_registry())
        .expect("the fight stands up inside the run")
}

/// A pinpoint costs one less for every skill its owner has played this turn.
#[test]
fn pinpoint_costs_less_for_every_skill_played() {
    let mut simulator = fight(&[("CARD.DEFLECT", 0), ("CARD.PINPOINT", 0)]);
    let before = energy(&simulator);
    play(&mut simulator, "CARD.DEFLECT");
    play(&mut simulator, "CARD.PINPOINT");
    assert_eq!(
        before - energy(&simulator),
        2,
        "the skill took a point off the pinpoint"
    );
}

/// A wraith form buys intangibility with a dexterity that never stops being
/// paid.
#[test]
fn wraith_form_pays_dexterity_every_turn() {
    for (upgrade, intangible) in [(0, 2), (1, 3)] {
        let mut simulator = fight(&[("CARD.WRAITH_FORM", upgrade)]);
        play(&mut simulator, "CARD.WRAITH_FORM");
        assert_eq!(
            power_amount(&simulator, 0, "POWER.INTANGIBLE_POWER"),
            intangible
        );
        assert_eq!(power_amount(&simulator, 0, "POWER.WRAITH_FORM_POWER"), 1);
    }
}
