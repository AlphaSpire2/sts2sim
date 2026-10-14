//! The pages the act's events lay out, and what taking an option does.
//!
//! Every event here is walked into the way a run walks into one: the act pulls
//! it off its own list as the map point is entered, so the page these tests
//! read is the page generation built, rolls and all.

use sts2_engine::{
    ActMap, ActRoomPools, Action, ActiveRoom, CardFingerprint, EventEffect, EventOption, MapCoord,
    MapPoint, MapPointType, RelicGrabBagState, RunConfig, RunPhase, RunState, Simulator,
};

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().unwrap()
}

/// A two-point map: the act's own ancient, and one more point to walk onto.
/// Only the start counts as the act's ancient, so the second point pulls an
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

/// A run standing one step in front of an event room.
struct Visit {
    event: &'static str,
    character: &'static str,
    act: usize,
    gold: i32,
    current_hp: i32,
    max_hp: i32,
    potions: usize,
    deck: Vec<&'static str>,
    bag: Vec<&'static str>,
    relics: Vec<&'static str>,
    named_potion: Option<&'static str>,
    /// A run seed other than the fixture's, for an event whose page is rolled
    /// off the seed.
    seed: Option<&'static str>,
}

impl Visit {
    fn new(event: &'static str) -> Self {
        Self {
            event,
            character: "CHARACTER.IRONCLAD",
            act: 0,
            gold: 0,
            current_hp: 80,
            max_hp: 80,
            potions: 0,
            deck: vec!["CARD.STRIKE_IRONCLAD", "CARD.DEFEND_IRONCLAD"],
            bag: Vec::new(),
            relics: Vec::new(),
            named_potion: None,
            seed: None,
        }
    }

    fn seed(mut self, seed: &'static str) -> Self {
        self.seed = Some(seed);
        self
    }

    fn character(mut self, character: &'static str) -> Self {
        self.character = character;
        self
    }

    fn deck(mut self, deck: &[&'static str]) -> Self {
        self.deck = deck.to_vec();
        self
    }

    fn gold(mut self, gold: i32) -> Self {
        self.gold = gold;
        self
    }

    fn act(mut self, act: usize) -> Self {
        self.act = act;
        self
    }

    /// One named potion on the belt rather than the block potions the count
    /// fills it with.
    fn potion(mut self, potion: &'static str) -> Self {
        self.named_potion = Some(potion);
        self.potions = 1;
        self
    }

    fn hp(mut self, current_hp: i32, max_hp: i32) -> Self {
        self.current_hp = current_hp;
        self.max_hp = max_hp;
        self
    }

    fn bag(mut self, bag: &[&'static str]) -> Self {
        self.bag = bag.to_vec();
        self
    }

    fn relics(mut self, relics: &[&'static str]) -> Self {
        self.relics = relics.to_vec();
        self
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one function stands a whole run's state up"
    )]
    fn enter(&self) -> Simulator {
        let mut state = sts2_content::shrinker_beetle_fixture()
            .unwrap()
            .state()
            .clone();
        state.combat = None;
        state.terminal = None;
        if let Some(seed) = self.seed {
            state.rng = sts2_rng::RunRngSet::new(seed);
        }
        state.run_player.gold = self.gold;
        state.run_player.current_hp = self.current_hp;
        state.run_player.max_hp = self.max_hp;
        let potion = self.named_potion.unwrap_or("POTION.BLOCK_POTION");
        state.run_player.potions = (0..3)
            .map(|slot| (slot < self.potions).then(|| id(potion)))
            .collect();
        state.run_player.deck = self
            .deck
            .iter()
            .map(|card| CardFingerprint::base(id(card)))
            .collect();
        state.run_player.relics.extend(
            self.relics
                .iter()
                .map(|relic| sts2_engine::RelicInstance::new(id(relic))),
        );
        let map = walk_map();
        // A bag holding the same relic at every rarity, so whichever rarity
        // the roll lands on has something on its shelf.
        let bucket: Vec<sts2_core::ModelId> = self.bag.iter().map(|relic| id(relic)).collect();
        let grab_bag = RelicGrabBagState {
            starter: Vec::new(),
            common: bucket.clone(),
            uncommon: bucket.clone(),
            rare: bucket.clone(),
            shop: bucket.clone(),
            event: bucket.clone(),
            ancient: bucket,
        };
        state.run = Some(RunState {
            config: RunConfig::fixture_baseline(self.seed.unwrap_or("N3Z9EE2CXB")),
            current_act: self.act,
            floor: 1,
            current_coord: Some(map.start),
            maps: std::iter::repeat_n(map, self.act + 1).collect(),
            rooms: vec![Vec::new(); self.act + 1],
            visited: Vec::new(),
            visited_events: std::collections::BTreeSet::new(),
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
                    events: vec![id(self.event)],
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
        state.run.as_mut().unwrap().config.players[0].character = id(self.character);
        let mut simulator =
            Simulator::from_generated_state(state, sts2_content::registry_for(&id(self.character)))
                .unwrap();
        simulator
            .step(Action::ChooseMap {
                destination: MapCoord { col: 0, row: 1 },
            })
            .expect("the event room stands up");
        simulator
    }
}

/// The options the page is standing on.
fn page(simulator: &Simulator) -> Vec<EventOption> {
    match simulator
        .state()
        .run
        .as_ref()
        .and_then(|run| run.active_room.as_ref())
    {
        Some(ActiveRoom::Event { options, .. }) => options.clone(),
        other => panic!("expected an event page, found {other:?}"),
    }
}

fn option_ids(simulator: &Simulator) -> Vec<String> {
    page(simulator)
        .into_iter()
        .map(|option| option.option_id)
        .collect()
}

/// Takes the option with this id off the page the run is standing on.
fn choose(simulator: &mut Simulator, option_id: &str) {
    let index = option_ids(simulator)
        .iter()
        .position(|id| id == option_id)
        .unwrap_or_else(|| panic!("{option_id} is not on the page"));
    simulator
        .step(Action::ChooseEvent {
            index,
            option_id: option_id.to_owned(),
        })
        .expect("taking the option applies");
}

fn holds_relic(simulator: &Simulator, relic: &str) -> bool {
    simulator
        .state()
        .run_player
        .relics
        .iter()
        .any(|held| held.model_id == id(relic))
}

fn deck_holds(simulator: &Simulator, card: &str) -> bool {
    simulator
        .state()
        .run_player
        .deck
        .iter()
        .any(|held| held.model_id == id(card))
}

/// What the belt offers at the decision the run is standing on.
fn belt_actions(simulator: &Simulator) -> Vec<Action> {
    simulator
        .legal_actions()
        .iter()
        .filter(|action| {
            matches!(
                action,
                Action::UsePotion { .. } | Action::DiscardPotion { .. }
            )
        })
        .cloned()
        .collect()
}

/// An event whose page picks a potion out of the belt holds the belt from the
/// moment it begins: a slot the page named still holds the potion it named,
/// because nothing is drunk or thrown away in between.
#[test]
fn an_event_that_picks_from_the_belt_holds_it_while_it_stands() {
    let ranwid = Visit::new("EVENT.RANWID_THE_ELDER")
        .act(1)
        .gold(100)
        .potion("POTION.BLOOD_POTION")
        .relics(&["RELIC.LANTERN"])
        .bag(&["RELIC.WHETSTONE"]);
    let stone = Visit::new("EVENT.STONE_OF_ALL_TIME").potion("POTION.BLOOD_POTION");
    let mut future = Visit::new("EVENT.THE_FUTURE_OF_POTIONS").potion("POTION.BLOOD_POTION");
    future.potions = 2;
    for visit in [ranwid, stone, future] {
        let simulator = visit.enter();
        assert!(
            belt_actions(&simulator).is_empty(),
            "{}: the belt is held: {:?}",
            visit.event,
            belt_actions(&simulator)
        );
    }
}

/// An event that picks nothing out of the belt leaves it in reach.
#[test]
fn an_event_that_picks_nothing_from_the_belt_leaves_it_in_reach() {
    let simulator = Visit::new("EVENT.UNREST_SITE")
        .potion("POTION.BLOOD_POTION")
        .enter();
    let belt = belt_actions(&simulator);
    assert!(
        belt.iter()
            .any(|action| matches!(action, Action::DiscardPotion { .. })),
        "a potion can be thrown away: {belt:?}"
    );
    assert!(
        belt.iter()
            .any(|action| matches!(action, Action::UsePotion { .. })),
        "and drunk: {belt:?}"
    );
}

/// The Future of Potions holds the belt through the card reward its trade
/// pays, which is still the event, and lets go once that reward is taken and
/// the event has finished.
#[test]
fn the_future_of_potions_holds_the_belt_until_its_trade_is_paid() {
    let mut visit = Visit::new("EVENT.THE_FUTURE_OF_POTIONS").potion("POTION.BLOOD_POTION");
    visit.potions = 2;
    let mut simulator = visit.enter();
    let option = page(&simulator).remove(0);
    choose(&mut simulator, &option.option_id);
    assert!(
        belt_actions(&simulator).is_empty(),
        "the reward the trade pays is still the event"
    );
    let claim = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::ClaimReward { fingerprint, .. }
                if fingerprint.reward_type == "card")
        })
        .cloned()
        .expect("the traded potion paid a card offer");
    simulator.step(claim).unwrap();
    assert!(
        matches!(
            simulator.decision(),
            sts2_engine::DecisionContext::ChooseCards { .. }
        ),
        "the card offer is open"
    );
    assert!(
        belt_actions(&simulator).is_empty(),
        "and the belt still held"
    );

    let pick = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseCards { cards, .. } if !cards.is_empty()))
        .cloned()
        .expect("the offer lays out its cards");
    simulator.step(pick).unwrap();
    assert!(
        simulator
            .state()
            .run
            .as_ref()
            .is_some_and(|run| run.active_room.is_none()),
        "the event finished once its reward was taken"
    );
    assert!(
        belt_actions(&simulator)
            .iter()
            .any(|action| matches!(action, Action::DiscardPotion { .. })),
        "the potion the trade left is in reach again"
    );
}

#[test]
fn ranwid_awaits_the_pickup_effects_of_the_relics_he_hands_over() {
    let mut simulator = Visit::new("EVENT.RANWID_THE_ELDER")
        .act(1)
        .gold(100)
        .potion("POTION.BLOCK_POTION")
        .relics(&["RELIC.LANTERN"])
        .bag(&["RELIC.WHETSTONE", "RELIC.RAZOR_TOOTH"])
        .deck(&[
            "CARD.STRIKE_IRONCLAD",
            "CARD.STRIKE_IRONCLAD",
            "CARD.BASH",
            "CARD.DEFEND_IRONCLAD",
        ])
        .enter();

    choose(
        &mut simulator,
        "RANWID_THE_ELDER.pages.INITIAL.options.RELIC",
    );

    assert!(holds_relic(&simulator, "RELIC.WHETSTONE"));
    assert_eq!(
        simulator
            .state()
            .run_player
            .deck
            .iter()
            .filter(|card| card.upgrade_level == 1)
            .count(),
        2,
        "Whetstone's AfterObtained body upgrades two attacks"
    );
}

#[test]
fn hungry_for_mushrooms_offers_two_relics_and_nothing_else() {
    let visit = Visit::new("EVENT.HUNGRY_FOR_MUSHROOMS");
    let mut simulator = visit.enter();
    assert_eq!(
        option_ids(&simulator),
        vec![
            "HUNGRY_FOR_MUSHROOMS.pages.INITIAL.options.BIG_MUSHROOM",
            "HUNGRY_FOR_MUSHROOMS.pages.INITIAL.options.FRAGRANT_MUSHROOM",
        ]
    );
    choose(
        &mut simulator,
        "HUNGRY_FOR_MUSHROOMS.pages.INITIAL.options.BIG_MUSHROOM",
    );
    assert!(holds_relic(&simulator, "RELIC.BIG_MUSHROOM"));
    // The mushroom is worth twenty maximum HP, which the relic pays rather
    // than the event.
    assert_eq!(simulator.state().run_player.max_hp, 100);
}

#[test]
fn the_bugslayer_hands_over_the_card_that_was_chosen() {
    let mut simulator = Visit::new("EVENT.BUGSLAYER").enter();
    assert_eq!(
        option_ids(&simulator),
        vec![
            "BUGSLAYER.pages.INITIAL.options.EXTERMINATION",
            "BUGSLAYER.pages.INITIAL.options.SQUASH",
        ]
    );
    choose(&mut simulator, "BUGSLAYER.pages.INITIAL.options.SQUASH");
    assert!(deck_holds(&simulator, "CARD.SQUASH"));
    assert!(!deck_holds(&simulator, "CARD.EXTERMINATE"));
}

