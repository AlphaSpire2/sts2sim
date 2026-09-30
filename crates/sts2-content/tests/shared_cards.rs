//! The cards that belong to no character, pinned one behaviour at a time.
//!
//! No recording in the corpus takes any of these, so a scenario is the evidence
//! for each: what a status does to whoever is still holding it, what a curse
//! refuses, what a token is worth, and what a colorless card reads off the
//! state it lands in.

use sts2_engine::{Action, CardFingerprint, PileName, ScenarioBuilder, Simulator};
use sts2_rng::RunStream;

const SEED: &str = "SHAREDCARDS";

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str, upgrade_level: u8) -> CardFingerprint {
    let mut fingerprint = CardFingerprint::base(id(model_id));
    fingerprint.upgrade_level = upgrade_level;
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

/// One fight against a single beetle, with the hand the test names and enough
/// energy that a cost never gets in the way of what is being measured.
fn fight(hand: &[(&str, u8)]) -> Simulator {
    fight_with_draw(hand, &[])
}

fn fight_with_draw(hand: &[(&str, u8)], draw: &[(&str, u8)]) -> Simulator {
    scenario(hand, draw, 39)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

fn scenario(hand: &[(&str, u8)], draw: &[(&str, u8)], enemy_hp: i32) -> ScenarioBuilder {
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .enemy(
            1,
            id("MONSTER.SHRINKER_BEETLE"),
            enemy_hp,
            enemy_hp,
            "SHRINKER_MOVE",
        );
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
}

fn play(simulator: &mut Simulator, model_id: &str) {
    let action = play_action(simulator, model_id)
        .unwrap_or_else(|| panic!("{model_id} was not a legal play"));
    simulator.step(action).expect("the play applies");
}

fn play_action(simulator: &Simulator, model_id: &str) -> Option<Action> {
    let wanted = id(model_id);
    simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::PlayCard { card, .. } if card.fingerprint.model_id == wanted)
        })
        .cloned()
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

fn pile(simulator: &Simulator, pile: PileName) -> Vec<String> {
    let piles = &simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles;
    let cards = match pile {
        PileName::Draw => &piles.draw,
        PileName::Hand => &piles.hand,
        PileName::Discard => &piles.discard,
        PileName::Exhaust => &piles.exhaust,
        PileName::Play => &piles.play,
    };
    cards
        .iter()
        .map(|card| card.fingerprint.model_id.to_string())
        .collect()
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

// --- statuses ------------------------------------------------------------

/// The three statuses that burn their holder at the end of the turn do it for
/// what their own damage var says, and block stops every point of it.
#[test]
fn a_burn_infection_and_wither_are_all_stopped_by_block() {
    for (model_id, damage) in [
        ("CARD.BURN", 2),
        ("CARD.INFECTION", 3),
        ("CARD.WITHER", 3),
        ("CARD.DECAY", 2),
    ] {
        let mut simulator = fight(&[(model_id, 0)]);
        end_turn(&mut simulator);
        assert_eq!(
            player(&simulator).current_hp,
            80 - damage,
            "{model_id} burns its holder for what its own damage var says"
        );

        // The same card behind block costs nothing at all.
        let mut simulator = fight(&[(model_id, 0), ("CARD.DEFEND_IRONCLAD", 0)]);
        play(&mut simulator, "CARD.DEFEND_IRONCLAD");
        let before = player(&simulator).current_hp;
        end_turn(&mut simulator);
        assert_eq!(player(&simulator).current_hp, before);
    }
}

/// A beckon and bad luck cost HP that block does not stop, and a beckon is a
/// playable card that does nothing while bad luck cannot be played at all.
#[test]
fn a_beckon_and_bad_luck_get_through_block() {
    for (model_id, damage) in [("CARD.BECKON", 6), ("CARD.BAD_LUCK", 13)] {
        let mut simulator = fight(&[(model_id, 0), ("CARD.DEFEND_IRONCLAD", 0)]);
        play(&mut simulator, "CARD.DEFEND_IRONCLAD");
        end_turn(&mut simulator);
        assert_eq!(player(&simulator).current_hp, 80 - damage);
    }

    // A beckon is playable — it has no Unplayable keyword — and does nothing.
    let mut simulator = fight(&[("CARD.BECKON", 0)]);
    play(&mut simulator, "CARD.BECKON");
    assert_eq!(player(&simulator).current_hp, 80);
    assert_eq!(pile(&simulator, PileName::Discard), ["CARD.BECKON"]);

    // Bad luck is not.
    let simulator = fight(&[("CARD.BAD_LUCK", 0)]);
    assert!(play_action(&simulator, "CARD.BAD_LUCK").is_none());
}

/// A void takes a point of energy the moment it is drawn, and burns up if the
/// turn ends with it still in hand.
#[test]
fn a_void_costs_energy_when_it_is_drawn_and_burns_up_when_held() {
    // A shrug it off draws the top card of the draw pile as part of its play.
    let mut simulator = fight_with_draw(&[("CARD.SHRUG_IT_OFF", 0)], &[("CARD.VOID", 0)]);
    let before = energy(&simulator);
    play(&mut simulator, "CARD.SHRUG_IT_OFF");
    assert_eq!(pile(&simulator, PileName::Hand), ["CARD.VOID"]);
    // One point for the shrug it off, and one more for the void it drew.
    assert_eq!(energy(&simulator), before - 2);

    end_turn(&mut simulator);
    assert!(pile(&simulator, PileName::Exhaust).contains(&"CARD.VOID".to_owned()));
}

/// Debris and a spore mind are playable cards that do nothing and burn up for
/// having been played; soot, an injury, and the curse of the bell cannot be
/// played at all.
#[test]
fn the_cards_that_only_take_up_room_take_up_the_room_they_take() {
    for model_id in ["CARD.DEBRIS", "CARD.SPORE_MIND"] {
        let mut simulator = fight(&[(model_id, 0)]);
        play(&mut simulator, model_id);
        assert_eq!(pile(&simulator, PileName::Exhaust), [model_id]);
        assert_eq!(player(&simulator).current_hp, 80);
    }
    for model_id in [
        "CARD.SOOT",
        "CARD.INJURY",
        "CARD.CURSE_OF_THE_BELL",
        "CARD.WRITHE",
        "CARD.NORMALITY",
        "CARD.POOR_SLEEP",
    ] {
        let simulator = fight(&[(model_id, 0)]);
        assert!(
            play_action(&simulator, model_id).is_none(),
            "{model_id} cannot be played"
        );
    }
}

// --- curses --------------------------------------------------------------

/// A debt takes ten gold at the end of a turn it is held through, and never
/// more than the purse holds.
#[test]
fn a_debt_takes_ten_gold_or_what_is_left_of_the_purse() {
    for (gold, left) in [(25, 15), (4, 0)] {
        let mut simulator = scenario(&[("CARD.DEBT", 0)], &[], 39)
            .gold(gold)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
        end_turn(&mut simulator);
        assert_eq!(simulator.state().run_player.gold, left);
    }
}

/// Doubt and shame work a debuff on their own holder with nobody behind it, and
/// the stack they stand up lasts through the turn it landed on.
#[test]
fn doubt_and_shame_land_a_debuff_that_survives_the_turn_it_landed_on() {
    for (model_id, power) in [
        ("CARD.DOUBT", "POWER.WEAK_POWER"),
        ("CARD.SHAME", "POWER.FRAIL_POWER"),
    ] {
        let mut simulator = fight(&[(model_id, 0)]);
        end_turn(&mut simulator);
        assert_eq!(
            power_amount(&simulator, 0, power),
            1,
            "{model_id} left a stack standing through the turn it landed on"
        );
    }
}

/// A regret costs its holder one HP for every card the hand held as the turn
/// began to end, counted before the hand acted rather than after.
#[test]
fn a_regret_counts_the_hand_it_was_held_in() {
    let mut simulator = fight(&[
        ("CARD.REGRET", 0),
        ("CARD.DEFEND_IRONCLAD", 0),
        ("CARD.STRIKE_IRONCLAD", 0),
    ]);
    end_turn(&mut simulator);
    // Three cards were in hand when the turn started to end, and block does not
    // stop what a regret costs.
    assert_eq!(player(&simulator).current_hp, 77);
}

/// An enthralled is the only card its holder may choose to play while it is in
/// hand, and it can never be taken out of the deck.
#[test]
fn an_enthralled_is_the_only_card_its_holder_may_play() {
    let mut simulator = fight(&[
        ("CARD.ENTHRALLED", 0),
        ("CARD.STRIKE_IRONCLAD", 0),
        ("CARD.ENTHRALLED", 0),
    ]);
    assert!(play_action(&simulator, "CARD.STRIKE_IRONCLAD").is_none());
    assert!(play_action(&simulator, "CARD.ENTHRALLED").is_some());
    play(&mut simulator, "CARD.ENTHRALLED");
    // One is still held, so the strike is still refused.
    assert!(play_action(&simulator, "CARD.STRIKE_IRONCLAD").is_none());
    play(&mut simulator, "CARD.ENTHRALLED");
    assert!(play_action(&simulator, "CARD.STRIKE_IRONCLAD").is_some());
}

/// Normality allows three card plays in a turn and no more, counted from when
/// each play starts.
#[test]
fn normality_allows_three_card_plays_a_turn() {
    let mut simulator = fight(&[
        ("CARD.NORMALITY", 0),
        ("CARD.DEFEND_IRONCLAD", 0),
        ("CARD.DEFEND_IRONCLAD", 0),
        ("CARD.DEFEND_IRONCLAD", 0),
        ("CARD.DEFEND_IRONCLAD", 0),
    ]);
    for _ in 0..3 {
        play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    }
    assert!(play_action(&simulator, "CARD.DEFEND_IRONCLAD").is_none());
}

/// A poor sleep is kept in hand when the turn ends, and everything beside it
/// goes to the discard pile.
#[test]
fn a_retained_card_is_not_put_down_with_the_rest_of_the_hand() {
    // The draw pile holds the next hand, so nothing is reshuffled and what was
    // put down stays down.
    let mut simulator = fight_with_draw(
        &[("CARD.POOR_SLEEP", 0), ("CARD.STRIKE_IRONCLAD", 0)],
        &[
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
        ],
    );
    end_turn(&mut simulator);
    assert!(pile(&simulator, PileName::Hand).contains(&"CARD.POOR_SLEEP".to_owned()));
    assert_eq!(
        pile(&simulator, PileName::Discard),
        ["CARD.STRIKE_IRONCLAD"]
    );
}

/// An ascender's bane and a folly burn up if the turn ends with them in hand,
/// and neither can be taken out of the deck.
#[test]
fn the_burning_curses_burn_and_cannot_be_removed() {
    let mut simulator = fight(&[("CARD.ASCENDERS_BANE", 0), ("CARD.FOLLY", 0)]);
    end_turn(&mut simulator);
    let exhausted = pile(&simulator, PileName::Exhaust);
    assert!(exhausted.contains(&"CARD.ASCENDERS_BANE".to_owned()));
    assert!(exhausted.contains(&"CARD.FOLLY".to_owned()));
    let registry = sts2_content::standard_registry();
    for model_id in ["CARD.ASCENDERS_BANE", "CARD.FOLLY", "CARD.ENTHRALLED"] {
        assert!(
            registry
                .card(&id(model_id))
                .expect("the curse is registered")
                .eternal,
            "{model_id} can never leave the deck"
        );
    }
}

/// A folly and a writhe are dealt into the opening hand.
#[test]
fn the_innate_curses_open_the_fight_in_hand() {
    for model_id in ["CARD.FOLLY", "CARD.WRITHE"] {
        assert!(
            sts2_content::standard_registry()
                .card(&id(model_id))
                .expect("the curse is registered")
                .innate,
            "{model_id} is dealt before the opening hand"
        );
    }
}

/// A frantic escape gives the sandpit that made it a turn back, and costs one
/// more for the rest of the fight for having been played. Played where no
/// sandpit stands it still costs itself the energy.
#[test]
fn a_frantic_escape_pays_the_sandpit_and_grows_dearer() {
    let mut simulator = fight(&[("CARD.FRANTIC_ESCAPE", 0)]);
    let action = play_action(&simulator, "CARD.FRANTIC_ESCAPE").expect("it is playable");
    simulator.step(action).expect("no sandpit is not an error");
    let card = simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles
        .discard
        .first()
        .expect("the card was put down");
    assert_eq!(card.cost_this_combat, 1);
    assert_eq!(card.energy_cost, 2);
}

/// A guilt counts the fights the run has seen on the deck's own card, and
/// leaves the deck once it has counted five.
#[test]
fn a_guilt_counts_the_fights_a_run_has_seen_and_then_leaves() {
    for (seen, still_held) in [(0, true), (4, false)] {
        let mut guilty = card("CARD.GUILTY", 0);
        if seen > 0 {
            guilty
                .properties
                .insert("CombatsSeen".to_owned(), serde_json::json!(seen));
        }
        let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .player_hp(80, 80)
            .energy(9, 9)
            .turn(1, 1)
            .rng_counter(RunStream::MonsterAi, 0)
            .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 6, 6, "SHRINKER_MOVE")
            .card(PileName::Hand, 0, guilty, -1)
            .card(PileName::Hand, 1, card("CARD.STRIKE_IRONCLAD", 0), 1)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
        play(&mut simulator, "CARD.STRIKE_IRONCLAD");

        let held = simulator
            .state()
            .run_player
            .deck
            .iter()
            .find(|held| held.model_id == id("CARD.GUILTY"));
        match held {
            Some(held) => {
                assert!(still_held);
                assert_eq!(
                    held.properties.get("CombatsSeen"),
                    Some(&serde_json::json!(seen + 1))
                );
            }
            None => assert!(!still_held, "the fifth fight takes it out of the deck"),
        }
    }
}

