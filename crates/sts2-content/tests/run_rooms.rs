use std::collections::BTreeSet;

#[path = "run_rooms/replay_storage.rs"]
mod replay_storage;

use sts2_engine::{
    ActMap, ActRoomPools, Action, ActiveRoom, CombatRoomDefinition, DecisionContext, EnemySpawn,
    EventEffect, EventOption, MapCoord, MapPoint, MapPointType, RelicGrabBagState, RoomAt,
    RoomDefinition, RunConfig, RunPhase, RunResult, RunState, ShopItem, ShopOffer, Simulator,
};

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().unwrap()
}

fn point(col: u8, row: u8, point_type: MapPointType, children: Vec<MapCoord>) -> MapPoint {
    MapPoint {
        coord: MapCoord { col, row },
        point_type,
        children,
        can_modify: true,
    }
}

fn branch_map() -> ActMap {
    let start = MapCoord { col: 2, row: 0 };
    let children = (0..=4).map(|col| MapCoord { col, row: 1 }).collect();
    let mut points = vec![point(2, 0, MapPointType::Ancient, children)];
    for (col, point_type) in [
        (0, MapPointType::Shop),
        (1, MapPointType::Treasure),
        (2, MapPointType::RestSite),
        (3, MapPointType::Unknown),
        (4, MapPointType::Monster),
    ] {
        // The rest site is the one point with somewhere to go, because a rest
        // site is left by walking on rather than by finishing it.
        let children = if col == 2 {
            vec![MapCoord { col: 2, row: 2 }]
        } else {
            Vec::new()
        };
        points.push(point(col, 1, point_type, children));
    }
    points.push(point(2, 2, MapPointType::Monster, Vec::new()));
    ActMap {
        width: 5,
        height: 2,
        start,
        boss: MapCoord { col: 4, row: 1 },
        second_boss: None,
        points,
    }
}

fn combat_room() -> RoomDefinition {
    RoomDefinition::Combat(CombatRoomDefinition {
        encounter_model: id("ENCOUNTER.SHRINKER_BEETLE_WEAK"),
        enemies: vec![EnemySpawn {
            model_id: id("MONSTER.SHRINKER_BEETLE"),
            fixed_hp: Some(39),
            initial_state: None,
            slot_name: None,
            locals: Vec::new(),
            rolled_locals: Vec::new(),
        }],
        rewards: Vec::new(),
        generated_rewards: None,
    })
}

fn generated_run() -> Simulator {
    generated_run_holding(&[])
}

#[allow(
    clippy::too_many_lines,
    reason = "one function stands a whole run's state up"
)]
fn generated_run_holding(extra_relics: &[&str]) -> Simulator {
    let mut state = sts2_content::shrinker_beetle_fixture()
        .unwrap()
        .state()
        .clone();
    state.combat = None;
    state.terminal = None;
    let map = branch_map();
    let rooms = vec![
        RoomAt {
            coord: MapCoord { col: 0, row: 1 },
            definition: RoomDefinition::Shop {
                model_id: None,
                offers: vec![ShopOffer {
                    index: 0,
                    item: ShopItem::Potion(id("POTION.VULNERABLE_POTION")),
                    slot: sts2_engine::ShopSlot::Potion,
                    base_price: 40,
                    price: 40,
                    sold: false,
                }],
                removal_price: 75,
            },
        },
        RoomAt {
            coord: MapCoord { col: 1, row: 1 },
            definition: RoomDefinition::Treasure {
                relics: vec![id("RELIC.LANTERN")],
                fills_itself: false,
            },
        },
        RoomAt {
            coord: MapCoord { col: 2, row: 1 },
            definition: RoomDefinition::RestSite,
        },
        RoomAt {
            coord: MapCoord { col: 3, row: 1 },
            definition: RoomDefinition::Event {
                standing_fight: None,
                rolled_card: None,
                rng_counter: 0,
                shelf: Vec::new(),
                model_id: id("EVENT.TEST"),
                options: vec![EventOption {
                    option_id: "gold".to_owned(),
                    label: "Gain 12 Gold".to_owned(),
                    effects: vec![EventEffect::GainGold(sts2_engine::EventAmount::Fixed(12))],
                    is_proceed: false,
                    was_chosen: false,
                    locked: false,
                }],
            },
        },
        RoomAt {
            coord: MapCoord { col: 4, row: 1 },
            definition: combat_room(),
        },
        RoomAt {
            coord: MapCoord { col: 2, row: 2 },
            definition: combat_room(),
        },
    ];
    state.run = Some(RunState {
        config: RunConfig::fixture_baseline("N3Z9EE2CXB"),
        current_act: 0,
        floor: 0,
        current_coord: Some(map.start),
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
    state.run_player.relics.extend(
        extra_relics
            .iter()
            .map(|relic| sts2_engine::RelicInstance::new(id(relic))),
    );
    Simulator::from_generated_state(state, sts2_content::standard_registry()).unwrap()
}

fn choose_map(simulator: &mut Simulator, col: u8) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::ChooseMap { destination }
                if destination.col == col && destination.row == 1)
        })
        .unwrap()
        .clone();
    simulator.step(action).unwrap();
}

