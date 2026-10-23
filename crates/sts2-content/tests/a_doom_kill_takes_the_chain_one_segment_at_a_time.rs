//! A kill over a list puts its creatures down one at a time, each death
//! settled before the next creature is touched. A Decimillipede's segments
//! show it: a segment that dies while another of its chain still stands is
//! held on its dead move to reattach, and the last of them to die is not. So
//! an End of Days that dooms the whole chain at once holds the first two
//! segments it reaches, and only the third finds the rest of the chain
//! already down. Seeds `A1EKJ8UBU0` at record 1323 and `J3XBU17D5L` at record
//! 1409, where every segment was left on its live move.

use sts2_engine::{Action, CardFingerprint, PileName, PowerType, ScenarioBuilder, Simulator};
use sts2_rng::RunStream;

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

const SEGMENTS: [(&str, &str); 3] = [
    ("MONSTER.DECIMILLIPEDE_SEGMENT_FRONT", "WRITHE_MOVE"),
    ("MONSTER.DECIMILLIPEDE_SEGMENT_MIDDLE", "BULK_MOVE"),
    ("MONSTER.DECIMILLIPEDE_SEGMENT_BACK", "CONSTRICT_MOVE"),
];

/// The three segments, each standing on the HP the test names and carrying
/// its Reattach, against the Necrobinder holding an End of Days and a Book
/// Repair Knife, which heals three for every death the doom kill counts.
fn chain(hp: [i32; 3]) -> Simulator {
    let mut fingerprint = CardFingerprint::base(id("CARD.END_OF_DAYS"));
    fingerprint.floor_added_to_deck = Some(1);
    let mut scenario = ScenarioBuilder::new("DOOMEDCHAIN", id("ENCOUNTER.DECIMILLIPEDE_ELITE"))
        .played_as(id("CHARACTER.NECROBINDER"))
        .player_hp(40, 70)
        .energy(9, 9)
        .turn(1, 1)
        .relics(&[id("RELIC.BOOK_REPAIR_KNIFE")])
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatTargets, 0)
        .card(PileName::Hand, 0, fingerprint, 0);
    for (index, ((model_id, move_state), hp)) in SEGMENTS.iter().zip(hp).enumerate() {
        let slot = u64::try_from(index).expect("small index") + 1;
        scenario = scenario
            .enemy(slot, id(model_id), hp, 60, *move_state)
            .power(slot, id("POWER.REATTACH_POWER"), 25, PowerType::Buff);
    }
    let mut simulator = scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::PlayCard { .. }))
        .cloned()
        .expect("the End of Days is playable");
    simulator.step(action).expect("the play applies");
    simulator
}

fn segment(simulator: &Simulator, slot: u64) -> &sts2_engine::Creature {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .creatures
        .iter()
        .find(|creature| creature.id.get() == slot)
        .expect("a segment keeps its place")
}

#[test]
fn the_whole_chain_doomed_at_once_holds_all_but_the_last_segment() {
    let simulator = chain([20, 20, 20]);
    for slot in [1, 2, 3] {
        assert_eq!(segment(&simulator, slot).current_hp, 0);
    }
    assert_eq!(segment(&simulator, 1).move_state, "DEAD_MOVE");
    assert_eq!(segment(&simulator, 2).move_state, "DEAD_MOVE");
    assert_eq!(
        segment(&simulator, 3).move_state,
        "CONSTRICT_MOVE",
        "the last segment down has no chain left to hold it"
    );
    assert!(
        simulator
            .state()
            .combat
            .as_ref()
            .is_some_and(|combat| combat.has_ended),
        "the whole chain is down"
    );
    // The deaths are counted once the whole list is down, when every segment
    // finds the rest of its chain dead: all three count.
    assert_eq!(simulator.state().run_player.current_hp, 40 + 3 * 3);
}

#[test]
fn segments_doomed_beside_a_standing_one_are_held_and_not_counted() {
    let simulator = chain([20, 20, 60]);
    assert_eq!(segment(&simulator, 1).move_state, "DEAD_MOVE");
    assert_eq!(segment(&simulator, 2).move_state, "DEAD_MOVE");
    assert_eq!(segment(&simulator, 3).current_hp, 60);
    assert_eq!(segment(&simulator, 3).move_state, "CONSTRICT_MOVE");
    assert_eq!(simulator.state().run_player.current_hp, 40);
}
