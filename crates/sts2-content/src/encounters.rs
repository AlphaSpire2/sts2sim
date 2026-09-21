//! Combat rooms the run generator can place on any monster point of its act.

use sts2_engine::{
    EncounterDefinition, EnemySpawn, GeneratedCombatRewardSpec, MonsterGeneration, MonsterPick,
    MonsterSlot, RarityOdds, RotatingSpawn,
};

use crate::id;

/// An encounter's monster slots, named and ordered the way the game has them.
fn slots(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

/// The single-player monster-room reward shape every normal combat shares.
fn monster_room_rewards() -> GeneratedCombatRewardSpec {
    GeneratedCombatRewardSpec {
        minimum_gold: 10,
        maximum_gold: 20,
        scales_with_poverty: true,
        rarity_odds: RarityOdds::RegularEncounter,
        offers_relic: false,
    }
}

/// Elites pay more gold, roll better card rarities, and add a relic.
fn elite_room_rewards() -> GeneratedCombatRewardSpec {
    GeneratedCombatRewardSpec {
        minimum_gold: 35,
        maximum_gold: 45,
        scales_with_poverty: true,
        rarity_odds: RarityOdds::EliteEncounter,
        offers_relic: true,
    }
}

/// Bosses pay a fixed purse and always offer a rare card.
fn boss_room_rewards() -> GeneratedCombatRewardSpec {
    GeneratedCombatRewardSpec {
        minimum_gold: 100,
        maximum_gold: 100,
        scales_with_poverty: true,
        rarity_odds: RarityOdds::BossEncounter,
        offers_relic: false,
    }
}

/// The act's boss: a priest flanked by two followers, one of them opening with
/// its dance.
#[must_use]
pub fn the_kin_boss() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.THE_KIN_BOSS"),
        slots: slots(&["slot1", "slot2", "leaderSlot"]),
        spawn_slot_names: slots(&["slot1", "slot2", "leaderSlot"]),
        generation: MonsterGeneration::Placed(vec![
            EnemySpawn::rolled(id("MONSTER.KIN_FOLLOWER")).opening("POWER_DANCE_MOVE"),
            EnemySpawn::rolled(id("MONSTER.KIN_FOLLOWER")),
            EnemySpawn::rolled(id("MONSTER.KIN_PRIEST")),
        ]),
        gives_rewards: true,
        rewards: Some(boss_room_rewards()),
    }
}

/// Two Nibbits: the one in front slices, the one behind hisses.
#[must_use]
pub fn nibbits_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.NIBBITS_NORMAL"),
        slots: slots(&["front", "back"]),
        spawn_slot_names: slots(&["front", "back"]),
        generation: MonsterGeneration::Placed(vec![
            EnemySpawn::rolled(id("MONSTER.NIBBIT")).opening("SLICE_MOVE"),
            EnemySpawn::rolled(id("MONSTER.NIBBIT")).opening("HISS_MOVE"),
        ]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A lone Cubex Construct.
#[must_use]
pub fn cubex_construct_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.CUBEX_CONSTRUCT_NORMAL"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.CUBEX_CONSTRUCT")]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// The last act's second boss, alone.
#[must_use]
pub fn test_subject_boss() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.TEST_SUBJECT_BOSS"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.TEST_SUBJECT")]),
        gives_rewards: true,
        rewards: Some(boss_room_rewards()),
    }
}

/// A Fabricator in the middle, with two slots either side for what it makes.
#[must_use]
pub fn fabricator_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.FABRICATOR_NORMAL"),
        slots: slots(&["bot1", "bot2", "fabricator", "bot3", "bot4"]),
        spawn_slot_names: slots(&["fabricator"]),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.FABRICATOR")]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// One Axebot at the front, and whatever it has left of itself behind it.
#[must_use]
pub fn axebots_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.AXEBOTS_NORMAL"),
        slots: slots(&["front"]),
        spawn_slot_names: slots(&["front"]),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.AXEBOT")]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// The Lost beside the Forgotten.
#[must_use]
pub fn the_lost_and_forgotten_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.THE_LOST_AND_FORGOTTEN_NORMAL"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![
            id("MONSTER.THE_LOST"),
            id("MONSTER.THE_FORGOTTEN"),
        ]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// The act's second boss, alone.
#[must_use]
pub fn the_insatiable_boss() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.THE_INSATIABLE_BOSS"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.THE_INSATIABLE")]),
        gives_rewards: true,
        rewards: Some(boss_room_rewards()),
    }
}

/// An Ovicopter at the back of six slots, with the five in front of it the
/// nest it lays into.
#[must_use]
pub fn ovicopter_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.OVICOPTER_NORMAL"),
        slots: slots(&["egg1", "egg2", "egg3", "egg4", "egg5", "ovicopter"]),
        spawn_slot_names: slots(&["ovicopter"]),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.OVICOPTER")]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A Phrog Parasite alone in the first of five slots; the four behind it are
