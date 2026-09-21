//! The pinned relic pools, in the exact order the build declares them.
//!
//! A relic pool says one thing a run reads: which relics stand in it, and in
//! what order. Filling a grab bag walks that order, buckets what it walks by
//! the relic's own rarity, and shuffles each bucket in the order the buckets
//! were first made, so both the membership and the order are fidelity. A pool
//! is pruned by epoch and this baseline has every epoch revealed, so what is
//! written here is the whole pool.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use sts2_core::ModelId;
use sts2_engine::{RelicGrabBagState, RelicRarity};
use sts2_rng::Rng;

/// The shared relic pool, whole. Every run's shared grab bag is this list, and
/// every run's player bag is this list with the walking character's own pool
/// appended and the four rolled rarities kept.
///
/// Three of the hundred and eighteen — a lasting candy, a razor tooth and
/// sparkling rouge — also stand in the event pool. A relic's own pool is the
/// first pool that holds it and the event pool is reached first, so those three
/// answer with the event pool; nothing but the energy icon reads that, and both
/// bags still hold all three.
pub(crate) const SHARED_RELIC_POOL: [&str; 118] = [
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
];

/// The Ironclad's relic pool, whole: the eight the character carries, one of
/// which is the starter they open the run holding. A starter is of no rolled
/// rarity, so the player bag drops it and keeps the other seven.
pub(crate) const IRONCLAD_RELIC_POOL: [&str; 8] = [
    "RELIC.BRIMSTONE",
    "RELIC.BURNING_BLOOD",
    "RELIC.CHARONS_ASHES",
    "RELIC.DEMON_TONGUE",
    "RELIC.PAPER_PHROG",
    "RELIC.RED_SKULL",
    "RELIC.RUINED_HELMET",
    "RELIC.SELF_FORMING_CLAY",
];

/// The Silent's relic pool, whole: the eight the character carries, one of
/// which is the starter they open the run holding. A starter is of no rolled
/// rarity, so the player bag drops it and keeps the other seven.
pub(crate) const SILENT_RELIC_POOL: [&str; 8] = [
    "RELIC.HELICAL_DART",
    "RELIC.NINJA_SCROLL",
    "RELIC.PAPER_KRANE",
    "RELIC.RING_OF_THE_SNAKE",
    "RELIC.SNECKO_SKULL",
    "RELIC.TINGSHA",
    "RELIC.TOUGH_BANDAGES",
    "RELIC.TWISTED_FUNNEL",
];

/// The Regent's relic pool, whole: the eight the character carries, one of
/// which is the starter they open the run holding. A starter is of no rolled
/// rarity, so the player bag drops it and keeps the other seven.
pub(crate) const REGENT_RELIC_POOL: [&str; 8] = [
    "RELIC.DIVINE_RIGHT",
    "RELIC.FENCING_MANUAL",
    "RELIC.GALACTIC_DUST",
    "RELIC.LUNAR_PASTRY",
    "RELIC.MINI_REGENT",
    "RELIC.ORANGE_DOUGH",
    "RELIC.REGALITE",
    "RELIC.VITRUVIAN_MINION",
];

/// The Necrobinder's relic pool, whole: the eight the character carries, one of
/// which is the starter they open the run holding. A starter is of no rolled
/// rarity, so the player bag drops it and keeps the other seven.
pub(crate) const NECROBINDER_RELIC_POOL: [&str; 8] = [
    "RELIC.BIG_HAT",
    "RELIC.BONE_FLUTE",
    "RELIC.BOOK_REPAIR_KNIFE",
    "RELIC.BOOKMARK",
    "RELIC.BOUND_PHYLACTERY",
    "RELIC.FUNERARY_MASK",
    "RELIC.IVORY_TILE",
    "RELIC.UNDYING_SIGIL",
];

/// The Defect's relic pool, whole: the eight the character carries, one of
/// which is the starter they open the run holding. A starter is of no rolled
/// rarity, so the player bag drops it and keeps the other seven.
pub(crate) const DEFECT_RELIC_POOL: [&str; 8] = [
    "RELIC.CRACKED_CORE",
    "RELIC.DATA_DISK",
    "RELIC.EMOTION_CHIP",
    "RELIC.GOLD_PLATED_CABLES",
    "RELIC.POWER_CELL",
    "RELIC.METRONOME",
    "RELIC.RUNIC_CAPACITOR",
    "RELIC.SYMBIOTIC_VIRUS",
];

/// The event relic pool, whole: what only an event hands over.
///
/// No bag is filled from it. Ninety-eight of the hundred and forty are ancients,
/// thirty-four carry the Event rarity, five are the refined starters a touch of
/// Orobas deals, and three stand in the shared pool as well.
pub(crate) const EVENT_RELIC_POOL: [&str; 140] = [
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
];

/// The fallback pool, whole: the circlet a refinement hands over where the
/// starter it was pointed at has no refined form. It is of no
/// rarity, so no bag holds it.
pub(crate) const FALLBACK_RELIC_POOL: [&str; 1] = ["RELIC.CIRCLET"];

/// The deprecated pool, whole: the stand-in a save reads back where the relic
/// it names is gone. It is of no rarity, so no bag holds
/// it.
pub(crate) const DEPRECATED_RELIC_POOL: [&str; 1] = ["RELIC.DEPRECATED_RELIC"];

