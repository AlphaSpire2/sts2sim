//! Patch identity and stable identifiers shared by simulator layers.

use std::cmp::Ordering;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::str::FromStr;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

pub const PINNED_COMPATIBILITY_ID: &str = "sts2-v0.107.1-59260271-models-3954186980";
pub const PINNED_GAME_VERSION: &str = "v0.107.1";
pub const PINNED_GAME_COMMIT: &str = "59260271";
pub const PINNED_MODEL_ID_HASH: u32 = 3_954_186_980;
/// The one unlock preset the pinned baseline generates runs against.
pub const FIXTURE_UNLOCK_PRESET: &str = "v0107_1_veteran";

/// A stable content identifier such as `CARD.BASH`.
///
/// An id is named once and then copied everywhere the thing it names is
/// held: onto every card in a deck, every power on a creature, every listener
/// a hook pass walks. So the text is shared rather than copied, and its hash
/// is taken as it is made, which makes a copy a refcount and a lookup by id a
/// word compare before it is ever a string compare.
#[derive(Clone)]
pub struct ModelId {
    /// `CATEGORY.ENTRY`, shared by every copy of the id.
    text: Arc<str>,
    /// Where the dot stands in `text`.
    split: usize,
    /// A hash of `text`, so two ids compare and hash without reading it.
    hash: u64,
}

impl ModelId {
    pub fn new(
        category: impl Into<String>,
        entry: impl Into<String>,
    ) -> Result<Self, ModelIdError> {
        let category = category.into();
        let entry = entry.into();
        if category.ends_with("_MODEL") {
            return Err(ModelIdError::BannedCategorySuffix);
        }
        if category.is_empty() || entry.is_empty() || category.contains('.') || entry.contains('.')
        {
            return Err(ModelIdError::InvalidShape);
        }
        let text = format!("{category}.{entry}");
        Ok(Self {
            hash: fnv1a(text.as_bytes()),
            split: category.len(),
            text: text.into(),
        })
    }

    #[must_use]
    pub fn category(&self) -> &str {
        &self.text[..self.split]
    }

    #[must_use]
    pub fn entry(&self) -> &str {
        &self.text[self.split + 1..]
    }
}

/// FNV-1a over the id's text: a hash that is a function of the text alone,
/// so it can be taken once and carried with the id.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

impl PartialEq for ModelId {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash && (Arc::ptr_eq(&self.text, &other.text) || self.text == other.text)
    }
}

impl Eq for ModelId {}

impl Hash for ModelId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash);
    }
}

/// Ordered by category and then by entry, which is the order of the pair of
/// strings the id is written as.
impl Ord for ModelId {
    fn cmp(&self, other: &Self) -> Ordering {
        if Arc::ptr_eq(&self.text, &other.text) {
            return Ordering::Equal;
        }
        self.category()
            .cmp(other.category())
            .then_with(|| self.entry().cmp(other.entry()))
    }
}

impl PartialOrd for ModelId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Debug for ModelId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "ModelId({})", self.text)
    }
}

impl Display for ModelId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.text)
    }
}

impl FromStr for ModelId {
    type Err = ModelIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut pieces = value.split('.');
        let category = pieces.next().ok_or(ModelIdError::InvalidShape)?;
        let entry = pieces.next().ok_or(ModelIdError::InvalidShape)?;
        if pieces.next().is_some() {
            return Err(ModelIdError::InvalidShape);
        }
        Self::new(category, entry)
    }
}

impl Serialize for ModelId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for ModelId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelIdError {
    InvalidShape,
    BannedCategorySuffix,
}

impl Display for ModelIdError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidShape => formatter.write_str("model ID must be CATEGORY.ENTRY"),
            Self::BannedCategorySuffix => {
                formatter.write_str("model category cannot end in _MODEL")
            }
        }
    }
}

impl Error for ModelIdError {}

macro_rules! stable_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(u64);

        impl $name {
            #[must_use]
            pub const fn get(self) -> u64 {
                self.0
            }
        }

        impl From<u64> for $name {
            fn from(value: u64) -> Self {
                Self(value)
            }
        }
    };
}

stable_id!(/// Stable identity of one card instance.
    CardInstanceId);
stable_id!(/// Stable identity of one creature during a combat.
    CreatureCombatId);
stable_id!(/// Stable identity of one orb standing in a player's orb queue.
    ///
    /// The build gives an orb no id of its own: an orb is one thing that can
    /// change while it stands in the queue, and everywhere one is named — a
    /// queue taking an orb out, a listener checking the orb it was watching is
    /// still standing, a relic asking whether this is the orb at the front —
    /// that one orb is meant, not any orb like it. Two Lightning orbs are two
    /// orbs, so they need two identities.
    OrbInstanceId);
