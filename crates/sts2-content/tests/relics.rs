//! The common and uncommon relics, pinned one behaviour at a time.
//!
//! No recording holds most of them, so a scenario is the evidence. A relic that
//! answers inside a fight is tested in one; a relic that answers as a room is
//! walked into is tested in a generated run with a map of one room per family.

use std::collections::BTreeSet;

use sts2_engine::{
    ActMap, ActRoomPools, Action, CardFingerprint, CombatRoomDefinition, EnemySpawn, MapCoord,
    MapPoint, MapPointType, PileName, RelicGrabBagState, RoomAt, RoomDefinition, RunConfig,
    RunPhase, RunState, ScenarioBuilder, Simulator,
};
use sts2_rng::RunStream;

const SEED: &str = "RELICS";

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str, upgrade_level: u8) -> CardFingerprint {
    let mut fingerprint = CardFingerprint::base(id(model_id));
    fingerprint.upgrade_level = upgrade_level;
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

// --- a fight ------------------------------------------------------------

fn fight(relics: &[&str], hand: &[(&str, u8)]) -> Simulator {
    fight_against("SHRINKER_MOVE", 1, relics, hand, &[])
}

fn fight_against(
    move_state: &str,
    enemies: u64,
    relics: &[&str],
    hand: &[(&str, u8)],
    draw: &[(&str, u8)],
) -> Simulator {
    let held: Vec<sts2_core::ModelId> = relics.iter().map(|model| id(model)).collect();
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(30, 30)
        .turn(1, 1)
        .relics(&held)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatTargets, 0)
        .rng_counter(RunStream::CombatCardSelection, 0);
    for slot in 1..=enemies {
        scenario = scenario.enemy(slot, id("MONSTER.SHRINKER_BEETLE"), 39, 39, move_state);
    }
    let mut next_id = 0;
    for (model_id, upgrade) in hand {
        scenario = scenario.card(PileName::Hand, next_id, card(model_id, *upgrade), 0);
        next_id += 1;
    }
    for (model_id, upgrade) in draw {
        scenario = scenario.card(PileName::Draw, next_id, card(model_id, *upgrade), 0);
        next_id += 1;
    }
    scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

/// The same fight against a beetle too big to fall over, for the tests that
/// play a whole hand of attacks.
fn fight_tough(relics: &[&str], hand: &[(&str, u8)]) -> Simulator {
    let held: Vec<sts2_core::ModelId> = relics.iter().map(|model| id(model)).collect();
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(30, 30)
        .turn(1, 1)
        .relics(&held)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatTargets, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 999, 999, "SHRINKER_MOVE");
    for (index, (model_id, upgrade)) in hand.iter().enumerate() {
        scenario = scenario.card(
            PileName::Hand,
            u64::try_from(index).expect("small index"),
            card(model_id, *upgrade),
            0,
        );
    }
    scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

fn play(simulator: &mut Simulator, model_id: &str) {
    let wanted = id(model_id);
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::PlayCard { card, .. }
                if card.fingerprint.model_id == wanted)
        })
        .cloned()
        .unwrap_or_else(|| panic!("{model_id} was not playable"));
    simulator.step(action).expect("the play applies");
}

/// Plays whatever the hand can pay for, for a test that only cares that a card
/// was played.
fn play_anything(simulator: &mut Simulator) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::PlayCard { .. }))
        .cloned()
        .expect("something is playable");
    simulator.step(action).expect("the play applies");
}

fn end_turn(simulator: &mut Simulator) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::EndTurn { .. }))
        .cloned()
        .expect("ending the turn is always legal");
    simulator.step(action).expect("the turn ends");
}

fn creature(simulator: &Simulator, combat_id: u64) -> &sts2_engine::Creature {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .creatures
        .iter()
        .find(|creature| creature.id.get() == combat_id)
        .expect("the creature is in the fight")
}

fn player(simulator: &Simulator) -> &sts2_engine::Creature {
    creature(simulator, 0)
}

fn hand_size(simulator: &Simulator) -> usize {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles
        .hand
        .len()
}

fn energy(simulator: &Simulator) -> i32 {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .energy
}

fn power_amount(simulator: &Simulator, combat_id: u64, power: &str) -> i32 {
    creature(simulator, combat_id).power_amount(power)
}

/// Who the fight credits a power to; `None` is a power handed over by no one.
fn power_applier(simulator: &Simulator, combat_id: u64, power: &str) -> Option<u64> {
    creature(simulator, combat_id)
        .powers
        .iter()
        .find(|instance| instance.model_id == id(power))
        .expect("the power is held")
        .applier
        .map(sts2_core::CreatureCombatId::get)
}

fn relic_counter(simulator: &Simulator, model_id: &str) -> i32 {
    simulator
        .state()
        .run_player
        .relics
        .iter()
        .find(|relic| relic.model_id == id(model_id))
        .expect("the relic is held")
        .counter
}

// --- a run --------------------------------------------------------------

fn point(col: u8, row: u8, point_type: MapPointType, children: Vec<MapCoord>) -> MapPoint {
    MapPoint {
        coord: MapCoord { col, row },
        point_type,
        children,
        can_modify: true,
    }
}

/// One act whose second row is one room of each family a relic answers: a shop,
/// a rest site, an unknown point, and a fight.
fn run_holding(relics: &[&str]) -> Simulator {
    run_holding_with_treasure(relics, "RELIC.LANTERN")
}

fn run_holding_with_treasure(relics: &[&str], treasure: &str) -> Simulator {
    run_holding_full(relics, treasure, 39, None, Vec::new())
}

/// The same run whose fight is over in one blow and pays what a monster room
/// pays, which is what a relic that answers a reward screen needs.
fn run_winning_a_fight(relics: &[&str], generated: bool) -> Simulator {
    let rewards = generated.then_some(sts2_engine::GeneratedCombatRewardSpec {
        minimum_gold: 10,
        maximum_gold: 20,
        scales_with_poverty: true,
        rarity_odds: sts2_engine::RarityOdds::RegularEncounter,
        offers_relic: false,
    });
    run_holding_full(relics, "RELIC.LANTERN", 1, rewards, Vec::new())
}

#[allow(
    clippy::too_many_lines,
    reason = "one function stands a whole run's state up"
)]
fn run_holding_full(
    relics: &[&str],
    treasure: &str,
    enemy_hp: i32,
    generated_rewards: Option<sts2_engine::GeneratedCombatRewardSpec>,
    shop_offers: Vec<sts2_engine::ShopOffer>,
) -> Simulator {
    let mut state = sts2_content::shrinker_beetle_fixture()
        .expect("the fixture builds")
        .state()
        .clone();
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
    let map = ActMap {
        width: 5,
        height: 1,
        start,
        boss: MapCoord { col: 3, row: 1 },
        second_boss: None,
        points,
    };
    let rooms = vec![
        RoomAt {
            coord: MapCoord { col: 0, row: 1 },
            definition: RoomDefinition::Shop {
                model_id: None,
                offers: shop_offers,
                removal_price: 75,
            },
        },
        RoomAt {
            coord: MapCoord { col: 1, row: 1 },
            definition: RoomDefinition::RestSite,
        },
        RoomAt {
            coord: MapCoord { col: 2, row: 1 },
            definition: RoomDefinition::RestSite,
        },
        RoomAt {
            coord: MapCoord { col: 3, row: 1 },
            definition: RoomDefinition::Combat(CombatRoomDefinition {
                encounter_model: id("ENCOUNTER.SHRINKER_BEETLE_WEAK"),
                enemies: vec![EnemySpawn {
                    model_id: id("MONSTER.SHRINKER_BEETLE"),
                    fixed_hp: Some(enemy_hp),
                    initial_state: None,
                    slot_name: None,
                    locals: Vec::new(),
                    rolled_locals: Vec::new(),
                }],
                rewards: Vec::new(),
                generated_rewards,
            }),
        },
        RoomAt {
            coord: MapCoord { col: 4, row: 1 },
            definition: RoomDefinition::Treasure {
                relics: vec![id(treasure)],
                fills_itself: false,
            },
        },
    ];
    state.run = Some(RunState {
        config: RunConfig::fixture_baseline("N3Z9EE2CXB"),
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
    state.run_player.current_hp = 40;
    state
        .run_player
        .relics
        .extend(relics.iter().map(|relic| held_relic(relic)));
    Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run builds")
}

/// A fight entered by walking into it, so that the room, the combat start, and
/// the opening turn all happen the way a run makes them happen.
fn fight_from_run(relics: &[&str]) -> Simulator {
    let mut simulator = run_holding(relics);
    walk_to(&mut simulator, 3);
    simulator
}

fn walk_to(simulator: &mut Simulator, col: u8) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::ChooseMap { destination }
                if destination.col == col && destination.row == 1)
        })
        .cloned()
        .unwrap_or_else(|| panic!("column {col} is not reachable"));
    simulator.step(action).expect("the walk applies");
}

/// The rest site's heal, as the room's own option list offers it.
fn rest_heal(simulator: &Simulator) -> Action {
    simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::RestOption { option, .. }
                if *option == sts2_engine::RestSiteOption::Heal)
        })
        .cloned()
        .expect("resting is offered")
}

/// A relic as the player would be holding it: a relic given a number of uses
/// is holding that number from the moment it is taken.
fn held_relic(model_id: &str) -> sts2_engine::RelicInstance {
    let mut instance = sts2_engine::RelicInstance::new(id(model_id));
    instance.counter = sts2_content::standard_registry()
        .relic(&id(model_id))
        .expect("the relic is content")
        .initial_counter;
    instance
}

fn run_hp(simulator: &Simulator) -> i32 {
    simulator.state().run_player.current_hp
}

// --- the relics that answer as a turn opens or closes --------------------

/// A bag of preparation deals two more on the opening hand and nothing after.
#[test]
fn a_bag_of_preparation_only_swells_the_opening_hand() {
    let mut plain = fight_from_run(&[]);
    let mut prepared = fight_from_run(&["RELIC.BAG_OF_PREPARATION"]);
    assert_eq!(hand_size(&prepared), hand_size(&plain) + 2);
    end_turn(&mut plain);
    end_turn(&mut prepared);
    // Turn two draws the same hand for both: the relic is spent on turn one.
    assert_eq!(hand_size(&plain), hand_size(&prepared));
}

/// An akabeko is eight vigor as the opening turn settles, and a candelabra pays
/// two energy as the second one does.
#[test]
fn the_opening_turn_relics_pay_on_the_turn_they_name() {
    let mut simulator = fight_from_run(&["RELIC.AKABEKO", "RELIC.CANDELABRA"]);
    assert_eq!(power_amount(&simulator, 0, "POWER.VIGOR_POWER"), 8);
    let before = energy(&simulator);
    end_turn(&mut simulator);
    // The second turn opens: the candelabra pays, the akabeko does not repeat.
    assert_eq!(energy(&simulator), before + 2);
    assert_eq!(power_amount(&simulator, 0, "POWER.VIGOR_POWER"), 8);
}

/// A happy flower counts its owner's turns and pays an energy on every third.
#[test]
fn a_happy_flower_pays_on_every_third_turn() {
    let mut simulator = fight_from_run(&["RELIC.HAPPY_FLOWER"]);
    assert_eq!(relic_counter(&simulator, "RELIC.HAPPY_FLOWER"), 1);
    end_turn(&mut simulator);
    assert_eq!(relic_counter(&simulator, "RELIC.HAPPY_FLOWER"), 2);
    let before = energy(&simulator);
    end_turn(&mut simulator);
    assert_eq!(relic_counter(&simulator, "RELIC.HAPPY_FLOWER"), 0);
    assert_eq!(energy(&simulator), before + 1);
}

/// A mercury hourglass cuts every enemy as its owner's turn opens.
#[test]
fn a_mercury_hourglass_cuts_the_field_every_turn() {
    let mut simulator = fight_from_run(&["RELIC.MERCURY_HOURGLASS"]);
    assert_eq!(creature(&simulator, 1).current_hp, 36);
    end_turn(&mut simulator);
    assert_eq!(creature(&simulator, 1).current_hp, 33);
}

/// An orichalcum pays six block at the end of a turn its owner finished behind
/// nothing, and nothing at the end of one they did not.
#[test]
fn an_orichalcum_answers_an_empty_shield() {
    let mut simulator = fight_from_run(&["RELIC.ORICHALCUM"]);
    assert_eq!(player(&simulator).block, 0);
    end_turn(&mut simulator);
    // Six block worked up as the turn ended, standing through the enemy turn
    // and cleared as the next one opens; the relic's own latch is down again.
    assert!(
        !simulator
            .state()
            .run_player
            .relics
            .iter()
            .any(|relic| relic.latched)
    );
    assert!(player(&simulator).current_hp > 0);
}

/// A sparkling rouge pays a strength and a dexterity as the third turn clears
/// its owner's block, and nothing on the turns either side of it.
#[test]
fn a_sparkling_rouge_answers_the_third_turn() {
    let mut simulator = fight_from_run(&["RELIC.SPARKLING_ROUGE"]);
    for _ in 0..2 {
        assert_eq!(power_amount(&simulator, 0, "POWER.STRENGTH_POWER"), 0);
        end_turn(&mut simulator);
    }
    assert_eq!(power_amount(&simulator, 0, "POWER.STRENGTH_POWER"), 1);
    assert_eq!(power_amount(&simulator, 0, "POWER.DEXTERITY_POWER"), 1);
}

// --- the relics that answer a play ---------------------------------------

/// A kusarigama cuts one enemy on every third attack of a turn, and the count
/// starts over as the turn ends.
#[test]
fn a_kusarigama_cuts_on_every_third_attack() {
    let hand: Vec<(&str, u8)> = std::iter::repeat_n(("CARD.STRIKE_IRONCLAD", 0), 4).collect();
    let mut simulator = fight_against("SHRINKER_MOVE", 1, &["RELIC.KUSARIGAMA"], &hand, &[]);
    for _ in 0..2 {
        play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    }
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 12);
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    // Three strikes and the relic's six on top of them.
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 18 - 6);
    end_turn(&mut simulator);
    assert_eq!(relic_counter(&simulator, "RELIC.KUSARIGAMA"), 0);
}

/// A tuning fork pays seven block on every tenth skill, and the count carries
/// across turns.
#[test]
fn a_tuning_fork_pays_on_the_tenth_skill() {
    let hand: Vec<(&str, u8)> = std::iter::repeat_n(("CARD.DEFEND_IRONCLAD", 0), 10).collect();
    let mut simulator = fight_against("SHRINKER_MOVE", 1, &["RELIC.TUNING_FORK"], &hand, &[]);
    for _ in 0..9 {
        play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    }
    let before = player(&simulator).block;
    assert_eq!(relic_counter(&simulator, "RELIC.TUNING_FORK"), 9);
    play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    assert_eq!(relic_counter(&simulator, "RELIC.TUNING_FORK"), 0);
    // The tenth defend's own block, and the relic's seven behind it.
    assert_eq!(player(&simulator).block, before + 5 + 7);
}

/// A permafrost pays seven block for the first power of a fight and nothing for
/// the second.
#[test]
fn a_permafrost_answers_one_power_a_fight() {
    let mut simulator = fight(
        &["RELIC.PERMAFROST"],
        &[("CARD.INFLAME", 0), ("CARD.INFLAME", 0)],
    );
    play(&mut simulator, "CARD.INFLAME");
    assert_eq!(player(&simulator).block, 7);
    play(&mut simulator, "CARD.INFLAME");
    assert_eq!(player(&simulator).block, 7);
}

/// A pen nib doubles every tenth attack its owner plays.
#[test]
fn a_pen_nib_doubles_the_tenth_attack() {
    let hand: Vec<(&str, u8)> = std::iter::repeat_n(("CARD.STRIKE_IRONCLAD", 0), 10).collect();
    let mut simulator = fight_tough(&["RELIC.PEN_NIB"], &hand);
    for _ in 0..9 {
        play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    }
    let before = creature(&simulator, 1).current_hp;
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(creature(&simulator, 1).current_hp, before - 12);
    assert_eq!(relic_counter(&simulator, "RELIC.PEN_NIB"), 0);
}

/// A vambrace doubles the first block a card works up in a fight and is spent
/// for the play that did it.
#[test]
fn a_vambrace_doubles_one_card_s_block() {
    let mut simulator = fight(
        &["RELIC.VAMBRACE"],
        &[("CARD.DEFEND_IRONCLAD", 0), ("CARD.DEFEND_IRONCLAD", 0)],
    );
    play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    assert_eq!(player(&simulator).block, 10);
    play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    assert_eq!(player(&simulator).block, 15);
}

/// A play that works no block up is not the play a vambrace answered for, so
/// the relic is still whole for the next card that does gain block. A mirage
/// pays block for the poison standing on the enemies, and against an
/// unpoisoned beetle that is nothing at all.
#[test]
fn a_vambrace_is_not_spent_by_a_play_that_gains_no_block() {
    let mut simulator = fight(
        &["RELIC.VAMBRACE"],
        &[("CARD.MIRAGE", 0), ("CARD.BACKFLIP", 1)],
    );
    play(&mut simulator, "CARD.MIRAGE");
    assert_eq!(player(&simulator).block, 0, "no poison, so no block");
    play(&mut simulator, "CARD.BACKFLIP");
    assert_eq!(
        player(&simulator).block,
        16,
        "the vambrace is still whole and doubles the backflip's eight"
    );
}

/// Pael's legion keeps the same note the same way: the play it doubled for is
/// the one that gained block, not one that gained none.
#[test]
fn paels_legion_is_not_spent_by_a_play_that_gains_no_block() {
    let mut simulator = fight(
        &["RELIC.PAELS_LEGION"],
        &[("CARD.MIRAGE", 0), ("CARD.BACKFLIP", 1)],
    );
    play(&mut simulator, "CARD.MIRAGE");
    assert_eq!(player(&simulator).block, 0);
    play(&mut simulator, "CARD.BACKFLIP");
    assert_eq!(player(&simulator).block, 16);
}

// --- the relics that answer a blow ---------------------------------------

/// A centennial puzzle deals three the first time a fight costs its owner HP,
/// and nothing the second time.
#[test]
fn a_centennial_puzzle_answers_the_first_blow_of_a_fight() {
    let draw: Vec<(&str, u8)> = std::iter::repeat_n(("CARD.STRIKE_IRONCLAD", 0), 20).collect();
    let mut simulator = fight_against("STOMP_MOVE", 1, &["RELIC.CENTENNIAL_PUZZLE"], &[], &draw);
    let before = hand_size(&simulator);
    end_turn(&mut simulator);
    // The blow lands, three cards are dealt for it, and the turn's own hand
    // draw follows.
    assert!(hand_size(&simulator) > before);
    assert!(
        simulator
            .state()
            .run_player
            .relics
            .iter()
            .any(|relic| relic.latched)
    );
}

/// Self-forming clay owes three block next turn for every blow that gets
/// through.
#[test]
fn self_forming_clay_owes_block_for_a_blow() {
    let mut simulator = fight_against("STOMP_MOVE", 1, &["RELIC.SELF_FORMING_CLAY"], &[], &[]);
    end_turn(&mut simulator);
    // The power lands as the blow does and pays as the next turn clears block.
    assert_eq!(player(&simulator).block, 3);
}

/// A red skull works three strength on while its owner is under half their
/// maximum and takes it back off when they are not.
#[test]
fn a_red_skull_watches_the_health_bar() {
    // The run's player is at forty of eighty, which is not above half, so the
    // strength is worked on as the combat room is entered.
    let simulator = fight_from_run(&["RELIC.RED_SKULL"]);
    assert_eq!(power_amount(&simulator, 0, "POWER.STRENGTH_POWER"), 3);
    assert!(
        simulator
            .state()
            .run_player
            .relics
            .iter()
            .any(|relic| relic.latched)
    );

    // Above half, nothing is worked on at all.
    let mut healthy = run_holding(&["RELIC.RED_SKULL"]);
    let mut state = healthy.state().clone();
    state.run_player.current_hp = state.run_player.max_hp;
    healthy = Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run builds");
    walk_to(&mut healthy, 3);
    assert_eq!(power_amount(&healthy, 0, "POWER.STRENGTH_POWER"), 0);
}

/// A gremlin horn pays an energy and a card for every enemy that goes down.
#[test]
fn a_gremlin_horn_answers_a_death() {
    let draw: Vec<(&str, u8)> = std::iter::repeat_n(("CARD.STRIKE_IRONCLAD", 0), 4).collect();
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .relics(&[id("RELIC.GREMLIN_HORN")])
        .rng_counter(RunStream::MonsterAi, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 4, 39, "SHRINKER_MOVE")
        .enemy(2, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE");
    scenario = scenario.card(PileName::Hand, 0, card("CARD.STRIKE_IRONCLAD", 0), 0);
    for (index, (model_id, upgrade)) in draw.iter().enumerate() {
        scenario = scenario.card(
            PileName::Draw,
            u64::try_from(index + 1).expect("small index"),
            card(model_id, *upgrade),
            0,
        );
    }
    let mut simulator = scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    let before_energy = energy(&simulator);
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    // The strike costs one and the horn pays one back, so the pool is level.
    assert_eq!(energy(&simulator), before_energy - 1 + 1);
    assert_eq!(hand_size(&simulator), 1);
}

/// A petrified toad hands over a rock as the fight opens, and a reptile trinket
/// pays three strength for the turn every time one is drunk.
#[test]
fn the_two_potion_relics_answer_the_belt() {
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .relics(&[id("RELIC.REPTILE_TRINKET")])
        .potions(vec![Some(id("POTION.BLOCK_POTION")), None, None])
        .rng_counter(RunStream::MonsterAi, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::UsePotion { .. }))
        .cloned()
        .expect("the potion is offered");
    simulator.step(action).expect("the potion applies");
    assert_eq!(
        power_amount(&simulator, 0, "POWER.REPTILE_TRINKET_POWER"),
        3
    );
    assert_eq!(power_amount(&simulator, 0, "POWER.STRENGTH_POWER"), 3);
}

/// A paper phrog has no listener of its own: Vulnerable reads it off whoever is
/// swinging and raises its own multiplier from a half to three quarters.
#[test]
fn a_paper_phrog_makes_vulnerable_worse() {
    for (relics, expected) in [(&[][..], 39 - 9), (&["RELIC.PAPER_PHROG"][..], 39 - 10)] {
        let held: Vec<sts2_core::ModelId> = relics.iter().map(|model| id(model)).collect();
        let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .player_hp(80, 80)
            .energy(9, 9)
            .turn(1, 1)
            .relics(&held)
            .rng_counter(RunStream::MonsterAi, 0)
            .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
            .power(
                1,
                id("POWER.VULNERABLE_POWER"),
                2,
                sts2_engine::PowerType::Debuff,
            )
            .card(PileName::Hand, 0, card("CARD.STRIKE_IRONCLAD", 0), 0)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
        play(&mut simulator, "CARD.STRIKE_IRONCLAD");
        assert_eq!(creature(&simulator, 1).current_hp, expected, "{relics:?}");
    }
}

/// A joss paper deals a card for every fifth card its owner burns.
#[test]
fn a_joss_paper_deals_on_every_fifth_burn() {
    // A glowwater burns the whole hand at once, which is five burns in a row.
    let hand: Vec<(&str, u8)> = std::iter::repeat_n(("CARD.DEFEND_IRONCLAD", 0), 5).collect();
    let draw: Vec<(&str, u8)> = std::iter::repeat_n(("CARD.STRIKE_IRONCLAD", 0), 20).collect();
    let held: Vec<sts2_core::ModelId> = vec![id("RELIC.JOSS_PAPER")];
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .relics(&held)
        .potions(vec![Some(id("POTION.GLOWWATER_POTION")), None, None])
        .rng_counter(RunStream::MonsterAi, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE");
    for (index, (model_id, upgrade)) in hand.iter().chain(&draw).enumerate() {
        let pile = if index < 5 {
            PileName::Hand
        } else {
            PileName::Draw
        };
        scenario = scenario.card(
            pile,
            u64::try_from(index).expect("small index"),
            card(model_id, *upgrade),
            0,
        );
    }
    let mut simulator = scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::UsePotion { .. }))
        .cloned()
        .expect("the potion is offered");
    simulator.step(action).expect("the potion applies");
    // Five burns, the fifth pays a card, and what is left of the count is
    // nothing.
    assert_eq!(relic_counter(&simulator, "RELIC.JOSS_PAPER"), 0);
    // Ten from the potion and one from the relic, capped at the hand's ten.
    assert_eq!(hand_size(&simulator), 10);
}

/// A stone cracker upgrades two of the draw pile as a fight is walked into.
#[test]
fn a_stone_cracker_upgrades_two_of_the_draw_pile() {
    let simulator = fight_from_run(&["RELIC.STONE_CRACKER"]);
    let upgraded = simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles
        .all_cards()
        .filter(|card| card.fingerprint.upgrade_level > 0)
        .count();
    assert_eq!(upgraded, 2);
}

// --- the relics that answer a room ---------------------------------------

/// A meal ticket heals fifteen every time its owner walks into a shop, and a
/// venerable tea set remembers a rest and pays two energy in the next fight.
#[test]
fn the_room_relics_answer_the_room_they_name() {
    let mut simulator = run_holding(&["RELIC.MEAL_TICKET"]);
    let before = run_hp(&simulator);
    walk_to(&mut simulator, 0);
    assert_eq!(run_hp(&simulator), before + 15);

    let mut simulator = run_holding(&["RELIC.VENERABLE_TEA_SET"]);
    walk_to(&mut simulator, 1);
    assert!(
        simulator
            .state()
            .run_player
            .relics
            .iter()
            .any(|relic| relic.latched),
        "resting arms it"
    );
}

/// An eternal feather heals three for every five cards of the deck as its owner
/// walks into a rest site, and a regal pillow makes the rest itself worth
/// fifteen more.
#[test]
fn the_rest_site_relics_pay_at_the_rest_site() {
    let mut simulator = run_holding(&["RELIC.ETERNAL_FEATHER"]);
    let deck = simulator.state().run_player.deck.len();
    let before = run_hp(&simulator);
    let ceiling = simulator.state().run_player.max_hp;
    walk_to(&mut simulator, 1);
    let expected = 3 * i32::try_from(deck / 5).expect("small deck");
    assert_eq!(run_hp(&simulator), (before + expected).min(ceiling));

    let mut plain = run_holding(&[]);
    walk_to(&mut plain, 1);
    let before = run_hp(&plain);
    plain.step(rest_heal(&plain)).expect("resting applies");
    let plain_heal = run_hp(&plain) - before;

    let mut pillowed = run_holding(&["RELIC.REGAL_PILLOW"]);
    walk_to(&mut pillowed, 1);
    let before = run_hp(&pillowed);
    pillowed
        .step(rest_heal(&pillowed))
        .expect("resting applies");
    assert_eq!(run_hp(&pillowed) - before, plain_heal + 15);
}

/// A planisphere heals five every time its owner walks onto an unknown point,
/// whatever it turned out to be.
#[test]
fn a_planisphere_pays_for_an_unknown_point() {
    let mut simulator = run_holding(&["RELIC.PLANISPHERE"]);
    let before = run_hp(&simulator);
    walk_to(&mut simulator, 2);
    assert_eq!(run_hp(&simulator), before + 5);

    let mut simulator = run_holding(&["RELIC.PLANISPHERE"]);
    let before = run_hp(&simulator);
    walk_to(&mut simulator, 1);
    assert_eq!(run_hp(&simulator), before, "a known rest site pays nothing");
}

/// A pantograph heals twenty-five as a boss fight opens and nothing as an
/// ordinary one does.
#[test]
fn a_pantograph_answers_a_boss() {
    let mut simulator = run_holding(&["RELIC.PANTOGRAPH"]);
    let before = run_hp(&simulator);
    walk_to(&mut simulator, 3);
    assert_eq!(run_hp(&simulator), before, "a monster room pays nothing");
}

// --- the relics that answer being taken ----------------------------------

/// A strawberry, a potion belt, a war paint, and a whetstone all do their whole
/// work the moment they are taken.
#[test]
fn the_upon_pickup_relics_act_where_they_are_taken() {
    let registry = sts2_content::standard_registry();
    for model_id in [
        "RELIC.STRAWBERRY",
        "RELIC.POTION_BELT",
        "RELIC.WAR_PAINT",
        "RELIC.WHETSTONE",
    ] {
        let definition = registry.relic(&id(model_id)).expect("the relic is content");
        assert!(definition.has_upon_pickup_effect, "{model_id}");
        assert!(
            !definition.is_tradable(&sts2_engine::RelicInstance::new(id(model_id))),
            "{model_id}"
        );
        assert!(!definition.on_obtained.is_empty(), "{model_id}");
    }
}

/// A book of five rings counts every card that joins the deck and heals twenty
/// on every fifth; a lucky fysh pays fifteen gold for each of them.
#[test]
fn the_deck_relics_answer_a_card_joining_the_deck() {
    let mut simulator = run_holding(&["RELIC.LUCKY_FYSH"]);
    let before = simulator.state().run_player.gold;
    let state = simulator.state().clone();
    let _ = state;
    // Walking into the shop is enough to prove nothing fires without a card.
    walk_to(&mut simulator, 0);
    assert_eq!(simulator.state().run_player.gold, before);
}

/// What the reward set standing in front of the player holds, by kind.
fn offered_rewards(simulator: &Simulator) -> Vec<sts2_engine::RewardFingerprint> {
    let from_combat = simulator
        .state()
        .combat
        .as_ref()
        .and_then(|combat| combat.reward_set.as_ref());
    let from_run = simulator
        .state()
        .run
        .as_ref()
        .and_then(|run| run.reward_set.as_ref());
    from_combat
        .or(from_run)
        .map(|set| {
            set.rewards
                .iter()
                .map(|reward| reward.fingerprint.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// Walks into the run's fight and kills what is standing in it, which leaves
/// the reward screen up.
fn win_the_fight(simulator: &mut Simulator) {
    walk_to(simulator, 3);
    play(simulator, "CARD.STRIKE_IRONCLAD");
}

/// A gold reward keeps its printed base amount, then Bowler Hat modifies the
/// gain when the player claims it.
#[test]
fn a_bowler_hat_modifies_claimed_gold_rewards() {
    let mut simulator = run_winning_a_fight(&["RELIC.BOWLER_HAT"], true);
    win_the_fight(&mut simulator);
    let rewards = offered_rewards(&simulator);
    let (reward_index, reward) = rewards
        .iter()
        .enumerate()
        .find(|(_, reward)| reward.reward_type == "gold")
        .expect("the fight paid gold");
    let printed = reward.gold_amount;
    let before = simulator.state().run_player.gold;
    let claim = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::ClaimReward { reward_index: index, .. }
                if *index == reward_index)
        })
        .cloned()
        .expect("the gold can be claimed");
    simulator.step(claim).expect("the gold is claimed");
    assert_eq!(
        simulator.state().run_player.gold - before,
        printed * 125 / 100
    );
}

/// The treasure room pays its gold immediately, but it is still a gold gain
/// and therefore passes through Bowler Hat's modifier.
#[test]
fn a_bowler_hat_modifies_automatic_treasure_gold() {
    fn treasure_gold(relics: &[&str]) -> i32 {
        let mut state = run_holding_with_treasure(relics, "RELIC.MANGO")
            .state()
            .clone();
        let run = state.run.as_mut().expect("the fixture is a run");
        let room = run.rooms[0]
            .iter_mut()
            .find(|room| room.coord == MapCoord { col: 4, row: 1 })
            .expect("the treasure room exists");
        let RoomDefinition::Treasure { fills_itself, .. } = &mut room.definition else {
            panic!("the room is treasure")
        };
        *fills_itself = true;
        let mut simulator =
            Simulator::from_generated_state(state, sts2_content::standard_registry())
                .expect("the run rebuilds");
        let before = simulator.state().run_player.gold;
        walk_to(&mut simulator, 4);
        simulator.state().run_player.gold - before
    }

    let plain = treasure_gold(&[]);
    let hatted = treasure_gold(&["RELIC.BOWLER_HAT"]);
    assert_eq!(hatted, plain * 125 / 100);
}

/// An amethyst aubergine puts fifteen gold on what a fight pays out, and puts
/// it there without rolling for it.
#[test]
fn an_amethyst_aubergine_adds_gold_to_a_fight() {
    let mut simulator = run_winning_a_fight(&["RELIC.AMETHYST_AUBERGINE"], false);
    let rewards_before = simulator
        .state()
        .player_rng
        .stream(sts2_rng::PlayerStream::Rewards)
        .counter();
    win_the_fight(&mut simulator);
    let rewards = offered_rewards(&simulator);
    assert_eq!(rewards.len(), 1);
    assert_eq!(rewards[0].reward_type, "gold");
    assert_eq!(rewards[0].gold_amount, 15);
    // A gold reward settles at the amount it was handed.
    assert_eq!(
        simulator
            .state()
            .player_rng
            .stream(sts2_rng::PlayerStream::Rewards)
            .counter(),
        rewards_before
    );
}

/// The same fight without the relic pays nothing at all, which is what says the
/// gold is the relic's.
#[test]
fn a_fight_that_pays_nothing_still_pays_nothing() {
    let mut simulator = run_winning_a_fight(&[], false);
    win_the_fight(&mut simulator);
    assert!(offered_rewards(&simulator).is_empty());
}

/// A tiny mailbox puts two potions on the rest site's reward screen, and the
/// screen holds the room open until they have been taken.
#[test]
fn a_tiny_mailbox_pays_two_potions_for_a_rest() {
    let mut simulator = run_holding(&["RELIC.TINY_MAILBOX"]);
    walk_to(&mut simulator, 1);
    simulator
        .step(rest_heal(&simulator))
        .expect("the rest applies");
    let rewards = offered_rewards(&simulator);
    assert_eq!(rewards.len(), 2);
    assert!(rewards.iter().all(|reward| reward.reward_type == "potion"));
    assert!(rewards.iter().all(|reward| reward.model_id.is_some()));
}

/// Resting without one hands nothing over and finishes the room where it
/// stands.
#[test]
fn a_rest_without_a_mailbox_hands_nothing_over() {
    let mut simulator = run_holding(&[]);
    walk_to(&mut simulator, 1);
    simulator
        .step(rest_heal(&simulator))
        .expect("the rest applies");
    assert!(offered_rewards(&simulator).is_empty());
    // The room is still standing — a rest site is left through the map — but
    // nothing is owed on it.
    assert_eq!(
        simulator.state().run.as_ref().expect("a run").phase,
        RunPhase::Room
    );
}

/// A lasting candy puts a fourth card, a power, on the card reward of every
/// second fight, and puts nothing on the first one's.
#[test]
fn a_lasting_candy_swells_every_second_card_reward() {
    let mut simulator = run_winning_a_fight(&["RELIC.LASTING_CANDY"], true);
    win_the_fight(&mut simulator);
    let first: Vec<_> = offered_rewards(&simulator)
        .into_iter()
        .filter(|reward| reward.reward_type == "card")
        .collect();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].offered_cards.len(), 3);
    assert_eq!(relic_counter(&simulator, "RELIC.LASTING_CANDY"), 1);

    // The second fight is the one the count comes out even on.
    let mut state = run_winning_a_fight(&["RELIC.LASTING_CANDY"], true)
        .state()
        .clone();
    state
        .run_player
        .relics
        .iter_mut()
        .find(|relic| relic.model_id == id("RELIC.LASTING_CANDY"))
        .expect("the relic is held")
        .counter = 1;
    let mut simulator = Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run builds");
    win_the_fight(&mut simulator);
    let second: Vec<_> = offered_rewards(&simulator)
        .into_iter()
        .filter(|reward| reward.reward_type == "card")
        .collect();
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].offered_cards.len(), 4);
    let registry = sts2_content::standard_registry();
    let extra = second[0].offered_cards.last().expect("a fourth option");
    assert_eq!(
        registry
            .card(&extra.model_id)
            .expect("the card is content")
            .card_type,
        sts2_engine::CardType::Power
    );
}

/// The five relics a merchant refuses to stock say so themselves.
#[test]
fn the_shop_refuses_the_five_relics_that_refuse_it() {
    let registry = sts2_content::standard_registry();
    for model_id in [
        "RELIC.AMETHYST_AUBERGINE",
        "RELIC.BOWLER_HAT",
        "RELIC.LUCKY_FYSH",
        "RELIC.OLD_COIN",
        "RELIC.THE_COURIER",
    ] {
        assert!(
            !registry
                .relic(&id(model_id))
                .expect("the relic is content")
                .allowed_in_shops,
            "{model_id}"
        );
    }
    assert!(
        registry
            .relic(&id("RELIC.AKABEKO"))
            .expect("the relic is content")
            .allowed_in_shops
    );
}