/// where the wrigglers it is carrying arrive.
#[must_use]
pub fn phrog_parasite_elite() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.PHROG_PARASITE_ELITE"),
        slots: slots(&["phrog", "wriggler1", "wriggler2", "wriggler3", "wriggler4"]),
        spawn_slot_names: slots(&["phrog"]),
        generation: MonsterGeneration::Placed(vec![EnemySpawn::rolled(id(
            "MONSTER.PHROG_PARASITE",
        ))]),
        gives_rewards: true,
        rewards: Some(elite_room_rewards()),
    }
}

/// The fight a Dense Vegetation rest picks: four wrigglers, one to a slot, none
/// of them stunned.
#[must_use]
pub fn dense_vegetation_event_encounter() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.DENSE_VEGETATION_EVENT_ENCOUNTER"),
        slots: slots(&["wriggler1", "wriggler2", "wriggler3", "wriggler4"]),
        spawn_slot_names: slots(&["wriggler1", "wriggler2", "wriggler3", "wriggler4"]),
        generation: MonsterGeneration::Placed(
            std::iter::repeat_with(|| EnemySpawn::rolled(id("MONSTER.WRIGGLER")))
                .take(4)
                .collect(),
        ),
        // An encounter an event stands up hands over nothing of its own.
        gives_rewards: true,
        rewards: None,
    }
}

/// A Slithering Strangler with company: a Snapping Jaxfruit, one medium slime,
/// or two small ones, rolled before the strangler is appended behind them.
#[must_use]
pub fn slithering_strangler_normal() -> EncounterDefinition {
    let small =
        || MonsterPick::Rolled(vec![id("MONSTER.LEAF_SLIME_S"), id("MONSTER.TWIG_SLIME_S")]);
    EncounterDefinition {
        model_id: id("ENCOUNTER.SLITHERING_STRANGLER_NORMAL"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Branched {
            branches: vec![
                vec![MonsterPick::Fixed(id("MONSTER.SNAPPING_JAXFRUIT"))],
                vec![MonsterPick::Rolled(vec![
                    id("MONSTER.LEAF_SLIME_M"),
                    id("MONSTER.TWIG_SLIME_M"),
                ])],
                vec![small(), small()],
            ],
            tail: vec![id("MONSTER.SLITHERING_STRANGLER")],
        },
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A lone Mawler.
#[must_use]
pub fn mawler_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.MAWLER_NORMAL"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.MAWLER")]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A lone Fogmog, which brings its own company.
///
/// Its `Slots` are `["illusion", "fogmog"]` and the Fogmog itself is generated
/// into the second of them, so the illusion it conjures sorts in front of it.
#[must_use]
pub fn fogmog_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.FOGMOG_NORMAL"),
        slots: slots(&["illusion", "fogmog"]),
        spawn_slot_names: slots(&["fogmog"]),
        generation: MonsterGeneration::Placed(vec![EnemySpawn::rolled(id("MONSTER.FOGMOG"))]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// Three Inklets, the middle one opening on its spin instead of its jab.
#[must_use]
pub fn inklets_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.INKLETS_NORMAL"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Placed(vec![
            EnemySpawn::rolled(id("MONSTER.INKLET")),
            EnemySpawn::rolled(id("MONSTER.INKLET")).opening("WHIRLWIND_MOVE"),
            EnemySpawn::rolled(id("MONSTER.INKLET")),
        ]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A lone Byrdonis.
#[must_use]
pub fn byrdonis_elite() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.BYRDONIS_ELITE"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.BYRDONIS")]),
        gives_rewards: true,
        rewards: Some(elite_room_rewards()),
    }
}

