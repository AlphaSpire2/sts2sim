use sts2_engine::{
    Action, CardFingerprint, CombatSide, DecisionContext, ErrorCode, PileName, ProjectionScope,
    RunResult, ScenarioBuilder, Simulator,
};
use sts2_rng::RunStream;

fn play(simulator: &mut Simulator, card_id: u64) -> sts2_engine::Transition {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::PlayCard { card, .. } if card.card_id.get() == card_id)
        })
        .cloned()
        .unwrap_or_else(|| panic!("card {card_id} was not legal"));
    simulator.step(action).unwrap()
}

fn end_turn(simulator: &mut Simulator) -> sts2_engine::Transition {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::EndTurn { .. }))
        .cloned()
        .unwrap();
    simulator.step(action).unwrap()
}

fn ids(cards: &[sts2_engine::CardInstance]) -> Vec<u64> {
    cards.iter().map(|card| card.id.get()).collect()
}

fn combat(simulator: &Simulator) -> &sts2_engine::CombatState {
    simulator.state().combat.as_ref().unwrap()
}

fn hp(simulator: &Simulator, creature_id: u64) -> i32 {
    combat(simulator)
        .creatures
        .iter()
        .find(|creature| creature.id.get() == creature_id)
        .unwrap()
        .current_hp
}

/// The Shrinker Beetle fight played along the line a run of this seed took:
/// cards 4, 3, 1, end turn, 6, 7, end turn, 6, 4. Every boundary the line
/// crosses is pinned — HP, energy, piles, powers, intents, the shuffle
/// counter, and the reward set the last blow opens.
#[test]
#[allow(clippy::too_many_lines)]
fn the_shrinker_beetle_fight_plays_out_along_its_action_line() {
    let mut simulator = sts2_content::shrinker_beetle_fixture().unwrap();
    assert_eq!(hp(&simulator, 1), 39);
    assert_eq!(ids(&combat(&simulator).player.piles.hand), [0, 1, 2, 3, 4]);

    let transition = play(&mut simulator, 4);
    assert_eq!(transition.checkpoints[0].context, "action.settled");
    assert_eq!(
        (hp(&simulator, 1), combat(&simulator).player.energy),
        (35, 3)
    );
    assert_eq!(ids(&combat(&simulator).player.piles.discard), [4]);

    play(&mut simulator, 3);
    assert_eq!(
        (hp(&simulator, 1), combat(&simulator).player.energy),
        (29, 2)
    );
    assert_eq!(ids(&combat(&simulator).player.piles.discard), [4, 3]);

    play(&mut simulator, 1);
    assert_eq!(
        (hp(&simulator, 1), combat(&simulator).player.energy),
        (23, 1)
    );
    assert_eq!(ids(&combat(&simulator).player.piles.discard), [4, 3, 1]);

    let transition = end_turn(&mut simulator);
    assert_eq!(
        transition
            .checkpoints
            .iter()
            .map(|checkpoint| checkpoint.context.as_str())
            .collect::<Vec<_>>(),
        [
            "combat.turn.end",
            "combat.enemy_turn.start",
            "combat.turn.end",
            "combat.player_turn.start"
        ]
    );
    assert_eq!(
        (combat(&simulator).round, combat(&simulator).player.turn),
        (2, 2)
    );
    assert_eq!(combat(&simulator).side, CombatSide::Player);
    assert_eq!(ids(&combat(&simulator).player.piles.hand), [5, 6, 7, 8, 9]);
    let player = &combat(&simulator).creatures[0];
    assert_eq!((player.current_hp, player.powers[0].amount), (80, -1));
    assert_eq!(player.powers[0].amount_on_turn_start, -1);
    assert_eq!(
        combat(&simulator).creatures[1]
            .intent
            .as_ref()
            .unwrap()
            .move_id,
        "CHOMP_MOVE"
    );

    play(&mut simulator, 6);
    assert_eq!(
        (hp(&simulator, 1), combat(&simulator).player.energy),
        (18, 1)
    );
    assert_eq!(
        combat(&simulator).creatures[1].power_amount("POWER.VULNERABLE_POWER"),
        2
    );
    play(&mut simulator, 7);
    assert_eq!(
        (hp(&simulator, 1), combat(&simulator).player.energy),
        (12, 0)
    );

    end_turn(&mut simulator);
    assert_eq!((combat(&simulator).round, hp(&simulator, 0)), (3, 73));
    assert_eq!(ids(&combat(&simulator).player.piles.hand), [10, 4, 7, 6, 0]);
    assert_eq!(
        ids(&combat(&simulator).player.piles.draw),
        [3, 1, 9, 5, 2, 8]
    );
    assert_eq!(
        simulator.state().rng.stream(RunStream::Shuffle).counter(),
        19
    );
    assert_eq!(
        combat(&simulator).creatures[1]
            .intent
            .as_ref()
            .unwrap()
            .move_id,
        "STOMP_MOVE"
    );

    play(&mut simulator, 6);
    assert_eq!(hp(&simulator, 1), 4);
    assert_eq!(
        combat(&simulator).creatures[1].power_amount("POWER.VULNERABLE_POWER"),
        3
    );
    let transition = play(&mut simulator, 4);
    assert_eq!(
        transition
            .checkpoints
            .iter()
            .map(|checkpoint| checkpoint.context.as_str())
            .collect::<Vec<_>>(),
        ["action.settled", "reward.offered"]
    );
    assert_eq!(simulator.state().run_player.current_hp, 79);
    assert!(matches!(
        simulator.decision(),
        DecisionContext::Rewards { .. }
    ));
    let reward_set = combat(&simulator).reward_set.as_ref().unwrap();
    assert_eq!(reward_set.rewards.len(), 3);
    assert_eq!(reward_set.rewards[0].fingerprint.gold_amount, 16);
    assert_eq!(
        reward_set.rewards[1]
            .fingerprint
            .model_id
            .as_ref()
            .unwrap()
            .to_string(),
        "POTION.VULNERABLE_POTION"
    );
}

