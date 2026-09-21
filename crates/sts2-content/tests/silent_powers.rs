//! The Silent's powers and her relics, pinned one listener body at a time.
//!
//! Both are listeners: a power answers for the creature carrying it and a relic
//! for the player holding it, and neither does anything until something else
//! happens. So each test here makes that something happen and reads what came
//! of it.

use sts2_engine::{
    Action, CardFingerprint, PileName, PowerType, ScenarioBuilder, Simulator, TargetType,
};
use sts2_rng::RunStream;

const SEED: &str = "SILENTPOWERS";

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str, upgrade_level: u8) -> CardFingerprint {
    let mut fingerprint = CardFingerprint::base(id(model_id));
    fingerprint.upgrade_level = upgrade_level;
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

/// A fight the test furnishes: the powers each side is standing with, the
/// hand, and the draw pile. The Silent's relics are pinned beside every other
/// relic in `relics.rs`, where a run can be walked into a room.
#[derive(Default)]
struct Fight<'a> {
    enemies: u64,
    enemy_hp: i32,
    powers: &'a [(u64, &'a str, i32, PowerType)],
    hand: &'a [(&'a str, u8)],
    draw: &'a [(&'a str, u8)],
    turn: u32,
    /// The move the enemy opens on. `SHRINKER_MOVE` is a debuff; `CHOMP_MOVE`
    /// is the seven-damage bite, which is what a test that needs to be hit
    /// asks for.
    enemy_move: Option<&'a str>,
}

impl Fight<'_> {
    fn build(&self) -> Simulator {
        let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .played_as(id("CHARACTER.SILENT"))
            .player_hp(70, 70)
            .energy(9, 9)
            .turn(self.turn.max(1), 1)
            .rng_counter(RunStream::MonsterAi, 0)
            .rng_counter(RunStream::CombatTargets, 0)
            .rng_counter(RunStream::CombatCardSelection, 0);
        for slot in 1..=self.enemies.max(1) {
            scenario = scenario.enemy(
                slot,
                id("MONSTER.SHRINKER_BEETLE"),
                if self.enemy_hp == 0 {
                    39
                } else {
                    self.enemy_hp
                },
                39,
                self.enemy_move.unwrap_or("SHRINKER_MOVE"),
            );
        }
        for (creature, power, amount, power_type) in self.powers {
            scenario = scenario.power(*creature, id(power), *amount, *power_type);
        }
        let mut next_id = 0;
        for (model_id, upgrade) in self.hand {
            scenario = scenario.card(PileName::Hand, next_id, card(model_id, *upgrade), 0);
            next_id += 1;
        }
        for (model_id, upgrade) in self.draw {
            scenario = scenario.card(PileName::Draw, next_id, card(model_id, *upgrade), 0);
            next_id += 1;
        }
        scenario
            .build(sts2_content::standard_registry())
            .expect("the scenario builds")
    }
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

