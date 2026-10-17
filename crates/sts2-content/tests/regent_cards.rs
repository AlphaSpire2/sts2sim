//! The Regent's pool, pinned one behaviour at a time.
//!
//! The recording walks about a quarter of it; a scenario is the whole of the
//! evidence for everything else, and the scenarios are here whether or not the
//! recording also witnesses the card, because a recorded segment pins one path
//! through a body and a scenario pins the body.
//!
//! Three shapes recur and are worth naming once. A star cost is the second half
//! of what a card costs, and a card that costs more stars than its owner is
//! standing on is not a legal play at all. A forge stands a Sovereign Blade up
//! where its owner holds none outside the exhaust pile and sharpens every blade
//! they hold. And the blade itself is a token: it swings for what the forge has
//! grown it to, at the whole enemy side while its owner carries a seeking edge,
//! and works up as much block behind the blow as they have parry.
//!
//! Where a family of cards is one shape with different numbers — a power laid
//! down, a blow and nothing else — the family is one table-driven test and each
//! card of it is a row.

use sts2_engine::{Action, CardFingerprint, PileName, ScenarioBuilder, Simulator};
use sts2_rng::RunStream;

const SEED: &str = "REGENTCARDS";

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str, upgrade_level: u8) -> CardFingerprint {
    let mut fingerprint = CardFingerprint::base(id(model_id));
    fingerprint.upgrade_level = upgrade_level;
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

/// One fight against a single beetle, played as the Regent, with the hand the
/// test names, energy enough that a cost never gets in the way, and stars
/// enough that a star cost never does either.
fn fight(hand: &[(&str, u8)]) -> Simulator {
    built(scenario(&[39], hand, &[], 20))
}

fn fight_with_stars(hand: &[(&str, u8)], stars: i32) -> Simulator {
    built(scenario(&[39], hand, &[], stars))
}

fn fight_with_draw(hand: &[(&str, u8)], draw: &[(&str, u8)]) -> Simulator {
    built(scenario(&[39], hand, draw, 20))
}

/// The same fight with two beetles, for the cards that sweep the whole side.
fn crowd(hand: &[(&str, u8)]) -> Simulator {
    built(scenario(&[39, 39], hand, &[], 20))
}

fn built(scenario: ScenarioBuilder) -> Simulator {
    scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

fn scenario(
    enemies: &[i32],
    hand: &[(&str, u8)],
    draw: &[(&str, u8)],
    stars: i32,
) -> ScenarioBuilder {
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .played_as(id("CHARACTER.REGENT"))
        .player_hp(75, 75)
        .energy(9, 9)
        .stars(stars)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatTargets, 0)
        .rng_counter(RunStream::CombatCardSelection, 0);
    for (index, hp) in enemies.iter().enumerate() {
        let slot = u64::try_from(index).expect("small index") + 1;
        scenario = scenario.enemy(
            slot,
            id("MONSTER.SHRINKER_BEETLE"),
            *hp,
            *hp,
            "SHRINKER_MOVE",
        );
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
}

/// A draw pile of `count` copies of one model, for the bodies that draw.
fn stack(model_id: &str, count: usize) -> Vec<(&str, u8)> {
    vec![(model_id, 0); count]
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

/// Answers a card screen with the one card the test names.
fn choose(simulator: &mut Simulator, model_id: &str) {
    let wanted = id(model_id);
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::ChooseCards { cards, .. }
                if cards.len() == 1 && cards[0].fingerprint.model_id == wanted)
        })
        .cloned()
        .unwrap_or_else(|| panic!("{model_id} was not on the screen"));
    simulator.step(action).expect("the choice applies");
}

/// Answers a card screen with the cards the test names, in the order it names
/// them, which is the order the screen hands them back in.
fn choose_in_order(simulator: &mut Simulator, model_ids: &[&str]) {
    let wanted: Vec<_> = model_ids.iter().map(|model_id| id(model_id)).collect();
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::ChooseCards { cards, .. }
                if cards.len() == wanted.len()
                    && cards
                        .iter()
                        .zip(&wanted)
                        .all(|(card, wanted)| card.fingerprint.model_id == *wanted))
        })
        .cloned()
        .unwrap_or_else(|| panic!("{model_ids:?} was not an answer the screen took"));
    simulator.step(action).expect("the choice applies");
}

/// Answers a card screen with whatever it offers first, which is what a test
/// that only cares that the screen went up says.
fn choose_anything(simulator: &mut Simulator) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseCards { cards, .. } if !cards.is_empty()))
        .cloned()
        .expect("the screen offers something");
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

fn hp_of(simulator: &Simulator, combat_id: u64) -> i32 {
    simulator
        .state()
        .combat
        .as_ref()
        .and_then(|combat| {
            combat
                .creatures
                .iter()
                .find(|creature| creature.id.get() == combat_id)
        })
        .map_or(0, |creature| creature.current_hp)
}

fn block(simulator: &Simulator) -> i32 {
    creature(simulator, 0).block
}

fn power_amount(simulator: &Simulator, combat_id: u64, power: &str) -> i32 {
    creature(simulator, combat_id).power_amount(power)
}

