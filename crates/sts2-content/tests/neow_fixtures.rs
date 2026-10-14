/// Every relic Neow can offer a single-player run, taken.
///
/// Every ancient's relic option is the same body — take the relic, then finish
/// — so what one costs is whatever the relic does on arrival. Every one of
/// them is taken to the end, and the room the opening page stands in is then
/// left or held open by a screen the option itself put up.
#[test]
fn every_neow_option_a_single_player_run_can_be_offered_is_taken() {
    // Every option Neow can offer, less the massive scroll, whose
    // availability turns on the number of players.
    const OFFERED: [&str; 27] = [
        "CURSED_PEARL",
        "HEFTY_TABLET",
        "LARGE_CAPSULE",
        "LEAFY_POULTICE",
        "NEOWS_BONES",
        "PRECARIOUS_SHEARS",
        "SILKEN_TRESS",
        "SILVER_CRUCIBLE",
        "ARCANE_SCROLL",
        "BOOMING_CONCH",
        "FISHING_ROD",
        "GOLDEN_PEARL",
        "KALEIDOSCOPE",
        "LEAD_PAPERWEIGHT",
        "LOST_COFFER",
        "NEOWS_TORMENT",
        "NEW_LEAF",
        "PHIAL_HOLSTER",
        "PRECISE_SCISSORS",
        "SCROLL_BOXES",
        "WINGED_BOOTS",
        "LAVA_ROCK",
        "NEOWS_TALISMAN",
        "NUTRITIOUS_OYSTER",
        "POMANDER",
        "SMALL_CAPSULE",
        "STONE_HUMIDIFIER",
    ];
    for relic in OFFERED {
        let simulator =
            sts2_content::standard_ironclad_run("N3Z9EE2CXB").expect("the run generates");
        let option_id = format!("NEOW.pages.INITIAL.options.{relic}");
        let mut state = simulator.state().clone();
        if let Some(run) = state.run.as_mut() {
            let option = sts2_engine::EventOption {
                option_id: option_id.clone(),
                label: option_id.clone(),
                effects: vec![sts2_engine::EventEffect::ObtainRelic(
                    format!("RELIC.{relic}").parse().expect("static model ID"),
                )],
                is_proceed: false,
                was_chosen: false,
                locked: false,
            };
            if let Some(sts2_engine::ActiveRoom::Event { options, .. }) = run.active_room.as_mut() {
                *options = vec![option.clone()];
            }
            if let Some(begun) = run.begun_event.as_mut() {
                begun.options = vec![option];
            }
        }
        let mut simulator =
            sts2_engine::Simulator::from_generated_state(state, sts2_content::standard_registry())
                .expect("the run stands back up");
        simulator
            .step(sts2_engine::Action::ChooseEvent {
                index: 0,
                option_id: option_id.clone(),
            })
            .unwrap_or_else(|error| panic!("{relic} stopped: {error:?}"));
        assert!(
            simulator
                .state()
                .run_player
                .relics
                .iter()
                .any(|held| held.model_id.entry() == relic),
            "{relic} was not obtained"
        );
    }
}

