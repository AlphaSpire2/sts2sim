//! Hive monsters whose behaviour no recorded run reaches.
//!
//! A recording is the default evidence for a content model, but the runs in the
//! corpus kill the Tunneler before it ever burrows, so what a burrow does — the
//! block it keeps through the turn, the stun when that block breaks, and the
//! block going with the power — is pinned here instead.
//!
//! A Thieving Hopper is the other one the corpus lets off lightly: what it
//! takes, what the fight owes for what got away, and what a theft does to the
//! deck it took the card out of are all pinned here.

use sts2_engine::{Action, PileName, ScenarioBuilder, Simulator};
use sts2_rng::RunStream;

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().unwrap()
}

fn card(model_id: &str) -> sts2_engine::CardFingerprint {
    let mut fingerprint = sts2_engine::CardFingerprint::base(id(model_id));
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

/// A Tunneler already standing on its burrow, with a hand that can break the
/// block it is about to put up.
fn burrowing_tunneler() -> Simulator {
    let mut scenario = ScenarioBuilder::new("BURROWTEST", id("ENCOUNTER.TUNNELER_WEAK"))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .relics(&[id("RELIC.BURNING_BLOOD")])
        .enemy(1, id("MONSTER.TUNNELER"), 87, 87, "BURROW_MOVE");
    for (card_id, model_id, cost) in [
        (0, "CARD.STRIKE_IRONCLAD", 1),
        (1, "CARD.STRIKE_IRONCLAD", 1),
        (2, "CARD.STRIKE_IRONCLAD", 1),
    ] {
        scenario = scenario.card(PileName::Hand, card_id, card(model_id), cost);
    }
    scenario.build(sts2_content::standard_registry()).unwrap()
}

fn enemy(simulator: &Simulator) -> &sts2_engine::Creature {
    simulator
        .state()
        .combat
        .as_ref()
        .unwrap()
        .creatures
        .iter()
        .find(|creature| creature.id.get() == 1)
        .unwrap()
}

fn end_turn(simulator: &mut Simulator) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::EndTurn { .. }))
        .cloned()
        .unwrap();
    simulator.step(action).unwrap();
}

fn strike(simulator: &mut Simulator) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::PlayCard { card, .. } if card.fingerprint.model_id == id("CARD.STRIKE_IRONCLAD")))
        .cloned()
        .unwrap();
    simulator.step(action).unwrap();
}

fn defend(simulator: &mut Simulator) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::PlayCard { card, .. } if card.fingerprint.model_id == id("CARD.DEFEND_IRONCLAD")))
        .cloned()
        .unwrap();
    simulator.step(action).unwrap();
}

/// A Rock Bowlbug's dizzy animation is the body of its one stunned turn. Once
/// that turn has passed, its next shown move is Headbutt; Dizzy is not a second
/// standalone turn in the machine.
#[test]
fn a_rock_bowlbug_resumes_headbutt_after_one_dizzy_turn() {
    let mut scenario = ScenarioBuilder::new("BOWLBUGSTUN", id("ENCOUNTER.BOWLBUGS_WEAK"))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(1, 1)
        .enemy(1, id("MONSTER.BOWLBUG_ROCK"), 45, 45, "HEADBUTT_MOVE")
        // ScenarioBuilder starts from a projected combat state, so spell out
        // the power that the monster's AfterAddedToRoom hook normally applies.
        .power(
            1,
            id("POWER.IMBALANCED_POWER"),
            1,
            sts2_engine::PowerType::Buff,
        );
    for card_id in 0..3 {
        scenario = scenario.card(PileName::Hand, card_id, card("CARD.DEFEND_IRONCLAD"), 1);
    }
    let mut simulator = scenario
        .build(sts2_content::standard_registry())
        .expect("the bowlbug fight builds");

    for _ in 0..3 {
        defend(&mut simulator);
    }
    end_turn(&mut simulator);
    assert_eq!(enemy(&simulator).move_state, "STUNNED");

    end_turn(&mut simulator);
    assert_eq!(
        enemy(&simulator).move_state,
        "HEADBUTT_MOVE",
        "the stun body clears off-balance and resumes the interrupted headbutt"
    );
}

