//! The fights the last of the encounter models put up, generated the way a run
//! generates one: the act's own list names the encounter, the point is walked
//! onto with nothing placed there, and the encounter's floor-local RNG decides
//! the rest.
//!
//! No recording in the corpse walks any of these rooms, so the composition each
//! one generates, the slots it stands its monsters in, and the purse it pays
//! out are pinned here.

use std::collections::BTreeSet;

use sts2_engine::{
    ActMap, ActRoomPools, Action, CombatSide, MapCoord, MapPoint, MapPointType, RelicGrabBagState,
    RunConfig, RunPhase, RunState, Simulator,
};

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().unwrap()
}

#[test]
fn a_second_phase_killed_by_the_same_card_stays_in_room_history() {
    let simulator = walk_in_point("ENCOUNTER.PHROG_PARASITE_ELITE", 12, MapPointType::Elite);
    let mut state = simulator.state().clone();
    let combat = state.combat.as_mut().unwrap();
    combat
        .creatures
        .iter_mut()
        .find(|c| c.side == CombatSide::Enemy)
        .unwrap()
        .current_hp = 26;
    combat.player.energy = 2;
    combat.player.stars = 7;
    let card = &mut combat.player.piles.hand[0];
    card.fingerprint = sts2_engine::CardFingerprint::base(id("CARD.SEVEN_STARS"));
    card.deck_version = None;
    let card_id = card.id;
    let mut simulator = Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the two-phase fight builds");
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::PlayCard { card, .. } if card.card_id == card_id))
        .cloned()
        .expect("Seven Stars is playable");
    simulator.step(action).expect("both phases are hit");
    let combat = simulator.state().combat.as_ref().unwrap();
    assert!(combat.has_ended, "the same play killed both phases");
    let run = simulator.state().run.as_ref().unwrap();
    let room = run
        .map_history
        .last()
        .unwrap()
        .points
        .last()
        .unwrap()
        .rooms
        .last()
        .unwrap();
    let monsters: Vec<_> = room
        .monsters
        .iter()
        .map(|seen| seen.model_id.to_string())
        .collect();
    assert_eq!(monsters, ["MONSTER.PHROG_PARASITE", "MONSTER.WRIGGLER"]);
}

fn point(col: u8, row: u8, point_type: MapPointType, children: Vec<MapCoord>) -> MapPoint {
    MapPoint {
        coord: MapCoord { col, row },
        point_type,
        children,
        can_modify: true,
    }
}

/// A run standing one step away from a point of the type named that nothing has
/// placed a room on, with the act's list for that type naming exactly one
/// encounter.
fn run_facing_point(encounter: &str, floor: u32, point_type: MapPointType) -> Simulator {
    let mut state = sts2_content::shrinker_beetle_fixture()
        .unwrap()
        .state()
        .clone();
    state.combat = None;
    state.terminal = None;
    let start = MapCoord { col: 0, row: 0 };
    let map = ActMap {
        width: 1,
        height: 2,
        start,
        boss: MapCoord { col: 0, row: 1 },
        second_boss: None,
        points: vec![
            point(
                0,
                0,
                MapPointType::Ancient,
                vec![MapCoord { col: 0, row: 1 }],
            ),
            point(0, 1, point_type, Vec::new()),
        ],
    };
    state.run = Some(RunState {
        config: RunConfig::fixture_baseline("N3Z9EE2CXB"),
        current_act: 0,
        // The encounter's RNG is seeded off the run's seed, the floor, and the
        // encounter's own entry, so the floor a room is walked on is part of
        // what it generates.
        floor,
        current_coord: Some(start),
        maps: vec![map],
        rooms: vec![Vec::new()],
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
            boss: id(if point_type == MapPointType::Boss {
                encounter
            } else {
                "ENCOUNTER.THE_KIN_BOSS"
            }),
            second_boss: None,
            normal_encounters: vec![id(encounter)],
            elite_encounters: if point_type == MapPointType::Elite {
                vec![id(encounter)]
            } else {
                Vec::new()
            },
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
    Simulator::from_generated_state(state, sts2_content::standard_registry()).unwrap()
}

/// Walks onto the monster point, which is what generates the room.
fn walk_in(encounter: &str, floor: u32) -> Simulator {
    walk_in_point(encounter, floor, MapPointType::Monster)
}

fn walk_in_point(encounter: &str, floor: u32, point_type: MapPointType) -> Simulator {
    let mut simulator = run_facing_point(encounter, floor, point_type);
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseMap { .. }))
        .cloned()
        .expect("the map offers the monster point");
    simulator.step(action).unwrap();
    simulator
}

