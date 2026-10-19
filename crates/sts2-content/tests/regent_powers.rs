//! The Regent's own powers, pinned one listener body at a time.
//!
//! A power is a listener: it answers for the creature carrying it and does
//! nothing at all until something else happens. So each test here stands a
//! fight up with the power already on somebody, makes the thing its body
//! answers for happen, and reads what came of it.
//!
//! Two of his declare no body at all — a Parry and a Seeking Edge are read off
//! their owner by the Sovereign Blade rather than answering anything — and both
//! are pinned where the blade reads them, in `regent_cards.rs`.

use sts2_engine::{Action, CardFingerprint, PileName, PowerType, ScenarioBuilder, Simulator};
use sts2_rng::RunStream;

const SEED: &str = "REGENTPOWERS";

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str, upgrade_level: u8) -> CardFingerprint {
    let mut fingerprint = CardFingerprint::base(id(model_id));
    fingerprint.upgrade_level = upgrade_level;
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

/// A fight the test furnishes: what each side is standing with, the powers
/// already on them, the hand and the draw pile, and how many stars the pool
/// holds. The player is the Regent throughout, since these are his powers.
#[derive(Default)]
struct Fight<'a> {
    /// One entry per enemy, in slot order, each the HP that enemy stands with.
    /// An empty list is the single beetle a test that does not care about the
    /// other side gets.
    enemies: &'a [i32],
    powers: &'a [(u64, &'a str, i32, PowerType)],
    hand: &'a [(&'a str, u8)],
    draw: &'a [(&'a str, u8)],
    stars: i32,
    /// The move every enemy of the fight opens on. `SHRINKER_MOVE` is a debuff
    /// and costs the player nothing; `CHOMP_MOVE` is the seven-damage bite,
    /// which is what a test that needs to be hit asks for.
    enemy_move: Option<&'a str>,
}

fn scenario(fight: &Fight) -> Simulator {
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .played_as(id("CHARACTER.REGENT"))
        .player_hp(75, 75)
        .energy(9, 9)
        .stars(fight.stars)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatTargets, 0)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .rng_counter(RunStream::CombatCardGeneration, 0);
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
    let mut next_id = 0;
    for (model_id, upgrade) in fight.hand {
        scenario = scenario.card(PileName::Hand, next_id, card(model_id, *upgrade), 0);
        next_id += 1;
    }
    for (model_id, upgrade) in fight.draw {
        scenario = scenario.card(PileName::Draw, next_id, card(model_id, *upgrade), 0);
        next_id += 1;
    }
    for (creature, power, amount, power_type) in fight.powers {
        scenario = scenario.power(*creature, id(power), *amount, *power_type);
    }
    scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

/// The shorthand every test but a handful uses: one power on the player, and a
/// hand.
fn holding(power: &str, amount: i32, hand: &[(&str, u8)]) -> Simulator {
    scenario(&Fight {
        powers: &[(0, power, amount, PowerType::Buff)],
        hand,
        ..Fight::default()
    })
}