fn stars(simulator: &Simulator) -> i32 {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .stars
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

fn count_in(simulator: &Simulator, pile_name: PileName, model_id: &str) -> usize {
    pile(simulator, pile_name)
        .iter()
        .filter(|model| *model == model_id)
        .count()
}

/// The upgrade level of every card of that model in the pile, in pile order.
fn upgrade_levels_in(simulator: &Simulator, pile_name: PileName, model_id: &str) -> Vec<u8> {
    let piles = &simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles;
    let cards = match pile_name {
        PileName::Draw => &piles.draw,
        PileName::Hand => &piles.hand,
        PileName::Discard => &piles.discard,
        PileName::Exhaust => &piles.exhaust,
        PileName::Play => &piles.play,
    };
    cards
        .iter()
        .filter(|card| card.fingerprint.model_id == id(model_id))
        .map(|card| card.fingerprint.upgrade_level)
        .collect()
}

/// Every card of the pile as its combat id and model, in pile order.
fn ids_in(simulator: &Simulator, pile_name: PileName) -> Vec<(u64, String)> {
    let piles = &simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles;
    let cards = match pile_name {
        PileName::Draw => &piles.draw,
        PileName::Hand => &piles.hand,
        PileName::Discard => &piles.discard,
        PileName::Exhaust => &piles.exhaust,
        PileName::Play => &piles.play,
    };
    cards
        .iter()
        .map(|card| (card.id.get(), card.fingerprint.model_id.to_string()))
        .collect()
}

/// What a card in hand costs right now — energy and then stars — which is what
/// the projection carries and what a play is refused for.
fn costs(simulator: &Simulator, model_id: &str) -> (i32, i32) {
    let card = simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles
        .hand
        .iter()
        .find(|card| card.fingerprint.model_id == id(model_id))
        .expect("the card is in hand");
    (card.energy_cost, card.star_cost)
}

// --- the star pool --------------------------------------------------------

/// A card that costs no stars at all carries `-1` rather than zero, and a card
/// that costs some carries what its model declares.
#[test]
fn a_star_cost_is_the_models_own_and_nothing_at_all_where_none_is_declared() {
    let simulator = fight(&[
        ("CARD.STRIKE_REGENT", 0),
        ("CARD.FALLING_STAR", 0),
        ("CARD.SEVEN_STARS", 0),
    ]);
    assert_eq!(costs(&simulator, "CARD.STRIKE_REGENT").1, -1);
    assert_eq!(costs(&simulator, "CARD.FALLING_STAR").1, 2);
    assert_eq!(costs(&simulator, "CARD.SEVEN_STARS").1, 7);
}

/// A play is refused for stars the way it is refused for energy, and the pool
/// does not refill between turns: what a fight has put into it stands until it
/// is spent.
#[test]
fn a_card_that_costs_more_stars_than_the_pool_holds_cannot_be_played() {
    let mut simulator = fight_with_stars(&[("CARD.FALLING_STAR", 0), ("CARD.STRIKE_REGENT", 0)], 1);
    assert!(play_action(&simulator, "CARD.FALLING_STAR").is_none());
    assert!(play_action(&simulator, "CARD.STRIKE_REGENT").is_some());
    end_turn(&mut simulator);
    assert_eq!(stars(&simulator), 1, "nothing refills the pool");
}

/// Playing a card spends its stars; a card that costs none spends none.
#[test]
fn a_play_spends_the_stars_it_costs() {
    let mut simulator = fight_with_stars(&[("CARD.FALLING_STAR", 0), ("CARD.STRIKE_REGENT", 0)], 5);
    play(&mut simulator, "CARD.FALLING_STAR");
    assert_eq!(stars(&simulator), 3);
    play(&mut simulator, "CARD.STRIKE_REGENT");
    assert_eq!(stars(&simulator), 3);
}

/// The cards that pay stars outright, each of them one row.
#[test]
fn the_cards_that_pay_stars() {
    for (model_id, level, gained) in [
        ("CARD.VENERATE", 0, 2),
        ("CARD.VENERATE", 1, 3),
        ("CARD.GATHER_LIGHT", 0, 1),
        ("CARD.GLOW", 0, 1),
        ("CARD.GLOW", 1, 2),
        ("CARD.HIDDEN_CACHE", 0, 1),
        ("CARD.SOLAR_STRIKE", 0, 1),
        ("CARD.SOLAR_STRIKE", 1, 2),
        ("CARD.ROYAL_GAMBLE", 0, 9),
        ("CARD.BIG_BANG", 0, 1),
    ] {
        let mut simulator = fight_with_stars(&[(model_id, level)], 6);
        let held = stars(&simulator);
        let cost = costs(&simulator, model_id).1.max(0);
        play(&mut simulator, model_id);
        assert_eq!(
            stars(&simulator),
            held - cost + gained,
            "{model_id} at level {level}"
        );
    }
}

/// A hidden cache pays a star now and more as the next turn opens.
#[test]
fn a_hidden_cache_owes_stars_next_turn() {
    let mut simulator = fight_with_stars(&[("CARD.HIDDEN_CACHE", 0)], 0);
    play(&mut simulator, "CARD.HIDDEN_CACHE");
    assert_eq!(stars(&simulator), 1);
    assert_eq!(power_amount(&simulator, 0, "POWER.STAR_NEXT_TURN_POWER"), 3);
    end_turn(&mut simulator);
    assert_eq!(
        stars(&simulator),
        4,
        "the three owed landed as the turn opened"
    );
}

/// A convergence keeps the hand and owes an energy and a star next turn.
#[test]
fn a_convergence_keeps_the_hand_and_owes_for_next_turn() {
    for (level, owed_stars) in [(0, 1), (1, 2)] {
        let mut simulator = fight_with_stars(&[("CARD.CONVERGENCE", level)], 0);
        play(&mut simulator, "CARD.CONVERGENCE");
        assert_eq!(power_amount(&simulator, 0, "POWER.RETAIN_HAND_POWER"), 1);
        assert_eq!(
            power_amount(&simulator, 0, "POWER.ENERGY_NEXT_TURN_POWER"),
            1
        );
        assert_eq!(
            power_amount(&simulator, 0, "POWER.STAR_NEXT_TURN_POWER"),
            owed_stars,
            "at level {level}"
        );
    }
}

/// A stardust costs X stars: the play spends the whole pool, and the body hits
/// once for every star it spent.
#[test]
fn a_stardust_spends_the_pool_and_hits_for_what_it_spent() {
    let mut simulator = built(scenario(&[80], &[("CARD.STARDUST", 0)], &[], 4));
    let hp = hp_of(&simulator, 1);
    play(&mut simulator, "CARD.STARDUST");
    assert_eq!(stars(&simulator), 0, "an X star cost spends everything");
    assert_eq!(hp - hp_of(&simulator, 1), 20, "four hits of five");
}

/// A radiate hits once for every star its owner has gained this turn, which is
/// a count of its own rather than what the pool is standing at.
#[test]
fn a_radiate_counts_the_stars_gained_this_turn() {
    let mut simulator = fight_with_stars(&[("CARD.VENERATE", 0), ("CARD.RADIATE", 0)], 0);
    play(&mut simulator, "CARD.VENERATE");
    let hp = hp_of(&simulator, 1);
    play(&mut simulator, "CARD.RADIATE");
    assert_eq!(hp - hp_of(&simulator, 1), 6, "two hits of three");
}

/// A crescent spear is worth eight and two more for every card of the fight
/// whose model declares a star cost at all — counted over every pile, and over
/// the spear itself.
#[test]
fn a_crescent_spear_counts_the_cards_that_cost_stars() {
    let mut simulator = built(scenario(
        &[80],
        &[("CARD.CRESCENT_SPEAR", 0), ("CARD.FALLING_STAR", 0)],
        &[("CARD.STRIKE_REGENT", 0), ("CARD.ASTRAL_PULSE", 0)],
        20,
    ));
    let hp = hp_of(&simulator, 1);
    play(&mut simulator, "CARD.CRESCENT_SPEAR");
    assert_eq!(
        hp - hp_of(&simulator, 1),
        8 + 2 * 3,
        "the spear, the star and the pulse cost stars; the strike does not"
    );
}

/// A knockout blow pays five stars only where it put something down.
#[test]
fn a_knockout_blow_pays_only_for_a_kill() {
    let mut simulator = built(scenario(&[20, 300], &[("CARD.KNOCKOUT_BLOW", 0)], &[], 0));
    play(&mut simulator, "CARD.KNOCKOUT_BLOW");
    assert_eq!(stars(&simulator), 5, "the beetle it was aimed at went down");

    let mut simulator = built(scenario(&[80], &[("CARD.KNOCKOUT_BLOW", 0)], &[], 0));
    play(&mut simulator, "CARD.KNOCKOUT_BLOW");
    assert_eq!(stars(&simulator), 0, "and this one did not");
}

// --- the forge ------------------------------------------------------------

/// A forge with no blade standing makes one and sharpens it; a second forge
/// sharpens the one already there rather than making another.
#[test]
fn a_forge_stands_a_blade_up_once_and_sharpens_it_every_time() {
    let mut simulator = built(scenario(
        &[200],
        &[("CARD.THE_SMITH", 0), ("CARD.REFINE_BLADE", 0)],
        &[],
        20,
    ));
    play(&mut simulator, "CARD.THE_SMITH");
    assert_eq!(
        count_in(&simulator, PileName::Hand, "CARD.SOVEREIGN_BLADE"),
        1,
        "the first forge stands a blade up"
    );
    play(&mut simulator, "CARD.REFINE_BLADE");
    assert_eq!(
        count_in(&simulator, PileName::Hand, "CARD.SOVEREIGN_BLADE"),
        1,
        "the second sharpens the one already standing"
    );
    let hp = hp_of(&simulator, 1);
    play(&mut simulator, "CARD.SOVEREIGN_BLADE");
    assert_eq!(
        hp - hp_of(&simulator, 1),
        10 + 30 + 9,
        "ten of its own, thirty from the smith and nine from the refinement"
    );
}

/// A blade already standing in a pile that is not the exhaust pile is one the
/// forge finds, so no second blade is made beside it.
#[test]
fn a_blade_outside_the_exhaust_pile_is_the_one_the_forge_finds() {
    let mut simulator = built(scenario(
        &[200],
        &[("CARD.SOVEREIGN_BLADE", 0), ("CARD.THE_SMITH", 0)],
        &[],
        20,
    ));
    play(&mut simulator, "CARD.SOVEREIGN_BLADE");
    assert_eq!(
        count_in(&simulator, PileName::Discard, "CARD.SOVEREIGN_BLADE"),
        1
    );
    play(&mut simulator, "CARD.THE_SMITH");
    assert_eq!(
        count_in(&simulator, PileName::Hand, "CARD.SOVEREIGN_BLADE"),
        0,
        "the discarded blade is the one it sharpened"
    );
    assert_eq!(
        count_in(&simulator, PileName::Discard, "CARD.SOVEREIGN_BLADE"),
        1
    );
}

/// A refine blade also owes its owner an energy next turn.
#[test]
fn a_refine_blade_owes_an_energy_next_turn() {
    let mut simulator = fight(&[("CARD.REFINE_BLADE", 0)]);
    play(&mut simulator, "CARD.REFINE_BLADE");
    assert_eq!(
        power_amount(&simulator, 0, "POWER.ENERGY_NEXT_TURN_POWER"),
        1
    );
}

/// A seeking edge sends the blade at the whole enemy side rather than at one of
/// them: the blade's target type is worked out rather than declared.
#[test]
fn a_seeking_edge_widens_the_blade_to_the_whole_side() {
    let mut simulator = built(scenario(&[200, 200], &[("CARD.SEEKING_EDGE", 0)], &[], 20));
    let first = hp_of(&simulator, 1);
    let second = hp_of(&simulator, 2);
    play(&mut simulator, "CARD.SEEKING_EDGE");
    play(&mut simulator, "CARD.SOVEREIGN_BLADE");
    assert_eq!(first - hp_of(&simulator, 1), 17);
    assert_eq!(second - hp_of(&simulator, 2), 17);
}

/// A parry pays the blade block behind its blow, and a blade standing behind no
/// parry works up none.
#[test]
fn a_parry_pays_the_blade_block_behind_its_blow() {
    let mut simulator = built(scenario(&[200], &[("CARD.THE_SMITH", 0)], &[], 20));
    play(&mut simulator, "CARD.THE_SMITH");
    play(&mut simulator, "CARD.SOVEREIGN_BLADE");
    assert_eq!(block(&simulator), 0, "no parry, no block");

    let mut simulator = built(scenario(
        &[200],
        &[("CARD.THE_SMITH", 0), ("CARD.PARRY", 0)],
        &[],
        20,
    ));
    play(&mut simulator, "CARD.THE_SMITH");
    play(&mut simulator, "CARD.PARRY");
    play(&mut simulator, "CARD.SOVEREIGN_BLADE");
    assert_eq!(block(&simulator), 10);
}

/// A summon forth calls every blade standing anywhere but the hand back into
/// it, and then forges.
#[test]
fn a_summon_forth_calls_the_blades_back() {
    let mut simulator = built(scenario(
        &[200],
        &[("CARD.SOVEREIGN_BLADE", 0), ("CARD.SUMMON_FORTH", 0)],
        &[],
        20,
    ));
    play(&mut simulator, "CARD.SOVEREIGN_BLADE");
    assert_eq!(
        count_in(&simulator, PileName::Discard, "CARD.SOVEREIGN_BLADE"),
        1
    );
    play(&mut simulator, "CARD.SUMMON_FORTH");
    assert_eq!(
        count_in(&simulator, PileName::Hand, "CARD.SOVEREIGN_BLADE"),
        1,
        "the summon called it back"
    );
    let hp = hp_of(&simulator, 1);
    play(&mut simulator, "CARD.SOVEREIGN_BLADE");
    assert_eq!(
        hp - hp_of(&simulator, 1),
        18,
        "ten of its own and the eight the summon forged"
    );
}

/// Every card of the pool that forges, each of them one row: what the blade is
/// worth after one play is the ten it carries and the amount the card forged.
#[test]
fn the_cards_that_forge() {
    for (model_id, level, forged) in [
        ("CARD.BULWARK", 0, 10),
        ("CARD.BULWARK", 1, 13),
        ("CARD.REFINE_BLADE", 0, 9),
        ("CARD.REFINE_BLADE", 1, 13),
        ("CARD.SEEKING_EDGE", 0, 7),
        ("CARD.SEEKING_EDGE", 1, 11),
        ("CARD.SPOILS_OF_BATTLE", 0, 5),
        ("CARD.SPOILS_OF_BATTLE", 1, 8),
        ("CARD.THE_SMITH", 0, 30),
        ("CARD.THE_SMITH", 1, 40),
        ("CARD.WROUGHT_IN_WAR", 0, 7),
        ("CARD.WROUGHT_IN_WAR", 1, 9),
        ("CARD.BIG_BANG", 0, 5),
    ] {
        let mut simulator = built(scenario(&[300], &[(model_id, level)], &[], 20));
        play(&mut simulator, model_id);
        assert_eq!(
            count_in(&simulator, PileName::Hand, "CARD.SOVEREIGN_BLADE"),
            1,
            "{model_id} at level {level} stood a blade up"
        );
        let hp = hp_of(&simulator, 1);
        play(&mut simulator, "CARD.SOVEREIGN_BLADE");
        assert_eq!(
            hp - hp_of(&simulator, 1),
            10 + forged,
            "{model_id} at level {level} forged {forged}"
        );
    }
}

/// A beat into shape forges for the powered blows already landed on its target
/// this turn, less the helping its own hit would otherwise have added.
#[test]
fn a_beat_into_shape_forges_for_the_blows_already_landed() {
    let mut simulator = built(scenario(
        &[300],
        &[("CARD.STRIKE_REGENT", 0), ("CARD.BEAT_INTO_SHAPE", 0)],
        &[],
        20,
    ));
    play(&mut simulator, "CARD.STRIKE_REGENT");
    play(&mut simulator, "CARD.BEAT_INTO_SHAPE");
    let hp = hp_of(&simulator, 1);
    play(&mut simulator, "CARD.SOVEREIGN_BLADE");
    assert_eq!(
        hp - hp_of(&simulator, 1),
        10 + 10,
        "five, and five for the strike, on top of the blade's own ten"
    );
}

// --- the bodies -----------------------------------------------------------

/// The basics: the strike, the defend, the falling star the deck opens with,
/// and the venerate that pays for it.
#[test]
fn the_basics_are_what_the_deck_opens_with() {
    let mut simulator = fight(&[
        ("CARD.STRIKE_REGENT", 0),
        ("CARD.DEFEND_REGENT", 0),
        ("CARD.VENERATE", 0),
    ]);
    let hp = hp_of(&simulator, 1);
    play(&mut simulator, "CARD.STRIKE_REGENT");
    assert_eq!(hp - hp_of(&simulator, 1), 6);
    play(&mut simulator, "CARD.DEFEND_REGENT");
    assert_eq!(block(&simulator), 5);
    let held = stars(&simulator);
    play(&mut simulator, "CARD.VENERATE");
    assert_eq!(stars(&simulator), held + 2);
}

/// The starry attacks: a blow, and the Weak and the Vulnerable behind it.
#[test]
fn the_starry_attacks_weaken_and_make_their_target_vulnerable() {
    for (model_id, level, damage, weak, vulnerable) in [
        ("CARD.FALLING_STAR", 0, 8, 1, 1),
        ("CARD.FALLING_STAR", 1, 12, 1, 1),
        ("CARD.GAMMA_BLAST", 0, 13, 2, 2),
        ("CARD.GAMMA_BLAST", 1, 18, 2, 2),
        ("CARD.COMET", 0, 33, 3, 3),
        ("CARD.COMET", 1, 44, 3, 3),
    ] {
        let mut simulator = built(scenario(&[200], &[(model_id, level)], &[], 20));
        let hp = hp_of(&simulator, 1);
        play(&mut simulator, model_id);
        assert_eq!(
            hp - hp_of(&simulator, 1),
            damage,
            "{model_id} at level {level}"
        );
        assert_eq!(power_amount(&simulator, 1, "POWER.WEAK_POWER"), weak);
        assert_eq!(
            power_amount(&simulator, 1, "POWER.VULNERABLE_POWER"),
            vulnerable
        );
    }
}

/// The attacks that are one blow at whoever the play was aimed at and nothing
/// else, at both the levels they support.
#[test]
fn the_attacks_that_are_one_blow() {
    for (model_id, base, upgraded) in [
        ("CARD.STRIKE_REGENT", 6, 9),
        ("CARD.CELESTIAL_MIGHT", 18, 24),
        ("CARD.DEVASTATE", 30, 40),
        ("CARD.KINGLY_KICK", 27, 35),
        ("CARD.KINGLY_PUNCH", 8, 10),
        ("CARD.BOMBARDMENT", 18, 24),
        ("CARD.MAKE_IT_SO", 6, 9),
    ] {
        for (level, expected) in [(0, base), (1, upgraded)] {
            let mut simulator = built(scenario(&[300], &[(model_id, level)], &[], 20));
            let hp = hp_of(&simulator, 1);
            play(&mut simulator, model_id);
            assert_eq!(
                hp - hp_of(&simulator, 1),
                expected,
                "{model_id} at level {level}"
            );
        }
    }
}

/// The cards whose whole body is block, at both levels.
#[test]
fn the_cards_that_are_block() {
    for (model_id, base, upgraded) in [
        ("CARD.DEFEND_REGENT", 5, 8),
        ("CARD.CLOAK_OF_STARS", 7, 10),
        ("CARD.I_AM_INVINCIBLE", 10, 13),
        ("CARD.PARTICLE_WALL", 9, 12),
        ("CARD.REFLECT", 15, 20),
    ] {
        for (level, expected) in [(0, base), (1, upgraded)] {
            let mut simulator = fight(&[(model_id, level)]);
            play(&mut simulator, model_id);
            assert_eq!(block(&simulator), expected, "{model_id} at level {level}");
        }
    }
}

/// The cards that sweep the whole enemy side for one number.
#[test]
fn the_cards_that_sweep_the_side() {
    for (model_id, base, upgraded) in [
        ("CARD.ASTRAL_PULSE", 12, 16),
        ("CARD.CRUSH_UNDER", 7, 8),
        ("CARD.DYING_STAR", 9, 11),
        ("CARD.METEOR_SHOWER", 14, 21),
        ("CARD.SEVEN_STARS", 49, 49),
        ("CARD.CRASH_LANDING", 21, 26),
    ] {
        for (level, expected) in [(0, base), (1, upgraded)] {
            let mut simulator = built(scenario(&[300, 300], &[(model_id, level)], &[], 20));
            let first = hp_of(&simulator, 1);
            let second = hp_of(&simulator, 2);
            play(&mut simulator, model_id);
            assert_eq!(
                first - hp_of(&simulator, 1),
                expected,
                "{model_id} at level {level}"
            );
            assert_eq!(
                second - hp_of(&simulator, 2),
                expected,
                "{model_id} at level {level}"
            );
        }
    }
}

/// The powers his cards lay down, each of them one row.
#[test]
fn the_powers_the_pool_lays_down() {
    for (model_id, level, power_id, amount) in [
        ("CARD.ARSENAL", 0, "POWER.ARSENAL_POWER", 1),
        ("CARD.BLACK_HOLE", 0, "POWER.BLACK_HOLE_POWER", 3),
        ("CARD.BLACK_HOLE", 1, "POWER.BLACK_HOLE_POWER", 4),
        (
            "CARD.CHILD_OF_THE_STARS",
            0,
            "POWER.CHILD_OF_THE_STARS_POWER",
            2,
        ),
        (
            "CARD.CHILD_OF_THE_STARS",
            1,
            "POWER.CHILD_OF_THE_STARS_POWER",
            3,
        ),
        (
            "CARD.FOREGONE_CONCLUSION",
            0,
            "POWER.FOREGONE_CONCLUSION_POWER",
            2,
        ),
        (
            "CARD.FOREGONE_CONCLUSION",
            1,
            "POWER.FOREGONE_CONCLUSION_POWER",
            3,
        ),
        ("CARD.FURNACE", 0, "POWER.FURNACE_POWER", 5),
        ("CARD.FURNACE", 1, "POWER.FURNACE_POWER", 7),
        ("CARD.GENESIS", 0, "POWER.GENESIS_POWER", 2),
        ("CARD.GENESIS", 1, "POWER.GENESIS_POWER", 3),
        ("CARD.HAMMER_TIME", 0, "POWER.HAMMER_TIME_POWER", 1),
        ("CARD.MONARCHS_GAZE", 0, "POWER.MONARCHS_GAZE_POWER", 1),
        (
            "CARD.PILLAR_OF_CREATION",
            0,
            "POWER.PILLAR_OF_CREATION_POWER",
            3,
        ),
        (
            "CARD.PILLAR_OF_CREATION",
            1,
            "POWER.PILLAR_OF_CREATION_POWER",
            4,
        ),
        ("CARD.MONOLOGUE", 0, "POWER.MONOLOGUE_POWER", 1),
        ("CARD.NEUTRON_AEGIS", 0, "POWER.PLATING_POWER", 8),
        ("CARD.NEUTRON_AEGIS", 1, "POWER.PLATING_POWER", 11),
        ("CARD.ORBIT", 0, "POWER.ORBIT_POWER", 1),
        ("CARD.PALE_BLUE_DOT", 0, "POWER.PALE_BLUE_DOT_POWER", 1),
        ("CARD.PALE_BLUE_DOT", 1, "POWER.PALE_BLUE_DOT_POWER", 2),
        ("CARD.PARRY", 0, "POWER.PARRY_POWER", 10),
        ("CARD.PARRY", 1, "POWER.PARRY_POWER", 14),
        ("CARD.PATTER", 0, "POWER.VIGOR_POWER", 2),
        ("CARD.PATTER", 1, "POWER.VIGOR_POWER", 3),
        ("CARD.REFLECT", 0, "POWER.REFLECT_POWER", 1),
        ("CARD.ROYALTIES", 0, "POWER.ROYALTIES_POWER", 30),
        ("CARD.ROYALTIES", 1, "POWER.ROYALTIES_POWER", 40),
        ("CARD.SEEKING_EDGE", 0, "POWER.SEEKING_EDGE_POWER", 1),
        ("CARD.SPECTRUM_SHIFT", 0, "POWER.SPECTRUM_SHIFT_POWER", 1),
        ("CARD.SWORD_SAGE", 0, "POWER.SWORD_SAGE_POWER", 1),
        ("CARD.TERRAFORMING", 0, "POWER.VIGOR_POWER", 6),
        ("CARD.TERRAFORMING", 1, "POWER.VIGOR_POWER", 8),
        (
            "CARD.THE_SEALED_THRONE",
            0,
            "POWER.THE_SEALED_THRONE_POWER",
            1,
        ),
        ("CARD.TYRANNY", 0, "POWER.TYRANNY_POWER", 1),
    ] {
        let mut simulator = fight(&[(model_id, level)]);
        play(&mut simulator, model_id);
        assert_eq!(
            power_amount(&simulator, 0, power_id),
            amount,
            "{model_id} at level {level} lays {power_id} down"
        );
    }
}

/// The debuffs his cards land on the whole enemy side.
#[test]
fn the_debuffs_the_pool_lands_on_the_side() {
    for (model_id, level, power_id, amount) in [
        ("CARD.CRUSH_UNDER", 0, "POWER.CRUSH_UNDER_POWER", 1),
        ("CARD.CRUSH_UNDER", 1, "POWER.CRUSH_UNDER_POWER", 2),
        ("CARD.DYING_STAR", 0, "POWER.DYING_STAR_POWER", 9),
        ("CARD.DYING_STAR", 1, "POWER.DYING_STAR_POWER", 11),
        ("CARD.METEOR_SHOWER", 0, "POWER.WEAK_POWER", 2),
        ("CARD.METEOR_SHOWER", 0, "POWER.VULNERABLE_POWER", 2),
    ] {
        let mut simulator = built(scenario(&[300, 300], &[(model_id, level)], &[], 20));
        play(&mut simulator, model_id);
        for enemy in [1, 2] {
            assert_eq!(
                power_amount(&simulator, enemy, power_id),
                amount,
                "{model_id} at level {level} on the enemy in slot {enemy}"
            );
        }
    }
}

/// A know thy place is the two debuffs and nothing else, and the upgrade takes
/// the burn off rather than changing what it does.
#[test]
fn a_know_thy_place_is_the_debuffs_and_the_upgrade_keeps_the_card() {
    let mut simulator = fight(&[("CARD.KNOW_THY_PLACE", 0)]);
    play(&mut simulator, "CARD.KNOW_THY_PLACE");
    assert_eq!(power_amount(&simulator, 1, "POWER.WEAK_POWER"), 1);
    assert_eq!(power_amount(&simulator, 1, "POWER.VULNERABLE_POWER"), 1);
    assert_eq!(
        count_in(&simulator, PileName::Exhaust, "CARD.KNOW_THY_PLACE"),
        1
    );

    let mut simulator = fight(&[("CARD.KNOW_THY_PLACE", 1)]);
    play(&mut simulator, "CARD.KNOW_THY_PLACE");
    assert_eq!(
        count_in(&simulator, PileName::Discard, "CARD.KNOW_THY_PLACE"),
        1,
        "the upgraded form is discarded rather than burned"
    );
}

/// A conqueror forges and puts its own debuff on what it was aimed at, and
/// that debuff is worth double the blade's blow to whoever is carrying it.
#[test]
fn a_conqueror_doubles_the_blade_against_what_it_marked() {
    for (level, forged) in [(0, 3), (1, 5)] {
        let mut simulator = built(scenario(&[300, 300], &[("CARD.CONQUEROR", level)], &[], 20));
        play(&mut simulator, "CARD.CONQUEROR");
        assert_eq!(power_amount(&simulator, 1, "POWER.CONQUEROR_POWER"), 1);
        assert_eq!(
            power_amount(&simulator, 2, "POWER.CONQUEROR_POWER"),
            0,
            "and only on what it was aimed at"
        );
        let hp = hp_of(&simulator, 1);
        play(&mut simulator, "CARD.SOVEREIGN_BLADE");
        assert_eq!(
            hp - hp_of(&simulator, 1),
            2 * (10 + forged),
            "at level {level}"
        );
    }
}

/// A resonance lends its owner strength and takes a point off everything still
/// standing.
#[test]
fn a_resonance_trades_strength_across_the_field() {
    for (level, gained) in [(0, 1), (1, 2)] {
        let mut simulator = crowd(&[("CARD.RESONANCE", level)]);
        play(&mut simulator, "CARD.RESONANCE");
        assert_eq!(power_amount(&simulator, 0, "POWER.STRENGTH_POWER"), gained);
        assert_eq!(power_amount(&simulator, 1, "POWER.STRENGTH_POWER"), -1);
        assert_eq!(power_amount(&simulator, 2, "POWER.STRENGTH_POWER"), -1);
    }
}

/// The cards that pay energy, now or next turn.
#[test]
fn the_cards_that_pay_energy() {
    for (model_id, level, now, next_turn) in [
        ("CARD.ALIGNMENT", 0, 2, 0),
        ("CARD.ALIGNMENT", 1, 3, 0),
        ("CARD.HEGEMONY", 0, 0, 2),
        ("CARD.HEGEMONY", 1, 0, 3),
        ("CARD.BIG_BANG", 0, 1, 0),
    ] {
        let mut simulator = fight(&[(model_id, level)]);
        let (cost, _) = costs(&simulator, model_id);
        let held = energy(&simulator);
        play(&mut simulator, model_id);
        assert_eq!(
            energy(&simulator),
            held - cost + now,
            "{model_id} at level {level}"
        );
        assert_eq!(
            power_amount(&simulator, 0, "POWER.ENERGY_NEXT_TURN_POWER"),
            next_turn,
            "{model_id} at level {level}"
        );
    }
}

/// A glitterstream works out what next turn's block is worth before it gains
/// any of its own, and only then does this turn's block land.
#[test]
fn a_glitterstream_settles_next_turns_block_first() {
    for (level, now, next_turn) in [(0, 11, 5), (1, 13, 7)] {
        let mut simulator = fight(&[("CARD.GLITTERSTREAM", level)]);
        play(&mut simulator, "CARD.GLITTERSTREAM");
        assert_eq!(block(&simulator), now, "at level {level}");
        assert_eq!(
            power_amount(&simulator, 0, "POWER.BLOCK_NEXT_TURN_POWER"),
            next_turn,
            "at level {level}"
        );
    }
}

/// The cards whose body is block and something else beside it.
#[test]
fn the_cards_that_are_block_and_something_else() {
    for (model_id, level, expected) in [
        ("CARD.GATHER_LIGHT", 0, 8),
        ("CARD.GATHER_LIGHT", 1, 11),
        ("CARD.PATTER", 0, 8),
        ("CARD.PATTER", 1, 10),
        ("CARD.MANIFEST_AUTHORITY", 0, 7),
        ("CARD.MANIFEST_AUTHORITY", 1, 8),
        ("CARD.BULWARK", 0, 12),
        ("CARD.BULWARK", 1, 15),
        ("CARD.COSMIC_INDIFFERENCE", 0, 6),
        ("CARD.COSMIC_INDIFFERENCE", 1, 9),
    ] {
        let mut simulator = fight(&[(model_id, level)]);
        play(&mut simulator, model_id);
        assert_eq!(block(&simulator), expected, "{model_id} at level {level}");
    }
}

/// A lunar blast hits once for every skill play its owner has finished this
/// turn, so one played with no skill behind it lands nothing at all.
#[test]
fn a_lunar_blast_counts_the_turns_skills() {
    let mut simulator = built(scenario(&[200], &[("CARD.LUNAR_BLAST", 0)], &[], 20));
    let hp = hp_of(&simulator, 1);
    play(&mut simulator, "CARD.LUNAR_BLAST");
    assert_eq!(hp - hp_of(&simulator, 1), 0);

    let mut simulator = built(scenario(
        &[200],
        &[
            ("CARD.DEFEND_REGENT", 0),
            ("CARD.VENERATE", 0),
            ("CARD.LUNAR_BLAST", 0),
        ],
        &[],
        20,
    ));
    play(&mut simulator, "CARD.DEFEND_REGENT");
    play(&mut simulator, "CARD.VENERATE");
    let hp = hp_of(&simulator, 1);
    play(&mut simulator, "CARD.LUNAR_BLAST");
    assert_eq!(hp - hp_of(&simulator, 1), 8, "two hits of four");
}

/// A heavenly drill hits once for every point of energy the play spent, and
/// twice as often where that was four or more.
#[test]
fn a_heavenly_drill_doubles_itself_at_four_energy() {
    let mut simulator = built(scenario(&[200], &[("CARD.HEAVENLY_DRILL", 0)], &[], 0).energy(3, 3));
    let hp = hp_of(&simulator, 1);
    play(&mut simulator, "CARD.HEAVENLY_DRILL");
    assert_eq!(hp - hp_of(&simulator, 1), 24, "three hits of eight");

    let mut simulator = built(scenario(&[200], &[("CARD.HEAVENLY_DRILL", 0)], &[], 0).energy(4, 4));
    let hp = hp_of(&simulator, 1);
    play(&mut simulator, "CARD.HEAVENLY_DRILL");
    assert_eq!(hp - hp_of(&simulator, 1), 64, "eight hits of eight");
}

/// A supermassive is worth five and more for every card its owner has had
/// conjured into the fight.
#[test]
fn a_supermassive_counts_the_cards_conjured_into_the_fight() {
    let mut simulator = built(scenario(&[200], &[("CARD.SUPERMASSIVE", 0)], &[], 20));
    let hp = hp_of(&simulator, 1);
    play(&mut simulator, "CARD.SUPERMASSIVE");
    assert_eq!(hp - hp_of(&simulator, 1), 5);

    let mut simulator = built(scenario(
        &[200],
        &[("CARD.COLLISION_COURSE", 0), ("CARD.SUPERMASSIVE", 0)],
        &[],
        20,
    ));
    play(&mut simulator, "CARD.COLLISION_COURSE");
    let hp = hp_of(&simulator, 1);
    play(&mut simulator, "CARD.SUPERMASSIVE");
    assert_eq!(
        hp - hp_of(&simulator, 1),
        8,
        "five, and three for the debris the collision conjured"
    );
}

/// A particle wall stays in the hand instead of being discarded.
#[test]
fn a_particle_wall_keeps_itself_in_hand() {
    let mut simulator = fight(&[("CARD.PARTICLE_WALL", 0)]);
    play(&mut simulator, "CARD.PARTICLE_WALL");
    assert_eq!(
        count_in(&simulator, PileName::Hand, "CARD.PARTICLE_WALL"),
        1
    );
    assert_eq!(pile(&simulator, PileName::Discard), [] as [String; 0]);
}

/// A shining strike puts itself back on top of the draw pile rather than being
/// discarded.
#[test]
fn a_shining_strike_puts_itself_back_on_the_draw_pile() {
    let simulator = fight_with_stars(&[("CARD.SHINING_STRIKE", 0)], 0);
    let mut state = simulator.state().clone();
    let shining = state
        .combat
        .as_mut()
        .expect("the fight is standing")
        .player
        .piles
        .hand
        .first_mut()
        .expect("the Shining Strike is held");
    shining.energy_cost = 0;
    shining
        .temporary_costs
        .push(sts2_engine::TemporaryCost::free_this_turn());
    shining
        .temporary_star_costs
        .push(sts2_engine::TemporaryCost::free_this_turn());
    let mut simulator = Simulator::from_scenario(state, sts2_content::standard_registry())
        .expect("the free-card scenario rebuilds");
    play(&mut simulator, "CARD.SHINING_STRIKE");
    assert_eq!(stars(&simulator), 2);
    assert_eq!(
        pile(&simulator, PileName::Draw).first().map(String::as_str),
        Some("CARD.SHINING_STRIKE")
    );
    let returned = simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles
        .draw
        .first()
        .expect("the Shining Strike returned");
    assert_eq!(returned.energy_cost, 1, "its temporary free cost cleared");
    assert_eq!(returned.temporary_costs.as_slice(), []);
}

/// A kingly kick takes a point off its own cost every time it is drawn, and the
/// discount stands for the rest of the fight.
#[test]
fn a_kingly_kick_is_cheaper_every_time_it_is_drawn() {
    let mut simulator = fight_with_draw(&[("CARD.PROPHESIZE", 0)], &[("CARD.KINGLY_KICK", 0)]);
    play(&mut simulator, "CARD.PROPHESIZE");
    assert_eq!(costs(&simulator, "CARD.KINGLY_KICK").0, 3);
}

/// A kingly punch grows its own blow every time it is drawn.
#[test]
fn a_kingly_punch_grows_every_time_it_is_drawn() {
    for (level, base, grown_by) in [(0, 8, 4), (1, 10, 6)] {
        let mut simulator = built(scenario(
            &[300],
            &[("CARD.PROPHESIZE", 0)],
            &[("CARD.KINGLY_PUNCH", level)],
            20,
        ));
        play(&mut simulator, "CARD.PROPHESIZE");
        let hp = hp_of(&simulator, 1);
        play(&mut simulator, "CARD.KINGLY_PUNCH");
        assert_eq!(
            hp - hp_of(&simulator, 1),
            base + grown_by,
            "at level {level}"
        );
    }
}

/// A prophesize is a draw and nothing else.
#[test]
fn a_prophesize_draws() {
    for (level, drawn) in [(0, 6), (1, 9)] {
        let draw = stack("CARD.STRIKE_REGENT", 9);
        let mut simulator = built(scenario(&[39], &[("CARD.PROPHESIZE", level)], &draw, 20));
        play(&mut simulator, "CARD.PROPHESIZE");
        assert_eq!(
            pile(&simulator, PileName::Hand).len(),
            drawn,
            "at level {level}"
        );
    }
}

/// A guiding star is a blow and a draw.
#[test]
fn a_guiding_star_draws_behind_its_blow() {
    for (level, damage, drawn) in [(0, 12, 2), (1, 13, 3)] {
        let draw = stack("CARD.STRIKE_REGENT", 3);
        let mut simulator = built(scenario(&[200], &[("CARD.GUIDING_STAR", level)], &draw, 20));
        let hp = hp_of(&simulator, 1);
        play(&mut simulator, "CARD.GUIDING_STAR");
        assert_eq!(hp - hp_of(&simulator, 1), damage, "at level {level}");
        assert_eq!(pile(&simulator, PileName::Hand).len(), drawn);
    }
}

/// A collision course leaves one piece of debris in the hand; a crash landing
/// fills whatever room the hand has left with them.
#[test]
fn the_cards_that_leave_debris() {
    let mut simulator = built(scenario(&[200], &[("CARD.COLLISION_COURSE", 0)], &[], 20));
    play(&mut simulator, "CARD.COLLISION_COURSE");
    assert_eq!(count_in(&simulator, PileName::Hand, "CARD.DEBRIS"), 1);

    let mut simulator = built(scenario(&[300, 300], &[("CARD.CRASH_LANDING", 0)], &[], 20));
    play(&mut simulator, "CARD.CRASH_LANDING");
    assert_eq!(
        count_in(&simulator, PileName::Hand, "CARD.DEBRIS"),
        10,
        "the hand is filled to its cap"
    );
}

/// A big bang draws, pays a star and an energy, forges, and burns up.
#[test]
fn a_big_bang_pays_everything_at_once() {
    let mut simulator = fight_with_draw(&[("CARD.BIG_BANG", 0)], &[("CARD.STRIKE_REGENT", 0)]);
    let held = stars(&simulator);
    play(&mut simulator, "CARD.BIG_BANG");
    assert_eq!(stars(&simulator), held + 1);
    assert_eq!(
        count_in(&simulator, PileName::Hand, "CARD.STRIKE_REGENT"),
        1
    );
    assert_eq!(
        count_in(&simulator, PileName::Hand, "CARD.SOVEREIGN_BLADE"),
        1
    );
    assert_eq!(count_in(&simulator, PileName::Exhaust, "CARD.BIG_BANG"), 1);
}

/// A void form makes its owner's next cards free and ends the turn the moment
/// it lands.
#[test]
fn a_void_form_ends_the_turn_the_moment_it_lands() {
    let mut simulator = fight(&[("CARD.VOID_FORM", 0)]);
    play(&mut simulator, "CARD.VOID_FORM");
    assert_eq!(power_amount(&simulator, 0, "POWER.VOID_FORM_POWER"), 2);
    assert_eq!(
        simulator
            .state()
            .combat
            .as_ref()
            .expect("the fight is standing")
            .player
            .turn,
        2,
        "the turn handed over behind the play"
    );
}

// --- the cards that put a screen up ---------------------------------------

/// A begone turns one card of the hand into a minion's strike.
#[test]
fn a_begone_turns_a_card_of_the_hand_into_a_minions_strike() {
    let mut simulator = fight(&[
        ("CARD.BEGONE", 0),
        ("CARD.DEFEND_REGENT", 0),
        ("CARD.STRIKE_REGENT", 0),
    ]);
    play(&mut simulator, "CARD.BEGONE");
    choose(&mut simulator, "CARD.DEFEND_REGENT");
    assert_eq!(
        count_in(&simulator, PileName::Hand, "CARD.MINION_STRIKE"),
        1
    );
    assert_eq!(
        count_in(&simulator, PileName::Hand, "CARD.DEFEND_REGENT"),
        0
    );
    assert_eq!(
        count_in(&simulator, PileName::Hand, "CARD.STRIKE_REGENT"),
        1
    );
}

/// An upgraded begone makes an upgraded minion's strike; an ordinary one makes
/// an ordinary strike. The token is upgraded as it is made, before it is put in
/// the chosen card's place, so what the upgrade reaches is the strike and not
/// the card it replaced.
#[test]
fn a_begone_upgrades_the_strike_it_makes_where_it_is_upgraded_itself() {
    for (begone_level, strike_level) in [(0, 0), (1, 1)] {
        let mut simulator = fight(&[
            ("CARD.BEGONE", begone_level),
            ("CARD.DEFEND_REGENT", 0),
            ("CARD.STRIKE_REGENT", 0),
        ]);
        play(&mut simulator, "CARD.BEGONE");
        choose(&mut simulator, "CARD.DEFEND_REGENT");
        assert_eq!(
            upgrade_levels_in(&simulator, PileName::Hand, "CARD.MINION_STRIKE"),
            vec![strike_level],
            "the strike a begone at {begone_level} makes"
        );
    }
}

/// A charge turns two cards of the draw pile into a minion's dive bomb.
#[test]
fn a_charge_turns_two_cards_of_the_draw_pile_into_dive_bombs() {
    let mut simulator = fight_with_draw(
        &[("CARD.CHARGE", 0)],
        &[("CARD.DEFEND_REGENT", 0), ("CARD.STRIKE_REGENT", 0)],
    );
    play(&mut simulator, "CARD.CHARGE");
    assert_eq!(
        count_in(&simulator, PileName::Draw, "CARD.MINION_DIVE_BOMB"),
        2
    );
}

/// A charge transforms the cards it was handed one at a time, in the order
/// they were named, so the dive bombs are made in that order too: the first
/// card named is the first dive bomb made, wherever in the pile it stands.
#[test]
fn a_charge_makes_its_dive_bombs_in_the_order_the_cards_were_named() {
    let mut simulator = fight_with_draw(
        &[("CARD.CHARGE", 0)],
        &[
            ("CARD.DEFEND_REGENT", 0),
            ("CARD.STRIKE_REGENT", 0),
            ("CARD.ASCENDERS_BANE", 0),
        ],
    );
    play(&mut simulator, "CARD.CHARGE");
    choose_in_order(
        &mut simulator,
        &["CARD.ASCENDERS_BANE", "CARD.DEFEND_REGENT"],
    );
    let draw = ids_in(&simulator, PileName::Draw);
    assert_eq!(
        draw.iter()
            .map(|(_, model)| model.as_str())
            .collect::<Vec<_>>(),
        vec![
            "CARD.MINION_DIVE_BOMB",
            "CARD.STRIKE_REGENT",
            "CARD.MINION_DIVE_BOMB"
        ],
        "each dive bomb stands where the card it replaced stood"
    );
    assert!(
        draw[2].0 < draw[0].0,
        "the bane was named first, so its dive bomb was made first: {draw:?}"
    );
}

/// A charge's two cards as the build records them: each named by its combat
/// card id and fingerprint, in the order they were picked.
fn recorded_charge(
    choice_id: sts2_core::ChoiceId,
    cards: &[sts2_engine::CardHandle],
) -> sts2_replay::Record {
    sts2_replay::Record {
        sequence: 1,
        kind: "choice.cards".into(),
        payload: serde_json::json!({
            "actor": "p0",
            "data": {
                "cards": cards.iter().map(|card| serde_json::json!({
                    "combat_card_index": card.card_id.get(),
                    "fingerprint": sts2_replay::observation::render_card_fingerprint(&card.fingerprint),
                })).collect::<Vec<_>>(),
                "choice_id": choice_id.get(),
                "choice_type": "combat_card",
                "offered": null,
            },
            "extensions": {},
        }),
    }
}

/// Plays a charge over a draw pile of `size` and replays a recorded answer
/// naming two of its cards out of pile order. Returns the action list the
/// charge stood with, the recorded action, and the action replay applied.
fn replay_a_charge_over(size: usize) -> (Vec<Action>, Action, Action, Simulator) {
    let draw: Vec<(&str, u8)> = (0..size)
        .map(|index| {
            if index % 2 == 0 {
                ("CARD.STRIKE_REGENT", 0)
            } else {
                ("CARD.DEFEND_REGENT", 0)
            }
        })
        .collect();
    let mut simulator = fight_with_draw(&[("CARD.CHARGE", 0)], &draw);
    play(&mut simulator, "CARD.CHARGE");
    let sts2_engine::DecisionContext::ChooseCards {
        choice_id,
        candidates,
        ..
    } = simulator.decision().clone()
    else {
        panic!("the charge names its cards: {:?}", simulator.decision());
    };
    assert_eq!(
        candidates.len(),
        size,
        "every card of the pile is a candidate"
    );
    let named = vec![candidates[9].clone(), candidates[2].clone()];
    let recorded = Action::ChooseCards {
        choice_id,
        cards: named.clone(),
    };
    let listed = simulator.legal_actions().to_vec();
    let mut adapter = sts2_replay::ReplayAdapter::new(simulator);
    let sts2_replay::AdapterOutcome::Applied(transition) = adapter
        .accept(&recorded_charge(choice_id, &named))
        .expect("the recorded charge applies")
    else {
        panic!("the recorded charge was not applied");
    };
    (
        listed,
        recorded,
        transition.accepted_action,
        adapter.simulator().clone(),
    )
}

/// A recorded charge over a draw pile of more than a dozen is replayed in the
/// order it was recorded. The action list carries each pair once, in pile
/// order, so the recorded order is not on it — but the dive bombs are made in
/// the order the cards were named, so replay keeps that order.
#[test]
fn a_recorded_charge_is_replayed_in_the_order_it_was_named() {
    let (listed, recorded, applied, after) = replay_a_charge_over(14);
    assert!(
        !listed.contains(&recorded),
        "the list does not carry this order"
    );
    assert_eq!(applied, recorded, "the record's own order");
    assert_eq!(count_in(&after, PileName::Draw, "CARD.MINION_DIVE_BOMB"), 2);
}

/// Over a pile of ninety-two the pairs are past the cap (C(92, 2) = 4186), so
/// the action list grows the selection a card at a time and carries no pair
/// at all. A recorded charge names its pair in one record and is replayed
/// all the same.
#[test]
fn a_recorded_charge_past_the_cap_is_replayed_in_one_record() {
    let (listed, recorded, applied, after) = replay_a_charge_over(92);
    assert!(
        listed
            .iter()
            .filter(|action| matches!(action, Action::ChooseCards { .. }))
            .all(|action| matches!(action, Action::ChooseCards { cards, .. } if cards.len() == 1)),
        "past the cap the list carries no whole pair"
    );
    assert_eq!(applied, recorded, "the record's own order");
    assert_eq!(count_in(&after, PileName::Draw, "CARD.MINION_DIVE_BOMB"), 2);
}

/// A card reads its numbers as each command of its body is reached, so a card
/// that gets its upgrade back while its own play is under way finishes the play
/// upgraded. A crush under dampened to the plain card strikes for seven, kills
/// the knight that was dampening it, and saps two strength from what is left —
/// the upgraded card's number. Seed `H9CTP910L8`, record 1542.
#[test]
fn a_crush_under_that_kills_its_dampener_saps_as_the_upgraded_card() {
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.KNIGHTS_ELITE"))
        .played_as(id("CHARACTER.REGENT"))
        .player_hp(75, 75)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatTargets, 0)
        .enemy(1, id("MONSTER.FLAIL_KNIGHT"), 90, 90, "RAM_MOVE")
        .enemy(2, id("MONSTER.MAGI_KNIGHT"), 5, 80, "DAMPEN_MOVE")
        .card(PileName::Draw, 0, card("CARD.CRUSH_UNDER", 1), 1)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    end_turn(&mut simulator);
    let held = &simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight is standing")
        .player
        .piles
        .hand;
    assert_eq!(held.len(), 1);
    assert_eq!(
        held[0].fingerprint.upgrade_level, 0,
        "the dampen took the upgrade off"
    );
    let hp = hp_of(&simulator, 1);
    play(&mut simulator, "CARD.CRUSH_UNDER");
    assert_eq!(hp - hp_of(&simulator, 1), 7, "the blow is the plain card's");
    assert_eq!(
        power_amount(&simulator, 1, "POWER.CRUSH_UNDER_POWER"),
        2,
        "and the strength it saps is the upgraded card's"
    );
}