#[test]
fn a_byrdonis_nest_is_eaten_or_taken() {
    let mut eaten = Visit::new("EVENT.BYRDONIS_NEST").hp(70, 80).enter();
    choose(&mut eaten, "BYRDONIS_NEST.pages.INITIAL.options.EAT");
    assert_eq!(eaten.state().run_player.max_hp, 87);
    assert_eq!(eaten.state().run_player.current_hp, 77);

    let mut taken = Visit::new("EVENT.BYRDONIS_NEST").enter();
    choose(&mut taken, "BYRDONIS_NEST.pages.INITIAL.options.TAKE");
    assert!(deck_holds(&taken, "CARD.BYRDONIS_EGG"));
}

#[test]
fn a_wellspring_bottles_a_potion_or_bathes_for_a_guilty() {
    let mut bottled = Visit::new("EVENT.WELLSPRING").enter();
    choose(&mut bottled, "WELLSPRING.pages.INITIAL.options.BOTTLE");
    // One potion rolled off the rewards stream over the belt's own options,
    // offered as a reward rather than handed over, so the room stays open
    // under the screen.
    let offered = bottled
        .state()
        .run
        .as_ref()
        .and_then(|run| run.reward_set.as_ref())
        .expect("the potion is offered");
    assert_eq!(offered.rewards.len(), 1);
    assert_eq!(offered.rewards[0].fingerprint.reward_type, "potion");

    let mut bathed = Visit::new("EVENT.WELLSPRING").enter();
    choose(&mut bathed, "WELLSPRING.pages.INITIAL.options.BATHE");
    let choice = bathed
        .state()
        .run
        .as_ref()
        .and_then(|run| run.suspended_choice.as_ref())
        .expect("the removal screen is standing");
    assert_eq!((choice.minimum, choice.maximum), (1, 1));
}

#[test]
fn a_drowning_beacon_names_its_potion_and_charges_for_its_lens() {
    let mut bottled = Visit::new("EVENT.DROWNING_BEACON").enter();
    choose(&mut bottled, "DROWNING_BEACON.pages.INITIAL.options.BOTTLE");
    let offered = bottled
        .state()
        .run
        .as_ref()
        .and_then(|run| run.reward_set.as_ref())
        .expect("the potion is offered");
    assert_eq!(
        offered.rewards[0].fingerprint.model_id.as_ref(),
        Some(&id("POTION.GLOWWATER_POTION"))
    );

    let mut climbed = Visit::new("EVENT.DROWNING_BEACON").hp(80, 80).enter();
    choose(&mut climbed, "DROWNING_BEACON.pages.INITIAL.options.CLIMB");
    assert_eq!(climbed.state().run_player.max_hp, 67);
    assert_eq!(climbed.state().run_player.current_hp, 67);
    assert!(holds_relic(&climbed, "RELIC.FRESNEL_LENS"));
}

#[test]
fn a_sunken_treasury_rolls_both_of_its_chests_before_either_is_opened() {
    // Both purses are rolled as the page's numbers are worked out, so the same
    // seed pays the same gold whichever chest is opened and the second one is
    // not the first.
    let mut small = Visit::new("EVENT.SUNKEN_TREASURY").enter();
    choose(
        &mut small,
        "SUNKEN_TREASURY.pages.INITIAL.options.FIRST_CHEST",
    );
    let small_gold = small.state().run_player.gold;
    assert!((52..=67).contains(&small_gold), "{small_gold}");

    let mut large = Visit::new("EVENT.SUNKEN_TREASURY").enter();
    choose(
        &mut large,
        "SUNKEN_TREASURY.pages.INITIAL.options.SECOND_CHEST",
    );
    let large_gold = large.state().run_player.gold;
    assert!((303..=363).contains(&large_gold), "{large_gold}");
    assert!(deck_holds(&large, "CARD.GREED"));
}

#[test]
fn a_morphic_grove_takes_every_coin_or_pays_five_maximum_hp() {
    let mut alone = Visit::new("EVENT.MORPHIC_GROVE")
        .gold(120)
        .hp(70, 80)
        .enter();
    choose(&mut alone, "MORPHIC_GROVE.pages.INITIAL.options.LONER");
    assert_eq!(alone.state().run_player.max_hp, 85);
    assert_eq!(alone.state().run_player.gold, 120);

    let mut grouped = Visit::new("EVENT.MORPHIC_GROVE")
        .gold(120)
        .deck(&["CARD.STRIKE_IRONCLAD", "CARD.DEFEND_IRONCLAD", "CARD.BASH"])
        .enter();
    choose(&mut grouped, "MORPHIC_GROVE.pages.INITIAL.options.GROUP");
    assert_eq!(grouped.state().run_player.gold, 0);
    let choice = grouped
        .state()
        .run
        .as_ref()
        .and_then(|run| run.suspended_choice.as_ref())
        .expect("the transform screen is standing");
    assert_eq!((choice.minimum, choice.maximum), (2, 2));
}

#[test]
fn an_unrest_site_sleeps_badly_or_kills_the_thing() {
    let mut rested = Visit::new("EVENT.UNREST_SITE").hp(40, 80).enter();
    choose(&mut rested, "UNREST_SITE.pages.INITIAL.options.REST");
    assert_eq!(rested.state().run_player.current_hp, 80);
    assert!(deck_holds(&rested, "CARD.POOR_SLEEP"));

    let mut killed = Visit::new("EVENT.UNREST_SITE")
        .hp(40, 80)
        .bag(&["RELIC.LANTERN"])
        .enter();
    choose(&mut killed, "UNREST_SITE.pages.INITIAL.options.KILL");
    assert_eq!(killed.state().run_player.max_hp, 72);
    assert_eq!(killed.state().run_player.current_hp, 40);
    // A relic pulled from the bag rolls a rarity and takes the front of that
    // shelf; a bag holding only commons hands over the common.
    assert!(holds_relic(&killed, "RELIC.LANTERN"));
}

#[test]
fn a_field_of_man_sized_holes_pays_two_cards_or_fits_one_perfectly() {
    let mut resisted = Visit::new("EVENT.FIELD_OF_MAN_SIZED_HOLES")
        .deck(&["CARD.STRIKE_IRONCLAD", "CARD.DEFEND_IRONCLAD", "CARD.BASH"])
        .enter();
    choose(
        &mut resisted,
        "FIELD_OF_MAN_SIZED_HOLES.pages.INITIAL.options.RESIST",
    );
    let choice = resisted
        .state()
        .run
        .as_ref()
        .and_then(|run| run.suspended_choice.as_ref())
        .expect("the removal screen is standing");
    assert_eq!((choice.minimum, choice.maximum), (2, 2));

    let mut entered = Visit::new("EVENT.FIELD_OF_MAN_SIZED_HOLES").enter();
    choose(
        &mut entered,
        "FIELD_OF_MAN_SIZED_HOLES.pages.INITIAL.options.ENTER_YOUR_HOLE",
    );
    let choice = entered
        .state()
        .run
        .as_ref()
        .and_then(|run| run.suspended_choice.as_ref())
        .expect("the enchantment screen is standing");
    assert_eq!((choice.minimum, choice.maximum), (1, 1));
}

/// The event choices the page offers, by place on the page.
fn event_choices(simulator: &Simulator) -> Vec<(usize, String)> {
    simulator
        .legal_actions()
        .iter()
        .filter_map(|action| match action {
            Action::ChooseEvent { index, option_id } => Some((*index, option_id.clone())),
            _ => None,
        })
        .collect()
}

/// A locked option is shown on the page but cannot be taken: it is left out
/// of the legal choices, and asking for it anyway is refused with the page
/// still standing.
#[test]
fn a_self_help_book_locks_the_reading_the_deck_has_no_card_for() {
    // The starter deck has attacks and skills but no power, so the whole
    // book cannot be read.
    let mut simulator = Visit::new("EVENT.SELF_HELP_BOOK").enter();
    assert_eq!(
        option_ids(&simulator),
        vec![
            "SELF_HELP_BOOK.pages.INITIAL.options.READ_THE_BACK",
            "SELF_HELP_BOOK.pages.INITIAL.options.READ_PASSAGE",
            "SELF_HELP_BOOK.pages.INITIAL.options.READ_ENTIRE_BOOK_LOCKED",
        ]
    );
    assert_eq!(
        event_choices(&simulator),
        vec![
            (
                0,
                "SELF_HELP_BOOK.pages.INITIAL.options.READ_THE_BACK".to_owned()
            ),
            (
                1,
                "SELF_HELP_BOOK.pages.INITIAL.options.READ_PASSAGE".to_owned()
            ),
        ],
        "the locked reading is not a choice"
    );
    let refused = simulator.step(Action::ChooseEvent {
        index: 2,
        option_id: "SELF_HELP_BOOK.pages.INITIAL.options.READ_ENTIRE_BOOK_LOCKED".to_owned(),
    });
    assert!(refused.is_err(), "the locked reading cannot be taken");
    assert_eq!(
        option_ids(&simulator).len(),
        3,
        "the page is still standing after the refusal"
    );

    // A power in the deck opens the reading.
    let powered = Visit::new("EVENT.SELF_HELP_BOOK")
        .deck(&[
            "CARD.STRIKE_IRONCLAD",
            "CARD.DEFEND_IRONCLAD",
            "CARD.INFLAME",
        ])
        .enter();
    assert_eq!(
        event_choices(&powered).len(),
        3,
        "every reading is open to a deck that has a card for each"
    );
}

#[test]
fn a_grave_of_the_forgotten_locks_its_first_option_without_an_exhausting_card() {
    // The Souls enchantment wants a card carrying Exhaust, which no starter
    // does.
    let plain = Visit::new("EVENT.GRAVE_OF_THE_FORGOTTEN").enter();
    assert_eq!(
        option_ids(&plain),
        vec![
            "GRAVE_OF_THE_FORGOTTEN.pages.INITIAL.options.CONFRONT_LOCKED",
            "GRAVE_OF_THE_FORGOTTEN.pages.INITIAL.options.ACCEPT",
        ]
    );

    let mut burning = Visit::new("EVENT.GRAVE_OF_THE_FORGOTTEN")
        .deck(&["CARD.TREMBLE"])
        .enter();
    assert_eq!(
        option_ids(&burning)[0],
        "GRAVE_OF_THE_FORGOTTEN.pages.INITIAL.options.CONFRONT"
    );
    choose(
        &mut burning,
        "GRAVE_OF_THE_FORGOTTEN.pages.INITIAL.options.CONFRONT",
    );
    assert!(deck_holds(&burning, "CARD.DECAY"));

    let mut accepted = Visit::new("EVENT.GRAVE_OF_THE_FORGOTTEN")
        .deck(&["CARD.TREMBLE"])
        .enter();
    choose(
        &mut accepted,
        "GRAVE_OF_THE_FORGOTTEN.pages.INITIAL.options.ACCEPT",
    );
    assert!(holds_relic(&accepted, "RELIC.FORGOTTEN_SOUL"));
}

#[test]
fn wood_carvings_turn_a_starter_into_a_named_card() {
    let mut carved = Visit::new("EVENT.WOOD_CARVINGS").enter();
    assert_eq!(
        option_ids(&carved),
        vec![
            "WOOD_CARVINGS.pages.INITIAL.options.BIRD",
            "WOOD_CARVINGS.pages.INITIAL.options.SNAKE",
            "WOOD_CARVINGS.pages.INITIAL.options.TORUS",
        ]
    );
    choose(&mut carved, "WOOD_CARVINGS.pages.INITIAL.options.BIRD");
    // The screen asks for one basic transformable card; the deck holds two, so
    // it stands rather than taking them both.
    let choice = carved
        .state()
        .run
        .as_ref()
        .and_then(|run| run.suspended_choice.as_ref())
        .expect("the transform screen is standing");
    assert_eq!(choice.candidates.len(), 2);
}

