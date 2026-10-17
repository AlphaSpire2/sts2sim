//! A screen with too many answers to lay out is still a screen with answers.
//!
//! `compute_decision_actions` stops enumerating a multi-pick screen's whole
//! answers past `ORDERED_SELECTION_LIMIT`. `RELIC.BIIIG_HUG`'s
//! remove-exactly-4 over a twenty-three card deck (C(23, 4) = 8855 sets) is
//! past that line, on a screen a real player answers without noticing
//! anything unusual.
//!
//! The build's own screen never enumerates: the player clicks cards into the
//! selection one at a time, each click putting a card in or taking it back
//! out, and the selection completes once it holds enough.
//! So past the cap the boundary offers the same protocol: each action names
//! the selection so far plus one more card, an action reaching the minimum
//! finishes the answer, and a whole set in one action — the shape a recorded
//! run answers with — is taken directly by the legality gate. Replay builds
//! that whole set from the screen's candidates rather than looking for it on
//! the action list.

use std::collections::BTreeSet;

use serde_json::{Value, json};
use sts2_engine::{
    ActMap, ActRoomPools, Action, CardFingerprint, CardHandle, DecisionContext, MapCoord, MapPoint,
    MapPointType, RelicGrabBagState, RoomAt, RoomDefinition, RunConfig, RunPhase, RunState,
    Simulator,
};
use sts2_replay::{AdapterErrorCode, AdapterOutcome, Record, ReplayAdapter};

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

/// A run standing one step below a chest holding `relic`, with the deck padded
/// out to twenty-three cards — enough to put a four-card pick past the cap.
#[allow(
    clippy::too_many_lines,
    reason = "one function stands a whole run's state up"
)]
fn run_at_a_chest(relic: &str) -> Simulator {
    let mut state = sts2_content::shrinker_beetle_fixture()
        .expect("the fixture builds")
        .state()
        .clone();
    state.combat = None;
    state.terminal = None;
    let strike = state.run_player.deck[0].clone();
    while state.run_player.deck.len() < 23 {
        state.run_player.deck.push(strike.clone());
    }
    let start = MapCoord { col: 0, row: 0 };
    let chest = MapCoord { col: 0, row: 1 };
    let map = ActMap {
        width: 1,
        height: 1,
        start,
        boss: chest,
        second_boss: None,
        points: vec![
            MapPoint {
                coord: start,
                point_type: MapPointType::Ancient,
                children: vec![chest],
                can_modify: true,
            },
            MapPoint {
                coord: chest,
                point_type: MapPointType::Treasure,
                children: Vec::new(),
                can_modify: true,
            },
        ],
    };
    let rooms = vec![RoomAt {
        coord: chest,
        definition: RoomDefinition::Treasure {
            relics: vec![id(relic)],
            fills_itself: false,
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

/// Walks to the chest and takes what is on it, which opens the relic's own
/// pick screen.
fn take_the_relic(simulator: &mut Simulator) {
    let walk = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseMap { .. }))
        .cloned()
        .expect("the chest is reachable");
    simulator.step(walk).expect("the walk applies");
    let take = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::TakeTreasure { .. }))
        .cloned()
        .expect("the relic is on the chest");
    simulator.step(take).expect("taking it applies");
}

/// The screen standing at the boundary, with what its answer already holds.
fn the_screen(simulator: &Simulator) -> (sts2_core::ChoiceId, usize, usize, usize, usize) {
    let DecisionContext::ChooseCards {
        choice_id,
        minimum,
        maximum,
        candidates,
        picked,
        ..
    } = simulator.decision()
    else {
        panic!("no screen is standing: {:?}", simulator.decision());
    };
    (
        *choice_id,
        *minimum,
        *maximum,
        candidates.len(),
        picked.len(),
    )
}

