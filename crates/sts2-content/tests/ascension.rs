//! The ten ascension levels, one rule at a time.
//!
//! No recording of this baseline was played above ascension zero, so a scenario
//! is the evidence for every level: a generated run at each of the eleven
//! levels for one reference seed, and beside it a focused test of each rule at
//! the level it starts applying and at the level below it, so that both sides
//! of every branch are read rather than only the ascended one.

use std::collections::BTreeSet;

use sts2_engine::{
    ActMap, ActRoomPools, Action, AscensionLevel, CombatRoomDefinition, EnemySpawn, MapCoord,
    MapPoint, MapPointType, RelicGrabBagState, RoomAt, RoomDefinition, RunConfig, RunPhase,
    RunState, Simulator,
};

const SEED: &str = "N3Z9EE2CXB";

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn run_at(ascension: u8) -> Simulator {
    sts2_content::standard_ironclad_run_at(SEED, ascension).expect("the run generates")
}

fn elite_counts(run: &RunState) -> Vec<usize> {
    run.maps
        .iter()
        .map(|map| {
            map.points
                .iter()
                .filter(|point| point.point_type == MapPointType::Elite)
                .count()
        })
        .collect()
}

// --- the reference run at every level ------------------------------------

/// A run generated at the baseline is the run the baseline generates: naming
/// the level explicitly changes nothing about it.
#[test]
fn naming_the_baseline_level_generates_the_baseline_run() {
    let named = run_at(0);
    let unnamed = sts2_content::standard_ironclad_run(SEED).expect("the run generates");
    assert_eq!(
        format!("{:?}", named.state()),
        format!("{:?}", unnamed.state())
    );
}