#[test]
fn a_waterlogged_scriptorium_locks_what_the_purse_cannot_reach() {
    let poor = Visit::new("EVENT.WATERLOGGED_SCRIPTORIUM").gold(55).enter();
    assert_eq!(
        option_ids(&poor),
        vec![
            "WATERLOGGED_SCRIPTORIUM.pages.INITIAL.options.BLOODY_INK",
            "WATERLOGGED_SCRIPTORIUM.pages.INITIAL.options.TENTACLE_QUILL",
            "WATERLOGGED_SCRIPTORIUM.pages.INITIAL.options.PRICKLY_SPONGE_LOCKED",
        ]
    );

    let mut rich = Visit::new("EVENT.WATERLOGGED_SCRIPTORIUM")
        .gold(99)
        .deck(&["CARD.STRIKE_IRONCLAD", "CARD.DEFEND_IRONCLAD", "CARD.BASH"])
        .enter();
    assert_eq!(
        option_ids(&rich)[2],
        "WATERLOGGED_SCRIPTORIUM.pages.INITIAL.options.PRICKLY_SPONGE"
    );
    choose(
        &mut rich,
        "WATERLOGGED_SCRIPTORIUM.pages.INITIAL.options.PRICKLY_SPONGE",
    );
    assert_eq!(rich.state().run_player.gold, 0);
    let choice = rich
        .state()
        .run
        .as_ref()
        .and_then(|run| run.suspended_choice.as_ref())
        .expect("the enchantment screen is standing");
    assert_eq!((choice.minimum, choice.maximum), (2, 2));
}

#[test]
fn a_luminous_choir_discounts_its_tribute_as_the_room_is_entered() {
    // The tribute opens at a hundred and fifty and is discounted by anything
    // from nothing to forty-nine, so it lands somewhere in a hundred and a
    // hundred and forty-nine, and the option is locked below it.
    let mut rich = Visit::new("EVENT.LUMINOUS_CHOIR")
        .gold(149)
        .bag(&["RELIC.LANTERN"])
        .enter();
    assert_eq!(
        option_ids(&rich)[1],
        "LUMINOUS_CHOIR.pages.INITIAL.options.OFFER_TRIBUTE"
    );
    choose(
        &mut rich,
        "LUMINOUS_CHOIR.pages.INITIAL.options.OFFER_TRIBUTE",
    );
    let paid = 149 - rich.state().run_player.gold;
    assert!((100..=149).contains(&paid), "{paid}");
    assert!(holds_relic(&rich, "RELIC.LANTERN"));

    // Below the discounted price the tribute is laid out locked, which is what
    // the page shows in place of an option nobody can take.
    let poor = Visit::new("EVENT.LUMINOUS_CHOIR")
        .gold(99)
        .bag(&["RELIC.LANTERN"])
        .enter();
    assert_eq!(
        option_ids(&poor)[1],
        "LUMINOUS_CHOIR.pages.INITIAL.options.OFFER_TRIBUTE_LOCKED"
    );
}

#[test]
fn the_abyssal_baths_pay_two_and_take_one_more_every_dip() {
    let mut simulator = Visit::new("EVENT.ABYSSAL_BATHS").hp(80, 80).enter();
    choose(
        &mut simulator,
        "ABYSSAL_BATHS.pages.INITIAL.options.IMMERSE",
    );
    // Two maximum HP, healed by as much, and then three that nothing blocks.
    assert_eq!(simulator.state().run_player.max_hp, 82);
    assert_eq!(simulator.state().run_player.current_hp, 79);
    // The page that follows carries the same two keys however many dips it is
    // on, which is the page the build names them under.
    assert_eq!(
        option_ids(&simulator),
        vec![
            "ABYSSAL_BATHS.pages.ALL.options.LINGER",
            "ABYSSAL_BATHS.pages.ALL.options.EXIT_BATHS",
        ]
    );
    choose(&mut simulator, "ABYSSAL_BATHS.pages.ALL.options.LINGER");
    assert_eq!(simulator.state().run_player.max_hp, 84);
    assert_eq!(simulator.state().run_player.current_hp, 77);
    choose(&mut simulator, "ABYSSAL_BATHS.pages.ALL.options.EXIT_BATHS");
    assert!(
        simulator
            .state()
            .run
            .as_ref()
            .is_some_and(|run| run.active_room.is_none())
    );

    let mut abstained = Visit::new("EVENT.ABYSSAL_BATHS").hp(40, 80).enter();
    choose(
        &mut abstained,
        "ABYSSAL_BATHS.pages.INITIAL.options.ABSTAIN",
    );
    assert_eq!(abstained.state().run_player.current_hp, 50);
}

#[test]
fn a_doll_room_shows_two_dolls_for_five_and_three_for_fifteen() {
    let mut some_time = Visit::new("EVENT.DOLL_ROOM").hp(80, 80).enter();
    some_time
        .step(Action::ChooseEvent {
            index: 1,
            option_id: "DOLL_ROOM.pages.INITIAL.options.TAKE_SOME_TIME".to_owned(),
        })
        .expect("taking some time applies");
    assert_eq!(some_time.state().run_player.current_hp, 75);
    // A doll's option is known by the doll's own name, not by a page key: the
    // recorder reads the same names off the screen.
    let shown = option_ids(&some_time);
    assert_eq!(shown.len(), 2);
    assert!(
        shown.iter().all(
            |option| ["Daughter of the Wind", "Mr. Struggles", "Bing Bong"]
                .contains(&option.as_str())
        ),
        "{shown:?}"
    );

    let mut examined = Visit::new("EVENT.DOLL_ROOM").hp(80, 80).enter();
    choose(&mut examined, "DOLL_ROOM.pages.INITIAL.options.EXAMINE");
    assert_eq!(examined.state().run_player.current_hp, 65);
    let shown = option_ids(&examined);
    assert_eq!(shown.len(), 3);
    let taken = shown[0].clone();
    choose(&mut examined, &taken);
    assert_eq!(examined.state().run_player.relics.len(), 2);

    let mut random = Visit::new("EVENT.DOLL_ROOM").enter();
    choose(&mut random, "DOLL_ROOM.pages.INITIAL.options.RANDOM");
    assert!(
        holds_relic(&random, "RELIC.DAUGHTER_OF_THE_WIND")
            || holds_relic(&random, "RELIC.MR_STRUGGLES")
            || holds_relic(&random, "RELIC.BING_BONG")
    );
}

#[test]
fn a_relic_trader_pairs_three_of_yours_against_three_of_its_own() {
    let mut simulator = Visit::new("EVENT.RELIC_TRADER")
        .bag(&["RELIC.LANTERN"])
        .relics(&[
            "RELIC.ART_OF_WAR",
            "RELIC.BAG_OF_MARBLES",
            "RELIC.BRONZE_SCALES",
            "RELIC.CENTENNIAL_PUZZLE",
            "RELIC.HAPPY_FLOWER",
        ])
        .enter();
    assert_eq!(
        option_ids(&simulator),
        vec![
            "RELIC_TRADER.pages.INITIAL.options.TOP",
            "RELIC_TRADER.pages.INITIAL.options.MIDDLE",
            "RELIC_TRADER.pages.INITIAL.options.BOTTOM",
        ]
    );
    let before = simulator.state().run_player.relics.len();
    choose(&mut simulator, "RELIC_TRADER.pages.INITIAL.options.TOP");
    // One out, one in.
    assert_eq!(simulator.state().run_player.relics.len(), before);
    assert!(holds_relic(&simulator, "RELIC.LANTERN"));
}

#[test]
fn wongos_locks_what_the_purse_cannot_reach_and_charges_on_the_way_out() {
    let poor = Visit::new("EVENT.WELCOME_TO_WONGOS")
        .gold(100)
        .bag(&["RELIC.LANTERN"])
        .enter();
    assert_eq!(
        option_ids(&poor),
        vec![
            "WELCOME_TO_WONGOS.pages.INITIAL.options.BARGAIN_BIN",
            "WELCOME_TO_WONGOS.pages.INITIAL.options.FEATURED_ITEM_LOCKED",
            "WELCOME_TO_WONGOS.pages.INITIAL.options.MYSTERY_BOX_LOCKED",
            "WELCOME_TO_WONGOS.pages.INITIAL.options.LEAVE",
        ]
    );

    let mut rich = Visit::new("EVENT.WELCOME_TO_WONGOS")
        .gold(300)
        .bag(&["RELIC.LANTERN"])
        .enter();
    choose(
        &mut rich,
        "WELCOME_TO_WONGOS.pages.INITIAL.options.MYSTERY_BOX",
    );
    assert_eq!(rich.state().run_player.gold, 0);
    assert!(holds_relic(&rich, "RELIC.WONGOS_MYSTERY_TICKET"));

    let mut bought = Visit::new("EVENT.WELCOME_TO_WONGOS")
        .gold(150)
        .bag(&["RELIC.LANTERN"])
        .enter();
    choose(
        &mut bought,
        "WELCOME_TO_WONGOS.pages.INITIAL.options.BARGAIN_BIN",
    );
    assert_eq!(bought.state().run_player.gold, 50);
    assert!(holds_relic(&bought, "RELIC.LANTERN"));
}

#[test]
fn a_punch_off_nabs_an_injury_and_a_relic_or_takes_them_both_on() {
    let mut nabbed = Visit::new("EVENT.PUNCH_OFF")
        .bag(&["RELIC.LANTERN"])
        .enter();
    let before = nabbed
        .state()
        .player_rng
        .stream(sts2_rng::PlayerStream::Rewards)
        .counter();
    choose(&mut nabbed, "PUNCH_OFF.pages.INITIAL.options.NAB");
    assert!(deck_holds(&nabbed, "CARD.INJURY"));
    let offered = nabbed
        .state()
        .run
        .as_ref()
        .and_then(|run| run.reward_set.as_ref())
        .expect("the relic is offered");
    assert_eq!(offered.rewards.len(), 1);
    assert_eq!(offered.rewards[0].fingerprint.reward_type, "relic");
    // The relic is pulled as the set is built rather than when it is taken, so
    // the screen names it, and the rarity behind the pull costs the rewards
    // stream one read.
    assert_eq!(
        offered.rewards[0].fingerprint.model_id.as_ref(),
        Some(&id("RELIC.LANTERN"))
    );
    let after = nabbed
        .state()
        .player_rng
        .stream(sts2_rng::PlayerStream::Rewards)
        .counter();
    assert_eq!(after - before, 1, "a rarity, and nothing more");

    let mut fought = Visit::new("EVENT.PUNCH_OFF")
        .bag(&["RELIC.LANTERN"])
        .enter();
    choose(
        &mut fought,
        "PUNCH_OFF.pages.INITIAL.options.I_CAN_TAKE_THEM",
    );
    assert_eq!(
        option_ids(&fought),
        vec!["PUNCH_OFF.pages.I_CAN_TAKE_THEM.options.FIGHT"]
    );
    choose(&mut fought, "PUNCH_OFF.pages.I_CAN_TAKE_THEM.options.FIGHT");
    // The fight the event was standing in front of is the one that starts.
    assert!(fought.state().combat.is_some());
}

#[test]
fn dusty_tome_grants_each_characters_eligible_ancient_upgraded() {
    for (character, card) in [
        ("CHARACTER.IRONCLAD", "CARD.CORRUPTION"),
        ("CHARACTER.SILENT", "CARD.WRAITH_FORM"),
        ("CHARACTER.NECROBINDER", "CARD.FORBIDDEN_GRIMOIRE"),
        ("CHARACTER.REGENT", "CARD.THE_SEALED_THRONE"),
        ("CHARACTER.DEFECT", "CARD.BIASED_COGNITION"),
    ] {
        let mut simulator = ["N3Z9EE2CXB", "N3Z9EE2CXA", "N3Z9EE2CXC", "N3Z9EE2CXD"]
            .into_iter()
            .map(|seed| {
                Visit::new("EVENT.DARV")
                    .seed(seed)
                    .character(character)
                    .act(1)
                    .enter()
            })
            .find(|simulator| {
                option_ids(simulator)
                    .iter()
                    .any(|option| option == "DARV.pages.INITIAL.options.DUSTY_TOME")
            })
            .expect("one of the scenarios offers the Tome");
        let rewards_before = simulator
            .state()
            .player_rng
            .stream(sts2_rng::PlayerStream::Rewards)
            .counter();
        assert_eq!(rewards_before, 1, "setting up the Tome consumes one draw");
        let deck_before = simulator.state().run_player.deck.len();
        choose(&mut simulator, "DARV.pages.INITIAL.options.DUSTY_TOME");
        let player = &simulator.state().run_player;
        assert_eq!(player.deck.len(), deck_before + 1);
        let added = player.deck.last().unwrap();
        assert_eq!(added.model_id, id(card), "{character}");
        assert_eq!(added.upgrade_level, 1, "{character}");
        let tome = player
            .relics
            .iter()
            .find(|relic| relic.model_id == id("RELIC.DUSTY_TOME"))
            .unwrap();
        assert_eq!(
            tome.saved_models,
            vec![("AncientCard".to_owned(), id(card))]
        );
        assert_eq!(
            simulator
                .state()
                .player_rng
                .stream(sts2_rng::PlayerStream::Rewards)
                .counter(),
            rewards_before,
            "claiming the Tome must not roll its card again"
        );
    }
}

