//! A screen a death listener asks for waits for the play that killed.
//!
//! The build gives every after-death listener a pause point of its own and
//! never waits for it: a listener that asks is put on the player's action
//! queue as an action of its own, and the pass, the kill, and the play all
//! run on. A Gremlin Horn draws as an enemy goes down; with the draw pile
//! empty the draw shuffles, and the shuffle wakes a Stratagem, which asks which
//! of the shuffled cards to take. That question is put to the player once the
//! play has finished, and the Horn's draw stands paused behind the answer.

use sts2_engine::{
    Action, CardFingerprint, DecisionContext, PileName, PowerType, ScenarioBuilder, Simulator,
};
use sts2_rng::RunStream;

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str) -> CardFingerprint {
    let mut fingerprint = CardFingerprint::base(id(model_id));
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

fn pile(simulator: &Simulator, pile: PileName) -> Vec<String> {
    let piles = &simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight stands")
        .player
        .piles;
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

fn energy(simulator: &Simulator) -> i32 {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight stands")
        .player
        .energy
}

/// The play of `model` aimed at the creature with `target`, or at nobody.
fn play(simulator: &mut Simulator, model: &str, target: Option<u64>) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| match action {
            Action::PlayCard { card, target: aim } => {
                card.fingerprint.model_id.to_string() == model
                    && aim.as_ref().map(|aim| aim.combat_id.get()) == target
            }
            _ => false,
        })
        .cloned()
        .unwrap_or_else(|| panic!("{model} is playable: {:?}", simulator.legal_actions()));
    simulator.step(action).expect("the play applies");
}

fn pick_first(simulator: &mut Simulator) {
    let DecisionContext::ChooseCards {
        choice_id,
        candidates,
        ..
    } = simulator.decision().clone()
    else {
        panic!("a card choice stands: {:?}", simulator.decision());
    };
    simulator
        .step(Action::ChooseCards {
            choice_id,
            cards: vec![candidates[0].clone()],
        })
        .expect("the pick applies");
}

/// A horn and a stratagem over an empty draw pile, three defends in the
/// discard, and a strike in hand.
fn horn_and_stratagem(encounter: &str) -> ScenarioBuilder {
    ScenarioBuilder::new("DEATHQUEUE", id(encounter))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .rng_counter(RunStream::Shuffle, 0)
        .relics(&[id("RELIC.GREMLIN_HORN")])
        .power(0, id("POWER.STRATAGEM_POWER"), 1, PowerType::Buff)
        .card(PileName::Hand, 0, card("CARD.STRIKE_IRONCLAD"), 0)
        .card(PileName::Discard, 1, card("CARD.DEFEND_IRONCLAD"), 0)
        .card(PileName::Discard, 2, card("CARD.DEFEND_IRONCLAD"), 0)
        .card(PileName::Discard, 3, card("CARD.DEFEND_IRONCLAD"), 0)
}

fn assert_the_pick_stands_behind_the_finished_play(simulator: &Simulator) {
    let DecisionContext::ChooseCards { candidates, .. } = simulator.decision() else {
        panic!("the stratagem asks: {:?}", simulator.decision());
    };
    assert_eq!(candidates.len(), 3, "the pick is over the shuffled pile");
    assert_eq!(
        energy(simulator),
        3,
        "the strike cost one and the horn paid one back, once"
    );
    assert_eq!(
        pile(simulator, PileName::Hand),
        Vec::<String>::new(),
        "the horn's draw stands paused behind the pick"
    );
    assert_eq!(
        pile(simulator, PileName::Discard),
        vec!["CARD.STRIKE_IRONCLAD".to_owned()],
        "the play finished, its card put away, before the pick was shown"
    );
    assert_eq!(pile(simulator, PileName::Draw).len(), 3);
}

#[test]
fn a_stratagem_woken_by_a_gremlin_horn_asks_once_the_killing_play_is_over() {
    let mut simulator = horn_and_stratagem("ENCOUNTER.SHRINKER_BEETLE_WEAK")
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 4, 39, "SHRINKER_MOVE")
        .enemy(2, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.STRIKE_IRONCLAD", Some(1));
    assert_the_pick_stands_behind_the_finished_play(&simulator);

    pick_first(&mut simulator);
    // The picked card joins the hand first, then the horn's draw takes one.
    assert_eq!(
        pile(&simulator, PileName::Hand),
        vec![
            "CARD.DEFEND_IRONCLAD".to_owned(),
            "CARD.DEFEND_IRONCLAD".to_owned()
        ],
    );
    assert_eq!(pile(&simulator, PileName::Draw).len(), 1);
    assert_eq!(energy(&simulator), 3, "nothing paid out twice");
    assert!(
        matches!(simulator.decision(), DecisionContext::CombatPriority { .. }),
        "the turn is the player's again: {:?}",
        simulator.decision()
    );
}