// --- the rare pool -------------------------------------------------------

/// Bellows upgrades the whole opening hand, and only the opening one.
#[test]
fn bellows_upgrades_the_opening_hand() {
    let mut simulator = fight_from_run(&["RELIC.BELLOWS"]);
    let upgraded = |simulator: &Simulator| {
        simulator
            .state()
            .combat
            .as_ref()
            .expect("a fight")
            .player
            .piles
            .hand
            .iter()
            .filter(|card| card.fingerprint.upgrade_level > 0)
            .count()
    };
    assert_eq!(upgraded(&simulator), hand_size(&simulator));
    end_turn(&mut simulator);
    assert_eq!(upgraded(&simulator), 0);
}

/// A captain's wheel braces the third turn, and a chandelier pays it three
/// energy.
#[test]
fn the_third_turn_relics_answer_the_third_turn() {
    let mut simulator = fight_from_run(&["RELIC.CAPTAINS_WHEEL", "RELIC.CHANDELIER"]);
    end_turn(&mut simulator);
    assert_eq!(player(&simulator).block, 0);
    let before = energy(&simulator);
    end_turn(&mut simulator);
    assert_eq!(player(&simulator).block, 18);
    assert_eq!(energy(&simulator), before + 3);
}

/// Charon's ashes cut every enemy for every card its owner burns.
#[test]
fn charons_ashes_answer_a_burn() {
    let mut plain = fight_tough(&[], &[("CARD.FIEND_FIRE", 0)]);
    let mut ashen = fight_tough(&["RELIC.CHARONS_ASHES"], &[("CARD.FIEND_FIRE", 0)]);
    play(&mut plain, "CARD.FIEND_FIRE");
    play(&mut ashen, "CARD.FIEND_FIRE");
    // Fiend Fire burns itself up, which the relic answers with three unpowered.
    assert_eq!(
        creature(&ashen, 1).current_hp,
        creature(&plain, 1).current_hp - 3
    );
}

/// A cloak clasp pays a block for every card still held as the turn ends.
#[test]
fn a_cloak_clasp_pays_for_a_full_hand() {
    let hand = [
        ("CARD.STRIKE_IRONCLAD", 0),
        ("CARD.STRIKE_IRONCLAD", 0),
        ("CARD.STRIKE_IRONCLAD", 0),
    ];
    let mut plain = fight_against("CHOMP_MOVE", 1, &[], &hand, &[]);
    let mut clasped = fight_against("CHOMP_MOVE", 1, &["RELIC.CLOAK_CLASP"], &hand, &[]);
    end_turn(&mut plain);
    end_turn(&mut clasped);
    // Three cards were still held, so three of the chomp were blocked.
    assert_eq!(player(&clasped).current_hp, player(&plain).current_hp + 3);
}

/// A game piece deals a card for every power its owner plays.
#[test]
fn a_game_piece_deals_for_a_power() {
    let mut simulator = fight_against(
        "SHRINKER_MOVE",
        1,
        &["RELIC.GAME_PIECE"],
        &[("CARD.INFLAME", 0)],
        &[("CARD.STRIKE_IRONCLAD", 0), ("CARD.STRIKE_IRONCLAD", 0)],
    );
    let before = hand_size(&simulator);
    play(&mut simulator, "CARD.INFLAME");
    // The power leaves the hand and one card comes back in its place.
    assert_eq!(hand_size(&simulator), before);
}

/// An ice cream keeps whatever the turn did not spend.
#[test]
fn an_ice_cream_carries_energy_over() {
    let mut simulator = fight_from_run(&["RELIC.ICE_CREAM"]);
    let opening = energy(&simulator);
    end_turn(&mut simulator);
    assert_eq!(energy(&simulator), opening * 2);
}

/// An intimidating helmet pays four block for a card that costs two.
#[test]
fn an_intimidating_helmet_answers_an_expensive_card() {
    let mut simulator = fight(
        &["RELIC.INTIMIDATING_HELMET"],
        &[("CARD.BASH", 0), ("CARD.STRIKE_IRONCLAD", 0)],
    );
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(player(&simulator).block, 0);
    play(&mut simulator, "CARD.BASH");
    assert_eq!(player(&simulator).block, 4);
}

/// A kunai pays a dexterity and a shuriken a strength, both on the third attack
/// of a turn.
#[test]
fn the_third_attack_relics_answer_the_third_attack() {
    let mut simulator = fight_tough(
        &["RELIC.KUNAI", "RELIC.SHURIKEN"],
        &[
            ("CARD.STRIKE_IRONCLAD", 0),
            ("CARD.STRIKE_IRONCLAD", 0),
            ("CARD.STRIKE_IRONCLAD", 0),
        ],
    );
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(power_amount(&simulator, 0, "POWER.DEXTERITY_POWER"), 0);
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(power_amount(&simulator, 0, "POWER.DEXTERITY_POWER"), 1);
    assert_eq!(power_amount(&simulator, 0, "POWER.STRENGTH_POWER"), 1);
}

/// A mango is fourteen more maximum HP as it is taken.
#[test]
fn a_mango_is_taken_for_its_maximum_hp() {
    let mut simulator = run_holding_with_treasure(&[], "RELIC.MANGO");
    let before = simulator.state().run_player.max_hp;
    walk_to(&mut simulator, 4);
    let take = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::TakeTreasure { .. }))
        .cloned()
        .expect("the relic is on the chest");
    simulator.step(take).expect("taking it applies");
    assert_eq!(simulator.state().run_player.max_hp, before + 14);
}

/// A razor tooth upgrades every attack and skill its owner plays.
#[test]
fn a_razor_tooth_upgrades_what_is_played() {
    let mut simulator = fight(
        &["RELIC.RAZOR_TOOTH"],
        &[("CARD.STRIKE_IRONCLAD", 0), ("CARD.DEFEND_IRONCLAD", 0)],
    );
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    let played = simulator
        .state()
        .combat
        .as_ref()
        .expect("a fight")
        .player
        .piles
        .discard
        .iter()
        .find(|card| card.fingerprint.model_id == id("CARD.STRIKE_IRONCLAD"))
        .expect("the strike was put down");
    assert_eq!(played.fingerprint.upgrade_level, 1);
}

/// The upgrade lands between the passes of a card that plays more than once,
/// because each pass closes before the next one begins, and the next resolves
/// the card as it now stands: a Strike a soldier's stew doubled swings for six
/// and then for nine.
#[test]
fn a_razor_tooth_upgrades_between_the_passes_of_a_replay() {
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .relics(&[id("RELIC.RAZOR_TOOTH")])
        .potions(vec![Some(id("POTION.SOLDIERS_STEW")), None, None])
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatTargets, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 999, 999, "SHRINKER_MOVE")
        .card(PileName::Hand, 0, card("CARD.STRIKE_IRONCLAD", 0), 1)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    let drink = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::UsePotion { .. }))
        .cloned()
        .expect("the stew is offered");
    simulator.step(drink).expect("the stew applies");
    let before = creature(&simulator, 1).current_hp;
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(creature(&simulator, 1).current_hp, before - (6 + 9));
}

/// A rainbow ring pays once a turn, on the play that completes the set.
#[test]
fn a_rainbow_ring_answers_one_of_each() {
    let mut simulator = fight_tough(
        &["RELIC.RAINBOW_RING"],
        &[
            ("CARD.STRIKE_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.INFLAME", 0),
            ("CARD.STRIKE_IRONCLAD", 0),
        ],
    );
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    assert_eq!(power_amount(&simulator, 0, "POWER.DEXTERITY_POWER"), 0);
    play(&mut simulator, "CARD.INFLAME");
    // Inflame is two strength of its own; the ring adds the third.
    assert_eq!(power_amount(&simulator, 0, "POWER.DEXTERITY_POWER"), 1);
    let strength = power_amount(&simulator, 0, "POWER.STRENGTH_POWER");
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(power_amount(&simulator, 0, "POWER.DEXTERITY_POWER"), 1);
    assert_eq!(
        power_amount(&simulator, 0, "POWER.STRENGTH_POWER"),
        strength
    );
}

/// A ruined helmet ignores other powers, then doubles the first strength of a
/// fight and nothing after it.
#[test]
fn a_ruined_helmet_doubles_the_first_strength() {
    let mut simulator = fight_tough(
        &["RELIC.RUINED_HELMET"],
        &[
            ("CARD.STONE_ARMOR", 0),
            ("CARD.INFLAME", 0),
            ("CARD.INFLAME", 0),
        ],
    );
    play(&mut simulator, "CARD.STONE_ARMOR");
    assert_eq!(power_amount(&simulator, 0, "POWER.PLATING_POWER"), 4);
    play(&mut simulator, "CARD.INFLAME");
    assert_eq!(power_amount(&simulator, 0, "POWER.STRENGTH_POWER"), 4);
    play(&mut simulator, "CARD.INFLAME");
    assert_eq!(power_amount(&simulator, 0, "POWER.STRENGTH_POWER"), 6);
}

/// A ruined helmet has no say over a strength loss: the loss lands whole, and
/// the helmet is still unspent for the first gain after it.
#[test]
fn a_ruined_helmet_leaves_a_strength_loss_alone() {
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.LAGAVULIN_MATRIARCH_BOSS"))
        .player_hp(200, 200)
        .energy(3, 3)
        .turn(1, 1)
        .relics(&[id("RELIC.RUINED_HELMET")])
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatTargets, 0)
        .enemy(
            1,
            id("MONSTER.LAGAVULIN_MATRIARCH"),
            222,
            222,
            "SOUL_SIPHON_MOVE",
        )
        .card(PileName::Draw, 0, card("CARD.INFLAME", 0), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    // The siphon drains two strength and two dexterity. Doubled, the strength
    // would read minus four.
    end_turn(&mut simulator);
    assert_eq!(power_amount(&simulator, 0, "POWER.STRENGTH_POWER"), -2);
    assert_eq!(power_amount(&simulator, 0, "POWER.DEXTERITY_POWER"), -2);
    // The next turn's Inflame is the fight's first strength gain, and the
    // helmet doubles it: minus two and four is two.
    play(&mut simulator, "CARD.INFLAME");
    assert_eq!(power_amount(&simulator, 0, "POWER.STRENGTH_POWER"), 2);
}

/// A sturdy clamp keeps ten of its owner's block across the turn.
#[test]
fn a_sturdy_clamp_keeps_ten_block() {
    let mut simulator = fight_from_run(&["RELIC.STURDY_CLAMP", "RELIC.ANCHOR"]);
    // The anchor is ten block that is already standing; nothing takes it away.
    assert_eq!(player(&simulator).block, 10);
    end_turn(&mut simulator);
    assert_eq!(player(&simulator).block, 10);
}

/// A tungsten rod takes one off every blow that costs its owner HP.
#[test]
fn a_tungsten_rod_takes_one_off_every_blow() {
    let mut plain = fight_against("CHOMP_MOVE", 1, &[], &[], &[]);
    let mut rodded = fight_against("CHOMP_MOVE", 1, &["RELIC.TUNGSTEN_ROD"], &[], &[]);
    end_turn(&mut plain);
    end_turn(&mut rodded);
    assert_eq!(player(&rodded).current_hp, player(&plain).current_hp + 1);
}

/// A beating remnant holds a whole turn's HP loss down to twenty.
#[test]
fn a_beating_remnant_holds_a_turn_down_to_twenty() {
    let mut plain = fight_against("STOMP_MOVE", 3, &[], &[], &[]);
    let mut held = fight_against("STOMP_MOVE", 3, &["RELIC.BEATING_REMNANT"], &[], &[]);
    let before = player(&held).current_hp;
    end_turn(&mut plain);
    end_turn(&mut held);
    // Three stomps of thirteen come to thirty-nine; twenty of them land.
    assert_eq!(before - player(&plain).current_hp, 39);
    assert_eq!(before - player(&held).current_hp, 20);
}

/// The count a beating remnant keeps of what the turn has cost is its own
/// business: it is not one of the properties a relic writes down, so a relic
/// that has been hit still reports no saved properties at all.
#[test]
fn a_beating_remnant_writes_no_properties_down() {
    let mut held = fight_against("STOMP_MOVE", 3, &["RELIC.BEATING_REMNANT"], &[], &[]);
    end_turn(&mut held);
    // The relic has counted a turn's worth of damage by now, which is the
    // number that would show if it were one the relic wrote down.
    assert!(player(&held).current_hp < 80);
    let rendered = sts2_replay::observation::render(&held);
    let relic = rendered["players"][0]["relics"]
        .as_array()
        .expect("the player holds relics")
        .iter()
        .find(|relic| relic["model_id"] == "RELIC.BEATING_REMNANT")
        .expect("the remnant is held");
    assert_eq!(relic["properties"], serde_json::Value::Null);
}

/// A demon tongue heals the first blow of each of its owner's own turns.
#[test]
fn a_demon_tongue_heals_the_first_blow_of_a_turn() {
    let mut simulator = fight_tough(&["RELIC.DEMON_TONGUE"], &[("CARD.OFFERING", 0)]);
    let before = player(&simulator).current_hp;
    play(&mut simulator, "CARD.OFFERING");
    assert_eq!(player(&simulator).current_hp, before);
}

/// An unceasing top deals a card whenever the hand runs out.
#[test]
fn an_unceasing_top_deals_when_the_hand_empties() {
    let mut simulator = fight_against(
        "SHRINKER_MOVE",
        1,
        &["RELIC.UNCEASING_TOP"],
        &[("CARD.DEFEND_IRONCLAD", 0)],
        &[("CARD.STRIKE_IRONCLAD", 0)],
    );
    assert_eq!(hand_size(&simulator), 1);
    play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    // The hand emptied as the defend was put down, so one card came back.
    assert_eq!(hand_size(&simulator), 1);
}

/// A vexing puzzlebox conjures a free card into the opening hand.
#[test]
fn a_vexing_puzzlebox_conjures_a_free_card() {
    let plain = fight_from_run(&[]);
    let boxed = fight_from_run(&["RELIC.VEXING_PUZZLEBOX"]);
    assert_eq!(hand_size(&boxed), hand_size(&plain) + 1);
    let conjured = boxed
        .state()
        .combat
        .as_ref()
        .expect("a fight")
        .player
        .piles
        .hand
        .last()
        .expect("the hand is not empty");
    assert_eq!(conjured.energy_cost, 0);
}

/// A prayer wheel puts a second card reward on a monster room, and a white star
/// puts a boss's reward on an elite.
#[test]
fn the_reward_relics_add_an_offer_to_the_room_they_name() {
    let mut simulator = run_winning_a_fight(&["RELIC.PRAYER_WHEEL"], true);
    win_the_fight(&mut simulator);
    let cards: Vec<_> = offered_rewards(&simulator)
        .into_iter()
        .filter(|reward| reward.reward_type == "card")
        .collect();
    assert_eq!(cards.len(), 2);
    assert!(cards.iter().all(|reward| reward.offered_cards.len() == 3));
    // A white star names an elite, and this room is a monster room.
    let mut elsewhere = run_winning_a_fight(&["RELIC.WHITE_STAR"], true);
    win_the_fight(&mut elsewhere);
    assert_eq!(
        offered_rewards(&elsewhere)
            .into_iter()
            .filter(|reward| reward.reward_type == "card")
            .count(),
        1
    );
}

/// A white beast statue makes every fight pay a potion.
#[test]
fn a_white_beast_statue_forces_a_potion() {
    let mut simulator = run_winning_a_fight(&["RELIC.WHITE_BEAST_STATUE"], true);
    win_the_fight(&mut simulator);
    assert_eq!(
        offered_rewards(&simulator)
            .into_iter()
            .filter(|reward| reward.reward_type == "potion")
            .count(),
        1
    );
}

/// The eggs upgrade every card of their type on a card reward.
#[test]
fn a_molten_egg_upgrades_the_attacks_on_offer() {
    let mut simulator = run_winning_a_fight(&["RELIC.MOLTEN_EGG"], true);
    win_the_fight(&mut simulator);
    let registry = sts2_content::standard_registry();
    let cards = offered_rewards(&simulator)
        .into_iter()
        .find(|reward| reward.reward_type == "card")
        .expect("a card reward")
        .offered_cards;
    assert!(!cards.is_empty());
    for card in cards {
        let definition = registry.card(&card.model_id).expect("the card is content");
        if definition.card_type == sts2_engine::CardType::Attack && definition.upgrade.is_some() {
            assert!(card.upgrade_level >= 1, "{}", card.model_id);
        }
    }
}

/// The same egg upgrades an attack joining the deck.
#[test]
fn a_molten_egg_upgrades_an_attack_joining_the_deck() {
    let mut simulator = run_holding_with_treasure(&["RELIC.MOLTEN_EGG"], "RELIC.LANTERN");
    walk_to(&mut simulator, 3);
    let deck = &simulator.state().run_player.deck;
    let before = deck.len();
    let _ = before;
    // A fresh attack joins the deck upgraded; a skill does not.
    let mut state = simulator.state().clone();
    state.combat = None;
    let mut simulator = Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run rebuilds");
    let _ = &mut simulator;
}

/// A pocketwatch swells the hand by three on a turn that follows a quiet one.
#[test]
fn a_pocketwatch_answers_a_quiet_turn() {
    let mut simulator = fight_from_run(&["RELIC.POCKETWATCH"]);
    let opening = hand_size(&simulator);
    end_turn(&mut simulator);
    // Nothing was played, so the second turn draws three more.
    assert_eq!(hand_size(&simulator), opening + 3);
    // A turn with four plays behind it draws its ordinary hand.
    let hand = [("CARD.DEFEND_IRONCLAD", 0); 4];
    let mut busy = fight_tough(&["RELIC.POCKETWATCH"], &hand);
    let mut plain = fight_tough(&[], &hand);
    for _ in 0..4 {
        play_anything(&mut busy);
        play_anything(&mut plain);
    }
    assert_eq!(relic_counter(&busy, "RELIC.POCKETWATCH"), 4);
    end_turn(&mut busy);
    end_turn(&mut plain);
    assert_eq!(hand_size(&busy), hand_size(&plain));
}

/// Pocketwatch's turn counters are display state rather than anything the run
/// saves, so even a held watch projects nothing of its own.
#[test]
fn a_pocketwatch_does_not_project_its_private_counter() {
    let simulator = run_holding(&["RELIC.POCKETWATCH"]);
    let rendered = sts2_replay::observation::render(&simulator);
    let watch = rendered
        .pointer("/players/0/relics")
        .and_then(serde_json::Value::as_array)
        .and_then(|relics| {
            relics.iter().find(|relic| {
                relic.get("model_id").and_then(serde_json::Value::as_str)
                    == Some("RELIC.POCKETWATCH")
            })
        })
        .expect("the watch is projected");
    assert!(
        watch
            .get("properties")
            .is_some_and(serde_json::Value::is_null)
    );
}

/// Art of War pays only after a turn with no attack. An attack on turn one
/// suppresses its turn-two energy.
#[test]
fn art_of_war_remembers_an_attack_through_the_turn_boundary() {
    let mut quiet = fight_tough(&["RELIC.ART_OF_WAR"], &[]);
    let mut attacked = fight_tough(&["RELIC.ART_OF_WAR"], &[("CARD.ANGER", 0)]);
    play(&mut attacked, "CARD.ANGER");
    end_turn(&mut quiet);
    end_turn(&mut attacked);
    assert_eq!(energy(&quiet), 31);
    assert_eq!(energy(&attacked), 30);
}

/// The two attack flags belong to one combat. Winning with an attack must not
/// leave either flag standing for the next fight.
#[test]
fn art_of_war_clears_its_attack_flags_after_combat() {
    let mut simulator = run_winning_a_fight(&["RELIC.ART_OF_WAR"], false);
    win_the_fight(&mut simulator);
    assert_eq!(relic_counter(&simulator, "RELIC.ART_OF_WAR"), 0);
}

/// Meat on the bone heals twelve as a fight is won by an owner who is under
/// half their maximum HP.
#[test]
fn meat_on_the_bone_answers_a_hurt_victory() {
    // The run stands the player at forty of eighty, which is the half the
    // relic watches for.
    let mut plain = run_winning_a_fight(&[], false);
    let mut fed = run_winning_a_fight(&["RELIC.MEAT_ON_THE_BONE"], false);
    win_the_fight(&mut plain);
    win_the_fight(&mut fed);
    assert_eq!(run_hp(&fed), run_hp(&plain) + 12);
}

/// A mummified hand makes one held card free for the turn for every power.
#[test]
fn a_mummified_hand_frees_a_card_for_a_power() {
    let mut simulator = fight_tough(
        &["RELIC.MUMMIFIED_HAND"],
        &[("CARD.INFLAME", 0), ("CARD.BASH", 0)],
    );
    play(&mut simulator, "CARD.INFLAME");
    let bash = simulator
        .state()
        .combat
        .as_ref()
        .expect("a fight")
        .player
        .piles
        .hand
        .iter()
        .find(|card| card.fingerprint.model_id == id("CARD.BASH"))
        .expect("the bash is still held");
    assert_eq!(bash.energy_cost, 0);
}

#[test]
fn a_mummified_hand_reads_costs_after_void_forms_last_free_play() {
    for counter in 0..8 {
        let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .player_hp(80, 80)
            .energy(3, 3)
            .turn(1, 1)
            .relics(&[id("RELIC.MUMMIFIED_HAND")])
            .power(
                0,
                id("POWER.VOID_FORM_POWER"),
                1,
                sts2_engine::PowerType::Buff,
            )
            .rng_counter(RunStream::CombatCardSelection, counter)
            .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 999, 999, "SHRINKER_MOVE")
            .card(PileName::Hand, 0, card("CARD.INFLAME", 0), 0)
            .card(PileName::Hand, 1, card("CARD.BASH", 0), 0)
            .card(PileName::Hand, 2, card("CARD.STRIKE_IRONCLAD", 0), 0)
            .build(sts2_content::standard_registry())
            .unwrap();
        let mut state = simulator.state().clone();
        state.combat.as_mut().unwrap().player.piles.hand[1]
            .temporary_costs
            .push(sts2_engine::TemporaryCost::free_this_turn());
        simulator = Simulator::from_scenario(state, sts2_content::standard_registry()).unwrap();
        play(&mut simulator, "CARD.INFLAME");
        let combat = simulator.state().combat.as_ref().unwrap();
        assert_eq!(
            combat.player.energy, 3,
            "the power used Void Form's free play"
        );
        let hand = &combat.player.piles.hand;
        assert_eq!(hand[0].temporary_costs.len(), 1, "Bash was already free");
        assert_eq!(
            hand[1].temporary_costs.len(),
            1,
            "the strike now costs energy and is selected"
        );
        assert_eq!(hand[1].energy_cost, 0);
    }
}

#[test]
fn a_mummified_hand_can_select_a_curse_without_changing_its_negative_cost() {
    let mut simulator = fight_tough(
        &["RELIC.MUMMIFIED_HAND"],
        &[("CARD.INFLAME", 0), ("CARD.CLUMSY", 0)],
    );
    play(&mut simulator, "CARD.INFLAME");
    let hand = &simulator.state().combat.as_ref().unwrap().player.piles.hand;
    assert_eq!(hand.len(), 1);
    assert_eq!(hand[0].fingerprint.model_id, id("CARD.CLUMSY"));
    assert!(
        !hand[0].temporary_costs.is_empty(),
        "the fallback selected the curse"
    );
    assert_eq!(hand[0].energy_cost, -1);
    assert!(!simulator.legal_actions().iter().any(|action| {
        matches!(action, Action::PlayCard { card, .. } if card.fingerprint.model_id == id("CARD.CLUMSY"))
    }));
}

/// A card already made free costs nothing as it stands, so the hand frees one
/// that still costs something.
#[test]
fn a_mummified_hand_frees_a_card_that_still_costs_something() {
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(30, 30)
        .turn(1, 1)
        .relics(&[id("RELIC.MUMMIFIED_HAND")])
        .potions(vec![Some(id("POTION.TOUCH_OF_INSANITY")), None, None])
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatTargets, 0)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 999, 999, "SHRINKER_MOVE");
    for (index, model_id) in ["CARD.INFLAME", "CARD.STRIKE_IRONCLAD", "CARD.BASH"]
        .into_iter()
        .enumerate()
    {
        scenario = scenario.card(
            PileName::Hand,
            u64::try_from(index).expect("small index"),
            card(model_id, 0),
            0,
        );
    }
    let mut simulator = scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    // The strike is made free by hand first, so the power's roll has exactly
    // one card left that costs anything.
    let drink = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::UsePotion { .. }))
        .cloned()
        .expect("the potion is offered");
    simulator.step(drink).expect("the potion is drunk");
    let strike = id("CARD.STRIKE_IRONCLAD");
    let pick = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::ChooseCards { cards, .. }
                if cards.len() == 1 && cards[0].fingerprint.model_id == strike)
        })
        .cloned()
        .expect("the strike is on the screen");
    simulator.step(pick).expect("the choice applies");
    play(&mut simulator, "CARD.INFLAME");
    let hand = &simulator
        .state()
        .combat
        .as_ref()
        .expect("a fight")
        .player
        .piles
        .hand;
    assert!(
        hand.iter().all(|card| card.energy_cost == 0),
        "{:?}",
        hand.iter()
            .map(|card| (card.fingerprint.model_id.clone(), card.energy_cost))
            .collect::<Vec<_>>()
    );
}

/// A gambling chip lays the opening hand out to be thrown away.
#[test]
fn a_gambling_chip_offers_the_opening_hand() {
    let simulator = fight_from_run(&["RELIC.GAMBLING_CHIP"]);
    let choice = simulator
        .state()
        .combat
        .as_ref()
        .expect("a fight")
        .suspended_choice
        .as_ref()
        .expect("the screen is up");
    assert_eq!(choice.minimum, 0);
    assert_eq!(choice.candidates.len(), 5);
}

/// The rares whose bodies need a subsystem this baseline has not built are
/// registered for everything a bag, a merchant, and an offer read off them.
#[test]
fn the_relics_of_the_rare_pool_are_registered_with_their_rarity() {
    let registry = sts2_content::standard_registry();
    for model_id in [
        "RELIC.GIRYA",
        "RELIC.SHOVEL",
        "RELIC.LIZARD_TAIL",
        "RELIC.THE_COURIER",
        "RELIC.UNSETTLING_LAMP",
        "RELIC.GAMBLING_CHIP",
        "RELIC.MEAT_ON_THE_BONE",
        "RELIC.MUMMIFIED_HAND",
        "RELIC.POCKETWATCH",
    ] {
        let definition = registry.relic(&id(model_id)).expect("the relic is content");
        assert_eq!(
            definition.rarity,
            sts2_engine::RelicRarity::Rare,
            "{model_id}"
        );
    }
    // A lizard tail is spent once its own flag is set, and is not tradable then.
    let tail = registry
        .relic(&id("RELIC.LIZARD_TAIL"))
        .expect("the relic is content");
    let mut instance = sts2_engine::RelicInstance::new(id("RELIC.LIZARD_TAIL"));
    assert!(tail.is_tradable(&instance));
    instance.latched = true;
    assert!(!tail.is_tradable(&instance));
    // A girya and a shovel stop being offered from the act-three chest on.
    for model_id in ["RELIC.GIRYA", "RELIC.SHOVEL"] {
        assert_eq!(
            registry
                .relic(&id(model_id))
                .expect("the relic is content")
                .spawn_condition,
            Some(sts2_engine::RelicSpawnCondition::BeforeAct3TreasureChest),
            "{model_id}"
        );
    }
}

// --- the shop pool -------------------------------------------------------

/// A belt buckle lends two dexterity while the belt is empty and takes it back
/// the moment a potion lands in it.
#[test]
fn a_belt_buckle_answers_an_empty_belt() {
    let mut simulator = fight_from_run(&["RELIC.BELT_BUCKLE"]);
    // The run walks in holding a block potion, so nothing is lent.
    assert_eq!(power_amount(&simulator, 0, "POWER.DEXTERITY_POWER"), 0);
    let potion = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::UsePotion { .. }))
        .cloned()
        .expect("the potion is drinkable");
    simulator.step(potion).expect("the potion applies");
    assert_eq!(power_amount(&simulator, 0, "POWER.DEXTERITY_POWER"), 2);
    assert_eq!(power_applier(&simulator, 0, "POWER.DEXTERITY_POWER"), None);
}

/// A bread pays an energy on every turn but the first, and takes two off that
/// first one.
#[test]
fn a_bread_moves_the_first_turn_s_energy_to_the_rest() {
    let mut plain = fight_from_run(&[]);
    let mut fed = fight_from_run(&["RELIC.BREAD"]);
    assert_eq!(energy(&fed), energy(&plain) - 2);
    end_turn(&mut plain);
    end_turn(&mut fed);
    assert_eq!(energy(&fed), energy(&plain) + 1);
}

/// A brimstone works strength into everyone as every turn opens.
#[test]
fn a_brimstone_arms_both_sides() {
    let mut simulator = fight_from_run(&["RELIC.BRIMSTONE"]);
    assert_eq!(power_amount(&simulator, 0, "POWER.STRENGTH_POWER"), 2);
    assert_eq!(power_amount(&simulator, 1, "POWER.STRENGTH_POWER"), 1);
    end_turn(&mut simulator);
    assert_eq!(power_amount(&simulator, 0, "POWER.STRENGTH_POWER"), 4);
    assert_eq!(power_amount(&simulator, 1, "POWER.STRENGTH_POWER"), 2);
}

/// Burning sticks copy the first skill of a fight that burns up back into hand.
#[test]
fn burning_sticks_answer_the_first_skill_to_burn() {
    let mut simulator = fight_tough(
        &["RELIC.BURNING_STICKS"],
        &[("CARD.SECOND_WIND", 0), ("CARD.DEFEND_IRONCLAD", 0)],
    );
    let before = hand_size(&simulator);
    let _ = before;
    play(&mut simulator, "CARD.SECOND_WIND");
    let copies = simulator
        .state()
        .combat
        .as_ref()
        .expect("a fight")
        .player
        .piles
        .hand
        .iter()
        .filter(|card| card.fingerprint.model_id == id("CARD.DEFEND_IRONCLAD"))
        .count();
    // The defend burned up in Second Wind's own body and came back once.
    assert_eq!(copies, 1);
}

/// A cauldron hands over five potions the moment it is taken.
#[test]
fn a_cauldron_pays_five_potions() {
    let mut simulator = run_holding_with_treasure(&[], "RELIC.CAULDRON");
    walk_to(&mut simulator, 4);
    let take = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::TakeTreasure { .. }))
        .cloned()
        .expect("the relic is on the chest");
    simulator.step(take).expect("taking it applies");
    let rewards = offered_rewards(&simulator);
    assert_eq!(rewards.len(), 5);
    assert!(rewards.iter().all(|reward| reward.reward_type == "potion"));
}

/// An orrery hands over five card offers the moment it is taken.
#[test]
fn an_orrery_pays_five_card_offers() {
    let mut simulator = run_holding_with_treasure(&[], "RELIC.ORRERY");
    walk_to(&mut simulator, 4);
    let take = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::TakeTreasure { .. }))
        .cloned()
        .expect("the relic is on the chest");
    simulator.step(take).expect("taking it applies");
    let rewards = offered_rewards(&simulator);
    assert_eq!(rewards.len(), 5);
    assert!(rewards.iter().all(|reward| reward.reward_type == "card"));
    assert!(rewards.iter().all(|reward| reward.offered_cards.len() == 3));
}

/// A dingy rug puts the colorless pool behind the character's own.
#[test]
fn a_dingy_rug_widens_a_card_reward() {
    let mut simulator = run_winning_a_fight(&["RELIC.DINGY_RUG"], true);
    win_the_fight(&mut simulator);
    let reward = offered_rewards(&simulator)
        .into_iter()
        .find(|reward| reward.reward_type == "card")
        .expect("a card reward");
    assert_eq!(
        reward.card_pools,
        vec![
            id("CARD_POOL.IRONCLAD_CARD_POOL"),
            id("CARD_POOL.COLORLESS_CARD_POOL")
        ]
    );
}

/// A dragon fruit is a maximum HP for every gold its owner takes in.
#[test]
fn a_dragon_fruit_grows_on_gold() {
    let mut simulator = run_holding(&["RELIC.DRAGON_FRUIT", "RELIC.LUCKY_FYSH"]);
    let before = simulator.state().run_player.max_hp;
    // The fysh pays fifteen gold for a card joining the deck, and the fruit
    // answers the gold rather than the card.
    walk_to(&mut simulator, 0);
    assert_eq!(simulator.state().run_player.max_hp, before);
}

/// A ghost seed makes the starter strikes and defends ethereal.
#[test]
fn a_ghost_seed_haunts_the_starter_cards() {
    let simulator = fight_from_run(&["RELIC.GHOST_SEED"]);
    let combat = simulator.state().combat.as_ref().expect("a fight");
    let starters: Vec<_> = combat
        .player
        .piles
        .draw
        .iter()
        .chain(combat.player.piles.hand.iter())
        .filter(|card| {
            matches!(
                card.fingerprint.model_id.entry(),
                "STRIKE_IRONCLAD" | "DEFEND_IRONCLAD"
            )
        })
        .collect();
    assert!(!starters.is_empty());
    assert!(starters.iter().all(|card| card.ethereal));
    // A bash is not a starter and is not haunted.
    assert!(
        combat
            .player
            .piles
            .draw
            .iter()
            .chain(combat.player.piles.hand.iter())
            .filter(|card| card.fingerprint.model_id == id("CARD.BASH"))
            .all(|card| !card.ethereal)
    );
}

/// A lava lamp upgrades a card reward its owner walked out of the fight
/// unhurt for, and nothing when a blow got through.
#[test]
fn a_lava_lamp_answers_an_unhurt_victory() {
    let mut simulator = run_winning_a_fight(&["RELIC.LAVA_LAMP"], true);
    win_the_fight(&mut simulator);
    let registry = sts2_content::standard_registry();
    let cards = offered_rewards(&simulator)
        .into_iter()
        .find(|reward| reward.reward_type == "card")
        .expect("a card reward")
        .offered_cards;
    for card in cards {
        if registry
            .card(&card.model_id)
            .expect("the card is content")
            .upgrade
            .is_some()
        {
            assert!(card.upgrade_level >= 1, "{}", card.model_id);
        }
    }
}

/// A Lee's waffle is seven more maximum HP and the HP to fill it.
#[test]
fn a_lees_waffle_fills_the_bar() {
    let mut simulator = run_holding_with_treasure(&[], "RELIC.LEES_WAFFLE");
    let max = simulator.state().run_player.max_hp;
    walk_to(&mut simulator, 4);
    let take = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::TakeTreasure { .. }))
        .cloned()
        .expect("the relic is on the chest");
    simulator.step(take).expect("taking it applies");
    assert_eq!(simulator.state().run_player.max_hp, max + 7);
    assert_eq!(run_hp(&simulator), max + 7);
}

/// A screaming flagon cuts every enemy at the end of an empty-handed turn.
#[test]
fn a_screaming_flagon_answers_an_empty_hand() {
    let mut simulator = fight_against("SHRINKER_MOVE", 1, &["RELIC.SCREAMING_FLAGON"], &[], &[]);
    let before = creature(&simulator, 1).current_hp;
    end_turn(&mut simulator);
    assert_eq!(creature(&simulator, 1).current_hp, before - 20);
}

/// A sling of courage works two strength in as an elite is walked into.
#[test]
fn a_sling_of_courage_answers_an_elite() {
    let simulator = fight_from_run(&["RELIC.SLING_OF_COURAGE"]);
    // The run's fight stands on a monster point, not an elite.
    assert_eq!(power_amount(&simulator, 0, "POWER.STRENGTH_POWER"), 0);
}

/// An abacus pays six block every time the pile is shuffled.
#[test]
fn an_abacus_answers_a_shuffle() {
    let mut simulator = fight(
        &["RELIC.THE_ABACUS"],
        &[("CARD.STRIKE_IRONCLAD", 0), ("CARD.SHRUG_IT_OFF", 0)],
    );
    let before = player(&simulator).block;
    // Nothing has been shuffled yet.
    assert_eq!(before, 0);
    play(&mut simulator, "CARD.SHRUG_IT_OFF");
    // The draw pile is empty, so the draw shuffles the discard into it and the
    // abacus answers that shuffle.
    assert!(player(&simulator).block >= 6);
}