#[test]
fn tanx_shuffles_ten_weapons_and_offers_three() {
    // A starter deck carries two attacks, which is one short of the three
    // instincts the boomerang wants, so it is not in the shuffle.
    let plain = Visit::new("EVENT.TANX")
        .deck(&["CARD.STRIKE_IRONCLAD", "CARD.DEFEND_IRONCLAD"])
        .enter();
    let offered = option_ids(&plain);
    assert_eq!(offered.len(), 3);
    assert!(
        offered
            .iter()
            .all(|option| option.starts_with("TANX.pages.INITIAL.options.")),
        "{offered:?}"
    );
    assert!(!offered.contains(&"TANX.pages.INITIAL.options.TRI_BOOMERANG".to_owned()));

    // The ancient restores whoever walks in before it says anything.
    let healed = Visit::new("EVENT.TANX").hp(40, 80).enter();
    assert_eq!(healed.state().run_player.current_hp, 80);
}

/// Plays whatever the hand can pay for.
/// An event the run had begun is not part of the save. A load builds the run's
/// list of events fresh, so a page that was still standing when the run quit —
/// a punch-off's fight option stands until the fight it started is over — is
/// gone once the run is resumed, and nothing is projected until the next event
/// begins.
#[test]
fn a_resumed_run_has_no_event_page_standing() {
    let mut fought = Visit::new("EVENT.PUNCH_OFF")
        .bag(&["RELIC.LANTERN"])
        .enter();
    choose(
        &mut fought,
        "PUNCH_OFF.pages.INITIAL.options.I_CAN_TAKE_THEM",
    );
    choose(&mut fought, "PUNCH_OFF.pages.I_CAN_TAKE_THEM.options.FIGHT");
    assert!(fought.state().combat.is_some(), "the fight is on");
    let standing = |simulator: &Simulator| {
        simulator
            .state()
            .run
            .as_ref()
            .and_then(|run| run.begun_event.as_ref())
            .map(|begun| begun.options.len())
    };
    assert_eq!(
        standing(&fought),
        Some(1),
        "the fight option stands while the fight is on"
    );

    fought.resumed_from_save().expect("the run resumes");
    assert_eq!(
        standing(&fought),
        None,
        "and is gone once the run is resumed"
    );
}

/// A load into an event's own room enters the room again, and the event begins
/// afresh off the same stream: the page it opened with stands once the run is
/// resumed. Seed `XGTTW8TTEQ`, record 1417, a quit on an ancient's first page.
#[test]
fn a_run_resumed_in_an_event_room_has_its_page_standing() {
    let mut visited = Visit::new("EVENT.PUNCH_OFF")
        .bag(&["RELIC.LANTERN"])
        .enter();
    let standing = |simulator: &Simulator| {
        simulator
            .state()
            .run
            .as_ref()
            .and_then(|run| run.begun_event.as_ref())
            .map(|begun| {
                begun
                    .options
                    .iter()
                    .map(|option| option.option_id.clone())
                    .collect::<Vec<_>>()
            })
    };
    let before = standing(&visited);
    assert!(before.is_some(), "the first page is up");
    visited.resumed_from_save().expect("the run resumes");
    assert_eq!(standing(&visited), before, "and is up again after the load");
    assert_eq!(option_ids(&visited), before.expect("the page is up"));
}

fn play_something(simulator: &mut Simulator) {
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

fn proceed(simulator: &mut Simulator) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::Proceed))
        .cloned()
        .expect("proceeding is offered");
    simulator.step(action).expect("proceeding applies");
}

/// Puts the one enemy of the standing fight on its last point of health, so a
/// test that is about what the fight pays out does not have to win it honestly.
fn leave_the_enemy_on_one(simulator: &Simulator) -> Simulator {
    let mut state = simulator.state().clone();
    for creature in &mut state
        .combat
        .as_mut()
        .expect("a fight is standing")
        .creatures
    {
        if creature.side == sts2_engine::CombatSide::Enemy {
            creature.current_hp = 1;
        }
    }
    Simulator::from_generated_state(state, sts2_content::standard_registry())
        .expect("the fight stands back up")
}

#[test]
fn a_battleworn_dummy_fights_the_setting_it_was_put_on_and_pays_for_the_win() {
    let mut simulator = Visit::new("EVENT.BATTLEWORN_DUMMY")
        .bag(&["RELIC.LANTERN"])
        .enter();
    assert_eq!(
        option_ids(&simulator),
        vec![
            "BATTLEWORN_DUMMY.pages.INITIAL.options.SETTING_1",
            "BATTLEWORN_DUMMY.pages.INITIAL.options.SETTING_2",
            "BATTLEWORN_DUMMY.pages.INITIAL.options.SETTING_3",
        ]
    );
    choose(
        &mut simulator,
        "BATTLEWORN_DUMMY.pages.INITIAL.options.SETTING_3",
    );
    // The setting the option wrote on the encounter is what it fields.
    let combat = simulator.state().combat.as_ref().expect("the fight starts");
    assert!(
        combat
            .creatures
            .iter()
            .any(|creature| creature.model_id == id("MONSTER.BATTLE_FRIEND_V3"))
    );

    let mut won = leave_the_enemy_on_one(&simulator);
    let sets_before = next_reward_set_id(&won);
    while won.state().combat.is_some()
        && won
            .legal_actions()
            .iter()
            .any(|action| matches!(action, Action::PlayCard { .. }))
    {
        play_something(&mut won);
    }
    // The kill does not end the fight: the game shows a proceed over the
    // fight as the blow left it, and ends it only on that proceed. So the
    // fight is still standing here, its turn still in its play phase, and
    // nothing that ends a fight has run yet.
    let standing = won.state().combat.as_ref().expect("the fight stands");
    assert_ne!(standing.player.phase, sts2_engine::PlayerTurnPhase::None);
    assert!(
        matches!(won.decision(), sts2_engine::DecisionContext::RoomProceed),
        "the proceed is what is offered: {:?}",
        won.decision()
    );
    // The fight pays nothing, and proceeding past it goes back to the event
    // rather than to the map: the event pays the setting's prize on a page
    // that is finished, and what is left to do is walk on.
    proceed(&mut won);
    assert!(won.state().combat.is_none(), "the proceed ended the fight");
    assert!(holds_relic(&won, "RELIC.LANTERN"));
    assert_only_the_map_is_left(&won);
    // The fight itself laid out no reward set; the relic came straight off
    // the event, so no identifier was spent on the way.
    assert_eq!(next_reward_set_id(&won), sets_before);
}

/// The finished dummy page: still the room the run is standing in, with
/// nothing on it to choose, so the decision is the map's and nothing else —
/// no proceed, and no event choice. The map these tests walk ends at the
/// event, so the walk itself has nowhere to go; what is pinned is the
/// decision being the map's.
fn assert_only_the_map_is_left(simulator: &Simulator) {
    assert!(option_ids(simulator).is_empty(), "the page is finished");
    assert_eq!(
        sts2_replay::observation::render(simulator)["choices"]["events"],
        serde_json::json!([])
    );
    assert!(
        matches!(
            simulator.decision(),
            sts2_engine::DecisionContext::MapNavigation { .. }
        ),
        "the map is what is left: {:?}",
        simulator.decision()
    );
    let actions = simulator.legal_actions();
    assert!(
        actions
            .iter()
            .all(|action| matches!(action, Action::ChooseMap { .. })),
        "nothing but the map is offered: {actions:?}"
    );
}

fn won_dummy_before_treasure() -> Simulator {
    let mut simulator = Visit::new("EVENT.BATTLEWORN_DUMMY")
        .deck(&["CARD.STRIKE_IRONCLAD", "CARD.DEFEND_IRONCLAD"])
        .enter();
    choose(
        &mut simulator,
        "BATTLEWORN_DUMMY.pages.INITIAL.options.SETTING_2",
    );
    let mut won = leave_the_enemy_on_one(&simulator);
    while won
        .legal_actions()
        .iter()
        .any(|action| matches!(action, Action::PlayCard { .. }))
    {
        play_something(&mut won);
    }
    let mut state = won.state().clone();
    let run = state.run.as_mut().unwrap();
    let map = &mut run.maps[run.current_act];
    let next = MapCoord { col: 0, row: 2 };
    map.height = 3;
    map.boss = next;
    map.points.last_mut().unwrap().children.push(next);
    map.points.push(MapPoint {
        coord: next,
        point_type: MapPointType::Treasure,
        children: Vec::new(),
        can_modify: true,
    });
    Simulator::from_generated_state(state, sts2_content::standard_registry()).unwrap()
}

#[test]
fn a_dummy_exit_precedes_resume_but_its_settled_rewards_are_still_verified() {
    use serde_json::json;
    use sts2_replay::{AdapterErrorCode, Record, ReplayAdapter, observation::render};

    let won = won_dummy_before_treasure();
    // The parent remains visible at the child's exit, before Resume upgrades
    // the deck and finishes the event. No recorded fixture is needed.
    let mut exiting = won.state().clone();
    exiting.combat = None;
    exiting.run.as_mut().unwrap().pushed_room = None;
    let exiting =
        Simulator::from_generated_state(exiting, sts2_content::standard_registry()).unwrap();
    let before_resume = render(&exiting);
    let mut adapter = ReplayAdapter::new(won).verifying_observations();
    adapter
        .accept(&Record {
            sequence: 1,
            kind: "room.proceed".into(),
            payload: json!({"data": {}}),
        })
        .unwrap();
    adapter
        .accept(&Record {
            sequence: 2,
            kind: "room.exit".into(),
            payload: json!({"data": {"stack_depth": 2}}),
        })
        .unwrap();
    adapter
        .accept(&Record {
            sequence: 3,
            kind: "state.observe".into(),
            payload: json!({"after": 2, "context": "room.exit", "state": before_resume}),
        })
        .unwrap();
    adapter.verify_pending_observation().unwrap();
    assert_eq!(adapter.dropped_room_entries(), 1);
    assert_eq!(adapter.compared_observations(), 0);
    let settled = render(adapter.simulator());
    assert_eq!(settled["players"][0]["deck"][0]["upgrade_level"], 1);
    assert_eq!(settled["players"][0]["deck"][1]["upgrade_level"], 1);
    assert_eq!(settled["choices"]["events"], json!([]));
    adapter
        .accept(&Record {
            sequence: 4,
            kind: "state.observe".into(),
            payload: json!({"after": 1, "context": "action.settled", "state": settled}),
        })
        .unwrap();
    adapter.verify_pending_observation().unwrap();
    assert_eq!(adapter.compared_observations(), 1);
    // A wrong reward in a settled observation must remain a divergence.
    let mut incorrect = settled;
    incorrect["players"][0]["deck"][0]["upgrade_level"] = json!(0);
    let mut incorrect_adapter = adapter.clone();
    incorrect_adapter
        .accept(&Record {
            sequence: 5,
            kind: "state.observe".into(),
            payload: json!({"after": 1, "context": "action.settled", "state": incorrect}),
        })
        .unwrap();
    assert_eq!(
        incorrect_adapter
            .verify_pending_observation()
            .unwrap_err()
            .code,
        AdapterErrorCode::StateDivergence
    );
    adapter
        .accept(&Record {
            sequence: 5,
            kind: "map.choose".into(),
            payload: json!({"data": {"col": 0, "row": 2}}),
        })
        .unwrap();
    let mut next_room = render(adapter.simulator());
    assert_eq!(next_room["choices"]["events"], json!([]));
    // An ordinary room exit after a different action is not exempt, even
    // when the preceding room happened to resume an event.
    next_room["players"][0]["deck"][0]["upgrade_level"] = json!(0);
    adapter
        .accept(&Record {
            sequence: 6,
            kind: "state.observe".into(),
            payload: json!({"after": 5, "context": "room.exit", "state": next_room}),
        })
        .unwrap();
    assert_eq!(
        adapter.verify_pending_observation().unwrap_err().code,
        AdapterErrorCode::StateDivergence
    );
}

#[test]
fn a_battleworn_dummy_pays_nothing_when_the_clock_runs_out() {
    let mut simulator = Visit::new("EVENT.BATTLEWORN_DUMMY")
        .bag(&["RELIC.LANTERN"])
        .enter();
    choose(
        &mut simulator,
        "BATTLEWORN_DUMMY.pages.INITIAL.options.SETTING_1",
    );
    let sets_before = next_reward_set_id(&simulator);
    // Three turns, and the friend walks out of the fight on the last one.
    for _ in 0..3 {
        end_turn(&mut simulator);
    }
    assert!(
        simulator.state().combat.is_none() || {
            proceed(&mut simulator);
            true
        }
    );
    assert!(!holds_relic(&simulator, "RELIC.LANTERN"));
    assert_only_the_map_is_left(&simulator);
    assert!(
        simulator
            .state()
            .run
            .as_ref()
            .is_some_and(|run| run.reward_set.is_none())
    );
    // The fight laid out no reward set, so the next set the run offers takes
    // the identifier this one would otherwise have taken.
    assert_eq!(next_reward_set_id(&simulator), sets_before);
}