#[must_use]
pub fn shrinker_beetle_weak() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.SHRINKER_BEETLE_WEAK"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.SHRINKER_BEETLE")]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// One medium slime drawn ahead of the Flyconid it travels with.
#[must_use]
pub fn flyconid_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.FLYCONID_NORMAL"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Draws {
            pools: vec![vec![id("MONSTER.LEAF_SLIME_M"), id("MONSTER.TWIG_SLIME_M")]],
            draws: vec![0],
            spawn_order: vec![
                MonsterSlot::Drawn(0),
                MonsterSlot::Fixed(id("MONSTER.FLYCONID")),
            ],
        },
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A single Nibbit, which the encounter marks as standing alone.
#[must_use]
pub fn nibbits_weak() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.NIBBITS_WEAK"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.NIBBIT")]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// Draws two distinct small slimes, then a medium slime, and spawns the medium
/// one between them.
#[must_use]
pub fn slimes_weak() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.SLIMES_WEAK"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Draws {
            pools: vec![
                vec![id("MONSTER.LEAF_SLIME_S"), id("MONSTER.TWIG_SLIME_S")],
                vec![id("MONSTER.LEAF_SLIME_M"), id("MONSTER.TWIG_SLIME_M")],
            ],
            draws: vec![0, 0, 1],
            spawn_order: vec![
                MonsterSlot::Drawn(0),
                MonsterSlot::Drawn(2),
                MonsterSlot::Drawn(1),
            ],
        },
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A lone Thieving Hopper, which opens the Hive.
#[must_use]
pub fn thieving_hopper_weak() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.THIEVING_HOPPER_WEAK"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.THIEVING_HOPPER")]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A rock bug, a silk worker, and the sleeper the room is named for.
#[must_use]
pub fn slumbering_beetle_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.SLUMBERING_BEETLE_NORMAL"),
        slots: Vec::new(),
        spawn_slot_names: slots(&["first", "second", "third"]),
        generation: MonsterGeneration::Fixed(vec![
            id("MONSTER.BOWLBUG_ROCK"),
            id("MONSTER.BOWLBUG_SILK"),
            id("MONSTER.SLUMBERING_BEETLE"),
        ]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// Three roaches, each opening on the move its place in the line calls for.
#[must_use]
pub fn exoskeletons_weak() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.EXOSKELETONS_WEAK"),
        slots: slots(&["first", "second", "third"]),
        spawn_slot_names: slots(&["first", "second", "third"]),
        generation: MonsterGeneration::Placed(vec![
            EnemySpawn::rolled(id("MONSTER.EXOSKELETON")).opening("SKITTER_MOVE"),
            EnemySpawn::rolled(id("MONSTER.EXOSKELETON")).opening("MANDIBLES_MOVE"),
            EnemySpawn::rolled(id("MONSTER.EXOSKELETON")).opening("ENRAGE_MOVE"),
        ]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A lone Hunter Killer.
#[must_use]
pub fn hunter_killer_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.HUNTER_KILLER_NORMAL"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.HUNTER_KILLER")]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// Two Chompers, the second of which screeches first.
#[must_use]
pub fn chompers_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.CHOMPERS_NORMAL"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Placed(vec![
            EnemySpawn::rolled(id("MONSTER.CHOMPER")),
            EnemySpawn::rolled(id("MONSTER.CHOMPER")).opening("SCREECH_MOVE"),
        ]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A rock bug and two of the three workers, drawn without repeating one.
#[must_use]
pub fn bowlbugs_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.BOWLBUGS_NORMAL"),
        slots: Vec::new(),
        spawn_slot_names: slots(&["first", "middle", "last"]),
        generation: MonsterGeneration::Draws {
            pools: vec![vec![
                id("MONSTER.BOWLBUG_EGG"),
                id("MONSTER.BOWLBUG_SILK"),
                id("MONSTER.BOWLBUG_NECTAR"),
            ]],
            draws: vec![0, 0],
            spawn_order: vec![
                MonsterSlot::Fixed(id("MONSTER.BOWLBUG_ROCK")),
                MonsterSlot::Drawn(0),
                MonsterSlot::Drawn(1),
            ],
        },
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// Two Mytes, the second of which opens by draining instead.
#[must_use]
pub fn mytes_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.MYTES_NORMAL"),
        slots: slots(&["first", "second"]),
        spawn_slot_names: slots(&["first", "second"]),
        generation: MonsterGeneration::Placed(vec![
            EnemySpawn::rolled(id("MONSTER.MYTE")),
            EnemySpawn::rolled(id("MONSTER.MYTE")).opening("SUCK_MOVE"),
        ]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// The Kaiser Crab: two halves that surround whoever fights them.
#[must_use]
pub fn kaiser_crab_boss() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.KAISER_CRAB_BOSS"),
        slots: slots(&["crusher", "rocket"]),
        spawn_slot_names: slots(&["crusher", "rocket"]),
        generation: MonsterGeneration::Placed(vec![
            EnemySpawn::rolled(id("MONSTER.CRUSHER")),
            EnemySpawn::rolled(id("MONSTER.ROCKET")),
        ]),
        gives_rewards: true,
        rewards: Some(boss_room_rewards()),
    }
}

/// A lone Fuzzy Wurm Crawler.
#[must_use]
pub fn fuzzy_wurm_crawler_weak() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.FUZZY_WURM_CRAWLER_WEAK"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.FUZZY_WURM_CRAWLER")]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// Two medium slimes, and the two small ones in whichever order one coin
/// settles on.
#[must_use]
pub fn slimes_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.SLIMES_NORMAL"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Coin {
            heads: vec![
                id("MONSTER.TWIG_SLIME_M"),
                id("MONSTER.LEAF_SLIME_M"),
                id("MONSTER.LEAF_SLIME_S"),
                id("MONSTER.TWIG_SLIME_S"),
            ],
            tails: vec![
                id("MONSTER.TWIG_SLIME_M"),
                id("MONSTER.LEAF_SLIME_M"),
                id("MONSTER.TWIG_SLIME_S"),
                id("MONSTER.LEAF_SLIME_S"),
            ],
        },
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A Shrinker Beetle with a Fuzzy Wurm Crawler behind it.
#[must_use]
pub fn overgrowth_crawlers() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.OVERGROWTH_CRAWLERS"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![
            id("MONSTER.SHRINKER_BEETLE"),
            id("MONSTER.FUZZY_WURM_CRAWLER"),
        ]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A lone Bygone Effigy.
