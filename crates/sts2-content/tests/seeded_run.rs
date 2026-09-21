use sts2_engine::{Action, DecisionContext, ErrorCode};
use sts2_rng::{PlayerStream, RunStream};

#[test]
fn generated_run_starts_at_exact_neow_boundary() {
    let simulator = sts2_content::standard_ironclad_run("N3Z9EE2CXB").unwrap();
    let state = simulator.state();
    let run = state.run.as_ref().unwrap();
    assert_eq!(state.origin, sts2_engine::ConformanceOrigin::Generation);
    assert_eq!(run.floor, 1);
    assert_eq!(run.current_coord, Some(run.map().start));
    assert_eq!(
        run.acts[0].normal_encounters[0].to_string(),
        "ENCOUNTER.SHRINKER_BEETLE_WEAK"
    );
    assert_eq!(state.rng.stream(RunStream::UpFront).counter(), 414);
    assert_eq!(state.run_player.current_hp, 80);
    assert_eq!(state.run_player.max_hp, 80);
    assert_eq!(state.run_player.gold, 99);
    assert_eq!(state.run_player.deck.len(), 10);
    assert_eq!(
        simulator.state().run_player.relics[0].model_id.to_string(),
        "RELIC.BURNING_BLOOD"
    );
    let DecisionContext::Event { model_id, options } = simulator.decision() else {
        panic!("generated run should stop at Neow")
    };
    assert_eq!(model_id.to_string(), "EVENT.NEOW");
    assert_eq!(
        options
            .iter()
            .map(|option| option.option_id.as_str())
            .collect::<Vec<_>>(),
        [
            "NEOW.pages.INITIAL.options.ARCANE_SCROLL",
            "NEOW.pages.INITIAL.options.LOST_COFFER",
            "NEOW.pages.INITIAL.options.CURSED_PEARL",
        ]
    );
    assert_eq!(simulator.legal_actions().len(), 3);
}

#[test]
fn arcane_scroll_matches_reference_and_other_relics_fail_atomically() {
    let mut simulator = sts2_content::standard_ironclad_run("N3Z9EE2CXB").unwrap();
    simulator
        .step(simulator.legal_actions()[0].clone())
        .unwrap();
    let state = simulator.state();
    assert_eq!(
        state.run_player.deck[10].model_id.to_string(),
        "CARD.PRIMAL_FORCE"
    );
    assert_eq!(
        state.run_player.relics[1].model_id.to_string(),
        "RELIC.ARCANE_SCROLL"
    );
    assert_eq!(state.player_rng.stream(PlayerStream::Rewards).counter(), 1);
    assert!(matches!(
        simulator.decision(),
        DecisionContext::MapNavigation { .. }
    ));

    let mut simulator = sts2_content::standard_ironclad_run("N3Z9EE2CXB").unwrap();
    let before = simulator.state_key().unwrap();
    let bogus = Action::ChooseEvent {
        index: 0,
        option_id: "bogus".to_owned(),
    };
    assert_eq!(
        simulator.step(bogus).unwrap_err().code,
        ErrorCode::IllegalAction
    );
    assert_eq!(simulator.state_key().unwrap(), before);
}

#[test]
fn lost_coffer_reward_flow_matches_example_prefix() {
    let mut simulator = sts2_content::standard_ironclad_run("N3Z9EE2CXB").unwrap();
    simulator
        .step(simulator.legal_actions()[1].clone())
        .unwrap();
    let state = simulator.state();
    assert_eq!(state.player_rng.stream(PlayerStream::Rewards).counter(), 11);
    assert_eq!(
        state.run_player.relics[1].model_id.to_string(),
        "RELIC.LOST_COFFER"
    );
    let rewards = &state
        .run
        .as_ref()
        .unwrap()
        .reward_set
        .as_ref()
        .unwrap()
        .rewards;
    assert_eq!(
        rewards[0]
            .fingerprint
            .model_id
            .as_ref()
            .unwrap()
            .to_string(),
        "POTION.BLOCK_POTION"
    );
    assert_eq!(
        rewards[1]
            .fingerprint
            .offered_cards
            .iter()
            .map(|card| card.model_id.to_string())
            .collect::<Vec<_>>(),
        ["CARD.PERFECTED_STRIKE", "CARD.SHRUG_IT_OFF", "CARD.BULLY"]
    );

    let card_reward = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(
                action,
                Action::ClaimReward {
                    reward_index: 1,
                    ..
                }
            )
        })
        .unwrap()
        .clone();
    simulator.step(card_reward).unwrap();
    let bully = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::ChooseCards { cards, .. }
                if cards.first().is_some_and(|card|
                    card.fingerprint.model_id.to_string() == "CARD.BULLY"))
        })
        .unwrap()
        .clone();
    simulator.step(bully).unwrap();
    let potion = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(
                action,
                Action::ClaimReward {
                    reward_index: 0,
                    ..
                }
            )
        })
        .unwrap()
        .clone();
    simulator.step(potion).unwrap();
    assert!(matches!(
        simulator.decision(),
        DecisionContext::MapNavigation { .. }
    ));
    assert_eq!(
        simulator.state().run_player.deck[10].model_id.to_string(),
        "CARD.BULLY"
    );
    assert_eq!(
        simulator.state().run_player.potions[0]
            .as_ref()
            .unwrap()
            .to_string(),
        "POTION.BLOCK_POTION"
    );
}

#[test]
fn lost_coffer_generation_matches_second_reference_seed() {
    let mut simulator = sts2_content::standard_ironclad_run("R8F8NFB8Y1").unwrap();
    simulator
        .step(simulator.legal_actions()[1].clone())
        .unwrap();
    let rewards = &simulator
        .state()
        .run
        .as_ref()
        .unwrap()
        .reward_set
        .as_ref()
        .unwrap()
        .rewards;
    assert_eq!(
        rewards[0]
            .fingerprint
            .model_id
            .as_ref()
            .unwrap()
            .to_string(),
        "POTION.STRENGTH_POTION"
    );
    assert_eq!(
        rewards[1]
            .fingerprint
            .offered_cards
            .iter()
            .map(|card| card.model_id.to_string())
            .collect::<Vec<_>>(),
        ["CARD.SHRUG_IT_OFF", "CARD.BREAKTHROUGH", "CARD.COLOSSUS"]
    );
    assert_eq!(
        simulator
            .state()
            .player_rng
            .stream(PlayerStream::Rewards)
            .counter(),
        11
    );
}
