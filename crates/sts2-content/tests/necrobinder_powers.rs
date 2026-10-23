//! The Necrobinder's own powers, pinned one listener body at a time.
//!
//! A power is a listener: it answers for the creature carrying it and does
//! nothing at all until something else happens. So each test here stands a
//! fight up with the power already on somebody, makes the thing its body
//! answers for happen, and reads what came of it.
//!
//! Doom is the one that is not a modifier at all but a countdown on somebody
//! else's life, and it gets a section of its own. Where a body is not carried
//! in full, what its test pins is where the stop falls.

use sts2_engine::{Action, CardFingerprint, PileName, PowerType, ScenarioBuilder, Simulator};
use sts2_rng::RunStream;

const SEED: &str = "NECROBINDERPOWERS";

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
/// already on them, and the hand. The player is the Necrobinder throughout,
/// since these are her powers and her pet is part of several of them.
#[derive(Default)]
struct Fight<'a> {
    /// One entry per enemy, in slot order, each the HP that enemy stands with.
    /// An empty list is the single full-health beetle a test that does not care
    /// about the other side gets.
    enemies: &'a [i32],
    /// What the player has left. Nothing at all means the seventy she starts a
    /// run with.
    player_hp: i32,
    powers: &'a [(u64, &'a str, i32, PowerType)],
    hand: &'a [(&'a str, u8)],
    /// The move every enemy of the fight opens on. `SHRINKER_MOVE` is a debuff
    /// and costs the player nothing; `CHOMP_MOVE` is the seven-damage bite, and
    /// is what a test asks for when it needs to read whether an enemy took its
    /// turn at all.
    enemy_move: Option<&'a str>,
}

/// The fight, stood up. Energy is set high enough that a cost never gets in the
/// way of what is being measured, and the rolled streams start where a fresh
/// combat leaves them so a rolled target is the same target every run.
fn scenario(fight: &Fight) -> Simulator {
    let hp = if fight.player_hp == 0 {
        70
    } else {
        fight.player_hp
    };
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .played_as(id("CHARACTER.NECROBINDER"))
        .player_hp(hp, 70)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatTargets, 0)
        .rng_counter(RunStream::CombatCardSelection, 0);
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
            39,
            fight.enemy_move.unwrap_or("SHRINKER_MOVE"),
        );
    }
    for (creature, power, amount, power_type) in fight.powers {
        scenario = scenario.power(*creature, id(power), *amount, *power_type);
    }
    for (index, (model_id, upgrade)) in fight.hand.iter().enumerate() {
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

fn play_action(simulator: &Simulator, model_id: &str) -> Action {
    let wanted = id(model_id);
    simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::PlayCard { card, .. } if card.fingerprint.model_id == wanted)
        })
        .cloned()
        .unwrap_or_else(|| panic!("{model_id} was not legal"))
}

fn play(simulator: &mut Simulator, model_id: &str) {
    let action = play_action(simulator, model_id);
    simulator.step(action).expect("the play applies");
}

fn end_turn_action(simulator: &Simulator) -> Action {
    simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::EndTurn { .. }))
        .cloned()
        .expect("ending the turn is always legal")
}