// --- tokens --------------------------------------------------------------

/// The tokens a body conjures: what each is worth, and that every one of them
/// burns up for having been played.
#[test]
fn the_conjured_tokens_pay_out_once_and_burn() {
    for (model_id, upgrade, energy_gained, cards_drawn) in [
        ("CARD.FUEL", 0, 1, 1),
        ("CARD.FUEL", 1, 1, 2),
        ("CARD.LUMINESCE", 0, 2, 0),
        ("CARD.LUMINESCE", 1, 3, 0),
        ("CARD.SOUL", 0, 0, 2),
        ("CARD.SOUL", 1, 0, 3),
    ] {
        let mut simulator = fight_with_draw(
            &[(model_id, upgrade)],
            &[
                ("CARD.STRIKE_IRONCLAD", 0),
                ("CARD.STRIKE_IRONCLAD", 0),
                ("CARD.STRIKE_IRONCLAD", 0),
            ],
        );
        let before = energy(&simulator);
        play(&mut simulator, model_id);
        assert_eq!(
            energy(&simulator),
            before + energy_gained,
            "{model_id} at +{upgrade}"
        );
        assert_eq!(pile(&simulator, PileName::Hand).len(), cards_drawn);
        assert_eq!(pile(&simulator, PileName::Exhaust), [model_id]);
    }
}

/// A luminesce is never put down at the end of the turn.
#[test]
fn a_luminesce_is_kept_in_hand() {
    let mut simulator = fight_with_draw(
        &[("CARD.LUMINESCE", 0)],
        &[
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
        ],
    );
    end_turn(&mut simulator);
    assert!(pile(&simulator, PileName::Hand).contains(&"CARD.LUMINESCE".to_owned()));
}

/// The three cards a summoned minion leaves behind, and a shiv.
#[test]
fn the_minion_cards_and_a_shiv_land_what_they_declare() {
    for (model_id, upgrade, damage) in [
        ("CARD.MINION_DIVE_BOMB", 0, 13),
        ("CARD.MINION_DIVE_BOMB", 1, 16),
        ("CARD.MINION_STRIKE", 0, 6),
        ("CARD.MINION_STRIKE", 1, 9),
        ("CARD.SHIV", 0, 4),
        ("CARD.SHIV", 1, 6),
    ] {
        let mut simulator = fight_with_draw(&[(model_id, upgrade)], &[("CARD.STRIKE_IRONCLAD", 0)]);
        play(&mut simulator, model_id);
        assert_eq!(
            creature(&simulator, 1).current_hp,
            39 - damage,
            "{model_id} at +{upgrade}"
        );
        assert_eq!(pile(&simulator, PileName::Exhaust), [model_id]);
    }
    for (upgrade, block) in [(0, 8), (1, 11)] {
        let mut simulator = fight(&[("CARD.MINION_SACRIFICE", upgrade)]);
        play(&mut simulator, "CARD.MINION_SACRIFICE");
        assert_eq!(player(&simulator).block, block);
    }
}

/// A minion strike draws a card as well as swinging, and carries both the
/// Strike and the Minion tags.
#[test]
fn a_minion_strike_is_a_strike_and_a_minion() {
    let mut simulator =
        fight_with_draw(&[("CARD.MINION_STRIKE", 0)], &[("CARD.STRIKE_IRONCLAD", 0)]);
    play(&mut simulator, "CARD.MINION_STRIKE");
    assert_eq!(pile(&simulator, PileName::Hand), ["CARD.STRIKE_IRONCLAD"]);
    let registry = sts2_content::standard_registry();
    let tags = &registry
        .card(&id("CARD.MINION_STRIKE"))
        .expect("the token is registered")
        .tags;
    assert!(tags.contains(&sts2_engine::CardTag::Strike));
    assert!(tags.contains(&sts2_engine::CardTag::Minion));
}

/// The one token whose body belongs to a character this baseline cannot play
/// stops where that character begins. The Regent's blade swings for what the
/// forge says, and there is no forge.
#[test]
fn the_regents_blade_swings_for_what_the_forge_has_grown_it_to() {
    let model_id = "CARD.SOVEREIGN_BLADE";
    let mut simulator = fight(&[(model_id, 0)]);
    let enemy_hp = creature(&simulator, 1).current_hp;
    play(&mut simulator, model_id);
    // A blade nobody has forged is worth its own ten, and the block behind it
    // is the parry its owner is standing on, which is none.
    assert_eq!(creature(&simulator, 1).current_hp, enemy_hp - 10);
    assert_eq!(creature(&simulator, 0).block, 0);
}

/// A Sweeping Gaze is the pet's, and with no pet standing it does nothing at
/// all: the guard every Osty attack opens with is the whole of the body.
#[test]
fn a_sweeping_gaze_with_no_pet_does_nothing() {
    let mut simulator = fight(&[("CARD.SWEEPING_GAZE", 0)]);
    let enemy_hp = creature(&simulator, 1).current_hp;
    play(&mut simulator, "CARD.SWEEPING_GAZE");
    assert_eq!(
        creature(&simulator, 1).current_hp,
        enemy_hp,
        "the swing needs a pet to swing it"
    );
}