/// Every enemy the generated fight stood up, in the order the side holds them.
fn enemies(simulator: &Simulator) -> Vec<(String, Option<String>, i32)> {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .creatures
        .iter()
        .filter(|creature| creature.side == CombatSide::Enemy)
        .map(|creature| {
            (
                creature.model_id.to_string(),
                creature.slot_name.clone(),
                creature.current_hp,
            )
        })
        .collect()
}

fn models(simulator: &Simulator) -> Vec<String> {
    enemies(simulator)
        .into_iter()
        .map(|(model, _, _)| model)
        .collect()
}

/// The move each enemy is telegraphing, which is what an encounter that writes
/// an opener onto its spawns decides.
fn openers(simulator: &Simulator) -> Vec<String> {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .creatures
        .iter()
        .filter(|creature| creature.side == CombatSide::Enemy)
        .map(|creature| creature.move_state.clone())
        .collect()
}

/// Three raiders out of five, and never two of the same: the encounter draws
/// three times, taking what it drew out of the list each time.
#[test]
fn the_raiders_are_three_of_the_five_and_all_different() {
    for floor in 1..12 {
        let fight = walk_in("ENCOUNTER.RUBY_RAIDERS_NORMAL", floor);
        let drawn = models(&fight);
        assert_eq!(drawn.len(), 3, "three raiders turn up on floor {floor}");
        let unique: BTreeSet<&String> = drawn.iter().collect();
        assert_eq!(unique.len(), 3, "and no raider twice on floor {floor}");
        for raider in &drawn {
            assert!(
                [
                    "MONSTER.AXE_RUBY_RAIDER",
                    "MONSTER.ASSASSIN_RUBY_RAIDER",
                    "MONSTER.BRUTE_RUBY_RAIDER",
                    "MONSTER.CROSSBOW_RUBY_RAIDER",
                    "MONSTER.TRACKER_RUBY_RAIDER",
                ]
                .contains(&raider.as_str()),
                "{raider} is one of the five"
            );
        }
    }
}

/// The raiders stand in no slot at all, and each opens on its own machine's
/// opening state: the crossbow one loads before it fires.
#[test]
fn a_raider_opens_on_its_own_machines_first_state() {
    // Floor five is a walk that draws the crossbow raider.
    let fight = walk_in("ENCOUNTER.RUBY_RAIDERS_NORMAL", 5);
    for (model, slot, _) in enemies(&fight) {
        assert_eq!(slot, None, "{model} stands in no named slot");
    }
    let telegraphed: Vec<(String, String)> = models(&fight)
        .into_iter()
        .zip(openers(&fight))
        .filter(|(model, _)| model == "MONSTER.CROSSBOW_RUBY_RAIDER")
        .collect();
    assert_eq!(telegraphed.len(), 1, "that walk draws the crossbow raider");
    for (_, opener) in telegraphed {
        assert_eq!(opener, "RELOAD_MOVE", "the crossbow raider loads first");
    }
    // The other two of that walk open where their own machines do.
    assert_eq!(
        models(&fight)
            .into_iter()
            .zip(openers(&fight))
            .collect::<Vec<_>>(),
        vec![
            (
                "MONSTER.ASSASSIN_RUBY_RAIDER".to_owned(),
                "KILLSHOT_MOVE".to_owned()
            ),
            (
                "MONSTER.CROSSBOW_RUBY_RAIDER".to_owned(),
                "RELOAD_MOVE".to_owned()
            ),
            (
                "MONSTER.BRUTE_RUBY_RAIDER".to_owned(),
                "BEAT_MOVE".to_owned()
            ),
        ]
    );
}

/// Every raider rolls its size out of its own range.
#[test]
fn a_raider_is_rolled_out_of_its_own_health_range() {
    let ranges = [
        ("MONSTER.AXE_RUBY_RAIDER", 20, 22),
        ("MONSTER.ASSASSIN_RUBY_RAIDER", 18, 23),
        ("MONSTER.BRUTE_RUBY_RAIDER", 30, 33),
        ("MONSTER.CROSSBOW_RUBY_RAIDER", 18, 21),
        ("MONSTER.TRACKER_RUBY_RAIDER", 21, 25),
    ];
    for floor in 1..12 {
        let fight = walk_in("ENCOUNTER.RUBY_RAIDERS_NORMAL", floor);
        for (model, _, hp) in enemies(&fight) {
            let (_, low, high) = ranges
                .iter()
                .find(|(name, _, _)| *name == model)
                .expect("the raider is one of the five");
            assert!(
                (*low..=*high).contains(&hp),
                "{model} rolled {hp}, outside {low}..={high}"
            );
        }
    }
}