/// Neow's bones deal two of Neow's own options and charge a curse for them.
///
/// The offer is one set the player may not walk away from, and the build waits
/// on it, so the curse joins the deck once the set is done with rather than
/// while
/// it is standing.
#[test]
fn neows_bones_offer_two_of_neows_own_and_charge_a_curse() {
    let simulator = sts2_content::standard_ironclad_run("N3Z9EE2CXB").expect("the run generates");
    let mut state = simulator.state().clone();
    let option_id = "NEOW.pages.INITIAL.options.NEOWS_BONES".to_owned();
    let option = sts2_engine::EventOption {
        option_id: option_id.clone(),
        label: option_id.clone(),
        effects: vec![sts2_engine::EventEffect::ObtainRelic(
            "RELIC.NEOWS_BONES".parse().expect("static model ID"),
        )],
        is_proceed: false,
        was_chosen: false,
        locked: false,
    };
    if let Some(run) = state.run.as_mut() {
        if let Some(sts2_engine::ActiveRoom::Event { options, .. }) = run.active_room.as_mut() {
            *options = vec![option.clone()];
        }
        if let Some(begun) = run.begun_event.as_mut() {
            begun.options = vec![option];
        }
    }
    let deck_before = state.run_player.deck.len();
    let mut simulator =
        sts2_engine::Simulator::from_generated_state(state, sts2_content::standard_registry())
            .expect("the run stands back up");
    simulator
        .step(sts2_engine::Action::ChooseEvent {
            index: 0,
            option_id,
        })
        .expect("taking the bones applies");
    let set = simulator
        .state()
        .run
        .as_ref()
        .and_then(|run| run.reward_set.as_ref())
        .expect("the two relics are offered");
    assert_eq!(set.rewards.len(), 2);
    assert!(set.disallow_skipping);
    assert!(
        set.rewards
            .iter()
            .all(|reward| reward.fingerprint.reward_type == "relic")
    );
    // The curse waits behind the set.
    assert_eq!(simulator.state().run_player.deck.len(), deck_before);
    for index in 0..set.rewards.len() {
        let action = simulator
            .legal_actions()
            .iter()
            .find(|action| matches!(action, sts2_engine::Action::ClaimReward { .. }))
            .cloned()
            .unwrap_or_else(|| panic!("reward {index} can be taken"));
        simulator.step(action).expect("taking the relic applies");
    }
    // The curse arrives once the set is done with. Whatever else the two
    // relics handed over arrives with it, so what is checked is the curse.
    let registry = sts2_content::standard_registry();
    let curses = simulator
        .state()
        .run_player
        .deck
        .iter()
        .filter(|card| {
            registry
                .card(&card.model_id)
                .is_some_and(|definition| definition.card_type == sts2_engine::CardType::Curse)
        })
        .count();
    // At least one: the bones' own. Two of Neow's options bring a curse of
    // their own, so which of them the shuffle dealt decides the rest.
    assert!(curses >= 1, "no curse joined the deck");
}

#[test]
fn neows_bones_wait_for_both_relics_without_holding_the_claimed_relics_body() {
    use sts2_engine::Action;
    use sts2_rng::RunStream;

    let mut simulator =
        sts2_content::standard_ironclad_run("N3EWU9Z7TK").expect("the run generates");
    let bones = simulator.legal_actions().iter().find(|action| {
        matches!(action, Action::ChooseEvent { option_id, .. } if option_id.ends_with(".NEOWS_BONES"))
    }).cloned().expect("Neow offers the bones");
    simulator.step(bones).expect("the two relics are offered");
    let before = simulator.state().run_player.deck.len();
    let niche = simulator.state().rng.stream(RunStream::Niche).counter();
    let claim = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::ClaimReward { fingerprint, .. }
            if fingerprint.model_id.as_ref().is_some_and(|id| id.entry() == "LARGE_CAPSULE"))
        })
        .cloned()
        .expect("the capsule is offered");
    simulator.step(claim).expect("the capsule is claimed");
    assert_eq!(
        simulator.state().run_player.deck.len(),
        before + 2,
        "the capsule adds both basics, while the bones' curse still waits"
    );
    assert_eq!(
        simulator.state().rng.stream(RunStream::Niche).counter(),
        niche,
        "the curse is not rolled until the remaining relic is claimed"
    );
    let claim = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ClaimReward { .. }))
        .cloned()
        .expect("the pearl is still offered");
    simulator.step(claim).expect("the pearl is claimed");
    assert_eq!(
        simulator.state().run_player.deck.len(),
        before + 4,
        "the pearl's Greed and the bones' curse each join the deck"
    );
    assert_eq!(
        simulator.state().rng.stream(RunStream::Niche).counter(),
        niche + 1
    );
    assert!(simulator.state().run.as_ref().unwrap().pending.is_empty());
}

