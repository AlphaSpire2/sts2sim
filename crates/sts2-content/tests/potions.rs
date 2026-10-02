//! Every potion this baseline registers, pinned one behaviour at a time.
//!
//! No recording drinks most of them, so a scenario is the evidence: what the
//! belt offers, what drinking one does, and — where a potion rolls — which
//! stream it moves and by how much.

use sts2_engine::{
    Action, CardFingerprint, DecisionContext, PileName, ScenarioBuilder, Simulator, TargetHandle,
};
use sts2_rng::RunStream;

const SEED: &str = "POTIONS";

#[test]
fn a_potion_choice_does_not_suspend_back_attack_intent_modifiers() {
    use sts2_engine::{Effect, PowerType};

    for (facing, crusher_damage, rocket_damage) in [(0, 18, 3), (1, 12, 4)] {
        let simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.KAISER_CRAB_BOSS"))
            .player_hp(80, 80)
            .energy(3, 3)
            .potions(vec![Some(id("POTION.ATTACK_POTION"))])
            .enemy(1, id("MONSTER.CRUSHER"), 209, 209, "THRASH_MOVE")
            .enemy(2, id("MONSTER.ROCKET"), 199, 199, "TARGETING_RETICLE_MOVE")
            .power(0, id("POWER.SURROUNDED_POWER"), 1, PowerType::Debuff)
            .power(1, id("POWER.BACK_ATTACK_LEFT_POWER"), 1, PowerType::Buff)
            .power(2, id("POWER.BACK_ATTACK_RIGHT_POWER"), 1, PowerType::Buff)
            .build(sts2_content::standard_registry())
            .unwrap();
        let mut state = simulator.state().clone();
        state.combat.as_mut().unwrap().creatures[0].powers[0].counter = facing;
        let mut simulator =
            Simulator::from_scenario(state, sts2_content::standard_registry()).unwrap();
        drink(&mut simulator, "POTION.ATTACK_POTION");
        let combat = simulator.state().combat.as_ref().unwrap();
        assert!(combat.suspended_choice.is_some());
        assert_eq!(combat.creatures[0].powers[0].counter, facing);
        for (creature, damage) in [(1, crusher_damage), (2, rocket_damage)] {
            assert_eq!(
                combat.creatures[creature].intent.as_ref().unwrap().intents[0].damage,
                Some(damage)
            );
        }
        assert!(
            !combat
                .queue
                .iter()
                .any(|effect| matches!(effect, Effect::RunRemaining(_))),
            "a damage preview must not add continuations to the potion's queue"
        );
    }
}

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str, upgrade_level: u8) -> CardFingerprint {
    let mut fingerprint = CardFingerprint::base(id(model_id));
    fingerprint.upgrade_level = upgrade_level;
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

/// One fight against a single beetle, with the belt and the piles a test names.
fn fight(potions: &[&str], hand: &[(&str, u8)], draw: &[(&str, u8)]) -> Simulator {
    fight_with_enemies(1, potions, hand, draw)
}

fn fight_with_enemies(
    enemies: u64,
    potions: &[&str],
    hand: &[(&str, u8)],
    draw: &[(&str, u8)],
) -> Simulator {
    fight_against("SHRINKER_MOVE", enemies, potions, hand, draw)
}

/// The same fight with the beetle already standing on the move a test needs:
/// its opening state is a debuff, and a test about taking a blow wants the
/// chomp.
fn fight_against(
    move_state: &str,
    enemies: u64,
    potions: &[&str],
    hand: &[(&str, u8)],
    draw: &[(&str, u8)],
) -> Simulator {
    let mut belt: Vec<Option<sts2_core::ModelId>> =
        potions.iter().map(|model| Some(id(model))).collect();
    while belt.len() < 3 {
        belt.push(None);
    }
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .potions(belt)
        .rng_counter(RunStream::MonsterAi, 0)
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

/// Drinks the potion in the first slot that holds it, at whatever the action
/// offers, preferring the first target the belt lays out.
fn drink(simulator: &mut Simulator, model_id: &str) {
    drink_at(simulator, model_id, None);
}

fn drink_at(simulator: &mut Simulator, model_id: &str, target: Option<u64>) {
    let wanted = id(model_id);
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| match action {
            Action::UsePotion {
                model_id,
                target: at,
                ..
            } => {
                *model_id == wanted
                    && match target {
                        Some(combat_id) => at
                            .as_ref()
                            .is_some_and(|at| at.combat_id.get() == combat_id),
                        None => true,
                    }
            }
            _ => false,
        })
        .cloned()
        .unwrap_or_else(|| panic!("{model_id} was not offered"));
    simulator.step(action).expect("the potion applies");
}

fn choose(simulator: &mut Simulator, models: &[&str]) {
    let wanted: Vec<sts2_core::ModelId> = models.iter().map(|model| id(model)).collect();
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| match action {
            Action::ChooseCards { cards, .. } => {
                cards.len() == wanted.len()
                    && cards
                        .iter()
                        .zip(&wanted)
                        .all(|(card, want)| card.fingerprint.model_id == *want)
            }
            _ => false,
        })
        .cloned()
        .unwrap_or_else(|| panic!("{models:?} was not on the screen"));
    simulator.step(action).expect("the choice applies");
}

fn choose_nothing(simulator: &mut Simulator) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseCards { cards, .. } if cards.is_empty()))
        .cloned()
        .expect("taking nothing is offered");
    simulator.step(action).expect("the choice applies");
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

fn belt(simulator: &Simulator) -> Vec<Option<String>> {
    simulator
        .state()
        .run_player
        .potions
        .iter()
        .map(|held| held.as_ref().map(ToString::to_string))
        .collect()
}

fn counter(simulator: &Simulator, stream: RunStream) -> u64 {
    simulator.state().rng.stream(stream).counter()
}

// --- the generation pool -------------------------------------------------