fn power_amount(simulator: &Simulator, combat_id: u64, power: &str) -> i32 {
    creature(simulator, combat_id).power_amount(power)
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

// --- poison and what reads it -------------------------------------------

/// Poison costs its holder what it is worth as their turn opens, and one of it
/// wears off for the bite.
#[test]
fn poison_bites_once_and_wears_off() {
    let mut simulator = Fight {
        powers: &[(0, "POWER.POISON_POWER", 5, PowerType::Debuff)],
        hand: &[("CARD.DEFLECT", 0)],
        ..Fight::default()
    }
    .build();
    end_turn(&mut simulator);
    assert_eq!(player(&simulator).current_hp, 70 - 5, "the whole of it bit");
    assert_eq!(power_amount(&simulator, 0, "POWER.POISON_POWER"), 4);
}

/// An accelerant on the other side buys poison extra bites, and the amount
/// comes down between them, so the second is worth one less than the first.
#[test]
fn an_accelerant_buys_poison_another_bite() {
    let mut simulator = Fight {
        powers: &[
            (0, "POWER.POISON_POWER", 5, PowerType::Debuff),
            (1, "POWER.ACCELERANT_POWER", 1, PowerType::Buff),
        ],
        hand: &[("CARD.DEFLECT", 0)],
        ..Fight::default()
    }
    .build();
    end_turn(&mut simulator);
    assert_eq!(
        player(&simulator).current_hp,
        70 - 5 - 4,
        "five, then four once the first bite wore off"
    );
    assert_eq!(power_amount(&simulator, 0, "POWER.POISON_POWER"), 3);
}

/// Noxious fumes poison everything as their owner's turn opens.
#[test]
fn noxious_fumes_poison_the_field_every_turn() {
    let mut simulator = Fight {
        enemies: 2,
        hand: &[("CARD.NOXIOUS_FUMES", 0), ("CARD.DEFLECT", 0)],
        ..Fight::default()
    }
    .build();
    play(&mut simulator, "CARD.NOXIOUS_FUMES");
    assert_eq!(power_amount(&simulator, 1, "POWER.POISON_POWER"), 0);
    end_turn(&mut simulator);
    assert_eq!(power_amount(&simulator, 1, "POWER.POISON_POWER"), 2);
    assert_eq!(power_amount(&simulator, 2, "POWER.POISON_POWER"), 2);
}

/// A corrosive wave poisons everything every time its owner draws, and washes
/// out at the end of the turn.
#[test]
fn a_corrosive_wave_poisons_on_every_draw_and_then_goes() {
    let mut simulator = Fight {
        hand: &[("CARD.CORROSIVE_WAVE", 0), ("CARD.PREPARED", 0)],
        draw: &[("CARD.DEFLECT", 0), ("CARD.DEFLECT", 0)],
        ..Fight::default()
    }
    .build();
    play(&mut simulator, "CARD.CORROSIVE_WAVE");
    play(&mut simulator, "CARD.PREPARED");
    assert_eq!(power_amount(&simulator, 1, "POWER.POISON_POWER"), 2);
    end_turn(&mut simulator);
    assert_eq!(power_amount(&simulator, 0, "POWER.CORROSIVE_WAVE_POWER"), 0);
}

/// Envenom poisons whatever its owner's attack takes HP off, and nothing that
/// block stopped whole.
#[test]
fn envenom_poisons_what_the_attack_actually_hurt() {
    let mut simulator = Fight {
        hand: &[("CARD.ENVENOM", 0), ("CARD.SLICE", 0)],
        ..Fight::default()
    }
    .build();
    play(&mut simulator, "CARD.ENVENOM");
    play(&mut simulator, "CARD.SLICE");
    assert_eq!(power_amount(&simulator, 1, "POWER.POISON_POWER"), 1);
}

/// An outbreak burns everything every third time its owner poisons anything.
#[test]
fn an_outbreak_pays_out_every_third_poison() {
    let mut simulator = Fight {
        enemies: 2,
        hand: &[
            ("CARD.OUTBREAK", 0),
            ("CARD.DEADLY_POISON", 0),
            ("CARD.DEADLY_POISON", 0),
            ("CARD.DEADLY_POISON", 0),
        ],
        ..Fight::default()
    }
    .build();
    play(&mut simulator, "CARD.OUTBREAK");
    play(&mut simulator, "CARD.DEADLY_POISON");
    play(&mut simulator, "CARD.DEADLY_POISON");
    assert_eq!(creature(&simulator, 2).current_hp, 39, "not yet");
    play(&mut simulator, "CARD.DEADLY_POISON");
    assert_eq!(
        creature(&simulator, 2).current_hp,
        39 - 11,
        "the third poison paid out over everything"
    );
}

// --- the ones that answer for a card play --------------------------------

/// An afterimage pays block for every card its owner puts down, and nothing for
/// the play that laid it.
#[test]
fn an_afterimage_pays_for_every_card_but_its_own() {
    let mut simulator = Fight {
        hand: &[("CARD.AFTERIMAGE", 0), ("CARD.SLICE", 0)],
        ..Fight::default()
    }
    .build();
    play(&mut simulator, "CARD.AFTERIMAGE");
    assert_eq!(player(&simulator).block, 0, "laying it down pays nothing");
    play(&mut simulator, "CARD.SLICE");
    assert_eq!(player(&simulator).block, 1);
}

/// A serpent form throws what it is worth at the other side for every card its
/// owner puts down, on the same rule.
#[test]
fn a_serpent_form_strikes_for_every_card_but_its_own() {
    let mut simulator = Fight {
        hand: &[("CARD.SERPENT_FORM", 0), ("CARD.DEFLECT", 0)],
        ..Fight::default()
    }
    .build();
    play(&mut simulator, "CARD.SERPENT_FORM");
    assert_eq!(creature(&simulator, 1).current_hp, 39);
    play(&mut simulator, "CARD.DEFLECT");
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 4);
}

