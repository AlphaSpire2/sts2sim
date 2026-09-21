//! A screen opened inside the player's turn setup does not hold back the rest
//! of the turn start.
//!
//! The build starts the setup as a task of its own and waits for it only until
//! it pauses, so a screen the setup put up is still standing while the side's
//! own turn-start listeners run. Their work lands in front of the player's
//! answer, not behind it. Only the auto-pre-play phase genuinely waits, because
//! it opens by awaiting the setup task.
//!
//! A rampart is the listener that shows it: it puts block up in front of the
//! gunner as the player's turn opens, and a screen standing over the setup must
//! not delay that block until the screen is answered.

use sts2_engine::{Action, CardFingerprint, PileName, PowerType, ScenarioBuilder, Simulator};
use sts2_rng::RunStream;

const SEED: &str = "SETUPSCREEN";

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str, upgrade_level: u8) -> CardFingerprint {
    let mut fingerprint = CardFingerprint::base(id(model_id));
    fingerprint.upgrade_level = upgrade_level;
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

/// The living shield and the gunner it stands in front of, with the shield
/// carrying the rampart that arms the gunner every player turn.
///
/// `tools` gives the player a power that lays a screen out inside the turn
/// setup, which is the thing the rampart must not end up behind.
fn fight(tools: bool) -> Simulator {
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.TURRET_OPERATOR_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .enemy(1, id("MONSTER.LIVING_SHIELD"), 55, 55, "SHIELD_SLAM_MOVE")
        .enemy(2, id("MONSTER.TURRET_OPERATOR"), 41, 41, "UNLOAD_MOVE")
        .power(1, id("POWER.RAMPART_POWER"), 25, PowerType::Buff)
        // A demon form is a listener on the same pass that stands in front of
        // the rampart: the player's own powers are walked before the enemy's.
        // It is what makes the pass stop somewhere rather than nowhere.
        .power(0, id("POWER.DEMON_FORM_POWER"), 2, PowerType::Buff);
    if tools {
        scenario = scenario.power(0, id("POWER.TOOLS_OF_THE_TRADE_POWER"), 1, PowerType::Buff);
    }
    for next_id in 0..5 {
        scenario = scenario.card(PileName::Hand, next_id, card("CARD.STRIKE_IRONCLAD", 0), 0);
    }
    for next_id in 5..10 {
        scenario = scenario.card(PileName::Draw, next_id, card("CARD.STRIKE_IRONCLAD", 0), 0);
    }
    scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
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

/// The gunner is already behind its block while the screen the turn setup put
/// up is still standing.
#[test]
fn a_rampart_arms_the_gunner_in_front_of_a_screen_the_setup_opened() {
    let mut simulator = fight(true);
    end_turn(&mut simulator);

    // The turn opened, the setup laid its screen out, and the player has not
    // answered it yet.
    assert!(
        matches!(
            simulator.decision(),
            sts2_engine::DecisionContext::ChooseCards { .. }
        ),
        "the setup's screen is what the player is being asked about"
    );
    // The pass reached the player's own power, which stands in front of the
    // rampart in the same walk.
    assert_eq!(
        creature(&simulator, 0).power_amount("POWER.STRENGTH_POWER"),
        2,
        "the pass got as far as the player's own listener"
    );
    assert_eq!(
        creature(&simulator, 2).block,
        25,
        "and carried on to the rampart, in front of the screen"
    );
}

/// The same turn with nothing standing over the setup arms the gunner just the
/// same, so the block is the rampart's doing rather than the screen's.
#[test]
fn a_rampart_arms_the_gunner_when_no_screen_stands_at_all() {
    let mut simulator = fight(false);
    end_turn(&mut simulator);

    assert!(
        !matches!(
            simulator.decision(),
            sts2_engine::DecisionContext::ChooseCards { .. }
        ),
        "nothing put a screen up"
    );
    assert_eq!(creature(&simulator, 2).block, 25);
}

/// A screen over the hand offers whatever stands in the hand while it is up —
/// the build hands it the filter, not a list — and a screen the turn setup
/// opened is up while the side's own listeners run. A crossbow conjures its
/// attack into the hand in front of an entropy's answer, so the attack is one of
/// the cards the entropy can be answered with. Seed `2CKB2N4LNB`, record 1802,
/// where the conjured card was the one picked.
#[test]
fn a_card_conjured_in_front_of_a_hand_screen_is_on_it() {
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .played_as(id("CHARACTER.REGENT"))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatCardGeneration, 0)
        .relics(&[id("RELIC.CROSSBOW")])
        .power(0, id("POWER.ENTROPY_POWER"), 1, PowerType::Buff)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE");
    for index in 0..5 {
        scenario = scenario.card(PileName::Draw, index, card("CARD.DEFEND_REGENT", 0), 1);
    }
    let mut simulator = scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    let end_turn = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::EndTurn { .. }))
        .cloned()
        .expect("ending the turn is legal");
    simulator.step(end_turn).expect("the turn ends");

    let hand: Vec<u64> = simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles
        .hand
        .iter()
        .map(|card| card.id.get())
        .collect();
    assert_eq!(hand.len(), 6, "five drawn and the crossbow's attack");
    let offered: Vec<u64> = simulator
        .legal_actions()
        .iter()
        .filter_map(|action| match action {
            Action::ChooseCards { cards, .. } if cards.len() == 1 => Some(cards[0].card_id.get()),
            _ => None,
        })
        .collect();
    assert_eq!(offered, hand, "the entropy's screen offers the whole hand");
}