/// A run's potion options are the character's pool followed by the shared one,
/// and every roll filters that one list by rarity. The order is
/// what a reward, a shop, and a mid-fight roll all index into, so it is pinned
/// whole.
#[test]
fn the_generation_pool_is_the_character_pool_then_the_shared_one() {
    let content = sts2_content::standard_registry();
    let [common, uncommon, rare] =
        content.potion_options(&"CHARACTER.IRONCLAD".parse().unwrap(), false);
    let names = |models: &[sts2_core::ModelId]| -> Vec<String> {
        models.iter().map(ToString::to_string).collect()
    };
    assert_eq!(
        names(&common),
        [
            "POTION.BLOOD_POTION",
            "POTION.ATTACK_POTION",
            "POTION.BLOCK_POTION",
            "POTION.COLORLESS_POTION",
            "POTION.DEXTERITY_POTION",
            "POTION.ENERGY_POTION",
            "POTION.EXPLOSIVE_AMPOULE",
            "POTION.FIRE_POTION",
            "POTION.FLEX_POTION",
            "POTION.POWER_POTION",
            "POTION.SKILL_POTION",
            "POTION.SPEED_POTION",
            "POTION.STRENGTH_POTION",
            "POTION.SWIFT_POTION",
            "POTION.VULNERABLE_POTION",
            "POTION.WEAK_POTION",
        ]
    );
    assert_eq!(
        names(&uncommon),
        [
            "POTION.ASHWATER",
            "POTION.BLESSING_OF_THE_FORGE",
            "POTION.CLARITY",
            "POTION.CURE_ALL",
            "POTION.DUPLICATOR",
            "POTION.FORTIFIER",
            "POTION.FYSH_OIL",
            "POTION.GAMBLERS_BREW",
            "POTION.HEART_OF_IRON",
            "POTION.LIQUID_BRONZE",
            "POTION.POTION_OF_BINDING",
            "POTION.POWDERED_DEMISE",
            "POTION.RADIANT_TINCTURE",
            "POTION.REGEN_POTION",
            "POTION.STABLE_SERUM",
            "POTION.TOUCH_OF_INSANITY",
        ]
    );
    assert_eq!(
        names(&rare),
        [
            "POTION.SOLDIERS_STEW",
            "POTION.BEETLE_JUICE",
            "POTION.BOTTLED_POTENTIAL",
            "POTION.DISTILLED_CHAOS",
            "POTION.DROPLET_OF_PRECOGNITION",
            "POTION.ENTROPIC_BREW",
            "POTION.FAIRY_IN_A_BOTTLE",
            "POTION.FRUIT_JUICE",
            "POTION.GIGANTIFICATION_POTION",
            "POTION.LIQUID_MEMORIES",
            "POTION.LUCKY_TONIC",
            "POTION.MAZALETHS_GIFT",
            "POTION.OROBIC_ACID",
            "POTION.SHACKLING_POTION",
            "POTION.SHIP_IN_A_BOTTLE",
            "POTION.SNECKO_OIL",
        ]
    );
}

/// A potion conjured mid-fight drops what may not be made in a fight, and
/// drops it from that roll alone: three potions say they cannot be, and the
/// out-of-combat options still hold all three.
#[test]
fn the_in_combat_roll_passes_over_what_may_not_be_conjured_in_a_fight() {
    let content = sts2_content::standard_registry();
    let held_back = [
        "POTION.REGEN_POTION",
        "POTION.FRUIT_JUICE",
        "POTION.FAIRY_IN_A_BOTTLE",
    ];
    let all_of = |options: [Vec<sts2_core::ModelId>; 3]| -> Vec<String> {
        options
            .into_iter()
            .flatten()
            .map(|model| model.to_string())
            .collect()
    };
    let out_of_combat =
        all_of(content.potion_options(&"CHARACTER.IRONCLAD".parse().unwrap(), false));
    let in_combat = all_of(content.potion_options(&"CHARACTER.IRONCLAD".parse().unwrap(), true));
    for model in held_back {
        assert!(out_of_combat.contains(&model.to_owned()), "{model}");
        assert!(!in_combat.contains(&model.to_owned()), "{model}");
    }
    assert_eq!(out_of_combat.len(), in_combat.len() + held_back.len());
}

/// An alchemize rolls one potion on the combat potion-generation stream and
/// procures it: one float for the rarity and one index for the potion, which is
/// two draws off that stream and nothing off any other.
#[test]
fn alchemize_rolls_one_potion_on_the_potion_stream() {
    let mut simulator = fight(&[], &[("CARD.ALCHEMIZE", 0)], &[]);
    assert_eq!(counter(&simulator, RunStream::CombatPotionGeneration), 0);
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::PlayCard { card, .. }
                if card.fingerprint.model_id == id("CARD.ALCHEMIZE"))
        })
        .cloned()
        .expect("alchemize is playable");
    simulator.step(action).expect("the play applies");
    assert_eq!(counter(&simulator, RunStream::CombatPotionGeneration), 2);
    assert_eq!(
        belt(&simulator),
        [Some("POTION.ENERGY_POTION".to_owned()), None, None]
    );
    // The card burns up for having been played.
    assert_eq!(pile(&simulator, PileName::Exhaust), ["CARD.ALCHEMIZE"]);
}

/// An entropic brew fills whatever is left of the belt, one roll per free slot,
/// and the slot it was drunk out of is one of them.
#[test]
fn an_entropic_brew_fills_every_free_slot() {
    let mut simulator = fight(&["POTION.ENTROPIC_BREW", "POTION.FIRE_POTION"], &[], &[]);
    drink(&mut simulator, "POTION.ENTROPIC_BREW");
    // Two slots were free once the brew left its own: two rolls, two potions.
    assert_eq!(counter(&simulator, RunStream::CombatPotionGeneration), 4);
    let held = belt(&simulator);
    assert!(held.iter().all(Option::is_some), "{held:?}");
    assert_eq!(held[1], Some("POTION.FIRE_POTION".to_owned()));
}

// --- what a potion works on ----------------------------------------------

