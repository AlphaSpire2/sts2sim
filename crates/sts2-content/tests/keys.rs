//! The two canonical keys and the boundary between them: `state_key` covers
//! everything, `observation_key` covers exactly what a policy can see. A
//! search client keys exact transpositions on the first and information-set
//! transpositions on the second, and the schema types it holds must cross
//! threads and serialize.

use sts2_engine::{
    Action, AgentObservation, CardFingerprint, PileName, ScenarioBuilder, Simulator,
};

const SEED: &str = "KEYS3AB5CD";

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().unwrap()
}

fn card(model: &str) -> CardFingerprint {
    CardFingerprint::base(id(model))
}

/// One combat against a beetle, with the draw pile laid out in the order the
/// ids are given: same cards, same everything visible, and only the hidden
/// order left to the caller.
fn beetle_fight(draw: &[(u64, &str)]) -> Simulator {
    beetle_fight_shuffled(draw, 0)
}

fn beetle_fight_shuffled(draw: &[(u64, &str)], shuffle: u64) -> Simulator {
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(1, 1)
        .shuffle_counter(shuffle)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .card(PileName::Hand, 10, card("CARD.STRIKE_IRONCLAD"), 1);
    for (card_id, model) in draw {
        scenario = scenario.card(PileName::Draw, *card_id, card(model), 1);
    }
    scenario.build(sts2_content::standard_registry()).unwrap()
}

#[test]
fn a_hidden_stream_position_moves_the_state_key_and_not_the_observation_key() {
    // The same fight with the shuffle stream at two different positions: the
    // next shuffle will land differently, and nothing a player can see says
    // so. (The run deck itself is visible, in acquisition order, so a deck
    // permutation is not a hidden difference.)
    let one = beetle_fight_shuffled(&[(20, "CARD.DEFEND_IRONCLAD"), (21, "CARD.BASH")], 0);
    let two = beetle_fight_shuffled(&[(20, "CARD.DEFEND_IRONCLAD"), (21, "CARD.BASH")], 5);
    assert_ne!(
        one.state_key().unwrap(),
        two.state_key().unwrap(),
        "a stream position is state"
    );
    assert_eq!(
        one.observation_key().unwrap(),
        two.observation_key().unwrap(),
        "a stream position is not observable"
    );
}

#[test]
fn a_hidden_draw_arrangement_moves_the_state_key_and_not_the_observation_key() {
    // Two same-looking Defends with their hidden positions swapped: which
    // copy sits where is state, and nothing a player can see says so. (The
    // pile's *contents* are visible — the observation carries them in the
    // view screen's canonical order — and permuting distinct cards here would
    // also permute the scenario's derived run deck, which is legitimately
    // visible, so the hidden difference is isolated on identical prints.)
    let one = beetle_fight(&[(20, "CARD.DEFEND_IRONCLAD"), (21, "CARD.DEFEND_IRONCLAD")]);
    let two = beetle_fight(&[(21, "CARD.DEFEND_IRONCLAD"), (20, "CARD.DEFEND_IRONCLAD")]);
    assert_ne!(
        one.state_key().unwrap(),
        two.state_key().unwrap(),
        "a draw arrangement is state"
    );
    assert_eq!(
        one.observation_key().unwrap(),
        two.observation_key().unwrap(),
        "a draw arrangement is not observable"
    );
}

