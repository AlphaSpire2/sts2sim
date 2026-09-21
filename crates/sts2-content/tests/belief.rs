//! The belief boundary over pinned content: an erased combat decision point
//! samples back into full worlds that agree with the original on everything a
//! player could see and on nothing they could not, deterministically in
//! `(analysis_seed, rollout)`, labeled analysis. This is the substrate a
//! search that plays the fight without peeking stands on.

use sts2_core::UnlockPresetManifest;
use sts2_engine::{Action, BeliefState, ConformanceOrigin, Simulator};

const SEED: &str = "NLD6VZXP94";

fn fresh_run() -> Simulator {
    let preset = UnlockPresetManifest::pinned().unwrap();
    let character = "CHARACTER.IRONCLAD".parse().unwrap();
    sts2_content::standard_run_on_preset_at(SEED, &character, &preset, 0).unwrap()
}

/// The first legal action, preferring a non-empty card pick so a skippable
/// reward cannot re-offer itself.
fn next_action(simulator: &Simulator) -> Action {
    simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseCards { cards, .. } if !cards.is_empty()))
        .or_else(|| simulator.legal_actions().first())
        .cloned()
        .expect("a live run offers an action")
}

/// The seeded run walked to its first in-combat decision.
fn first_combat() -> Simulator {
    let mut simulator = fresh_run();
    for _ in 0..40 {
        if simulator.state().combat.is_some() {
            return simulator;
        }
        let action = next_action(&simulator);
        simulator.step_quietly(&action).unwrap();
    }
    panic!("the seed reaches a combat inside forty decisions");
}

#[test]
fn a_sample_agrees_on_everything_visible_and_on_nothing_hidden() {
    let original = first_combat();
    let belief = BeliefState::from_simulator(&original).unwrap();
    let sample = belief.sample(7, 0).unwrap();
    assert_eq!(
        sample.agent_observation(),
        original.agent_observation(),
        "a sample shows the player the very screen they stood on"
    );
    assert_eq!(
        sample.observation_key().unwrap(),
        original.observation_key().unwrap()
    );
    assert_ne!(
        sample.state_key().unwrap(),
        original.state_key().unwrap(),
        "the hidden state is sampled, not copied"
    );
    assert_eq!(
        sample.legal_actions(),
        original.legal_actions(),
        "legality is observable, so every sample offers the same actions"
    );
    assert_eq!(sample.state().origin, ConformanceOrigin::Analysis);
}

#[test]
fn sampling_is_deterministic_in_the_pair_and_varied_across_it() {
    let belief = BeliefState::from_simulator(&first_combat()).unwrap();
    let one = belief.sample(7, 3).unwrap();
    let again = belief.sample(7, 3).unwrap();
    assert_eq!(
        one.state_key().unwrap(),
        again.state_key().unwrap(),
        "the same pair yields the same world"
    );
    let other = belief.sample(7, 4).unwrap();
    assert_ne!(
        one.state_key().unwrap(),
        other.state_key().unwrap(),
        "another rollout yields another world"
    );
    assert_eq!(
        one.observation_key().unwrap(),
        other.observation_key().unwrap(),
        "the worlds differ only where the player cannot see"
    );
}

#[test]
fn a_sampled_world_plays_on() {
    let belief = BeliefState::from_simulator(&first_combat()).unwrap();
    let mut sample = belief.sample(11, 2).unwrap();
    // The sampled fight steps like any other: walk it a dozen decisions.
    for _ in 0..12 {
        if sample.state().terminal.is_some() {
            break;
        }
        let action = next_action(&sample);
        sample.step_quietly(&action).unwrap();
    }
}

#[test]
fn two_indistinguishable_states_erase_to_the_same_belief() {
    // The original and one of its own samples are indistinguishable to the
    // player, so erasing either yields belief states whose samples coincide.
    let original = first_combat();
    let belief = BeliefState::from_simulator(&original).unwrap();
    let world = belief.sample(3, 5).unwrap();
    let re_erased = BeliefState::from_simulator(&world).unwrap();
    assert_eq!(
        belief.sample(9, 1).unwrap().state_key().unwrap(),
        re_erased.sample(9, 1).unwrap().state_key().unwrap(),
        "erasure is a projection: erasing a sample changes nothing"
    );
}