/// The plain buff potions: each works its power on whoever it was aimed at, and
/// clears its own slot doing it.
#[test]
fn the_buff_potions_work_their_power_on_the_drinker() {
    for (potion, power, amount) in [
        ("POTION.STRENGTH_POTION", "POWER.STRENGTH_POWER", 2),
        ("POTION.DEXTERITY_POTION", "POWER.DEXTERITY_POWER", 2),
        ("POTION.REGEN_POTION", "POWER.REGEN_POWER", 5),
        ("POTION.LIQUID_BRONZE", "POWER.THORNS_POWER", 3),
        ("POTION.HEART_OF_IRON", "POWER.PLATING_POWER", 7),
        ("POTION.LUCKY_TONIC", "POWER.BUFFER_POWER", 1),
        ("POTION.MAZALETHS_GIFT", "POWER.RITUAL_POWER", 1),
        ("POTION.STABLE_SERUM", "POWER.RETAIN_HAND_POWER", 2),
        (
            "POTION.GIGANTIFICATION_POTION",
            "POWER.GIGANTIFICATION_POWER",
            1,
        ),
        ("POTION.DUPLICATOR", "POWER.DUPLICATION_POWER", 1),
    ] {
        let mut simulator = fight(&[potion], &[], &[]);
        drink(&mut simulator, potion);
        assert_eq!(power_amount(&simulator, 0, power), amount, "{potion}");
        assert_eq!(belt(&simulator), [None, None, None], "{potion}");
    }
}

/// A fysh oil is two powers off one drink, strength before dexterity.
#[test]
fn a_fysh_oil_works_both_stats_on_at_once() {
    let mut simulator = fight(&["POTION.FYSH_OIL"], &[], &[]);
    drink(&mut simulator, "POTION.FYSH_OIL");
    assert_eq!(power_amount(&simulator, 0, "POWER.STRENGTH_POWER"), 1);
    assert_eq!(power_amount(&simulator, 0, "POWER.DEXTERITY_POWER"), 1);
}

/// The debuff potions land on whatever they were thrown at, and the two that
/// sweep land on everything still standing.
#[test]
fn the_debuff_potions_land_where_they_are_thrown() {
    for (potion, power, amount) in [
        ("POTION.WEAK_POTION", "POWER.WEAK_POWER", 3),
        ("POTION.VULNERABLE_POTION", "POWER.VULNERABLE_POWER", 3),
        ("POTION.POWDERED_DEMISE", "POWER.DEMISE_POWER", 9),
        ("POTION.BEETLE_JUICE", "POWER.SHRINK_POWER", 4),
    ] {
        let mut simulator = fight_with_enemies(2, &[potion], &[], &[]);
        drink_at(&mut simulator, potion, Some(1));
        assert_eq!(power_amount(&simulator, 1, power), amount, "{potion}");
        assert_eq!(power_amount(&simulator, 2, power), 0, "{potion}");
    }
    let mut simulator = fight_with_enemies(2, &["POTION.POTION_OF_BINDING"], &[], &[]);
    drink(&mut simulator, "POTION.POTION_OF_BINDING");
    for enemy in [1, 2] {
        assert_eq!(power_amount(&simulator, enemy, "POWER.WEAK_POWER"), 1);
        assert_eq!(power_amount(&simulator, enemy, "POWER.VULNERABLE_POWER"), 1);
    }
    let mut simulator = fight_with_enemies(2, &["POTION.SHACKLING_POTION"], &[], &[]);
    drink(&mut simulator, "POTION.SHACKLING_POTION");
    for enemy in [1, 2] {
        assert_eq!(power_amount(&simulator, enemy, "POWER.STRENGTH_POWER"), -7);
    }
}

/// Demise costs its owner its whole amount as their side's turn ends, and block
/// does not stop it.
#[test]
fn demise_costs_its_owner_its_amount_at_the_end_of_their_turn() {
    let mut simulator = fight_with_enemies(1, &["POTION.POWDERED_DEMISE"], &[], &[]);
    drink_at(&mut simulator, "POTION.POWDERED_DEMISE", Some(1));
    let before = creature(&simulator, 1).current_hp;
    end_turn(&mut simulator);
    assert_eq!(creature(&simulator, 1).current_hp, before - 9);
}

/// The damage potions deal their own damage, which nothing scales: a fire
/// potion at one enemy, an ampoule at every enemy, and a rock at whatever it
/// was thrown at.
#[test]
fn the_damage_potions_deal_their_own_damage() {
    let mut simulator = fight_with_enemies(2, &["POTION.FIRE_POTION"], &[], &[]);
    simulator
        .step(
            simulator
                .legal_actions()
                .iter()
                .find(|action| matches!(action, Action::UsePotion { target: Some(TargetHandle { combat_id, .. }), .. } if combat_id.get() == 1))
                .cloned()
                .expect("a target is offered"),
        )
        .expect("the potion applies");
    assert_eq!(creature(&simulator, 1).current_hp, 19);
    assert_eq!(creature(&simulator, 2).current_hp, 39);

    let mut simulator = fight_with_enemies(2, &["POTION.EXPLOSIVE_AMPOULE"], &[], &[]);
    drink(&mut simulator, "POTION.EXPLOSIVE_AMPOULE");
    assert_eq!(creature(&simulator, 1).current_hp, 29);
    assert_eq!(creature(&simulator, 2).current_hp, 29);

    let mut simulator = fight_with_enemies(1, &["POTION.POTION_SHAPED_ROCK"], &[], &[]);
    drink_at(&mut simulator, "POTION.POTION_SHAPED_ROCK", Some(1));
    assert_eq!(creature(&simulator, 1).current_hp, 24);
}

/// A foul potion splashes everything in the fight, the drinker included: its
/// target list is the combat's creatures rather than the enemy side.
#[test]
fn a_foul_potion_splashes_the_drinker_too() {
    let mut simulator = fight_with_enemies(2, &["POTION.FOUL_POTION"], &[], &[]);
    drink(&mut simulator, "POTION.FOUL_POTION");
    assert_eq!(player(&simulator).current_hp, 68);
    assert_eq!(creature(&simulator, 1).current_hp, 27);
    assert_eq!(creature(&simulator, 2).current_hp, 27);
}