stable_id!(/// Stable identity of a suspended player choice.
    ChoiceId);
stable_id!(/// Stable identity of an offered reward set.
    RewardSetId);
stable_id!(/// Stable identity of a room instance.
    RoomInstanceId);

/// A deterministic monotonic allocator whose next value is part of state.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StableIdAllocator<I> {
    next: u64,
    #[serde(skip)]
    marker: PhantomData<fn() -> I>,
}

impl<I> Default for StableIdAllocator<I> {
    fn default() -> Self {
        Self {
            next: 1,
            marker: PhantomData,
        }
    }
}

impl<I: From<u64>> StableIdAllocator<I> {
    #[must_use]
    pub const fn next_raw(&self) -> u64 {
        self.next
    }

    pub fn allocate(&mut self) -> Result<I, StableIdError> {
        if self.next == u64::MAX {
            return Err(StableIdError::Exhausted);
        }
        let allocated = self.next;
        self.next += 1;
        Ok(I::from(allocated))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StableIdError {
    Exhausted,
}

impl Display for StableIdError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("stable ID domain exhausted")
    }
}

impl Error for StableIdError {}

/// The checked-in patch compatibility manifest.
///
/// The pinned identity is the `PINNED_*` constants; the manifest restates it
/// for readers outside the build and is checked against them, so the two
/// cannot drift without a test failing. `compatibility_id` is derived from
/// the game block: `sts2-{version}-{commit}-models-{model_id_hash}`.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CompatibilityManifest {
    pub schema_version: u32,
    pub compatibility_id: String,
    pub game: GameBuild,
    pub support_policy: SupportPolicy,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameBuild {
    pub version: String,
    pub commit: String,
    pub model_id_hash: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct ModIdentity {
    pub id: String,
    pub version: String,
    pub affects_gameplay: bool,
}

/// What the compatibility gate refuses beyond build identity. Every field
/// here is read by the gate; policy the code does not enforce belongs in the
/// fidelity contract, not in the manifest.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SupportPolicy {
    pub player_counts: Vec<u32>,
    pub game_modes: Vec<String>,
    /// Whether a recording may have loaded a mod that declares itself
    /// gameplay-affecting. Mod ids and versions are never compared: any mod
    /// may be loaded, and only its own declaration decides.
    pub gameplay_affecting_mods: bool,
}

impl CompatibilityManifest {
    /// Parses the manifest embedded at build time; no game files are accessed.
    pub fn pinned() -> Result<Self, ManifestError> {
        let manifest: Self =
            serde_json::from_str(include_str!("../../../manifests/compatibility.json"))
                .map_err(ManifestError::Json)?;
        manifest.validate_pinned()?;
        Ok(manifest)
    }

    pub fn validate_pinned(&self) -> Result<(), ManifestError> {
        let valid = self.schema_version == 1
            && self.compatibility_id == PINNED_COMPATIBILITY_ID
            && self.game.version == PINNED_GAME_VERSION
            && self.game.commit == PINNED_GAME_COMMIT
            && self.game.model_id_hash == PINNED_MODEL_ID_HASH
            && self.compatibility_id
                == format!(
                    "sts2-{}-{}-models-{}",
                    self.game.version, self.game.commit, self.game.model_id_hash
                );
        if valid {
            Ok(())
        } else {
            Err(ManifestError::IdentityMismatch)
        }
    }
}

/// The normalized unlock inputs a run is generated against.
///
/// These are part of the seed-to-run contract, not cosmetic profile data,
/// but each field's reach is exactly what the pinned build reads of it:
///
/// - Revealed epochs decide what content is eligible — card, potion, and
///   relic pools filter through them, alternate acts require theirs, and the
///   Overgrowth and Underdocks acts offer the Neow ancient only to a profile
///   that has revealed `NEOW_EPOCH`.
/// - The seen-encounter list has one generation read: an act's rolled boss is
///   replaced by the first boss of that act's discovery order the profile has
///   never seen. Only the seen-status of those listed bosses
///   ([`BOSS_DISCOVERY_ENCOUNTERS`]) can change a run; every other entry is
///   projection data.
/// - The prior-run count matters only as first-run-or-not: every place the
///   pinned build's generation reads it asks whether this is the profile's
///   first run and nothing else — the odds an unknown map point rolls
///   against, the Overgrowth act, the reward sets a room offers, Lasting
///   Candy, and the relic a treasure room holds — so two profiles past their
///   first run generate identically whatever their counts.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UnlockPresetManifest {
    pub schema_version: u32,
    pub preset_id: String,
    pub compatibility_id: String,
    pub unlocks: UnlockState,
    pub profile: ProfileInputs,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UnlockState {
    pub epochs: Vec<String>,
    pub encounters_seen: Vec<String>,
    pub number_of_runs: u32,
}