#[must_use]
pub fn bygone_effigy_elite() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.BYGONE_EFFIGY_ELITE"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.BYGONE_EFFIGY")]),
        gives_rewards: true,
        rewards: Some(elite_room_rewards()),
    }
}

/// The act's boss, alone.
#[must_use]
pub fn ceremonial_beast_boss() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.CEREMONIAL_BEAST_BOSS"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.CEREMONIAL_BEAST")]),
        gives_rewards: true,
        rewards: Some(boss_room_rewards()),
    }
}

/// A lone Tunneler, which opens the Hive's second floor.
#[must_use]
pub fn tunneler_weak() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.TUNNELER_WEAK"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.TUNNELER")]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// Four roaches, each opening on the move its slot calls for; the fourth rolls
/// between the first two.
#[must_use]
pub fn exoskeletons_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.EXOSKELETONS_NORMAL"),
        slots: slots(&["first", "second", "third", "fourth"]),
        spawn_slot_names: slots(&["first", "second", "third", "fourth"]),
        generation: MonsterGeneration::Placed(vec![
            EnemySpawn::rolled(id("MONSTER.EXOSKELETON")).opening("SKITTER_MOVE"),
            EnemySpawn::rolled(id("MONSTER.EXOSKELETON")).opening("MANDIBLES_MOVE"),
            EnemySpawn::rolled(id("MONSTER.EXOSKELETON")).opening("ENRAGE_MOVE"),
            EnemySpawn::rolled(id("MONSTER.EXOSKELETON")).opening("RAND"),
        ]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A lone Spiny Toad.
#[must_use]
pub fn spiny_toad_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.SPINY_TOAD_NORMAL"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.SPINY_TOAD")]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A lone Devoted Sculptor.
#[must_use]
pub fn devoted_sculptor_weak() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.DEVOTED_SCULPTOR_WEAK"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.DEVOTED_SCULPTOR")]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// Three scrolls, each opening on a different move: one roll settles which,
/// and the others follow it round the list.
#[must_use]
pub fn scrolls_of_biting_weak() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.SCROLLS_OF_BITING_WEAK"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::RotatingOpeners {
            spawns: std::iter::repeat_with(|| RotatingSpawn {
                model_id: id("MONSTER.SCROLL_OF_BITING"),
                fixed_opener: None,
                openers: vec![
                    "CHOMP".to_owned(),
                    "CHEW".to_owned(),
                    "MORE_TEETH".to_owned(),
                ],
            })
            .take(3)
            .collect(),
        },
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// Four scrolls: three rotating off one roll the way the weak encounter's do,
/// and a fourth that always opens on the last move of the list.
#[must_use]
pub fn scrolls_of_biting_normal() -> EncounterDefinition {
    let openers = || {
        vec![
            "CHOMP".to_owned(),
            "CHEW".to_owned(),
            "MORE_TEETH".to_owned(),
        ]
    };
    let mut spawns: Vec<RotatingSpawn> = std::iter::repeat_with(|| RotatingSpawn {
        model_id: id("MONSTER.SCROLL_OF_BITING"),
        openers: openers(),
        fixed_opener: None,
    })
    .take(3)
    .collect();
    spawns.push(RotatingSpawn {
        model_id: id("MONSTER.SCROLL_OF_BITING"),
        openers: openers(),
        fixed_opener: Some(2),
    });
    EncounterDefinition {
        model_id: id("ENCOUNTER.SCROLLS_OF_BITING_NORMAL"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::RotatingOpeners { spawns },
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// The Decimillipede: three segments of one creature, each opening on its own
/// move.
#[must_use]
pub fn decimillipede_elite() -> EncounterDefinition {
    let openers = || {
        vec![
            "WRITHE_MOVE".to_owned(),
            "BULK_MOVE".to_owned(),
            "CONSTRICT_MOVE".to_owned(),
        ]
    };
    EncounterDefinition {
        model_id: id("ENCOUNTER.DECIMILLIPEDE_ELITE"),
        slots: slots(&["segment1", "segment2", "segment3"]),
        spawn_slot_names: slots(&["segment1", "segment2", "segment3"]),
        generation: MonsterGeneration::RotatingOpeners {
            spawns: vec![
                RotatingSpawn {
                    model_id: id("MONSTER.DECIMILLIPEDE_SEGMENT_FRONT"),
                    fixed_opener: None,
                    openers: openers(),
                },
                RotatingSpawn {
                    model_id: id("MONSTER.DECIMILLIPEDE_SEGMENT_MIDDLE"),
                    fixed_opener: None,
                    openers: openers(),
                },
                RotatingSpawn {
                    model_id: id("MONSTER.DECIMILLIPEDE_SEGMENT_BACK"),
                    fixed_opener: None,
                    openers: openers(),
                },
            ],
        },
        gives_rewards: true,
        rewards: Some(elite_room_rewards()),
    }
}

/// A lone Frog Knight.
#[must_use]
pub fn frog_knight_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.FROG_KNIGHT_NORMAL"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.FROG_KNIGHT")]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// Glory's three knights, in the order the encounter places them.
#[must_use]
pub fn knights_elite() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.KNIGHTS_ELITE"),
        slots: Vec::new(),
        spawn_slot_names: slots(&["first", "second", "third"]),
        generation: MonsterGeneration::Fixed(vec![
            id("MONSTER.FLAIL_KNIGHT"),
            id("MONSTER.SPECTRAL_KNIGHT"),
            id("MONSTER.MAGI_KNIGHT"),
        ]),
        gives_rewards: true,
        rewards: Some(elite_room_rewards()),
    }
}