#[test]
fn a_burrow_keeps_its_block_and_breaking_it_leaves_the_tunneler_dizzy() {
    let mut simulator = burrowing_tunneler();
    end_turn(&mut simulator);

    // The burrow puts the power on and then works up its block.
    let burrowed = enemy(&simulator);
    assert_eq!(burrowed.block, 32, "the burrow is thirty-two block");
    assert!(
        burrowed
            .powers
            .iter()
            .any(|power| power.model_id == id("POWER.BURROWED_POWER")),
        "and the power that keeps it"
    );

    // The power stops its owner's block from clearing, so the block is still
    // there once the enemy side has taken another turn.
    end_turn(&mut simulator);
    assert_eq!(enemy(&simulator).block, 32, "a burrowed creature keeps it");
    assert_eq!(
        enemy(&simulator).move_state,
        "BELOW_MOVE",
        "and the machine has moved on"
    );

    // Six strikes is thirty-six, which is more than the burrow can take.
    for _ in 0..3 {
        strike(&mut simulator);
    }
    end_turn(&mut simulator);
    for _ in 0..3 {
        strike(&mut simulator);
    }
    let broken = enemy(&simulator);
    assert_eq!(broken.block, 0, "the block goes with the power");
    assert!(
        !broken
            .powers
            .iter()
            .any(|power| power.model_id == id("POWER.BURROWED_POWER")),
        "which is spent breaking"
    );
    assert_eq!(broken.move_state, "STUNNED", "and the tunneler is dizzy");
    assert_eq!(
        broken.stunned_follow_up.as_deref(),
        Some("BITE_MOVE"),
        "picking its machine back up on the bite"
    );
}