/// The berserker's room is one berserker, at the one size its range holds.
#[test]
fn the_slimed_berserker_stands_alone_at_two_hundred_and_sixty_one() {
    let fight = walk_in("ENCOUNTER.SLIMED_BERSERKER_NORMAL", 3);
    assert_eq!(
        enemies(&fight),
        vec![("MONSTER.SLIMED_BERSERKER".to_owned(), None, 261)]
    );
    assert_eq!(openers(&fight), vec!["VOMIT_ICHOR_MOVE".to_owned()]);
}

/// The tunneler's normal room stands a chomper up with a screech named as its
/// opener, so the chomper screeches before it clamps, and a tunneler behind
/// it.
#[test]
fn the_tunnelers_room_puts_a_screeching_chomper_in_front() {
    let fight = walk_in("ENCOUNTER.TUNNELER_NORMAL", 4);
    assert_eq!(
        models(&fight),
        vec!["MONSTER.CHOMPER".to_owned(), "MONSTER.TUNNELER".to_owned()]
    );
    assert_eq!(
        openers(&fight),
        vec!["SCREECH_MOVE".to_owned(), "BITE_MOVE".to_owned()],
        "the chomper screams first and the tunneler opens where its own machine does"
    );
}

/// Three slugs, and one roll decides which move the first opens on; the other
/// two follow it round the list.
#[test]
fn the_three_corpse_slugs_open_on_three_different_moves() {
    let round = [
        "WHIP_SLAP_MOVE".to_owned(),
        "GLOMP_MOVE".to_owned(),
        "GOOP_MOVE".to_owned(),
    ];
    let mut seen = BTreeSet::new();
    for floor in 1..12 {
        let fight = walk_in("ENCOUNTER.CORPSE_SLUGS_NORMAL", floor);
        assert_eq!(models(&fight).len(), 3, "three slugs on floor {floor}");
        let opened = openers(&fight);
        let first = round
            .iter()
            .position(|move_id| *move_id == opened[0])
            .expect("the opener is one of the three");
        assert_eq!(
            opened,
            vec![
                round[first].clone(),
                round[(first + 1) % 3].clone(),
                round[(first + 2) % 3].clone(),
            ],
            "the slugs walk the list from wherever the roll landed, on floor {floor}"
        );
        seen.insert(first);
    }
    assert_eq!(seen.len(), 3, "and the roll reaches all three openers");
}

/// Three rats in the back three of five slots, each opening on a different
/// move: one roll settles the first and the others follow it round the list.
/// The two slots in front are what the calls for backup fill.
#[test]
fn the_rat_room_fills_its_back_three_slots_and_rotates_their_openers() {
    let round = [
        "SCRATCH_MOVE".to_owned(),
        "DISEASE_BITE_MOVE".to_owned(),
        "SCREECH_MOVE".to_owned(),
    ];
    let mut seen = BTreeSet::new();
    for floor in 1..12 {
        let fight = walk_in("ENCOUNTER.TWO_TAILED_RATS_NORMAL", floor);
        assert_eq!(
            enemies(&fight)
                .into_iter()
                .map(|(model, slot, _)| (model, slot))
                .collect::<Vec<_>>(),
            vec![
                (
                    "MONSTER.TWO_TAILED_RAT".to_owned(),
                    Some("third".to_owned())
                ),
                (
                    "MONSTER.TWO_TAILED_RAT".to_owned(),
                    Some("fourth".to_owned())
                ),
                (
                    "MONSTER.TWO_TAILED_RAT".to_owned(),
                    Some("fifth".to_owned())
                ),
            ],
            "on floor {floor}"
        );
        let opened = openers(&fight);
        let first = round
            .iter()
            .position(|move_id| *move_id == opened[0])
            .expect("the opener is one of the three");
        assert_eq!(
            opened,
            vec![
                round[first].clone(),
                round[(first + 1) % 3].clone(),
                round[(first + 2) % 3].clone(),
            ]
        );
        seen.insert(first);
    }
    assert_eq!(seen.len(), 3, "and the roll reaches all three openers");
}