#[test]
fn a_sample_taken_on_a_screen_answers_like_the_screen_it_shows() {
    // An Armaments mid-resolution: the fight is suspended on a card choice,
    // with the rest of the play still owed behind the answer. A belief sample
    // taken here must stand on the same screen and settle the same answer.
    use sts2_engine::{DecisionContext, PileName, ScenarioBuilder};
    let card = |model: &str| sts2_engine::CardFingerprint::base(model.parse().unwrap());
    let mut original =
        ScenarioBuilder::new(SEED, "ENCOUNTER.SHRINKER_BEETLE_WEAK".parse().unwrap())
            .player_hp(80, 80)
            .energy(9, 9)
            .turn(1, 1)
            .enemy(
                1,
                "MONSTER.SHRINKER_BEETLE".parse().unwrap(),
                39,
                39,
                "SHRINKER_MOVE",
            )
            .card(PileName::Hand, 10, card("CARD.ARMAMENTS"), 1)
            .card(PileName::Hand, 11, card("CARD.STRIKE_IRONCLAD"), 1)
            .card(PileName::Hand, 12, card("CARD.DEFEND_IRONCLAD"), 1)
            .card(PileName::Draw, 20, card("CARD.BASH"), 1)
            .build(sts2_content::standard_registry())
            .unwrap();
    let play = original
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::PlayCard { .. }))
        .cloned()
        .expect("armaments is playable");
    original.step_quietly(&play).unwrap();
    assert!(matches!(
        original.decision(),
        DecisionContext::ChooseCards { .. }
    ));

    let belief = BeliefState::from_simulator(&original).unwrap();
    let mut sample = belief.sample(5, 0).unwrap();
    assert_eq!(
        sample.agent_observation(),
        original.agent_observation(),
        "the sample stands on the very screen"
    );
    let answer = original
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::ChooseCards { cards, .. }
                if cards.len() == 1
                    && cards[0].fingerprint.model_id == "CARD.STRIKE_IRONCLAD".parse().unwrap())
        })
        .cloned()
        .expect("the strike is on the screen");
    original.step_quietly(&answer).unwrap();
    sample.step_quietly(&answer).unwrap();
    assert_eq!(
        sample.agent_observation(),
        original.agent_observation(),
        "the answer settles the same visible fight"
    );
}

#[test]
fn a_run_outside_a_fight_is_refused() {
    let simulator = fresh_run();
    assert!(simulator.state().combat.is_none());
    assert!(BeliefState::from_simulator(&simulator).is_err());
}

#[test]
fn the_v6_sights_survive_erasure() {
    // Observation v6 widened what a player is held to see: the act's map with
    // its previewed boss, a digest of each creature's watched move history,
    // the live card-text tallies, the run's ascension and drift odds. Every
    // one of them must come through erasure unchanged — the equality above
    // covers them, and this pins that they are *populated*, so a field that
    // quietly stopped being filled could not pass by being empty on both
    // sides.
    let original = first_combat();
    let sample = BeliefState::from_simulator(&original)
        .unwrap()
        .sample(13, 2)
        .unwrap();
    let seen = original.agent_observation();
    let sampled = sample.agent_observation();

    let map = seen
        .map
        .as_ref()
        .expect("a run in progress carries its map");
    assert!(map.points.len() > 5, "the act laid out a map");
    assert_eq!(map.act, seen.current_act.unwrap());
    assert_eq!(sampled.map.as_ref(), Some(map), "the map is not sampled");

    let enemy = seen
        .creatures
        .iter()
        .find(|creature| creature.side != sts2_engine::CombatSide::Player)
        .expect("a fight has an enemy");
    assert!(
        !enemy.move_history.entered_counts.is_empty(),
        "a creature showing an intent has stood on a state"
    );
    assert_eq!(
        sampled.creatures, seen.creatures,
        "a watched history is not sampled"
    );

    assert_eq!(seen.ascension, Some(0), "the pinned baseline is A0");
    assert!(seen.odds.is_some(), "a run carries its drift odds");
    assert!(
        seen.unknown_room_odds.is_some(),
        "an act carries its unknown-room pity"
    );
    assert_eq!(sampled.ascension, seen.ascension);
    assert_eq!(sampled.odds, seen.odds);
    assert_eq!(sampled.unknown_room_odds, seen.unknown_room_odds);
    assert_eq!(sampled.tallies, seen.tallies);
}