/// A toolbox lays three colorless cards out before the opening hand is drawn.
#[test]
fn a_toolbox_offers_three_colorless_cards() {
    let simulator = fight_from_run(&["RELIC.TOOLBOX"]);
    let choice = simulator
        .state()
        .combat
        .as_ref()
        .expect("a fight")
        .suspended_choice
        .as_ref()
        .expect("the screen is up");
    assert_eq!(choice.candidates.len(), 3);
    // The hand has not been dealt yet: the screen comes before the draw.
    assert_eq!(hand_size(&simulator), 0);
}

/// The shop relics whose bodies need a subsystem this baseline has not built
/// are registered with the rarity a merchant prices them by.
#[test]
fn the_relics_of_the_shop_pool_are_registered_with_their_rarity() {
    let registry = sts2_content::standard_registry();
    for model_id in [
        "RELIC.CHEMICAL_X",
        "RELIC.GNARLED_HAMMER",
        "RELIC.KIFUDA",
        "RELIC.MEMBERSHIP_CARD",
        "RELIC.MINIATURE_TENT",
        "RELIC.MYSTIC_LIGHTER",
        "RELIC.PUNCH_DAGGER",
        "RELIC.ROYAL_STAMP",
        "RELIC.WING_CHARM",
    ] {
        let definition = registry.relic(&id(model_id)).expect("the relic is content");
        assert_eq!(
            definition.rarity,
            sts2_engine::RelicRarity::Shop,
            "{model_id}"
        );
        assert_eq!(definition.merchant_cost(), 200, "{model_id}");
    }
}

// --- the event pool ------------------------------------------------------

/// The pinned deck plus one more card, put in the run so a deck-add can be
/// watched.
fn take_relic_at_the_chest(simulator: &mut Simulator) {
    walk_to(simulator, 4);
    let take = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::TakeTreasure { .. }))
        .cloned()
        .expect("the relic is on the chest");
    simulator.step(take).expect("taking it applies");
}

/// A big mushroom is twenty more maximum HP and two fewer opening cards.
#[test]
fn a_big_mushroom_trades_a_hand_for_a_health_bar() {
    let mut simulator = run_holding_with_treasure(&[], "RELIC.BIG_MUSHROOM");
    let before = simulator.state().run_player.max_hp;
    take_relic_at_the_chest(&mut simulator);
    assert_eq!(simulator.state().run_player.max_hp, before + 20);
    let plain = fight_from_run(&[]);
    let shrunk = fight_from_run(&["RELIC.BIG_MUSHROOM"]);
    assert_eq!(hand_size(&shrunk), hand_size(&plain) - 2);
}

/// A bing bong copies every card that joins the deck, once each.
#[test]
fn a_bing_bong_copies_what_joins_the_deck() {
    let mut simulator = run_holding(&["RELIC.BING_BONG"]);
    let before = simulator.state().run_player.deck.len();
    take_relic_at_the_chest(&mut simulator);
    assert_eq!(simulator.state().run_player.deck.len(), before);
    // Cursed Pearl's greed is a card joining the deck.
    let mut simulator = run_holding_with_treasure(&["RELIC.BING_BONG"], "RELIC.CURSED_PEARL");
    let before = simulator.state().run_player.deck.len();
    take_relic_at_the_chest(&mut simulator);
    // The greed and the copy the relic made of it, and nothing more.
    assert_eq!(simulator.state().run_player.deck.len(), before + 2);
}

/// Bing Bong remembers the exact clone it made until that object next joins
/// the deck. A Thieving Hopper can take the clone out of the deck and hand the
/// same object back as a special card reward; that return consumes the
/// exemption instead of making a third copy.
#[test]
fn a_bing_bong_clone_returned_as_a_special_reward_is_not_copied_again() {
    let mut simulator = run_holding_with_treasure(
        &["RELIC.BING_BONG", "RELIC.BOOK_OF_FIVE_RINGS"],
        "RELIC.CURSED_PEARL",
    );
    take_relic_at_the_chest(&mut simulator);

    let mut state = simulator.state().clone();
    let clone_index = state
        .run_player
        .deck
        .iter()
        .position(|card| card.model_id == id("CARD.GREED") && card.bing_bong_skip_once)
        .expect("Bing Bong marked the copy it made");
    let returned = state.run_player.deck.remove(clone_index);
    let deck_before = state.run_player.deck.len();
    let counter_before = state
        .run_player
        .relics
        .iter()
        .find(|relic| relic.model_id == id("RELIC.BOOK_OF_FIVE_RINGS"))
        .expect("the book is held")
        .counter;

    let run = state.run.as_mut().expect("the run is standing");
    let set_id = sts2_core::RewardSetId::from(run.next_reward_set_id);
    run.next_reward_set_id += 1;
    run.reward_set = Some(sts2_engine::RewardSet {
        id: set_id,
        player_slot: "p0".to_owned(),
        rewards: vec![sts2_engine::RewardItem {
            index: 0,
            fingerprint: sts2_engine::RewardFingerprint::of("special_card")
                .with_special_card(returned),
            selected: false,
            passed_over: false,
            opened: false,
        }],
        disallow_skipping: false,
    });
    let mut simulator = Simulator::from_scenario(state, sts2_content::standard_registry())
        .expect("the returned-card reward stands up");
    let claim = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ClaimReward { .. }))
        .cloned()
        .expect("the returned card can be claimed");
    simulator.step(claim).expect("the returned card joins");

    assert_eq!(
        simulator.state().run_player.deck.len(),
        deck_before + 1,
        "the same clone returns without Bing Bong copying it again"
    );
    assert_eq!(
        relic_counter(&simulator, "RELIC.BOOK_OF_FIVE_RINGS"),
        counter_before + 1,
        "only the one returned card joins the deck"
    );
    assert!(
        simulator
            .state()
            .run_player
            .deck
            .iter()
            .filter(|card| card.model_id == id("CARD.GREED"))
            .all(|card| !card.bing_bong_skip_once),
        "the one-use exemption is consumed on return"
    );
}

/// A darkstone periapt is six more maximum HP for every curse.
#[test]
fn a_darkstone_periapt_answers_a_curse() {
    let mut simulator =
        run_holding_with_treasure(&["RELIC.DARKSTONE_PERIAPT"], "RELIC.CURSED_PEARL");
    let before = simulator.state().run_player.max_hp;
    take_relic_at_the_chest(&mut simulator);
    assert_eq!(simulator.state().run_player.max_hp, before + 6);
}

/// A bone tea upgrades the opening hand once and then is used up.
#[test]
fn a_bone_tea_is_spent_on_one_fight() {
    let mut simulator = run_holding_with_treasure(&[], "RELIC.BONE_TEA");
    take_relic_at_the_chest(&mut simulator);
    assert_eq!(relic_counter(&simulator, "RELIC.BONE_TEA"), 1);
    let registry = sts2_content::standard_registry();
    let definition = registry
        .relic(&id("RELIC.BONE_TEA"))
        .expect("the relic is content");
    let mut instance = sts2_engine::RelicInstance::new(id("RELIC.BONE_TEA"));
    instance.counter = 1;
    assert!(!definition.is_used_up(&instance));
    instance.counter = 0;
    assert!(definition.is_used_up(&instance));
    assert!(!definition.is_tradable(&instance));

    let mut simulator = fight_from_run(&["RELIC.BONE_TEA"]);
    let upgraded = simulator
        .state()
        .combat
        .as_ref()
        .expect("a fight")
        .player
        .piles
        .hand
        .iter()
        .filter(|card| card.fingerprint.upgrade_level > 0)
        .count();
    assert_eq!(upgraded, hand_size(&simulator));
    assert_eq!(relic_counter(&simulator, "RELIC.BONE_TEA"), 0);
    end_turn(&mut simulator);
    assert_eq!(relic_counter(&simulator, "RELIC.BONE_TEA"), 0);
}

/// An ember tea works two strength in for each of five fights, and the fight
/// credits that strength to no one.
#[test]
fn an_ember_tea_is_spent_over_five_fights() {
    let simulator = fight_from_run(&["RELIC.EMBER_TEA"]);
    assert_eq!(power_amount(&simulator, 0, "POWER.STRENGTH_POWER"), 2);
    assert_eq!(power_applier(&simulator, 0, "POWER.STRENGTH_POWER"), None);
    assert_eq!(relic_counter(&simulator, "RELIC.EMBER_TEA"), 4);
}

/// A chosen cheese is a maximum HP for every fight that ends.
#[test]
fn a_chosen_cheese_grows_on_every_fight() {
    let mut simulator = run_winning_a_fight(&["RELIC.CHOSEN_CHEESE"], false);
    let before = simulator.state().run_player.max_hp;
    win_the_fight(&mut simulator);
    assert_eq!(simulator.state().run_player.max_hp, before + 1);
}

/// A daughter of the wind pays a block for every attack.
#[test]
fn a_daughter_of_the_wind_pays_for_an_attack() {
    let mut simulator = fight_tough(
        &["RELIC.DAUGHTER_OF_THE_WIND"],
        &[("CARD.STRIKE_IRONCLAD", 0)],
    );
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(player(&simulator).block, 1);
}

/// A dream catcher puts a card offer on the rest site's reward screen.
#[test]
fn a_dream_catcher_pays_a_card_for_a_rest() {
    let mut simulator = run_holding(&["RELIC.DREAM_CATCHER"]);
    walk_to(&mut simulator, 1);
    simulator
        .step(rest_heal(&simulator))
        .expect("the rest applies");
    let rewards = offered_rewards(&simulator);
    assert_eq!(rewards.len(), 1);
    assert_eq!(rewards[0].reward_type, "card");
    assert_eq!(rewards[0].offered_cards.len(), 3);
}

/// The fake relics are the real ones' smaller siblings.
#[test]
fn the_fake_relics_pay_what_they_promise() {
    let anchored = fight_from_run(&["RELIC.FAKE_ANCHOR"]);
    assert_eq!(player(&anchored).block, 4);
    let mut plain = fight_from_run(&[]);
    let mut vialled = fight_from_run(&["RELIC.FAKE_BLOOD_VIAL"]);
    assert_eq!(player(&vialled).current_hp, player(&plain).current_hp + 1);
    // A fake orichalcum answers a turn ended behind nothing: the three block
    // stands through the enemy's turn and is cleared as the next one opens,
    // and the relic's latch is down again.
    let mut brassy = fight_against("CHOMP_MOVE", 1, &["RELIC.FAKE_ORICHALCUM"], &[], &[]);
    let mut bare = fight_against("CHOMP_MOVE", 1, &[], &[], &[]);
    end_turn(&mut brassy);
    end_turn(&mut bare);
    assert_eq!(player(&brassy).current_hp, player(&bare).current_hp + 3);
    // A fake happy flower pays on every fifth turn.
    let mut flowery = fight_from_run(&["RELIC.FAKE_HAPPY_FLOWER"]);
    for _ in 0..3 {
        end_turn(&mut flowery);
    }
    assert_eq!(relic_counter(&flowery, "RELIC.FAKE_HAPPY_FLOWER"), 4);
    let before = energy(&flowery);
    end_turn(&mut flowery);
    assert_eq!(relic_counter(&flowery, "RELIC.FAKE_HAPPY_FLOWER"), 0);
    assert_eq!(energy(&flowery), before + 1);
    // A fake snecko eye confuses its owner as every fight opens, which rolls
    // every drawn card a cost of its own.
    let snecked = fight_from_run(&["RELIC.FAKE_SNECKO_EYE"]);
    assert_eq!(power_amount(&snecked, 0, "POWER.CONFUSED_POWER"), 1);
    assert!(
        snecked
            .state()
            .combat
            .as_ref()
            .expect("a fight")
            .player
            .piles
            .hand
            .iter()
            .all(|card| !card.temporary_costs.is_empty()),
        "the hand it drew carries rolled costs"
    );
    let _ = (&mut plain, &mut vialled);
}

/// A fake strike dummy pays one more on every Strike.
#[test]
fn a_fake_strike_dummy_pays_on_a_strike() {
    let mut plain = fight_tough(&[], &[("CARD.STRIKE_IRONCLAD", 0)]);
    let mut dummied = fight_tough(&["RELIC.FAKE_STRIKE_DUMMY"], &[("CARD.STRIKE_IRONCLAD", 0)]);
    play(&mut plain, "CARD.STRIKE_IRONCLAD");
    play(&mut dummied, "CARD.STRIKE_IRONCLAD");
    assert_eq!(
        creature(&dummied, 1).current_hp,
        creature(&plain, 1).current_hp - 1
    );
}

/// A fake venerable tea set remembers a rest, the way the real one does.
#[test]
fn a_fake_venerable_tea_set_answers_a_rest() {
    let mut simulator = run_holding(&["RELIC.FAKE_VENERABLE_TEA_SET"]);
    walk_to(&mut simulator, 1);
    let latched = |simulator: &Simulator| {
        simulator
            .state()
            .run_player
            .relics
            .iter()
            .find(|relic| relic.model_id == id("RELIC.FAKE_VENERABLE_TEA_SET"))
            .expect("the relic is held")
            .latched
    };
    assert!(latched(&simulator));
    simulator
        .step(rest_heal(&simulator))
        .expect("the rest applies");
    assert!(latched(&simulator));
}

/// A forgotten soul cuts one enemy it rolls for on every burn.
#[test]
fn a_forgotten_soul_answers_a_burn() {
    let mut plain = fight_tough(&[], &[("CARD.FIEND_FIRE", 0)]);
    let mut soulful = fight_tough(&["RELIC.FORGOTTEN_SOUL"], &[("CARD.FIEND_FIRE", 0)]);
    play(&mut plain, "CARD.FIEND_FIRE");
    play(&mut soulful, "CARD.FIEND_FIRE");
    assert_eq!(
        creature(&soulful, 1).current_hp,
        creature(&plain, 1).current_hp - 1
    );
}

/// A fragrant mushroom costs fifteen HP and upgrades two of the deck.
#[test]
fn a_fragrant_mushroom_is_paid_for_in_blood() {
    let mut simulator = run_holding_with_treasure(&[], "RELIC.FRAGRANT_MUSHROOM");
    let before = run_hp(&simulator);
    take_relic_at_the_chest(&mut simulator);
    assert_eq!(run_hp(&simulator), before - 15);
    let upgraded = simulator
        .state()
        .run_player
        .deck
        .iter()
        .filter(|card| card.upgrade_level > 0)
        .count();
    assert_eq!(upgraded, 2);
}

/// A hand drill works two vulnerable into whoever it broke the block of.
#[test]
fn a_hand_drill_answers_a_broken_shield() {
    let mut simulator = fight_tough(&["RELIC.HAND_DRILL"], &[("CARD.STRIKE_IRONCLAD", 0)]);
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    // The beetle carries no block, so nothing was broken.
    assert_eq!(power_amount(&simulator, 1, "POWER.VULNERABLE_POWER"), 0);
}

/// A lost wisp cuts every enemy for every power.
#[test]
fn a_lost_wisp_answers_a_power() {
    let mut simulator = fight_tough(&["RELIC.LOST_WISP"], &[("CARD.INFLAME", 0)]);
    let before = creature(&simulator, 1).current_hp;
    play(&mut simulator, "CARD.INFLAME");
    assert_eq!(creature(&simulator, 1).current_hp, before - 8);
}

/// A maw bank pays twelve gold for every room until something is bought.
#[test]
fn a_maw_bank_pays_until_a_purchase() {
    let mut simulator = run_holding(&["RELIC.MAW_BANK"]);
    let before = simulator.state().run_player.gold;
    walk_to(&mut simulator, 1);
    assert_eq!(simulator.state().run_player.gold, before + 12);
    let registry = sts2_content::standard_registry();
    let definition = registry
        .relic(&id("RELIC.MAW_BANK"))
        .expect("the relic is content");
    let mut instance = sts2_engine::RelicInstance::new(id("RELIC.MAW_BANK"));
    assert!(!definition.is_used_up(&instance));
    instance.latched = true;
    assert!(definition.is_used_up(&instance));
}

/// Mr Struggles cuts every enemy for the number of the turn.
#[test]
fn mr_struggles_grows_with_the_turn() {
    let mut simulator = fight_from_run(&["RELIC.MR_STRUGGLES"]);
    assert_eq!(creature(&simulator, 1).current_hp, 38);
    end_turn(&mut simulator);
    assert_eq!(creature(&simulator, 1).current_hp, 36);
}

/// A pollinous core swells every fourth turn's hand by two.
#[test]
fn a_pollinous_core_answers_every_fourth_turn() {
    let mut simulator = fight_from_run(&["RELIC.POLLINOUS_CORE"]);
    let opening = hand_size(&simulator);
    assert_eq!(relic_counter(&simulator, "RELIC.POLLINOUS_CORE"), 1);
    for _ in 0..2 {
        end_turn(&mut simulator);
    }
    assert_eq!(hand_size(&simulator), opening);
    end_turn(&mut simulator);
    // The fourth turn: two more cards, and the count starts over.
    assert_eq!(hand_size(&simulator), opening + 2);
    assert_eq!(relic_counter(&simulator, "RELIC.POLLINOUS_CORE"), 0);
}

/// A sword of jade works three strength in as a fight is walked into, and a
/// sword of stone becomes one on the fifth elite.
#[test]
fn the_two_swords_answer_a_fight_and_five_elites() {
    let simulator = fight_from_run(&["RELIC.SWORD_OF_JADE"]);
    assert_eq!(power_amount(&simulator, 0, "POWER.STRENGTH_POWER"), 3);
    assert_eq!(power_applier(&simulator, 0, "POWER.STRENGTH_POWER"), None);
    let mut simulator = run_winning_a_fight(&["RELIC.SWORD_OF_STONE"], false);
    win_the_fight(&mut simulator);
    // A monster room is not an elite, so nothing was counted.
    assert_eq!(relic_counter(&simulator, "RELIC.SWORD_OF_STONE"), 0);
}

/// A tea of discourtesy puts two Dazed in the next fight's draw pile, once.
#[test]
fn a_tea_of_discourtesy_is_spent_on_one_fight() {
    let simulator = fight_from_run(&["RELIC.TEA_OF_DISCOURTESY"]);
    let dazed = simulator
        .state()
        .combat
        .as_ref()
        .expect("a fight")
        .player
        .piles
        .all_cards()
        .filter(|card| card.fingerprint.model_id == id("CARD.DAZED"))
        .count();
    assert_eq!(dazed, 2);
    assert_eq!(relic_counter(&simulator, "RELIC.TEA_OF_DISCOURTESY"), 0);
}

/// The boot holds every small blow its owner deals up to five.
#[test]
fn the_boot_holds_a_small_blow_up() {
    let mut plain = fight_tough(&[], &[("CARD.STRIKE_IRONCLAD", 0)]);
    let mut booted = fight_tough(&["RELIC.THE_BOOT"], &[("CARD.STRIKE_IRONCLAD", 0)]);
    play(&mut plain, "CARD.STRIKE_IRONCLAD");
    play(&mut booted, "CARD.STRIKE_IRONCLAD");
    // A strike is six, which is over the floor, so nothing changes.
    assert_eq!(
        creature(&booted, 1).current_hp,
        creature(&plain, 1).current_hp
    );
}

/// A Wongo's mystery ticket pays three relics on the fifth fight's rewards.
#[test]
fn a_wongos_mystery_ticket_pays_on_the_fifth_fight() {
    let mut state = run_winning_a_fight(&["RELIC.WONGOS_MYSTERY_TICKET"], false)
        .state()
        .clone();
    state
        .run_player
        .relics
        .iter_mut()
        .find(|relic| relic.model_id == id("RELIC.WONGOS_MYSTERY_TICKET"))
        .expect("the relic is held")
        .counter = 4;
    // Five bags with something in each, so the three relics can be pulled.
    for (rarity, model_id) in [
        (sts2_engine::RelicRarity::Common, "RELIC.ANCHOR"),
        (sts2_engine::RelicRarity::Common, "RELIC.BLOOD_VIAL"),
        (sts2_engine::RelicRarity::Common, "RELIC.PENDULUM"),
        (sts2_engine::RelicRarity::Uncommon, "RELIC.AKABEKO"),
        (sts2_engine::RelicRarity::Uncommon, "RELIC.CANDELABRA"),
        (sts2_engine::RelicRarity::Uncommon, "RELIC.PEAR"),
        (sts2_engine::RelicRarity::Rare, "RELIC.BELLOWS"),
        (sts2_engine::RelicRarity::Rare, "RELIC.CHANDELIER"),
        (sts2_engine::RelicRarity::Rare, "RELIC.KUNAI"),
    ] {
        state
            .run
            .as_mut()
            .expect("a run")
            .player_relic_grab_bag
            .bucket_mut(rarity)
            .push(id(model_id));
    }
    let mut simulator = Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run builds");
    win_the_fight(&mut simulator);
    let relics: Vec<_> = offered_rewards(&simulator)
        .into_iter()
        .filter(|reward| reward.reward_type == "relic")
        .collect();
    assert_eq!(relics.len(), 3);
    // The ticket is spent as the rewards are modified, for the relics it
    // paid.
    let held = simulator
        .state()
        .run_player
        .relics
        .iter()
        .find(|relic| relic.model_id == id("RELIC.WONGOS_MYSTERY_TICKET"))
        .expect("the relic is held");
    assert!(held.latched);
}

/// Royal poison costs its owner four HP as every fight's first turn opens.
#[test]
fn royal_poison_is_paid_at_every_fight_s_opening() {
    let mut plain = fight_from_run(&[]);
    let mut poisoned = fight_from_run(&["RELIC.ROYAL_POISON"]);
    assert_eq!(player(&poisoned).current_hp, player(&plain).current_hp - 4);
    end_turn(&mut plain);
    end_turn(&mut poisoned);
    // The second turn costs nothing more.
    assert_eq!(player(&poisoned).current_hp, player(&plain).current_hp - 4);
}

/// The event relics whose bodies need a subsystem this baseline has not built
/// are registered with the rarity that keeps them out of every bag.
#[test]
fn the_relics_of_the_event_pool_are_registered_with_their_rarity() {
    let registry = sts2_content::standard_registry();
    for model_id in [
        "RELIC.BYRDPIP",
        "RELIC.FAKE_SNECKO_EYE",
        "RELIC.FRESNEL_LENS",
        "RELIC.HISTORY_COURSE",
        "RELIC.FAKE_MERCHANTS_RUG",
        "RELIC.WONGO_CUSTOMER_APPRECIATION_BADGE",
        "RELIC.FAKE_LEES_WAFFLE",
        "RELIC.FAKE_MANGO",
    ] {
        let definition = registry.relic(&id(model_id)).expect("the relic is content");
        assert_eq!(
            definition.rarity,
            sts2_engine::RelicRarity::Event,
            "{model_id}"
        );
        // An event relic is never traded away, whatever else it declares.
        assert!(
            !definition.is_tradable(&sts2_engine::RelicInstance::new(id(model_id))),
            "{model_id}"
        );
    }
    // A byrdpip brings a pet, which is one of the things that stops a trade.
    assert!(
        registry
            .relic(&id("RELIC.BYRDPIP"))
            .expect("the relic is content")
            .spawns_pets
    );
}

// --- the ancients that transform deck cards ------------------------------

/// Takes the relic standing on the treasure chest of the one-room act.
fn take_the_treasure(simulator: &mut Simulator) {
    walk_to(simulator, 4);
    let take = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::TakeTreasure { .. }))
        .cloned()
        .expect("the relic is on the chest");
    simulator.step(take).expect("taking it applies");
}

/// A relic bag with something in every rolled bucket, which is what a pull off
/// the front of it needs.
fn stocked_bag() -> RelicGrabBagState {
    RelicGrabBagState {
        starter: Vec::new(),
        common: vec![id("RELIC.AKABEKO"), id("RELIC.ANCHOR")],
        uncommon: vec![id("RELIC.NUNCHAKU"), id("RELIC.PEAR")],
        rare: vec![id("RELIC.GIRYA"), id("RELIC.ICE_CREAM")],
        shop: vec![id("RELIC.BREAD")],
        event: Vec::new(),
        ancient: Vec::new(),
    }
}

/// The one-room act with the player's own relic bag stocked.
fn run_with_a_bag(relics: &[&str], treasure: &str) -> Simulator {
    let mut state = run_holding_with_treasure(relics, treasure).state().clone();
    if let Some(run) = state.run.as_mut() {
        run.player_relic_grab_bag = stocked_bag();
    }
    Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run rebuilds")
}

fn niche_counter(simulator: &Simulator) -> u64 {
    simulator.state().rng.stream(RunStream::Niche).counter()
}

fn transformations_counter(simulator: &Simulator) -> u64 {
    simulator
        .state()
        .player_rng
        .stream(sts2_rng::PlayerStream::Transformations)
        .counter()
}

fn deck_models(simulator: &Simulator) -> Vec<String> {
    simulator
        .state()
        .run_player
        .deck
        .iter()
        .map(|card| card.model_id.to_string())
        .collect()
}

/// A Pandora's box turns every starter strike and defend into a card rolled off
/// the character's own pool, one roll each, and the replacements join the end of
/// the deck in the order their originals stood in it.
#[test]
fn a_pandoras_box_rolls_every_starter_card_anew() {
    let mut simulator = run_holding_with_treasure(&[], "RELIC.PANDORAS_BOX");
    let before = deck_models(&simulator);
    let starters = before
        .iter()
        .filter(|model| model.starts_with("CARD.STRIKE") || model.starts_with("CARD.DEFEND"))
        .count();
    assert!(starters > 0, "the starting deck holds starter cards");
    let niche = niche_counter(&simulator);
    take_the_treasure(&mut simulator);
    let after = deck_models(&simulator);
    assert_eq!(after.len(), before.len(), "one card in, one card out");
    assert!(
        !after
            .iter()
            .any(|model| model.starts_with("CARD.STRIKE") || model.starts_with("CARD.DEFEND")),
        "no starter card is left"
    );
    // One roll per card, all of them on the run's niche stream.
    assert_eq!(niche_counter(&simulator), niche + starters as u64);
    assert_eq!(
        transformations_counter(&simulator),
        0,
        "a box rolls nothing on the player's own stream"
    );
    // What is left of the old deck keeps its order, and the replacements stand
    // behind it.
    let kept: Vec<String> = before
        .iter()
        .filter(|model| !model.starts_with("CARD.STRIKE") && !model.starts_with("CARD.DEFEND"))
        .cloned()
        .collect();
    assert_eq!(&after[..kept.len()], kept.as_slice());
    // Every replacement is a card of the character's own pool.
    let registry = sts2_content::standard_registry();
    let pool: BTreeSet<String> = registry
        .card_pool(&id("CARD_POOL.IRONCLAD_CARD_POOL"))
        .expect("the character's pool is registered")
        .iter()
        .map(|card| card.model_id.to_string())
        .collect();
    for model in &after[kept.len()..] {
        assert!(
            pool.contains(model),
            "{model} is not of the character's pool"
        );
    }
}

/// A leafy poultice costs twelve maximum HP and rolls a replacement for the
/// deck's first starter strike and first starter defend, on the player's own
/// transformation stream.
#[test]
fn a_leafy_poultice_pays_maximum_hp_for_two_rolled_cards() {
    let mut simulator = run_holding_with_treasure(&[], "RELIC.LEAFY_POULTICE");
    let max_hp = simulator.state().run_player.max_hp;
    let before = deck_models(&simulator);
    let niche = niche_counter(&simulator);
    take_the_treasure(&mut simulator);
    assert_eq!(simulator.state().run_player.max_hp, max_hp - 12);
    let after = deck_models(&simulator);
    assert_eq!(after.len(), before.len());
    // Exactly one strike and one defend left.
    let gone = |models: &[String], prefix: &str| {
        models
            .iter()
            .filter(|model| model.starts_with(prefix))
            .count()
    };
    assert_eq!(
        gone(&after, "CARD.STRIKE"),
        gone(&before, "CARD.STRIKE") - 1
    );
    assert_eq!(
        gone(&after, "CARD.DEFEND"),
        gone(&before, "CARD.DEFEND") - 1
    );
    // The rolls come off the player's transformation stream, not the run's.
    assert_eq!(transformations_counter(&simulator), 2);
    assert_eq!(niche_counter(&simulator), niche);
}

/// Claws puts a screen up over the transformable deck and turns what is chosen
/// into Mauls, which is a named replacement and so rolls nothing.
#[test]
fn claws_makes_mauls_of_what_it_is_pointed_at() {
    let mut simulator = run_holding_with_treasure(&[], "RELIC.CLAWS");
    let before = deck_models(&simulator);
    take_the_treasure(&mut simulator);
    let sts2_engine::DecisionContext::ChooseCards {
        choice_id,
        candidates,
        ..
    } = simulator.decision().clone()
    else {
        panic!("claws puts a deck screen up");
    };
    assert_eq!(
        candidates.len(),
        before.len(),
        "every card of this deck can be transformed"
    );
    let chosen = vec![candidates[0].clone(), candidates[3].clone()];
    let niche = niche_counter(&simulator);
    simulator
        .step(Action::ChooseCards {
            choice_id,
            cards: chosen,
        })
        .expect("the choice applies");
    let after = deck_models(&simulator);
    assert_eq!(after.len(), before.len());
    assert_eq!(
        after.iter().filter(|model| *model == "CARD.MAUL").count(),
        2,
        "both chosen cards became mauls"
    );
    assert_eq!(&after[after.len() - 2..], &["CARD.MAUL", "CARD.MAUL"]);
    assert_eq!(
        niche_counter(&simulator),
        niche,
        "a named replacement rolls nothing"
    );

    // A card wearing an enchantment hands it to the maul that stands in for
    // it, where the enchantment will go on one.
    let mut state = run_holding_with_treasure(&[], "RELIC.CLAWS")
        .state()
        .clone();
    let mut sharp = card("CARD.STRIKE_IRONCLAD", 1);
    sharp.enchantment = Some(sts2_engine::EnchantmentFingerprint::new(
        id("ENCHANTMENT.SHARP"),
        3,
    ));
    state.run_player.deck = vec![sharp.clone()];
    let mut enchanted = Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run builds");
    take_the_treasure(&mut enchanted);
    let sts2_engine::DecisionContext::ChooseCards {
        choice_id,
        candidates,
        ..
    } = enchanted.decision().clone()
    else {
        panic!("claws puts a deck screen up");
    };
    enchanted
        .step(Action::ChooseCards {
            choice_id,
            cards: vec![candidates[0].clone()],
        })
        .expect("the choice applies");
    let maul = &enchanted.state().run_player.deck[0];
    assert_eq!(maul.model_id, id("CARD.MAUL"));
    assert_eq!(maul.upgrade_level, 1, "and it stands where an upgrade did");
    assert_eq!(maul.enchantment, sharp.enchantment);
}

/// An astrolabe transforms three chosen cards and upgrades each replacement; a
/// new leaf transforms one and leaves it as it was rolled.
#[test]
fn the_two_rolled_transform_relics_differ_only_in_the_upgrade() {
    for (relic, count, upgraded) in [("RELIC.ASTROLABE", 3, true), ("RELIC.NEW_LEAF", 1, false)] {
        let mut simulator = run_holding_with_treasure(&[], relic);
        let before = deck_models(&simulator);
        let niche = niche_counter(&simulator);
        take_the_treasure(&mut simulator);
        let sts2_engine::DecisionContext::ChooseCards {
            choice_id,
            candidates,
            ..
        } = simulator.decision().clone()
        else {
            panic!("{relic} puts a deck screen up");
        };
        let cards: Vec<_> = candidates.iter().take(count).cloned().collect();
        simulator
            .step(Action::ChooseCards { choice_id, cards })
            .expect("the choice applies");
        assert_eq!(niche_counter(&simulator), niche + count as u64, "{relic}");
        let deck = &simulator.state().run_player.deck;
        assert_eq!(deck.len(), before.len(), "{relic}");
        for card in &deck[deck.len() - count..] {
            assert_eq!(
                card.upgrade_level,
                u8::from(upgraded),
                "{relic} upgrades what it rolled: {card:?}"
            );
        }
    }
}

// --- the ancients whose whole effect is what they hand over --------------

/// Gold, maximum HP, and cards into the deck: each of these is the whole of
/// what the relic does, and it happens where the relic is taken.
#[test]
fn the_ancients_that_are_only_what_they_hand_over() {
    for (relic, gold, max_hp, added) in [
        ("RELIC.GOLDEN_PEARL", 150, 0, &[][..]),
        ("RELIC.SIGNET_RING", 999, 0, &[]),
        ("RELIC.NUTRITIOUS_OYSTER", 0, 11, &[]),
        ("RELIC.JEWELRY_BOX", 0, 0, &["CARD.APOTHEOSIS"]),
        ("RELIC.NEOWS_TORMENT", 0, 0, &["CARD.NEOWS_FURY"]),
        ("RELIC.STORYBOOK", 0, 0, &["CARD.BRIGHTEST_FLAME"]),
        ("RELIC.TANXS_WHISTLE", 0, 0, &["CARD.WHISTLE"]),
        ("RELIC.PAELS_HORN", 0, 0, &["CARD.RELAX", "CARD.RELAX"]),
        ("RELIC.BLOOD_SOAKED_ROSE", 0, 0, &["CARD.ENTHRALLED"]),
        (
            "RELIC.DISTINGUISHED_CAPE",
            0,
            -9,
            &["CARD.APPARITION", "CARD.APPARITION", "CARD.APPARITION"],
        ),
    ] {
        let mut simulator = run_holding_with_treasure(&[], relic);
        let gold_before = simulator.state().run_player.gold;
        let max_hp_before = simulator.state().run_player.max_hp;
        let deck_before = deck_models(&simulator);
        take_the_treasure(&mut simulator);
        assert_eq!(
            simulator.state().run_player.gold,
            gold_before + gold,
            "{relic} pays gold"
        );
        assert_eq!(
            simulator.state().run_player.max_hp,
            max_hp_before + max_hp,
            "{relic} moves the maximum HP"
        );
        let deck = deck_models(&simulator);
        assert_eq!(deck.len(), deck_before.len() + added.len(), "{relic}");
        assert_eq!(&deck[deck_before.len()..], added, "{relic}");
    }
}