/// A strangle costs whoever carries it what it is worth for every card the
/// player who worked it plays.
#[test]
fn a_strangle_costs_its_holder_for_every_card_played() {
    let mut simulator = Fight {
        hand: &[("CARD.STRANGLE", 0), ("CARD.DEFLECT", 0)],
        ..Fight::default()
    }
    .build();
    play(&mut simulator, "CARD.STRANGLE");
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 8);
    play(&mut simulator, "CARD.DEFLECT");
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 8 - 2);
}

/// A burst plays the next skill twice and is spent doing it.
#[test]
fn a_burst_plays_the_next_skill_twice() {
    let mut simulator = Fight {
        hand: &[("CARD.BURST", 0), ("CARD.DEFLECT", 0)],
        ..Fight::default()
    }
    .build();
    play(&mut simulator, "CARD.BURST");
    play(&mut simulator, "CARD.DEFLECT");
    assert_eq!(player(&simulator).block, 8, "four twice over");
    assert_eq!(power_amount(&simulator, 0, "POWER.BURST_POWER"), 0);
}

/// A master planner makes every skill its owner plays Sly, for good.
#[test]
fn a_master_planner_makes_skills_sly() {
    let mut simulator = Fight {
        hand: &[("CARD.MASTER_PLANNER", 0), ("CARD.DEFLECT", 0)],
        ..Fight::default()
    }
    .build();
    play(&mut simulator, "CARD.MASTER_PLANNER");
    play(&mut simulator, "CARD.DEFLECT");
    let combat = simulator.state().combat.as_ref().expect("a fight");
    assert!(
        combat.player.piles.discard.iter().any(|card| card.sly),
        "the skill it played is Sly from now on"
    );
}

/// Being sneaky pays block whenever somebody else plays an attack. A
/// single-player run has nobody else, so it never fires; the card is
/// multiplayer-only and no single-player pool holds it.
#[test]
fn being_sneaky_never_fires_in_a_single_player_fight() {
    let mut simulator = Fight {
        hand: &[("CARD.SNEAKY", 0), ("CARD.SLICE", 0)],
        ..Fight::default()
    }
    .build();
    play(&mut simulator, "CARD.SNEAKY");
    play(&mut simulator, "CARD.SLICE");
    assert_eq!(player(&simulator).block, 0);
}

// --- the ones that answer for a draw, a discard, or a turn ---------------

/// Infinite blades deal their owner a Shiv before every hand.
#[test]
fn infinite_blades_deal_a_shiv_before_the_hand() {
    let mut simulator = Fight {
        hand: &[("CARD.INFINITE_BLADES", 0)],
        draw: &[("CARD.DEFLECT", 0)],
        ..Fight::default()
    }
    .build();
    play(&mut simulator, "CARD.INFINITE_BLADES");
    end_turn(&mut simulator);
    assert!(hand(&simulator).contains(&"CARD.SHIV".to_owned()));
}

