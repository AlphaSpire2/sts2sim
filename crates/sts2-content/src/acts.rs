//! Which acts a run opens with, and which character a lobby that asked for a
//! random one ends up playing.
//!
//! Both come off one stream the run never keeps: the lobby builds
//! `act_selection` from the seed string, rolls one act per index and one
//! character per player who left the choice to it, and drops the stream before
//! the run is stood up.

use sts2_core::{ModelId, UnlockPresetManifest, UnlockState};
use sts2_engine::{ActOneSetting, EngineError, ErrorCode};
use sts2_rng::{Rng, SeedStream};

use crate::id;

/// One act the game can build a run out of.
struct ActModelEntry {
    id: &'static str,
    /// Which act of the run this one can be, counting from zero.
    index: usize,
    is_default: bool,
    /// The unlock epoch this act asks for, where it asks for one at all.
    required_epoch: Option<&'static str>,
}

/// The game's act list is ordered exactly this way and the roll depends on it:
/// by act index, and the default act of an index before its alternates. The
/// act roll indexes into a candidate list filtered out of this order, so the
/// order is part of the roll.
///
/// The one retired act is absent because the game no longer lists it, and it
/// would fall out anyway: only acts with a non-negative index are grouped into
/// an act slot, and its index is negative.
const ACT_MODELS: [ActModelEntry; 4] = [
    ActModelEntry {
        id: "ACT.OVERGROWTH",
        index: 0,
        is_default: true,
        required_epoch: None,
    },
    ActModelEntry {
        id: "ACT.UNDERDOCKS",
        index: 0,
        is_default: false,
        required_epoch: Some("UNDERDOCKS_EPOCH"),
    },
    ActModelEntry {
        id: "ACT.HIVE",
        index: 1,
        is_default: true,
        required_epoch: None,
    },
    ActModelEntry {
        id: "ACT.GLORY",
        index: 2,
        is_default: true,
        required_epoch: None,
    },
];

/// How many act slots the game groups its acts into, which is how many acts a
/// run has.
pub const ACT_INDEX_COUNT: usize = 3;

/// Every character, in the order the game lists them. A random character is an
/// index into this list, so the order is part of the roll, and the list is not
/// filtered by unlock state.
const ALL_CHARACTERS: [&str; 5] = [
    "CHARACTER.IRONCLAD",
    "CHARACTER.SILENT",
    "CHARACTER.REGENT",
    "CHARACTER.NECROBINDER",
    "CHARACTER.DEFECT",
];

/// The stand-in a lobby carries until the roll names a real character.
pub const RANDOM_CHARACTER: &str = "CHARACTER.RANDOM_CHARACTER";

/// What one pass over the act-selection stream settled.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActSelection {
    /// One act model per act index, in run order.
    pub acts: Vec<ModelId>,
    /// The character the run is played with: the configured one, or the one
    /// the stream rolled for a lobby that asked for a random character.
    pub character: ModelId,
    /// How far the stream was read. It is not a run counter — nothing keeps
    /// this stream — but it is what the rolls above cost.
    pub counter: u64,
}

/// Rolls the act list against the one unlock preset and profile this baseline
/// generates runs for.
pub fn pinned_act_selection(
    seed: &str,
    act_one: ActOneSetting,
    character: &ModelId,
) -> Result<ActSelection, EngineError> {
    let preset = UnlockPresetManifest::pinned().map_err(|error| {
        EngineError::new(ErrorCode::InternalInvariant)
            .with_detail("unlock_preset", error.to_string())
    })?;
    select_acts(
        seed,
        act_one,
        &preset.unlocks,
        &preset.profile.discovered_acts,
        character,
    )
}

fn is_unlocked(entry: &ActModelEntry, unlocks: &UnlockState) -> bool {
    entry
        .required_epoch
        .is_none_or(|epoch| unlocks.epochs.iter().any(|revealed| revealed == epoch))
}

/// Rolls the run's act list and resolves a random character, in the order the
/// lobby does both.
///
/// The rules are the build's. Every act index rolls one act out of its
/// unlocked candidates, including an index with a single candidate, which is
/// why a roll is spent on act two and act three all the same. An unlocked
/// alternate act the local profile has never seen is taken instead of rolling
/// for it, and taking it stops the scan, so that index reads no dice at all
/// and every later roll moves up. The lobby's act-one setting is applied
/// afterwards and reads no dice either: it replaces what index 0 settled on,
/// whatever that was. Then one draw per player who left the character to
/// chance.
///
/// Two conditions the build also tests are constants here: this simulator is
/// single-player, and test mode is off in a shipped build.
pub fn select_acts(
    seed: &str,
    act_one: ActOneSetting,
    unlocks: &UnlockState,
    discovered_acts: &[ModelId],
    character: &ModelId,
) -> Result<ActSelection, EngineError> {
    let mut rng = Rng::for_seed(seed, SeedStream::ActSelection);
    let mut acts = Vec::with_capacity(ACT_INDEX_COUNT);
    for index in 0..ACT_INDEX_COUNT {
        let mut undiscovered = None;
        let mut candidates = Vec::new();
        for entry in ACT_MODELS.iter().filter(|entry| entry.index == index) {
            if !is_unlocked(entry, unlocks) {
                continue;
            }
            if !entry.is_default
                && !discovered_acts
                    .iter()
                    .any(|act| act.to_string() == entry.id)
            {
                undiscovered = Some(entry.id);
                break;
            }
            candidates.push(entry.id);
        }
        let selected = match undiscovered {
            Some(entry) => entry,
            None => *rng
                .next_item(&candidates)
                .expect("candidate count fits a game integer")
                .ok_or_else(|| {
                    EngineError::new(ErrorCode::InternalInvariant)
                        .with_detail("act_index", index as u64)
                        .with_detail("reason", "no unlocked acts for this index")
                })?,
        };
        acts.push(id(selected));
    }

    if let Some(overridden) = act_one.overridden_act() {
        acts[0] = id(overridden);
    }

    let character = if character.to_string() == RANDOM_CHARACTER {
        id(rng
            .next_item(&ALL_CHARACTERS)
            .expect("character count fits a game integer")
            .expect("the build always has five characters"))
    } else {
        character.clone()
    };

    Ok(ActSelection {
        acts,
        character,
        counter: rng.counter(),
    })
}