/// The identifier the run's next reward set will take.
fn next_reward_set_id(simulator: &Simulator) -> u64 {
    simulator
        .state()
        .run
        .as_ref()
        .expect("the run is standing")
        .next_reward_set_id
}

/// What the belt's grab says it costs and does.
fn grab_on_the_belt(simulator: &Simulator) -> (Vec<EventEffect>, i32) {
    page(simulator)
        .into_iter()
        .find_map(|option| match option.effects.into_iter().next() {
            Some(EventEffect::GrabOffTheBelt { body, cost, .. }) => Some((body, cost)),
            _ => None,
        })
        .expect("the belt offers a grab")
}

#[test]
fn an_endless_conveyor_rolls_a_dish_and_puts_the_next_one_up_behind_it() {
    let mut simulator = Visit::new("EVENT.ENDLESS_CONVEYOR").gold(200).enter();
    let page = option_ids(&simulator);
    assert_eq!(page.len(), 2);
    assert!(
        page[0].starts_with("ENDLESS_CONVEYOR.pages.ALL.options."),
        "{page:?}"
    );
    // The golden fysh only joins the list from the second roll on.
    assert_ne!(page[0], "ENDLESS_CONVEYOR.pages.ALL.options.GOLDEN_FYSH");
    assert_eq!(
        page[1],
        "ENDLESS_CONVEYOR.pages.INITIAL.options.OBSERVE_CHEF"
    );

    // The grab says what the dish costs and what it does, rather than leaving
    // both behind an index into a list only the registry holds.
    let (body, cost) = grab_on_the_belt(&simulator);
    assert_eq!(cost, 40, "the plate's price is on the option");
    assert!(!body.is_empty(), "and so is the dish's own body");

    let first = page[0].clone();
    choose(&mut simulator, &first);
    // The plate is paid for and the next one is standing there.
    assert_eq!(simulator.state().run_player.gold, 160);
    let next = option_ids(&simulator);
    assert_eq!(next.len(), 2);
    assert_eq!(
        next[1],
        "ENDLESS_CONVEYOR.pages.GRAB_SOMETHING_OFF_THE_BELT.options.LEAVE"
    );
    // A dish never follows itself.
    assert_ne!(next[0], first);
    choose(
        &mut simulator,
        "ENDLESS_CONVEYOR.pages.GRAB_SOMETHING_OFF_THE_BELT.options.LEAVE",
    );
    assert!(
        simulator
            .state()
            .run
            .as_ref()
            .is_some_and(|run| run.active_room.is_none())
    );
}

#[test]
fn an_endless_conveyor_locks_the_belt_when_the_purse_runs_out() {
    // The belt is only offered at a hundred and twenty gold or more; two
    // plates leave forty and the third is the last the purse can pay for.
    let mut simulator = Visit::new("EVENT.ENDLESS_CONVEYOR").gold(120).enter();
    for _ in 0..3 {
        let grab = option_ids(&simulator)[0].clone();
        assert_ne!(grab, "ENDLESS_CONVEYOR.pages.ALL.options.LOCKED");
        choose(&mut simulator, &grab);
        // A plate that put a potion on the table is not done with until the
        // potion is taken or walked away from, and the belt waits for it.
        if simulator
            .state()
            .run
            .as_ref()
            .is_some_and(|run| run.reward_set.is_some())
        {
            proceed(&mut simulator);
        }
    }
    assert_eq!(simulator.state().run_player.gold, 0);
    assert_eq!(
        option_ids(&simulator)[0],
        "ENDLESS_CONVEYOR.pages.ALL.options.LOCKED"
    );
}

#[test]
fn an_endless_conveyor_serves_the_salad_every_fifth_plate() {
    let mut simulator = Visit::new("EVENT.ENDLESS_CONVEYOR").gold(1000).enter();
    for _ in 0..4 {
        let grab = option_ids(&simulator)[0].clone();
        choose(&mut simulator, &grab);
        // A dish that opened a screen leaves it standing over the page.
        if simulator
            .state()
            .run
            .as_ref()
            .is_some_and(|run| run.reward_set.is_some())
        {
            proceed(&mut simulator);
        }
    }
    assert_eq!(
        option_ids(&simulator)[0],
        "ENDLESS_CONVEYOR.pages.ALL.options.SEAPUNK_SALAD"
    );
    choose(
        &mut simulator,
        "ENDLESS_CONVEYOR.pages.ALL.options.SEAPUNK_SALAD",
    );
    assert!(deck_holds(&simulator, "CARD.FEEDING_FRENZY"));
}

/// The belt turns after the plate has been eaten, not while the player is
/// still looking at a screen the plate opened. A jelly liver puts a card
/// selection up and transforms what is chosen, and both the transform and the
/// roll that decides the next dish read the event's own die — so the order they
/// read it in decides both what the deck gets and what comes round next.
#[test]
fn a_jelly_liver_turns_the_belt_only_once_its_screen_is_answered() {
    let mut simulator = Visit::new("EVENT.ENDLESS_CONVEYOR").gold(2000).enter();
    let liver = "ENDLESS_CONVEYOR.pages.ALL.options.JELLY_LIVER";
    for _ in 0..40 {
        if option_ids(&simulator)[0] == liver {
            break;
        }
        let grab = option_ids(&simulator)[0].clone();
        choose(&mut simulator, &grab);
        if simulator
            .state()
            .run
            .as_ref()
            .is_some_and(|run| run.reward_set.is_some())
        {
            proceed(&mut simulator);
        }
    }
    assert_eq!(option_ids(&simulator)[0], liver, "the liver came up");

    choose(&mut simulator, liver);

    // The screen the liver opened is standing, and the belt has not turned:
    // the page is still the one the grab was taken from.
    let choice = simulator
        .legal_actions()
        .iter()
        .find_map(|action| match action {
            Action::ChooseCards { choice_id, cards } => Some((*choice_id, cards.clone())),
            _ => None,
        })
        .expect("the liver asks which card to transform");
    assert_eq!(
        option_ids(&simulator)[0],
        liver,
        "the belt has not turned while the screen is up"
    );

    let (choice_id, candidates) = choice;
    simulator
        .step(Action::ChooseCards {
            choice_id,
            cards: vec![candidates[0].clone()],
        })
        .expect("the pick applies");

    // Now the belt has turned, and it does not serve the same plate twice
    // running.
    let next = option_ids(&simulator)[0].clone();
    assert!(
        next.starts_with("ENDLESS_CONVEYOR.pages.ALL.options."),
        "a plate is on the belt again, not {next}"
    );
    assert_ne!(next, liver, "and it is not the one just eaten");
}

/// Where the page's own die stands.
fn event_die(simulator: &Simulator) -> u64 {
    match simulator
        .state()
        .run
        .as_ref()
        .and_then(|run| run.active_room.as_ref())
    {
        Some(ActiveRoom::Event { rng_counter, .. }) => *rng_counter,
        other => panic!("expected an event page, found {other:?}"),
    }
}

/// Grabs plates off the belt, answering whatever each one puts up, until the
/// one named is on the belt.
fn bring_round(simulator: &mut Simulator, dish: &str) {
    for _ in 0..60 {
        if option_ids(simulator)[0] == dish {
            return;
        }
        let grab = option_ids(simulator)[0].clone();
        choose(simulator, &grab);
        while simulator
            .state()
            .run
            .as_ref()
            .is_some_and(|run| run.reward_set.is_some())
        {
            proceed(simulator);
        }
    }
    panic!("{dish} never came round");
}

/// A suspicious condiment puts a potion on the table, and the grab waits on
/// the offer: the belt turns only once the potion has been taken. The recorder
/// shows the initial page — the condiment still on it — over the potion, and
/// the next plate only after the claim. Seed `EG700HG8FW`, record 407.
#[test]
fn a_suspicious_condiment_turns_the_belt_only_once_its_potion_is_taken() {
    let mut simulator = Visit::new("EVENT.ENDLESS_CONVEYOR").gold(4000).enter();
    let condiment = "ENDLESS_CONVEYOR.pages.ALL.options.SUSPICIOUS_CONDIMENT";
    bring_round(&mut simulator, condiment);
    let counter_before = event_die(&simulator);

    choose(&mut simulator, condiment);

    // The potion is on the table and the page behind it has not moved: the
    // belt has not turned, and its die has not been read.
    assert!(
        simulator
            .state()
            .run
            .as_ref()
            .is_some_and(|run| run.reward_set.is_some()),
        "a potion is offered"
    );
    assert_eq!(
        option_ids(&simulator)[0],
        condiment,
        "the belt has not turned while the potion stands"
    );
    assert_eq!(
        event_die(&simulator),
        counter_before,
        "the next plate is not rolled while the potion stands"
    );

    let claim = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ClaimReward { .. }))
        .cloned()
        .expect("the potion is claimable");
    simulator.step(claim).expect("the claim applies");

    // Now the belt has turned, the page is the belt's own, and the event is
    // still standing rather than finished by the claim.
    let next = option_ids(&simulator);
    assert!(
        next[0].starts_with("ENDLESS_CONVEYOR.pages.ALL.options."),
        "a plate is on the belt again, not {}",
        next[0]
    );
    assert_ne!(next[0], condiment, "and it is not the one just eaten");
    assert_eq!(
        next[1],
        "ENDLESS_CONVEYOR.pages.GRAB_SOMETHING_OFF_THE_BELT.options.LEAVE"
    );
    assert_eq!(
        event_die(&simulator),
        counter_before + 1,
        "the next plate was rolled once the potion was taken"
    );
    assert!(simulator.state().run.as_ref().unwrap().pending.is_empty());
    assert!(
        simulator
            .state()
            .run
            .as_ref()
            .unwrap()
            .owed_by_rewards
            .is_empty()
    );
}

/// Walking away from the potion finishes the offer the same as taking it
/// would, and the grab goes on from there: the belt turns, and the room is
/// not left.
#[test]
fn a_suspicious_condiment_passed_over_still_turns_the_belt() {
    let mut simulator = Visit::new("EVENT.ENDLESS_CONVEYOR").gold(4000).enter();
    let condiment = "ENDLESS_CONVEYOR.pages.ALL.options.SUSPICIOUS_CONDIMENT";
    bring_round(&mut simulator, condiment);

    choose(&mut simulator, condiment);
    assert_eq!(option_ids(&simulator)[0], condiment);
    proceed(&mut simulator);

    let next = option_ids(&simulator);
    assert!(
        next[0].starts_with("ENDLESS_CONVEYOR.pages.ALL.options."),
        "a plate is on the belt again, not {}",
        next[0]
    );
    assert_ne!(next[0], condiment);
    assert_eq!(
        next[1],
        "ENDLESS_CONVEYOR.pages.GRAB_SOMETHING_OFF_THE_BELT.options.LEAVE"
    );
    assert!(
        simulator
            .state()
            .run
            .as_ref()
            .is_some_and(|run| run.reward_set.is_none() && run.active_room.is_some()),
        "the potion is gone and the room is still standing"
    );
}

/// A fried eel is a plate, not a card reward: the colourless card it puts in
/// the deck costs a roll for its rarity and a roll for the card, and no third
/// roll for an upgrade the plate never offers.
#[test]
fn a_fried_eel_costs_two_reads_of_the_rewards_stream() {
    let mut simulator = Visit::new("EVENT.ENDLESS_CONVEYOR").gold(2000).enter();
    let eel = "ENDLESS_CONVEYOR.pages.ALL.options.FRIED_EEL";
    // The belt rolls what it serves, so eat along it until the eel comes up.
    for _ in 0..40 {
        if option_ids(&simulator)[0] == eel {
            break;
        }
        let grab = option_ids(&simulator)[0].clone();
        choose(&mut simulator, &grab);
        if simulator
            .state()
            .run
            .as_ref()
            .is_some_and(|run| run.reward_set.is_some())
        {
            proceed(&mut simulator);
        }
    }
    assert_eq!(option_ids(&simulator)[0], eel, "the eel came up");

    let before = simulator
        .state()
        .player_rng
        .stream(sts2_rng::PlayerStream::Rewards)
        .counter();
    choose(&mut simulator, eel);
    let after = simulator
        .state()
        .player_rng
        .stream(sts2_rng::PlayerStream::Rewards)
        .counter();
    assert_eq!(after - before, 2, "a rarity and a card, and nothing more");
}