/// The rest-site option with this id, as the room's own list offers it.
fn rest_option(simulator: &Simulator, option_id: &str) -> Action {
    simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::RestOption { option, .. }
                if option.option_id() == option_id)
        })
        .unwrap()
        .clone()
}

#[test]
fn map_event_treasure_rest_and_shop_use_public_actions() {
    let base = generated_run();

    let mut event = base.clone();
    choose_map(&mut event, 3);
    assert!(matches!(event.decision(), DecisionContext::Event { .. }));
    event.step(event.legal_actions()[0].clone()).unwrap();
    assert_eq!(event.state().run_player.gold, 111);
    assert!(matches!(
        event.decision(),
        DecisionContext::MapNavigation { .. }
    ));

    let mut treasure = base.clone();
    choose_map(&mut treasure, 1);
    treasure.step(treasure.legal_actions()[0].clone()).unwrap();
    assert_eq!(
        treasure.state().run_player.relics[1],
        sts2_engine::RelicInstance::new(id("RELIC.LANTERN")).obtained_on(1)
    );

    let mut rest = base.clone();
    choose_map(&mut rest, 2);
    assert!(matches!(rest.decision(), DecisionContext::RestSite { .. }));
    // The proceed button is disabled until an option has been taken, so a rest
    // site cannot be walked out of without resting.
    assert!(
        !rest
            .legal_actions()
            .iter()
            .any(|action| matches!(action, Action::ChooseMap { .. })),
        "the map is not reachable until an option has been taken"
    );
    rest.step(rest_option(&rest, "HEAL")).unwrap();
    // Taking an option never leaves the room: the build enables the proceed
    // button and the player walks out through the map screen it opens.
    let DecisionContext::RestSite { options, left_open } = rest.decision().clone() else {
        panic!("the rest site stays standing");
    };
    assert!(left_open, "the proceed is enabled once an option is taken");
    assert!(
        options.iter().all(|state| !state.standing),
        "the whole list is cleared where nothing keeps it standing"
    );
    let onward: Vec<_> = rest
        .legal_actions()
        .iter()
        .filter(|action| matches!(action, Action::ChooseMap { .. }))
        .collect();
    assert!(!onward.is_empty(), "and the map is now reachable");
    // The belt answers wherever the player is free to act, so a held potion is
    // still discardable here. Everything else left to do is walk on.
    let left: Vec<_> = rest
        .legal_actions()
        .iter()
        .filter(|action| !matches!(action, Action::DiscardPotion { .. }))
        .collect();
    assert_eq!(onward.len(), left.len(), "what is left to do is walk on");

    let mut shop = base;
    choose_map(&mut shop, 0);
    let before_deck = shop.state().run_player.deck.len();
    let remove = shop
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::BuyCardRemoval))
        .unwrap()
        .clone();
    let buy = shop
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::BuyShopItem { .. }))
        .unwrap()
        .clone();
    shop.step(buy).unwrap();
    assert_eq!(shop.state().run_player.gold, 59);
    let before = shop.state_key().unwrap();
    assert_eq!(
        shop.step(remove).unwrap_err().code,
        sts2_engine::ErrorCode::IllegalAction
    );
    assert_eq!(shop.state_key().unwrap(), before);
    assert_eq!(shop.state().run_player.deck.len(), before_deck);
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one test walks both deck screens end to end"
)]
fn the_smith_and_the_merchant_removal_are_deck_selection_screens() {
    // Taking the smith opens a screen over the deck and upgrades what comes
    // back, so taking the option is a decision of its own and the card is a
    // second one.
    // A curse has no upgraded form at all, so the screen has to hide it.
    let mut state = generated_run().state().clone();
    state
        .run_player
        .deck
        .push(sts2_engine::CardFingerprint::base(id("CARD.CLUMSY")));
    let mut rest =
        Simulator::from_generated_state(state, sts2_content::standard_registry()).unwrap();
    choose_map(&mut rest, 2);
    assert!(matches!(rest.decision(), DecisionContext::RestSite { .. }));
    rest.step(rest_option(&rest, "SMITH")).unwrap();
    let DecisionContext::ChooseCards {
        choice_id,
        identity,
        cancelable,
        candidates,
        ..
    } = rest.decision().clone()
    else {
        panic!("the smith puts a deck screen up");
    };
    assert_eq!(identity, sts2_engine::ChoiceIdentity::DeckCard);
    assert!(cancelable, "the smith can be backed out of");
    assert!(
        candidates
            .iter()
            .all(|handle| handle.fingerprint.upgrade_level == 0),
        "only what can be upgraded is offered"
    );
    let deck_before = rest.state().run_player.deck.clone();
    assert!(
        candidates
            .windows(2)
            .all(|pair| pair[0].index < pair[1].index),
        "the offer is in deck order"
    );
    assert!(
        candidates
            .iter()
            .all(|handle| deck_before[handle.index] == handle.fingerprint),
        "and every candidate is named by where it stands in the deck"
    );
    assert!(
        candidates.len() < deck_before.len(),
        "a card with no upgraded form is not on the screen"
    );

    // Backing out leaves the rest site standing, because a smith that took no
    // card never spent the option.
    let mut backed_out = rest.clone();
    backed_out
        .step(Action::ChooseCards {
            choice_id,
            cards: Vec::new(),
        })
        .unwrap();
    let DecisionContext::RestSite { options, left_open } = backed_out.decision().clone() else {
        panic!("backing out leaves the rest site standing");
    };
    assert!(!left_open, "and the proceed stays down");
    assert!(
        options.iter().all(|state| state.standing),
        "and the option that was backed out of is not spent"
    );
    assert_eq!(backed_out.state().run_player.deck, deck_before);

    rest.step(Action::ChooseCards {
        choice_id,
        cards: vec![candidates[1].clone()],
    })
    .unwrap();
    assert_eq!(
        rest.state().run_player.deck[candidates[1].index].upgrade_level,
        1,
        "the card at the chosen deck position is the one that was upgraded"
    );
    assert!(matches!(
        rest.decision(),
        DecisionContext::RestSite {
            left_open: true,
            ..
        }
    ));

    // The merchant's removal is the same screen family, and nothing is paid
    // until a card is actually chosen.
    let mut shop = generated_run();
    choose_map(&mut shop, 0);
    let gold_before = shop.state().run_player.gold;
    let deck_before = shop.state().run_player.deck.len();
    shop.step(Action::BuyCardRemoval).unwrap();
    let DecisionContext::ChooseCards {
        choice_id,
        identity,
        cancelable,
        ..
    } = shop.decision().clone()
    else {
        panic!("the merchant puts a deck screen up");
    };
    assert_eq!(identity, sts2_engine::ChoiceIdentity::DeckCard);
    assert!(cancelable);
    let backed_out = shop
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseCards { cards, .. } if cards.is_empty()))
        .expect("the removal can be backed out of")
        .clone();
    let mut kept = shop.clone();
    kept.step(backed_out).unwrap();
    assert_eq!(kept.state().run_player.gold, gold_before);
    assert_eq!(kept.state().run_player.deck.len(), deck_before);

    let taken = shop
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::ChooseCards { choice_id: id, cards }
                if *id == choice_id && cards.len() == 1)
        })
        .expect("a card can be named")
        .clone();
    shop.step(taken).unwrap();
    assert!(shop.state().run_player.gold < gold_before);
    assert_eq!(shop.state().run_player.deck.len(), deck_before - 1);
}

