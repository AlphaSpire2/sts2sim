//! Every pool of the pinned build, pinned by its membership.
//!
//! A pool model declares which models stand in it and in what order, and a run
//! reads that order positionally: a reward indexes into a card pool, a grab bag
//! buckets a relic pool in the order it walks it, and a potion roll walks the
//! character's pool and then the shared one. So the whole list is the model, and
//! the cheapest exact way to pin one is to write it down.
//!
//! Each list here is the pool's whole membership in the pinned build. A
//! character card pool is written as a run reads it — every card the pool
//! holds with every epoch revealed and multiplayer content left out, less the
//! basics, ancients and multiplayer-only cards no rolled rarity ever offers —
//! and the count of what was left out is asserted beside it.

use sts2_core::ModelId;

fn id(value: &str) -> ModelId {
    value.parse().expect("test model ID is valid")
}

fn card_pool(pool: &str) -> Vec<String> {
    sts2_content::standard_registry()
        .card_pool(&id(pool))
        .expect("the pool is registered")
        .iter()
        .map(|card| card.model_id.to_string())
        .collect()
}

fn potion_pool(pool: &str) -> Vec<String> {
    sts2_content::standard_registry()
        .potion_pool(&id(pool))
        .expect("the pool is registered")
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn relic_pool(pool: &str) -> Vec<String> {
    sts2_content::standard_registry()
        .relic_pool(&id(pool))
        .expect("the pool is registered")
        .iter()
        .map(ToString::to_string)
        .collect()
}

/// The Ironclad card pool holds eighty-seven cards. The seven this list does
/// not hold are the three basics the character opens with, the two ancients,
/// and the two multiplayer-only cards a single-player run's pool never
/// carries.
#[test]
fn the_ironclad_card_pool_is_the_eighty_a_rarity_rolls() {
    assert_eq!(
        card_pool("CARD_POOL.IRONCLAD_CARD_POOL"),
        [
            "CARD.AGGRESSION",
            "CARD.ANGER",
            "CARD.ARMAMENTS",
            "CARD.ASHEN_STRIKE",
            "CARD.BARRICADE",
            "CARD.BATTLE_TRANCE",
            "CARD.BLOOD_WALL",
            "CARD.BLOODLETTING",
            "CARD.BLUDGEON",
            "CARD.BODY_SLAM",
            "CARD.BRAND",
            "CARD.BREAKTHROUGH",
            "CARD.BULLY",
            "CARD.BURNING_PACT",
            "CARD.CASCADE",
            "CARD.CINDER",
            "CARD.COLOSSUS",
            "CARD.CONFLAGRATION",
            "CARD.CRIMSON_MANTLE",
            "CARD.CRUELTY",
            "CARD.DARK_EMBRACE",
            "CARD.DEMON_FORM",
            "CARD.DISMANTLE",
            "CARD.DOMINATE",
            "CARD.DRUM_OF_BATTLE",
            "CARD.EVIL_EYE",
            "CARD.EXPECT_A_FIGHT",
            "CARD.FEED",
            "CARD.FEEL_NO_PAIN",
            "CARD.FIEND_FIRE",
            "CARD.FIGHT_ME",
            "CARD.FLAME_BARRIER",
            "CARD.FORGOTTEN_RITUAL",
            "CARD.HAVOC",
            "CARD.HEADBUTT",
            "CARD.HELLRAISER",
            "CARD.HEMOKINESIS",
            "CARD.HOWL_FROM_BEYOND",
            "CARD.IMPERVIOUS",
            "CARD.INFERNAL_BLADE",
            "CARD.INFERNO",
            "CARD.INFLAME",
            "CARD.IRON_WAVE",
            "CARD.JUGGERNAUT",
            "CARD.JUGGLING",
            "CARD.MANGLE",
            "CARD.MOLTEN_FIST",
            "CARD.NOT_YET",
            "CARD.OFFERING",
            "CARD.ONE_TWO_PUNCH",
            "CARD.PACTS_END",
            "CARD.PERFECTED_STRIKE",
            "CARD.PILLAGE",
            "CARD.POMMEL_STRIKE",
            "CARD.PRIMAL_FORCE",
            "CARD.PYRE",
            "CARD.RAGE",
            "CARD.RAMPAGE",
            "CARD.RUPTURE",
            "CARD.SECOND_WIND",
            "CARD.SETUP_STRIKE",
            "CARD.SHRUG_IT_OFF",
            "CARD.SPITE",
            "CARD.STAMPEDE",
            "CARD.STOKE",
            "CARD.STOMP",
            "CARD.STONE_ARMOR",
            "CARD.SWORD_BOOMERANG",
            "CARD.TAUNT",
            "CARD.TEAR_ASUNDER",
            "CARD.THRASH",
            "CARD.THUNDERCLAP",
            "CARD.TREMBLE",
            "CARD.TRUE_GRIT",
            "CARD.TWIN_STRIKE",
            "CARD.UNMOVABLE",
            "CARD.UNRELENTING",
            "CARD.UPPERCUT",
            "CARD.VICIOUS",
            "CARD.WHIRLWIND",
        ]
    );
}

/// The Silent card pool holds eighty-eight: the eighty below, four
/// basics, two ancients, and two multiplayer-only cards.
#[test]
fn the_silent_card_pool_is_the_eighty_a_rarity_rolls() {
    assert_eq!(
        card_pool("CARD_POOL.SILENT_CARD_POOL"),
        [
            "CARD.ABRASIVE",
            "CARD.ACCELERANT",
            "CARD.ACCURACY",
            "CARD.ACROBATICS",
            "CARD.ADRENALINE",
            "CARD.AFTERIMAGE",
            "CARD.ANTICIPATE",
            "CARD.ASSASSINATE",
            "CARD.BACKFLIP",
            "CARD.BACKSTAB",
            "CARD.BLADE_OF_INK",
            "CARD.BLADE_DANCE",
            "CARD.BLUR",
            "CARD.BOUNCING_FLASK",
            "CARD.BUBBLE_BUBBLE",
            "CARD.BULLET_TIME",
            "CARD.BURST",
            "CARD.CALCULATED_GAMBLE",
            "CARD.CLOAK_AND_DAGGER",
            "CARD.CORROSIVE_WAVE",
            "CARD.DAGGER_SPRAY",
            "CARD.DAGGER_THROW",
            "CARD.DASH",
            "CARD.DEADLY_POISON",
            "CARD.DEFLECT",
            "CARD.DODGE_AND_ROLL",
            "CARD.ECHOING_SLASH",
            "CARD.ENVENOM",
            "CARD.ESCAPE_PLAN",
            "CARD.EXPERTISE",
            "CARD.EXPOSE",
            "CARD.FAN_OF_KNIVES",
            "CARD.FINISHER",
            "CARD.FLECHETTES",
            "CARD.FLICK_FLACK",
            "CARD.SCARE",
            "CARD.FOOTWORK",
            "CARD.GRAND_FINALE",
            "CARD.HAND_TRICK",
            "CARD.HAZE",
            "CARD.HIDDEN_DAGGERS",
            "CARD.INFINITE_BLADES",
            "CARD.KNIFE_TRAP",
            "CARD.LEADING_STRIKE",
            "CARD.LEG_SWEEP",
            "CARD.MALAISE",
            "CARD.MASTER_PLANNER",
            "CARD.MEMENTO_MORI",
            "CARD.MIRAGE",
            "CARD.MURDER",
            "CARD.NIGHTMARE",
            "CARD.NOXIOUS_FUMES",
            "CARD.OUTBREAK",
            "CARD.PHANTOM_BLADES",
            "CARD.PIERCING_WAIL",
            "CARD.PINPOINT",
            "CARD.POISONED_STAB",
            "CARD.POUNCE",
            "CARD.PRECISE_CUT",
            "CARD.PREDATOR",
            "CARD.PREPARED",
            "CARD.REFLEX",
            "CARD.RICOCHET",
            "CARD.SERPENT_FORM",
            "CARD.SHADOW_STEP",
            "CARD.SHADOWMELD",
            "CARD.SKEWER",
            "CARD.SLICE",
            "CARD.SNAKEBITE",
            "CARD.SPEEDSTER",
            "CARD.STORM_OF_STEEL",
            "CARD.STRANGLE",
            "CARD.SUCKER_PUNCH",
            "CARD.TACTICIAN",
            "CARD.THE_HUNT",
            "CARD.TOOLS_OF_THE_TRADE",
            "CARD.TRACKING",
            "CARD.UNTOUCHABLE",
            "CARD.UP_MY_SLEEVE",
            "CARD.WELL_LAID_PLANS",
        ]
    );
}

/// The Regent card pool holds eighty-eight, on the same shape.
#[test]
fn the_regent_card_pool_is_the_eighty_a_rarity_rolls() {
    assert_eq!(
        card_pool("CARD_POOL.REGENT_CARD_POOL"),
        [
            "CARD.ALIGNMENT",
            "CARD.ARSENAL",
            "CARD.ASTRAL_PULSE",
            "CARD.BEAT_INTO_SHAPE",
            "CARD.BEGONE",
            "CARD.BIG_BANG",
            "CARD.BLACK_HOLE",
            "CARD.BOMBARDMENT",
            "CARD.BULWARK",
            "CARD.BUNDLE_OF_JOY",
            "CARD.CELESTIAL_MIGHT",
            "CARD.CHARGE",
            "CARD.CHILD_OF_THE_STARS",
            "CARD.CLOAK_OF_STARS",
            "CARD.COLLISION_COURSE",
            "CARD.COMET",
            "CARD.CONQUEROR",
            "CARD.CONVERGENCE",
            "CARD.COSMIC_INDIFFERENCE",
            "CARD.CRASH_LANDING",
            "CARD.CRESCENT_SPEAR",
            "CARD.CRUSH_UNDER",
            "CARD.DECISIONS_DECISIONS",
            "CARD.DEVASTATE",
            "CARD.DYING_STAR",
            "CARD.FOREGONE_CONCLUSION",
            "CARD.FURNACE",
            "CARD.GAMMA_BLAST",
            "CARD.GATHER_LIGHT",
            "CARD.GENESIS",
            "CARD.GLIMMER",
            "CARD.GLITTERSTREAM",
            "CARD.GLOW",
            "CARD.GUARDS",
            "CARD.GUIDING_STAR",
            "CARD.HEAVENLY_DRILL",
            "CARD.HEGEMONY",
            "CARD.HEIRLOOM_HAMMER",
            "CARD.HIDDEN_CACHE",
            "CARD.I_AM_INVINCIBLE",
            "CARD.KINGLY_KICK",
            "CARD.KINGLY_PUNCH",
            "CARD.KNOCKOUT_BLOW",
            "CARD.KNOW_THY_PLACE",
            "CARD.LUNAR_BLAST",
            "CARD.MAKE_IT_SO",
            "CARD.MANIFEST_AUTHORITY",
            "CARD.MONARCHS_GAZE",
            "CARD.MONOLOGUE",
            "CARD.NEUTRON_AEGIS",
            "CARD.ORBIT",
            "CARD.PALE_BLUE_DOT",
            "CARD.PARRY",
            "CARD.PARTICLE_WALL",
            "CARD.PATTER",
            "CARD.PHOTON_CUT",
            "CARD.PILLAR_OF_CREATION",
            "CARD.PROPHESIZE",
            "CARD.QUASAR",
            "CARD.RADIATE",
            "CARD.REFINE_BLADE",
            "CARD.REFLECT",
            "CARD.RESONANCE",
            "CARD.ROYAL_GAMBLE",
            "CARD.ROYALTIES",
            "CARD.SEEKING_EDGE",
            "CARD.SEVEN_STARS",
            "CARD.SHINING_STRIKE",
            "CARD.SOLAR_STRIKE",
            "CARD.SPECTRUM_SHIFT",
            "CARD.SPOILS_OF_BATTLE",
            "CARD.STARDUST",
            "CARD.SUMMON_FORTH",
            "CARD.SUPERMASSIVE",
            "CARD.SWORD_SAGE",
            "CARD.TERRAFORMING",
            "CARD.THE_SMITH",
            "CARD.TYRANNY",
            "CARD.VOID_FORM",
            "CARD.WROUGHT_IN_WAR",
        ]
    );
}

/// The Necrobinder card pool holds eighty-eight, on the same shape.
#[test]
fn the_necrobinder_card_pool_is_the_eighty_a_rarity_rolls() {
    assert_eq!(
        card_pool("CARD_POOL.NECROBINDER_CARD_POOL"),
        [
            "CARD.AFTERLIFE",
            "CARD.BANSHEES_CRY",
            "CARD.BLIGHT_STRIKE",
            "CARD.BONE_SHARDS",
            "CARD.BORROWED_TIME",
            "CARD.BURY",
            "CARD.CALCIFY",
            "CARD.CALL_OF_THE_VOID",
            "CARD.CAPTURE_SPIRIT",
            "CARD.CLEANSE",
            "CARD.COUNTDOWN",
            "CARD.DANSE_MACABRE",
            "CARD.DEATH_MARCH",
            "CARD.DEATHBRINGER",
            "CARD.DEATHS_DOOR",
            "CARD.DEBILITATE",
            "CARD.DEFILE",
            "CARD.DEFY",
            "CARD.DELAY",
            "CARD.DEMESNE",
            "CARD.DEVOUR_LIFE",
            "CARD.DIRGE",
            "CARD.DRAIN_POWER",
            "CARD.DREDGE",
            "CARD.EIDOLON",
            "CARD.END_OF_DAYS",
            "CARD.ENFEEBLING_TOUCH",
            "CARD.ERADICATE",
            "CARD.FEAR",
            "CARD.FETCH",
            "CARD.FLATTEN",
            "CARD.FRIENDSHIP",
            "CARD.GRAVE_WARDEN",
            "CARD.GRAVEBLAST",
            "CARD.HANG",
            "CARD.HAUNT",
            "CARD.HIGH_FIVE",
            "CARD.INVOKE",
            "CARD.LETHALITY",
            "CARD.MELANCHOLY",
            "CARD.MISERY",
            "CARD.NECRO_MASTERY",
            "CARD.NEGATIVE_PULSE",
            "CARD.NEUROSURGE",
            "CARD.NO_ESCAPE",
            "CARD.OBLIVION",
            "CARD.PAGESTORM",
            "CARD.PARSE",
            "CARD.POKE",
            "CARD.PULL_AGGRO",
            "CARD.PULL_FROM_BELOW",
            "CARD.PUTREFY",
            "CARD.RATTLE",
            "CARD.REANIMATE",
            "CARD.REAP",
            "CARD.REAPER_FORM",
            "CARD.REAVE",
            "CARD.RIGHT_HAND_HAND",
            "CARD.SACRIFICE",
            "CARD.SCOURGE",
            "CARD.SCULPTING_STRIKE",
            "CARD.SEANCE",
            "CARD.SENTRY_MODE",
            "CARD.SEVERANCE",
            "CARD.SHARED_FATE",
            "CARD.SHROUD",
            "CARD.SIC_EM",
            "CARD.SLEIGHT_OF_FLESH",
            "CARD.SNAP",
            "CARD.SOUL_STORM",
            "CARD.SOW",
            "CARD.SPIRIT_OF_ASH",
            "CARD.SPUR",
            "CARD.SQUEEZE",
            "CARD.THE_SCYTHE",
            "CARD.TIMES_UP",
            "CARD.TRANSFIGURE",
            "CARD.UNDEATH",
            "CARD.VEILPIERCER",
            "CARD.WISP",
        ]
    );
}

/// The Defect card pool holds eighty-eight, on the same shape.
#[test]
fn the_defect_card_pool_is_the_eighty_a_rarity_rolls() {
    assert_eq!(
        card_pool("CARD_POOL.DEFECT_CARD_POOL"),
        [
            "CARD.ADAPTIVE_STRIKE",
            "CARD.ALL_FOR_ONE",
            "CARD.BALL_LIGHTNING",
            "CARD.BARRAGE",
            "CARD.BEAM_CELL",
            "CARD.BOOST_AWAY",
            "CARD.BOOT_SEQUENCE",
            "CARD.BUFFER",
            "CARD.BULK_UP",
            "CARD.CAPACITOR",
            "CARD.CHAOS",
            "CARD.CHARGE_BATTERY",
            "CARD.CHILL",
            "CARD.CLAW",
            "CARD.COLD_SNAP",
            "CARD.COMPACT",
            "CARD.COMPILE_DRIVER",
            "CARD.CONSUMING_SHADOW",
            "CARD.COOLANT",
            "CARD.COOLHEADED",
            "CARD.CREATIVE_AI",
            "CARD.DARKNESS",
            "CARD.DEFRAGMENT",
            "CARD.DOUBLE_ENERGY",
            "CARD.ECHO_FORM",
            "CARD.FERAL",
            "CARD.FIGHT_THROUGH",
            "CARD.FLAK_CANNON",
            "CARD.FOCUSED_STRIKE",
            "CARD.FTL",
            "CARD.FUSION",
            "CARD.GENETIC_ALGORITHM",
            "CARD.GLACIER",
            "CARD.GLASSWORK",
            "CARD.GO_FOR_THE_EYES",
            "CARD.GUNK_UP",
            "CARD.HAILSTORM",
            "CARD.HELIX_DRILL",
            "CARD.HOLOGRAM",
            "CARD.HOTFIX",
            "CARD.HYPERBEAM",
            "CARD.ICE_LANCE",
            "CARD.ITERATION",
            "CARD.LEAP",
            "CARD.LIGHTNING_ROD",
            "CARD.LOOP",
            "CARD.MACHINE_LEARNING",
            "CARD.METEOR_STRIKE",
            "CARD.MODDED",
            "CARD.MOMENTUM_STRIKE",
            "CARD.MULTI_CAST",
            "CARD.NULL",
            "CARD.OVERCLOCK",
            "CARD.RAINBOW",
            "CARD.REBOOT",
            "CARD.REFRACT",
            "CARD.ROCKET_PUNCH",
            "CARD.SCAVENGE",
            "CARD.SCRAPE",
            "CARD.SHADOW_SHIELD",
            "CARD.SHATTER",
            "CARD.SIGNAL_BOOST",
            "CARD.SKIM",
            "CARD.SMOKESTACK",
            "CARD.SPINNER",
            "CARD.STORM",
            "CARD.SUBROUTINE",
            "CARD.SUNDER",
            "CARD.SUPERCRITICAL",
            "CARD.SWEEPING_BEAM",
            "CARD.SYNCHRONIZE",
            "CARD.SYNTHESIS",
            "CARD.TEMPEST",
            "CARD.TESLA_COIL",
            "CARD.THUNDER",
            "CARD.TRASH_TO_TREASURE",
            "CARD.TURBO",
            "CARD.UPROAR",
            "CARD.VOLTAIC",
            "CARD.WHITE_NOISE",
        ]
    );
}

/// The colourless card pool holds sixty-four. The eleven left out are every
/// multiplayer-only card of the pool; nothing else in it is filtered.
#[test]
fn the_colorless_card_pool_is_the_fifty_three_single_player_cards() {
    assert_eq!(
        card_pool("CARD_POOL.COLORLESS_CARD_POOL"),
        [
            "CARD.ALCHEMIZE",
            "CARD.ANOINTED",
            "CARD.AUTOMATION",
            "CARD.BEAT_DOWN",
            "CARD.BOLAS",
            "CARD.CALAMITY",
            "CARD.CATASTROPHE",
            "CARD.DARK_SHACKLES",
            "CARD.DISCOVERY",
            "CARD.DRAMATIC_ENTRANCE",
            "CARD.ENTROPY",
            "CARD.EQUILIBRIUM",
            "CARD.ETERNAL_ARMOR",
            "CARD.FASTEN",
            "CARD.FINESSE",
            "CARD.FISTICUFFS",
            "CARD.FLASH_OF_STEEL",
            "CARD.GOLD_AXE",
            "CARD.HAND_OF_GREED",
            "CARD.HIDDEN_GEM",
            "CARD.IMPATIENCE",
            "CARD.JACK_OF_ALL_TRADES",
            "CARD.JACKPOT",
            "CARD.MASTER_OF_STRATEGY",
            "CARD.MAYHEM",
            "CARD.MIND_BLAST",
            "CARD.NOSTALGIA",
            "CARD.OMNISLICE",
            "CARD.PANACHE",
            "CARD.PANIC_BUTTON",
            "CARD.PREP_TIME",
            "CARD.PRODUCTION",
            "CARD.PROLONG",
            "CARD.PROWESS",
            "CARD.PURITY",
            "CARD.REND",
            "CARD.RESTLESSNESS",
            "CARD.ROLLING_BOULDER",
            "CARD.SALVO",
            "CARD.SCRAWL",
            "CARD.SECRET_TECHNIQUE",
            "CARD.SECRET_WEAPON",
            "CARD.SEEKER_STRIKE",
            "CARD.SHOCKWAVE",
            "CARD.SPLASH",
            "CARD.STRATAGEM",
            "CARD.THE_BOMB",
            "CARD.THE_GAMBIT",
            "CARD.THINKING_AHEAD",
            "CARD.THRUMMING_HATCHET",
            "CARD.ULTIMATE_DEFEND",
            "CARD.ULTIMATE_STRIKE",
            "CARD.VOLLEY",
        ]
    );
}

/// The curse card pool, whole: what anything that charges a deck a curse
/// draws from, and what a curse transforming rolls against.
#[test]
fn the_curse_card_pool_is_the_eighteen_curses() {
    assert_eq!(
        card_pool("CARD_POOL.CURSE_CARD_POOL"),
        [
            "CARD.ASCENDERS_BANE",
            "CARD.BAD_LUCK",
            "CARD.CLUMSY",
            "CARD.CURSE_OF_THE_BELL",
            "CARD.DEBT",
            "CARD.DECAY",
            "CARD.DOUBT",
            "CARD.ENTHRALLED",
            "CARD.FOLLY",
            "CARD.GREED",
            "CARD.GUILTY",
            "CARD.INJURY",
            "CARD.NORMALITY",
            "CARD.POOR_SLEEP",
            "CARD.REGRET",
            "CARD.SHAME",
            "CARD.SPORE_MIND",
            "CARD.WRITHE",
        ]
    );
}

/// The deprecated card pool, whole.
#[test]
fn the_deprecated_card_pool_holds_only_the_stand_in() {
    assert_eq!(
        card_pool("CARD_POOL.DEPRECATED_CARD_POOL"),
        ["CARD.DEPRECATED_CARD",]
    );
}

/// The event card pool, whole. The last two are out of alphabetical order in
/// the build as well.
#[test]
fn the_event_card_pool_is_the_twenty_seven_an_event_hands_over() {
    assert_eq!(
        card_pool("CARD_POOL.EVENT_CARD_POOL"),
        [
            "CARD.APOTHEOSIS",
            "CARD.APPARITION",
            "CARD.BRIGHTEST_FLAME",
            "CARD.BYRD_SWOOP",
            "CARD.CALTROPS",
            "CARD.CLASH",
            "CARD.DISTRACTION",
            "CARD.DUAL_WIELD",
            "CARD.ENLIGHTENMENT",
            "CARD.ENTRENCH",
            "CARD.EXTERMINATE",
            "CARD.FEEDING_FRENZY",
            "CARD.HELLO_WORLD",
            "CARD.MAD_SCIENCE",
            "CARD.MAUL",
            "CARD.METAMORPHOSIS",
            "CARD.NEOWS_FURY",
            "CARD.OUTMANEUVER",
            "CARD.PECK",
            "CARD.REBOUND",
            "CARD.RELAX",
            "CARD.RIP_AND_TEAR",
            "CARD.SQUASH",
            "CARD.STACK",
            "CARD.TORIC_TOUGHNESS",
            "CARD.WISH",
            "CARD.WHISTLE",
        ]
    );
}

/// The quest card pool, whole.
#[test]
fn the_quest_card_pool_is_the_three_errands() {
    assert_eq!(
        card_pool("CARD_POOL.QUEST_CARD_POOL"),
        ["CARD.BYRDONIS_EGG", "CARD.LANTERN_KEY", "CARD.SPOILS_MAP",]
    );
}

/// The status card pool, whole. A status card transforming rolls against this
/// list unnarrowed, because the rolled-rarity cut a transform makes is kept
/// for originals outside the Status and Curse rarities.
#[test]
fn the_status_card_pool_is_the_twelve_a_status_transforms_into() {
    assert_eq!(
        card_pool("CARD_POOL.STATUS_CARD_POOL"),
        [
            "CARD.BECKON",
            "CARD.BURN",
            "CARD.DAZED",
            "CARD.DEBRIS",
            "CARD.FRANTIC_ESCAPE",
            "CARD.INFECTION",
            "CARD.WITHER",
            "CARD.SLIMED",
            "CARD.SOOT",
            "CARD.TOXIC",
            "CARD.VOID",
            "CARD.WOUND",
        ]
    );
}

/// The token card pool, whole.
#[test]
fn the_token_card_pool_is_the_fourteen_a_body_conjures() {
    assert_eq!(
        card_pool("CARD_POOL.TOKEN_CARD_POOL"),
        [
            "CARD.DISINTEGRATION",
            "CARD.FUEL",
            "CARD.GIANT_ROCK",
            "CARD.LUMINESCE",
            "CARD.MIND_ROT",
            "CARD.MINION_DIVE_BOMB",
            "CARD.MINION_SACRIFICE",
            "CARD.MINION_STRIKE",
            "CARD.SHIV",
            "CARD.SLOTH",
            "CARD.SOUL",
            "CARD.SOVEREIGN_BLADE",
            "CARD.SWEEPING_GAZE",
            "CARD.WASTE_AWAY",
        ]
    );
}

/// The shared relic pool, whole: the pool both grab bags are filled from.
#[test]
#[allow(clippy::too_many_lines, reason = "one line per relic of the pool")]
fn the_shared_relic_pool_is_the_hundred_and_eighteen_every_run_shares() {
    assert_eq!(
        relic_pool("RELIC_POOL.SHARED_RELIC_POOL"),
        [
            "RELIC.AKABEKO",
            "RELIC.AMETHYST_AUBERGINE",
            "RELIC.ANCHOR",
            "RELIC.ART_OF_WAR",
            "RELIC.BAG_OF_MARBLES",
            "RELIC.BAG_OF_PREPARATION",
            "RELIC.BEATING_REMNANT",
            "RELIC.BELLOWS",
            "RELIC.BELT_BUCKLE",
            "RELIC.BLOOD_VIAL",
            "RELIC.BOOK_OF_FIVE_RINGS",
            "RELIC.BOWLER_HAT",
            "RELIC.BREAD",
            "RELIC.BRONZE_SCALES",
            "RELIC.BURNING_STICKS",
            "RELIC.CANDELABRA",
            "RELIC.CAPTAINS_WHEEL",
            "RELIC.CAULDRON",
            "RELIC.CENTENNIAL_PUZZLE",
            "RELIC.CHANDELIER",
            "RELIC.CHEMICAL_X",
            "RELIC.CLOAK_CLASP",
            "RELIC.DINGY_RUG",
            "RELIC.DOLLYS_MIRROR",
            "RELIC.DRAGON_FRUIT",
            "RELIC.ETERNAL_FEATHER",
            "RELIC.FESTIVE_POPPER",
            "RELIC.FRESNEL_LENS",
            "RELIC.FROZEN_EGG",
            "RELIC.GAMBLING_CHIP",
            "RELIC.GAME_PIECE",
            "RELIC.GHOST_SEED",
            "RELIC.GIRYA",
            "RELIC.GNARLED_HAMMER",
            "RELIC.GORGET",
            "RELIC.GREMLIN_HORN",
            "RELIC.HAPPY_FLOWER",
            "RELIC.HORN_CLEAT",
            "RELIC.ICE_CREAM",
            "RELIC.INTIMIDATING_HELMET",
            "RELIC.JOSS_PAPER",
            "RELIC.JUZU_BRACELET",
            "RELIC.KIFUDA",
            "RELIC.KUNAI",
            "RELIC.KUSARIGAMA",
            "RELIC.LANTERN",
            "RELIC.LASTING_CANDY",
            "RELIC.LAVA_LAMP",
            "RELIC.LEES_WAFFLE",
            "RELIC.LETTER_OPENER",
            "RELIC.LIZARD_TAIL",
            "RELIC.LOOMING_FRUIT",
            "RELIC.LUCKY_FYSH",
            "RELIC.MANGO",
            "RELIC.MEAL_TICKET",
            "RELIC.MEAT_ON_THE_BONE",
            "RELIC.MEMBERSHIP_CARD",
            "RELIC.MERCURY_HOURGLASS",
            "RELIC.MINIATURE_CANNON",
            "RELIC.MINIATURE_TENT",
            "RELIC.MOLTEN_EGG",
            "RELIC.MUMMIFIED_HAND",
            "RELIC.MYSTIC_LIGHTER",
            "RELIC.NUNCHAKU",
            "RELIC.ODDLY_SMOOTH_STONE",
            "RELIC.OLD_COIN",
            "RELIC.ORICHALCUM",
            "RELIC.ORNAMENTAL_FAN",
            "RELIC.ORRERY",
            "RELIC.PANTOGRAPH",
            "RELIC.PARRYING_SHIELD",
            "RELIC.PEAR",
            "RELIC.PEN_NIB",
            "RELIC.PENDULUM",
            "RELIC.PERMAFROST",
            "RELIC.PETRIFIED_TOAD",
            "RELIC.PLANISPHERE",
            "RELIC.POCKETWATCH",
            "RELIC.POTION_BELT",
            "RELIC.PRAYER_WHEEL",
            "RELIC.PUNCH_DAGGER",
            "RELIC.RAINBOW_RING",
            "RELIC.RAZOR_TOOTH",
            "RELIC.RED_MASK",
            "RELIC.REGAL_PILLOW",
            "RELIC.REPTILE_TRINKET",
            "RELIC.RINGING_TRIANGLE",
            "RELIC.RIPPLE_BASIN",
            "RELIC.ROYAL_STAMP",
            "RELIC.SCREAMING_FLAGON",
            "RELIC.SHOVEL",
            "RELIC.SHURIKEN",
            "RELIC.SLING_OF_COURAGE",
            "RELIC.SPARKLING_ROUGE",
            "RELIC.STONE_CALENDAR",
            "RELIC.STONE_CRACKER",
            "RELIC.STRAWBERRY",
            "RELIC.STRIKE_DUMMY",
            "RELIC.STURDY_CLAMP",
            "RELIC.THE_ABACUS",
            "RELIC.THE_COURIER",
            "RELIC.TINY_MAILBOX",
            "RELIC.TOOLBOX",
            "RELIC.TOXIC_EGG",
            "RELIC.TUNGSTEN_ROD",
            "RELIC.TUNING_FORK",
            "RELIC.UNCEASING_TOP",
            "RELIC.UNSETTLING_LAMP",
            "RELIC.VAJRA",
            "RELIC.VAMBRACE",
            "RELIC.VENERABLE_TEA_SET",
            "RELIC.VERY_HOT_COCOA",
            "RELIC.VEXING_PUZZLEBOX",
            "RELIC.WAR_PAINT",
            "RELIC.WHETSTONE",
            "RELIC.WHITE_BEAST_STATUE",
            "RELIC.WHITE_STAR",
            "RELIC.WING_CHARM",
        ]
    );
}

/// The Ironclad relic pool, whole. The first is the starter the
/// character opens the run holding, which no bag keeps.
#[test]
fn the_ironclad_relic_pool_is_the_eight_the_character_carries() {
    assert_eq!(
        relic_pool("RELIC_POOL.IRONCLAD_RELIC_POOL"),
        [
            "RELIC.BRIMSTONE",
            "RELIC.BURNING_BLOOD",
            "RELIC.CHARONS_ASHES",
            "RELIC.DEMON_TONGUE",
            "RELIC.PAPER_PHROG",
            "RELIC.RED_SKULL",
            "RELIC.RUINED_HELMET",
            "RELIC.SELF_FORMING_CLAY",
        ]
    );
}

/// The Silent relic pool, whole.
#[test]
fn the_silent_relic_pool_is_the_eight_the_character_carries() {
    assert_eq!(
        relic_pool("RELIC_POOL.SILENT_RELIC_POOL"),
        [
            "RELIC.HELICAL_DART",
            "RELIC.NINJA_SCROLL",
            "RELIC.PAPER_KRANE",
            "RELIC.RING_OF_THE_SNAKE",
            "RELIC.SNECKO_SKULL",
            "RELIC.TINGSHA",
            "RELIC.TOUGH_BANDAGES",
            "RELIC.TWISTED_FUNNEL",
        ]
    );
}

/// The Regent relic pool, whole.
#[test]
fn the_regent_relic_pool_is_the_eight_the_character_carries() {
    assert_eq!(
        relic_pool("RELIC_POOL.REGENT_RELIC_POOL"),
        [
            "RELIC.DIVINE_RIGHT",
            "RELIC.FENCING_MANUAL",
            "RELIC.GALACTIC_DUST",
            "RELIC.LUNAR_PASTRY",
            "RELIC.MINI_REGENT",
            "RELIC.ORANGE_DOUGH",
            "RELIC.REGALITE",
            "RELIC.VITRUVIAN_MINION",
        ]
    );
}

/// The Necrobinder relic pool, whole.
#[test]
fn the_necrobinder_relic_pool_is_the_eight_the_character_carries() {
    assert_eq!(
        relic_pool("RELIC_POOL.NECROBINDER_RELIC_POOL"),
        [
            "RELIC.BIG_HAT",
            "RELIC.BONE_FLUTE",
            "RELIC.BOOK_REPAIR_KNIFE",
            "RELIC.BOOKMARK",
            "RELIC.BOUND_PHYLACTERY",
            "RELIC.FUNERARY_MASK",
            "RELIC.IVORY_TILE",
            "RELIC.UNDYING_SIGIL",
        ]
    );
}

/// The Defect relic pool, whole.
#[test]
fn the_defect_relic_pool_is_the_eight_the_character_carries() {
    assert_eq!(
        relic_pool("RELIC_POOL.DEFECT_RELIC_POOL"),
        [
            "RELIC.CRACKED_CORE",
            "RELIC.DATA_DISK",
            "RELIC.EMOTION_CHIP",
            "RELIC.GOLD_PLATED_CABLES",
            "RELIC.POWER_CELL",
            "RELIC.METRONOME",
            "RELIC.RUNIC_CAPACITOR",
            "RELIC.SYMBIOTIC_VIRUS",
        ]
    );
}

/// The event relic pool, whole: ninety-eight ancients,
/// thirty-four relics of the Event rarity, the five refined starters a touch of
/// Orobas hands over, and three that stand in the shared pool as well.
#[test]
#[allow(clippy::too_many_lines, reason = "one line per relic of the pool")]
fn the_event_relic_pool_is_the_hundred_and_forty_only_an_event_deals() {
    assert_eq!(
        relic_pool("RELIC_POOL.EVENT_RELIC_POOL"),
        [
            "RELIC.ALCHEMICAL_COFFER",
            "RELIC.ARCANE_SCROLL",
            "RELIC.ARCHAIC_TOOTH",
            "RELIC.ASTROLABE",
            "RELIC.BEAUTIFUL_BRACELET",
            "RELIC.BIG_MUSHROOM",
            "RELIC.BIIIG_HUG",
            "RELIC.BING_BONG",
            "RELIC.BLACK_BLOOD",
            "RELIC.BLACK_STAR",
            "RELIC.BLESSED_ANTLER",
            "RELIC.BLOOD_SOAKED_ROSE",
            "RELIC.BONE_TEA",
            "RELIC.BOOMING_CONCH",
            "RELIC.BRILLIANT_SCARF",
            "RELIC.BYRDPIP",
            "RELIC.CALLING_BELL",
            "RELIC.CHOICES_PARADOX",
            "RELIC.CHOSEN_CHEESE",
            "RELIC.CLAWS",
            "RELIC.CROSSBOW",
            "RELIC.CURSED_PEARL",
            "RELIC.DARKSTONE_PERIAPT",
            "RELIC.DAUGHTER_OF_THE_WIND",
            "RELIC.DELICATE_FROND",
            "RELIC.DIAMOND_DIADEM",
            "RELIC.DISTINGUISHED_CAPE",
            "RELIC.DIVINE_DESTINY",
            "RELIC.DREAM_CATCHER",
            "RELIC.DRIFTWOOD",
            "RELIC.DUSTY_TOME",
            "RELIC.ECTOPLASM",
            "RELIC.ELECTRIC_SHRYMP",
            "RELIC.EMBER_TEA",
            "RELIC.EMPTY_CAGE",
            "RELIC.FAKE_ANCHOR",
            "RELIC.FAKE_BLOOD_VIAL",
            "RELIC.FAKE_HAPPY_FLOWER",
            "RELIC.FAKE_LEES_WAFFLE",
            "RELIC.FAKE_MANGO",
            "RELIC.FAKE_MERCHANTS_RUG",
            "RELIC.FAKE_ORICHALCUM",
            "RELIC.FAKE_SNECKO_EYE",
            "RELIC.FAKE_STRIKE_DUMMY",
            "RELIC.FAKE_VENERABLE_TEA_SET",
            "RELIC.FIDDLE",
            "RELIC.FISHING_ROD",
            "RELIC.FORGOTTEN_SOUL",
            "RELIC.FRAGRANT_MUSHROOM",
            "RELIC.FUR_COAT",
            "RELIC.GLASS_EYE",
            "RELIC.GLITTER",
            "RELIC.GOLDEN_COMPASS",
            "RELIC.GOLDEN_PEARL",
            "RELIC.HAND_DRILL",
            "RELIC.HEFTY_TABLET",
            "RELIC.HISTORY_COURSE",
            "RELIC.INFUSED_CORE",
            "RELIC.IRON_CLUB",
            "RELIC.JEWELED_MASK",
            "RELIC.JEWELRY_BOX",
            "RELIC.KALEIDOSCOPE",
            "RELIC.LARGE_CAPSULE",
            "RELIC.LASTING_CANDY",
            "RELIC.LAVA_ROCK",
            "RELIC.LEAD_PAPERWEIGHT",
            "RELIC.LEAFY_POULTICE",
            "RELIC.LORDS_PARASOL",
            "RELIC.LOST_COFFER",
            "RELIC.LOST_WISP",
            "RELIC.MASSIVE_SCROLL",
            "RELIC.MAW_BANK",
            "RELIC.MEAT_CLEAVER",
            "RELIC.MR_STRUGGLES",
            "RELIC.MUSIC_BOX",
            "RELIC.NEOWS_BONES",
            "RELIC.NEOWS_TALISMAN",
            "RELIC.NEOWS_TORMENT",
            "RELIC.NEW_LEAF",
            "RELIC.NUTRITIOUS_OYSTER",
            "RELIC.NUTRITIOUS_SOUP",
            "RELIC.PAELS_BLOOD",
            "RELIC.PAELS_CLAW",
            "RELIC.PAELS_EYE",
            "RELIC.PAELS_FLESH",
            "RELIC.PAELS_GROWTH",
            "RELIC.PAELS_HORN",
            "RELIC.PAELS_LEGION",
            "RELIC.PAELS_TEARS",
            "RELIC.PAELS_TOOTH",
            "RELIC.PAELS_WING",
            "RELIC.PANDORAS_BOX",
            "RELIC.PHIAL_HOLSTER",
            "RELIC.PHILOSOPHERS_STONE",
            "RELIC.PHYLACTERY_UNBOUND",
            "RELIC.POLLINOUS_CORE",
            "RELIC.POMANDER",
            "RELIC.PRECARIOUS_SHEARS",
            "RELIC.PRECISE_SCISSORS",
            "RELIC.PRESERVED_FOG",
            "RELIC.PRISMATIC_GEM",
            "RELIC.PUMPKIN_CANDLE",
            "RELIC.RADIANT_PEARL",
            "RELIC.RAZOR_TOOTH",
            "RELIC.RING_OF_THE_DRAKE",
            "RELIC.ROYAL_POISON",
            "RELIC.RUNIC_PYRAMID",
            "RELIC.SAI",
            "RELIC.SAND_CASTLE",
            "RELIC.SCROLL_BOXES",
            "RELIC.SEA_GLASS",
            "RELIC.SEAL_OF_GOLD",
            "RELIC.SERE_TALON",
            "RELIC.SIGNET_RING",
            "RELIC.SILKEN_TRESS",
            "RELIC.SILVER_CRUCIBLE",
            "RELIC.SMALL_CAPSULE",
            "RELIC.SNECKO_EYE",
            "RELIC.SOZU",
            "RELIC.SPARKLING_ROUGE",
            "RELIC.SPIKED_GAUNTLETS",
            "RELIC.STONE_HUMIDIFIER",
            "RELIC.STORYBOOK",
            "RELIC.SWORD_OF_JADE",
            "RELIC.SWORD_OF_STONE",
            "RELIC.TANXS_WHISTLE",
            "RELIC.TEA_OF_DISCOURTESY",
            "RELIC.THE_BOOT",
            "RELIC.THROWING_AXE",
            "RELIC.TOASTY_MITTENS",
            "RELIC.TOUCH_OF_OROBAS",
            "RELIC.TOY_BOX",
            "RELIC.TRI_BOOMERANG",
            "RELIC.VELVET_CHOKER",
            "RELIC.WAR_HAMMER",
            "RELIC.WHISPERING_EARRING",
            "RELIC.WINGED_BOOTS",
            "RELIC.WONGO_CUSTOMER_APPRECIATION_BADGE",
            "RELIC.WONGOS_MYSTERY_TICKET",
            "RELIC.YUMMY_COOKIE",
        ]
    );
}

/// The fallback relic pool, whole.
#[test]
fn the_fallback_relic_pool_holds_only_the_circlet() {
    assert_eq!(
        relic_pool("RELIC_POOL.FALLBACK_RELIC_POOL"),
        ["RELIC.CIRCLET",]
    );
}

/// The deprecated relic pool, whole.
#[test]
fn the_deprecated_relic_pool_holds_only_the_stand_in() {
    assert_eq!(
        relic_pool("RELIC_POOL.DEPRECATED_RELIC_POOL"),
        ["RELIC.DEPRECATED_RELIC",]
    );
}

/// The shared potion pool, whole: the second half of every
/// character's potion options, in the order a roll walks it.
#[test]
fn the_shared_potion_pool_is_the_forty_five_behind_every_character() {
    assert_eq!(
        potion_pool("POTION_POOL.SHARED_POTION_POOL"),
        [
            "POTION.ATTACK_POTION",
            "POTION.BEETLE_JUICE",
            "POTION.BLESSING_OF_THE_FORGE",
            "POTION.BLOCK_POTION",
            "POTION.BOTTLED_POTENTIAL",
            "POTION.CLARITY",
            "POTION.COLORLESS_POTION",
            "POTION.CURE_ALL",
            "POTION.DEXTERITY_POTION",
            "POTION.DISTILLED_CHAOS",
            "POTION.DROPLET_OF_PRECOGNITION",
            "POTION.DUPLICATOR",
            "POTION.ENERGY_POTION",
            "POTION.ENTROPIC_BREW",
            "POTION.EXPLOSIVE_AMPOULE",
            "POTION.FAIRY_IN_A_BOTTLE",
            "POTION.FIRE_POTION",
            "POTION.FLEX_POTION",
            "POTION.FORTIFIER",
            "POTION.FRUIT_JUICE",
            "POTION.FYSH_OIL",
            "POTION.GAMBLERS_BREW",
            "POTION.GIGANTIFICATION_POTION",
            "POTION.HEART_OF_IRON",
            "POTION.LIQUID_BRONZE",
            "POTION.LIQUID_MEMORIES",
            "POTION.LUCKY_TONIC",
            "POTION.MAZALETHS_GIFT",
            "POTION.OROBIC_ACID",
            "POTION.POTION_OF_BINDING",
            "POTION.POWDERED_DEMISE",
            "POTION.POWER_POTION",
            "POTION.RADIANT_TINCTURE",
            "POTION.REGEN_POTION",
            "POTION.SHACKLING_POTION",
            "POTION.SHIP_IN_A_BOTTLE",
            "POTION.SKILL_POTION",
            "POTION.SNECKO_OIL",
            "POTION.SPEED_POTION",
            "POTION.STABLE_SERUM",
            "POTION.STRENGTH_POTION",
            "POTION.SWIFT_POTION",
            "POTION.TOUCH_OF_INSANITY",
            "POTION.VULNERABLE_POTION",
            "POTION.WEAK_POTION",
        ]
    );
}

/// The Ironclad potion pool is the character's fourth-epoch potions, in that
/// list's own order. The epoch is revealed at this baseline, so all three are
/// unlocked rather than none.
#[test]
fn the_ironclad_potion_pool_is_the_three_of_the_pact() {
    assert_eq!(
        potion_pool("POTION_POOL.IRONCLAD_POTION_POOL"),
        [
            "POTION.BLOOD_POTION",
            "POTION.SOLDIERS_STEW",
            "POTION.ASHWATER",
        ]
    );
}

/// The Silent potion pool is that character's fourth-epoch potions.
#[test]
fn the_silent_potion_pool_is_the_three_of_her_epoch() {
    assert_eq!(
        potion_pool("POTION_POOL.SILENT_POTION_POOL"),
        [
            "POTION.POISON_POTION",
            "POTION.GHOST_IN_A_JAR",
            "POTION.CUNNING_POTION",
        ]
    );
}

/// The Regent potion pool is that character's fourth-epoch potions.
#[test]
fn the_regent_potion_pool_is_the_three_of_his_epoch() {
    assert_eq!(
        potion_pool("POTION_POOL.REGENT_POTION_POOL"),
        [
            "POTION.STAR_POTION",
            "POTION.COSMIC_CONCOCTION",
            "POTION.KINGS_COURAGE",
        ]
    );
}

/// The Necrobinder potion pool is that character's fourth-epoch potions.
#[test]
fn the_necrobinder_potion_pool_is_the_three_of_her_epoch() {
    assert_eq!(
        potion_pool("POTION_POOL.NECROBINDER_POTION_POOL"),
        [
            "POTION.POTION_OF_DOOM",
            "POTION.POT_OF_GHOULS",
            "POTION.BONE_BREW",
        ]
    );
}

/// The Defect potion pool is that character's fourth-epoch potions.
#[test]
fn the_defect_potion_pool_is_the_three_of_his_epoch() {
    assert_eq!(
        potion_pool("POTION_POOL.DEFECT_POTION_POOL"),
        [
            "POTION.FOCUS_POTION",
            "POTION.ESSENCE_OF_DARKNESS",
            "POTION.POTION_OF_CAPACITY",
        ]
    );
}

/// The event potion pool, whole. Neither carries a rolled
/// rarity, so no generation reaches them.
#[test]
fn the_event_potion_pool_is_the_two_an_event_hands_over() {
    assert_eq!(
        potion_pool("POTION_POOL.EVENT_POTION_POOL"),
        ["POTION.FOUL_POTION", "POTION.GLOWWATER_POTION",]
    );
}

/// The token potion pool, whole: the potion an effect conjures that a reward
/// or a shop must never offer.
#[test]
fn the_token_potion_pool_holds_only_the_rock() {
    assert_eq!(
        potion_pool("POTION_POOL.TOKEN_POTION_POOL"),
        ["POTION.POTION_SHAPED_ROCK",]
    );
}

/// The deprecated potion pool, whole.
#[test]
fn the_deprecated_potion_pool_holds_only_the_stand_in() {
    assert_eq!(
        potion_pool("POTION_POOL.DEPRECATED_POTION_POOL"),
        ["POTION.DEPRECATED_POTION",]
    );
}

/// Every character card pool: the one pool each character's colour is,
/// and nothing else. It is what an offer of another colour is rolled out of, so
/// a shared pool leaking into it would put a status or a token on the shelf.
#[test]
fn only_the_five_colours_are_character_card_pools() {
    let content = sts2_content::standard_registry();
    let pools: Vec<String> = content
        .character_card_pools()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        pools,
        [
            "CARD_POOL.DEFECT_CARD_POOL",
            "CARD_POOL.IRONCLAD_CARD_POOL",
            "CARD_POOL.NECROBINDER_CARD_POOL",
            "CARD_POOL.REGENT_CARD_POOL",
            "CARD_POOL.SILENT_CARD_POOL",
        ]
    );
}