/// A fight built with the hand laid out in the order the ids are given.
fn beetle_fight_holding(hand: &[(u64, &str)]) -> Simulator {
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(1, 1)
        .shuffle_counter(0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE");
    for (card_id, model) in hand {
        scenario = scenario.card(PileName::Hand, *card_id, card(model), 1);
    }
    scenario.build(sts2_content::standard_registry()).unwrap()
}

#[test]
fn the_hands_order_and_its_copies_are_state_and_not_part_of_the_key() {
    // The same two cards held in either order: a player sees the order, but
    // no rule reads it and a search's action classes already discard the
    // position a play names, so the key folds it. Which copy of two
    // same-looking Strikes sits where folds the same way.
    // (The swap is made on the built state: the scenario derives the visible
    // run deck from the cards in the order they are given, and a deck
    // permutation is a different sight.)
    let strike_first =
        beetle_fight_holding(&[(10, "CARD.STRIKE_IRONCLAD"), (11, "CARD.DEFEND_IRONCLAD")]);
    let defend_first = edited(&strike_first, |state| {
        state.combat.as_mut().unwrap().player.piles.hand.swap(0, 1);
    });
    assert_ne!(
        strike_first.state_key().unwrap(),
        defend_first.state_key().unwrap(),
        "the hand's order is state"
    );
    assert_eq!(
        strike_first.observation_key().unwrap(),
        defend_first.observation_key().unwrap(),
        "the hand's order is not part of the key"
    );
    assert_ne!(
        strike_first.agent_observation().hand,
        defend_first.agent_observation().hand,
        "the observation itself still shows the order"
    );

    let one_way =
        beetle_fight_holding(&[(10, "CARD.STRIKE_IRONCLAD"), (12, "CARD.STRIKE_IRONCLAD")]);
    let other_way =
        beetle_fight_holding(&[(12, "CARD.STRIKE_IRONCLAD"), (10, "CARD.STRIKE_IRONCLAD")]);
    assert_ne!(one_way.state_key().unwrap(), other_way.state_key().unwrap());
    assert_eq!(
        one_way.observation_key().unwrap(),
        other_way.observation_key().unwrap(),
        "which copy sits where is not part of the key"
    );
}

#[test]
fn a_different_hand_is_a_different_key() {
    // The fold is over order and copies only: hold a different card and the
    // key moves, hold one card more and it moves.
    let strike_and_defend =
        beetle_fight_holding(&[(10, "CARD.STRIKE_IRONCLAD"), (11, "CARD.DEFEND_IRONCLAD")]);
    let two_strikes =
        beetle_fight_holding(&[(10, "CARD.STRIKE_IRONCLAD"), (11, "CARD.STRIKE_IRONCLAD")]);
    let three_cards = beetle_fight_holding(&[
        (10, "CARD.STRIKE_IRONCLAD"),
        (11, "CARD.DEFEND_IRONCLAD"),
        (12, "CARD.DEFEND_IRONCLAD"),
    ]);
    assert_ne!(
        strike_and_defend.observation_key().unwrap(),
        two_strikes.observation_key().unwrap(),
        "a different card in hand is a different sight"
    );
    assert_ne!(
        strike_and_defend.observation_key().unwrap(),
        three_cards.observation_key().unwrap(),
        "a bigger hand is a different sight"
    );
}

#[test]
fn an_ordered_pile_keeps_its_order_and_loses_its_copies() {
    // The discard pile lies in the order it was filled and a rule may read
    // the top of it, so its order stays in the key; which copy of a
    // same-looking pair lies where does not.
    let with = |discard: &[(u64, &str)]| {
        let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .player_hp(80, 80)
            .energy(3, 3)
            .turn(1, 1)
            .shuffle_counter(0)
            .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
            .card(PileName::Hand, 10, card("CARD.STRIKE_IRONCLAD"), 1);
        for (card_id, model) in discard {
            scenario = scenario.card(PileName::Discard, *card_id, card(model), 1);
        }
        scenario.build(sts2_content::standard_registry()).unwrap()
    };
    let defend_then_bash = with(&[(20, "CARD.DEFEND_IRONCLAD"), (21, "CARD.BASH")]);
    let bash_then_defend = with(&[(21, "CARD.BASH"), (20, "CARD.DEFEND_IRONCLAD")]);
    assert_ne!(
        defend_then_bash.observation_key().unwrap(),
        bash_then_defend.observation_key().unwrap(),
        "the order a discard pile was filled in is part of the key"
    );
    let copies_one_way = with(&[(20, "CARD.DEFEND_IRONCLAD"), (21, "CARD.DEFEND_IRONCLAD")]);
    let copies_other_way = with(&[(21, "CARD.DEFEND_IRONCLAD"), (20, "CARD.DEFEND_IRONCLAD")]);
    assert_ne!(
        copies_one_way.state_key().unwrap(),
        copies_other_way.state_key().unwrap()
    );
    assert_eq!(
        copies_one_way.observation_key().unwrap(),
        copies_other_way.observation_key().unwrap(),
        "which copy lies where in the discard is not part of the key"
    );
}

#[test]
fn the_draw_pile_reads_as_contents_and_not_as_an_order() {
    // Distinct cards laid down in the two orders read back as the same
    // canonical contents: the field carries what the pile holds, never what
    // comes off first.
    let one = beetle_fight(&[(20, "CARD.DEFEND_IRONCLAD"), (21, "CARD.BASH")]);
    let two = beetle_fight(&[(21, "CARD.BASH"), (20, "CARD.DEFEND_IRONCLAD")]);
    assert_eq!(
        one.agent_observation().draw_pile,
        two.agent_observation().draw_pile,
        "the contents read the same either way"
    );
    assert_eq!(
        one.agent_observation().draw_pile.len(),
        2,
        "both cards are visible contents"
    );
}

#[test]
fn an_exhausted_card_is_part_of_what_a_player_sees() {
    // Two fights alike except for a card in the exhaust pile: the pile is
    // open to the player at any time, so the keys must differ.
    let bare = beetle_fight(&[(20, "CARD.DEFEND_IRONCLAD")]);
    let exhausted = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(1, 1)
        .shuffle_counter(0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .card(PileName::Hand, 10, card("CARD.STRIKE_IRONCLAD"), 1)
        .card(PileName::Draw, 20, card("CARD.DEFEND_IRONCLAD"), 1)
        .card(PileName::Exhaust, 30, card("CARD.BASH"), 1)
        .build(sts2_content::standard_registry())
        .unwrap();
    assert_ne!(
        bare.observation_key().unwrap(),
        exhausted.observation_key().unwrap(),
        "the exhaust pile is observable"
    );
    assert_eq!(exhausted.agent_observation().exhaust.len(), 1);
}

#[test]
fn a_relic_counter_is_part_of_what_a_player_sees() {
    // The same fight with the same relic at two counts: the count is on the
    // screen, so the keys must differ.
    let charged = |counter: i32| {
        ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .player_hp(80, 80)
            .energy(3, 3)
            .turn(1, 1)
            .shuffle_counter(0)
            .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
            .card(PileName::Hand, 10, card("CARD.STRIKE_IRONCLAD"), 1)
            .relic_instances(vec![sts2_engine::RelicInstance {
                counter,
                ..sts2_engine::RelicInstance::new(id("RELIC.RAZOR_TOOTH"))
            }])
            .build(sts2_content::standard_registry())
            .unwrap()
    };
    assert_ne!(
        charged(0).observation_key().unwrap(),
        charged(3).observation_key().unwrap(),
        "a relic's count is observable"
    );
}

#[test]
fn a_stewed_strike_does_not_look_like_a_plain_one() {
    // A Soldier's Stew marks every Strike with a replay: the mark is on the
    // card's face, in the hand and in the draw pile's contents alike, and
    // the key moves with it.
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(1, 1)
        .shuffle_counter(0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .card(PileName::Hand, 10, card("CARD.STRIKE_IRONCLAD"), 1)
        .card(PileName::Draw, 20, card("CARD.STRIKE_IRONCLAD"), 1)
        .potions(vec![Some(id("POTION.SOLDIERS_STEW")), None])
        .build(sts2_content::standard_registry())
        .unwrap();
    let before = simulator.observation_key().unwrap();
    let drink = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::UsePotion { .. }))
        .cloned()
        .expect("the stew is drinkable");
    simulator.step(drink).unwrap();
    let observation = simulator.agent_observation();
    assert_eq!(
        observation.hand[0].face.replay_count, 1,
        "the hand's strike wears the mark"
    );
    assert_eq!(
        observation.draw_pile[0].face.replay_count, 1,
        "the draw pile's contents wear it too"
    );
    assert_ne!(
        simulator.observation_key().unwrap(),
        before,
        "a marked deck is a different sight"
    );
}

#[test]
fn a_visible_change_moves_the_observation_key() {
    let mut simulator = beetle_fight(&[(20, "CARD.DEFEND_IRONCLAD")]);
    let before = simulator.observation_key().unwrap();
    let play = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::PlayCard { .. }))
        .cloned()
        .expect("a strike in hand is playable");
    simulator.step(play).unwrap();
    assert_ne!(
        simulator.observation_key().unwrap(),
        before,
        "a played card is visible"
    );
}

