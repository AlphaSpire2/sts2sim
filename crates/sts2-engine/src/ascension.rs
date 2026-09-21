//! The ascension a run is played at, and the branch every scaled number reads.
//!
//! A run carries one level, `RunConfig::ascension`, and everything the level
//! changes asks the same two questions the build asks: is this run at or above
//! a named level, and which of two values does that answer select. Both
//! are written here so that a model or a rule that branches on an ascension
//! names the level rather than the number, and so that both sides of every
//! branch stay written down whatever level a run is generated at.

/// The ascension levels, in the order the build numbers them. The order is the
/// rule: a level applies to every run whose number is at or above it, so each
/// name is the level at which its effect starts applying.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum AscensionLevel {
    None = 0,
    SwarmingElites = 1,
    WearyTraveler = 2,
    Poverty = 3,
    TightBelt = 4,
    AscendersBane = 5,
    Inflation = 6,
    Scarcity = 7,
    ToughEnemies = 8,
    DeadlyEnemies = 9,
    DoubleBoss = 10,
}

/// The highest level the build will start a run at, and the highest
/// `RunConfig::validate` accepts.
pub const MAX_ASCENSION: u8 = 10;

/// The ascension a run is generated at when nothing names one: the level the
/// registered fixture baseline was recorded at, and the level every model's
/// unbranched form is written for.
pub const BASELINE_ASCENSION: u8 = 0;

/// Whether a run at `ascension` is at or above this level.
#[must_use]
pub const fn has_ascension(ascension: u8, level: AscensionLevel) -> bool {
    ascension >= level as u8
}

/// The ascension value at or above that level, and the fallback below it.
///
/// The ascended value is the first of the pair, the way every scaled pair is
/// stated here: reading the two arguments the other way around silently
/// inverts a scaling.
#[must_use]
pub const fn value_if_ascension(
    ascension: u8,
    level: AscensionLevel,
    ascension_value: i32,
    fallback: i32,
) -> i32 {
    if has_ascension(ascension, level) {
        ascension_value
    } else {
        fallback
    }
}

/// The same branch over a number that scales fractionally rather than in whole
/// steps.
#[must_use]
pub fn float_if_ascension(
    ascension: u8,
    level: AscensionLevel,
    ascension_value: f32,
    fallback: f32,
) -> f32 {
    if has_ascension(ascension, level) {
        ascension_value
    } else {
        fallback
    }
}

/// What a Poverty run keeps of the gold it would otherwise be awarded.
pub const POVERTY_GOLD_MULTIPLIER: f64 = 0.75;

/// What a Weary Traveler run keeps of the health an ancient would otherwise
/// restore.
pub const WEARY_TRAVELER_HEAL_MULTIPLIER: f64 = 0.8;

/// How much of the health an ancient would restore a run at this level gets
/// back.
///
/// An ancient heals everything the player is missing, and Weary Traveler takes
/// a fifth off that amount before the heal rather than after it. The heal
/// lands as a whole number of health, truncated, so what the fifths leave over
/// is dropped rather than rounded.
#[must_use]
pub fn ancient_heal_amount(ascension: u8, missing_hp: i32) -> i32 {
    if has_ascension(ascension, AscensionLevel::WearyTraveler) {
        (f64::from(missing_hp) * WEARY_TRAVELER_HEAL_MULTIPLIER) as i32
    } else {
        missing_hp
    }
}

/// How many elite points an act's map is laid out with: a base of five,
/// multiplied by 1.6 under Swarming Elites and by 1.0 below it, then rounded.
///
/// The multiplier is applied to the count rather than the count being written
/// twice, because that is how the build scales it.
#[must_use]
pub fn elite_map_point_count(ascension: u8) -> usize {
    let scaled = 5.0_f32
        * if has_ascension(ascension, AscensionLevel::SwarmingElites) {
            1.6
        } else {
            1.0
        };
    // The build rounds half to even here, and both `8.0` and `5.0` are
    // reached exactly, so the tie-breaking mode never shows; it is written
    // the build's way regardless.
    crate::simulator::round_half_even(f64::from(scaled)) as usize
}

/// How many potion slots a run opens with before any ascension takes one away.
/// It is a run-level constant rather than anything a character declares.
pub const INITIAL_MAX_POTION_SLOTS: usize = 3;

/// Tight Belt takes one potion slot away: how many slots a run opens with,
/// starting from the character's own count.
#[must_use]
pub fn potion_slot_count(ascension: u8, character_slots: usize) -> usize {
    if has_ascension(ascension, AscensionLevel::TightBelt) {
        character_slots.saturating_sub(1)
    } else {
        character_slots
    }
}

/// How much likelier a non-rare offered card is to arrive upgraded for each
/// act already walked.
#[must_use]
pub fn upgraded_card_odd_scaling(ascension: u8) -> f32 {
    float_if_ascension(ascension, AscensionLevel::Scarcity, 0.125, 0.25)
}

/// What a merchant's card removal costs the first time and how much each
/// removal adds to the price, under Inflation and below it.
#[must_use]
pub const fn card_removal_prices(ascension: u8) -> (i32, i32) {
    (
        value_if_ascension(ascension, AscensionLevel::Inflation, 100, 75),
        value_if_ascension(ascension, AscensionLevel::Inflation, 50, 25),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_level_applies_at_its_own_number_and_every_number_above_it() {
        assert!(!has_ascension(0, AscensionLevel::SwarmingElites));
        assert!(has_ascension(1, AscensionLevel::SwarmingElites));
        assert!(has_ascension(10, AscensionLevel::SwarmingElites));
        assert!(!has_ascension(9, AscensionLevel::DoubleBoss));
        assert!(has_ascension(10, AscensionLevel::DoubleBoss));
        // `AscensionLevel::None` is zero, so every run has it.
        assert!(has_ascension(0, AscensionLevel::None));
    }

    #[test]
    fn the_ascended_value_is_the_first_of_the_pair() {
        assert_eq!(
            value_if_ascension(7, AscensionLevel::Scarcity, 100, 75),
            100
        );
        assert_eq!(value_if_ascension(6, AscensionLevel::Scarcity, 100, 75), 75);
    }

    #[test]
    fn swarming_elites_rounds_five_times_one_and_six_tenths_to_eight() {
        assert_eq!(elite_map_point_count(0), 5);
        assert_eq!(elite_map_point_count(1), 8);
        assert_eq!(elite_map_point_count(10), 8);
    }
}