/// The block potions: a flat twelve, a fortifier that doubles whatever is
/// already there, and a ship that pays the same again next turn.
#[test]
fn the_block_potions_work_up_block_nothing_scales() {
    let mut simulator = fight(&["POTION.BLOCK_POTION", "POTION.FORTIFIER"], &[], &[]);
    drink(&mut simulator, "POTION.BLOCK_POTION");
    assert_eq!(player(&simulator).block, 12);
    drink(&mut simulator, "POTION.FORTIFIER");
    assert_eq!(player(&simulator).block, 36);

    let mut simulator = fight(&["POTION.SHIP_IN_A_BOTTLE"], &[], &[]);
    drink(&mut simulator, "POTION.SHIP_IN_A_BOTTLE");
    assert_eq!(player(&simulator).block, 10);
    assert_eq!(
        power_amount(&simulator, 0, "POWER.BLOCK_NEXT_TURN_POWER"),
        10
    );
}

/// A blood potion heals a fifth of the maximum, worked out and handed over
/// whole.
#[test]
fn a_blood_potion_heals_a_fifth_of_the_maximum() {
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(30, 80)
        .energy(9, 9)
        .turn(1, 1)
        .potions(vec![Some(id("POTION.BLOOD_POTION")), None, None])
        .rng_counter(RunStream::MonsterAi, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    drink(&mut simulator, "POTION.BLOOD_POTION");
    assert_eq!(player(&simulator).current_hp, 46);
}

/// A fruit juice raises the maximum and heals by as much.
#[test]
fn a_fruit_juice_raises_the_maximum() {
    let mut simulator = fight(&["POTION.FRUIT_JUICE"], &[], &[]);
    drink(&mut simulator, "POTION.FRUIT_JUICE");
    assert_eq!(player(&simulator).max_hp, 85);
    assert_eq!(player(&simulator).current_hp, 85);
}

// --- the belt as a listener ----------------------------------------------

/// A fairy in a bottle is never offered: it drinks itself, so it is never on
/// the belt's action list, and what drinks it is the death it turns away. The
/// potion leaves the belt and heals three tenths of the maximum.
#[test]
fn a_fairy_in_a_bottle_turns_a_death_away_and_is_spent_doing_it() {
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(3, 80)
        .energy(9, 9)
        .turn(1, 1)
        .potions(vec![Some(id("POTION.FAIRY_IN_A_BOTTLE")), None, None])
        .rng_counter(RunStream::MonsterAi, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "STOMP_MOVE")
        .card(PileName::Hand, 0, card("CARD.BREAKTHROUGH", 0), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    assert!(
        !simulator
            .legal_actions()
            .iter()
            .any(|action| matches!(action, Action::UsePotion { .. })),
        "an automatic potion is never offered"
    );
    // Ending the turn hands the beetle its swing, which is more than three HP.
    end_turn(&mut simulator);
    assert_eq!(belt(&simulator), [None, None, None]);
    assert_eq!(player(&simulator).current_hp, 24);
    assert!(simulator.state().terminal.is_none());
}

// --- what a potion does to the hand --------------------------------------

/// The draw potions: a swift three, a cure-all's energy and two, and a clarity
/// that draws one now and a card more on each of the next three turns.
#[test]
fn the_draw_potions_deal_what_they_promise() {
    let deck = [
        ("CARD.STRIKE_IRONCLAD", 0),
        ("CARD.STRIKE_IRONCLAD", 0),
        ("CARD.STRIKE_IRONCLAD", 0),
        ("CARD.DEFEND_IRONCLAD", 0),
        ("CARD.DEFEND_IRONCLAD", 0),
        ("CARD.DEFEND_IRONCLAD", 0),
        ("CARD.DEFEND_IRONCLAD", 0),
    ];
    let mut simulator = fight(&["POTION.SWIFT_POTION"], &[], &deck);
    drink(&mut simulator, "POTION.SWIFT_POTION");
    assert_eq!(pile(&simulator, PileName::Hand).len(), 3);

    let mut simulator = fight(&["POTION.CURE_ALL"], &[], &deck);
    let before = energy(&simulator);
    drink(&mut simulator, "POTION.CURE_ALL");
    assert_eq!(energy(&simulator), before + 1);
    assert_eq!(pile(&simulator, PileName::Hand).len(), 2);

    let mut simulator = fight(&["POTION.CLARITY"], &[], &deck);
    drink(&mut simulator, "POTION.CLARITY");
    assert_eq!(pile(&simulator, PileName::Hand).len(), 1);
    assert_eq!(power_amount(&simulator, 0, "POWER.CLARITY_POWER"), 3);
}

/// A radiant tincture and an energy potion both pay energy; the tincture also
/// works its radiance on.
#[test]
fn the_energy_potions_pay_the_pool() {
    let mut simulator = fight(
        &["POTION.ENERGY_POTION", "POTION.RADIANT_TINCTURE"],
        &[],
        &[],
    );
    let before = energy(&simulator);
    drink(&mut simulator, "POTION.ENERGY_POTION");
    assert_eq!(energy(&simulator), before + 2);
    drink(&mut simulator, "POTION.RADIANT_TINCTURE");
    assert_eq!(energy(&simulator), before + 3);
    assert_eq!(power_amount(&simulator, 0, "POWER.RADIANCE_POWER"), 3);
}

/// A blessing of the forge upgrades every card of the hand that can take one,
/// and passes over what cannot.
#[test]
fn a_blessing_of_the_forge_upgrades_the_whole_hand() {
    let mut simulator = fight(
        &["POTION.BLESSING_OF_THE_FORGE"],
        &[("CARD.STRIKE_IRONCLAD", 0), ("CARD.DEFEND_IRONCLAD", 1)],
        &[],
    );
    drink(&mut simulator, "POTION.BLESSING_OF_THE_FORGE");
    let levels: Vec<u8> = simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles
        .hand
        .iter()
        .map(|card| card.fingerprint.upgrade_level)
        .collect();
    assert_eq!(levels, [1, 1]);
}

/// Ashwater lays the hand out and burns whatever is picked; taking nothing
/// burns nothing.
#[test]
fn ashwater_burns_what_is_picked_out_of_the_hand() {
    let mut simulator = fight(
        &["POTION.ASHWATER"],
        &[("CARD.STRIKE_IRONCLAD", 0), ("CARD.DEFEND_IRONCLAD", 0)],
        &[],
    );
    drink(&mut simulator, "POTION.ASHWATER");
    assert!(matches!(
        simulator.decision(),
        DecisionContext::ChooseCards { .. }
    ));
    choose(&mut simulator, &["CARD.STRIKE_IRONCLAD"]);
    assert_eq!(
        pile(&simulator, PileName::Exhaust),
        ["CARD.STRIKE_IRONCLAD"]
    );
    assert_eq!(pile(&simulator, PileName::Hand), ["CARD.DEFEND_IRONCLAD"]);
}

/// Glowwater burns the whole hand outright and deals ten fresh cards.
#[test]
fn glowwater_burns_the_hand_and_deals_ten() {
    let draw: Vec<(&str, u8)> = std::iter::repeat_n(("CARD.STRIKE_IRONCLAD", 0), 12).collect();
    let mut simulator = fight(
        &["POTION.GLOWWATER_POTION"],
        &[("CARD.DEFEND_IRONCLAD", 0), ("CARD.DEFEND_IRONCLAD", 0)],
        &draw,
    );
    drink(&mut simulator, "POTION.GLOWWATER_POTION");
    assert_eq!(pile(&simulator, PileName::Exhaust).len(), 2);
    assert_eq!(pile(&simulator, PileName::Hand).len(), 10);
}

/// A touch of insanity makes one card of the hand that costs something free for
/// the rest of the fight.
#[test]
fn a_touch_of_insanity_makes_one_card_free_for_the_fight() {
    let mut simulator = fight(
        &["POTION.TOUCH_OF_INSANITY"],
        &[("CARD.BREAKTHROUGH", 0), ("CARD.STRIKE_IRONCLAD", 0)],
        &[],
    );
    drink(&mut simulator, "POTION.TOUCH_OF_INSANITY");
    choose(&mut simulator, &["CARD.BREAKTHROUGH"]);
    let cost = simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles
        .hand
        .iter()
        .find(|card| card.fingerprint.model_id == id("CARD.BREAKTHROUGH"))
        .expect("the card is still held")
        .energy_cost;
    assert_eq!(cost, 0);
}

/// A card already made free is not one that costs anything, so a second
/// touch of insanity does not offer it.
#[test]
fn a_touch_of_insanity_does_not_offer_a_card_already_made_free() {
    let mut simulator = fight(
        &["POTION.TOUCH_OF_INSANITY", "POTION.TOUCH_OF_INSANITY"],
        &[("CARD.BREAKTHROUGH", 0), ("CARD.STRIKE_IRONCLAD", 0)],
        &[],
    );
    drink(&mut simulator, "POTION.TOUCH_OF_INSANITY");
    choose(&mut simulator, &["CARD.BREAKTHROUGH"]);
    // The one card left that costs anything is the whole screen, and a screen
    // of one answers itself.
    drink(&mut simulator, "POTION.TOUCH_OF_INSANITY");
    let combat = simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing");
    assert!(combat.suspended_choice.is_none());
    assert!(
        combat
            .player
            .piles
            .hand
            .iter()
            .all(|card| card.energy_cost == 0)
    );
}

/// A gambler's brew puts down whatever is picked and deals back exactly as
/// many; picking nothing at all leaves the hand where it was.
#[test]
fn a_gamblers_brew_trades_what_is_picked_one_for_one() {
    let draw: Vec<(&str, u8)> = std::iter::repeat_n(("CARD.STRIKE_IRONCLAD", 0), 5).collect();
    let mut simulator = fight(
        &["POTION.GAMBLERS_BREW", "POTION.GAMBLERS_BREW"],
        &[("CARD.DEFEND_IRONCLAD", 0), ("CARD.DEFEND_IRONCLAD", 0)],
        &draw,
    );
    drink(&mut simulator, "POTION.GAMBLERS_BREW");
    choose(
        &mut simulator,
        &["CARD.DEFEND_IRONCLAD", "CARD.DEFEND_IRONCLAD"],
    );
    assert_eq!(pile(&simulator, PileName::Discard).len(), 2);
    assert_eq!(
        pile(&simulator, PileName::Hand),
        ["CARD.STRIKE_IRONCLAD", "CARD.STRIKE_IRONCLAD"]
    );
    // An empty pick returns before the draw as well as before the discard.
    let held = pile(&simulator, PileName::Hand).len();
    drink(&mut simulator, "POTION.GAMBLERS_BREW");
    choose_nothing(&mut simulator);
    assert_eq!(pile(&simulator, PileName::Hand).len(), held);
}

/// A bottled potential puts the hand back, shuffles the draw pile, and deals a
/// fresh five off it. The shuffle is the one thing it rolls for.
#[test]
fn a_bottled_potential_shuffles_the_hand_back_in_and_deals_five() {
    let draw: Vec<(&str, u8)> = std::iter::repeat_n(("CARD.STRIKE_IRONCLAD", 0), 6).collect();
    let mut simulator = fight(
        &["POTION.BOTTLED_POTENTIAL"],
        &[("CARD.DEFEND_IRONCLAD", 0), ("CARD.DEFEND_IRONCLAD", 0)],
        &draw,
    );
    let before = counter(&simulator, RunStream::Shuffle);
    drink(&mut simulator, "POTION.BOTTLED_POTENTIAL");
    assert_eq!(pile(&simulator, PileName::Hand).len(), 5);
    assert_eq!(pile(&simulator, PileName::Draw).len(), 3);
    assert!(counter(&simulator, RunStream::Shuffle) > before);
}

/// A droplet lays the draw pile out and takes one card of it straight into
/// hand; liquid memories does the same to the discard pile and makes the card
/// free for the turn.
#[test]
fn the_two_pile_potions_take_one_card_back() {
    let mut simulator = fight(
        &["POTION.DROPLET_OF_PRECOGNITION"],
        &[],
        &[("CARD.STRIKE_IRONCLAD", 0), ("CARD.DEFEND_IRONCLAD", 0)],
    );
    drink(&mut simulator, "POTION.DROPLET_OF_PRECOGNITION");
    choose(&mut simulator, &["CARD.DEFEND_IRONCLAD"]);
    assert_eq!(pile(&simulator, PileName::Hand), ["CARD.DEFEND_IRONCLAD"]);

    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .potions(vec![Some(id("POTION.LIQUID_MEMORIES")), None, None])
        .rng_counter(RunStream::MonsterAi, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE");
    scenario = scenario.card(PileName::Discard, 0, card("CARD.BREAKTHROUGH", 0), 0);
    scenario = scenario.card(PileName::Discard, 1, card("CARD.STRIKE_IRONCLAD", 0), 0);
    let mut simulator = scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    drink(&mut simulator, "POTION.LIQUID_MEMORIES");
    choose(&mut simulator, &["CARD.BREAKTHROUGH"]);
    assert_eq!(pile(&simulator, PileName::Hand), ["CARD.BREAKTHROUGH"]);
    let cost = simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles
        .hand[0]
        .energy_cost;
    assert_eq!(cost, 0);
}

/// Distilled chaos plays the top three cards of the draw pile without asking.
#[test]
fn distilled_chaos_plays_the_top_three_of_the_draw_pile() {
    let draw: Vec<(&str, u8)> = std::iter::repeat_n(("CARD.STRIKE_IRONCLAD", 0), 4).collect();
    let mut simulator = fight_with_enemies(1, &["POTION.DISTILLED_CHAOS"], &[], &draw);
    drink(&mut simulator, "POTION.DISTILLED_CHAOS");
    assert_eq!(pile(&simulator, PileName::Draw).len(), 1);
    assert_eq!(pile(&simulator, PileName::Discard).len(), 3);
    // Three basic strikes, six damage each.
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 18);
}

/// The three cards distilled chaos takes are all put on the play pile before the
/// first is played, and a play puts its card on the pile again as it starts —
/// so the card being played stands behind the ones still waiting. A quasar
/// second of three holds its screen up with the third card standing in front
/// of it. Seed `PCFYFYGRAS`, record 2318.
#[test]
fn a_card_being_auto_played_stands_behind_the_ones_still_waiting() {
    let draw = [
        ("CARD.DEFEND_IRONCLAD", 0),
        ("CARD.QUASAR", 0),
        ("CARD.STRIKE_IRONCLAD", 0),
        ("CARD.STRIKE_IRONCLAD", 0),
    ];
    let mut simulator = fight_with_enemies(1, &["POTION.DISTILLED_CHAOS"], &[], &draw);
    drink(&mut simulator, "POTION.DISTILLED_CHAOS");
    assert!(
        matches!(
            simulator.decision(),
            sts2_engine::DecisionContext::ChooseCards { .. }
        ),
        "the quasar's screen is up"
    );
    assert_eq!(
        pile(&simulator, PileName::Play),
        vec!["CARD.STRIKE_IRONCLAD", "CARD.QUASAR"],
        "the defend is played and gone, and the strike waits in front"
    );
}

/// A screen one of distilled chaos's cards opens holds the cards behind it, and
/// they are played once it is answered rather than left on the play pile. Seed
/// `PCFYFYGRAS`, record 2320, where a falling star waiting behind a quasar's
/// screen was never played.
#[test]
fn a_card_waiting_behind_an_auto_played_screen_is_played_after_it() {
    let draw = [
        ("CARD.DEFEND_IRONCLAD", 0),
        ("CARD.QUASAR", 0),
        ("CARD.STRIKE_IRONCLAD", 0),
        ("CARD.STRIKE_IRONCLAD", 0),
    ];
    let mut simulator = fight_with_enemies(1, &["POTION.DISTILLED_CHAOS"], &[], &draw);
    drink(&mut simulator, "POTION.DISTILLED_CHAOS");
    assert_eq!(
        creature(&simulator, 1).current_hp,
        39,
        "nothing has hit yet"
    );
    let skip = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseCards { cards, .. } if cards.is_empty()))
        .cloned()
        .expect("a quasar's screen can be passed over");
    simulator.step(skip).expect("the screen is answered");
    assert_eq!(
        creature(&simulator, 1).current_hp,
        39 - 6,
        "the strike behind the screen landed"
    );
    assert_eq!(pile(&simulator, PileName::Play), [] as [String; 0]);
    assert_eq!(pile(&simulator, PileName::Draw).len(), 1);
    assert_eq!(pile(&simulator, PileName::Discard).len(), 3);
}