/// The last act's boss: Aeonglass alone.
#[must_use]
pub fn aeonglass_boss() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.AEONGLASS_BOSS"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.AEONGLASS")]),
        gives_rewards: true,
        rewards: Some(boss_room_rewards()),
    }
}

/// The Underdocks' waterfall giant, alone.
#[must_use]
pub fn waterfall_giant_boss() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.WATERFALL_GIANT_BOSS"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.WATERFALL_GIANT")]),
        gives_rewards: true,
        rewards: Some(boss_room_rewards()),
    }
}

/// A calcified cultist standing in front of a seapunk.
#[must_use]
pub fn seapunk_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.SEAPUNK_NORMAL"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![
            id("MONSTER.CALCIFIED_CULTIST"),
            id("MONSTER.SEAPUNK"),
        ]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A lone haunted ship.
#[must_use]
pub fn haunted_ship_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.HAUNTED_SHIP_NORMAL"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.HAUNTED_SHIP")]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A living fog, standing in the last of six slots: the five in front of it are
/// where the bombs it bloats out go.
#[must_use]
pub fn living_fog_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.LIVING_FOG_NORMAL"),
        slots: slots(&["bomb1", "bomb2", "bomb3", "bomb4", "bomb5", "livingFog"]),
        spawn_slot_names: slots(&["livingFog"]),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.LIVING_FOG")]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// Four phantasmal gardeners, one to a slot, each opening on the move its slot
/// names.
#[must_use]
pub fn phantasmal_gardeners_elite() -> EncounterDefinition {
    let gardener =
        |opening: &str| EnemySpawn::rolled(id("MONSTER.PHANTASMAL_GARDENER")).opening(opening);
    EncounterDefinition {
        model_id: id("ENCOUNTER.PHANTASMAL_GARDENERS_ELITE"),
        slots: slots(&["first", "second", "third", "fourth"]),
        spawn_slot_names: slots(&["first", "second", "third", "fourth"]),
        generation: MonsterGeneration::Placed(vec![
            gardener("FLAIL_MOVE"),
            gardener("BITE_MOVE"),
            gardener("LASH_MOVE"),
            gardener("ENLARGE_MOVE"),
        ]),
        gives_rewards: true,
        rewards: Some(elite_room_rewards()),
    }
}

/// The two cultists, calcified in front of damp.
#[must_use]
pub fn cultists_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.CULTISTS_NORMAL"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![
            id("MONSTER.CALCIFIED_CULTIST"),
            id("MONSTER.DAMP_CULTIST"),
        ]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A lone sludge spinner.
#[must_use]
pub fn sludge_spinner_weak() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.SLUDGE_SPINNER_WEAK"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.SLUDGE_SPINNER")]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// Two corpse slugs, each opening on a different move: one roll settles which,
/// and the other follows it round the list.
#[must_use]
pub fn corpse_slugs_weak() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.CORPSE_SLUGS_WEAK"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::RotatingOpeners {
            spawns: std::iter::repeat_with(|| RotatingSpawn {
                model_id: id("MONSTER.CORPSE_SLUG"),
                fixed_opener: None,
                openers: vec![
                    "WHIP_SLAP_MOVE".to_owned(),
                    "GLOMP_MOVE".to_owned(),
                    "GOOP_MOVE".to_owned(),
                ],
            })
            .take(2)
            .collect(),
        },
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// Two toadpoles: the one in front spikes up first, the one behind whirls.
#[must_use]
pub fn toadpoles_weak() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.TOADPOLES_WEAK"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        // Which spawn is in front is written on each, and the branch the
        // machine opens on reads it: the front one goes to the spiking, the
        // other to the whirl.
        generation: MonsterGeneration::Placed(vec![
            EnemySpawn::rolled(id("MONSTER.TOADPOLE")).opening("SPIKEN_MOVE"),
            EnemySpawn::rolled(id("MONSTER.TOADPOLE")).opening("WHIRL_MOVE"),
        ]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A lone Globe Head.
#[must_use]
pub fn globe_head_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.GLOBE_HEAD_NORMAL"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.GLOBE_HEAD")]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A lone Entomancer, which is an elite.
#[must_use]
pub fn entomancer_elite() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.ENTOMANCER_ELITE"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.ENTOMANCER")]),
        gives_rewards: true,
        rewards: Some(elite_room_rewards()),
    }
}

