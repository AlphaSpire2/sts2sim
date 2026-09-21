//! The orb subsystem, pinned one behaviour at a time.
//!
//! An orb only ever acts through its own passive and its own evoke, so every
//! test here stands a fight up with a queue already in it — or channels one —
//! and reads what came of the trigger, the evoke, or the slot being taken away.
//!
//! What the recordings witness is the ordinary run of it: three slots, orbs
//! channelled and evoked, and the turn-end tick. What is pinned here is the
//! edges no recorded run walks — a channel past a full queue, an evoke that
//! leaves the orb standing, the trigger count a relic raises, and the slots a
//! card takes away.

use sts2_engine::{Action, CardFingerprint, PileName, PowerType, ScenarioBuilder, Simulator};
use sts2_rng::RunStream;

const SEED: &str = "DEFECTORBS";

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str, upgrade_level: u8) -> CardFingerprint {
    let mut fingerprint = CardFingerprint::base(id(model_id));
    fingerprint.upgrade_level = upgrade_level;
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

/// A fight the test furnishes: the enemies, the slots, the orbs already
/// standing in them, the hand, and whatever the player is holding.
#[derive(Default)]
struct Fight<'a> {
    /// One entry per enemy, in slot order, each the HP that enemy stands with.
    enemies: &'a [i32],
    slots: i32,
    orbs: &'a [&'a str],
    hand: &'a [(&'a str, u8)],
    draw: &'a [(&'a str, u8)],
    powers: &'a [(u64, &'a str, i32, PowerType)],
    relics: &'a [&'a str],
    enemy_move: Option<&'a str>,
    turn: u32,
}

fn scenario(fight: &Fight) -> Simulator {
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .played_as(id("CHARACTER.DEFECT"))
        .player_hp(75, 75)
        .energy(9, 9)
        .turn(if fight.turn == 0 { 1 } else { fight.turn }, 1)
        .orb_slots(if fight.slots == 0 { 3 } else { fight.slots })
        .relics(
            &fight
                .relics
                .iter()
                .map(|relic| id(relic))
                .collect::<Vec<_>>(),
        )
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::CombatTargets, 0)
        .rng_counter(RunStream::CombatOrbs, 0)
        .rng_counter(RunStream::CombatCardSelection, 0);
    let enemies = if fight.enemies.is_empty() {
        &[39][..]
    } else {
        fight.enemies
    };
    for (index, enemy_hp) in enemies.iter().enumerate() {
        let slot = u64::try_from(index).expect("small index") + 1;
        scenario = scenario.enemy(
            slot,
            id("MONSTER.SHRINKER_BEETLE"),
            *enemy_hp,
            *enemy_hp,
            fight.enemy_move.unwrap_or("SHRINKER_MOVE"),
        );
    }
    for orb in fight.orbs {
        scenario = scenario.orb(id(orb));
    }
    let mut next_id = 0;
    for (model_id, upgrade) in fight.hand {
        scenario = scenario.card(PileName::Hand, next_id, card(model_id, *upgrade), 0);
        next_id += 1;
    }
    for (model_id, upgrade) in fight.draw {
        scenario = scenario.card(PileName::Draw, next_id, card(model_id, *upgrade), 0);
        next_id += 1;
    }
    for (creature, power, amount, power_type) in fight.powers {
        scenario = scenario.power(*creature, id(power), *amount, *power_type);
    }
    scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

fn holding_orbs(orbs: &[&str], hand: &[(&str, u8)]) -> Simulator {
    scenario(&Fight {
        orbs,
        hand,
        ..Fight::default()
    })
}

fn play(simulator: &mut Simulator, model_id: &str) {
    let wanted = id(model_id);
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::PlayCard { card, .. } if card.fingerprint.model_id == wanted)
        })
        .cloned()
        .unwrap_or_else(|| panic!("{model_id} was not a legal play"));
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

fn queue(simulator: &Simulator) -> &sts2_engine::OrbQueue {
    &simulator
        .state()
        .combat
        .as_ref()
        .expect("a combat is standing")
        .player
        .orbs
}

fn models(simulator: &Simulator) -> Vec<String> {
    queue(simulator)
        .orbs
        .iter()
        .map(|orb| orb.model_id.to_string())
        .collect()
}

fn creature(simulator: &Simulator, combat_id: u64) -> &sts2_engine::Creature {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("a combat is standing")
        .creatures
        .iter()
        .find(|creature| creature.id.get() == combat_id)
        .expect("the creature is in the fight")
}

fn player(simulator: &Simulator) -> &sts2_engine::PlayerCombatState {
    &simulator
        .state()
        .combat
        .as_ref()
        .expect("a combat is standing")
        .player
}

/// The Defect's own slot count is three: a fight opens with the character's
/// own slots and nothing standing in them.
#[test]
fn a_fight_opens_with_the_characters_own_slots() {
    let simulator = scenario(&Fight::default());
    assert_eq!(queue(&simulator).capacity, 3);
    assert!(queue(&simulator).orbs.is_empty());
}

