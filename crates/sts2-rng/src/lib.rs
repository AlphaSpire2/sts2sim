//! Deterministic random behavior from STS2 `v0.107.1`.
//!
//! The high-level [`Rng`] counts one consumed Xoshiro output per uniform draw.
//! Composite operations such as Gaussian sampling and shuffle consume through
//! those primitives, so their counters match save data and replay projections.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

const SPLITMIX_INCREMENT: u64 = 11_400_714_819_323_198_485;
const SPLITMIX_MULTIPLIER_1: u64 = 13_787_848_793_156_543_929;
const SPLITMIX_MULTIPLIER_2: u64 = 10_723_151_780_598_845_931;
const DOUBLE_UNIT: f64 = 1.110_223_024_625_156_5e-16;

/// Errors raised before an RNG operation consumes a value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RngError {
    /// The exclusive upper bound was not greater than the lower bound.
    EmptyRange,
    /// A counter cannot be fast-forwarded to a smaller value.
    CounterRewind { current: u64, target: u64 },
    /// Serialized state named a different seed.
    SeedMismatch,
}

impl Display for RngError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRange => formatter.write_str("minimum must be lower than maximum"),
            Self::CounterRewind { current, target } => {
                write!(
                    formatter,
                    "cannot fast-forward counter from {current} to {target}"
                )
            }
            Self::SeedMismatch => formatter.write_str("RNG seed cannot change while loading"),
        }
    }
}

impl Error for RngError {}

/// The `SplitMix64` initialization step used by Xoshiro256**.
#[must_use]
pub fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(SPLITMIX_INCREMENT);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(SPLITMIX_MULTIPLIER_1);
    value = (value ^ (value >> 27)).wrapping_mul(SPLITMIX_MULTIPLIER_2);
    value ^ (value >> 31)
}

/// The raw Xoshiro256** generator embedded in the pinned game build.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MegaRandom {
    state: [u64; 4],
}

impl MegaRandom {
    /// Initializes all four words by running `SplitMix64` from `seed`.
    #[must_use]
    pub fn new(mut seed: u64) -> Self {
        Self {
            state: [
                splitmix64(&mut seed),
                splitmix64(&mut seed),
                splitmix64(&mut seed),
                splitmix64(&mut seed),
            ],
        }
    }

    /// Returns the complete internal state, primarily for golden-vector tests.
    #[must_use]
    pub const fn state(&self) -> [u64; 4] {
        self.state
    }

    /// Returns the next 64 output bits and advances the generator once.
    pub fn next_u64(&mut self) -> u64 {
        let [mut s0, mut s1, mut s2, mut s3] = self.state;
        let result = s1.wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let shifted = s1 << 17;
        s2 ^= s0;
        s3 ^= s1;
        s1 ^= s2;
        s0 ^= s3;
        s2 ^= shifted;
        s3 = s3.rotate_left(45);
        self.state = [s0, s1, s2, s3];
        result
    }

    /// Fills bytes in the little-endian order used by supported STS2 targets.
    pub fn fill_bytes(&mut self, destination: &mut [u8]) {
        for chunk in destination.chunks_mut(8) {
            let bytes = self.next_u64().to_le_bytes();
            chunk.copy_from_slice(&bytes[..chunk.len()]);
        }
    }

    /// Returns a sample in `[0, 1)` using the upper 53 output bits.
    pub fn next_double(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * DOUBLE_UNIT
    }

    /// Returns a sample in `[0, 1)` using the upper 24 output bits.
    pub fn next_float(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 * 5.960_464_5e-8_f32
    }

    /// Returns all 32 low output bits.
    pub fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    /// Returns a nonnegative integer, including `i32::MAX`.
    pub fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 33) as i32
    }

    /// Returns the top output bit as a boolean.
    pub fn next_bool(&mut self) -> bool {
        self.next_u64() & (1_u64 << 63) != 0
    }
}

impl Default for MegaRandom {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Serializable identity of one high-level RNG.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RngState {
    pub seed: u32,
    pub counter: u64,
}

/// A counted gameplay RNG matching the build's own.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rng {
    seed: u32,
    counter: u64,
    random: MegaRandom,
}

