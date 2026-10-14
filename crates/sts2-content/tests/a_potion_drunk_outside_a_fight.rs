//! A potion drunk outside a fight is worked out against the run's own numbers.
//!
//! Out here there is no creature list at all: the player's HP and maximum are
//! the run's, and a drink that reads a share of the drinker's maximum has to
//! read it off the run rather than off a creature that does not exist. A blood
//! potion is the one that shows it, being a fifth of whoever drinks it.

use std::collections::BTreeSet;

use sts2_engine::{
    ActMap, ActRoomPools, Action, MapCoord, MapPoint, MapPointType, RelicGrabBagState, RoomAt,
    RoomDefinition, RunConfig, RunPhase, RunState, Simulator,
};

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

/// A run standing on the map with the belt a test names, no fight anywhere.
#[allow(
    clippy::too_many_lines,
    reason = "one function stands a whole run's state up"
)]
fn standing_on_the_map(current_hp: i32, max_hp: i32, held: &[&str]) -> Simulator {
    let mut state = sts2_content::shrinker_beetle_fixture()
        .expect("the fixture builds")
        .state()
        .clone();
    state.combat = None;
    state.terminal = None;
    state.run_player.gold = 500;
    state.run_player.current_hp = current_hp;
    state.run_player.max_hp = max_hp;
    for (slot, model) in held.iter().enumerate() {
        state.run_player.potions[slot] = Some(id(model));
    }
    let start = MapCoord { col: 0, row: 0 };
    let shelf = MapCoord { col: 0, row: 1 };
    let map = ActMap {
        width: 1,
        height: 1,
        start,
        boss: shelf,
        second_boss: None,
        points: vec![
            MapPoint {
                coord: start,
                point_type: MapPointType::Ancient,
                children: vec![shelf],
                can_modify: true,
            },
            MapPoint {
                coord: shelf,
                point_type: MapPointType::RestSite,
                children: Vec::new(),
                can_modify: true,
            },
        ],
    };
    state.run = Some(RunState {
        config: RunConfig::fixture_baseline("N3Z9EE2CXB"),
        current_act: 0,
        floor: 0,
        current_coord: Some(start),
        maps: vec![map],
        rooms: vec![vec![RoomAt {
            coord: shelf,
            definition: RoomDefinition::RestSite,
        }]],
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

fn drink(simulator: &mut Simulator, model_id: &str) {
    let wanted = id(model_id);
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::UsePotion { model_id, .. } if *model_id == wanted))
        .cloned()
        .unwrap_or_else(|| panic!("{model_id} may be drunk out here"));
    simulator.step(action).expect("the drink applies");
}

/// A blood potion drunk on the map heals a fifth of the run's maximum.
#[test]
fn a_blood_potion_drunk_outside_a_fight_heals_a_fifth_of_the_maximum() {
    let mut simulator = standing_on_the_map(67, 100, &["POTION.BLOOD_POTION"]);
    assert!(
        simulator.state().combat.is_none(),
        "there is no fight to drink it in"
    );

    drink(&mut simulator, "POTION.BLOOD_POTION");
    assert_eq!(simulator.state().run_player.current_hp, 87);
}

/// The share is of the maximum rather than a fixed number, so a smaller player
/// heals less.
#[test]
fn the_share_a_blood_potion_heals_is_of_the_runs_own_maximum() {
    let mut simulator = standing_on_the_map(10, 50, &["POTION.BLOOD_POTION"]);
    drink(&mut simulator, "POTION.BLOOD_POTION");
    assert_eq!(simulator.state().run_player.current_hp, 20);
}

/// The heal stops at the maximum, and the potion is spent either way.
#[test]
fn a_blood_potion_drunk_near_full_heals_only_up_to_the_maximum() {
    let mut simulator = standing_on_the_map(95, 100, &["POTION.BLOOD_POTION"]);
    drink(&mut simulator, "POTION.BLOOD_POTION");
    assert_eq!(simulator.state().run_player.current_hp, 100);
    assert!(
        simulator.state().run_player.potions.iter().all(|slot| slot
            .as_ref()
            .is_none_or(|held| *held != id("POTION.BLOOD_POTION"))),
        "the potion was spent"
    );
}
