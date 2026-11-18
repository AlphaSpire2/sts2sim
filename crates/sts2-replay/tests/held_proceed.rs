//! Leaving a fight's rewards or a treasure room only opens the map, and the
//! recorder writes that `room.proceed` as the next point is chosen. A script
//! holds it the same way, so a potion drunk on the map in between is written
//! ahead of it, where a recording of the same play has it.

use std::collections::{BTreeSet, VecDeque};

use sts2_engine::{
    ActMap, ActRoomPools, Action, CombatRoomDefinition, DecisionContext, EnemySpawn, MapCoord,
    MapPoint, MapPointType, RelicGrabBagState, RoomAt, RoomDefinition, RunConfig, RunPhase,
    RunState, Simulator,
};
use sts2_replay::{AdapterOutcome, Event, Parser, ReplayAdapter, ScriptWriter};

const SEED: &str = "N3Z9EE2CXB";

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

const FIGHT: MapCoord = MapCoord { col: 0, row: 1 };
const TREASURE: MapCoord = MapCoord { col: 1, row: 1 };
const REST: MapCoord = MapCoord { col: 0, row: 2 };

/// One act: from the start, a fight against a beetle on one hit point that
/// pays a monster room's rewards, or a treasure room; both lead to a rest
/// site. The belt holds a fruit juice, which can be drunk on the map.
#[allow(
    clippy::too_many_lines,
    reason = "one function stands a whole run's state up"
)]
fn run() -> Simulator {
    let mut state = sts2_content::shrinker_beetle_fixture()
        .expect("the fixture builds")
        .state()
        .clone();
    state.combat = None;
    state.terminal = None;
    let start = MapCoord { col: 2, row: 0 };
    let map = ActMap {
        width: 3,
        height: 2,
        start,
        boss: REST,
        second_boss: None,
        points: vec![
            point(2, 0, MapPointType::Ancient, vec![FIGHT, TREASURE]),
            point(0, 1, MapPointType::Monster, vec![REST]),
            point(1, 1, MapPointType::Treasure, vec![REST]),
            point(0, 2, MapPointType::RestSite, Vec::new()),
        ],
    };
    let rooms = vec![
        RoomAt {
            coord: FIGHT,
            definition: RoomDefinition::Combat(CombatRoomDefinition {
                encounter_model: id("ENCOUNTER.SHRINKER_BEETLE_WEAK"),
                enemies: vec![EnemySpawn {
                    model_id: id("MONSTER.SHRINKER_BEETLE"),
                    fixed_hp: Some(1),
                    initial_state: None,
                    slot_name: None,
                    locals: Vec::new(),
                    rolled_locals: Vec::new(),
                }],
                rewards: Vec::new(),
                generated_rewards: Some(sts2_engine::GeneratedCombatRewardSpec {
                    minimum_gold: 10,
                    maximum_gold: 20,
                    scales_with_poverty: true,
                    rarity_odds: sts2_engine::RarityOdds::RegularEncounter,
                    offers_relic: false,
                }),
            }),
        },
        RoomAt {
            coord: TREASURE,
            definition: RoomDefinition::Treasure {
                relics: vec![id("RELIC.LANTERN")],
                fills_itself: false,
            },
        },
        RoomAt {
            coord: REST,
            definition: RoomDefinition::RestSite,
        },
    ];
    state.run = Some(RunState {
        config: RunConfig::fixture_baseline(SEED),
        current_act: 0,
        floor: 0,
        current_coord: Some(map.start),
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
        pending: VecDeque::new(),
        next_choice_id: 0,
        next_reward_set_id: 0,
        finish_event_after_rewards: false,
        finish_event_after_pending: false,
        purchase_owed_by_rewards: None,
        owed_by_rewards: VecDeque::new(),
        card_removals_used: 0,
        owed_purchases: VecDeque::new(),
        owed_removal: false,
        unknown_room_odds: sts2_engine::UnknownRoomOdds::start_of_act(),
    });
    state.run_player.current_hp = 40;
    state.run_player.potions = vec![Some(id("POTION.FRUIT_JUICE")), None, None];
    Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run builds")
}

