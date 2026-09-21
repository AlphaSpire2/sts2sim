//! Every enchantment this baseline registers, pinned one behaviour at a time.
//!
//! No recording carries an enchanted card — every card fingerprint in the whole
//! corpus records `"enchantment": null` — so a scenario is the evidence: what
//! the enchantment may go on, what it does to the card it lands on, what it
//! adds to the card's play, and what it says about the damage, the block, and
//! the number of times the body runs.

use sts2_engine::{
    Action, CardFingerprint, EnchantmentFingerprint, PileName, ScenarioBuilder, Simulator,
};
use sts2_rng::RunStream;

const SEED: &str = "ENCHANTMENTS";

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str, upgrade_level: u8) -> CardFingerprint {
    let mut fingerprint = CardFingerprint::base(id(model_id));
    fingerprint.upgrade_level = upgrade_level;
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

/// The same card with an enchantment on it, worth `amount`.
fn enchanted(model_id: &str, enchantment: &str, amount: i32) -> CardFingerprint {
    let mut fingerprint = card(model_id, 0);
    fingerprint.enchantment = Some(EnchantmentFingerprint::new(id(enchantment), amount));
    fingerprint
}

/// One fight against a single beetle standing on a move that only debuffs, with
/// the hand and draw pile a test names.
fn fight(hand: &[CardFingerprint], draw: &[CardFingerprint]) -> Simulator {
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE");
    for (index, fingerprint) in hand.iter().chain(draw).enumerate() {
        let pile = if index < hand.len() {
            PileName::Hand
        } else {
            PileName::Draw
        };
        scenario = scenario.card(pile, index as u64, fingerprint.clone(), 1);
    }
    scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

/// The same fight with a potion in the belt.
fn fight_with_potion(
    potion: &str,
    hand: &[CardFingerprint],
    draw: &[CardFingerprint],
) -> Simulator {
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .potions(vec![Some(id(potion)), None, None])
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE");
    for (index, fingerprint) in hand.iter().chain(draw).enumerate() {
        let pile = if index < hand.len() {
            PileName::Hand
        } else {
            PileName::Draw
        };
        scenario = scenario.card(pile, index as u64, fingerprint.clone(), 1);
    }
    scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

/// A fight standing before its first player turn, so that ending the turn walks
/// into the opening turn's own setup.
fn opening_fight(draw: &[CardFingerprint]) -> Simulator {
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(0, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE");
    for (index, fingerprint) in draw.iter().enumerate() {
        scenario = scenario.card(PileName::Draw, index as u64, fingerprint.clone(), 1);
    }
    scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

fn drink(simulator: &mut Simulator, model_id: &str) {
    let wanted = id(model_id);
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::UsePotion { model_id, .. } if *model_id == wanted))
        .cloned()
        .expect("the potion is drinkable");
    simulator.step(action).expect("the potion settles");
}

fn play(simulator: &Simulator, model_id: &str) -> Action {
    simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::PlayCard { card, .. }
                if card.fingerprint.model_id == id(model_id))
        })
        .expect("the card is playable")
        .clone()
}

/// Plays the named card. A legal action already carries whatever the card was
/// aimed at, and every scenario here has one enemy to aim at.
fn play_at_enemy(simulator: &mut Simulator, model_id: &str) {
    let action = play(simulator, model_id);
    simulator.step(action).expect("the play settles");
}

fn end_turn(simulator: &mut Simulator) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::EndTurn { .. }))
        .cloned()
        .expect("ending the turn is always legal");
    simulator.step(action).expect("the turn hands over");
}

fn enemy_hp(simulator: &Simulator) -> i32 {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("in a fight")
        .creatures[1]
        .current_hp
}

fn player_block(simulator: &Simulator) -> i32 {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("in a fight")
        .creatures[0]
        .block
}

fn player_hp(simulator: &Simulator) -> i32 {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("in a fight")
        .creatures[0]
        .current_hp
}

