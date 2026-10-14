//! A purchase whose item asks a question is not done until it is answered.
//!
//! A purchase waits on the whole of what the item does — the gold, taking the
//! relic, and the screen the relic opens on arrival — before it clears or
//! restocks the shelf and announces the sale. The tail of the purchase is
//! itself the parked state, settling the sale behind whatever the item still
//! owes, and the out-of-combat drain processes a parked pass the way combat's
//! own resume discipline does.
//!
//! The fixture buys `RELIC.PUNCH_DAGGER` while `RELIC.MAW_BANK` listens for
//! the sale: the dagger opens a pick screen as it arrives, and the maw bank's
//! pass is what has to wait behind it.

use std::collections::BTreeSet;

use sts2_engine::{
    ActMap, ActRoomPools, Action, CardFingerprint, CommandContext, DecisionContext, Effect,
    HookContext, HookEvent, Listener, ListenerSource, MapCoord, MapPoint, MapPointType,
    RelicGrabBagState, RoomAt, RoomDefinition, RunConfig, RunPhase, RunState, Simulator,
    SuspendedHookPass,
};

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

/// A run standing one step below a merchant whose shelf holds the punch
/// dagger, with the maw bank in the player's relic row.
#[allow(
    clippy::too_many_lines,
    reason = "one function stands a whole run's state up"
)]
fn run_below_the_shop() -> Simulator {
    run_below_a_shop_stocking("RELIC.PUNCH_DAGGER")
}

/// The same run with the shelf's one slot holding whichever relic is named.
#[allow(
    clippy::too_many_lines,
    reason = "one function stands a whole run's state up"
)]
fn run_below_a_shop_stocking(relic: &str) -> Simulator {
    let mut state = sts2_content::shrinker_beetle_fixture()
        .expect("the fixture builds")
        .state()
        .clone();
    state.combat = None;
    state.terminal = None;
    state.run_player.gold = 500;
    state
        .run_player
        .relics
        .push(sts2_engine::RelicInstance::new(id("RELIC.MAW_BANK")));
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
                item: sts2_engine::ShopItem::Relic(id(relic)),
                slot: sts2_engine::ShopSlot::Relic,
                base_price: 100,
                price: 100,
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

/// Walks into the shop and buys the dagger, which opens its Momentum pick.
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

fn maw_bank_is_latched(simulator: &Simulator) -> bool {
    simulator
        .state()
        .run_player
        .relics
        .iter()
        .find(|relic| relic.model_id == id("RELIC.MAW_BANK"))
        .expect("the maw bank is held")
        .latched
}

fn shelf_slot_is_sold(simulator: &Simulator) -> bool {
    match simulator
        .state()
        .run
        .as_ref()
        .and_then(|run| run.active_room.as_ref())
    {
        Some(sts2_engine::ActiveRoom::Shop { offers, .. }) => offers[0].sold,
        _ => panic!("the run is not standing in a shop"),
    }
}

/// With the maw bank listening, the dagger's pick screen is answerable: the
/// answer applies, and everything the wrapper owed lands behind it.
#[test]
fn the_daggers_screen_is_answerable_with_a_maw_bank_held() {
    let mut simulator = run_below_the_shop();
    buy_the_dagger(&mut simulator);

    assert!(
        matches!(simulator.decision(), DecisionContext::ChooseCards { .. }),
        "the purchase opened the dagger's pick screen"
    );
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
            card.enchantment
                .as_ref()
                .is_some_and(|held| held.model_id == id("ENCHANTMENT.MOMENTUM"))
        })
        .count();
    assert_eq!(enchanted, 1, "the chosen card carries the Momentum");
    assert!(
        maw_bank_is_latched(&simulator),
        "the sale reached the maw bank once the screen was answered"
    );
    assert!(shelf_slot_is_sold(&simulator), "and the shelf was cleared");
}

/// The build's ordering, watched through the screen: the gold is paid, but the
/// shelf is not cleared and the sale is not announced until the whole of the
/// purchase — screen included — has come back.
#[test]
fn the_wrapper_waits_behind_the_screen() {
    let mut simulator = run_below_the_shop();
    let walk = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseMap { .. }))
        .cloned()
        .expect("the shop is reachable");
    simulator.step(walk).expect("the walk applies");
    // Read after the walk: the maw bank pays its twelve on the way in.
    let gold_before = simulator.state().run_player.gold;
    let buy = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::BuyShopItem { .. }))
        .cloned()
        .expect("the dagger is on the shelf");
    simulator.step(buy).expect("the purchase applies");

    assert_eq!(
        simulator.state().run_player.gold,
        gold_before - 100,
        "the gold went first, inside the purchase"
    );
    assert!(
        !shelf_slot_is_sold(&simulator),
        "the shelf is untouched while the screen stands"
    );
    assert!(
        !maw_bank_is_latched(&simulator),
        "and the sale is not announced yet"
    );
}

