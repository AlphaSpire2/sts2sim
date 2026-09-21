//! A potion offer the belt has no room for is clickable, and the click does
//! nothing.
//!
//! A purchase gates on stock and gold and then hands off to the item itself,
//! and a potion entry fails for want of space if the belt has no slot. The
//! tail of the purchase — the gold, the shelf, the announced sale — is skipped
//! on that answer.

use std::collections::BTreeSet;

use sts2_engine::{
    ActMap, ActRoomPools, Action, ActiveRoom, MapCoord, MapPoint, MapPointType, RelicGrabBagState,
    RoomAt, RoomDefinition, RunConfig, RunPhase, RunState, ShopItem, ShopOffer, Simulator,
};

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

const POTION: &str = "POTION.VULNERABLE_POTION";
const PRICE: i32 = 40;

fn potion_offer() -> ShopOffer {
    ShopOffer {
        index: 0,
        item: ShopItem::Potion(id(POTION)),
        slot: sts2_engine::ShopSlot::Potion,
        base_price: PRICE,
        price: PRICE,
        sold: false,
    }
}

/// A run standing one step below a room that stocks a single potion, with the
/// belt filled by the models named.
#[allow(
    clippy::too_many_lines,
    reason = "one function stands a whole run's state up"
)]
fn run_below(room: RoomDefinition, point_type: MapPointType, held: &[&str]) -> Simulator {
    let mut state = sts2_content::shrinker_beetle_fixture()
        .expect("the fixture builds")
        .state()
        .clone();
    state.combat = None;
    state.terminal = None;
    state.run_player.gold = 500;
    for (slot, model) in held.iter().enumerate() {
        state.run_player.potions[slot] = Some(id(model));
    }
    let start = MapCoord { col: 0, row: 0 };
    let shelf = MapCoord { col: 0, row: 1 };
    let map = ActMap {
        width: 1,
        height: 1,
        start,
        boss: shelf,
        second_boss: None,
        points: vec![
            MapPoint {
                coord: start,
                point_type: MapPointType::Ancient,
                children: vec![shelf],
                can_modify: true,
            },
            MapPoint {
                coord: shelf,
                point_type,
                children: Vec::new(),
                can_modify: true,
            },
        ],
    };
    state.run = Some(RunState {
        config: RunConfig::fixture_baseline("N3Z9EE2CXB"),
        current_act: 0,
        floor: 0,
        current_coord: Some(start),
        maps: vec![map],
        rooms: vec![vec![RoomAt {
            coord: shelf,
            definition: room,
        }]],
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
    let mut simulator = Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run builds");
    let walk = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseMap { .. }))
        .cloned()
        .expect("the room is reachable");
    simulator.step(walk).expect("the walk applies");
    simulator
}

/// A merchant whose only stocked slot is the potion.
fn merchant(held: &[&str]) -> Simulator {
    run_below(
        RoomDefinition::Shop {
            model_id: None,
            offers: vec![potion_offer()],
            removal_price: 75,
        },
        MapPointType::Shop,
        held,
    )
}

/// An event that owns a shelf, whose only entry is the potion. The build's own
/// event shelf is the fake merchant's, which stocks relics, but every entry on
/// either shelf is bought the same one way.
fn event_shelf(held: &[&str]) -> Simulator {
    run_below(
        RoomDefinition::Event {
            standing_fight: None,
            rolled_card: None,
            rng_counter: 0,
            shelf: vec![potion_offer()],
            model_id: id("EVENT.TEST"),
            options: Vec::new(),
        },
        MapPointType::Unknown,
        held,
    )
}

const FULL: &[&str] = &[POTION, POTION, POTION];

fn buy(simulator: &Simulator) -> Option<Action> {
    simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::BuyShopItem { .. }))
        .cloned()
}

fn offer_is_sold(simulator: &Simulator) -> bool {
    match simulator
        .state()
        .run
        .as_ref()
        .and_then(|run| run.active_room.as_ref())
    {
        Some(ActiveRoom::Shop { offers, .. }) => offers[0].sold,
        Some(ActiveRoom::Event { shelf, .. }) => shelf[0].sold,
        _ => panic!("the run is not standing at a shelf"),
    }
}

/// The build lets the click through, so the action list carries the offer.
#[test]
fn a_full_belt_still_enumerates_an_affordable_potion() {
    assert!(
        buy(&merchant(FULL)).is_some(),
        "the merchant's potion is offered with the belt full"
    );
    assert!(
        buy(&event_shelf(FULL)).is_some(),
        "and so is the event shelf's"
    );
}

/// A purchase that fails for want of space is accepted and settles nothing.
#[test]
fn buying_onto_a_full_belt_is_accepted_and_changes_nothing() {
    for mut simulator in [merchant(FULL), event_shelf(FULL)] {
        let before = simulator.state_key().expect("the state keys");
        let gold_before = simulator.state().run_player.gold;
        let belt_before = simulator.state().run_player.potions.clone();

        let action = buy(&simulator).expect("the potion is offered");
        simulator.step(action).expect("the purchase applies");

        assert_eq!(
            simulator.state().run_player.gold,
            gold_before,
            "nothing was paid"
        );
        assert_eq!(
            simulator.state().run_player.potions,
            belt_before,
            "and nothing was taken"
        );
        assert!(
            !offer_is_sold(&simulator),
            "the offer is still on the shelf"
        );
        assert_eq!(
            simulator.state_key().expect("the state keys"),
            before,
            "the whole run is where it was"
        );
        assert!(
            buy(&simulator).is_some(),
            "and the player can make room and click again"
        );
    }
}

/// A slot to put it in is all that separates the two answers.
#[test]
fn a_free_slot_takes_the_potion_and_pays_for_it() {
    for mut simulator in [merchant(&[POTION, POTION]), event_shelf(&[POTION, POTION])] {
        let gold_before = simulator.state().run_player.gold;

        let action = buy(&simulator).expect("the potion is offered");
        simulator.step(action).expect("the purchase applies");

        assert_eq!(
            simulator.state().run_player.gold,
            gold_before - PRICE,
            "the price was paid"
        );
        assert_eq!(
            simulator.state().run_player.potions[2],
            Some(id(POTION)),
            "and the potion reached the belt"
        );
        assert!(offer_is_sold(&simulator), "the slot is cleared");
        assert!(buy(&simulator).is_none(), "and nothing is left to buy");
    }
}