#[test]
fn map_combat_can_reach_a_real_terminal_defeat() {
    let mut simulator = generated_run();
    choose_map(&mut simulator, 4);
    assert!(matches!(
        simulator.decision(),
        DecisionContext::CombatPriority { .. }
    ));
    while !matches!(simulator.decision(), DecisionContext::Terminal { .. }) {
        let end = simulator
            .legal_actions()
            .iter()
            .find(|action| matches!(action, Action::EndTurn { .. }))
            .unwrap()
            .clone();
        simulator.step(end).unwrap();
    }
    assert_eq!(simulator.state().terminal, Some(RunResult::Defeat));
    assert_eq!(simulator.state().run_player.current_hp, 0);
}

#[test]
fn active_room_state_is_portable() {
    let mut simulator = generated_run();
    choose_map(&mut simulator, 0);
    assert!(matches!(
        simulator.state().run.as_ref().unwrap().active_room,
        Some(ActiveRoom::Shop { .. })
    ));
    let portable = simulator
        .project(sts2_engine::ProjectionScope::RunCore)
        .unwrap();
    let imported =
        Simulator::from_portable_state(portable, sts2_content::standard_registry()).unwrap();
    assert_eq!(simulator.legal_actions(), imported.legal_actions());
}

/// A fight's entrance is a stage of its opening, not a state the build rests
/// in: the recorder writes a `room.enter` observation of the combat before its
/// first turn has started, and the turn start right after it with no input
/// between — unless the player acted during the entrance itself. The build
/// takes such an input and holds it until the turn is standing, so the effect
/// lands after the hand is drawn; here the room is entered and the fight opened
/// in one step. The fixture drinks a potion during the Kaiser Crab's entrance,
/// and its Dexterity shows up only in the observation after the turn start.
#[test]
fn a_fights_entrance_is_passed_over_when_the_player_acted_during_it() {
    use sts2_replay::{AdapterErrorCode, Record, ReplayAdapter};

    fn record(sequence: u64, kind: &str, payload: serde_json::Value) -> Record {
        Record {
            sequence,
            kind: kind.to_owned(),
            payload,
        }
    }
    fn observe(sequence: u64, after: u64, context: &str, state: &serde_json::Value) -> Record {
        record(
            sequence,
            "state.observe",
            serde_json::json!({ "after": after, "context": context, "state": state }),
        )
    }
    /// The engine's own rendering, with the player's turn not yet begun.
    fn before_the_turn(adapter: &ReplayAdapter) -> serde_json::Value {
        let mut state = sts2_replay::observation::render(adapter.simulator());
        state["combat"]["players"][0]["phase"] = serde_json::json!("none");
        state["combat"]["players"][0]["energy"] = serde_json::json!(0);
        state
    }

    let mut adapter = ReplayAdapter::new(generated_run()).verifying_observations();
    adapter
        .accept(&record(
            1,
            "map.choose",
            serde_json::json!({ "data": { "col": 4, "row": 1 } }),
        ))
        .expect("the monster room is walked into");
    let entrance = before_the_turn(&adapter);
    adapter
        .accept(&observe(2, 1, "room.enter", &entrance))
        .expect("an observation is buffered");
    // The player acted during the entrance: the buffered snapshot is settled
    // here, and passed over rather than compared.
    adapter
        .accept(&record(
            3,
            "combat.end_turn",
            serde_json::json!({ "data": {} }),
        ))
        .expect("the entrance is not compared against a turn already standing");
    assert_eq!(adapter.fight_entrance_observations(), 1);

    // The first turn standing is the state the build rests in, and is compared:
    // the same doctored snapshot under the turn's own context is a divergence.
    let rest = before_the_turn(&adapter);
    adapter
        .accept(&observe(4, 3, "combat.player_turn.start", &rest))
        .expect("an observation is buffered");
    let error = adapter
        .accept(&record(
            5,
            "combat.end_turn",
            serde_json::json!({ "data": {} }),
        ))
        .expect_err("a turn already standing is compared, and differs");
    assert_eq!(error.code, AdapterErrorCode::StateDivergence);
    assert_eq!(adapter.fight_entrance_observations(), 1);
}

