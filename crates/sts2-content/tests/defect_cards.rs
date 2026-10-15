//! The Defect's pool, pinned one card at a time.
//!
//! The two registered recordings walk about a third of his cards. What is
//! pinned here is the rest of the pool — every card the corpus never plays, at
//! both the levels it supports — plus the handful whose interesting branch no
//! recorded run takes.
//!
//! Each test stands a fight up with the card in hand and reads what came of
//! playing it: the damage, the block, the orbs, the piles, the powers.

use sts2_engine::{Action, CardFingerprint, PileName, PowerType, ScenarioBuilder, Simulator};
use sts2_rng::RunStream;

const SEED: &str = "DEFECTCARDS";

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str, upgrade_level: u8) -> CardFingerprint {
    let mut fingerprint = CardFingerprint::base(id(model_id));
    fingerprint.upgrade_level = upgrade_level;
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

/// The fight a test furnishes.
#[derive(Default)]
struct Fight<'a> {
    /// One entry per enemy, in slot order, each the HP that enemy stands with.
    enemies: &'a [i32],
    slots: i32,
    orbs: &'a [&'a str],
    hand: &'a [(&'a str, u8)],
    draw: &'a [(&'a str, u8)],
    discard: &'a [(&'a str, u8)],
    powers: &'a [(u64, &'a str, i32, PowerType)],
    relics: &'a [&'a str],
    energy: i32,
    enemy_move: Option<&'a str>,
}

fn scenario(fight: &Fight) -> Simulator {
    let energy = if fight.energy == 0 { 9 } else { fight.energy };
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .played_as(id("CHARACTER.DEFECT"))
        .player_hp(75, 75)
        .energy(energy, energy)
        .turn(1, 1)
        .orb_slots(if fight.slots == 0 { 3 } else { fight.slots })
        .relics(
            &fight
                .relics
                .iter()
                .map(|relic| id(relic))
                .collect::<Vec<_>>(),
        )
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatTargets, 0)
        .rng_counter(RunStream::CombatOrbs, 0)
        .rng_counter(RunStream::CombatCardGeneration, 0)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .rng_counter(RunStream::Shuffle, 0);
    let enemies = if fight.enemies.is_empty() {
        &[39][..]
    } else {
        fight.enemies
    };
    for (index, enemy_hp) in enemies.iter().enumerate() {
        let slot = u64::try_from(index).expect("small index") + 1;
        scenario = scenario.enemy(
            slot,
            id("MONSTER.SHRINKER_BEETLE"),
            *enemy_hp,
            *enemy_hp,
            fight.enemy_move.unwrap_or("SHRINKER_MOVE"),
        );
    }
    for orb in fight.orbs {
        scenario = scenario.orb(id(orb));
    }
    let mut next_id = 0;
    for (pile, cards) in [
        (PileName::Hand, fight.hand),
        (PileName::Draw, fight.draw),
        (PileName::Discard, fight.discard),
    ] {
        for (model_id, upgrade) in cards {
            let cost = card_cost(model_id, *upgrade);
            scenario = scenario.card(pile, next_id, card(model_id, *upgrade), cost);
            next_id += 1;
        }
    }
    for (creature, power, amount, power_type) in fight.powers {
        scenario = scenario.power(*creature, id(power), *amount, *power_type);
    }
    scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

/// What the registry says the card costs, so that a scenario's own cards are
/// standing at the cost the engine would have written onto them.
fn card_cost(model_id: &str, upgrade: u8) -> i32 {
    sts2_content::standard_registry()
        .card(&id(model_id))
        .map_or(0, |definition| match (&definition.upgrade, upgrade) {
            (Some(upgrade), 1) => upgrade.energy_cost.unwrap_or(definition.energy_cost),
            _ => definition.energy_cost,
        })
}

fn holding(hand: &[(&str, u8)]) -> Simulator {
    scenario(&Fight {
        hand,
        ..Fight::default()
    })
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

/// Plays one named card instance, which is how a test says which of two
/// copies of the same card it means.
fn play_the_card_with_id(simulator: &mut Simulator, card_id: u64) {
    let wanted = sts2_core::CardInstanceId::from(card_id);
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::PlayCard { card, .. } if card.card_id == wanted))
        .cloned()
        .unwrap_or_else(|| panic!("card {card_id} was not a legal play"));
    simulator.step(action).expect("the play applies");
}

/// The run deck read off as the models standing in it, in deck order.
fn deck_models(simulator: &Simulator) -> Vec<String> {
    simulator
        .state()
        .run_player
        .deck
        .iter()
        .map(|fingerprint| fingerprint.model_id.to_string())
        .collect()
}

fn choose_first(simulator: &mut Simulator) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseCards { cards, .. } if !cards.is_empty()))
        .cloned()
        .expect("the screen offers something");
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
        .expect("a combat is standing")
        .creatures
        .iter()
        .find(|creature| creature.id.get() == combat_id)
        .expect("the creature is in the fight")
}

fn player(simulator: &Simulator) -> &sts2_engine::PlayerCombatState {
    &simulator
        .state()
        .combat
        .as_ref()
        .expect("a combat is standing")
        .player
}

fn orbs(simulator: &Simulator) -> Vec<String> {
    player(simulator)
        .orbs
        .orbs
        .iter()
        .map(|orb| orb.model_id.to_string())
        .collect()
}

fn pile(simulator: &Simulator, pile: PileName) -> Vec<String> {
    let piles = &player(simulator).piles;
    let cards = match pile {
        PileName::Hand => &piles.hand,
        PileName::Draw => &piles.draw,
        PileName::Discard => &piles.discard,
        PileName::Exhaust => &piles.exhaust,
        PileName::Play => &piles.play,
    };
    cards
        .iter()
        .map(|card| card.fingerprint.model_id.to_string())
        .collect()
}

fn power_amount(simulator: &Simulator, combat_id: u64, power: &str) -> i32 {
    creature(simulator, combat_id)
        .powers
        .iter()
        .find(|held| held.model_id == id(power))
        .map_or(0, |held| held.amount)
}

/// A card played against one beetle, and what it left the beetle standing on.
fn damage_dealt(model_id: &str, upgrade: u8) -> i32 {
    let mut simulator = holding(&[(model_id, upgrade)]);
    play(&mut simulator, model_id);
    39 - creature(&simulator, 1).current_hp
}

/// The block a card worked up for its owner.
fn block_gained(model_id: &str, upgrade: u8) -> i32 {
    let mut simulator = holding(&[(model_id, upgrade)]);
    play(&mut simulator, model_id);
    creature(&simulator, 0).block
}

// ---------------------------------------------------------------------------
// Attacks that only swing
// ---------------------------------------------------------------------------

/// The plain attacks of the pool, at both levels.
#[test]
fn the_pools_plain_attacks_swing_for_what_they_declare() {
    for (model_id, base, upgraded) in [
        ("CARD.ALL_FOR_ONE", 10, 14),
        ("CARD.ICE_LANCE", 19, 24),
        ("CARD.METEOR_STRIKE", 24, 30),
        ("CARD.MOMENTUM_STRIKE", 10, 13),
        ("CARD.SUNDER", 24, 32),
        ("CARD.SYNTHESIS", 14, 20),
    ] {
        assert_eq!(damage_dealt(model_id, 0), base, "{model_id}");
        assert_eq!(damage_dealt(model_id, 1), upgraded, "{model_id} upgraded");
    }
}

/// The attacks that channel an orb on the way, and which orb each channels.
#[test]
fn the_attacks_that_channel_say_which_orb() {
    for (model_id, base, upgraded, orb, count) in [
        ("CARD.BALL_LIGHTNING", 7, 10, "ORB.LIGHTNING_ORB", 1),
        ("CARD.COLD_SNAP", 6, 9, "ORB.FROST_ORB", 1),
        ("CARD.ICE_LANCE", 19, 24, "ORB.FROST_ORB", 3),
        ("CARD.METEOR_STRIKE", 24, 30, "ORB.PLASMA_ORB", 3),
    ] {
        for (upgrade, dealt) in [(0, base), (1, upgraded)] {
            let mut simulator = scenario(&Fight {
                slots: 3,
                hand: &[(model_id, upgrade)],
                ..Fight::default()
            });
            play(&mut simulator, model_id);
            assert_eq!(39 - creature(&simulator, 1).current_hp, dealt, "{model_id}");
            assert_eq!(orbs(&simulator), vec![orb.to_owned(); count], "{model_id}");
        }
    }
}

