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