#[test]
fn a_held_red_mask_weakens_every_enemy_as_the_first_turn_opens() {
    let mut simulator = generated_run_holding(&["RELIC.RED_MASK"]);
    choose_map(&mut simulator, 4);
    let combat = simulator.state().combat.as_ref().unwrap();
    let enemy = &combat.creatures[1];
    assert_eq!(
        enemy
            .powers
            .iter()
            .map(|power| (power.model_id.to_string(), power.amount))
            .collect::<Vec<_>>(),
        [("POWER.WEAK_POWER".to_owned(), 1)]
    );
}

/// Taking a relic ends by running whatever the relic does on arrival, and
/// twenty-five relics open a card-selection screen from there. Out of combat
/// there is no queue to defer into, so what is left of that body waits on the
/// run until the player has answered.
#[test]
#[allow(clippy::too_many_lines, reason = "one test walks a whole obtain body")]
fn a_relic_obtained_outside_a_combat_holds_the_rest_of_its_own_body() {
    let mut state = generated_run().state().clone();
    let run = state.run.as_mut().expect("the harness builds a run");
    run.rooms[0][3].definition = RoomDefinition::Event {
        standing_fight: None,
        rolled_card: None,
        rng_counter: 0,
        shelf: Vec::new(),
        model_id: id("EVENT.TEST"),
        options: vec![EventOption {
            option_id: "relic".to_owned(),
            label: "Take the relic".to_owned(),
            effects: vec![EventEffect::ObtainRelic(id("RELIC.TEST_ASKER"))],
            is_proceed: false,
            was_chosen: false,
            locked: false,
        }],
    };
    let mut registry = (*sts2_content::standard_registry()).clone();
    registry.add_relic(sts2_engine::RelicDefinition {
        model_id: id("RELIC.TEST_ASKER"),
        grants_an_extra_turn: None,
        map_rule: None,
        rarity: sts2_engine::RelicRarity::Common,
        counter_property: None,
        counter_saved_only_when_set: false,
        keeps_hand: false,
        keeps_first_hand: false,
        on_obtained: vec![
            sts2_engine::Command::CardSelect(sts2_engine::CardSelectCmd::FromDeckGeneric {
                prefs: sts2_engine::SelectorPrefs::exactly(1).confirmed(),
                filter: sts2_engine::CardFilter::Any,
                then: Vec::new(),
            }),
            sts2_engine::Command::CardPile(sts2_engine::CardPileCmd::AddToDeck {
                card: sts2_engine::CardOrigin::Model(id("CARD.RAGE")),
            }),
        ],
        hooks: Vec::new(),
        setup: None,
        removes_unknown_room_types: Vec::new(),
        grants_free_travel: false,
        card_reward_alternative: None,
        has_upon_pickup_effect: false,
        spawns_pets: false,
        adds_pet: false,
        rolled_skin: None,
        is_stackable: false,
        allowed_in_shops: true,
        merchant_cost_override: None,
        modifies_x_value_by: 0,
        restocks_merchant_entries: false,
        adds_rest_site_option: None,
        keeps_rest_site_options_standing: false,
        keeps_energy_after_first_turn: false,
        keeps_block_up_to: None,
        forces_potion_reward_in_combat_rooms: false,
        refuses_potions: false,
        stops_draws_on_its_own_side_turn: false,
        card_cost_rule: None,
        cards_playable_each_turn: None,
        upgrades_cards_of_type: None,
        enchants_cards_it_meets: None,
        suppresses_treasure_while_second_counter_at_most: None,
        second_counter_property: None,
        counter_kind: None,
        latch_property: None,
        saved_card_array_property: None,
        prevents_owners_death_late: Vec::new(),
        adds_card_pool_to_rewards: None,
        adds_every_character_pool_to_rewards: false,
        initial_counter: 0,
        used_up: None,
        spawn_condition: None,
    });
    let mut simulator =
        Simulator::from_generated_state(state, std::sync::Arc::new(registry)).unwrap();

    let deck_before = simulator.state().run_player.deck.len();
    choose_map(&mut simulator, 3);
    simulator
        .step(simulator.legal_actions()[0].clone())
        .unwrap();

    assert!(matches!(
        simulator.decision(),
        DecisionContext::ChooseCards { .. }
    ));
    assert!(
        simulator
            .state()
            .run_player
            .relics
            .iter()
            .any(|relic| relic.model_id == id("RELIC.TEST_ASKER")),
        "the relic is the player's before its own body runs"
    );
    assert_eq!(
        simulator.state().run_player.deck.len(),
        deck_before,
        "and the call after the screen has not run"
    );
    assert!(
        simulator
            .legal_actions()
            .iter()
            .all(|action| matches!(action, Action::ChooseCards { .. })),
        "nothing but answers to the screen is on offer"
    );

    // The same state cloned at the screen answers to the same place.
    let mut copy = simulator.clone();
    let answer = simulator.legal_actions()[0].clone();
    simulator.step(answer.clone()).unwrap();
    copy.step(answer).unwrap();
    assert_eq!(copy.state_key().unwrap(), simulator.state_key().unwrap());

    assert_eq!(
        simulator.state().run_player.deck.len(),
        deck_before + 1,
        "the rest of the body ran once the player answered"
    );
    assert!(matches!(
        simulator.decision(),
        DecisionContext::MapNavigation { .. }
    ));
}

