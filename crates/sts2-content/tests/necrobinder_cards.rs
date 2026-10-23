//! The Necrobinder's pool, pinned one behaviour at a time.
//!
//! No recording in the corpus plays any of these, so a scenario is the whole of
//! the evidence for each of them. Three shapes recur and are worth naming once.
//! Doom is a countdown written onto somebody else's life, and most of her pool
//! is one way or another of putting more of it there. A Soul is a token her own
//! cards conjure. And her pet is summoned rather than assumed: an Osty attack
//! is guarded on the pet standing, so a play made with no pet issues nothing at
//! all, which is a behaviour of its own and has its own test here.
//!
//! Where a family of cards is one shape with different numbers — a power laid
//! down, a body the baseline stops on — the family is one table-driven test
//! and each card of it is a row.

use sts2_engine::{Action, CardFingerprint, PileName, PowerType, ScenarioBuilder, Simulator};
use sts2_rng::RunStream;

const SEED: &str = "NECROBINDERCARDS";

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str, upgrade_level: u8) -> CardFingerprint {
    let mut fingerprint = CardFingerprint::base(id(model_id));
    fingerprint.upgrade_level = upgrade_level;
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

/// One fight against a single beetle, played as the Necrobinder, with the hand
/// the test names and enough energy that a cost never gets in the way of what
/// is being measured.
fn fight(hand: &[(&str, u8)]) -> Simulator {
    built(scenario(&[39], hand, &[]))
}

fn fight_with_draw(hand: &[(&str, u8)], draw: &[(&str, u8)]) -> Simulator {
    built(scenario(&[39], hand, draw))
}

/// The same fight with two beetles, for the cards that sweep the whole side.
fn crowd(hand: &[(&str, u8)]) -> Simulator {
    built(scenario(&[39, 39], hand, &[]))
}

fn built(scenario: ScenarioBuilder) -> Simulator {
    scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

/// One beetle per entry of `enemies`, each standing on the HP it names, with
/// the hand and the draw pile the test asks for. The counters every one of
/// these tests reads are pinned so a roll never moves under a number.
fn scenario(enemies: &[i32], hand: &[(&str, u8)], draw: &[(&str, u8)]) -> ScenarioBuilder {
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .played_as(id("CHARACTER.NECROBINDER"))
        .player_hp(70, 70)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatTargets, 0)
        .rng_counter(RunStream::CombatCardSelection, 0);
    for (index, hp) in enemies.iter().enumerate() {
        let slot = u64::try_from(index).expect("small index") + 1;
        scenario = scenario.enemy(
            slot,
            id("MONSTER.SHRINKER_BEETLE"),
            *hp,
            39,
            "SHRINKER_MOVE",
        );
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

/// The pet standing behind the player, if a summon has stood one up at all.
fn pet(simulator: &Simulator) -> Option<&sts2_engine::Creature> {
    simulator
        .state()
        .combat
        .as_ref()?
        .creatures
        .iter()
        .find(|creature| creature.model_id == id("MONSTER.OSTY"))
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

fn doom_on(simulator: &Simulator, combat_id: u64) -> i32 {
    power_amount(simulator, combat_id, "POWER.DOOM_POWER")
}

// --- the cards written around Doom ---------------------------------------

/// A scourge dooms whatever it is aimed at and then draws; the upgrade raises
/// both the Doom and the number of cards.
#[test]
fn scourge_dooms_what_it_is_aimed_at_and_draws() {
    for (upgrade, doom, drawn) in [(0, 13, 1), (1, 16, 2)] {
        let mut simulator = fight_with_draw(
            &[("CARD.SCOURGE", upgrade)],
            &[
                ("CARD.DEFEND_NECROBINDER", 0),
                ("CARD.DEFEND_NECROBINDER", 0),
            ],
        );
        play(&mut simulator, "CARD.SCOURGE");
        assert_eq!(doom_on(&simulator, 1), doom, "at level {upgrade}");
        assert_eq!(pile(&simulator, PileName::Hand).len(), drawn);
    }
}

/// A negative pulse works up block first and then dooms the whole enemy side.
/// The upgrade raises both numbers.
#[test]
fn negative_pulse_blocks_and_dooms_everything() {
    for (upgrade, block, doom) in [(0, 5, 7), (1, 6, 11)] {
        let mut simulator = crowd(&[("CARD.NEGATIVE_PULSE", upgrade)]);
        play(&mut simulator, "CARD.NEGATIVE_PULSE");
        assert_eq!(player(&simulator).block, block, "at level {upgrade}");
        assert_eq!(doom_on(&simulator, 1), doom, "at level {upgrade}");
        assert_eq!(doom_on(&simulator, 2), doom, "at level {upgrade}");
    }
}

/// A deathbringer dooms the whole enemy side and shakes it. Only the Doom
/// grows with the upgrade; the weakness stays at one.
#[test]
fn deathbringer_dooms_and_weakens_everything() {
    for (upgrade, doom) in [(0, 21), (1, 26)] {
        let mut simulator = crowd(&[("CARD.DEATHBRINGER", upgrade)]);
        play(&mut simulator, "CARD.DEATHBRINGER");
        for enemy in [1, 2] {
            assert_eq!(doom_on(&simulator, enemy), doom, "at level {upgrade}");
            assert_eq!(
                power_amount(&simulator, enemy, "POWER.WEAK_POWER"),
                1,
                "at level {upgrade}"
            );
        }
    }
}

/// The end of days dooms the whole enemy side and then collects on it where it
/// stands, rather than waiting for the turn to end: everything carrying at
/// least as much Doom as it has HP left goes down inside the play, and
/// everything else is left standing with the Doom on it.
#[test]
fn end_of_days_dooms_everything_and_collects_at_once() {
    for (upgrade, doom) in [(0, 29), (1, 37)] {
        let mut simulator = built(scenario(&[20, 39], &[("CARD.END_OF_DAYS", upgrade)], &[]));
        play(&mut simulator, "CARD.END_OF_DAYS");
        assert_eq!(
            hp_of(&simulator, 1),
            None,
            "twenty is less than the Doom it took, at level {upgrade}"
        );
        assert_eq!(hp_of(&simulator, 2), Some(39));
        assert_eq!(doom_on(&simulator, 2), doom, "at level {upgrade}");
    }
}

/// No escape is worth ten, and five more for every whole ten of Doom the target
/// is already carrying. The upgrade raises the ten it starts from and leaves the
/// five alone.
#[test]
fn no_escape_is_worth_more_for_the_doom_already_there() {
    for (upgrade, base) in [(0, 10), (1, 15)] {
        let mut simulator = fight(&[("CARD.NO_ESCAPE", upgrade)]);
        play(&mut simulator, "CARD.NO_ESCAPE");
        assert_eq!(
            doom_on(&simulator, 1),
            base,
            "nothing to count, at level {upgrade}"
        );

        // Twenty-five is two whole tens, so the play is worth ten more than it
        // would be on a clean target.
        let mut simulator = built(scenario(&[39], &[("CARD.NO_ESCAPE", upgrade)], &[]).power(
            1,
            id("POWER.DOOM_POWER"),
            25,
            PowerType::Debuff,
        ));
        play(&mut simulator, "CARD.NO_ESCAPE");
        assert_eq!(doom_on(&simulator, 1), 25 + base + 10, "at level {upgrade}");
    }
}

/// Time's up swings for exactly the Doom its target is carrying, and for
/// nothing at all against a target carrying none. Its upgrade moves a keyword
/// rather than a number: the card is kept through the flush.
#[test]
fn times_up_is_worth_exactly_the_doom_it_finds() {
    for upgrade in [0, 1] {
        let mut simulator = fight(&[("CARD.TIMES_UP", upgrade)]);
        play(&mut simulator, "CARD.TIMES_UP");
        assert_eq!(
            hp_of(&simulator, 1),
            Some(39),
            "no Doom is no damage, at level {upgrade}"
        );
        assert_eq!(pile(&simulator, PileName::Exhaust), ["CARD.TIMES_UP"]);

        let mut simulator = built(scenario(&[39], &[("CARD.TIMES_UP", upgrade)], &[]).power(
            1,
            id("POWER.DOOM_POWER"),
            12,
            PowerType::Debuff,
        ));
        play(&mut simulator, "CARD.TIMES_UP");
        assert_eq!(hp_of(&simulator, 1), Some(39 - 12), "at level {upgrade}");
    }
}

/// The upgraded time's up keeps itself in hand through the end of the turn; the
/// base card is put down with the rest of the hand. The draw pile is stocked
/// deeper than a hand so the next draw never shuffles the discard pile back in
/// and hands the card its owner just put down straight back.
#[test]
fn the_upgraded_times_up_is_kept_through_the_flush() {
    let filler: Vec<(&str, u8)> = std::iter::repeat_n(("CARD.DEFEND_NECROBINDER", 0), 6).collect();
    for (upgrade, kept) in [(0, false), (1, true)] {
        let mut simulator = fight_with_draw(&[("CARD.TIMES_UP", upgrade)], &filler);
        end_turn(&mut simulator);
        assert_eq!(
            pile(&simulator, PileName::Hand).contains(&"CARD.TIMES_UP".to_owned()),
            kept,
            "at level {upgrade}"
        );
    }
}

/// A blight strike swings and then dooms its target for exactly what the blow
/// came to.
#[test]
fn blight_strike_dooms_for_what_the_blow_came_to() {
    for (upgrade, damage) in [(0, 8), (1, 10)] {
        let mut simulator = fight(&[("CARD.BLIGHT_STRIKE", upgrade)]);
        play(&mut simulator, "CARD.BLIGHT_STRIKE");
        assert_eq!(hp_of(&simulator, 1), Some(39 - damage));
        assert_eq!(doom_on(&simulator, 1), damage, "at level {upgrade}");
    }
}

// --- the powers she lays down --------------------------------------------

/// The powers whose whole body is one application on their owner, and what each
/// of them is worth at both levels.
#[test]
fn the_plain_powers_lay_down_what_they_say() {
    for (model_id, power, base, upgraded) in [
        ("CARD.COUNTDOWN", "POWER.COUNTDOWN_POWER", 6, 9),
        ("CARD.DANSE_MACABRE", "POWER.DANSE_MACABRE_POWER", 4, 6),
        ("CARD.DEVOUR_LIFE", "POWER.DEVOUR_LIFE_POWER", 1, 2),
        ("CARD.HAUNT", "POWER.HAUNT_POWER", 6, 8),
        ("CARD.SHROUD", "POWER.SHROUD_POWER", 2, 3),
        ("CARD.SPIRIT_OF_ASH", "POWER.SPIRIT_OF_ASH_POWER", 4, 5),
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

/// A neurosurge is free, pays three energy and two cards out at once, and
/// leaves behind the power that will doom its own owner for it. Only the energy
/// grows with the upgrade.
#[test]
fn neurosurge_pays_energy_and_cards_before_it_dooms_its_owner() {
    for (upgrade, gained) in [(0, 3), (1, 4)] {
        let mut simulator = fight_with_draw(
            &[("CARD.NEUROSURGE", upgrade)],
            &[
                ("CARD.DEFEND_NECROBINDER", 0),
                ("CARD.DEFEND_NECROBINDER", 0),
            ],
        );
        let before = energy(&simulator);
        play(&mut simulator, "CARD.NEUROSURGE");
        assert_eq!(energy(&simulator), before + gained, "at level {upgrade}");
        assert_eq!(pile(&simulator, PileName::Hand).len(), 2);
        assert_eq!(power_amount(&simulator, 0, "POWER.NEUROSURGE_POWER"), 3);
    }
}

// --- the cards that read the state they land in --------------------------

/// A pull from below lands once for every play its owner has finished this
/// fight with an ethereal card, and so not at all before the first one.
#[test]
fn pull_from_below_lands_once_per_ethereal_play() {
    for (upgrade, damage) in [(0, 5), (1, 7)] {
        let mut simulator = fight(&[("CARD.PULL_FROM_BELOW", upgrade)]);
        play(&mut simulator, "CARD.PULL_FROM_BELOW");
        assert_eq!(
            hp_of(&simulator, 1),
            Some(39),
            "no ethereal play is no hit, at level {upgrade}"
        );

        // A defy is ethereal, so two of them are two hits behind the pull.
        let mut simulator = fight(&[
            ("CARD.DEFY", 0),
            ("CARD.DEFY", 0),
            ("CARD.PULL_FROM_BELOW", upgrade),
        ]);
        play(&mut simulator, "CARD.DEFY");
        play(&mut simulator, "CARD.DEFY");
        play(&mut simulator, "CARD.PULL_FROM_BELOW");
        assert_eq!(
            hp_of(&simulator, 1),
            Some(39 - 2 * damage),
            "at level {upgrade}"
        );
    }
}

/// A soul storm is worth nine, and two more for every Soul that has burned up
/// this fight. The upgrade raises what a Soul is worth rather than the nine.
#[test]
fn soul_storm_grows_with_the_souls_that_burned() {
    for (upgrade, per) in [(0, 2), (1, 3)] {
        let mut simulator = fight(&[("CARD.SOUL_STORM", upgrade)]);
        play(&mut simulator, "CARD.SOUL_STORM");
        assert_eq!(
            hp_of(&simulator, 1),
            Some(39 - 9),
            "an empty exhaust pile is the base, at level {upgrade}"
        );

        let mut simulator = built(
            scenario(&[39], &[("CARD.SOUL_STORM", upgrade)], &[])
                .card(PileName::Exhaust, 90, card("CARD.SOUL", 0), 0)
                .card(PileName::Exhaust, 91, card("CARD.SOUL", 0), 0)
                .card(PileName::Exhaust, 92, card("CARD.SOUL", 0), 0),
        );
        play(&mut simulator, "CARD.SOUL_STORM");
        assert_eq!(
            hp_of(&simulator, 1),
            Some(39 - (9 + 3 * per)),
            "at level {upgrade}"
        );
    }
}

// --- the pet -------------------------------------------------------------

/// A bodyguard stands the pet up on exactly what it summons for, which is what
/// the cards below are measured against.
#[test]
fn a_bodyguard_stands_the_pet_up_on_what_it_summons_for() {
    for (upgrade, summoned) in [(0, 5), (1, 7)] {
        let mut simulator = fight(&[("CARD.BODYGUARD", upgrade)]);
        assert!(pet(&simulator).is_none(), "no pet before the summon");
        play(&mut simulator, "CARD.BODYGUARD");
        let pet = pet(&simulator).expect("the summon stood one up");
        assert_eq!((pet.current_hp, pet.max_hp), (summoned, summoned));
    }
}

/// Bone shards throw the pet at the whole enemy side, block, and put the pet
/// down. With no pet standing the card does nothing at all, block included,
/// which is what a second play — or a replay of the first — meets once the
/// first has spent the pet. Seed `KCFCP0TAE9`, record 1749, where a hidden
/// gem's three replays each blocked for nine over a pet already gone.
#[test]
fn bone_shards_do_nothing_at_all_without_a_pet() {
    let mut simulator = crowd(&[
        ("CARD.BODYGUARD", 0),
        ("CARD.BONE_SHARDS", 0),
        ("CARD.BONE_SHARDS", 0),
    ]);
    play(&mut simulator, "CARD.BODYGUARD");
    play(&mut simulator, "CARD.BONE_SHARDS");
    assert_eq!(hp_of(&simulator, 1), Some(30));
    assert_eq!(hp_of(&simulator, 2), Some(30));
    assert_eq!(player(&simulator).block, 9);
    assert_eq!(pet(&simulator).expect("the corpse stays").current_hp, 0);

    play(&mut simulator, "CARD.BONE_SHARDS");
    assert_eq!(hp_of(&simulator, 1), Some(30), "no pet to throw");
    assert_eq!(hp_of(&simulator, 2), Some(30), "no pet to throw");
    assert_eq!(player(&simulator).block, 9, "and no block either");
}

/// A sweep that wins the fight still puts the pet down, while the block the
/// card would have given after it is not given over a won fight. Seed
/// `KCFCP0TAE9`, record 541.
#[test]
fn bone_shards_that_win_the_fight_still_put_the_pet_down() {
    let mut simulator = built(scenario(
        &[9],
        &[("CARD.BODYGUARD", 0), ("CARD.BONE_SHARDS", 0)],
        &[],
    ));
    play(&mut simulator, "CARD.BODYGUARD");
    play(&mut simulator, "CARD.BONE_SHARDS");
    assert!(
        simulator
            .state()
            .combat
            .as_ref()
            .is_some_and(|combat| combat.has_ended),
        "the sweep won the fight"
    );
    assert_eq!(pet(&simulator).expect("the corpse stays").current_hp, 0);
    assert_eq!(player(&simulator).block, 0);
}

/// A pen nib doubles its owner's tenth attack whoever lands it: a blow the pet
/// swings for a card of its owner's is doubled like any other. Seed
/// `XGTTW8TTEQ`, record 1752, where a rattle played on a nib standing at nine
/// killed a knight the engine left alive.
#[test]
fn a_pen_nib_doubles_a_blow_the_pet_swings() {
    let poke_with_the_nib_at = |attacks_played: i32| {
        let mut nib = sts2_engine::RelicInstance::new(id("RELIC.PEN_NIB"));
        nib.counter = attacks_played;
        let mut simulator = built(
            scenario(&[300], &[("CARD.BODYGUARD", 0), ("CARD.POKE", 0)], &[])
                .relic_instances(vec![nib]),
        );
        play(&mut simulator, "CARD.BODYGUARD");
        let before = hp_of(&simulator, 1).expect("the enemy stands");
        play(&mut simulator, "CARD.POKE");
        before - hp_of(&simulator, 1).expect("the enemy stands")
    };
    let plain = poke_with_the_nib_at(0);
    assert!(plain > 0, "the poke lands");
    assert_eq!(poke_with_the_nib_at(9), plain * 2, "the tenth attack");
}

/// A right hand hand comes back from the discard pile behind a play that paid
/// two energy or more, and what a play paid is what it paid as it began: a
/// flatten makes itself free for the turn once the pet has swung, inside its
/// own play, and is still a two-energy play. Seed `XGTTW8TTEQ`, record 2036.
#[test]
fn a_right_hand_hand_answers_what_the_play_paid_not_what_the_card_costs_now() {
    let mut simulator = built(
        scenario(&[300], &[("CARD.BODYGUARD", 0), ("CARD.FLATTEN", 0)], &[]).card(
            PileName::Discard,
            50,
            card("CARD.RIGHT_HAND_HAND", 0),
            0,
        ),
    );
    play(&mut simulator, "CARD.BODYGUARD");
    let before = energy(&simulator);
    play(&mut simulator, "CARD.FLATTEN");
    assert_eq!(before - energy(&simulator), 2, "the flatten paid two");
    assert!(
        pile(&simulator, PileName::Hand).contains(&"CARD.RIGHT_HAND_HAND".to_owned()),
        "and the hand came back behind it"
    );
}

/// A squeeze swings the pet for twenty-five, and five more for every other card
/// of the fight that swings the pet too. The card being played is not one of
/// them, so a squeeze standing alone is worth the base. The upgrade raises both
/// numbers.
#[test]
fn squeeze_counts_every_other_card_that_swings_the_pet() {
    for (upgrade, base, per) in [(0, 25, 5), (1, 30, 6)] {
        let mut simulator = fight(&[("CARD.BODYGUARD", 0), ("CARD.SQUEEZE", upgrade)]);
        play(&mut simulator, "CARD.BODYGUARD");
        play(&mut simulator, "CARD.SQUEEZE");
        assert_eq!(
            hp_of(&simulator, 1),
            Some(39 - base),
            "nothing else swings the pet, at level {upgrade}"
        );

        // A poke carries the pet's own tag, and it counts wherever in the fight
        // it is standing.
        let mut simulator = fight_with_draw(
            &[("CARD.BODYGUARD", 0), ("CARD.SQUEEZE", upgrade)],
            &[("CARD.POKE", 0)],
        );
        play(&mut simulator, "CARD.BODYGUARD");
        play(&mut simulator, "CARD.SQUEEZE");
        assert_eq!(
            hp_of(&simulator, 1),
            Some(39 - (base + per)),
            "at level {upgrade}"
        );
    }
}

/// A sacrifice puts the pet down and stands its owner behind twice what the pet
/// was worth. The block is worked out from the pet's maximum before the kill,
/// because after it there is nothing left to read. Its upgrade makes it free
/// rather than bigger.
#[test]
fn sacrifice_puts_the_pet_down_for_twice_its_size() {
    for (bodyguard, summoned) in [(0, 5), (1, 7)] {
        for (upgrade, cost) in [(0, 1), (1, 0)] {
            let mut simulator =
                fight(&[("CARD.BODYGUARD", bodyguard), ("CARD.SACRIFICE", upgrade)]);
            play(&mut simulator, "CARD.BODYGUARD");
            let before = energy(&simulator);
            play(&mut simulator, "CARD.SACRIFICE");
            assert_eq!(
                player(&simulator).block,
                2 * summoned,
                "a pet of {summoned} at level {upgrade}"
            );
            assert_eq!(
                pet(&simulator)
                    .expect("the corpse never leaves the fight")
                    .current_hp,
                0
            );
            assert_eq!(before - energy(&simulator), cost, "at level {upgrade}");
        }
    }
}

/// Every one of the pet's cards is guarded on the pet standing, and a play made
/// with no pet issues nothing at all: the card is still spent and the energy is
/// still paid, and that is the whole of what happens.
#[test]
fn with_no_pet_standing_the_pet_cards_do_nothing() {
    let mut simulator = fight(&[("CARD.SQUEEZE", 0)]);
    let before = energy(&simulator);
    play(&mut simulator, "CARD.SQUEEZE");
    assert_eq!(hp_of(&simulator, 1), Some(39), "there was nothing to swing");
    assert_eq!(
        before - energy(&simulator),
        3,
        "the play was still paid for"
    );
    assert_eq!(pile(&simulator, PileName::Discard), ["CARD.SQUEEZE"]);

    let mut simulator = fight(&[("CARD.SACRIFICE", 0)]);
    play(&mut simulator, "CARD.SACRIFICE");
    assert_eq!(
        player(&simulator).block,
        0,
        "twice nothing is what a missing pet is worth"
    );
    assert!(pet(&simulator).is_none());
}

// --- the cards written around a history the fight keeps ------------------

/// A death march is worth its base and the extra for every card drawn this
/// turn that the hand draw did not deal. The scenario opens mid-turn with
/// nothing drawn, so a draw made by another card is the only thing that counts.
#[test]
fn a_death_march_grows_with_the_cards_drawn_outside_the_hand_draw() {
    for (upgrade, base, per) in [(0, 8, 4), (1, 9, 6)] {
        let mut simulator = fight_with_draw(
            &[("CARD.DEATH_MARCH", upgrade), ("CARD.SCOURGE", 0)],
            &[
                ("CARD.DEFEND_NECROBINDER", 0),
                ("CARD.DEFEND_NECROBINDER", 0),
            ],
        );
        let mut plain = fight_with_draw(&[("CARD.DEATH_MARCH", upgrade)], &[]);
        play(&mut plain, "CARD.DEATH_MARCH");
        assert_eq!(hp_of(&plain, 1), Some(39 - base), "at level {upgrade}");

        // A scourge draws one card, which the hand draw did not deal.
        play(&mut simulator, "CARD.SCOURGE");
        play(&mut simulator, "CARD.DEATH_MARCH");
        assert_eq!(
            hp_of(&simulator, 1),
            Some(39 - base - per),
            "at level {upgrade}"
        );
    }
}

/// A death's door gains its block once, and three times over where its owner
/// has worked a Doom onto anything this turn.
#[test]
fn a_deaths_door_gains_its_block_three_times_after_a_doom() {
    for (upgrade, block) in [(0, 6), (1, 7)] {
        let mut plain = fight(&[("CARD.DEATHS_DOOR", upgrade)]);
        play(&mut plain, "CARD.DEATHS_DOOR");
        assert_eq!(player(&plain).block, block, "at level {upgrade}");

        let mut doomed = fight_with_draw(
            &[("CARD.DEATHS_DOOR", upgrade), ("CARD.SCOURGE", 0)],
            &[("CARD.DEFEND_NECROBINDER", 0)],
        );
        play(&mut doomed, "CARD.SCOURGE");
        play(&mut doomed, "CARD.DEATHS_DOOR");
        assert_eq!(
            doomed
                .state()
                .combat
                .as_ref()
                .unwrap()
                .player
                .piles
                .hand
                .len(),
            1
        );
        assert_eq!(player(&doomed).block, block * 3, "at level {upgrade}");
    }
}

/// A dredge takes up to three cards back out of the discard pile, and the
/// count it asks for is what the hand has room for.
#[test]
fn a_dredge_takes_what_the_hand_has_room_for() {
    // Three in the discard and room for all of them: the screen takes them
    // all without asking, because it wants exactly as many as it is offered.
    let mut simulator = fight(&[
        ("CARD.DREDGE", 0),
        ("CARD.STRIKE_NECROBINDER", 0),
        ("CARD.STRIKE_NECROBINDER", 0),
        ("CARD.STRIKE_NECROBINDER", 0),
    ]);
    play(&mut simulator, "CARD.STRIKE_NECROBINDER");
    play(&mut simulator, "CARD.STRIKE_NECROBINDER");
    play(&mut simulator, "CARD.STRIKE_NECROBINDER");
    assert_eq!(pile(&simulator, PileName::Discard).len(), 3);
    play(&mut simulator, "CARD.DREDGE");
    assert_eq!(pile(&simulator, PileName::Hand).len(), 3);
    assert_eq!(pile(&simulator, PileName::Discard).len(), 0);
    // Exhaust, not discard.
    assert!(pile(&simulator, PileName::Exhaust).contains(&"CARD.DREDGE".to_owned()));
}

/// A dredge over a discard pile of more than a dozen is offered each set of
/// three once, in pile order, and still takes the three in whatever order they
/// were picked: they join the hand in that order. Seed `XGTTW8TTEQ`, record
/// 1125, three of eighteen named against pile order.
#[test]
fn a_dredge_over_a_big_discard_takes_its_cards_in_the_order_they_were_named() {
    let mut scenario = scenario(&[39], &[("CARD.DREDGE", 0)], &[]);
    let discarded = [
        "CARD.STRIKE_NECROBINDER",
        "CARD.DEFEND_NECROBINDER",
        "CARD.SIC_EM",
        "CARD.POKE",
        "CARD.SNAP",
        "CARD.FETCH",
        "CARD.SQUASH",
        "CARD.FLATTEN",
        "CARD.REAP",
        "CARD.DEFY",
        "CARD.UNLEASH",
        "CARD.HIGH_FIVE",
        "CARD.BONE_SHARDS",
    ];
    for (index, model_id) in discarded.iter().enumerate() {
        let card_id = u64::try_from(index).expect("small index") + 1;
        scenario = scenario.card(PileName::Discard, card_id, card(model_id, 0), 0);
    }
    let mut simulator = built(scenario);
    play(&mut simulator, "CARD.DREDGE");
    let named = ["CARD.UNLEASH", "CARD.BONE_SHARDS", "CARD.SIC_EM"];
    let (choice_id, cards) = simulator
        .legal_actions()
        .iter()
        .find_map(|action| match action {
            Action::ChooseCards { choice_id, cards }
                if cards.len() == named.len()
                    && named.iter().all(|model| {
                        cards
                            .iter()
                            .any(|card| card.fingerprint.model_id == id(model))
                    }) =>
            {
                Some((*choice_id, cards.clone()))
            }
            _ => None,
        })
        .expect("the set is on the list");
    assert_eq!(
        cards[0].fingerprint.model_id,
        id("CARD.SIC_EM"),
        "the list carries the set in pile order"
    );
    let in_order: Vec<_> = named
        .iter()
        .map(|model| {
            cards
                .iter()
                .find(|card| card.fingerprint.model_id == id(model))
                .expect("the card is in the set")
                .clone()
        })
        .collect();
    simulator
        .step(Action::ChooseCards {
            choice_id,
            cards: in_order,
        })
        .expect("an order of a set the list carries is an answer");
    assert_eq!(pile(&simulator, PileName::Hand), named);
}

/// A dredge with nothing to make room for opens no screen at all, because a
/// pick of nothing is no pick.
#[test]
fn a_dredge_with_no_room_asks_for_nothing() {
    let mut filler: Vec<(&str, u8)> =
        std::iter::repeat_n(("CARD.DEFEND_NECROBINDER", 0), 9).collect();
    filler.push(("CARD.DREDGE", 0));
    let mut simulator = fight(&filler);
    play(&mut simulator, "CARD.DREDGE");
    // The hand is full to the hand limit, so the cap is nothing and the
    // discard pile is untouched.
    assert!(
        simulator
            .state()
            .combat
            .as_ref()
            .unwrap()
            .suspended_choice
            .is_none()
    );
}

/// An eidolon burns the whole hand and pays a point of Intangible where nine
/// or more cards burned.
#[test]
fn an_eidolon_burns_the_hand_and_pays_intangible_for_nine() {
    for (held, intangible) in [(8, 0), (9, 1)] {
        let mut hand: Vec<(&str, u8)> = vec![("CARD.EIDOLON", 0)];
        hand.extend(std::iter::repeat_n(("CARD.DEFEND_NECROBINDER", 0), held));
        let mut simulator = fight(&hand);
        play(&mut simulator, "CARD.EIDOLON");
        assert_eq!(pile(&simulator, PileName::Hand).len(), 0);
        assert_eq!(
            power_amount(&simulator, 0, "POWER.INTANGIBLE_POWER"),
            intangible,
            "with {held} cards held"
        );
    }
}

/// A fetch swings the pet and draws, but only on its own first play of the
/// turn: a play history the fight keeps per card.
#[test]
fn a_fetch_draws_on_its_own_first_play_of_the_turn() {
    let mut simulator = fight_with_draw(
        &[("CARD.BODYGUARD", 0), ("CARD.FETCH", 0)],
        &[
            ("CARD.DEFEND_NECROBINDER", 0),
            ("CARD.DEFEND_NECROBINDER", 0),
        ],
    );
    play(&mut simulator, "CARD.BODYGUARD");
    let before = pile(&simulator, PileName::Hand).len();
    play(&mut simulator, "CARD.FETCH");
    // The play drew one, so the hand is one longer for it even though the
    // fetch itself left.
    assert_eq!(pile(&simulator, PileName::Hand).len(), before);
    assert_eq!(pile(&simulator, PileName::Draw).len(), 1);
}

/// The pet's swing is what a fetch is worth, and the upgrade doubles it.
#[test]
fn a_fetch_swings_the_pet_for_three() {
    for (upgrade, damage) in [(0, 3), (1, 6)] {
        let mut simulator = fight(&[("CARD.BODYGUARD", 0), ("CARD.FETCH", upgrade)]);
        play(&mut simulator, "CARD.BODYGUARD");
        let hp = hp_of(&simulator, 1).expect("the beetle is standing");
        play(&mut simulator, "CARD.FETCH");
        assert_eq!(
            hp_of(&simulator, 1),
            Some(hp - damage),
            "at level {upgrade}"
        );
    }
}

/// A flatten writes itself free for the turn as soon as the pet has swung, and
/// the swing it answers is any of the pet's.
#[test]
fn a_flatten_is_free_once_the_pet_has_swung() {
    let mut simulator = fight(&[("CARD.BODYGUARD", 0), ("CARD.POKE", 0), ("CARD.FLATTEN", 0)]);
    let cost = |simulator: &Simulator| {
        simulator
            .state()
            .combat
            .as_ref()
            .unwrap()
            .player
            .piles
            .hand
            .iter()
            .find(|card| card.fingerprint.model_id == id("CARD.FLATTEN"))
            .map(|card| card.energy_cost)
    };
    assert_eq!(cost(&simulator), Some(2));
    play(&mut simulator, "CARD.BODYGUARD");
    assert_eq!(cost(&simulator), Some(2), "a summon is not a swing");
    play(&mut simulator, "CARD.POKE");
    assert_eq!(cost(&simulator), Some(0));
}

/// A flatten swings the pet for twelve, sixteen upgraded.
#[test]
fn a_flatten_swings_the_pet_hard() {
    for (upgrade, damage) in [(0, 12), (1, 16)] {
        let mut simulator = fight(&[("CARD.BODYGUARD", 0), ("CARD.FLATTEN", upgrade)]);
        play(&mut simulator, "CARD.BODYGUARD");
        let hp = hp_of(&simulator, 1).expect("the beetle is standing");
        play(&mut simulator, "CARD.FLATTEN");
        assert_eq!(
            hp_of(&simulator, 1),
            Some(hp - damage),
            "at level {upgrade}"
        );
    }
}

/// A lethality raises the turn's first attack by half again, and nothing after
/// it. The power is a percentage: fifty at level nought, seventy-five upgraded.
#[test]
fn a_lethality_raises_only_the_turns_first_attack() {
    for (upgrade, percent) in [(0, 50), (1, 75)] {
        let mut simulator = scenario(
            &[200],
            &[
                ("CARD.LETHALITY", upgrade),
                ("CARD.STRIKE_NECROBINDER", 0),
                ("CARD.STRIKE_NECROBINDER", 0),
            ],
            &[],
        )
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
        play(&mut simulator, "CARD.LETHALITY");
        assert_eq!(
            power_amount(&simulator, 0, "POWER.LETHALITY_POWER"),
            percent
        );
        play(&mut simulator, "CARD.STRIKE_NECROBINDER");
        let raised = 6 * (100 + percent) / 100;
        assert_eq!(
            hp_of(&simulator, 1),
            Some(200 - raised),
            "at level {upgrade}"
        );
        play(&mut simulator, "CARD.STRIKE_NECROBINDER");
        assert_eq!(hp_of(&simulator, 1), Some(200 - raised - 6));
    }
}

/// A misery swings and then copies every lasting debuff its target was already
/// carrying onto everything else, with the amount the original carried.
#[test]
fn a_misery_copies_the_targets_debuffs_onto_everything_else() {
    for (upgrade, damage) in [(0, 7), (1, 9)] {
        let mut simulator = crowd(&[("CARD.SCOURGE", 0), ("CARD.MISERY", upgrade)]);
        play(&mut simulator, "CARD.SCOURGE");
        assert_eq!(doom_on(&simulator, 1), 13);
        assert_eq!(doom_on(&simulator, 2), 0);
        play(&mut simulator, "CARD.MISERY");
        assert_eq!(
            hp_of(&simulator, 1),
            Some(39 - damage),
            "at level {upgrade}"
        );
        // The copy carries the amount the original held, and the target keeps
        // its own.
        assert_eq!(doom_on(&simulator, 1), 13);
        assert_eq!(doom_on(&simulator, 2), 13);
    }
}

/// An oblivion marks its target and pays Doom for every card its applier plays
/// afterwards this turn. The play that lands the mark pays nothing for itself,
/// and the mark goes at the end of the turn.
#[test]
fn an_oblivion_pays_doom_for_every_later_play_of_the_turn() {
    for (upgrade, amount) in [(0, 3), (1, 4)] {
        let mut simulator = fight(&[
            ("CARD.OBLIVION", upgrade),
            ("CARD.STRIKE_NECROBINDER", 0),
            ("CARD.STRIKE_NECROBINDER", 0),
        ]);
        play(&mut simulator, "CARD.OBLIVION");
        assert_eq!(power_amount(&simulator, 1, "POWER.OBLIVION_POWER"), amount);
        assert_eq!(doom_on(&simulator, 1), 0, "at level {upgrade}");
        play(&mut simulator, "CARD.STRIKE_NECROBINDER");
        assert_eq!(doom_on(&simulator, 1), amount);
        play(&mut simulator, "CARD.STRIKE_NECROBINDER");
        assert_eq!(doom_on(&simulator, 1), amount * 2);
    }
}

/// A rattle lands once, and once more for every swing the pet has already made
/// this turn.
#[test]
fn a_rattle_lands_once_per_swing_the_pet_has_made() {
    for (upgrade, damage) in [(0, 7), (1, 9)] {
        let mut simulator = scenario(
            &[200],
            &[
                ("CARD.BODYGUARD", 0),
                ("CARD.POKE", 0),
                ("CARD.RATTLE", upgrade),
            ],
            &[],
        )
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
        // The poke is the pet's first swing of the turn, so the rattle lands
        // twice.
        play(&mut simulator, "CARD.BODYGUARD");
        play(&mut simulator, "CARD.POKE");
        let hp = hp_of(&simulator, 1).expect("the beetle is standing");
        play(&mut simulator, "CARD.RATTLE");
        assert_eq!(
            hp_of(&simulator, 1),
            Some(hp - damage * 2),
            "at level {upgrade}"
        );
    }
}

/// A right hand hand comes back out of the discard pile whenever its owner
/// spends two energy on a play.
#[test]
fn a_right_hand_hand_comes_back_for_a_two_energy_play() {
    let mut simulator = fight(&[
        ("CARD.BODYGUARD", 0),
        ("CARD.RIGHT_HAND_HAND", 0),
        ("CARD.DEATHBRINGER", 0),
        ("CARD.STRIKE_NECROBINDER", 0),
    ]);
    play(&mut simulator, "CARD.BODYGUARD");
    play(&mut simulator, "CARD.RIGHT_HAND_HAND");
    assert!(pile(&simulator, PileName::Discard).contains(&"CARD.RIGHT_HAND_HAND".to_owned()));
    // A one-energy play leaves it where it is.
    play(&mut simulator, "CARD.STRIKE_NECROBINDER");
    assert!(pile(&simulator, PileName::Discard).contains(&"CARD.RIGHT_HAND_HAND".to_owned()));
    // A two-energy play calls it back.
    play(&mut simulator, "CARD.DEATHBRINGER");
    assert!(pile(&simulator, PileName::Hand).contains(&"CARD.RIGHT_HAND_HAND".to_owned()));
}

/// A sleight of flesh cuts an enemy every time a lasting debuff its owner
/// works lands on it. The blow is unpowered, so nothing scales it.
#[test]
fn a_sleight_of_flesh_cuts_for_every_lasting_debuff_it_lands() {
    for (upgrade, amount) in [(0, 9), (1, 13)] {
        let mut simulator = scenario(
            &[200],
            &[("CARD.SLEIGHT_OF_FLESH", upgrade), ("CARD.SCOURGE", 0)],
            &[("CARD.DEFEND_NECROBINDER", 0)],
        )
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
        play(&mut simulator, "CARD.SLEIGHT_OF_FLESH");
        assert_eq!(
            power_amount(&simulator, 0, "POWER.SLEIGHT_OF_FLESH_POWER"),
            amount
        );
        play(&mut simulator, "CARD.SCOURGE");
        assert_eq!(doom_on(&simulator, 1), 13);
        assert_eq!(
            hp_of(&simulator, 1),
            Some(200 - amount),
            "at level {upgrade}"
        );
    }
}

/// A transfigure makes one card of the hand a point dearer for the rest of the
/// fight and gives it another run of its own body.
#[test]
fn a_transfigure_makes_one_card_dearer_and_gives_it_a_replay() {
    let mut simulator = fight(&[("CARD.TRANSFIGURE", 0), ("CARD.STRIKE_NECROBINDER", 0)]);
    play(&mut simulator, "CARD.TRANSFIGURE");
    let strike = simulator
        .state()
        .combat
        .as_ref()
        .unwrap()
        .player
        .piles
        .hand
        .iter()
        .find(|card| card.fingerprint.model_id == id("CARD.STRIKE_NECROBINDER"))
        .expect("the strike is still held")
        .clone();
    assert_eq!(strike.energy_cost, 2);
    assert_eq!(strike.replay_count, 1);
    play(&mut simulator, "CARD.STRIKE_NECROBINDER");
    assert_eq!(hp_of(&simulator, 1), Some(39 - 12));
}

/// A veilpiercer swings and stands up the power that makes the next ethereal
/// card free — and the power is spent by the very play it pays for.
#[test]
fn a_veilpiercer_pays_for_the_next_ethereal_card() {
    for (upgrade, damage) in [(0, 10), (1, 13)] {
        let mut simulator = scenario(
            &[200],
            &[("CARD.VEILPIERCER", upgrade), ("CARD.LETHALITY", 0)],
            &[],
        )
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
        play(&mut simulator, "CARD.VEILPIERCER");
        assert_eq!(
            hp_of(&simulator, 1),
            Some(200 - damage),
            "at level {upgrade}"
        );
        assert_eq!(power_amount(&simulator, 0, "POWER.VEILPIERCER_POWER"), 1);
        // A Lethality carries Ethereal, so it is free while the power stands.
        let cost = simulator
            .state()
            .combat
            .as_ref()
            .unwrap()
            .player
            .piles
            .hand
            .iter()
            .find(|card| card.fingerprint.model_id == id("CARD.LETHALITY"))
            .map(|card| card.energy_cost);
        assert_eq!(cost, Some(0));
        let energy_before = energy(&simulator);
        play(&mut simulator, "CARD.LETHALITY");
        assert_eq!(energy(&simulator), energy_before);
        assert_eq!(power_amount(&simulator, 0, "POWER.VEILPIERCER_POWER"), 0);
    }
}

/// A necro mastery summons and then stands the power up that turns every point
/// the pet loses into unblockable damage over the whole enemy side. Only the
/// summon grows with the upgrade.
#[test]
fn necro_mastery_summons_and_then_answers_for_the_pet() {
    for (upgrade, summoned) in [(0, 5), (1, 8)] {
        let mut simulator = crowd(&[("CARD.NECRO_MASTERY", upgrade)]);
        play(&mut simulator, "CARD.NECRO_MASTERY");
        assert_eq!(
            pet(&simulator).expect("the mastery summons first").max_hp,
            summoned
        );
        assert_eq!(power_amount(&simulator, 0, "POWER.NECRO_MASTERY_POWER"), 1);
    }
}

/// A reaper form is one stack either way; the upgrade only keeps the card in
/// hand.
#[test]
fn a_reaper_form_is_one_stack_and_an_upgrade_that_keeps_itself() {
    for upgrade in [0, 1] {
        let mut simulator = fight(&[("CARD.REAPER_FORM", upgrade)]);
        play(&mut simulator, "CARD.REAPER_FORM");
        assert_eq!(power_amount(&simulator, 0, "POWER.REAPER_FORM_POWER"), 1);
    }
    // The upgraded form keeps itself through the flush; the plain one does not.
    let filler: Vec<(&str, u8)> = std::iter::repeat_n(("CARD.DEFEND_NECROBINDER", 0), 6).collect();
    for (upgrade, kept) in [(0, false), (1, true)] {
        let mut simulator = fight_with_draw(&[("CARD.REAPER_FORM", upgrade)], &filler);
        end_turn(&mut simulator);
        assert_eq!(
            pile(&simulator, PileName::Hand).contains(&"CARD.REAPER_FORM".to_owned()),
            kept,
            "at level {upgrade}"
        );
    }
}

/// Sic 'em swings the pet and marks the target, and the mark is worked on
/// whether or not the pet was there to swing. Both halves grow with the
/// upgrade.
#[test]
fn sic_em_swings_the_pet_and_marks_whatever_it_was_aimed_at() {
    for (upgrade, damage, mark) in [(0, 5, 3), (1, 6, 4)] {
        let mut simulator = fight(&[("CARD.SIC_EM", upgrade)]);
        play(&mut simulator, "CARD.SIC_EM");
        assert_eq!(
            hp_of(&simulator, 1),
            Some(39),
            "no pet, no blow, at level {upgrade}"
        );
        assert_eq!(power_amount(&simulator, 1, "POWER.SIC_EM_POWER"), mark);

        let mut simulator = fight(&[("CARD.BODYGUARD", 0), ("CARD.SIC_EM", upgrade)]);
        play(&mut simulator, "CARD.BODYGUARD");
        play(&mut simulator, "CARD.SIC_EM");
        assert_eq!(hp_of(&simulator, 1), Some(39 - damage));
        assert_eq!(power_amount(&simulator, 1, "POWER.SIC_EM_POWER"), mark);
        // The swing comes first and the mark after it, so the card's own blow
        // is not one the mark can pay for: the pet is exactly what the
        // bodyguard summoned.
        assert_eq!(pet(&simulator).expect("the pet is standing").max_hp, 5);
    }
}

/// An eradicate lands as many times as the energy its own X swallowed.
#[test]
fn an_eradicate_swings_once_per_point_of_energy() {
    let mut simulator = scenario(&[200], &[("CARD.ERADICATE", 0)], &[])
        .energy(3, 3)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.ERADICATE");
    assert_eq!(hp_of(&simulator, 1), Some(200 - 33));

    let mut upgraded = scenario(&[200], &[("CARD.ERADICATE", 1)], &[])
        .energy(2, 2)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut upgraded, "CARD.ERADICATE");
    assert_eq!(hp_of(&upgraded, 1), Some(200 - 28));
}

/// A drain power swings and then upgrades a couple of cards of the discard
/// pile, taken at random on the combat card-selection stream.
#[test]
fn a_drain_power_upgrades_two_cards_of_the_discard_pile() {
    let mut simulator = scenario(&[200], &[("CARD.DRAIN_POWER", 0)], &[])
        .card(PileName::Discard, 20, card("CARD.STRIKE_NECROBINDER", 0), 0)
        .card(PileName::Discard, 21, card("CARD.DEFEND_NECROBINDER", 0), 0)
        .card(PileName::Discard, 22, card("CARD.STRIKE_NECROBINDER", 0), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.DRAIN_POWER");
    assert_eq!(hp_of(&simulator, 1), Some(190));
    let upgraded = simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles
        .discard
        .iter()
        .filter(|held| held.fingerprint.upgrade_level > 0)
        .count();
    assert_eq!(upgraded, 2);
}

/// The scythe grows every time it is played, on the fight's copy and on the run
/// deck's own card alike.
#[test]
fn the_scythe_grows_on_the_deck_card_as_well_as_the_copy() {
    let mut simulator = scenario(&[200], &[("CARD.THE_SCYTHE", 0)], &[])
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.THE_SCYTHE");
    assert_eq!(hp_of(&simulator, 1), Some(200 - 13));
    let deck = &simulator.state().run_player.deck[0];
    assert_eq!(
        deck.properties
            .get("CurrentDamage")
            .and_then(serde_json::Value::as_i64),
        Some(17)
    );
    assert_eq!(
        deck.properties
            .get("IncreasedDamage")
            .and_then(serde_json::Value::as_i64),
        Some(4)
    );
}

/// A melancholy gets a point cheaper for the rest of the fight every time
/// anything goes down, and it is the card in the pile that listens.
#[test]
fn a_melancholy_gets_cheaper_every_time_something_dies() {
    let mut simulator = scenario(
        &[1, 39],
        &[("CARD.MELANCHOLY", 0), ("CARD.STRIKE_NECROBINDER", 0)],
        &[],
    )
    .build(sts2_content::standard_registry())
    .expect("the scenario builds");
    let cost_of = |simulator: &Simulator| {
        simulator
            .state()
            .combat
            .as_ref()
            .expect("the fight is standing")
            .player
            .piles
            .hand
            .iter()
            .find(|held| held.fingerprint.model_id == id("CARD.MELANCHOLY"))
            .map(|held| held.energy_cost)
    };
    assert_eq!(cost_of(&simulator), Some(3));
    play(&mut simulator, "CARD.STRIKE_NECROBINDER");
    assert_eq!(hp_of(&simulator, 1), None);
    assert_eq!(cost_of(&simulator), Some(2));
    play(&mut simulator, "CARD.MELANCHOLY");
    assert_eq!(player(&simulator).block, 13);
}

// --- her two ancients ----------------------------------------------------

/// A call of the void stocks the hand with one card of its owner's own pool
/// before every draw, and makes each of them ethereal.
#[test]
fn a_call_of_the_void_deals_an_ethereal_card_before_every_draw() {
    let filler: Vec<(&str, u8)> = std::iter::repeat_n(("CARD.DEFEND_NECROBINDER", 0), 6).collect();
    let mut simulator = scenario(&[200], &[("CARD.CALL_OF_THE_VOID", 0)], &filler)
        .rng_counter(RunStream::CombatCardGeneration, 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.CALL_OF_THE_VOID");
    assert_eq!(
        power_amount(&simulator, 0, "POWER.CALL_OF_THE_VOID_POWER"),
        1
    );
    // Nothing is stocked until the next hand draw.
    assert_eq!(pile(&simulator, PileName::Hand).len(), 0);
    end_turn(&mut simulator);
    let hand = &simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles
        .hand;
    // The turn's own draw is five, and the power stocked one more in front of
    // it — carrying Ethereal, which its own pool's cards do not.
    assert_eq!(hand.len(), 6);
    assert_eq!(
        hand.iter().filter(|card| card.ethereal).count(),
        1,
        "the rolled card is the ethereal one"
    );
}

/// A forbidden grimoire puts a card removal on the room's own reward set at
/// the end of every fight, one for each stack it carries.
#[test]
fn a_forbidden_grimoire_pays_a_card_removal_when_the_fight_ends() {
    let mut simulator = necrobinder_fight_in_a_room(&[("CARD.FORBIDDEN_GRIMOIRE", 0)], 6);
    play(&mut simulator, "CARD.FORBIDDEN_GRIMOIRE");
    assert_eq!(
        power_amount(&simulator, 0, "POWER.FORBIDDEN_GRIMOIRE_POWER"),
        1
    );
    play(&mut simulator, "CARD.STRIKE_NECROBINDER");
    assert_eq!(hp_of(&simulator, 1), None, "six is what the beetle had");
    let on_the_room = simulator
        .state()
        .combat
        .as_ref()
        .map(|combat| {
            combat
                .victory_rewards
                .iter()
                .filter(|reward| reward.reward_type == "card_removal")
                .count()
        })
        .unwrap_or_default();
    let offered = simulator
        .state()
        .run
        .as_ref()
        .expect("a run")
        .reward_set
        .iter()
        .flat_map(|set| set.rewards.iter())
        .filter(|reward| reward.fingerprint.reward_type == "card_removal")
        .count();
    assert_eq!(
        on_the_room + offered,
        1,
        "one removal for the one stack it carried"
    );
}

/// A fight standing inside a run's own combat room, so that what a body puts on
/// the room's reward set can be read back once the fight is over.
fn necrobinder_fight_in_a_room(hand: &[(&str, u8)], enemy_hp: i32) -> Simulator {
    let mut with_strike: Vec<(&str, u8)> = hand.to_vec();
    with_strike.push(("CARD.STRIKE_NECROBINDER", 0));
    let combat = scenario(&[enemy_hp], &with_strike, &[])
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
