//! A foul potion is thrown at the merchant beside whatever screen stands.
//!
//! A foul potion asks only that the room is a merchant's with a merchant still
//! in it, and the arm it takes there is a
//! hundred gold and nothing else — no question of its own, so nothing of it
//! waits. A shop reached with the belt full is routinely standing a screen
//! all the same: its own card removal, or the pick a relic bought there opens
//! as it arrives. The throw goes through beside that screen, the way every
//! other potion drunk out of a fight does.

use std::collections::BTreeSet;

use sts2_engine::{
    ActMap, ActRoomPools, Action, DecisionContext, MapCoord, MapPoint, MapPointType,
    RelicGrabBagState, RoomAt, RoomDefinition, RunConfig, RunPhase, RunState, Simulator,
};

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

const DAGGER: &str = "RELIC.PUNCH_DAGGER";
const FOUL: &str = "POTION.FOUL_POTION";
const DAGGER_PRICE: i32 = 150;
const THROW_GOLD: i32 = 100;

/// A run one step below a merchant who stocks a punch dagger, with the purse
/// holding exactly the dagger's price and a foul potion in the first belt
/// slot.
#[allow(
    clippy::too_many_lines,
    reason = "one function stands a whole run's state up"
)]
fn a_foul_potion_below_the_shop() -> Simulator {
    let mut state = sts2_content::shrinker_beetle_fixture()
        .expect("the fixture builds")
        .state()
        .clone();
    state.combat = None;
    state.terminal = None;
    // Enough for the dagger and nothing over, so the hundred the throw pays
    // can only have come from the throw.
    state.run_player.gold = DAGGER_PRICE;
    state.run_player.potions.fill(None);
    state.run_player.potions[0] = Some(id(FOUL));
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
            offers: vec![sts2_engine::ShopOffer {
                index: 0,
                item: sts2_engine::ShopItem::Relic(id(DAGGER)),
                slot: sts2_engine::ShopSlot::Relic,
                base_price: DAGGER_PRICE,
                price: DAGGER_PRICE,
                sold: false,
            }],
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

/// Walks into the shop and buys the dagger, which leaves its enchantment pick
/// standing.
fn buy_the_dagger(simulator: &mut Simulator) {
    let walk = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseMap { .. }))
        .cloned()
        .expect("the shop is reachable");
    simulator.step(walk).expect("the walk applies");
    let buy = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::BuyShopItem { .. }))
        .cloned()
        .expect("the dagger is on the shelf");
    simulator.step(buy).expect("the purchase applies");
}

fn the_throw(simulator: &Simulator) -> Option<Action> {
    simulator
        .legal_actions()
        .iter()
        .find(
            |action| matches!(action, Action::UsePotion { model_id, .. } if *model_id == id(FOUL)),
        )
        .cloned()
}

/// The dagger's pick stands, and the belt still offers the throw beside it.
#[test]
fn the_throw_is_offered_beside_the_daggers_screen() {
    let mut simulator = a_foul_potion_below_the_shop();
    buy_the_dagger(&mut simulator);

    assert!(
        matches!(simulator.decision(), DecisionContext::ChooseCards { .. }),
        "the dagger opened its pick as it arrived: {:?}",
        simulator.decision()
    );
    assert!(
        the_throw(&simulator).is_some(),
        "the belt answers over the screen: {:?}",
        simulator.legal_actions()
    );
}

/// The throw goes through where it stands: the gold arrives, the belt slot
/// empties, and the screen it was thrown beside is still the one standing.
#[test]
fn the_throw_pays_out_and_leaves_the_screen_standing() {
    let mut simulator = a_foul_potion_below_the_shop();
    buy_the_dagger(&mut simulator);
    let standing = match simulator.decision() {
        DecisionContext::ChooseCards { choice_id, .. } => *choice_id,
        other => panic!("the dagger's pick stands: {other:?}"),
    };
    let gold_before = simulator.state().run_player.gold;
    let throw = the_throw(&simulator).expect("the belt offers the throw");

    simulator.step(throw).expect("the throw applies");

    assert_eq!(
        simulator.state().run_player.gold,
        gold_before + THROW_GOLD,
        "the merchant paid for it"
    );
    assert!(
        simulator
            .state()
            .run_player
            .potions
            .iter()
            .flatten()
            .count()
            == 0,
        "and the belt slot emptied"
    );
    match simulator.decision() {
        DecisionContext::ChooseCards { choice_id, .. } => assert_eq!(
            *choice_id, standing,
            "the same screen is still the one waiting"
        ),
        other => panic!("the dagger's pick still stands after the throw: {other:?}"),
    }
}

/// And answering it afterwards still settles the purchase the screen belonged
/// to: the throw parked nothing ahead of the pick's own continuation.
#[test]
fn the_screen_still_answers_after_the_throw() {
    let mut simulator = a_foul_potion_below_the_shop();
    buy_the_dagger(&mut simulator);
    let throw = the_throw(&simulator).expect("the belt offers the throw");
    simulator.step(throw).expect("the throw applies");

    let answer = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseCards { cards, .. } if !cards.is_empty()))
        .cloned()
        .expect("the screen offers a card");
    simulator.step(answer).expect("the answer applies");

    let enchanted = simulator
        .state()
        .run_player
        .deck
        .iter()
        .filter(|card| {
            card.enchantment.as_ref().is_some_and(|enchantment| {
                enchantment.model_id == id("ENCHANTMENT.MOMENTUM") && enchantment.amount == 5
            })
        })
        .count();
    assert_eq!(
        enchanted, 1,
        "the dagger's five momentum landed on one card"
    );
    let run = simulator.state().run.as_ref().expect("the run stands");
    assert!(
        run.suspended_choice.is_none(),
        "nothing is left standing: {:?}",
        run.suspended_choice
    );
    assert!(
        run.pending.is_empty(),
        "and nothing is left waiting: {:?}",
        run.pending
    );
    assert!(
        matches!(simulator.decision(), DecisionContext::Shop { .. }),
        "the shop is what is left to answer: {:?}",
        simulator.decision()
    );
}