#[test]
fn observing_the_chef_upgrades_one_card_and_leaves() {
    let mut simulator = Visit::new("EVENT.ENDLESS_CONVEYOR")
        .gold(200)
        .deck(&["CARD.STRIKE_IRONCLAD"])
        .enter();
    choose(
        &mut simulator,
        "ENDLESS_CONVEYOR.pages.INITIAL.options.OBSERVE_CHEF",
    );
    assert_eq!(simulator.state().run_player.deck[0].upgrade_level, 1);
    assert!(
        simulator
            .state()
            .run
            .as_ref()
            .is_some_and(|run| run.active_room.is_none())
    );
}

#[test]
fn tinker_time_builds_a_mad_science_two_choices_at_a_time() {
    let mut simulator = Visit::new("EVENT.TINKER_TIME").enter();
    assert_eq!(
        option_ids(&simulator),
        vec!["TINKER_TIME.pages.INITIAL.options.CHOOSE_CARD_TYPE"]
    );
    choose(
        &mut simulator,
        "TINKER_TIME.pages.INITIAL.options.CHOOSE_CARD_TYPE",
    );
    // Two of the three types.
    let types = option_ids(&simulator);
    assert_eq!(types.len(), 2);
    assert!(
        types
            .iter()
            .all(|option| option.starts_with("TINKER_TIME.pages.CHOOSE_CARD_TYPE.options.")),
        "{types:?}"
    );
    let chosen = types[0].clone();
    choose(&mut simulator, &chosen);

    // Two of that type's three riders.
    let riders = option_ids(&simulator);
    assert_eq!(riders.len(), 2);
    assert!(
        riders
            .iter()
            .all(|option| option.starts_with("TINKER_TIME.pages.CHOOSE_RIDER.options.")),
        "{riders:?}"
    );
    let rider = riders[0].clone();
    choose(&mut simulator, &rider);

    let made = simulator
        .state()
        .run_player
        .deck
        .iter()
        .find(|card| card.model_id == id("CARD.MAD_SCIENCE"))
        .expect("the card the table built joins the deck");
    // Both of what the table wrote are on it, and the type it wrote matches the
    // page the rider came off.
    let card_type = made
        .properties
        .get("TinkerTimeType")
        .and_then(serde_json::Value::as_i64);
    let written = made
        .properties
        .get("TinkerTimeRider")
        .and_then(serde_json::Value::as_i64);
    assert!(matches!(card_type, Some(1..=3)), "{card_type:?}");
    let expected_type = match chosen.rsplit('.').next() {
        Some("ATTACK") => 1,
        Some("SKILL") => 2,
        _ => 3,
    };
    assert_eq!(card_type, Some(expected_type));
    let expected_riders: &[i64] = match expected_type {
        1 => &[1, 2, 3],
        2 => &[4, 5, 6],
        _ => &[7, 8, 9],
    };
    assert!(
        written.is_some_and(|value| expected_riders.contains(&value)),
        "{written:?}"
    );
}

/// Every event an act can put in its own room pool has a page. The act's walk
/// asks each one whether it may be pulled and then stands the room up, so an
/// event on a list with nothing registered for it would stop a run cold.
#[test]
fn every_event_an_act_can_offer_is_registered() {
    let registry = sts2_content::standard_registry();
    for act in sts2_content::implemented_act_ids() {
        for event in sts2_content::act_event_ids(&act) {
            assert!(
                registry.event(&event).is_some(),
                "{event} is on {act}'s list with nothing registered for it"
            );
        }
    }
}

/// Pages without a scenario of their own elsewhere in this file. Each is
/// walked into and each option taken, so that the page the generation builds
/// and what taking an option does are both pinned here.
#[test]
#[allow(clippy::too_many_lines, reason = "one walk per page")]
fn the_pages_a_recording_walked_are_walked_here_too() {
    // Doors of light and dark: two upgrades off a sorted-then-shuffled deck, or
    // one card removed for nothing.
    let mut light = Visit::new("EVENT.DOORS_OF_LIGHT_AND_DARK")
        .deck(&["CARD.STRIKE_IRONCLAD", "CARD.DEFEND_IRONCLAD", "CARD.BASH"])
        .enter();
    choose(
        &mut light,
        "DOORS_OF_LIGHT_AND_DARK.pages.INITIAL.options.LIGHT",
    );
    assert_eq!(
        light
            .state()
            .run_player
            .deck
            .iter()
            .filter(|card| card.upgrade_level > 0)
            .count(),
        2
    );
    let mut dark = Visit::new("EVENT.DOORS_OF_LIGHT_AND_DARK")
        .deck(&["CARD.STRIKE_IRONCLAD", "CARD.DEFEND_IRONCLAD", "CARD.BASH"])
        .enter();
    choose(
        &mut dark,
        "DOORS_OF_LIGHT_AND_DARK.pages.INITIAL.options.DARK",
    );
    let choice = dark
        .state()
        .run
        .as_ref()
        .and_then(|run| run.suspended_choice.as_ref())
        .expect("the removal screen is standing");
    assert_eq!((choice.minimum, choice.maximum), (1, 1));

    // A spirit grafter: twenty-five healed and a curse, or ten paid and a card
    // of the deck removed.
    let mut grafted = Visit::new("EVENT.SPIRIT_GRAFTER").hp(40, 80).enter();
    choose(
        &mut grafted,
        "SPIRIT_GRAFTER.pages.INITIAL.options.LET_IT_IN",
    );
    assert_eq!(grafted.state().run_player.current_hp, 65);
    let mut rejected = Visit::new("EVENT.SPIRIT_GRAFTER")
        .hp(40, 80)
        .deck(&["CARD.STRIKE_IRONCLAD", "CARD.DEFEND_IRONCLAD", "CARD.BASH"])
        .enter();
    choose(
        &mut rejected,
        "SPIRIT_GRAFTER.pages.INITIAL.options.REJECTION",
    );
    assert!(
        rejected
            .state()
            .run
            .as_ref()
            .is_some_and(|run| run.suspended_choice.is_some())
    );

    // A tea master: three teas, the first two priced and the third free.
    let poor = Visit::new("EVENT.TEA_MASTER").gold(150).enter();
    assert_eq!(
        option_ids(&poor),
        vec![
            "TEA_MASTER.pages.INITIAL.options.BONE_TEA",
            "TEA_MASTER.pages.INITIAL.options.EMBER_TEA",
            "TEA_MASTER.pages.INITIAL.options.TEA_OF_DISCOURTESY",
        ]
    );
    let mut broke = Visit::new("EVENT.TEA_MASTER").gold(49).enter();
    assert_eq!(
        option_ids(&broke)[0],
        "TEA_MASTER.pages.INITIAL.options.BONE_TEA_LOCKED"
    );
    choose(
        &mut broke,
        "TEA_MASTER.pages.INITIAL.options.TEA_OF_DISCOURTESY",
    );
    assert!(holds_relic(&broke, "RELIC.TEA_OF_DISCOURTESY"));

    // The legends were true: the quest card, or eight and a potion.
    let mut nabbed = Visit::new("EVENT.THE_LEGENDS_WERE_TRUE").enter();
    choose(
        &mut nabbed,
        "THE_LEGENDS_WERE_TRUE.pages.INITIAL.options.NAB_THE_MAP",
    );
    assert!(deck_holds(&nabbed, "CARD.SPOILS_MAP"));
    let mut slow = Visit::new("EVENT.THE_LEGENDS_WERE_TRUE").hp(80, 80).enter();
    choose(
        &mut slow,
        "THE_LEGENDS_WERE_TRUE.pages.INITIAL.options.SLOWLY_FIND_AN_EXIT",
    );
    assert_eq!(slow.state().run_player.current_hp, 72);
    assert!(
        slow.state()
            .run
            .as_ref()
            .is_some_and(|run| run.reward_set.is_some())
    );

    // A trash heap: eight for one of five relics, or a hundred gold and one of
    // ten cards.
    let mut dived = Visit::new("EVENT.TRASH_HEAP").hp(80, 80).enter();
    choose(&mut dived, "TRASH_HEAP.pages.INITIAL.options.DIVE_IN");
    assert_eq!(dived.state().run_player.current_hp, 72);
    assert_eq!(dived.state().run_player.relics.len(), 2);
    let mut grabbed = Visit::new("EVENT.TRASH_HEAP").enter();
    choose(&mut grabbed, "TRASH_HEAP.pages.INITIAL.options.GRAB");
    assert_eq!(grabbed.state().run_player.gold, 100);
    assert_eq!(grabbed.state().run_player.deck.len(), 3);

    // A whispering hollow: gold rolled around thirty-five for two potions, or
    // a hug.
    let mut paid = Visit::new("EVENT.WHISPERING_HOLLOW").gold(60).enter();
    choose(&mut paid, "WHISPERING_HOLLOW.pages.INITIAL.options.GOLD");
    assert!(paid.state().run_player.gold < 60);
    let mut hugged = Visit::new("EVENT.WHISPERING_HOLLOW").gold(60).enter();
    choose(&mut hugged, "WHISPERING_HOLLOW.pages.INITIAL.options.HUG");
    // The transform screen goes up first, and the nine it costs waits behind
    // it: nothing else of the option runs until a card is picked.
    let choice = hugged
        .state()
        .run
        .as_ref()
        .and_then(|run| run.suspended_choice.as_ref())
        .expect("the transform screen is standing");
    assert_eq!((choice.minimum, choice.maximum), (1, 1));
    assert_eq!(hugged.state().run_player.current_hp, 80);
}

/// An option that put a reward set up has not finished yet: the gold option
/// offers two potions and waits on them before it finishes the page, so the
/// page stands for as long as the offers do and the room ends with the last of
/// them taken.
///
/// A page closed as soon as the option's effects returned is a state the
/// build is never in: the two options are still projected while the potions
/// are on the table.
#[test]
fn a_hollow_keeps_its_page_up_behind_the_potions_it_offered() {
    let mut paid = Visit::new("EVENT.WHISPERING_HOLLOW").gold(60).enter();
    choose(&mut paid, "WHISPERING_HOLLOW.pages.INITIAL.options.GOLD");

    assert_eq!(
        option_ids(&paid),
        vec![
            "WHISPERING_HOLLOW.pages.INITIAL.options.GOLD".to_owned(),
            "WHISPERING_HOLLOW.pages.INITIAL.options.HUG".to_owned(),
        ],
        "the page is still standing while the potions are offered"
    );

    let offered = paid
        .state()
        .run
        .as_ref()
        .and_then(|run| run.reward_set.as_ref())
        .expect("two potions are on the table")
        .rewards
        .len();
    assert_eq!(offered, 2);

    for _ in 0..offered {
        let claim = paid
            .legal_actions()
            .iter()
            .find(|action| matches!(action, Action::ClaimReward { .. }))
            .cloned()
            .expect("a potion is claimable");
        paid.step(claim).expect("the claim applies");
    }

    assert!(
        paid.state()
            .run
            .as_ref()
            .is_some_and(|run| run.active_room.is_none()),
        "and the room ends with the last potion taken"
    );
}

/// The gorge lays its eight commons out on the rewards grid rather than on the
/// plain one, and the screen is named for what it is: a script written from
/// it names the screen the game shows.
#[test]
fn a_gorge_lays_its_commons_out_on_a_rewards_grid() {
    let mut gorged = Visit::new("EVENT.ROOM_FULL_OF_CHEESE").enter();
    choose(
        &mut gorged,
        "ROOM_FULL_OF_CHEESE.pages.INITIAL.options.GORGE",
    );
    let choice = gorged
        .state()
        .run
        .as_ref()
        .and_then(|run| run.suspended_choice.as_ref())
        .expect("the grid is standing");
    assert_eq!(choice.screen, sts2_engine::ChoiceScreen::RewardGrid);
    assert_eq!(choice.candidates.len(), 8);
    assert_eq!((choice.minimum, choice.maximum), (2, 2));
}