/// What one reference run opens with at one level: the level, the health it
/// opens on, its potion slots, its deck size, how many elite points each of its
/// maps carries, and the second boss its last act drew, if any.
type ReferenceRun = (u8, i32, usize, usize, usize, Option<&'static str>);

/// One reference seed generated at each of the eleven levels, pinned detail by
/// detail: what the run opens with, what its maps are shaped like, and which
/// bosses stand at the end of them.
///
/// The map's own dice never move — every act keeps its length whatever the
/// level — because nothing an ascension changes about a map is rolled.
#[test]
fn the_reference_seed_generates_the_same_run_at_every_level_but_for_the_rules() {
    // level, opening hp, potion slots, deck size, elites per act, second boss
    let expected: [ReferenceRun; 11] = [
        (0, 80, 3, 10, 5, None),
        (1, 80, 3, 10, 8, None),
        (2, 64, 3, 10, 8, None),
        (3, 64, 3, 10, 8, None),
        (4, 64, 2, 10, 8, None),
        (5, 64, 2, 11, 8, None),
        (6, 64, 2, 11, 8, None),
        (7, 64, 2, 11, 8, None),
        (8, 64, 2, 11, 8, None),
        (9, 64, 2, 11, 8, None),
        (10, 64, 2, 11, 8, Some("ENCOUNTER.AEONGLASS_BOSS")),
    ];
    for (ascension, hp, potions, deck, elites, second_boss) in expected {
        let simulator = run_at(ascension);
        let state = simulator.state();
        let run = state.run.as_ref().expect("a generated run has run state");
        assert_eq!(run.config.ascension, ascension);
        assert_eq!(state.run_player.current_hp, hp, "hp at {ascension}");
        assert_eq!(state.run_player.max_hp, 80, "max hp at {ascension}");
        // Nothing in the ten levels touches the gold a run opens with.
        assert_eq!(state.run_player.gold, 99, "gold at {ascension}");
        assert_eq!(
            state.run_player.potions.len(),
            potions,
            "potion slots at {ascension}"
        );
        assert_eq!(state.run_player.deck.len(), deck, "deck at {ascension}");
        assert_eq!(
            elite_counts(run),
            vec![elites; 3],
            "elite points at {ascension}"
        );
        // The acts, their lengths and their first bosses are the same run
        // whatever the level: no rule here moves the up-front stream before the
        // last act's own rooms are drawn.
        assert_eq!(
            run.acts
                .iter()
                .map(|act| act.act_model.to_string())
                .collect::<Vec<_>>(),
            ["ACT.OVERGROWTH", "ACT.HIVE", "ACT.GLORY"],
            "acts at {ascension}"
        );
        assert_eq!(
            run.maps.iter().map(|map| map.height).collect::<Vec<_>>(),
            [16, 15, 14],
            "map lengths at {ascension}"
        );
        assert_eq!(
            run.acts
                .iter()
                .map(|act| act.boss.to_string())
                .collect::<Vec<_>>(),
            [
                "ENCOUNTER.THE_KIN_BOSS",
                "ENCOUNTER.KAISER_CRAB_BOSS",
                "ENCOUNTER.TEST_SUBJECT_BOSS"
            ],
            "bosses at {ascension}"
        );
        assert_eq!(
            run.acts[2].second_boss.as_ref().map(ToString::to_string),
            second_boss.map(ToOwned::to_owned),
            "second boss at {ascension}"
        );
        // Only the last act draws one, so the two before it never hold one.
        assert!(run.acts[0].second_boss.is_none());
        assert!(run.acts[1].second_boss.is_none());
    }
}

// --- 1: Swarming Elites ---------------------------------------------------

/// A map's elite count is five multiplied by one and six tenths at Swarming
/// Elites, and five multiplied by one below it, each rounded: eight points
/// rather than five, on a map laid out by exactly the same dice.
///
/// Three more elites have to stand somewhere. The shops, the unknown points and
/// the forced treasure row are untouched, but a map that cannot hold both loses
/// rest sites to them, because the repair pass tops the types back up in the
/// order shop, elite, rest site, unknown and the elites are
/// repaired first. The last act of this seed is such a map.
#[test]
fn swarming_elites_puts_eight_elite_points_on_a_map_that_carried_five() {
    let baseline = run_at(0);
    let swarming = run_at(1);
    let below = baseline.state().run.as_ref().expect("run state");
    let at = swarming.state().run.as_ref().expect("run state");
    assert_eq!(elite_counts(below), vec![5, 5, 5]);
    assert_eq!(elite_counts(at), vec![8, 8, 8]);
    let count = |run: &RunState, family: MapPointType| {
        run.maps
            .iter()
            .map(|map| {
                map.points
                    .iter()
                    .filter(|point| point.point_type == family)
                    .count()
            })
            .collect::<Vec<_>>()
    };
    for family in [
        MapPointType::Shop,
        MapPointType::Unknown,
        MapPointType::Treasure,
    ] {
        assert_eq!(count(below, family), count(at, family), "{family:?} points");
    }
    assert_eq!(count(below, MapPointType::RestSite), [9, 9, 7]);
    assert_eq!(count(at, MapPointType::RestSite), [9, 9, 5]);
}

// --- 2: Weary Traveler ----------------------------------------------------

/// Neow zeroes the player and then heals back everything that is missing, so a
/// Weary Traveler run opens on four fifths of the bar.
#[test]
fn weary_traveler_opens_the_run_four_fifths_alive() {
    assert_eq!(run_at(1).state().run_player.current_hp, 80);
    assert_eq!(run_at(2).state().run_player.current_hp, 64);
}

/// And the same at any other ancient, where what is missing is only what the
/// run has lost. The heal is truncated rather than rounded, so a run missing
/// forty points gets thirty-two of them back rather than a fraction more.
#[test]
fn weary_traveler_leaves_a_later_ancient_healing_four_fifths_of_what_is_missing() {
    for (ascension, healed_to) in [(1, 80), (2, 72)] {
        let mut simulator = one_room_run(ascension, RoomChoice::Ancient);
        assert_eq!(simulator.state().run_player.current_hp, 40);
        walk_to(&mut simulator, 4);
        assert_eq!(
            simulator.state().run_player.current_hp,
            healed_to,
            "ancient heal at {ascension}"
        );
    }
}

// --- 3: Poverty -----------------------------------------------------------

/// The least and most gold a fight pays are the room type's ten to twenty with
/// a quarter taken off and the remainder dropped: seven to fifteen.
#[test]
fn poverty_pays_three_quarters_of_a_monster_room_s_purse() {
    for (ascension, least, most) in [(2, 10, 20), (3, 7, 15)] {
        let gold = purse_of_a_won_fight(ascension);
        assert!(
            (least..=most).contains(&gold),
            "a purse of {gold} at ascension {ascension} is outside {least}..={most}"
        );
    }
    // The same seed rolls the same point in the range, so the two purses are
    // the same roll scaled rather than two different rolls.
}

/// And a chest's own gold, which is rolled and then scaled the same way.
#[test]
fn poverty_takes_a_quarter_off_a_chest_s_gold() {
    for (ascension, gained) in [(2, 50_i32), (3, 37)] {
        let mut simulator = one_room_run(ascension, RoomChoice::Treasure);
        let before = simulator.state().run_player.gold;
        walk_to(&mut simulator, 4);
        assert_eq!(
            simulator.state().run_player.gold - before,
            gained,
            "chest gold at {ascension}"
        );
    }
}

/// The one encounter of the build that names both its gold numbers for itself
/// carries a purse of its own rather than the room type's, so Poverty never
/// reaches it.
#[test]
fn poverty_leaves_an_overridden_purse_alone() {
    let spec = sts2_content::registry_at(&id("CHARACTER.IRONCLAD"), 10)
        .encounter(&id("ENCOUNTER.FAKE_MERCHANT_EVENT_ENCOUNTER"))
        .expect("the encounter is content")
        .rewards
        .clone()
        .expect("the encounter pays a purse");
    assert_eq!((spec.minimum_gold, spec.maximum_gold), (300, 300));
    assert!(!spec.scales_with_poverty);
}

// --- 4: Tight Belt --------------------------------------------------------

#[test]
fn tight_belt_takes_one_of_the_three_potion_slots() {
    assert_eq!(run_at(3).state().run_player.potions.len(), 3);
    assert_eq!(run_at(4).state().run_player.potions.len(), 2);
}

// --- 5: Ascender's Bane ---------------------------------------------------

/// The curse is appended, so it stands behind the whole starting deck, and it
/// is written on the first floor like everything else the run opens with.
#[test]
fn ascenders_bane_appends_its_curse_to_the_starting_deck() {
    let below = run_at(4);
    assert_eq!(below.state().run_player.deck.len(), 10);
    assert!(
        !below
            .state()
            .run_player
            .deck
            .iter()
            .any(|card| card.model_id == id("CARD.ASCENDERS_BANE"))
    );
    let at = run_at(5);
    let deck = &at.state().run_player.deck;
    assert_eq!(deck.len(), 11);
    let curse = deck.last().expect("the deck is not empty");
    assert_eq!(curse.model_id, id("CARD.ASCENDERS_BANE"));
    assert_eq!(curse.floor_added_to_deck, Some(1));
}

// --- 6: Inflation ---------------------------------------------------------

/// What a card removal opens at and what each one adds to the next, both
/// raised, and the price a generated shop actually asks.
#[test]
fn inflation_raises_the_removal_price_and_what_each_removal_adds_to_it() {
    for (ascension, base, increase) in [(5, 75, 25), (6, 100, 50)] {
        let merchant = sts2_content::registry_at(&id("CHARACTER.IRONCLAD"), ascension)
            .merchant(&id("CHARACTER.IRONCLAD"))
            .expect("the character has a merchant")
            .clone();
        assert_eq!(merchant.removal_base_cost, base, "base at {ascension}");
        assert_eq!(
            merchant.removal_price_increase, increase,
            "increase at {ascension}"
        );
        let mut simulator = one_room_run(ascension, RoomChoice::Shop);
        walk_to(&mut simulator, 0);
        let removal_price = match simulator
            .state()
            .run
            .as_ref()
            .and_then(|run| run.active_room.as_ref())
        {
            Some(sts2_engine::ActiveRoom::Shop { removal_price, .. }) => *removal_price,
            other => panic!("the shop point stands up a shop, not {other:?}"),
        };
        assert_eq!(removal_price, base, "shop removal price at {ascension}");
    }
}

// --- 7: Scarcity ----------------------------------------------------------

/// The rare threshold of every source that rolls one, halved, and the drift a
/// missed roll adds, halved with it.
#[test]
fn scarcity_halves_the_rare_odds_and_the_drift_behind_them() {
    let same = |left: f32, right: f32| (left - right).abs() < 1e-9;
    for (odds, below, at) in [
        (sts2_engine::RarityOdds::RegularEncounter, 0.03, 0.0149),
        (sts2_engine::RarityOdds::EliteEncounter, 0.1, 0.05),
        (sts2_engine::RarityOdds::Shop, 0.09, 0.045),
    ] {
        assert!(same(odds.base(6).0, below), "{odds:?} below Scarcity");
        assert!(same(odds.base(7).0, at), "{odds:?} at Scarcity");
        // The uncommon threshold above it is the same at every level.
        assert!(same(odds.base(6).1, odds.base(7).1));
    }
    // A boss offer is a rare whatever the level.
    assert!(same(sts2_engine::RarityOdds::BossEncounter.base(7).0, 1.0));
    assert!(same(sts2_engine::PlayerOdds::card_rarity_growth(6), 0.01));
    assert!(same(sts2_engine::PlayerOdds::card_rarity_growth(7), 0.005));
    // The upgraded-card odds, per act already walked.
    assert!(same(sts2_engine::upgraded_card_odd_scaling(6), 0.25));
    assert!(same(sts2_engine::upgraded_card_odd_scaling(7), 0.125));
}

/// And the drift as a run actually moves it: a monster room's offer that came
/// up short pushes the next roll by a hundredth below Scarcity and by half of
/// that at it.
#[test]
fn scarcity_halves_the_drift_a_missed_roll_leaves_behind() {
    for (ascension, offset) in [(6, -0.02_f32), (7, -0.035_f32)] {
        let mut simulator = one_room_run(ascension, RoomChoice::Fight);
        assert!(
            (simulator.state().run_player.odds.card_rarity_offset()
                - sts2_engine::PlayerOdds::CARD_RARITY_START)
                .abs()
                < 1e-9
        );
        walk_to(&mut simulator, 3);
        play_a_strike(&mut simulator);
        assert!(
            (simulator.state().run_player.odds.card_rarity_offset() - offset).abs() < 1e-6,
            "drift at {ascension} is {}",
            simulator.state().run_player.odds.card_rarity_offset()
        );
    }
}

// --- 8 and 9: Tough and Deadly Enemies ------------------------------------

/// The two per-monster levels, read through a generated run rather than
/// through a registry built by hand: an ascended run configuration is what
/// stands the ascended monster up.
#[test]
fn a_generated_run_fields_the_ascended_monsters() {
    for (ascension, least, most) in [(7, 38, 40), (8, 40, 42)] {
        let mut simulator = one_room_run(ascension, RoomChoice::RolledFight);
        walk_to(&mut simulator, 3);
        let enemy = simulator
            .state()
            .combat
            .as_ref()
            .expect("the fight stands up")
            .creatures
            .iter()
            .find(|creature| creature.side == sts2_engine::CombatSide::Enemy)
            .expect("the fight has an enemy");
        assert!(
            (least..=most).contains(&enemy.max_hp),
            "a shrinker beetle of {} at ascension {ascension} is outside {least}..={most}",
            enemy.max_hp
        );
    }
    // And the deadly side of the same pair, which is what the monster hits for.
    for (ascension, damage) in [(8, 7), (9, 8)] {
        let mut simulator = one_room_run(ascension, RoomChoice::Chomping);
        walk_to(&mut simulator, 3);
        let before = player_hp(&simulator);
        end_the_turn(&mut simulator);
        assert_eq!(
            before - player_hp(&simulator),
            damage,
            "a chomp at ascension {ascension}"
        );
    }
}

// --- 10: Double Boss ------------------------------------------------------

/// The last act draws a second boss out of the ones it did not already roll,
/// and lays out the point that fight stands on a row behind the first.
#[test]
fn double_boss_puts_a_second_fight_behind_the_last_act_s_boss() {
    let below = run_at(9);
    let below = below.state().run.as_ref().expect("run state");
    assert!(below.acts[2].second_boss.is_none());
    assert!(below.maps[2].second_boss.is_none());

    let at = run_at(10);
    let at = at.state().run.as_ref().expect("run state");
    let second = at.acts[2]
        .second_boss
        .clone()
        .expect("the last act draws a second boss");
    assert_ne!(second, at.acts[2].boss);
    // It is drawn out of the act's own boss table.
    assert!(
        ["ENCOUNTER.AEONGLASS_BOSS", "ENCOUNTER.QUEEN_BOSS"].contains(&second.to_string().as_str())
    );
    let point = at.maps[2]
        .second_boss
        .expect("the last act's map lays out the point");
    assert_eq!(point.row, at.maps[2].height + 1);
    let boss_point = at.maps[2]
        .points
        .iter()
        .find(|candidate| candidate.coord == at.maps[2].boss)
        .expect("the map holds its boss point");
    assert_eq!(boss_point.children, [point]);
    at.maps[2].validate().expect("the map is internally valid");
}

/// The next boss is the act's own until one has been fought, and the second
/// one after that. Proceeding past the first opens the map,
/// because the fight behind it is still a point to walk onto.
#[test]
fn the_second_boss_is_fielded_only_after_the_first_one_has_been_fought() {
    let mut simulator = double_boss_run();
    walk_to_coord(&mut simulator, MapCoord { col: 1, row: 1 });
    assert_eq!(
        current_encounter(&simulator),
        Some("ENCOUNTER.TEST_SUBJECT_BOSS".to_owned())
    );
    play_a_strike(&mut simulator);
    // The last act's boss pays nothing, so proceeding is all there is to do —
    // and it opens the map rather than asking for the next act.
    assert_eq!(
        simulator.legal_actions().first(),
        Some(&Action::Proceed),
        "the first boss of a double-boss map proceeds into the map"
    );
    simulator.step(Action::Proceed).expect("proceeding applies");
    assert_eq!(
        simulator.state().run.as_ref().expect("run state").phase,
        RunPhase::Map
    );
    walk_to_coord(&mut simulator, MapCoord { col: 1, row: 2 });
    assert_eq!(
        current_encounter(&simulator),
        Some("ENCOUNTER.AEONGLASS_BOSS".to_owned())
    );
    play_a_strike(&mut simulator);
    // The second boss ends gameplay without a departure into presentation.
    assert_eq!(
        simulator.state().terminal,
        Some(sts2_engine::RunResult::Victory),
        "the second boss completes the run"
    );
    assert_eq!(simulator.legal_actions(), []);
}

// --- the harness ----------------------------------------------------------

fn player_hp(simulator: &Simulator) -> i32 {
    simulator.state().run_player.current_hp
}

fn play_a_strike(simulator: &mut Simulator) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::PlayCard { card, .. }
            if card.fingerprint.model_id == id("CARD.STRIKE_IRONCLAD"))
        })
        .cloned()
        .expect("a strike is playable");
    simulator.step(action).expect("the play applies");
}