/// The matriarch's boss room is one matriarch, asleep behind the plating its
/// arrival works on it, and the plating's own round-one pass has already put
/// twelve block up before the player has looked at the fight.
#[test]
fn the_matriarch_boss_stands_one_matriarch_up_asleep_behind_its_shell() {
    let fight = walk_in_point("ENCOUNTER.LAGAVULIN_MATRIARCH_BOSS", 15, MapPointType::Boss);
    assert_eq!(
        enemies(&fight),
        vec![("MONSTER.LAGAVULIN_MATRIARCH".to_owned(), None, 222)]
    );
    assert_eq!(openers(&fight), vec!["SLEEP_MOVE".to_owned()]);
    let matriarch = fight
        .state()
        .combat
        .as_ref()
        .unwrap()
        .creatures
        .iter()
        .find(|creature| creature.side == CombatSide::Enemy)
        .unwrap();
    assert_eq!(matriarch.power_amount("POWER.PLATING_POWER"), 12);
    assert_eq!(matriarch.power_amount("POWER.ASLEEP_POWER"), 3);
    assert_eq!(
        matriarch.block, 12,
        "the plating puts its block up on the first round before the player acts"
    );
}

/// The fake merchant's room is one merchant in the one slot the encounter
/// names, and its purse is three hundred rather than a monster room's ten to
/// twenty.
#[test]
fn the_fake_merchants_room_is_one_merchant_worth_three_hundred_gold() {
    let fight = walk_in("ENCOUNTER.FAKE_MERCHANT_EVENT_ENCOUNTER", 6);
    assert_eq!(
        enemies(&fight),
        vec![(
            "MONSTER.FAKE_MERCHANT_MONSTER".to_owned(),
            Some("merchant".to_owned()),
            165
        )]
    );
    assert_eq!(openers(&fight), vec!["SWIPE_MOVE".to_owned()]);

    let spec = sts2_content::standard_registry()
        .encounter(&id("ENCOUNTER.FAKE_MERCHANT_EVENT_ENCOUNTER"))
        .expect("the room is content")
        .rewards
        .clone()
        .expect("it pays out");
    assert_eq!(spec.minimum_gold, 300);
    assert_eq!(spec.maximum_gold, 300);
    assert!(!spec.offers_relic);
}

/// The punch-off stands two constructs up: the first opens on its fast punch,
/// and each one is already worn down by a number the room rolled for it, two to
/// nine, in spawn order.
#[test]
fn the_punch_off_wears_both_constructs_down_before_the_fight() {
    let mut wounds = BTreeSet::new();
    for floor in 1..12 {
        let fight = walk_in("ENCOUNTER.PUNCH_OFF_EVENT_ENCOUNTER", floor);
        let standing = enemies(&fight);
        assert_eq!(
            standing
                .iter()
                .map(|(model, _, _)| model.clone())
                .collect::<Vec<_>>(),
            vec![
                "MONSTER.PUNCH_CONSTRUCT".to_owned(),
                "MONSTER.PUNCH_CONSTRUCT".to_owned()
            ]
        );
        assert_eq!(
            openers(&fight),
            vec!["FAST_PUNCH_MOVE".to_owned(), "READY_MOVE".to_owned()],
            "the first opens on the fast punch and the second where its own machine does"
        );
        for (_, _, hp) in &standing {
            let wound = 55 - hp;
            assert!(
                (2..10).contains(&wound),
                "a construct came in {wound} short of fifty-five on floor {floor}"
            );
            wounds.insert(wound);
        }
        let maxima: Vec<i32> = fight
            .state()
            .combat
            .as_ref()
            .unwrap()
            .creatures
            .iter()
            .filter(|creature| creature.side == CombatSide::Enemy)
            .map(|creature| creature.max_hp)
            .collect();
        assert_eq!(maxima, vec![55, 55], "the reduction is off current HP only");
    }
    assert!(wounds.len() > 1, "the roll is a roll");
}

/// The battleworn dummy's room fields one of three, and which one is the event's
/// to say: nothing on the map can generate it, and it pays out nothing of its
/// own.
#[test]
fn the_battleworn_dummys_room_is_the_events_to_settle() {
    let registry = sts2_content::standard_registry();
    let encounter = registry
        .encounter(&id("ENCOUNTER.BATTLEWORN_DUMMY_EVENT_ENCOUNTER"))
        .expect("the room is content");
    assert!(
        encounter.rewards.is_none(),
        "the encounter gives no rewards: the event pays out instead"
    );
    assert_eq!(
        encounter.generation,
        sts2_engine::MonsterGeneration::ChosenByTheEvent {
            options: vec![
                id("MONSTER.BATTLE_FRIEND_V1"),
                id("MONSTER.BATTLE_FRIEND_V2"),
                id("MONSTER.BATTLE_FRIEND_V3"),
            ],
        }
    );
}

/// The powers standing on a creature as the fight opens, by model and amount.
fn powers_on(simulator: &Simulator, model: &str) -> Vec<(String, i32)> {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .creatures
        .iter()
        .find(|creature| creature.model_id.to_string() == model)
        .expect("the creature is standing")
        .powers
        .iter()
        .map(|power| (power.model_id.to_string(), power.amount))
        .collect()
}