/// A simulator walked one action at a time, with every action written.
struct Walk {
    simulator: Simulator,
    writer: ScriptWriter,
}

impl Walk {
    fn new() -> Self {
        let simulator = run();
        let writer = ScriptWriter::open(&simulator, "test/1", SEED, 0).expect("the run opens");
        Self { simulator, writer }
    }

    fn take(&mut self, wanted: impl Fn(&Action) -> bool, what: &str) {
        let action = self
            .simulator
            .legal_actions()
            .iter()
            .find(|action| wanted(action))
            .cloned()
            .unwrap_or_else(|| {
                panic!(
                    "{what} is not offered: {:?}",
                    self.simulator.legal_actions()
                )
            });
        self.writer
            .record(&self.simulator, &action)
            .expect("the action is scriptable");
        self.simulator.step(action).expect("the action applies");
    }

    fn walk_to(&mut self, destination: MapCoord) {
        self.take(
            |action| matches!(action, Action::ChooseMap { destination: at } if *at == destination),
            "the map point",
        );
    }

    fn win_the_fight(&mut self) {
        self.walk_to(FIGHT);
        self.take(
            |action| {
                matches!(
                    action,
                    Action::PlayCard {
                        target: Some(_),
                        ..
                    }
                )
            },
            "an attack",
        );
        assert!(
            matches!(self.simulator.decision(), DecisionContext::Rewards { .. }),
            "the fight is won and its rewards are up"
        );
    }

    fn proceed(&mut self) {
        self.take(|action| matches!(action, Action::Proceed), "the proceed");
    }

    fn drink_the_juice(&mut self) {
        self.take(
            |action| matches!(action, Action::UsePotion { model_id, .. } if *model_id == id("POTION.FRUIT_JUICE")),
            "the fruit juice",
        );
    }

    /// The script, and the record kinds after `run.start` in order.
    fn finish(self) -> (Simulator, String, Vec<String>) {
        let script = self.writer.finish();
        let kinds = script
            .lines()
            .filter_map(|line| {
                let mut parts = line.splitn(3, ' ');
                let sequence = parts.next()?;
                sequence.parse::<u64>().ok()?;
                parts.next().map(str::to_owned)
            })
            .filter(|kind| kind != "run.start")
            .collect();
        (self.simulator, script, kinds)
    }
}

/// The script replays through the adapter, one applied action per record, to
/// the state the walk stood in.
fn assert_replays_to(script: &str, walked: &Simulator) {
    let mut adapter = ReplayAdapter::new(run());
    let parser = Parser::new(std::io::Cursor::new(script.to_owned())).expect("the script parses");
    for event in parser {
        match event.expect("the record parses") {
            Event::Record(record) if record.kind == "run.start" => {}
            Event::Record(record) => {
                assert!(
                    matches!(
                        adapter.accept(&record).expect("the record replays"),
                        AdapterOutcome::Applied(_)
                    ),
                    "{} applies",
                    record.kind
                );
            }
            Event::Terminal(token) => assert_eq!(token, "*"),
        }
    }
    // The two orders reach the same game. What differs is the log of what the
    // run was seen doing, which follows the order the actions were taken in:
    // a held proceed replays after the potion, as a recording of the same
    // play does. Every other part of the state is compared.
    let unwatched = |simulator: &Simulator| {
        let mut state = simulator.state().clone();
        state.visible_history = sts2_engine::VisibleHistory::default();
        format!("{state:?}")
    };
    assert!(
        unwatched(adapter.simulator()) == unwatched(walked),
        "the replay reaches the walked state"
    );
}

fn tail(kinds: &[String], count: usize) -> Vec<&str> {
    kinds[kinds.len() - count..]
        .iter()
        .map(String::as_str)
        .collect()
}

