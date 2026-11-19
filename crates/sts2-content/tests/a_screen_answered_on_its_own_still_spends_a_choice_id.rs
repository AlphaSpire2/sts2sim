//! The hand, pile and plain-grid screens reserve their choice id before they
//! ask whether a question is needed, so a screen the build answers on its own
//! still spends one, and an empty one does too. A fight that is already over
//! hands back nothing before any id is reserved.

use sts2_engine::{Action, CardFingerprint, DecisionContext, PileName, ScenarioBuilder, Simulator};
use sts2_rng::RunStream;

const SEED: &str = "CHOICEIDS";

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str) -> CardFingerprint {
    let mut fingerprint = CardFingerprint::base(id(model_id));
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

fn fight(enemy_hp: i32, hand: &[&str], draw: &[&str]) -> Simulator {
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .played_as(id("CHARACTER.SILENT"))
        .player_hp(70, 70)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatTargets, 0)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .enemy(
            1,
            id("MONSTER.SHRINKER_BEETLE"),
            enemy_hp,
            39,
            "SHRINKER_MOVE",
        );
    let mut next_id = 0;
    for (pile, cards) in [(PileName::Hand, hand), (PileName::Draw, draw)] {
        for model_id in cards {
            scenario = scenario.card(pile, next_id, card(model_id), 0);
            next_id += 1;
        }
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

/// The id the next screen would be given.
fn next_choice_id(simulator: &Simulator) -> u64 {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("a fight is standing")
        .next_choice_id
}

fn discarded(simulator: &Simulator) -> Vec<String> {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("a fight is standing")
        .player
        .piles
        .discard
        .iter()
        .map(|card| card.fingerprint.model_id.to_string())
        .collect()
}

#[test]
fn a_discard_with_one_card_to_take_spends_an_id() {
    let mut simulator = fight(39, &["CARD.SURVIVOR", "CARD.SLICE"], &[]);
    let before = next_choice_id(&simulator);
    play(&mut simulator, "CARD.SURVIVOR");
    assert!(
        !matches!(simulator.decision(), DecisionContext::ChooseCards { .. }),
        "the only card is taken without asking"
    );
    assert!(discarded(&simulator).contains(&"CARD.SLICE".to_owned()));
    assert_eq!(next_choice_id(&simulator), before + 1);
}

#[test]
fn a_discard_from_an_empty_hand_spends_an_id() {
    let mut simulator = fight(39, &["CARD.SURVIVOR"], &[]);
    let before = next_choice_id(&simulator);
    play(&mut simulator, "CARD.SURVIVOR");
    assert_eq!(next_choice_id(&simulator), before + 1);
}

#[test]
fn a_discard_that_asks_spends_one_id_and_shows_it() {
    let mut simulator = fight(
        39,
        &["CARD.SURVIVOR", "CARD.SLICE", "CARD.DEFEND_SILENT"],
        &[],
    );
    let before = next_choice_id(&simulator);
    play(&mut simulator, "CARD.SURVIVOR");
    match simulator.decision() {
        DecisionContext::ChooseCards { choice_id, .. } => {
            assert_eq!(choice_id.get(), before);
        }
        other => panic!("two cards to choose from put a screen up, not {other:?}"),
    }
    assert_eq!(next_choice_id(&simulator), before + 1);
}

#[test]
fn a_discard_after_the_killing_blow_spends_nothing() {
    let mut simulator = fight(
        5,
        &["CARD.DAGGER_THROW", "CARD.SLICE", "CARD.DEFEND_SILENT"],
        &["CARD.STRIKE_SILENT"],
    );
    let before = next_choice_id(&simulator);
    play(&mut simulator, "CARD.DAGGER_THROW");
    assert!(
        !matches!(simulator.decision(), DecisionContext::ChooseCards { .. }),
        "a won fight asks for no discard"
    );
    assert_eq!(next_choice_id(&simulator), before);
}
