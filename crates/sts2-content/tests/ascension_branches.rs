//! The far side of the ascension branches the earlier chunks left unread.
//!
//! Twenty-one monsters of the acts already covered branch on the run's
//! ascension for their sizes and their damage. Their baseline numbers are
//! pinned elsewhere; this is what reads the other side of each branch: the
//! registry a run at ascension nine is played against, and a scenario driving
//! what each of those monsters does there.

use std::sync::Arc;

use sts2_engine::{
    Action, CombatSide, ContentRegistry, Creature, PileName, ScenarioBuilder, Simulator,
};
use sts2_rng::RunStream;

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().unwrap()
}

fn card(model_id: &str) -> sts2_engine::CardFingerprint {
    let mut fingerprint = sts2_engine::CardFingerprint::base(id(model_id));
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

/// The registry a run at this level is played against, which is where the
/// ascended monsters come from: nothing here patches a baseline registry
/// afterwards.
fn content_at(ascension: u8) -> Arc<ContentRegistry> {
    sts2_content::registry_at(&id("CHARACTER.IRONCLAD"), ascension)
}

fn fight(monster: &str, opener: &str, ascension: u8) -> Simulator {
    let mut scenario = ScenarioBuilder::new("ASCENSIONTEST", id("ENCOUNTER.NIBBITS_WEAK"))
        .player_hp(400, 400)
        .energy(3, 3)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .relics(&[id("RELIC.BURNING_BLOOD")])
        .enemy(1, id(monster), 400, 400, opener);
    for card_id in 0..5 {
        scenario = scenario.card(PileName::Hand, card_id, card("CARD.STRIKE_IRONCLAD"), 1);
    }
    scenario.build(content_at(ascension)).unwrap()
}

fn creature(simulator: &Simulator, side: CombatSide) -> &Creature {
    simulator
        .state()
        .combat
        .as_ref()
        .unwrap()
        .creatures
        .iter()
        .find(|creature| creature.side == side)
        .unwrap()
}

fn damage_of_one_turn(simulator: &mut Simulator) -> i32 {
    let before = creature(simulator, CombatSide::Player).current_hp;
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::EndTurn { .. }))
        .cloned()
        .unwrap();
    simulator.step(action).unwrap();
    before - creature(simulator, CombatSide::Player).current_hp
}

/// Every size these twenty-one roll at Tough Enemies, and the baseline size
/// beside it.
#[test]
fn the_earlier_monsters_roll_their_ascended_sizes() {
    let sizes = [
        ("MONSTER.SHRINKER_BEETLE", (38, 40), (40, 42)),
        ("MONSTER.KIN_PRIEST", (190, 190), (199, 199)),
        ("MONSTER.KIN_FOLLOWER", (58, 59), (62, 63)),
        ("MONSTER.FOGMOG", (74, 74), (78, 78)),
        ("MONSTER.INKLET", (11, 17), (12, 18)),
        ("MONSTER.BYRDONIS", (81, 84), (90, 90)),
        ("MONSTER.FLYCONID", (47, 49), (51, 53)),
        ("MONSTER.NIBBIT", (42, 46), (44, 48)),
        ("MONSTER.TWIG_SLIME_S", (7, 11), (8, 12)),
        ("MONSTER.LEAF_SLIME_S", (11, 15), (12, 16)),
        ("MONSTER.LEAF_SLIME_M", (32, 35), (33, 36)),
        ("MONSTER.TWIG_SLIME_M", (26, 28), (27, 29)),
        ("MONSTER.THIEVING_HOPPER", (79, 79), (84, 84)),
        ("MONSTER.EXOSKELETON", (24, 28), (25, 29)),
        ("MONSTER.HUNTER_KILLER", (121, 121), (126, 126)),
        ("MONSTER.CHOMPER", (60, 64), (63, 67)),
        ("MONSTER.BOWLBUG_SILK", (40, 43), (41, 44)),
        ("MONSTER.BOWLBUG_NECTAR", (35, 38), (36, 39)),
        ("MONSTER.MYTE", (61, 67), (64, 69)),
        ("MONSTER.CRUSHER", (209, 209), (219, 219)),
        ("MONSTER.ROCKET", (199, 199), (209, 209)),
    ];
    let baseline = sts2_content::standard_registry();
    let ascended = content_at(9);
    for (model, base, tough) in sizes {
        let plain = baseline
            .monster(&id(model))
            .expect("the monster is content");
        assert_eq!(
            (plain.min_initial_hp, plain.max_initial_hp),
            base,
            "{model}"
        );
        let raised = ascended
            .monster(&id(model))
            .expect("the monster is content");
        assert_eq!(
            (raised.min_initial_hp, raised.max_initial_hp),
            tough,
            "{model} at Tough Enemies"
        );
    }
}