/// With a pet standing it is a blow the pet lands, rolled over the enemies: the
/// Necrobinder's own Bodyguard stands one up first, and the Gaze swings for ten
/// at whoever the roll picks.
#[test]
fn a_sweeping_gaze_is_swung_by_the_pet() {
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .played_as(id("CHARACTER.NECROBINDER"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatTargets, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .card(PileName::Hand, 0, card("CARD.BODYGUARD", 0), 0)
        .card(PileName::Hand, 1, card("CARD.SWEEPING_GAZE", 0), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.BODYGUARD");
    play(&mut simulator, "CARD.SWEEPING_GAZE");
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 10);
}

/// The deprecated placeholder draws a card and takes the deck card it stood for
/// out of the deck, and does the second half only when it came from the deck.
#[test]
fn the_deprecated_placeholder_removes_the_deck_card_it_stood_for() {
    let mut simulator = fight_with_draw(
        &[("CARD.DEPRECATED_CARD", 0)],
        &[("CARD.STRIKE_IRONCLAD", 0)],
    );
    assert!(
        simulator
            .state()
            .run_player
            .deck
            .iter()
            .any(|held| held.model_id == id("CARD.DEPRECATED_CARD"))
    );
    play(&mut simulator, "CARD.DEPRECATED_CARD");
    assert_eq!(pile(&simulator, PileName::Hand), ["CARD.STRIKE_IRONCLAD"]);
    assert_eq!(
        pile(&simulator, PileName::Exhaust),
        ["CARD.DEPRECATED_CARD"]
    );
    assert!(
        !simulator
            .state()
            .run_player
            .deck
            .iter()
            .any(|held| held.model_id == id("CARD.DEPRECATED_CARD")),
        "the deck card it stood for is gone"
    );
}

// --- quest cards ---------------------------------------------------------

/// The three quest cards are unplayable errands; what each of them does happens
/// outside a combat, on a surface this baseline does not have.
#[test]
fn the_quest_cards_are_unplayable_errands() {
    let registry = sts2_content::standard_registry();
    for model_id in ["CARD.BYRDONIS_EGG", "CARD.LANTERN_KEY", "CARD.SPOILS_MAP"] {
        let definition = registry
            .card(&id(model_id))
            .expect("the quest card is registered");
        assert!(definition.unplayable);
        assert_eq!(definition.energy_cost, -1);
        assert_eq!(definition.card_type, sts2_engine::CardType::Quest);
        assert_eq!(definition.rarity, sts2_engine::CardRarity::Quest);
        let simulator = fight(&[(model_id, 0)]);
        assert!(play_action(&simulator, model_id).is_none());
    }
}

// --- event cards ---------------------------------------------------------

/// The event cards that are one number and nothing else.
#[test]
fn the_plain_event_attacks_land_what_they_declare() {
    for (model_id, upgrade, damage) in [
        ("CARD.BYRD_SWOOP", 0, 14),
        ("CARD.BYRD_SWOOP", 1, 18),
        ("CARD.PECK", 0, 6),
        ("CARD.PECK", 1, 8),
        ("CARD.RIP_AND_TEAR", 0, 14),
        ("CARD.RIP_AND_TEAR", 1, 18),
        ("CARD.SQUASH", 0, 10),
        ("CARD.SQUASH", 1, 12),
        ("CARD.WHISTLE", 0, 33),
        ("CARD.WHISTLE", 1, 44),
    ] {
        let mut simulator = scenario(&[(model_id, upgrade)], &[], 60)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
        play(&mut simulator, model_id);
        assert_eq!(
            creature(&simulator, 1).current_hp,
            60 - damage,
            "{model_id} at +{upgrade}"
        );
    }

    // A squash leaves its target open, and a whistle stuns.
    let mut simulator = scenario(&[("CARD.SQUASH", 0)], &[], 60)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.SQUASH");
    assert_eq!(power_amount(&simulator, 1, "POWER.VULNERABLE_POWER"), 2);
    let mut simulator = scenario(&[("CARD.WHISTLE", 0)], &[], 60)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.WHISTLE");
    assert_eq!(
        creature(&simulator, 1)
            .intent
            .as_ref()
            .map(|intent| intent.move_id.clone()),
        Some("STUNNED".to_owned())
    );
}

/// An exterminate hits everything four times over.
#[test]
fn an_exterminate_sweeps_four_times() {
    for (upgrade, damage) in [(0, 3), (1, 4)] {
        let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .player_hp(80, 80)
            .energy(9, 9)
            .turn(1, 1)
            .rng_counter(RunStream::MonsterAi, 0)
            .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
            .enemy(2, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
            .card(PileName::Hand, 0, card("CARD.EXTERMINATE", upgrade), 1)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
        play(&mut simulator, "CARD.EXTERMINATE");
        assert_eq!(creature(&simulator, 1).current_hp, 39 - 4 * damage);
        assert_eq!(creature(&simulator, 2).current_hp, 39 - 4 * damage);
    }
}

/// An apotheosis upgrades every other card of the fight and leaves itself
/// alone.
#[test]
fn an_apotheosis_upgrades_everything_but_itself() {
    let mut simulator = fight_with_draw(
        &[("CARD.APOTHEOSIS", 0), ("CARD.STRIKE_IRONCLAD", 0)],
        &[("CARD.DEFEND_IRONCLAD", 0)],
    );
    play(&mut simulator, "CARD.APOTHEOSIS");
    let upgraded: Vec<(String, u8)> = simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles
        .all_cards()
        .map(|held| {
            (
                held.fingerprint.model_id.to_string(),
                held.fingerprint.upgrade_level,
            )
        })
        .collect();
    for (model_id, level) in upgraded {
        if model_id == "CARD.APOTHEOSIS" {
            assert_eq!(level, 0, "it never upgrades itself");
        } else {
            assert_eq!(level, 1, "{model_id} was upgraded");
        }
    }
}

/// An apparition holds every blow that reaches its owner down to one, and stops
/// burning up in hand once it is upgraded.
#[test]
fn an_apparition_holds_every_blow_to_one() {
    // The beetle chomps for seven; intangible holds it to one.
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "CHOMP_MOVE")
        .card(PileName::Hand, 0, card("CARD.APPARITION", 0), 1)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.APPARITION");
    assert_eq!(power_amount(&simulator, 0, "POWER.INTANGIBLE_POWER"), 1);
    end_turn(&mut simulator);
    assert_eq!(player(&simulator).current_hp, 79);
    assert_eq!(power_amount(&simulator, 0, "POWER.INTANGIBLE_POWER"), 0);

    let registry = sts2_content::standard_registry();
    let apparition = registry
        .card(&id("CARD.APPARITION"))
        .expect("the card is registered");
    assert!(apparition.ethereal_at(0));
    assert!(!apparition.ethereal_at(1));
}

/// The brightest flame pays a point of maximum HP for energy and cards.
#[test]
fn the_brightest_flame_pays_maximum_hp() {
    for (upgrade, gained, cards) in [(0, 2, 2), (1, 3, 3)] {
        let mut simulator = fight_with_draw(
            &[("CARD.BRIGHTEST_FLAME", upgrade)],
            &[
                ("CARD.STRIKE_IRONCLAD", 0),
                ("CARD.STRIKE_IRONCLAD", 0),
                ("CARD.STRIKE_IRONCLAD", 0),
            ],
        );
        let before = energy(&simulator);
        play(&mut simulator, "CARD.BRIGHTEST_FLAME");
        assert_eq!(energy(&simulator), before + gained);
        assert_eq!(pile(&simulator, PileName::Hand).len(), cards);
        assert_eq!(player(&simulator).max_hp, 79);
    }
}

/// A clash may only be played out of a hand of nothing but attacks.
#[test]
fn a_clash_needs_a_hand_of_nothing_but_attacks() {
    let simulator = fight(&[("CARD.CLASH", 0), ("CARD.DEFEND_IRONCLAD", 0)]);
    assert!(play_action(&simulator, "CARD.CLASH").is_none());
    let mut simulator = fight(&[("CARD.CLASH", 0), ("CARD.STRIKE_IRONCLAD", 0)]);
    play(&mut simulator, "CARD.CLASH");
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 14);
}

/// An entrench doubles the block its owner is already standing behind, and
/// nothing scales what it adds.
#[test]
fn an_entrench_doubles_the_block_already_standing() {
    for (upgrade, cost) in [(0, 2), (1, 1)] {
        let mut simulator = fight(&[
            ("CARD.ENTRENCH", upgrade),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
        ]);
        play(&mut simulator, "CARD.DEFEND_IRONCLAD");
        play(&mut simulator, "CARD.DEFEND_IRONCLAD");
        assert_eq!(player(&simulator).block, 10);
        let before = energy(&simulator);
        play(&mut simulator, "CARD.ENTRENCH");
        assert_eq!(player(&simulator).block, 20);
        assert_eq!(energy(&simulator), before - cost);
    }
}

/// A stack is worth the discard pile, and its upgrade raises the base rather
/// than the rate.
#[test]
fn a_stack_is_worth_the_discard_pile() {
    for (upgrade, base) in [(0, 0), (1, 3)] {
        let mut simulator = fight(&[
            ("CARD.STACK", upgrade),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
        ]);
        play(&mut simulator, "CARD.DEFEND_IRONCLAD");
        play(&mut simulator, "CARD.DEFEND_IRONCLAD");
        let before = player(&simulator).block;
        play(&mut simulator, "CARD.STACK");
        assert_eq!(player(&simulator).block, before + base + 2);
    }
}

/// Caltrops and a feeding frenzy are one power each; the frenzy's strength is
/// given back at the end of the turn.
#[test]
fn caltrops_and_a_feeding_frenzy_are_one_power_each() {
    for (upgrade, thorns) in [(0, 3), (1, 5)] {
        let mut simulator = fight(&[("CARD.CALTROPS", upgrade)]);
        play(&mut simulator, "CARD.CALTROPS");
        assert_eq!(power_amount(&simulator, 0, "POWER.THORNS_POWER"), thorns);
    }
    for (upgrade, strength) in [(0, 5), (1, 7)] {
        let mut simulator = fight(&[("CARD.FEEDING_FRENZY", upgrade)]);
        play(&mut simulator, "CARD.FEEDING_FRENZY");
        assert_eq!(
            power_amount(&simulator, 0, "POWER.STRENGTH_POWER"),
            strength
        );
        end_turn(&mut simulator);
        assert_eq!(power_amount(&simulator, 0, "POWER.STRENGTH_POWER"), 0);
    }
}

/// An outmaneuver and a relax pay out when the next turn opens.
#[test]
fn the_next_turn_powers_pay_out_as_the_turn_opens() {
    for (upgrade, gained) in [(0, 2), (1, 3)] {
        let mut simulator = fight_with_draw(
            &[("CARD.OUTMANEUVER", upgrade)],
            &[
                ("CARD.DEFEND_IRONCLAD", 0),
                ("CARD.DEFEND_IRONCLAD", 0),
                ("CARD.DEFEND_IRONCLAD", 0),
                ("CARD.DEFEND_IRONCLAD", 0),
                ("CARD.DEFEND_IRONCLAD", 0),
            ],
        );
        play(&mut simulator, "CARD.OUTMANEUVER");
        end_turn(&mut simulator);
        // The pool refilled to what the scenario gave it, and the power paid on
        // top of that.
        assert_eq!(energy(&simulator), 9 + gained);
        assert_eq!(
            power_amount(&simulator, 0, "POWER.ENERGY_NEXT_TURN_POWER"),
            0
        );
    }

    for (upgrade, block, cards, gained) in [(0, 15, 2, 2), (1, 17, 3, 3)] {
        let mut simulator = fight_with_draw(
            &[("CARD.RELAX", upgrade)],
            &[
                ("CARD.DEFEND_IRONCLAD", 0),
                ("CARD.DEFEND_IRONCLAD", 0),
                ("CARD.DEFEND_IRONCLAD", 0),
                ("CARD.DEFEND_IRONCLAD", 0),
                ("CARD.DEFEND_IRONCLAD", 0),
                ("CARD.DEFEND_IRONCLAD", 0),
                ("CARD.DEFEND_IRONCLAD", 0),
                ("CARD.DEFEND_IRONCLAD", 0),
            ],
        );
        play(&mut simulator, "CARD.RELAX");
        assert_eq!(player(&simulator).block, block);
        end_turn(&mut simulator);
        assert_eq!(energy(&simulator), 9 + gained);
        assert_eq!(pile(&simulator, PileName::Hand).len(), 5 + cards);
    }
}

/// A rebound sends the next card its owner plays back to the top of the draw
/// pile, and is spent for having sent it.
#[test]
fn a_rebound_sends_the_next_card_back_on_top_of_the_draw_pile() {
    let mut simulator = fight_with_draw(
        &[("CARD.REBOUND", 0), ("CARD.DEFEND_IRONCLAD", 0)],
        &[("CARD.STRIKE_IRONCLAD", 0)],
    );
    play(&mut simulator, "CARD.REBOUND");
    // Where a played card goes is settled the moment it reaches the play pile,
    // before its own body runs, so the rebound does not answer for its own
    // play: it is discarded and the power still stands.
    assert_eq!(power_amount(&simulator, 0, "POWER.REBOUND_POWER"), 1);
    assert_eq!(pile(&simulator, PileName::Discard), ["CARD.REBOUND"]);

    play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    assert_eq!(
        pile(&simulator, PileName::Draw),
        ["CARD.DEFEND_IRONCLAD", "CARD.STRIKE_IRONCLAD"]
    );
    assert_eq!(
        power_amount(&simulator, 0, "POWER.REBOUND_POWER"),
        0,
        "and the power is spent for the card it sent back"
    );
}

/// Toric toughness works its block up now and again for two more turns, for the
/// amount the play actually gained.
#[test]
fn toric_toughness_works_its_block_up_again_for_two_more_turns() {
    for (upgrade, block) in [(0, 5), (1, 7)] {
        let mut simulator = fight_with_draw(
            &[("CARD.TORIC_TOUGHNESS", upgrade)],
            &[
                ("CARD.DEFEND_IRONCLAD", 0),
                ("CARD.DEFEND_IRONCLAD", 0),
                ("CARD.DEFEND_IRONCLAD", 0),
                ("CARD.DEFEND_IRONCLAD", 0),
                ("CARD.DEFEND_IRONCLAD", 0),
                ("CARD.DEFEND_IRONCLAD", 0),
                ("CARD.DEFEND_IRONCLAD", 0),
                ("CARD.DEFEND_IRONCLAD", 0),
                ("CARD.DEFEND_IRONCLAD", 0),
                ("CARD.DEFEND_IRONCLAD", 0),
            ],
        );
        play(&mut simulator, "CARD.TORIC_TOUGHNESS");
        assert_eq!(player(&simulator).block, block);
        end_turn(&mut simulator);
        // Block clears as the turn opens and the power works the same amount
        // back up.
        assert_eq!(player(&simulator).block, block);
        assert_eq!(
            power_amount(&simulator, 0, "POWER.TORIC_TOUGHNESS_POWER"),
            1
        );
    }
}

#[test]
fn overlapping_toric_toughness_keeps_each_plays_block_and_duration() {
    for (first, second, first_block, second_block) in [(0, 0, 5, 5), (0, 1, 5, 7), (1, 0, 7, 5)] {
        let mut draw = vec![("CARD.TORIC_TOUGHNESS", second)];
        draw.extend(std::iter::repeat_n(("CARD.DEFEND_IRONCLAD", 0), 20));
        let mut simulator = fight_with_draw(&[("CARD.TORIC_TOUGHNESS", first)], &draw);
        play(&mut simulator, "CARD.TORIC_TOUGHNESS");
        end_turn(&mut simulator);
        assert_eq!(player(&simulator).block, first_block);
        play(&mut simulator, "CARD.TORIC_TOUGHNESS");
        end_turn(&mut simulator);
        assert_eq!(player(&simulator).block, first_block + second_block);
        end_turn(&mut simulator);
        assert_eq!(player(&simulator).block, second_block);
        end_turn(&mut simulator);
        assert_eq!(player(&simulator).block, 0);
    }
}

/// A wish takes any one card out of the draw pile, and its upgrade keeps it in
/// hand rather than changing what it does.
#[test]
fn a_wish_takes_any_one_card_out_of_the_draw_pile() {
    let mut simulator = fight_with_draw(
        &[("CARD.WISH", 0)],
        &[("CARD.STRIKE_IRONCLAD", 0), ("CARD.BASH", 0)],
    );
    play(&mut simulator, "CARD.WISH");
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::ChooseCards { cards, .. }
                if cards.len() == 1 && cards[0].fingerprint.model_id == id("CARD.BASH"))
        })
        .cloned()
        .expect("the draw pile is on the screen");
    simulator.step(action).expect("the choice applies");
    assert_eq!(pile(&simulator, PileName::Hand), ["CARD.BASH"]);
    assert!(
        sts2_content::standard_registry()
            .card(&id("CARD.WISH"))
            .expect("the card is registered")
            .retains_at(1)
    );
}