/// Which pools count as colourless, pool by pool. Only a filter asking for a
/// colourless card asks, and the four that answer yes are the colorless
/// pool, the event pool, the token pool, and the deprecated one.
#[test]
fn four_of_the_twelve_card_pools_are_colorless() {
    let content = sts2_content::standard_registry();
    let colorless: Vec<&str> = [
        "CARD_POOL.COLORLESS_CARD_POOL",
        "CARD_POOL.CURSE_CARD_POOL",
        "CARD_POOL.DEFECT_CARD_POOL",
        "CARD_POOL.DEPRECATED_CARD_POOL",
        "CARD_POOL.EVENT_CARD_POOL",
        "CARD_POOL.IRONCLAD_CARD_POOL",
        "CARD_POOL.NECROBINDER_CARD_POOL",
        "CARD_POOL.QUEST_CARD_POOL",
        "CARD_POOL.REGENT_CARD_POOL",
        "CARD_POOL.SILENT_CARD_POOL",
        "CARD_POOL.STATUS_CARD_POOL",
        "CARD_POOL.TOKEN_CARD_POOL",
    ]
    .into_iter()
    .filter(|pool| content.card_pool_is_colorless(&id(pool)))
    .collect();
    assert_eq!(
        colorless,
        [
            "CARD_POOL.COLORLESS_CARD_POOL",
            "CARD_POOL.DEPRECATED_CARD_POOL",
            "CARD_POOL.EVENT_CARD_POOL",
            "CARD_POOL.TOKEN_CARD_POOL",
        ]
    );
}