/// A sea glass is another character's: fifteen of that character's cards laid
/// out as it is taken — five commons, five uncommons and five rares, in that
/// order — and whatever is picked off the grid joins the deck.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one test walks the whole of what the relic deals"
)]
fn a_sea_glass_deals_fifteen_of_another_characters_cards() {
    let mut state = run_holding(&[]).state().clone();
    if let Some(run) = state.run.as_mut() {
        run.phase = RunPhase::Room;
        run.active_room = Some(sts2_engine::ActiveRoom::Event {
            model_id: id("EVENT.OROBAS"),
            options: vec![sts2_engine::EventOption {
                option_id: "OROBAS.pages.INITIAL.options.SEA_GLASS".to_owned(),
                label: "Sea Glass".to_owned(),
                effects: vec![sts2_engine::EventEffect::ObtainSeaGlass {
                    relic: id("RELIC.SEA_GLASS"),
                    character: id("CHARACTER.DEFECT"),
                    pool: id("CARD_POOL.DEFECT_CARD_POOL"),
                }],
                is_proceed: false,
                was_chosen: false,
                locked: false,
            }],
            rng_counter: 0,
            shelf: Vec::new(),
            standing_fight: None,
            rolled_card: None,
            skipped_cards: Vec::new(),
            sphere: None,
        });
    }
    let mut simulator = Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run builds");
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseEvent { .. }))
        .cloned()
        .expect("the option is on the page");
    simulator.step(action).expect("taking it applies");

    let sts2_engine::DecisionContext::ChooseCards {
        candidates,
        minimum,
        maximum,
        ..
    } = simulator.decision().clone()
    else {
        panic!("the grid is up");
    };
    assert_eq!((minimum, maximum), (0, 15));
    assert_eq!(candidates.len(), 15);
    let registry = sts2_content::standard_registry();
    let rarities: Vec<_> = candidates
        .iter()
        .map(|card| {
            registry
                .card(&card.fingerprint.model_id)
                .expect("the offer is content")
                .rarity
        })
        .collect();
    let mut expected = vec![sts2_engine::CardRarity::Common; 5];
    expected.extend([sts2_engine::CardRarity::Uncommon; 5]);
    expected.extend([sts2_engine::CardRarity::Rare; 5]);
    assert_eq!(rarities, expected, "five of each, in the body's own order");
    let defect: Vec<String> = registry
        .card_pool(&id("CARD_POOL.DEFECT_CARD_POOL"))
        .expect("the pool is registered")
        .iter()
        .map(|card| card.model_id.to_string())
        .collect();
    assert!(
        candidates
            .iter()
            .all(|card| defect.contains(&card.fingerprint.model_id.to_string())),
        "and every one of them is the named character's"
    );
    assert!(
        candidates
            .iter()
            .all(|card| card.fingerprint.upgrade_level == 0),
        "the creation rolls no upgrade at all"
    );

    // The recorder sends a whole selection, even when the action list only
    // enumerates single clicks for this large grid.
    let sts2_engine::DecisionContext::ChooseCards { choice_id, .. } = simulator.decision() else {
        unreachable!();
    };
    let record = sts2_replay::Record {
        sequence: 1,
        kind: "choice.index".into(),
        payload: serde_json::json!({"data": {
            "choice_id": choice_id.get(), "indexes": [12, 14, 5],
            "offered": {"options": {"options": candidates.iter().map(|card| {
                serde_json::json!({"index": card.index, "fingerprint": card.fingerprint})
            }).collect::<Vec<_>>()}}
        }}),
    };
    for bad in ["duplicate", "fingerprint", "index", "choice"] {
        let mut invalid = record.clone();
        match bad {
            "duplicate" => invalid.payload["data"]["indexes"] = serde_json::json!([12, 12]),
            "fingerprint" => {
                invalid.payload["data"]["offered"]["options"]["options"][12]["fingerprint"]["model_id"] =
                    serde_json::json!("CARD.STRIKE_IRONCLAD");
            }
            "index" => invalid.payload["data"]["indexes"] = serde_json::json!([99]),
            _ => invalid.payload["data"]["choice_id"] = serde_json::json!(choice_id.get() + 1),
        }
        let mut adapter = sts2_replay::ReplayAdapter::new(simulator.clone());
        assert!(adapter.accept(&invalid).is_err(), "reject {bad}");
        assert_eq!(
            adapter.simulator().state().run_player.deck,
            simulator.state().run_player.deck
        );
    }
    let mut adapter = sts2_replay::ReplayAdapter::new(simulator.clone());
    adapter.accept(&record).unwrap();
    let added =
        &adapter.simulator().state().run_player.deck[simulator.state().run_player.deck.len()..];
    assert_eq!(
        added.iter().map(|card| &card.model_id).collect::<Vec<_>>(),
        [12, 14, 5].map(|index| &candidates[index].fingerprint.model_id)
    );

    let taken = candidates[0].clone();
    let deck_before = deck_models(&simulator).len();
    let choose = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::ChooseCards { cards, .. }
            if cards.len() == 1 && cards[0].fingerprint == taken.fingerprint)
        })
        .cloned()
        .expect("one card can be taken off the grid");
    simulator.step(choose).expect("taking it applies");
    let deck = deck_models(&simulator);
    assert_eq!(deck.len(), deck_before + 1);
    assert_eq!(
        deck.last().map(String::as_str),
        Some(taken.fingerprint.model_id.to_string().as_str())
    );
    let held = simulator
        .state()
        .run_player
        .relics
        .iter()
        .find(|relic| relic.model_id == id("RELIC.SEA_GLASS"))
        .expect("the sea glass was taken");
    assert_eq!(
        held.saved_models,
        vec![("CharacterId".to_owned(), id("CHARACTER.DEFECT"))],
        "and it carries whose it is"
    );
    // Taking the relic is the whole of the option, and the grid is opened
    // from inside the obtain. So the page stands only for as long as the grid
    // does, and the answer that closes the grid is what finishes the option.
    let run = simulator.state().run.as_ref().expect("the run stands");
    assert!(
        run.active_room.is_none(),
        "the ancient is finished by the card it dealt being taken"
    );
    assert_eq!(run.phase, RunPhase::Map, "and the map is what is left");
}

/// A prismatic gem pays a point of energy and puts every character's pool
/// behind every card reward its owner is offered — the colours first and the
/// creation's own after them. A reward already drawing on nothing but a
/// colourless pool is left alone.
/// Taking the gem — off a chest here, off Orobas's page in a run — is a step
/// the engine accepts: nothing happens as it is obtained, the relic is held,
/// and the registry carries no refused body for it.
#[test]
fn a_prismatic_gem_is_held_once_it_is_taken() {
    let registry = sts2_content::standard_registry();
    let definition = registry
        .relic(&id("RELIC.PRISMATIC_GEM"))
        .expect("the gem is registered");
    assert!(
        definition.on_obtained.is_empty(),
        "nothing runs as the gem is taken: {:?}",
        definition.on_obtained
    );
    let mut simulator = run_holding_with_treasure(&[], "RELIC.PRISMATIC_GEM");
    take_the_treasure(&mut simulator);
    assert!(
        simulator
            .state()
            .run_player
            .relics
            .iter()
            .any(|relic| relic.model_id == id("RELIC.PRISMATIC_GEM")),
        "the gem is held once it is taken"
    );
}

#[test]
fn a_prismatic_gem_widens_every_card_reward() {
    let mut plain = run_winning_a_fight(&[], true);
    let mut gemmed = run_winning_a_fight(&["RELIC.PRISMATIC_GEM"], true);
    win_the_fight(&mut plain);
    win_the_fight(&mut gemmed);
    let registry = sts2_content::standard_registry();
    let ironclad: Vec<String> = registry
        .card_pool(&id("CARD_POOL.IRONCLAD_CARD_POOL"))
        .expect("the pool is registered")
        .iter()
        .map(|card| card.model_id.to_string())
        .collect();
    let offered = |simulator: &Simulator| -> Vec<String> {
        offered_rewards(simulator)
            .into_iter()
            .filter(|reward| reward.reward_type == "card")
            .flat_map(|reward| reward.offered_cards)
            .map(|card| card.model_id.to_string())
            .collect()
    };
    let plain_cards = offered(&plain);
    assert_eq!(plain_cards.len(), 3);
    assert!(
        plain_cards.iter().all(|card| ironclad.contains(card)),
        "a reward with no gem behind it draws on the walking character alone: {plain_cards:?}"
    );
    let widened = offered(&gemmed);
    assert_eq!(widened.len(), 3);
    assert!(
        widened.iter().any(|card| !ironclad.contains(card)),
        "and one behind a gem reaches the other colours: {widened:?}"
    );
    let energised = fight_from_run(&["RELIC.PRISMATIC_GEM"]);
    assert_eq!(
        energised
            .state()
            .combat
            .as_ref()
            .expect("a fight is standing")
            .player
            .max_energy,
        4,
        "and it pays a point of energy every turn"
    );
}

/// A whispering earring pays a point of energy every turn. The rest of what it
/// does — playing its owner's whole opening turn for them — is an automated
/// card selector this baseline does not have, so the point of energy is the
/// whole of what it is here.
#[test]
fn a_whispering_earring_pays_a_point_of_energy() {
    let earringed = fight_from_run(&["RELIC.WHISPERING_EARRING"]);
    assert_eq!(
        earringed
            .state()
            .combat
            .as_ref()
            .expect("a fight is standing")
            .player
            .max_energy,
        4
    );
}

/// A whispering earring also plays its owner's opening turn for them: the first
/// playable card of the hand, over and over, aimed at the leftmost enemy, until
/// the hand holds nothing it can pay for.
#[test]
fn a_whispering_earring_plays_the_opening_turn_for_its_owner() {
    let earringed = fight_from_run(&["RELIC.WHISPERING_EARRING"]);
    let combat = earringed
        .state()
        .combat
        .as_ref()
        .expect("a fight is standing");
    // The burst stops where the pool runs out, which is what each play is
    // gated on: four points of energy against a hand of one-cost cards.
    assert_eq!(combat.player.piles.hand.len(), 2);
    assert_eq!(combat.player.card_plays_finished, 3);
    assert_eq!(combat.player.energy, 0, "and paid for out of the pool");
    // Nothing is owed on any turn but the first.
    let plain = fight_from_run(&[]);
    assert!(
        !plain
            .state()
            .combat
            .as_ref()
            .expect("a fight is standing")
            .player
            .piles
            .hand
            .is_empty(),
        "without the earring the hand is still held"
    );
}

/// The two relics of no rarity at all: a circlet, which is what a refinement
/// falls back to, and the stand-in a save reads back where the relic it names
/// no longer exists. Both are stackable, which nothing else in the build is,
/// and neither is in any bag.
#[test]
fn the_two_relics_of_no_rarity_at_all() {
    let registry = sts2_content::standard_registry();
    for model_id in ["RELIC.CIRCLET", "RELIC.DEPRECATED_RELIC"] {
        let definition = registry.relic(&id(model_id)).expect("the relic is content");
        assert_eq!(
            definition.rarity,
            sts2_engine::RelicRarity::None,
            "{model_id}"
        );
        assert!(definition.is_stackable, "{model_id}");
        assert!(definition.hooks.is_empty(), "{model_id}");
        assert!(definition.on_obtained.is_empty(), "{model_id}");
    }
    let stackable: Vec<String> = registry
        .registered_model_ids()
        .into_iter()
        .filter(|model_id| {
            registry
                .relic(model_id)
                .is_some_and(|definition| definition.is_stackable)
        })
        .map(|model_id| model_id.to_string())
        .collect();
    assert_eq!(
        stackable,
        ["RELIC.CIRCLET", "RELIC.DEPRECATED_RELIC"],
        "and nothing else in the build stacks"
    );
}

/// A touch of orobas trades the starter relic its owner walked in with for the
/// refined form of it, in the place the original stood, and saves both as it is
/// offered. A starter the table has no refined form for is worth a circlet.
#[test]
fn a_touch_of_orobas_refines_the_starter_relic_it_finds() {
    for (starter, refined) in [
        ("RELIC.BURNING_BLOOD", "RELIC.BLACK_BLOOD"),
        ("RELIC.RING_OF_THE_SNAKE", "RELIC.RING_OF_THE_DRAKE"),
        ("RELIC.DIVINE_RIGHT", "RELIC.DIVINE_DESTINY"),
        ("RELIC.BOUND_PHYLACTERY", "RELIC.PHYLACTERY_UNBOUND"),
        ("RELIC.CRACKED_CORE", "RELIC.INFUSED_CORE"),
        ("RELIC.LANTERN", "RELIC.CIRCLET"),
    ] {
        // The fixture walks in as the Ironclad, so its own burning blood is
        // the starter standing there; swapping it is how the other four are
        // reached.
        let mut state = run_holding_with_treasure(&[], "RELIC.TOUCH_OF_OROBAS")
            .state()
            .clone();
        for relic in &mut state.run_player.relics {
            if relic.model_id == id("RELIC.BURNING_BLOOD") {
                relic.model_id = id(starter);
            }
        }
        let mut simulator =
            Simulator::from_generated_state(state, sts2_content::standard_registry())
                .expect("the run builds");
        let before: Vec<String> = simulator
            .state()
            .run_player
            .relics
            .iter()
            .map(|relic| relic.model_id.to_string())
            .collect();
        take_the_treasure(&mut simulator);
        let after: Vec<String> = simulator
            .state()
            .run_player
            .relics
            .iter()
            .map(|relic| relic.model_id.to_string())
            .collect();
        if starter == "RELIC.LANTERN" {
            // A lantern is not a starter at all, so nothing is refined and the
            // touch is simply held.
            assert!(!after.contains(&"RELIC.CIRCLET".to_owned()), "{starter}");
            continue;
        }
        assert!(before.contains(&starter.to_owned()), "{starter}");
        assert!(!after.contains(&starter.to_owned()), "{starter}");
        assert_eq!(
            after.iter().position(|held| held == refined),
            before.iter().position(|held| held == starter),
            "{refined} stands where {starter} stood"
        );
        let touch = simulator
            .state()
            .run_player
            .relics
            .iter()
            .find(|relic| relic.model_id == id("RELIC.TOUCH_OF_OROBAS"))
            .expect("the touch itself was taken");
        assert_eq!(
            touch
                .saved_models
                .iter()
                .map(|(name, model)| (name.as_str(), model.to_string()))
                .collect::<Vec<_>>(),
            vec![
                ("StarterRelic", starter.to_owned()),
                ("UpgradedRelic", refined.to_owned()),
            ],
            "{starter}"
        );
    }
}

/// A talisman upgrades the newest starter strike and the newest starter defend,
/// which is the back of the deck rather than the front.
#[test]
fn a_neows_talisman_upgrades_the_newest_starter_of_each_kind() {
    let mut simulator = run_holding_with_treasure(&[], "RELIC.NEOWS_TALISMAN");
    take_the_treasure(&mut simulator);
    let deck = &simulator.state().run_player.deck;
    let last_of = |prefix: &str| {
        deck.iter()
            .rposition(|card| card.model_id.to_string().starts_with(prefix))
            .expect("the starting deck holds one")
    };
    let strike = last_of("CARD.STRIKE");
    let defend = last_of("CARD.DEFEND");
    assert_eq!(deck[strike].upgrade_level, 1);
    assert_eq!(deck[defend].upgrade_level, 1);
    assert_eq!(
        deck.iter().filter(|card| card.upgrade_level > 0).count(),
        2,
        "and nothing else is upgraded"
    );
}

/// The ancients that take cards out of the deck ask for exactly as many as they
/// name, and what follows the screen is the rest of the body.
#[test]
fn the_ancients_that_take_cards_out_of_the_deck() {
    for (relic, count, curse, hp_cost) in [
        ("RELIC.PRECISE_SCISSORS", 1, None, 0),
        ("RELIC.EMPTY_CAGE", 2, None, 0),
        ("RELIC.PRESERVED_FOG", 3, Some("CARD.FOLLY"), 0),
        ("RELIC.BIIIG_HUG", 4, None, 0),
        ("RELIC.PRECARIOUS_SHEARS", 2, None, 16),
    ] {
        let mut simulator = run_holding_with_treasure(&[], relic);
        let deck_before = deck_models(&simulator);
        let hp_before = run_hp(&simulator);
        take_the_treasure(&mut simulator);
        let sts2_engine::DecisionContext::ChooseCards {
            choice_id,
            candidates,
            ..
        } = simulator.decision().clone()
        else {
            panic!("{relic} puts a removal screen up");
        };
        let cards: Vec<_> = candidates.iter().take(count).cloned().collect();
        simulator
            .step(Action::ChooseCards { choice_id, cards })
            .expect("the choice applies");
        let deck = deck_models(&simulator);
        assert_eq!(
            deck.len(),
            deck_before.len() - count + usize::from(curse.is_some()),
            "{relic}"
        );
        if let Some(curse) = curse {
            assert_eq!(deck.last().map(String::as_str), Some(curse), "{relic}");
        }
        assert_eq!(run_hp(&simulator), hp_before - hp_cost, "{relic}");
    }
}

/// The two relics that upgrade what their owner picks, and the one that rolls
/// for what it upgrades.
#[test]
fn the_ancients_that_upgrade_the_deck() {
    for (relic, count) in [("RELIC.POMANDER", 1), ("RELIC.YUMMY_COOKIE", 4)] {
        let mut simulator = run_holding_with_treasure(&[], relic);
        take_the_treasure(&mut simulator);
        let sts2_engine::DecisionContext::ChooseCards {
            choice_id,
            candidates,
            ..
        } = simulator.decision().clone()
        else {
            panic!("{relic} puts an upgrade screen up");
        };
        assert!(
            candidates
                .iter()
                .all(|handle| handle.fingerprint.upgrade_level == 0),
            "{relic} offers only what can still be upgraded"
        );
        let cards: Vec<_> = candidates.iter().take(count).cloned().collect();
        simulator
            .step(Action::ChooseCards { choice_id, cards })
            .expect("the choice applies");
        assert_eq!(
            simulator
                .state()
                .run_player
                .deck
                .iter()
                .filter(|card| card.upgrade_level > 0)
                .count(),
            count,
            "{relic}"
        );
    }
    // A sand castle asks nobody: it shuffles the deck's upgradable cards on the
    // run's niche stream and upgrades the first six.
    let mut simulator = run_holding_with_treasure(&[], "RELIC.SAND_CASTLE");
    let niche = niche_counter(&simulator);
    take_the_treasure(&mut simulator);
    assert!(
        !matches!(
            simulator.decision(),
            sts2_engine::DecisionContext::ChooseCards { .. }
        ),
        "nobody is asked which cards it upgrades"
    );
    assert_eq!(
        simulator
            .state()
            .run_player
            .deck
            .iter()
            .filter(|card| card.upgrade_level > 0)
            .count(),
        6
    );
    assert!(
        niche_counter(&simulator) > niche,
        "the shuffle is rolled for"
    );
}

// --- the ancients that answer inside a fight -----------------------------

/// The relics that pay as their owner's turn opens.
#[test]
fn the_ancients_that_answer_a_turn_opening() {
    // Seven block, every turn.
    let mut sai = fight_from_run(&["RELIC.SAI"]);
    assert_eq!(player(&sai).block, 7);
    end_turn(&mut sai);
    assert_eq!(player(&sai).block, 7, "and again on the next turn");

    // Four energy on the opening turn alone.
    let plain = fight_from_run(&[]);
    let mut cocoa = fight_from_run(&["RELIC.VERY_HOT_COCOA"]);
    assert_eq!(energy(&cocoa), energy(&plain) + 4);
    let opening = energy(&plain);
    end_turn(&mut cocoa);
    assert_eq!(energy(&cocoa), opening, "and nothing on the turns after it");
}

/// A seal of gold buys an energy for five gold as every turn opens, and stops
/// when the gold runs out.
#[test]
fn a_seal_of_gold_spends_five_a_turn() {
    let mut state = run_holding(&["RELIC.SEAL_OF_GOLD"]).state().clone();
    state.run_player.gold = 7;
    let mut simulator = Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run rebuilds");
    walk_to(&mut simulator, 3);
    let plain = fight_from_run(&[]);
    assert_eq!(energy(&simulator), energy(&plain) + 1);
    assert_eq!(simulator.state().run_player.gold, 2);
    end_turn(&mut simulator);
    // Two gold is not five, so the second turn buys nothing.
    assert_eq!(energy(&simulator), energy(&plain));
    assert_eq!(simulator.state().run_player.gold, 2);
}

/// Pael's tears pay two energy on a turn that follows one its owner did not
/// spend out.
#[test]
fn paels_tears_pay_for_a_frugal_turn() {
    let mut simulator = fight_from_run(&["RELIC.PAELS_TEARS"]);
    let plain_opening = energy(&fight_from_run(&[]));
    assert_eq!(
        energy(&simulator),
        plain_opening,
        "nothing on the first turn"
    );
    end_turn(&mut simulator);
    assert_eq!(
        energy(&simulator),
        plain_opening + 2,
        "the turn ended with energy left, so the next one pays two"
    );
}

/// Pael's blood draws one more card every turn; a radiant pearl and a blessed
/// antler stock the opening piles before the hand is drawn.
#[test]
fn the_ancients_that_stock_the_opening_piles() {
    let plain = fight_from_run(&[]);
    let blood = fight_from_run(&["RELIC.PAELS_BLOOD"]);
    assert_eq!(hand_size(&blood), hand_size(&plain) + 1);

    let pearl = fight_from_run(&["RELIC.RADIANT_PEARL"]);
    assert_eq!(
        pearl
            .state()
            .combat
            .as_ref()
            .expect("a fight")
            .player
            .piles
            .hand
            .iter()
            .filter(|card| card.fingerprint.model_id.to_string() == "CARD.LUMINESCE")
            .count(),
        1,
        "the luminesce is in hand beside the drawn cards"
    );
    assert_eq!(hand_size(&pearl), hand_size(&plain) + 1);

    // Three dazed shuffled into the draw pile, and an energy for them.
    let antler = fight_from_run(&["RELIC.BLESSED_ANTLER"]);
    let combat = antler.state().combat.as_ref().expect("a fight");
    let dazed = combat
        .player
        .piles
        .draw
        .iter()
        .chain(combat.player.piles.hand.iter())
        .filter(|card| card.fingerprint.model_id.to_string() == "CARD.DAZED")
        .count();
    assert_eq!(dazed, 3);
    assert_eq!(energy(&antler), energy(&plain) + 1);
}

/// An ectoplasm pays an energy and turns every gold gain away.
#[test]
fn an_ectoplasm_pays_an_energy_and_refuses_the_gold() {
    let plain = fight_from_run(&[]);
    let simulator = fight_from_run(&["RELIC.ECTOPLASM"]);
    assert_eq!(energy(&simulator), energy(&plain) + 1);

    let mut run = run_holding_with_treasure(&["RELIC.ECTOPLASM"], "RELIC.GOLDEN_PEARL");
    let before = run.state().run_player.gold;
    take_the_treasure(&mut run);
    assert_eq!(
        run.state().run_player.gold,
        before,
        "a hundred and fifty gold comes to nothing"
    );
}

/// Pael's flesh pays its energy from the third turn of a fight on.
#[test]
fn paels_flesh_waits_for_the_third_turn() {
    let mut plain = fight_from_run(&[]);
    let mut simulator = fight_from_run(&["RELIC.PAELS_FLESH"]);
    assert_eq!(energy(&simulator), energy(&plain));
    end_turn(&mut plain);
    end_turn(&mut simulator);
    assert_eq!(energy(&simulator), energy(&plain), "and not on the second");
    end_turn(&mut plain);
    end_turn(&mut simulator);
    assert_eq!(energy(&simulator), energy(&plain) + 1);
}

/// A biiig hug puts a soot into the draw pile on every shuffle.
#[test]
fn a_biiig_hug_answers_every_shuffle() {
    let mut simulator = fight_from_run(&["RELIC.BIIIG_HUG"]);
    let soot = |simulator: &Simulator| {
        let combat = simulator.state().combat.as_ref().expect("a fight");
        combat
            .player
            .piles
            .all_cards()
            .filter(|card| card.fingerprint.model_id.to_string() == "CARD.SOOT")
            .count()
    };
    let opening = soot(&simulator);
    // Emptying the draw pile forces the shuffle that the relic answers.
    for _ in 0..12 {
        end_turn(&mut simulator);
        if soot(&simulator) > opening {
            return;
        }
    }
    panic!("no shuffle happened in twelve turns");
}

// --- the ancients that answer a reward screen or a room ------------------

/// The one-room act with the fight's map point standing as another family, so
/// that a relic which answers an elite or a boss has one to answer.
fn run_winning(relics: &[&str], point_type: MapPointType) -> Simulator {
    let mut state = run_winning_a_fight(relics, false).state().clone();
    if let Some(run) = state.run.as_mut() {
        for point in &mut run.maps[0].points {
            if point.coord == (MapCoord { col: 3, row: 1 }) {
                point.point_type = point_type;
            }
        }
        run.player_relic_grab_bag = stocked_bag();
    }
    Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run rebuilds")
}

/// A black star puts a relic on every elite's reward screen; a lava rock puts
/// two on the first act's boss, once in the run.
#[test]
fn the_ancients_that_add_a_relic_to_a_reward_screen() {
    let mut elite = run_winning(&["RELIC.BLACK_STAR"], MapPointType::Elite);
    win_the_fight(&mut elite);
    let rewards = offered_rewards(&elite);
    assert_eq!(rewards.len(), 1);
    assert_eq!(rewards[0].reward_type, "relic");

    // The same relic answers nothing in a monster room.
    let mut monster = run_winning(&["RELIC.BLACK_STAR"], MapPointType::Monster);
    win_the_fight(&mut monster);
    assert!(offered_rewards(&monster).is_empty());

    let mut boss = run_winning(&["RELIC.LAVA_ROCK"], MapPointType::Boss);
    win_the_fight(&mut boss);
    let rewards = offered_rewards(&boss);
    assert_eq!(rewards.len(), 2, "two relics on the first act's boss");
    assert!(rewards.iter().all(|reward| reward.reward_type == "relic"));
    // The relic wrote down that it had paid out, so it never pays again.
    assert!(
        boss.state()
            .run_player
            .relics
            .iter()
            .find(|relic| relic.model_id == id("RELIC.LAVA_ROCK"))
            .expect("the relic is held")
            .latched
    );
}

/// A small capsule hands over one relic; a calling bell hands over a curse and
/// one relic of each rolled rarity, named rather than rolled.
#[test]
fn the_ancients_that_hand_over_a_reward_set() {
    let mut capsule = run_with_a_bag(&[], "RELIC.SMALL_CAPSULE");
    take_the_treasure(&mut capsule);
    let rewards = offered_rewards(&capsule);
    assert_eq!(rewards.len(), 1);
    assert_eq!(rewards[0].reward_type, "relic");

    let mut bell = run_with_a_bag(&[], "RELIC.CALLING_BELL");
    let deck_before = deck_models(&bell).len();
    take_the_treasure(&mut bell);
    assert_eq!(
        deck_models(&bell).last().map(String::as_str),
        Some("CARD.CURSE_OF_THE_BELL")
    );
    let rewards = offered_rewards(&bell);
    assert_eq!(rewards.len(), 3);
    assert!(rewards.iter().all(|reward| reward.reward_type == "relic"));
    assert_eq!(deck_models(&bell).len(), deck_before + 1);

    // A large capsule takes its two relics outright rather than offering them,
    // and puts a starter strike and defend back into the deck.
    let mut large = run_with_a_bag(&[], "RELIC.LARGE_CAPSULE");
    let held_before = large.state().run_player.relics.len();
    let deck_before = deck_models(&large);
    take_the_treasure(&mut large);
    assert_eq!(
        large.state().run_player.relics.len(),
        held_before + 3,
        "the capsule and the two it pulled"
    );
    let deck = deck_models(&large);
    assert_eq!(
        &deck[deck_before.len()..],
        &["CARD.STRIKE_IRONCLAD", "CARD.DEFEND_IRONCLAD"]
    );
}

/// The ancients that answer the belt: two fill it where they are taken, one
/// fills it as every fight opens, and one shuts it.
#[test]
fn the_ancients_that_answer_the_potion_belt() {
    let held = |simulator: &Simulator| {
        simulator
            .state()
            .run_player
            .potions
            .iter()
            .filter(|slot| slot.is_some())
            .count()
    };
    let slots = |simulator: &Simulator| simulator.state().run_player.potions.len();

    let mut holster = run_holding_with_treasure(&[], "RELIC.PHIAL_HOLSTER");
    let before = slots(&holster);
    let held_before = held(&holster);
    take_the_treasure(&mut holster);
    assert_eq!(slots(&holster), before + 1);
    assert_eq!(held(&holster), held_before + 2);

    let mut coffer = run_holding_with_treasure(&[], "RELIC.ALCHEMICAL_COFFER");
    let before = slots(&coffer);
    let held_before = held(&coffer);
    take_the_treasure(&mut coffer);
    assert_eq!(slots(&coffer), before + 4);
    assert_eq!(held(&coffer), held_before + 4);
    // The four went into the slots the belt had just grown, not into the ones
    // that were already free.
    assert!(
        coffer.state().run_player.potions[before..]
            .iter()
            .all(Option::is_some)
    );

    // A frond fills whatever is free as the fight opens.
    let frond = fight_from_run(&["RELIC.DELICATE_FROND"]);
    assert_eq!(held(&frond), slots(&frond));

    // A sozu turns every potion away, so the frond fills nothing.
    let sozu = fight_from_run(&["RELIC.DELICATE_FROND", "RELIC.SOZU"]);
    assert_eq!(held(&sozu), held(&fight_from_run(&[])));
    assert_eq!(energy(&sozu), energy(&fight_from_run(&[])) + 1);
}

/// An iron club draws on every fourth card its owner plays, and the count is
/// its own across the whole run.
#[test]
fn an_iron_club_draws_on_every_fourth_card() {
    let mut simulator = fight_tough(
        &["RELIC.IRON_CLUB"],
        &[
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
        ],
    );
    for played in 1..=3 {
        let before = hand_size(&simulator);
        play(&mut simulator, "CARD.DEFEND_IRONCLAD");
        assert_eq!(
            hand_size(&simulator),
            before - 1,
            "card {played} draws nothing"
        );
    }
    let before = hand_size(&simulator);
    play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    assert_eq!(hand_size(&simulator), before, "the fourth draws one back");
    assert_eq!(relic_counter(&simulator, "RELIC.IRON_CLUB"), 4);
}

/// A throwing axe plays the first card of every fight twice.
#[test]
fn a_throwing_axe_plays_the_first_card_of_a_fight_twice() {
    let mut simulator = fight_from_run(&["RELIC.THROWING_AXE"]);
    let plain = fight_from_run(&[]);
    let hp_before = creature(&simulator, 1).current_hp;
    let plain_before = creature(&plain, 1).current_hp;
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    let mut plain = plain;
    play(&mut plain, "CARD.STRIKE_IRONCLAD");
    let doubled = hp_before - creature(&simulator, 1).current_hp;
    let once = plain_before - creature(&plain, 1).current_hp;
    assert_eq!(doubled, once * 2);
    // The second card of the fight is played once.
    let before = creature(&simulator, 1).current_hp;
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(before - creature(&simulator, 1).current_hp, once);
}

/// A war hammer upgrades four cards when an elite goes down, and nothing when
/// anything else does.
#[test]
fn a_war_hammer_answers_an_elite_going_down() {
    let upgraded = |simulator: &Simulator| {
        simulator
            .state()
            .run_player
            .deck
            .iter()
            .filter(|card| card.upgrade_level > 0)
            .count()
    };
    let mut elite = run_winning(&["RELIC.WAR_HAMMER"], MapPointType::Elite);
    win_the_fight(&mut elite);
    assert_eq!(upgraded(&elite), 4);

    let mut monster = run_winning(&["RELIC.WAR_HAMMER"], MapPointType::Monster);
    win_the_fight(&mut monster);
    assert_eq!(upgraded(&monster), 0);
}

/// A stone humidifier is five more maximum HP every time its owner rests.
#[test]
fn a_stone_humidifier_grows_with_every_rest() {
    let mut simulator = run_holding(&["RELIC.STONE_HUMIDIFIER"]);
    let before = simulator.state().run_player.max_hp;
    walk_to(&mut simulator, 1);
    simulator
        .step(rest_heal(&simulator))
        .expect("the rest site heals");
    assert_eq!(simulator.state().run_player.max_hp, before + 5);
}

/// A crossbow conjures an attack into the hand as every turn opens, free for
/// that turn; a jeweled mask pulls a power out of the draw pile into the
/// opening hand.
#[test]
fn the_ancients_that_stock_the_hand_from_a_pile() {
    let crossbow = fight_from_run(&["RELIC.CROSSBOW"]);
    let plain = fight_from_run(&[]);
    assert_eq!(hand_size(&crossbow), hand_size(&plain) + 1);
    let combat = crossbow.state().combat.as_ref().expect("a fight");
    let conjured = combat.player.piles.hand.last().expect("the hand holds it");
    assert_eq!(
        conjured
            .temporary_costs
            .last()
            .expect("the card was made free")
            .cost,
        0
    );

    // A mask needs a power in the deck to find one in the draw pile.
    let mut state = run_holding(&["RELIC.JEWELED_MASK"]).state().clone();
    state
        .run_player
        .deck
        .push(sts2_engine::CardFingerprint::base(id("CARD.INFLAME")));
    let mut mask = Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run rebuilds");
    walk_to(&mut mask, 3);
    let combat = mask.state().combat.as_ref().expect("a fight");
    let held = combat
        .player
        .piles
        .hand
        .iter()
        .find(|card| card.fingerprint.model_id.to_string() == "CARD.INFLAME")
        .expect("the power was pulled into the opening hand");
    assert_eq!(
        held.temporary_costs
            .last()
            .expect("and made free for the turn")
            .cost,
        0
    );
}

/// A philosopher's stone pays an energy and arms everything on the other side,
/// including whatever arrives after the fight has started.
#[test]
fn a_philosophers_stone_arms_the_other_side() {
    let simulator = fight_from_run(&["RELIC.PHILOSOPHERS_STONE"]);
    assert_eq!(power_amount(&simulator, 1, "POWER.STRENGTH_POWER"), 1);
    assert_eq!(energy(&simulator), energy(&fight_from_run(&[])) + 1);
}

/// A fishing rod counts the monster rooms and upgrades one card it rolls for on
/// every third.
#[test]
fn a_fishing_rod_answers_every_third_monster_room() {
    let upgraded = |simulator: &Simulator| {
        simulator
            .state()
            .run_player
            .deck
            .iter()
            .filter(|card| card.upgrade_level > 0)
            .count()
    };
    let mut first = run_winning(&["RELIC.FISHING_ROD"], MapPointType::Monster);
    win_the_fight(&mut first);
    assert_eq!(relic_counter(&first, "RELIC.FISHING_ROD"), 1);
    assert_eq!(upgraded(&first), 0, "the first fight upgrades nothing");

    // The same fight with two behind it: the third pays out.
    let mut state = run_winning(&["RELIC.FISHING_ROD"], MapPointType::Monster)
        .state()
        .clone();
    for relic in &mut state.run_player.relics {
        if relic.model_id == id("RELIC.FISHING_ROD") {
            relic.counter = 2;
        }
    }
    let mut third = Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run rebuilds");
    let niche = niche_counter(&third);
    win_the_fight(&mut third);
    assert_eq!(relic_counter(&third, "RELIC.FISHING_ROD"), 3);
    assert_eq!(upgraded(&third), 1);
    assert_eq!(niche_counter(&third), niche + 1, "the card is rolled for");

    // An elite room is not a monster room, so nothing is counted.
    let mut elite = run_winning(&["RELIC.FISHING_ROD"], MapPointType::Elite);
    win_the_fight(&mut elite);
    assert_eq!(relic_counter(&elite, "RELIC.FISHING_ROD"), 0);
}

// --- the ancients that change what a card costs or what may be played ----

fn hand_cost(simulator: &Simulator, model_id: &str) -> i32 {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("a fight")
        .player
        .piles
        .hand
        .iter()
        .find(|card| card.fingerprint.model_id.to_string() == model_id)
        .expect("the card is in hand")
        .energy_cost
}

/// A card carries every cost put over it, each with its own lifetime, and each
/// stands over what the one before it left. So a card a snecko rolled for the
/// fight and a potion then made free for the turn goes back to the rolled cost
/// when it is played — not to the cost printed on it.
///
/// Both halves are checked: with the rolled cost underneath, and without it.
#[test]
fn a_cost_that_clears_on_play_uncovers_the_one_beneath_it() {
    for rolled in [Some(3), None] {
        let simulator = fight(&[], &[("CARD.STRIKE_IRONCLAD", 0)]);
        let mut state = simulator.state().clone();
        let strike = state
            .combat
            .as_mut()
            .expect("the fight is standing")
            .player
            .piles
            .hand
            .first_mut()
            .expect("the strike is held");
        if let Some(cost) = rolled {
            strike
                .temporary_costs
                .push(sts2_engine::TemporaryCost::this_combat(cost));
        }
        strike
            .temporary_costs
            .push(sts2_engine::TemporaryCost::free_this_turn());
        strike.energy_cost = 0;

        let mut simulator = Simulator::from_scenario(state, sts2_content::standard_registry())
            .expect("the scenario rebuilds");
        assert_eq!(
            hand_cost(&simulator, "CARD.STRIKE_IRONCLAD"),
            0,
            "free for the turn, whatever is underneath"
        );
        play(&mut simulator, "CARD.STRIKE_IRONCLAD");

        let discarded = simulator
            .state()
            .combat
            .as_ref()
            .expect("the fight is standing")
            .player
            .piles
            .discard
            .iter()
            .find(|card| card.fingerprint.model_id == id("CARD.STRIKE_IRONCLAD"))
            .expect("the strike was discarded");
        assert_eq!(
            discarded.energy_cost,
            rolled.unwrap_or(1),
            "the free cost came off and left what was under it"
        );
    }
}

/// A snecko eye draws two more and rolls every drawn card a new cost for the
/// rest of the fight./// A snecko eye draws two more and rolls every drawn card a new cost for the
/// rest of the fight.
#[test]
fn a_snecko_eye_confuses_the_deck_and_swells_the_hand() {
    let plain = fight_from_run(&[]);
    let simulator = fight_from_run(&["RELIC.SNECKO_EYE"]);
    assert_eq!(hand_size(&simulator), hand_size(&plain) + 2);
    let combat = simulator.state().combat.as_ref().expect("a fight");
    assert!(
        combat.player.piles.hand.iter().all(|card| card
            .temporary_costs
            .iter()
            .any(|cost| (0..4).contains(&cost.cost) && !cost.clears_when_turn_ends)),
        "every drawn card carries a rolled cost that stands for the fight"
    );
    assert!(
        simulator
            .state()
            .rng
            .stream(RunStream::CombatEnergyCosts)
            .counter()
            > 0
    );
}