/// And what each of them does at Deadly Enemies, driven a move at a time.
#[test]
fn the_earlier_monsters_hit_for_their_ascended_damage() {
    let blows = [
        ("MONSTER.SHRINKER_BEETLE", "CHOMP_MOVE", 7, 8),
        ("MONSTER.SHRINKER_BEETLE", "STOMP_MOVE", 13, 14),
        ("MONSTER.KIN_PRIEST", "ORB_OF_FRAILTY_MOVE", 8, 9),
        ("MONSTER.KIN_PRIEST", "BEAM_MOVE", 9, 9),
        ("MONSTER.KIN_FOLLOWER", "QUICK_SLASH_MOVE", 5, 5),
        ("MONSTER.KIN_FOLLOWER", "BOOMERANG_MOVE", 4, 4),
        ("MONSTER.FOGMOG", "SWIPE_MOVE", 8, 9),
        ("MONSTER.FOGMOG", "HEADBUTT_MOVE", 14, 16),
        ("MONSTER.INKLET", "JAB_MOVE", 3, 4),
        ("MONSTER.INKLET", "WHIRLWIND_MOVE", 6, 9),
        ("MONSTER.INKLET", "PIERCING_GAZE_MOVE", 10, 11),
        ("MONSTER.BYRDONIS", "SWOOP_MOVE", 17, 19),
        ("MONSTER.BYRDONIS", "PECK_MOVE", 9, 12),
        ("MONSTER.FLYCONID", "SMASH_MOVE", 11, 12),
        ("MONSTER.FLYCONID", "FRAIL_SPORES_MOVE", 8, 9),
        ("MONSTER.NIBBIT", "BUTT_MOVE", 12, 13),
        ("MONSTER.NIBBIT", "SLICE_MOVE", 6, 7),
        ("MONSTER.TWIG_SLIME_S", "TACKLE_MOVE", 4, 5),
        ("MONSTER.LEAF_SLIME_S", "TACKLE_MOVE", 3, 4),
        ("MONSTER.LEAF_SLIME_M", "CLUMP_SHOT", 8, 9),
        ("MONSTER.TWIG_SLIME_M", "POKEY_POUNCE_MOVE", 11, 12),
        ("MONSTER.THIEVING_HOPPER", "HAT_TRICK_MOVE", 21, 23),
        ("MONSTER.THIEVING_HOPPER", "NAB_MOVE", 14, 16),
        ("MONSTER.EXOSKELETON", "SKITTER_MOVE", 3, 4),
        ("MONSTER.EXOSKELETON", "MANDIBLES_MOVE", 8, 9),
        ("MONSTER.HUNTER_KILLER", "BITE_MOVE", 17, 19),
        ("MONSTER.HUNTER_KILLER", "PUNCTURE_MOVE", 21, 24),
        ("MONSTER.CHOMPER", "CLAMP_MOVE", 16, 18),
        ("MONSTER.BOWLBUG_SILK", "THRASH_MOVE", 8, 10),
        ("MONSTER.MYTE", "BITE_MOVE", 13, 15),
        ("MONSTER.MYTE", "SUCK_MOVE", 4, 6),
        ("MONSTER.CRUSHER", "THRASH_MOVE", 12, 14),
        ("MONSTER.CRUSHER", "BUG_STING_MOVE", 12, 14),
        ("MONSTER.CRUSHER", "GUARDED_STRIKE_MOVE", 12, 14),
        ("MONSTER.ROCKET", "TARGETING_RETICLE_MOVE", 3, 4),
        ("MONSTER.ROCKET", "PRECISION_BEAM_MOVE", 18, 20),
        ("MONSTER.ROCKET", "LASER_MOVE", 31, 35),
    ];
    for (model, move_state, base, deadly) in blows {
        let mut plain = fight(model, move_state, 0);
        assert_eq!(
            damage_of_one_turn(&mut plain),
            base,
            "{model} {move_state} at the baseline"
        );
        let mut raised = fight(model, move_state, 9);
        assert_eq!(
            damage_of_one_turn(&mut raised),
            deadly,
            "{model} {move_state} at Deadly Enemies"
        );
    }
}

/// The branches that are not damage: the strength three of them work in, the
/// block a nibbit's slice guards behind, and the extra hound in a skitter.
#[test]
fn the_earlier_monsters_buff_and_guard_for_their_ascended_amounts() {
    for (model, move_state, power, base, deadly) in [
        (
            "MONSTER.KIN_PRIEST",
            "RITUAL_MOVE",
            "POWER.STRENGTH_POWER",
            2,
            3,
        ),
        (
            "MONSTER.KIN_FOLLOWER",
            "POWER_DANCE_MOVE",
            "POWER.STRENGTH_POWER",
            2,
            3,
        ),
        ("MONSTER.NIBBIT", "HISS_MOVE", "POWER.STRENGTH_POWER", 2, 3),
        (
            "MONSTER.BOWLBUG_NECTAR",
            "BUFF_MOVE",
            "POWER.STRENGTH_POWER",
            15,
            16,
        ),
        ("MONSTER.MYTE", "SUCK_MOVE", "POWER.STRENGTH_POWER", 2, 3),
        (
            "MONSTER.CRUSHER",
            "ADAPT_MOVE",
            "POWER.STRENGTH_POWER",
            2,
            3,
        ),
        (
            "MONSTER.ROCKET",
            "CHARGE_UP_MOVE",
            "POWER.STRENGTH_POWER",
            2,
            3,
        ),
    ] {
        for (ascension, expected) in [(0, base), (9, deadly)] {
            let mut fighting = fight(model, move_state, ascension);
            let _ = damage_of_one_turn(&mut fighting);
            assert_eq!(
                creature(&fighting, CombatSide::Enemy).power_amount(power),
                expected,
                "{model} {move_state} at ascension {ascension}"
            );
        }
    }

    // A nibbit's slice guards behind six at Tough Enemies rather than five.
    for (ascension, block) in [(0, 5), (9, 6)] {
        let mut slicing = fight("MONSTER.NIBBIT", "SLICE_MOVE", ascension);
        let _ = damage_of_one_turn(&mut slicing);
        assert_eq!(
            creature(&slicing, CombatSide::Enemy).block,
            block,
            "the nibbit's slice at ascension {ascension}"
        );
    }
}