/// The powers standing on the walking player as the fight opens.
fn player_powers(simulator: &Simulator) -> Vec<(String, i32)> {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .creatures
        .iter()
        .find(|creature| creature.side == CombatSide::Player)
        .expect("the player is standing")
        .powers
        .iter()
        .map(|power| (power.model_id.to_string(), power.amount))
        .collect()
}

/// An entomancer stands its elite alone, and opens holding one hive of its own.
#[test]
fn an_entomancer_stands_alone_carrying_a_hive() {
    let fight = walk_in_point("ENCOUNTER.ENTOMANCER_ELITE", 7, MapPointType::Elite);
    assert_eq!(
        enemies(&fight),
        vec![("MONSTER.ENTOMANCER".to_owned(), None, 145)],
        "one entomancer, in no named slot, at its unascended health"
    );
    assert_eq!(
        powers_on(&fight, "MONSTER.ENTOMANCER"),
        vec![("POWER.PERSONAL_HIVE_POWER".to_owned(), 1)],
        "and it opens with one hive on it"
    );
}

/// A globe head stands its fight alone, already galvanised six.
#[test]
fn a_globe_head_stands_alone_already_galvanised() {
    let fight = walk_in("ENCOUNTER.GLOBE_HEAD_NORMAL", 7);
    assert_eq!(
        enemies(&fight),
        vec![("MONSTER.GLOBE_HEAD".to_owned(), None, 148)],
        "one globe head, in no named slot, at its unascended health"
    );
    assert_eq!(
        powers_on(&fight, "MONSTER.GLOBE_HEAD"),
        vec![("POWER.GALVANIC_POWER".to_owned(), 6)],
        "and it opens galvanised six"
    );
}

/// Four scrolls, and the moves they open on are dealt rather than rolled one by
/// one: the first is drawn, the next two follow it around a ring of three, and
/// the fourth always takes the third of the ring. So three of the four always
/// open differently and the fourth repeats one of them.
#[test]
fn the_scrolls_of_biting_deal_their_openers_around_a_ring() {
    for floor in 1..12 {
        let fight = walk_in("ENCOUNTER.SCROLLS_OF_BITING_NORMAL", floor);
        let drawn = models(&fight);
        assert_eq!(drawn.len(), 4, "four scrolls turn up on floor {floor}");
        for scroll in &drawn {
            assert_eq!(scroll, "MONSTER.SCROLL_OF_BITING", "on floor {floor}");
        }
        let opened = openers(&fight);
        let ring: BTreeSet<&String> = opened[..3].iter().collect();
        assert_eq!(ring.len(), 3, "the first three differ on floor {floor}");
        assert!(
            ring.contains(&opened[3]),
            "and the fourth repeats one of them on floor {floor}"
        );
    }
}

/// The queen of the last act stands up carrying one withering presence for each
/// creature she faces — the power sits on her and names its target, rather than
/// standing on the target — and three artifacts behind them.
#[test]
fn an_aeonglass_carries_a_withering_presence_for_each_opponent() {
    let fight = walk_in_point("ENCOUNTER.AEONGLASS_BOSS", 47, MapPointType::Boss);
    assert!(
        models(&fight).contains(&"MONSTER.AEONGLASS".to_owned()),
        "the queen is standing"
    );
    assert_eq!(
        powers_on(&fight, "MONSTER.AEONGLASS"),
        vec![
            ("POWER.WITHERING_PRESENCE_POWER".to_owned(), 6),
            ("POWER.ARTIFACT_POWER".to_owned(), 3),
        ],
        "one withering presence for the one player she faces, then three artifacts"
    );
    assert!(
        player_powers(&fight).is_empty(),
        "and nothing stands on the player, who is only named by it"
    );
}

/// The knight an event stands up: one of them, already six strong and behind
/// six plating.
#[test]
fn a_mysterious_knight_stands_up_strong_and_plated() {
    let fight = walk_in("ENCOUNTER.MYSTERIOUS_KNIGHT_EVENT_ENCOUNTER", 7);
    assert_eq!(
        models(&fight),
        vec!["MONSTER.MYSTERIOUS_KNIGHT".to_owned()],
        "one knight stands the fight up"
    );
    assert_eq!(
        powers_on(&fight, "MONSTER.MYSTERIOUS_KNIGHT"),
        vec![
            ("POWER.STRENGTH_POWER".to_owned(), 6),
            ("POWER.PLATING_POWER".to_owned(), 6),
        ],
        "six strength, then six plating"
    );
}