/// The `ChooseCards` answers on offer, apart from anything else the boundary
/// carries beside the screen.
fn offered_answers(simulator: &Simulator) -> Vec<Action> {
    simulator
        .legal_actions()
        .iter()
        .filter(|action| matches!(action, Action::ChooseCards { .. }))
        .cloned()
        .collect()
}

/// The hug's remove-exactly-4 over twenty-three cards is past the cap, and
/// the boundary offers a click per card.
#[test]
fn a_biiig_hug_over_a_big_deck_offers_a_click_per_card() {
    let mut simulator = run_at_a_chest("RELIC.BIIIG_HUG");
    take_the_relic(&mut simulator);

    let (_, minimum, maximum, candidates, picked) = the_screen(&simulator);
    assert_eq!((minimum, maximum), (4, 4), "the hug asks for exactly four");
    assert_eq!(candidates, 23, "over the whole padded deck");
    assert_eq!(picked, 0, "nothing is clicked in yet");
    let answers = offered_answers(&simulator);
    assert_eq!(
        answers.len(),
        23,
        "one click per candidate: the screen the player sees has answers"
    );
    assert!(
        answers.iter().all(|action| matches!(
            action,
            Action::ChooseCards { cards, .. } if cards.len() == 1
        )),
        "each action clicks one card into the selection"
    );
}

/// Clicking cards in one at a time builds the selection, and the click that
/// reaches the count finishes the answer: four cards leave the deck.
#[test]
fn four_clicks_build_and_finish_the_removal() {
    let mut simulator = run_at_a_chest("RELIC.BIIIG_HUG");
    take_the_relic(&mut simulator);
    let deck_before = simulator.state().run_player.deck.len();

    for click in 1..=4_usize {
        let answer = offered_answers(&simulator)
            .first()
            .cloned()
            .expect("a click is offered");
        simulator.step(answer).expect("the click applies");
        if click < 4 {
            let (_, _, _, _, picked) = the_screen(&simulator);
            assert_eq!(picked, click, "the selection holds every card so far");
            assert_eq!(
                offered_answers(&simulator).len(),
                23 - click,
                "what is already picked is not offered again"
            );
        }
    }
    assert!(
        !matches!(simulator.decision(), DecisionContext::ChooseCards { .. }),
        "the fourth click finished the answer"
    );
    assert_eq!(
        simulator.state().run_player.deck.len(),
        deck_before - 4,
        "exactly four cards left the deck"
    );
}

/// A recorded run answers the whole screen in one record, in whatever order
/// the player clicked — the build's selection is an unordered set, so no order
/// is the wrong one.
/// The legality gate takes the whole set directly even though the action list
/// never laid it out.
#[test]
fn a_whole_set_in_one_answer_is_taken_directly() {
    let mut simulator = run_at_a_chest("RELIC.BIIIG_HUG");
    take_the_relic(&mut simulator);
    let deck_before = simulator.state().run_player.deck.len();
    let DecisionContext::ChooseCards {
        choice_id,
        candidates,
        ..
    } = simulator.decision().clone()
    else {
        panic!("no screen is standing");
    };
    // Out of candidate order, the way a recorded answer may arrive.
    let cards = vec![
        candidates[19].clone(),
        candidates[2].clone(),
        candidates[11].clone(),
        candidates[7].clone(),
    ];
    simulator
        .step(Action::ChooseCards { choice_id, cards })
        .expect("the whole set answers the screen");
    assert_eq!(simulator.state().run_player.deck.len(), deck_before - 4);
}

