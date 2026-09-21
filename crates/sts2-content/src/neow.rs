use sts2_rng::{Rng, deterministic_hash};

const CURSES: [&str; 8] = [
    "CURSED_PEARL",
    "HEFTY_TABLET",
    "LARGE_CAPSULE",
    "LEAFY_POULTICE",
    "NEOWS_BONES",
    "PRECARIOUS_SHEARS",
    "SILKEN_TRESS",
    "SILVER_CRUCIBLE",
];

const POSITIVE: [&str; 13] = [
    "ARCANE_SCROLL",
    "BOOMING_CONCH",
    "FISHING_ROD",
    "GOLDEN_PEARL",
    "KALEIDOSCOPE",
    "LEAD_PAPERWEIGHT",
    "LOST_COFFER",
    "NEOWS_TORMENT",
    "NEW_LEAF",
    "PHIAL_HOLSTER",
    "PRECISE_SCISSORS",
    "SCROLL_BOXES",
    "WINGED_BOOTS",
];

/// Generates Neow's three opening relic choices for the pinned all-unlocked
/// single-player baseline. Event-local RNG is independent from run counters.
#[must_use]
pub fn standard_neow_options(seed: &str) -> Vec<String> {
    standard_neow_page(seed).0
}

/// The same page, with where the event's own RNG stands once it is built.
///
/// An event has a single RNG stream, created once when the event is set up, so
/// the counter the page leaves behind is what any later read of Neow's stream
/// starts from.
#[must_use]
pub fn standard_neow_page(seed: &str) -> (Vec<String>, u64) {
    let run_seed = deterministic_hash(seed) as u32;
    let event_seed = run_seed.wrapping_add(deterministic_hash("NEOW") as u32);
    let mut rng = Rng::new(event_seed);

    let curse = CURSES[rng
        .next_index(CURSES.len())
        .expect("static option count is valid")
        .expect("curse list is nonempty")];
    let mut positive = POSITIVE.to_vec();
    match curse {
        "CURSED_PEARL" => positive.retain(|option| *option != "GOLDEN_PEARL"),
        "HEFTY_TABLET" => positive.retain(|option| *option != "ARCANE_SCROLL"),
        "LEAFY_POULTICE" => positive.retain(|option| *option != "NEW_LEAF"),
        "PRECARIOUS_SHEARS" => positive.retain(|option| *option != "PRECISE_SCISSORS"),
        _ => {}
    }
    if curse != "LARGE_CAPSULE" {
        positive.push(if rng.next_bool() {
            "LAVA_ROCK"
        } else {
            "SMALL_CAPSULE"
        });
    }
    positive.push(if rng.next_bool() {
        "NUTRITIOUS_OYSTER"
    } else {
        "STONE_HUMIDIFIER"
    });
    positive.push(if rng.next_bool() {
        "NEOWS_TALISMAN"
    } else {
        "POMANDER"
    });
    rng.shuffle(&mut positive);

    let page = positive
        .into_iter()
        .take(2)
        .chain(std::iter::once(curse))
        .map(|relic| format!("NEOW.pages.INITIAL.options.{relic}"))
        .collect();
    (page, rng.counter())
}