/// A card taken off that grid joins the deck the way any other card does, so
/// what answers a card joining the deck answers for it: a book of five rings
/// counts both cheeses.
#[test]
fn what_a_gorge_takes_joins_the_deck_where_the_relics_can_see_it() {
    let mut gorged = Visit::new("EVENT.ROOM_FULL_OF_CHEESE")
        .relics(&["RELIC.BOOK_OF_FIVE_RINGS"])
        .enter();
    choose(
        &mut gorged,
        "ROOM_FULL_OF_CHEESE.pages.INITIAL.options.GORGE",
    );
    let (choice_id, candidates) = gorged
        .legal_actions()
        .iter()
        .find_map(|action| match action {
            Action::ChooseCards { choice_id, cards } => Some((*choice_id, cards.clone())),
            _ => None,
        })
        .expect("the grid asks for two");
    let before = gorged.state().run_player.deck.len();

    gorged
        .step(Action::ChooseCards {
            choice_id,
            cards: candidates[..2].to_vec(),
        })
        .expect("the pick applies");

    assert_eq!(
        gorged.state().run_player.deck.len(),
        before + 2,
        "both cheeses are in the deck"
    );
    let book = gorged
        .state()
        .run_player
        .relics
        .iter()
        .find(|relic| relic.model_id == id("RELIC.BOOK_OF_FIVE_RINGS"))
        .expect("the book is held");
    assert_eq!(book.counter, 2, "the book counted both of them");
}

/// What the bridge's page says: the card it is offering to take, and what one
/// more hold costs./// What the bridge's page says: the card it is offering to take, and what one
/// more hold costs.
fn bridge_page(simulator: &Simulator) -> (String, i32) {
    let options = page(simulator);
    let mut card = None;
    let mut cost = None;
    for option in &options {
        match option.effects.first() {
            Some(EventEffect::RemoveTheRolledCard { card: rolled }) => {
                card = rolled.as_ref().map(|rolled| rolled.model_id.to_string());
            }
            Some(EventEffect::HoldOnToTheBridge { cost: advertised }) => {
                cost = Some(*advertised);
            }
            _ => {}
        }
    }
    (
        card.expect("the page names the card it rolled"),
        cost.expect("the page advertises what a hold costs"),
    )
}

/// A slippery bridge rolls a card of the deck as the page is built, and holding
/// on costs three and one more for every hold before it. The page names both:
/// the card the option would take, and the damage the next hold does.
#[test]
fn a_slippery_bridge_rolls_a_card_and_charges_more_for_every_hold() {
    let mut simulator = Visit::new("EVENT.SLIPPERY_BRIDGE")
        .hp(80, 80)
        .deck(&["CARD.BASH", "CARD.ANGER", "CARD.CLASH", "CARD.THRASH"])
        .enter();
    assert_eq!(
        option_ids(&simulator),
        vec![
            "SLIPPERY_BRIDGE.pages.INITIAL.options.OVERCOME",
            "SLIPPERY_BRIDGE.pages.INITIAL.options.HOLD_ON_0",
        ]
    );
    let (first_card, cost) = bridge_page(&simulator);
    assert_eq!(cost, 3);
    assert!(deck_holds(&simulator, &first_card));
    choose(
        &mut simulator,
        "SLIPPERY_BRIDGE.pages.INITIAL.options.HOLD_ON_0",
    );
    assert_eq!(simulator.state().run_player.current_hp, 77);
    assert_eq!(
        option_ids(&simulator)[1],
        "SLIPPERY_BRIDGE.pages.HOLD_ON_0.options.HOLD_ON_1"
    );
    // The re-roll passes over what it just offered, and the page charges one
    // more for the hold behind it.
    let (second_card, cost) = bridge_page(&simulator);
    assert_eq!(cost, 4);
    assert_ne!(second_card, first_card);
    choose(
        &mut simulator,
        "SLIPPERY_BRIDGE.pages.HOLD_ON_0.options.HOLD_ON_1",
    );
    assert_eq!(simulator.state().run_player.current_hp, 73);
    let (taken, cost) = bridge_page(&simulator);
    assert_eq!(cost, 5);
    choose(
        &mut simulator,
        "SLIPPERY_BRIDGE.pages.INITIAL.options.OVERCOME",
    );
    assert_eq!(simulator.state().run_player.deck.len(), 3);
    // The card that left is the one the option named.
    assert!(!deck_holds(&simulator, &taken));
}

/// The Stone of All Time's push: six unblockable HP, and a Vigorous worth eight
/// worked onto an attack of the deck. With one attack in the deck the screen
/// takes it without asking.
#[test]
fn the_stone_of_all_time_pushes_a_vigorous_onto_an_attack() {
    let mut simulator = Visit::new("EVENT.STONE_OF_ALL_TIME")
        .deck(&["CARD.STRIKE_IRONCLAD", "CARD.DEFEND_IRONCLAD"])
        .hp(80, 80)
        .enter();
    choose(
        &mut simulator,
        "STONE_OF_ALL_TIME.pages.INITIAL.options.PUSH",
    );
    assert_eq!(simulator.state().run_player.current_hp, 74);
    let enchanted: Vec<(String, i32)> = simulator
        .state()
        .run_player
        .deck
        .iter()
        .filter_map(|card| {
            card.enchantment
                .as_ref()
                .map(|enchantment| (card.model_id.to_string(), enchantment.amount))
        })
        .collect();
    assert_eq!(
        enchanted,
        vec![("CARD.STRIKE_IRONCLAD".to_owned(), 8)],
        "the only attack of the deck took the enchantment, worth eight"
    );
}

/// The fake merchant's shelf is bought off the same way a real one's is — the
/// event owns it rather than a merchant's room, so nothing prices it through
/// the merchant hook and nothing restocks it — and walking on is what leaves.
#[test]
fn the_fake_merchants_shelf_is_bought_off_and_never_restocks() {
    let mut simulator = Visit::new("EVENT.FAKE_MERCHANT").act(1).gold(400).enter();
    assert!(
        option_ids(&simulator).is_empty(),
        "the page carries no options at all"
    );
    let shelf = |simulator: &Simulator| match simulator
        .state()
        .run
        .as_ref()
        .and_then(|run| run.active_room.as_ref())
    {
        Some(ActiveRoom::Event { shelf, .. }) => shelf.clone(),
        other => panic!("expected the event's own shelf, found {other:?}"),
    };
    assert_eq!(
        shelf(&simulator).len(),
        6,
        "six of the nine fakes are stocked"
    );
    let buy = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::BuyShopItem { .. }))
        .cloned()
        .expect("the shelf is affordable");
    let Action::BuyShopItem { fingerprint, .. } = buy.clone() else {
        unreachable!()
    };
    let sts2_engine::ShopItem::Relic(relic) = fingerprint else {
        panic!("the shelf holds relics")
    };
    let gold_before = simulator.state().run_player.gold;
    let price = shelf(&simulator)
        .iter()
        .find(|offer| offer.item == sts2_engine::ShopItem::Relic(relic.clone()))
        .expect("the offer is on the shelf")
        .price;
    simulator.step(buy).expect("the purchase applies");
    assert!(holds_relic(&simulator, &relic.to_string()));
    assert_eq!(simulator.state().run_player.gold, gold_before - price);
    assert_eq!(
        shelf(&simulator).iter().filter(|offer| offer.sold).count(),
        1,
        "the slot is cleared rather than refilled"
    );
    // The rest of the shelf is still there to be bought off.
    assert_eq!(
        simulator
            .legal_actions()
            .iter()
            .filter(|action| matches!(action, Action::BuyShopItem { .. }))
            .count(),
        5
    );
}

/// A foul potion thrown at the fake merchant turns him on the player: the fight
/// he becomes pays the rug and everything still stocked on the shelf he was
/// holding.
#[test]
fn a_foul_potion_thrown_at_the_fake_merchant_starts_the_fight() {
    let mut simulator = Visit::new("EVENT.FAKE_MERCHANT")
        .act(1)
        .gold(400)
        .potion("POTION.FOUL_POTION")
        .enter();
    let throw = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::UsePotion { .. }))
        .cloned()
        .expect("the potion may be thrown at the merchant standing there");
    simulator.step(throw).expect("the throw applies");
    assert!(
        simulator.state().combat.is_some(),
        "the merchant turned on the player"
    );
    assert!(
        simulator.state().run_player.potions.iter().all(|slot| slot
            .as_ref()
            .is_none_or(|held| *held != id("POTION.FOUL_POTION"))),
        "the potion was spent"
    );
}

/// The same potion is not usable at all where there is no merchant to throw it
/// at, which is its own usability check.
#[test]
fn a_foul_potion_cannot_be_thrown_where_no_merchant_stands() {
    let simulator = Visit::new("EVENT.HUNGRY_FOR_MUSHROOMS")
        .potion("POTION.FOUL_POTION")
        .enter();
    assert!(
        !simulator
            .legal_actions()
            .iter()
            .any(|action| matches!(action, Action::UsePotion { .. }))
    );
}

/// An aroma of chaos offers the transform and the refusal, and nothing else.
#[test]
fn an_aroma_of_chaos_offers_letting_go_or_holding_on() {
    let simulator = Visit::new("EVENT.AROMA_OF_CHAOS").enter();
    assert_eq!(
        option_ids(&simulator),
        vec![
            "AROMA_OF_CHAOS.pages.INITIAL.options.LET_GO",
            "AROMA_OF_CHAOS.pages.INITIAL.options.MAINTAIN_CONTROL",
        ]
    );
}

/// A sapphire seed is either eaten or planted.
#[test]
fn a_sapphire_seed_is_eaten_or_planted() {
    let simulator = Visit::new("EVENT.SAPPHIRE_SEED").enter();
    assert_eq!(
        option_ids(&simulator),
        vec![
            "SAPPHIRE_SEED.pages.INITIAL.options.EAT",
            "SAPPHIRE_SEED.pages.INITIAL.options.PLANT",
        ]
    );
}

/// A table of philosophers lays out one option per character the run is not,
/// named by that character's colour, and the page then comes down to three of
/// them. So an ironclad is offered three of the other four colours, always in
/// the build's own colour order and never its own.
#[test]
fn the_philosophers_offer_three_colours_that_are_not_the_players_own() {
    const ORDER: [&str; 4] = ["NECROBINDER", "REGENT", "SILENT", "DEFECT"];
    let simulator = Visit::new("EVENT.COLORFUL_PHILOSOPHERS").enter();
    let offered = option_ids(&simulator);
    assert_eq!(offered.len(), 3, "the page comes down to three");
    let colours: Vec<&str> = offered
        .iter()
        .map(|id| {
            id.strip_prefix("COLORFUL_PHILOSOPHERS.pages.INITIAL.options.")
                .expect("an option of this page")
        })
        .collect();
    assert!(
        !colours.contains(&"IRONCLAD"),
        "the walking character's own colour is never one of them"
    );
    let mut walked = ORDER.iter();
    for colour in &colours {
        assert!(
            walked.any(|known| known == colour),
            "{colour} stands in the build's colour order"
        );
    }
}

/// Tezcatara lays out one relic per pool, in pool order, and the soup of the
/// first pool is only on the table for a deck that still holds a strike.
#[test]
fn tezcatara_offers_one_relic_out_of_each_of_its_three_pools() {
    const POOLS: [&[&str]; 3] = [
        &["VERY_HOT_COCOA", "YUMMY_COOKIE", "NUTRITIOUS_SOUP"],
        &["BIIIG_HUG", "STORYBOOK", "TOASTY_MITTENS"],
        &[
            "GOLDEN_COMPASS",
            "PUMPKIN_CANDLE",
            "TOY_BOX",
            "SEAL_OF_GOLD",
        ],
    ];
    let offered = |deck: &[&'static str]| -> Vec<String> {
        let simulator = Visit::new("EVENT.TEZCATARA").deck(deck).enter();
        option_ids(&simulator)
            .into_iter()
            .map(|id| {
                id.strip_prefix("TEZCATARA.pages.INITIAL.options.")
                    .expect("an option of this page")
                    .to_owned()
            })
            .collect()
    };

    let with_a_strike = offered(&["CARD.STRIKE_IRONCLAD"]);
    assert_eq!(with_a_strike.len(), 3, "one relic out of each pool");
    for (relic, pool) in with_a_strike.iter().zip(POOLS) {
        assert!(
            pool.contains(&relic.as_str()),
            "{relic} came out of the pool standing in its place"
        );
    }

    // The soup is on the table only while the deck holds a strike to trade,
    // so a deck without one is offered the rest of that pool instead.
    let without = offered(&["CARD.DEFEND_IRONCLAD"]);
    assert_ne!(
        without[0], "NUTRITIOUS_SOUP",
        "no strike in the deck leaves the soup off the table"
    );
    assert!(POOLS[0].contains(&without[0].as_str()));
}