/// A snecko oil deals seven and then rolls a fresh cost for every card of the
/// hand, one roll each off the combat energy-cost stream.
#[test]
fn a_snecko_oil_rolls_a_cost_for_every_card_of_the_hand() {
    let draw: Vec<(&str, u8)> = std::iter::repeat_n(("CARD.STRIKE_IRONCLAD", 0), 8).collect();
    let mut simulator = fight(
        &["POTION.SNECKO_OIL"],
        &[("CARD.DEFEND_IRONCLAD", 0)],
        &draw,
    );
    assert_eq!(counter(&simulator, RunStream::CombatEnergyCosts), 0);
    drink(&mut simulator, "POTION.SNECKO_OIL");
    let hand = &simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles
        .hand;
    assert_eq!(hand.len(), 8);
    assert_eq!(counter(&simulator, RunStream::CombatEnergyCosts), 8);
    assert!(hand.iter().all(|card| (0..4).contains(&card.energy_cost)));
}

// --- what a potion conjures ----------------------------------------------

/// The three type potions lay out three cards of one type rolled out of the
/// drinker's own pool, and the colorless potion does the same over the
/// colorless pool. What is taken is free for the turn.
#[test]
fn the_offer_potions_lay_out_three_of_their_own_kind() {
    for (potion, card_type) in [
        ("POTION.ATTACK_POTION", sts2_engine::CardType::Attack),
        ("POTION.SKILL_POTION", sts2_engine::CardType::Skill),
        ("POTION.POWER_POTION", sts2_engine::CardType::Power),
    ] {
        let mut simulator = fight(&[potion], &[], &[]);
        drink(&mut simulator, potion);
        let DecisionContext::ChooseCards { candidates, .. } = simulator.decision() else {
            panic!("{potion} opens a screen");
        };
        let cards = candidates;
        assert_eq!(cards.len(), 3, "{potion}");
        let content = sts2_content::standard_registry();
        for offered in cards {
            let definition = content
                .card(&offered.fingerprint.model_id)
                .expect("an offered card is content");
            assert_eq!(definition.card_type, card_type, "{potion}");
        }
        let taken = cards[0].fingerprint.model_id.to_string();
        choose(&mut simulator, &[&taken]);
        assert_eq!(pile(&simulator, PileName::Hand), [taken]);
        let cost = simulator
            .state()
            .combat
            .as_ref()
            .expect("the fight is standing")
            .player
            .piles
            .hand[0]
            .energy_cost;
        assert_eq!(cost, 0, "{potion}");
    }
}