/// A dual wield copies the attack or power the screen picked out of the hand.
#[test]
fn a_dual_wield_copies_what_the_screen_picked() {
    for (upgrade, copies) in [(0, 1), (1, 2)] {
        let mut simulator = fight(&[
            ("CARD.DUAL_WIELD", upgrade),
            ("CARD.STRIKE_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
        ]);
        play(&mut simulator, "CARD.DUAL_WIELD");
        // The screen only offers the attack: a defend is neither an attack nor
        // a power, so there is nothing to choose between and the build takes
        // the one candidate without asking.
        let strikes = pile(&simulator, PileName::Hand)
            .into_iter()
            .filter(|model_id| model_id == "CARD.STRIKE_IRONCLAD")
            .count();
        assert_eq!(strikes, 1 + copies, "at +{upgrade}");
    }
}

/// Enlightenment holds every card in hand to one energy, for the turn or for
/// the fight, and never makes a cheap card dearer.
#[test]
fn enlightenment_holds_the_hand_to_one_energy() {
    for (upgrade, lasts) in [(0, false), (1, true)] {
        let mut simulator = fight_with_draw(
            &[
                ("CARD.ENLIGHTENMENT", upgrade),
                ("CARD.BLUDGEON", 0),
                ("CARD.ANGER", 0),
            ],
            &[
                ("CARD.DEFEND_IRONCLAD", 0),
                ("CARD.DEFEND_IRONCLAD", 0),
                ("CARD.DEFEND_IRONCLAD", 0),
                ("CARD.DEFEND_IRONCLAD", 0),
                ("CARD.DEFEND_IRONCLAD", 0),
            ],
        );
        play(&mut simulator, "CARD.ENLIGHTENMENT");
        assert_eq!(cost_of(&simulator, "CARD.BLUDGEON"), 1, "at +{upgrade}");
        assert_eq!(
            cost_of(&simulator, "CARD.ANGER"),
            0,
            "a card that already cost nothing is left alone"
        );
        end_turn(&mut simulator);
        let carried = simulator
            .state()
            .combat
            .as_ref()
            .expect("the fight is standing")
            .player
            .piles
            .all_cards()
            .find(|held| held.fingerprint.model_id == id("CARD.BLUDGEON"))
            .expect("the bludgeon is somewhere")
            .energy_cost;
        assert_eq!(carried, if lasts { 1 } else { 3 });
    }
}

fn cost_of(simulator: &Simulator, model_id: &str) -> i32 {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles
        .hand
        .iter()
        .find(|held| held.fingerprint.model_id == id(model_id))
        .expect("the card is held")
        .energy_cost
}

/// A maul grows every maul in the fight, itself included.
#[test]
fn a_maul_grows_every_maul_in_the_fight() {
    for (upgrade, damage, increase) in [(0, 5, 1), (1, 6, 2)] {
        let mut simulator = scenario(&[("CARD.MAUL", upgrade), ("CARD.MAUL", upgrade)], &[], 120)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
        play(&mut simulator, "CARD.MAUL");
        assert_eq!(creature(&simulator, 1).current_hp, 120 - 2 * damage);
        // The second maul grew too, so its own two hits are worth more.
        play(&mut simulator, "CARD.MAUL");
        assert_eq!(
            creature(&simulator, 1).current_hp,
            120 - 2 * damage - 2 * (damage + increase)
        );
    }
}

/// A distraction and a metamorphosis roll cards of their owner's own pool: one
/// skill free for the turn, and attacks free for the fight.
#[test]
fn the_generating_event_cards_roll_the_owners_own_pool() {
    let mut simulator = fight(&[("CARD.DISTRACTION", 0)]);
    play(&mut simulator, "CARD.DISTRACTION");
    let hand = simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles
        .hand
        .clone();
    assert_eq!(hand.len(), 1);
    assert_eq!(hand[0].energy_cost, 0, "free for the turn");
    assert_eq!(
        sts2_content::standard_registry()
            .card(&hand[0].fingerprint.model_id)
            .expect("the rolled card is registered")
            .card_type,
        sts2_engine::CardType::Skill
    );

    for (upgrade, count) in [(0, 3), (1, 5)] {
        let mut simulator = fight(&[("CARD.METAMORPHOSIS", upgrade)]);
        play(&mut simulator, "CARD.METAMORPHOSIS");
        let draw = simulator
            .state()
            .combat
            .as_ref()
            .expect("the fight is standing")
            .player
            .piles
            .draw
            .clone();
        assert_eq!(draw.len(), count);
        for rolled in &draw {
            assert_eq!(rolled.energy_cost, 0);
            assert_eq!(
                sts2_content::standard_registry()
                    .card(&rolled.fingerprint.model_id)
                    .expect("the rolled card is registered")
                    .card_type,
                sts2_engine::CardType::Attack
            );
        }
    }
}

/// Hello world stocks the hand with a common card of its owner's own pool
/// before the turn's own draw.
#[test]
fn hello_world_stocks_the_hand_before_the_draw() {
    let mut simulator = fight_with_draw(
        &[("CARD.HELLO_WORLD", 0)],
        &[
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
        ],
    );
    play(&mut simulator, "CARD.HELLO_WORLD");
    end_turn(&mut simulator);
    // Five drawn, and one common card the power put there first.
    assert_eq!(pile(&simulator, PileName::Hand).len(), 6);
}

/// Neow's fury swings and takes back as many cards off the discard pile as the
/// hand has room for.
#[test]
fn neows_fury_takes_back_what_the_hand_has_room_for() {
    let mut simulator = fight(&[
        ("CARD.NEOWS_FURY", 0),
        ("CARD.DEFEND_IRONCLAD", 0),
        ("CARD.DEFEND_IRONCLAD", 0),
    ]);
    play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    play(&mut simulator, "CARD.NEOWS_FURY");
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 10);
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseCards { cards, .. } if cards.len() == 2))
        .cloned()
        .expect("the discard pile is on the screen, asking for up to two");
    simulator.step(action).expect("the choice applies");
    assert_eq!(pile(&simulator, PileName::Hand).len(), 2);
}

/// Mad science is whatever a tinker's table made of it, and a card no table
/// ever wrote on is nothing: the kind of tinkering it carries opens at a value
/// no branch of its body answers to, so playing one costs an energy and does
/// nothing.
#[test]
fn mad_science_that_no_table_wrote_on_does_nothing() {
    let mut simulator = fight(&[("CARD.MAD_SCIENCE", 0)]);
    play(&mut simulator, "CARD.MAD_SCIENCE");
    assert_eq!(creature(&simulator, 1).current_hp, 39);
    assert_eq!(player(&simulator).block, 0);
    assert!(player(&simulator).powers.is_empty());
}

// --- the colorless pool --------------------------------------------------

/// The colorless cards that are one number and nothing else.
#[test]
fn the_plain_colorless_cards_are_what_they_say() {
    for (model_id, upgrade, damage) in [
        ("CARD.FLASH_OF_STEEL", 0, 5),
        ("CARD.FLASH_OF_STEEL", 1, 8),
        ("CARD.HAND_OF_GREED", 0, 20),
        ("CARD.HAND_OF_GREED", 1, 25),
        ("CARD.JACKPOT", 0, 25),
        ("CARD.JACKPOT", 1, 30),
        ("CARD.SEEKER_STRIKE", 0, 9),
        ("CARD.SEEKER_STRIKE", 1, 12),
    ] {
        let mut simulator = scenario(&[(model_id, upgrade)], &[("CARD.STRIKE_IRONCLAD", 0)], 60)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
        play(&mut simulator, model_id);
        assert_eq!(
            creature(&simulator, 1).current_hp,
            60 - damage,
            "{model_id} at +{upgrade}"
        );
    }
    for (model_id, upgrade, block_amount, cards) in [
        ("CARD.FINESSE", 0, 4, 1),
        ("CARD.FINESSE", 1, 7, 1),
        ("CARD.EQUILIBRIUM", 0, 13, 0),
        ("CARD.EQUILIBRIUM", 1, 16, 0),
        ("CARD.PANIC_BUTTON", 0, 30, 0),
        ("CARD.PANIC_BUTTON", 1, 40, 0),
    ] {
        let mut simulator = fight_with_draw(
            &[(model_id, upgrade)],
            &[("CARD.STRIKE_IRONCLAD", 0), ("CARD.STRIKE_IRONCLAD", 0)],
        );
        play(&mut simulator, model_id);
        assert_eq!(
            player(&simulator).block,
            block_amount,
            "{model_id} at +{upgrade}"
        );
        assert_eq!(pile(&simulator, PileName::Hand).len(), cards);
    }
    for (model_id, upgrade, cards) in [
        ("CARD.MASTER_OF_STRATEGY", 0, 3),
        ("CARD.MASTER_OF_STRATEGY", 1, 4),
    ] {
        let mut simulator = fight_with_draw(
            &[(model_id, upgrade)],
            &[
                ("CARD.STRIKE_IRONCLAD", 0),
                ("CARD.STRIKE_IRONCLAD", 0),
                ("CARD.STRIKE_IRONCLAD", 0),
                ("CARD.STRIKE_IRONCLAD", 0),
            ],
        );
        play(&mut simulator, model_id);
        assert_eq!(pile(&simulator, PileName::Hand).len(), cards);
    }
    for (upgrade, amount) in [(0, 1), (1, 2)] {
        let mut simulator = fight(&[("CARD.PROWESS", upgrade)]);
        play(&mut simulator, "CARD.PROWESS");
        assert_eq!(power_amount(&simulator, 0, "POWER.STRENGTH_POWER"), amount);
        assert_eq!(power_amount(&simulator, 0, "POWER.DEXTERITY_POWER"), amount);
    }
}

/// A hand of greed pays gold only for a kill that counts.
#[test]
fn a_hand_of_greed_pays_only_for_a_kill() {
    let mut simulator = scenario(&[("CARD.HAND_OF_GREED", 0)], &[], 60)
        .gold(0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.HAND_OF_GREED");
    assert_eq!(simulator.state().run_player.gold, 0);

    let mut simulator = scenario(&[("CARD.HAND_OF_GREED", 0)], &[], 5)
        .gold(0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.HAND_OF_GREED");
    assert_eq!(simulator.state().run_player.gold, 20);
}

/// A dark shackles takes an enemy's strength for the turn and gives it back.
#[test]
fn dark_shackles_lends_strength_the_other_way() {
    for (upgrade, taken) in [(0, 9), (1, 15)] {
        let mut simulator = fight(&[("CARD.DARK_SHACKLES", upgrade)]);
        play(&mut simulator, "CARD.DARK_SHACKLES");
        assert_eq!(power_amount(&simulator, 1, "POWER.STRENGTH_POWER"), -taken);
        end_turn(&mut simulator);
        assert_eq!(power_amount(&simulator, 1, "POWER.STRENGTH_POWER"), 0);
    }
}

/// A gold axe is worth every card play the fight has seen, and a mind blast is
/// worth the draw pile.
#[test]
fn the_counting_colorless_attacks_read_what_they_count() {
    let mut simulator = fight(&[
        ("CARD.GOLD_AXE", 0),
        ("CARD.DEFEND_IRONCLAD", 0),
        ("CARD.DEFEND_IRONCLAD", 0),
    ]);
    play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    play(&mut simulator, "CARD.GOLD_AXE");
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 2);

    let mut simulator = fight_with_draw(
        &[("CARD.MIND_BLAST", 0)],
        &[
            ("CARD.STRIKE_IRONCLAD", 0),
            ("CARD.STRIKE_IRONCLAD", 0),
            ("CARD.STRIKE_IRONCLAD", 0),
        ],
    );
    play(&mut simulator, "CARD.MIND_BLAST");
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 3);
    assert!(
        sts2_content::standard_registry()
            .card(&id("CARD.MIND_BLAST"))
            .expect("the card is registered")
            .innate
    );
}