/// A guards over a hand of seven has more orders than the action list lays
/// out, so the list carries each set once — and the screen still takes the set
/// in whatever order it was picked, because the sacrifices are made in that
/// order. Seed `PCFYFYGRAS`, record 2171: four of seven named against hand
/// order.
#[test]
fn a_guards_over_a_big_hand_takes_its_cards_in_the_order_they_were_named() {
    let mut simulator = fight(&[
        ("CARD.GUARDS", 0),
        ("CARD.DEFEND_REGENT", 0),
        ("CARD.STRIKE_REGENT", 0),
        ("CARD.FALLING_STAR", 0),
        ("CARD.VENERATE", 0),
        ("CARD.ASCENDERS_BANE", 0),
        ("CARD.GLOW", 0),
        ("CARD.PARTICLE_WALL", 0),
    ]);
    play(&mut simulator, "CARD.GUARDS");
    let named = ["CARD.GLOW", "CARD.STRIKE_REGENT", "CARD.DEFEND_REGENT"];
    let set = simulator
        .legal_actions()
        .iter()
        .find_map(|action| match action {
            Action::ChooseCards { choice_id, cards }
                if cards.len() == named.len()
                    && named.iter().all(|model| {
                        cards
                            .iter()
                            .any(|card| card.fingerprint.model_id == id(model))
                    }) =>
            {
                Some((*choice_id, cards.clone()))
            }
            _ => None,
        })
        .expect("the set is on the list");
    let (choice_id, cards) = set;
    assert_eq!(
        cards[0].fingerprint.model_id,
        id("CARD.DEFEND_REGENT"),
        "the list carries the set in hand order"
    );
    let in_order: Vec<_> = named
        .iter()
        .map(|model| {
            cards
                .iter()
                .find(|card| card.fingerprint.model_id == id(model))
                .expect("the card is in the set")
                .clone()
        })
        .collect();
    simulator
        .step(Action::ChooseCards {
            choice_id,
            cards: in_order,
        })
        .expect("an order of a set the list carries is an answer");
    let hand = ids_in(&simulator, PileName::Hand);
    let sacrifice_at = |index: usize| {
        assert_eq!(hand[index].1, "CARD.MINION_SACRIFICE", "{hand:?}");
        hand[index].0
    };
    // The hand stood defend, strike, star, venerate, bane, glow, wall.
    let (defend, strike, glow) = (sacrifice_at(0), sacrifice_at(1), sacrifice_at(5));
    assert!(
        glow < strike && strike < defend,
        "the glow was named first and the defend last: {hand:?}"
    );

    // A card the screen never offered is still refused.
    let mut refused = fight(&[
        ("CARD.GUARDS", 0),
        ("CARD.DEFEND_REGENT", 0),
        ("CARD.STRIKE_REGENT", 0),
        ("CARD.FALLING_STAR", 0),
        ("CARD.VENERATE", 0),
        ("CARD.ASCENDERS_BANE", 0),
        ("CARD.GLOW", 0),
        ("CARD.PARTICLE_WALL", 0),
    ]);
    play(&mut refused, "CARD.GUARDS");
    let (choice_id, mut cards) = refused
        .legal_actions()
        .iter()
        .find_map(|action| match action {
            Action::ChooseCards { choice_id, cards } if cards.len() == 2 => {
                Some((*choice_id, cards.clone()))
            }
            _ => None,
        })
        .expect("a pair is on the list");
    cards[1] = cards[0].clone();
    assert!(
        refused
            .step(Action::ChooseCards { choice_id, cards })
            .is_err(),
        "the same card twice is not an answer"
    );
}