impl Rng {
    /// Creates a stream and advances it to `counter`.
    #[must_use]
    pub fn with_counter(seed: u32, counter: u64) -> Self {
        let mut rng = Self {
            seed,
            counter: 0,
            random: MegaRandom::new(u64::from(seed)),
        };
        // Construction starts from zero, so this cannot fail.
        rng.fast_forward(counter).expect("fresh RNG can advance");
        rng
    }

    /// Creates a stream at counter zero.
    #[must_use]
    pub fn new(seed: u32) -> Self {
        Self::with_counter(seed, 0)
    }

    /// Derives the named stream using wrapping 32-bit addition.
    #[must_use]
    pub fn named(seed: u32, name: &str) -> Self {
        Self::new(seed.wrapping_add(deterministic_hash(name) as u32))
    }

    /// Derives a stream the build builds from the run seed string alone.
    ///
    /// The seed must already be canonical; the stream is the one the lobby
    /// stands up, seeded from the deterministic hash of that seed string taken
    /// as an unsigned 32-bit number and then named.
    #[must_use]
    pub fn for_seed(seed: &str, stream: SeedStream) -> Self {
        Self::named(deterministic_hash(seed) as u32, stream.name())
    }

    /// Derives a content-local stream from run, player, model entry, and mixin.
    #[must_use]
    pub fn content_local(run_seed: u32, player_slot: u32, model_entry: &str, mixin: u32) -> Self {
        let seed = run_seed
            .wrapping_add(player_slot)
            .wrapping_add(deterministic_hash(model_entry) as u32)
            .wrapping_add(mixin);
        Self::new(seed)
    }

    #[must_use]
    pub const fn seed(&self) -> u32 {
        self.seed
    }

    #[must_use]
    pub const fn counter(&self) -> u64 {
        self.counter
    }

    #[must_use]
    pub const fn to_state(&self) -> RngState {
        RngState {
            seed: self.seed,
            counter: self.counter,
        }
    }

    #[must_use]
    pub fn from_state(state: RngState) -> Self {
        Self::with_counter(state.seed, state.counter)
    }

    /// Advances by discarding values, exactly as the game's load path does.
    pub fn fast_forward(&mut self, target: u64) -> Result<(), RngError> {
        if target < self.counter {
            return Err(RngError::CounterRewind {
                current: self.counter,
                target,
            });
        }
        while self.counter < target {
            self.consume();
        }
        Ok(())
    }

    /// Loads a counter. A lower counter reconstructs the stream from its seed.
    pub fn load_counter(&mut self, target: u64) {
        if target < self.counter {
            *self = Self::with_counter(self.seed, target);
        } else {
            self.fast_forward(target).expect("target was checked");
        }
    }

    fn consume(&mut self) -> u64 {
        self.counter += 1;
        self.random.next_u64()
    }

    fn uniform_double(&mut self) -> f64 {
        (self.consume() >> 11) as f64 * DOUBLE_UNIT
    }

    /// The game's fair coin flip: one roll over two outcomes, true on the
    /// first of them.
    pub fn next_bool(&mut self) -> bool {
        (self.uniform_double() * 2.0) as i32 == 0
    }

    /// Samples `[0, max_exclusive)`.
    pub fn next_int(&mut self, max_exclusive: i32) -> Result<i32, RngError> {
        if max_exclusive < 1 {
            return Err(RngError::EmptyRange);
        }
        Ok((self.uniform_double() * f64::from(max_exclusive)) as i32)
    }

    /// Samples `[min_inclusive, max_exclusive)`, including ranges wider than `i32::MAX`.
    pub fn next_int_range(
        &mut self,
        min_inclusive: i32,
        max_exclusive: i32,
    ) -> Result<i32, RngError> {
        if min_inclusive >= max_exclusive {
            return Err(RngError::EmptyRange);
        }
        let width = i64::from(max_exclusive) - i64::from(min_inclusive);
        let offset = (self.uniform_double() * width as f64) as i64;
        Ok((i64::from(min_inclusive) + offset) as i32)
    }