/// Impatience and restlessness pay out only out of the hand they ask for.
#[test]
fn the_conditional_colorless_skills_ask_about_the_hand() {
    // A hand with an attack in it gets nothing.
    let mut simulator = fight_with_draw(
        &[("CARD.IMPATIENCE", 0), ("CARD.STRIKE_IRONCLAD", 0)],
        &[("CARD.DEFEND_IRONCLAD", 0), ("CARD.DEFEND_IRONCLAD", 0)],
    );
    play(&mut simulator, "CARD.IMPATIENCE");
    assert_eq!(pile(&simulator, PileName::Hand), ["CARD.STRIKE_IRONCLAD"]);

    let mut simulator = fight_with_draw(
        &[("CARD.IMPATIENCE", 0), ("CARD.DEFEND_IRONCLAD", 0)],
        &[("CARD.DEFEND_IRONCLAD", 0), ("CARD.DEFEND_IRONCLAD", 0)],
    );
    play(&mut simulator, "CARD.IMPATIENCE");
    assert_eq!(pile(&simulator, PileName::Hand).len(), 3);

    // Restlessness pays out only when nothing else is held.
    let mut simulator = fight_with_draw(
        &[("CARD.RESTLESSNESS", 0), ("CARD.DEFEND_IRONCLAD", 0)],
        &[("CARD.DEFEND_IRONCLAD", 0), ("CARD.DEFEND_IRONCLAD", 0)],
    );
    let before = energy(&simulator);
    play(&mut simulator, "CARD.RESTLESSNESS");
    assert_eq!(energy(&simulator), before);

    let mut simulator = fight_with_draw(
        &[("CARD.RESTLESSNESS", 0)],
        &[("CARD.DEFEND_IRONCLAD", 0), ("CARD.DEFEND_IRONCLAD", 0)],
    );
    let before = energy(&simulator);
    play(&mut simulator, "CARD.RESTLESSNESS");
    assert_eq!(energy(&simulator), before + 2);
    assert_eq!(pile(&simulator, PileName::Hand).len(), 2);
}

/// Fasten adds to the block a Defend works up and to nothing else.
#[test]
fn fasten_pays_only_for_a_defend() {
    for (upgrade, extra) in [(0, 4), (1, 6)] {
        let mut simulator = fight(&[
            ("CARD.FASTEN", upgrade),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.IRON_WAVE", 0),
        ]);
        play(&mut simulator, "CARD.FASTEN");
        play(&mut simulator, "CARD.DEFEND_IRONCLAD");
        assert_eq!(player(&simulator).block, 5 + extra);
        play(&mut simulator, "CARD.IRON_WAVE");
        assert_eq!(
            player(&simulator).block,
            5 + extra + 5,
            "an iron wave is no Defend"
        );
    }
}

/// A panic button pays no block off a card at all while it stands.
#[test]
fn a_panic_button_stops_the_block_that_follows_it() {
    let mut simulator = fight_with_draw(
        &[("CARD.PANIC_BUTTON", 0), ("CARD.DEFEND_IRONCLAD", 0)],
        &[
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
        ],
    );
    play(&mut simulator, "CARD.PANIC_BUTTON");
    assert_eq!(player(&simulator).block, 30);
    play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    assert_eq!(
        player(&simulator).block,
        30,
        "a defend is worth nothing now"
    );
    end_turn(&mut simulator);
    end_turn(&mut simulator);
    assert_eq!(power_amount(&simulator, 0, "POWER.NO_BLOCK_POWER"), 0);
}

/// A prolong carries the block its owner is standing behind into the next turn.
#[test]
fn a_prolong_carries_block_into_the_next_turn() {
    let mut simulator = fight_with_draw(
        &[("CARD.PROLONG", 0), ("CARD.DEFEND_IRONCLAD", 0)],
        &[
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
        ],
    );
    play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    play(&mut simulator, "CARD.PROLONG");
    end_turn(&mut simulator);
    assert_eq!(player(&simulator).block, 5);
}

/// Mayhem plays what is on top of the draw pile as the next turn's auto-play
/// phase opens.
#[test]
fn mayhem_plays_the_top_of_the_draw_pile_every_turn() {
    let mut simulator = fight_with_draw(
        &[("CARD.MAYHEM", 0)],
        &[
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
        ],
    );
    play(&mut simulator, "CARD.MAYHEM");
    end_turn(&mut simulator);
    // Five drawn, one played off the top of what was left.
    assert_eq!(player(&simulator).block, 5);
}

/// A scrawl fills the hand, and a thinking ahead draws two and puts one back.
#[test]
fn the_hand_shaping_colorless_skills_move_what_they_say() {
    let draw: Vec<(&str, u8)> = (0..8).map(|_| ("CARD.DEFEND_IRONCLAD", 0)).collect();
    let mut simulator = fight_with_draw(&[("CARD.SCRAWL", 0)], &draw);
    play(&mut simulator, "CARD.SCRAWL");
    assert_eq!(pile(&simulator, PileName::Hand).len(), 8);

    let mut simulator = fight_with_draw(
        &[("CARD.THINKING_AHEAD", 0)],
        &[("CARD.STRIKE_IRONCLAD", 0), ("CARD.BASH", 0)],
    );
    play(&mut simulator, "CARD.THINKING_AHEAD");
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::ChooseCards { cards, .. }
                if cards.len() == 1 && cards[0].fingerprint.model_id == id("CARD.BASH"))
        })
        .cloned()
        .expect("the hand is on the screen");
    simulator.step(action).expect("the choice applies");
    assert_eq!(pile(&simulator, PileName::Draw), ["CARD.BASH"]);
    assert_eq!(pile(&simulator, PileName::Hand), ["CARD.STRIKE_IRONCLAD"]);
}

/// A secret weapon takes an attack out of the draw pile, and a seeker strike
/// offers three of it.
#[test]
fn the_draw_pile_screens_offer_what_they_should() {
    let mut simulator = fight_with_draw(
        &[("CARD.SECRET_WEAPON", 0)],
        &[("CARD.DEFEND_IRONCLAD", 0), ("CARD.BASH", 0)],
    );
    play(&mut simulator, "CARD.SECRET_WEAPON");
    // Only the attack is a candidate, so the build takes it without asking.
    assert_eq!(pile(&simulator, PileName::Hand), ["CARD.BASH"]);

    let draw: Vec<(&str, u8)> = (0..5).map(|_| ("CARD.DEFEND_IRONCLAD", 0)).collect();
    let mut simulator = fight_with_draw(&[("CARD.SEEKER_STRIKE", 0)], &draw);
    play(&mut simulator, "CARD.SEEKER_STRIKE");
    let offered = simulator
        .legal_actions()
        .iter()
        .filter(|action| matches!(action, Action::ChooseCards { cards, .. } if cards.len() == 1))
        .count();
    assert_eq!(offered, 3, "three of the five are on the screen");

    // A Seeker Strike's shuffle is stable, so changing the pile's hidden order
    // cannot change which three cards the same RNG state puts on screen.
    let offered_by = |draw: &[(&str, u8)]| {
        let mut simulator = fight_with_draw(&[("CARD.SEEKER_STRIKE", 0)], draw);
        play(&mut simulator, "CARD.SEEKER_STRIKE");
        let mut models: Vec<_> = simulator
            .legal_actions()
            .iter()
            .filter_map(|action| match action {
                Action::ChooseCards { cards, .. } if cards.len() == 1 => {
                    Some(cards[0].fingerprint.model_id.clone())
                }
                _ => None,
            })
            .collect();
        models.sort();
        models
    };
    let ordered = [
        ("CARD.BASH", 0),
        ("CARD.DEFEND_IRONCLAD", 0),
        ("CARD.SHRUG_IT_OFF", 0),
        ("CARD.STOMP", 0),
        ("CARD.STRIKE_IRONCLAD", 0),
    ];
    let mut reversed = ordered;
    reversed.reverse();
    assert_eq!(offered_by(&ordered), offered_by(&reversed));
}

/// A shockwave leaves every enemy shaken and open.
#[test]
fn a_shockwave_shakes_and_opens_everything() {
    for (upgrade, amount) in [(0, 3), (1, 5)] {
        let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .player_hp(80, 80)
            .energy(9, 9)
            .turn(1, 1)
            .rng_counter(RunStream::MonsterAi, 0)
            .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
            .enemy(2, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
            .card(PileName::Hand, 0, card("CARD.SHOCKWAVE", upgrade), 2)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
        play(&mut simulator, "CARD.SHOCKWAVE");
        for enemy in [1, 2] {
            assert_eq!(power_amount(&simulator, enemy, "POWER.WEAK_POWER"), amount);
            assert_eq!(
                power_amount(&simulator, enemy, "POWER.VULNERABLE_POWER"),
                amount
            );
        }
    }
}

/// A volley lands once for every point of energy it swallowed.
#[test]
fn a_volley_lands_once_per_point_of_energy() {
    let mut simulator = scenario(&[("CARD.VOLLEY", 0)], &[], 120)
        .energy(4, 4)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.VOLLEY");
    assert_eq!(creature(&simulator, 1).current_hp, 120 - 40);
    assert_eq!(energy(&simulator), 0);
}

/// Automation pays a point of energy for every tenth card drawn.
#[test]
fn automation_pays_for_every_tenth_card_drawn() {
    let counter = |simulator: &Simulator| {
        creature(simulator, 0)
            .powers
            .iter()
            .find(|power| power.model_id == id("POWER.AUTOMATION_POWER"))
            .expect("automation is standing")
            .counter
    };
    let draw: Vec<(&str, u8)> = (0..12).map(|_| ("CARD.DEFEND_IRONCLAD", 0)).collect();
    // A scrawl fills the hand, and the spare card in it holds the draw to nine.
    let mut simulator = fight_with_draw(
        &[
            ("CARD.AUTOMATION", 0),
            ("CARD.SCRAWL", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
        ],
        &draw,
    );
    play(&mut simulator, "CARD.AUTOMATION");
    assert_eq!(counter(&simulator), 10, "it starts the count at ten");
    let before = energy(&simulator);
    play(&mut simulator, "CARD.SCRAWL");
    assert_eq!(pile(&simulator, PileName::Hand).len(), 10);
    assert_eq!(
        energy(&simulator),
        before - 1,
        "the scrawl cost a point and nine drawn is not ten"
    );
    assert_eq!(counter(&simulator), 1);

    // The tenth card pays, and the count goes back to ten.
    let mut simulator = fight_with_draw(&[("CARD.AUTOMATION", 0), ("CARD.SCRAWL", 0)], &draw);
    play(&mut simulator, "CARD.AUTOMATION");
    let before = energy(&simulator);
    play(&mut simulator, "CARD.SCRAWL");
    assert_eq!(pile(&simulator, PileName::Hand).len(), 10);
    assert_eq!(
        energy(&simulator),
        before,
        "the scrawl cost a point and the tenth card paid it back"
    );
    assert_eq!(counter(&simulator), 10);
}

/// Panache hits everything for every fifth card its owner plays, and never
/// counts the card that applied it.
#[test]
fn panache_hits_everything_every_fifth_card() {
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 60, 60, "SHRINKER_MOVE")
        .card(PileName::Hand, 0, card("CARD.PANACHE", 0), 0)
        .card(PileName::Hand, 1, card("CARD.DEFEND_IRONCLAD", 0), 1)
        .card(PileName::Hand, 2, card("CARD.DEFEND_IRONCLAD", 0), 1)
        .card(PileName::Hand, 3, card("CARD.DEFEND_IRONCLAD", 0), 1)
        .card(PileName::Hand, 4, card("CARD.DEFEND_IRONCLAD", 0), 1)
        .card(PileName::Hand, 5, card("CARD.DEFEND_IRONCLAD", 0), 1)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.PANACHE");
    for _ in 0..4 {
        play(&mut simulator, "CARD.DEFEND_IRONCLAD");
        assert_eq!(creature(&simulator, 1).current_hp, 60);
    }
    play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    assert_eq!(creature(&simulator, 1).current_hp, 50);
}

#[test]
fn overlapping_panaches_initialize_and_advance_their_own_counters() {
    let mut hand = vec![("CARD.PANACHE", 0), ("CARD.PANACHE", 1)];
    hand.extend(std::iter::repeat_n(("CARD.DEFEND_IRONCLAD", 0), 6));
    let mut simulator = scenario(&hand, &[], 200)
        .build(sts2_content::standard_registry())
        .unwrap();
    let counters = |simulator: &Simulator| {
        creature(simulator, 0)
            .powers
            .iter()
            .filter(|power| power.model_id == id("POWER.PANACHE_POWER"))
            .map(|power| power.counter)
            .collect::<Vec<_>>()
    };
    play(&mut simulator, "CARD.PANACHE");
    play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    assert_eq!(counters(&simulator), vec![4]);
    play(&mut simulator, "CARD.PANACHE");
    assert_eq!(counters(&simulator), vec![3, 5]);
    for _ in 0..3 {
        play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    }
    assert_eq!(creature(&simulator, 1).current_hp, 190);
    assert_eq!(counters(&simulator), vec![5, 2]);
    for _ in 0..2 {
        play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    }
    assert_eq!(creature(&simulator, 1).current_hp, 176);
    assert_eq!(counters(&simulator), vec![3, 5]);
}

/// A rolling boulder rolls over everything as the turn opens, and rolls harder
/// every turn.
#[test]
fn a_rolling_boulder_rolls_harder_every_turn() {
    let draw: Vec<(&str, u8)> = (0..10).map(|_| ("CARD.DEFEND_IRONCLAD", 0)).collect();
    let mut simulator = fight_with_draw(&[("CARD.ROLLING_BOULDER", 0)], &draw);
    play(&mut simulator, "CARD.ROLLING_BOULDER");
    assert_eq!(creature(&simulator, 1).current_hp, 39);
    end_turn(&mut simulator);
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 5);
    end_turn(&mut simulator);
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 5 - 10);
}