#[test]
fn the_erasure_still_hides_what_it_always_hid() {
    // The v6 fields are all pinned by the erasure, which is the same thing as
    // saying they are visible. The guard that matters is the other side: the
    // hidden state a sample deals must still differ, or a new field would
    // have leaked the world into the observation.
    let original = first_combat();
    let belief = BeliefState::from_simulator(&original).unwrap();
    let one = belief.sample(5, 0).unwrap();
    let two = belief.sample(5, 1).unwrap();
    assert_ne!(
        one.state_key().unwrap(),
        two.state_key().unwrap(),
        "two rollouts are two worlds"
    );
    assert_eq!(
        one.agent_observation(),
        two.agent_observation(),
        "and one sight"
    );
}

#[test]
fn a_sample_agrees_on_a_card_the_fight_rolled_up() {
    // A card the fight made is not a card `from_scenario` stood up, and a
    // sample is rebuilt through `from_scenario`. A generated card missing one
    // of the saved properties its model declares would come back from the
    // rebuild repaired, and the sample would then differ from the world it
    // erased on
    // a card lying in the player's hand.
    use sts2_engine::{
        Amount, CardFilter, CardFingerprint, CardPileCmd, CardPoolDefinition, CardPoolRef, Command,
        DrawPilePosition, PileName, ScenarioBuilder,
    };
    let id = |value: &str| value.parse::<sts2_core::ModelId>().unwrap();
    let pool = id("CARD_POOL.BELIEF_GENERATION");
    let generator = id("CARD.BELIEF_GENERATOR");
    let generated = id("CARD.GENETIC_ALGORITHM");

    let mut registry = (*sts2_content::registry_for(&id("CHARACTER.DEFECT"))).clone();
    let entry = registry
        .card_pool(&id("CARD_POOL.DEFECT_CARD_POOL"))
        .unwrap()
        .iter()
        .find(|card| card.model_id == generated)
        .expect("the card stands in its character pool")
        .clone();
    registry.add_card_pool(CardPoolDefinition {
        model_id: pool.clone(),
        is_colorless: false,
        cards: vec![entry],
    });
    let mut definition = registry.card(&id("CARD.DEFEND_IRONCLAD")).unwrap().clone();
    definition.model_id = generator.clone();
    definition.energy_cost = 0;
    definition.upgrade = None;
    definition.supported_upgrade_levels = vec![0];
    definition.effects = vec![Command::CardPile(CardPileCmd::AddGeneratedFromPool {
        pool: CardPoolRef::Named(pool),
        count: Amount::Fixed(1),
        distinct: true,
        filter: CardFilter::Any,
        pile: PileName::Hand,
        position: DrawPilePosition::Bottom,
        excludes_source: false,
        upgraded: false,
        temporary_cost: None,
        ethereal: false,
        rolled_one_at_a_time: false,
    })];
    registry.add_card(definition);

    let mut original = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .played_as(id("CHARACTER.DEFECT"))
        .player_hp(75, 75)
        .energy(9, 9)
        .turn(1, 1)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .card(PileName::Hand, 10, CardFingerprint::base(generator), 0)
        .build(std::sync::Arc::new(registry))
        .unwrap();
    let play = original
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::PlayCard { .. }))
        .cloned()
        .expect("the generator is playable");
    original.step_quietly(&play).unwrap();
    assert!(
        original
            .state()
            .combat
            .as_ref()
            .unwrap()
            .player
            .piles
            .hand
            .iter()
            .any(|card| card.fingerprint.model_id == generated),
        "the roll put the card in hand"
    );

    let sample = BeliefState::from_simulator(&original)
        .unwrap()
        .sample(7, 0)
        .unwrap();
    assert_eq!(sample.agent_observation(), original.agent_observation());
    assert_eq!(
        sample.observation_key().unwrap(),
        original.observation_key().unwrap()
    );
    assert_eq!(sample.legal_actions(), original.legal_actions());
}