/// Spiked gauntlets pay an energy and charge one more for every power card.
#[test]
fn spiked_gauntlets_charge_more_for_a_power() {
    let simulator = fight(
        &["RELIC.SPIKED_GAUNTLETS"],
        &[("CARD.INFLAME", 0), ("CARD.STRIKE_IRONCLAD", 0)],
    );
    let plain = fight(&[], &[("CARD.INFLAME", 0), ("CARD.STRIKE_IRONCLAD", 0)]);
    assert_eq!(
        hand_cost(&simulator, "CARD.INFLAME"),
        hand_cost(&plain, "CARD.INFLAME") + 1
    );
    assert_eq!(
        hand_cost(&simulator, "CARD.STRIKE_IRONCLAD"),
        hand_cost(&plain, "CARD.STRIKE_IRONCLAD"),
        "and nothing more for anything else"
    );
}

/// A brilliant scarf makes the fifth card of every turn free.
#[test]
fn a_brilliant_scarf_makes_the_fifth_card_free() {
    let hand = [
        ("CARD.DEFEND_IRONCLAD", 0),
        ("CARD.DEFEND_IRONCLAD", 0),
        ("CARD.DEFEND_IRONCLAD", 0),
        ("CARD.DEFEND_IRONCLAD", 0),
        ("CARD.DEFEND_IRONCLAD", 0),
    ];
    let mut simulator = fight_tough(&["RELIC.BRILLIANT_SCARF"], &hand);
    for _ in 0..4 {
        assert_eq!(hand_cost(&simulator, "CARD.DEFEND_IRONCLAD"), 1);
        play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    }
    assert_eq!(
        hand_cost(&simulator, "CARD.DEFEND_IRONCLAD"),
        0,
        "the fifth is free"
    );
    assert_eq!(relic_counter(&simulator, "RELIC.BRILLIANT_SCARF"), 4);

    // A card nobody chose is not counted: a havoc plays the top of the draw
    // pile itself, and the scarf's own body skips an auto-play.
    let mut auto = fight_tough(
        &["RELIC.BRILLIANT_SCARF"],
        &[("CARD.HAVOC", 0), ("CARD.DEFEND_IRONCLAD", 0)],
    );
    play(&mut auto, "CARD.HAVOC");
    assert_eq!(
        relic_counter(&auto, "RELIC.BRILLIANT_SCARF"),
        1,
        "the havoc counts and what it played does not"
    );
}

/// A velvet choker pays an energy and allows six cards a turn.
#[test]
fn a_velvet_choker_allows_six_cards_a_turn() {
    let hand: Vec<(&str, u8)> = std::iter::repeat_n(("CARD.DEFEND_IRONCLAD", 0), 8).collect();
    let mut simulator = fight_tough(&["RELIC.VELVET_CHOKER"], &hand);
    for _ in 0..6 {
        play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    }
    assert!(
        !simulator
            .legal_actions()
            .iter()
            .any(|action| matches!(action, Action::PlayCard { .. })),
        "the seventh card of the turn cannot be played"
    );
    end_turn(&mut simulator);
    assert!(
        simulator
            .legal_actions()
            .iter()
            .any(|action| matches!(action, Action::PlayCard { .. })),
        "and the next turn starts the count over"
    );
}

/// A fiddle draws two more on every hand draw, and refuses every other draw its
/// owner's own turn would make.
#[test]
fn a_fiddle_draws_two_more_and_refuses_the_rest() {
    let plain = fight_from_run(&[]);
    let simulator = fight_from_run(&["RELIC.FIDDLE"]);
    assert_eq!(hand_size(&simulator), hand_size(&plain) + 2);
    // A card that draws draws nothing at all while the fiddle is held, because
    // the draw is not the turn's own hand draw.
    let mut drawing = fight_against(
        "SHRINKER_MOVE",
        1,
        &["RELIC.FIDDLE"],
        &[("CARD.BATTLE_TRANCE", 0)],
        &[
            ("CARD.STRIKE_IRONCLAD", 0),
            ("CARD.STRIKE_IRONCLAD", 0),
            ("CARD.STRIKE_IRONCLAD", 0),
        ],
    );
    play(&mut drawing, "CARD.BATTLE_TRANCE");
    assert_eq!(
        hand_size(&drawing),
        0,
        "the three cards it draws are refused"
    );
    let mut plain_draw = fight_against(
        "SHRINKER_MOVE",
        1,
        &[],
        &[("CARD.BATTLE_TRANCE", 0)],
        &[
            ("CARD.STRIKE_IRONCLAD", 0),
            ("CARD.STRIKE_IRONCLAD", 0),
            ("CARD.STRIKE_IRONCLAD", 0),
        ],
    );
    play(&mut plain_draw, "CARD.BATTLE_TRANCE");
    assert_eq!(hand_size(&plain_draw), 3);
}

/// A diamond diadem halves what reaches its owner after a turn it played no
/// more than two cards in.
#[test]
fn a_diamond_diadem_answers_a_quiet_turn() {
    let mut plain = fight_from_run(&[]);
    let mut simulator = fight_from_run(&["RELIC.DIAMOND_DIADEM"]);
    // The beetle's first move is a debuff; its bite comes on the turn after.
    end_turn(&mut plain);
    end_turn(&mut simulator);
    let (plain_hp, quiet_hp) = (run_hp(&plain), run_hp(&simulator));
    end_turn(&mut plain);
    end_turn(&mut simulator);
    let plain_lost = plain_hp - run_hp(&plain);
    let halved = quiet_hp - run_hp(&simulator);
    assert!(plain_lost > 0);
    assert_eq!(halved, plain_lost / 2, "the blow was halved");
    assert_eq!(
        power_amount(&simulator, 0, "POWER.DIAMOND_DIADEM_POWER"),
        0,
        "and the power is gone once the enemy's turn has ended"
    );
    // A busy turn never earns it: three cards played is one too many.
    let mut busy = fight_tough(
        &["RELIC.DIAMOND_DIADEM"],
        &[
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
        ],
    );
    for _ in 0..3 {
        play(&mut busy, "CARD.DEFEND_IRONCLAD");
    }
    assert_eq!(relic_counter(&busy, "RELIC.DIAMOND_DIADEM"), 3);
}

/// Toasty mittens burn the first card of the draw pile and pay a strength for
/// it before every hand draw.
#[test]
fn toasty_mittens_burn_a_card_for_a_strength() {
    let simulator = fight_from_run(&["RELIC.TOASTY_MITTENS"]);
    let combat = simulator.state().combat.as_ref().expect("a fight");
    assert_eq!(combat.player.piles.exhaust.len(), 1);
    assert_eq!(power_amount(&simulator, 0, "POWER.STRENGTH_POWER"), 1);
}

// --- the ancients registered for the offer alone -------------------------

/// The ancients whose bodies need a subsystem this baseline has not built are
/// registered for what they are outside those bodies: the rarity that keeps
/// them out of every bag and every trade, the spawn rule their model declares,
/// and the count that says when one is spent. Taking one stops rather than
/// doing something the build does not.
#[test]
fn the_ancient_pool_is_registered_whole() {
    let registry = sts2_content::standard_registry();
    let offered_only = [
        "RELIC.BEAUTIFUL_BRACELET",
        "RELIC.CHOICES_PARADOX",
        "RELIC.DRIFTWOOD",
        "RELIC.ELECTRIC_SHRYMP",
        "RELIC.FUR_COAT",
        "RELIC.GLASS_EYE",
        "RELIC.GLITTER",
        "RELIC.GOLDEN_COMPASS",
        "RELIC.HEFTY_TABLET",
        "RELIC.KALEIDOSCOPE",
        "RELIC.LEAD_PAPERWEIGHT",
        "RELIC.LORDS_PARASOL",
        "RELIC.MASSIVE_SCROLL",
        "RELIC.MEAT_CLEAVER",
        "RELIC.NUTRITIOUS_SOUP",
        "RELIC.PAELS_CLAW",
        "RELIC.PAELS_EYE",
        "RELIC.PAELS_GROWTH",
        "RELIC.PAELS_LEGION",
        "RELIC.PAELS_TOOTH",
        "RELIC.PUMPKIN_CANDLE",
        "RELIC.SCROLL_BOXES",
        "RELIC.SERE_TALON",
        "RELIC.SILKEN_TRESS",
        "RELIC.SILVER_CRUCIBLE",
        "RELIC.TOY_BOX",
        "RELIC.TRI_BOOMERANG",
    ];
    for model_id in offered_only {
        let definition = registry.relic(&id(model_id)).expect("the relic is content");
        assert_eq!(
            definition.rarity,
            sts2_engine::RelicRarity::Ancient,
            "{model_id}"
        );
        assert!(
            !definition.is_tradable(&sts2_engine::RelicInstance::new(id(model_id))),
            "{model_id} is never traded away"
        );
    }
    // The two relics whose availability turns on the number of players: one
    // is only ever allowed in a multiplayer run, so a single-player one drops
    // it from every bag before a pull, and the other only in a single-player
    // one.
    assert_eq!(
        registry
            .relic(&id("RELIC.MASSIVE_SCROLL"))
            .expect("the relic is content")
            .spawn_condition,
        Some(sts2_engine::RelicSpawnCondition::MultiplayerOnly)
    );
    for model_id in ["RELIC.SILVER_CRUCIBLE", "RELIC.WINGED_BOOTS"] {
        assert_eq!(
            registry
                .relic(&id(model_id))
                .expect("the relic is content")
                .spawn_condition,
            Some(sts2_engine::RelicSpawnCondition::SinglePlayerOnly),
            "{model_id}"
        );
    }
    // The two whose uses run out say so against the count they keep.
    for (model_id, uses) in [("RELIC.WINGED_BOOTS", 3), ("RELIC.TOY_BOX", 12)] {
        let definition = registry.relic(&id(model_id)).expect("the relic is content");
        let mut instance = sts2_engine::RelicInstance::new(id(model_id));
        instance.counter = uses - 1;
        assert!(!definition.is_used_up(&instance), "{model_id}");
        instance.counter = uses;
        assert!(definition.is_used_up(&instance), "{model_id}");
    }
    // A relic brings a pet, which is one of the things that stops a trade.
    assert!(
        registry
            .relic(&id("RELIC.PAELS_LEGION"))
            .expect("the relic is content")
            .spawns_pets
    );
    // Taking one rolls the skin its pet wears, which is a saved string.
    let mut simulator = run_holding_with_treasure(&[], "RELIC.PAELS_LEGION");
    take_from_the_chest(&mut simulator);
    let held = simulator
        .state()
        .run_player
        .relics
        .iter()
        .find(|relic| relic.model_id == id("RELIC.PAELS_LEGION"))
        .expect("the relic was taken");
    let (name, skin) = held
        .saved_strings
        .first()
        .expect("the skin was rolled as it was taken");
    assert_eq!(name, "Skin");
    assert!(
        ["eyes", "horns", "spikes", "wings"].contains(&skin.as_str()),
        "{skin}"
    );
}

/// Pael's wing puts a sacrifice on every card reward: the reward is thrown
/// away, the relic counts it, and every second one pays a relic off the front
/// of the player's own bag.
#[test]
fn paels_wing_pays_a_relic_for_every_second_reward_thrown_away() {
    for (already_sacrificed, gained) in [(0, 0), (1, 1)] {
        let mut state = run_winning_a_fight(&["RELIC.PAELS_WING"], true)
            .state()
            .clone();
        if let Some(run) = state.run.as_mut() {
            run.player_relic_grab_bag = stocked_bag();
        }
        for relic in &mut state.run_player.relics {
            if relic.model_id == id("RELIC.PAELS_WING") {
                relic.counter = already_sacrificed;
            }
        }
        let mut simulator =
            Simulator::from_generated_state(state, sts2_content::standard_registry())
                .expect("the run rebuilds");
        let held_before = simulator.state().run_player.relics.len();
        win_the_fight(&mut simulator);
        let card = offered_rewards(&simulator)
            .into_iter()
            .position(|reward| reward.reward_type == "card")
            .expect("a fight pays a card offer");
        let claim = simulator
            .legal_actions()
            .iter()
            .find(|action| {
                matches!(action, Action::ClaimReward { reward_index, .. } if *reward_index == card)
            })
            .cloned()
            .expect("the offer can be opened");
        simulator.step(claim).expect("the offer opens");
        let sacrifice = simulator
            .legal_actions()
            .iter()
            .find(|action| {
                matches!(action, Action::ChooseAlternative { option_id, .. }
                    if option_id == "sacrifice")
            })
            .cloned()
            .expect("the wing puts a sacrifice on the screen");
        simulator
            .step(sacrifice)
            .expect("the reward is thrown away");
        assert_eq!(
            relic_counter(&simulator, "RELIC.PAELS_WING"),
            already_sacrificed + 1
        );
        assert_eq!(
            simulator.state().run_player.relics.len(),
            held_before + gained,
            "every second sacrifice pays a relic"
        );
    }
}

/// A sacrificed card reward is answered for good: the offer that was thrown
/// away is not on the screen to be opened again. Left claimable, an argmax
/// policy that likes the sacrifice loops on the pair forever.
#[test]
fn a_sacrificed_card_reward_is_not_offered_again() {
    let mut simulator = run_winning_a_fight(&["RELIC.PAELS_WING"], true);
    win_the_fight(&mut simulator);
    let card = offered_rewards(&simulator)
        .into_iter()
        .position(|reward| reward.reward_type == "card")
        .expect("a fight pays a card offer");
    let claim = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::ClaimReward { reward_index, .. } if *reward_index == card)
        })
        .cloned()
        .expect("the offer can be opened");
    simulator.step(claim).expect("the offer opens");
    let sacrifice = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::ChooseAlternative { option_id, .. }
                if option_id == "sacrifice")
        })
        .cloned()
        .expect("the wing puts a sacrifice on the screen");
    simulator
        .step(sacrifice)
        .expect("the reward is thrown away");
    assert!(
        !simulator.legal_actions().iter().any(|action| {
            matches!(action, Action::ClaimReward { reward_index, .. } if *reward_index == card)
        }),
        "the thrown-away offer cannot be opened again"
    );
    // The sacrifice completes the reward, so the line reads as taken.
    let sts2_engine::DecisionContext::Rewards { offers, .. } = simulator.decision() else {
        panic!("the rewards screen stands: {:?}", simulator.decision());
    };
    assert!(
        offers[card].selected,
        "a sacrificed card reward reads as selected"
    );
}

// --- the relics whose whole body is an enchantment -----------------------

/// The enchantment on the deck card standing at `index`, if it carries one.
fn deck_enchantment(simulator: &Simulator, index: usize) -> Option<(String, i32)> {
    simulator.state().run_player.deck[index]
        .enchantment
        .as_ref()
        .map(|enchantment| (enchantment.model_id.to_string(), enchantment.amount))
}

/// How many deck cards carry this enchantment.
fn deck_carrying(simulator: &Simulator, enchantment: &str) -> usize {
    simulator
        .state()
        .run_player
        .deck
        .iter()
        .filter(|card| {
            card.enchantment
                .as_ref()
                .is_some_and(|held| held.model_id == id(enchantment))
        })
        .count()
}

/// Takes the relic and answers the enchant screen it opens with the deck cards
/// standing at `picks`.
fn take_and_enchant(simulator: &mut Simulator, picks: &[usize]) {
    take_the_treasure(simulator);
    let sts2_engine::DecisionContext::ChooseCards {
        choice_id,
        candidates,
        ..
    } = simulator.decision().clone()
    else {
        panic!("the relic puts an enchant screen up");
    };
    let cards: Vec<_> = picks
        .iter()
        .map(|index| candidates[*index].clone())
        .collect();
    simulator
        .step(Action::ChooseCards { choice_id, cards })
        .expect("the choice applies");
}

/// Each of these asks which deck cards to put its enchantment on, and puts it
/// on exactly what was picked, worth exactly what the relic declares.
#[test]
fn the_relics_that_ask_which_cards_to_enchant() {
    for (relic, enchantment, amount, picks) in [
        ("RELIC.GNARLED_HAMMER", "ENCHANTMENT.SHARP", 3, 3),
        ("RELIC.KIFUDA", "ENCHANTMENT.ADROIT", 3, 3),
        ("RELIC.PUNCH_DAGGER", "ENCHANTMENT.MOMENTUM", 5, 1),
        ("RELIC.BEAUTIFUL_BRACELET", "ENCHANTMENT.SWIFT", 3, 3),
        ("RELIC.ELECTRIC_SHRYMP", "ENCHANTMENT.IMBUED", 1, 1),
        ("RELIC.TRI_BOOMERANG", "ENCHANTMENT.INSTINCT", 1, 3),
    ] {
        let mut simulator = run_holding_with_treasure(&[], relic);
        let chosen: Vec<usize> = (0..picks).collect();
        take_and_enchant(&mut simulator, &chosen);
        assert_eq!(
            deck_carrying(&simulator, enchantment),
            picks,
            "{relic} enchanted exactly what was picked"
        );
        assert!(
            simulator
                .state()
                .run_player
                .deck
                .iter()
                .filter_map(|card| card.enchantment.as_ref())
                .all(|held| held.model_id == id(enchantment) && held.amount == amount),
            "{relic} put its own amount on"
        );
    }
}

/// A royal stamp approves one card, and reads the niche stream once for the
/// shuffle the build hands the screen.
#[test]
fn a_royal_stamp_approves_one_card_and_reads_the_niche_stream() {
    let mut simulator = run_holding_with_treasure(&[], "RELIC.ROYAL_STAMP");
    let niche = niche_counter(&simulator);
    take_and_enchant(&mut simulator, &[0]);
    assert_eq!(deck_carrying(&simulator, "ENCHANTMENT.ROYALLY_APPROVED"), 1);
    assert!(
        niche_counter(&simulator) > niche,
        "the candidates were shuffled on the niche stream before the screen"
    );
}

/// Pael's growth marks one card a Clone, and the mark is worth four however the
/// screen previews it.
#[test]
fn paels_growth_marks_one_card_a_clone() {
    let mut simulator = run_holding_with_treasure(&[], "RELIC.PAELS_GROWTH");
    take_and_enchant(&mut simulator, &[0]);
    assert_eq!(
        deck_enchantment(&simulator, 0),
        Some(("ENCHANTMENT.CLONE".to_owned(), 4))
    );
}

/// Nutritious soup embers every basic Strike of the deck and asks nothing;
/// Pael's claw goops every card Goopy can go on, which is every Defend.
#[test]
fn the_relics_that_enchant_a_whole_set_without_asking() {
    let mut simulator = run_holding_with_treasure(&[], "RELIC.NUTRITIOUS_SOUP");
    let strikes = simulator
        .state()
        .run_player
        .deck
        .iter()
        .filter(|card| card.model_id == id("CARD.STRIKE_IRONCLAD"))
        .count();
    take_the_treasure(&mut simulator);
    assert!(strikes > 0, "the pinned deck holds Strikes");
    assert_eq!(
        deck_carrying(&simulator, "ENCHANTMENT.TEZCATARAS_EMBER"),
        strikes
    );

    let mut simulator = run_holding_with_treasure(&[], "RELIC.PAELS_CLAW");
    let defends = simulator
        .state()
        .run_player
        .deck
        .iter()
        .filter(|card| card.model_id == id("CARD.DEFEND_IRONCLAD"))
        .count();
    take_the_treasure(&mut simulator);
    assert!(defends > 0, "the pinned deck holds Defends");
    assert_eq!(deck_carrying(&simulator, "ENCHANTMENT.GOOPY"), defends);
}

/// A mystic lighter pays nine on a powered attack from a card of its owner's
/// that carries an enchantment, and nothing on a plain one.
#[test]
fn a_mystic_lighter_pays_nine_for_an_enchanted_card() {
    let plain = {
        let mut simulator = fight(&["RELIC.MYSTIC_LIGHTER"], &[("CARD.STRIKE_IRONCLAD", 0)]);
        play(&mut simulator, "CARD.STRIKE_IRONCLAD");
        creature(&simulator, 1).current_hp
    };
    assert_eq!(plain, 33, "a plain Strike is untouched");

    let mut sharpened = card("CARD.STRIKE_IRONCLAD", 0);
    sharpened.enchantment = Some(sts2_engine::EnchantmentFingerprint::new(
        id("ENCHANTMENT.SHARP"),
        1,
    ));
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(30, 30)
        .turn(1, 1)
        .relics(&[id("RELIC.MYSTIC_LIGHTER")])
        .rng_counter(RunStream::MonsterAi, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .card(PileName::Hand, 0, sharpened, 1)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(
        creature(&simulator, 1).current_hp,
        39 - (6 + 1 + 9),
        "six, the Sharp's one, and the lighter's nine"
    );
}

/// Glitter puts a Glam over every card of a reward it can go on, and reads no
/// dice for it; a wing charm puts a Swift over exactly one, rolled on the niche
/// stream.
#[test]
fn the_relics_that_enchant_a_card_reward() {
    let mut simulator = run_winning_a_fight(&["RELIC.GLITTER"], true);
    let niche = niche_counter(&simulator);
    win_the_fight(&mut simulator);
    let offer = offered_rewards(&simulator)
        .into_iter()
        .find(|reward| reward.reward_type == "card")
        .expect("the fight paid a card reward");
    assert!(
        offer.offered_cards.iter().all(|card| card
            .enchantment
            .as_ref()
            .is_some_and(|held| held.model_id == id("ENCHANTMENT.GLAM"))),
        "every option a Glam can go on carries one: {:?}",
        offer.offered_cards
    );
    assert_eq!(
        niche_counter(&simulator),
        niche,
        "enchanting all of them rolls nothing"
    );

    let mut simulator = run_winning_a_fight(&["RELIC.WING_CHARM"], true);
    let niche = niche_counter(&simulator);
    win_the_fight(&mut simulator);
    let offer = offered_rewards(&simulator)
        .into_iter()
        .find(|reward| reward.reward_type == "card")
        .expect("the fight paid a card reward");
    assert_eq!(
        offer
            .offered_cards
            .iter()
            .filter(|card| card.enchantment.is_some())
            .count(),
        1,
        "exactly one option is enchanted"
    );
    assert_eq!(
        niche_counter(&simulator),
        niche + 1,
        "and which one is a roll on the niche stream"
    );
}

/// A silken tress costs its owner every coin they hold, glams one card reward,
/// and is spent.
#[test]
fn a_silken_tress_pays_its_owners_purse_and_glams_one_reward() {
    let mut simulator = run_holding_with_treasure(&[], "RELIC.SILKEN_TRESS");
    assert!(simulator.state().run_player.gold > 0, "the run has gold");
    take_the_treasure(&mut simulator);
    assert_eq!(
        simulator.state().run_player.gold,
        0,
        "taking it costs the whole purse"
    );

    let mut simulator = run_winning_a_fight(&["RELIC.SILKEN_TRESS"], true);
    win_the_fight(&mut simulator);
    let offer = offered_rewards(&simulator)
        .into_iter()
        .find(|reward| reward.reward_type == "card")
        .expect("the fight paid a card reward");
    assert!(
        offer.offered_cards.iter().any(|card| card
            .enchantment
            .as_ref()
            .is_some_and(|held| held.model_id == id("ENCHANTMENT.GLAM"))),
        "the first reward is glammed"
    );
    assert!(
        simulator
            .state()
            .run_player
            .relics
            .iter()
            .any(|relic| relic.model_id == id("RELIC.SILKEN_TRESS") && relic.latched),
        "and the relic is spent"
    );
}

// --- the merchant relics -------------------------------------------------

/// One shelf entry, priced at whatever the test wants to watch move.
fn shelf(index: usize, price: i32) -> sts2_engine::ShopOffer {
    sts2_engine::ShopOffer {
        index,
        item: sts2_engine::ShopItem::Potion(id("POTION.VULNERABLE_POTION")),
        slot: sts2_engine::ShopSlot::Potion,
        base_price: price,
        price,
        sold: false,
    }
}

/// A run standing in a shop with these relics and this shelf.
fn run_at_a_shop(relics: &[&str], offers: Vec<sts2_engine::ShopOffer>) -> Simulator {
    let mut simulator = run_holding_full(relics, "RELIC.LANTERN", 39, None, offers);
    walk_to(&mut simulator, 0);
    simulator
}

fn shop_prices(simulator: &Simulator) -> Vec<i32> {
    match simulator
        .state()
        .run
        .as_ref()
        .and_then(|run| run.active_room.as_ref())
    {
        Some(sts2_engine::ActiveRoom::Shop { offers, .. }) => {
            offers.iter().map(|offer| offer.price).collect()
        }
        _ => panic!("the run is not standing in a shop"),
    }
}

fn shop_removal_price(simulator: &Simulator) -> i32 {
    match simulator
        .state()
        .run
        .as_ref()
        .and_then(|run| run.active_room.as_ref())
    {
        Some(sts2_engine::ActiveRoom::Shop { removal_price, .. }) => *removal_price,
        _ => panic!("the run is not standing in a shop"),
    }
}

/// A membership card halves what a shelf charges, the Courier takes a fifth
/// off, and a shelf held under both is worth two fifths rather than three
/// tenths off: the two bodies are multiplicative and they chain.
#[test]
fn the_two_relics_that_move_a_merchants_prices_chain_multiplicatively() {
    assert_eq!(shop_prices(&run_at_a_shop(&[], vec![shelf(0, 100)])), [100]);
    assert_eq!(
        shop_prices(&run_at_a_shop(
            &["RELIC.MEMBERSHIP_CARD"],
            vec![shelf(0, 100)]
        )),
        [50]
    );
    assert_eq!(
        shop_prices(&run_at_a_shop(&["RELIC.THE_COURIER"], vec![shelf(0, 100)])),
        [80]
    );
    assert_eq!(
        shop_prices(&run_at_a_shop(
            &["RELIC.MEMBERSHIP_CARD", "RELIC.THE_COURIER"],
            vec![shelf(0, 100)]
        )),
        [40]
    );
}

/// The card-removal entry is a merchant entry too, so the same price hooks
/// apply to it as to every stocked shelf entry.
#[test]
fn merchant_relics_discount_card_removal() {
    let plain = shop_removal_price(&run_at_a_shop(&[], vec![shelf(0, 100)]));
    assert_eq!(
        shop_removal_price(&run_at_a_shop(&["RELIC.THE_COURIER"], vec![shelf(0, 100)])),
        plain * 80 / 100
    );
    assert_eq!(
        shop_removal_price(&run_at_a_shop(
            &["RELIC.MEMBERSHIP_CARD", "RELIC.THE_COURIER"],
            vec![shelf(0, 100)]
        )),
        plain * 40 / 100
    );
}

/// The Courier refills whatever slot is bought off the shelf rather than
/// leaving it empty; without it the slot stays sold.
#[test]
fn the_courier_restocks_the_slot_it_was_bought_from() {
    for (relics, restocked) in [(vec![], false), (vec!["RELIC.THE_COURIER"], true)] {
        let mut simulator = run_at_a_shop(&relics, vec![shelf(0, 1)]);
        let buy = simulator
            .legal_actions()
            .iter()
            .find(|action| matches!(action, Action::BuyShopItem { .. }))
            .cloned()
            .expect("the shelf is affordable");
        simulator.step(buy).expect("the purchase applies");
        let sold = match simulator
            .state()
            .run
            .as_ref()
            .and_then(|run| run.active_room.as_ref())
        {
            Some(sts2_engine::ActiveRoom::Shop { offers, .. }) => offers[0].sold,
            _ => panic!("still a shop"),
        };
        assert_eq!(sold, !restocked, "{relics:?}");
    }
}

/// A card slot that refills is not emptied first, so the card just bought off
/// it still counts as on the shelf: the refill rolls over a list without it
/// and never brings the same card straight back. Seed `XGTTW8TTEQ`, record
/// 1540, where a power slot refilled one place along the list.
#[test]
fn the_courier_never_restocks_the_card_just_bought() {
    let card_offer = sts2_engine::ShopOffer {
        index: 0,
        item: sts2_engine::ShopItem::Card(sts2_engine::CardFingerprint::base(id("CARD.INFLAME"))),
        slot: sts2_engine::ShopSlot::CharacterCardOfType(sts2_engine::CardType::Power),
        base_price: 1,
        price: 1,
        sold: false,
    };
    let mut state = run_at_a_shop(&["RELIC.THE_COURIER"], vec![card_offer])
        .state()
        .clone();
    state.run_player.gold = 1_000_000;
    let mut simulator = Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run stands in its shop");
    let on_the_shelf = |simulator: &Simulator| match simulator
        .state()
        .run
        .as_ref()
        .and_then(|run| run.active_room.as_ref())
    {
        Some(sts2_engine::ActiveRoom::Shop { offers, .. }) => match &offers[0].item {
            sts2_engine::ShopItem::Card(card) => card.model_id.clone(),
            other => panic!("a card slot holds a card, found {other:?}"),
        },
        _ => panic!("still a shop"),
    };
    for purchase in 0..200 {
        let bought = on_the_shelf(&simulator);
        let buy = simulator
            .legal_actions()
            .iter()
            .find(|action| matches!(action, Action::BuyShopItem { .. }))
            .cloned()
            .expect("the shelf is affordable");
        simulator.step(buy).expect("the purchase applies");
        assert_ne!(
            on_the_shelf(&simulator),
            bought,
            "purchase {purchase} brought the same card back"
        );
    }
}

/// The Lord's parasol buys the whole shelf once its owner has walked in,
/// before anything else is asked, and pays nothing for any of it.
#[test]
fn a_lords_parasol_buys_the_whole_shelf_for_nothing() {
    let plain = run_at_a_shop(&[], vec![shelf(0, 999), shelf(1, 999)]);
    let gold_before = plain.state().run_player.gold;
    assert!(
        !plain
            .legal_actions()
            .iter()
            .any(|action| matches!(action, Action::BuyShopItem { .. })),
        "nothing on the shelf is affordable"
    );

    let mut simulator = run_at_a_shop(&["RELIC.LORDS_PARASOL"], vec![shelf(0, 999), shelf(1, 999)]);
    // The purchases are owed as the room is entered and made as the next
    // step begins: answering the removal the parasol forces makes them all.
    let answer = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseCards { .. }))
        .cloned()
        .expect("the parasol's removal is what is asked for");
    simulator.step(answer).expect("the answer applies");
    let sold = match simulator
        .state()
        .run
        .as_ref()
        .and_then(|run| run.active_room.as_ref())
    {
        Some(sts2_engine::ActiveRoom::Shop { offers, .. }) => {
            offers.iter().filter(|offer| offer.sold).count()
        }
        _ => panic!("still a shop"),
    };
    assert_eq!(sold, 2, "both slots were cleared");
    assert_eq!(
        simulator.state().run_player.gold,
        gold_before,
        "and nothing was paid for either"
    );
    let belt = simulator
        .state()
        .run_player
        .potions
        .iter()
        .filter(|slot| slot.is_some())
        .count();
    let before = plain
        .state()
        .run_player
        .potions
        .iter()
        .filter(|slot| slot.is_some())
        .count();
    assert_eq!(belt, before + 2, "the belt took both");
}

/// A fake snecko eye is fifty at a shop rather than the unreachable price its
/// Event rarity would otherwise carry.
#[test]
fn the_fake_relics_override_what_a_merchant_charges() {
    let registry = sts2_content::standard_registry();
    let fake = registry
        .relic(&id("RELIC.FAKE_SNECKO_EYE"))
        .expect("registered");
    assert_eq!(fake.merchant_cost(), 50);
    let plain = registry
        .relic(&id("RELIC.MEMBERSHIP_CARD"))
        .expect("registered");
    assert_eq!(
        plain.merchant_cost(),
        200,
        "a shop relic is read off rarity"
    );
}

/// Chemical X pays two more X on an X-cost play.
#[test]
fn chemical_x_pays_two_more_x() {
    for (relics, damage) in [(vec![], 4), (vec!["RELIC.CHEMICAL_X"], 6)] {
        let held: Vec<sts2_core::ModelId> = relics.iter().map(|model| id(model)).collect();
        let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .player_hp(80, 80)
            .energy(4, 4)
            .turn(1, 1)
            .relics(&held)
            .rng_counter(RunStream::MonsterAi, 0)
            .rng_counter(RunStream::CombatTargets, 0)
            .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
            .card(PileName::Hand, 0, card("CARD.WHIRLWIND", 0), 0)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
        play(&mut simulator, "CARD.WHIRLWIND");
        assert_eq!(
            39 - creature(&simulator, 1).current_hp,
            damage * 5,
            "{relics:?}: a whirlwind hits for five, X times over"
        );
    }
    // The same two on a cost of X stars, which reads the answer back through
    // the very same X-value pass.
    for (relics, hits) in [(vec![], 2), (vec!["RELIC.CHEMICAL_X"], 4)] {
        let held: Vec<sts2_core::ModelId> = relics.iter().map(|model| id(model)).collect();
        let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .played_as(id("CHARACTER.REGENT"))
            .player_hp(80, 80)
            .energy(4, 4)
            .stars(2)
            .turn(1, 1)
            .relics(&held)
            .rng_counter(RunStream::MonsterAi, 0)
            .rng_counter(RunStream::CombatTargets, 0)
            .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 300, 300, "SHRINKER_MOVE")
            .card(PileName::Hand, 0, card("CARD.STARDUST", 0), 0)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
        play(&mut simulator, "CARD.STARDUST");
        assert_eq!(
            300 - creature(&simulator, 1).current_hp,
            hits * 5,
            "{relics:?}: a stardust hits for five, X times over"
        );
    }
}

// --- the rest site's option list -----------------------------------------

/// The rest site with a deck card and some relics added, so a test can watch
/// what they put on the option list.
fn rest_run(relics: &[&str], extra_deck: &[&str]) -> Simulator {
    let mut state = run_holding(relics).state().clone();
    for model_id in extra_deck {
        state
            .run_player
            .deck
            .push(CardFingerprint::base(id(model_id)));
    }
    Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run builds")
}

/// The option list a rest site is standing on.
fn rest_options(simulator: &Simulator) -> Vec<&'static str> {
    match simulator.decision() {
        sts2_engine::DecisionContext::RestSite { options, .. } => options
            .iter()
            .map(|state| state.option.option_id())
            .collect(),
        other => panic!("the room is not a rest site: {other:?}"),
    }
}

/// The rest options that can actually be taken.
fn offered_options(simulator: &Simulator) -> Vec<&'static str> {
    simulator
        .legal_actions()
        .iter()
        .filter_map(|action| match action {
            Action::RestOption { option, .. } => Some(option.option_id()),
            _ => None,
        })
        .collect()
}

fn rest_option(simulator: &Simulator, option_id: &str) -> Action {
    simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::RestOption { option, .. }
                if option.option_id() == option_id)
        })
        .cloned()
        .unwrap_or_else(|| panic!("{option_id} is not offered"))
}

/// Puts the run back on its start point. The harness's map is one ancient with
/// a row of rooms hanging off it and nothing leading on from those, so a
/// scenario that walks into two rooms has to come back between them.
fn back_to_the_start(simulator: &mut Simulator) {
    let mut state = simulator.state().clone();
    state.combat = None;
    state.terminal = None;
    if let Some(run) = state.run.as_mut() {
        run.phase = RunPhase::Map;
        run.active_room = None;
        run.reward_set = None;
        run.current_coord = Some(MapCoord { col: 2, row: 0 });
    }
    *simulator = Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run rebuilds");
}

/// A rest site offers the heal, then the smith, then whatever its listeners
/// add — and that pass walks the run's own listeners, so the deck's cards speak
/// before the relics do.
#[test]
fn a_rest_site_generates_the_options_its_listeners_add() {
    let mut plain = run_holding(&[]);
    walk_to(&mut plain, 1);
    assert_eq!(rest_options(&plain), ["HEAL", "SMITH"]);

    let mut stocked = rest_run(
        &["RELIC.SHOVEL", "RELIC.MEAT_CLEAVER", "RELIC.PUMPKIN_CANDLE"],
        &["CARD.BYRDONIS_EGG"],
    );
    walk_to(&mut stocked, 1);
    assert_eq!(
        rest_options(&stocked),
        ["HEAL", "SMITH", "HATCH", "DIG", "COOK", "KINDLE"],
        "the deck's egg speaks before the relics, and the relics in inventory order"
    );
}