/// The drain itself keeps combat's resume discipline: a listener pass parked
/// on the run's pending list runs once the screen ahead of it is answered.
#[test]
fn a_parked_pass_on_the_pending_list_still_runs() {
    let simulator = run_below_the_shop();
    let mut state = simulator.state().clone();
    let run = state.run.as_mut().expect("the run stands");
    // A screen standing over the pending list, with nothing behind its answer.
    run.suspended_choice = Some(sts2_engine::SuspendedChoice {
        semantics: sts2_engine::ChoiceSemantics::default(),
        id: sts2_core::ChoiceId::from(0),
        identity: sts2_engine::ChoiceIdentity::Offered,
        screen: sts2_engine::ChoiceScreen::ChooseACard,
        candidates: vec![sts2_engine::CardHandle {
            index: 0,
            card_id: sts2_core::CardInstanceId::from(0_u64),
            fingerprint: CardFingerprint::base(id("CARD.FOLLY")),
            face: sts2_engine::CardFace::default(),
        }],
        minimum: 0,
        maximum: 1,
        cancelable: false,
        alternatives: Vec::new(),
        bundles: Vec::new(),
        purpose: sts2_engine::ChoicePurpose::RunCommands {
            commands: Vec::new(),
            context: CommandContext::default(),
        },
        picked: Vec::new(),
        hand_filters: None,
    });
    run.next_choice_id = 1;
    // The parked pass: the maw bank answering the sale, snapshotted the way a
    // suspension would have left it.
    run.pending
        .push_back(Effect::ResumeHookPass(Box::new(SuspendedHookPass {
            event: HookEvent::AfterItemPurchased,
            listeners: vec![Listener {
                owner: sts2_core::CreatureCombatId::from(0),
                source: ListenerSource::Relic,
                model_id: id("RELIC.MAW_BANK"),
                power_instance: None,
            }],
            next: 0,
            context: HookContext::default(),
        })));
    let mut simulator = Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the surgered run builds");
    assert!(!maw_bank_is_latched(&simulator));

    let answer = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseCards { cards, .. } if cards.is_empty()))
        .cloned()
        .expect("the screen can be answered with nothing");
    simulator.step(answer).expect("the answer applies");
    assert!(
        maw_bank_is_latched(&simulator),
        "the parked pass ran off the drain once the screen was answered"
    );
}

/// A reward screen the item opened is the same wait as a pick screen, and the
/// shelf keeps its relic behind it.
///
/// An orrery offers five card rewards as it arrives and waits on them, so the
/// whole tail of the purchase — the shelf, the sale — is behind those offers.
/// The slot still reads stocked while the offers are on the table; a shop
/// with the slot already empty under them is one the build never shows.
#[test]
fn an_orrerys_offers_hold_the_shelf_and_leave_the_shop_standing() {
    let mut simulator = run_below_a_shop_stocking("RELIC.ORRERY");
    buy_the_dagger(&mut simulator);

    assert!(
        !shelf_slot_is_sold(&simulator),
        "the shelf keeps the orrery while its offers stand"
    );
    assert!(
        simulator
            .state()
            .run
            .as_ref()
            .is_some_and(|run| run.reward_set.is_some()),
        "the offers are on the table"
    );

    // Every offer answered: five card rewards, each one a pick of three.
    let mut answered = 0;
    while simulator
        .state()
        .run
        .as_ref()
        .is_some_and(|run| run.reward_set.is_some())
    {
        let action = simulator
            .legal_actions()
            .iter()
            .find(|action| {
                matches!(
                    action,
                    Action::ClaimReward { .. } | Action::ChooseCards { .. }
                )
            })
            .cloned()
            .expect("an offer is answerable");
        if matches!(action, Action::ClaimReward { .. }) {
            answered += 1;
        }
        simulator.step(action).expect("the answer applies");
        assert!(answered <= 5, "the orrery offers five and no more");
    }

    assert!(
        shelf_slot_is_sold(&simulator),
        "the shelf is cleared once the last offer is answered"
    );
    assert!(
        matches!(
            simulator
                .state()
                .run
                .as_ref()
                .and_then(|run| run.active_room.as_ref()),
            Some(sts2_engine::ActiveRoom::Shop { .. })
        ),
        "and the shop is still standing: only an event's room is finished by \
         the rewards an option of it put up"
    );
}