/// An answer the screen would never take is still refused: too many cards,
/// a card named twice, or a card that is not on the screen.
#[test]
fn the_gate_still_reads_the_screens_counts() {
    let mut simulator = run_at_a_chest("RELIC.BIIIG_HUG");
    take_the_relic(&mut simulator);
    let DecisionContext::ChooseCards {
        choice_id,
        candidates,
        ..
    } = simulator.decision().clone()
    else {
        panic!("no screen is standing");
    };
    let five = candidates[..5].to_vec();
    assert!(
        simulator
            .step(Action::ChooseCards {
                choice_id,
                cards: five
            })
            .is_err(),
        "five cards on an exactly-four screen refuse"
    );
    let twice = vec![
        candidates[0].clone(),
        candidates[0].clone(),
        candidates[1].clone(),
        candidates[2].clone(),
    ];
    assert!(
        simulator
            .step(Action::ChooseCards {
                choice_id,
                cards: twice
            })
            .is_err(),
        "one card named twice refuses"
    );
    let mut stranger = candidates[..4].to_vec();
    stranger[0].fingerprint = CardFingerprint::base(id("CARD.FOLLY"));
    assert!(
        simulator
            .step(Action::ChooseCards {
                choice_id,
                cards: stranger
            })
            .is_err(),
        "a card the screen does not offer refuses"
    );
}

/// Pael's tooth asks for exactly five of the upgradable deck — another
/// over-cap screen a chest can open — and it is answered the same way.
#[test]
fn a_paels_tooth_over_a_big_deck_is_answered_in_clicks() {
    let mut simulator = run_at_a_chest("RELIC.PAELS_TOOTH");
    take_the_relic(&mut simulator);
    let deck_before = simulator.state().run_player.deck.len();

    let (_, minimum, maximum, candidates, _) = the_screen(&simulator);
    assert_eq!(
        (minimum, maximum),
        (5, 5),
        "the tooth asks for exactly five"
    );
    assert_eq!(candidates, 23, "every padded card is upgradable");
    assert_eq!(
        offered_answers(&simulator).len(),
        23,
        "the screen has answers"
    );
    for _ in 0..5 {
        let answer = offered_answers(&simulator)
            .first()
            .cloned()
            .expect("a click is offered");
        simulator.step(answer).expect("the click applies");
    }
    assert!(
        !matches!(simulator.decision(), DecisionContext::ChooseCards { .. }),
        "the fifth click finished the answer"
    );
    assert_eq!(
        simulator.state().run_player.deck.len(),
        deck_before - 5,
        "the tooth took its five"
    );
}

/// The standing screen's id and candidates.
fn the_candidates(simulator: &Simulator) -> (sts2_core::ChoiceId, Vec<CardHandle>) {
    let DecisionContext::ChooseCards {
        choice_id,
        candidates,
        ..
    } = simulator.decision()
    else {
        panic!("no screen is standing: {:?}", simulator.decision());
    };
    (*choice_id, candidates.clone())
}

/// A deck screen's answer as the build records it: every card picked, in the
/// order the player picked them, each named by its deck index and fingerprint.
fn recorded_answer(choice_id: sts2_core::ChoiceId, cards: &[CardHandle]) -> Record {
    let cards: Vec<Value> = cards
        .iter()
        .map(|card| {
            json!({
                "deck_index": card.index,
                "fingerprint": sts2_replay::observation::render_card_fingerprint(&card.fingerprint),
            })
        })
        .collect();
    Record {
        sequence: 1,
        kind: "choice.cards".into(),
        payload: json!({
            "actor": "p0",
            "data": {
                "cards": cards,
                "choice_id": choice_id.get(),
                "choice_type": "deck_card",
                "offered": null,
            },
            "extensions": {},
        }),
    }
}

/// The action a replayed record applied.
fn replayed(simulator: Simulator, record: &Record) -> (Simulator, Action) {
    let mut adapter = ReplayAdapter::new(simulator);
    let AdapterOutcome::Applied(transition) =
        adapter.accept(record).expect("the recorded answer applies")
    else {
        panic!("the recorded answer was not applied");
    };
    (adapter.simulator().clone(), transition.accepted_action)
}