/// A hopper standing on the escape its machine ends in, with a beetle beside it
/// so the fight is still standing once it has gone.
fn hopper_about_to_leave() -> Simulator {
    ScenarioBuilder::new("ESCAPETEST", id("ENCOUNTER.THIEVING_HOPPER_WEAK"))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .relics(&[id("RELIC.BURNING_BLOOD")])
        .enemy(1, id("MONSTER.THIEVING_HOPPER"), 79, 79, "ESCAPE_MOVE")
        .enemy(2, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .card(PileName::Hand, 0, card("CARD.STRIKE_IRONCLAD"), 1)
        .build(sts2_content::standard_registry())
        .unwrap()
}

fn combat(simulator: &Simulator) -> &sts2_engine::CombatState {
    simulator.state().combat.as_ref().unwrap()
}

/// An escape: the hopper leaves on its own feet. Nothing about it is a death —
/// the beetle beside it never answers for one — and the fight writes down that
/// it got away.
#[test]
fn a_hopper_that_has_had_enough_leaves_the_fight() {
    let mut simulator = hopper_about_to_leave();
    assert_eq!(
        combat(&simulator).spawned_enemy_models.len(),
        2,
        "the encounter laid down two monsters"
    );
    end_turn(&mut simulator);

    assert!(
        !combat(&simulator)
            .creatures
            .iter()
            .any(|creature| creature.id.get() == 1),
        "the hopper is out of the fight"
    );
    assert_eq!(
        combat(&simulator).escaped_creatures,
        vec![id("MONSTER.THIEVING_HOPPER")],
        "and the fight knows it left rather than died"
    );
    assert!(
        combat(&simulator)
            .creatures
            .iter()
            .any(|creature| creature.id.get() == 2),
        "the beetle keeps the fight standing"
    );

    // The escape follows itself, so a hopper goes on leaving and a second
    // enemy turn asks nothing more of it.
    end_turn(&mut simulator);
    assert_eq!(
        combat(&simulator).escaped_creatures.len(),
        1,
        "and it only leaves once"
    );
}

/// A monster room pays the share of its purse that matches the share of its
/// monsters that were put down. One of
/// two got away here, so half the purse is what is offered, and the half is
/// taken off both ends of the range before the roll.
#[test]
fn a_fight_pays_less_gold_for_what_got_away() {
    let mut state = sts2_content::final_boss_fixture()
        .expect("the fixture builds")
        .state()
        .clone();
    let mut combat = hopper_about_to_leave().state().combat.clone().unwrap();
    // A beetle one strike from going down, so the win can be read off the room
    // once the hopper has gone. The shrink it lands first is why one strike has
    // to be enough on its own.
    for creature in &mut combat.creatures {
        if creature.id.get() == 2 {
            creature.current_hp = 1;
        }
    }
    combat.generated_victory_rewards = Some(sts2_engine::GeneratedCombatRewardSpec {
        minimum_gold: 10,
        maximum_gold: 20,
        scales_with_poverty: true,
        rarity_odds: sts2_engine::RarityOdds::RegularEncounter,
        offers_relic: false,
    });
    state.combat = Some(combat);
    // The purse is a monster room's own: a boss pays its whole one however many
    // of its monsters walked away.
    let run = state.run.as_mut().expect("the fixture has a run");
    run.pushed_room = None;
    run.current_room_type = Some(sts2_engine::MapPointType::Monster);
    let mut simulator = Simulator::from_scenario(state, sts2_content::standard_registry())
        .expect("the fight stands up inside the run");

    // The hopper leaves on the enemy turn; the beetle is all that keeps the
    // fight alive, and one strike is enough to finish it.
    end_turn(&mut simulator);
    strike(&mut simulator);

    let rewards = simulator
        .state()
        .combat
        .as_ref()
        .and_then(|combat| combat.reward_set.as_ref())
        .or_else(|| {
            simulator
                .state()
                .run
                .as_ref()
                .and_then(|run| run.reward_set.as_ref())
        })
        .expect("the win laid out a reward set");
    let gold = rewards
        .rewards
        .iter()
        .find_map(|item| {
            (item.fingerprint.reward_type == "gold").then_some(item.fingerprint.gold_amount)
        })
        .expect("a monster room pays gold");
    assert!(
        (5..=10).contains(&gold),
        "half of ten to twenty is five to ten, and the purse paid {gold}"
    );
}

/// A Fat Gremlin has a share of its own rather than the ordinary escaped
/// creature's: one that escapes with stolen gold removes the room's base-gold
/// reward entirely.
#[test]
fn a_fat_gremlin_escaping_with_stolen_gold_takes_the_rooms_purse() {
    let mut state = sts2_content::final_boss_fixture()
        .expect("the fixture builds")
        .state()
        .clone();
    let mut combat = hopper_about_to_leave().state().combat.clone().unwrap();
    combat.encounter_model = id("ENCOUNTER.GREMLIN_MERC_NORMAL");
    combat.escaped_creatures = vec![id("MONSTER.FAT_GREMLIN")];
    combat.escaped_stolen_gold = 20;
    combat.generated_victory_rewards = Some(sts2_engine::GeneratedCombatRewardSpec {
        minimum_gold: 10,
        maximum_gold: 20,
        scales_with_poverty: true,
        rarity_odds: sts2_engine::RarityOdds::RegularEncounter,
        offers_relic: false,
    });
    combat.creatures.retain(|creature| creature.id.get() != 1);
    for creature in &mut combat.creatures {
        if creature.id.get() == 2 {
            creature.current_hp = 1;
        }
    }
    state.combat = Some(combat);
    let run = state.run.as_mut().expect("the fixture has a run");
    run.pushed_room = None;
    run.current_room_type = Some(sts2_engine::MapPointType::Monster);
    let mut simulator = Simulator::from_scenario(state, sts2_content::standard_registry())
        .expect("the fight stands up inside the run");
    strike(&mut simulator);
    let rewards = simulator
        .state()
        .combat
        .as_ref()
        .and_then(|combat| combat.reward_set.as_ref())
        .expect("the win laid out a reward set");
    assert!(
        rewards
            .rewards
            .iter()
            .all(|item| item.fingerprint.reward_type != "gold"),
        "the escaped stolen purse is not offered"
    );
}

/// A hopper standing on its theft, over a deck whose write-through card stands
/// behind what it is about to take. The thief takes the one uncommon in the
/// piles it looks at, which is the Refract in front of the algorithm.
fn hopper_about_to_steal() -> Simulator {
    let mut scenario = ScenarioBuilder::new("STEALTEST", id("ENCOUNTER.THIEVING_HOPPER_WEAK"))
        .played_as(id("CHARACTER.DEFECT"))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(1, 1)
        .orb_slots(3)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatCardGeneration, 0)
        .rng_counter(RunStream::Shuffle, 0)
        .enemy(1, id("MONSTER.THIEVING_HOPPER"), 79, 79, "THIEVERY_MOVE");
    for (card_id, model_id, cost) in [
        (0, "CARD.REFRACT", 3),
        (1, "CARD.GENETIC_ALGORITHM", 1),
        (2, "CARD.STRIKE_DEFECT", 1),
    ] {
        scenario = scenario.card(PileName::Draw, card_id, card(model_id), cost);
    }
    scenario.build(sts2_content::standard_registry()).unwrap()
}

