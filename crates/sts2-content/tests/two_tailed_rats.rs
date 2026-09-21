//! The two-tailed rats and the room that stands three of them up.
//!
//! No recording walks the Underdocks' rat room, so what is driven here is the
//! whole of what makes it a rat room: the countdown before a rat may call for
//! backup, the free slot the call fills, the count the whole side shares
//! afterwards, and the weights the branch reads all of that through.

use std::collections::BTreeSet;
use std::sync::Arc;

use sts2_engine::{
    Action, CombatSide, ContentRegistry, Creature, PileName, ScenarioBuilder, Simulator,
};
use sts2_rng::RunStream;

const ENCOUNTER: &str = "ENCOUNTER.TWO_TAILED_RATS_NORMAL";
const RAT: &str = "MONSTER.TWO_TAILED_RAT";

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

/// One rat, standing where the room's third slot is and on the move named, with
/// the countdown already at whatever the caller says.
fn one_rat(opener: &str, turns_until_summonable: i32, ascension: u8) -> Simulator {
    rats(&[("third", opener)], turns_until_summonable, ascension, 0)
}

/// The same rat, with the monster-AI stream already read this far, which is how
/// a different branch is rolled each time round.
fn one_rat_rolling(opener: &str, turns_until_summonable: i32, counter: u64) -> Simulator {
    rats(&[("third", opener)], turns_until_summonable, 0, counter)
}

/// Rats in the slots named, each already `turns_until_summonable` from being
/// able to call.
fn rats(
    placed: &[(&str, &str)],
    turns_until_summonable: i32,
    ascension: u8,
    counter: u64,
) -> Simulator {
    let mut scenario = ScenarioBuilder::new("RATTEST", id(ENCOUNTER))
        .player_hp(200, 200)
        .energy(3, 3)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, counter)
        .relics(&[id("RELIC.BURNING_BLOOD")]);
    for (index, (slot, opener)) in placed.iter().enumerate() {
        scenario = scenario
            .enemy(index as u64 + 1, id(RAT), 21, 21, *opener)
            .in_slot(slot)
            .enemy_local("TurnsUntilSummonable", turns_until_summonable);
    }
    for card_id in 0..5 {
        scenario = scenario.card(PileName::Hand, card_id, card("CARD.STRIKE_IRONCLAD"), 1);
    }
    scenario.build(content_at(ascension)).unwrap()
}