/// Every encounter the pinned build's boss discovery reads: the discovery
/// orders of the four acts, in act order. This is the complete generation
/// surface of the seen-encounter list — replacing an act's rolled boss with
/// the first of these it has never shown is its only gameplay read, and that
/// override spends no dice — so two profiles agreeing on the seen-status of
/// these twelve generate identical runs whatever else their encounter
/// histories hold.
pub const BOSS_DISCOVERY_ENCOUNTERS: [&str; 12] = [
    "ENCOUNTER.VANTOM_BOSS",
    "ENCOUNTER.CEREMONIAL_BEAST_BOSS",
    "ENCOUNTER.THE_KIN_BOSS",
    "ENCOUNTER.WATERFALL_GIANT_BOSS",
    "ENCOUNTER.SOUL_FYSH_BOSS",
    "ENCOUNTER.LAGAVULIN_MATRIARCH_BOSS",
    "ENCOUNTER.THE_INSATIABLE_BOSS",
    "ENCOUNTER.KNOWLEDGE_DEMON_BOSS",
    "ENCOUNTER.KAISER_CRAB_BOSS",
    "ENCOUNTER.QUEEN_BOSS",
    "ENCOUNTER.TEST_SUBJECT_BOSS",
    "ENCOUNTER.AEONGLASS_BOSS",
];

/// Profile facts outside the `unlocks` projection. New recordings carry these
/// in `run.start.data.profile`: discovered acts decide whether an unlocked
/// alternate act is taken outright or rolled for.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProfileInputs {
    pub discovered_acts: Vec<ModelId>,
}

/// Every unlock preset registered for this build.
///
/// One: the veteran environment — a profile past its first run with every
/// patch epoch revealed and every boss-discovery encounter seen. All such
/// profiles generate identically, so one preset carries the whole
/// equivalence class, and its `unlocks` state only what generation reads:
/// the epochs exactly, the twelve boss-discovery encounters, and a
/// prior-run count that is merely nonzero. One entry stands for one
/// generation environment.
const REGISTERED_UNLOCK_PRESETS: [&str; 1] = [include_str!(
    "../../../manifests/unlock-presets/v0107_1_veteran.json"
)];

impl UnlockPresetManifest {
    /// Parses the pinned fixture preset embedded at build time.
    pub fn pinned() -> Result<Self, ManifestError> {
        Self::with_id(FIXTURE_UNLOCK_PRESET)
    }

    /// Every registered preset, parsed and identity-checked.
    pub fn registered() -> Result<Vec<Self>, ManifestError> {
        REGISTERED_UNLOCK_PRESETS
            .iter()
            .map(|source| {
                let manifest: Self = serde_json::from_str(source).map_err(ManifestError::Json)?;
                if manifest.schema_version != 1
                    || manifest.compatibility_id != PINNED_COMPATIBILITY_ID
                {
                    return Err(ManifestError::IdentityMismatch);
                }
                Ok(manifest)
            })
            .collect()
    }

    /// The registered preset with this id.
    pub fn with_id(preset_id: &str) -> Result<Self, ManifestError> {
        Self::registered()?
            .into_iter()
            .find(|manifest| manifest.preset_id == preset_id)
            .ok_or(ManifestError::IdentityMismatch)
    }

    /// The registered preset whose generation inputs are this state's.
    ///
    /// This optional registry lookup considers only unlock state; replay
    /// constructs its explicit manifest directly. Two states are one unlock
    /// environment when the pinned build's
    /// reads cannot tell them apart: the revealed epochs must match exactly,
    /// the prior-run count only as first-run-or-not, and the seen-encounter
    /// lists only on the seen-status of [`BOSS_DISCOVERY_ENCOUNTERS`] — the
    /// boss-discovery override is that list's single gameplay read, so every
    /// other entry is projection data. The preset answers carrying the asked
    /// state's own count and encounter list, because a run generated on the
    /// answer projects what the profile recorded, not what the preset's own
    /// recording happened to carry. The answer also supplies
    /// `profile.discovered_acts`, which this lookup does not compare. A state
    /// whose unlock inputs no preset carries returns `None`; it is not
    /// approximated to the nearest registered profile.
    pub fn matching(unlocks: &UnlockState) -> Result<Option<Self>, ManifestError> {
        let seen = |state: &UnlockState, boss: &str| {
            state.encounters_seen.iter().any(|entry| entry == boss)
        };
        Ok(Self::registered()?
            .into_iter()
            .find(|manifest| {
                manifest.unlocks.epochs == unlocks.epochs
                    && (manifest.unlocks.number_of_runs == 0) == (unlocks.number_of_runs == 0)
                    && BOSS_DISCOVERY_ENCOUNTERS
                        .iter()
                        .all(|boss| seen(&manifest.unlocks, boss) == seen(unlocks, boss))
            })
            .map(|mut manifest| {
                manifest.unlocks.number_of_runs = unlocks.number_of_runs;
                manifest
                    .unlocks
                    .encounters_seen
                    .clone_from(&unlocks.encounters_seen);
                manifest
            }))
    }
}