fn end_turn(simulator: &mut Simulator) {
    let action = end_turn_action(simulator);
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

fn power_amount(simulator: &Simulator, combat_id: u64, power: &str) -> i32 {
    creature(simulator, combat_id).power_amount(power)
}

/// The Necrobinder's pet, which is the only creature of the fight whose model
/// is Osty, and nothing at all before a summon has made one.
fn pet(simulator: &Simulator) -> Option<&sts2_engine::Creature> {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .creatures
        .iter()
        .find(|creature| creature.model_id.to_string() == "MONSTER.OSTY")
}

// --- the ones that work Doom --------------------------------------------

/// A countdown works its whole amount of Doom onto one enemy, rolled on the
/// combat-targets stream, as its owner's side turn opens. It is the owner's own
/// side turn only: a round hands out one countdown's worth, not two.
#[test]
fn a_countdown_dooms_a_rolled_enemy_as_its_owners_turn_opens() {
    let mut simulator = scenario(&Fight {
        powers: &[(0, "POWER.COUNTDOWN_POWER", 6, PowerType::Buff)],
        ..Fight::default()
    });
    assert_eq!(
        power_amount(&simulator, 1, "POWER.DOOM_POWER"),
        0,
        "the turn the fight was stood up on had already opened"
    );
    end_turn(&mut simulator);
    assert_eq!(power_amount(&simulator, 1, "POWER.DOOM_POWER"), 6);
    end_turn(&mut simulator);
    assert_eq!(
        power_amount(&simulator, 1, "POWER.DOOM_POWER"),
        12,
        "once a round, as the player's own side turn opens"
    );
}

/// A neurosurge is the same listener pointed the other way: it dooms its own
/// owner, which is what the energy and the cards the card paid out cost.
#[test]
fn a_neurosurge_dooms_its_own_owner_as_their_turn_opens() {
    let mut simulator = scenario(&Fight {
        powers: &[(0, "POWER.NEUROSURGE_POWER", 3, PowerType::Debuff)],
        ..Fight::default()
    });
    end_turn(&mut simulator);
    assert_eq!(power_amount(&simulator, 0, "POWER.DOOM_POWER"), 3);
    assert_eq!(
        power_amount(&simulator, 1, "POWER.DOOM_POWER"),
        0,
        "it never leaves its owner"
    );
    end_turn(&mut simulator);
    assert_eq!(power_amount(&simulator, 0, "POWER.DOOM_POWER"), 6);
}

/// A shroud pays what it is worth in block every time its owner works Doom onto
/// anything, and nothing for any other power she works.
#[test]
fn a_shroud_pays_block_for_every_doom_its_owner_works() {
    let mut simulator = scenario(&Fight {
        powers: &[(0, "POWER.SHROUD_POWER", 2, PowerType::Buff)],
        hand: &[
            ("CARD.PUTREFY", 0),
            ("CARD.SCOURGE", 0),
            ("CARD.SCOURGE", 0),
        ],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.PUTREFY");
    assert_eq!(
        player(&simulator).block,
        0,
        "a weakness and a vulnerability are not Doom"
    );
    play(&mut simulator, "CARD.SCOURGE");
    assert_eq!(player(&simulator).block, 2);
    play(&mut simulator, "CARD.SCOURGE");
    assert_eq!(player(&simulator).block, 4, "every Doom, not the first");
}

/// The same listener answers the Doom a turn's own opening works, whether the
/// countdown threw it at an enemy or the neurosurge worked it onto its owner:
/// both name their owner as the applier, and both land after the block clear,
/// so the block they buy is standing when the hand is played.
#[test]
fn a_shroud_pays_for_the_doom_a_turn_start_works() {
    let mut simulator = scenario(&Fight {
        powers: &[
            (0, "POWER.SHROUD_POWER", 2, PowerType::Buff),
            (0, "POWER.COUNTDOWN_POWER", 6, PowerType::Buff),
            (0, "POWER.NEUROSURGE_POWER", 3, PowerType::Debuff),
        ],
        ..Fight::default()
    });
    end_turn(&mut simulator);
    assert_eq!(
        player(&simulator).block,
        4,
        "one for the enemy's Doom and one for her own"
    );
}

// --- the ones that answer for a card play --------------------------------

/// A danse macabre pays block before every card of its owner's that costs two
/// or more, and nothing for a cheaper one.
#[test]
fn a_danse_macabre_pays_block_before_every_card_worth_two() {
    let mut simulator = scenario(&Fight {
        powers: &[(0, "POWER.DANSE_MACABRE_POWER", 4, PowerType::Buff)],
        hand: &[("CARD.PUTREFY", 0), ("CARD.DEATHBRINGER", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.PUTREFY");
    assert_eq!(player(&simulator).block, 0, "a one is not a two");
    play(&mut simulator, "CARD.DEATHBRINGER");
    assert_eq!(player(&simulator).block, 4);
}

/// A spirit of ash pays block before every ethereal card its owner plays, and
/// nothing for one that is not.
#[test]
fn a_spirit_of_ash_pays_block_before_every_ethereal_card() {
    let mut simulator = scenario(&Fight {
        powers: &[(0, "POWER.SPIRIT_OF_ASH_POWER", 4, PowerType::Buff)],
        hand: &[("CARD.PUTREFY", 0), ("CARD.DEFILE", 0), ("CARD.DEFILE", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.PUTREFY");
    assert_eq!(player(&simulator).block, 0, "a putrefy is not ethereal");
    play(&mut simulator, "CARD.DEFILE");
    assert_eq!(player(&simulator).block, 4);
    play(&mut simulator, "CARD.DEFILE");
    assert_eq!(player(&simulator).block, 8, "every ethereal card");
}

/// Devouring life summons every time its owner plays a Soul, and for nothing
/// else. The summon is the whole of it: the first one makes a pet with the
/// power's amount for a maximum, and the next raises that maximum by
/// as much again and heals the pet by what it went up by.
#[test]
fn devouring_life_summons_for_every_soul_its_owner_plays() {
    let mut simulator = scenario(&Fight {
        powers: &[(0, "POWER.DEVOUR_LIFE_POWER", 1, PowerType::Buff)],
        hand: &[("CARD.PUTREFY", 0), ("CARD.SOUL", 0), ("CARD.SOUL", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.PUTREFY");
    assert!(pet(&simulator).is_none(), "a putrefy is not a Soul");
    play(&mut simulator, "CARD.SOUL");
    let summoned = pet(&simulator).expect("the Soul brought the pet");
    assert_eq!(summoned.max_hp, 1);
    assert_eq!(summoned.current_hp, 1);
    play(&mut simulator, "CARD.SOUL");
    let grown = pet(&simulator).expect("the pet is still standing");
    assert_eq!(grown.max_hp, 2);
    assert_eq!(grown.current_hp, 2);
}

/// A haunt cuts one enemy, rolled on the combat-targets stream, every time its
/// owner plays a Soul. What it cuts for is its own amount, and nothing scales
/// it and no block stops it.
#[test]
fn a_haunt_cuts_a_rolled_enemy_for_every_soul() {
    let mut simulator = scenario(&Fight {
        powers: &[
            (0, "POWER.HAUNT_POWER", 6, PowerType::Buff),
            (0, "POWER.STRENGTH_POWER", 3, PowerType::Buff),
        ],
        hand: &[("CARD.PUTREFY", 0), ("CARD.SOUL", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.PUTREFY");
    assert_eq!(hp_of(&simulator, 1), Some(39), "a putrefy is not a Soul");
    play(&mut simulator, "CARD.SOUL");
    assert_eq!(
        hp_of(&simulator, 1),
        Some(39 - 6),
        "what the power is worth, and no strength on top of it"
    );
}

// --- Doom, which is a countdown on somebody else's life ------------------

/// A creature carrying at least as much Doom as it has HP left goes down as the
/// side it is on ends its turn — its own side, not whichever side is ending
/// one. The enemy side's turn ends inside the player's own hand-over, so the
/// doomed beetle takes its whole turn first and is gone before the next player
/// turn opens.
#[test]
fn doom_kills_as_the_side_it_is_on_ends_its_turn() {
    let mut simulator = scenario(&Fight {
        enemies: &[10, 39],
        powers: &[(1, "POWER.DOOM_POWER", 10, PowerType::Debuff)],
        enemy_move: Some("CHOMP_MOVE"),
        ..Fight::default()
    });
    assert_eq!(
        hp_of(&simulator, 1),
        Some(10),
        "nothing happens where the Doom is worked"
    );
    end_turn(&mut simulator);
    assert_eq!(hp_of(&simulator, 1), None, "as much Doom as HP is enough");
    assert_eq!(
        hp_of(&simulator, 2),
        Some(39),
        "the one carrying no Doom is left standing"
    );
    assert_eq!(
        player(&simulator).current_hp,
        70 - 7 - 7,
        "both beetles bit before the doomed one went down"
    );
}

/// It kills nothing while the creature still has more HP left than the Doom is
/// worth, however many turns it sits there.
#[test]
fn doom_leaves_a_creature_with_more_hp_than_doom_standing() {
    let mut simulator = scenario(&Fight {
        powers: &[(1, "POWER.DOOM_POWER", 10, PowerType::Debuff)],
        ..Fight::default()
    });
    end_turn(&mut simulator);
    end_turn(&mut simulator);
    assert_eq!(hp_of(&simulator, 1), Some(39));
    assert_eq!(
        power_amount(&simulator, 1, "POWER.DOOM_POWER"),
        10,
        "Doom is a counter and nothing takes it back down"
    );
}

/// Every doomed creature on the side goes down in the one pass. The build
/// gathers the whole doomed list from the first doomed creature's own listener
/// and kills it there, so the ones behind it find themselves already down when
/// their own listener is reached.
#[test]
fn the_whole_doomed_list_goes_down_in_one_pass() {
    let mut simulator = scenario(&Fight {
        enemies: &[10, 39, 5],
        powers: &[
            (1, "POWER.DOOM_POWER", 10, PowerType::Debuff),
            (3, "POWER.DOOM_POWER", 30, PowerType::Debuff),
        ],
        ..Fight::default()
    });
    end_turn(&mut simulator);
    assert_eq!(hp_of(&simulator, 1), None, "the first of the doomed");
    assert_eq!(hp_of(&simulator, 3), None, "and the one behind it");
    assert_eq!(
        hp_of(&simulator, 2),
        Some(39),
        "the undoomed beetle between them stands"
    );
}

/// The player's own side is a side like any other: as her turn ends, as much
/// Doom on her as she has HP left takes her down, and the enemies never get to
/// act on the turn it happens.
#[test]
fn doom_takes_the_player_down_as_her_own_turn_ends() {
    let mut simulator = scenario(&Fight {
        player_hp: 10,
        powers: &[(0, "POWER.DOOM_POWER", 10, PowerType::Debuff)],
        ..Fight::default()
    });
    end_turn(&mut simulator);
    assert_eq!(
        simulator.state().terminal,
        Some(sts2_engine::RunResult::Defeat)
    );
}

// --- the three that read the fight back ---------------------------------

/// A necro mastery answers every point of HP its owner's pet loses by throwing
/// that many times its own worth at the whole enemy side, unblockably. The
/// blow that costs the pet its health is what sets it off, and the health the
/// owner loses is not: the health-changed pass hands every listener the
/// creature whose health moved, and the mastery answers only for its owner's
/// own pet.
#[test]
fn a_necro_mastery_throws_what_the_pet_loses_at_the_whole_enemy_side() {
    for (stacks, per_point) in [(1, 1), (3, 3)] {
        let mut simulator = scenario(&Fight {
            enemies: &[39, 39],
            powers: &[(0, "POWER.NECRO_MASTERY_POWER", stacks, PowerType::Buff)],
            hand: &[("CARD.BODYGUARD", 0)],
            enemy_move: Some("CHOMP_MOVE"),
            ..Fight::default()
        });
        play(&mut simulator, "CARD.BODYGUARD");
        assert_eq!(pet(&simulator).expect("a pet is standing").current_hp, 5);
        // The chomp is redirected onto the pet by its Die For You, so the five
        // it had left is what it loses; the two beyond that trample through
        // onto the player and are her health rather than the pet's.
        end_turn(&mut simulator);
        assert_eq!(pet(&simulator).expect("the corpse stays").current_hp, 0);
        for slot in [1, 2] {
            assert_eq!(
                hp_of(&simulator, slot),
                Some(39 - 5 * per_point),
                "five points off the pet at {per_point} each, on every enemy"
            );
        }
    }
}

/// A kill is a loss of HP like any other: whatever the creature had left is
/// announced on the health-changed pass before it dies. So a pet put down by
/// its own card sets its owner's necro mastery off for every point it had.
/// Seed `Q8C003YSS3`, record 169, where bone shards put down a pet on one HP
/// and the mastery's point went missing.
#[test]
fn a_pet_put_down_by_a_kill_sets_the_necro_mastery_off_for_what_it_had() {
    let mut simulator = scenario(&Fight {
        enemies: &[39, 39],
        powers: &[(0, "POWER.NECRO_MASTERY_POWER", 1, PowerType::Buff)],
        hand: &[("CARD.BODYGUARD", 0), ("CARD.BONE_SHARDS", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.BODYGUARD");
    assert_eq!(pet(&simulator).expect("a pet is standing").current_hp, 5);
    play(&mut simulator, "CARD.BONE_SHARDS");
    assert_eq!(pet(&simulator).expect("the corpse stays").current_hp, 0);
    for slot in [1, 2] {
        assert_eq!(
            hp_of(&simulator, slot),
            Some(39 - 9 - 5),
            "the sweep, and then the five the pet had left"
        );
    }
}

/// A necro mastery says nothing at all for its owner's own health, which is
/// the half of the guard asking whether the creature that lost it is a pet.
#[test]
fn a_necro_mastery_says_nothing_for_its_owners_own_health() {
    let mut simulator = scenario(&Fight {
        powers: &[(0, "POWER.NECRO_MASTERY_POWER", 1, PowerType::Buff)],
        enemy_move: Some("CHOMP_MOVE"),
        ..Fight::default()
    });
    end_turn(&mut simulator);
    assert_eq!(hp_of(&simulator, 1), Some(39), "the beetle is untouched");
}

/// A reaper form dooms whatever its owner's powered attack lands on, for the
/// whole blow times what the form is worth — the half block stopped and the
/// half that got through, together.
#[test]
fn a_reaper_form_dooms_for_the_whole_blow() {
    for (stacks, doom) in [(1, 13), (2, 26)] {
        let mut simulator = scenario(&Fight {
            powers: &[(0, "POWER.REAPER_FORM_POWER", stacks, PowerType::Buff)],
            hand: &[("CARD.DEFILE", 0)],
            ..Fight::default()
        });
        play(&mut simulator, "CARD.DEFILE");
        assert_eq!(hp_of(&simulator, 1), Some(39 - 13));
        assert_eq!(power_amount(&simulator, 1, "POWER.DOOM_POWER"), doom);
    }
}

/// The pet's blows count too, and a blow worth nothing dooms nothing: a Sweeping
/// Gaze with no pet to swing it issues no attack at all.
#[test]
fn a_reaper_form_dooms_for_the_pets_blows_and_not_for_a_swing_that_never_happened() {
    let mut simulator = scenario(&Fight {
        powers: &[(0, "POWER.REAPER_FORM_POWER", 1, PowerType::Buff)],
        hand: &[
            ("CARD.SWEEPING_GAZE", 0),
            ("CARD.BODYGUARD", 0),
            ("CARD.SWEEPING_GAZE", 0),
        ],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.SWEEPING_GAZE");
    assert_eq!(power_amount(&simulator, 1, "POWER.DOOM_POWER"), 0);
    play(&mut simulator, "CARD.BODYGUARD");
    play(&mut simulator, "CARD.SWEEPING_GAZE");
    assert_eq!(hp_of(&simulator, 1), Some(39 - 10));
    assert_eq!(power_amount(&simulator, 1, "POWER.DOOM_POWER"), 10);
}

/// Sic 'Em marks one enemy: a blow the marking player's own pet lands on it
/// summons for what the mark is worth, and the mark is gone at the end of a
/// turn its holder took part in. Her own blow is not the pet's and buys
/// nothing.
#[test]
fn a_sic_em_is_paid_by_the_pets_blow_and_wears_off_at_the_turn_end() {
    let mut simulator = scenario(&Fight {
        powers: &[(1, "POWER.SIC_EM_POWER", 3, PowerType::Debuff)],
        ..Fight::default()
    });
    end_turn(&mut simulator);
    assert_eq!(power_amount(&simulator, 1, "POWER.SIC_EM_POWER"), 0);

    // The mark has to be laid down by the player for the summon to be hers:
    // the pet's own owner being the mark's applier is the whole of the guard.
    let mut simulator = scenario(&Fight {
        hand: &[
            ("CARD.SIC_EM", 0),
            ("CARD.DEFILE", 0),
            ("CARD.BODYGUARD", 0),
            ("CARD.UNLEASH", 0),
        ],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.SIC_EM");
    assert_eq!(power_amount(&simulator, 1, "POWER.SIC_EM_POWER"), 3);
    play(&mut simulator, "CARD.DEFILE");
    assert!(
        pet(&simulator).is_none(),
        "no pet swung that, and none arrived"
    );
    play(&mut simulator, "CARD.BODYGUARD");
    assert_eq!(pet(&simulator).expect("the bodyguard's pet").max_hp, 5);
    // The unleash is the pet's blow on the marked beetle, so the mark pays its
    // three: a standing pet takes the summon as a maximum raised by three and
    // a heal for as much again.
    play(&mut simulator, "CARD.UNLEASH");
    let standing = pet(&simulator).expect("the pet is still up");
    assert_eq!((standing.max_hp, standing.current_hp), (8, 8));
}

/// A double damage doubles what a pet of its owner's swings off a card as
/// readily as what its owner does: the dealer being one of its owner's own
/// pets is the other half of the guard. It is spent as the side's turn ends
/// either way.
#[test]
fn a_double_damage_counts_the_owners_pet_as_the_owner() {
    let mut simulator = scenario(&Fight {
        powers: &[(0, "POWER.DOUBLE_DAMAGE_POWER", 1, PowerType::Buff)],
        hand: &[("CARD.BODYGUARD", 0), ("CARD.POKE", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.BODYGUARD");
    play(&mut simulator, "CARD.POKE");
    let doubled = 39 - hp_of(&simulator, 1).expect("the beetle is standing");

    let mut plain = scenario(&Fight {
        hand: &[("CARD.BODYGUARD", 0), ("CARD.POKE", 0)],
        ..Fight::default()
    });
    play(&mut plain, "CARD.BODYGUARD");
    play(&mut plain, "CARD.POKE");
    let single = 39 - hp_of(&plain, 1).expect("the beetle is standing");
    assert_eq!(doubled, single * 2, "the pet's blow is the owner's");
}