fn hand_size(simulator: &Simulator) -> usize {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("in a fight")
        .player
        .piles
        .hand
        .len()
}

/// A Strike deals six and a Defend works up five, which is what every
/// difference below is measured against.
#[test]
fn the_plain_cards_the_enchantments_are_measured_against() {
    let mut simulator = fight(&[card("CARD.STRIKE_IRONCLAD", 0)], &[]);
    play_at_enemy(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(enemy_hp(&simulator), 33, "a plain Strike deals six");

    let mut simulator = fight(&[card("CARD.DEFEND_IRONCLAD", 0)], &[]);
    simulator
        .step(play(&simulator, "CARD.DEFEND_IRONCLAD"))
        .expect("the play settles");
    assert_eq!(player_block(&simulator), 5, "a plain Defend works up five");
}

#[test]
fn an_enchantment_is_projected_beside_its_card_as_an_object() {
    let simulator = fight(
        &[enchanted("CARD.STRIKE_IRONCLAD", "ENCHANTMENT.SHARP", 3)],
        &[],
    );
    let observation = sts2_replay::observation::render(&simulator);
    let deck = observation
        .pointer("/players/0/deck/0/enchantment")
        .expect("the deck card renders its enchantment");
    assert_eq!(
        deck.get("model_id").and_then(serde_json::Value::as_str),
        Some("ENCHANTMENT.SHARP")
    );
    assert_eq!(
        deck.get("amount").and_then(serde_json::Value::as_i64),
        Some(3),
        "the amount is the enchantment's own state and the recording carries it"
    );
    assert!(
        deck.get("properties")
            .is_some_and(serde_json::Value::is_null),
        "no pinned enchantment declares a saved property"
    );
}

#[test]
fn sharp_adds_its_amount_to_the_attack() {
    let mut simulator = fight(
        &[enchanted("CARD.STRIKE_IRONCLAD", "ENCHANTMENT.SHARP", 3)],
        &[],
    );
    play_at_enemy(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(enemy_hp(&simulator), 30, "six and the three it adds");
}

#[test]
fn instinct_doubles_the_attack() {
    let mut simulator = fight(
        &[enchanted("CARD.STRIKE_IRONCLAD", "ENCHANTMENT.INSTINCT", 0)],
        &[],
    );
    play_at_enemy(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(enemy_hp(&simulator), 27, "twelve rather than six");
}

#[test]
fn corrupted_scales_the_attack_by_half_again_and_bleeds_its_owner() {
    let mut simulator = fight(
        &[enchanted(
            "CARD.STRIKE_IRONCLAD",
            "ENCHANTMENT.CORRUPTED",
            0,
        )],
        &[],
    );
    play_at_enemy(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(enemy_hp(&simulator), 30, "nine rather than six");
    assert_eq!(
        player_hp(&simulator),
        78,
        "and two unblockable to whoever played it, every play"
    );
}

#[test]
fn tezcataras_ember_makes_the_card_free_and_adds_three() {
    let mut simulator = fight(
        &[enchanted(
            "CARD.STRIKE_IRONCLAD",
            "ENCHANTMENT.TEZCATARAS_EMBER",
            0,
        )],
        &[],
    );
    let energy = simulator
        .state()
        .combat
        .as_ref()
        .expect("in a fight")
        .player
        .energy;
    play_at_enemy(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(enemy_hp(&simulator), 30, "six and the three it adds");
    assert_eq!(
        simulator
            .state()
            .combat
            .as_ref()
            .expect("in a fight")
            .player
            .energy,
        energy,
        "and the card costs nothing at all"
    );
}

#[test]
fn vigorous_adds_its_amount_once_a_fight() {
    let mut simulator = fight(
        &[
            enchanted("CARD.STRIKE_IRONCLAD", "ENCHANTMENT.VIGOROUS", 4),
            card("CARD.STRIKE_IRONCLAD", 0),
        ],
        &[],
    );
    play_at_enemy(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(enemy_hp(&simulator), 29, "six and the four it adds");
    play_at_enemy(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(
        enemy_hp(&simulator),
        23,
        "the play that got it spent it, so the next Strike is plain"
    );
}

#[test]
fn momentum_grows_the_attack_by_its_amount_every_play() {
    // A soldier's stew makes every Strike in the fight run its body twice, so
    // one play is two plays: the build's loop runs the card's body, then the
    // enchantment's, then round again — and the second body sees what the
    // first one wrote.
    let mut simulator = fight_with_potion(
        "POTION.SOLDIERS_STEW",
        &[enchanted("CARD.STRIKE_IRONCLAD", "ENCHANTMENT.MOMENTUM", 2)],
        &[],
    );
    drink(&mut simulator, "POTION.SOLDIERS_STEW");
    play_at_enemy(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(
        enemy_hp(&simulator),
        25,
        "six for the first body and eight for the second: the damage is worked \
         out before each body runs, so the first play adds nothing to itself"
    );
    let discarded = simulator
        .state()
        .combat
        .as_ref()
        .expect("in a fight")
        .player
        .piles
        .discard
        .iter()
        .find(|card| card.fingerprint.model_id == id("CARD.STRIKE_IRONCLAD"))
        .expect("the Strike was put down")
        .clone();
    assert_eq!(
        discarded.enchantment_locals.get("ExtraDamage"),
        Some(&4),
        "and it has four written down for the next fight's worth of plays"
    );
}

#[test]
fn nimble_adds_its_amount_to_the_block() {
    let mut simulator = fight(
        &[enchanted("CARD.DEFEND_IRONCLAD", "ENCHANTMENT.NIMBLE", 4)],
        &[],
    );
    simulator
        .step(play(&simulator, "CARD.DEFEND_IRONCLAD"))
        .expect("the play settles");
    assert_eq!(player_block(&simulator), 9, "five and the four it adds");
}

#[test]
fn adroit_works_up_block_worth_its_amount_every_play() {
    let mut simulator = fight(
        &[enchanted("CARD.STRIKE_IRONCLAD", "ENCHANTMENT.ADROIT", 3)],
        &[],
    );
    play_at_enemy(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(enemy_hp(&simulator), 33, "the attack is untouched");
    assert_eq!(player_block(&simulator), 3, "and the play works up three");
}

#[test]
fn goopy_burns_the_defend_it_is_on_and_grows_a_block_a_play() {
    let mut simulator = fight(
        &[
            enchanted("CARD.DEFEND_IRONCLAD", "ENCHANTMENT.GOOPY", 1),
            enchanted("CARD.DEFEND_IRONCLAD", "ENCHANTMENT.GOOPY", 1),
        ],
        &[],
    );
    simulator
        .step(play(&simulator, "CARD.DEFEND_IRONCLAD"))
        .expect("the play settles");
    assert_eq!(
        player_block(&simulator),
        5,
        "an amount of one adds nothing to the first play"
    );
    let combat = simulator.state().combat.as_ref().expect("in a fight");
    assert!(
        combat.player.piles.discard.is_empty(),
        "the Defend burned up rather than being put down"
    );
    assert_eq!(combat.player.piles.exhaust.len(), 1);
    assert_eq!(
        simulator.state().run_player.deck[0]
            .enchantment
            .as_ref()
            .expect("the deck card carries it")
            .amount,
        2,
        "and the growth is written onto the run-deck card as well"
    );
}

#[test]
fn inky_adds_one_damage_and_weakens_what_the_play_reached() {
    let mut simulator = fight(
        &[enchanted("CARD.STRIKE_IRONCLAD", "ENCHANTMENT.INKY", 0)],
        &[],
    );
    play_at_enemy(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(enemy_hp(&simulator), 32, "seven rather than six");
    let combat = simulator.state().combat.as_ref().expect("in a fight");
    assert!(
        combat.creatures[1]
            .powers
            .iter()
            .any(|power| power.model_id == id("POWER.WEAK_POWER") && power.amount == 1),
        "and one Weak on whoever the play was aimed at"
    );
}

#[test]
fn sown_pays_its_amount_in_energy_once() {
    let mut simulator = fight(
        &[
            enchanted("CARD.STRIKE_IRONCLAD", "ENCHANTMENT.SOWN", 2),
            card("CARD.STRIKE_IRONCLAD", 0),
        ],
        &[],
    );
    let before = simulator
        .state()
        .combat
        .as_ref()
        .expect("in a fight")
        .player
        .energy;
    play_at_enemy(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(
        simulator
            .state()
            .combat
            .as_ref()
            .expect("in a fight")
            .player
            .energy,
        before - 1 + 2,
        "the play costs one and pays two back"
    );
}

#[test]
fn swift_draws_its_amount_once() {
    let mut simulator = fight(
        &[enchanted("CARD.STRIKE_IRONCLAD", "ENCHANTMENT.SWIFT", 2)],
        &[
            card("CARD.DEFEND_IRONCLAD", 0),
            card("CARD.DEFEND_IRONCLAD", 0),
        ],
    );
    assert_eq!(hand_size(&simulator), 1);
    play_at_enemy(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(hand_size(&simulator), 2, "the play drew the two it owes");
}

#[test]
fn spiral_runs_a_basic_cards_body_once_more_every_time() {
    let mut simulator = fight(
        &[enchanted("CARD.STRIKE_IRONCLAD", "ENCHANTMENT.SPIRAL", 0)],
        &[],
    );
    play_at_enemy(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(enemy_hp(&simulator), 27, "six twice over");
}

#[test]
fn glam_replays_the_first_play_of_the_fight_and_spends_itself() {
    let mut simulator = fight(
        &[
            enchanted("CARD.STRIKE_IRONCLAD", "ENCHANTMENT.GLAM", 0),
            enchanted("CARD.STRIKE_IRONCLAD", "ENCHANTMENT.GLAM", 0),
        ],
        &[],
    );
    play_at_enemy(&mut simulator, "CARD.STRIKE_IRONCLAD");
    assert_eq!(enemy_hp(&simulator), 27, "the first play lands twice");
    let spent = simulator
        .state()
        .combat
        .as_ref()
        .expect("in a fight")
        .player
        .piles
        .discard[0]
        .clone();
    assert_eq!(spent.enchantment_locals.get("UsedThisCombat"), Some(&1));
    assert_eq!(
        spent.enchantment_status,
        sts2_engine::EnchantmentStatus::Disabled
    );
}

#[test]
fn steady_keeps_the_card_through_the_flush() {
    let mut simulator = fight(
        &[enchanted("CARD.STRIKE_IRONCLAD", "ENCHANTMENT.STEADY", 0)],
        &[],
    );
    end_turn(&mut simulator);
    assert!(
        simulator
            .state()
            .combat
            .as_ref()
            .expect("in a fight")
            .player
            .piles
            .hand
            .iter()
            .any(|card| card.fingerprint.model_id == id("CARD.STRIKE_IRONCLAD")),
        "a retained card is not put down"
    );
}

#[test]
fn royally_approved_makes_the_card_innate_and_retained() {
    let registry = sts2_content::standard_registry();
    let fingerprint = enchanted("CARD.STRIKE_IRONCLAD", "ENCHANTMENT.ROYALLY_APPROVED", 0);
    assert!(registry.card_has_keyword(&fingerprint, sts2_engine::CardKeyword::Innate));
    assert!(registry.card_has_keyword(&fingerprint, sts2_engine::CardKeyword::Retain));
    let plain = card("CARD.STRIKE_IRONCLAD", 0);
    assert!(!registry.card_has_keyword(&plain, sts2_engine::CardKeyword::Innate));
    assert!(!registry.card_has_keyword(&plain, sts2_engine::CardKeyword::Retain));
}

#[test]
fn souls_power_stops_the_card_burning_up() {
    let registry = sts2_content::standard_registry();
    // Molten Fist is an Ironclad attack whose own keyword is Exhaust, which is
    // what the Souls enchantment wants and what it takes away as it goes on.
    let plain = card("CARD.MOLTEN_FIST", 0);
    assert!(registry.card_has_keyword(&plain, sts2_engine::CardKeyword::Exhaust));
    let freed = enchanted("CARD.MOLTEN_FIST", "ENCHANTMENT.SOULS_POWER", 0);
    assert!(!registry.card_has_keyword(&freed, sts2_engine::CardKeyword::Exhaust));

    let mut simulator = fight(&[freed], &[]);
    play_at_enemy(&mut simulator, "CARD.MOLTEN_FIST");
    let combat = simulator.state().combat.as_ref().expect("in a fight");
    assert!(
        combat.player.piles.exhaust.is_empty(),
        "nothing burned up this play"
    );
    assert_eq!(combat.player.piles.discard.len(), 1);
}

#[test]
fn tezcataras_ember_stops_the_card_leaving_the_deck() {
    let registry = sts2_content::standard_registry();
    let fingerprint = enchanted("CARD.STRIKE_IRONCLAD", "ENCHANTMENT.TEZCATARAS_EMBER", 0);
    assert!(registry.card_has_keyword(&fingerprint, sts2_engine::CardKeyword::Eternal));
}

#[test]
fn slumbering_essence_lowers_the_cost_for_every_turn_the_card_was_held() {
    let mut simulator = fight(
        &[enchanted(
            "CARD.STRIKE_IRONCLAD",
            "ENCHANTMENT.SLUMBERING_ESSENCE",
            0,
        )],
        &[],
    );
    let cost = |simulator: &Simulator| {
        simulator
            .state()
            .combat
            .as_ref()
            .expect("in a fight")
            .player
            .piles
            .hand
            .iter()
            .find(|card| card.fingerprint.model_id == id("CARD.STRIKE_IRONCLAD"))
            .map(|card| card.energy_cost)
    };
    assert_eq!(cost(&simulator), Some(1), "a Strike costs one to start");
    end_turn(&mut simulator);
    // The card is not retained, so it goes down with the hand; what the
    // enchantment wrote onto it stands until the card is played.
    let held = simulator
        .state()
        .combat
        .as_ref()
        .expect("in a fight")
        .player
        .piles
        .all_cards()
        .find(|card| card.fingerprint.model_id == id("CARD.STRIKE_IRONCLAD"))
        .expect("the card is somewhere")
        .clone();
    assert_eq!(
        held.cost_until_played, -1,
        "one turn held in hand is worth one off its cost"
    );
}

#[test]
fn slither_rolls_the_cards_cost_as_it_is_drawn() {
    let mut simulator = fight(
        &[],
        &[enchanted("CARD.STRIKE_IRONCLAD", "ENCHANTMENT.SLITHER", 0)],
    );
    let before = simulator
        .state()
        .rng
        .stream(RunStream::CombatEnergyCosts)
        .counter();
    end_turn(&mut simulator);
    let after = simulator
        .state()
        .rng
        .stream(RunStream::CombatEnergyCosts)
        .counter();
    assert!(
        after > before,
        "the draw rolled a cost on the combat energy-cost stream"
    );
    let drawn = simulator
        .state()
        .combat
        .as_ref()
        .expect("in a fight")
        .player
        .piles
        .hand
        .iter()
        .find(|card| card.fingerprint.model_id == id("CARD.STRIKE_IRONCLAD"))
        .expect("the card was drawn")
        .clone();
    assert!(
        (0..4).contains(&drawn.energy_cost),
        "and the cost it settled on is one of the four the build rolls"
    );
    assert!(
        drawn
            .temporary_costs
            .iter()
            .any(|cost| !cost.clears_when_turn_ends && !cost.clears_when_played),
        "the roll stands for the rest of the fight"
    );
}

#[test]
fn imbued_sinks_the_skill_and_plays_it_on_the_opening_turn() {
    // The scenario stands the fight up before its first player turn, so ending
    // the turn walks into the opening turn's own setup: the draw pile is sorted
    // and the auto-play phase runs.
    let mut simulator = opening_fight(&[
        card("CARD.DEFEND_IRONCLAD", 0),
        enchanted("CARD.DEFEND_IRONCLAD", "ENCHANTMENT.IMBUED", 0),
        card("CARD.STRIKE_IRONCLAD", 0),
    ]);
    end_turn(&mut simulator);
    let combat = simulator.state().combat.as_ref().expect("in a fight");
    assert_eq!(combat.player.turn, 1, "this is the opening turn");
    assert!(
        combat.player.piles.discard.iter().any(|card| card
            .fingerprint
            .enchantment
            .as_ref()
            .is_some_and(|enchantment| enchantment.model_id == id("ENCHANTMENT.IMBUED"))),
        "the enchanted skill played itself and was put down"
    );
    assert!(
        player_block(&simulator) >= 5,
        "and the block it works up landed"
    );
}

#[test]
fn perfect_fit_puts_its_card_on_top_after_a_shuffle() {
    // Everything is in the discard pile and the draw pile is empty, so the next
    // turn's draw shuffles first. The enchantment moves its card to the front
    // of what the shuffle settled on, which makes it the first card dealt.
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE");
    for index in 0..8u64 {
        let fingerprint = if index == 5 {
            enchanted("CARD.STRIKE_IRONCLAD", "ENCHANTMENT.PERFECT_FIT", 0)
        } else {
            card("CARD.DEFEND_IRONCLAD", 0)
        };
        scenario = scenario.card(PileName::Discard, index, fingerprint, 1);
    }
    let mut simulator = scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    end_turn(&mut simulator);
    let hand = &simulator
        .state()
        .combat
        .as_ref()
        .expect("in a fight")
        .player
        .piles
        .hand;
    assert_eq!(
        hand[0].fingerprint.model_id,
        id("CARD.STRIKE_IRONCLAD"),
        "the enchanted card was on top of the shuffled pile, so it was dealt first"
    );
}

#[test]
fn clone_and_the_deprecated_enchantment_do_nothing_of_their_own() {
    for enchantment in ["ENCHANTMENT.CLONE", "ENCHANTMENT.DEPRECATED_ENCHANTMENT"] {
        let mut simulator = fight(&[enchanted("CARD.STRIKE_IRONCLAD", enchantment, 0)], &[]);
        play_at_enemy(&mut simulator, "CARD.STRIKE_IRONCLAD");
        assert_eq!(
            enemy_hp(&simulator),
            33,
            "{enchantment} changes nothing about the play"
        );
        assert_eq!(player_block(&simulator), 0);
    }
}

/// Whether an enchantment can go on a card: the base test every enchantment
/// shares, and the test each one adds on top of it.
#[test]
fn can_enchant_refuses_exactly_what_the_build_refuses() {
    let registry = sts2_content::standard_registry();
    let can = |enchantment: &str, model: &str| {
        registry.can_enchant(
            registry
                .enchantment(&id(enchantment))
                .expect("the enchantment is registered"),
            &card(model, 0),
            true,
        )
    };

    // The base test: a status, a curse, and a quest card are all out.
    assert!(!can("ENCHANTMENT.SWIFT", "CARD.BURN"));
    assert!(!can("ENCHANTMENT.SWIFT", "CARD.CLUMSY"));
    assert!(!can("ENCHANTMENT.SWIFT", "CARD.BYRDONIS_EGG"));
    assert!(can("ENCHANTMENT.SWIFT", "CARD.STRIKE_IRONCLAD"));

    // A card that already carries one refuses every enchantment, because
    // nothing in the pinned build is stackable.
    assert!(
        !registry.can_enchant(
            registry
                .enchantment(&id("ENCHANTMENT.SWIFT"))
                .expect("registered"),
            &enchanted("CARD.STRIKE_IRONCLAD", "ENCHANTMENT.SHARP", 1),
            true,
        )
    );

    // The card type each enchantment will go on.
    assert!(can("ENCHANTMENT.SHARP", "CARD.STRIKE_IRONCLAD"));
    assert!(!can("ENCHANTMENT.SHARP", "CARD.DEFEND_IRONCLAD"));
    assert!(can("ENCHANTMENT.IMBUED", "CARD.DEFEND_IRONCLAD"));
    assert!(!can("ENCHANTMENT.IMBUED", "CARD.STRIKE_IRONCLAD"));
    assert!(can("ENCHANTMENT.ROYALLY_APPROVED", "CARD.STRIKE_IRONCLAD"));
    assert!(can("ENCHANTMENT.ROYALLY_APPROVED", "CARD.DEFEND_IRONCLAD"));
    assert!(!can("ENCHANTMENT.ROYALLY_APPROVED", "CARD.DEMON_FORM"));

    // The tests the individual models add.
    assert!(can("ENCHANTMENT.GOOPY", "CARD.DEFEND_IRONCLAD"));
    assert!(!can("ENCHANTMENT.GOOPY", "CARD.STRIKE_IRONCLAD"));
    assert!(can("ENCHANTMENT.NIMBLE", "CARD.DEFEND_IRONCLAD"));
    assert!(!can("ENCHANTMENT.NIMBLE", "CARD.STRIKE_IRONCLAD"));
    // Whether a card gains block is declared card by card rather than worked
    // out from what the card does, so one card of every character that
    // declares it is asked here.
    for model_id in [
        "CARD.DEFEND_SILENT",
        "CARD.DEFEND_REGENT",
        "CARD.DEFEND_NECROBINDER",
        "CARD.DEFEND_DEFECT",
        "CARD.GLACIER",
        "CARD.GENETIC_ALGORITHM",
        "CARD.HOLOGRAM",
        "CARD.LEAP",
    ] {
        assert!(can("ENCHANTMENT.NIMBLE", model_id), "{model_id}");
    }
    for model_id in ["CARD.STRIKE_DEFECT", "CARD.ZAP", "CARD.DUALCAST"] {
        assert!(!can("ENCHANTMENT.NIMBLE", model_id), "{model_id}");
    }
    assert!(can("ENCHANTMENT.SOULS_POWER", "CARD.MOLTEN_FIST"));
    assert!(!can("ENCHANTMENT.SOULS_POWER", "CARD.STRIKE_IRONCLAD"));
    assert!(can("ENCHANTMENT.SPIRAL", "CARD.STRIKE_IRONCLAD"));
    assert!(can("ENCHANTMENT.SPIRAL", "CARD.DEFEND_IRONCLAD"));
    assert!(!can("ENCHANTMENT.SPIRAL", "CARD.MOLTEN_FIST"));
    assert!(can("ENCHANTMENT.SLITHER", "CARD.STRIKE_IRONCLAD"));
    assert!(
        !can("ENCHANTMENT.SLITHER", "CARD.CASCADE"),
        "an X-cost card has no cost to roll"
    );
}

/// An unplayable card is refused only where it is standing in the run deck,
/// which is the one half of the base test that asks where the card is.
#[test]
fn an_unplayable_card_is_refused_only_in_the_deck() {
    let registry = sts2_content::standard_registry();
    let enchantment = registry
        .enchantment(&id("ENCHANTMENT.SWIFT"))
        .expect("registered");
    // Wound is a status, so the type test refuses it wherever it stands; Writhe
    // is a curse for the same reason. What is left is a playable card, which
    // both halves accept.
    let strike = card("CARD.STRIKE_IRONCLAD", 0);
    assert!(registry.can_enchant(enchantment, &strike, true));
    assert!(registry.can_enchant(enchantment, &strike, false));
}