/// Channelling puts the orb at the back of the queue, and a Zap says which
/// orb.
#[test]
fn a_channel_joins_the_back_of_the_queue() {
    let mut simulator = holding_orbs(&["ORB.FROST_ORB"], &[("CARD.ZAP", 0)]);
    play(&mut simulator, "CARD.ZAP");
    assert_eq!(models(&simulator), ["ORB.FROST_ORB", "ORB.LIGHTNING_ORB"]);
}

/// A channel into a full queue evokes the front to make room, and the new orb
/// joins behind what is left.
#[test]
fn channelling_past_a_full_queue_evokes_the_front() {
    let mut simulator = scenario(&Fight {
        slots: 2,
        orbs: &["ORB.FROST_ORB", "ORB.DARK_ORB"],
        hand: &[("CARD.ZAP", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.ZAP");
    // The Frost orb at the front was evoked to make room — five block — and the
    // Lightning orb joined behind the Dark one.
    assert_eq!(models(&simulator), ["ORB.DARK_ORB", "ORB.LIGHTNING_ORB"]);
    assert_eq!(creature(&simulator, 0).block, 5);
}

/// A Dualcast evokes the front orb twice, and only the second of the two takes
/// it out of the queue.
#[test]
fn a_dualcast_evokes_the_front_orb_twice_and_dequeues_it_once() {
    let mut simulator = holding_orbs(&["ORB.FROST_ORB", "ORB.DARK_ORB"], &[("CARD.DUALCAST", 0)]);
    play(&mut simulator, "CARD.DUALCAST");
    assert_eq!(models(&simulator), ["ORB.DARK_ORB"]);
    // Five block twice, because the same Frost orb answered both evokes.
    assert_eq!(creature(&simulator, 0).block, 10);
}

/// A Dualcast with nothing in the queue does nothing at all.
#[test]
fn a_dualcast_with_an_empty_queue_does_nothing() {
    let mut simulator = holding_orbs(&[], &[("CARD.DUALCAST", 0)]);
    play(&mut simulator, "CARD.DUALCAST");
    assert!(queue(&simulator).orbs.is_empty());
    assert_eq!(creature(&simulator, 0).block, 0);
}

/// Every orb of the queue fires its passive as the turn ends, in queue order.
/// A Frost orb is two block and a Lightning orb is
/// three damage.
#[test]
fn the_turn_end_pass_fires_every_orbs_passive_in_queue_order() {
    let mut simulator = holding_orbs(&["ORB.FROST_ORB", "ORB.LIGHTNING_ORB"], &[]);
    end_turn(&mut simulator);
    // The beetle's own move is a debuff, so the only damage in the fight is the
    // Lightning orb's three.
    assert_eq!(creature(&simulator, 1).current_hp, 36);
    // The two block the Frost orb worked up is cleared as the next turn opens,
    // so what is read here is the next turn's own tick rather than this one's.
    assert_eq!(models(&simulator), ["ORB.FROST_ORB", "ORB.LIGHTNING_ORB"]);
}

/// A Dark orb banks its passive into its own evoke rather than doing anything
/// when it fires: six, then six more every turn end.
#[test]
fn a_dark_orb_banks_its_passive_into_its_evoke() {
    let mut simulator = holding_orbs(&["ORB.DARK_ORB"], &[]);
    assert_eq!(queue(&simulator).orbs[0].evoke, 6);
    end_turn(&mut simulator);
    assert_eq!(queue(&simulator).orbs[0].evoke, 12);
    end_turn(&mut simulator);
    assert_eq!(queue(&simulator).orbs[0].evoke, 18);
}

/// A Glass orb wears itself down by one every time its passive fires, and its
/// evoke is worth double whatever the passive is worth now.
#[test]
fn a_glass_orb_wears_down_as_it_fires() {
    let mut simulator = holding_orbs(&["ORB.GLASS_ORB"], &[]);
    assert_eq!(queue(&simulator).orbs[0].passive, 4);
    assert_eq!(queue(&simulator).orbs[0].evoke, 8);
    end_turn(&mut simulator);
    assert_eq!(queue(&simulator).orbs[0].passive, 3);
    assert_eq!(queue(&simulator).orbs[0].evoke, 6);
    // Four damage the first time it fired.
    assert_eq!(creature(&simulator, 1).current_hp, 35);
}

/// A Plasma orb is the one orb whose trigger is the turn opening rather than
/// the turn ending, and its energy lands on top of the reset that refilled it.
#[test]
fn a_plasma_orb_pays_out_as_the_next_turn_opens() {
    let mut simulator = holding_orbs(&["ORB.PLASMA_ORB"], &[]);
    let before = player(&simulator).energy;
    assert_eq!(before, 9);
    end_turn(&mut simulator);
    // The reset put the maximum back and the orb added one to it.
    assert_eq!(player(&simulator).energy, 10);
}

/// Focus raises what an orb is worth the moment it lands, because an orb's
/// value is worked out afresh every time it is asked for.
#[test]
fn focus_raises_what_an_orb_is_worth() {
    let mut simulator = scenario(&Fight {
        orbs: &["ORB.LIGHTNING_ORB", "ORB.FROST_ORB", "ORB.PLASMA_ORB"],
        powers: &[(0, "POWER.FOCUS_POWER", 2, PowerType::Buff)],
        ..Fight::default()
    });
    let orbs = &queue(&simulator).orbs;
    assert_eq!((orbs[0].passive, orbs[0].evoke), (5, 10));
    assert_eq!((orbs[1].passive, orbs[1].evoke), (4, 7));
    // A Plasma orb offers neither of its numbers to the pass, so Focus is worth
    // nothing to it.
    assert_eq!((orbs[2].passive, orbs[2].evoke), (1, 2));
    end_turn(&mut simulator);
    // Five damage from the Lightning orb rather than three.
    assert_eq!(creature(&simulator, 1).current_hp, 34);
}

/// Focus worked below zero holds an orb's value at nothing rather than turning
/// it negative.
#[test]
fn focus_below_zero_holds_an_orb_at_nothing() {
    let simulator = scenario(&Fight {
        orbs: &["ORB.LIGHTNING_ORB"],
        powers: &[(0, "POWER.FOCUS_POWER", -9, PowerType::Debuff)],
        ..Fight::default()
    });
    let orbs = &queue(&simulator).orbs;
    assert_eq!((orbs[0].passive, orbs[0].evoke), (0, 0));
}

/// Gold-plated cables make the orb at the front of the queue fire its passive
/// twice, and nothing else does.
#[test]
fn gold_plated_cables_fires_the_front_orb_twice() {
    let mut simulator = scenario(&Fight {
        orbs: &["ORB.LIGHTNING_ORB", "ORB.LIGHTNING_ORB"],
        relics: &["RELIC.GOLD_PLATED_CABLES"],
        ..Fight::default()
    });
    end_turn(&mut simulator);
    // Three hits of three rather than two: the front orb went twice.
    assert_eq!(creature(&simulator, 1).current_hp, 30);
}

/// Slots are taken off the back, and any orb standing past the
/// new capacity comes off with them — with no evoke and no announcement.
#[test]
fn taking_a_slot_away_drops_the_orb_standing_in_it() {
    let mut simulator = scenario(&Fight {
        slots: 3,
        orbs: &["ORB.FROST_ORB", "ORB.FROST_ORB", "ORB.FROST_ORB"],
        hand: &[("CARD.BULK_UP", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.BULK_UP");
    assert_eq!(queue(&simulator).capacity, 2);
    assert_eq!(models(&simulator), ["ORB.FROST_ORB", "ORB.FROST_ORB"]);
    // Nothing was evoked, so no block was worked up.
    assert_eq!(creature(&simulator, 0).block, 0);
}

/// The queue is held at ten slots however many more are asked for.
#[test]
fn orb_slots_are_held_at_ten() {
    let mut simulator = scenario(&Fight {
        slots: 9,
        hand: &[("CARD.CAPACITOR", 0), ("CARD.CAPACITOR", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.CAPACITOR");
    assert_eq!(queue(&simulator).capacity, 10);
    play(&mut simulator, "CARD.CAPACITOR");
    assert_eq!(queue(&simulator).capacity, 10);
}

/// A Dark orb's evoke picks out whoever is standing on the least health, and
/// a tie goes to the first of them.
#[test]
fn a_dark_orb_evokes_onto_the_weakest_enemy() {
    let mut simulator = scenario(&Fight {
        enemies: &[39, 13, 13],
        orbs: &["ORB.DARK_ORB"],
        hand: &[("CARD.DUALCAST", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.DUALCAST");
    // Six twice on the first of the two thirteens, because a tie goes to the
    // first of them and the second evoke found it still the weakest.
    assert_eq!(creature(&simulator, 1).current_hp, 39);
    assert_eq!(creature(&simulator, 2).current_hp, 1);
    assert_eq!(creature(&simulator, 3).current_hp, 13);
}

/// A Tesla Coil fires every Lightning orb it has at whoever the play was aimed
/// at rather than at a rolled enemy.
#[test]
fn a_tesla_coil_aims_every_lightning_orb_at_its_target() {
    let mut simulator = scenario(&Fight {
        enemies: &[39, 39],
        orbs: &["ORB.LIGHTNING_ORB", "ORB.FROST_ORB", "ORB.LIGHTNING_ORB"],
        hand: &[("CARD.TESLA_COIL", 0)],
        ..Fight::default()
    });
    play(&mut simulator, "CARD.TESLA_COIL");
    // Three from the card and three from each of the two Lightning orbs, all on
    // the target; the Frost orb is not one of them.
    assert_eq!(creature(&simulator, 1).current_hp, 30);
    assert_eq!(creature(&simulator, 2).current_hp, 39);
}