#[test]
fn neows_bones_wait_for_shears_acquisition_in_either_claim_order() {
    use sts2_engine::{Action, Simulator};
    use sts2_rng::RunStream;

    for shears_first in [true, false] {
        let mut simulator = sts2_content::standard_ironclad_run("N3EWU9Z7TK").unwrap();
        let bones = simulator
            .legal_actions()
            .iter()
            .find(|action| {
                matches!(action, Action::ChooseEvent { option_id, .. }
                if option_id.ends_with(".NEOWS_BONES"))
            })
            .cloned()
            .unwrap();
        simulator.step(bones).unwrap();
        // Choose the offered relics independently of the seeded shuffle.
        let mut state = simulator.state().clone();
        let set = state.run.as_mut().unwrap().reward_set.as_mut().unwrap();
        for (reward, model) in set
            .rewards
            .iter_mut()
            .zip(["RELIC.PRECARIOUS_SHEARS", "RELIC.SILVER_CRUCIBLE"])
        {
            reward.fingerprint.model_id = Some(model.parse().unwrap());
        }
        let mut simulator =
            Simulator::from_generated_state(state, sts2_content::standard_registry()).unwrap();
        let hp = simulator.state().run_player.current_hp;
        let deck = simulator.state().run_player.deck.len();
        let niche = simulator.state().rng.stream(RunStream::Niche).counter();
        let claim = |simulator: &mut Simulator, relic: &str| {
            let action = simulator
                .legal_actions()
                .iter()
                .find(|action| {
                    matches!(action, Action::ClaimReward { fingerprint, .. }
                    if fingerprint.model_id.as_ref().is_some_and(|id| id.entry() == relic))
                })
                .cloned()
                .unwrap();
            simulator.step(action).unwrap();
        };
        if !shears_first {
            claim(&mut simulator, "SILVER_CRUCIBLE");
        }
        claim(&mut simulator, "PRECARIOUS_SHEARS");
        assert_eq!(simulator.state().run_player.current_hp, hp);
        assert_eq!(simulator.state().run_player.deck.len(), deck);
        assert_eq!(
            simulator.state().rng.stream(RunStream::Niche).counter(),
            niche
        );
        let run = simulator.state().run.as_ref().unwrap();
        assert!(!run.reward_set.as_ref().unwrap().rewards[0].selected);

        // A saved choice retains the claim's continuation and its parent wait.
        let mut saved = simulator.state().clone();
        let run_json = serde_json::to_vec(&saved.run).unwrap();
        saved.run = serde_json::from_slice(&run_json).unwrap();
        let restored =
            Simulator::from_generated_state(saved, sts2_content::standard_registry()).unwrap();
        simulator.restore(&restored.snapshot()).unwrap();
        let choice = simulator
            .legal_actions()
            .iter()
            .find(|action| matches!(action, Action::ChooseCards { cards, .. } if cards.len() == 2))
            .cloned()
            .unwrap();
        simulator.step(choice).unwrap();
        assert_eq!(simulator.state().run_player.current_hp, hp - 16);
        if shears_first {
            assert_eq!(simulator.state().run_player.deck.len(), deck - 2);
            assert_eq!(
                simulator.state().rng.stream(RunStream::Niche).counter(),
                niche
            );
            let run = simulator.state().run.as_ref().unwrap();
            assert!(run.reward_set.as_ref().unwrap().rewards[0].selected);
            claim(&mut simulator, "SILVER_CRUCIBLE");
        }
        assert_eq!(simulator.state().run_player.deck.len(), deck - 1);
        assert_eq!(
            simulator.state().rng.stream(RunStream::Niche).counter(),
            niche + 1
        );
        let run = simulator.state().run.as_ref().unwrap();
        assert!(run.reward_set.is_none());
        assert!(run.pending.is_empty());
        assert!(run.suspended_choice.is_none());
    }
}

fn bones_offering_kaleidoscope() -> sts2_engine::Simulator {
    use sts2_engine::{Action, Simulator};

    let mut simulator = sts2_content::standard_ironclad_run("N3EWU9Z7TK").unwrap();
    let bones = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::ChooseEvent { option_id, .. }
            if option_id.ends_with(".NEOWS_BONES"))
        })
        .cloned()
        .unwrap();
    simulator.step(bones).unwrap();
    let mut state = simulator.state().clone();
    let set = state.run.as_mut().unwrap().reward_set.as_mut().unwrap();
    for (reward, model) in set
        .rewards
        .iter_mut()
        .zip(["RELIC.KALEIDOSCOPE", "RELIC.SILVER_CRUCIBLE"])
    {
        reward.fingerprint.model_id = Some(model.parse().unwrap());
    }
    Simulator::from_generated_state(state, sts2_content::standard_registry()).unwrap()
}

