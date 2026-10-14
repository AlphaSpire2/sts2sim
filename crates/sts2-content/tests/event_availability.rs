//! What the act's event walk does with an event the run cannot have.
//!
//! The act's event walk goes forward from where the act stands, counting each
//! event it passes over as visited, until it finds one the run is allowed and
//! has not already seen. Every act event is registered with the condition it
//! declares so that the walk can ask, whether or not its page is transcribed,
//! so these are the tests that the conditions are asked and that the walk
//! counts what it passes.

use sts2_engine::{
    ActMap, ActRoomPools, Action, ActiveRoom, CardFingerprint, MapCoord, MapPoint, MapPointType,
    RelicGrabBagState, RunConfig, RunPhase, RunState, Simulator,
};

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().unwrap()
}

/// A two-point map: the act's ancient, and one more ancient point to walk to.
/// Only the start counts as the act's own ancient, so the second point pulls an
/// event off the act's list the way any event room does.
fn walk_map() -> ActMap {
    let start = MapCoord { col: 0, row: 0 };
    let next = MapCoord { col: 0, row: 1 };
    ActMap {
        width: 1,
        height: 2,
        start,
        boss: next,
        second_boss: None,
        points: vec![
            MapPoint {
                coord: start,
                point_type: MapPointType::Ancient,
                children: vec![next],
                can_modify: true,
            },
            MapPoint {
                coord: next,
                point_type: MapPointType::Ancient,
                children: Vec::new(),
                can_modify: true,
            },
        ],
    }
}

struct Walk {
    events: Vec<&'static str>,
    act: usize,
    gold: i32,
    current_hp: i32,
    max_hp: i32,
    potions: usize,
    deck: Vec<&'static str>,
    relics: Vec<&'static str>,
    seen: Vec<&'static str>,
    grab_bag: bool,
    /// The floor the run is standing on before it walks. The point it walks
    /// onto is the one after this.
    floor: u32,
}