/// An upgraded charge upgrades each dive bomb as it is made. Seed `2CKB2N4LNB`,
/// record 1993.
#[test]
fn an_upgraded_charge_makes_upgraded_dive_bombs() {
    for level in [0, 1] {
        let mut simulator = fight_with_draw(
            &[("CARD.CHARGE", level)],
            &[("CARD.DEFEND_REGENT", 0), ("CARD.STRIKE_REGENT", 0)],
        );
        play(&mut simulator, "CARD.CHARGE");
        let draw = &simulator
            .state()
            .combat
            .as_ref()
            .expect("the fight is standing")
            .player
            .piles
            .draw;
        assert_eq!(draw.len(), 2);
        for bomb in draw {
            assert_eq!(bomb.fingerprint.model_id, id("CARD.MINION_DIVE_BOMB"));
            assert_eq!(
                bomb.fingerprint.upgrade_level, level,
                "a charge at level {level}"
            );
        }
    }
}

/// A charge over a draw pile of exactly two cards asks nothing: the build
/// answers the screen itself with the pile as it stands, in pile order, and not
/// in the order the screen would have laid the cards out in. The first card of
/// the pile is the first dive bomb made.
#[test]
fn a_charge_over_a_pile_of_two_takes_them_in_pile_order() {
    let mut simulator = fight_with_draw(
        &[("CARD.CHARGE", 0)],
        &[("CARD.SHINING_STRIKE", 0), ("CARD.STRIKE_REGENT", 0)],
    );
    play(&mut simulator, "CARD.CHARGE");
    assert!(
        !matches!(
            simulator.decision(),
            sts2_engine::DecisionContext::ChooseCards { .. }
        ),
        "a pile of two is taken without a screen"
    );
    let draw = ids_in(&simulator, PileName::Draw);
    assert_eq!(
        draw.iter()
            .map(|(_, model)| model.as_str())
            .collect::<Vec<_>>(),
        vec!["CARD.MINION_DIVE_BOMB", "CARD.MINION_DIVE_BOMB"]
    );
    assert!(
        draw[0].0 < draw[1].0,
        "the shining strike stood first in the pile, so its dive bomb was made first: {draw:?}"
    );
}