/// A Barrage lands one hit per orb its owner is holding, and none at all with
/// an empty queue.
#[test]
fn a_barrage_lands_one_hit_per_orb() {
    for (held, hits) in [(0_usize, 0), (1, 1), (3, 3)] {
        let orbs = vec!["ORB.FROST_ORB"; held];
        let mut simulator = scenario(&Fight {
            orbs: &orbs,
            hand: &[("CARD.BARRAGE", 0)],
            ..Fight::default()
        });
        play(&mut simulator, "CARD.BARRAGE");
        assert_eq!(39 - creature(&simulator, 1).current_hp, 5 * hits);
    }
}

/// A Compile Driver draws one card for every distinct orb model its owner
/// holds, which is not the same as one per orb.
#[test]
fn a_compile_driver_draws_one_card_per_distinct_orb() {
    let mut simulator = scenario(&Fight {
        orbs: &["ORB.FROST_ORB", "ORB.FROST_ORB", "ORB.LIGHTNING_ORB"],
        hand: &[("CARD.COMPILE_DRIVER", 0)],
        draw: &[
            ("CARD.STRIKE_DEFECT", 0),
            ("CARD.STRIKE_DEFECT", 0),
            ("CARD.STRIKE_DEFECT", 0),
        ],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.COMPILE_DRIVER");
    assert_eq!(39 - creature(&simulator, 1).current_hp, 7);
    assert_eq!(pile(&simulator, PileName::Hand).len(), 2);
}

/// A Beam Cell works Vulnerable on, and its upgrade raises both halves.
#[test]
fn a_beam_cell_marks_what_it_hits() {
    let mut simulator = holding(&[("CARD.BEAM_CELL", 0)]);
    play(&mut simulator, "CARD.BEAM_CELL");
    assert_eq!(39 - creature(&simulator, 1).current_hp, 3);
    assert_eq!(power_amount(&simulator, 1, "POWER.VULNERABLE_POWER"), 1);
    let mut upgraded = holding(&[("CARD.BEAM_CELL", 1)]);
    play(&mut upgraded, "CARD.BEAM_CELL");
    assert_eq!(39 - creature(&upgraded, 1).current_hp, 4);
    assert_eq!(power_amount(&upgraded, 1, "POWER.VULNERABLE_POWER"), 2);
}

/// A Go For The Eyes weakens only a target that is showing an attack.
#[test]
fn go_for_the_eyes_weakens_only_an_attacker() {
    // The beetle's own move is a debuff, so nothing is weakened.
    let mut debuffing = holding(&[("CARD.GO_FOR_THE_EYES", 0)]);
    play(&mut debuffing, "CARD.GO_FOR_THE_EYES");
    assert_eq!(39 - creature(&debuffing, 1).current_hp, 3);
    assert_eq!(power_amount(&debuffing, 1, "POWER.WEAK_POWER"), 0);

    let mut attacking = scenario(&Fight {
        hand: &[("CARD.GO_FOR_THE_EYES", 0)],
        enemy_move: Some("CHOMP_MOVE"),
        ..Fight::default()
    });
    play(&mut attacking, "CARD.GO_FOR_THE_EYES");
    assert_eq!(power_amount(&attacking, 1, "POWER.WEAK_POWER"), 1);
}

/// A Gunk Up hits three times and leaves a Slimed in the discard pile.
#[test]
fn a_gunk_up_hits_three_times_and_slimes() {
    let mut simulator = holding(&[("CARD.GUNK_UP", 0)]);
    play(&mut simulator, "CARD.GUNK_UP");
    assert_eq!(39 - creature(&simulator, 1).current_hp, 12);
    assert!(pile(&simulator, PileName::Discard).contains(&"CARD.SLIMED".to_owned()));
}

/// A Null hits, weakens, and channels a Dark orb.
#[test]
fn a_null_weakens_and_channels_darkness() {
    let mut simulator = holding(&[("CARD.NULL", 0)]);
    play(&mut simulator, "CARD.NULL");
    assert_eq!(39 - creature(&simulator, 1).current_hp, 10);
    assert_eq!(power_amount(&simulator, 1, "POWER.WEAK_POWER"), 2);
    assert_eq!(orbs(&simulator), ["ORB.DARK_ORB"]);
}

/// A Refract hits twice and channels two Glass orbs.
#[test]
fn a_refract_hits_twice_and_channels_two_glass_orbs() {
    let mut simulator = holding(&[("CARD.REFRACT", 0)]);
    play(&mut simulator, "CARD.REFRACT");
    assert_eq!(39 - creature(&simulator, 1).current_hp, 18);
    assert_eq!(orbs(&simulator), ["ORB.GLASS_ORB", "ORB.GLASS_ORB"]);
}

/// A Rocket Punch swings and draws.
#[test]
fn a_rocket_punch_swings_and_draws() {
    let mut simulator = scenario(&Fight {
        hand: &[("CARD.ROCKET_PUNCH", 0)],
        draw: &[("CARD.STRIKE_DEFECT", 0), ("CARD.STRIKE_DEFECT", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.ROCKET_PUNCH");
    assert_eq!(39 - creature(&simulator, 1).current_hp, 13);
    assert_eq!(pile(&simulator, PileName::Hand).len(), 1);
}

fn rocket_costs(simulator: &Simulator) -> Vec<i32> {
    player(simulator)
        .piles
        .all_cards()
        .filter(|card| card.fingerprint.model_id == id("CARD.ROCKET_PUNCH"))
        .map(|card| card.energy_cost)
        .collect()
}

#[test]
fn rocket_punch_discounts_each_copy_in_every_combat_pile() {
    for upgrade in [0, 1] {
        let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .played_as(id("CHARACTER.DEFECT"))
            .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 100, 100, "SHRINKER_MOVE")
            .card(PileName::Hand, 0, card("CARD.BOOST_AWAY", 0), 0);
        for (index, pile) in [
            PileName::Hand,
            PileName::Draw,
            PileName::Discard,
            PileName::Exhaust,
        ]
        .into_iter()
        .enumerate()
        {
            scenario = scenario.card(
                pile,
                index as u64 + 1,
                card("CARD.ROCKET_PUNCH", upgrade),
                2,
            );
        }
        let mut simulator = scenario.build(sts2_content::standard_registry()).unwrap();
        assert_eq!(rocket_costs(&simulator), [2; 4]);
        play(&mut simulator, "CARD.BOOST_AWAY");
        assert_eq!(rocket_costs(&simulator), [0; 4]);
    }
}

#[test]
fn rocket_punchs_discount_survives_the_turn_and_clears_when_played() {
    for upgrade in [0, 1] {
        let mut simulator = holding(&[("CARD.BOOST_AWAY", 0), ("CARD.ROCKET_PUNCH", upgrade)]);
        play(&mut simulator, "CARD.BOOST_AWAY");
        assert_eq!(rocket_costs(&simulator), [0]);
        let discounted = simulator.snapshot();
        end_turn(&mut simulator);
        assert_eq!(rocket_costs(&simulator), [0]);
        let before_play = player(&simulator).energy;
        play(&mut simulator, "CARD.ROCKET_PUNCH");
        assert_eq!(player(&simulator).energy, before_play);
        assert_eq!(rocket_costs(&simulator), [2]);
        simulator.restore(&discounted).unwrap();
        assert_eq!(rocket_costs(&simulator), [0]);
    }
}

#[test]
fn rocket_punch_ignores_generated_non_status_cards_and_existing_statuses() {
    let mut simulator = scenario(&Fight {
        hand: &[
            ("CARD.ROCKET_PUNCH", 0),
            ("CARD.WHITE_NOISE", 0),
            ("CARD.COOLHEADED", 0),
        ],
        draw: &[("CARD.DAZED", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.WHITE_NOISE");
    assert_eq!(rocket_costs(&simulator), [2]);
    play(&mut simulator, "CARD.COOLHEADED");
    assert_eq!(rocket_costs(&simulator), [2]);
}

#[test]
fn rocket_punch_ignores_enemy_generated_status_cards() {
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .played_as(id("CHARACTER.DEFECT"))
        .enemy(1, id("MONSTER.NOISEBOT"), 100, 100, "NOISE_MOVE")
        .card(PileName::Hand, 0, card("CARD.ROCKET_PUNCH", 0), 2)
        .build(sts2_content::standard_registry())
        .unwrap();
    end_turn(&mut simulator);
    assert!(
        player(&simulator)
            .piles
            .all_cards()
            .any(|card| card.fingerprint.model_id == id("CARD.DAZED"))
    );
    assert_eq!(rocket_costs(&simulator), [2]);
}

/// A Scrape draws four and discards whatever still costs something.
#[test]
fn a_scrape_keeps_only_what_costs_nothing() {
    let mut simulator = scenario(&Fight {
        hand: &[("CARD.SCRAPE", 0)],
        draw: &[
            ("CARD.CLAW", 0),
            ("CARD.STRIKE_DEFECT", 0),
            ("CARD.BEAM_CELL", 0),
            ("CARD.DEFEND_DEFECT", 0),
        ],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.SCRAPE");
    assert_eq!(39 - creature(&simulator, 1).current_hp, 7);
    // The Claw and the Beam Cell cost nothing and stay; the Strike and the
    // Defend cost one each and go.
    assert_eq!(
        pile(&simulator, PileName::Hand),
        ["CARD.CLAW", "CARD.BEAM_CELL"]
    );
    assert_eq!(
        pile(&simulator, PileName::Discard),
        ["CARD.STRIKE_DEFECT", "CARD.DEFEND_DEFECT", "CARD.SCRAPE"]
    );
}

/// A Sunder pays energy only where the swing killed something.
#[test]
fn a_sunder_pays_only_for_a_kill() {
    let mut survived = scenario(&Fight {
        energy: 5,
        hand: &[("CARD.SUNDER", 0)],
        ..Fight::default()
    });
    play(&mut survived, "CARD.SUNDER");
    assert_eq!(player(&survived).energy, 2);

    // Two enemies, so the fight is still standing when the body reads what its
    // swing came to.
    let mut killed = scenario(&Fight {
        energy: 5,
        enemies: &[10, 39],
        hand: &[("CARD.SUNDER", 0)],
        ..Fight::default()
    });
    play(&mut killed, "CARD.SUNDER");
    assert_eq!(player(&killed).energy, 5);
}

/// A Sweeping Beam hits everything and draws.
#[test]
fn a_sweeping_beam_hits_everything_and_draws() {
    let mut simulator = scenario(&Fight {
        enemies: &[39, 39],
        hand: &[("CARD.SWEEPING_BEAM", 0)],
        draw: &[("CARD.STRIKE_DEFECT", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.SWEEPING_BEAM");
    assert_eq!(39 - creature(&simulator, 1).current_hp, 6);
    assert_eq!(39 - creature(&simulator, 2).current_hp, 6);
    assert_eq!(pile(&simulator, PileName::Hand).len(), 1);
}

/// A Hyperbeam hits everything and takes three Focus off its owner.
#[test]
fn a_hyperbeam_costs_its_owner_focus() {
    let mut simulator = scenario(&Fight {
        enemies: &[39, 39],
        powers: &[(0, "POWER.FOCUS_POWER", 4, PowerType::Buff)],
        hand: &[("CARD.HYPERBEAM", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.HYPERBEAM");
    assert_eq!(39 - creature(&simulator, 1).current_hp, 28);
    assert_eq!(39 - creature(&simulator, 2).current_hp, 28);
    assert_eq!(power_amount(&simulator, 0, "POWER.FOCUS_POWER"), 1);
}

/// A Helix Drill lands one hit for every point of energy its owner has already
/// spent this turn.
#[test]
fn a_helix_drill_hits_once_per_energy_spent() {
    let mut simulator = scenario(&Fight {
        energy: 9,
        hand: &[
            ("CARD.STRIKE_DEFECT", 0),
            ("CARD.STRIKE_DEFECT", 0),
            ("CARD.HELIX_DRILL", 0),
        ],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.STRIKE_DEFECT");
    play(&mut simulator, "CARD.STRIKE_DEFECT");
    let before = creature(&simulator, 1).current_hp;
    play(&mut simulator, "CARD.HELIX_DRILL");
    // Two energy spent, two hits of three.
    assert_eq!(before - creature(&simulator, 1).current_hp, 6);
}

/// A Flak Cannon burns every status outside the exhaust pile and hits once for
/// each of them.
#[test]
fn a_flak_cannon_burns_the_statuses_and_hits_for_each() {
    let mut simulator = scenario(&Fight {
        hand: &[("CARD.FLAK_CANNON", 0), ("CARD.DAZED", 0)],
        draw: &[("CARD.SLIMED", 0)],
        discard: &[("CARD.WOUND", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.FLAK_CANNON");
    assert_eq!(39 - creature(&simulator, 1).current_hp, 24);
    assert_eq!(
        pile(&simulator, PileName::Exhaust),
        ["CARD.DAZED", "CARD.SLIMED", "CARD.WOUND"]
    );
    let _ = &simulator;
}

/// An Uproar hits twice and plays an attack off the draw pile for free.
#[test]
fn an_uproar_plays_an_attack_off_the_draw_pile() {
    let mut simulator = scenario(&Fight {
        hand: &[("CARD.UPROAR", 0)],
        draw: &[("CARD.STRIKE_DEFECT", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.UPROAR");
    // Two hits of six, and the Strike it played for six more.
    assert_eq!(39 - creature(&simulator, 1).current_hp, 18);
    assert_eq!(pile(&simulator, PileName::Draw), [] as [String; 0]);
}

/// A Tesla Coil fires every Lightning orb at the target, twice when upgraded.
#[test]
fn a_tesla_coil_fires_twice_when_upgraded() {
    let mut simulator = scenario(&Fight {
        orbs: &["ORB.LIGHTNING_ORB"],
        hand: &[("CARD.TESLA_COIL", 1)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.TESLA_COIL");
    // Four from the card and three from each of the orb's two firings.
    assert_eq!(39 - creature(&simulator, 1).current_hp, 10);
}

/// An Adaptive Strike leaves a free copy of itself in the discard pile.
#[test]
fn an_adaptive_strike_copies_itself_free() {
    let mut simulator = holding(&[("CARD.ADAPTIVE_STRIKE", 0)]);
    play(&mut simulator, "CARD.ADAPTIVE_STRIKE");
    let discard = pile(&simulator, PileName::Discard);
    assert_eq!(discard, ["CARD.ADAPTIVE_STRIKE", "CARD.ADAPTIVE_STRIKE"]);
    let copy = player(&simulator)
        .piles
        .discard
        .first()
        .expect("the copy is in the discard pile");
    assert_eq!(copy.energy_cost, 0);
}

// ---------------------------------------------------------------------------
// Skills
// ---------------------------------------------------------------------------

/// The block skills of the pool, at both levels.
#[test]
fn the_pools_block_skills_work_up_what_they_declare() {
    for (model_id, base, upgraded) in [
        ("CARD.BOOT_SEQUENCE", 10, 13),
        ("CARD.GLASSWORK", 5, 8),
        ("CARD.LEAP", 9, 12),
        ("CARD.SHADOW_SHIELD", 11, 15),
    ] {
        assert_eq!(block_gained(model_id, 0), base, "{model_id}");
        assert_eq!(block_gained(model_id, 1), upgraded, "{model_id} upgraded");
    }
}

/// A Charge Battery works up block and promises energy for next turn.
#[test]
fn a_charge_battery_promises_energy_next_turn() {
    let mut simulator = holding(&[("CARD.CHARGE_BATTERY", 0)]);
    play(&mut simulator, "CARD.CHARGE_BATTERY");
    assert_eq!(creature(&simulator, 0).block, 7);
    assert_eq!(
        power_amount(&simulator, 0, "POWER.ENERGY_NEXT_TURN_POWER"),
        1
    );
}

/// A Fight Through works up block and charges the discard pile two Wounds.
#[test]
fn a_fight_through_charges_two_wounds() {
    let mut simulator = holding(&[("CARD.FIGHT_THROUGH", 0)]);
    play(&mut simulator, "CARD.FIGHT_THROUGH");
    assert_eq!(creature(&simulator, 0).block, 13);
    assert_eq!(
        pile(&simulator, PileName::Discard),
        ["CARD.WOUND", "CARD.WOUND", "CARD.FIGHT_THROUGH"]
    );
}

/// A Chill channels one Frost orb per hittable enemy.
#[test]
fn a_chill_channels_one_frost_orb_per_enemy() {
    let mut simulator = scenario(&Fight {
        enemies: &[39, 39],
        slots: 4,
        hand: &[("CARD.CHILL", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.CHILL");
    assert_eq!(orbs(&simulator), ["ORB.FROST_ORB", "ORB.FROST_ORB"]);
    // The base form burns up for having been played; the upgrade takes the
    // keyword off.
    assert_eq!(pile(&simulator, PileName::Exhaust), ["CARD.CHILL"]);
}

/// A Coolheaded channels a Frost orb and draws.
#[test]
fn a_coolheaded_channels_and_draws() {
    let mut simulator = scenario(&Fight {
        hand: &[("CARD.COOLHEADED", 0)],
        draw: &[("CARD.STRIKE_DEFECT", 0), ("CARD.STRIKE_DEFECT", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.COOLHEADED");
    assert_eq!(orbs(&simulator), ["ORB.FROST_ORB"]);
    assert_eq!(pile(&simulator, PileName::Hand).len(), 1);
}

/// A Darkness channels a Dark orb and then fires every Dark orb standing.
#[test]
fn a_darkness_fires_every_dark_orb_it_holds() {
    let mut simulator = scenario(&Fight {
        orbs: &["ORB.DARK_ORB"],
        hand: &[("CARD.DARKNESS", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.DARKNESS");
    let held = &player(&simulator).orbs.orbs;
    assert_eq!(held.len(), 2);
    // Both orbs banked six, so both are worth twelve.
    assert_eq!(held[0].evoke, 12);
    assert_eq!(held[1].evoke, 12);
}

/// A Chaos channels an orb rolled off the combat orb stream.
#[test]
fn a_chaos_channels_a_rolled_orb() {
    let mut simulator = holding(&[("CARD.CHAOS", 0)]);
    play(&mut simulator, "CARD.CHAOS");
    assert_eq!(orbs(&simulator).len(), 1);
    let mut upgraded = holding(&[("CARD.CHAOS", 1)]);
    play(&mut upgraded, "CARD.CHAOS");
    assert_eq!(orbs(&upgraded).len(), 2);
}

/// A Compact turns every status of the hand into a Fuel.
#[test]
fn a_compact_turns_the_hands_statuses_into_fuel() {
    let mut simulator = scenario(&Fight {
        hand: &[
            ("CARD.COMPACT", 0),
            ("CARD.DAZED", 0),
            ("CARD.SLIMED", 0),
            ("CARD.STRIKE_DEFECT", 0),
        ],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.COMPACT");
    assert_eq!(creature(&simulator, 0).block, 6);
    // A transform stands the replacement where the original stood, so the two
    // Fuels are where the two statuses were.
    assert_eq!(
        pile(&simulator, PileName::Hand),
        ["CARD.FUEL", "CARD.FUEL", "CARD.STRIKE_DEFECT"]
    );
}

/// A Double Energy hands its owner back whatever they are standing on.
#[test]
fn a_double_energy_doubles_what_is_left() {
    let mut simulator = scenario(&Fight {
        energy: 4,
        hand: &[("CARD.DOUBLE_ENERGY", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.DOUBLE_ENERGY");
    // Three left after the play, and three more handed back.
    assert_eq!(player(&simulator).energy, 6);
    assert_eq!(pile(&simulator, PileName::Exhaust), ["CARD.DOUBLE_ENERGY"]);
}

/// An FTL draws only while its owner has played few enough cards this turn.
#[test]
fn an_ftl_draws_under_its_own_cap() {
    let mut simulator = scenario(&Fight {
        hand: &[
            ("CARD.FTL", 0),
            ("CARD.STRIKE_DEFECT", 0),
            ("CARD.STRIKE_DEFECT", 0),
            ("CARD.STRIKE_DEFECT", 0),
            ("CARD.FTL", 0),
        ],
        draw: &[("CARD.DEFEND_DEFECT", 0), ("CARD.DEFEND_DEFECT", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.FTL");
    // No play had finished, so the first one draws.
    assert_eq!(pile(&simulator, PileName::Hand).len(), 5);
    play(&mut simulator, "CARD.STRIKE_DEFECT");
    play(&mut simulator, "CARD.STRIKE_DEFECT");
    let before = pile(&simulator, PileName::Hand).len();
    play(&mut simulator, "CARD.FTL");
    // Three plays have finished, which is the cap, so nothing is drawn.
    assert_eq!(pile(&simulator, PileName::Hand).len(), before - 1);
}

/// A Genetic Algorithm grows the block it pays every time it is played, and
/// the growth is a saved property.
#[test]
fn a_genetic_algorithm_grows_its_own_block() {
    let mut simulator = scenario(&Fight {
        hand: &[("CARD.GENETIC_ALGORITHM", 0)],
        discard: &[("CARD.GENETIC_ALGORITHM", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.GENETIC_ALGORITHM");
    assert_eq!(creature(&simulator, 0).block, 1);
    let grown = player(&simulator)
        .piles
        .exhaust
        .first()
        .expect("it burns up for having been played");
    assert_eq!(
        grown.fingerprint.properties.get("IncreasedBlock"),
        Some(&serde_json::Value::from(3))
    );
}

/// The write-through goes to the deck card the combat card was cloned from,
/// and stays with it when another card leaves the deck.
///
/// A combat card knows which card of the deck it came from. A Deprecated Card
/// takes its own deck card out mid-fight, which moves everything behind it up
/// one place, and the Genetic Algorithm standing behind it still has to
/// grow itself rather than whatever now stands where it used to.
#[test]
fn a_genetic_algorithm_grows_itself_after_a_card_leaves_the_deck() {
    let mut simulator = scenario(&Fight {
        hand: &[
            ("CARD.DEPRECATED_CARD", 0),
            ("CARD.GENETIC_ALGORITHM", 0),
            ("CARD.SYNTHESIS", 0),
        ],
        draw: &[("CARD.STRIKE_DEFECT", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.DEPRECATED_CARD");
    play(&mut simulator, "CARD.GENETIC_ALGORITHM");

    let deck = &simulator.state().run_player.deck;
    let bystander = deck
        .iter()
        .find(|fingerprint| fingerprint.model_id == id("CARD.SYNTHESIS"))
        .expect("the synthesis stands in the deck");
    assert!(
        bystander.properties.is_empty(),
        "the card standing behind the gap took nothing"
    );
    assert!(
        sts2_content::standard_registry().supports_card(bystander),
        "a synthesis carrying a property another card saved is a card no \
         registry has"
    );
    let grown = deck
        .iter()
        .find(|fingerprint| fingerprint.model_id == id("CARD.GENETIC_ALGORITHM"))
        .expect("the algorithm stands in the deck");
    assert_eq!(
        grown.properties.get("IncreasedBlock"),
        Some(&serde_json::Value::from(3)),
        "and the deck's own copy of the algorithm is what grew"
    );
    // The block the card pays is one plus what it has grown by, so the block
    // the next fight's copy starts at is four.
    assert_eq!(
        grown.properties.get("CurrentBlock"),
        Some(&serde_json::Value::from(4)),
        "and the block it carries out of the fight grew with it"
    );
}

/// Two copies of one card in the deck are two cards, and the removal takes the
/// one it was handed rather than the first that looks like it.
///
/// A removal from the deck is given one particular card, and a Deprecated Card
/// hands it the deck card it came from rather than a card of the same name.
/// Which of two identical copies goes is what the deck reads as
/// afterwards, and the deck's order is what the next fight is dealt from.
#[test]
fn a_played_copy_takes_its_own_deck_card_and_not_its_twin() {
    let mut simulator = scenario(&Fight {
        hand: &[
            ("CARD.STRIKE_DEFECT", 0),
            ("CARD.DEPRECATED_CARD", 0),
            ("CARD.GENETIC_ALGORITHM", 0),
            ("CARD.DEPRECATED_CARD", 0),
        ],
        draw: &[("CARD.DEFEND_DEFECT", 0)],
        ..Fight::default()
    });
    // The scenario's piles are the deck, draw pile first, so the deck reads
    // Defend, Strike, Deprecated, Genetic Algorithm, Deprecated — and the copy
    // standing behind the algorithm is the one holding card id 3.
    assert_eq!(
        deck_models(&simulator),
        [
            "CARD.DEFEND_DEFECT",
            "CARD.STRIKE_DEFECT",
            "CARD.DEPRECATED_CARD",
            "CARD.GENETIC_ALGORITHM",
            "CARD.DEPRECATED_CARD",
        ],
        "the deck stands with a copy on either side of the algorithm"
    );
    play_the_card_with_id(&mut simulator, 3);

    assert_eq!(
        deck_models(&simulator),
        [
            "CARD.DEFEND_DEFECT",
            "CARD.STRIKE_DEFECT",
            "CARD.DEPRECATED_CARD",
            "CARD.GENETIC_ALGORITHM",
        ],
        "the copy that was played is the copy that left, so the one in front \
         of the algorithm kept its place and the algorithm kept its own"
    );

    play(&mut simulator, "CARD.GENETIC_ALGORITHM");
    let grown = &simulator.state().run_player.deck[3];
    assert_eq!(
        grown.model_id,
        id("CARD.GENETIC_ALGORITHM"),
        "the algorithm is still standing where it stood"
    );
    assert_eq!(
        grown.properties.get("IncreasedBlock"),
        Some(&serde_json::Value::from(3)),
        "which is the card its own play grew"
    );
}

/// A Hologram takes one card back out of the discard pile.
#[test]
fn a_hologram_takes_a_card_back() {
    let mut simulator = scenario(&Fight {
        hand: &[("CARD.HOLOGRAM", 0)],
        discard: &[("CARD.STRIKE_DEFECT", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.HOLOGRAM");
    assert_eq!(creature(&simulator, 0).block, 3);
    assert_eq!(pile(&simulator, PileName::Hand), ["CARD.STRIKE_DEFECT"]);
}

/// An Overclock draws and charges the discard pile a Burn.
#[test]
fn an_overclock_draws_and_burns() {
    let mut simulator = scenario(&Fight {
        hand: &[("CARD.OVERCLOCK", 0)],
        draw: &[("CARD.STRIKE_DEFECT", 0), ("CARD.STRIKE_DEFECT", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.OVERCLOCK");
    assert_eq!(pile(&simulator, PileName::Hand).len(), 2);
    assert!(pile(&simulator, PileName::Discard).contains(&"CARD.BURN".to_owned()));
}

/// A Rainbow channels one of each of three colours.
#[test]
fn a_rainbow_channels_three_colours() {
    let mut simulator = holding(&[("CARD.RAINBOW", 0)]);
    play(&mut simulator, "CARD.RAINBOW");
    assert_eq!(
        orbs(&simulator),
        ["ORB.LIGHTNING_ORB", "ORB.FROST_ORB", "ORB.DARK_ORB"]
    );
    assert_eq!(pile(&simulator, PileName::Exhaust), ["CARD.RAINBOW"]);
}

/// A Reboot puts the hand back into the draw pile, shuffles, and deals again.
#[test]
fn a_reboot_reshuffles_the_hand() {
    let mut simulator = scenario(&Fight {
        hand: &[
            ("CARD.REBOOT", 0),
            ("CARD.STRIKE_DEFECT", 0),
            ("CARD.STRIKE_DEFECT", 0),
        ],
        draw: &[
            ("CARD.DEFEND_DEFECT", 0),
            ("CARD.DEFEND_DEFECT", 0),
            ("CARD.DEFEND_DEFECT", 0),
        ],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.REBOOT");
    assert_eq!(pile(&simulator, PileName::Hand).len(), 4);
    assert_eq!(pile(&simulator, PileName::Draw).len(), 1);
    assert_eq!(pile(&simulator, PileName::Exhaust), ["CARD.REBOOT"]);
}

/// A Scavenge burns one card of the hand and promises energy for next turn.
#[test]
fn a_scavenge_burns_a_card_for_energy() {
    let mut simulator = scenario(&Fight {
        hand: &[("CARD.SCAVENGE", 0), ("CARD.STRIKE_DEFECT", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.SCAVENGE");
    assert_eq!(pile(&simulator, PileName::Exhaust), ["CARD.STRIKE_DEFECT"]);
    assert_eq!(
        power_amount(&simulator, 0, "POWER.ENERGY_NEXT_TURN_POWER"),
        2
    );
}

/// A Supercritical is energy and nothing else.
#[test]
fn a_supercritical_is_energy_and_a_burn_up() {
    let mut simulator = scenario(&Fight {
        energy: 1,
        hand: &[("CARD.SUPERCRITICAL", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.SUPERCRITICAL");
    assert_eq!(player(&simulator).energy, 5);
    assert_eq!(pile(&simulator, PileName::Exhaust), ["CARD.SUPERCRITICAL"]);
}

/// A Turbo is energy and a Void in the discard pile.
#[test]
fn a_turbo_pays_energy_and_a_void() {
    let mut simulator = scenario(&Fight {
        energy: 1,
        hand: &[("CARD.TURBO", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.TURBO");
    assert_eq!(player(&simulator).energy, 3);
    assert!(pile(&simulator, PileName::Discard).contains(&"CARD.VOID".to_owned()));
}

/// A Synchronize works two Focus on for every distinct orb model its owner
/// holds, and gives it back at the end of the turn.
#[test]
fn a_synchronize_lends_focus_for_the_turn() {
    let mut simulator = scenario(&Fight {
        orbs: &["ORB.FROST_ORB", "ORB.FROST_ORB", "ORB.LIGHTNING_ORB"],
        hand: &[("CARD.SYNCHRONIZE", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.SYNCHRONIZE");
    assert_eq!(power_amount(&simulator, 0, "POWER.FOCUS_POWER"), 4);
    end_turn(&mut simulator);
    assert_eq!(power_amount(&simulator, 0, "POWER.FOCUS_POWER"), 0);
}

/// A Tempest channels one Lightning orb per point of energy the play spent.
#[test]
fn a_tempest_channels_one_orb_per_energy_spent() {
    let mut simulator = scenario(&Fight {
        energy: 3,
        slots: 5,
        hand: &[("CARD.TEMPEST", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.TEMPEST");
    assert_eq!(player(&simulator).energy, 0);
    assert_eq!(orbs(&simulator).len(), 3);

    let mut upgraded = scenario(&Fight {
        energy: 2,
        slots: 5,
        hand: &[("CARD.TEMPEST", 1)],
        ..Fight::default()
    });
    play(&mut upgraded, "CARD.TEMPEST");
    assert_eq!(orbs(&upgraded).len(), 3);
}

/// A Voltaic channels one Lightning orb for every one already channelled in
/// the fight.
#[test]
fn a_voltaic_repays_every_lightning_orb_of_the_fight() {
    let mut simulator = scenario(&Fight {
        energy: 6,
        slots: 6,
        hand: &[("CARD.ZAP", 0), ("CARD.ZAP", 0), ("CARD.VOLTAIC", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.ZAP");
    play(&mut simulator, "CARD.ZAP");
    play(&mut simulator, "CARD.VOLTAIC");
    assert_eq!(orbs(&simulator).len(), 4);
}

/// A White Noise deals a power card out of its owner's own pool, free for the
/// turn.
#[test]
fn a_white_noise_deals_a_free_power() {
    let mut simulator = holding(&[("CARD.WHITE_NOISE", 0)]);
    play(&mut simulator, "CARD.WHITE_NOISE");
    let hand = &player(&simulator).piles.hand;
    assert_eq!(hand.len(), 1);
    assert_eq!(hand[0].energy_cost, 0);
    let definition = sts2_content::standard_registry()
        .card(&hand[0].fingerprint.model_id)
        .expect("the conjured card is registered")
        .card_type;
    assert_eq!(definition, sts2_engine::CardType::Power);
}

/// A Boost Away works up block and charges the discard pile a Dazed.
#[test]
fn a_boost_away_charges_a_dazed() {
    let mut simulator = holding(&[("CARD.BOOST_AWAY", 0)]);
    play(&mut simulator, "CARD.BOOST_AWAY");
    assert_eq!(creature(&simulator, 0).block, 6);
    assert!(pile(&simulator, PileName::Discard).contains(&"CARD.DAZED".to_owned()));
}

/// An All For One takes every free attack, skill, or power back out of the
/// discard pile.
#[test]
fn an_all_for_one_takes_the_free_cards_back() {
    let mut simulator = scenario(&Fight {
        hand: &[("CARD.ALL_FOR_ONE", 0)],
        discard: &[
            ("CARD.CLAW", 0),
            ("CARD.STRIKE_DEFECT", 0),
            ("CARD.BEAM_CELL", 0),
            ("CARD.DAZED", 0),
        ],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.ALL_FOR_ONE");
    assert_eq!(
        pile(&simulator, PileName::Hand),
        ["CARD.CLAW", "CARD.BEAM_CELL"]
    );
}

// ---------------------------------------------------------------------------
// Evokes
// ---------------------------------------------------------------------------

/// A Shatter hits everything and then evokes every orb twice.
#[test]
fn a_shatter_evokes_every_orb_twice() {
    let mut simulator = scenario(&Fight {
        orbs: &["ORB.FROST_ORB", "ORB.FROST_ORB"],
        hand: &[("CARD.SHATTER", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.SHATTER");
    assert_eq!(39 - creature(&simulator, 1).current_hp, 7);
    // Four evokes of five block each.
    assert_eq!(creature(&simulator, 0).block, 20);
    assert_eq!(orbs(&simulator), [] as [String; 0]);
}

/// A Quadcast evokes the front orb four times and dequeues it once.
#[test]
fn a_quadcast_evokes_the_front_orb_four_times() {
    let mut simulator = scenario(&Fight {
        orbs: &["ORB.FROST_ORB", "ORB.DARK_ORB"],
        hand: &[("CARD.QUADCAST", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.QUADCAST");
    assert_eq!(creature(&simulator, 0).block, 20);
    assert_eq!(orbs(&simulator), ["ORB.DARK_ORB"]);
}

/// A Multi-Cast evokes the front orb once per point of energy the play spent.
#[test]
fn a_multi_cast_evokes_once_per_energy_spent() {
    let mut simulator = scenario(&Fight {
        energy: 2,
        orbs: &["ORB.FROST_ORB", "ORB.DARK_ORB"],
        hand: &[("CARD.MULTI_CAST", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.MULTI_CAST");
    assert_eq!(creature(&simulator, 0).block, 10);
    assert_eq!(orbs(&simulator), ["ORB.DARK_ORB"]);
}

// ---------------------------------------------------------------------------
// Powers
// ---------------------------------------------------------------------------

/// The powers the pool applies and the amount each lands with.
#[test]
fn the_pools_powers_land_for_what_they_declare() {
    for (model_id, power, base, upgraded) in [
        ("CARD.BIASED_COGNITION", "POWER.FOCUS_POWER", 4, 5),
        ("CARD.BUFFER", "POWER.BUFFER_POWER", 1, 2),
        ("CARD.COOLANT", "POWER.COOLANT_POWER", 2, 3),
        ("CARD.CREATIVE_AI", "POWER.CREATIVE_AI_POWER", 1, 1),
        ("CARD.DEFRAGMENT", "POWER.FOCUS_POWER", 1, 2),
        ("CARD.ECHO_FORM", "POWER.ECHO_FORM_POWER", 1, 1),
        ("CARD.FERAL", "POWER.FERAL_POWER", 1, 1),
        ("CARD.HAILSTORM", "POWER.HAILSTORM_POWER", 6, 8),
        ("CARD.HOTFIX", "POWER.HOTFIX_POWER", 2, 2),
        ("CARD.ITERATION", "POWER.ITERATION_POWER", 2, 3),
        ("CARD.LIGHTNING_ROD", "POWER.LIGHTNING_ROD_POWER", 2, 2),
        ("CARD.LOOP", "POWER.LOOP_POWER", 1, 2),
        (
            "CARD.MACHINE_LEARNING",
            "POWER.MACHINE_LEARNING_POWER",
            1,
            1,
        ),
        ("CARD.SIGNAL_BOOST", "POWER.SIGNAL_BOOST_POWER", 1, 1),
        ("CARD.SMOKESTACK", "POWER.SMOKESTACK_POWER", 5, 7),
        ("CARD.SPINNER", "POWER.SPINNER_POWER", 1, 1),
        ("CARD.STORM", "POWER.STORM_POWER", 1, 2),
        ("CARD.SUBROUTINE", "POWER.SUBROUTINE_POWER", 1, 1),
        ("CARD.THUNDER", "POWER.THUNDER_POWER", 6, 8),
        (
            "CARD.TRASH_TO_TREASURE",
            "POWER.TRASH_TO_TREASURE_POWER",
            1,
            1,
        ),
    ] {
        for (level, expected) in [(0, base), (1, upgraded)] {
            let mut simulator = scenario(&Fight {
                slots: 4,
                hand: &[(model_id, level)],
                ..Fight::default()
            });
            play(&mut simulator, model_id);
            assert_eq!(
                power_amount(&simulator, 0, power),
                expected,
                "{model_id} at level {level}"
            );
        }
    }
}

/// A Capacitor gives orb slots, and a Bulk Up pays one for two stats.
#[test]
fn the_slot_cards_give_and_take() {
    let mut given = holding(&[("CARD.CAPACITOR", 0)]);
    play(&mut given, "CARD.CAPACITOR");
    assert_eq!(player(&given).orbs.capacity, 5);

    let mut taken = holding(&[("CARD.BULK_UP", 0)]);
    play(&mut taken, "CARD.BULK_UP");
    assert_eq!(player(&taken).orbs.capacity, 2);
    assert_eq!(power_amount(&taken, 0, "POWER.STRENGTH_POWER"), 2);
    assert_eq!(power_amount(&taken, 0, "POWER.DEXTERITY_POWER"), 2);
}

/// A Modded gives a slot, draws, and makes itself dearer for the rest of the
/// fight.
#[test]
fn a_modded_gets_dearer_every_time() {
    let mut simulator = scenario(&Fight {
        hand: &[("CARD.MODDED", 0)],
        draw: &[("CARD.STRIKE_DEFECT", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.MODDED");
    assert_eq!(player(&simulator).orbs.capacity, 4);
    let played = player(&simulator)
        .piles
        .discard
        .iter()
        .find(|card| card.fingerprint.model_id == id("CARD.MODDED"))
        .expect("the Modded went to the discard pile");
    assert_eq!(played.energy_cost, 1);
}

/// A Consuming Shadow channels two Dark orbs and evokes the back of the queue
/// as the turn ends.
#[test]
fn a_consuming_shadow_evokes_the_back_at_the_turns_end() {
    let mut simulator = scenario(&Fight {
        slots: 4,
        orbs: &["ORB.FROST_ORB"],
        hand: &[("CARD.CONSUMING_SHADOW", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.CONSUMING_SHADOW");
    assert_eq!(
        orbs(&simulator),
        ["ORB.FROST_ORB", "ORB.DARK_ORB", "ORB.DARK_ORB"]
    );
    end_turn(&mut simulator);
    // The turn end fired both Dark orbs' passives and then the power evoked the
    // last of them, which left two standing.
    assert_eq!(orbs(&simulator).len(), 2);
}

/// A Spinner channels a Glass orb every turn as the energy is reset, and its
/// upgrade channels one as it lands.
#[test]
fn a_spinner_channels_every_turn() {
    let mut simulator = scenario(&Fight {
        slots: 5,
        hand: &[("CARD.SPINNER", 1)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.SPINNER");
    assert_eq!(orbs(&simulator), ["ORB.GLASS_ORB"]);
    end_turn(&mut simulator);
    assert_eq!(orbs(&simulator).len(), 2);
}

/// A Lightning Rod channels one orb at the next energy reset and is spent for
/// it.
#[test]
fn a_lightning_rod_is_spent_a_turn_at_a_time() {
    let mut simulator = scenario(&Fight {
        slots: 5,
        hand: &[("CARD.LIGHTNING_ROD", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.LIGHTNING_ROD");
    assert_eq!(creature(&simulator, 0).block, 4);
    end_turn(&mut simulator);
    assert_eq!(orbs(&simulator), ["ORB.LIGHTNING_ORB"]);
    assert_eq!(power_amount(&simulator, 0, "POWER.LIGHTNING_ROD_POWER"), 1);
}

/// A Signal Boost runs one power card of its owner's twice and is spent for it.
#[test]
fn a_signal_boost_runs_one_power_twice() {
    let mut simulator = scenario(&Fight {
        hand: &[("CARD.SIGNAL_BOOST", 0), ("CARD.DEFRAGMENT", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.SIGNAL_BOOST");
    play(&mut simulator, "CARD.DEFRAGMENT");
    assert_eq!(power_amount(&simulator, 0, "POWER.FOCUS_POWER"), 2);
    assert_eq!(power_amount(&simulator, 0, "POWER.SIGNAL_BOOST_POWER"), 0);
}

/// A Synthesis makes the next power card of its owner's free.
#[test]
fn a_synthesis_makes_one_power_free() {
    let mut simulator = scenario(&Fight {
        energy: 3,
        hand: &[("CARD.SYNTHESIS", 0), ("CARD.DEFRAGMENT", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.SYNTHESIS");
    assert_eq!(player(&simulator).energy, 1);
    play(&mut simulator, "CARD.DEFRAGMENT");
    assert_eq!(player(&simulator).energy, 1);
    assert_eq!(power_amount(&simulator, 0, "POWER.FREE_POWER_POWER"), 0);
}

/// A Hailstorm hits everything at the end of a turn its owner holds a Frost
/// orb through, and nothing where they hold none.
#[test]
fn a_hailstorm_needs_a_frost_orb() {
    let mut without = scenario(&Fight {
        orbs: &["ORB.DARK_ORB"],
        hand: &[("CARD.HAILSTORM", 0)],
        ..Fight::default()
    });
    play(&mut without, "CARD.HAILSTORM");
    end_turn(&mut without);
    assert_eq!(creature(&without, 1).current_hp, 39);

    let mut with = scenario(&Fight {
        orbs: &["ORB.FROST_ORB"],
        hand: &[("CARD.HAILSTORM", 0)],
        ..Fight::default()
    });
    play(&mut with, "CARD.HAILSTORM");
    end_turn(&mut with);
    // Six from the power and two from the Frost orb's own passive.
    assert_eq!(39 - creature(&with, 1).current_hp, 6);
}

/// A Smokestack hits everything for every status its owner conjures.
#[test]
fn a_smokestack_answers_a_conjured_status() {
    let mut simulator = scenario(&Fight {
        hand: &[("CARD.SMOKESTACK", 0), ("CARD.BOOST_AWAY", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.SMOKESTACK");
    play(&mut simulator, "CARD.BOOST_AWAY");
    assert_eq!(39 - creature(&simulator, 1).current_hp, 5);
}

/// A Trash To Treasure channels a rolled orb for every status its owner
/// conjures.
#[test]
fn a_trash_to_treasure_turns_a_status_into_an_orb() {
    let mut simulator = scenario(&Fight {
        hand: &[("CARD.TRASH_TO_TREASURE", 0), ("CARD.BOOST_AWAY", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.TRASH_TO_TREASURE");
    play(&mut simulator, "CARD.BOOST_AWAY");
    assert_eq!(orbs(&simulator).len(), 1);
}

/// An Iteration pays for the first status its owner draws in a turn and for
/// nothing after it.
#[test]
fn an_iteration_pays_for_the_first_status_drawn() {
    let mut simulator = scenario(&Fight {
        hand: &[("CARD.ITERATION", 0), ("CARD.SKIM", 0)],
        draw: &[
            ("CARD.DAZED", 0),
            ("CARD.DAZED", 0),
            ("CARD.STRIKE_DEFECT", 0),
            ("CARD.STRIKE_DEFECT", 0),
            ("CARD.STRIKE_DEFECT", 0),
        ],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.ITERATION");
    play(&mut simulator, "CARD.SKIM");
    // Three drawn by the Skim, and two more the first Dazed paid for.
    assert_eq!(pile(&simulator, PileName::Hand).len(), 5);
}

/// A Creative AI deals a power card out of its owner's own pool before the
/// turn's hand is dealt.
#[test]
fn a_creative_ai_deals_a_power_every_turn() {
    let mut simulator = scenario(&Fight {
        hand: &[("CARD.CREATIVE_AI", 0)],
        draw: &[("CARD.STRIKE_DEFECT", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.CREATIVE_AI");
    let before = pile(&simulator, PileName::Hand).len();
    end_turn(&mut simulator);
    assert!(pile(&simulator, PileName::Hand).len() > before);
}

/// A Machine Learning raises how many cards the turn deals.
#[test]
fn a_machine_learning_raises_the_hand_draw() {
    let draw = [("CARD.STRIKE_DEFECT", 0); 9];
    let mut simulator = scenario(&Fight {
        hand: &[("CARD.MACHINE_LEARNING", 0)],
        draw: &draw,
        ..Fight::default()
    });
    play(&mut simulator, "CARD.MACHINE_LEARNING");
    end_turn(&mut simulator);
    assert_eq!(pile(&simulator, PileName::Hand).len(), 6);
}

/// A Feral sends its owner's free attacks back to the top of the hand, as many
/// of them a turn as it is worth.
#[test]
fn a_feral_returns_one_free_attack_a_turn() {
    let mut simulator = scenario(&Fight {
        hand: &[("CARD.FERAL", 0), ("CARD.CLAW", 0), ("CARD.CLAW", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.FERAL");
    play(&mut simulator, "CARD.CLAW");
    assert_eq!(pile(&simulator, PileName::Hand).len(), 2);
    play(&mut simulator, "CARD.CLAW");
    // The second free attack is past the cap and goes to the discard.
    assert_eq!(pile(&simulator, PileName::Hand).len(), 1);
}

/// An Echo Form runs the first play of its owner's turn twice.
#[test]
fn an_echo_form_runs_the_first_play_twice() {
    let mut simulator = scenario(&Fight {
        hand: &[("CARD.ECHO_FORM", 0), ("CARD.STRIKE_DEFECT", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.ECHO_FORM");
    play(&mut simulator, "CARD.STRIKE_DEFECT");
    // The Echo Form itself was the turn's first play, so the Strike is the
    // second and runs once.
    assert_eq!(39 - creature(&simulator, 1).current_hp, 6);
}

/// A Storm channels a Lightning orb for every power card of its owner's, and
/// pays nothing for the play that brought it in.
#[test]
fn a_storm_answers_a_power_but_not_its_own_play() {
    let mut simulator = scenario(&Fight {
        slots: 5,
        hand: &[("CARD.STORM", 0), ("CARD.DEFRAGMENT", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.STORM");
    assert_eq!(orbs(&simulator), [] as [String; 0]);
    play(&mut simulator, "CARD.DEFRAGMENT");
    assert_eq!(orbs(&simulator), ["ORB.LIGHTNING_ORB"]);
}

/// A Subroutine is the same shape, paying energy instead.
#[test]
fn a_subroutine_pays_for_a_power_but_not_its_own_play() {
    let mut simulator = scenario(&Fight {
        energy: 4,
        hand: &[("CARD.SUBROUTINE", 0), ("CARD.DEFRAGMENT", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.SUBROUTINE");
    assert_eq!(player(&simulator).energy, 3);
    play(&mut simulator, "CARD.DEFRAGMENT");
    assert_eq!(player(&simulator).energy, 3);
}

/// A Loop fires the orb at the front of the queue as its owner's turn opens.
#[test]
fn a_loop_fires_the_front_orb_every_turn() {
    let mut simulator = scenario(&Fight {
        orbs: &["ORB.FROST_ORB"],
        hand: &[("CARD.LOOP", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.LOOP");
    end_turn(&mut simulator);
    // The turn end fired the orb's own passive and the block was cleared; the
    // Loop fired it again as the next turn opened.
    assert_eq!(creature(&simulator, 0).block, 2);
}

/// A Coolant pays block for every distinct orb model its owner holds as their
/// turn opens.
#[test]
fn a_coolant_pays_block_per_distinct_orb() {
    let mut simulator = scenario(&Fight {
        slots: 4,
        orbs: &["ORB.LIGHTNING_ORB", "ORB.LIGHTNING_ORB", "ORB.DARK_ORB"],
        hand: &[("CARD.COOLANT", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.COOLANT");
    end_turn(&mut simulator);
    // Two distinct models, two block each.
    assert_eq!(creature(&simulator, 0).block, 4);
}

/// A Buffer turns one blow's HP loss away and is spent for it.
#[test]
fn a_buffer_turns_one_blow_away() {
    let mut simulator = scenario(&Fight {
        hand: &[("CARD.BUFFER", 0)],
        enemy_move: Some("CHOMP_MOVE"),
        ..Fight::default()
    });
    play(&mut simulator, "CARD.BUFFER");
    end_turn(&mut simulator);
    assert_eq!(creature(&simulator, 0).current_hp, 75);
    assert_eq!(power_amount(&simulator, 0, "POWER.BUFFER_POWER"), 0);
}

/// A Biased Cognition hands four Focus over and takes one back every turn.
#[test]
fn a_biased_cognition_takes_its_focus_back() {
    let mut simulator = holding(&[("CARD.BIASED_COGNITION", 0)]);
    play(&mut simulator, "CARD.BIASED_COGNITION");
    assert_eq!(power_amount(&simulator, 0, "POWER.FOCUS_POWER"), 4);
    end_turn(&mut simulator);
    assert_eq!(power_amount(&simulator, 0, "POWER.FOCUS_POWER"), 3);
}

/// A Thunder swings at whatever a Lightning orb's evoke reached.
#[test]
fn a_thunder_answers_a_lightning_evoke() {
    let mut simulator = scenario(&Fight {
        orbs: &["ORB.LIGHTNING_ORB", "ORB.FROST_ORB"],
        hand: &[("CARD.THUNDER", 0), ("CARD.DUALCAST", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.THUNDER");
    play(&mut simulator, "CARD.DUALCAST");
    // Two evokes of eight, and six from the power for each of them.
    assert_eq!(39 - creature(&simulator, 1).current_hp, 28);
}

/// The two multiplayer-only cards are registered for what they are. An
/// Ignition aims at another player, and a card aimed at any ally is refused
/// wherever its owner has no living ally — which in a
/// single-player run is always — so it is in the hand and is never a legal
/// play. An Energy Surge aims at every ally, which is no targeting at all, so
/// it plays and pays its owner.
#[test]
fn the_multiplayer_only_cards_are_registered() {
    let ignition = holding(&[("CARD.IGNITION", 0)]);
    assert!(
        !ignition.legal_actions().iter().any(|action| matches!(
            action,
            Action::PlayCard { card, .. } if card.fingerprint.model_id == id("CARD.IGNITION")
        )),
        "an Ignition has no ally to aim at"
    );

    let mut surge = scenario(&Fight {
        energy: 2,
        hand: &[("CARD.ENERGY_SURGE", 0)],
        ..Fight::default()
    });
    play(&mut surge, "CARD.ENERGY_SURGE");
    assert_eq!(player(&surge).energy, 3);
}

/// The Defect's own starters.
#[test]
fn the_starting_deck_does_what_it_says() {
    assert_eq!(damage_dealt("CARD.STRIKE_DEFECT", 0), 6);
    assert_eq!(damage_dealt("CARD.STRIKE_DEFECT", 1), 9);
    assert_eq!(block_gained("CARD.DEFEND_DEFECT", 0), 5);
    assert_eq!(block_gained("CARD.DEFEND_DEFECT", 1), 8);
    let mut zap = holding(&[("CARD.ZAP", 0)]);
    play(&mut zap, "CARD.ZAP");
    assert_eq!(orbs(&zap), ["ORB.LIGHTNING_ORB"]);
}

/// A Glacier works up block and channels two Frost orbs, and a Glasswork one
/// Glass orb.
#[test]
fn the_block_and_channel_skills_do_both() {
    let mut glacier = scenario(&Fight {
        slots: 4,
        hand: &[("CARD.GLACIER", 0)],
        ..Fight::default()
    });
    play(&mut glacier, "CARD.GLACIER");
    assert_eq!(creature(&glacier, 0).block, 6);
    assert_eq!(orbs(&glacier), ["ORB.FROST_ORB", "ORB.FROST_ORB"]);

    let mut glasswork = holding(&[("CARD.GLASSWORK", 0)]);
    play(&mut glasswork, "CARD.GLASSWORK");
    assert_eq!(creature(&glasswork, 0).block, 5);
    assert_eq!(orbs(&glasswork), ["ORB.GLASS_ORB"]);
}

/// A Fusion channels a Plasma orb and burns up, unless it is upgraded.
#[test]
fn a_fusion_channels_plasma() {
    let mut simulator = holding(&[("CARD.FUSION", 0)]);
    play(&mut simulator, "CARD.FUSION");
    assert_eq!(orbs(&simulator), ["ORB.PLASMA_ORB"]);
    assert_eq!(pile(&simulator, PileName::Exhaust), ["CARD.FUSION"]);

    let mut upgraded = holding(&[("CARD.FUSION", 1)]);
    play(&mut upgraded, "CARD.FUSION");
    assert_eq!(pile(&upgraded, PileName::Exhaust), [] as [String; 0]);
}

/// A Skim draws and nothing else.
#[test]
fn a_skim_draws() {
    let mut simulator = scenario(&Fight {
        hand: &[("CARD.SKIM", 0)],
        draw: &[
            ("CARD.STRIKE_DEFECT", 0),
            ("CARD.STRIKE_DEFECT", 0),
            ("CARD.STRIKE_DEFECT", 0),
            ("CARD.STRIKE_DEFECT", 0),
        ],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.SKIM");
    assert_eq!(pile(&simulator, PileName::Hand).len(), 3);
}

/// A Focused Strike swings and lends Focus for the turn.
#[test]
fn a_focused_strike_lends_focus() {
    let mut simulator = holding(&[("CARD.FOCUSED_STRIKE", 0)]);
    play(&mut simulator, "CARD.FOCUSED_STRIKE");
    assert_eq!(39 - creature(&simulator, 1).current_hp, 9);
    assert_eq!(power_amount(&simulator, 0, "POWER.FOCUS_POWER"), 1);
    end_turn(&mut simulator);
    assert_eq!(power_amount(&simulator, 0, "POWER.FOCUS_POWER"), 0);
}

/// A Hotfix is Focus for the turn and nothing else.
#[test]
fn a_hotfix_lends_two_focus() {
    let mut simulator = holding(&[("CARD.HOTFIX", 0)]);
    play(&mut simulator, "CARD.HOTFIX");
    assert_eq!(power_amount(&simulator, 0, "POWER.FOCUS_POWER"), 2);
    assert_eq!(pile(&simulator, PileName::Exhaust), ["CARD.HOTFIX"]);
}

/// A Momentum Strike makes itself free for the rest of the fight.
#[test]
fn a_momentum_strike_makes_itself_free() {
    let mut simulator = holding(&[("CARD.MOMENTUM_STRIKE", 0)]);
    play(&mut simulator, "CARD.MOMENTUM_STRIKE");
    let played = player(&simulator)
        .piles
        .discard
        .first()
        .expect("it went to the discard pile");
    assert_eq!(played.energy_cost, 0);
}

/// A Claw grows every Claw of the fight, the one that swung included.
#[test]
fn a_claw_grows_every_claw() {
    let mut simulator = scenario(&Fight {
        hand: &[("CARD.CLAW", 0), ("CARD.CLAW", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.CLAW");
    assert_eq!(39 - creature(&simulator, 1).current_hp, 3);
    play(&mut simulator, "CARD.CLAW");
    assert_eq!(39 - creature(&simulator, 1).current_hp, 8);
}

/// Backing out of a Hologram's screen with an empty discard pile leaves the
/// block and takes nothing.
#[test]
fn a_hologram_with_nothing_to_take_still_pays_block() {
    let mut simulator = holding(&[("CARD.HOLOGRAM", 0)]);
    play(&mut simulator, "CARD.HOLOGRAM");
    assert_eq!(creature(&simulator, 0).block, 3);
    assert_eq!(pile(&simulator, PileName::Hand), [] as [String; 0]);
    let _ = choose_first;
}