#[test]
fn a_test_subjects_first_death_queues_the_pick_without_ending_the_fight() {
    // The death the reproduced failure came from: the boss's first form goes
    // down and stays in the fight behind its Adaptable, so the fight is not
    // won and the queued pick is shown.
    let mut simulator = horn_and_stratagem("ENCOUNTER.TEST_SUBJECT_BOSS")
        .enemy(1, id("MONSTER.TEST_SUBJECT"), 4, 100, "BITE_MOVE")
        .power(1, id("POWER.ADAPTABLE_POWER"), 1, PowerType::Buff)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.STRIKE_IRONCLAD", Some(1));
    assert_the_pick_stands_behind_the_finished_play(&simulator);

    pick_first(&mut simulator);
    let combat = simulator.state().combat.as_ref().expect("the fight stands");
    assert!(
        combat.in_progress,
        "a first-form death does not end the fight"
    );
    assert_eq!(combat.queued_hook_screens.len(), 0);
    assert_eq!(pile(&simulator, PileName::Hand).len(), 2);
    assert!(matches!(
        simulator.decision(),
        DecisionContext::CombatPriority { .. }
    ));
}

#[test]
fn a_final_kill_asks_nothing_and_shuffles_nothing() {
    // With the last enemy down the fight is already ending when the horn
    // answers: its draw hands nothing over, so nothing is shuffled and the
    // stratagem is never woken.
    let mut simulator = horn_and_stratagem("ENCOUNTER.SHRINKER_BEETLE_WEAK")
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 4, 39, "SHRINKER_MOVE")
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.STRIKE_IRONCLAD", Some(1));
    assert!(
        !matches!(simulator.decision(), DecisionContext::ChooseCards { .. }),
        "nothing is asked over a won fight: {:?}",
        simulator.decision()
    );
    assert_eq!(
        simulator.state().rng.stream(RunStream::Shuffle).counter(),
        0,
        "no shuffle drew on the shuffle stream"
    );
    if let Some(combat) = simulator.state().combat.as_ref() {
        assert!(!combat.in_progress, "the fight is over");
        assert_eq!(combat.queued_hook_screens.len(), 0);
    }
}

#[test]
fn a_queued_pick_over_a_pile_drawn_down_to_its_minimum_is_still_shown() {
    // Two deaths in one sweep. The first horn shuffles and wakes the
    // stratagem over two cards, which is more than it takes, so it asks. The
    // second horn draws one of those two while the question waits. When the
    // queued pick is reached the pile holds one card, as many as it takes —
    // but whether to ask was settled when it asked, and the pile screen stands
    // over whatever is left of the pile.
    let mut simulator = ScenarioBuilder::new("DEATHQUEUE", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .relics(&[id("RELIC.GREMLIN_HORN")])
        .power(0, id("POWER.STRATAGEM_POWER"), 1, PowerType::Buff)
        .card(PileName::Hand, 0, card("CARD.THUNDERCLAP"), 0)
        .card(PileName::Discard, 1, card("CARD.DEFEND_IRONCLAD"), 0)
        .card(PileName::Discard, 2, card("CARD.DEFEND_IRONCLAD"), 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 4, 39, "SHRINKER_MOVE")
        .enemy(2, id("MONSTER.SHRINKER_BEETLE"), 4, 39, "SHRINKER_MOVE")
        .enemy(3, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.THUNDERCLAP", None);
    let DecisionContext::ChooseCards {
        candidates,
        minimum,
        ..
    } = simulator.decision().clone()
    else {
        panic!("the stratagem asks: {:?}", simulator.decision());
    };
    assert_eq!(candidates.len(), 1, "the second horn drew the other card");
    assert_eq!(minimum, 1);
    assert_eq!(
        pile(&simulator, PileName::Hand),
        vec!["CARD.DEFEND_IRONCLAD".to_owned()],
        "the second horn's draw ran inside the play"
    );
    assert_eq!(energy(&simulator), 3 - 1 + 2, "both horns paid out");

    pick_first(&mut simulator);
    // The pick joins the hand, and the first horn's draw picks back up over
    // an empty pile: it shuffles the swept card back in, the stratagem takes
    // it without asking because it is all there is, and nothing is left to
    // draw.
    let mut hand = pile(&simulator, PileName::Hand);
    hand.sort();
    assert_eq!(
        hand,
        vec![
            "CARD.DEFEND_IRONCLAD".to_owned(),
            "CARD.DEFEND_IRONCLAD".to_owned(),
            "CARD.THUNDERCLAP".to_owned()
        ],
    );
    assert_eq!(pile(&simulator, PileName::Draw), Vec::<String>::new());
    assert!(matches!(
        simulator.decision(),
        DecisionContext::CombatPriority { .. }
    ));
}