    /// Samples `[min_inclusive, max_exclusive)` with unsigned bounds.
    pub fn next_unsigned_int(
        &mut self,
        min_inclusive: u32,
        max_exclusive: u32,
    ) -> Result<u32, RngError> {
        if min_inclusive >= max_exclusive {
            return Err(RngError::EmptyRange);
        }
        let width = f64::from(max_exclusive - min_inclusive);
        Ok(min_inclusive + (self.uniform_double() * width) as u32)
    }

    /// Samples the same double-based float path as the high-level game wrapper.
    pub fn next_float(&mut self, min: f32, max: f32) -> Result<f32, RngError> {
        if min > max {
            return Err(RngError::EmptyRange);
        }
        Ok((self.uniform_double() * f64::from(max - min) + f64::from(min)) as f32)
    }

    pub fn next_double(&mut self, min: f64, max: f64) -> Result<f64, RngError> {
        if min > max {
            return Err(RngError::EmptyRange);
        }
        Ok(self.uniform_double() * (max - min) + min)
    }

    /// Box-Muller Gaussian with rejection in normalized `[0, 1]` space.
    pub fn next_gaussian_double(
        &mut self,
        mean: f64,
        std_dev: f64,
        min: f64,
        max: f64,
    ) -> Result<f64, RngError> {
        if min > max {
            return Err(RngError::EmptyRange);
        }
        loop {
            let first = 1.0 - self.next_double(0.0, 1.0)?;
            let second = 1.0 - self.next_double(0.0, 1.0)?;
            let magnitude = (-2.0 * first.ln()).sqrt();
            let normalized = magnitude * (std::f64::consts::TAU * second).cos();
            let value = mean + normalized * std_dev;
            if gaussian_value_is_accepted(value) {
                return Ok(value * (max - min) + min);
            }
        }
    }

    pub fn next_gaussian_float(
        &mut self,
        mean: f32,
        std_dev: f32,
        min: f32,
        max: f32,
    ) -> Result<f32, RngError> {
        Ok(self.next_gaussian_double(
            f64::from(mean),
            f64::from(std_dev),
            f64::from(min),
            f64::from(max),
        )? as f32)
    }

    /// Integer Gaussian using sine and ties-to-even rounding.
    pub fn next_gaussian_int(&mut self, mean: i32, std_dev: i32, min: i32, max: i32) -> i32 {
        loop {
            let first = 1.0 - self.uniform_double();
            let second = 1.0 - self.uniform_double();
            let normal = (-2.0 * first.ln()).sqrt() * (std::f64::consts::TAU * second).sin();
            let value = round_ties_even(f64::from(mean) + f64::from(std_dev) * normal) as i32;
            if (min..=max).contains(&value) {
                return value;
            }
        }
    }

    /// Selects an index, consuming nothing for an empty slice.
    pub fn next_index(&mut self, length: usize) -> Result<Option<usize>, RngError> {
        if length == 0 {
            return Ok(None);
        }
        let length = i32::try_from(length).map_err(|_| RngError::EmptyRange)?;
        Ok(Some(self.next_int(length)? as usize))
    }

    /// Selects an item, consuming nothing for an empty slice.
    pub fn next_item<'a, T>(&mut self, items: &'a [T]) -> Result<Option<&'a T>, RngError> {
        self.next_index(items.len())
            .map(|index| index.map(|index| &items[index]))
    }

    /// Selects using the game's `f32` accumulation and inclusive-zero threshold.
    pub fn weighted_index(&mut self, weights: &[f32]) -> Option<usize> {
        let random = self.next_float(0.0, 1.0).expect("ordered unit range");
        Self::weighted_index_from(random, weights)
    }

    #[must_use]
    pub fn weighted_index_from(random: f32, weights: &[f32]) -> Option<usize> {
        let total = weights.iter().copied().sum::<f32>();
        let mut remainder = random * total;
        for (index, weight) in weights.iter().copied().enumerate() {
            remainder -= weight;
            if remainder <= 0.0 {
                return Some(index);
            }
        }
        None
    }

    /// In-place Fisher-Yates shuffle.
    pub fn shuffle<T>(&mut self, values: &mut [T]) {
        for upper in (1..values.len()).rev() {
            let selected = self
                .next_index(upper + 1)
                .expect("nonempty slice length fits game integer")
                .expect("length is nonzero");
            values.swap(upper, selected);
        }
    }
}