fn end_the_turn(simulator: &mut Simulator) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::EndTurn { .. }))
        .cloned()
        .expect("ending the turn is offered");
    simulator.step(action).expect("the turn ends");
}

fn walk_to(simulator: &mut Simulator, col: u8) {
    walk_to_coord(simulator, MapCoord { col, row: 1 });
}

fn walk_to_coord(simulator: &mut Simulator, destination: MapCoord) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(
            |action| matches!(action, Action::ChooseMap { destination: at } if *at == destination),
        )
        .cloned()
        .unwrap_or_else(|| panic!("{destination:?} is not reachable"));
    simulator.step(action).expect("the walk applies");
}

fn current_encounter(simulator: &Simulator) -> Option<String> {
    simulator
        .state()
        .run
        .as_ref()
        .and_then(|run| run.current_room_model.as_ref())
        .map(ToString::to_string)
}

/// The purse a won monster room actually paid, which is the gold reward the
/// set was built with.
fn purse_of_a_won_fight(ascension: u8) -> i32 {
    let mut simulator = one_room_run(ascension, RoomChoice::Fight);
    walk_to(&mut simulator, 3);
    play_a_strike(&mut simulator);
    let before = simulator.state().run_player.gold;
    let claim = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ClaimReward { .. }))
        .cloned()
        .expect("a won monster room offers its purse");
    simulator.step(claim).expect("the purse is claimed");
    simulator.state().run_player.gold - before
}

