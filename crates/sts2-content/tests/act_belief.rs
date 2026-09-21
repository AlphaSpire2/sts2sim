//! The act belief boundary over pinned content: an erased run decision point
//! samples back into full worlds that agree with the original on everything a
//! player could see and on nothing they could not, deterministically in
//! `(analysis_seed, rollout)`, labeled analysis. Model `act-v0.5`: each
//! sample deals a uniform permutation of the current act's hidden remainder —
//! pool suffixes, placed rooms, grab bags — behind the evidence the walk has
//! already pinned, with the act transition as the horizon.

use sts2_core::UnlockPresetManifest;
use sts2_engine::{
    ActBeliefState, Action, BeliefState, ConformanceOrigin, DecisionContext, Simulator,
};

const SEED: &str = "NLD6VZXP94";

fn fresh_run() -> Simulator {
    let preset = UnlockPresetManifest::pinned().unwrap();
    let character = "CHARACTER.IRONCLAD".parse().unwrap();
    sts2_content::standard_run_on_preset_at(SEED, &character, &preset, 0).unwrap()
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

/// The seeded run walked until `stop` answers, under the simple policy.
fn walk_until(mut simulator: Simulator, stop: impl Fn(&Simulator) -> bool) -> Simulator {
    for _ in 0..200 {
        if stop(&simulator) {
            return simulator;
        }
        let action = next_action(&simulator);
        simulator.step_quietly(&action).unwrap();
    }
    panic!("the seed reaches the stop inside two hundred decisions");
}

/// The seeded run walked to its first in-combat decision.
fn first_combat() -> Simulator {
    walk_until(fresh_run(), |simulator| simulator.state().combat.is_some())
}

/// The seeded run walked to a map decision in the middle of act one: two
/// monster fights and an event behind it, thirteen pool encounters still
/// undealt ahead — a remainder big enough that permutations can differ.
fn mid_act() -> Simulator {
    let simulator = walk_until(fresh_run(), |simulator| {
        simulator.state().combat.is_none()
            && simulator
                .state()
                .run
                .as_ref()
                .is_some_and(|run| run.floor >= 4)
            && matches!(simulator.decision(), DecisionContext::MapNavigation { .. })
    });
    let run = simulator.state().run.as_ref().unwrap();
    let act = &run.acts[run.current_act];
    assert!(
        act.normal_encounters.len() - act.normal_encounters_visited >= 3,
        "the anchor leaves a remainder worth permuting"
    );
    simulator
}

#[test]
fn a_sample_agrees_on_everything_visible_and_on_nothing_hidden() {
    let original = mid_act();
    let belief = ActBeliefState::from_simulator(&original).unwrap();
    let sample = belief.sample(7, 0).unwrap();
    assert_eq!(
        sample.agent_observation(),
        original.agent_observation(),
        "a sample shows the player the very screen they stood on"
    );
    assert_eq!(
        sample.observation_key().unwrap(),
        original.observation_key().unwrap()
    );
    assert_ne!(
        sample.state_key().unwrap(),
        original.state_key().unwrap(),
        "the hidden state is sampled, not copied"
    );
    assert_eq!(
        sample.legal_actions(),
        original.legal_actions(),
        "legality is observable, so every sample offers the same actions"
    );
    assert_eq!(sample.state().origin, ConformanceOrigin::Analysis);
}

#[test]
fn sampling_is_deterministic_in_the_pair_and_varied_across_it() {
    let belief = ActBeliefState::from_simulator(&mid_act()).unwrap();
    let one = belief.sample(7, 3).unwrap();
    let again = belief.sample(7, 3).unwrap();
    assert_eq!(
        one.state_key().unwrap(),
        again.state_key().unwrap(),
        "the same pair yields the same world"
    );
    let other = belief.sample(7, 4).unwrap();
    assert_ne!(
        one.state_key().unwrap(),
        other.state_key().unwrap(),
        "another rollout yields another world"
    );
    assert_eq!(
        one.observation_key().unwrap(),
        other.observation_key().unwrap(),
        "the worlds differ only where the player cannot see"
    );
}

#[test]
fn two_indistinguishable_states_erase_to_the_same_belief() {
    // The original and one of its own samples are indistinguishable to the
    // player, so erasing either yields belief states whose samples coincide.
    let original = mid_act();
    let belief = ActBeliefState::from_simulator(&original).unwrap();
    let world = belief.sample(3, 5).unwrap();
    let re_erased = ActBeliefState::from_simulator(&world).unwrap();
    assert_eq!(
        belief.sample(9, 1).unwrap().state_key().unwrap(),
        re_erased.sample(9, 1).unwrap().state_key().unwrap(),
        "erasure is a projection: erasing a sample changes nothing"
    );
}

#[test]
fn the_evidence_is_pinned_and_the_remainder_is_dealt() {
    let original = mid_act();
    let run = original.state().run.as_ref().unwrap();
    let act_index = run.current_act;
    let true_act = &run.acts[act_index];
    let cursor = true_act.normal_encounters_visited;
    let belief = ActBeliefState::from_simulator(&original).unwrap();
    let one = belief.sample(7, 0).unwrap();
    let two = belief.sample(7, 1).unwrap();
    for sample in [&one, &two] {
        let sampled = sample.state().run.as_ref().unwrap();
        assert_eq!(
            sampled.maps, run.maps,
            "the map layout is on the screen and stays, act by act"
        );
        assert_eq!(sampled.visited, run.visited, "the walked path stays");
        let act = &sampled.acts[act_index];
        assert_eq!(
            act.normal_encounters[..cursor],
            true_act.normal_encounters[..cursor],
            "the fights already dealt are evidence and stay"
        );
        let mut dealt = act.normal_encounters[cursor..].to_vec();
        let mut truth = true_act.normal_encounters[cursor..].to_vec();
        dealt.sort();
        truth.sort();
        assert_eq!(
            dealt, truth,
            "a sample's remainder is a permutation of the true one — the \
             model's honest crudeness, pinned"
        );
    }
    let one_ahead = &one.state().run.as_ref().unwrap().acts[act_index].normal_encounters;
    let two_ahead = &two.state().run.as_ref().unwrap().acts[act_index].normal_encounters;
    assert_ne!(
        one_ahead[cursor..],
        two_ahead[cursor..],
        "two rollouts deal the remainder in two orders"
    );
}

#[test]
fn an_unvisited_encounter_varies_across_samples() {
    // Walking two samples onto the same monster point puts up two different
    // fights: the encounter behind an unwalked icon was hidden, so each
    // sample dealt its own.
    let original = mid_act();
    let destination = match original.decision() {
        DecisionContext::MapNavigation { destinations, .. } => {
            let run = original.state().run.as_ref().unwrap();
            *destinations
                .iter()
                .find(|coord| {
                    run.map().points.iter().any(|point| {
                        point.coord == **coord
                            && point.point_type == sts2_engine::MapPointType::Monster
                    })
                })
                .expect("a monster point is one step ahead")
        }
        other => panic!("the anchor stands on the map, not {other:?}"),
    };
    let belief = ActBeliefState::from_simulator(&original).unwrap();
    let step_onto = |rollout: u64| {
        let mut sample = belief.sample(7, rollout).unwrap();
        sample
            .step_quietly(&Action::ChooseMap { destination })
            .unwrap();
        sample
            .state()
            .combat
            .as_ref()
            .expect("a monster point stands up a fight")
            .encounter_model
            .clone()
    };
    let first = step_onto(0);
    assert!(
        (1..8).map(step_onto).any(|encounter| encounter != first),
        "some rollout deals a different fight behind the same icon"
    );
}

#[test]
fn a_mid_fight_erasure_composes_with_combat_v0() {
    // Inside a fight the act model erases nothing combat-v0 does not, and its
    // sampler spends its dice in the same order, so the fight entry of an act
    // sample is the very world combat-v0 deals for the pair: same draw-pile
    // order, same stream positions. The act model only widens the horizon.
    let original = first_combat();
    let act_world = ActBeliefState::from_simulator(&original)
        .unwrap()
        .sample(11, 4)
        .unwrap();
    let combat_world = BeliefState::from_simulator(&original)
        .unwrap()
        .sample(11, 4)
        .unwrap();
    assert_eq!(
        act_world.state().combat,
        combat_world.state().combat,
        "the fight's hidden state is dealt exactly as combat-v0 deals it"
    );
    assert_eq!(act_world.state().rng, combat_world.state().rng);
    assert_eq!(
        act_world.state().player_rng,
        combat_world.state().player_rng
    );
    assert_eq!(act_world.agent_observation(), original.agent_observation());
}

#[test]
fn a_sampled_world_walks_the_act() {
    // A sampled world steps like the true one: the simple policy that walks
    // the seeded run walks the sample floor after floor — fights, rewards,
    // shops, events — with every action it takes accepted.
    let belief = ActBeliefState::from_simulator(&mid_act()).unwrap();
    let mut sample = belief.sample(11, 2).unwrap();
    let floor = sample.state().run.as_ref().unwrap().floor;
    for _ in 0..120 {
        if sample.state().terminal.is_some()
            || matches!(sample.decision(), DecisionContext::ActTransition { .. })
        {
            break;
        }
        let action = next_action(&sample);
        sample.step_quietly(&action).unwrap();
    }
    assert!(
        sample.state().run.as_ref().unwrap().floor >= floor + 3,
        "the walk crossed floors rather than standing still"
    );
}

#[test]
fn placed_rooms_permute_within_their_map_type_and_visited_ones_stay() {
    // A standard run pre-places nothing — every room is rolled as it is
    // entered — but a fixture may pin definitions onto map points, and those
    // are hidden until walked onto: each sample deals them among the unwalked
    // points of their own type, definitions riding whole, while a definition
    // behind a walked point is evidence and stays.
    use sts2_engine::{CombatRoomDefinition, EnemySpawn, MapPointType, RoomAt, RoomDefinition};
    let original = mid_act();
    let mut state = original.state().clone();
    let run = state.run.as_mut().unwrap();
    let act_index = run.current_act;
    let unwalked = |run: &sts2_engine::RunState, wanted: MapPointType| {
        run.maps[act_index]
            .points
            .iter()
            .filter(|point| point.point_type == wanted && !run.visited.contains(&point.coord))
            .map(|point| point.coord)
            .collect::<Vec<_>>()
    };
    let monsters = unwalked(run, MapPointType::Monster);
    assert!(monsters.len() >= 2, "the act has unwalked monster points");
    let elsewhere = *unwalked(run, MapPointType::Unknown)
        .first()
        .expect("the act has an unwalked unknown point");
    let walked = *run.visited.first().expect("the walk stood somewhere");
    let pin = |encounter: &str, monster: &str| {
        RoomDefinition::Combat(CombatRoomDefinition {
            encounter_model: encounter.parse().unwrap(),
            enemies: vec![EnemySpawn::rolled(monster.parse().unwrap())],
            rewards: Vec::new(),
            generated_rewards: None,
        })
    };
    let coords = [monsters[0], monsters[1], elsewhere, walked];
    let pinned = [
        pin("ENCOUNTER.NIBBITS_WEAK", "MONSTER.NIBBIT"),
        pin("ENCOUNTER.SLIMES_WEAK", "MONSTER.TWIG_SLIME_S"),
        pin("ENCOUNTER.FOGMOG_NORMAL", "MONSTER.FOGMOG"),
        pin("ENCOUNTER.MAWLER_NORMAL", "MONSTER.MAWLER"),
    ];
    for (coord, definition) in coords.into_iter().zip(pinned.clone()) {
        run.rooms[act_index].push(RoomAt { coord, definition });
    }
    let original = Simulator::from_scenario(state, sts2_content::standard_registry()).unwrap();
    let belief = ActBeliefState::from_simulator(&original).unwrap();
    let mut swapped = false;
    for rollout in 0..8 {
        let sample = belief.sample(7, rollout).unwrap();
        let rooms = &sample.state().run.as_ref().unwrap().rooms[act_index];
        let at = |coord| {
            rooms
                .iter()
                .find(|room| room.coord == coord)
                .map(|room| room.definition.clone())
                .expect("the pinned room is still on the map")
        };
        let pair = [at(monsters[0]), at(monsters[1])];
        assert!(
            pair.contains(&pinned[0]) && pair.contains(&pinned[1]),
            "the monster points hold the two monster definitions, dealt whole"
        );
        swapped |= pair[0] == pinned[1];
        assert_eq!(
            at(elsewhere),
            pinned[2],
            "a definition never crosses into another map type's points"
        );
        assert_eq!(at(walked), pinned[3], "a walked point's room is evidence");
    }
    assert!(
        swapped,
        "some rollout deals the two icons the other way round"
    );
}

#[test]
fn a_fight_outside_a_run_is_refused() {
    // A bare scenario fight has no act to bound a sample: combat-v0's scope.
    let simulator =
        sts2_engine::ScenarioBuilder::new(SEED, "ENCOUNTER.SHRINKER_BEETLE_WEAK".parse().unwrap())
            .player_hp(80, 80)
            .energy(3, 3)
            .turn(1, 1)
            .enemy(
                1,
                "MONSTER.SHRINKER_BEETLE".parse().unwrap(),
                39,
                39,
                "SHRINKER_MOVE",
            )
            .card(
                sts2_engine::PileName::Hand,
                10,
                sts2_engine::CardFingerprint::base("CARD.STRIKE_IRONCLAD".parse().unwrap()),
                1,
            )
            .build(sts2_content::standard_registry())
            .unwrap();
    assert!(simulator.state().run.is_none());
    assert!(ActBeliefState::from_simulator(&simulator).is_err());
}

/// The seeded run walked to its first open reward screen.
fn first_reward_screen() -> Simulator {
    walk_until(fresh_run(), |simulator| {
        matches!(simulator.decision(), DecisionContext::Rewards { .. })
    })
}

#[test]
fn an_open_reward_screen_shows_its_offers_and_two_screens_never_share_a_key() {
    // A `Rewards` decision carries its offers, not only its set id and its
    // player, so two reward screens that rolled differently key differently
    // on the observation alone; nothing outside the decision has to tell
    // them apart.
    let screen = first_reward_screen();
    let DecisionContext::Rewards { offers, .. } = screen.decision() else {
        panic!("the walk stands on a reward screen");
    };
    assert!(!offers.is_empty(), "the screen says what it is offering");

    // Pinned across samples: a standing screen is evidence, never resampled.
    let belief = ActBeliefState::from_simulator(&screen).unwrap();
    let one = belief.sample(7, 0).unwrap();
    let two = belief.sample(7, 9).unwrap();
    assert_eq!(one.decision(), screen.decision(), "the screen is pinned");
    assert_eq!(two.decision(), screen.decision());
    assert_eq!(
        one.observation_key().unwrap(),
        two.observation_key().unwrap()
    );

    // And two screens that differ only in what they offer are two keys.
    let mut state = screen.state().clone();
    let set = state
        .combat
        .as_mut()
        .and_then(|combat| combat.reward_set.as_mut())
        .or_else(|| state.run.as_mut().and_then(|run| run.reward_set.as_mut()))
        .expect("the screen is open");
    for reward in &mut set.rewards {
        reward.fingerprint.gold_amount += 40;
    }
    let richer = Simulator::from_scenario(state, sts2_content::standard_registry()).unwrap();
    assert_ne!(
        screen.observation_key().unwrap(),
        richer.observation_key().unwrap(),
        "a screen offering more gold is a different screen"
    );
}

#[test]
fn a_rolled_reward_screen_inside_a_rollout_keys_by_what_it_offers() {
    // The same rule inside a rollout: walk two sampled worlds through the
    // same fight and onto their own reward screens. The fights are dealt from
    // analysis streams, so the screens roll differently — and they key
    // differently on the observation alone.
    let original = mid_act();
    let destination = match original.decision() {
        DecisionContext::MapNavigation { destinations, .. } => {
            let run = original.state().run.as_ref().unwrap();
            *destinations
                .iter()
                .find(|coord| {
                    run.map().points.iter().any(|point| {
                        point.coord == **coord
                            && point.point_type == sts2_engine::MapPointType::Monster
                    })
                })
                .expect("a monster point is one step ahead")
        }
        other => panic!("the anchor stands on the map, not {other:?}"),
    };
    let belief = ActBeliefState::from_simulator(&original).unwrap();
    let walk_to_rewards = |rollout: u64| -> Option<(sts2_engine::ObservationKey, usize)> {
        let mut sample = belief.sample(7, rollout).unwrap();
        sample
            .step_quietly(&Action::ChooseMap { destination })
            .unwrap();
        for _ in 0..400 {
            if let DecisionContext::Rewards { offers, .. } = sample.decision() {
                return Some((sample.observation_key().unwrap(), offers.len()));
            }
            if sample.state().terminal.is_some() {
                return None;
            }
            let action = next_action(&sample);
            sample.step_quietly(&action).unwrap();
        }
        None
    };
    let screens: Vec<_> = (0..8).filter_map(walk_to_rewards).collect();
    assert!(
        screens.len() >= 2,
        "some sampled worlds reach their reward screen"
    );
    assert!(
        screens.iter().all(|(_, offers)| *offers > 0),
        "every rolled screen says what it offers"
    );
    let distinct: std::collections::HashSet<_> = screens.iter().map(|(key, _)| *key).collect();
    assert!(
        distinct.len() > 1,
        "two differently-rolled reward screens are two observation keys"
    );
}

#[test]
fn the_acts_map_and_its_previews_are_pinned_by_the_erasure() {
    // The map is on the screen from the moment the act opens, so act-v0.5
    // pins it — layout, walked trail, and the boss and ancient the map
    // previews. What sits behind an unwalked icon is what the model erases,
    // and the pieces must not have traded places.
    let original = mid_act();
    let belief = ActBeliefState::from_simulator(&original).unwrap();
    let seen = original.agent_observation();
    let map = seen.map.as_ref().expect("an act carries its map");
    for rollout in 0..4 {
        let sample = belief.sample(3, rollout).unwrap();
        assert_eq!(
            sample.agent_observation().map.as_ref(),
            Some(map),
            "the map, its trail, and its previews are evidence"
        );
    }
}