/// A swipe's theft takes the deck card the stolen combat card was cloned from,
/// and takes nothing else with it. Every other combat card is
/// still linked to the deck card it was cloned from, so the Genetic Algorithm
/// left standing behind the gap writes its growth onto itself and not onto
/// whatever now stands where it used to.
#[test]
fn a_theft_does_not_move_what_the_rest_of_the_deck_writes_to() {
    let mut simulator = hopper_about_to_steal();
    end_turn(&mut simulator);

    let deck = &simulator.state().run_player.deck;
    assert_eq!(
        deck.iter()
            .map(|fingerprint| fingerprint.model_id.to_string())
            .collect::<Vec<_>>(),
        ["CARD.GENETIC_ALGORITHM", "CARD.STRIKE_DEFECT"],
        "the thief took the refract out of the deck"
    );

    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(
                action,
                Action::PlayCard { card, .. }
                    if card.fingerprint.model_id == id("CARD.GENETIC_ALGORITHM")
            )
        })
        .cloned()
        .expect("the algorithm was drawn back into the hand");
    simulator.step(action).expect("the algorithm applies");

    let deck = &simulator.state().run_player.deck;
    assert_eq!(
        deck[0].properties.get("IncreasedBlock"),
        Some(&serde_json::Value::from(3)),
        "the algorithm grew its own deck card"
    );
    assert!(
        deck[1].properties.is_empty(),
        "and the card behind it took nothing the theft handed it"
    );
}