/// The bomb goes off three turns later, over everything.
#[test]
fn the_bomb_goes_off_three_turns_later() {
    let draw: Vec<(&str, u8)> = (0..15).map(|_| ("CARD.DEFEND_IRONCLAD", 0)).collect();
    let mut simulator = scenario(&[("CARD.THE_BOMB", 0)], &draw, 120)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.THE_BOMB");
    end_turn(&mut simulator);
    assert_eq!(creature(&simulator, 1).current_hp, 120);
    end_turn(&mut simulator);
    assert_eq!(creature(&simulator, 1).current_hp, 120);
    end_turn(&mut simulator);
    assert_eq!(creature(&simulator, 1).current_hp, 120 - 40);
    assert_eq!(power_amount(&simulator, 0, "POWER.THE_BOMB_POWER"), 0);
}

/// Prep time works vigor up every turn, and vigor is spent whole for one swing.
#[test]
fn prep_time_works_vigor_up_and_a_swing_spends_it() {
    // The beetle chomps rather than shrinking, so nothing scales the swing.
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 120, 120, "CHOMP_MOVE")
        .card(PileName::Hand, 0, card("CARD.PREP_TIME", 0), 1);
    for index in 0..10 {
        scenario = scenario.card(
            PileName::Draw,
            1 + index,
            card("CARD.STRIKE_IRONCLAD", 0),
            1,
        );
    }
    let mut simulator = scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.PREP_TIME");
    end_turn(&mut simulator);
    assert_eq!(power_amount(&simulator, 0, "POWER.VIGOR_POWER"), 4);
    let before = creature(&simulator, 1).current_hp;
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(before - creature(&simulator, 1).current_hp, 6 + 4);
    assert_eq!(power_amount(&simulator, 0, "POWER.VIGOR_POWER"), 0);
    let before = creature(&simulator, 1).current_hp;
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(before - creature(&simulator, 1).current_hp, 6);
}

/// Vigor binds the swing it will pay for and refuses every other: a swing from
/// a different card while the binding stands gets nothing, and the binding is
/// never cleared, so a vigor that survived its own swing keeps answering only
/// for the card it bound.
#[test]
fn vigor_pays_for_the_one_swing_it_bound_and_no_other() {
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 300, 300, "CHOMP_MOVE")
        .power(0, id("POWER.VIGOR_POWER"), 4, sts2_engine::PowerType::Buff)
        .card(PileName::Hand, 0, card("CARD.STRIKE_IRONCLAD", 0), 0)
        .card(PileName::Hand, 1, card("CARD.STRIKE_IRONCLAD", 0), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    let before = creature(&simulator, 1).current_hp;
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(before - creature(&simulator, 1).current_hp, 6 + 4);
    assert_eq!(power_amount(&simulator, 0, "POWER.VIGOR_POWER"), 0);
    // The binding stands even though the power is gone; a second strike, which
    // is a different card, is worth its own six.
    let before = creature(&simulator, 1).current_hp;
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(before - creature(&simulator, 1).current_hp, 6);
}

/// A jack of all trades rolls colorless cards other than itself.
#[test]
fn a_jack_of_all_trades_never_rolls_another_jack() {
    for (upgrade, count) in [(0, 1), (1, 2)] {
        let mut simulator = fight(&[("CARD.JACK_OF_ALL_TRADES", upgrade)]);
        play(&mut simulator, "CARD.JACK_OF_ALL_TRADES");
        let hand = pile(&simulator, PileName::Hand);
        assert_eq!(hand.len(), count);
        assert!(!hand.contains(&"CARD.JACK_OF_ALL_TRADES".to_owned()));
    }
}

/// A discovery lays three of the character's own cards out, and what is picked
/// is free for the turn.
#[test]
fn a_discovery_offers_three_and_gives_one_free() {
    let mut simulator = fight(&[("CARD.DISCOVERY", 0)]);
    play(&mut simulator, "CARD.DISCOVERY");
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseCards { cards, .. } if cards.len() == 1))
        .cloned()
        .expect("three cards are on the screen");
    simulator.step(action).expect("the choice applies");
    let hand = simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles
        .hand
        .clone();
    assert_eq!(hand.len(), 1);
    assert_eq!(hand[0].energy_cost, 0);
}

/// A hidden gem makes one card of the draw pile play twice more.
#[test]
fn a_hidden_gem_makes_one_card_play_again() {
    for (upgrade, replays) in [(0, 2), (1, 3)] {
        let mut simulator = fight_with_draw(
            &[("CARD.HIDDEN_GEM", upgrade)],
            &[("CARD.STRIKE_IRONCLAD", 0)],
        );
        play(&mut simulator, "CARD.HIDDEN_GEM");
        assert_eq!(
            simulator
                .state()
                .combat
                .as_ref()
                .expect("the fight is standing")
                .player
                .piles
                .draw[0]
                .replay_count,
            replays
        );
    }

    // With no attack, skill, or power in the pile, the roll falls back to the
    // wider list, which a playable status is in.
    let mut status = fight_with_draw(
        &[("CARD.HIDDEN_GEM", 0)],
        &[("CARD.SLIMED", 0), ("CARD.CLUMSY", 0)],
    );
    play(&mut status, "CARD.HIDDEN_GEM");
    let draw = &status
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles
        .draw;
    let slimed = draw
        .iter()
        .find(|card| card.fingerprint.model_id.to_string() == "CARD.SLIMED")
        .expect("the slimed is in the pile");
    assert_eq!(slimed.replay_count, 2, "the status takes the replay");
    assert!(
        draw.iter()
            .filter(|card| card.fingerprint.model_id.to_string() == "CARD.CLUMSY")
            .all(|card| card.replay_count == 0),
        "and a curse never does"
    );
}

/// The gem's own filter drops a card whose enchantment already grants it a
/// replay, from both lists: a Spiral on the only Strike in the pile leaves
/// nothing for the roll to land on.
#[test]
fn a_hidden_gem_passes_over_a_card_an_enchantment_already_replays() {
    let mut spiralled = card("CARD.STRIKE_IRONCLAD", 0);
    spiralled.enchantment = Some(sts2_engine::EnchantmentFingerprint {
        model_id: id("ENCHANTMENT.SPIRAL"),
        amount: 1,
        properties: std::collections::BTreeMap::new(),
    });
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(70, 70)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .card(PileName::Hand, 0, card("CARD.HIDDEN_GEM", 0), 0)
        .card(PileName::Draw, 1, spiralled, 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.HIDDEN_GEM");
    let draw = &simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles
        .draw;
    assert_eq!(
        draw[0].replay_count, 0,
        "the spiral already grants the replay, so the gem passes it over"
    );
}

/// An alchemize brews one potion out of nothing: a rarity and a potion rolled
/// off the combat potion-generation stream, and the belt takes it.
#[test]
fn an_alchemize_brews_one_potion() {
    let mut simulator = fight(&[("CARD.ALCHEMIZE", 0)]);
    assert!(
        simulator
            .state()
            .run_player
            .potions
            .iter()
            .all(Option::is_none)
    );
    play(&mut simulator, "CARD.ALCHEMIZE");
    assert_eq!(
        simulator
            .state()
            .rng
            .stream(RunStream::CombatPotionGeneration)
            .counter(),
        2
    );
    assert_eq!(
        simulator
            .state()
            .run_player
            .potions
            .iter()
            .flatten()
            .count(),
        1
    );
}

/// A catastrophe plays cards off the draw pile that nobody chose, and falls
/// back to the whole pile where it holds no playable card at all.
#[test]
fn a_catastrophe_plays_what_it_rolls() {
    let mut simulator = fight_with_draw(
        &[("CARD.CATASTROPHE", 0)],
        &[("CARD.DEFEND_IRONCLAD", 0), ("CARD.DEFEND_IRONCLAD", 0)],
    );
    play(&mut simulator, "CARD.CATASTROPHE");
    assert_eq!(player(&simulator).block, 10);
    assert!(pile(&simulator, PileName::Draw).is_empty());

    // A pile of nothing but unplayable cards is still played off: the build
    // shuffles the whole pile when its own list comes up empty, and an
    // unplayable card auto-played does nothing but move.
    let mut unplayable = fight_with_draw(
        &[("CARD.CATASTROPHE", 0)],
        &[("CARD.CLUMSY", 0), ("CARD.CLUMSY", 0)],
    );
    play(&mut unplayable, "CARD.CATASTROPHE");
    assert!(
        pile(&unplayable, PileName::Draw).is_empty(),
        "both cards were taken off the pile all the same"
    );
}

/// Entropy upgrades to Innate, and its turn-start transform uses the combat
/// eligible slice of its owner's pool and records the replacement as generated.
#[test]
fn an_entropy_puts_its_power_on_and_nothing_else() {
    let model_id = "CARD.ENTROPY";
    let registry = sts2_content::standard_registry();
    let upgrade = registry
        .card(&id(model_id))
        .and_then(|definition| definition.upgrade.as_ref())
        .expect("Entropy has an upgraded form");
    assert!(upgrade.innate, "its whole upgrade is becoming Innate");

    let mut simulator = fight(&[(model_id, 0)]);
    play(&mut simulator, model_id);
    assert_eq!(
        creature(&simulator, 0)
            .powers
            .iter()
            .find(|power| power.model_id.to_string() == "POWER.ENTROPY_POWER")
            .map(|power| power.amount),
        Some(1),
        "one card of the hand turns into another every turn while it stands"
    );

    // A stream position at which the two filters part ways: with the
    // combat-ineligible cards taken out ahead of the roll, the transform
    // lands on Refine Blade rather than Reflect.
    let mut transformed = ScenarioBuilder::new("ZPBUDFAAAA", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .played_as(id("CHARACTER.REGENT"))
        .player_hp(75, 75)
        .energy(3, 3)
        .turn(3, 3)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .power(
            0,
            id("POWER.ENTROPY_POWER"),
            1,
            sts2_engine::PowerType::Buff,
        )
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .card(PileName::Draw, 10, card("CARD.DEFEND_REGENT", 0), 1)
        .build(registry)
        .expect("the transform scenario builds");
    end_turn(&mut transformed);
    assert_eq!(
        pile(&transformed, PileName::Hand),
        ["CARD.REFINE_BLADE"],
        "the combat-ineligible Royalties is removed before this roll"
    );
    assert_eq!(
        transformed
            .state()
            .combat
            .as_ref()
            .expect("the fight is standing")
            .player
            .cards_generated_this_combat,
        1,
        "a combat transform is a CardGeneratedEntry"
    );
}

/// The eleven multiplayer-only colorless cards are not registered at all: a
/// single-player run's pools never hold one, so nothing can offer, hold, or
/// play it.
#[test]
fn the_multiplayer_only_colorless_cards_are_not_here() {
    let registry = sts2_content::standard_registry();
    for model_id in [
        "CARD.BEACON_OF_HOPE",
        "CARD.BELIEVE_IN_YOU",
        "CARD.COORDINATE",
        "CARD.GANG_UP",
        "CARD.HUDDLE_UP",
        "CARD.INTERCEPT",
        "CARD.KNOCKDOWN",
        "CARD.LIFT",
        "CARD.MIMIC",
        "CARD.RALLY",
        "CARD.TAG_TEAM",
    ] {
        assert!(
            registry.card(&id(model_id)).is_none(),
            "{model_id} is multiplayer-only"
        );
    }
}

// --- the colorless cards the recorded runs are offered --------------------

/// The offered colorless cards that are one number and nothing else.
#[test]
fn the_offered_colorless_cards_are_what_they_say() {
    for (model_id, upgrade, damage) in [
        ("CARD.BOLAS", 0, 3),
        ("CARD.BOLAS", 1, 4),
        ("CARD.SALVO", 0, 12),
        ("CARD.SALVO", 1, 16),
        ("CARD.THRUMMING_HATCHET", 0, 11),
        ("CARD.THRUMMING_HATCHET", 1, 14),
        ("CARD.ULTIMATE_STRIKE", 0, 14),
        ("CARD.ULTIMATE_STRIKE", 1, 20),
        ("CARD.DRAMATIC_ENTRANCE", 0, 11),
        ("CARD.DRAMATIC_ENTRANCE", 1, 15),
    ] {
        let mut simulator = scenario(&[(model_id, upgrade)], &[], 60)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
        play(&mut simulator, model_id);
        assert_eq!(
            creature(&simulator, 1).current_hp,
            60 - damage,
            "{model_id} at +{upgrade}"
        );
    }
    for (model_id, upgrade, block_amount) in [
        ("CARD.ULTIMATE_DEFEND", 0, 11),
        ("CARD.ULTIMATE_DEFEND", 1, 15),
        ("CARD.THE_GAMBIT", 0, 50),
        ("CARD.THE_GAMBIT", 1, 75),
    ] {
        let mut simulator = fight(&[(model_id, upgrade)]);
        play(&mut simulator, model_id);
        assert_eq!(
            player(&simulator).block,
            block_amount,
            "{model_id} at +{upgrade}"
        );
    }
    for (upgrade, gained) in [(0, 2), (1, 3)] {
        let mut simulator = fight(&[("CARD.PRODUCTION", upgrade)]);
        let before = energy(&simulator);
        play(&mut simulator, "CARD.PRODUCTION");
        assert_eq!(energy(&simulator), before + gained);
        assert_eq!(pile(&simulator, PileName::Exhaust), ["CARD.PRODUCTION"]);
    }
    // A salvo keeps the hand for a turn, and the two ultimates count as what
    // they are named after.
    let registry = sts2_content::standard_registry();
    assert!(
        registry
            .card(&id("CARD.ULTIMATE_DEFEND"))
            .expect("registered")
            .tags
            .contains(&sts2_engine::CardTag::Defend)
    );
    assert!(
        registry
            .card(&id("CARD.ULTIMATE_STRIKE"))
            .expect("registered")
            .tags
            .contains(&sts2_engine::CardTag::Strike)
    );
    assert!(
        registry
            .card(&id("CARD.DRAMATIC_ENTRANCE"))
            .expect("registered")
            .innate
    );
}

/// Fisticuffs works up block worth every point its swing dealt, overkill
/// included.
#[test]
fn fisticuffs_works_up_block_worth_its_own_swing() {
    for (upgrade, damage) in [(0, 7), (1, 9)] {
        let mut simulator = scenario(&[("CARD.FISTICUFFS", upgrade)], &[], 60)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
        play(&mut simulator, "CARD.FISTICUFFS");
        assert_eq!(player(&simulator).block, damage);
    }
    // A blow worth more than the target had left counts the overkill too. The
    // second beetle keeps the fight standing, because winning strips the player
    // of everything the fight gave them.
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 3, 3, "SHRINKER_MOVE")
        .enemy(2, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .card(PileName::Hand, 0, card("CARD.FISTICUFFS", 0), 1)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::PlayCard { card, target: Some(target) }
            if card.fingerprint.model_id == id("CARD.FISTICUFFS") && target.combat_id.get() == 1)
        })
        .cloned()
        .expect("the low beetle can be aimed at");
    simulator.step(action).expect("the play applies");
    assert_eq!(player(&simulator).block, 7);
}

