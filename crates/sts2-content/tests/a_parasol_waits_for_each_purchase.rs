//! A parasol's walk down the shelf waits for each purchase, screen and all.
//!
//! A parasol buys every entry on the shelf in turn, after the room is entered
//! rather than as it is entered, waiting on each purchase, and ends with the
//! card removal asked for with the cost waived and no way to back out. A relic bought on the way —
//! a gnarled hammer, say — opens its own pick screen as it arrives, and the
//! build's walk does not reach the next entry until that screen is answered.
//! The removal the parasol asks for at the end is its own, not the shop's:
//! it charges nothing and is answered whatever the gold reads.

use std::collections::BTreeSet;

use sts2_engine::{
    ActMap, ActRoomPools, Action, DecisionContext, MapCoord, MapPoint, MapPointType,
    RelicGrabBagState, RoomAt, RoomDefinition, RunConfig, RunPhase, RunState, Simulator,
};

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

const HAMMER: &str = "RELIC.GNARLED_HAMMER";
const POTION: &str = "POTION.FIRE_POTION";

/// A penniless run standing one step below a merchant whose shelf holds a
/// gnarled hammer and a potion, with the parasol in the relic row and the
/// belt empty.
#[allow(
    clippy::too_many_lines,
    reason = "one function stands a whole run's state up"
)]
fn a_parasol_below_the_shop() -> Simulator {
    let mut state = sts2_content::shrinker_beetle_fixture()
        .expect("the fixture builds")
        .state()
        .clone();
    state.combat = None;
    state.terminal = None;
    // Nothing in hand: the parasol pays for none of it, the removal included.
    state.run_player.gold = 0;
    state.run_player.potions.fill(None);
    state
        .run_player
        .relics
        .push(sts2_engine::RelicInstance::new(id("RELIC.LORDS_PARASOL")));
    let start = MapCoord { col: 0, row: 0 };
    let shop = MapCoord { col: 0, row: 1 };
    let map = ActMap {
        width: 1,
        height: 1,
        start,
        boss: shop,
        second_boss: None,
        points: vec![
            MapPoint {
                coord: start,
                point_type: MapPointType::Ancient,
                children: vec![shop],
                can_modify: true,
            },
            MapPoint {
                coord: shop,
                point_type: MapPointType::Shop,
                children: Vec::new(),
                can_modify: true,
            },
        ],
    };
    let rooms = vec![RoomAt {
        coord: shop,
        definition: RoomDefinition::Shop {
            model_id: None,
            offers: vec![
                sts2_engine::ShopOffer {
                    index: 0,
                    item: sts2_engine::ShopItem::Relic(id(HAMMER)),
                    slot: sts2_engine::ShopSlot::Relic,
                    base_price: 150,
                    price: 150,
                    sold: false,
                },
                sts2_engine::ShopOffer {
                    index: 1,
                    item: sts2_engine::ShopItem::Potion(id(POTION)),
                    slot: sts2_engine::ShopSlot::Potion,
                    base_price: 50,
                    price: 50,
                    sold: false,
                },
            ],
            removal_price: 75,
        },
    }];
    state.run = Some(RunState {
        config: RunConfig::fixture_baseline("N3Z9EE2CXB"),
        current_act: 0,
        floor: 0,
        current_coord: Some(start),
        maps: vec![map],
        rooms: vec![rooms],
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
            events: Vec::new(),
            normal_encounters_visited: 0,
            elite_encounters_visited: 0,
            events_visited: 0,
            boss_encounters_visited: 0,
        }],
        phase: RunPhase::Map,
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
        reward_set: None,
        reward_parents: Vec::new(),
        dismissed_reward_set: None,
        rest_option_owed: None,
        rest_options_owed: Vec::new(),
        suspended_choice: None,
        pending: std::collections::VecDeque::new(),
        next_choice_id: 0,
        next_reward_set_id: 0,
        finish_event_after_rewards: false,
        finish_event_after_pending: false,
        purchase_owed_by_rewards: None,
        owed_by_rewards: std::collections::VecDeque::new(),
        card_removals_used: 0,
        owed_purchases: std::collections::VecDeque::new(),
        owed_removal: false,
        unknown_room_odds: sts2_engine::UnknownRoomOdds::start_of_act(),
    });
    Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run builds")
}

/// Walks into the shop, which is what sets the parasol off.
fn walk_in(simulator: &mut Simulator) {
    let walk = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseMap { .. }))
        .cloned()
        .expect("the shop is reachable");
    simulator.step(walk).expect("the walk applies");
}

/// Answers the standing pick with its first card.
fn answer_with_a_card(simulator: &mut Simulator) {
    let answer = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseCards { cards, .. } if !cards.is_empty()))
        .cloned()
        .expect("the screen offers a card");
    simulator.step(answer).expect("the answer applies");
}