/// A speedster cuts everything on a draw that is not the turn's own hand draw.
#[test]
fn a_speedster_cuts_on_every_draw_but_the_hand() {
    let mut simulator = Fight {
        enemies: 2,
        hand: &[("CARD.SPEEDSTER", 0), ("CARD.PREPARED", 0)],
        draw: &[("CARD.DEFLECT", 0), ("CARD.DEFLECT", 0)],
        ..Fight::default()
    }
    .build();
    play(&mut simulator, "CARD.SPEEDSTER");
    play(&mut simulator, "CARD.PREPARED");
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 2);
    assert_eq!(creature(&simulator, 2).current_hp, 39 - 2);
}

/// Tools of the trade draw one more card a turn and ask for one back.
#[test]
fn tools_of_the_trade_draw_one_more_and_take_one_back() {
    let mut simulator = Fight {
        hand: &[("CARD.TOOLS_OF_THE_TRADE", 0)],
        draw: &[
            ("CARD.DEFLECT", 0),
            ("CARD.DEFLECT", 0),
            ("CARD.DEFLECT", 0),
            ("CARD.DEFLECT", 0),
            ("CARD.DEFLECT", 0),
            ("CARD.SLICE", 0),
        ],
        ..Fight::default()
    }
    .build();
    play(&mut simulator, "CARD.TOOLS_OF_THE_TRADE");
    end_turn(&mut simulator);
    // Six came up rather than five, and one of them is asked for back.
    choose(&mut simulator, "CARD.DEFLECT");
    assert_eq!(hand(&simulator).len(), 5);
}

/// Well laid plans ask which cards to keep back on the late half of the
/// before-flush pass, where every listener gets to act before any of them
/// finishes. The engine banks the screen a deferred body asks for and opens it
/// once the whole pass has run, so ending the turn puts the
/// retain pick to the player before the hand goes down — see
/// `a_deferred_pass_still_asks.rs` for the discipline itself.
#[test]
fn well_laid_plans_ask_on_the_way_out_of_the_turn() {
    let mut simulator = Fight {
        hand: &[("CARD.WELL_LAID_PLANS", 0), ("CARD.SLICE", 0)],
        ..Fight::default()
    }
    .build();
    play(&mut simulator, "CARD.WELL_LAID_PLANS");
    end_turn(&mut simulator);
    let kept = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::ChooseCards { cards, .. }
                if cards.len() == 1 && cards[0].fingerprint.model_id == id("CARD.SLICE"))
        })
        .cloned()
        .expect("the retain pick offers the slice");
    simulator.step(kept).expect("keeping it applies");
    assert!(
        hand(&simulator).iter().any(|card| card == "CARD.SLICE"),
        "the kept slice survived the flush into the next turn"
    );
}

/// A blur keeps the block its owner is standing behind through the clear, and
/// one of it is spent as their turn opens.
#[test]
fn a_blur_keeps_block_through_the_clear() {
    let mut simulator = Fight {
        hand: &[("CARD.BLUR", 0)],
        draw: &[("CARD.DEFLECT", 0)],
        ..Fight::default()
    }
    .build();
    play(&mut simulator, "CARD.BLUR");
    end_turn(&mut simulator);
    assert_eq!(player(&simulator).block, 5, "the block stood");
    assert_eq!(power_amount(&simulator, 0, "POWER.BLUR_POWER"), 0);
}

/// A shadowmeld doubles every block its owner works up, and is gone at the end
/// of the turn.
#[test]
fn a_shadowmeld_doubles_block_for_one_turn() {
    let mut simulator = Fight {
        hand: &[("CARD.SHADOWMELD", 0), ("CARD.DEFLECT", 0)],
        draw: &[("CARD.DEFLECT", 0)],
        ..Fight::default()
    }
    .build();
    play(&mut simulator, "CARD.SHADOWMELD");
    play(&mut simulator, "CARD.DEFLECT");
    assert_eq!(player(&simulator).block, 8, "four, doubled");
    end_turn(&mut simulator);
    assert_eq!(power_amount(&simulator, 0, "POWER.SHADOWMELD_POWER"), 0);
}