/// A rend is worth more for every lasting debuff its target carries, and a
/// debuff that gives itself back at the end of the turn is not one of them.
#[test]
fn a_rend_counts_the_debuffs_its_target_carries() {
    // Weak and Frail both hold down what their carrier does rather than what
    // reaches it, so neither of them scales the rend itself.
    let rend_damage = |debuffs: &[&str], upgrade: u8| {
        let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .player_hp(80, 80)
            .energy(9, 9)
            .turn(1, 1)
            .rng_counter(RunStream::MonsterAi, 0)
            .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 120, 120, "SHRINKER_MOVE")
            .card(PileName::Hand, 0, card("CARD.REND", upgrade), 2);
        for model_id in debuffs {
            scenario = scenario.power(1, id(model_id), 2, sts2_engine::PowerType::Debuff);
        }
        let mut simulator = scenario
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
        let before = creature(&simulator, 1).current_hp;
        play(&mut simulator, "CARD.REND");
        before - creature(&simulator, 1).current_hp
    };
    for (upgrade, base, per) in [(0, 15, 5), (1, 18, 8)] {
        assert_eq!(rend_damage(&[], upgrade), base);
        assert_eq!(rend_damage(&["POWER.WEAK_POWER"], upgrade), base + per);
        assert_eq!(
            rend_damage(&["POWER.WEAK_POWER", "POWER.FRAIL_POWER"], upgrade),
            base + 2 * per
        );
        // Dark shackles gives the strength it took back at the end of the turn,
        // and the build's count passes over a debuff that does.
        assert_eq!(
            rend_damage(&["POWER.DARK_SHACKLES_POWER"], upgrade),
            base,
            "a temporary debuff is not counted"
        );
    }
}

/// An anointed takes every rare card the hand has room for out of the draw
/// pile, and leaves everything else where it is.
#[test]
fn an_anointed_takes_the_rare_cards_of_the_draw_pile() {
    let mut simulator = fight_with_draw(
        &[("CARD.ANOINTED", 0)],
        &[
            ("CARD.OFFERING", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.BARRICADE", 0),
        ],
    );
    play(&mut simulator, "CARD.ANOINTED");
    let mut hand = pile(&simulator, PileName::Hand);
    hand.sort();
    assert_eq!(hand, ["CARD.BARRICADE", "CARD.OFFERING"]);
    assert_eq!(pile(&simulator, PileName::Draw), ["CARD.DEFEND_IRONCLAD"]);
    assert!(
        sts2_content::standard_registry()
            .card(&id("CARD.ANOINTED"))
            .expect("registered")
            .retains_at(1)
    );
}

/// A beat down plays attacks off the discard pile that nobody chose.
#[test]
fn a_beat_down_plays_attacks_out_of_the_discard_pile() {
    let mut simulator = scenario(
        &[
            ("CARD.BEAT_DOWN", 0),
            ("CARD.STRIKE_IRONCLAD", 0),
            ("CARD.STRIKE_IRONCLAD", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
        ],
        &[],
        120,
    )
    .build(sts2_content::standard_registry())
    .expect("the scenario builds");
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    let before = creature(&simulator, 1).current_hp;
    play(&mut simulator, "CARD.BEAT_DOWN");
    // Only the two strikes are attacks, and each lands once more.
    assert_eq!(before - creature(&simulator, 1).current_hp, 12);
}

/// Purity burns up to three cards of the hand, and is never put down.
#[test]
fn purity_burns_what_the_screen_picked() {
    let mut simulator = fight(&[
        ("CARD.PURITY", 0),
        ("CARD.DEFEND_IRONCLAD", 0),
        ("CARD.STRIKE_IRONCLAD", 0),
    ]);
    play(&mut simulator, "CARD.PURITY");
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseCards { cards, .. } if cards.len() == 2))
        .cloned()
        .expect("the hand is on the screen, asking for up to three");
    simulator.step(action).expect("the choice applies");
    let mut exhausted = pile(&simulator, PileName::Exhaust);
    exhausted.sort();
    assert_eq!(
        exhausted,
        [
            "CARD.DEFEND_IRONCLAD",
            "CARD.PURITY",
            "CARD.STRIKE_IRONCLAD"
        ]
    );
}

/// A calamity deals out attacks of its owner's own pool for every attack they
/// play.
#[test]
fn a_calamity_deals_out_an_attack_for_every_attack() {
    let mut simulator = fight(&[("CARD.CALAMITY", 0), ("CARD.STRIKE_IRONCLAD", 0)]);
    play(&mut simulator, "CARD.CALAMITY");
    assert_eq!(pile(&simulator, PileName::Hand).len(), 1);
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    let hand = pile(&simulator, PileName::Hand);
    assert_eq!(hand.len(), 1, "one attack of the pool joined the hand");
    assert_eq!(
        sts2_content::standard_registry()
            .card(&id(&hand[0]))
            .expect("the rolled card is registered")
            .card_type,
        sts2_engine::CardType::Attack
    );
}