#[test]
fn the_search_schema_types_cross_threads_and_serialize() {
    fn sendable<T: Send>() {}
    sendable::<Simulator>();
    sendable::<Action>();
    sendable::<AgentObservation>();

    let simulator = beetle_fight(&[(20, "CARD.DEFEND_IRONCLAD")]);
    let observation = simulator.agent_observation();
    let json = serde_json::to_string(&observation).unwrap();
    let back: AgentObservation = serde_json::from_str(&json).unwrap();
    assert_eq!(back, observation, "the observation round-trips");
    let action = simulator.legal_actions().first().cloned().unwrap();
    let json = serde_json::to_string(&action).unwrap();
    let back: Action = serde_json::from_str(&json).unwrap();
    assert_eq!(back, action, "an action round-trips");
}

// --- observation v6: what the bump added to the boundary -------------------
//
// Each of these is the same test in both directions: a change the player
// watches happen must move the observation key, and a change the player never
// sees must not. The v4 and v5 bumps were each found
// by asking it of a screen; v6 asks it of the reward screen's offers, the
// act's map, the move machine's history, and the card's enchantment.

const WALK_SEED: &str = "NLD6VZXP94";

fn fresh_run() -> Simulator {
    let preset = sts2_core::UnlockPresetManifest::pinned().unwrap();
    let character = "CHARACTER.IRONCLAD".parse().unwrap();
    sts2_content::standard_run_on_preset_at(WALK_SEED, &character, &preset, 0).unwrap()
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

fn walk_until(mut simulator: Simulator, stop: impl Fn(&Simulator) -> bool) -> Simulator {
    for _ in 0..300 {
        if stop(&simulator) {
            return simulator;
        }
        let action = next_action(&simulator);
        simulator.step_quietly(&action).unwrap();
    }
    panic!("the seed reaches the stop inside three hundred decisions");
}

/// The seeded run walked to its first open reward screen.
fn first_reward_screen() -> Simulator {
    walk_until(fresh_run(), |simulator| {
        matches!(
            simulator.decision(),
            sts2_engine::DecisionContext::Rewards { .. }
        )
    })
}

/// The seeded run walked to its first in-combat decision.
fn first_combat() -> Simulator {
    walk_until(fresh_run(), |simulator| simulator.state().combat.is_some())
}

/// The same state with one edit applied to it, rebuilt into a simulator.
fn edited(simulator: &Simulator, edit: impl FnOnce(&mut sts2_engine::GameState)) -> Simulator {
    let mut state = simulator.state().clone();
    edit(&mut state);
    Simulator::from_scenario(state, sts2_content::standard_registry()).unwrap()
}

fn open_reward_set(state: &mut sts2_engine::GameState) -> &mut sts2_engine::RewardSet {
    if let Some(combat) = state.combat.as_mut()
        && combat.reward_set.is_some()
    {
        return combat.reward_set.as_mut().expect("the screen is open");
    }
    state
        .run
        .as_mut()
        .expect("a reward screen stands inside a run")
        .reward_set
        .as_mut()
        .expect("the screen is open")
}

#[test]
fn a_reward_screens_offers_are_part_of_what_a_player_sees() {
    // The gap the offers were standing in for: a `Rewards` decision named its
    // set id and its player, so two screens offering different gold shared an
    // observation key. They are two different sights.
    let screen = first_reward_screen();
    let observation = screen.agent_observation();
    let sts2_engine::DecisionContext::Rewards { offers, .. } = &observation.decision else {
        panic!("the walk stands on a reward screen");
    };
    assert!(
        !offers.is_empty(),
        "an open reward screen carries what it offers"
    );
    let richer = edited(&screen, |state| {
        let set = open_reward_set(state);
        for reward in &mut set.rewards {
            reward.fingerprint.gold_amount += 25;
        }
    });
    assert_ne!(
        screen.observation_key().unwrap(),
        richer.observation_key().unwrap(),
        "a screen offering more gold is a different screen"
    );

    // And the cards on a card offer, likewise.
    let mut swapped_any = false;
    let swapped = edited(&screen, |state| {
        let set = open_reward_set(state);
        for reward in &mut set.rewards {
            if let Some(card) = reward.fingerprint.offered_cards.first_mut() {
                *card = CardFingerprint::base(id("CARD.BASH"));
                swapped_any = true;
            }
        }
    });
    if swapped_any {
        assert_ne!(
            screen.observation_key().unwrap(),
            swapped.observation_key().unwrap(),
            "a screen offering another card is a different screen"
        );
    }
}

#[test]
fn a_reward_offer_carries_the_screen_and_not_the_recipe() {
    // The other half of the fairness test. `RewardFingerprint` also holds
    // what the offer was rolled *from* — its pools, its odds, the creation a
    // reroll would re-run — and none of that is drawn anywhere. Two screens
    // showing the same cards are one sight however they were rolled, so the
    // key must not split them.
    let screen = first_reward_screen();
    let relabelled = edited(&screen, |state| {
        let set = open_reward_set(state);
        for reward in &mut set.rewards {
            reward.fingerprint.card_pools = vec![id("CARD_POOL.INVENTED")];
            reward.fingerprint.rarity_odds = Some(sts2_engine::RarityOdds::Shop);
        }
    });
    assert_ne!(
        screen.state_key().unwrap(),
        relabelled.state_key().unwrap(),
        "the recipe is state"
    );
    assert_eq!(
        screen.observation_key().unwrap(),
        relabelled.observation_key().unwrap(),
        "the recipe is not on the screen"
    );
}

#[test]
fn the_acts_map_and_its_boss_preview_are_part_of_what_a_player_sees() {
    // The map screen draws the whole act — every icon, every path, the walked
    // trail — and the boss point wears the art of the act's own boss
    // encounter, the ancient point its ancient's icon.
    let simulator = first_combat();
    let observation = simulator.agent_observation();
    let map = observation.map.as_ref().expect("a run carries its map");
    assert!(map.points.len() > 5, "the act laid out a map");
    assert!(
        map.points.iter().any(|point| point.visited),
        "the walked trail is on the screen"
    );

    let other_boss = edited(&simulator, |state| {
        let run = state.run.as_mut().unwrap();
        let act = run.current_act;
        run.acts[act].boss = id("ENCOUNTER.KAISER_CRAB_BOSS");
    });
    assert_ne!(
        simulator.observation_key().unwrap(),
        other_boss.observation_key().unwrap(),
        "the boss the map previews is on the screen"
    );

    let other_ancient = edited(&simulator, |state| {
        let run = state.run.as_mut().unwrap();
        let act = run.current_act;
        run.acts[act].ancient = id("EVENT.ANCIENT_INVENTED");
    });
    assert_ne!(
        simulator.observation_key().unwrap(),
        other_ancient.observation_key().unwrap(),
        "the ancient the map previews is on the screen"
    );

    let other_shape = edited(&simulator, |state| {
        let run = state.run.as_mut().unwrap();
        let act = run.current_act;
        run.maps[act].points[0].point_type = sts2_engine::MapPointType::Shop;
    });
    assert_ne!(
        simulator.observation_key().unwrap(),
        other_shape.observation_key().unwrap(),
        "a point's icon is on the screen"
    );
}

#[test]
fn what_stands_behind_an_unwalked_icon_is_still_hidden() {
    // The map is on the screen; the room behind an unwalked point is not.
    // Adding one must not move the observation key, or the map field would
    // have smuggled the belief layer's own erasure into the observation.
    let simulator = first_combat();
    let pinned = edited(&simulator, |state| {
        let run = state.run.as_mut().unwrap();
        let act = run.current_act;
        let coord = run.maps[act]
            .points
            .iter()
            .find(|point| !run.visited.contains(&point.coord))
            .expect("some point is unwalked")
            .coord;
        run.rooms[act].push(sts2_engine::RoomAt {
            coord,
            definition: sts2_engine::RoomDefinition::Combat(sts2_engine::CombatRoomDefinition {
                encounter_model: id("ENCOUNTER.NIBBITS_WEAK"),
                enemies: vec![sts2_engine::EnemySpawn::rolled(id("MONSTER.NIBBIT"))],
                rewards: Vec::new(),
                generated_rewards: None,
            }),
        });
    });
    assert_ne!(
        simulator.state_key().unwrap(),
        pinned.state_key().unwrap(),
        "a placed room is state"
    );
    assert_eq!(
        simulator.observation_key().unwrap(),
        pinned.observation_key().unwrap(),
        "what is behind an unwalked icon is not on the screen"
    );
}

#[test]
fn a_watched_move_history_is_part_of_what_a_player_sees_and_a_private_counter_is_not() {
    // A creature's two logs are watchable: a state is entered where its
    // intent goes up and performed where the move runs. The numbers the
    // monster's own body writes for itself are not.
    let simulator = first_combat();
    let observation = simulator.agent_observation();
    let enemy = observation
        .creatures
        .iter()
        .find(|creature| creature.side != sts2_engine::CombatSide::Player)
        .expect("a fight has an enemy");
    assert!(
        !enemy.move_history.recent_states.is_empty(),
        "a creature showing an intent has stood on its state"
    );
    assert_eq!(
        enemy.move_history.recent_states.last().map(String::as_str),
        enemy.intent.as_ref().map(|intent| intent.move_id.as_str()),
        "the last state entered is the move the intent advertises"
    );

    let seen_more = edited(&simulator, |state| {
        let combat = state.combat.as_mut().unwrap();
        for creature in &mut combat.creatures {
            if creature.side != sts2_engine::CombatSide::Player {
                let repeat = creature.state_log[0].clone();
                creature.state_log.insert(0, repeat);
            }
        }
    });
    assert_ne!(
        simulator.observation_key().unwrap(),
        seen_more.observation_key().unwrap(),
        "a move the player watched twice is a fight they have seen more of"
    );

    let private = edited(&simulator, |state| {
        let combat = state.combat.as_mut().unwrap();
        for creature in &mut combat.creatures {
            if creature.side != sts2_engine::CombatSide::Player {
                creature.locals.insert("PressureGunDamage".to_owned(), 40);
            }
        }
    });
    assert_ne!(
        simulator.state_key().unwrap(),
        private.state_key().unwrap(),
        "a monster's own note is state"
    );
    assert_eq!(
        simulator.observation_key().unwrap(),
        private.observation_key().unwrap(),
        "a monster's own note is not on the screen — only what it makes the \
         intent show, which the intent already shows"
    );
}

#[test]
fn an_enchantments_grown_damage_shows_and_its_private_flag_does_not() {
    // The extra damage a Momentum has grown reaches the damage number the
    // card draws, so the face carries it. Regret's hand count is not one of
    // the card's displayed variables and adds no card text, so it stays where
    // it is.
    let simulator = beetle_fight(&[(20, "CARD.DEFEND_IRONCLAD")]);
    let grown = edited(&simulator, |state| {
        let combat = state.combat.as_mut().unwrap();
        for card in &mut combat.player.piles.hand {
            card.enchantment_locals.insert("ExtraDamage".to_owned(), 6);
        }
    });
    assert_ne!(
        simulator.observation_key().unwrap(),
        grown.observation_key().unwrap(),
        "a momentum-grown attack shows a bigger number"
    );
    assert_eq!(
        grown.agent_observation().hand[0]
            .face
            .enchantment_damage_bonus,
        6
    );

    let noted = edited(&simulator, |state| {
        let combat = state.combat.as_mut().unwrap();
        for card in &mut combat.player.piles.hand {
            card.locals.insert("CardsInHand".to_owned(), 7);
        }
    });
    assert_ne!(
        simulator.state_key().unwrap(),
        noted.state_key().unwrap(),
        "a card's own note is state"
    );
    assert_eq!(
        simulator.observation_key().unwrap(),
        noted.observation_key().unwrap(),
        "a card's own note is nowhere on its face"
    );
}

#[test]
fn a_spent_enchantment_shows_as_spent() {
    // A Glam greys itself out on the play it is used, which is the display
    // its `UsedThisCombat` local comes to.
    let simulator = beetle_fight(&[(20, "CARD.DEFEND_IRONCLAD")]);
    let spent = edited(&simulator, |state| {
        let combat = state.combat.as_mut().unwrap();
        for card in &mut combat.player.piles.hand {
            card.enchantment_status = sts2_engine::EnchantmentStatus::Disabled;
        }
    });
    assert_ne!(
        simulator.observation_key().unwrap(),
        spent.observation_key().unwrap(),
        "a spent enchantment is a different card face"
    );
}

/// `observation_key` is built on a borrowed mirror of `agent_observation`;
/// the mirror and the owned observation must stay byte-for-byte alike, in
/// and out of combat, or the key stops naming what a policy can see.
#[test]
fn the_borrowed_observation_hashes_like_the_owned_one() {
    use sha2::{Digest as _, Sha256};
    let preset = sts2_core::UnlockPresetManifest::pinned().unwrap();
    let character = "CHARACTER.IRONCLAD".parse().unwrap();
    let mut simulator =
        sts2_content::standard_run_on_preset_at("NLD6VZXP94", &character, &preset, 0).unwrap();
    for _ in 0..60 {
        let owned: [u8; 32] =
            Sha256::digest(serde_json::to_vec(&simulator.agent_observation()).unwrap()).into();
        assert_eq!(
            simulator.observation_mirror_digest().unwrap(),
            owned,
            "the borrowed mirror drifted from the owned observation"
        );
        if simulator.state().terminal.is_some() {
            break;
        }
        let action = simulator
            .legal_actions()
            .iter()
            .find(|action| matches!(action, Action::ChooseCards { cards, .. } if !cards.is_empty()))
            .or_else(|| simulator.legal_actions().first())
            .cloned()
            .expect("a live run offers an action");
        simulator.step(action).unwrap();
    }
}

/// The visible history stays bounded however long the run: a fixed tail of
/// recent entries, everything older folded into a digest that still tells
/// different pasts apart.
#[test]
fn the_visible_history_stays_bounded_and_the_digest_tells_pasts_apart() {
    use sts2_engine::{VisibleHistory, VisibleHistoryEntry};
    let entry = |sequence: u64, event: &str| VisibleHistoryEntry {
        sequence,
        event: event.to_owned(),
    };
    let mut plain = VisibleHistory::default();
    let mut divergent = VisibleHistory::default();
    for index in 0..200 {
        plain.push(entry(index, "combat_started"));
        divergent.push(entry(
            index,
            if index == 0 {
                "room_entered"
            } else {
                "combat_started"
            },
        ));
    }
    assert_eq!(plain.recent.len(), VisibleHistory::RECENT_CAP);
    assert_eq!(plain.len(), 200);
    assert_eq!(plain.recent, divergent.recent, "the tails agree");
    assert_ne!(
        plain.folded_digest, divergent.folded_digest,
        "a past that differed only in folded entries still reads different"
    );
}

/// The key writes its view as compact bytes rather than JSON; it must still
/// tell apart exactly the views JSON told apart. Seeded walks of every
/// character, with a few hidden-state samples at each fight decision for
/// views that repeat, are keyed both ways, and the two keys must stand in
/// one-to-one correspondence.
#[test]
fn the_compact_key_separates_exactly_what_the_json_view_does() {
    use std::collections::HashMap;
    use sts2_engine::{BeliefState, ObservationKey};

    let preset = sts2_core::UnlockPresetManifest::pinned().unwrap();
    let mut by_key: HashMap<ObservationKey, [u8; 32]> = HashMap::new();
    let mut by_json: HashMap<[u8; 32], ObservationKey> = HashMap::new();
    let mut keyed = 0_usize;
    let mut check = |simulator: &Simulator| {
        let key = simulator.observation_key().unwrap();
        let json = simulator.observation_key_json_digest().unwrap();
        assert_eq!(
            *by_key.entry(key).or_insert(json),
            json,
            "one key, two views"
        );
        assert_eq!(
            *by_json.entry(json).or_insert(key),
            key,
            "one view, two keys"
        );
        keyed += 1;
    };
    for (character, seed) in [
        ("CHARACTER.IRONCLAD", "NLD6VZXP94"),
        ("CHARACTER.SILENT", "KEYS3AB5CD"),
        ("CHARACTER.DEFECT", "Q7M2XK4PLA"),
        ("CHARACTER.REGENT", "B8TZ5R1HNW"),
        ("CHARACTER.NECROBINDER", "W3C9D6FJ2E"),
    ] {
        let character = character.parse().unwrap();
        let mut simulator =
            sts2_content::standard_run_on_preset_at(seed, &character, &preset, 0).unwrap();
        // A fixed walk that does not always take the first action, so the
        // views cover more than one line of play.
        let mut pick = 0x9E37_79B9_u64;
        for step in 0..150 {
            check(&simulator);
            if simulator.state().combat.is_some()
                && step % 3 == 0
                && let Ok(belief) = BeliefState::from_simulator(&simulator)
            {
                for rollout in 0..3 {
                    check(&belief.sample(11, rollout).unwrap());
                }
            }
            if simulator.state().terminal.is_some() {
                break;
            }
            let legal = simulator.legal_actions();
            pick = pick.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let action = legal
                .iter()
                .find(|action| matches!(action, Action::ChooseCards { cards, .. } if !cards.is_empty()))
                .filter(|_| pick % 2 == 0)
                .unwrap_or(&legal[(pick >> 33) as usize % legal.len()])
                .clone();
            simulator.step_quietly(&action).unwrap();
        }
    }
    assert!(by_key.len() > 200, "the walks reach {} views", by_key.len());
    assert!(
        keyed > by_key.len(),
        "some views repeat, so equality is checked too"
    );
}