impl Default for Rng {
    fn default() -> Self {
        Self::new(0)
    }
}

fn round_ties_even(value: f64) -> f64 {
    let floor = value.floor();
    let fraction = value - floor;
    if fraction < 0.5 {
        floor
    } else if fraction > 0.5 {
        floor + 1.0
    } else if floor % 2.0 == 0.0 {
        floor
    } else {
        floor + 1.0
    }
}

// A range test rejects NaN, but the build accepts it: it asks the two
// comparisons separately and both are false for NaN. Keep the two comparisons
// rather than a range, for exact edge behavior.
#[allow(clippy::manual_range_contains)]
fn gaussian_value_is_accepted(value: f64) -> bool {
    !(value < 0.0 || value > 1.0)
}

/// Canonicalizes a display seed in the order the build does: upper-case
/// first, then O to zero and I to one, then trim.
#[must_use]
pub fn canonicalize_seed(seed: &str) -> String {
    seed.to_uppercase()
        .replace('O', "0")
        .replace('I', "1")
        .trim()
        .to_owned()
}

/// Deterministic string hash over UTF-16 code units with wrapping `i32` math.
#[must_use]
pub fn deterministic_hash(value: &str) -> i32 {
    let units: Vec<u16> = value.encode_utf16().collect();
    let mut first = 352_654_597_i32;
    let mut second = first;
    for pair in units.chunks(2) {
        first = first.wrapping_mul(33) ^ i32::from(pair[0]);
        if let Some(unit) = pair.get(1) {
            second = second.wrapping_mul(33) ^ i32::from(*unit);
        }
    }
    first.wrapping_add(second.wrapping_mul(1_566_083_941))
}

/// Named run-level streams, in the pinned enum order.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStream {
    UpFront,
    Shuffle,
    UnknownMapPoint,
    CombatCardGeneration,
    CombatPotionGeneration,
    CombatCardSelection,
    CombatEnergyCosts,
    CombatTargets,
    MonsterAi,
    Niche,
    CombatOrbs,
    TreasureRoomRelics,
}

impl RunStream {
    pub const ALL: [Self; 12] = [
        Self::UpFront,
        Self::Shuffle,
        Self::UnknownMapPoint,
        Self::CombatCardGeneration,
        Self::CombatPotionGeneration,
        Self::CombatCardSelection,
        Self::CombatEnergyCosts,
        Self::CombatTargets,
        Self::MonsterAi,
        Self::Niche,
        Self::CombatOrbs,
        Self::TreasureRoomRelics,
    ];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::UpFront => "up_front",
            Self::Shuffle => "shuffle",
            Self::UnknownMapPoint => "unknown_map_point",
            Self::CombatCardGeneration => "combat_card_generation",
            Self::CombatPotionGeneration => "combat_potion_generation",
            Self::CombatCardSelection => "combat_card_selection",
            Self::CombatEnergyCosts => "combat_energy_costs",
            Self::CombatTargets => "combat_targets",
            Self::MonsterAi => "monster_ai",
            Self::Niche => "niche",
            Self::CombatOrbs => "combat_orbs",
            Self::TreasureRoomRelics => "treasure_room_relics",
        }
    }
}

/// Named streams the build derives from the run seed and then drops.
///
/// A [`RunStream`] belongs to the run: it is created with the run's state,
/// carried through it, saved with it, and reported in its counters. These are
/// not. The lobby builds `act_selection` from the seed string before a run
/// exists, reads it, and lets it go, so it never joins a run's stream set and
/// never appears in a projection's counters. Keeping it out of [`RunStream`]
/// is what keeps those counters the twelve the recording carries.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeedStream {
    ActSelection,
}

impl SeedStream {
    pub const ALL: [Self; 1] = [Self::ActSelection];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ActSelection => "act_selection",
        }
    }
}

/// Named per-player streams, in the pinned enum order.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerStream {
    Rewards,
    Shops,
    Transformations,
}