fn enemies(simulator: &Simulator) -> Vec<&Creature> {
    simulator
        .state()
        .combat
        .as_ref()
        .unwrap()
        .creatures
        .iter()
        .filter(|creature| creature.side == CombatSide::Enemy)
        .collect()
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

fn damage_of_one_turn(simulator: &mut Simulator) -> i32 {
    let before = player(simulator).current_hp;
    end_turn(simulator);
    before - player(simulator).current_hp
}

fn local(creature: &Creature, name: &str) -> i32 {
    creature.locals.get(name).copied().unwrap_or(0)
}

/// Each of the three moves that is not the call takes one off the countdown,
/// and the call takes none.
#[test]
fn every_move_but_the_call_brings_the_countdown_down() {
    let mut fight = one_rat("SCRATCH_MOVE", 2, 0);
    assert_eq!(local(enemies(&fight)[0], "TurnsUntilSummonable"), 2);
    assert_eq!(damage_of_one_turn(&mut fight), 8, "the scratch is eight");
    assert_eq!(
        local(enemies(&fight)[0], "TurnsUntilSummonable"),
        1,
        "and takes one off the countdown"
    );

    let mut biting = one_rat("DISEASE_BITE_MOVE", 2, 0);
    assert_eq!(damage_of_one_turn(&mut biting), 6, "the bite is six");
    assert_eq!(local(enemies(&biting)[0], "TurnsUntilSummonable"), 1);

    let mut screeching = one_rat("SCREECH_MOVE", 2, 0);
    assert_eq!(damage_of_one_turn(&mut screeching), 0);
    assert_eq!(
        player(&screeching).power_amount("POWER.FRAIL_POWER"),
        1,
        "the screech leaves one frail"
    );
    assert_eq!(local(enemies(&screeching)[0], "TurnsUntilSummonable"), 1);
}

/// While the countdown is still standing the call weighs nothing, so the roll
/// can only ever land on one of the three attacks.
#[test]
fn a_rat_that_has_not_waited_never_calls() {
    for counter in 0..24 {
        // The stream is read a different distance each time, so the branch
        // rolls somewhere different each time.
        let mut fight = one_rat_rolling("SCRATCH_MOVE", 2, counter);
        end_turn(&mut fight);
        assert_ne!(
            enemies(&fight)[0].move_state,
            "CALL_FOR_BACKUP_MOVE",
            "the countdown is still at one after one move"
        );
    }
}

/// Once the countdown has run out the call is three parts in four, so a rat
/// standing alone in a room with free slots calls almost at once.
#[test]
fn a_rat_that_has_waited_calls_for_backup() {
    let mut called = false;
    for counter in 0..24 {
        let mut fight = one_rat_rolling("SCRATCH_MOVE", 1, counter);
        end_turn(&mut fight);
        if enemies(&fight)[0].move_state == "CALL_FOR_BACKUP_MOVE" {
            called = true;
            break;
        }
    }
    assert!(
        called,
        "the call is reachable once the countdown has run out"
    );
}

/// The call brings a rat into the last free slot, and every rat standing —
/// the new one included — takes the shared count one higher.
#[test]
fn a_call_fills_the_last_free_slot_and_bumps_the_shared_count() {
    // The room's own three rats, in the back three slots, so the two free ones
    // are the two in front.
    let mut fight = rats(
        &[
            ("third", "CALL_FOR_BACKUP_MOVE"),
            ("fourth", "SCRATCH_MOVE"),
            ("fifth", "SCRATCH_MOVE"),
        ],
        0,
        0,
        0,
    );
    end_turn(&mut fight);
    let standing = enemies(&fight);
    assert_eq!(standing.len(), 4, "the call brought another rat in");
    assert_eq!(
        standing
            .iter()
            .map(|creature| creature.slot_name.clone())
            .collect::<Vec<_>>(),
        vec![
            Some("second".to_owned()),
            Some("third".to_owned()),
            Some("fourth".to_owned()),
            Some("fifth".to_owned()),
        ],
        "into the last of the free slots, and the side sorts by slot"
    );
    for creature in &standing {
        assert_eq!(
            local(creature, "CallForBackupCount"),
            1,
            "and every rat now carries the same count"
        );
    }
    // The arrival opens on the branch rather than on any written move, and
    // stands its own countdown up at two.
    let arrival = standing
        .iter()
        .find(|creature| creature.slot_name.as_deref() == Some("second"))
        .unwrap();
    assert_eq!(local(arrival, "TurnsUntilSummonable"), 2);
    assert!(
        [
            "SCRATCH_MOVE",
            "DISEASE_BITE_MOVE",
            "SCREECH_MOVE",
            "CALL_FOR_BACKUP_MOVE"
        ]
        .contains(&arrival.move_state.as_str()),
        "and rolled its first move rather than being handed one"
    );
}

/// A call made with every slot taken brings nobody, and still counts.
#[test]
fn a_call_with_no_free_slot_brings_nobody_and_still_counts() {
    let mut fight = rats(
        &[
            ("first", "SCRATCH_MOVE"),
            ("second", "SCRATCH_MOVE"),
            ("third", "CALL_FOR_BACKUP_MOVE"),
            ("fourth", "SCRATCH_MOVE"),
            ("fifth", "SCRATCH_MOVE"),
        ],
        0,
        0,
        0,
    );
    end_turn(&mut fight);
    assert_eq!(enemies(&fight).len(), 5, "there was nowhere to put one");
    for creature in enemies(&fight) {
        assert_eq!(
            local(creature, "CallForBackupCount"),
            1,
            "the count goes up all the same"
        );
    }
}

/// A rat whose side has already called three times never calls again, however
/// many slots are free.
#[test]
fn three_calls_is_all_the_side_ever_makes() {
    for counter in 0..40 {
        let mut scenario = ScenarioBuilder::new("RATTEST", id(ENCOUNTER))
            .player_hp(200, 200)
            .energy(3, 3)
            .turn(1, 1)
            .rng_counter(RunStream::MonsterAi, counter)
            .relics(&[id("RELIC.BURNING_BLOOD")])
            .enemy(1, id(RAT), 21, 21, "SCRATCH_MOVE")
            .in_slot("third")
            .enemy_local("TurnsUntilSummonable", 0)
            .enemy_local("CallForBackupCount", 3);
        for card_id in 0..5 {
            scenario = scenario.card(PileName::Hand, card_id, card("CARD.STRIKE_IRONCLAD"), 1);
        }
        let mut fight = scenario.build(content_at(0)).unwrap();
        end_turn(&mut fight);
        assert_ne!(enemies(&fight)[0].move_state, "CALL_FOR_BACKUP_MOVE");
    }
}

/// While one rat is already telegraphing the call, no other rat may roll it.
#[test]
fn only_one_rat_of_a_side_may_be_about_to_call() {
    for counter in 0..40 {
        // The rat in front takes its turn first and rolls its next move while
        // the one behind it is still telegraphing the call, which is the
        // teammate's next move the branch asks about.
        let mut fight = rats(
            &[("third", "SCRATCH_MOVE"), ("fifth", "CALL_FOR_BACKUP_MOVE")],
            0,
            0,
            counter,
        );
        end_turn(&mut fight);
        let front = enemies(&fight)
            .into_iter()
            .find(|creature| creature.slot_name.as_deref() == Some("third"))
            .expect("the front rat is still standing");
        assert_ne!(
            front.move_state, "CALL_FOR_BACKUP_MOVE",
            "a teammate was already about to call"
        );
    }
}

/// The screech sits on a cooldown of three moves, so it cannot come round again
/// until three others have.
#[test]
fn the_screech_waits_three_moves_before_it_comes_round_again() {
    let mut seen = BTreeSet::new();
    for counter in 0..60 {
        let mut fight = one_rat_rolling("SCREECH_MOVE", 2, counter);
        end_turn(&mut fight);
        let next = enemies(&fight)[0].move_state.clone();
        assert_ne!(next, "SCREECH_MOVE", "the screech is on its own cooldown");
        seen.insert(next);
    }
    assert!(
        seen.contains("SCRATCH_MOVE") && seen.contains("DISEASE_BITE_MOVE"),
        "and the other two attacks both come up"
    );
}

/// Its ascension branches: a bigger rat, a nine scratch and a seven bite.
#[test]
fn the_rat_carries_its_ascension_branches() {
    let ascended = content_at(9);
    let definition = ascended.monster(&id(RAT)).expect("the rat is content");
    assert_eq!(definition.min_initial_hp, 18);
    assert_eq!(definition.max_initial_hp, 22);

    let mut scratching = one_rat("SCRATCH_MOVE", 2, 9);
    assert_eq!(damage_of_one_turn(&mut scratching), 9);
    let mut biting = one_rat("DISEASE_BITE_MOVE", 2, 9);
    assert_eq!(damage_of_one_turn(&mut biting), 7);
}