/// A trial's guilty verdict against the nondescript entrant lays out two card
/// rewards, drawn at a regular encounter's rarity odds and never rolled for an
/// upgrade: each card costs the rewards stream a rarity and a pick, nothing
/// more, and every card comes as written whatever act the trial sits in.
#[test]
fn a_trials_guilty_verdict_lays_out_cards_that_are_never_rolled_for_an_upgrade() {
    // Who stands trial is rolled off the run seed, so the seed is chosen for
    // the nondescript entrant.
    let guilty = "TRIAL.pages.NONDESCRIPT.options.GUILTY";
    let mut simulator = [
        "N3Z9EE2CX1",
        "N3Z9EE2CX2",
        "N3Z9EE2CX3",
        "N3Z9EE2CX4",
        "N3Z9EE2CX5",
        "N3Z9EE2CX6",
    ]
    .into_iter()
    .map(|seed| {
        let mut simulator = Visit::new("EVENT.TRIAL").act(2).seed(seed).enter();
        choose(&mut simulator, "TRIAL.pages.INITIAL.options.ACCEPT");
        simulator
    })
    .find(|simulator| option_ids(simulator).iter().any(|id| id == guilty))
    .expect("one of the seeds puts the nondescript entrant on trial");
    let rewards_reads = |simulator: &Simulator| {
        simulator
            .state()
            .player_rng
            .stream(sts2_rng::PlayerStream::Rewards)
            .counter()
    };
    let before = rewards_reads(&simulator);
    choose(&mut simulator, guilty);
    let after = rewards_reads(&simulator);
    let rewards = simulator
        .state()
        .run
        .as_ref()
        .and_then(|run| run.reward_set.as_ref())
        .expect("the card rewards are on the table");
    assert_eq!(rewards.rewards.len(), 2);
    for item in &rewards.rewards {
        assert_eq!(item.fingerprint.offered_cards.len(), 3);
        assert!(
            item.fingerprint
                .offered_cards
                .iter()
                .all(|card| card.upgrade_level == 0),
            "no card is rolled for an upgrade: {:?}",
            item.fingerprint.offered_cards
        );
    }
    assert_eq!(
        after - before,
        12,
        "a rarity and a pick for each of six cards, and nothing more"
    );
}

/// An event's HP cost is dealt as unblockable, unpowered damage, so what
/// answers HP being lost answers for it: a tungsten rod takes one off the
/// brain leech's rip, and without the rod the rip costs what it says.
#[test]
fn a_brain_leechs_rip_is_one_cheaper_with_a_tungsten_rod() {
    let rip = "BRAIN_LEECH.pages.INITIAL.options.RIP";
    let mut bare = Visit::new("EVENT.BRAIN_LEECH").hp(61, 80).enter();
    choose(&mut bare, rip);
    assert_eq!(bare.state().run_player.current_hp, 61 - 5);

    let mut rodded = Visit::new("EVENT.BRAIN_LEECH")
        .hp(61, 80)
        .relics(&["RELIC.TUNGSTEN_ROD"])
        .enter();
    choose(&mut rodded, rip);
    assert_eq!(rodded.state().run_player.current_hp, 61 - 4);
}

#[test]
fn reloading_a_finished_ordinary_event_rewinds_to_its_entrance() {
    use serde_json::json;
    use sts2_replay::{Record, ReplayAdapter, observation::render};

    let entered = Visit::new("EVENT.BRAIN_LEECH").hp(61, 80).enter();
    let entrance = render(&entered);
    let mut adapter = ReplayAdapter::new(entered);
    for attempt in 0..2 {
        let sequence = attempt * 3 + 1;
        adapter
            .accept(&Record {
                sequence,
                kind: "event.choose".into(),
                payload: json!({"data": {"option_index": 1,
                "option_id": "BRAIN_LEECH.pages.INITIAL.options.RIP"}}),
            })
            .unwrap();
        adapter
            .accept(&Record {
                sequence: sequence + 1,
                kind: "room.proceed".into(),
                payload: json!({"data": {}}),
            })
            .unwrap();
        assert_eq!(adapter.simulator().state().run_player.current_hp, 56);
        assert_eq!(
            render(adapter.simulator())["choices"]["events"]
                .as_array()
                .unwrap()
                .as_slice(),
            [] as [serde_json::Value; 0]
        );
        adapter
            .accept(&Record {
                sequence: sequence + 2,
                kind: "run.resume".into(),
                payload: json!({"data": {}}),
            })
            .unwrap();
        let restored = render(adapter.simulator());
        assert_eq!(
            restored["players"], entrance["players"],
            "HP and reward rolls rewind"
        );
        assert_eq!(
            restored["choices"], entrance["choices"],
            "the initial page returns"
        );
    }
}

/// A card reward an event lays out runs the same reward modify passes a
/// fight's does, so a driftwood lets the brain leech's card offer be
/// rerolled: the offer's screen carries a reroll beside its skip.
#[test]
fn a_driftwood_lets_an_events_card_offer_be_rerolled() {
    let alternatives = |relics: &[&'static str]| {
        let mut simulator = Visit::new("EVENT.BRAIN_LEECH")
            .hp(61, 80)
            .relics(relics)
            .enter();
        choose(&mut simulator, "BRAIN_LEECH.pages.INITIAL.options.RIP");
        let claim = simulator
            .legal_actions()
            .iter()
            .find(|action| {
                matches!(action, Action::ClaimReward { fingerprint, .. }
                    if fingerprint.reward_type == "card")
            })
            .cloned()
            .expect("a card offer is on the table");
        simulator.step(claim).expect("the offer opens");
        simulator
            .state()
            .run
            .as_ref()
            .and_then(|run| run.suspended_choice.as_ref())
            .expect("the offer's screen is standing")
            .alternatives
            .iter()
            .map(|alternative| alternative.option_id.clone())
            .collect::<Vec<_>>()
    };
    // The skip is the screen's own and is not among the alternatives a
    // relic puts there.
    assert_eq!(alternatives(&[]), Vec::<String>::new());
    assert_eq!(
        alternatives(&["RELIC.DRIFTWOOD"]),
        vec!["reroll".to_owned()]
    );
}

/// The Future of Potions keeps its rarity/type filter and its post-generation
/// upgrade callback when Driftwood rerolls the offer, including after a state
/// round trip. No random upgrade roll belongs to either generation.
#[test]
fn a_potion_trade_rerolls_filtered_upgraded_cards_once() {
    use sts2_engine::DecisionContext;

    for potion in [
        "POTION.BLOCK_POTION",
        "POTION.LIQUID_BRONZE",
        "POTION.FAIRY_IN_A_BOTTLE",
    ] {
        for driftwood in [false, true] {
            let relics = if driftwood {
                vec!["RELIC.DRIFTWOOD", "RELIC.PRISMATIC_SHARD"]
            } else {
                Vec::new()
            };
            let mut visit = Visit::new("EVENT.THE_FUTURE_OF_POTIONS")
                .potion(potion)
                .relics(&relics);
            visit.potions = 2;
            let mut simulator = visit.enter();
            let option = page(&simulator).remove(0);
            let EventEffect::TradePotionForUpgradedCards {
                rarity,
                card_type,
                pool,
                ..
            } = &option.effects[0]
            else {
                panic!("the first potion trades for cards");
            };
            let allowed: Vec<_> = pool
                .iter()
                .filter(|card| card.rarity == *rarity && card.card_type == *card_type)
                .map(|card| card.model_id.clone())
                .collect();
            let before = simulator
                .state()
                .player_rng
                .stream(sts2_rng::PlayerStream::Rewards)
                .counter();
            choose(&mut simulator, &option.option_id);
            let claim = simulator
                .legal_actions()
                .iter()
                .find(|action| {
                    matches!(action, Action::ClaimReward { fingerprint, .. }
                    if fingerprint.reward_type == "card")
                })
                .cloned()
                .expect("the traded potion paid a card offer");
            simulator.step(claim).unwrap();
            for generation in 1..=if driftwood { 2 } else { 1 } {
                let DecisionContext::ChooseCards { candidates, .. } = simulator.decision() else {
                    panic!("the reward holds a card selection");
                };
                assert_eq!(candidates.len(), 3);
                for card in candidates {
                    assert!(allowed.contains(&card.fingerprint.model_id));
                    assert_eq!(card.fingerprint.upgrade_level, 1);
                }
                let after = simulator
                    .state()
                    .player_rng
                    .stream(sts2_rng::PlayerStream::Rewards)
                    .counter();
                assert_eq!(after - before, 3 * generation, "only three card picks");
                let reroll = simulator
                    .legal_actions()
                    .iter()
                    .find(|action| {
                        matches!(action, Action::ChooseAlternative { option_id, .. }
                        if option_id == "reroll")
                    })
                    .cloned();
                if driftwood && generation == 1 {
                    let portable = serde_json::from_str(
                        &serde_json::to_string(
                            &simulator
                                .project(sts2_engine::ProjectionScope::RunCore)
                                .unwrap(),
                        )
                        .unwrap(),
                    )
                    .unwrap();
                    simulator =
                        Simulator::from_portable_state(portable, sts2_content::standard_registry())
                            .unwrap();
                    simulator
                        .step(reroll.expect("Driftwood enables the reroll"))
                        .expect("the filtered, upgraded offer can be rerolled");
                } else {
                    assert!(reroll.is_none(), "no reroll without Driftwood or after use");
                }
            }
        }
    }
}

/// The board the crystal sphere stood up.
fn sphere(simulator: &Simulator) -> sts2_engine::CrystalSphereGame {
    match simulator
        .state()
        .run
        .as_ref()
        .and_then(|run| run.active_room.as_ref())
    {
        Some(ActiveRoom::Event {
            sphere: Some(sphere),
            ..
        }) => (**sphere).clone(),
        other => panic!("expected a crystal sphere board, found {other:?}"),
    }
}

/// Spends every divination with the small tool: first on the cells named, then
/// on fog with nothing under it.
fn divine(simulator: &mut Simulator, first: &[(i32, i32)]) {
    for (x, y) in first {
        simulator
            .step(Action::UncoverCrystalSphere {
                x: *x,
                y: *y,
                tool: sts2_engine::DivinationTool::Small,
            })
            .expect("a hidden cell takes the click");
    }
    while simulator
        .state()
        .run
        .as_ref()
        .is_some_and(|run| run.reward_set.is_none())
    {
        let board = sphere(simulator);
        let (x, y) = (0..sts2_engine::CrystalSphereGame::WIDTH)
            .flat_map(|x| (0..sts2_engine::CrystalSphereGame::HEIGHT).map(move |y| (x, y)))
            .find(|(x, y)| {
                board
                    .cell(*x, *y)
                    .is_some_and(|cell| cell.hidden && cell.item.is_none())
            })
            .expect("the board has empty fog left");
        simulator
            .step(Action::UncoverCrystalSphere {
                x,
                y,
                tool: sts2_engine::DivinationTool::Small,
            })
            .expect("a hidden cell takes the click");
    }
}

/// A gold prize on the crystal sphere's board rolls its amount as the rewards
/// screen is filled, between bounds that are one number, and the roll costs the
/// board's stream a read — so whatever the board rolls after it is one further
/// along. Seed `PCFYFYGRAS`, record 963, where a card prize revealed behind a
/// gold one offered the cards one read on.
#[test]
fn a_crystal_spheres_gold_costs_the_boards_stream_a_read() {
    let small_gold = |board: &sts2_engine::CrystalSphereGame| {
        let item = board
            .items
            .iter()
            .position(|item| item.kind == sts2_engine::CrystalSphereItemKind::Gold { big: false })
            .expect("every board holds small gold");
        (0..sts2_engine::CrystalSphereGame::WIDTH)
            .flat_map(|x| (0..sts2_engine::CrystalSphereGame::HEIGHT).map(move |y| (x, y)))
            .find(|(x, y)| {
                board
                    .cell(*x, *y)
                    .is_some_and(|cell| cell.item == Some(item))
            })
            .expect("the gold stands on a cell")
    };

    let mut with_gold = Visit::new("EVENT.CRYSTAL_SPHERE").enter();
    choose(
        &mut with_gold,
        "CRYSTAL_SPHERE.pages.INITIAL.options.PAYMENT_PLAN",
    );
    let before = event_die(&with_gold);
    let gold = small_gold(&sphere(&with_gold));
    divine(&mut with_gold, &[gold]);
    let offered = with_gold
        .state()
        .run
        .as_ref()
        .and_then(|run| run.reward_set.as_ref())
        .expect("the gold is offered");
    assert_eq!(offered.rewards.len(), 1);
    assert_eq!(offered.rewards[0].fingerprint.reward_type, "gold");
    assert_eq!(offered.rewards[0].fingerprint.gold_amount, 10);
    assert_eq!(event_die(&with_gold) - before, 1, "one read for the gold");
}