/// Frail scales the block a card works up by three quarters, and leaves alone
/// the block a power grants its own owner, which is unpowered.
#[test]
fn frail_takes_a_quarter_off_the_block_a_card_works_up() {
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(70, 70)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .power(
            0,
            id("POWER.FRAIL_POWER"),
            1,
            sts2_engine::PowerType::Debuff,
        )
        .card(PileName::Hand, 0, card("CARD.DEFEND_IRONCLAD", 0), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    // Block is truncated to a whole number as it is gained: three quarters of
    // five is three and three quarters, and three is what stands.
    assert_eq!(player(&simulator).block, 3);
}

/// The cards a power owes for the next turn are added to that turn's own draw,
/// and the gate is what the power was worth as the turn opened: a stack applied
/// after the snapshot pays nothing and stays for the turn after.
#[test]
fn cards_owed_for_the_next_turn_are_gated_on_the_turn_start_snapshot() {
    let filler: Vec<(&str, u8)> = std::iter::repeat_n(("CARD.DEFEND_IRONCLAD", 0), 12).collect();
    let mut simulator = fight_with_draw(&[("CARD.HAND_OF_GREED", 0)], &filler);
    // Nothing owes anything yet, so the turn deals its own five.
    end_turn(&mut simulator);
    let plain = pile(&simulator, PileName::Hand).len();

    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(70, 70)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .power(
            0,
            id("POWER.DRAW_CARDS_NEXT_TURN_POWER"),
            2,
            sts2_engine::PowerType::Buff,
        );
    for (index, (model_id, upgrade)) in filler.iter().enumerate() {
        simulator = simulator.card(
            PileName::Draw,
            u64::try_from(index).expect("small index"),
            card(model_id, *upgrade),
            0,
        );
    }
    let mut simulator = simulator
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    end_turn(&mut simulator);
    // The power was standing at the snapshot, so it paid and came off.
    assert_eq!(pile(&simulator, PileName::Hand).len(), plain + 2);
    assert_eq!(
        power_amount(&simulator, 0, "POWER.DRAW_CARDS_NEXT_TURN_POWER"),
        0
    );
}

/// A calamity is written down as an attack play opens and pays out as the same
/// play finishes, so a calamity that landed *inside* an attack's own play owes
/// that play nothing. A play with no calamity standing when it began pays for
/// nothing at all.
#[test]
fn a_calamity_owes_nothing_for_a_play_it_landed_inside() {
    // A dual wield replays the attack it copies; playing the calamity in the
    // middle of a turn leaves the plays already begun unpaid for.
    let mut simulator = fight(&[("CARD.CALAMITY", 0), ("CARD.STRIKE_IRONCLAD", 0)]);
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    play(&mut simulator, "CARD.CALAMITY");
    assert_eq!(
        pile(&simulator, PileName::Hand).len(),
        0,
        "the strike had finished before the calamity stood up"
    );
}

/// Nostalgia sends the first attack or skill of a turn back on top of the draw
/// pile, and leaves the rest of them alone.
#[test]
fn nostalgia_sends_the_first_card_of_a_turn_back_on_top() {
    let mut simulator = fight_with_draw(
        &[
            ("CARD.NOSTALGIA", 0),
            ("CARD.DEFEND_IRONCLAD", 0),
            ("CARD.STRIKE_IRONCLAD", 0),
        ],
        &[("CARD.BASH", 0)],
    );
    // The power card itself is not an attack or a skill, and leaves the state.
    play(&mut simulator, "CARD.NOSTALGIA");
    play(&mut simulator, "CARD.DEFEND_IRONCLAD");
    assert_eq!(
        pile(&simulator, PileName::Draw),
        ["CARD.DEFEND_IRONCLAD", "CARD.BASH"]
    );
    play(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(
        pile(&simulator, PileName::Discard),
        ["CARD.STRIKE_IRONCLAD"]
    );
}

/// A bolas and a thrumming hatchet come back to the hand as the next turn's
/// draw opens.
#[test]
fn the_returning_attacks_come_back_to_the_hand() {
    for model_id in ["CARD.BOLAS", "CARD.THRUMMING_HATCHET"] {
        let draw: Vec<(&str, u8)> = (0..6).map(|_| ("CARD.DEFEND_IRONCLAD", 0)).collect();
        let mut simulator = fight_with_draw(&[(model_id, 0)], &draw);
        play(&mut simulator, model_id);
        assert_eq!(pile(&simulator, PileName::Discard), [model_id]);
        end_turn(&mut simulator);
        assert!(
            pile(&simulator, PileName::Hand).contains(&model_id.to_owned()),
            "{model_id} came back"
        );
    }
}

/// The gambit kills its owner on the first blow that gets through the wall it
/// put up.
#[test]
fn the_gambit_is_paid_for_by_the_first_blow_through() {
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "CHOMP_MOVE")
        .card(PileName::Hand, 0, card("CARD.THE_GAMBIT", 0), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.THE_GAMBIT");
    assert_eq!(player(&simulator).block, 50);
    // The chomp is stopped whole by the block, so nothing gets through.
    end_turn(&mut simulator);
    assert_eq!(player(&simulator).current_hp, 80);
    assert_eq!(power_amount(&simulator, 0, "POWER.THE_GAMBIT_POWER"), 1);
    // Block is cleared as the turn opens, so the next chomp lands and the
    // gambit is paid: its owner goes down, and there is no fight left.
    end_turn(&mut simulator);
    assert!(simulator.state().combat.is_none());
}

/// A splash offers three attacks out of every character pool but the walking
/// one's, and whichever is taken lands in the hand free for the turn.
#[test]
fn a_splash_offers_the_other_characters_attacks() {
    let mut simulator = fight(&[("CARD.SPLASH", 0)]);
    play(&mut simulator, "CARD.SPLASH");
    let offered: Vec<String> = simulator
        .legal_actions()
        .iter()
        .filter_map(|action| match action {
            sts2_engine::Action::ChooseCards { cards, .. } => cards.first(),
            _ => None,
        })
        .map(|card| card.fingerprint.model_id.to_string())
        .collect();
    assert_eq!(offered.len(), 3, "three of them");
    let registry = sts2_content::standard_registry();
    let ironclad: Vec<String> = registry
        .card_pool(&id("CARD_POOL.IRONCLAD_CARD_POOL"))
        .expect("the walking character's pool is registered")
        .iter()
        .map(|card| card.model_id.to_string())
        .collect();
    for model_id in &offered {
        let definition = registry
            .card(&id(model_id))
            .expect("the offer is registered content");
        assert_eq!(
            definition.card_type,
            sts2_engine::CardType::Attack,
            "{model_id}"
        );
        assert!(
            !ironclad.contains(model_id),
            "{model_id} is not the walking character's"
        );
    }
    let taken = offered[0].clone();
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, sts2_engine::Action::ChooseCards { cards, .. }
            if cards.first().is_some_and(|card| card.fingerprint.model_id == id(&taken)))
        })
        .cloned()
        .expect("the first offer can be taken");
    simulator.step(action).expect("taking it applies");
    let landed = simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles
        .hand
        .iter()
        .find(|card| card.fingerprint.model_id == id(&taken))
        .expect("it landed in the hand");
    assert_eq!(landed.energy_cost, 0, "free for the turn it landed on");
}

/// A stratagem hands as many cards as it has stacks back out of the draw pile
/// every time that pile is shuffled.
#[test]
fn a_stratagem_deals_off_every_shuffle() {
    // The draw pile is empty and the discard pile holds three, so the next
    // draw shuffles before it deals: the shuffle is what the power answers.
    let mut simulator = scenario(&[("CARD.STRATAGEM", 0)], &[], 39)
        .card(PileName::Discard, 8, card("CARD.STRIKE_IRONCLAD", 0), 0)
        .card(PileName::Discard, 9, card("CARD.DEFEND_IRONCLAD", 0), 0)
        .card(PileName::Discard, 10, card("CARD.DEFEND_IRONCLAD", 0), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    play(&mut simulator, "CARD.STRATAGEM");
    assert!(
        player(&simulator)
            .powers
            .iter()
            .any(|power| power.model_id == id("POWER.STRATAGEM_POWER") && power.amount == 1),
        "the power landed"
    );
    end_turn(&mut simulator);
    // The turn's draw shuffled the discard pile back in and stopped there: the
    // power asks which of the shuffled cards to take back, with the whole pile
    // on offer and nothing drawn yet.
    let sts2_engine::DecisionContext::ChooseCards {
        choice_id,
        candidates,
        ..
    } = simulator.decision().clone()
    else {
        panic!("the stratagem asks: {:?}", simulator.decision());
    };
    assert_eq!(
        candidates.len(),
        3,
        "the pick is over the whole shuffled pile"
    );
    assert!(
        pile(&simulator, PileName::Hand).is_empty(),
        "the draw waits on the pick"
    );
    let taken = candidates[0].clone();
    simulator
        .step(sts2_engine::Action::ChooseCards {
            choice_id,
            cards: vec![taken],
        })
        .expect("the pick applies");
    // The picked card joined the hand and the draw dealt the rest behind it.
    assert_eq!(
        pile(&simulator, PileName::Hand).len() + pile(&simulator, PileName::Draw).len(),
        3,
        "every card that was in the discard pile is accounted for"
    );
    assert!(
        pile(&simulator, PileName::Draw).is_empty(),
        "three cards is fewer than a turn deals, so the power's own draw \
         emptied what the shuffle left"
    );
}

/// A Mad Science standing in the hand with the type and the rider a tinker's
/// table wrote onto it.
fn tinkered(card_type: i32, rider: i32) -> CardFingerprint {
    card("CARD.MAD_SCIENCE", 0)
        .with_property("TinkerTimeType", card_type)
        .with_property("TinkerTimeRider", rider)
}

fn tinkered_fight(card_type: i32, rider: i32) -> Simulator {
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 200, 200, "SHRINKER_MOVE");
    scenario = scenario.card(PileName::Hand, 0, tinkered(card_type, rider), 0);
    scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

#[test]
fn a_mad_science_attack_swings_once_or_three_times_for_violence() {
    // Sapping: one swing for twelve, then Weak and Vulnerable on what it hit.
    let mut sapping = tinkered_fight(1, 1);
    play(&mut sapping, "CARD.MAD_SCIENCE");
    assert_eq!(creature(&sapping, 1).current_hp, 188);
    let powers: Vec<String> = creature(&sapping, 1)
        .powers
        .iter()
        .map(|power| power.model_id.to_string())
        .collect();
    assert!(
        powers.contains(&"POWER.WEAK_POWER".to_owned()),
        "{powers:?}"
    );
    assert!(
        powers.contains(&"POWER.VULNERABLE_POWER".to_owned()),
        "{powers:?}"
    );

    // Violence: the same twelve, three times.
    let mut violence = tinkered_fight(1, 2);
    play(&mut violence, "CARD.MAD_SCIENCE");
    assert_eq!(creature(&violence, 1).current_hp, 164);

    // Choking is the rider behind the swing rather than inside it.
    let mut choking = tinkered_fight(1, 3);
    play(&mut choking, "CARD.MAD_SCIENCE");
    assert_eq!(creature(&choking, 1).current_hp, 188);
    assert!(
        creature(&choking, 1)
            .powers
            .iter()
            .any(|power| power.model_id == id("POWER.STRANGLE_POWER") && power.amount == 6)
    );
}

#[test]
fn a_mad_science_skill_blocks_eight_and_pays_its_rider() {
    let mut energized = tinkered_fight(2, 4);
    let energy = energized.state().combat.as_ref().unwrap().player.energy;
    play(&mut energized, "CARD.MAD_SCIENCE");
    assert_eq!(player(&energized).block, 8);
    // Two energy back, less the one the card cost.
    assert_eq!(
        energized.state().combat.as_ref().unwrap().player.energy,
        energy + 1
    );

    let mut wisdom = tinkered_fight(2, 5);
    play(&mut wisdom, "CARD.MAD_SCIENCE");
    assert_eq!(player(&wisdom).block, 8);
}

/// What a Mad Science *is* is the type the tinker's table wrote on it: an
/// attack is aimed at an enemy and a skill or a power at its owner, a power
/// leaves the state rather than going to the discard, and a filter counting
/// skills counts one.
#[test]
fn a_mad_science_is_whatever_type_the_table_wrote_on_it() {
    let target_of = |simulator: &Simulator| {
        simulator
            .legal_actions()
            .iter()
            .find_map(|action| match action {
                sts2_engine::Action::PlayCard { card, target }
                    if card.fingerprint.model_id == id("CARD.MAD_SCIENCE") =>
                {
                    Some(target.is_some())
                }
                _ => None,
            })
    };
    let mut attack = tinkered_fight(1, 0);
    assert_eq!(target_of(&attack), Some(true), "an attack is aimed");
    let mut skill = tinkered_fight(2, 0);
    assert_eq!(target_of(&skill), Some(false), "a skill is not");
    let mut power = tinkered_fight(3, 0);
    assert_eq!(target_of(&power), Some(false), "and neither is a power");

    play(&mut attack, "CARD.MAD_SCIENCE");
    assert_eq!(
        pile(&attack, PileName::Discard),
        vec!["CARD.MAD_SCIENCE".to_owned()],
        "an attack is put down"
    );
    play(&mut skill, "CARD.MAD_SCIENCE");
    assert_eq!(
        pile(&skill, PileName::Discard),
        vec!["CARD.MAD_SCIENCE".to_owned()],
        "and so is a skill"
    );
    play(&mut power, "CARD.MAD_SCIENCE");
    assert!(
        power
            .state()
            .combat
            .as_ref()
            .expect("the fight is standing")
            .player
            .piles
            .all_cards()
            .next()
            .is_none(),
        "a power leaves the state"
    );
}

#[test]
fn a_mad_science_power_is_nothing_but_its_own_rider() {
    let mut expertise = tinkered_fight(3, 7);
    play(&mut expertise, "CARD.MAD_SCIENCE");
    assert_eq!(player(&expertise).block, 0);
    let powers: Vec<(String, i32)> = player(&expertise)
        .powers
        .iter()
        .map(|power| (power.model_id.to_string(), power.amount))
        .collect();
    assert!(
        powers.contains(&("POWER.STRENGTH_POWER".to_owned(), 2)),
        "{powers:?}"
    );
    assert!(
        powers.contains(&("POWER.DEXTERITY_POWER".to_owned(), 2)),
        "{powers:?}"
    );

    // Curious makes the owner's powers cost one less, never below nothing.
    let mut curious = tinkered_fight(3, 8);
    play(&mut curious, "CARD.MAD_SCIENCE");
    assert!(
        player(&curious)
            .powers
            .iter()
            .any(|power| power.model_id == id("POWER.CURIOUS_POWER") && power.amount == 1)
    );

    let mut improvement = tinkered_fight(3, 9);
    play(&mut improvement, "CARD.MAD_SCIENCE");
    assert!(
        player(&improvement)
            .powers
            .iter()
            .any(|power| power.model_id == id("POWER.IMPROVEMENT_POWER") && power.amount == 1)
    );
}