/// A guards turns as many cards of the hand as its owner names into a minion's
/// sacrifice.
#[test]
fn a_guards_turns_as_many_cards_as_are_named_into_sacrifices() {
    let mut simulator = fight(&[
        ("CARD.GUARDS", 0),
        ("CARD.DEFEND_REGENT", 0),
        ("CARD.STRIKE_REGENT", 0),
    ]);
    play(&mut simulator, "CARD.GUARDS");
    choose(&mut simulator, "CARD.DEFEND_REGENT");
    assert_eq!(
        count_in(&simulator, PileName::Hand, "CARD.MINION_SACRIFICE"),
        1
    );
    assert_eq!(
        count_in(&simulator, PileName::Hand, "CARD.STRIKE_REGENT"),
        1
    );
}

/// An upgraded guards makes upgraded sacrifices, one for every card named.
#[test]
fn a_guards_upgrades_the_sacrifices_it_makes_where_it_is_upgraded_itself() {
    for (guards_level, sacrifice_level) in [(0, 0), (1, 1)] {
        let mut simulator = fight(&[
            ("CARD.GUARDS", guards_level),
            ("CARD.DEFEND_REGENT", 0),
            ("CARD.STRIKE_REGENT", 0),
        ]);
        play(&mut simulator, "CARD.GUARDS");
        choose(&mut simulator, "CARD.DEFEND_REGENT");
        assert_eq!(
            upgrade_levels_in(&simulator, PileName::Hand, "CARD.MINION_SACRIFICE"),
            vec![sacrifice_level],
            "the sacrifice a guards at {guards_level} makes"
        );
    }
}