/// The knight who wants the lantern key back, which an event stands up rather
/// than a map point.
#[must_use]
pub fn mysterious_knight_event_encounter() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.MYSTERIOUS_KNIGHT_EVENT_ENCOUNTER"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.MYSTERIOUS_KNIGHT")]),
        // The fight counts as a monster room, so it carries monster-room
        // rewards even though no map point put it up.
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// The Architect's own room, which is a layout rather than a fight: the event
/// stands his creature up so that there is somebody to talk to, and nothing
/// ever takes a turn in it.
#[must_use]
pub fn the_architect_event_encounter() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.THE_ARCHITECT_EVENT_ENCOUNTER"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.ARCHITECT")]),
        // It counts as a monster room, the same as every other event
        // encounter, though nothing ever ends this one to claim the rewards.
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A lone Owl Magistrate.
#[must_use]
pub fn owl_magistrate_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.OWL_MAGISTRATE_NORMAL"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.OWL_MAGISTRATE")]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A lone Soul Nexus.
#[must_use]
pub fn soul_nexus_elite() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.SOUL_NEXUS_ELITE"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.SOUL_NEXUS")]),
        gives_rewards: true,
        rewards: Some(elite_room_rewards()),
    }
}

/// The act's boss: the queen and the puppet she keeps burning for her, in the
/// order the encounter places them.
#[must_use]
pub fn queen_boss() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.QUEEN_BOSS"),
        slots: slots(&["amalgam", "queen"]),
        spawn_slot_names: slots(&["amalgam", "queen"]),
        generation: MonsterGeneration::Placed(vec![
            EnemySpawn::rolled(id("MONSTER.TORCH_HEAD_AMALGAM")),
            EnemySpawn::rolled(id("MONSTER.QUEEN")),
        ]),
        gives_rewards: true,
        rewards: Some(boss_room_rewards()),
    }
}

/// A rock bug and one of the two smaller workers.
#[must_use]
pub fn bowlbugs_weak() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.BOWLBUGS_WEAK"),
        slots: Vec::new(),
        spawn_slot_names: slots(&["odd", "even"]),
        generation: MonsterGeneration::Draws {
            pools: vec![vec![
                id("MONSTER.BOWLBUG_EGG"),
                id("MONSTER.BOWLBUG_NECTAR"),
            ]],
            draws: vec![0],
            spawn_order: vec![
                MonsterSlot::Fixed(id("MONSTER.BOWLBUG_ROCK")),
                MonsterSlot::Drawn(0),
            ],
        },
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// The Obscura alone; the fear it brings is conjured on its first move.
#[must_use]
pub fn the_obscura_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.THE_OBSCURA_NORMAL"),
        slots: slots(&["illusion", "obscura"]),
        spawn_slot_names: slots(&["obscura"]),
        // `Slots` is `["illusion", "obscura"]`, and the Obscura is placed in
        // the second of them.
        generation: MonsterGeneration::Placed(vec![EnemySpawn::rolled(id("MONSTER.THE_OBSCURA"))]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A jaxfruit and a flyconid.
#[must_use]
pub fn snapping_jaxfruit_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.SNAPPING_JAXFRUIT_NORMAL"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![
            id("MONSTER.SNAPPING_JAXFRUIT"),
            id("MONSTER.FLYCONID"),
        ]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A lone Vine Shambler.
#[must_use]
pub fn vine_shambler_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.VINE_SHAMBLER_NORMAL"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.VINE_SHAMBLER")]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// Act one's boss on this map: the Vantom alone.
#[must_use]
pub fn vantom_boss() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.VANTOM_BOSS"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.VANTOM")]),
        gives_rewards: true,
        rewards: Some(boss_room_rewards()),
    }
}

/// A lone Louse Progenitor.
#[must_use]
pub fn louse_progenitor_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.LOUSE_PROGENITOR_NORMAL"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.LOUSE_PROGENITOR")]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A lone Infested Prism.
#[must_use]
pub fn infested_prisms_elite() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.INFESTED_PRISMS_ELITE"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.INFESTED_PRISM")]),
        gives_rewards: true,
        rewards: Some(elite_room_rewards()),
    }
}

