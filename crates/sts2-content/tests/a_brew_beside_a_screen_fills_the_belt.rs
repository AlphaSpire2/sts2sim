//! A screen standing when a potion is drunk beside it holds the body that
//! offered it, and nothing else.
//!
//! An entropic brew fills whatever is left of the belt, one roll at a time,
//! and nothing in that loop asks the player anything. Drunk beside a standing
//! screen — a reward set the player may not skip, or a card pick an event
//! put up; either is reached with the belt in hand — the loop is not waiting
//! on that screen: it never offered it. Reading the standing screen as a hold
//! stops the loop after its first roll and parks the rounds it had left on
//! the run's pending list, where nothing picks them up: the belt comes out
//! short by every slot but one, and the next answer to the screen fails on a
//! frame it cannot run.

use std::collections::BTreeSet;

use sts2_engine::{
    ActMap, ActRoomPools, Action, CardFingerprint, CardHandle, ChoiceIdentity, ChoicePurpose,
    ChoiceScreen, MapCoord, MapPoint, MapPointType, RelicGrabBagState, RewardFingerprint,
    RewardItem, RewardSet, RunConfig, RunPhase, RunState, Simulator, SuspendedChoice,
};

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

const BREW: &str = "POTION.ENTROPIC_BREW";

/// What is standing when the brew is drunk.
#[derive(Clone, Copy)]
enum Standing {
    /// A reward set the player may not skip.
    ASetThatCannotBeSkipped,
    /// A card pick an event put up, answered on the run's own screen.
    ACardPick,
}

/// A run standing on a reward set it may not skip, with the brew in hand and
/// the rest of the belt free.
fn a_brew_beside_a_standing_set() -> Simulator {
    a_brew_beside(Standing::ASetThatCannotBeSkipped)
}

/// The same run standing on a card pick instead.
fn a_brew_beside_a_card_pick() -> Simulator {
    a_brew_beside(Standing::ACardPick)
}

/// A run standing on the named screen, with the brew in hand and the rest of
/// the belt free.
#[allow(
    clippy::too_many_lines,
    reason = "one function stands a whole run's state up"
)]
fn a_brew_beside(standing: Standing) -> Simulator {
    let mut state = sts2_content::shrinker_beetle_fixture()
        .expect("the fixture builds")
        .state()
        .clone();
    state.combat = None;
    state.terminal = None;
    // The drink clears its own slot before the body runs, so the loop has the
    // whole belt to fill.
    state.run_player.potions[0] = Some(id(BREW));
    for slot in 1..state.run_player.potions.len() {
        state.run_player.potions[slot] = None;
    }
    let start = MapCoord { col: 0, row: 0 };
    let map = ActMap {
        width: 1,
        height: 1,
        start,
        boss: start,
        second_boss: None,
        points: vec![MapPoint {
            coord: start,
            point_type: MapPointType::Ancient,
            children: Vec::new(),
            can_modify: true,
        }],
    };
    state.run = Some(RunState {
        config: RunConfig::fixture_baseline("N3Z9EE2CXB"),
        current_act: 0,
        floor: 1,
        current_coord: Some(start),
        maps: vec![map],
        rooms: vec![Vec::new()],
        visited: Vec::new(),
        visited_events: BTreeSet::new(),
        started_with_neow: true,
        reloads: 0,
        map_history: vec![sts2_engine::ActHistory {
            act: 0,
            points: Vec::new(),
        }],
        unlocks: sts2_core::UnlockPresetManifest::pinned()
            .expect("pinned unlock preset")
            .unlocks,
        acts: vec![ActRoomPools {
            act_model: id("ACT.OVERGROWTH"),
            ancient: id("EVENT.NEOW"),
            boss: id("ENCOUNTER.THE_KIN_BOSS"),
            second_boss: None,
            normal_encounters: vec![id("ENCOUNTER.SHRINKER_BEETLE_WEAK")],
            elite_encounters: Vec::new(),
            events: vec![id("EVENT.TEST")],
            normal_encounters_visited: 0,
            elite_encounters_visited: 0,
            events_visited: 0,
            boss_encounters_visited: 0,
        }],
        phase: RunPhase::Room,
        current_room_model: None,
        next_room_id: 1,
        current_room_id: 0,
        current_room_type: None,
        previous_room_type: None,
        active_room: None,
        pushed_room: None,
        begun_event: None,
        shared_relic_grab_bag: RelicGrabBagState::default(),
        player_relic_grab_bag: RelicGrabBagState::default(),
        marked_map_coord: None,
        reward_set: matches!(standing, Standing::ASetThatCannotBeSkipped).then(|| RewardSet {
            id: sts2_core::RewardSetId::from(1),
            player_slot: String::new(),
            rewards: vec![RewardItem {
                index: 0,
                fingerprint: RewardFingerprint::of("gold").with_gold(10),
                selected: false,
                passed_over: false,
                opened: false,
            }],
            disallow_skipping: true,
        }),
        reward_parents: Vec::new(),
        dismissed_reward_set: None,
        rest_option_owed: None,
        rest_options_owed: Vec::new(),
        suspended_choice: matches!(standing, Standing::ACardPick).then(|| SuspendedChoice {
            semantics: sts2_engine::ChoiceSemantics::default(),
            id: sts2_core::ChoiceId::from(0),
            identity: ChoiceIdentity::Offered,
            screen: ChoiceScreen::RewardGrid,
            candidates: ["CARD.FOLLY", "CARD.CLEAVE"]
                .into_iter()
                .enumerate()
                .map(|(index, model)| CardHandle {
                    index,
                    card_id: sts2_core::CardInstanceId::from(index as u64),
                    fingerprint: CardFingerprint::base(id(model)),
                    face: sts2_engine::CardFace::default(),
                })
                .collect(),
            minimum: 1,
            maximum: 1,
            cancelable: false,
            alternatives: Vec::new(),
            bundles: Vec::new(),
            purpose: ChoicePurpose::AddChosenCardsToDeck,
            picked: Vec::new(),
            hand_filters: None,
            queued: false,
        }),
        pending: std::collections::VecDeque::new(),
        next_choice_id: 1,
        next_reward_set_id: 2,
        finish_event_after_rewards: false,
        finish_event_after_pending: false,
        purchase_owed_by_rewards: None,
        owed_by_rewards: std::collections::VecDeque::new(),
        card_removals_used: 0,
        owed_purchases: std::collections::VecDeque::new(),
        owed_removal: false,
        unknown_room_odds: sts2_engine::UnknownRoomOdds::start_of_act(),
    });
    Simulator::from_generated_state(state, sts2_content::standard_registry()).expect("run builds")
}