fn play(simulator: &mut Simulator, model_id: &str) {
    let action = play_action(simulator, model_id)
        .unwrap_or_else(|| panic!("{model_id} was not a legal play"));
    simulator.step(action).expect("the play applies");
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

fn choose_anything(simulator: &mut Simulator) {
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
        .expect("the fight is standing")
        .creatures
        .iter()
        .find(|creature| creature.id.get() == combat_id)
        .expect("the creature is in the fight")
}

fn hp_of(simulator: &Simulator, combat_id: u64) -> i32 {
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
        .map_or(0, |creature| creature.current_hp)
}

fn block(simulator: &Simulator) -> i32 {
    creature(simulator, 0).block
}

fn power_amount(simulator: &Simulator, combat_id: u64, power: &str) -> i32 {
    creature(simulator, combat_id).power_amount(power)
}

fn has_power(simulator: &Simulator, combat_id: u64, power: &str) -> bool {
    creature(simulator, combat_id)
        .powers
        .iter()
        .any(|held| held.model_id == id(power))
}

fn stars(simulator: &Simulator) -> i32 {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .stars
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

fn hand(simulator: &Simulator) -> Vec<String> {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles
        .hand
        .iter()
        .map(|card| card.fingerprint.model_id.to_string())
        .collect()
}

// --- the powers that answer a card being conjured -------------------------

/// An arsenal pays its owner strength for every card conjured into the fight by
/// them, and a card merely drawn is not one.
#[test]
fn an_arsenal_pays_strength_for_every_card_conjured() {
    let mut simulator = holding(
        "POWER.ARSENAL_POWER",
        1,
        &[("CARD.COLLISION_COURSE", 0), ("CARD.BUNDLE_OF_JOY", 0)],
    );
    play(&mut simulator, "CARD.COLLISION_COURSE");
    assert_eq!(power_amount(&simulator, 0, "POWER.STRENGTH_POWER"), 1);
    play(&mut simulator, "CARD.BUNDLE_OF_JOY");
    assert_eq!(
        power_amount(&simulator, 0, "POWER.STRENGTH_POWER"),
        4,
        "three more for the three the bundle conjured"
    );
}

/// A pillar of creation is the same listener paying block instead, and nothing
/// scales it.
#[test]
fn a_pillar_of_creation_pays_block_for_every_card_conjured() {
    let mut simulator = holding(
        "POWER.PILLAR_OF_CREATION_POWER",
        3,
        &[("CARD.COLLISION_COURSE", 0)],
    );
    play(&mut simulator, "CARD.COLLISION_COURSE");
    assert_eq!(block(&simulator), 3);
}

// --- the powers that answer the star pool ---------------------------------

/// A black hole hurts the whole enemy side whenever its owner's stars move,
/// once for a gain and once for a play that spent them.
#[test]
fn a_black_hole_hurts_the_side_whenever_the_stars_move() {
    let mut simulator = scenario(&Fight {
        enemies: &[300, 300],
        powers: &[(0, "POWER.BLACK_HOLE_POWER", 3, PowerType::Buff)],
        hand: &[("CARD.VENERATE", 0), ("CARD.CLOAK_OF_STARS", 0)],
        stars: 0,
        ..Fight::default()
    });
    let first = hp_of(&simulator, 1);
    play(&mut simulator, "CARD.VENERATE");
    assert_eq!(first - hp_of(&simulator, 1), 3, "the gain answered");
    assert_eq!(first - hp_of(&simulator, 2), 3, "over the whole side");
    play(&mut simulator, "CARD.CLOAK_OF_STARS");
    assert_eq!(
        first - hp_of(&simulator, 1),
        6,
        "and the spend answered too"
    );
}

/// A child of the stars works up block for every star its owner spends, as much
/// per star as it is worth.
#[test]
fn a_child_of_the_stars_pays_block_for_every_star_spent() {
    let mut simulator = scenario(&Fight {
        powers: &[(0, "POWER.CHILD_OF_THE_STARS_POWER", 2, PowerType::Buff)],
        hand: &[("CARD.FALLING_STAR", 0)],
        stars: 5,
        ..Fight::default()
    });
    play(&mut simulator, "CARD.FALLING_STAR");
    assert_eq!(block(&simulator), 4, "two stars at two block each");
}

/// A genesis pays stars off the energy refill, which is where the build hangs
/// everything a turn owes its owner.
#[test]
fn a_genesis_pays_stars_as_the_turn_opens() {
    let mut simulator = holding("POWER.GENESIS_POWER", 2, &[]);
    assert_eq!(stars(&simulator), 0);
    end_turn(&mut simulator);
    assert_eq!(stars(&simulator), 2);
}

/// A star next turn pays its stars the same way and is spent for it.
#[test]
fn a_star_next_turn_pays_once_and_goes() {
    let mut simulator = holding("POWER.STAR_NEXT_TURN_POWER", 4, &[]);
    end_turn(&mut simulator);
    assert_eq!(stars(&simulator), 4);
    assert!(
        !has_power(&simulator, 0, "POWER.STAR_NEXT_TURN_POWER"),
        "it is spent for the payment"
    );
    end_turn(&mut simulator);
    assert_eq!(stars(&simulator), 4, "and pays nothing the turn after");
}

/// A sealed throne pays its owner stars before every card they play.
#[test]
fn a_sealed_throne_pays_stars_before_every_play() {
    let mut simulator = holding(
        "POWER.THE_SEALED_THRONE_POWER",
        1,
        &[("CARD.STRIKE_REGENT", 0), ("CARD.STRIKE_REGENT", 0)],
    );
    play(&mut simulator, "CARD.STRIKE_REGENT");
    assert_eq!(stars(&simulator), 1);
    play(&mut simulator, "CARD.STRIKE_REGENT");
    assert_eq!(stars(&simulator), 2);
}

// --- the powers that answer the turn --------------------------------------

/// A furnace forges as its owner's turn opens.
#[test]
fn a_furnace_forges_as_the_turn_opens() {
    let mut simulator = scenario(&Fight {
        enemies: &[300],
        powers: &[(0, "POWER.FURNACE_POWER", 5, PowerType::Buff)],
        enemy_move: Some("CHOMP_MOVE"),
        ..Fight::default()
    });
    end_turn(&mut simulator);
    let hp = hp_of(&simulator, 1);
    play(&mut simulator, "CARD.SOVEREIGN_BLADE");
    assert_eq!(hp - hp_of(&simulator, 1), 15, "the ten it carries and five");
}

/// A foregone conclusion picks cards out of the draw pile before the hand is
/// dealt, and is spent for it.
#[test]
fn a_foregone_conclusion_picks_from_the_draw_pile_and_goes() {
    let mut simulator = scenario(&Fight {
        powers: &[(0, "POWER.FOREGONE_CONCLUSION_POWER", 1, PowerType::Buff)],
        draw: &[("CARD.DEVASTATE", 0), ("CARD.STRIKE_REGENT", 0)],
        ..Fight::default()
    });
    end_turn(&mut simulator);
    choose_anything(&mut simulator);
    assert!(
        !has_power(&simulator, 0, "POWER.FOREGONE_CONCLUSION_POWER"),
        "it is spent for the pick"
    );
    assert!(hand(&simulator).contains(&"CARD.DEVASTATE".to_owned()));
}

/// A spectrum shift deals its owner colorless cards before the hand is dealt.
#[test]
fn a_spectrum_shift_deals_colorless_cards_as_the_turn_opens() {
    let mut simulator = holding("POWER.SPECTRUM_SHIFT_POWER", 1, &[]);
    end_turn(&mut simulator);
    assert_eq!(
        hand(&simulator).len(),
        1,
        "one card, out of the colorless pool"
    );
}

/// A tyranny deals its owner more cards and makes them burn as many as it is
/// worth once the hand is in front of them.
#[test]
fn a_tyranny_deals_more_and_burns_as_many() {
    let mut simulator = scenario(&Fight {
        powers: &[(0, "POWER.TYRANNY_POWER", 1, PowerType::Buff)],
        draw: &[
            ("CARD.STRIKE_REGENT", 0),
            ("CARD.STRIKE_REGENT", 0),
            ("CARD.DEFEND_REGENT", 0),
        ],
        ..Fight::default()
    });
    end_turn(&mut simulator);
    choose_anything(&mut simulator);
    assert_eq!(
        simulator
            .state()
            .combat
            .as_ref()
            .expect("the fight is standing")
            .player
            .piles
            .exhaust
            .len(),
        1,
        "one card burned for the one it is worth"
    );
}

/// A pale blue dot deals its owner more cards, but only on a turn that follows
/// one they played at least five on.
#[test]
fn a_pale_blue_dot_only_pays_after_a_busy_turn() {
    let quiet = &[
        ("CARD.STRIKE_REGENT", 0),
        ("CARD.STRIKE_REGENT", 0),
        ("CARD.STRIKE_REGENT", 0),
        ("CARD.STRIKE_REGENT", 0),
    ];
    let mut simulator = scenario(&Fight {
        enemies: &[300],
        powers: &[(0, "POWER.PALE_BLUE_DOT_POWER", 2, PowerType::Buff)],
        hand: quiet,
        draw: &[
            ("CARD.DEFEND_REGENT", 0),
            ("CARD.DEFEND_REGENT", 0),
            ("CARD.DEFEND_REGENT", 0),
            ("CARD.DEFEND_REGENT", 0),
            ("CARD.DEFEND_REGENT", 0),
            ("CARD.DEFEND_REGENT", 0),
            ("CARD.DEFEND_REGENT", 0),
        ],
        ..Fight::default()
    });
    for _ in 0..4 {
        play(&mut simulator, "CARD.STRIKE_REGENT");
    }
    end_turn(&mut simulator);
    assert_eq!(hand(&simulator).len(), 5, "four plays is not five");

    let mut simulator = scenario(&Fight {
        enemies: &[300],
        powers: &[(0, "POWER.PALE_BLUE_DOT_POWER", 2, PowerType::Buff)],
        hand: &[
            ("CARD.STRIKE_REGENT", 0),
            ("CARD.STRIKE_REGENT", 0),
            ("CARD.STRIKE_REGENT", 0),
            ("CARD.STRIKE_REGENT", 0),
            ("CARD.STRIKE_REGENT", 0),
        ],
        draw: &[
            ("CARD.DEFEND_REGENT", 0),
            ("CARD.DEFEND_REGENT", 0),
            ("CARD.DEFEND_REGENT", 0),
            ("CARD.DEFEND_REGENT", 0),
            ("CARD.DEFEND_REGENT", 0),
            ("CARD.DEFEND_REGENT", 0),
            ("CARD.DEFEND_REGENT", 0),
        ],
        ..Fight::default()
    });
    for _ in 0..5 {
        play(&mut simulator, "CARD.STRIKE_REGENT");
    }
    end_turn(&mut simulator);
    assert_eq!(hand(&simulator).len(), 7, "five plays buys the two");
}

// --- the powers that answer a play or a blow ------------------------------

/// A monologue pays its owner strength for every card they play while it
/// stands, and gives the whole of it back at the end of a turn they took part
/// in.
#[test]
fn a_monologue_lends_strength_for_the_turn_and_takes_it_back() {
    let mut simulator = holding(
        "POWER.MONOLOGUE_POWER",
        1,
        &[("CARD.STRIKE_REGENT", 0), ("CARD.DEFEND_REGENT", 0)],
    );
    play(&mut simulator, "CARD.STRIKE_REGENT");
    assert_eq!(power_amount(&simulator, 0, "POWER.STRENGTH_POWER"), 1);
    play(&mut simulator, "CARD.DEFEND_REGENT");
    assert_eq!(power_amount(&simulator, 0, "POWER.STRENGTH_POWER"), 2);
    end_turn(&mut simulator);
    assert!(!has_power(&simulator, 0, "POWER.MONOLOGUE_POWER"));
    assert_eq!(
        power_amount(&simulator, 0, "POWER.STRENGTH_POWER"),
        0,
        "the whole tally handed back"
    );
}

/// An orbit pays an energy for every four its owner spends, counted across the
/// whole fight rather than reset each turn.
#[test]
fn an_orbit_pays_an_energy_for_every_four_spent() {
    let mut simulator = holding(
        "POWER.ORBIT_POWER",
        1,
        &[("CARD.PROPHESIZE", 0), ("CARD.PROPHESIZE", 0)],
    );
    let held = energy(&simulator);
    play(&mut simulator, "CARD.PROPHESIZE");
    assert_eq!(energy(&simulator), held - 2, "two spent, nothing owed yet");
    play(&mut simulator, "CARD.PROPHESIZE");
    assert_eq!(
        energy(&simulator),
        held - 4 + 1,
        "four spent, and the one it pays for them"
    );
}

#[test]
fn an_orbit_pays_every_threshold_crossed_by_one_x_cost() {
    for (spent_before, energy_before, stacks, refund, remainder) in
        [(0, 8, 1, 2, 0), (3, 5, 1, 2, 0), (2, 11, 2, 6, 1)]
    {
        let simulator = scenario(&Fight {
            enemies: &[500],
            powers: &[(0, "POWER.ORBIT_POWER", stacks, PowerType::Buff)],
            hand: &[("CARD.ERADICATE", 1), ("CARD.FALLING_STAR", 0)],
            stars: 2,
            ..Fight::default()
        });
        let mut state = simulator.state().clone();
        let combat = state.combat.as_mut().unwrap();
        combat.player.energy = energy_before;
        combat.creatures[0].powers[0].counter = spent_before;
        let mut simulator = Simulator::from_scenario(state, sts2_content::standard_registry())
            .expect("the spending scenario builds");
        play(&mut simulator, "CARD.ERADICATE");
        assert_eq!(energy(&simulator), refund);
        assert_eq!(creature(&simulator, 0).powers[0].counter, remainder);
        play(&mut simulator, "CARD.FALLING_STAR");
        assert_eq!(energy(&simulator), refund, "a free play earns no refund");
        assert_eq!(creature(&simulator, 0).powers[0].counter, remainder);
    }
}

/// A monarch's gaze takes strength off whatever its owner's powered blows land
/// on, for the turn.
#[test]
fn a_monarchs_gaze_weakens_what_it_strikes() {
    let mut simulator = holding("POWER.MONARCHS_GAZE_POWER", 1, &[("CARD.STRIKE_REGENT", 0)]);
    play(&mut simulator, "CARD.STRIKE_REGENT");
    assert_eq!(
        power_amount(&simulator, 1, "POWER.MONARCHS_GAZE_STRENGTH_DOWN_POWER"),
        1
    );
    assert_eq!(power_amount(&simulator, 1, "POWER.STRENGTH_POWER"), -1);
}

/// The strength a crush and a dying star take away is worked off as the power
/// lands and handed back at the end of a turn its owner took part in.
#[test]
fn the_strengths_taken_away_for_a_turn_come_back() {
    for (model_id, power_id, amount) in [
        ("CARD.CRUSH_UNDER", "POWER.CRUSH_UNDER_POWER", 1),
        ("CARD.DYING_STAR", "POWER.DYING_STAR_POWER", 9),
    ] {
        let mut simulator = scenario(&Fight {
            enemies: &[300],
            hand: &[(model_id, 0)],
            stars: 20,
            ..Fight::default()
        });
        play(&mut simulator, model_id);
        assert_eq!(power_amount(&simulator, 1, power_id), amount, "{model_id}");
        assert_eq!(
            power_amount(&simulator, 1, "POWER.STRENGTH_POWER"),
            -amount,
            "{model_id}"
        );
        end_turn(&mut simulator);
        assert_eq!(
            power_amount(&simulator, 1, "POWER.STRENGTH_POWER"),
            0,
            "{model_id} hands it back at the end of the turn"
        );
    }
}

/// A reflect deals back whatever its owner's block stopped of a powered blow,
/// and counts down as their turn opens.
#[test]
fn a_reflect_deals_back_what_the_block_stopped() {
    let mut simulator = scenario(&Fight {
        enemies: &[300],
        powers: &[(0, "POWER.REFLECT_POWER", 1, PowerType::Buff)],
        hand: &[("CARD.DEFEND_REGENT", 0)],
        enemy_move: Some("CHOMP_MOVE"),
        ..Fight::default()
    });
    play(&mut simulator, "CARD.DEFEND_REGENT");
    let hp = hp_of(&simulator, 1);
    end_turn(&mut simulator);
    assert_eq!(
        hp - hp_of(&simulator, 1),
        5,
        "the whole of a seven-damage bite that five block stopped five of"
    );
    assert!(
        !has_power(&simulator, 0, "POWER.REFLECT_POWER"),
        "and it counted itself down as the turn opened"
    );
}

/// A sword sage gives every Sovereign Blade its owner holds another play, and
/// gives one to each blade that joins the fight later.
#[test]
fn a_sword_sage_gives_every_blade_another_swing() {
    let mut simulator = scenario(&Fight {
        enemies: &[300],
        hand: &[("CARD.SWORD_SAGE", 0), ("CARD.THE_SMITH", 0)],
        stars: 20,
        ..Fight::default()
    });
    play(&mut simulator, "CARD.SWORD_SAGE");
    play(&mut simulator, "CARD.THE_SMITH");
    let hp = hp_of(&simulator, 1);
    play(&mut simulator, "CARD.SOVEREIGN_BLADE");
    assert_eq!(
        hp - hp_of(&simulator, 1),
        2 * 40,
        "forty a swing, and the sage bought a second swing"
    );
}

/// A void form makes its owner's first cards free, of energy and of stars
/// alike, and counts what they have played itself.
#[test]
fn a_void_form_makes_the_first_cards_free_of_both_pools() {
    let mut simulator = scenario(&Fight {
        enemies: &[300],
        powers: &[(0, "POWER.VOID_FORM_POWER", 2, PowerType::Buff)],
        hand: &[
            ("CARD.SEVEN_STARS", 0),
            ("CARD.DEVASTATE", 0),
            ("CARD.DEVASTATE", 0),
            ("CARD.KINGLY_KICK", 0),
        ],
        stars: 0,
        ..Fight::default()
    });
    let held = energy(&simulator);
    play(&mut simulator, "CARD.SEVEN_STARS");
    assert_eq!(energy(&simulator), held, "the first is free of energy");
    assert_eq!(
        stars(&simulator),
        0,
        "and free of stars, at seven stars owed"
    );
    play(&mut simulator, "CARD.DEVASTATE");
    assert_eq!(energy(&simulator), held, "and so is the second");
    assert!(
        play_action(&simulator, "CARD.DEVASTATE").is_none(),
        "the third is not free, and four stars are more than the pool holds"
    );
    play(&mut simulator, "CARD.KINGLY_KICK");
    assert_eq!(
        energy(&simulator),
        held - 4,
        "and it pays what it costs again"
    );
}

/// A hammer time mirrors every forge onto the other players. No single-player
/// run has one, so what it declares never answers anything: a forge made while
/// it stands is the one forge.
#[test]
fn a_hammer_time_does_nothing_a_single_player_fight_can_see() {
    let mut simulator = scenario(&Fight {
        enemies: &[300],
        powers: &[(0, "POWER.HAMMER_TIME_POWER", 1, PowerType::Buff)],
        hand: &[("CARD.THE_SMITH", 0)],
        stars: 20,
        ..Fight::default()
    });
    play(&mut simulator, "CARD.THE_SMITH");
    let hp = hp_of(&simulator, 1);
    play(&mut simulator, "CARD.SOVEREIGN_BLADE");
    assert_eq!(hp - hp_of(&simulator, 1), 40);
}

/// Royalties put gold on the fight's reward set as the fight ends.
#[test]
fn royalties_put_gold_on_the_room() {
    let mut simulator = scenario(&Fight {
        enemies: &[5],
        powers: &[(0, "POWER.ROYALTIES_POWER", 30, PowerType::Buff)],
        hand: &[("CARD.STRIKE_REGENT", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.STRIKE_REGENT");
    let claimable = simulator
        .legal_actions()
        .iter()
        .filter_map(|action| match action {
            Action::ClaimReward { fingerprint, .. } => Some(format!("{fingerprint:?}")),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        claimable.iter().any(|reward| reward.contains("30")),
        "the thirty is on the room: {claimable:?}"
    );
}
