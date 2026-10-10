//! A card played automatically runs the whole of its body, including the part
//! behind the attack that won the fight.
//!
//! An automatic play ends by running the card the same way a chosen play does,
//! and the only thing that stops that short after the card's own body is the
//! player's own death. The fight being over is not checked there because it
//! cannot be:
//! the win is settled after the whole action has finished, so that the state
//! stays intact for the rest of the dispatch. Cutting the body off at the
//! killing blow would drop what is behind it in listener order.
//!
//! Stampede rolls an attack out of the hand at the end of its owner's turn and
//! plays it for them. A Cinder is eighteen damage and then a card rolled out
//! of the hand and burned, so a Cinder that finishes the fight is the case:
//! the roll behind the kill is still made, and it is made on the
//! card-selection stream. That counter is what the rest of the run draws
//! from, so a roll left out would put every later card one position off.

use sts2_engine::{Action, CardFingerprint, PileName, PowerType, ScenarioBuilder, Simulator};
use sts2_rng::RunStream;

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str) -> CardFingerprint {
    let mut fingerprint = CardFingerprint::base(id(model_id));
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

/// A beetle, a Stampede, and a hand of one Cinder and two Defends. The Cinder
/// is the only attack in the hand, so it is what Stampede plays; the Defends
/// are what its second half has to roll between.
fn a_stampede_holding_a_cinder(enemy_hp: i32) -> Simulator {
    ScenarioBuilder::new("STAMPEDE", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(0, 3)
        .turn(1, 1)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .power(0, id("POWER.STAMPEDE_POWER"), 1, PowerType::Buff)
        .enemy(
            1,
            id("MONSTER.SHRINKER_BEETLE"),
            enemy_hp,
            39,
            "SHRINKER_MOVE",
        )
        .card(PileName::Hand, 0, card("CARD.CINDER"), 0)
        .card(PileName::Hand, 1, card("CARD.DEFEND_IRONCLAD"), 0)
        .card(PileName::Hand, 2, card("CARD.DEFEND_IRONCLAD"), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

fn end_the_turn(simulator: &mut Simulator) {
    let end_turn = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::EndTurn { .. }))
        .cloned()
        .expect("the turn can be ended");
    simulator.step(end_turn).expect("the turn ends");
}

fn card_selection_rolls(simulator: &Simulator) -> u64 {
    simulator
        .state()
        .rng
        .stream(RunStream::CombatCardSelection)
        .counter()
}

#[test]
fn a_cinder_stampede_plays_into_a_kill_still_rolls_the_card_it_burns() {
    let mut standing = a_stampede_holding_a_cinder(39);
    end_the_turn(&mut standing);
    assert_eq!(
        card_selection_rolls(&standing),
        1,
        "a Cinder the beetle survives rolls once for the card it burns"
    );

    let mut lethal = a_stampede_holding_a_cinder(6);
    end_the_turn(&mut lethal);
    let combat = lethal.state().combat.as_ref().expect("the fight is held");
    assert!(
        combat.has_ended,
        "the Cinder Stampede played killed the beetle outright"
    );
    assert_eq!(
        card_selection_rolls(&lethal),
        1,
        "and the kill does not excuse the roll the rest of its body makes"
    );
}

#[test]
fn a_lethal_auto_play_finishes_the_card_it_played() {
    let mut lethal = a_stampede_holding_a_cinder(6);
    end_the_turn(&mut lethal);

    let piles = &lethal
        .state()
        .combat
        .as_ref()
        .expect("the fight is held")
        .player
        .piles;
    assert!(
        piles.play.is_empty(),
        "the play is finished, so the Cinder is not left standing in the play pile"
    );
    assert!(
        piles.exhaust.is_empty(),
        "and the Defend the roll landed on stays where it is: the roll is read, \
         but the exhaust it feeds refuses while the fight is ending"
    );
}

/// A Stampede holding one Strike, drunk Duplicator, and tenderized: the Strike
/// is played twice as one play, and every pass of it is announced on its own.
fn a_tenderized_stampede_holding_a_duplicated_strike() -> Simulator {
    ScenarioBuilder::new("STAMPEDE", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(0, 3)
        .turn(1, 1)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .power(0, id("POWER.STAMPEDE_POWER"), 1, PowerType::Buff)
        .power(0, id("POWER.DUPLICATION_POWER"), 1, PowerType::Buff)
        .power(0, id("POWER.TENDER_POWER"), 0, PowerType::Debuff)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .card(PileName::Hand, 0, card("CARD.STRIKE_IRONCLAD"), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

fn enemy_hp(simulator: &Simulator) -> i32 {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .creatures
        .iter()
        .find(|creature| creature.id.get() == 1)
        .expect("the beetle is in the fight")
        .current_hp
}

/// Every pass of an automatic play's repeat loop is announced as played
/// before the next pass runs, so a listener on the play changes the second
/// pass. A tenderized player loses a strength for every card played; a Strike
/// that Stampede plays twice under Duplication lands six and then five, not
/// six and six.
#[test]
fn an_auto_play_announces_each_pass_before_the_next_runs() {
    let mut simulator = a_tenderized_stampede_holding_a_duplicated_strike();
    end_the_turn(&mut simulator);
    assert_eq!(enemy_hp(&simulator), 39 - 6 - 5);
}

#[test]
fn stampede_finishes_its_selection_rolls_after_a_lethal_play() {
    for (attacks, stacks, expected_rolls) in [(3, 2, 2), (3, 4, 4), (1, 3, 1)] {
        let mut scenario = ScenarioBuilder::new("STAMPEDE", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .player_hp(80, 80)
            .energy(0, 3)
            .turn(1, 1)
            .rng_counter(RunStream::Shuffle, 0)
            .rng_counter(RunStream::CombatTargets, 0)
            .power(0, id("POWER.STAMPEDE_POWER"), stacks, PowerType::Buff)
            .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 1, 39, "SHRINKER_MOVE");
        for index in 0..attacks {
            scenario = scenario.card(PileName::Hand, index, card("CARD.STRIKE_IRONCLAD"), 0);
        }
        let mut simulator = scenario
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
        end_the_turn(&mut simulator);
        assert!(simulator.state().combat.as_ref().unwrap().has_ended);
        assert_eq!(
            simulator.state().rng.stream(RunStream::Shuffle).counter(),
            expected_rolls,
            "remaining selections still roll; an empty candidate list consumes nothing"
        );
        assert_eq!(
            simulator
                .state()
                .rng
                .stream(RunStream::CombatTargets)
                .counter(),
            1,
            "only the lethal first card is played; later selections never roll a target"
        );
    }
}