/// Winged boots let their owner step off the map's own edges three times.
///
/// One listener saying yes is enough to open the travel, and while anything
/// does the points that can be walked onto are the whole row above rather than
/// the point's own children. A step that was not onto a child is the step the
/// boots paid for.
#[test]
fn winged_boots_open_the_whole_row_and_spend_a_charge_on_a_step_off_the_graph() {
    let barefoot = generated_run();
    let onward: Vec<u8> = barefoot
        .legal_actions()
        .iter()
        .filter_map(|action| match action {
            Action::ChooseMap { destination } => Some(destination.col),
            _ => None,
        })
        .collect();
    assert_eq!(onward, vec![0, 1, 2, 3, 4]);

    // A map whose start leads to one point only, so free travel is the
    // difference between one destination and five.
    let mut narrow = generated_run_holding(&["RELIC.WINGED_BOOTS"])
        .state()
        .clone();
    if let Some(run) = narrow.run.as_mut() {
        for point in &mut run.maps[0].points {
            if point.coord == (MapCoord { col: 2, row: 0 }) {
                point.children = vec![MapCoord { col: 2, row: 1 }];
            }
        }
    }
    let mut simulator =
        Simulator::from_generated_state(narrow, sts2_content::standard_registry()).unwrap();
    let onward: Vec<u8> = simulator
        .legal_actions()
        .iter()
        .filter_map(|action| match action {
            Action::ChooseMap { destination } => Some(destination.col),
            _ => None,
        })
        .collect();
    assert_eq!(onward, vec![0, 1, 2, 3, 4]);

    // Stepping onto a point the start does not lead to spends one of the three.
    choose_map(&mut simulator, 1);
    let boots = simulator
        .state()
        .run_player
        .relics
        .iter()
        .find(|relic| relic.model_id == id("RELIC.WINGED_BOOTS"))
        .expect("the boots are held");
    assert_eq!(boots.counter, 1);
}