#[test]
fn a_potion_drunk_after_leaving_a_fights_rewards_is_written_before_the_proceed() {
    let mut walk = Walk::new();
    walk.win_the_fight();
    walk.proceed();
    walk.drink_the_juice();
    walk.walk_to(REST);
    let (walked, script, kinds) = walk.finish();
    assert_eq!(
        tail(&kinds, 3),
        ["combat.use_potion", "room.proceed", "map.choose"]
    );
    assert_replays_to(&script, &walked);
}

#[test]
fn a_proceed_after_every_reward_was_taken_is_held_the_same_way() {
    let mut walk = Walk::new();
    walk.win_the_fight();
    while matches!(walk.simulator.decision(), DecisionContext::Rewards { .. }) {
        let claim = walk
            .simulator
            .legal_actions()
            .iter()
            .any(|action| matches!(action, Action::ClaimReward { .. }));
        if !claim {
            break;
        }
        walk.take(
            |action| matches!(action, Action::ClaimReward { .. }),
            "a reward",
        );
        if matches!(
            walk.simulator.decision(),
            DecisionContext::ChooseCards { .. }
        ) {
            walk.take(
                |action| matches!(action, Action::ChooseCards { cards, .. } if !cards.is_empty()),
                "a card",
            );
        }
    }
    assert!(
        matches!(walk.simulator.decision(), DecisionContext::RoomProceed),
        "every reward is taken and only the proceed is left: {:?}",
        walk.simulator.decision()
    );
    walk.proceed();
    walk.drink_the_juice();
    walk.walk_to(REST);
    let (walked, script, kinds) = walk.finish();
    assert_eq!(
        tail(&kinds, 3),
        ["combat.use_potion", "room.proceed", "map.choose"]
    );
    assert_replays_to(&script, &walked);
}

#[test]
fn a_potion_drunk_after_leaving_a_treasure_room_is_written_before_the_proceed() {
    let mut walk = Walk::new();
    walk.walk_to(TREASURE);
    walk.take(
        |action| matches!(action, Action::TakeTreasure { .. }),
        "the relic",
    );
    walk.proceed();
    walk.drink_the_juice();
    walk.walk_to(REST);
    let (walked, script, kinds) = walk.finish();
    assert_eq!(
        tail(&kinds, 4),
        [
            "treasure.choose_relic",
            "combat.use_potion",
            "room.proceed",
            "map.choose"
        ]
    );
    assert_replays_to(&script, &walked);
}

#[test]
fn a_proceed_with_nothing_drunk_is_still_written_right_before_the_map_choice() {
    let mut walk = Walk::new();
    walk.win_the_fight();
    walk.proceed();
    walk.walk_to(REST);
    let (walked, script, kinds) = walk.finish();
    assert_eq!(tail(&kinds, 2), ["room.proceed", "map.choose"]);
    assert_replays_to(&script, &walked);
}

/// A proceed no map choice ever followed is never written: the recorder only
/// writes one beside the choice.
#[test]
fn a_held_proceed_with_no_map_choice_after_it_is_dropped() {
    let mut walk = Walk::new();
    walk.win_the_fight();
    walk.proceed();
    let (_, _, kinds) = walk.finish();
    assert!(
        !kinds.iter().any(|kind| kind == "room.proceed"),
        "{kinds:?}"
    );
}

/// A fight that returns to the event that started it is left on the spot:
/// the recorder writes that proceed as it is taken, and the event goes on.
#[test]
fn a_proceed_back_into_the_event_that_started_the_fight_is_written_where_it_is_taken() {
    let mut walk = Walk::new();
    walk.win_the_fight();
    let mut state = walk.simulator.state().clone();
    state.run.as_mut().expect("a run").pushed_room = Some(sts2_engine::PushedRoom {
        model_id: id("ENCOUNTER.SHRINKER_BEETLE_WEAK"),
        id: 1,
        room_type: MapPointType::Monster,
        resumes_parent: true,
        resume: Vec::new(),
    });
    let resuming = Simulator::from_scenario(state, sts2_content::standard_registry())
        .expect("the state stands");
    let mut writer = ScriptWriter::open(&run(), "test/1", SEED, 0).expect("the run opens");
    writer
        .record(&resuming, &Action::Proceed)
        .expect("the proceed is scriptable");
    let script = writer.finish();
    assert!(script.contains(" room.proceed "), "{script}");
}