/// Which relic pool and which potion pool belong to whom. A run's grab bag is
/// the shared pool plus the first, and its
/// potion options are the second followed by the shared potion pool.
#[test]
fn every_character_names_its_own_relic_and_potion_pools() {
    let content = sts2_content::standard_registry();
    for (character, relics, potions) in [
        (
            "CHARACTER.IRONCLAD",
            "RELIC_POOL.IRONCLAD_RELIC_POOL",
            "POTION_POOL.IRONCLAD_POTION_POOL",
        ),
        (
            "CHARACTER.SILENT",
            "RELIC_POOL.SILENT_RELIC_POOL",
            "POTION_POOL.SILENT_POTION_POOL",
        ),
        (
            "CHARACTER.REGENT",
            "RELIC_POOL.REGENT_RELIC_POOL",
            "POTION_POOL.REGENT_POTION_POOL",
        ),
        (
            "CHARACTER.NECROBINDER",
            "RELIC_POOL.NECROBINDER_RELIC_POOL",
            "POTION_POOL.NECROBINDER_POTION_POOL",
        ),
        (
            "CHARACTER.DEFECT",
            "RELIC_POOL.DEFECT_RELIC_POOL",
            "POTION_POOL.DEFECT_POTION_POOL",
        ),
    ] {
        let character = id(character);
        assert_eq!(
            content
                .character_relic_pool(&character)
                .map(ToString::to_string),
            Some(relics.to_owned())
        );
        let walked: Vec<String> = content
            .character_potion_pools(&character)
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(walked, [potions, "POTION_POOL.SHARED_POTION_POOL"]);
    }
}