/// Orobic acid rolls one attack, one skill, and one power out of the drinker's
/// own pool and hands all three over free for the turn: three distinct rolls
/// off the combat card-generation stream, no screen at all.
#[test]
fn orobic_acid_conjures_one_of_each_kind() {
    let mut simulator = fight(&["POTION.OROBIC_ACID"], &[], &[]);
    drink(&mut simulator, "POTION.OROBIC_ACID");
    let hand = pile(&simulator, PileName::Hand);
    assert_eq!(hand.len(), 3);
    let content = sts2_content::standard_registry();
    let types: Vec<sts2_engine::CardType> = simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles
        .hand
        .iter()
        .map(|card| {
            content
                .card(&card.fingerprint.model_id)
                .expect("a conjured card is content")
                .card_type
        })
        .collect();
    assert_eq!(
        types,
        [
            sts2_engine::CardType::Attack,
            sts2_engine::CardType::Skill,
            sts2_engine::CardType::Power
        ]
    );
    assert!(
        simulator
            .state()
            .combat
            .as_ref()
            .expect("the fight is standing")
            .player
            .piles
            .hand
            .iter()
            .all(|card| card.energy_cost == 0)
    );
}

/// A soldier's stew gives every Strike in the fight one more play, wherever it
/// is standing.
#[test]
fn a_soldiers_stew_replays_every_strike() {
    let mut simulator = fight_with_enemies(
        1,
        &["POTION.SOLDIERS_STEW"],
        &[("CARD.STRIKE_IRONCLAD", 0)],
        &[("CARD.STRIKE_IRONCLAD", 0)],
    );
    drink(&mut simulator, "POTION.SOLDIERS_STEW");
    let replays: Vec<i32> = simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles
        .all_cards()
        .map(|card| card.replay_count)
        .collect();
    assert_eq!(replays, [1, 1]);
}

