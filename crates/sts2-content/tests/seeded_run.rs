use sts2_engine::{Action, DecisionContext, ErrorCode};
use sts2_rng::{PlayerStream, RunStream};

/// The run start recorded for this seed at ascension ten: its first
/// encounter, Neow's page, and the draws the up-front stream spent generating
/// the run — one fewer here at ascension zero, where the last act draws no
/// second boss.
#[test]
fn generated_run_starts_at_exact_neow_boundary() {
    let simulator = sts2_content::standard_ironclad_run("7CM298WJ2J2Y").unwrap();
    let state = simulator.state();
    let run = state.run.as_ref().unwrap();
    assert_eq!(state.origin, sts2_engine::ConformanceOrigin::Generation);
    assert_eq!(run.floor, 1);
    assert_eq!(run.current_coord, Some(run.map().start));
    assert_eq!(
        run.acts[0].normal_encounters[0].to_string(),
        "ENCOUNTER.CORPSE_SLUGS_WEAK"
    );
    assert_eq!(state.rng.stream(RunStream::UpFront).counter(), 412);
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
            "NEOW.pages.INITIAL.options.NUTRITIOUS_OYSTER",
            "NEOW.pages.INITIAL.options.KALEIDOSCOPE",
            "NEOW.pages.INITIAL.options.NEOWS_BONES",
        ]
    );
    assert_eq!(simulator.legal_actions().len(), 3);
}

/// A seed whose Neow page opens on Arcane Scroll: the relic hands over one
/// card off the rewards stream, and an option the page does not hold is
/// refused without touching the state.
#[test]
fn arcane_scroll_adds_a_card_and_other_relics_fail_atomically() {
    let mut simulator = sts2_content::standard_ironclad_run("SEED0000N000").unwrap();
    simulator
        .step(simulator.legal_actions()[0].clone())
        .unwrap();
    let state = simulator.state();
    assert_eq!(
        state.run_player.deck[10].model_id.to_string(),
        "CARD.NOT_YET"
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

    let mut simulator = sts2_content::standard_ironclad_run("SEED0000N000").unwrap();
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

/// A seed whose Neow page offers Lost Coffer second: the relic puts a potion
/// and a card reward up on the rewards stream, and claiming both settles the
/// room onto the map.
#[test]
fn lost_coffer_reward_flow_claims_its_card_and_potion() {
    let mut simulator = sts2_content::standard_ironclad_run("SEED0000T000").unwrap();
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
        "POTION.SWIFT_POTION"
    );
    assert_eq!(
        rewards[1]
            .fingerprint
            .offered_cards
            .iter()
            .map(|card| card.model_id.to_string())
            .collect::<Vec<_>>(),
        ["CARD.RAMPAGE", "CARD.TRUE_GRIT", "CARD.STAMPEDE"]
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
    let stampede = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::ChooseCards { cards, .. }
                if cards.first().is_some_and(|card|
                    card.fingerprint.model_id.to_string() == "CARD.STAMPEDE"))
        })
        .unwrap()
        .clone();
    simulator.step(stampede).unwrap();
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
        "CARD.STAMPEDE"
    );
    assert_eq!(
        simulator.state().run_player.potions[0]
            .as_ref()
            .unwrap()
            .to_string(),
        "POTION.SWIFT_POTION"
    );
}

/// The same relic on a second seed draws a different potion and three
/// different cards for the same eleven rewards-stream draws.
#[test]
fn lost_coffer_generation_on_a_second_seed() {
    let mut simulator = sts2_content::standard_ironclad_run("SEED0000Z000").unwrap();
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
        "POTION.DUPLICATOR"
    );
    assert_eq!(
        rewards[1]
            .fingerprint
            .offered_cards
            .iter()
            .map(|card| card.model_id.to_string())
            .collect::<Vec<_>>(),
        ["CARD.BLOODLETTING", "CARD.RUPTURE", "CARD.DRUM_OF_BATTLE"]
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
