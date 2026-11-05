//! A screen that takes several cards is worked a pick at a time, whatever its
//! size.
//!
//! The build's own screen never enumerates: the player clicks cards into the
//! selection one at a time, and the selection completes once it holds enough —
//! or is confirmed, on a screen that takes a range. `compute_decision_actions`
//! offers the same protocol on every such screen: a `PickCard` per card not yet
//! in the selection, a `ConfirmCards` once the selection has reached the
//! minimum with room left, and the pick that fills the screen finishing the
//! answer on its own. Laying out whole answers instead would put one action
//! per set on the list — `RELIC.BIIIG_HUG`'s remove-exactly-4 over a
//! twenty-three card deck is C(23, 4) = 8855 of them.
//!
//! The build's recorder writes only the completed choice, so a recorded run
//! answers such a screen with its whole set in one action the list never lays
//! out. The legality gate takes that directly, in the order it was picked, and
//! replay builds the set from the screen's candidates rather than looking for
//! it on the action list.

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
/// out to `deck` cards.
#[allow(
    clippy::too_many_lines,
    reason = "one function stands a whole run's state up"
)]
fn run_at_a_chest(relic: &str, deck: usize) -> Simulator {
    let mut state = sts2_content::shrinker_beetle_fixture()
        .expect("the fixture builds")
        .state()
        .clone();
    state.combat = None;
    state.terminal = None;
    let strike = state.run_player.deck[0].clone();
    while state.run_player.deck.len() < deck {
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

/// The picks on offer: one per card not yet in the selection.
fn offered_picks(simulator: &Simulator) -> Vec<Action> {
    simulator
        .legal_actions()
        .iter()
        .filter(|action| matches!(action, Action::PickCard { .. }))
        .cloned()
        .collect()
}

/// Whether the list lays out any whole answer naming cards.
fn lays_out_a_whole_answer(simulator: &Simulator) -> bool {
    simulator
        .legal_actions()
        .iter()
        .any(|action| matches!(action, Action::ChooseCards { cards, .. } if !cards.is_empty()))
}

/// Whether the confirm button is on offer.
fn offers_a_confirm(simulator: &Simulator) -> bool {
    simulator
        .legal_actions()
        .iter()
        .any(|action| matches!(action, Action::ConfirmCards { .. }))
}

/// Takes the first pick on offer.
fn pick_one(simulator: &mut Simulator) {
    let pick = offered_picks(simulator)
        .first()
        .cloned()
        .expect("a pick is offered");
    simulator.step(pick).expect("the pick applies");
}

/// How many deck cards carry an enchantment.
fn enchanted(simulator: &Simulator) -> usize {
    simulator
        .state()
        .run_player
        .deck
        .iter()
        .filter(|card| card.enchantment.is_some())
        .count()
}

/// The hug's remove-exactly-4 over twenty-three cards is offered as a pick
/// per card, and nothing else: no whole set, and no confirm while the
/// selection is short of the four.
#[test]
fn a_biiig_hug_over_a_big_deck_offers_a_pick_per_card() {
    let mut simulator = run_at_a_chest("RELIC.BIIIG_HUG", 23);
    take_the_relic(&mut simulator);

    let (_, minimum, maximum, candidates, picked) = the_screen(&simulator);
    assert_eq!((minimum, maximum), (4, 4), "the hug asks for exactly four");
    assert_eq!(candidates, 23, "over the whole padded deck");
    assert_eq!(picked, 0, "nothing is picked yet");
    assert_eq!(
        offered_picks(&simulator).len(),
        23,
        "one pick per candidate: the screen the player sees has answers"
    );
    assert!(!lays_out_a_whole_answer(&simulator), "and no whole set");
    assert!(
        !offers_a_confirm(&simulator),
        "confirm stays off the list until the selection reaches the minimum"
    );
}

/// A deck small enough that the sets could be laid out — C(12, 4) = 495 — is
/// worked the same way: the protocol is the screen's, not a cap's.
#[test]
fn a_biiig_hug_over_a_small_deck_is_worked_the_same_way() {
    let mut simulator = run_at_a_chest("RELIC.BIIIG_HUG", 12);
    take_the_relic(&mut simulator);
    let deck_before = simulator.state().run_player.deck.len();

    let (_, _, _, candidates, _) = the_screen(&simulator);
    assert_eq!(candidates, 12);
    assert_eq!(offered_picks(&simulator).len(), 12, "one pick per card");
    assert!(!lays_out_a_whole_answer(&simulator), "no set is laid out");
    for _ in 0..4 {
        pick_one(&mut simulator);
    }
    assert!(
        !matches!(simulator.decision(), DecisionContext::ChooseCards { .. }),
        "the fourth pick finished the answer"
    );
    assert_eq!(simulator.state().run_player.deck.len(), deck_before - 4);
}

/// Picking cards in one at a time builds the selection, and the pick that
/// reaches the count finishes the answer: four cards leave the deck.
#[test]
fn four_picks_build_and_finish_the_removal() {
    let mut simulator = run_at_a_chest("RELIC.BIIIG_HUG", 23);
    take_the_relic(&mut simulator);
    let deck_before = simulator.state().run_player.deck.len();

    for click in 1..=4_usize {
        pick_one(&mut simulator);
        if click < 4 {
            let (_, _, _, _, picked) = the_screen(&simulator);
            assert_eq!(picked, click, "the selection holds every card so far");
            assert_eq!(
                offered_picks(&simulator).len(),
                23 - click,
                "what is already picked is not offered again"
            );
            assert!(
                !offers_a_confirm(&simulator),
                "an exact screen has no confirm to press: the last pick closes it"
            );
        }
    }
    assert!(
        !matches!(simulator.decision(), DecisionContext::ChooseCards { .. }),
        "the fourth pick finished the answer"
    );
    assert_eq!(
        simulator.state().run_player.deck.len(),
        deck_before - 4,
        "exactly four cards left the deck"
    );
}

/// A pick names a card the screen offers and the selection does not already
/// hold; anything else is refused, and so is a confirm the screen is not
/// showing.
#[test]
fn a_pick_the_screen_would_not_take_is_refused() {
    let mut simulator = run_at_a_chest("RELIC.BIIIG_HUG", 23);
    take_the_relic(&mut simulator);
    let (choice_id, candidates) = the_candidates(&simulator);
    let before = simulator.snapshot();
    assert!(
        simulator.step(Action::ConfirmCards { choice_id }).is_err(),
        "nothing is picked, so there is nothing to confirm"
    );
    let mut stranger = candidates[0].clone();
    stranger.fingerprint = CardFingerprint::base(id("CARD.FOLLY"));
    assert!(
        simulator
            .step(Action::PickCard {
                choice_id,
                card: stranger
            })
            .is_err(),
        "a card the screen does not offer refuses"
    );
    assert_eq!(simulator.snapshot(), before, "a refusal changes nothing");

    pick_one(&mut simulator);
    let picked = candidates[0].clone();
    assert!(
        simulator
            .step(Action::PickCard {
                choice_id,
                card: picked
            })
            .is_err(),
        "a card already in the selection is not picked twice"
    );
}

/// A recorded run answers the whole screen in one record, in whatever order
/// the player clicked — the build's selection is an unordered set, so no order
/// is the wrong one.
/// The legality gate takes the whole set directly even though the action list
/// never laid it out.
#[test]
fn a_whole_set_in_one_answer_is_taken_directly() {
    let mut simulator = run_at_a_chest("RELIC.BIIIG_HUG", 23);
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
    let mut simulator = run_at_a_chest("RELIC.BIIIG_HUG", 23);
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
    let three = candidates[..3].to_vec();
    assert!(
        simulator
            .step(Action::ChooseCards {
                choice_id,
                cards: three
            })
            .is_err(),
        "three cards on an exactly-four screen refuse"
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
/// screen a chest can open — and it is answered the same way.
#[test]
fn a_paels_tooth_over_a_big_deck_is_answered_in_picks() {
    let mut simulator = run_at_a_chest("RELIC.PAELS_TOOTH", 23);
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
        offered_picks(&simulator).len(),
        23,
        "the screen has answers"
    );
    for _ in 0..5 {
        pick_one(&mut simulator);
    }
    assert!(
        !matches!(simulator.decision(), DecisionContext::ChooseCards { .. }),
        "the fifth pick finished the answer"
    );
    assert_eq!(
        simulator.state().run_player.deck.len(),
        deck_before - 5,
        "the tooth took its five"
    );
}

/// A gnarled hammer puts Sharp on up to three cards of the deck: a screen that
/// takes a range, with a confirm button and a way to take nothing. The confirm
/// comes onto the list once a card is picked, and answers with the selection
/// as it stands.
#[test]
fn a_gnarled_hammer_confirms_what_is_picked_so_far() {
    let mut simulator = run_at_a_chest("RELIC.GNARLED_HAMMER", 23);
    take_the_relic(&mut simulator);

    let (choice_id, minimum, maximum, candidates, _) = the_screen(&simulator);
    assert_eq!((minimum, maximum), (0, 3), "up to three");
    assert!(candidates > 3, "more than the hammer can take");
    assert_eq!(
        offered_picks(&simulator).len(),
        candidates,
        "a pick per card"
    );
    assert!(!lays_out_a_whole_answer(&simulator), "and no set of them");
    assert!(
        !offers_a_confirm(&simulator),
        "with nothing picked there is nothing to confirm"
    );
    assert!(
        simulator.legal_actions().contains(&Action::ChooseCards {
            choice_id,
            cards: Vec::new()
        }),
        "taking nothing is on the list"
    );

    pick_one(&mut simulator);
    assert_eq!(offered_picks(&simulator).len(), candidates - 1);
    assert!(
        offers_a_confirm(&simulator),
        "one card picked can be confirmed"
    );
    simulator
        .step(Action::ConfirmCards { choice_id })
        .expect("the confirm applies");
    assert!(
        !matches!(simulator.decision(), DecisionContext::ChooseCards { .. }),
        "the confirm finished the answer"
    );
    assert_eq!(
        enchanted(&simulator),
        1,
        "the one card picked was enchanted"
    );
}

/// The pick that fills a ranged screen finishes it without a confirm, and
/// taking nothing part-way through enchants nothing.
#[test]
fn a_gnarled_hammer_fills_on_the_third_pick_or_takes_nothing() {
    let mut simulator = run_at_a_chest("RELIC.GNARLED_HAMMER", 23);
    take_the_relic(&mut simulator);
    for _ in 0..3 {
        pick_one(&mut simulator);
    }
    assert!(
        !matches!(simulator.decision(), DecisionContext::ChooseCards { .. }),
        "the third pick filled the screen"
    );
    assert_eq!(enchanted(&simulator), 3);

    let mut simulator = run_at_a_chest("RELIC.GNARLED_HAMMER", 23);
    take_the_relic(&mut simulator);
    let (choice_id, _) = the_candidates(&simulator);
    pick_one(&mut simulator);
    pick_one(&mut simulator);
    simulator
        .step(Action::ChooseCards {
            choice_id,
            cards: Vec::new(),
        })
        .expect("taking nothing is still an answer");
    assert!(
        !matches!(simulator.decision(), DecisionContext::ChooseCards { .. }),
        "the screen closed"
    );
    assert_eq!(enchanted(&simulator), 0, "nothing was enchanted");
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
/// picked. The action list never lays that set out, and replay answers the
/// screen with it all the same.
#[test]
fn a_recorded_hug_answers_its_screen_in_one_record() {
    let mut simulator = run_at_a_chest("RELIC.BIIIG_HUG", 23);
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
        !lays_out_a_whole_answer(&simulator),
        "the list carries no whole set"
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
    let mut simulator = run_at_a_chest("RELIC.BIIIG_HUG", 23);
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

/// A beautiful bracelet asks for three of twenty-three. The list works the
/// screen a pick at a time and carries no set of three in any order, while a
/// record that picked them in its own order is replayed in that order, which
/// is the order the build hands them back in.
#[test]
fn a_recorded_bracelet_keeps_the_order_it_was_picked_in() {
    let mut simulator = run_at_a_chest("RELIC.BEAUTIFUL_BRACELET", 23);
    take_the_relic(&mut simulator);
    let (_, minimum, maximum, _, _) = the_screen(&simulator);
    assert_eq!(
        (minimum, maximum),
        (3, 3),
        "the bracelet asks for exactly three"
    );
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
        "the list does not carry the set"
    );
    assert!(!lays_out_a_whole_answer(&simulator), "nor any other");

    let (after, applied) = replayed(simulator, &recorded_answer(choice_id, &picked));
    assert_eq!(applied, recorded, "the record's own order");
    assert!(
        !matches!(after.decision(), DecisionContext::ChooseCards { .. }),
        "the record finished the answer"
    );
}

/// A recorded hammer that enchanted one card names that one card, which is
/// also the shape a single pick has on the list — and a recorded answer is a
/// finished one, not a pick: the gate takes it whole and the screen closes.
#[test]
fn a_recorded_single_card_on_a_ranged_screen_finishes_it() {
    let mut simulator = run_at_a_chest("RELIC.GNARLED_HAMMER", 23);
    take_the_relic(&mut simulator);
    let (choice_id, candidates) = the_candidates(&simulator);
    let picked = vec![candidates[5].clone()];

    let (after, applied) = replayed(simulator, &recorded_answer(choice_id, &picked));
    assert_eq!(
        applied,
        Action::ChooseCards {
            choice_id,
            cards: picked
        }
    );
    assert!(
        !matches!(after.decision(), DecisionContext::ChooseCards { .. }),
        "the record finished the answer"
    );
    assert_eq!(enchanted(&after), 1);
}
