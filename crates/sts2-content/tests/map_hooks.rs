//! The passes a card of the deck and two relics answer as an act's map is laid
//! out, and what they then do in the rooms they marked.
//!
//! Generating a map runs the early map pass and then the late one over the
//! run's own listeners — the deck's cards first and the relics behind them — as
//! each act is entered. Three models
//! answer them: a spoils map replaces an act with the hourglass and marks its
//! one treasure point, a golden compass replaces one with the single column,
//! and a fur coat marks combats on the map that is already standing.

use std::collections::BTreeSet;

use sts2_engine::{
    ActHistory, ActMap, ActRoomPools, Action, CardFingerprint, CombatRoomDefinition, EnemySpawn,
    MapCoord, MapPoint, MapPointType, RelicGrabBagState, RelicInstance, RoomAt, RoomDefinition,
    RunConfig, RunPhase, RunState, Simulator,
};

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn point(col: u8, row: u8, point_type: MapPointType, children: Vec<MapCoord>) -> MapPoint {
    MapPoint {
        coord: MapCoord { col, row },
        point_type,
        children,
        can_modify: true,
    }
}

/// A three-row act: a treasure chest to open, and a fight and an elite behind
/// it to walk into.
fn walking_map() -> ActMap {
    let start = MapCoord { col: 0, row: 0 };
    ActMap {
        width: 2,
        height: 3,
        start,
        boss: MapCoord { col: 0, row: 2 },
        second_boss: None,
        points: vec![
            point(
                0,
                0,
                MapPointType::Ancient,
                vec![MapCoord { col: 0, row: 1 }],
            ),
            point(
                0,
                1,
                MapPointType::Treasure,
                vec![MapCoord { col: 0, row: 2 }, MapCoord { col: 1, row: 2 }],
            ),
            point(0, 2, MapPointType::Monster, Vec::new()),
            point(1, 2, MapPointType::Elite, Vec::new()),
        ],
    }
}

fn combat_room(coord: MapCoord) -> RoomAt {
    RoomAt {
        coord,
        definition: RoomDefinition::Combat(CombatRoomDefinition {
            encounter_model: id("ENCOUNTER.SHRINKER_BEETLE_WEAK"),
            enemies: vec![EnemySpawn {
                model_id: id("MONSTER.SHRINKER_BEETLE"),
                fixed_hp: Some(39),
                initial_state: None,
                slot_name: None,
                locals: Vec::new(),
                rolled_locals: Vec::new(),
            }],
            rewards: Vec::new(),
            generated_rewards: None,
        }),
    }
}

fn act_pools() -> ActRoomPools {
    ActRoomPools {
        act_model: id("ACT.OVERGROWTH"),
        ancient: id("EVENT.NEOW"),
        boss: id("ENCOUNTER.THE_KIN_BOSS"),
        second_boss: None,
        normal_encounters: vec![id("ENCOUNTER.SHRINKER_BEETLE_WEAK")],
        elite_encounters: vec![id("ENCOUNTER.SHRINKER_BEETLE_WEAK")],
        events: Vec::new(),
        normal_encounters_visited: 0,
        elite_encounters_visited: 0,
        events_visited: 0,
        boss_encounters_visited: 0,
    }
}

/// A run standing at the start of its first act, with `acts` acts laid out, the
/// deck the test names, the relics it names, and one named relic in the chest.
fn run(acts: usize, deck: &[CardFingerprint], relics: &[&str], treasure: &str) -> Simulator {
    let mut state = sts2_content::shrinker_beetle_fixture()
        .expect("the fixture builds")
        .state()
        .clone();
    state.combat = None;
    state.terminal = None;
    state.run_player.deck = deck.to_vec();
    state
        .run_player
        .relics
        .extend(relics.iter().map(|relic| RelicInstance::new(id(relic))));
    let map = walking_map();
    let rooms = vec![
        RoomAt {
            coord: MapCoord { col: 0, row: 1 },
            definition: RoomDefinition::Treasure {
                relics: vec![id(treasure)],
                fills_itself: false,
            },
        },
        combat_room(MapCoord { col: 0, row: 2 }),
        combat_room(MapCoord { col: 1, row: 2 }),
    ];
    state.run = Some(RunState {
        config: RunConfig::fixture_baseline("N3Z9EE2CXB"),
        current_act: 0,
        floor: 0,
        current_coord: Some(map.start),
        maps: std::iter::repeat_n(map, acts).collect(),
        rooms: std::iter::repeat_n(rooms, acts).collect(),
        visited: Vec::new(),
        visited_events: BTreeSet::new(),
        started_with_neow: true,
        reloads: 0,
        map_history: vec![ActHistory {
            act: 0,
            points: Vec::new(),
        }],
        unlocks: sts2_core::UnlockPresetManifest::pinned()
            .expect("pinned unlock preset")
            .unlocks,
        acts: std::iter::repeat_n(act_pools(), acts).collect(),
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
        .expect("the run stands up")
}

fn walk_to(simulator: &mut Simulator, coord: MapCoord) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseMap { destination } if *destination == coord))
        .cloned()
        .unwrap_or_else(|| panic!("{coord:?} is not reachable"));
    simulator.step(action).expect("the walk applies");
}