/// Act two's boss on this map: the Knowledge Demon alone.
#[must_use]
pub fn knowledge_demon_boss() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.KNOWLEDGE_DEMON_BOSS"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.KNOWLEDGE_DEMON")]),
        gives_rewards: true,
        rewards: Some(boss_room_rewards()),
    }
}

/// A living shield standing in front of a turret operator.
#[must_use]
pub fn turret_operator_weak() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.TURRET_OPERATOR_WEAK"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![
            id("MONSTER.LIVING_SHIELD"),
            id("MONSTER.TURRET_OPERATOR"),
        ]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A lone seapunk, which is the weak room the Seapunk tag opens with.
#[must_use]
pub fn seapunk_weak() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.SEAPUNK_WEAK"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.SEAPUNK")]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A lone Fossil Stalker.
#[must_use]
pub fn fossil_stalker_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.FOSSIL_STALKER_NORMAL"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.FOSSIL_STALKER")]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A lone Skulking Colony, which is an elite.
#[must_use]
pub fn skulking_colony_elite() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.SKULKING_COLONY_ELITE"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.SKULKING_COLONY")]),
        gives_rewards: true,
        rewards: Some(elite_room_rewards()),
    }
}

/// A lone Punch Construct, which opens on its ready and at whatever size it
/// rolled; the punch-off event is the room that stands two up smaller.
#[must_use]
pub fn punch_construct_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.PUNCH_CONSTRUCT_NORMAL"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.PUNCH_CONSTRUCT")]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A lone Terror Eel, which is an elite.
#[must_use]
pub fn terror_eel_elite() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.TERROR_EEL_ELITE"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.TERROR_EEL")]),
        gives_rewards: true,
        rewards: Some(elite_room_rewards()),
    }
}

/// A punch construct and two cubexes, in that order.
#[must_use]
pub fn construct_menagerie_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.CONSTRUCT_MENAGERIE_NORMAL"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![
            id("MONSTER.PUNCH_CONSTRUCT"),
            id("MONSTER.CUBEX_CONSTRUCT"),
            id("MONSTER.CUBEX_CONSTRUCT"),
        ]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A lone Mecha Knight, which is an elite.
#[must_use]
pub fn mecha_knight_elite() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.MECHA_KNIGHT_ELITE"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.MECHA_KNIGHT")]),
        gives_rewards: true,
        rewards: Some(elite_room_rewards()),
    }
}

/// A lone sewer clam.
#[must_use]
pub fn sewer_clam_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.SEWER_CLAM_NORMAL"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.SEWER_CLAM")]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A gremlin merc, and the two it lets out when it goes down.
///
/// The encounter names three slots as constants of its own but overrides
/// `Slots` with none of them, so nothing sorts the side: the merc, and then the
/// sneaky and the fat one in the order the Surprise adds them.
#[must_use]
pub fn gremlin_merc_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.GREMLIN_MERC_NORMAL"),
        slots: Vec::new(),
        spawn_slot_names: slots(&["merc"]),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.GREMLIN_MERC")]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// The soul fysh, alone: the Underdocks' second boss.
#[must_use]
pub fn soul_fysh_boss() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.SOUL_FYSH_BOSS"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.SOUL_FYSH")]),
        gives_rewards: true,
        rewards: Some(boss_room_rewards()),
    }
}