impl Walk {
    fn new(events: &[&'static str]) -> Self {
        Self {
            events: events.to_vec(),
            act: 0,
            gold: 0,
            current_hp: 80,
            max_hp: 80,
            potions: 0,
            deck: vec!["CARD.STRIKE_IRONCLAD"],
            relics: Vec::new(),
            seen: Vec::new(),
            grab_bag: false,
            floor: 1,
        }
    }

    fn simulator(&self) -> Simulator {
        let mut state = sts2_content::shrinker_beetle_fixture()
            .unwrap()
            .state()
            .clone();
        state.combat = None;
        state.terminal = None;
        state.run_player.gold = self.gold;
        state.run_player.current_hp = self.current_hp;
        state.run_player.max_hp = self.max_hp;
        state.run_player.potions = (0..3)
            .map(|slot| (slot < self.potions).then(|| id("POTION.BLOCK_POTION")))
            .collect();
        state.run_player.deck = self
            .deck
            .iter()
            .map(|card| CardFingerprint::base(id(card)))
            .collect();
        state.run_player.relics = self
            .relics
            .iter()
            .map(|relic| sts2_engine::RelicInstance::new(id(relic)))
            .collect();
        let map = walk_map();
        let mut grab_bag = RelicGrabBagState::default();
        if self.grab_bag {
            grab_bag.common.push(id("RELIC.LANTERN"));
        }
        state.run = Some(RunState {
            config: RunConfig::fixture_baseline("N3Z9EE2CXB"),
            current_act: self.act,
            floor: self.floor,
            current_coord: Some(map.start),
            maps: std::iter::repeat_n(map, self.act + 1).collect(),
            rooms: vec![Vec::new(); self.act + 1],
            visited: Vec::new(),
            visited_events: self.seen.iter().map(|event| id(event)).collect(),
            started_with_neow: true,
            reloads: 0,
            map_history: vec![sts2_engine::ActHistory {
                act: 0,
                points: Vec::new(),
            }],
            unlocks: sts2_core::UnlockPresetManifest::pinned()
                .expect("pinned unlock preset")
                .unlocks,
            acts: std::iter::repeat_n(
                ActRoomPools {
                    act_model: id("ACT.OVERGROWTH"),
                    ancient: id("EVENT.NEOW"),
                    boss: id("ENCOUNTER.THE_KIN_BOSS"),
                    second_boss: None,
                    normal_encounters: vec![id("ENCOUNTER.SHRINKER_BEETLE_WEAK")],
                    elite_encounters: Vec::new(),
                    events: self.events.iter().map(|event| id(event)).collect(),
                    normal_encounters_visited: 0,
                    elite_encounters_visited: 0,
                    events_visited: 0,
                    boss_encounters_visited: 0,
                },
                self.act + 1,
            )
            .collect(),
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
            player_relic_grab_bag: grab_bag,
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

    /// Which event the walk lands on, and how far the act counted.
    ///
    /// An event whose page is not transcribed stops as unsupported content
    /// naming itself, which is what says the walk landed on it. A step that
    /// stops leaves the run as it was, so the count is only what a landing the
    /// run could actually walk into settled.
    fn lands_on(&self) -> (String, usize) {
        let mut simulator = self.simulator();
        let action = Action::ChooseMap {
            destination: MapCoord { col: 0, row: 1 },
        };
        let landed = match simulator.step(action) {
            Ok(_) => match simulator.state().run.as_ref().unwrap().active_room.as_ref() {
                Some(ActiveRoom::Event { model_id, .. }) => model_id.to_string(),
                other => panic!("expected an event room, found {other:?}"),
            },
            Err(error) => error
                .details
                .get("model_id")
                .and_then(serde_json::Value::as_str)
                .expect("an unsupported event names itself")
                .to_owned(),
        };
        let visited = simulator.state().run.as_ref().unwrap().acts[0].events_visited;
        (landed, visited)
    }
}

#[test]
fn an_event_this_act_cannot_offer_is_passed_over_and_counted() {
    // A symbiote is only offered from the second act on, so act one walks past
    // it. Walking into the event that follows counts a second visit.
    let walk = Walk::new(&["EVENT.SYMBIOTE", "EVENT.THIS_OR_THAT"]);
    assert_eq!(walk.lands_on(), ("EVENT.THIS_OR_THAT".to_owned(), 2));

    let mut later = Walk::new(&["EVENT.SYMBIOTE", "EVENT.THIS_OR_THAT"]);
    later.act = 1;
    assert_eq!(later.lands_on().0, "EVENT.SYMBIOTE");
}

/// A punch-off wants six floors walked, and the floor count is the map points
/// the run has walked. The point being walked onto joins that count only
/// once the room in front of it has been built, and the walk over the act's
/// events runs while it is being built — so the floor the condition reads is
/// the one being left, not the one being arrived on.
///
/// A run leaving floor five therefore walks past a punch-off and takes the
/// event behind it, counting both. A run leaving floor six lands on it.
#[test]
fn a_floor_gated_event_is_judged_by_the_floor_being_left() {
    let mut arriving_on_six = Walk::new(&["EVENT.PUNCH_OFF", "EVENT.BRAIN_LEECH"]);
    arriving_on_six.floor = 5;
    assert_eq!(
        arriving_on_six.lands_on(),
        ("EVENT.BRAIN_LEECH".to_owned(), 2)
    );

    let mut arriving_on_seven = Walk::new(&["EVENT.PUNCH_OFF", "EVENT.BRAIN_LEECH"]);
    arriving_on_seven.floor = 6;
    assert_eq!(
        arriving_on_seven.lands_on(),
        ("EVENT.PUNCH_OFF".to_owned(), 1)
    );
}

#[test]
fn an_event_the_run_has_already_seen_is_passed_over() {
    let mut walk = Walk::new(&["EVENT.THIS_OR_THAT", "EVENT.TRIAL"]);
    walk.seen = vec!["EVENT.THIS_OR_THAT"];
    assert_eq!(walk.lands_on().0, "EVENT.TRIAL");
}

#[test]
fn the_walk_reads_gold_hp_potions_and_the_deck() {
    // A whispering hollow wants forty-four gold or more.
    let mut poor = Walk::new(&["EVENT.WHISPERING_HOLLOW", "EVENT.TRIAL"]);
    poor.gold = 43;
    assert_eq!(poor.lands_on().0, "EVENT.TRIAL");
    let mut rich = Walk::new(&["EVENT.WHISPERING_HOLLOW", "EVENT.TRIAL"]);
    rich.gold = 44;
    assert_eq!(rich.lands_on().0, "EVENT.WHISPERING_HOLLOW");

    // A colossal flower wants nineteen current HP or more.
    let mut hurt = Walk::new(&["EVENT.COLOSSAL_FLOWER", "EVENT.TRIAL"]);
    hurt.current_hp = 18;
    assert_eq!(hurt.lands_on().0, "EVENT.TRIAL");

    // An unrest site wants current HP at most seven tenths of maximum.
    let mut healthy = Walk::new(&["EVENT.UNREST_SITE", "EVENT.TRIAL"]);
    healthy.current_hp = 57;
    healthy.max_hp = 80;
    assert_eq!(healthy.lands_on().0, "EVENT.TRIAL");
    let mut wounded = Walk::new(&["EVENT.UNREST_SITE", "EVENT.TRIAL"]);
    wounded.current_hp = 56;
    wounded.max_hp = 80;
    assert_eq!(wounded.lands_on().0, "EVENT.UNREST_SITE");

    // The future of potions wants two potions held.
    let mut one_potion = Walk::new(&["EVENT.THE_FUTURE_OF_POTIONS", "EVENT.TRIAL"]);
    one_potion.potions = 1;
    assert_eq!(one_potion.lands_on().0, "EVENT.TRIAL");
    let mut two_potions = Walk::new(&["EVENT.THE_FUTURE_OF_POTIONS", "EVENT.TRIAL"]);
    two_potions.potions = 2;
    assert_eq!(two_potions.lands_on().0, "EVENT.THE_FUTURE_OF_POTIONS");

    // An amalgamator counts basic removable cards by tag, two of each.
    let mut thin = Walk::new(&["EVENT.AMALGAMATOR", "EVENT.TRIAL"]);
    thin.deck = vec![
        "CARD.STRIKE_IRONCLAD",
        "CARD.STRIKE_IRONCLAD",
        "CARD.DEFEND_IRONCLAD",
    ];
    assert_eq!(thin.lands_on().0, "EVENT.TRIAL");
    let mut whole = Walk::new(&["EVENT.AMALGAMATOR", "EVENT.TRIAL"]);
    whole.deck = vec![
        "CARD.STRIKE_IRONCLAD",
        "CARD.STRIKE_IRONCLAD",
        "CARD.DEFEND_IRONCLAD",
        "CARD.DEFEND_IRONCLAD",
    ];
    assert_eq!(whole.lands_on().0, "EVENT.AMALGAMATOR");

    // A grave of the forgotten wants a deck card the Souls enchantment can go
    // on, which is one carrying Exhaust.
    let mut plain = Walk::new(&["EVENT.GRAVE_OF_THE_FORGOTTEN", "EVENT.TRIAL"]);
    plain.deck = vec!["CARD.STRIKE_IRONCLAD"];
    assert_eq!(plain.lands_on().0, "EVENT.TRIAL");
    let mut burning = Walk::new(&["EVENT.GRAVE_OF_THE_FORGOTTEN", "EVENT.TRIAL"]);
    burning.deck = vec!["CARD.TREMBLE"];
    assert_eq!(burning.lands_on().0, "EVENT.GRAVE_OF_THE_FORGOTTEN");
}

#[test]
fn the_walk_reads_the_relics_and_the_bag() {
    // Ranwid the Elder wants act two or later, a tradable relic, a hundred
    // gold, and a potion.
    let mut ranwid = Walk::new(&["EVENT.RANWID_THE_ELDER", "EVENT.TRIAL"]);
    ranwid.act = 1;
    ranwid.gold = 100;
    ranwid.potions = 1;
    ranwid.relics = vec!["RELIC.LANTERN"];
    assert_eq!(ranwid.lands_on().0, "EVENT.RANWID_THE_ELDER");
    // Cursed Pearl does its work as it is picked up, so it is not tradable.
    let untradable = ranwid.simulator_relics(&["RELIC.CURSED_PEARL"]);
    assert_eq!(untradable.lands_on().0, "EVENT.TRIAL");

    // A luminous choir wants gold and a bag with something left in it.
    let mut empty_bag = Walk::new(&["EVENT.LUMINOUS_CHOIR", "EVENT.TRIAL"]);
    empty_bag.gold = 149;
    assert_eq!(empty_bag.lands_on().0, "EVENT.TRIAL");
    let mut full_bag = Walk::new(&["EVENT.LUMINOUS_CHOIR", "EVENT.TRIAL"]);
    full_bag.gold = 149;
    full_bag.grab_bag = true;
    assert_eq!(full_bag.lands_on().0, "EVENT.LUMINOUS_CHOIR");
}

#[test]
fn an_event_no_run_of_this_baseline_can_have_is_never_landed_on() {
    // War Historian Repy is never allowed at all.
    let walk = Walk::new(&["EVENT.WAR_HISTORIAN_REPY", "EVENT.TRIAL"]);
    assert_eq!(walk.lands_on().0, "EVENT.TRIAL");
}

impl Walk {
    /// The same walk with a different relic in hand.
    fn simulator_relics(&self, relics: &[&'static str]) -> Self {
        Self {
            relics: relics.to_vec(),
            events: self.events.clone(),
            deck: self.deck.clone(),
            seen: self.seen.clone(),
            ..*self
        }
    }
}