/// A cosmic indifference puts one card of the discard pile back on top of the
/// draw pile.
#[test]
fn a_cosmic_indifference_puts_a_discard_back_on_top() {
    let mut simulator = fight(&[
        ("CARD.STRIKE_REGENT", 0),
        ("CARD.DEFEND_REGENT", 0),
        ("CARD.COSMIC_INDIFFERENCE", 0),
    ]);
    play(&mut simulator, "CARD.STRIKE_REGENT");
    play(&mut simulator, "CARD.DEFEND_REGENT");
    play(&mut simulator, "CARD.COSMIC_INDIFFERENCE");
    choose(&mut simulator, "CARD.STRIKE_REGENT");
    assert_eq!(
        pile(&simulator, PileName::Draw).first().map(String::as_str),
        Some("CARD.STRIKE_REGENT")
    );
}

/// A glimmer draws and then puts one card of the hand back on top of the draw
/// pile; a photon cut is the same shape behind a blow.
#[test]
fn the_cards_that_put_a_card_of_the_hand_back_on_top() {
    for model_id in ["CARD.GLIMMER", "CARD.PHOTON_CUT"] {
        let draw = stack("CARD.DEFEND_REGENT", 4);
        let mut simulator = built(scenario(
            &[300],
            &[(model_id, 0), ("CARD.STRIKE_REGENT", 0)],
            &draw,
            20,
        ));
        play(&mut simulator, model_id);
        choose(&mut simulator, "CARD.STRIKE_REGENT");
        assert_eq!(
            pile(&simulator, PileName::Draw).first().map(String::as_str),
            Some("CARD.STRIKE_REGENT"),
            "{model_id}"
        );
    }
}