#[test]
fn the_brew_fills_every_slot_beside_a_set_that_cannot_be_skipped() {
    let mut simulator = a_brew_beside_a_standing_set();
    let drink = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::UsePotion { .. }))
        .cloned()
        .expect("the brew is drinkable beside the screen");
    simulator.step(drink).expect("the drink applies");

    let held = &simulator.state().run_player.potions;
    assert!(
        held.iter().all(Option::is_some),
        "every slot the brew was drunk over is filled: {held:?}"
    );
    let run = simulator.state().run.as_ref().expect("the run stands");
    assert!(
        run.pending.is_empty(),
        "the loop finished where it stood rather than parking its rounds: {:?}",
        run.pending
    );
}

#[test]
fn the_screen_behind_the_brew_still_answers() {
    let mut simulator = a_brew_beside_a_standing_set();
    let drink = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::UsePotion { .. }))
        .cloned()
        .expect("the brew is drinkable beside the screen");
    simulator.step(drink).expect("the drink applies");
    let claim = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ClaimReward { .. }))
        .cloned()
        .expect("the set is still standing");
    simulator
        .step(claim)
        .expect("the reward is claimed over a belt the brew already filled");
}

#[test]
fn the_brew_fills_every_slot_beside_a_card_pick() {
    let mut simulator = a_brew_beside_a_card_pick();
    let drink = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::UsePotion { .. }))
        .cloned()
        .expect("the brew is drinkable beside the pick");
    simulator.step(drink).expect("the drink applies");

    let held = &simulator.state().run_player.potions;
    assert!(
        held.iter().all(Option::is_some),
        "every slot the brew was drunk over is filled: {held:?}"
    );
    let run = simulator.state().run.as_ref().expect("the run stands");
    assert!(
        run.pending.is_empty(),
        "the loop finished where it stood rather than parking its rounds: {:?}",
        run.pending
    );
    assert!(
        run.suspended_choice.is_some(),
        "the pick the brew was drunk beside is still standing"
    );
}

#[test]
fn the_card_pick_behind_the_brew_still_answers() {
    let mut simulator = a_brew_beside_a_card_pick();
    let drink = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::UsePotion { .. }))
        .cloned()
        .expect("the brew is drinkable beside the pick");
    simulator.step(drink).expect("the drink applies");
    let deck_before = simulator.state().run_player.deck.len();
    let pick = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseCards { .. }))
        .cloned()
        .expect("the pick is still standing");
    simulator
        .step(pick)
        .expect("the card is picked over a belt the brew already filled");
    assert_eq!(
        simulator.state().run_player.deck.len(),
        deck_before + 1,
        "the pick went into the deck"
    );
}