fn claim_relic(simulator: &mut sts2_engine::Simulator, relic: &str) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, sts2_engine::Action::ClaimReward { fingerprint, .. }
            if fingerprint.model_id.as_ref().is_some_and(|id| id.entry() == relic))
        })
        .cloned()
        .unwrap();
    simulator.step(action).unwrap();
}

fn restore_reward_snapshot(simulator: &mut sts2_engine::Simulator) {
    let key = simulator.state_key().unwrap();
    let mut saved = simulator.state().clone();
    saved.run = serde_json::from_slice(&serde_json::to_vec(&saved.run).unwrap()).unwrap();
    let restored =
        sts2_engine::Simulator::from_generated_state(saved, simulator.content().clone().into())
            .unwrap();
    simulator.restore(&restored.snapshot()).unwrap();
    assert_eq!(simulator.state_key().unwrap(), key);
}

fn cancel_card_reward(simulator: &mut sts2_engine::Simulator) {
    use sts2_engine::Action;

    let open = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ClaimReward { .. }))
        .cloned()
        .unwrap();
    simulator.step(open.clone()).unwrap();
    restore_reward_snapshot(simulator);
    let cancel = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseCards { cards, .. } if cards.is_empty()))
        .cloned()
        .unwrap();
    simulator.step(cancel).unwrap();
    assert!(simulator.legal_actions().contains(&open));
}

#[test]
fn kaleidoscope_rewards_preserve_neows_bones_until_acquisition_finishes() {
    use sts2_engine::Action;
    use sts2_rng::RunStream;

    for kaleidoscope_first in [true, false] {
        for cards_taken in 0..=2 {
            let mut simulator = bones_offering_kaleidoscope();
            let deck = simulator.state().run_player.deck.len();
            let niche = simulator.state().rng.stream(RunStream::Niche).counter();
            let parent_id = simulator
                .state()
                .run
                .as_ref()
                .unwrap()
                .reward_set
                .as_ref()
                .unwrap()
                .id;
            if !kaleidoscope_first {
                claim_relic(&mut simulator, "SILVER_CRUCIBLE");
            }
            claim_relic(&mut simulator, "KALEIDOSCOPE");
            let projection = sts2_replay::observation::render(&simulator);
            let sets = projection["choices"]["rewards"].as_array().unwrap();
            assert_eq!(sets.len(), 2, "both reward sets remain outstanding");
            assert_eq!(sets[0]["set_id"], parent_id.get());
            assert_eq!(sets[0]["rewards"][0]["selected"], false);
            assert_eq!(sets[0]["rewards"][1]["selected"], !kaleidoscope_first);
            assert_eq!(sets[1]["rewards"].as_array().unwrap().len(), 2);
            assert_eq!(simulator.state().run_player.deck.len(), deck);
            assert_eq!(
                simulator.state().rng.stream(RunStream::Niche).counter(),
                niche + 6
            );
            assert!(simulator.legal_actions().iter().all(|action|
                !matches!(action, Action::ClaimReward { set_id, .. } if *set_id == parent_id)));
            restore_reward_snapshot(&mut simulator);

            // Dismissing one card offer leaves it available on the nested set.
            cancel_card_reward(&mut simulator);
            for taken in 0..cards_taken {
                let open = simulator
                    .legal_actions()
                    .iter()
                    .find(|action| matches!(action, Action::ClaimReward { .. }))
                    .cloned()
                    .unwrap();
                simulator.step(open).unwrap();
                let take = simulator.legal_actions().iter()
                    .find(|action| matches!(action, Action::ChooseCards { cards, .. } if cards.len() == 1))
                    .cloned().unwrap();
                simulator.step(take).unwrap();
                if taken == 0 {
                    assert_eq!(simulator.state().run_player.deck.len(), deck + 1);
                    assert_eq!(
                        simulator.state().rng.stream(RunStream::Niche).counter(),
                        niche + 6
                    );
                    restore_reward_snapshot(&mut simulator);
                }
            }
            if cards_taken < 2 {
                simulator.step(Action::Proceed).unwrap();
            }
            if kaleidoscope_first {
                let run = simulator.state().run.as_ref().unwrap();
                let parent = run.reward_set.as_ref().unwrap();
                assert_eq!(parent.id, parent_id);
                assert!(parent.rewards[0].selected);
                assert!(!parent.rewards[1].selected);
                assert_eq!(simulator.state().run_player.deck.len(), deck + cards_taken);
                assert_eq!(
                    simulator.state().rng.stream(RunStream::Niche).counter(),
                    niche + 6
                );
                assert!(!simulator.legal_actions().contains(&Action::Proceed));
                restore_reward_snapshot(&mut simulator);
                claim_relic(&mut simulator, "SILVER_CRUCIBLE");
            }
            assert_eq!(
                simulator.state().run_player.deck.len(),
                deck + cards_taken + 1
            );
            assert_eq!(
                simulator.state().rng.stream(RunStream::Niche).counter(),
                niche + 7
            );
            let run = simulator.state().run.as_ref().unwrap();
            assert!(run.reward_set.is_none());
            assert_eq!(run.reward_parents, []);
            assert!(run.pending.is_empty());
            assert!(run.owed_by_rewards.is_empty());
            assert!(run.suspended_choice.is_none());
            assert_eq!(run.phase, sts2_engine::RunPhase::Map);
            assert_eq!(
                sts2_replay::observation::render(&simulator)["choices"]["rewards"],
                serde_json::json!([])
            );
        }
    }
}