impl PlayerStream {
    pub const ALL: [Self; 3] = [Self::Rewards, Self::Shops, Self::Transformations];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Rewards => "rewards",
            Self::Shops => "shops",
            Self::Transformations => "transformations",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SerializableRunRngSet {
    pub seed: String,
    pub counters: BTreeMap<RunStream, u64>,
}

/// Complete run-level stream set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunRngSet {
    string_seed: String,
    seed: u32,
    streams: BTreeMap<RunStream, Rng>,
}

impl RunRngSet {
    /// Creates streams from an already canonical seed string.
    #[must_use]
    pub fn new(seed: impl Into<String>) -> Self {
        let string_seed = seed.into();
        let numeric_seed = deterministic_hash(&string_seed) as u32;
        let streams = RunStream::ALL
            .into_iter()
            .map(|stream| (stream, Rng::named(numeric_seed, stream.name())))
            .collect();
        Self {
            string_seed,
            seed: numeric_seed,
            streams,
        }
    }

    #[must_use]
    pub fn canonicalized(seed: &str) -> Self {
        Self::new(canonicalize_seed(seed))
    }

    #[must_use]
    pub fn string_seed(&self) -> &str {
        &self.string_seed
    }

    #[must_use]
    pub const fn seed(&self) -> u32 {
        self.seed
    }

    #[must_use]
    pub fn stream(&self, stream: RunStream) -> &Rng {
        &self.streams[&stream]
    }

    pub fn stream_mut(&mut self, stream: RunStream) -> &mut Rng {
        self.streams
            .get_mut(&stream)
            .expect("all run streams exist")
    }

    #[must_use]
    pub fn to_serializable(&self) -> SerializableRunRngSet {
        SerializableRunRngSet {
            seed: self.string_seed.clone(),
            counters: self
                .streams
                .iter()
                .map(|(kind, rng)| (*kind, rng.counter()))
                .collect(),
        }
    }

    #[must_use]
    pub fn from_serializable(state: &SerializableRunRngSet) -> Self {
        let mut result = Self::new(state.seed.clone());
        for (&kind, &counter) in &state.counters {
            result.stream_mut(kind).load_counter(counter);
        }
        result
    }