#[test]
fn snapshot_restore_replays_the_same_combat_digest() {
    let mut simulator = sts2_content::shrinker_beetle_fixture().unwrap();
    play(&mut simulator, 4);
    play(&mut simulator, 3);
    play(&mut simulator, 1);
    let snapshot = simulator.snapshot();
    end_turn(&mut simulator);
    let expected = simulator
        .projection_digest(ProjectionScope::CombatCore)
        .unwrap();
    simulator.restore(&snapshot).unwrap();
    end_turn(&mut simulator);
    assert_eq!(
        simulator
            .projection_digest(ProjectionScope::CombatCore)
            .unwrap(),
        expected
    );
}

#[test]
fn snapshot_replay_matches_key_digest_and_actions_at_every_fixture_boundary() {
    let mut simulator = sts2_content::shrinker_beetle_fixture().unwrap();
    for card_id in [
        Some(4),
        Some(3),
        Some(1),
        None,
        Some(6),
        Some(7),
        None,
        Some(6),
        Some(4),
    ] {
        let action = if let Some(card_id) = card_id {
            simulator
                .legal_actions()
                .iter()
                .find(|action| {
                    matches!(action, Action::PlayCard { card, .. }
                        if card.card_id.get() == card_id)
                })
                .unwrap()
                .clone()
        } else {
            simulator
                .legal_actions()
                .iter()
                .find(|action| matches!(action, Action::EndTurn { .. }))
                .unwrap()
                .clone()
        };
        let snapshot = simulator.snapshot();
        let first = simulator.step(action.clone()).unwrap();
        let expected_key = simulator.state_key().unwrap();
        let expected_digest = simulator
            .projection_digest(ProjectionScope::CombatCore)
            .unwrap();
        let expected_actions = simulator.legal_actions().to_vec();
        simulator.restore(&snapshot).unwrap();
        let second = simulator.step(action).unwrap();
        assert_eq!(second.state_key, first.state_key);
        assert_eq!(simulator.state_key().unwrap(), expected_key);
        assert_eq!(
            simulator
                .projection_digest(ProjectionScope::CombatCore)
                .unwrap(),
            expected_digest
        );
        assert_eq!(simulator.legal_actions(), expected_actions);
    }
}

#[test]
fn reward_flow_can_be_completed_through_public_actions() {
    let mut simulator = sts2_content::shrinker_beetle_fixture().unwrap();
    for card_id in [4, 3, 1] {
        play(&mut simulator, card_id);
    }
    end_turn(&mut simulator);
    for card_id in [6, 7] {
        play(&mut simulator, card_id);
    }
    end_turn(&mut simulator);
    for card_id in [6, 4] {
        play(&mut simulator, card_id);
    }
    while !matches!(simulator.decision(), DecisionContext::Terminal { .. }) {
        let action = simulator.legal_actions()[0].clone();
        simulator.step(action).unwrap();
    }
    assert_eq!(simulator.state().terminal, Some(RunResult::Victory));
    assert_eq!(simulator.state().run_player.gold, 115);
    assert_eq!(
        simulator.state().run_player.potions[1]
            .as_ref()
            .unwrap()
            .to_string(),
        "POTION.VULNERABLE_POTION"
    );
}

#[test]
fn unsupported_card_variant_fails_instead_of_using_base_values() {
    // A curse has no upgraded form at all, so an upgraded one is a variant the
    // pinned build cannot stand up.
    let mut upgraded = CardFingerprint::base("CARD.CLUMSY".parse().unwrap());
    upgraded.upgrade_level = 1;
    let error = ScenarioBuilder::new("VARIANTTEST", "ENCOUNTER.TEST".parse().unwrap())
        .card(PileName::Hand, 1, upgraded, -1)
        .build(sts2_content::standard_registry())
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::UnsupportedContent);
    assert_eq!(error.details["model_id"], "CARD.CLUMSY");
    assert_eq!(error.details["upgrade_level"], 1);
}