/// A gigantification trebles the first attack that follows and is spent for it.
#[test]
fn a_gigantification_trebles_one_attack() {
    let mut simulator = fight_with_enemies(
        1,
        &["POTION.GIGANTIFICATION_POTION"],
        &[("CARD.STRIKE_IRONCLAD", 0), ("CARD.STRIKE_IRONCLAD", 0)],
        &[],
    );
    drink(&mut simulator, "POTION.GIGANTIFICATION_POTION");
    let play = |simulator: &mut Simulator| {
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
    };
    play(&mut simulator);
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 18);
    assert_eq!(
        power_amount(&simulator, 0, "POWER.GIGANTIFICATION_POWER"),
        0
    );
    play(&mut simulator);
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 24);
}

/// A duplicator plays the next card twice and is gone by the end of the turn.
#[test]
fn a_duplicator_plays_the_next_card_twice() {
    let mut simulator = fight_with_enemies(
        1,
        &["POTION.DUPLICATOR"],
        &[("CARD.STRIKE_IRONCLAD", 0), ("CARD.STRIKE_IRONCLAD", 0)],
        &[],
    );
    drink(&mut simulator, "POTION.DUPLICATOR");
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
    assert_eq!(creature(&simulator, 1).current_hp, 39 - 12);
    assert_eq!(power_amount(&simulator, 0, "POWER.DUPLICATION_POWER"), 0);
}

/// A lucky tonic's buffer takes one blow whole and is spent for it.
#[test]
fn a_buffer_takes_one_blow_whole() {
    let mut simulator = fight_against("STOMP_MOVE", 1, &["POTION.LUCKY_TONIC"], &[], &[]);
    drink(&mut simulator, "POTION.LUCKY_TONIC");
    end_turn(&mut simulator);
    assert_eq!(player(&simulator).current_hp, 80);
    assert_eq!(power_amount(&simulator, 0, "POWER.BUFFER_POWER"), 0);
}