    pub fn load(&mut self, state: &SerializableRunRngSet) -> Result<(), RngError> {
        if self.string_seed != state.seed {
            return Err(RngError::SeedMismatch);
        }
        for (&kind, &counter) in &state.counters {
            self.stream_mut(kind).load_counter(counter);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SerializablePlayerRngSet {
    pub seed: u32,
    pub counters: BTreeMap<PlayerStream, u64>,
}

/// Complete stream set for one stable player slot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayerRngSet {
    seed: u32,
    streams: BTreeMap<PlayerStream, Rng>,
}

impl PlayerRngSet {
    #[must_use]
    pub fn new(seed: u32) -> Self {
        let streams = PlayerStream::ALL
            .into_iter()
            .map(|stream| (stream, Rng::named(seed, stream.name())))
            .collect();
        Self { seed, streams }
    }

    /// Derives the player seed from the run seed string and stable slot index.
    #[must_use]
    pub fn for_player(run_seed: &str, slot_index: u32) -> Self {
        Self::new((deterministic_hash(run_seed) as u32).wrapping_add(slot_index))
    }

    #[must_use]
    pub const fn seed(&self) -> u32 {
        self.seed
    }

    #[must_use]
    pub fn stream(&self, stream: PlayerStream) -> &Rng {
        &self.streams[&stream]
    }

    pub fn stream_mut(&mut self, stream: PlayerStream) -> &mut Rng {
        self.streams
            .get_mut(&stream)
            .expect("all player streams exist")
    }

    #[must_use]
    pub fn to_serializable(&self) -> SerializablePlayerRngSet {
        SerializablePlayerRngSet {
            seed: self.seed,
            counters: self
                .streams
                .iter()
                .map(|(kind, rng)| (*kind, rng.counter()))
                .collect(),
        }
    }

    #[must_use]
    pub fn from_serializable(state: &SerializablePlayerRngSet) -> Self {
        let mut result = Self::new(state.seed);
        for (&kind, &counter) in &state.counters {
            result.stream_mut(kind).load_counter(counter);
        }
        result
    }

    pub fn load(&mut self, state: &SerializablePlayerRngSet) -> Result<(), RngError> {
        if self.seed != state.seed {
            return Err(RngError::SeedMismatch);
        }
        for (&kind, &counter) in &state.counters {
            self.stream_mut(kind).load_counter(counter);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_canonicalization_and_utf16_hashing_are_pinned() {
        assert_eq!(canonicalize_seed("  n3z9ee2cob\n"), "N3Z9EE2C0B");
        assert_eq!(deterministic_hash("N3Z9EE2CXB") as u32, 420_526_101);
        assert_eq!(deterministic_hash("A😀B"), -314_900_378);
    }

    #[test]
    fn splitmix_and_xoshiro_vectors_are_pinned() {
        let mut seed = 0;
        assert_eq!(splitmix64(&mut seed), 0xe220_a839_7b1d_cdaf);
        let mut random = MegaRandom::new(0);
        assert_eq!(random.state()[0], 0xe220_a839_7b1d_cdaf);
        assert_eq!(random.next_u64(), 0x99ec_5f36_cb75_f2b4);
        assert_eq!(random.next_u64(), 0xbf6e_1f78_4956_452a);
        assert_eq!(random.next_u64(), 0x1a5f_849d_4933_e6e0);
    }

    #[test]
    fn raw_primitive_vectors_are_pinned() {
        let mut random = MegaRandom::new(0);
        let mut bytes = [0_u8; 13];
        random.fill_bytes(&mut bytes);
        assert_eq!(
            bytes,
            [
                0xb4, 0xf2, 0x75, 0xcb, 0x36, 0x5f, 0xec, 0x99, 0x2a, 0x45, 0x56, 0x49, 0x78
            ]
        );

        assert_eq!(
            MegaRandom::new(0).next_double().to_bits(),
            0x3fe3_3d8b_e6d9_6ebe
        );
        assert_eq!(MegaRandom::new(0).next_float().to_bits(), 0x3f19_ec5f);
        assert_eq!(MegaRandom::new(0).next_u32(), 0xcb75_f2b4);
        assert_eq!(MegaRandom::new(0).next_i32(), 1_291_202_459);
        assert!(MegaRandom::new(0).next_bool());
    }

    #[test]
    fn primitive_operations_and_boundaries_match_golden_values() {
        let mut bool_rng = Rng::new(123);
        assert!(bool_rng.next_bool());
        let mut int_rng = Rng::new(123);
        assert_eq!(int_rng.next_int(10), Ok(1));
        let mut range_rng = Rng::new(123);
        assert_eq!(
            range_rng.next_int_range(i32::MIN, i32::MAX),
            Ok(-1_302_687_839)
        );
        let mut unsigned_rng = Rng::new(123);
        assert_eq!(
            unsigned_rng.next_unsigned_int(17, u32::MAX),
            Ok(844_795_823)
        );
        let mut float_rng = Rng::new(123);
        assert_eq!(
            float_rng.next_float(-2.5, 9.25).map(f32::to_bits),
            Ok(3_191_955_361)
        );
        let mut double_rng = Rng::new(123);
        assert_eq!(
            double_rng.next_double(-2.5, 9.25).map(f64::to_bits),
            Ok(13_819_343_784_251_519_392)
        );
        assert_eq!(bool_rng.counter(), 1);

        let mut invalid = Rng::new(9);
        assert_eq!(invalid.next_int(0), Err(RngError::EmptyRange));
        assert_eq!(invalid.next_float(2.0, 1.0), Err(RngError::EmptyRange));
        assert_eq!(invalid.counter(), 0);

        let mut equal_bounds = Rng::new(9);
        assert_eq!(equal_bounds.next_float(2.0, 2.0), Ok(2.0));
        assert_eq!(equal_bounds.next_double(-3.0, -3.0), Ok(-3.0));
        assert_eq!(equal_bounds.counter(), 2);

        let mut empty = Rng::new(9);
        assert_eq!(empty.next_index(0), Ok(None));
        assert_eq!(empty.next_item::<i32>(&[]), Ok(None));
        assert_eq!(empty.counter(), 0);

        let mut item = Rng::new(123);
        assert_eq!(item.next_item(&["a", "b", "c"]), Ok(Some(&"a")));
        assert_eq!(item.counter(), 1);
    }

    #[test]
    fn named_player_and_content_streams_match_fixture_seeds() {
        let run = RunRngSet::new("N3Z9EE2CXB");
        assert_eq!(run.seed(), 420_526_101);
        assert_eq!(run.stream(RunStream::UpFront).seed(), 3_480_746_352);
        assert_eq!(run.stream(RunStream::CombatOrbs).seed(), 1_668_475_230);

        let player = PlayerRngSet::for_player("N3Z9EE2CXB", 0);
        assert_eq!(player.seed(), 420_526_101);
        assert_eq!(player.stream(PlayerStream::Rewards).seed(), 4_256_445_359);
        assert_eq!(player.stream(PlayerStream::Shops).seed(), 2_840_321_520);

        let act_selection = Rng::for_seed("N3Z9EE2CXB", SeedStream::ActSelection);
        assert_eq!(act_selection.seed(), 3_953_569_258);
        assert_eq!(
            Rng::for_seed("X7JFABM2QS", SeedStream::ActSelection).seed(),
            2_880_999_506
        );

        let local = Rng::content_local(run.seed(), 0, "BASH", 7);
        assert_eq!(local.seed(), 3_612_350_988);
        let monster = Rng::new(run.seed().wrapping_add(3).wrapping_add(12));
        assert_eq!(monster.seed(), 420_526_116);
    }

    #[test]
    fn state_round_trip_resumes_at_exactly_the_next_value() {
        let mut original = Rng::new(0xdead_beef);
        for _ in 0..37 {
            original.next_double(0.0, 1.0).unwrap();
        }
        let encoded = serde_json::to_string(&original.to_state()).unwrap();
        let state: RngState = serde_json::from_str(&encoded).unwrap();
        let mut resumed = Rng::from_state(state);
        assert_eq!(original.next_int(i32::MAX), resumed.next_int(i32::MAX));
        assert_eq!(original.counter(), 38);
        assert_eq!(resumed.counter(), 38);

        let before = resumed.clone();
        assert_eq!(
            resumed.fast_forward(2),
            Err(RngError::CounterRewind {
                current: 38,
                target: 2
            })
        );
        assert_eq!(resumed, before);
    }

    #[test]
    fn stream_set_round_trip_and_backward_load_are_exact() {
        let mut run = RunRngSet::new("N3Z9EE2CXB");
        run.stream_mut(RunStream::UpFront)
            .fast_forward(414)
            .unwrap();
        run.stream_mut(RunStream::Shuffle).fast_forward(19).unwrap();
        let serialized = run.to_serializable();
        let json = serde_json::to_string(&serialized).unwrap();
        let serialized: SerializableRunRngSet = serde_json::from_str(&json).unwrap();
        let mut restored = RunRngSet::from_serializable(&serialized);
        assert_eq!(
            run.stream_mut(RunStream::Shuffle).next_int(100),
            restored.stream_mut(RunStream::Shuffle).next_int(100)
        );

        let mut earlier = serialized;
        earlier.counters.insert(RunStream::Shuffle, 2);
        restored.load(&earlier).unwrap();
        assert_eq!(restored.stream(RunStream::Shuffle).counter(), 2);

        let mut player = PlayerRngSet::for_player("N3Z9EE2CXB", 0);
        player
            .stream_mut(PlayerStream::Rewards)
            .fast_forward(24)
            .unwrap();
        let json = serde_json::to_string(&player.to_serializable()).unwrap();
        let state: SerializablePlayerRngSet = serde_json::from_str(&json).unwrap();
        let mut player_restored = PlayerRngSet::from_serializable(&state);
        assert_eq!(
            player.stream_mut(PlayerStream::Rewards).next_int(100),
            player_restored
                .stream_mut(PlayerStream::Rewards)
                .next_int(100)
        );
    }

    #[test]
    fn shuffle_result_and_counter_are_pinned() {
        let mut rng = Rng::new(42);
        let mut values = [0, 1, 2, 3, 4, 5, 6, 7];
        rng.shuffle(&mut values);
        assert_eq!(values, [7, 1, 6, 3, 5, 4, 2, 0]);
        assert_eq!(rng.counter(), 7);
    }

    #[test]
    fn gaussian_rejection_result_and_counter_are_pinned() {
        let mut rng = Rng::new(6);
        let value = rng.next_gaussian_double(0.5, 1.0, -4.0, 4.0).unwrap();
        assert_eq!(value.to_bits(), 4_606_286_160_458_955_376);
        assert_eq!(rng.counter(), 8, "three candidates must be rejected");

        let mut integer = Rng::new(18);
        assert_eq!(integer.next_gaussian_int(4, 3, 3, 5), 4);
        assert_eq!(integer.counter(), 4);
    }

    /// The Box-Muller transform is the one place in the kernel that calls a
    /// function IEEE 754 does not require to be correctly rounded: `ln`, `sin`
    /// and `cos` resolve to the platform's own library and are allowed to
    /// differ in the last place. `next_gaussian_int` decides an act's rest and
    /// unknown room counts, so a value that rounded differently would consume
    /// a different number of draws and take the whole run with it.
    ///
    /// These are that risk's alarm. The results are pinned as bit patterns
    /// rather than as approximate numbers, and the rejection loop's draw count
    /// is pinned beside each: a library that disagrees fails here, on the
    /// platform it disagrees on, instead of desynchronizing a run somewhere in
    /// act one.
    #[test]
    fn gaussian_vectors_are_pinned_across_platforms() {
        const VECTORS: [(u32, u64, u64, i32, u64, u32, u64); 5] = [
            (1, 13_835_631_089_280_702_353, 8, 12, 2, 3_222_292_831, 8),
            (7, 13_835_580_905_022_827_277, 2, 10, 2, 3_222_199_356, 2),
            (42, 13_836_017_557_355_122_814, 2, 12, 2, 3_223_012_684, 2),
            (123, 4_607_812_752_334_704_944, 12, 12, 2, 1_066_527_304, 12),
            (999, 13_836_675_647_037_667_714, 2, 12, 2, 3_224_238_471, 2),
        ];
        for (seed, double_bits, double_draws, integer, integer_draws, float_bits, float_draws) in
            VECTORS
        {
            let mut rng = Rng::new(seed);
            let value = rng.next_gaussian_double(0.5, 1.0, -4.0, 4.0).unwrap();
            assert_eq!(value.to_bits(), double_bits, "double for seed {seed}");
            assert_eq!(rng.counter(), double_draws, "double draws for seed {seed}");

            // The mean, deviation and bounds an act's unknown room count uses.
            let mut counted = Rng::new(seed);
            assert_eq!(
                counted.next_gaussian_int(12, 1, 10, 14),
                integer,
                "integer for seed {seed}"
            );
            assert_eq!(counted.counter(), integer_draws, "integer draws for {seed}");

            let mut single = Rng::new(seed);
            let narrowed = single.next_gaussian_float(0.5, 1.0, -4.0, 4.0).unwrap();
            assert_eq!(narrowed.to_bits(), float_bits, "float for seed {seed}");
            assert_eq!(single.counter(), float_draws, "float draws for {seed}");
        }
    }

    #[test]
    fn weighted_selection_uses_f32_and_consumes_once() {
        assert_eq!(Rng::weighted_index_from(0.0, &[0.0, 2.0]), Some(0));
        let mut rng = Rng::new(77);
        assert_eq!(rng.weighted_index(&[1.0, 3.0, 2.0]), Some(1));
        assert_eq!(rng.counter(), 1);
    }

    #[test]
    fn repeated_transcripts_are_byte_identical() {
        fn transcript() -> Vec<u8> {
            let mut streams = RunRngSet::new("N3Z9EE2CXB");
            let values: Vec<_> = RunStream::ALL
                .into_iter()
                .map(|stream| streams.stream_mut(stream).next_int(i32::MAX).unwrap())
                .collect();
            serde_json::to_vec(&(values, streams.to_serializable())).unwrap()
        }

        let first = transcript();
        assert_eq!(first, transcript());
        assert_eq!(first, transcript());
    }
}