/// What a thief carries, and hands back as a reward when it goes down, is the
/// deck's card and not the fight's copy of it: an upgrade that lasts only the
/// combat stays behind. Seed `TMP4ZDVAZQ`, record 776, where a compact a bellows
/// had upgraded for the fight came back upgraded.
#[test]
fn a_thief_carries_the_decks_card_not_the_fights_copy() {
    let mut simulator = ScenarioBuilder::new("STEALTEST", id("ENCOUNTER.THIEVING_HOPPER_WEAK"))
        .played_as(id("CHARACTER.DEFECT"))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(1, 1)
        .orb_slots(3)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatCardGeneration, 0)
        .rng_counter(RunStream::Shuffle, 0)
        .enemy(1, id("MONSTER.THIEVING_HOPPER"), 79, 79, "THIEVERY_MOVE")
        .card(PileName::Hand, 0, card("CARD.ARMAMENTS"), 1)
        .card(PileName::Hand, 1, card("CARD.REFRACT"), 3)
        .build(sts2_content::standard_registry())
        .unwrap();
    let armaments = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::PlayCard { card, .. }
                if card.fingerprint.model_id == id("CARD.ARMAMENTS"))
        })
        .cloned()
        .expect("the armaments is playable");
    simulator.step(armaments).expect("the armaments applies");
    let upgraded_in_the_fight = simulator
        .state()
        .combat
        .as_ref()
        .unwrap()
        .player
        .piles
        .hand
        .iter()
        .any(|card| {
            card.fingerprint.model_id == id("CARD.REFRACT") && card.fingerprint.upgrade_level == 1
        });
    assert!(
        upgraded_in_the_fight,
        "the refract is upgraded for the fight"
    );
    assert!(
        simulator
            .state()
            .run_player
            .deck
            .iter()
            .all(|card| card.upgrade_level == 0),
        "and the deck's refract is not"
    );

    end_turn(&mut simulator);
    let held = enemy(&simulator)
        .powers
        .iter()
        .find_map(|power| power.held_card.clone())
        .expect("the thief is carrying a card");
    assert_eq!(held.model_id, id("CARD.REFRACT"));
    assert_eq!(
        held.upgrade_level, 0,
        "the deck's card, as the deck held it"
    );
}

/// A Tunneler on the move it burrows from, with an expose in hand: block taken
/// off rather than hit off.
fn exposing_tunneler(move_state: &str) -> Simulator {
    ScenarioBuilder::new("EXPOSETEST", id("ENCOUNTER.TUNNELER_WEAK"))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .enemy(1, id("MONSTER.TUNNELER"), 87, 87, move_state)
        .card(PileName::Hand, 0, card("CARD.EXPOSE"), 0)
        .build(sts2_content::standard_registry())
        .unwrap()
}

fn expose(simulator: &mut Simulator) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::PlayCard { card, .. } if card.fingerprint.model_id == id("CARD.EXPOSE")))
        .cloned()
        .unwrap();
    simulator.step(action).unwrap();
}

/// Block a card takes away is broken block. An expose strips the burrow's
/// thirty-two off in one go, and the tunneler is left as dizzy as it would be
/// if the block had been beaten down.
#[test]
fn a_burrow_an_expose_strips_off_still_leaves_the_tunneler_dizzy() {
    let mut simulator = exposing_tunneler("BURROW_MOVE");
    end_turn(&mut simulator);
    assert_eq!(enemy(&simulator).block, 32, "the burrow is up");

    expose(&mut simulator);

    let stripped = enemy(&simulator);
    assert_eq!(stripped.block, 0, "the expose takes all of it");
    assert!(
        !stripped
            .powers
            .iter()
            .any(|power| power.model_id == id("POWER.BURROWED_POWER")),
        "the power is spent breaking, however the block went"
    );
    assert_eq!(stripped.move_state, "STUNNED", "and the tunneler is dizzy");
    assert_eq!(
        stripped.stunned_follow_up.as_deref(),
        Some("BITE_MOVE"),
        "picking its machine back up on the bite"
    );
}

/// The other side of it: a tunneler standing behind no block has none to break,
/// so an expose leaves it on the move it was on.
#[test]
fn an_expose_against_a_tunneler_with_no_block_stuns_nothing() {
    let mut simulator = exposing_tunneler("BITE_MOVE");
    assert_eq!(enemy(&simulator).block, 0, "nothing is up yet");

    expose(&mut simulator);

    let exposed = enemy(&simulator);
    assert_eq!(exposed.move_state, "BITE_MOVE", "the bite still stands");
    assert!(
        exposed
            .powers
            .iter()
            .any(|power| power.model_id == id("POWER.VULNERABLE_POWER")),
        "the expose still leaves it open"
    );
}