/// Seeded walks that use the belt on the map whenever they have just left a
/// room: no script ever has the belt between a `room.proceed` and the map
/// choice after it — the order a playback refuses — and every script replays
/// to its walk.
#[test]
fn walks_that_drink_on_the_map_never_write_the_belt_after_a_proceed() {
    let mut gap_actions = 0_usize;
    for seed in [
        "NLD6VZXP94",
        "N3Z9EE2CXB",
        "QEY5K1P4LY",
        "A1B2C3D4E5",
        "ZXCVBNMASD",
        "HPWALK000000",
        "HPWALK000025",
    ] {
        let mut walked = sts2_content::standard_ironclad_run_at(seed, 0).expect("seeded run");
        let mut writer = ScriptWriter::open(&walked, "test/1", seed, 0).expect("the run opens");
        let mut pick = 0x2545_f491_u64;
        let mut just_proceeded = false;
        for _ in 0..600 {
            if walked.state().terminal.is_some() {
                break;
            }
            pick = pick.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let legal = walked.legal_actions();
            let on_the_map = matches!(walked.decision(), DecisionContext::MapNavigation { .. });
            let belt = legal
                .iter()
                .find(|action| {
                    matches!(
                        action,
                        Action::UsePotion { .. } | Action::DiscardPotion { .. }
                    )
                })
                .cloned();
            #[allow(
                clippy::cast_possible_truncation,
                reason = "a pick among a few actions"
            )]
            let action = match belt {
                Some(belt) if just_proceeded && on_the_map => {
                    gap_actions += 1;
                    belt
                }
                _ => legal[(pick >> 33) as usize % legal.len()].clone(),
            };
            just_proceeded = matches!(action, Action::Proceed);
            writer
                .record(&walked, &action)
                .expect("the walk stays scriptable");
            walked.step(action).expect("the action applies");
        }
        let script = writer.finish();
        let kinds: Vec<&str> = script
            .lines()
            .filter_map(|line| {
                let mut parts = line.splitn(3, ' ');
                parts.next()?.parse::<u64>().ok()?;
                parts.next()
            })
            .collect();
        // The shape a playback refuses: a proceed, nothing but the belt, then
        // the map choice. A fight that returns to its event writes its proceed
        // on the spot, and the event's own screens can follow it.
        for (index, kind) in kinds.iter().enumerate() {
            if *kind != "room.proceed" {
                continue;
            }
            let after: Vec<&str> = kinds[index + 1..]
                .iter()
                .copied()
                .take_while(|kind| matches!(*kind, "combat.use_potion" | "combat.discard_potion"))
                .collect();
            assert!(
                after.is_empty() || kinds.get(index + 1 + after.len()) != Some(&"map.choose"),
                "seed {seed}: the belt sits between a proceed and the map choice: {kinds:?}"
            );
        }
        let mut adapter = ReplayAdapter::new(
            sts2_content::standard_ironclad_run_at(seed, 0).expect("seeded run"),
        );
        for event in Parser::new(std::io::Cursor::new(script)).expect("the script parses") {
            if let Event::Record(record) = event.expect("the record parses")
                && record.kind != "run.start"
            {
                adapter.accept(&record).expect("the record replays");
            }
        }
        let unwatched = |simulator: &Simulator| {
            let mut state = simulator.state().clone();
            state.visible_history = sts2_engine::VisibleHistory::default();
            format!("{state:?}")
        };
        assert!(
            unwatched(adapter.simulator()) == unwatched(&walked),
            "seed {seed}: the replay reaches the walked state"
        );
    }
    assert!(
        gap_actions > 0,
        "some walk used the belt between a proceed and the map"
    );
}