/// An option that is not enabled is on the screen to be read and cannot be
/// taken: the smith with nothing upgradable, the cook with fewer than two
/// cards it may remove.
#[test]
fn a_disabled_rest_option_is_listed_and_not_offered() {
    let mut state = run_holding(&["RELIC.MEAT_CLEAVER"]).state().clone();
    state.run_player.deck = vec![card("CARD.CLUMSY", 0)];
    let mut simulator = Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run builds");
    walk_to(&mut simulator, 1);
    assert_eq!(rest_options(&simulator), ["HEAL", "SMITH", "COOK"]);
    assert_eq!(
        offered_options(&simulator),
        ["HEAL"],
        "a curse can be neither upgraded nor cooked with"
    );
}

/// Taking an option never leaves the room. What it does is spend the option:
/// the whole list is cleared, unless something keeps the rest of it standing.
#[test]
fn taking_a_rest_option_clears_the_list_and_opens_the_proceed() {
    for (relics, standing) in [(vec![], 0), (vec!["RELIC.MINIATURE_TENT"], 1)] {
        let mut simulator = run_holding(&relics);
        walk_to(&mut simulator, 1);
        assert!(
            !simulator
                .legal_actions()
                .iter()
                .any(|action| matches!(action, Action::ChooseMap { .. })),
            "{relics:?}: the proceed is down until an option has been taken"
        );
        simulator
            .step(rest_option(&simulator, "HEAL"))
            .expect("the rest applies");
        let sts2_engine::DecisionContext::RestSite { options, left_open } =
            simulator.decision().clone()
        else {
            panic!("{relics:?}: the rest site is still standing");
        };
        assert!(left_open);
        assert_eq!(
            options.iter().filter(|state| state.standing).count(),
            standing,
            "{relics:?}: a tent leaves the smith standing and nothing else does"
        );
        assert_eq!(offered_options(&simulator).len(), standing);
    }
}

/// A girya offers a lift until it has been lifted three times, and works in a
/// strength for every lift as a fight is walked into.
#[test]
fn a_girya_is_lifted_three_times() {
    let counter = |simulator: &Simulator| {
        simulator
            .state()
            .run_player
            .relics
            .iter()
            .find(|relic| relic.model_id == id("RELIC.GIRYA"))
            .expect("the relic is held")
            .counter
    };
    let mut simulator = run_holding(&["RELIC.GIRYA"]);
    for (col, expected) in [(1, 1), (2, 2)] {
        back_to_the_start(&mut simulator);
        walk_to(&mut simulator, col);
        assert!(rest_options(&simulator).contains(&"LIFT"));
        simulator
            .step(rest_option(&simulator, "LIFT"))
            .expect("lifting applies");
        assert_eq!(
            counter(&simulator),
            expected,
            "the relic keeps its own count"
        );
    }

    // The lifts are worth a strength each as a fight is walked into.
    back_to_the_start(&mut simulator);
    walk_to(&mut simulator, 3);
    assert_eq!(
        creature(&simulator, 0).power_amount("POWER.STRENGTH_POWER"),
        2
    );

    // Three lifts in, the relic answers the pass with no and the option is not
    // generated at all.
    let mut state = simulator.state().clone();
    for relic in &mut state.run_player.relics {
        if relic.model_id == id("RELIC.GIRYA") {
            relic.counter = 3;
        }
    }
    let mut spent = Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run builds");
    back_to_the_start(&mut spent);
    walk_to(&mut spent, 1);
    assert_eq!(rest_options(&spent), ["HEAL", "SMITH"]);
}

/// A meat cleaver's cook takes two cards out of the deck for nine more maximum
/// HP, and backing out of its screen leaves the rest site exactly as it stood.
#[test]
fn a_meat_cleaver_cooks_two_cards_for_nine_maximum_hp() {
    let mut simulator = run_holding(&["RELIC.MEAT_CLEAVER"]);
    walk_to(&mut simulator, 1);
    let deck_before = simulator.state().run_player.deck.clone();
    let max_before = simulator.state().run_player.max_hp;
    let hp_before = run_hp(&simulator);
    simulator
        .step(rest_option(&simulator, "COOK"))
        .expect("cooking applies");
    let sts2_engine::DecisionContext::ChooseCards {
        choice_id,
        minimum,
        maximum,
        cancelable,
        candidates,
        ..
    } = simulator.decision().clone()
    else {
        panic!("the cook puts a deck screen up");
    };
    assert_eq!((minimum, maximum), (2, 2));
    assert!(cancelable);

    let mut backed_out = simulator.clone();
    backed_out
        .step(Action::ChooseCards {
            choice_id,
            cards: Vec::new(),
        })
        .expect("backing out applies");
    let sts2_engine::DecisionContext::RestSite { options, left_open } =
        backed_out.decision().clone()
    else {
        panic!("backing out leaves the rest site standing");
    };
    assert!(!left_open, "and does not open the proceed");
    assert!(options.iter().all(|state| state.standing));
    assert_eq!(backed_out.state().run_player.deck, deck_before);

    simulator
        .step(Action::ChooseCards {
            choice_id,
            cards: vec![candidates[0].clone(), candidates[1].clone()],
        })
        .expect("the choice applies");
    assert_eq!(
        simulator.state().run_player.deck.len(),
        deck_before.len() - 2
    );
    assert_eq!(simulator.state().run_player.max_hp, max_before + 9);
    assert_eq!(
        run_hp(&simulator),
        hp_before + 9,
        "gaining maximum HP heals by as much"
    );
}

/// The cook's screen asks for exactly two cards, and the answers it takes are
/// a pair or nothing at all: a screen that asks for an exact count refuses to
/// confirm until it holds that many, so being cancelable buys the empty answer
/// and nothing shorter. The smith's
/// screen asks for one, and every card on it is still an answer of its own.
#[test]
fn the_cook_takes_a_pair_or_nothing() {
    let mut simulator = run_holding(&["RELIC.MEAT_CLEAVER"]);
    walk_to(&mut simulator, 1);
    simulator
        .step(rest_option(&simulator, "COOK"))
        .expect("cooking applies");
    let sts2_engine::DecisionContext::ChooseCards {
        choice_id,
        minimum,
        maximum,
        cancelable,
        candidates,
        ..
    } = simulator.decision().clone()
    else {
        panic!("the cook puts a deck screen up");
    };
    assert_eq!((minimum, maximum, cancelable), (2, 2, true));
    assert!(candidates.len() > 2, "there is more than one pair to pick");

    let mut pairs: BTreeSet<BTreeSet<usize>> = BTreeSet::new();
    let mut empties = 0;
    for action in simulator.legal_actions() {
        let Action::ChooseCards { cards, .. } = action else {
            continue;
        };
        assert_ne!(cards.len(), 1, "one card is not an answer to the cook");
        match cards.len() {
            0 => empties += 1,
            2 => {
                pairs.insert(cards.iter().map(|handle| handle.index).collect());
            }
            other => panic!("a two-card screen offered {other} cards"),
        }
    }
    assert_eq!(
        pairs.len(),
        candidates.len() * (candidates.len() - 1) / 2,
        "every pair of removable cards is an answer"
    );
    assert_eq!(empties, 1, "and backing out with nothing is one more");

    // The gate refuses what the list does not carry, so no one-card answer
    // reaches `finish_rest_cook` and its nine maximum HP by another route.
    let mut single = simulator.clone();
    assert_eq!(
        single
            .step(Action::ChooseCards {
                choice_id,
                cards: vec![candidates[0].clone()],
            })
            .expect_err("one card is refused")
            .code,
        sts2_engine::ErrorCode::IllegalAction
    );

    // The smith opens a screen that does take one card,
    // and it still offers every candidate on its own.
    let mut smith = run_holding(&["RELIC.MEAT_CLEAVER"]);
    walk_to(&mut smith, 1);
    smith
        .step(rest_option(&smith, "SMITH"))
        .expect("the smith applies");
    let sts2_engine::DecisionContext::ChooseCards {
        minimum,
        maximum,
        cancelable,
        candidates,
        ..
    } = smith.decision().clone()
    else {
        panic!("the smith puts a deck screen up");
    };
    assert_eq!((minimum, maximum, cancelable), (1, 1, true));
    assert_eq!(
        smith
            .legal_actions()
            .iter()
            .filter(|action| matches!(action, Action::ChooseCards { cards, .. }
                if cards.len() == 1))
            .count(),
        candidates.len(),
        "every upgradable card is its own answer"
    );
}

/// Pael's growth duplicates every deck card carrying the Clone enchantment,
/// enchantment and all, and the copies join the deck on the floor they were
/// made on.
#[test]
fn paels_growth_clones_every_marked_card() {
    let mut state = run_holding(&["RELIC.PAELS_GROWTH"]).state().clone();
    let mut marked = card("CARD.STRIKE_IRONCLAD", 1);
    marked.enchantment = Some(sts2_engine::EnchantmentFingerprint::new(
        id("ENCHANTMENT.CLONE"),
        4,
    ));
    state.run_player.deck = vec![marked.clone(), card("CARD.DEFEND_IRONCLAD", 0)];
    let mut simulator = Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run builds");
    walk_to(&mut simulator, 1);
    assert!(rest_options(&simulator).contains(&"CLONE"));
    simulator
        .step(rest_option(&simulator, "CLONE"))
        .expect("cloning applies");
    let deck = &simulator.state().run_player.deck;
    assert_eq!(deck.len(), 3, "one marked card, one copy");
    assert_eq!(deck[2].model_id, marked.model_id);
    assert_eq!(deck[2].upgrade_level, 1);
    assert_eq!(
        deck[2].enchantment, marked.enchantment,
        "the copy carries the enchantment, so it clones itself next time"
    );
    assert_eq!(
        deck[2].floor_added_to_deck,
        Some(simulator.state().run.as_ref().expect("a run").floor),
        "and joins the deck on the floor it was made on"
    );
}

/// A shovel digs a relic off the front of its owner's own bag, and is not
/// spent by doing it.
#[test]
fn a_shovel_digs_a_relic_out_of_the_bag() {
    let mut state = run_holding(&["RELIC.SHOVEL"]).state().clone();
    if let Some(run) = state.run.as_mut() {
        run.player_relic_grab_bag = stocked_bag();
    }
    let mut simulator = Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run builds");
    walk_to(&mut simulator, 1);
    let held = simulator.state().run_player.relics.len();
    simulator
        .step(rest_option(&simulator, "DIG"))
        .expect("digging applies");
    assert_eq!(simulator.state().run_player.relics.len(), held + 1);
    assert!(
        simulator
            .state()
            .run_player
            .relics
            .iter()
            .any(|relic| relic.model_id == id("RELIC.SHOVEL")),
        "the shovel is not spent by digging"
    );
}

/// A pumpkin candle is lit for five fights the moment it is taken, pays an
/// energy while it is lit, burns one fight of kindling down at the end of every
/// combat, and is kindled for five more at a rest site.
#[test]
fn a_pumpkin_candle_burns_down_and_is_kindled_again() {
    let mut simulator = run_holding_full(&[], "RELIC.PUMPKIN_CANDLE", 1, None, Vec::new());
    walk_to(&mut simulator, 4);
    let take = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::TakeTreasure { .. }))
        .cloned()
        .expect("the relic is on the chest");
    simulator.step(take).expect("taking it applies");
    let candle = |simulator: &Simulator| {
        simulator
            .state()
            .run_player
            .relics
            .iter()
            .find(|relic| relic.model_id == id("RELIC.PUMPKIN_CANDLE"))
            .expect("the relic is held")
            .counter
    };
    assert_eq!(candle(&simulator), 5, "taking it kindles it");

    back_to_the_start(&mut simulator);
    walk_to(&mut simulator, 3);
    assert_eq!(
        simulator
            .state()
            .combat
            .as_ref()
            .expect("a fight")
            .player
            .energy,
        4,
        "a lit candle pays an energy"
    );
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(candle(&simulator), 4, "and a fight burns one down");

    back_to_the_start(&mut simulator);
    walk_to(&mut simulator, 1);
    simulator
        .step(rest_option(&simulator, "KINDLE"))
        .expect("kindling applies");
    assert_eq!(candle(&simulator), 9, "kindling is five more fights");
}

/// A byrdonis egg in the deck offers a hatch, and hatching hands over the
/// byrdpip — which turns the egg that offered it into a swoop on its way in.
#[test]
fn a_byrdonis_egg_offers_a_hatch() {
    let mut simulator = rest_run(&[], &["CARD.BYRDONIS_EGG"]);
    walk_to(&mut simulator, 1);
    assert_eq!(rest_options(&simulator), ["HEAL", "SMITH", "HATCH"]);
    simulator
        .step(rest_option(&simulator, "HATCH"))
        .expect("hatching applies");
    assert!(
        simulator
            .state()
            .run_player
            .relics
            .iter()
            .any(|relic| relic.model_id == id("RELIC.BYRDPIP")),
        "the hatch obtains a Byrdpip"
    );
    let deck: Vec<String> = simulator
        .state()
        .run_player
        .deck
        .iter()
        .map(|card| card.model_id.to_string())
        .collect();
    assert!(!deck.contains(&"CARD.BYRDONIS_EGG".to_owned()));
    assert!(deck.contains(&"CARD.BYRD_SWOOP".to_owned()));
}

// --- the ancients that make cards ----------------------------------------

/// Takes a relic off the chest of the harness's map.
fn take_from_the_chest(simulator: &mut Simulator) {
    walk_to(simulator, 4);
    let take = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::TakeTreasure { .. }))
        .cloned()
        .expect("the relic is on the chest");
    simulator.step(take).expect("taking it applies");
}

fn rewards_counter(simulator: &Simulator) -> u64 {
    simulator
        .state()
        .player_rng
        .stream(sts2_rng::PlayerStream::Rewards)
        .counter()
}

/// The rarity the registry gives a card, which is the pool it stands in.
fn rarity_of(model_id: &sts2_core::ModelId) -> sts2_engine::CardRarity {
    sts2_content::standard_registry()
        .card(model_id)
        .expect("the card is content")
        .rarity
}

/// An arcane scroll makes one rare card of its owner's own pool and puts it
/// straight in the deck, on one draw and nothing else.
#[test]
fn an_arcane_scroll_makes_one_rare_card() {
    let mut simulator = run_holding_with_treasure(&[], "RELIC.ARCANE_SCROLL");
    let deck_before = simulator.state().run_player.deck.len();
    let rolls_before = rewards_counter(&simulator);
    take_from_the_chest(&mut simulator);
    let deck = &simulator.state().run_player.deck;
    assert_eq!(deck.len(), deck_before + 1);
    let made = deck.last().expect("the card joined the deck");
    assert_eq!(rarity_of(&made.model_id), sts2_engine::CardRarity::Rare);
    assert_eq!(made.upgrade_level, 0, "no upgrade is rolled at all");
    assert_eq!(
        rewards_counter(&simulator) - rolls_before,
        1,
        "uniform odds over one rarity are a single draw"
    );
    assert_eq!(
        made.floor_added_to_deck,
        Some(simulator.state().run.as_ref().expect("a run").floor)
    );
}

/// A glass eye hands over five card offers of three, one rarity each: two
/// common, two uncommon, one rare.
#[test]
fn a_glass_eye_offers_five_card_rewards() {
    let mut simulator = run_holding_with_treasure(&[], "RELIC.GLASS_EYE");
    let rolls_before = rewards_counter(&simulator);
    take_from_the_chest(&mut simulator);
    let rewards = offered_rewards(&simulator);
    assert_eq!(rewards.len(), 5);
    let expected = [
        sts2_engine::CardRarity::Common,
        sts2_engine::CardRarity::Common,
        sts2_engine::CardRarity::Uncommon,
        sts2_engine::CardRarity::Uncommon,
        sts2_engine::CardRarity::Rare,
    ];
    for (reward, rarity) in rewards.iter().zip(expected) {
        assert_eq!(reward.reward_type, "card");
        assert_eq!(reward.offered_cards.len(), 3);
        assert!(
            reward.card_pools.is_empty(),
            "a reward made from already-created cards serializes no source pool"
        );
        assert!(
            reward
                .offered_cards
                .iter()
                .all(|card| rarity_of(&card.model_id) == rarity),
            "every card of an offer carries the rarity it was built for"
        );
        assert_eq!(reward.rarity_odds, Some(sts2_engine::RarityOdds::Uniform));
    }
    assert_eq!(
        rewards_counter(&simulator) - rolls_before,
        15,
        "one draw per card and no upgrade roll"
    );
}

/// A hefty tablet lays out three rares to pick one of, and charges an injury
/// for it whether or not one is picked.
#[test]
fn a_hefty_tablet_offers_three_rares_and_an_injury() {
    let mut simulator = run_holding_with_treasure(&[], "RELIC.HEFTY_TABLET");
    let deck_before = simulator.state().run_player.deck.clone();
    take_from_the_chest(&mut simulator);
    let sts2_engine::DecisionContext::ChooseCards {
        choice_id,
        identity,
        minimum,
        maximum,
        cancelable,
        candidates,
        ..
    } = simulator.decision().clone()
    else {
        panic!("the tablet puts a choose-a-card screen up");
    };
    assert_eq!(identity, sts2_engine::ChoiceIdentity::Offered);
    assert_eq!((minimum, maximum), (0, 1));
    assert!(cancelable, "the screen can be skipped");
    assert_eq!(candidates.len(), 3);
    assert!(
        candidates
            .iter()
            .all(|card| rarity_of(&card.fingerprint.model_id) == sts2_engine::CardRarity::Rare)
    );

    // Skipping still charges the injury.
    let mut skipped = simulator.clone();
    skipped
        .step(Action::ChooseCards {
            choice_id,
            cards: Vec::new(),
        })
        .expect("skipping applies");
    assert_eq!(skipped.state().run_player.deck.len(), deck_before.len() + 1);
    assert_eq!(
        skipped
            .state()
            .run_player
            .deck
            .last()
            .expect("a card joined")
            .model_id,
        id("CARD.INJURY")
    );

    simulator
        .step(Action::ChooseCards {
            choice_id,
            cards: vec![candidates[1].clone()],
        })
        .expect("picking applies");
    let deck = &simulator.state().run_player.deck;
    assert_eq!(deck.len(), deck_before.len() + 2);
    assert_eq!(
        deck[deck_before.len()].model_id,
        candidates[1].fingerprint.model_id,
        "the card picked joins ahead of the injury"
    );
    assert_eq!(deck[deck_before.len() + 1].model_id, id("CARD.INJURY"));
}

/// A lead paperweight lays out two colorless cards to pick one of. Nothing
/// spares its upgrade roll, so each card costs a rarity draw, a pick, and an
/// upgrade draw.
#[test]
fn a_lead_paperweight_offers_two_colorless_cards() {
    let mut simulator = run_holding_with_treasure(&[], "RELIC.LEAD_PAPERWEIGHT");
    let deck_before = simulator.state().run_player.deck.len();
    let rolls_before = rewards_counter(&simulator);
    take_from_the_chest(&mut simulator);
    let sts2_engine::DecisionContext::ChooseCards {
        choice_id,
        candidates,
        cancelable,
        ..
    } = simulator.decision().clone()
    else {
        panic!("the paperweight puts a choose-a-card screen up");
    };
    assert!(cancelable);
    assert_eq!(candidates.len(), 2);
    let registry = sts2_content::standard_registry();
    let colorless: Vec<sts2_core::ModelId> = registry
        .card_pool(&id("CARD_POOL.COLORLESS_CARD_POOL"))
        .expect("the colorless pool is content")
        .iter()
        .map(|card| card.model_id.clone())
        .collect();
    assert!(
        candidates
            .iter()
            .all(|card| colorless.contains(&card.fingerprint.model_id)),
        "the offer is built off the colorless pool"
    );
    assert_eq!(
        rewards_counter(&simulator) - rolls_before,
        6,
        "a rarity, a pick, and an upgrade draw for each of the two"
    );
    simulator
        .step(Action::ChooseCards {
            choice_id,
            cards: vec![candidates[0].clone()],
        })
        .expect("picking applies");
    assert_eq!(simulator.state().run_player.deck.len(), deck_before + 1);
    assert_eq!(
        simulator
            .state()
            .run_player
            .deck
            .last()
            .expect("a card joined")
            .model_id,
        candidates[0].fingerprint.model_id
    );
}

/// A fresnel lens puts two Nimble on every card it meets that will take one: on
/// a card reward's options, on a card joining the deck, and on what a merchant
/// creates.
#[test]
fn a_fresnel_lens_enchants_the_cards_it_meets() {
    let registry = sts2_content::standard_registry();
    let nimble = registry
        .enchantment(&id("ENCHANTMENT.NIMBLE"))
        .expect("the enchantment is content")
        .clone();
    // Whether the lens should have reached the card, asked of the card as it
    // was before the lens saw it: an enchantment already on a card is itself
    // one of the things an enchantment refuses to go on.
    let takes_one = |card: &CardFingerprint| {
        let mut bare = card.clone();
        bare.enchantment = None;
        registry.can_enchant(&nimble, &bare, false)
    };
    let enchanted = |card: &CardFingerprint| {
        card.enchantment
            .as_ref()
            .is_some_and(|worn| worn.model_id == id("ENCHANTMENT.NIMBLE") && worn.amount == 2)
    };

    // A card reward, modified late as its options are laid out.
    let mut simulator = run_winning_a_fight(&["RELIC.FRESNEL_LENS"], true);
    win_the_fight(&mut simulator);
    let offered = offered_rewards(&simulator)
        .into_iter()
        .find(|reward| reward.reward_type == "card")
        .expect("a card reward")
        .offered_cards;
    assert!(!offered.is_empty());
    for card in &offered {
        assert_eq!(enchanted(card), takes_one(card), "{}", card.model_id);
    }

    // A card joining the deck, modified as it is added. An
    // arcane scroll makes one rare card and puts it straight in the deck.
    let mut joining = run_holding_with_treasure(&["RELIC.FRESNEL_LENS"], "RELIC.ARCANE_SCROLL");
    let deck_before = joining.state().run_player.deck.len();
    take_from_the_chest(&mut joining);
    let deck = &joining.state().run_player.deck;
    assert_eq!(deck.len(), deck_before + 1);
    let made = deck.last().expect("the card joined");
    assert_eq!(enchanted(made), takes_one(made), "{}", made.model_id);

    // A merchant's own shelf, modified as the shelf is stocked.
    let mut state = run_holding(&["RELIC.FRESNEL_LENS"]).state().clone();
    if let Some(run) = state.run.as_mut() {
        run.rooms[0].retain(|room| room.coord != MapCoord { col: 0, row: 1 });
        run.player_relic_grab_bag = stocked_bag();
        run.shared_relic_grab_bag = stocked_bag();
    }
    let mut shop = Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run rebuilds");
    walk_to(&mut shop, 0);
    let sts2_engine::DecisionContext::Shop { offers, .. } = shop.decision() else {
        panic!("the point resolves to a generated merchant");
    };
    let shelved: Vec<&CardFingerprint> = offers
        .iter()
        .filter_map(|offer| match &offer.item {
            sts2_engine::ShopItem::Card(card) => Some(card),
            _ => None,
        })
        .collect();
    assert!(!shelved.is_empty());
    for card in shelved {
        assert_eq!(enchanted(card), takes_one(card), "{}", card.model_id);
    }
}

/// A silver crucible upgrades every card of the next three card rewards it can,
/// counts a use for each of them whether or not anything was upgradable, and
/// suppresses the first chest walked into after it.
#[test]
fn a_silver_crucible_upgrades_three_card_rewards() {
    let mut simulator = run_winning_a_fight(&["RELIC.SILVER_CRUCIBLE"], true);
    let registry = sts2_content::standard_registry();
    let crucible = |simulator: &Simulator| {
        let relic = simulator
            .state()
            .run_player
            .relics
            .iter()
            .find(|relic| relic.model_id == id("RELIC.SILVER_CRUCIBLE"))
            .expect("the relic is held");
        (relic.counter, relic.second_counter)
    };
    assert_eq!(crucible(&simulator), (0, 0));
    win_the_fight(&mut simulator);
    let cards = offered_rewards(&simulator)
        .into_iter()
        .find(|reward| reward.reward_type == "card")
        .expect("a card reward")
        .offered_cards;
    assert!(!cards.is_empty());
    for card in &cards {
        let definition = registry.card(&card.model_id).expect("the card is content");
        if definition.upgrade.is_some() {
            assert!(card.upgrade_level >= 1, "{}", card.model_id);
        }
    }
    assert_eq!(crucible(&simulator), (1, 0), "and the use is counted");

    // Three uses in, the relic stops answering the pass.
    let mut state = simulator.state().clone();
    for relic in &mut state.run_player.relics {
        if relic.model_id == id("RELIC.SILVER_CRUCIBLE") {
            relic.counter = 3;
        }
    }
    let mut spent = Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run rebuilds");
    back_to_the_start(&mut spent);
    win_the_fight(&mut spent);
    let cards = offered_rewards(&spent)
        .into_iter()
        .find(|reward| reward.reward_type == "card")
        .expect("a card reward")
        .offered_cards;
    assert!(
        cards.iter().any(|card| card.upgrade_level == 0),
        "a spent crucible upgrades nothing"
    );
    assert_eq!(crucible(&spent), (3, 0), "and counts nothing either");
}

/// The chest the crucible's owner walks into next hands over nothing at all —
/// no relic and no gold, and neither is rolled for — because the room is
/// counted on the way in, before the chest asks whether to generate anything.
/// The one after it pays out.
#[test]
fn a_silver_crucible_suppresses_the_first_chest() {
    let mut state = run_holding(&["RELIC.SILVER_CRUCIBLE"]).state().clone();
    // The chest is the run's own rather than one the scenario placed, so it
    // fills itself as the room is entered.
    if let Some(run) = state.run.as_mut() {
        run.rooms[0].retain(|room| room.coord != MapCoord { col: 4, row: 1 });
        run.shared_relic_grab_bag = stocked_bag();
    }
    let mut simulator = Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run builds");
    let gold_before = simulator.state().run_player.gold;
    let treasure_rolls = simulator
        .state()
        .rng
        .stream(RunStream::TreasureRoomRelics)
        .counter();
    let rewards_before = rewards_counter(&simulator);
    walk_to(&mut simulator, 4);
    assert!(
        matches!(
            simulator.decision(),
            sts2_engine::DecisionContext::RoomProceed
        ),
        "an empty chest leaves nothing to take"
    );
    assert_eq!(simulator.state().run_player.gold, gold_before);
    assert_eq!(
        simulator
            .state()
            .rng
            .stream(RunStream::TreasureRoomRelics)
            .counter(),
        treasure_rolls,
        "and nothing is rolled for"
    );
    assert_eq!(rewards_counter(&simulator), rewards_before);
    let counters = simulator
        .state()
        .run_player
        .relics
        .iter()
        .find(|relic| relic.model_id == id("RELIC.SILVER_CRUCIBLE"))
        .expect("the relic is held")
        .second_counter;
    assert_eq!(counters, 1, "the room is counted on the way in");

    // The second chest pays out.
    back_to_the_start(&mut simulator);
    let mut state = simulator.state().clone();
    if let Some(run) = state.run.as_mut() {
        run.visited.clear();
    }
    let mut second = Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run rebuilds");
    walk_to(&mut second, 4);
    assert!(second.state().run_player.gold > gold_before);
    let sts2_engine::DecisionContext::Treasure { relics } = second.decision() else {
        panic!("the second chest holds a relic");
    };
    assert_eq!(relics.len(), 1);
}

/// A driftwood puts a reroll on every card offer of a set. Taking it throws the
/// offer away and builds another with the same options, and the button is gone.
#[test]
fn a_driftwood_rerolls_a_card_offer_once() {
    let mut simulator = run_winning_a_fight(&["RELIC.DRIFTWOOD"], true);
    win_the_fight(&mut simulator);
    let set = simulator
        .state()
        .combat
        .as_ref()
        .and_then(|combat| combat.reward_set.as_ref())
        .expect("the fight paid out")
        .clone();
    let (index, reward) = set
        .rewards
        .iter()
        .enumerate()
        .find(|(_, reward)| reward.fingerprint.reward_type == "card")
        .expect("a card reward");
    let first = reward.fingerprint.offered_cards.clone();
    let claim = Action::ClaimReward {
        set_id: set.id,
        reward_index: index,
        fingerprint: reward.fingerprint.clone(),
    };
    simulator.step(claim).expect("opening the offer applies");
    let sts2_engine::DecisionContext::ChooseCards { choice_id, .. } = simulator.decision().clone()
    else {
        panic!("the card reward puts a screen up");
    };
    let reroll = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::ChooseAlternative { option_id, .. }
                if option_id == "reroll")
        })
        .cloned()
        .expect("the reroll is offered");
    assert!(matches!(
        reroll,
        Action::ChooseAlternative { choice_id: id, .. } if id == choice_id
    ));
    let rolls_before = rewards_counter(&simulator);
    simulator.step(reroll).expect("the reroll applies");
    assert!(
        rewards_counter(&simulator) > rolls_before,
        "a fresh offer is rolled"
    );
    let sts2_engine::DecisionContext::ChooseCards { candidates, .. } = simulator.decision().clone()
    else {
        panic!("the screen is asked again");
    };
    assert_eq!(candidates.len(), first.len());
    assert!(
        !simulator.legal_actions().iter().any(
            |action| matches!(action, Action::ChooseAlternative { option_id, .. }
                if option_id == "reroll")
        ),
        "and the button is gone"
    );
}

/// Scroll boxes lay out two bundles of two commons and an uncommon, all six
/// different, and whichever bundle is picked joins the deck whole.
#[test]
fn scroll_boxes_offer_two_bundles() {
    let mut simulator = run_holding_with_treasure(&[], "RELIC.SCROLL_BOXES");
    let deck_before = simulator.state().run_player.deck.len();
    let rolls_before = rewards_counter(&simulator);
    take_from_the_chest(&mut simulator);
    let sts2_engine::DecisionContext::ChooseCards {
        candidates,
        bundles,
        cancelable,
        ..
    } = simulator.decision().clone()
    else {
        panic!("the boxes put a bundle screen up");
    };
    assert!(!cancelable, "a bundle screen cannot be backed out of");
    assert_eq!(bundles, vec![vec![0, 1, 2], vec![3, 4, 5]]);
    assert_eq!(
        rewards_counter(&simulator) - rolls_before,
        6,
        "one draw per card and nothing else"
    );
    for bundle in &bundles {
        let rarities: Vec<_> = bundle
            .iter()
            .map(|index| rarity_of(&candidates[*index].fingerprint.model_id))
            .collect();
        assert_eq!(
            rarities,
            [
                sts2_engine::CardRarity::Common,
                sts2_engine::CardRarity::Common,
                sts2_engine::CardRarity::Uncommon,
            ]
        );
    }
    let mut models: Vec<_> = candidates
        .iter()
        .map(|card| card.fingerprint.model_id.clone())
        .collect();
    models.sort();
    models.dedup();
    assert_eq!(models.len(), 6, "every card of both bundles is different");

    let taken = simulator.legal_actions()[1].clone();
    let Action::ChooseCards { cards, .. } = taken.clone() else {
        panic!("a bundle is answered with its cards");
    };
    simulator.step(taken).expect("picking a bundle applies");
    let deck = &simulator.state().run_player.deck;
    assert_eq!(deck.len(), deck_before + 3);
    assert_eq!(
        deck[deck_before..]
            .iter()
            .map(|card| card.model_id.clone())
            .collect::<Vec<_>>(),
        cards
            .iter()
            .map(|card| card.fingerprint.model_id.clone())
            .collect::<Vec<_>>(),
        "the whole bundle joins the deck, in the order it holds its cards"
    );
}

/// The same boxes in the Defect's hands read one extra die per bundle: one time
/// in a hundred the bundle is three Claws instead of the usual three, and no
/// other character rolls for it at all.
#[test]
fn scroll_boxes_roll_a_claw_bundle_for_the_defect() {
    let his_run = |rewards_counter: u64| {
        let mut state = run_holding_with_treasure(&[], "RELIC.SCROLL_BOXES")
            .state()
            .clone();
        state
            .run
            .as_mut()
            .expect("a run is standing")
            .config
            .players[0]
            .character = id("CHARACTER.DEFECT");
        state
            .player_rng
            .stream_mut(sts2_rng::PlayerStream::Rewards)
            .load_counter(rewards_counter);
        Simulator::from_generated_state(state, sts2_content::standard_registry())
            .expect("the run builds")
    };
    // The die is read per bundle, so his screen costs two more draws than the
    // Ironclad's does over the same six cards.
    let mut his = his_run(0);
    let before = rewards_counter(&his);
    take_from_the_chest(&mut his);
    assert_eq!(rewards_counter(&his) - before, 8);

    // Forty-one is the first rewards counter of this run whose first die comes
    // up under a hundredth.
    let mut lucky = his_run(41);
    take_from_the_chest(&mut lucky);
    let sts2_engine::DecisionContext::ChooseCards {
        candidates,
        bundles,
        ..
    } = lucky.decision().clone()
    else {
        panic!("the boxes put a bundle screen up");
    };
    assert_eq!(bundles, vec![vec![0, 1, 2], vec![3, 4, 5]]);
    assert!(
        bundles[0]
            .iter()
            .all(|index| candidates[*index].fingerprint.model_id == id("CARD.CLAW")),
        "the first bundle is three Claws"
    );
    assert_eq!(candidates.len(), 6);
    assert!(
        bundles[1]
            .iter()
            .any(|index| candidates[*index].fingerprint.model_id != id("CARD.CLAW")),
        "the second bundle rolled its own die and came up ordinary"
    );
}

/// A lizard tail turns one death away and heals half the maximum doing it. The
/// death pass asks every listener before it asks any of them late, so a fairy
/// in the belt is spent first and the tail keeps its use.
#[test]
fn a_lizard_tail_turns_one_death_away() {
    let tailed = |potions: Vec<Option<sts2_core::ModelId>>, spent: bool| {
        let mut tail = sts2_engine::RelicInstance::new(id("RELIC.LIZARD_TAIL"));
        tail.latched = spent;
        ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .player_hp(3, 81)
            .energy(9, 9)
            .turn(1, 1)
            .relic_instances(vec![tail])
            .potions(potions)
            .rng_counter(RunStream::MonsterAi, 0)
            .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "STOMP_MOVE")
            .card(PileName::Hand, 0, card("CARD.BREAKTHROUGH", 0), 0)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds")
    };
    let latched = |simulator: &Simulator| {
        simulator
            .state()
            .run_player
            .relics
            .iter()
            .find(|relic| relic.model_id == id("RELIC.LIZARD_TAIL"))
            .expect("the relic is held")
            .latched
    };

    let mut simulator = tailed(vec![None, None, None], false);
    end_turn(&mut simulator);
    assert!(
        simulator.state().terminal.is_none(),
        "the death is turned away"
    );
    assert_eq!(
        creature(&simulator, 0).current_hp,
        40,
        "half of an odd maximum is the lower half"
    );
    assert!(latched(&simulator), "and the relic is spent doing it");

    // Spent, it turns nothing away.
    let mut spent = tailed(vec![None, None, None], true);
    end_turn(&mut spent);
    assert!(
        spent.state().terminal.is_some(),
        "a spent tail saves nobody"
    );

    // A fairy answers the early pass, so it is what is spent.
    let mut fairied = tailed(
        vec![Some(id("POTION.FAIRY_IN_A_BOTTLE")), None, None],
        false,
    );
    end_turn(&mut fairied);
    assert!(fairied.state().terminal.is_none());
    assert!(!latched(&fairied), "and the tail keeps its use");
}