fn holds_relic(simulator: &Simulator, relic: &str) -> bool {
    simulator
        .state()
        .run_player
        .relics
        .iter()
        .any(|held| held.model_id == id(relic))
}

fn holds_potion(simulator: &Simulator, potion: &str) -> bool {
    simulator
        .state()
        .run_player
        .potions
        .iter()
        .flatten()
        .any(|held| *held == id(potion))
}

fn shelf_slot_is_sold(simulator: &Simulator, index: usize) -> bool {
    match simulator
        .state()
        .run
        .as_ref()
        .and_then(|run| run.active_room.as_ref())
    {
        Some(sts2_engine::ActiveRoom::Shop { offers, .. }) => offers[index].sold,
        _ => panic!("the run is not standing in a shop"),
    }
}

fn removal_is_used(simulator: &Simulator) -> bool {
    match simulator
        .state()
        .run
        .as_ref()
        .and_then(|run| run.active_room.as_ref())
    {
        Some(sts2_engine::ActiveRoom::Shop { removal_used, .. }) => *removal_used,
        _ => panic!("the run is not standing in a shop"),
    }
}

/// The hammer's pick stands with the rest of the shelf waiting behind it; the
/// next purchase is not made until the pick is answered.
#[test]
fn the_walk_stops_at_the_hammers_screen() {
    let mut simulator = a_parasol_below_the_shop();
    walk_in(&mut simulator);

    // Nothing is bought as the room is entered: the shelf stands full and the
    // purchases are owed. What is asked for is what the walk reaches first
    // once they are made — the hammer's pick, which its purchase opens.
    assert!(
        matches!(simulator.decision(), DecisionContext::ChooseCards { .. }),
        "the hammer's pick is what is asked for: {:?}",
        simulator.decision()
    );
    assert!(
        !holds_relic(&simulator, HAMMER),
        "the hammer is not bought yet"
    );
    assert!(
        !shelf_slot_is_sold(&simulator, 0),
        "the hammer's slot is untouched while its purchase is owed"
    );
    assert!(
        simulator.automatic_purchase(0).is_some(),
        "the hammer is the parasol's next purchase"
    );
    assert!(
        !holds_potion(&simulator, POTION),
        "the potion behind it has not been bought yet"
    );
    assert!(
        !removal_is_used(&simulator),
        "nor has the removal been asked for yet"
    );
}

#[test]
fn the_rest_of_the_shelf_follows_the_answer() {
    let mut simulator = a_parasol_below_the_shop();
    walk_in(&mut simulator);
    // Answering the hammer's pick makes the hammer's purchase first — which is
    // what opened the pick — and then lands the answer.
    answer_with_a_card(&mut simulator);

    assert!(holds_relic(&simulator, HAMMER), "the hammer was obtained");
    assert!(
        shelf_slot_is_sold(&simulator, 0),
        "the hammer's purchase settled once its screen was answered"
    );
    assert!(
        !holds_potion(&simulator, POTION),
        "the potion behind it is still owed"
    );
    let DecisionContext::ChooseCards { cancelable, .. } = simulator.decision() else {
        panic!("the removal is asked for last: {:?}", simulator.decision());
    };
    assert!(!cancelable, "and it cannot be backed out of");
    // Answering the removal buys the potion first, then removes the card.
    answer_with_a_card(&mut simulator);
    assert!(holds_potion(&simulator, POTION), "the potion was bought");
    assert!(
        shelf_slot_is_sold(&simulator, 1),
        "and its slot was cleared"
    );
}

#[test]
fn the_parasols_removal_is_free() {
    let mut simulator = a_parasol_below_the_shop();
    walk_in(&mut simulator);
    answer_with_a_card(&mut simulator);
    let deck_before = simulator.state().run_player.deck.len();
    answer_with_a_card(&mut simulator);

    assert_eq!(
        simulator.state().run_player.deck.len(),
        deck_before - 1,
        "a card left the deck"
    );
    assert_eq!(simulator.state().run_player.gold, 0, "nothing was paid");
    assert!(
        removal_is_used(&simulator),
        "the merchant's removal is spent"
    );
    let run = simulator.state().run.as_ref().expect("the run stands");
    assert_eq!(run.card_removals_used, 1, "and the run's tally counts it");
    assert!(
        run.pending.is_empty(),
        "nothing of the walk is left waiting: {:?}",
        run.pending
    );
    assert!(
        matches!(simulator.decision(), DecisionContext::Shop { .. }),
        "the shop is what is left to answer: {:?}",
        simulator.decision()
    );
}