/// A heirloom hammer is a blow and a copy of one colorless card of the hand.
/// A status card is not colorless and is never a candidate.
#[test]
fn a_heirloom_hammer_copies_a_colorless_card_of_the_hand() {
    for (level, damage) in [(0, 20), (1, 25)] {
        let mut simulator = built(scenario(
            &[300],
            &[
                ("CARD.HEIRLOOM_HAMMER", level),
                ("CARD.FINESSE", 0),
                ("CARD.FLASH_OF_STEEL", 0),
            ],
            &[],
            20,
        ));
        let hp = hp_of(&simulator, 1);
        play(&mut simulator, "CARD.HEIRLOOM_HAMMER");
        assert_eq!(hp - hp_of(&simulator, 1), damage, "at level {level}");
        choose(&mut simulator, "CARD.FINESSE");
        assert_eq!(
            count_in(&simulator, PileName::Hand, "CARD.FINESSE"),
            2,
            "the colorless card it copied, and the copy"
        );
    }
}

/// The cards that conjure colorless cards into the hand.
#[test]
fn the_cards_that_conjure_colorless_cards() {
    for (model_id, level, conjured) in [
        ("CARD.MANIFEST_AUTHORITY", 0, 1),
        ("CARD.MANIFEST_AUTHORITY", 1, 1),
        ("CARD.BUNDLE_OF_JOY", 0, 3),
        ("CARD.BUNDLE_OF_JOY", 1, 4),
    ] {
        let mut simulator = fight(&[(model_id, level)]);
        play(&mut simulator, model_id);
        assert_eq!(
            pile(&simulator, PileName::Hand).len(),
            conjured,
            "{model_id} at level {level}"
        );
    }
}