/// A recorded hug names its four cards in one record, in the order they were
/// picked. The action list past the cap never lays that set out, and replay
/// answers the screen with it all the same.
#[test]
fn a_recorded_hug_answers_its_screen_in_one_record() {
    let mut simulator = run_at_a_chest("RELIC.BIIIG_HUG");
    take_the_relic(&mut simulator);
    let deck_before = simulator.state().run_player.deck.len();
    let (choice_id, candidates) = the_candidates(&simulator);
    let picked = vec![
        candidates[19].clone(),
        candidates[2].clone(),
        candidates[11].clone(),
        candidates[7].clone(),
    ];
    assert!(
        offered_answers(&simulator)
            .iter()
            .all(|action| matches!(action, Action::ChooseCards { cards, .. } if cards.len() == 1)),
        "past the cap the list carries no whole set"
    );

    let (after, applied) = replayed(simulator, &recorded_answer(choice_id, &picked));
    assert_eq!(
        applied,
        Action::ChooseCards {
            choice_id,
            cards: picked
        },
        "the record's own cards, in its own order"
    );
    assert!(
        !matches!(after.decision(), DecisionContext::ChooseCards { .. }),
        "the record finished the answer"
    );
    assert_eq!(after.state().run_player.deck.len(), deck_before - 4);
}

/// Building the answer from the screen does not make every record an answer:
/// a card the screen does not offer, or more cards than it takes, is still
/// refused.
#[test]
fn a_recorded_answer_the_screen_would_not_take_is_refused() {
    let mut simulator = run_at_a_chest("RELIC.BIIIG_HUG");
    take_the_relic(&mut simulator);
    let (choice_id, candidates) = the_candidates(&simulator);

    let mut stranger = candidates[..4].to_vec();
    stranger[0].fingerprint = CardFingerprint::base(id("CARD.FOLLY"));
    let five = candidates[..5].to_vec();
    for (cards, name) in [(stranger, "a card not on the screen"), (five, "five cards")] {
        let mut adapter = ReplayAdapter::new(simulator.clone());
        let before = adapter.simulator().snapshot();
        let error = adapter
            .accept(&recorded_answer(choice_id, &cards))
            .expect_err(name);
        assert_eq!(error.code, AdapterErrorCode::IllegalAction, "{name}");
        assert_eq!(
            adapter.simulator().snapshot(),
            before,
            "{name} changes nothing"
        );
    }
}

/// A beautiful bracelet asks for three of twenty-three: few enough sets for
/// the list to carry each once, in candidate order, but not every order of
/// them. A record that picked them in another order is replayed in that
/// order, which is the order the build hands them back in.
#[test]
fn a_recorded_bracelet_keeps_the_order_it_was_picked_in() {
    let mut simulator = run_at_a_chest("RELIC.BEAUTIFUL_BRACELET");
    take_the_relic(&mut simulator);
    let (_, minimum, maximum, count, _) = the_screen(&simulator);
    assert_eq!(
        (minimum, maximum),
        (3, 3),
        "the bracelet asks for exactly three"
    );
    assert!(count > 12, "a screen this big lists sets, not orders");
    let (choice_id, candidates) = the_candidates(&simulator);
    let picked = vec![
        candidates[9].clone(),
        candidates[0].clone(),
        candidates[4].clone(),
    ];
    let recorded = Action::ChooseCards {
        choice_id,
        cards: picked.clone(),
    };
    assert!(
        !simulator.legal_actions().contains(&recorded),
        "the list does not carry this order"
    );
    assert!(
        simulator.legal_actions().contains(&Action::ChooseCards {
            choice_id,
            cards: vec![
                candidates[0].clone(),
                candidates[4].clone(),
                candidates[9].clone(),
            ],
        }),
        "the list carries the set in candidate order"
    );

    let (after, applied) = replayed(simulator, &recorded_answer(choice_id, &picked));
    assert_eq!(applied, recorded, "the record's own order");
    assert!(
        !matches!(after.decision(), DecisionContext::ChooseCards { .. }),
        "the record finished the answer"
    );
}