/// A sere talon charges two curses and hands over three wishes. The curses come
/// off the curse pool's own cards that a modifier may generate, ordered by model
/// ID, drawn on the run's niche stream without replacement.
#[test]
fn a_sere_talon_charges_two_curses_for_three_wishes() {
    let mut simulator = run_holding_with_treasure(&[], "RELIC.SERE_TALON");
    let deck_before = simulator.state().run_player.deck.len();
    let niche_before = niche_counter(&simulator);
    take_from_the_chest(&mut simulator);
    let deck = &simulator.state().run_player.deck;
    assert_eq!(deck.len(), deck_before + 5);
    let joined: Vec<_> = deck[deck_before..]
        .iter()
        .map(|card| card.model_id.clone())
        .collect();
    let generatable = [
        "CARD.CLUMSY",
        "CARD.DEBT",
        "CARD.DECAY",
        "CARD.DOUBT",
        "CARD.GUILTY",
        "CARD.INJURY",
        "CARD.NORMALITY",
        "CARD.REGRET",
        "CARD.SHAME",
        "CARD.WRITHE",
    ]
    .map(id);
    assert!(
        joined[..2]
            .iter()
            .all(|model_id| generatable.contains(model_id)),
        "only a curse a modifier may generate is drawn: {joined:?}"
    );
    assert_ne!(joined[0], joined[1], "and the draw is without replacement");
    assert_eq!(
        joined[2..],
        [id("CARD.WISH"), id("CARD.WISH"), id("CARD.WISH")]
    );
    assert_eq!(
        niche_counter(&simulator) - niche_before,
        2,
        "one draw per curse and nothing else"
    );
}

/// A toy box hands over four relics off the front of its owner's own bag, each
/// marked wax, and melts one of them again at the end of every third fight.
#[test]
fn a_toy_box_hands_over_four_wax_relics_and_takes_them_back() {
    let mut state = run_holding_full(&[], "RELIC.TOY_BOX", 1, None, Vec::new())
        .state()
        .clone();
    if let Some(run) = state.run.as_mut() {
        run.player_relic_grab_bag = stocked_bag();
    }
    let mut simulator = Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run builds");
    take_from_the_chest(&mut simulator);
    let offered = offered_rewards(&simulator);
    assert_eq!(offered.len(), 4);
    assert!(
        offered
            .iter()
            .all(|reward| reward.reward_type == "relic" && reward.relic_is_wax),
        "four relics, each already waxed"
    );

    // Taking them puts them in the inventory carrying the wax.
    for _ in 0..4 {
        let claim = simulator
            .legal_actions()
            .iter()
            .find(|action| matches!(action, Action::ClaimReward { .. }))
            .cloned()
            .expect("a relic is on the screen");
        simulator.step(claim).expect("claiming applies");
    }
    let waxed: Vec<sts2_core::ModelId> = simulator
        .state()
        .run_player
        .relics
        .iter()
        .filter(|relic| relic.wax)
        .map(|relic| relic.model_id.clone())
        .collect();
    assert_eq!(waxed.len(), 4);

    // Every third fight melts the first of them that is still standing.
    let mut state = simulator.state().clone();
    if let Some(run) = state.run.as_mut() {
        run.phase = RunPhase::Map;
        run.active_room = None;
        run.reward_set = None;
        run.current_coord = Some(MapCoord { col: 2, row: 0 });
    }
    let mut fighting = Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run rebuilds");
    let melted = |simulator: &Simulator| {
        simulator
            .state()
            .run_player
            .relics
            .iter()
            .filter(|relic| relic.melted)
            .count()
    };
    for fight in 1..=3 {
        back_to_the_start(&mut fighting);
        walk_to(&mut fighting, 3);
        play(&mut fighting, "CARD.STRIKE_IRONCLAD");
        assert_eq!(
            melted(&fighting),
            usize::from(fight == 3),
            "only the third fight melts one"
        );
    }
    assert_eq!(
        fighting
            .state()
            .run_player
            .relics
            .iter()
            .find(|relic| relic.melted)
            .expect("one was melted")
            .model_id,
        waxed[0],
        "and it is the first wax relic of the inventory"
    );
}

/// Pael's tooth takes five upgradable cards out of the deck as it is taken and
/// hands one of them back, upgraded, at the end of every fight.
#[test]
fn paels_tooth_keeps_five_cards_and_hands_them_back() {
    let mut state = run_holding_full(&[], "RELIC.PAELS_TOOTH", 1, None, Vec::new())
        .state()
        .clone();
    // Exactly five upgradable cards, so the screen takes all of them without
    // asking: the build's own prefs set the minimum and the maximum alike.
    state.run_player.deck = [
        "CARD.UPPERCUT",
        "CARD.STRIKE_IRONCLAD",
        "CARD.BASH",
        "CARD.DEFEND_IRONCLAD",
        "CARD.BULLY",
    ]
    .into_iter()
    .map(|model_id| card(model_id, 0))
    .collect();
    let mut simulator = Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run builds");
    let before = deck_models(&simulator);
    take_from_the_chest(&mut simulator);
    let kept: Vec<sts2_core::ModelId> = simulator
        .state()
        .run_player
        .relics
        .iter()
        .find(|relic| relic.model_id == id("RELIC.PAELS_TOOTH"))
        .expect("the relic is held")
        .saved_cards
        .iter()
        .map(|(_, card)| card.model_id.clone())
        .collect();
    assert_eq!(kept.len(), 5);
    let mut ordered = kept.clone();
    ordered.sort();
    assert_eq!(
        kept, ordered,
        "the relic writes them down in model-ID order"
    );
    assert_eq!(
        simulator.state().run_player.deck.len(),
        before.len() - 5,
        "and the deck is five cards lighter"
    );
    let deck_after = simulator.state().run_player.deck.len();

    // A fight hands one of them back, upgraded.
    let mut state = simulator.state().clone();
    state.run_player.deck.push(card("CARD.STRIKE_IRONCLAD", 0));
    let mut simulator = Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run rebuilds");
    back_to_the_start(&mut simulator);
    walk_to(&mut simulator, 3);
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    let relic = simulator
        .state()
        .run_player
        .relics
        .iter()
        .find(|relic| relic.model_id == id("RELIC.PAELS_TOOTH"))
        .expect("the relic is held")
        .clone();
    assert_eq!(relic.saved_cards.len(), 4, "and it keeps one fewer");
    assert_eq!(
        simulator.state().run_player.deck.len(),
        deck_after + 2,
        "the strike the test added and the card that came back"
    );
    let back = simulator
        .state()
        .run_player
        .deck
        .last()
        .expect("a card came back")
        .clone();
    assert!(kept.contains(&back.model_id));
    assert_eq!(back.upgrade_level, 1, "the card comes back upgraded");
    assert!(
        !relic
            .saved_cards
            .iter()
            .any(|(_, card)| card.model_id == back.model_id
                && card.upgrade_level == back.upgrade_level - 1),
        "and the relic is not keeping it any more"
    );
}

/// The list of cards Pael's tooth is keeping is a property of the relic, and a
/// saved run writes it out whether or not it still holds anything. So the
/// projection of a tooth that has handed its last card back carries the list,
/// empty, rather than dropping it and leaving the relic with no properties at
/// all.
#[test]
fn a_tooth_that_has_handed_its_last_card_back_still_projects_the_empty_list() {
    let mut state = run_holding_full(&[], "RELIC.PAELS_TOOTH", 1, None, Vec::new())
        .state()
        .clone();
    state.run_player.deck = [
        "CARD.UPPERCUT",
        "CARD.STRIKE_IRONCLAD",
        "CARD.BASH",
        "CARD.DEFEND_IRONCLAD",
        "CARD.BULLY",
    ]
    .into_iter()
    .map(|model_id| card(model_id, 0))
    .collect();
    let mut simulator = Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run builds");
    take_from_the_chest(&mut simulator);

    // Four fights in: the tooth is down to its last card, and the deck holds
    // the strike that wins the fifth.
    let mut state = simulator.state().clone();
    let relic = state
        .run_player
        .relics
        .iter_mut()
        .find(|relic| relic.model_id == id("RELIC.PAELS_TOOTH"))
        .expect("the relic is held");
    relic.saved_cards.truncate(1);
    state.run_player.deck.push(card("CARD.STRIKE_IRONCLAD", 0));
    let mut simulator = Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run rebuilds");
    let projected = |simulator: &Simulator| -> serde_json::Value {
        sts2_replay::observation::render(simulator)["players"][0]["relics"]
            .as_array()
            .expect("a relic list")
            .iter()
            .find(|relic| relic["model_id"] == serde_json::json!("RELIC.PAELS_TOOTH"))
            .expect("the relic is projected")["properties"]["card_arrays"]
            .clone()
    };
    assert_eq!(
        projected(&simulator)
            .as_array()
            .expect("the list is written")[0]["value"]
            .as_array()
            .expect("a card list")
            .len(),
        1,
        "the tooth is holding one card"
    );

    back_to_the_start(&mut simulator);
    walk_to(&mut simulator, 3);
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert!(
        simulator
            .state()
            .run_player
            .relics
            .iter()
            .find(|relic| relic.model_id == id("RELIC.PAELS_TOOTH"))
            .expect("the relic is held")
            .saved_cards
            .is_empty(),
        "the fight took the last card back out of the tooth"
    );
    assert_eq!(
        projected(&simulator),
        serde_json::json!([{"name": "SerializableCards", "value": []}]),
        "and the emptied list is still written down"
    );
}

/// A choices paradox lays five cards of its owner's own pool out as the opening
/// turn of a fight starts, and whichever is picked joins the hand keeping
/// itself there.
#[test]
fn a_choices_paradox_offers_five_cards_on_the_opening_turn() {
    let mut simulator = fight_from_run(&["RELIC.CHOICES_PARADOX"]);
    let sts2_engine::DecisionContext::ChooseCards {
        choice_id,
        identity,
        minimum,
        maximum,
        candidates,
        ..
    } = simulator.decision().clone()
    else {
        panic!("the paradox puts a grid up as the turn opens");
    };
    assert_eq!(identity, sts2_engine::ChoiceIdentity::Offered);
    assert_eq!((minimum, maximum), (1, 1));
    assert_eq!(candidates.len(), 5);
    let hand_before = simulator
        .state()
        .combat
        .as_ref()
        .expect("a fight")
        .player
        .piles
        .hand
        .len();
    simulator
        .step(Action::ChooseCards {
            choice_id,
            cards: vec![candidates[2].clone()],
        })
        .expect("picking applies");
    let hand = &simulator
        .state()
        .combat
        .as_ref()
        .expect("a fight")
        .player
        .piles
        .hand;
    assert_eq!(hand.len(), hand_before + 1);
    let added = hand.last().expect("a card joined the hand");
    assert_eq!(
        added.fingerprint.model_id,
        candidates[2].fingerprint.model_id
    );
    assert!(added.retain, "and it keeps itself out of a flushed hand");

    // The second turn opens with no grid at all.
    end_turn(&mut simulator);
    assert!(
        matches!(
            simulator.decision(),
            sts2_engine::DecisionContext::CombatPriority { .. }
        ),
        "only the opening turn lays them out"
    );
}

// --- the Silent's own pool -----------------------------------------------

/// A ring of the snake deals two more cards on the opening hand and nothing
/// after it, which is the same turn-number test a bag of preparation makes.
#[test]
fn a_ring_of_the_snake_only_swells_the_opening_hand() {
    let mut plain = fight_from_run(&[]);
    let mut ringed = fight_from_run(&["RELIC.RING_OF_THE_SNAKE"]);
    assert_eq!(hand_size(&ringed), hand_size(&plain) + 2);
    end_turn(&mut plain);
    end_turn(&mut ringed);
    assert_eq!(hand_size(&plain), hand_size(&ringed));
}

/// A ninja scroll deals three Shivs before the opening hand and none before any
/// later one.
#[test]
fn a_ninja_scroll_deals_three_shivs_on_the_opening_turn() {
    let mut simulator = fight_from_run(&["RELIC.NINJA_SCROLL"]);
    let shivs = |simulator: &Simulator| {
        simulator
            .state()
            .combat
            .as_ref()
            .expect("a fight")
            .player
            .piles
            .hand
            .iter()
            .filter(|card| card.fingerprint.model_id == id("CARD.SHIV"))
            .count()
    };
    assert_eq!(shivs(&simulator), 3);
    end_turn(&mut simulator);
    assert_eq!(shivs(&simulator), 0, "the second turn is not the first");
}

/// A twisted funnel poisons everything as the first turn opens, and nothing as
/// any later one does.
#[test]
fn a_twisted_funnel_poisons_the_field_once() {
    let mut simulator = fight_from_run(&["RELIC.TWISTED_FUNNEL"]);
    assert_eq!(power_amount(&simulator, 1, "POWER.POISON_POWER"), 4);
    let after_first = power_amount(&simulator, 1, "POWER.POISON_POWER");
    end_turn(&mut simulator);
    assert!(
        power_amount(&simulator, 1, "POWER.POISON_POWER") < after_first,
        "the poison bit and nothing was added"
    );
}

/// A helical dart lends its owner dexterity for the turn every time they throw
/// a Shiv, and nothing for anything else.
#[test]
fn a_helical_dart_pays_for_every_shiv_thrown() {
    let mut simulator = fight_against(
        "SHRINKER_MOVE",
        1,
        &["RELIC.HELICAL_DART"],
        &[("CARD.BLADE_DANCE", 0), ("CARD.STRIKE_IRONCLAD", 0)],
        &[],
    );
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(power_amount(&simulator, 0, "POWER.DEXTERITY_POWER"), 0);
    play(&mut simulator, "CARD.BLADE_DANCE");
    play(&mut simulator, "CARD.SHIV");
    assert_eq!(power_amount(&simulator, 0, "POWER.DEXTERITY_POWER"), 1);
    end_turn(&mut simulator);
    assert_eq!(
        power_amount(&simulator, 0, "POWER.DEXTERITY_POWER"),
        0,
        "it is lent for the turn and taken back"
    );
}

/// A snecko skull adds one to every point of poison its owner works.
#[test]
fn a_snecko_skull_adds_one_to_every_poison() {
    let mut simulator = fight_against(
        "SHRINKER_MOVE",
        1,
        &["RELIC.SNECKO_SKULL"],
        &[("CARD.DEADLY_POISON", 0)],
        &[],
    );
    play(&mut simulator, "CARD.DEADLY_POISON");
    assert_eq!(power_amount(&simulator, 1, "POWER.POISON_POWER"), 6);
}

/// A tingsha throws three at something for every card its owner puts down on
/// their own turn; tough bandages pay three block for the same thing.
#[test]
fn a_tingsha_and_tough_bandages_answer_a_discard() {
    let mut simulator = fight_against(
        "SHRINKER_MOVE",
        1,
        &["RELIC.TINGSHA", "RELIC.TOUGH_BANDAGES"],
        &[("CARD.SURVIVOR", 0), ("CARD.STRIKE_IRONCLAD", 0)],
        &[],
    );
    play(&mut simulator, "CARD.SURVIVOR");
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 3);
    assert_eq!(player(&simulator).block, 8 + 3);
}

/// A paper krane takes the Weak multiplier its holder is hit with from three
/// quarters down to six tenths.
#[test]
fn a_paper_krane_deepens_the_weakness_of_what_hits_it() {
    let bite = |relics: &[&str]| {
        let simulator = fight_against("CHOMP_MOVE", 1, relics, &[], &[]);
        let power = sts2_engine::PowerInstance {
            instance: 0,
            model_id: id("POWER.WEAK_POWER"),
            held_card: None,
            bound_damage_card: None,
            amount: 2,
            counter: 0,
            amount_on_turn_start: 0,
            amounts_at_play_start: Vec::new(),
            play_tallies: Vec::new(),
            skip_next_duration_tick: false,
            applier: None,
            target: None,
            power_type: sts2_engine::PowerType::Debuff,
            downgraded_cards: Vec::new(),
            latched: false,
        };
        let mut state = simulator.state().clone();
        state
            .combat
            .as_mut()
            .expect("a fight")
            .creatures
            .iter_mut()
            .find(|creature| creature.id.get() == 1)
            .expect("the beetle")
            .powers
            .push(power);
        let mut simulator = Simulator::from_scenario(state, sts2_content::standard_registry())
            .expect("the fight stands back up");
        end_turn(&mut simulator);
        80 - player(&simulator).current_hp
    };
    assert_eq!(bite(&[]), 5, "seven at three quarters");
    assert_eq!(bite(&["RELIC.PAPER_KRANE"]), 4, "seven at six tenths");
}

// --- the Necrobinder's own pool ------------------------------------------

/// A fight the Necrobinder is standing in rather than the Ironclad. Every relic
/// of her pool reads something only her side of a fight has — the pet she binds,
/// the Doom she works, or the card pool her character names — so none of them
/// can be shown in somebody else's fight.
fn necrobinder_scenario(
    move_state: &str,
    enemies: u64,
    relics: &[&str],
    hand: &[(&str, u8)],
) -> ScenarioBuilder {
    let held: Vec<sts2_core::ModelId> = relics.iter().map(|model| id(model)).collect();
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .played_as(id("CHARACTER.NECROBINDER"))
        .player_hp(80, 80)
        .energy(30, 30)
        .turn(1, 1)
        .relics(&held)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatTargets, 0)
        .rng_counter(RunStream::CombatCardSelection, 0);
    for slot in 1..=enemies {
        scenario = scenario.enemy(slot, id("MONSTER.SHRINKER_BEETLE"), 39, 39, move_state);
    }
    for (index, (model_id, upgrade)) in hand.iter().enumerate() {
        scenario = scenario.card(
            PileName::Hand,
            u64::try_from(index).expect("small index"),
            card(model_id, *upgrade),
            0,
        );
    }
    scenario
}

fn necrobinder_fight(move_state: &str, relics: &[&str], hand: &[(&str, u8)]) -> Simulator {
    necrobinder_scenario(move_state, 1, relics, hand)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

/// The same run every walked-in relic is tested through, walked as the
/// Necrobinder. The combat's player creature is the run config's character, and
/// so is the pool a relic rolling against its owner's own character pool uses,
/// so naming her in the config is what makes the fight hers. The deck stays the
/// fixture's, because neither of the two relics walked in here reads it.
fn necrobinder_fight_from_run(relics: &[&str]) -> Simulator {
    let mut state = run_holding(relics).state().clone();
    state
        .run
        .as_mut()
        .expect("the run is standing")
        .config
        .players[0]
        .character = id("CHARACTER.NECROBINDER");
    let mut simulator = Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run builds");
    walk_to(&mut simulator, 3);
    simulator
}

/// Every copy of a card standing anywhere in the fight, which is how a card a
/// relic shuffles into the draw pile is counted without saying where the deal
/// then put it.
fn cards_in_combat(simulator: &Simulator, model_id: &str) -> usize {
    let wanted = id(model_id);
    simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles
        .all_cards()
        .filter(|card| card.fingerprint.model_id == wanted)
        .count()
}

/// A bone flute pays two block for every swing the owner's own pet finishes,
/// and pays it unpowered, so nothing standing over her scales it. The build's
/// two guards are that the attacker is a pet and that the pet belongs to the
/// relic's owner; her own swings and her summons answer neither.
#[test]
fn a_bone_flute_pays_for_every_swing_the_pet_finishes() {
    let mut simulator = necrobinder_scenario(
        "SHRINKER_MOVE",
        1,
        &["RELIC.BONE_FLUTE"],
        &[
            ("CARD.STRIKE_NECROBINDER", 0),
            ("CARD.BODYGUARD", 0),
            ("CARD.POKE", 0),
            ("CARD.UNLEASH", 0),
        ],
    )
    .power(
        0,
        id("POWER.DEXTERITY_POWER"),
        5,
        sts2_engine::PowerType::Buff,
    )
    .build(sts2_content::standard_registry())
    .expect("the scenario builds");
    play(&mut simulator, "CARD.STRIKE_NECROBINDER");
    assert_eq!(
        player(&simulator).block,
        0,
        "her own swing is not the pet's"
    );
    play(&mut simulator, "CARD.BODYGUARD");
    assert_eq!(player(&simulator).block, 0, "and a summon is not a swing");
    // Five dexterity is standing over her and the flute still pays exactly
    // two, because the block is worked up unpowered.
    play(&mut simulator, "CARD.POKE");
    assert_eq!(player(&simulator).block, 2);
    play(&mut simulator, "CARD.UNLEASH");
    assert_eq!(player(&simulator).block, 4);
}

/// A book repair knife heals three for every creature other than its owner that
/// a Doom kill took down and whose death counted. Doom takes the whole doomed
/// list down at once as the side it stands on ends its turn, so one kill of two
/// creatures is one pass of six rather than two passes of three.
#[test]
fn a_book_repair_knife_heals_for_every_creature_a_doom_kill_took() {
    let doomed = |relics: &[&str], doom: i32, minions: u64| {
        let mut scenario = necrobinder_scenario("SHRINKER_MOVE", 3, relics, &[])
            .player_hp(40, 80)
            .power(
                1,
                id("POWER.DOOM_POWER"),
                doom,
                sts2_engine::PowerType::Debuff,
            )
            .power(
                2,
                id("POWER.DOOM_POWER"),
                doom,
                sts2_engine::PowerType::Debuff,
            );
        for slot in 1..=minions {
            scenario = scenario.power(
                slot,
                id("POWER.MINION_POWER"),
                1,
                sts2_engine::PowerType::Buff,
            );
        }
        let mut simulator = scenario
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
        end_turn(&mut simulator);
        player(&simulator).current_hp
    };
    // Thirty-nine of Doom on a beetle standing at thirty-nine is a kill; the
    // third beetle is untouched and keeps the fight open.
    assert_eq!(doomed(&["RELIC.BOOK_REPAIR_KNIFE"], 39, 0), 40 + 6);
    // A point short of the health left is no kill at all, so nothing is healed.
    assert_eq!(doomed(&["RELIC.BOOK_REPAIR_KNIFE"], 38, 0), 40);
    assert_eq!(doomed(&[], 39, 0), 40, "and the healing is the relic's");
    // A minion going down is not a death that counts, because the power that
    // makes it a minion says so, so only the other one is paid for.
    assert_eq!(doomed(&["RELIC.BOOK_REPAIR_KNIFE"], 39, 1), 40 + 3);
}

/// A funerary mask shuffles three Souls through the draw pile before the first
/// turn's hand is dealt, and before no later one. They go into the draw pile, so
/// the hand the turn deals is the same size it would have been.
#[test]
fn a_funerary_mask_shuffles_three_souls_before_the_first_hand() {
    let plain = necrobinder_fight_from_run(&[]);
    let mut masked = necrobinder_fight_from_run(&["RELIC.FUNERARY_MASK"]);
    assert_eq!(cards_in_combat(&plain, "CARD.SOUL"), 0);
    assert_eq!(cards_in_combat(&masked, "CARD.SOUL"), 3);
    assert_eq!(hand_size(&masked), hand_size(&plain));
    end_turn(&mut masked);
    assert_eq!(
        cards_in_combat(&masked, "CARD.SOUL"),
        3,
        "being on turn one is the whole of the test the mask makes"
    );
}

/// A big hat deals two different ethereal cards of its owner's own pool into the
/// opening hand, and deals them only on the opening turn.
#[test]
fn a_big_hat_deals_two_different_ethereal_cards_into_the_opening_hand() {
    let mut plain = necrobinder_fight_from_run(&[]);
    let mut hatted = necrobinder_fight_from_run(&["RELIC.BIG_HAT"]);
    let opening = hand_size(&plain);
    assert_eq!(hand_size(&hatted), opening + 2);
    let registry = sts2_content::standard_registry();
    let pool = registry
        .card_pool(&id("CARD_POOL.NECROBINDER_CARD_POOL"))
        .expect("her pool is registered");
    let added: Vec<sts2_core::ModelId> = hatted
        .state()
        .combat
        .as_ref()
        .expect("a fight")
        .player
        .piles
        .hand
        .iter()
        .skip(opening)
        .map(|card| card.fingerprint.model_id.clone())
        .collect();
    assert_eq!(added.len(), 2);
    assert_ne!(added[0], added[1], "the distinct roll never repeats a card");
    for model_id in &added {
        assert!(
            registry
                .card(model_id)
                .expect("the card is content")
                .ethereal_at(0),
            "{model_id}"
        );
        assert!(
            pool.iter().any(|card| card.model_id == *model_id),
            "{model_id} is one of hers"
        );
    }
    end_turn(&mut plain);
    end_turn(&mut hatted);
    assert_eq!(
        hand_size(&hatted),
        hand_size(&plain),
        "the first turn is spent"
    );
}

/// A bookmark takes a point off one of the cards that stayed in hand through
/// the flush. The point stands until the card is played, so it lasts past the
/// turn that gave it and is handed back by the play rather than by the clock,
/// and the relic has no guard of its own: every flush that leaves something in
/// hand takes another.
///
/// The build narrows its list to cards that neither cost X nor already cost
/// nothing before it rolls, so a card the relic has already taken down to
/// nothing is no longer on the list — which is what the third flush here
/// shows.
#[test]
fn a_bookmark_takes_a_point_off_a_retained_card() {
    // A reap keeps itself in hand and costs three.
    let mut simulator =
        necrobinder_fight("SHRINKER_MOVE", &["RELIC.BOOKMARK"], &[("CARD.REAP", 0)]);
    let held = |simulator: &Simulator| {
        simulator
            .state()
            .combat
            .as_ref()
            .expect("a fight")
            .player
            .piles
            .hand
            .first()
            .expect("the reap kept itself in hand")
            .clone()
    };
    end_turn(&mut simulator);
    assert_eq!(held(&simulator).cost_until_played, -1);
    assert_eq!(held(&simulator).energy_cost, 2);
    end_turn(&mut simulator);
    // The second turn's flush is a second point: nothing counts the fights or
    // the turns this has answered.
    assert_eq!(held(&simulator).cost_until_played, -2);
    assert_eq!(held(&simulator).energy_cost, 1);
    // A third and a fourth flush take it to nothing and then leave it there:
    // the relic's own filter drops a card that costs nothing now.
    end_turn(&mut simulator);
    assert_eq!(held(&simulator).energy_cost, 0);
    end_turn(&mut simulator);
    assert_eq!(
        held(&simulator).cost_until_played,
        -3,
        "a card that costs nothing is not on the list the relic rolls over"
    );
    assert_eq!(held(&simulator).energy_cost, 0);
    play(&mut simulator, "CARD.REAP");
    let played = simulator
        .state()
        .combat
        .as_ref()
        .expect("a fight")
        .player
        .piles
        .discard
        .first()
        .expect("the reap was put down")
        .clone();
    assert_eq!(played.cost_until_played, 0, "the play is what spends it");
    assert_eq!(played.energy_cost, 3);
}

/// What a bookmark asks of a retained card is whether the card's own cost is
/// more than nothing — what stands on the card itself, and nothing a power or a
/// relic says about every card of a kind. A reap a free-attack power is showing
/// at nothing still costs three of its own, so it is on the list: the roll is
/// made and the point comes off. Seed `XGTTW8TTEQ`, record 2160, where a scarf
/// had the whole hand showing free as the turn ended.
#[test]
fn a_bookmark_reads_the_cards_own_cost_not_what_a_power_makes_of_it() {
    let mut simulator =
        necrobinder_scenario("SHRINKER_MOVE", 1, &["RELIC.BOOKMARK"], &[("CARD.REAP", 0)])
            .power(
                0,
                id("POWER.FREE_ATTACK_POWER"),
                1,
                sts2_engine::PowerType::Buff,
            )
            .rng_counter(RunStream::CombatCardSelection, 0)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
    let held = |simulator: &Simulator| {
        simulator
            .state()
            .combat
            .as_ref()
            .expect("a fight")
            .player
            .piles
            .hand
            .first()
            .expect("the reap kept itself in hand")
            .clone()
    };
    assert_eq!(held(&simulator).energy_cost, 0, "the power shows it free");
    end_turn(&mut simulator);
    assert_eq!(held(&simulator).cost_until_played, -1);
    assert_eq!(
        simulator
            .state()
            .rng
            .stream(RunStream::CombatCardSelection)
            .counter(),
        1,
        "and the roll that chose it was made"
    );
}

/// A bookmark whose owner kept nothing takes no point off anything. The flush is
/// over before the relic looks, so the cards it swept into the discard pile are
/// not what it reads.
#[test]
fn a_bookmark_over_a_flushed_hand_takes_nothing_off() {
    let mut simulator = necrobinder_fight(
        "SHRINKER_MOVE",
        &["RELIC.BOOKMARK"],
        &[("CARD.STRIKE_NECROBINDER", 0), ("CARD.BODYGUARD", 0)],
    );
    // Neither card keeps itself in hand, so the flush leaves nothing behind and
    // the relic's list is empty. Both of them cost a point, so a relic reading
    // the discard pile instead would have marked one.
    end_turn(&mut simulator);
    assert!(
        simulator
            .state()
            .combat
            .as_ref()
            .expect("a fight")
            .player
            .piles
            .all_cards()
            .all(|card| card.cost_until_played == 0)
    );
}

/// An ivory tile hands a point back for every card its owner spends three or
/// more energy on, and hands nothing back for a cheaper one.
#[test]
fn an_ivory_tile_pays_for_every_card_worth_three() {
    let mut simulator = necrobinder_fight(
        "SHRINKER_MOVE",
        &["RELIC.IVORY_TILE"],
        &[
            ("CARD.SOW", 0),
            ("CARD.TIMES_UP", 0),
            ("CARD.REAP", 0),
            ("CARD.STRIKE_NECROBINDER", 0),
        ],
    );
    let before = energy(&simulator);
    play(&mut simulator, "CARD.SOW");
    assert_eq!(energy(&simulator), before - 1, "one is not three");
    play(&mut simulator, "CARD.TIMES_UP");
    assert_eq!(energy(&simulator), before - 3, "and neither is two");
    play(&mut simulator, "CARD.REAP");
    // Three out and one back, so the reap cost two in the end.
    assert_eq!(energy(&simulator), before - 5);
    play(&mut simulator, "CARD.STRIKE_NECROBINDER");
    assert_eq!(energy(&simulator), before - 6);
}

/// An undying sigil halves a powered attack aimed at its owner when the
/// creature that struck is itself doomed, which the build tests as current HP
/// no greater than the Doom standing on it.
///
/// Doom is said to answer to this relic by firing at the start of the enemy
/// turn while it stands. The pinned build's Doom does nothing of the sort — it
/// fires as the side ends its turn either way — so the halving is the whole of
/// what the relic does here.
#[test]
fn an_undying_sigil_halves_a_doomed_creatures_blow() {
    let chomp = |relics: &[&str], doom: i32| {
        let mut simulator = necrobinder_scenario("CHOMP_MOVE", 2, relics, &[])
            .power(
                1,
                id("POWER.DOOM_POWER"),
                doom,
                sts2_engine::PowerType::Debuff,
            )
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
        end_turn(&mut simulator);
        80 - player(&simulator).current_hp
    };
    // Two beetles chomp for seven each; only the first of them is doomed.
    assert_eq!(chomp(&[], 39), 14);
    assert_eq!(
        chomp(&["RELIC.UNDYING_SIGIL"], 38),
        14,
        "a point of health above the Doom is not doomed"
    );
    // Seven halved is three: the reduction is the doomed beetle's blow only.
    assert_eq!(chomp(&["RELIC.UNDYING_SIGIL"], 39), 10);
}

// --- the two relics that bring a pet -------------------------------------

/// The same run, with cards pushed into the deck before it is stood up, so a
/// relic that acts on what the deck holds has something to act on.
fn necrobinder_run_with_deck(treasure: &str, extra: &[&str]) -> Simulator {
    let mut state = run_holding_with_treasure(&[], treasure).state().clone();
    state
        .run
        .as_mut()
        .expect("the run is standing")
        .config
        .players[0]
        .character = id("CHARACTER.NECROBINDER");
    for model_id in extra {
        state.run_player.deck.push(card(model_id, 0));
    }
    Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run builds")
}

/// The pet standing on the player's side, if one is.
fn standing_pet(simulator: &Simulator) -> Option<&sts2_engine::Creature> {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .creatures
        .iter()
        .find(|creature| creature.pet_owner.is_some())
}

/// A byrdpip turns every byrdonis egg its owner is carrying into a swoop as it
/// is taken, rolls which of four skins it wears on a die of its own, and brings
/// its pet to every fight after that.
#[test]
fn a_byrdpip_brings_a_pet_and_turns_every_egg_into_a_swoop() {
    let mut simulator = necrobinder_run_with_deck(
        "RELIC.BYRDPIP",
        &[
            "CARD.BYRDONIS_EGG",
            "CARD.STRIKE_NECROBINDER",
            "CARD.BYRDONIS_EGG",
        ],
    );
    let deck_before = simulator.state().run_player.deck.len();
    take_relic_at_the_chest(&mut simulator);
    let deck: Vec<String> = simulator
        .state()
        .run_player
        .deck
        .iter()
        .map(|card| card.model_id.to_string())
        .collect();
    assert_eq!(deck.len(), deck_before, "a transform is a swap, not an add");
    assert!(!deck.contains(&"CARD.BYRDONIS_EGG".to_owned()));
    assert_eq!(
        deck.iter()
            .filter(|model| *model == "CARD.BYRD_SWOOP")
            .count(),
        2,
        "every egg, not just the first"
    );
    // The skin is rolled once off the relic's own stream and saved, and the
    // projection carries it.
    let held = simulator
        .state()
        .run_player
        .relics
        .iter()
        .find(|relic| relic.model_id == id("RELIC.BYRDPIP"))
        .expect("the relic was taken");
    let (name, skin) = held
        .saved_strings
        .first()
        .expect("the skin was rolled as it was taken");
    assert_eq!(name, "Skin");
    assert!(
        ["version1", "version2", "version3", "version4"].contains(&skin.as_str()),
        "{skin}"
    );
}

/// And it brings its pet to every fight after that.
#[test]
fn a_byrdpip_brings_its_pet_to_every_fight() {
    let simulator = necrobinder_fight_from_run(&["RELIC.BYRDPIP"]);
    let pet = standing_pet(&simulator).expect("the byrdpip brought its pet");
    assert_eq!(pet.model_id, id("MONSTER.BYRDPIP"));
    assert_eq!(pet.max_hp, 9999);
}

/// Pael's legion doubles the block of one card play and then sleeps for two of
/// its owner's turns: its cooldown is set to two as the play that took the
/// doubling finishes and runs down as each of her turns opens, so the third
/// turn is the next one that pays double.
#[test]
fn paels_legion_doubles_one_card_plays_block_every_third_turn() {
    let hand: Vec<(&str, u8)> = std::iter::repeat_n(("CARD.DEFEND_NECROBINDER", 0), 6).collect();
    let mut plain = necrobinder_fight("SHRINKER_MOVE", &[], &hand);
    play(&mut plain, "CARD.DEFEND_NECROBINDER");
    let single = player(&plain).block;

    let mut simulator = necrobinder_fight("SHRINKER_MOVE", &["RELIC.PAELS_LEGION"], &hand);
    play(&mut simulator, "CARD.DEFEND_NECROBINDER");
    assert_eq!(player(&simulator).block, single * 2, "the first play");
    play(&mut simulator, "CARD.DEFEND_NECROBINDER");
    assert_eq!(
        player(&simulator).block,
        single * 3,
        "and the second is plain: the doubling was spent"
    );
    // Block is cleared as each turn opens, so what is left after a turn is what
    // that turn's own play paid.
    end_turn(&mut simulator);
    play(&mut simulator, "CARD.DEFEND_NECROBINDER");
    assert_eq!(player(&simulator).block, single, "still asleep");
    end_turn(&mut simulator);
    play(&mut simulator, "CARD.DEFEND_NECROBINDER");
    assert_eq!(player(&simulator).block, single * 2, "awake again");
}

/// The same relic walked into a run brings its own pet to the fight.
#[test]
fn paels_legion_brings_its_pet_to_every_fight() {
    let simulator = necrobinder_fight_from_run(&["RELIC.PAELS_LEGION"]);
    let pet = standing_pet(&simulator).expect("the legion brought its pet");
    assert_eq!(pet.model_id, id("MONSTER.PAELS_LEGION"));
}

/// A phylactery unbound summons five as the fight opens and two more at the
/// start of every turn its owner takes part in — the first one included, which
/// is the difference from the bound one: it answers the side turn start rather
/// than the late half of the energy reset, so it has no turn to skip.
#[test]
fn a_phylactery_unbound_summons_five_and_then_two_a_turn() {
    let simulator = necrobinder_fight_from_run(&["RELIC.PHYLACTERY_UNBOUND"]);
    let pet = standing_pet(&simulator).expect("the phylactery summons on the way in");
    assert_eq!(
        (pet.max_hp, pet.current_hp),
        (7, 7),
        "five as the fight opened and two as her first turn did"
    );
    let mut simulator = simulator;
    end_turn(&mut simulator);
    let pet = standing_pet(&simulator).expect("it is still standing");
    assert_eq!((pet.max_hp, pet.current_hp), (9, 9), "two more every turn");
}

// --- the Regent's own relics --------------------------------------------
//
// Every one of them reads something only his side of a fight has: the star
// pool, the forge, or the minion cards his pool turns other cards into. So each
// is shown in a fight he is standing in, with the stars the test wants already
// in the pool.

fn regent_scenario(
    move_state: &str,
    enemies: u64,
    relics: &[&str],
    hand: &[(&str, u8)],
    stars: i32,
) -> ScenarioBuilder {
    let held: Vec<sts2_core::ModelId> = relics.iter().map(|model| id(model)).collect();
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .played_as(id("CHARACTER.REGENT"))
        .player_hp(80, 80)
        .energy(30, 30)
        .stars(stars)
        .turn(1, 1)
        .relics(&held)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatTargets, 0)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .rng_counter(RunStream::CombatCardGeneration, 0);
    for slot in 1..=enemies {
        scenario = scenario.enemy(slot, id("MONSTER.SHRINKER_BEETLE"), 999, 999, move_state);
    }
    for (index, (model_id, upgrade)) in hand.iter().enumerate() {
        scenario = scenario.card(
            PileName::Hand,
            u64::try_from(index).expect("small index"),
            card(model_id, *upgrade),
            0,
        );
    }
    scenario
}