/// A fight one blow from over, with the turn's resources still on the table.
/// The stars stand for the Regent's second pool: the fixture's own character
/// has none, and what the test watches is that a reload clears the whole of
/// what a turn was given rather than the energy alone.
fn a_fight_one_blow_from_over() -> Simulator {
    let mut simulator = generated_run();
    choose_map(&mut simulator, 4);
    let mut state = simulator.state().clone();
    let combat = state.combat.as_mut().expect("the fight is standing");
    combat.player.stars = 3;
    for creature in &mut combat.creatures {
        if creature.id.get() == 1 {
            creature.current_hp = 1;
        }
    }
    Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the fight rebuilds")
}

fn strike(simulator: &mut Simulator) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::PlayCard { card, .. }
                if card.fingerprint.model_id == id("CARD.STRIKE_IRONCLAD"))
        })
        .expect("a strike is in hand")
        .clone();
    simulator.step(action).expect("the strike lands");
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

/// The stars the player has gained this turn, which is the count a radiate
/// reads.
fn stars_gained_this_turn(simulator: &Simulator) -> i32 {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .stars_gained_this_turn
}

/// The fight's round, and its player's turn, energy and stars.
fn fight_standing(simulator: &Simulator) -> (u32, u32, i32, i32) {
    let combat = simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing");
    (
        combat.round,
        combat.player.turn,
        combat.player.energy,
        combat.player.stars,
    )
}