/// The Necrobinder's own three are drunk by her, so the belt is stood up in a
/// fight she is walking: a pet is a creature on her side, and only her run ever
/// has one.
fn her_fight(potions: &[&str]) -> Simulator {
    let mut belt: Vec<Option<sts2_core::ModelId>> =
        potions.iter().map(|model| Some(id(model))).collect();
    while belt.len() < 3 {
        belt.push(None);
    }
    ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .played_as(id("CHARACTER.NECROBINDER"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .potions(belt)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

/// The pet standing on the player's side, if one is.
fn pet(simulator: &Simulator) -> Option<&sts2_engine::Creature> {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .creatures
        .iter()
        .find(|creature| creature.pet_owner.is_some())
}

/// A bone brew is a summon for fifteen and nothing else: the pet stands
/// up at fifteen, and a second brew raises the maximum by fifteen more and
/// heals it by exactly what it went up by.
#[test]
fn a_bone_brew_summons_the_pet_for_fifteen() {
    let mut simulator = her_fight(&["POTION.BONE_BREW", "POTION.BONE_BREW"]);
    assert!(pet(&simulator).is_none(), "she walks in without one");
    drink(&mut simulator, "POTION.BONE_BREW");
    let standing = pet(&simulator).expect("the brew stood one up");
    assert_eq!((standing.max_hp, standing.current_hp), (15, 15));
    drink(&mut simulator, "POTION.BONE_BREW");
    let standing = pet(&simulator).expect("the pet is still there");
    assert_eq!((standing.max_hp, standing.current_hp), (30, 30));
}

/// A potion of doom works thirty-three Doom onto whatever it is thrown at. A
/// creature with more health than that keeps standing; one with no more health
/// left than its Doom goes down as its own side's turn ends.
#[test]
fn a_potion_of_doom_works_thirty_three_doom_onto_its_target() {
    let mut simulator = her_fight(&["POTION.POTION_OF_DOOM"]);
    drink_at(&mut simulator, "POTION.POTION_OF_DOOM", Some(1));
    assert_eq!(power_amount(&simulator, 1, "POWER.DOOM_POWER"), 33);
    end_turn(&mut simulator);
    assert_eq!(
        creature(&simulator, 1).current_hp,
        39,
        "thirty-nine is more than thirty-three, so the beetle is not doomed yet"
    );
}

/// The same potion against a creature it does outlast: thirty Doom is more than
/// the thirty health the beetle has left, so the enemy turn ending kills it.
#[test]
fn a_doomed_creature_goes_down_as_its_own_side_finishes() {
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .played_as(id("CHARACTER.NECROBINDER"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .potions(vec![Some(id("POTION.POTION_OF_DOOM")), None, None])
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 30, 30, "SHRINKER_MOVE")
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    drink_at(&mut simulator, "POTION.POTION_OF_DOOM", Some(1));
    end_turn(&mut simulator);
    let combat = simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is still the room");
    assert!(
        !combat
            .creatures
            .iter()
            .any(|creature| creature.side == sts2_engine::CombatSide::Enemy),
        "the doomed beetle went down as its own side finished"
    );
    assert!(combat.rewards_open, "and the fight was won");
}

/// A pot of ghouls makes two Souls, straight into the hand.
#[test]
fn a_pot_of_ghouls_deals_two_souls_into_the_hand() {
    let mut simulator = her_fight(&["POTION.POT_OF_GHOULS"]);
    drink(&mut simulator, "POTION.POT_OF_GHOULS");
    assert_eq!(
        pile(&simulator, PileName::Hand),
        vec!["CARD.SOUL".to_owned(), "CARD.SOUL".to_owned()]
    );
}

/// The Defect's own three are drunk by him, in a fight with orb slots under it.
fn his_fight(potions: &[&str], slots: i32, orbs: &[&str]) -> Simulator {
    let mut belt: Vec<Option<sts2_core::ModelId>> =
        potions.iter().map(|model| Some(id(model))).collect();
    while belt.len() < 3 {
        belt.push(None);
    }
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .played_as(id("CHARACTER.DEFECT"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .orb_slots(slots)
        .potions(belt)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatTargets, 0)
        .rng_counter(RunStream::CombatCardSelection, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 999, 999, "SHRINKER_MOVE");
    for orb in orbs {
        scenario = scenario.orb(id(orb));
    }
    scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

fn orbs_held(simulator: &Simulator) -> Vec<String> {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .orbs
        .orbs
        .iter()
        .map(|orb| orb.model_id.to_string())
        .collect()
}

/// A focus potion is two Focus and nothing else, which is one more of every
/// orb its drinker is holding.
#[test]
fn a_focus_potion_works_two_focus_on() {
    let mut simulator = his_fight(&["POTION.FOCUS_POTION"], 3, &["ORB.LIGHTNING_ORB"]);
    drink(&mut simulator, "POTION.FOCUS_POTION");
    assert_eq!(power_amount(&simulator, 0, "POWER.FOCUS_POWER"), 2);
    let orb = &simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .orbs
        .orbs[0];
    assert_eq!((orb.passive, orb.evoke), (5, 10));
}

/// A potion of capacity adds two slots to the queue and leaves what is in it
/// where it stands.
#[test]
fn a_potion_of_capacity_adds_two_slots() {
    let mut simulator = his_fight(&["POTION.POTION_OF_CAPACITY"], 3, &["ORB.FROST_ORB"]);
    drink(&mut simulator, "POTION.POTION_OF_CAPACITY");
    let queue = &simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .orbs;
    assert_eq!(queue.capacity, 5);
    assert_eq!(orbs_held(&simulator), ["ORB.FROST_ORB"]);
}

/// An essence of darkness channels one Dark orb per slot the queue has, however
/// many of them are already filled: the count is taken before anything is
/// channelled, so a full queue evokes its way through itself.
#[test]
fn an_essence_of_darkness_fills_the_queue_with_dark() {
    let mut simulator = his_fight(&["POTION.ESSENCE_OF_DARKNESS"], 3, &[]);
    drink(&mut simulator, "POTION.ESSENCE_OF_DARKNESS");
    assert_eq!(
        orbs_held(&simulator),
        ["ORB.DARK_ORB", "ORB.DARK_ORB", "ORB.DARK_ORB"]
    );

    let mut full = his_fight(
        &["POTION.ESSENCE_OF_DARKNESS"],
        2,
        &["ORB.FROST_ORB", "ORB.FROST_ORB"],
    );
    let before = creature(&full, 0).block;
    drink(&mut full, "POTION.ESSENCE_OF_DARKNESS");
    assert_eq!(orbs_held(&full), ["ORB.DARK_ORB", "ORB.DARK_ORB"]);
    assert_eq!(
        creature(&full, 0).block - before,
        10,
        "both Frost orbs were evoked out of the front of the queue"
    );
}