/// Which of the four rooms the one-room run's second row stands up.
#[derive(Clone, Copy)]
enum RoomChoice {
    /// A shrinker beetle with one point of health and a monster room's purse,
    /// so the fight is over in the opening turn.
    Fight,
    /// The same fight with the monster's health rolled rather than fixed,
    /// which is what reads the Tough Enemies side of its size.
    RolledFight,
    /// The same again, standing on its chomp, which is what reads the Deadly
    /// Enemies side of its damage.
    Chomping,
    Treasure,
    Shop,
    Ancient,
}

/// One act with a single row of rooms, generated at a named ascension.
///
/// The player walks in at forty of eighty, which is what a healing rule has
/// something to heal.
#[allow(
    clippy::too_many_lines,
    reason = "one function stands a whole run's state up"
)]
fn one_room_run(ascension: u8, choice: RoomChoice) -> Simulator {
    let mut state = run_at(ascension).state().clone();
    state.combat = None;
    state.terminal = None;
    let start = MapCoord { col: 2, row: 0 };
    let children = (0..=4).map(|col| MapCoord { col, row: 1 }).collect();
    let mut points = vec![point(2, 0, MapPointType::Ancient, children)];
    for (col, point_type) in [
        (0, MapPointType::Shop),
        (1, MapPointType::RestSite),
        (2, MapPointType::Unknown),
        (3, MapPointType::Monster),
        (4, MapPointType::Treasure),
    ] {
        points.push(point(col, 1, point_type, Vec::new()));
    }
    if matches!(choice, RoomChoice::Ancient) {
        let treasure = points
            .iter_mut()
            .find(|point| point.coord == MapCoord { col: 4, row: 1 })
            .expect("the row holds a treasure point");
        treasure.point_type = MapPointType::Ancient;
    }
    let map = ActMap {
        width: 5,
        height: 1,
        start,
        boss: MapCoord { col: 3, row: 1 },
        second_boss: None,
        points,
    };
    let fight = |fixed_hp: Option<i32>, initial_state: Option<String>| RoomAt {
        coord: MapCoord { col: 3, row: 1 },
        definition: RoomDefinition::Combat(CombatRoomDefinition {
            encounter_model: id("ENCOUNTER.SHRINKER_BEETLE_WEAK"),
            enemies: vec![EnemySpawn {
                model_id: id("MONSTER.SHRINKER_BEETLE"),
                fixed_hp,
                initial_state,
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
    };
    let rooms = vec![match choice {
        RoomChoice::Fight | RoomChoice::Treasure | RoomChoice::Shop | RoomChoice::Ancient => {
            fight(Some(1), None)
        }
        RoomChoice::RolledFight => fight(None, None),
        RoomChoice::Chomping => fight(Some(400), Some("CHOMP_MOVE".to_owned())),
    }];
    let run = state.run.as_mut().expect("a generated run has run state");
    run.current_act = 0;
    run.floor = 0;
    run.current_coord = Some(start);
    run.maps = vec![map];
    run.rooms = vec![rooms];
    run.visited = Vec::new();
    run.visited_events = BTreeSet::new();
    run.map_history = vec![sts2_engine::ActHistory {
        act: 0,
        points: Vec::new(),
    }];
    run.acts = vec![ActRoomPools {
        act_model: id("ACT.OVERGROWTH"),
        ancient: id("EVENT.NEOW"),
        boss: id("ENCOUNTER.THE_KIN_BOSS"),
        second_boss: None,
        normal_encounters: vec![id("ENCOUNTER.SHRINKER_BEETLE_WEAK")],
        elite_encounters: Vec::new(),
        events: vec![id("EVENT.OROBAS")],
        normal_encounters_visited: 0,
        elite_encounters_visited: 0,
        events_visited: 0,
        boss_encounters_visited: 0,
    }];
    run.phase = RunPhase::Map;
    run.current_room_model = None;
    run.next_room_id = 1;
    run.current_room_id = 0;
    run.current_room_type = None;
    run.previous_room_type = None;
    run.active_room = None;
    run.pushed_room = None;
    run.begun_event = None;
    run.shared_relic_grab_bag = RelicGrabBagState::default();
    run.player_relic_grab_bag = RelicGrabBagState::default();
    run.reward_set = None;
    state.run_player.current_hp = 40;
    state.run_player.max_hp = 80;
    Simulator::from_generated_state(
        state,
        sts2_content::registry_at(&id("CHARACTER.IRONCLAD"), ascension),
    )
    .expect("the run builds")
}

/// The last act of a Double Boss run standing in front of its two bosses:
/// one monster point, the boss behind it, and the second boss behind that.
fn double_boss_run() -> Simulator {
    let mut state = run_at(10).state().clone();
    state.combat = None;
    state.terminal = None;
    let start = MapCoord { col: 1, row: 0 };
    let boss = MapCoord { col: 1, row: 1 };
    let second_boss = MapCoord { col: 1, row: 2 };
    let map = ActMap {
        width: 3,
        height: 1,
        start,
        boss,
        second_boss: Some(second_boss),
        points: vec![
            point(1, 0, MapPointType::Ancient, vec![boss]),
            point(1, 1, MapPointType::Boss, vec![second_boss]),
            point(1, 2, MapPointType::Boss, Vec::new()),
        ],
    };
    map.validate().expect("the map is internally valid");
    let boss_fight = |coord: MapCoord, encounter: &str| RoomAt {
        coord,
        definition: RoomDefinition::Combat(CombatRoomDefinition {
            encounter_model: id(encounter),
            enemies: vec![EnemySpawn {
                model_id: id("MONSTER.SHRINKER_BEETLE"),
                fixed_hp: Some(1),
                initial_state: None,
                slot_name: None,
                locals: Vec::new(),
                rolled_locals: Vec::new(),
            }],
            rewards: Vec::new(),
            generated_rewards: None,
        }),
    };
    let run = state.run.as_mut().expect("a generated run has run state");
    // The rooms are placed so the fight is over in a blow; which encounter each
    // point fields is still the act's own, and that is what is being read.
    run.rooms = vec![vec![
        boss_fight(boss, "ENCOUNTER.TEST_SUBJECT_BOSS"),
        boss_fight(second_boss, "ENCOUNTER.AEONGLASS_BOSS"),
    ]];
    run.current_act = 0;
    run.floor = 0;
    run.current_coord = Some(start);
    run.maps = vec![map];
    run.visited = Vec::new();
    run.visited_events = BTreeSet::new();
    run.map_history = vec![sts2_engine::ActHistory {
        act: 0,
        points: Vec::new(),
    }];
    run.acts = vec![ActRoomPools {
        act_model: id("ACT.GLORY"),
        ancient: id("EVENT.VAKUU"),
        boss: id("ENCOUNTER.TEST_SUBJECT_BOSS"),
        second_boss: Some(id("ENCOUNTER.AEONGLASS_BOSS")),
        normal_encounters: Vec::new(),
        elite_encounters: Vec::new(),
        events: Vec::new(),
        normal_encounters_visited: 0,
        elite_encounters_visited: 0,
        events_visited: 0,
        boss_encounters_visited: 0,
    }];
    run.phase = RunPhase::Map;
    run.current_room_model = None;
    run.next_room_id = 1;
    run.current_room_id = 0;
    run.current_room_type = None;
    run.previous_room_type = None;
    run.active_room = None;
    run.pushed_room = None;
    run.begun_event = None;
    run.reward_set = None;
    state.run_player.current_hp = 400;
    state.run_player.max_hp = 400;
    Simulator::from_generated_state(
        state,
        sts2_content::registry_at(&id("CHARACTER.IRONCLAD"), 10),
    )
    .expect("the run builds")
}

fn point(col: u8, row: u8, point_type: MapPointType, children: Vec<MapCoord>) -> MapPoint {
    MapPoint {
        coord: MapCoord { col, row },
        point_type,
        children,
        can_modify: true,
    }
}

/// Every level applies at its own number and at every number above it, and a
/// run configuration accepts each of them, which is what makes each of the
/// rules above reachable from a run configuration alone.
#[test]
fn the_level_a_rule_reads_is_the_one_the_run_configuration_carries() {
    for level in [
        AscensionLevel::SwarmingElites,
        AscensionLevel::WearyTraveler,
        AscensionLevel::Poverty,
        AscensionLevel::TightBelt,
        AscensionLevel::AscendersBane,
        AscensionLevel::Inflation,
        AscensionLevel::Scarcity,
        AscensionLevel::ToughEnemies,
        AscensionLevel::DeadlyEnemies,
        AscensionLevel::DoubleBoss,
    ] {
        let config = RunConfig::fixture_baseline(SEED).at_ascension(level as u8);
        config.validate().expect("every level is a run");
        assert!(sts2_engine::has_ascension(config.ascension, level));
        assert!(!sts2_engine::has_ascension(config.ascension - 1, level));
    }
}