/// The relic pool that belongs to a character.
pub(crate) fn character_relic_pool(
    character: &ModelId,
) -> Option<(&'static str, &'static [&'static str])> {
    match character.to_string().as_str() {
        "CHARACTER.IRONCLAD" => Some(("RELIC_POOL.IRONCLAD_RELIC_POOL", &IRONCLAD_RELIC_POOL)),
        "CHARACTER.SILENT" => Some(("RELIC_POOL.SILENT_RELIC_POOL", &SILENT_RELIC_POOL)),
        "CHARACTER.REGENT" => Some(("RELIC_POOL.REGENT_RELIC_POOL", &REGENT_RELIC_POOL)),
        "CHARACTER.NECROBINDER" => {
            Some(("RELIC_POOL.NECROBINDER_RELIC_POOL", &NECROBINDER_RELIC_POOL))
        }
        "CHARACTER.DEFECT" => Some(("RELIC_POOL.DEFECT_RELIC_POOL", &DEFECT_RELIC_POOL)),
        _ => None,
    }
}

/// The rarity of every relic this workspace registers.
///
/// Filling a grab bag buckets a pool by the relic's own rarity, so the bags
/// need it; reading it off the registered definitions is what keeps a relic's
/// rarity declared once, where the relic is.
fn rarities() -> &'static BTreeMap<ModelId, RelicRarity> {
    static RARITIES: OnceLock<BTreeMap<ModelId, RelicRarity>> = OnceLock::new();
    RARITIES.get_or_init(|| {
        crate::relics::every_relic()
            .into_iter()
            .map(|definition| (definition.model_id, definition.rarity))
            .collect()
    })
}

/// What rarity the pinned build gives this relic. A relic this baseline does
/// not register is of no rarity, which is what no bag holds.
fn rarity_of(relic: &ModelId) -> RelicRarity {
    rarities().get(relic).copied().unwrap_or(RelicRarity::None)
}

/// The four rarities a pull ever asks for, which is what the player's own bag
/// is narrowed to as it is filled.
const ROLLED_RARITIES: [RelicRarity; 4] = [
    RelicRarity::Common,
    RelicRarity::Uncommon,
    RelicRarity::Rare,
    RelicRarity::Shop,
];

/// Fills a grab bag: the relics walked in the pool's own order and dropped
/// into a deque per rarity, then every deque shuffled in the order the deques
/// were first made.
///
/// `rolled_only` is the cut the player's own bag is filled through, which drops
/// every relic whose rarity is not one of the four that are rolled. The shared
/// bag is filled without that cut, so it carries the event and ancient relics
/// of the shared pool as well.
fn populate(rng: &mut Rng, relics: &[ModelId], rolled_only: bool) -> RelicGrabBagState {
    let mut bag = RelicGrabBagState::default();
    let mut deques: Vec<RelicRarity> = Vec::new();
    for relic in relics {
        let rarity = rarity_of(relic);
        if rolled_only && !ROLLED_RARITIES.contains(&rarity) {
            continue;
        }
        if !deques.contains(&rarity) {
            deques.push(rarity);
        }
        bag.bucket_mut(rarity).push(relic.clone());
    }
    for rarity in deques {
        rng.shuffle(bag.bucket_mut(rarity));
    }
    bag
}

/// The two bags a run opens with, in the order the build fills them: the
/// shared one off the shared pool alone, then the player's off that pool with
/// the walking character's appended.
pub(crate) fn generate_relic_bags(
    rng: &mut Rng,
    character: &ModelId,
) -> (RelicGrabBagState, RelicGrabBagState) {
    let shared: Vec<ModelId> = SHARED_RELIC_POOL.iter().copied().map(crate::id).collect();
    let mut player = shared.clone();
    if let Some((_, own)) = character_relic_pool(character) {
        player.extend(own.iter().copied().map(crate::id));
    }
    let shared = populate(rng, &shared, false);
    let player = populate(rng, &player, true);
    (shared, player)
}

/// Every relic pool the game defines, registered with the membership the game
/// gives it: the five character pools first, then the four shared ones, which
/// is the order a relic's own pool is looked up in.
pub(crate) fn register_relic_pools(content: &mut sts2_engine::ContentRegistry) {
    for character in crate::rewards::IMPLEMENTED_CHARACTERS {
        let character = crate::id(character);
        if let Some((pool, members)) = character_relic_pool(&character) {
            let pool = crate::id(pool);
            content.add_relic_pool(
                pool.clone(),
                members.iter().copied().map(crate::id).collect(),
            );
            content.set_character_relic_pool(character, pool);
        }
    }
    for (model_id, members) in [
        (
            "RELIC_POOL.DEPRECATED_RELIC_POOL",
            DEPRECATED_RELIC_POOL.as_slice(),
        ),
        ("RELIC_POOL.EVENT_RELIC_POOL", EVENT_RELIC_POOL.as_slice()),
        (
            "RELIC_POOL.FALLBACK_RELIC_POOL",
            FALLBACK_RELIC_POOL.as_slice(),
        ),
        ("RELIC_POOL.SHARED_RELIC_POOL", SHARED_RELIC_POOL.as_slice()),
    ] {
        content.add_relic_pool(
            crate::id(model_id),
            members.iter().copied().map(crate::id).collect(),
        );
    }
}