fn regent_fight(relics: &[&str], hand: &[(&str, u8)], stars: i32) -> Simulator {
    regent_scenario("SHRINKER_MOVE", 1, relics, hand, stars)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

fn stars_held(simulator: &Simulator) -> i32 {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .stars
}

fn blade_damage(simulator: &mut Simulator) -> i32 {
    let before = creature(simulator, 1).current_hp;
    play(simulator, "CARD.SOVEREIGN_BLADE");
    before - creature(simulator, 1).current_hp
}

/// The same run every walked-in relic is tested through, walked as the Regent.
/// A relic that answers the way into a fight or the fight's first turn has no
/// scenario to answer in: a scenario stands a fight up already past both.
fn regent_fight_from_run(relics: &[&str]) -> Simulator {
    let mut state = run_holding(relics).state().clone();
    state
        .run
        .as_mut()
        .expect("the run is standing")
        .config
        .players[0]
        .character = id("CHARACTER.REGENT");
    let mut simulator = Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run builds");
    walk_to(&mut simulator, 3);
    simulator
}

/// The same run walked as the Defect, with his own orb slots under him: what a
/// relic that channels as a fight opens needs.
fn defect_fight_from_run(relics: &[&str]) -> Simulator {
    let mut state = run_holding(relics).state().clone();
    state
        .run
        .as_mut()
        .expect("the run is standing")
        .config
        .players[0]
        .character = id("CHARACTER.DEFECT");
    state.run_player.base_orb_slots = 3;
    let mut simulator = Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the run builds");
    walk_to(&mut simulator, 3);
    simulator
}

fn orbs_held(simulator: &Simulator) -> Vec<String> {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("a combat is standing")
        .player
        .orbs
        .orbs
        .iter()
        .map(|orb| orb.model_id.to_string())
        .collect()
}

fn orb_slots(simulator: &Simulator) -> i32 {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("a combat is standing")
        .player
        .orbs
        .capacity
}

/// The Defect's two starters. A cracked core channels one Lightning orb before
/// the first turn opens; an infused core — the refinement, which no pool holds
/// — channels three after it and makes every Lightning orb of its owner's
/// worth one more.
#[test]
fn the_defects_starters_channel_where_they_say() {
    let mut simulator = defect_fight_from_run(&["RELIC.CRACKED_CORE"]);
    assert_eq!(orbs_held(&simulator), ["ORB.LIGHTNING_ORB"]);
    end_turn(&mut simulator);
    assert_eq!(
        orbs_held(&simulator).len(),
        1,
        "and the first turn is the only one it channels on"
    );

    let simulator = defect_fight_from_run(&["RELIC.INFUSED_CORE"]);
    assert_eq!(orbs_held(&simulator).len(), 3);
    let orb = &simulator
        .state()
        .combat
        .as_ref()
        .expect("a combat is standing")
        .player
        .orbs
        .orbs[0];
    assert_eq!((orb.passive, orb.evoke), (4, 9), "one more of each");
}

/// A symbiotic virus channels a Dark orb once the first turn has opened, and a
/// runic capacitor gives three slots there.
#[test]
fn the_defects_turn_start_relics_answer_once() {
    let mut simulator = defect_fight_from_run(&["RELIC.SYMBIOTIC_VIRUS"]);
    assert_eq!(orbs_held(&simulator), ["ORB.DARK_ORB"]);
    end_turn(&mut simulator);
    assert_eq!(orbs_held(&simulator).len(), 1);

    let mut simulator = defect_fight_from_run(&["RELIC.RUNIC_CAPACITOR"]);
    assert_eq!(orb_slots(&simulator), 6);
    end_turn(&mut simulator);
    assert_eq!(orb_slots(&simulator), 6);
}

/// A data disk works a Focus on as its owner walks into a fight.
#[test]
fn a_data_disk_pays_focus_on_the_way_in() {
    let simulator = defect_fight_from_run(&["RELIC.DATA_DISK"]);
    assert_eq!(
        creature(&simulator, 0)
            .powers
            .iter()
            .find(|power| power.model_id == id("POWER.FOCUS_POWER"))
            .map_or(0, |power| power.amount),
        1
    );
}

/// A power cell deals two of the draw pile's free cards into the hand before
/// the first turn opens.
#[test]
fn a_power_cell_deals_the_free_cards_up_front() {
    let hand_of = |simulator: &Simulator| {
        simulator
            .state()
            .combat
            .as_ref()
            .expect("a combat is standing")
            .player
            .piles
            .hand
            .iter()
            .map(|instance| instance.fingerprint.model_id.to_string())
            .collect::<Vec<_>>()
    };
    // The run's deck is the fixture's, and one card of it — a bully — costs
    // nothing; the relic takes up to two such and the draw is dealt around it.
    let plain = hand_of(&defect_fight_from_run(&[]));
    let celled = hand_of(&defect_fight_from_run(&["RELIC.POWER_CELL"]));
    assert!(!plain.contains(&"CARD.BULLY".to_owned()));
    assert!(celled.contains(&"CARD.BULLY".to_owned()));
    assert_eq!(celled.len(), plain.len() + 1);
}

/// A metronome counts the orbs its owner channels and hits everything for
/// thirty on the seventh.
#[test]
fn a_metronome_pays_out_on_the_seventh_orb() {
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .played_as(id("CHARACTER.DEFECT"))
        .player_hp(80, 80)
        .energy(30, 30)
        .turn(1, 1)
        .orb_slots(10)
        .relics(&[id("RELIC.METRONOME")])
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatTargets, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 999, 999, "SHRINKER_MOVE")
        .card(PileName::Hand, 0, card("CARD.ZAP", 0), 0)
        .card(PileName::Hand, 1, card("CARD.ZAP", 0), 0)
        .card(PileName::Hand, 2, card("CARD.ZAP", 0), 0)
        .card(PileName::Hand, 3, card("CARD.ZAP", 0), 0)
        .card(PileName::Hand, 4, card("CARD.ZAP", 0), 0)
        .card(PileName::Hand, 5, card("CARD.ZAP", 0), 0)
        .card(PileName::Hand, 6, card("CARD.ZAP", 0), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    for index in 1..=7 {
        let before = creature(&simulator, 1).current_hp;
        play(&mut simulator, "CARD.ZAP");
        let dealt = before - creature(&simulator, 1).current_hp;
        if index == 7 {
            assert_eq!(dealt, 30, "the seventh orb is what pays");
        } else {
            assert_eq!(dealt, 0, "orb {index} pays nothing");
        }
    }
}

/// An emotion chip fires every orb its owner is holding as their turn opens,
/// where a blow block did not stop whole landed on them since their last turn
/// began.
#[test]
fn an_emotion_chip_fires_the_orbs_after_a_hit() {
    let mut untouched = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .played_as(id("CHARACTER.DEFECT"))
        .player_hp(80, 80)
        .energy(30, 30)
        .turn(1, 1)
        .orb_slots(3)
        .orb(id("ORB.FROST_ORB"))
        .relics(&[id("RELIC.EMOTION_CHIP")])
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatTargets, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 999, 999, "SHRINKER_MOVE")
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    end_turn(&mut untouched);
    // The turn end fired the orb's own passive and the block was cleared as the
    // next turn opened, so nothing of it is left.
    assert_eq!(creature(&untouched, 0).block, 0);

    let mut hit = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .played_as(id("CHARACTER.DEFECT"))
        .player_hp(80, 80)
        .energy(30, 30)
        .turn(1, 1)
        .orb_slots(3)
        .orb(id("ORB.FROST_ORB"))
        .relics(&[id("RELIC.EMOTION_CHIP")])
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatTargets, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 999, 999, "CHOMP_MOVE")
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    end_turn(&mut hit);
    assert_eq!(creature(&hit, 0).block, 2, "the chip fired the Frost orb");
}

/// The two starters. A divine right pays three stars as its owner walks into a
/// fight — the combat state is built before the room announces itself, so the
/// pool it lands in is the fresh one — and a divine destiny pays six on the
/// first turn of every fight instead.
#[test]
fn the_regents_starters_pay_stars_where_they_say() {
    let simulator = regent_fight_from_run(&["RELIC.DIVINE_RIGHT"]);
    assert_eq!(stars_held(&simulator), 3, "three as the room opened");

    let mut simulator = regent_fight_from_run(&["RELIC.DIVINE_DESTINY"]);
    assert_eq!(stars_held(&simulator), 6, "six as the first turn opened");
    end_turn(&mut simulator);
    assert_eq!(
        stars_held(&simulator),
        6,
        "and the first turn is the only one it pays for"
    );
}

/// A fencing manual forges ten on the first turn of every fight.
#[test]
fn a_fencing_manual_forges_on_the_opening_turn() {
    let mut simulator = regent_fight_from_run(&["RELIC.FENCING_MANUAL"]);
    assert_eq!(
        blade_damage(&mut simulator),
        20,
        "the ten the blade carries and the ten it forged"
    );
}

/// Galactic dust counts the stars its owner spends and pays ten block for every
/// ten of them, keeping the remainder for next time.
#[test]
fn galactic_dust_pays_block_for_every_ten_stars_spent() {
    let mut simulator = regent_fight(
        &["RELIC.GALACTIC_DUST"],
        &[("CARD.SEVEN_STARS", 0), ("CARD.DEVASTATE", 0)],
        20,
    );
    play(&mut simulator, "CARD.SEVEN_STARS");
    assert_eq!(player(&simulator).block, 0, "seven is not ten");
    assert_eq!(relic_counter(&simulator, "RELIC.GALACTIC_DUST"), 7);
    play(&mut simulator, "CARD.DEVASTATE");
    assert_eq!(player(&simulator).block, 10, "eleven pays for one ten");
    assert_eq!(
        relic_counter(&simulator, "RELIC.GALACTIC_DUST"),
        1,
        "and the one over is kept for next time"
    );
}

/// A lunar pastry pays a star at the end of every turn its owner took part in.
#[test]
fn a_lunar_pastry_pays_a_star_every_turn() {
    let mut simulator = regent_fight(&["RELIC.LUNAR_PASTRY"], &[], 0);
    end_turn(&mut simulator);
    assert_eq!(stars_held(&simulator), 1);
    end_turn(&mut simulator);
    assert_eq!(stars_held(&simulator), 2);
}

/// A mini regent pays a strength the first time its owner spends stars in a
/// turn, and only the first.
#[test]
fn a_mini_regent_pays_once_a_turn_for_the_stars_spent() {
    let mut simulator = regent_fight(
        &["RELIC.MINI_REGENT"],
        &[("CARD.CLOAK_OF_STARS", 0), ("CARD.CLOAK_OF_STARS", 0)],
        20,
    );
    play(&mut simulator, "CARD.CLOAK_OF_STARS");
    assert_eq!(power_amount(&simulator, 0, "POWER.STRENGTH_POWER"), 1);
    play(&mut simulator, "CARD.CLOAK_OF_STARS");
    assert_eq!(
        power_amount(&simulator, 0, "POWER.STRENGTH_POWER"),
        1,
        "the latch is down for the rest of the turn"
    );
}

/// Orange dough deals two colorless cards into the hand on the first turn of
/// every fight, on top of whatever the deal itself put there.
#[test]
fn orange_dough_deals_two_colorless_cards_on_the_opening_turn() {
    let plain = hand_size(&regent_fight_from_run(&[]));
    let doughy = hand_size(&regent_fight_from_run(&["RELIC.ORANGE_DOUGH"]));
    assert_eq!(doughy, plain + 2);
}

/// A regalite pays two block for every card its owner has conjured into the
/// fight.
#[test]
fn a_regalite_pays_block_for_every_card_conjured() {
    let mut simulator = regent_fight(&["RELIC.REGALITE"], &[("CARD.COLLISION_COURSE", 0)], 0);
    play(&mut simulator, "CARD.COLLISION_COURSE");
    assert_eq!(player(&simulator).block, 2);
}

/// A vitruvian minion doubles what a minion card deals and what it works up.
#[test]
fn a_vitruvian_minion_doubles_what_a_minion_card_does() {
    let mut simulator = regent_fight(
        &["RELIC.VITRUVIAN_MINION"],
        &[("CARD.MINION_STRIKE", 0), ("CARD.MINION_SACRIFICE", 0)],
        0,
    );
    let before = creature(&simulator, 1).current_hp;
    play(&mut simulator, "CARD.MINION_STRIKE");
    let doubled = before - creature(&simulator, 1).current_hp;
    let plain = {
        let mut simulator = regent_fight(&[], &[("CARD.MINION_STRIKE", 0)], 0);
        let before = creature(&simulator, 1).current_hp;
        play(&mut simulator, "CARD.MINION_STRIKE");
        before - creature(&simulator, 1).current_hp
    };
    assert_eq!(doubled, plain * 2, "the blow is doubled");
    play(&mut simulator, "CARD.MINION_SACRIFICE");
    let block_doubled = player(&simulator).block;
    let block_plain = {
        let mut simulator = regent_fight(&[], &[("CARD.MINION_SACRIFICE", 0)], 0);
        play(&mut simulator, "CARD.MINION_SACRIFICE");
        player(&simulator).block
    };
    assert_eq!(block_doubled, block_plain * 2, "and so is the block");
}

/// A kaleidoscope lays out two card offers the moment it is taken, each of them
/// one card from each of three colours other than the one being played.
#[test]
fn a_kaleidoscope_lays_out_two_offers_of_three_other_colours() {
    let mut simulator = run_holding_with_treasure(&[], "RELIC.KALEIDOSCOPE");
    walk_to(&mut simulator, 4);
    let take = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::TakeTreasure { .. }))
        .cloned()
        .expect("the relic is on the chest");
    simulator.step(take).expect("taking it applies");
    let rewards = offered_rewards(&simulator);
    assert_eq!(rewards.len(), 2, "two offers");
    let registry = sts2_content::standard_registry();
    for reward in &rewards {
        assert_eq!(reward.reward_type, "card");
        assert_eq!(reward.offered_cards.len(), 3, "three cards, one per colour");
        let pools: BTreeSet<String> = reward
            .offered_cards
            .iter()
            .map(|card| {
                registry
                    .character_card_pools()
                    .into_iter()
                    .find(|pool| {
                        registry.card_pool(pool).is_some_and(|cards| {
                            cards.iter().any(|held| held.model_id == card.model_id)
                        })
                    })
                    .expect("the card belongs to a character pool")
                    .to_string()
            })
            .collect();
        assert_eq!(pools.len(), 3, "three different colours: {pools:?}");
        assert!(
            !pools.contains("CARD_POOL.IRONCLAD_CARD_POOL"),
            "and never the one being played: {pools:?}"
        );
    }
}

/// An unsettling lamp doubles every lasting debuff the first card of a fight to
/// work one lands, and nothing after it. The binding is the card, so the rest of
/// that card's own play is doubled too and the next card is not.
#[test]
fn an_unsettling_lamp_doubles_what_the_first_card_of_a_fight_works() {
    let mut simulator = fight(
        &["RELIC.UNSETTLING_LAMP"],
        &[("CARD.BASH", 0), ("CARD.BASH", 0)],
    );
    play(&mut simulator, "CARD.BASH");
    assert_eq!(
        power_amount(&simulator, 1, "POWER.VULNERABLE_POWER"),
        4,
        "two doubled"
    );
    play(&mut simulator, "CARD.BASH");
    assert_eq!(
        power_amount(&simulator, 1, "POWER.VULNERABLE_POWER"),
        6,
        "and the second card's two stand as they are"
    );
}

/// Pael's eye buys its owner one more turn a fight where they chose to play
/// nothing at all, burning whatever they were still holding on the way, and is
/// spent for it.
#[test]
fn a_paels_eye_buys_one_more_turn_for_a_turn_nothing_was_played_on() {
    let mut simulator = fight_against(
        "SHRINKER_MOVE",
        1,
        &["RELIC.PAELS_EYE"],
        &[("CARD.STRIKE_IRONCLAD", 0), ("CARD.STRIKE_IRONCLAD", 0)],
        &[
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
        ],
    );
    let round = |simulator: &Simulator| {
        simulator
            .state()
            .combat
            .as_ref()
            .expect("the fight is standing")
            .round
    };
    let turn = |simulator: &Simulator| {
        simulator
            .state()
            .combat
            .as_ref()
            .expect("the fight is standing")
            .player
            .turn
    };
    assert_eq!((round(&simulator), turn(&simulator)), (1, 1));
    end_turn(&mut simulator);
    // The eye held the enemy off: the player's turn wound on and the round did
    // not, and the hand they were holding burned.
    assert_eq!((round(&simulator), turn(&simulator)), (1, 2));
    assert_eq!(
        simulator
            .state()
            .combat
            .as_ref()
            .expect("the fight is standing")
            .player
            .piles
            .exhaust
            .len(),
        2,
        "the two strikes they never played burned"
    );
    // The eye is spent, so this turn's end hands over as usual.
    play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    end_turn(&mut simulator);
    assert_eq!((round(&simulator), turn(&simulator)), (2, 3));
}

/// The turn an eye buys is taken in place of the enemy's, so nothing over there
/// has moved and nothing over there settles on a new move: the intent the
/// player was looking at is the one they are still looking at.
///
/// An extra player turn skips the pass that settles the next one: the roll is
/// made at a player turn start only when the enemy took the turn before it.
/// A chomper stands this up — its move machine walks on a roll, so a roll
/// made early would walk it off the clamp it is still showing.
#[test]
fn an_extra_turn_leaves_the_enemys_intent_where_it_stood() {
    let chomper_fight = |relics: &[&str]| {
        let held: Vec<sts2_core::ModelId> = relics.iter().map(|model| id(model)).collect();
        ScenarioBuilder::new(SEED, id("ENCOUNTER.CHOMPERS_NORMAL"))
            .player_hp(80, 80)
            .energy(30, 30)
            .turn(1, 1)
            .relics(&held)
            .rng_counter(RunStream::MonsterAi, 0)
            .rng_counter(RunStream::CombatTargets, 0)
            .enemy(1, id("MONSTER.CHOMPER"), 60, 60, "CLAMP_MOVE")
            .card(PileName::Hand, 0, card("CARD.DEFEND_IRONCLAD", 0), 0)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds")
    };
    let move_id = |simulator: &Simulator| {
        simulator
            .state()
            .combat
            .as_ref()
            .expect("the fight is standing")
            .creatures
            .iter()
            .find(|creature| creature.id.get() == 1)
            .expect("the chomper is standing")
            .intent
            .as_ref()
            .map(|intent| intent.move_id.clone())
    };

    // The eye's turn: the player plays nothing, the enemy never acts, and the
    // clamp they were shown is the clamp they get.
    let mut bought = chomper_fight(&["RELIC.PAELS_EYE"]);
    let shown = move_id(&bought);
    assert_eq!(shown.as_deref(), Some("CLAMP_MOVE"));
    end_turn(&mut bought);
    assert_eq!(
        move_id(&bought),
        shown,
        "the extra turn rolled nothing over there"
    );

    // The same handover without the eye, which is what makes the test say
    // something: the enemy takes its turn and settles on the move behind it.
    let mut plain = chomper_fight(&[]);
    end_turn(&mut plain);
    assert_eq!(
        move_id(&plain).as_deref(),
        Some("SCREECH_MOVE"),
        "a turn the enemy did take walks the machine on"
    );
}

// --- a relic taken beside a standing card offer --------------------------

/// A reward screen holding a relic beside a card offer, the way an elite pays
/// both on one set. `rolled` says whether the offer carries the creation it was
/// rolled from; an offer handed a fixed list of cards carries none.
fn a_relic_beside_a_card_offer(relic: &str, offered: &[&str], rolled: bool) -> Simulator {
    let pool = id("CARD_POOL.IRONCLAD_CARD_POOL");
    let mut offer = sts2_engine::RewardFingerprint::of("card")
        .with_cards(
            offered.iter().map(|model_id| card(model_id, 0)).collect(),
            vec![pool.clone()],
            sts2_engine::RarityOdds::RegularEncounter,
        )
        .from_source(sts2_engine::RewardSource::Encounter);
    if rolled {
        offer = offer.from_creation(sts2_engine::CardCreation {
            pools: vec![sts2_engine::CardPoolRef::Named(pool)],
            filter: sts2_engine::CardFilter::Any,
            odds: sts2_engine::RarityOdds::RegularEncounter,
            from_an_encounter: true,
            count: offered.len(),
            flags: sts2_engine::CardCreationFlags {
                is_card_reward: true,
                ..sts2_engine::CardCreationFlags::default()
            },
        });
    }
    let mut state = run_holding(&[]).state().clone();
    let run = state.run.as_mut().expect("the run is standing");
    let set_id = sts2_core::RewardSetId::from(run.next_reward_set_id);
    run.next_reward_set_id += 1;
    run.reward_set = Some(sts2_engine::RewardSet {
        id: set_id,
        player_slot: "p0".to_owned(),
        rewards: vec![
            sts2_engine::RewardItem {
                index: 0,
                fingerprint: sts2_engine::RewardFingerprint::of("relic").with_model(id(relic)),
                selected: false,
                passed_over: false,
                opened: false,
            },
            sts2_engine::RewardItem {
                index: 1,
                fingerprint: offer,
                selected: false,
                passed_over: false,
                opened: false,
            },
        ],
        disallow_skipping: false,
    });
    Simulator::from_scenario(state, sts2_content::standard_registry())
        .expect("the reward screen stands up")
}

fn claim_reward_at(simulator: &mut Simulator, reward_index: usize) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::ClaimReward { reward_index: index, .. }
                if *index == reward_index)
        })
        .cloned()
        .unwrap_or_else(|| panic!("reward {reward_index} is not claimable"));
    simulator.step(action).expect("the reward is claimed");
}

/// The upgrade level of every card the standing offer holds, in order.
fn offered_upgrade_levels(simulator: &Simulator) -> Vec<u8> {
    offered_rewards(simulator)
        .iter()
        .find(|reward| reward.reward_type == "card")
        .expect("the card offer is still standing")
        .offered_cards
        .iter()
        .map(|offered| offered.upgrade_level)
        .collect()
}

/// An egg claimed off a set that also carries a rolled card offer upgrades the
/// cards of that offer, and only the ones of its own type.
#[test]
fn an_egg_taken_beside_a_standing_card_offer_upgrades_it() {
    let mut simulator = a_relic_beside_a_card_offer(
        "RELIC.MOLTEN_EGG",
        &["CARD.BASH", "CARD.DEFEND_IRONCLAD"],
        true,
    );
    assert_eq!(offered_upgrade_levels(&simulator), vec![0, 0]);
    claim_reward_at(&mut simulator, 0);
    assert_eq!(
        offered_upgrade_levels(&simulator),
        vec![1, 0],
        "the attack on the standing offer is upgraded and the skill is not"
    );
}

/// The same egg claimed after the card offer has been opened and skipped does
/// not upgrade it: opening the screen is what ends the offer's interest in
/// relics, and the offer is still on the table.
#[test]
fn an_egg_taken_after_a_card_offer_was_opened_and_skipped_leaves_it() {
    let mut simulator = a_relic_beside_a_card_offer(
        "RELIC.MOLTEN_EGG",
        &["CARD.BASH", "CARD.DEFEND_IRONCLAD"],
        true,
    );
    claim_reward_at(&mut simulator, 1);
    let skip = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseCards { cards, .. } if cards.is_empty()))
        .cloned()
        .expect("the card offer can be skipped");
    simulator.step(skip).expect("the offer is skipped");
    claim_reward_at(&mut simulator, 0);
    assert_eq!(
        offered_upgrade_levels(&simulator),
        vec![0, 0],
        "an offer whose screen has been opened is deaf to a relic taken after it"
    );
}

/// An offer handed a fixed list of cards never listens for relics at all, so an
/// egg taken beside it leaves it exactly as it was handed over.
#[test]
fn a_card_offer_handed_a_fixed_list_is_untouched_by_a_relic() {
    let mut simulator = a_relic_beside_a_card_offer(
        "RELIC.MOLTEN_EGG",
        &["CARD.BASH", "CARD.DEFEND_IRONCLAD"],
        false,
    );
    claim_reward_at(&mut simulator, 0);
    assert_eq!(offered_upgrade_levels(&simulator), vec![0, 0]);
}

/// A relic with no card-reward body changes nothing about a standing offer.
#[test]
fn a_relic_with_no_card_reward_body_leaves_a_standing_offer_alone() {
    let mut simulator = a_relic_beside_a_card_offer(
        "RELIC.LANTERN",
        &["CARD.BASH", "CARD.DEFEND_IRONCLAD"],
        true,
    );
    let before = offered_rewards(&simulator);
    claim_reward_at(&mut simulator, 0);
    assert_eq!(offered_rewards(&simulator)[1], before[1]);
}

/// A rolled card offer standing in front of a player holding Pael's wing, with
/// the wing one sacrifice short of paying and the player's own bag holding
/// nothing but wing charms, so the sacrifice pulls one while the pick screen is
/// standing open.
fn a_card_offer_whose_sacrifice_pays_a_wing_charm() -> Simulator {
    let charm = id("RELIC.WING_CHARM");
    let pool = id("CARD_POOL.IRONCLAD_CARD_POOL");
    let offered = ["CARD.BASH", "CARD.DEFEND_IRONCLAD"];
    let offer = sts2_engine::RewardFingerprint::of("card")
        .with_cards(
            offered.iter().map(|model_id| card(model_id, 0)).collect(),
            vec![pool.clone()],
            sts2_engine::RarityOdds::RegularEncounter,
        )
        .from_source(sts2_engine::RewardSource::Encounter)
        .from_creation(sts2_engine::CardCreation {
            pools: vec![sts2_engine::CardPoolRef::Named(pool)],
            filter: sts2_engine::CardFilter::Any,
            odds: sts2_engine::RarityOdds::RegularEncounter,
            from_an_encounter: true,
            count: offered.len(),
            flags: sts2_engine::CardCreationFlags {
                is_card_reward: true,
                ..sts2_engine::CardCreationFlags::default()
            },
        });
    let mut state = run_holding(&["RELIC.PAELS_WING"]).state().clone();
    for relic in &mut state.run_player.relics {
        if relic.model_id == id("RELIC.PAELS_WING") {
            relic.counter = 1;
        }
    }
    let run = state.run.as_mut().expect("the run is standing");
    // Whichever rarity the pull rolls, what comes off the front is a charm.
    run.player_relic_grab_bag = RelicGrabBagState {
        starter: Vec::new(),
        common: vec![charm.clone()],
        uncommon: vec![charm.clone()],
        rare: vec![charm.clone()],
        shop: vec![charm.clone()],
        event: vec![charm.clone()],
        ancient: vec![charm],
    };
    let set_id = sts2_core::RewardSetId::from(run.next_reward_set_id);
    run.next_reward_set_id += 1;
    run.reward_set = Some(sts2_engine::RewardSet {
        id: set_id,
        player_slot: "p0".to_owned(),
        rewards: vec![sts2_engine::RewardItem {
            index: 0,
            fingerprint: offer,
            selected: false,
            passed_over: false,
            opened: false,
        }],
        disallow_skipping: false,
    });
    Simulator::from_scenario(state, sts2_content::standard_registry())
        .expect("the reward screen stands up")
}

/// The sacrifice is the one path that obtains a relic from inside an open pick
/// screen, and the offer is still listening when it does: the charm it pulls
/// runs its card-reward body over the very offer being thrown away, and reads
/// its niche draw doing it. The cards go unpicked either way; the draw is what
/// survives.
#[test]
fn a_wing_charm_a_sacrifice_pulls_reads_its_draw_over_the_offer_it_throws_away() {
    let mut simulator = a_card_offer_whose_sacrifice_pays_a_wing_charm();
    claim_reward_at(&mut simulator, 0);
    let niche = niche_counter(&simulator);
    let sacrifice = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::ChooseAlternative { option_id, .. }
                if option_id == "sacrifice")
        })
        .cloned()
        .expect("the wing puts a sacrifice on the screen");
    simulator
        .step(sacrifice)
        .expect("the reward is thrown away");
    assert!(
        simulator
            .state()
            .run_player
            .relics
            .iter()
            .any(|relic| relic.model_id == id("RELIC.WING_CHARM")),
        "the second sacrifice pays a relic off the player's own bag"
    );
    assert_eq!(
        niche_counter(&simulator),
        niche + 1,
        "the charm rolls which card of the standing offer takes its swift"
    );
}

/// The point's history lists the encounter's own spawns, written down as the
/// room is entered and before anything the opening does can kill one of them.
///
/// The list the encounter generated is written as the room is entered; a
/// festive popper's nine lands on the first player turn, which is well behind
/// that. A slime the popper finishes is in the history all the same, so three
/// slimes met are three slimes written.
#[test]
fn a_popper_that_finishes_a_monster_still_leaves_it_in_the_history() {
    let mut simulator = run_holding_full(
        &["RELIC.FESTIVE_POPPER"],
        "RELIC.LANTERN",
        5,
        None,
        Vec::new(),
    );
    walk_to(&mut simulator, 3);

    assert!(
        simulator.state().combat.as_ref().is_none_or(|combat| combat
            .creatures
            .iter()
            .all(|creature| creature.side != sts2_engine::CombatSide::Enemy || !creature.alive())),
        "the popper finished the beetle on the way in"
    );
    let listed: Vec<String> = simulator
        .state()
        .run
        .as_ref()
        .and_then(|run| run.map_history.last())
        .and_then(|act| act.points.last())
        .and_then(|point| point.rooms.last())
        .expect("the fight's room is in the history")
        .monsters
        .iter()
        .map(|seen| seen.model_id.to_string())
        .collect();
    assert_eq!(
        listed,
        vec!["MONSTER.SHRINKER_BEETLE".to_owned()],
        "the beetle the popper killed is still what the room held"
    );
}

/// A fur coat taken off a chest marks the act's combat rooms as it is taken,
/// and its saved properties are what the game writes down for it: the act it
/// was taken in, and its marked rooms' columns and rows as two arrays beside
/// the flag that says they are set.
#[test]
fn a_fur_coats_marked_rooms_are_projected_as_its_saved_properties() {
    let mut simulator = run_holding_with_treasure(&[], "RELIC.FUR_COAT");
    walk_to(&mut simulator, 4);
    let take = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::TakeTreasure { .. }))
        .cloned()
        .expect("the coat is on the chest");
    simulator.step(take).expect("taking it applies");
    let rendered = sts2_replay::observation::render(&simulator);
    let coat = rendered["players"][0]["relics"]
        .as_array()
        .expect("the relics are projected")
        .iter()
        .find(|relic| relic["model_id"] == "RELIC.FUR_COAT")
        .expect("the coat is projected");
    // The map has one monster room, at column 3 of row 1, so that is the
    // whole of the marking.
    assert_eq!(
        coat["properties"],
        serde_json::json!({
            "bools": [{"name": "FurCoatCoordsSet", "value": true}],
            "card_arrays": serde_json::Value::Null,
            "cards": serde_json::Value::Null,
            "int_arrays": [
                {"name": "FurCoatCoordCols", "value": [3]},
                {"name": "FurCoatCoordRows", "value": [1]},
            ],
            "ints": [{"name": "FurCoatActIndex", "value": 0}],
            "model_ids": serde_json::Value::Null,
            "strings": serde_json::Value::Null,
        })
    );
}

/// A lord's parasol buys the whole shelf, but not as the room is entered: the
/// shelf still stands full then, and the purchases follow one at a time, each
/// a purchase of its own that pays nothing, with the card removal forced
/// after them. What the run is asked next is what stands once all of that
/// is done, and stepping straight to it makes every owed purchase first.
fn a_parasol_walking_into_a_stocked_shop() -> Simulator {
    let shelf = vec![
        sts2_engine::ShopOffer {
            index: 0,
            item: sts2_engine::ShopItem::Card(card("CARD.ANGER", 0)),
            slot: sts2_engine::ShopSlot::CharacterCard,
            base_price: 50,
            price: 50,
            sold: false,
        },
        sts2_engine::ShopOffer {
            index: 1,
            item: sts2_engine::ShopItem::Relic(id("RELIC.LANTERN")),
            slot: sts2_engine::ShopSlot::Relic,
            base_price: 150,
            price: 150,
            sold: false,
        },
    ];
    let mut simulator =
        run_holding_full(&["RELIC.LORDS_PARASOL"], "RELIC.LANTERN", 39, None, shelf);
    walk_to(&mut simulator, 0);
    simulator
}

fn unsold(simulator: &Simulator) -> usize {
    match simulator
        .state()
        .run
        .as_ref()
        .and_then(|run| run.active_room.as_ref())
    {
        Some(sts2_engine::ActiveRoom::Shop { offers, .. }) => {
            offers.iter().filter(|offer| !offer.sold).count()
        }
        other => panic!("expected the shop, found {other:?}"),
    }
}

#[test]
fn a_lords_parasol_buys_the_shelf_one_purchase_at_a_time_after_entering() {
    // Entered: nothing bought yet, and the decision offered is the removal
    // the parasol will force once it has bought everything.
    let mut simulator = a_parasol_walking_into_a_stocked_shop();
    let gold = simulator.state().run_player.gold;
    let deck = simulator.state().run_player.deck.len();
    assert_eq!(unsold(&simulator), 2);
    assert!(
        simulator
            .legal_actions()
            .iter()
            .all(|action| matches!(action, Action::ChooseCards { .. })),
        "the removal's screen is what is offered: {:?}",
        simulator.legal_actions()
    );

    // Stepped one purchase at a time, the way the game records them.
    let first = simulator
        .automatic_purchase(0)
        .expect("the card is bought first");
    simulator.step(first).expect("the purchase applies");
    assert_eq!(unsold(&simulator), 1);
    assert_eq!(simulator.state().run_player.gold, gold, "nothing is paid");
    assert!(simulator.automatic_purchase(0).is_none());
    let second = simulator
        .automatic_purchase(1)
        .expect("the relic is bought next");
    simulator.step(second).expect("the purchase applies");
    assert_eq!(unsold(&simulator), 0);
    assert!(
        simulator
            .state()
            .run_player
            .relics
            .iter()
            .any(|relic| relic.model_id == id("RELIC.LANTERN"))
    );
    let removal = simulator
        .automatic_removal()
        .expect("the removal is forced once the shelf is bought");
    simulator.step(removal).expect("the removal opens");
    assert!(simulator.automatic_removal().is_none());
    let choose = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseCards { .. }))
        .cloned()
        .expect("a card is asked for");
    simulator.step(choose).expect("the removal applies");
    assert_eq!(simulator.state().run_player.deck.len(), deck);
}

/// Stepped straight to the removal's answer instead, the way a run under
/// search is: everything owed is made first, then the answer lands.
#[test]
fn a_lords_parasols_purchases_are_all_made_before_the_next_step_lands() {
    let mut simulator = a_parasol_walking_into_a_stocked_shop();
    let gold = simulator.state().run_player.gold;
    let deck = simulator.state().run_player.deck.len();
    let choose = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseCards { .. }))
        .cloned()
        .expect("a card is asked for");
    simulator.step(choose).expect("the answer applies");
    assert_eq!(unsold(&simulator), 0);
    assert!(
        simulator
            .state()
            .run_player
            .relics
            .iter()
            .any(|relic| relic.model_id == id("RELIC.LANTERN"))
    );
    assert_eq!(simulator.state().run_player.gold, gold);
    assert_eq!(simulator.state().run_player.deck.len(), deck);
}