/// Three raiders drawn out of the five, one of each at most: the encounter
/// draws three times from its own list, taking what it drew out of the list
/// before the next draw.
#[must_use]
pub fn ruby_raiders_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.RUBY_RAIDERS_NORMAL"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Draws {
            // The raiders that can be drawn, in the order the game lists
            // them, which is the order the draw indexes into.
            pools: vec![vec![
                id("MONSTER.AXE_RUBY_RAIDER"),
                id("MONSTER.ASSASSIN_RUBY_RAIDER"),
                id("MONSTER.BRUTE_RUBY_RAIDER"),
                id("MONSTER.CROSSBOW_RUBY_RAIDER"),
                id("MONSTER.TRACKER_RUBY_RAIDER"),
            ]],
            draws: vec![0, 0, 0],
            spawn_order: vec![
                MonsterSlot::Drawn(0),
                MonsterSlot::Drawn(1),
                MonsterSlot::Drawn(2),
            ],
        },
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A lone slimed berserker.
#[must_use]
pub fn slimed_berserker_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.SLIMED_BERSERKER_NORMAL"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.SLIMED_BERSERKER")]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// A chomper that screeches first, and a tunneler behind it.
///
/// No act's room list names this encounter, so no map point can ever be it;
/// what it is is written down all the same.
#[must_use]
pub fn tunneler_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.TUNNELER_NORMAL"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        // The chomper is marked to scream first, and the mark is what its
        // machine opens on.
        generation: MonsterGeneration::Placed(vec![
            EnemySpawn::rolled(id("MONSTER.CHOMPER")).opening("SCREECH_MOVE"),
            EnemySpawn::rolled(id("MONSTER.TUNNELER")),
        ]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// Three corpse slugs, each opening on a different move: one roll settles
/// which, and the others follow it round the list.
#[must_use]
pub fn corpse_slugs_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.CORPSE_SLUGS_NORMAL"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::RotatingOpeners {
            spawns: std::iter::repeat_with(|| RotatingSpawn {
                model_id: id("MONSTER.CORPSE_SLUG"),
                fixed_opener: None,
                openers: vec![
                    "WHIP_SLAP_MOVE".to_owned(),
                    "GLOMP_MOVE".to_owned(),
                    "GOOP_MOVE".to_owned(),
                ],
            })
            .take(3)
            .collect(),
        },
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// Three two-tailed rats in the back three of five slots, each opening on a
/// different move: one roll settles which, and the others follow it round the
/// list. The two slots in front are where the rats they call for arrive.
#[must_use]
pub fn two_tailed_rats_normal() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.TWO_TAILED_RATS_NORMAL"),
        slots: slots(&["first", "second", "third", "fourth", "fifth"]),
        spawn_slot_names: slots(&["third", "fourth", "fifth"]),
        generation: MonsterGeneration::RotatingOpeners {
            spawns: std::iter::repeat_with(|| RotatingSpawn {
                model_id: id("MONSTER.TWO_TAILED_RAT"),
                fixed_opener: None,
                // The opener cycles through three, in the order the machine
                // switches on.
                openers: vec![
                    "SCRATCH_MOVE".to_owned(),
                    "DISEASE_BITE_MOVE".to_owned(),
                    "SCREECH_MOVE".to_owned(),
                ],
            })
            .take(3)
            .collect(),
        },
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// The Lagavulin Matriarch, alone: the Underdocks' third boss.
#[must_use]
pub fn lagavulin_matriarch_boss() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.LAGAVULIN_MATRIARCH_BOSS"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.LAGAVULIN_MATRIARCH")]),
        gives_rewards: true,
        rewards: Some(boss_room_rewards()),
    }
}

/// The fake merchant's own room, which an event stands up rather than a map
/// point: one merchant, in the one slot the encounter names, and a purse of
/// three hundred rather than a monster room's ten to twenty.
#[must_use]
pub fn fake_merchant_event_encounter() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.FAKE_MERCHANT_EVENT_ENCOUNTER"),
        slots: slots(&["merchant"]),
        spawn_slot_names: slots(&["merchant"]),
        generation: MonsterGeneration::Fixed(vec![id("MONSTER.FAKE_MERCHANT_MONSTER")]),
        gives_rewards: true,
        rewards: Some(GeneratedCombatRewardSpec {
            minimum_gold: 300,
            maximum_gold: 300,
            // The fake merchant's encounter declares both gold bounds itself, so
            // its three hundred is its own number and Poverty never sees it.
            scales_with_poverty: false,
            rarity_odds: RarityOdds::RegularEncounter,
            offers_relic: false,
        }),
    }
}

/// The punch-off: two constructs, the first of them opening on its fast punch,
/// and each already worn down by a number the room rolls for it.
#[must_use]
pub fn punch_off_event_encounter() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.PUNCH_OFF_EVENT_ENCOUNTER"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::Placed(vec![
            EnemySpawn::rolled(id("MONSTER.PUNCH_CONSTRUCT"))
                .opening("FAST_PUNCH_MOVE")
                .rolling("StartingHpReduction", 2, 10),
            EnemySpawn::rolled(id("MONSTER.PUNCH_CONSTRUCT")).rolling("StartingHpReduction", 2, 10),
        ]),
        gives_rewards: true,
        rewards: Some(monster_room_rewards()),
    }
}

/// The battleworn dummy's room: one of three battle friends, and which one is
/// the setting the event writes on the encounter before the fight stands up.
#[must_use]
pub fn battleworn_dummy_event_encounter() -> EncounterDefinition {
    EncounterDefinition {
        model_id: id("ENCOUNTER.BATTLEWORN_DUMMY_EVENT_ENCOUNTER"),
        slots: Vec::new(),
        spawn_slot_names: Vec::new(),
        generation: MonsterGeneration::ChosenByTheEvent {
            options: vec![
                id("MONSTER.BATTLE_FRIEND_V1"),
                id("MONSTER.BATTLE_FRIEND_V2"),
                id("MONSTER.BATTLE_FRIEND_V3"),
            ],
        },
        // The fight lays out no reward set at all: the event pays out instead,
        // and what it pays depends on which setting was fought and whether
        // the time ran out.
        gives_rewards: false,
        rewards: None,
    }
}
