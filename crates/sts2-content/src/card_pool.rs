//! The pinned card pools, in the exact order the build declares them.
//!
//! Generation draws are positional, so both order and membership matter. The
//! supported baseline is single-player with every epoch revealed, so
//! multiplayer-only cards are already filtered out and no epoch prunes apply.
//! Basic and ancient cards are dropped from a character pool and from the
//! colorless one as well: generation excludes basics outright and never rolls
//! an ancient rarity. The shared families are written whole, because what reads
//! one of those reads all of it.
//!
//! `own_pool` is the other half of this module: the pool a card counts as
//! belonging to. The game settles that by asking each pool in turn whether it
//! holds the card, so the answer can be any pool the game defines rather than
//! only the ones a rolled rarity draws from.

use sts2_engine::{CardRarity, CardType};

pub(crate) const IRONCLAD_POOL: [(&str, CardType, CardRarity, i32); 80] = [
    ("CARD.AGGRESSION", CardType::Power, CardRarity::Rare, 1),
    ("CARD.ANGER", CardType::Attack, CardRarity::Common, 0),
    ("CARD.ARMAMENTS", CardType::Skill, CardRarity::Common, 1),
    (
        "CARD.ASHEN_STRIKE",
        CardType::Attack,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.BARRICADE", CardType::Power, CardRarity::Rare, 3),
    (
        "CARD.BATTLE_TRANCE",
        CardType::Skill,
        CardRarity::Uncommon,
        0,
    ),
    ("CARD.BLOOD_WALL", CardType::Skill, CardRarity::Common, 2),
    ("CARD.BLOODLETTING", CardType::Skill, CardRarity::Common, 0),
    ("CARD.BLUDGEON", CardType::Attack, CardRarity::Uncommon, 3),
    ("CARD.BODY_SLAM", CardType::Attack, CardRarity::Common, 1),
    ("CARD.BRAND", CardType::Skill, CardRarity::Rare, 0),
    ("CARD.BREAKTHROUGH", CardType::Attack, CardRarity::Common, 1),
    ("CARD.BULLY", CardType::Attack, CardRarity::Uncommon, 0),
    (
        "CARD.BURNING_PACT",
        CardType::Skill,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.CASCADE", CardType::Skill, CardRarity::Rare, -1),
    ("CARD.CINDER", CardType::Attack, CardRarity::Common, 2),
    ("CARD.COLOSSUS", CardType::Skill, CardRarity::Uncommon, 1),
    ("CARD.CONFLAGRATION", CardType::Attack, CardRarity::Rare, 1),
    ("CARD.CRIMSON_MANTLE", CardType::Power, CardRarity::Rare, 1),
    ("CARD.CRUELTY", CardType::Power, CardRarity::Rare, 1),
    ("CARD.DARK_EMBRACE", CardType::Power, CardRarity::Rare, 2),
    ("CARD.DEMON_FORM", CardType::Power, CardRarity::Rare, 3),
    ("CARD.DISMANTLE", CardType::Attack, CardRarity::Uncommon, 1),
    ("CARD.DOMINATE", CardType::Skill, CardRarity::Uncommon, 1),
    (
        "CARD.DRUM_OF_BATTLE",
        CardType::Skill,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.EVIL_EYE", CardType::Skill, CardRarity::Uncommon, 1),
    (
        "CARD.EXPECT_A_FIGHT",
        CardType::Skill,
        CardRarity::Uncommon,
        2,
    ),
    ("CARD.FEED", CardType::Attack, CardRarity::Rare, 1),
    (
        "CARD.FEEL_NO_PAIN",
        CardType::Power,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.FIEND_FIRE", CardType::Attack, CardRarity::Rare, 2),
    ("CARD.FIGHT_ME", CardType::Attack, CardRarity::Uncommon, 2),
    (
        "CARD.FLAME_BARRIER",
        CardType::Skill,
        CardRarity::Uncommon,
        2,
    ),
    (
        "CARD.FORGOTTEN_RITUAL",
        CardType::Skill,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.HAVOC", CardType::Skill, CardRarity::Common, 1),
    ("CARD.HEADBUTT", CardType::Attack, CardRarity::Common, 1),
    ("CARD.HELLRAISER", CardType::Power, CardRarity::Rare, 2),
    (
        "CARD.HEMOKINESIS",
        CardType::Attack,
        CardRarity::Uncommon,
        1,
    ),
    (
        "CARD.HOWL_FROM_BEYOND",
        CardType::Attack,
        CardRarity::Uncommon,
        3,
    ),
    ("CARD.IMPERVIOUS", CardType::Skill, CardRarity::Rare, 2),
    (
        "CARD.INFERNAL_BLADE",
        CardType::Skill,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.INFERNO", CardType::Power, CardRarity::Uncommon, 1),
    ("CARD.INFLAME", CardType::Power, CardRarity::Uncommon, 1),
    ("CARD.IRON_WAVE", CardType::Attack, CardRarity::Common, 1),
    ("CARD.JUGGERNAUT", CardType::Power, CardRarity::Rare, 2),
    ("CARD.JUGGLING", CardType::Power, CardRarity::Uncommon, 1),
    ("CARD.MANGLE", CardType::Attack, CardRarity::Rare, 3),
    ("CARD.MOLTEN_FIST", CardType::Attack, CardRarity::Common, 1),
    ("CARD.NOT_YET", CardType::Skill, CardRarity::Rare, 2),
    ("CARD.OFFERING", CardType::Skill, CardRarity::Rare, 0),
    ("CARD.ONE_TWO_PUNCH", CardType::Skill, CardRarity::Rare, 1),
    ("CARD.PACTS_END", CardType::Attack, CardRarity::Rare, 0),
    (
        "CARD.PERFECTED_STRIKE",
        CardType::Attack,
        CardRarity::Common,
        2,
    ),
    ("CARD.PILLAGE", CardType::Attack, CardRarity::Uncommon, 1),
    (
        "CARD.POMMEL_STRIKE",
        CardType::Attack,
        CardRarity::Common,
        1,
    ),
    ("CARD.PRIMAL_FORCE", CardType::Skill, CardRarity::Rare, 0),
    ("CARD.PYRE", CardType::Power, CardRarity::Rare, 2),
    ("CARD.RAGE", CardType::Skill, CardRarity::Uncommon, 0),
    ("CARD.RAMPAGE", CardType::Attack, CardRarity::Uncommon, 1),
    ("CARD.RUPTURE", CardType::Power, CardRarity::Uncommon, 1),
    ("CARD.SECOND_WIND", CardType::Skill, CardRarity::Uncommon, 1),
    ("CARD.SETUP_STRIKE", CardType::Attack, CardRarity::Common, 1),
    ("CARD.SHRUG_IT_OFF", CardType::Skill, CardRarity::Common, 1),
    ("CARD.SPITE", CardType::Attack, CardRarity::Uncommon, 0),
    ("CARD.STAMPEDE", CardType::Power, CardRarity::Uncommon, 2),
    ("CARD.STOKE", CardType::Skill, CardRarity::Rare, 1),
    ("CARD.STOMP", CardType::Attack, CardRarity::Uncommon, 3),
    ("CARD.STONE_ARMOR", CardType::Power, CardRarity::Uncommon, 1),
    (
        "CARD.SWORD_BOOMERANG",
        CardType::Attack,
        CardRarity::Common,
        1,
    ),
    ("CARD.TAUNT", CardType::Skill, CardRarity::Uncommon, 1),
    ("CARD.TEAR_ASUNDER", CardType::Attack, CardRarity::Rare, 2),
    ("CARD.THRASH", CardType::Attack, CardRarity::Rare, 1),
    ("CARD.THUNDERCLAP", CardType::Attack, CardRarity::Common, 1),
    ("CARD.TREMBLE", CardType::Skill, CardRarity::Common, 1),
    ("CARD.TRUE_GRIT", CardType::Skill, CardRarity::Common, 1),
    ("CARD.TWIN_STRIKE", CardType::Attack, CardRarity::Common, 1),
    ("CARD.UNMOVABLE", CardType::Power, CardRarity::Rare, 2),
    (
        "CARD.UNRELENTING",
        CardType::Attack,
        CardRarity::Uncommon,
        2,
    ),
    ("CARD.UPPERCUT", CardType::Attack, CardRarity::Uncommon, 2),
    ("CARD.VICIOUS", CardType::Power, CardRarity::Uncommon, 1),
    ("CARD.WHIRLWIND", CardType::Attack, CardRarity::Uncommon, 0),
];

/// The Silent's pool, in the build's own declaration order.
///
/// The eight cards of the Silent's full pool this list does not hold are the
/// ones a rolled rarity never offers: the four basics the character starts
/// with, the two ancients, and the two multiplayer-only cards no single-player
/// run's pool carries.
pub(crate) const SILENT_POOL: [(&str, CardType, CardRarity, i32); 80] = [
    ("CARD.ABRASIVE", CardType::Power, CardRarity::Rare, 3),
    ("CARD.ACCELERANT", CardType::Power, CardRarity::Rare, 1),
    ("CARD.ACCURACY", CardType::Power, CardRarity::Uncommon, 1),
    ("CARD.ACROBATICS", CardType::Skill, CardRarity::Uncommon, 1),
    ("CARD.ADRENALINE", CardType::Skill, CardRarity::Rare, 0),
    ("CARD.AFTERIMAGE", CardType::Power, CardRarity::Rare, 1),
    ("CARD.ANTICIPATE", CardType::Skill, CardRarity::Common, 0),
    ("CARD.ASSASSINATE", CardType::Attack, CardRarity::Rare, 0),
    ("CARD.BACKFLIP", CardType::Skill, CardRarity::Common, 1),
    ("CARD.BACKSTAB", CardType::Attack, CardRarity::Uncommon, 0),
    ("CARD.BLADE_OF_INK", CardType::Skill, CardRarity::Rare, 1),
    ("CARD.BLADE_DANCE", CardType::Skill, CardRarity::Common, 1),
    ("CARD.BLUR", CardType::Skill, CardRarity::Uncommon, 1),
    (
        "CARD.BOUNCING_FLASK",
        CardType::Skill,
        CardRarity::Uncommon,
        2,
    ),
    (
        "CARD.BUBBLE_BUBBLE",
        CardType::Skill,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.BULLET_TIME", CardType::Skill, CardRarity::Rare, 3),
    ("CARD.BURST", CardType::Skill, CardRarity::Rare, 1),
    (
        "CARD.CALCULATED_GAMBLE",
        CardType::Skill,
        CardRarity::Uncommon,
        0,
    ),
    (
        "CARD.CLOAK_AND_DAGGER",
        CardType::Skill,
        CardRarity::Common,
        1,
    ),
    ("CARD.CORROSIVE_WAVE", CardType::Skill, CardRarity::Rare, 1),
    ("CARD.DAGGER_SPRAY", CardType::Attack, CardRarity::Common, 1),
    ("CARD.DAGGER_THROW", CardType::Attack, CardRarity::Common, 1),
    ("CARD.DASH", CardType::Attack, CardRarity::Uncommon, 2),
    ("CARD.DEADLY_POISON", CardType::Skill, CardRarity::Common, 1),
    ("CARD.DEFLECT", CardType::Skill, CardRarity::Common, 0),
    (
        "CARD.DODGE_AND_ROLL",
        CardType::Skill,
        CardRarity::Common,
        1,
    ),
    ("CARD.ECHOING_SLASH", CardType::Attack, CardRarity::Rare, 1),
    ("CARD.ENVENOM", CardType::Power, CardRarity::Rare, 2),
    ("CARD.ESCAPE_PLAN", CardType::Skill, CardRarity::Uncommon, 0),
    ("CARD.EXPERTISE", CardType::Skill, CardRarity::Uncommon, 1),
    ("CARD.EXPOSE", CardType::Skill, CardRarity::Uncommon, 0),
    ("CARD.FAN_OF_KNIVES", CardType::Power, CardRarity::Rare, 2),
    ("CARD.FINISHER", CardType::Attack, CardRarity::Uncommon, 1),
    ("CARD.FLECHETTES", CardType::Attack, CardRarity::Uncommon, 1),
    ("CARD.FLICK_FLACK", CardType::Attack, CardRarity::Common, 1),
    ("CARD.SCARE", CardType::Skill, CardRarity::Uncommon, 0),
    ("CARD.FOOTWORK", CardType::Power, CardRarity::Uncommon, 1),
    ("CARD.GRAND_FINALE", CardType::Attack, CardRarity::Rare, 0),
    ("CARD.HAND_TRICK", CardType::Skill, CardRarity::Uncommon, 1),
    ("CARD.HAZE", CardType::Skill, CardRarity::Uncommon, 3),
    (
        "CARD.HIDDEN_DAGGERS",
        CardType::Skill,
        CardRarity::Uncommon,
        0,
    ),
    (
        "CARD.INFINITE_BLADES",
        CardType::Power,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.KNIFE_TRAP", CardType::Skill, CardRarity::Rare, 2),
    (
        "CARD.LEADING_STRIKE",
        CardType::Attack,
        CardRarity::Common,
        1,
    ),
    ("CARD.LEG_SWEEP", CardType::Skill, CardRarity::Uncommon, 2),
    ("CARD.MALAISE", CardType::Skill, CardRarity::Rare, 0),
    ("CARD.MASTER_PLANNER", CardType::Power, CardRarity::Rare, 2),
    (
        "CARD.MEMENTO_MORI",
        CardType::Attack,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.MIRAGE", CardType::Skill, CardRarity::Uncommon, 1),
    ("CARD.MURDER", CardType::Attack, CardRarity::Rare, 3),
    ("CARD.NIGHTMARE", CardType::Skill, CardRarity::Rare, 3),
    (
        "CARD.NOXIOUS_FUMES",
        CardType::Power,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.OUTBREAK", CardType::Power, CardRarity::Uncommon, 1),
    (
        "CARD.PHANTOM_BLADES",
        CardType::Power,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.PIERCING_WAIL", CardType::Skill, CardRarity::Common, 1),
    ("CARD.PINPOINT", CardType::Attack, CardRarity::Uncommon, 3),
    (
        "CARD.POISONED_STAB",
        CardType::Attack,
        CardRarity::Common,
        1,
    ),
    ("CARD.POUNCE", CardType::Attack, CardRarity::Uncommon, 2),
    (
        "CARD.PRECISE_CUT",
        CardType::Attack,
        CardRarity::Uncommon,
        0,
    ),
    ("CARD.PREDATOR", CardType::Attack, CardRarity::Common, 2),
    ("CARD.PREPARED", CardType::Skill, CardRarity::Common, 0),
    ("CARD.REFLEX", CardType::Skill, CardRarity::Uncommon, 3),
    ("CARD.RICOCHET", CardType::Attack, CardRarity::Common, 2),
    ("CARD.SERPENT_FORM", CardType::Power, CardRarity::Rare, 3),
    ("CARD.SHADOW_STEP", CardType::Skill, CardRarity::Rare, 1),
    ("CARD.SHADOWMELD", CardType::Skill, CardRarity::Rare, 1),
    ("CARD.SKEWER", CardType::Attack, CardRarity::Uncommon, 0),
    ("CARD.SLICE", CardType::Attack, CardRarity::Common, 0),
    ("CARD.SNAKEBITE", CardType::Skill, CardRarity::Common, 2),
    ("CARD.SPEEDSTER", CardType::Power, CardRarity::Uncommon, 2),
    ("CARD.STORM_OF_STEEL", CardType::Skill, CardRarity::Rare, 1),
    ("CARD.STRANGLE", CardType::Attack, CardRarity::Uncommon, 1),
    ("CARD.SUCKER_PUNCH", CardType::Attack, CardRarity::Common, 1),
    ("CARD.TACTICIAN", CardType::Skill, CardRarity::Uncommon, 3),
    ("CARD.THE_HUNT", CardType::Attack, CardRarity::Rare, 1),
    (
        "CARD.TOOLS_OF_THE_TRADE",
        CardType::Power,
        CardRarity::Rare,
        1,
    ),
    ("CARD.TRACKING", CardType::Power, CardRarity::Rare, 2),
    ("CARD.UNTOUCHABLE", CardType::Skill, CardRarity::Common, 2),
    (
        "CARD.UP_MY_SLEEVE",
        CardType::Skill,
        CardRarity::Uncommon,
        2,
    ),
    (
        "CARD.WELL_LAID_PLANS",
        CardType::Power,
        CardRarity::Uncommon,
        1,
    ),
];

pub(crate) const NECROBINDER_POOL: [(&str, CardType, CardRarity, i32); 80] = [
    ("CARD.AFTERLIFE", CardType::Skill, CardRarity::Common, 1),
    ("CARD.BANSHEES_CRY", CardType::Attack, CardRarity::Rare, 9),
    (
        "CARD.BLIGHT_STRIKE",
        CardType::Attack,
        CardRarity::Common,
        1,
    ),
    (
        "CARD.BONE_SHARDS",
        CardType::Attack,
        CardRarity::Uncommon,
        1,
    ),
    (
        "CARD.BORROWED_TIME",
        CardType::Skill,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.BURY", CardType::Attack, CardRarity::Uncommon, 4),
    ("CARD.CALCIFY", CardType::Power, CardRarity::Uncommon, 1),
    (
        "CARD.CALL_OF_THE_VOID",
        CardType::Power,
        CardRarity::Rare,
        1,
    ),
    (
        "CARD.CAPTURE_SPIRIT",
        CardType::Skill,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.CLEANSE", CardType::Skill, CardRarity::Uncommon, 1),
    ("CARD.COUNTDOWN", CardType::Power, CardRarity::Uncommon, 1),
    (
        "CARD.DANSE_MACABRE",
        CardType::Power,
        CardRarity::Uncommon,
        1,
    ),
    (
        "CARD.DEATH_MARCH",
        CardType::Attack,
        CardRarity::Uncommon,
        1,
    ),
    (
        "CARD.DEATHBRINGER",
        CardType::Skill,
        CardRarity::Uncommon,
        2,
    ),
    ("CARD.DEATHS_DOOR", CardType::Skill, CardRarity::Uncommon, 1),
    ("CARD.DEBILITATE", CardType::Attack, CardRarity::Uncommon, 1),
    ("CARD.DEFILE", CardType::Attack, CardRarity::Common, 1),
    ("CARD.DEFY", CardType::Skill, CardRarity::Common, 1),
    ("CARD.DELAY", CardType::Skill, CardRarity::Uncommon, 2),
    ("CARD.DEMESNE", CardType::Power, CardRarity::Rare, 3),
    ("CARD.DEVOUR_LIFE", CardType::Power, CardRarity::Rare, 1),
    ("CARD.DIRGE", CardType::Skill, CardRarity::Uncommon, 0),
    ("CARD.DRAIN_POWER", CardType::Attack, CardRarity::Common, 1),
    ("CARD.DREDGE", CardType::Skill, CardRarity::Uncommon, 1),
    ("CARD.EIDOLON", CardType::Skill, CardRarity::Rare, 2),
    ("CARD.END_OF_DAYS", CardType::Skill, CardRarity::Rare, 3),
    (
        "CARD.ENFEEBLING_TOUCH",
        CardType::Skill,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.ERADICATE", CardType::Attack, CardRarity::Rare, 0),
    ("CARD.FEAR", CardType::Attack, CardRarity::Common, 1),
    ("CARD.FETCH", CardType::Attack, CardRarity::Uncommon, 0),
    ("CARD.FLATTEN", CardType::Attack, CardRarity::Common, 2),
    ("CARD.FRIENDSHIP", CardType::Power, CardRarity::Uncommon, 1),
    ("CARD.GRAVE_WARDEN", CardType::Skill, CardRarity::Common, 1),
    ("CARD.GRAVEBLAST", CardType::Attack, CardRarity::Common, 1),
    ("CARD.HANG", CardType::Attack, CardRarity::Rare, 1),
    ("CARD.HAUNT", CardType::Power, CardRarity::Uncommon, 1),
    ("CARD.HIGH_FIVE", CardType::Attack, CardRarity::Uncommon, 2),
    ("CARD.INVOKE", CardType::Skill, CardRarity::Common, 1),
    ("CARD.LETHALITY", CardType::Power, CardRarity::Uncommon, 1),
    ("CARD.MELANCHOLY", CardType::Skill, CardRarity::Uncommon, 3),
    ("CARD.MISERY", CardType::Attack, CardRarity::Rare, 0),
    ("CARD.NECRO_MASTERY", CardType::Power, CardRarity::Rare, 2),
    (
        "CARD.NEGATIVE_PULSE",
        CardType::Skill,
        CardRarity::Common,
        1,
    ),
    ("CARD.NEUROSURGE", CardType::Power, CardRarity::Rare, 0),
    ("CARD.NO_ESCAPE", CardType::Skill, CardRarity::Uncommon, 1),
    ("CARD.OBLIVION", CardType::Skill, CardRarity::Rare, 0),
    ("CARD.PAGESTORM", CardType::Power, CardRarity::Uncommon, 1),
    ("CARD.PARSE", CardType::Skill, CardRarity::Uncommon, 1),
    ("CARD.POKE", CardType::Attack, CardRarity::Common, 0),
    ("CARD.PULL_AGGRO", CardType::Skill, CardRarity::Common, 2),
    (
        "CARD.PULL_FROM_BELOW",
        CardType::Attack,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.PUTREFY", CardType::Skill, CardRarity::Uncommon, 1),
    ("CARD.RATTLE", CardType::Attack, CardRarity::Uncommon, 1),
    ("CARD.REANIMATE", CardType::Skill, CardRarity::Rare, 3),
    ("CARD.REAP", CardType::Attack, CardRarity::Common, 3),
    ("CARD.REAPER_FORM", CardType::Power, CardRarity::Rare, 3),
    ("CARD.REAVE", CardType::Attack, CardRarity::Common, 1),
    (
        "CARD.RIGHT_HAND_HAND",
        CardType::Attack,
        CardRarity::Uncommon,
        0,
    ),
    ("CARD.SACRIFICE", CardType::Skill, CardRarity::Rare, 1),
    ("CARD.SCOURGE", CardType::Skill, CardRarity::Common, 1),
    (
        "CARD.SCULPTING_STRIKE",
        CardType::Attack,
        CardRarity::Common,
        1,
    ),
    ("CARD.SEANCE", CardType::Skill, CardRarity::Rare, 1),
    ("CARD.SENTRY_MODE", CardType::Power, CardRarity::Rare, 2),
    ("CARD.SEVERANCE", CardType::Attack, CardRarity::Uncommon, 2),
    ("CARD.SHARED_FATE", CardType::Skill, CardRarity::Rare, 0),
    ("CARD.SHROUD", CardType::Power, CardRarity::Uncommon, 1),
    ("CARD.SIC_EM", CardType::Attack, CardRarity::Uncommon, 1),
    (
        "CARD.SLEIGHT_OF_FLESH",
        CardType::Power,
        CardRarity::Uncommon,
        2,
    ),
    ("CARD.SNAP", CardType::Attack, CardRarity::Common, 1),
    ("CARD.SOUL_STORM", CardType::Attack, CardRarity::Rare, 1),
    ("CARD.SOW", CardType::Attack, CardRarity::Common, 1),
    ("CARD.SPIRIT_OF_ASH", CardType::Power, CardRarity::Rare, 1),
    ("CARD.SPUR", CardType::Skill, CardRarity::Uncommon, 1),
    ("CARD.SQUEEZE", CardType::Attack, CardRarity::Rare, 3),
    ("CARD.THE_SCYTHE", CardType::Attack, CardRarity::Rare, 2),
    ("CARD.TIMES_UP", CardType::Attack, CardRarity::Rare, 2),
    ("CARD.TRANSFIGURE", CardType::Skill, CardRarity::Rare, 1),
    ("CARD.UNDEATH", CardType::Skill, CardRarity::Rare, 0),
    (
        "CARD.VEILPIERCER",
        CardType::Attack,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.WISP", CardType::Skill, CardRarity::Common, 0),
];

pub(crate) const REGENT_POOL: [(&str, CardType, CardRarity, i32); 80] = [
    ("CARD.ALIGNMENT", CardType::Skill, CardRarity::Uncommon, 0),
    ("CARD.ARSENAL", CardType::Power, CardRarity::Rare, 1),
    ("CARD.ASTRAL_PULSE", CardType::Attack, CardRarity::Common, 0),
    (
        "CARD.BEAT_INTO_SHAPE",
        CardType::Attack,
        CardRarity::Rare,
        1,
    ),
    ("CARD.BEGONE", CardType::Skill, CardRarity::Common, 1),
    ("CARD.BIG_BANG", CardType::Skill, CardRarity::Rare, 0),
    ("CARD.BLACK_HOLE", CardType::Power, CardRarity::Uncommon, 1),
    ("CARD.BOMBARDMENT", CardType::Attack, CardRarity::Rare, 3),
    ("CARD.BULWARK", CardType::Skill, CardRarity::Uncommon, 2),
    ("CARD.BUNDLE_OF_JOY", CardType::Skill, CardRarity::Rare, 1),
    (
        "CARD.CELESTIAL_MIGHT",
        CardType::Attack,
        CardRarity::Common,
        2,
    ),
    ("CARD.CHARGE", CardType::Skill, CardRarity::Uncommon, 1),
    (
        "CARD.CHILD_OF_THE_STARS",
        CardType::Power,
        CardRarity::Uncommon,
        1,
    ),
    (
        "CARD.CLOAK_OF_STARS",
        CardType::Skill,
        CardRarity::Common,
        0,
    ),
    (
        "CARD.COLLISION_COURSE",
        CardType::Attack,
        CardRarity::Common,
        0,
    ),
    ("CARD.COMET", CardType::Attack, CardRarity::Rare, 0),
    ("CARD.CONQUEROR", CardType::Skill, CardRarity::Uncommon, 1),
    ("CARD.CONVERGENCE", CardType::Skill, CardRarity::Uncommon, 1),
    (
        "CARD.COSMIC_INDIFFERENCE",
        CardType::Skill,
        CardRarity::Common,
        1,
    ),
    ("CARD.CRASH_LANDING", CardType::Attack, CardRarity::Rare, 1),
    (
        "CARD.CRESCENT_SPEAR",
        CardType::Attack,
        CardRarity::Common,
        1,
    ),
    ("CARD.CRUSH_UNDER", CardType::Attack, CardRarity::Common, 1),
    (
        "CARD.DECISIONS_DECISIONS",
        CardType::Skill,
        CardRarity::Rare,
        0,
    ),
    ("CARD.DEVASTATE", CardType::Attack, CardRarity::Uncommon, 1),
    ("CARD.DYING_STAR", CardType::Attack, CardRarity::Rare, 1),
    (
        "CARD.FOREGONE_CONCLUSION",
        CardType::Skill,
        CardRarity::Rare,
        1,
    ),
    ("CARD.FURNACE", CardType::Power, CardRarity::Uncommon, 1),
    (
        "CARD.GAMMA_BLAST",
        CardType::Attack,
        CardRarity::Uncommon,
        0,
    ),
    ("CARD.GATHER_LIGHT", CardType::Skill, CardRarity::Common, 1),
    ("CARD.GENESIS", CardType::Power, CardRarity::Rare, 2),
    ("CARD.GLIMMER", CardType::Skill, CardRarity::Uncommon, 1),
    ("CARD.GLITTERSTREAM", CardType::Skill, CardRarity::Common, 2),
    ("CARD.GLOW", CardType::Skill, CardRarity::Common, 1),
    ("CARD.GUARDS", CardType::Skill, CardRarity::Rare, 2),
    ("CARD.GUIDING_STAR", CardType::Attack, CardRarity::Common, 1),
    ("CARD.HEAVENLY_DRILL", CardType::Attack, CardRarity::Rare, 0),
    ("CARD.HEGEMONY", CardType::Attack, CardRarity::Uncommon, 2),
    (
        "CARD.HEIRLOOM_HAMMER",
        CardType::Attack,
        CardRarity::Rare,
        2,
    ),
    ("CARD.HIDDEN_CACHE", CardType::Skill, CardRarity::Common, 1),
    ("CARD.I_AM_INVINCIBLE", CardType::Skill, CardRarity::Rare, 1),
    (
        "CARD.KINGLY_KICK",
        CardType::Attack,
        CardRarity::Uncommon,
        4,
    ),
    (
        "CARD.KINGLY_PUNCH",
        CardType::Attack,
        CardRarity::Uncommon,
        1,
    ),
    (
        "CARD.KNOCKOUT_BLOW",
        CardType::Attack,
        CardRarity::Uncommon,
        3,
    ),
    (
        "CARD.KNOW_THY_PLACE",
        CardType::Skill,
        CardRarity::Common,
        0,
    ),
    (
        "CARD.LUNAR_BLAST",
        CardType::Attack,
        CardRarity::Uncommon,
        0,
    ),
    ("CARD.MAKE_IT_SO", CardType::Attack, CardRarity::Rare, 0),
    (
        "CARD.MANIFEST_AUTHORITY",
        CardType::Skill,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.MONARCHS_GAZE", CardType::Power, CardRarity::Rare, 2),
    ("CARD.MONOLOGUE", CardType::Skill, CardRarity::Uncommon, 0),
    ("CARD.NEUTRON_AEGIS", CardType::Power, CardRarity::Rare, 1),
    ("CARD.ORBIT", CardType::Power, CardRarity::Uncommon, 2),
    (
        "CARD.PALE_BLUE_DOT",
        CardType::Power,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.PARRY", CardType::Power, CardRarity::Uncommon, 1),
    (
        "CARD.PARTICLE_WALL",
        CardType::Skill,
        CardRarity::Uncommon,
        0,
    ),
    ("CARD.PATTER", CardType::Skill, CardRarity::Common, 1),
    ("CARD.PHOTON_CUT", CardType::Attack, CardRarity::Common, 1),
    (
        "CARD.PILLAR_OF_CREATION",
        CardType::Power,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.PROPHESIZE", CardType::Skill, CardRarity::Uncommon, 2),
    ("CARD.QUASAR", CardType::Skill, CardRarity::Uncommon, 0),
    ("CARD.RADIATE", CardType::Attack, CardRarity::Uncommon, 0),
    ("CARD.REFINE_BLADE", CardType::Skill, CardRarity::Common, 1),
    ("CARD.REFLECT", CardType::Skill, CardRarity::Uncommon, 1),
    ("CARD.RESONANCE", CardType::Skill, CardRarity::Uncommon, 1),
    (
        "CARD.ROYAL_GAMBLE",
        CardType::Skill,
        CardRarity::Uncommon,
        0,
    ),
    ("CARD.ROYALTIES", CardType::Power, CardRarity::Rare, 1),
    ("CARD.SEEKING_EDGE", CardType::Power, CardRarity::Rare, 1),
    ("CARD.SEVEN_STARS", CardType::Attack, CardRarity::Rare, 2),
    (
        "CARD.SHINING_STRIKE",
        CardType::Attack,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.SOLAR_STRIKE", CardType::Attack, CardRarity::Common, 1),
    (
        "CARD.SPECTRUM_SHIFT",
        CardType::Power,
        CardRarity::Uncommon,
        2,
    ),
    (
        "CARD.SPOILS_OF_BATTLE",
        CardType::Skill,
        CardRarity::Common,
        1,
    ),
    ("CARD.STARDUST", CardType::Attack, CardRarity::Uncommon, 0),
    (
        "CARD.SUMMON_FORTH",
        CardType::Skill,
        CardRarity::Uncommon,
        1,
    ),
    (
        "CARD.SUPERMASSIVE",
        CardType::Attack,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.SWORD_SAGE", CardType::Power, CardRarity::Rare, 2),
    (
        "CARD.TERRAFORMING",
        CardType::Skill,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.THE_SMITH", CardType::Skill, CardRarity::Rare, 1),
    ("CARD.TYRANNY", CardType::Power, CardRarity::Rare, 1),
    ("CARD.VOID_FORM", CardType::Power, CardRarity::Rare, 3),
    (
        "CARD.WROUGHT_IN_WAR",
        CardType::Attack,
        CardRarity::Common,
        1,
    ),
];

/// The Defect's pool, in the pool's own declaration order.
pub(crate) const DEFECT_POOL: [(&str, CardType, CardRarity, i32); 80] = [
    (
        "CARD.ADAPTIVE_STRIKE",
        CardType::Attack,
        CardRarity::Rare,
        2,
    ),
    ("CARD.ALL_FOR_ONE", CardType::Attack, CardRarity::Rare, 2),
    (
        "CARD.BALL_LIGHTNING",
        CardType::Attack,
        CardRarity::Common,
        1,
    ),
    ("CARD.BARRAGE", CardType::Attack, CardRarity::Common, 1),
    ("CARD.BEAM_CELL", CardType::Attack, CardRarity::Common, 0),
    ("CARD.BOOST_AWAY", CardType::Skill, CardRarity::Common, 0),
    (
        "CARD.BOOT_SEQUENCE",
        CardType::Skill,
        CardRarity::Uncommon,
        0,
    ),
    ("CARD.BUFFER", CardType::Power, CardRarity::Rare, 2),
    ("CARD.BULK_UP", CardType::Power, CardRarity::Uncommon, 2),
    ("CARD.CAPACITOR", CardType::Power, CardRarity::Uncommon, 1),
    ("CARD.CHAOS", CardType::Skill, CardRarity::Uncommon, 1),
    (
        "CARD.CHARGE_BATTERY",
        CardType::Skill,
        CardRarity::Common,
        1,
    ),
    ("CARD.CHILL", CardType::Skill, CardRarity::Uncommon, 0),
    ("CARD.CLAW", CardType::Attack, CardRarity::Common, 0),
    ("CARD.COLD_SNAP", CardType::Attack, CardRarity::Common, 1),
    ("CARD.COMPACT", CardType::Skill, CardRarity::Uncommon, 1),
    (
        "CARD.COMPILE_DRIVER",
        CardType::Attack,
        CardRarity::Common,
        1,
    ),
    (
        "CARD.CONSUMING_SHADOW",
        CardType::Power,
        CardRarity::Rare,
        2,
    ),
    ("CARD.COOLANT", CardType::Power, CardRarity::Rare, 1),
    ("CARD.COOLHEADED", CardType::Skill, CardRarity::Common, 1),
    ("CARD.CREATIVE_AI", CardType::Power, CardRarity::Rare, 3),
    ("CARD.DARKNESS", CardType::Skill, CardRarity::Uncommon, 1),
    ("CARD.DEFRAGMENT", CardType::Power, CardRarity::Rare, 1),
    (
        "CARD.DOUBLE_ENERGY",
        CardType::Skill,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.ECHO_FORM", CardType::Power, CardRarity::Rare, 3),
    ("CARD.FERAL", CardType::Power, CardRarity::Uncommon, 2),
    (
        "CARD.FIGHT_THROUGH",
        CardType::Skill,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.FLAK_CANNON", CardType::Attack, CardRarity::Rare, 2),
    (
        "CARD.FOCUSED_STRIKE",
        CardType::Attack,
        CardRarity::Common,
        1,
    ),
    ("CARD.FTL", CardType::Attack, CardRarity::Uncommon, 0),
    ("CARD.FUSION", CardType::Skill, CardRarity::Uncommon, 1),
    (
        "CARD.GENETIC_ALGORITHM",
        CardType::Skill,
        CardRarity::Rare,
        1,
    ),
    ("CARD.GLACIER", CardType::Skill, CardRarity::Uncommon, 2),
    ("CARD.GLASSWORK", CardType::Skill, CardRarity::Uncommon, 1),
    (
        "CARD.GO_FOR_THE_EYES",
        CardType::Attack,
        CardRarity::Common,
        0,
    ),
    ("CARD.GUNK_UP", CardType::Attack, CardRarity::Common, 1),
    ("CARD.HAILSTORM", CardType::Power, CardRarity::Uncommon, 1),
    ("CARD.HELIX_DRILL", CardType::Attack, CardRarity::Rare, 0),
    ("CARD.HOLOGRAM", CardType::Skill, CardRarity::Common, 1),
    ("CARD.HOTFIX", CardType::Skill, CardRarity::Common, 0),
    ("CARD.HYPERBEAM", CardType::Attack, CardRarity::Rare, 2),
    ("CARD.ICE_LANCE", CardType::Attack, CardRarity::Rare, 3),
    ("CARD.ITERATION", CardType::Power, CardRarity::Uncommon, 1),
    ("CARD.LEAP", CardType::Skill, CardRarity::Common, 1),
    ("CARD.LIGHTNING_ROD", CardType::Skill, CardRarity::Common, 1),
    ("CARD.LOOP", CardType::Power, CardRarity::Uncommon, 1),
    (
        "CARD.MACHINE_LEARNING",
        CardType::Power,
        CardRarity::Rare,
        1,
    ),
    ("CARD.METEOR_STRIKE", CardType::Attack, CardRarity::Rare, 5),
    ("CARD.MODDED", CardType::Skill, CardRarity::Rare, 0),
    (
        "CARD.MOMENTUM_STRIKE",
        CardType::Attack,
        CardRarity::Common,
        1,
    ),
    ("CARD.MULTI_CAST", CardType::Skill, CardRarity::Rare, 0),
    ("CARD.NULL", CardType::Attack, CardRarity::Uncommon, 2),
    ("CARD.OVERCLOCK", CardType::Skill, CardRarity::Uncommon, 0),
    ("CARD.RAINBOW", CardType::Skill, CardRarity::Rare, 2),
    ("CARD.REBOOT", CardType::Skill, CardRarity::Rare, 0),
    ("CARD.REFRACT", CardType::Attack, CardRarity::Uncommon, 3),
    (
        "CARD.ROCKET_PUNCH",
        CardType::Attack,
        CardRarity::Uncommon,
        2,
    ),
    ("CARD.SCAVENGE", CardType::Skill, CardRarity::Uncommon, 1),
    ("CARD.SCRAPE", CardType::Attack, CardRarity::Uncommon, 1),
    (
        "CARD.SHADOW_SHIELD",
        CardType::Skill,
        CardRarity::Uncommon,
        2,
    ),
    ("CARD.SHATTER", CardType::Attack, CardRarity::Rare, 1),
    ("CARD.SIGNAL_BOOST", CardType::Skill, CardRarity::Rare, 1),
    ("CARD.SKIM", CardType::Skill, CardRarity::Uncommon, 1),
    ("CARD.SMOKESTACK", CardType::Power, CardRarity::Uncommon, 1),
    ("CARD.SPINNER", CardType::Power, CardRarity::Rare, 1),
    ("CARD.STORM", CardType::Power, CardRarity::Uncommon, 1),
    ("CARD.SUBROUTINE", CardType::Power, CardRarity::Uncommon, 1),
    ("CARD.SUNDER", CardType::Attack, CardRarity::Uncommon, 3),
    ("CARD.SUPERCRITICAL", CardType::Skill, CardRarity::Rare, 0),
    (
        "CARD.SWEEPING_BEAM",
        CardType::Attack,
        CardRarity::Common,
        1,
    ),
    ("CARD.SYNCHRONIZE", CardType::Skill, CardRarity::Uncommon, 1),
    ("CARD.SYNTHESIS", CardType::Attack, CardRarity::Uncommon, 2),
    ("CARD.TEMPEST", CardType::Skill, CardRarity::Uncommon, 0),
    ("CARD.TESLA_COIL", CardType::Attack, CardRarity::Uncommon, 0),
    ("CARD.THUNDER", CardType::Power, CardRarity::Uncommon, 1),
    (
        "CARD.TRASH_TO_TREASURE",
        CardType::Power,
        CardRarity::Rare,
        1,
    ),
    ("CARD.TURBO", CardType::Skill, CardRarity::Common, 0),
    ("CARD.UPROAR", CardType::Attack, CardRarity::Common, 2),
    ("CARD.VOLTAIC", CardType::Skill, CardRarity::Rare, 3),
    ("CARD.WHITE_NOISE", CardType::Skill, CardRarity::Uncommon, 1),
];

pub(crate) const COLORLESS_POOL: [(&str, CardType, CardRarity, i32); 53] = [
    ("CARD.ALCHEMIZE", CardType::Skill, CardRarity::Rare, 1),
    ("CARD.ANOINTED", CardType::Skill, CardRarity::Rare, 1),
    ("CARD.AUTOMATION", CardType::Power, CardRarity::Uncommon, 1),
    ("CARD.BEAT_DOWN", CardType::Skill, CardRarity::Rare, 3),
    ("CARD.BOLAS", CardType::Attack, CardRarity::Rare, 0),
    ("CARD.CALAMITY", CardType::Power, CardRarity::Rare, 3),
    ("CARD.CATASTROPHE", CardType::Skill, CardRarity::Uncommon, 2),
    (
        "CARD.DARK_SHACKLES",
        CardType::Skill,
        CardRarity::Uncommon,
        0,
    ),
    ("CARD.DISCOVERY", CardType::Skill, CardRarity::Uncommon, 1),
    (
        "CARD.DRAMATIC_ENTRANCE",
        CardType::Attack,
        CardRarity::Uncommon,
        0,
    ),
    ("CARD.ENTROPY", CardType::Power, CardRarity::Rare, 1),
    ("CARD.EQUILIBRIUM", CardType::Skill, CardRarity::Uncommon, 2),
    ("CARD.ETERNAL_ARMOR", CardType::Power, CardRarity::Rare, 3),
    ("CARD.FASTEN", CardType::Power, CardRarity::Uncommon, 1),
    ("CARD.FINESSE", CardType::Skill, CardRarity::Uncommon, 0),
    ("CARD.FISTICUFFS", CardType::Attack, CardRarity::Uncommon, 1),
    (
        "CARD.FLASH_OF_STEEL",
        CardType::Attack,
        CardRarity::Uncommon,
        0,
    ),
    ("CARD.GOLD_AXE", CardType::Attack, CardRarity::Rare, 1),
    ("CARD.HAND_OF_GREED", CardType::Attack, CardRarity::Rare, 2),
    ("CARD.HIDDEN_GEM", CardType::Skill, CardRarity::Rare, 1),
    ("CARD.IMPATIENCE", CardType::Skill, CardRarity::Uncommon, 0),
    (
        "CARD.JACK_OF_ALL_TRADES",
        CardType::Skill,
        CardRarity::Uncommon,
        0,
    ),
    ("CARD.JACKPOT", CardType::Attack, CardRarity::Rare, 3),
    (
        "CARD.MASTER_OF_STRATEGY",
        CardType::Skill,
        CardRarity::Rare,
        0,
    ),
    ("CARD.MAYHEM", CardType::Power, CardRarity::Rare, 2),
    ("CARD.MIND_BLAST", CardType::Attack, CardRarity::Uncommon, 1),
    ("CARD.NOSTALGIA", CardType::Power, CardRarity::Rare, 1),
    ("CARD.OMNISLICE", CardType::Attack, CardRarity::Uncommon, 0),
    ("CARD.PANACHE", CardType::Power, CardRarity::Uncommon, 0),
    (
        "CARD.PANIC_BUTTON",
        CardType::Skill,
        CardRarity::Uncommon,
        0,
    ),
    ("CARD.PREP_TIME", CardType::Power, CardRarity::Uncommon, 1),
    ("CARD.PRODUCTION", CardType::Skill, CardRarity::Uncommon, 0),
    ("CARD.PROLONG", CardType::Skill, CardRarity::Uncommon, 0),
    ("CARD.PROWESS", CardType::Power, CardRarity::Uncommon, 1),
    ("CARD.PURITY", CardType::Skill, CardRarity::Uncommon, 0),
    ("CARD.REND", CardType::Attack, CardRarity::Rare, 2),
    (
        "CARD.RESTLESSNESS",
        CardType::Skill,
        CardRarity::Uncommon,
        0,
    ),
    ("CARD.ROLLING_BOULDER", CardType::Power, CardRarity::Rare, 3),
    ("CARD.SALVO", CardType::Attack, CardRarity::Rare, 1),
    ("CARD.SCRAWL", CardType::Skill, CardRarity::Rare, 1),
    (
        "CARD.SECRET_TECHNIQUE",
        CardType::Skill,
        CardRarity::Rare,
        0,
    ),
    ("CARD.SECRET_WEAPON", CardType::Skill, CardRarity::Rare, 0),
    (
        "CARD.SEEKER_STRIKE",
        CardType::Attack,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.SHOCKWAVE", CardType::Skill, CardRarity::Uncommon, 2),
    ("CARD.SPLASH", CardType::Skill, CardRarity::Uncommon, 1),
    ("CARD.STRATAGEM", CardType::Power, CardRarity::Uncommon, 1),
    ("CARD.THE_BOMB", CardType::Skill, CardRarity::Uncommon, 2),
    ("CARD.THE_GAMBIT", CardType::Skill, CardRarity::Rare, 0),
    (
        "CARD.THINKING_AHEAD",
        CardType::Skill,
        CardRarity::Uncommon,
        0,
    ),
    (
        "CARD.THRUMMING_HATCHET",
        CardType::Attack,
        CardRarity::Uncommon,
        1,
    ),
    (
        "CARD.ULTIMATE_DEFEND",
        CardType::Skill,
        CardRarity::Uncommon,
        1,
    ),
    (
        "CARD.ULTIMATE_STRIKE",
        CardType::Attack,
        CardRarity::Uncommon,
        1,
    ),
    ("CARD.VOLLEY", CardType::Attack, CardRarity::Uncommon, 0),
];

/// The status card pool, in the pool's own declaration order.
///
/// A status card transforming rolls against this whole list: the transform's
/// common/uncommon/rare narrowing is skipped for an original of the Status or
/// Curse rarity, so nothing is filtered out but the card itself.
const STATUS_POOL: [&str; 12] = [
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
];

/// The token card pool, in the pool's own declaration order. Every card of it
/// carries the Token rarity, so a transform of one rolls against the colorless
/// pool rather than against this list.
const TOKEN_POOL: [&str; 14] = [
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
];

/// The quest card pool, in the pool's own declaration order. A quest card
/// transforms against the colorless pool, so this list is what says the three
/// stand in a pool of their own rather than what any roll reads.
const QUEST_POOL: [&str; 3] = ["CARD.BYRDONIS_EGG", "CARD.LANTERN_KEY", "CARD.SPOILS_MAP"];

/// The event card pool, in the pool's own declaration order. The last two are not
/// in alphabetical order in the build either.
const EVENT_POOL: [&str; 27] = [
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
];

/// The deprecated pool, whole: the stand-in a save reads back where the card it
/// names is gone stands in a pool of its own, and nothing else does.
const DEPRECATED_POOL: [&str; 1] = ["CARD.DEPRECATED_CARD"];

/// The cards that display as belonging to a pool other than their own. Ten
/// cards of the event pool are the whole of that set: they are event cards
/// read as belonging to the colour they came from, which is what a filter
/// asking for a colourless card sees.
const VISUAL_POOL_OVERRIDES: [(&str, &str); 10] = [
    ("CARD.CALTROPS", "CARD_POOL.SILENT_CARD_POOL"),
    ("CARD.CLASH", "CARD_POOL.IRONCLAD_CARD_POOL"),
    ("CARD.DISTRACTION", "CARD_POOL.SILENT_CARD_POOL"),
    ("CARD.DUAL_WIELD", "CARD_POOL.IRONCLAD_CARD_POOL"),
    ("CARD.ENTRENCH", "CARD_POOL.IRONCLAD_CARD_POOL"),
    ("CARD.HELLO_WORLD", "CARD_POOL.DEFECT_CARD_POOL"),
    ("CARD.OUTMANEUVER", "CARD_POOL.SILENT_CARD_POOL"),
    ("CARD.REBOUND", "CARD_POOL.DEFECT_CARD_POOL"),
    ("CARD.RIP_AND_TEAR", "CARD_POOL.DEFECT_CARD_POOL"),
    ("CARD.STACK", "CARD_POOL.DEFECT_CARD_POOL"),
];

/// The cards a generation effect in combat is never allowed to hand out.
/// Everything else is fair game for an effect that rolls against a pool.
const NEVER_GENERATED_IN_COMBAT: [&str; 14] = [
    "CARD.ALCHEMIZE",
    "CARD.DISINTEGRATION",
    "CARD.FEED",
    "CARD.FRANTIC_ESCAPE",
    "CARD.HAND_OF_GREED",
    "CARD.HIDDEN_GEM",
    "CARD.MIND_ROT",
    "CARD.NEOWS_FURY",
    "CARD.NOT_YET",
    "CARD.ROYALTIES",
    "CARD.SLOTH",
    "CARD.SOOT",
    "CARD.THE_HUNT",
    "CARD.WASTE_AWAY",
];

/// The cards whose energy cost is X. Their canonical cost reads as
/// nothing, and a filter that asks for a card costing nothing rules them out
/// all the same.
const COSTS_X: [&str; 8] = [
    "CARD.CASCADE",
    "CARD.DIRGE",
    "CARD.ERADICATE",
    "CARD.HEAVENLY_DRILL",
    "CARD.MALAISE",
    "CARD.SKEWER",
    "CARD.VOLLEY",
    "CARD.WHIRLWIND",
];

/// What rarity a card carries because of the pool it stands in. The character
/// pools and the colorless pool are the only ones that hand a card a rolled
/// rarity; every other family is one rarity, which the card itself declares.
pub(crate) fn pool_rarity(model_id: &str) -> Option<CardRarity> {
    IRONCLAD_POOL
        .iter()
        .chain(SILENT_POOL.iter())
        .chain(NECROBINDER_POOL.iter())
        .chain(REGENT_POOL.iter())
        .chain(DEFECT_POOL.iter())
        .chain(COLORLESS_POOL.iter())
        .find(|(id, _, _, _)| *id == model_id)
        .map(|(_, _, rarity, _)| *rarity)
}

/// The Silent's cards that stand in the Silent's pool without a rolled rarity:
/// the four basics, the two ancients, and the two multiplayer-only cards. A
/// pool holds them all, which is what a transform of one rolls against.
const SILENT_OUTSIDE_THE_ROLLED_RARITIES: [&str; 8] = [
    "CARD.DEFEND_SILENT",
    "CARD.FLANKING",
    "CARD.NEUTRALIZE",
    "CARD.SNEAKY",
    "CARD.STRIKE_SILENT",
    "CARD.SUPPRESS",
    "CARD.SURVIVOR",
    "CARD.WRAITH_FORM",
];

/// The Necrobinder's cards that stand in the Necrobinder's pool without a
/// rolled rarity: the four basics, the two ancients, and the two
/// multiplayer-only cards.
const NECROBINDER_OUTSIDE_THE_ROLLED_RARITIES: [&str; 8] = [
    "CARD.BODYGUARD",
    "CARD.DEFEND_NECROBINDER",
    "CARD.FORBIDDEN_GRIMOIRE",
    "CARD.GLIMPSE_BEYOND",
    "CARD.LEGION_OF_BONE",
    "CARD.PROTECTOR",
    "CARD.STRIKE_NECROBINDER",
    "CARD.UNLEASH",
];

/// The Regent's cards that stand in the Regent's pool without a rolled rarity:
/// the four basics, the two ancients, and the two multiplayer-only cards.
const REGENT_OUTSIDE_THE_ROLLED_RARITIES: [&str; 8] = [
    "CARD.DEFEND_REGENT",
    "CARD.FALLING_STAR",
    "CARD.HAMMER_TIME",
    "CARD.LARGESSE",
    "CARD.METEOR_SHOWER",
    "CARD.STRIKE_REGENT",
    "CARD.THE_SEALED_THRONE",
    "CARD.VENERATE",
];

/// The Defect's cards that stand in the Defect's pool without a rolled rarity:
/// the four basics, the two ancients, and the two multiplayer-only cards.
const DEFECT_OUTSIDE_THE_ROLLED_RARITIES: [&str; 8] = [
    "CARD.BIASED_COGNITION",
    "CARD.DEFEND_DEFECT",
    "CARD.DUALCAST",
    "CARD.ENERGY_SURGE",
    "CARD.IGNITION",
    "CARD.QUADCAST",
    "CARD.STRIKE_DEFECT",
    "CARD.ZAP",
];

/// The pool a card belongs to: the first pool that holds it. The game checks
/// the five character pools, in its own order, and then the seven shared ones;
/// no card of the pinned build stands in two of them, so the walk settles it
/// whatever the order.
///
/// A character pool's own basics and ancients stand in it beside the rolled
/// three, so they are named here as well. The run's own colour is the one
/// character whose basics are registered as its cards rather than listed, which
/// is what `is_the_characters` answers for.
pub(crate) fn own_pool(model_id: &str, is_the_characters: bool) -> Option<&'static str> {
    let rolled = |pool: &[(&str, CardType, CardRarity, i32)]| {
        pool.iter().any(|(id, _, _, _)| *id == model_id)
    };
    if rolled(&IRONCLAD_POOL) {
        return Some("CARD_POOL.IRONCLAD_CARD_POOL");
    }
    if rolled(&SILENT_POOL) || SILENT_OUTSIDE_THE_ROLLED_RARITIES.contains(&model_id) {
        return Some("CARD_POOL.SILENT_CARD_POOL");
    }
    if rolled(&REGENT_POOL) || REGENT_OUTSIDE_THE_ROLLED_RARITIES.contains(&model_id) {
        return Some("CARD_POOL.REGENT_CARD_POOL");
    }
    if rolled(&NECROBINDER_POOL) || NECROBINDER_OUTSIDE_THE_ROLLED_RARITIES.contains(&model_id) {
        return Some("CARD_POOL.NECROBINDER_CARD_POOL");
    }
    if rolled(&DEFECT_POOL) || DEFECT_OUTSIDE_THE_ROLLED_RARITIES.contains(&model_id) {
        return Some("CARD_POOL.DEFECT_CARD_POOL");
    }
    if rolled(&COLORLESS_POOL) {
        return Some("CARD_POOL.COLORLESS_CARD_POOL");
    }
    if CURSE_POOL.iter().any(|(id, _)| *id == model_id) {
        return Some("CARD_POOL.CURSE_CARD_POOL");
    }
    if DEPRECATED_POOL.contains(&model_id) {
        return Some("CARD_POOL.DEPRECATED_CARD_POOL");
    }
    if EVENT_POOL.contains(&model_id) {
        return Some("CARD_POOL.EVENT_CARD_POOL");
    }
    if QUEST_POOL.contains(&model_id) {
        return Some("CARD_POOL.QUEST_CARD_POOL");
    }
    if STATUS_POOL.contains(&model_id) {
        return Some("CARD_POOL.STATUS_CARD_POOL");
    }
    if TOKEN_POOL.contains(&model_id) {
        return Some("CARD_POOL.TOKEN_CARD_POOL");
    }
    is_the_characters.then_some("CARD_POOL.IRONCLAD_CARD_POOL")
}

/// The pool a card displays as: its own pool, except for the ten event cards
/// that show a character's instead.
pub(crate) fn visual_pool(model_id: &str, is_the_characters: bool) -> Option<&'static str> {
    VISUAL_POOL_OVERRIDES
        .iter()
        .find(|(card, _)| *card == model_id)
        .map_or_else(
            || own_pool(model_id, is_the_characters),
            |(_, pool)| Some(pool),
        )
}

pub(crate) fn costs_x(model_id: &str) -> bool {
    COSTS_X.contains(&model_id)
}

pub(crate) fn can_be_generated_in_combat(model_id: &str) -> bool {
    !NEVER_GENERATED_IN_COMBAT.contains(&model_id)
}

/// The curse card pool, in the pool's own declaration order, and which of its
/// cards a modifier may generate.
///
/// Eight of the eighteen may not be generated by a modifier. A body
/// drawing a curse out of the pool orders what is left by model ID, so the
/// order this list is written in does not decide the draw; it is the pool's own
/// all the same.
const CURSE_POOL: [(&str, bool); 18] = [
    ("CARD.ASCENDERS_BANE", false),
    ("CARD.BAD_LUCK", false),
    ("CARD.CLUMSY", true),
    ("CARD.CURSE_OF_THE_BELL", false),
    ("CARD.DEBT", true),
    ("CARD.DECAY", true),
    ("CARD.DOUBT", true),
    ("CARD.ENTHRALLED", false),
    ("CARD.FOLLY", false),
    ("CARD.GREED", false),
    ("CARD.GUILTY", true),
    ("CARD.INJURY", true),
    ("CARD.NORMALITY", true),
    ("CARD.POOR_SLEEP", false),
    ("CARD.REGRET", true),
    ("CARD.SHAME", true),
    ("CARD.SPORE_MIND", false),
    ("CARD.WRITHE", true),
];

/// The curse pool as a body drawing one reads it.
#[must_use]
pub fn curse_pool_cards() -> Vec<sts2_engine::PoolCard> {
    CURSE_POOL
        .iter()
        .map(|(model_id, generated)| sts2_engine::PoolCard {
            model_id: crate::id(model_id),
            card_type: CardType::Curse,
            rarity: CardRarity::Curse,
            energy_cost: -1,
            costs_x: false,
            can_be_generated_in_combat: can_be_generated_in_combat(model_id),
            can_be_generated_by_modifiers: *generated,
        })
        .collect()
}

/// The colorless pool as a generator reads it.
#[must_use]
pub fn colorless_pool_cards() -> Vec<sts2_engine::PoolCard> {
    COLORLESS_POOL
        .iter()
        .map(
            |(model_id, card_type, rarity, energy_cost)| sts2_engine::PoolCard {
                model_id: crate::id(model_id),
                card_type: *card_type,
                rarity: *rarity,
                energy_cost: *energy_cost,
                costs_x: costs_x(model_id),
                can_be_generated_in_combat: can_be_generated_in_combat(model_id),
                can_be_generated_by_modifiers: true,
            },
        )
        .collect()
}

/// One card of a shared family, as the pool holds it.
///
/// The five families whose cards declare their own rarity — the deprecated
/// stand-in, the event cards, the quest cards, the statuses and the tokens —
/// take what the pool needs off the registered card rather than off a table
/// beside it: only the membership and its order belong to the pool.
fn family_card(
    content: &sts2_engine::ContentRegistry,
    model_id: &str,
) -> Option<sts2_engine::PoolCard> {
    let id = crate::id(model_id);
    let definition = content.card(&id)?;
    Some(sts2_engine::PoolCard {
        model_id: id,
        card_type: definition.card_type,
        rarity: definition.rarity,
        energy_cost: definition.energy_cost,
        costs_x: definition.costs_x,
        can_be_generated_in_combat: can_be_generated_in_combat(model_id),
        can_be_generated_by_modifiers: true,
    })
}

/// Every card pool the game defines, registered with the membership the game
/// gives it.
///
/// The five character pools come first and the seven shared ones after, which
/// is the order a card's own pool is looked up in. The game's mock pool is not
/// among them: it is deliberately left out of the shared pools, and every card
/// it holds is test scaffolding.
pub(crate) fn register_card_pools(content: &mut sts2_engine::ContentRegistry) {
    for character in crate::rewards::IMPLEMENTED_CHARACTERS {
        let character = crate::id(character);
        if let Some(pool) = crate::rewards::character_card_pool_id(&character) {
            content.add_card_pool(sts2_engine::CardPoolDefinition {
                model_id: pool.clone(),
                is_colorless: false,
                cards: crate::rewards::pool_cards(&character),
            });
            content.set_character_card_pool(character, pool);
        }
    }
    content.add_card_pool(sts2_engine::CardPoolDefinition {
        model_id: crate::id("CARD_POOL.COLORLESS_CARD_POOL"),
        is_colorless: true,
        cards: colorless_pool_cards(),
    });
    // The curse pool, which is what a body that charges a deck a curse draws
    // against, and what a curse transforming rolls its replacement out of.
    content.add_card_pool(sts2_engine::CardPoolDefinition {
        model_id: crate::id("CARD_POOL.CURSE_CARD_POOL"),
        is_colorless: false,
        cards: curse_pool_cards(),
    });
    for (model_id, is_colorless, members) in [
        (
            "CARD_POOL.DEPRECATED_CARD_POOL",
            true,
            DEPRECATED_POOL.as_slice(),
        ),
        ("CARD_POOL.EVENT_CARD_POOL", true, EVENT_POOL.as_slice()),
        ("CARD_POOL.QUEST_CARD_POOL", false, QUEST_POOL.as_slice()),
        ("CARD_POOL.STATUS_CARD_POOL", false, STATUS_POOL.as_slice()),
        ("CARD_POOL.TOKEN_CARD_POOL", true, TOKEN_POOL.as_slice()),
    ] {
        let cards = members
            .iter()
            .filter_map(|member| family_card(content, member))
            .collect();
        content.add_card_pool(sts2_engine::CardPoolDefinition {
            model_id: crate::id(model_id),
            is_colorless,
            cards,
        });
    }
}