/// A wraith form costs its owner dexterity every turn, and never stops.
#[test]
fn a_wraith_form_costs_dexterity_every_turn() {
    let mut simulator = Fight {
        hand: &[("CARD.WRAITH_FORM", 0)],
        draw: &[("CARD.DEFLECT", 0)],
        ..Fight::default()
    }
    .build();
    play(&mut simulator, "CARD.WRAITH_FORM");
    end_turn(&mut simulator);
    assert_eq!(power_amount(&simulator, 0, "POWER.DEXTERITY_POWER"), -1);
    assert_eq!(power_amount(&simulator, 0, "POWER.WRAITH_FORM_POWER"), 1);
}

// --- the ones that shape a value ----------------------------------------

/// Anticipating lends dexterity for the turn and takes it back.
#[test]
fn anticipating_lends_dexterity_for_the_turn() {
    let mut simulator = Fight {
        hand: &[("CARD.ANTICIPATE", 0)],
        draw: &[("CARD.DEFLECT", 0)],
        ..Fight::default()
    }
    .build();
    play(&mut simulator, "CARD.ANTICIPATE");
    assert_eq!(power_amount(&simulator, 0, "POWER.DEXTERITY_POWER"), 2);
    end_turn(&mut simulator);
    assert_eq!(power_amount(&simulator, 0, "POWER.DEXTERITY_POWER"), 0);
}

/// A piercing wail takes strength for the turn and hands it back.
#[test]
fn a_piercing_wail_takes_strength_for_the_turn() {
    let mut simulator = Fight {
        hand: &[("CARD.PIERCING_WAIL", 0)],
        draw: &[("CARD.DEFLECT", 0)],
        ..Fight::default()
    }
    .build();
    play(&mut simulator, "CARD.PIERCING_WAIL");
    assert_eq!(power_amount(&simulator, 1, "POWER.STRENGTH_POWER"), -6);
    end_turn(&mut simulator);
    assert_eq!(power_amount(&simulator, 1, "POWER.STRENGTH_POWER"), 0);
}

/// Accuracy puts its whole worth behind a Shiv and nothing behind anything
/// else.
#[test]
fn accuracy_pays_only_for_a_shiv() {
    let mut simulator = Fight {
        hand: &[
            ("CARD.ACCURACY", 0),
            ("CARD.BLADE_DANCE", 0),
            ("CARD.SLICE", 0),
        ],
        ..Fight::default()
    }
    .build();
    play(&mut simulator, "CARD.ACCURACY");
    play(&mut simulator, "CARD.SLICE");
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 6, "not a Shiv");
    play(&mut simulator, "CARD.BLADE_DANCE");
    play(&mut simulator, "CARD.SHIV");
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 6 - (4 + 4));
}

/// Phantom blades make every Shiv keep itself, and put their whole worth behind
/// the first Shiv of a turn and nothing behind the rest.
#[test]
fn phantom_blades_pay_for_the_first_shiv_of_a_turn() {
    let mut simulator = Fight {
        hand: &[("CARD.PHANTOM_BLADES", 0), ("CARD.BLADE_DANCE", 0)],
        ..Fight::default()
    }
    .build();
    play(&mut simulator, "CARD.PHANTOM_BLADES");
    play(&mut simulator, "CARD.BLADE_DANCE");
    let combat = simulator.state().combat.as_ref().expect("a fight");
    assert!(
        combat.player.piles.hand.iter().all(|card| card.retain),
        "every Shiv keeps itself"
    );
    play(&mut simulator, "CARD.SHIV");
    assert_eq!(creature(&simulator, 1).current_hp, 39 - (4 + 9));
    play(&mut simulator, "CARD.SHIV");
    assert_eq!(
        creature(&simulator, 1).current_hp,
        39 - (4 + 9) - 4,
        "only the first of the turn is paid for"
    );
}