#[derive(Debug)]
pub enum ManifestError {
    Json(serde_json::Error),
    IdentityMismatch,
}

impl Display for ManifestError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(formatter, "invalid compatibility manifest: {error}"),
            Self::IdentityMismatch => formatter.write_str("pinned compatibility identity mismatch"),
        }
    }
}

impl Error for ManifestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            Self::IdentityMismatch => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_ids_validate_and_round_trip() {
        let id: ModelId = "CARD.BASH".parse().unwrap();
        assert_eq!(id.category(), "CARD");
        assert_eq!(id.entry(), "BASH");
        assert_eq!(serde_json::to_string(&id).unwrap(), "\"CARD.BASH\"");
        assert!("CARD_MODEL.BASH".parse::<ModelId>().is_err());
        assert!("CARD.BASH.EXTRA".parse::<ModelId>().is_err());
    }

    #[test]
    fn allocators_are_domain_typed_and_serializable() {
        let mut cards = StableIdAllocator::<CardInstanceId>::default();
        assert_eq!(cards.allocate().unwrap().get(), 1);
        assert_eq!(cards.allocate().unwrap().get(), 2);
        let encoded = serde_json::to_string(&cards).unwrap();
        let mut restored: StableIdAllocator<CardInstanceId> =
            serde_json::from_str(&encoded).unwrap();
        assert_eq!(restored.allocate().unwrap().get(), 3);
    }

    #[test]
    fn embedded_manifest_is_the_pinned_build() {
        let manifest = CompatibilityManifest::pinned().unwrap();
        assert_eq!(manifest.compatibility_id, PINNED_COMPATIBILITY_ID);
        assert!(!manifest.support_policy.gameplay_affecting_mods);
    }

    #[test]
    fn preset_matching_relaxes_the_prior_run_count_past_a_first_run() {
        let registered = UnlockPresetManifest::with_id("v0107_1_veteran").unwrap();
        let mut unlocks = registered.unlocks.clone();
        unlocks.number_of_runs = 101;
        let named = UnlockPresetManifest::matching(&unlocks).unwrap().unwrap();
        // The one registered environment answers, carrying the asked count
        // and encounter list — projection data rather than generation inputs.
        assert_eq!(named.preset_id, "v0107_1_veteran");
        assert_eq!(named.unlocks.number_of_runs, 101);
        assert_eq!(named.unlocks.encounters_seen, unlocks.encounters_seen);
    }

    #[test]
    fn preset_matching_ignores_encounters_the_build_never_reads() {
        let registered = UnlockPresetManifest::with_id("v0107_1_veteran").unwrap();
        let mut seen_more = registered.unlocks.clone();
        seen_more
            .encounters_seen
            .push("ENCOUNTER.NOBODY_HAS_SEEN".to_owned());
        seen_more.number_of_runs = 92;
        let named = UnlockPresetManifest::matching(&seen_more).unwrap().unwrap();
        // The extra encounter is in no act's boss-discovery list, so the
        // profile generates identically and the answer projects it back.
        assert!(
            named
                .unlocks
                .encounters_seen
                .contains(&"ENCOUNTER.NOBODY_HAS_SEEN".to_owned())
        );
    }

    #[test]
    fn preset_matching_holds_the_generation_inputs_apart() {
        let registered = UnlockPresetManifest::with_id("v0107_1_veteran").unwrap();
        let mut fresh = registered.unlocks.clone();
        fresh.number_of_runs = 0;
        assert!(UnlockPresetManifest::matching(&fresh).unwrap().is_none());
        let mut unseen_boss = registered.unlocks.clone();
        unseen_boss
            .encounters_seen
            .retain(|entry| entry != "ENCOUNTER.CEREMONIAL_BEAST_BOSS");
        assert!(
            UnlockPresetManifest::matching(&unseen_boss)
                .unwrap()
                .is_none()
        );
        let mut fewer_epochs = registered.unlocks.clone();
        fewer_epochs.epochs.pop();
        assert!(
            UnlockPresetManifest::matching(&fewer_epochs)
                .unwrap()
                .is_none()
        );
    }
}