/// A quasar lays three colorless cards out and the one that is taken joins the
/// hand.
#[test]
fn a_quasar_lays_three_colorless_cards_out() {
    let mut simulator = fight(&[("CARD.QUASAR", 0)]);
    play(&mut simulator, "CARD.QUASAR");
    let offered = simulator
        .legal_actions()
        .iter()
        .filter(|action| matches!(action, Action::ChooseCards { cards, .. } if !cards.is_empty()))
        .count();
    assert_eq!(offered, 3, "three cards on the screen");
    choose_anything(&mut simulator);
    assert_eq!(pile(&simulator, PileName::Hand).len(), 1);
}

/// An upgraded quasar upgrades its three cards before the screen goes up, so
/// the one that is taken joins the hand upgraded. Seed `PCFYFYGRAS`, record
/// 1926, where the screen held three upgraded cards and the engine's held the
/// same three plain.
#[test]
fn an_upgraded_quasar_lays_its_cards_out_upgraded() {
    let offered_levels = |simulator: &Simulator| -> Vec<u8> {
        simulator
            .legal_actions()
            .iter()
            .filter_map(|action| match action {
                Action::ChooseCards { cards, .. } if cards.len() == 1 => {
                    Some(cards[0].fingerprint.upgrade_level)
                }
                _ => None,
            })
            .collect()
    };
    let mut plain = fight(&[("CARD.QUASAR", 0)]);
    play(&mut plain, "CARD.QUASAR");
    assert_eq!(offered_levels(&plain), vec![0, 0, 0]);

    let mut upgraded = fight(&[("CARD.QUASAR", 1)]);
    play(&mut upgraded, "CARD.QUASAR");
    let levels = offered_levels(&upgraded);
    assert_eq!(levels.len(), 3);
    // A card with no upgraded form stays as it is; the colorless pool holds
    // few enough of them that three at once does not happen on this seed.
    assert!(levels.contains(&1), "{levels:?}");
    choose_anything(&mut upgraded);
    let hand = pile(&upgraded, PileName::Hand);
    assert_eq!(hand.len(), 1);
}

/// A decisions decisions draws and then plays one skill of the hand three times
/// over.
#[test]
fn a_decisions_decisions_plays_one_skill_three_times() {
    let mut simulator = built(scenario(
        &[300],
        &[("CARD.DECISIONS_DECISIONS", 0)],
        &[("CARD.DEFEND_REGENT", 0), ("CARD.PARTICLE_WALL", 0)],
        20,
    ));
    play(&mut simulator, "CARD.DECISIONS_DECISIONS");
    choose(&mut simulator, "CARD.DEFEND_REGENT");
    assert_eq!(block(&simulator), 15, "three defends of five");
}

// --- the cards that play themselves ---------------------------------------

/// A bombardment standing in the exhaust pile plays itself again as the turn
/// opens.
#[test]
fn a_bombardment_plays_itself_out_of_the_exhaust_pile() {
    let mut simulator = built(scenario(&[300], &[("CARD.BOMBARDMENT", 0)], &[], 20));
    play(&mut simulator, "CARD.BOMBARDMENT");
    assert_eq!(
        count_in(&simulator, PileName::Exhaust, "CARD.BOMBARDMENT"),
        1
    );
    let hp = hp_of(&simulator, 1);
    end_turn(&mut simulator);
    assert_eq!(
        hp - hp_of(&simulator, 1),
        12,
        "it swung again as the new turn opened, at the seven tenths a shrink \
         leaves of a blow"
    );
}

/// An I am invincible standing on top of the draw pile plays itself as the
/// turn's acting ends.
#[test]
fn an_i_am_invincible_plays_itself_off_the_draw_pile() {
    let mut simulator = built(scenario(&[300], &[], &[("CARD.I_AM_INVINCIBLE", 0)], 20));
    end_turn(&mut simulator);
    assert_eq!(
        count_in(&simulator, PileName::Draw, "CARD.I_AM_INVINCIBLE"),
        0,
        "it played itself rather than staying on the pile"
    );
}

/// Every third skill play of the turn calls a make it so back into the hand.
#[test]
fn a_make_it_so_comes_back_on_every_third_skill() {
    let mut simulator = built(scenario(
        &[300],
        &[
            ("CARD.MAKE_IT_SO", 0),
            ("CARD.DEFEND_REGENT", 0),
            ("CARD.DEFEND_REGENT", 0),
            ("CARD.DEFEND_REGENT", 0),
        ],
        &[],
        20,
    ));
    play(&mut simulator, "CARD.MAKE_IT_SO");
    assert_eq!(
        count_in(&simulator, PileName::Discard, "CARD.MAKE_IT_SO"),
        1
    );
    play(&mut simulator, "CARD.DEFEND_REGENT");
    play(&mut simulator, "CARD.DEFEND_REGENT");
    assert_eq!(count_in(&simulator, PileName::Hand, "CARD.MAKE_IT_SO"), 0);
    play(&mut simulator, "CARD.DEFEND_REGENT");
    assert_eq!(
        count_in(&simulator, PileName::Hand, "CARD.MAKE_IT_SO"),
        1,
        "the third skill called it back"
    );
}

/// The two multiplayer-only cards his pool holds, which no
/// single-player run is ever offered. They are registered for what a transform
/// of one rolls against and for the pool's own membership, and one of them says
/// something on its own: a largesse is aimed at an ally, and a fight with no
/// ally has nothing to aim it at, so it is not a legal play at all.
#[test]
fn the_multiplayer_only_cards_are_in_the_pool_and_one_cannot_be_aimed() {
    let simulator = fight(&[("CARD.LARGESSE", 0), ("CARD.HAMMER_TIME", 0)]);
    assert!(
        play_action(&simulator, "CARD.LARGESSE").is_none(),
        "there is no ally to hand it to"
    );
    assert!(play_action(&simulator, "CARD.HAMMER_TIME").is_some());
}

/// A supermassive counts the cards its owner conjured and no others: what a
/// monster puts in the piles is made with no creator named, so a chomper's
/// three dazed leave it worth what it was. Seed `PCFYFYGRAS`, record 1301,
/// where two chompers' screeches were counted and the blow that left one on
/// twenty-one killed it instead.
#[test]
fn a_supermassive_does_not_count_what_a_monster_conjured() {
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.CHOMPERS_NORMAL"))
        .played_as(id("CHARACTER.REGENT"))
        .player_hp(75, 75)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatTargets, 0)
        .enemy(1, id("MONSTER.CHOMPER"), 200, 200, "SCREECH_MOVE")
        .card(PileName::Draw, 0, card("CARD.SUPERMASSIVE", 0), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    end_turn(&mut simulator);
    let combat = simulator.state().combat.as_ref().expect("the fight stands");
    assert_eq!(
        combat
            .player
            .piles
            .all_cards()
            .filter(|card| card.fingerprint.model_id == id("CARD.DAZED"))
            .count(),
        3,
        "the screech landed"
    );
    assert_eq!(combat.player.cards_generated_this_combat, 0);
    let hp = hp_of(&simulator, 1);
    play(&mut simulator, "CARD.SUPERMASSIVE");
    assert_eq!(
        hp - hp_of(&simulator, 1),
        5,
        "five, and nothing for the dazed"
    );
}