/// Tracking multiplies what its owner's cards deal to anything shaken, and
/// leaves everything else where it was.
#[test]
fn tracking_multiplies_what_lands_on_the_weakened() {
    let mut simulator = Fight {
        hand: &[("CARD.TRACKING", 0), ("CARD.SLICE", 0), ("CARD.SLICE", 0)],
        ..Fight::default()
    }
    .build();
    play(&mut simulator, "CARD.TRACKING");
    play(&mut simulator, "CARD.SLICE");
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 6);

    let mut simulator = Fight {
        powers: &[(1, "POWER.WEAK_POWER", 1, PowerType::Debuff)],
        hand: &[("CARD.TRACKING", 0), ("CARD.SLICE", 0)],
        ..Fight::default()
    }
    .build();
    play(&mut simulator, "CARD.TRACKING");
    play(&mut simulator, "CARD.SLICE");
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 12, "twice over");
}

/// Flanking makes everyone but the player who worked it hit its holder harder.
/// It is a debuff a card puts on an enemy, and a power the scenario stands the
/// fight up with was worked by nobody, so every blow counts.
#[test]
fn flanking_doubles_what_lands_on_its_holder() {
    let mut simulator = Fight {
        powers: &[(1, "POWER.FLANKING_POWER", 2, PowerType::Debuff)],
        hand: &[("CARD.SLICE", 0)],
        ..Fight::default()
    }
    .build();
    play(&mut simulator, "CARD.SLICE");
    assert_eq!(
        creature(&simulator, 1).current_hp,
        39 - 2 * 6,
        "a blow from anyone but whoever worked it lands twice as hard"
    );
    end_turn(&mut simulator);
    end_turn(&mut simulator);
    assert_eq!(
        power_amount(&simulator, 1, "POWER.FLANKING_POWER"),
        0,
        "it comes off at the end of a turn its holder took part in"
    );
}

// --- her potion, and the pools that are hers ----------------------------

/// A poison potion works six poison onto whatever it is thrown at.
#[test]
fn a_poison_potion_poisons_what_it_is_thrown_at() {
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .played_as(id("CHARACTER.SILENT"))
        .player_hp(70, 70)
        .energy(3, 3)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .potions(vec![Some(id("POTION.POISON_POTION")), None, None])
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::UsePotion { .. }))
        .cloned()
        .expect("the potion is on the belt");
    simulator.step(action).expect("the potion applies");
    assert_eq!(power_amount(&simulator, 1, "POWER.POISON_POWER"), 6);
}

/// The three the Silent's own pool carries, and the order the epoch lists them
/// in: a potion roll reads the character's own before the shared ones.
#[test]
fn the_silents_potion_pool_is_her_three_in_the_epochs_order() {
    let content = sts2_content::standard_registry();
    let [common, uncommon, rare] = content.potion_options(&id("CHARACTER.SILENT"), false);
    assert_eq!(
        common.first().map(ToString::to_string).as_deref(),
        Some("POTION.POISON_POTION")
    );
    assert_eq!(
        uncommon.first().map(ToString::to_string).as_deref(),
        Some("POTION.CUNNING_POTION")
    );
    assert_eq!(
        rare.first().map(ToString::to_string).as_deref(),
        Some("POTION.GHOST_IN_A_JAR")
    );
}

/// The Silent's own reward pools are her card pool split by rarity, in the
/// pool's own declaration order.
#[test]
fn the_silents_reward_spec_is_her_own_pool() {
    let content = sts2_content::standard_registry();
    let spec = content
        .character_reward_spec(&id("CHARACTER.SILENT"))
        .expect("the Silent's spec is registered");
    assert_eq!(
        spec.card_pools
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["CARD_POOL.SILENT_CARD_POOL"]
    );
    assert_eq!(
        spec.common_cards
            .first()
            .map(ToString::to_string)
            .as_deref(),
        Some("CARD.ANTICIPATE")
    );
    assert_eq!(
        spec.rare_cards.first().map(ToString::to_string).as_deref(),
        Some("CARD.ABRASIVE")
    );
    assert_eq!(spec.card_option_count, 3);
    let _ = TargetType::SelfTarget;
}