/// The build saves the moment a fight is won, with the room marked
/// pre-finished, and a load off that save stands the finished room back up
/// without replaying the fight: the combat state around it is built fresh, so
/// it is not inside a turn at all. The round and the turn come back at one and
/// the resources the turn that won the fight was given come back at nothing.
#[test]
fn a_reload_over_a_won_fight_comes_back_outside_any_turn() {
    let mut simulator = a_fight_one_blow_from_over();
    strike(&mut simulator);
    assert!(
        simulator
            .state()
            .combat
            .as_ref()
            .expect("the fight is standing")
            .has_ended,
        "the strike won the fight"
    );
    let (_, _, energy, stars) = fight_standing(&simulator);
    assert!(energy > 0, "the turn that won it had energy left over");
    assert_eq!(stars, 3, "and its stars were never spent");

    simulator.resumed_from_save().expect("the run resumes");
    assert_eq!(fight_standing(&simulator), (1, 1, 0, 0));
}

/// A save taken on the way into a fight is a different thing: the fight is
/// played out from its first turn again, and that turn holds what it was
/// handed. Only the round and the turn are put back, and they were already
/// the fight's first.
#[test]
fn a_reload_into_a_fight_still_standing_keeps_what_the_turn_was_given() {
    let mut simulator = generated_run();
    choose_map(&mut simulator, 4);
    let before = fight_standing(&simulator);
    strike(&mut simulator);
    assert!(
        !simulator
            .state()
            .combat
            .as_ref()
            .expect("the fight is standing")
            .has_ended,
        "the beetle outlived the strike"
    );
    let spent = fight_standing(&simulator);
    assert!(spent.2 < before.2, "the strike cost energy");

    simulator.resumed_from_save().expect("the run resumes");
    assert_eq!(fight_standing(&simulator), spent);
}

/// The second-phase mark of a turn handover is put up as the handover begins
/// and taken down only as it ends. A run quit with the enemy's turn under way
/// never reaches that end, and nothing else takes the mark down: the fight the
/// load stands back up opens with it up, keeps it through the whole of its
/// first turn, and loses it the ordinary way once a handover of its own has
/// run out. Seed `2CKB2N4LNB`, records 807 to 826.
#[test]
fn a_run_quit_inside_a_handover_comes_back_with_the_mark_still_up() {
    let phase_two = |simulator: &Simulator| {
        simulator
            .state()
            .combat
            .as_ref()
            .expect("the fight is standing")
            .ending_player_turn_phase_two
    };
    let mut simulator = generated_run();
    choose_map(&mut simulator, 4);
    let entrance = simulator.snapshot();
    assert!(!phase_two(&simulator));
    end_turn(&mut simulator);

    // The quit, mid-handover, and the load over the save the entrance wrote.
    simulator.restore(&entrance).expect("the save restores");
    simulator
        .resumed_from_save_inside_a_handover()
        .expect("the run resumes");
    assert!(phase_two(&simulator), "the fight opens with the mark up");
    strike(&mut simulator);
    assert!(phase_two(&simulator), "and acting does not take it down");
    end_turn(&mut simulator);
    assert!(phase_two(&simulator), "a handover of its own is what does");
    strike(&mut simulator);
    assert!(
        !phase_two(&simulator),
        "once the player has acted behind it"
    );

    // A load that interrupted nothing leaves no mark.
    let mut plain = generated_run();
    choose_map(&mut plain, 4);
    plain.resumed_from_save().expect("the run resumes");
    strike(&mut plain);
    assert!(!phase_two(&plain));
}

/// Stars gained as a fight's room is entered — what a divine right pays —
/// belong to the fight's first turn: a star gained counts for the turn the
/// player's turn number names, and that number already stands at one as the
/// room opens. The count starts over as the number winds on, not as the turn
/// opens.
#[test]
fn stars_gained_as_a_fight_opens_belong_to_its_first_turn() {
    let mut simulator = generated_run_holding(&["RELIC.DIVINE_RIGHT"]);
    choose_map(&mut simulator, 4);
    assert_eq!(
        fight_standing(&simulator),
        (1, 1, 3, 3),
        "the divine right paid its three stars as the room was entered"
    );
    assert_eq!(
        stars_gained_this_turn(&simulator),
        3,
        "and they count for the first turn"
    );

    end_turn(&mut simulator);
    assert_eq!(fight_standing(&simulator).1, 2, "the second turn is up");
    assert_eq!(
        stars_gained_this_turn(&simulator),
        0,
        "and the count started over with it"
    );
}