/// How a grab bag is filled: the shared bag is the shared pool bucketed by
/// rarity, every rarity kept, and the player's is that pool with the walking
/// character's own appended and only the four rolled rarities kept. Both are
/// then shuffled, so what is checked here is what each bucket holds rather than
/// the order the shuffle left it in.
#[test]
fn the_two_bags_are_the_pools_bucketed_by_rarity() {
    let generated =
        sts2_content::standard_act_room_pools("N3Z9EE2CXB").expect("the pinned seed generates");
    let content = sts2_content::standard_registry();
    let shared: Vec<ModelId> = content
        .relic_pool(&id("RELIC_POOL.SHARED_RELIC_POOL"))
        .expect("the shared pool is registered")
        .clone();
    let own: Vec<ModelId> = content
        .relic_pool(&id("RELIC_POOL.IRONCLAD_RELIC_POOL"))
        .expect("the Ironclad's pool is registered")
        .clone();
    let of_rarity = |relics: &[ModelId], rarity: sts2_engine::RelicRarity| {
        let mut held: Vec<String> = relics
            .iter()
            .filter(|relic| {
                content
                    .relic(relic)
                    .is_some_and(|definition| definition.rarity == rarity)
            })
            .map(ToString::to_string)
            .collect();
        held.sort();
        held
    };
    let sorted = |bucket: &[ModelId]| {
        let mut held: Vec<String> = bucket.iter().map(ToString::to_string).collect();
        held.sort();
        held
    };
    for rarity in [
        sts2_engine::RelicRarity::Common,
        sts2_engine::RelicRarity::Uncommon,
        sts2_engine::RelicRarity::Rare,
        sts2_engine::RelicRarity::Shop,
        sts2_engine::RelicRarity::Event,
        sts2_engine::RelicRarity::Ancient,
    ] {
        assert_eq!(
            sorted(generated.shared_relics.bucket(rarity)),
            of_rarity(&shared, rarity),
            "the shared bag's {rarity:?} bucket is the shared pool's"
        );
    }
    let mut player = shared;
    player.extend(own);
    for rarity in [
        sts2_engine::RelicRarity::Common,
        sts2_engine::RelicRarity::Uncommon,
        sts2_engine::RelicRarity::Rare,
        sts2_engine::RelicRarity::Shop,
    ] {
        assert_eq!(
            sorted(generated.player_relics.bucket(rarity)),
            of_rarity(&player, rarity),
            "the player's bag holds their own colour's {rarity:?} relics too"
        );
    }
    // The starter the character opens holding is of no rolled rarity, so the
    // cut to the four rolled rarities drops it and no bag ever offers it back.
    assert!(
        generated
            .player_relics
            .bucket(sts2_engine::RelicRarity::Event)
            .is_empty()
    );
    assert!(
        generated
            .player_relics
            .bucket(sts2_engine::RelicRarity::Ancient)
            .is_empty()
    );
    assert!(
        generated
            .player_relics
            .bucket(sts2_engine::RelicRarity::Starter)
            .is_empty()
    );
}

/// The two stand-in pools of the build are not registered. One is the card
/// pool the two unplayable stand-in characters carry, which the build leaves
/// out of its shared card pools on purpose; the other stands among the potion
/// pools but holds a single stand-in potion alone.
#[test]
fn the_mock_pools_are_not_registered() {
    let content = sts2_content::standard_registry();
    assert!(content.card_pool(&id("CARD_POOL.MOCK_CARD_POOL")).is_none());
    assert!(
        content
            .potion_pool(&id("POTION_POOL.MOCK_POTION_POOL"))
            .is_none()
    );
}