#[test]
fn nested_reward_completion_runs_the_acquisition_tail_before_the_parent_body() {
    use sts2_engine::{
        Action, Amount, Command, PlayerCmd, RewardFingerprint, RewardsCmd, Simulator,
    };

    for skip in [false, true] {
        let simulator = bones_offering_kaleidoscope();
        let mut registry = sts2_content::standard_registry();
        // An immediate reward and a delayed grant make both continuation
        // boundaries observable without opening a card-selection screen.
        let mut relic = registry
            .relic(&"RELIC.KALEIDOSCOPE".parse().unwrap())
            .unwrap()
            .clone();
        relic.on_obtained = vec![
            Command::Rewards(RewardsCmd::OfferCustom(vec![
                RewardFingerprint::of("gold").with_gold(10),
            ])),
            Command::Player(PlayerCmd::GainGold(Amount::Fixed(7))),
        ];
        std::sync::Arc::make_mut(&mut registry).add_relic(relic);
        let mut simulator =
            Simulator::from_generated_state(simulator.state().clone(), registry).unwrap();
        let gold = simulator.state().run_player.gold;
        let deck = simulator.state().run_player.deck.len();
        claim_relic(&mut simulator, "KALEIDOSCOPE");
        assert_eq!(simulator.state().run_player.gold, gold);
        restore_reward_snapshot(&mut simulator);
        let action = if skip {
            Action::Proceed
        } else {
            simulator
                .legal_actions()
                .iter()
                .find(|action| matches!(action, Action::ClaimReward { .. }))
                .cloned()
                .unwrap()
        };
        simulator.step(action).unwrap();
        assert_eq!(
            simulator.state().run_player.gold,
            gold + if skip { 7 } else { 17 }
        );
        assert_eq!(simulator.state().run_player.deck.len(), deck);
        assert!(
            simulator
                .state()
                .run
                .as_ref()
                .unwrap()
                .reward_set
                .as_ref()
                .unwrap()
                .rewards[0]
                .selected
        );
        claim_relic(&mut simulator, "SILVER_CRUCIBLE");
        assert_eq!(
            simulator.state().run_player.gold,
            gold + if skip { 7 } else { 17 }
        );
        assert_eq!(simulator.state().run_player.deck.len(), deck + 1);
    }
}