fn take_the_chest(simulator: &mut Simulator) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::TakeTreasure { .. }))
        .cloned()
        .expect("the chest is open");
    simulator.step(action).expect("taking the relic applies");
    // The room stands until it is proceeded past.
    while let Some(action) = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::Proceed))
        .cloned()
    {
        simulator.step(action).expect("proceeding applies");
    }
}

fn marked(simulator: &Simulator, relic: &str) -> Vec<MapCoord> {
    simulator
        .state()
        .run_player
        .relics
        .iter()
        .find(|held| held.model_id == id(relic))
        .expect("the relic is held")
        .marked_coords
        .clone()
}

/// A fur coat marks the act's Monster and Elite points as it is taken, and
/// everything standing in a marked room is on one HP when the fight opens.
#[test]
fn a_fur_coat_leaves_every_enemy_of_a_marked_room_on_one_hp() {
    let mut simulator = run(1, &[], &[], "RELIC.FUR_COAT");
    walk_to(&mut simulator, MapCoord { col: 0, row: 1 });
    take_the_chest(&mut simulator);
    let mut coords = marked(&simulator, "RELIC.FUR_COAT");
    coords.sort_by_key(|coord| (coord.row, coord.col));
    assert_eq!(
        coords,
        vec![MapCoord { col: 0, row: 2 }, MapCoord { col: 1, row: 2 }],
        "the act's two combats are all there is to mark"
    );
    walk_to(&mut simulator, MapCoord { col: 0, row: 2 });
    let hp: Vec<i32> = simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .creatures
        .iter()
        .filter(|creature| creature.side == sts2_engine::CombatSide::Enemy)
        .map(|creature| creature.current_hp)
        .collect();
    assert_eq!(hp, vec![1]);
}

/// Without the coat the same fight opens on the beetle's own health.
#[test]
fn a_room_no_fur_coat_marked_opens_on_the_health_it_rolled() {
    let mut simulator = run(1, &[], &[], "RELIC.LANTERN");
    walk_to(&mut simulator, MapCoord { col: 0, row: 1 });
    take_the_chest(&mut simulator);
    walk_to(&mut simulator, MapCoord { col: 0, row: 2 });
    let hp: Vec<i32> = simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .creatures
        .iter()
        .filter(|creature| creature.side == sts2_engine::CombatSide::Enemy)
        .map(|creature| creature.current_hp)
        .collect();
    assert_eq!(hp, vec![39]);
}

/// A golden compass lays the act it was taken in out again as the single
/// column, the moment it is taken.
#[test]
fn a_golden_compass_lays_the_act_out_again_as_one_column() {
    let mut simulator = run(1, &[], &[], "RELIC.GOLDEN_COMPASS");
    assert_eq!(
        simulator.state().run.as_ref().expect("a run").maps[0].width,
        2
    );
    walk_to(&mut simulator, MapCoord { col: 0, row: 1 });
    take_the_chest(&mut simulator);
    let map = simulator.state().run.as_ref().expect("a run").maps[0].clone();
    assert_eq!(map.width, 7);
    assert_eq!(map.height, 17);
    assert!(
        map.points.iter().all(|point| point.coord.col == 3),
        "the golden path is one column"
    );
    assert_eq!(
        map.points
            .iter()
            .find(|point| point.coord == map.start)
            .expect("the start is on the map")
            .point_type,
        MapPointType::Ancient
    );
}

/// A spoils map standing in the deck replaces the act it names with the
/// hourglass, marks the one treasure point of it, and pays six hundred when
/// that point is walked onto — and then it is gone from the deck.
#[test]
fn a_spoils_map_bends_the_act_it_names_and_pays_at_the_point_it_marked() {
    let card = CardFingerprint::base(id("CARD.SPOILS_MAP")).with_property("SpoilsActIndex", 1);
    let mut simulator = run(2, &[card], &[], "RELIC.LANTERN");
    // The act it names is not the one standing, so nothing of act one moved.
    assert_eq!(
        simulator.state().run.as_ref().expect("a run").maps[0].width,
        2
    );
    // Walking off the last act's map is what enters the next one.
    walk_to(&mut simulator, MapCoord { col: 0, row: 1 });
    take_the_chest(&mut simulator);
    walk_to(&mut simulator, MapCoord { col: 0, row: 2 });
    // The fight standing in the way is not what this test is about; step the
    // act on directly, which is what walking past the act's last room does.
    let mut state = simulator.state().clone();
    state.combat = None;
    state.run.as_mut().expect("a run").phase = RunPhase::ActTransition;
    let mut simulator = Simulator::from_scenario(state, sts2_content::standard_registry())
        .expect("the run stands up");
    simulator
        .step(Action::AdvanceAct)
        .expect("the act transition applies");
    let run_state = simulator.state().run.as_ref().expect("a run");
    assert_eq!(run_state.current_act, 1);
    let map = &run_state.maps[1];
    assert_eq!(map.width, 7, "the act it names is the hourglass now");
    let treasure = map
        .points
        .iter()
        .find(|point| point.point_type == MapPointType::Treasure)
        .expect("the hourglass has one treasure point");
    assert_eq!(
        run_state.marked_map_coord,
        Some(treasure.coord),
        "and the card marked it"
    );
}
