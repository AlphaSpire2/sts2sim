//! Sampling possible hidden worlds from what a player can see.
//!
//! This is the belief-state substrate a fair analysis samples worlds from: a
//! belief state is extracted from a decision point by *erasing* hidden
//! state, and each call to `sample` fills the erased holes
//! back in from an analysis seed, yielding a full simulator that agrees with
//! the original on everything a player could see and on nothing they could
//! not. A search that holds only the belief state cannot be clairvoyant: the
//! type is the boundary, because the hidden state is gone before the search
//! sees it.
//!
//! The models are versioned and labeled analysis, not a fidelity claim. Every
//! sampled simulator carries [`ConformanceOrigin::Analysis`], so a sampled
//! world can never be passed off as conformance evidence.
//!
//! Two models live here, one per horizon:
//!
//! - [`BeliefState`], version `combat-v0`: the erasure covers hidden *combat*
//!   state — the draw pile's order and every RNG stream position. Run-level
//!   hidden state (the unrevealed map, undealt rewards, unentered events) is
//!   carried as it stands, so a search over a sample is fair only while the
//!   fight lasts: a consumer must treat the end of the combat as the sample's
//!   horizon and not walk past it.
//! - [`ActBeliefState`], version `act-v0.5`: the erasure additionally covers
//!   the current act's hidden remainder — the encounter and event pools past
//!   their visited cursors, room definitions placed at unwalked map points,
//!   the relic grab bags — so a search over a sample is fair until the act
//!   transition, which is this model's horizon. Later acts' maps and pools
//!   are carried as they stand, fenced by that horizon exactly as `combat-v0`
//!   fences the run.
//!
//! # Honesty, `act-v0.5`
//!
//! An `act-v0.5` sample is a uniform *permutation* of the true world's hidden
//! remainder, not a draw from the full generative posterior. Across many
//! samples the multiset of upcoming content is exactly the true one — which a
//! player cannot know — and a permuted pool ignores the generation shape the
//! real game dealt it under (an act's weak-encounter opening, the rule that
//! keeps same-tagged content from being dealt back to back), so a sample can
//! put up a world the real game could never have dealt. For the relic grab
//! bags the permutation happens to be exact — a bag is a whole pool shuffled
//! once, so its remainder really is uniform given what the player has seen —
//! but for everything else this is the model's accepted crudeness. The model
//! does not re-run structured generation consistent with visited evidence;
//! it permutes what is left.

use std::collections::BTreeSet;
use std::sync::Arc;

use sts2_rng::{MegaRandom, PlayerRngSet, RunRngSet, splitmix64};

use crate::model::{CombatState, ConformanceOrigin, GameState};
use crate::run::{MapCoord, MapPointType, RelicRarity, RoomDefinition, RunState};
use crate::simulator::{ContentRegistry, Simulator};
use crate::{EngineError, ErrorCode, ObservationKey};

/// The version of the combat belief model: what [`BeliefState`] erases and
/// how [`BeliefState::sample`] fills the holes. Any change to either moves
/// this name, because a script searched under one model is not reproducible
/// under another.
pub const BELIEF_MODEL_VERSION: &str = "combat-v0";

/// The version of the act belief model: what [`ActBeliefState`] erases and
/// how [`ActBeliefState::sample`] fills the holes, under the same rule.
pub const ACT_BELIEF_MODEL_VERSION: &str = "act-v0.5";

/// A combat decision point with its hidden state erased: what a fair analyst
/// is allowed to stand on.
///
/// Constructed by [`BeliefState::from_simulator`], which sorts the draw pile
/// into the canonical order the observation shows and discards every RNG
/// stream position. What remains is a function of player-visible information,
/// and [`BeliefState::sample`] turns it back into full simulators by sampling
/// the erased parts from an analysis seed.
#[derive(Clone, Debug)]
pub struct BeliefState {
    state: GameState,
    content: Arc<ContentRegistry>,
}

impl BeliefState {
    /// Erases the hidden state of a combat decision point.
    ///
    /// Combat-scoped: a simulator standing outside a fight is refused,
    /// because `combat-v0` erases nothing at the run level and a sample of it
    /// would be clairvoyant about the map and the streams ahead.
    pub fn from_simulator(simulator: &Simulator) -> Result<Self, EngineError> {
        let mut state = simulator.state().clone();
        let Some(combat) = state.combat.as_mut() else {
            return Err(EngineError::new(ErrorCode::InvalidConfig)
                .with_detail("belief", "combat-v0 samples combats only"));
        };
        sort_hidden_draw_order(combat);
        // A stream's position is hidden state. The erased form holds streams
        // at a constant nothing; every sample replaces them wholesale.
        state.rng = RunRngSet::new("BELIEF");
        state.player_rng = PlayerRngSet::new(0);
        state.origin = ConformanceOrigin::Analysis;
        Ok(Self {
            state,
            content: simulator.content_handle(),
        })
    }

    /// The canonical key over what this belief state was built from. Two
    /// decision points a player cannot tell apart erase to belief states
    /// whose samples share this key.
    pub fn observation_key(&self) -> Result<ObservationKey, EngineError> {
        self.sample_with(0, 0)?.observation_key()
    }

    /// One possible hidden world: the erased holes filled back in from
    /// `(analysis_seed, rollout)`, deterministically — the same pair always
    /// yields the same world, and different pairs differ only in what a
    /// player could not see.
    pub fn sample(&self, analysis_seed: u64, rollout: u64) -> Result<Simulator, EngineError> {
        self.sample_with(analysis_seed, rollout)
    }

    fn sample_with(&self, analysis_seed: u64, rollout: u64) -> Result<Simulator, EngineError> {
        let mut state = self.state.clone();
        // One SplitMix64 walk from the pair, spent on the shuffle and the
        // stream seeds. The golden-ratio fold keeps rollout 0 and seed 0 from
        // collapsing onto each other.
        let mut mix = analysis_seed ^ rollout.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let mut analysis = MegaRandom::new(splitmix64(&mut mix));
        let combat = state
            .combat
            .as_mut()
            .expect("a belief state holds a combat");
        shuffle(&mut combat.player.piles.draw, &mut analysis);
        // Fresh streams from the analysis walk, never the real run seed: the
        // sampled world's future randomness is analysis randomness.
        state.rng = RunRngSet::new(format!("ANALYSIS{:016X}", splitmix64(&mut mix)));
        state.player_rng = PlayerRngSet::new(splitmix64(&mut mix));
        Simulator::from_scenario(state, Arc::clone(&self.content))
    }
}

/// A run decision point with its hidden state erased to the act's horizon:
/// what a fair analyst planning across the current act is allowed to stand
/// on.
///
/// Constructed by [`ActBeliefState::from_simulator`], which erases everything
/// `combat-v0` erases and, on top of it, the current act's hidden remainder:
///
/// - the act's encounter and event pools past their visited cursors — the
///   walked prefix is player evidence and stays; the order ahead is hidden;
/// - room definitions a fixture placed at map points not yet walked onto,
///   which are put beyond reach among the unwalked points of their map type
///   (their pre-rolled rewards ride with them); a standard generated run
///   places none, and rolls each room on the streams as it is entered;
/// - both relic grab bags, whose remaining contents are derivable from the
///   pools and the relics already seen but whose order is not;
/// - every RNG stream position, wholesale — which is also what hides the
///   resolutions of `?` rooms ahead, the rewards and shop stocks not yet
///   rolled, and every event page not yet built, because all of them are
///   rolled from the streams (or from RNGs derived from the stream seed) at
///   the moment they are walked into.
///
/// What stays, because the player can see it or derive it: the current act's
/// map layout (shown on act entry), the act's boss and ancient (previewed on
/// the map screen), the visited pool prefixes, any standing screen — an open
/// event page, shop shelf, rest site, treasure chest, or reward set is
/// pinned, never resampled — the unknown-room pity odds
/// (`unknown_room_odds`, derivable from walked history), and the whole
/// visible history. Later acts' maps, pools, and placed rooms are carried as
/// they stand: they are beyond this model's horizon — the engine's own
/// [`DecisionContext::ActTransition`](crate::DecisionContext::ActTransition)
/// — and a consumer must not walk a sample past it, exactly as `combat-v0`
/// consumers must not walk past the fight. A finished act's unwalked pool
/// remainder is likewise carried: it is dead state nothing ahead reads.
///
/// Mid-fight, `combat-v0` composes in unchanged: the fight entry inside an
/// act sample is the very world `combat-v0` would deal for the same
/// `(analysis_seed, rollout)` — same draw-pile deal, same stream seeds —
/// because the sampler spends its analysis randomness in the same order
/// before touching anything run-level.
#[derive(Clone, Debug)]
pub struct ActBeliefState {
    state: GameState,
    content: Arc<ContentRegistry>,
}

impl ActBeliefState {
    /// Erases the hidden state of a run decision point, to the act horizon.
    ///
    /// Run-scoped: a simulator standing outside a run — a bare scenario
    /// fight — is refused, because there is no act to bound the sample and
    /// `combat-v0` already covers the fight alone.
    pub fn from_simulator(simulator: &Simulator) -> Result<Self, EngineError> {
        let mut state = simulator.state().clone();
        if state.run.is_none() {
            return Err(EngineError::new(ErrorCode::InvalidConfig)
                .with_detail("belief", "act-v0.5 samples runs only"));
        }
        if let Some(combat) = state.combat.as_mut() {
            sort_hidden_draw_order(combat);
        }
        rearrange_act_remainder(
            state.run.as_mut().expect("just found the run"),
            &mut Rearrange::Canonical,
        );
        state.rng = RunRngSet::new("BELIEF");
        state.player_rng = PlayerRngSet::new(0);
        state.origin = ConformanceOrigin::Analysis;
        Ok(Self {
            state,
            content: simulator.content_handle(),
        })
    }

    /// The canonical key over what this belief state was built from. Two
    /// decision points a player cannot tell apart erase to belief states
    /// whose samples share this key.
    pub fn observation_key(&self) -> Result<ObservationKey, EngineError> {
        self.sample_with(0, 0)?.observation_key()
    }

    /// One possible hidden world: each hidden remainder dealt a fresh uniform
    /// permutation from `(analysis_seed, rollout)`, deterministically — the
    /// same pair always yields the same world, and different pairs differ
    /// only in what a player could not see.
    pub fn sample(&self, analysis_seed: u64, rollout: u64) -> Result<Simulator, EngineError> {
        self.sample_with(analysis_seed, rollout)
    }

    fn sample_with(&self, analysis_seed: u64, rollout: u64) -> Result<Simulator, EngineError> {
        let mut state = self.state.clone();
        // The same SplitMix64 walk as `combat-v0`, spent in the same order —
        // draw pile first, stream seeds last — so a mid-fight sample's combat
        // is bit-for-bit the one `combat-v0` deals for the pair. The
        // run-level permutations spend only the analysis generator between
        // those two, which the walk never reads back.
        let mut mix = analysis_seed ^ rollout.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let mut analysis = MegaRandom::new(splitmix64(&mut mix));
        if let Some(combat) = state.combat.as_mut() {
            shuffle(&mut combat.player.piles.draw, &mut analysis);
        }
        rearrange_act_remainder(
            state.run.as_mut().expect("an act belief state holds a run"),
            &mut Rearrange::Deal(&mut analysis),
        );
        state.rng = RunRngSet::new(format!("ANALYSIS{:016X}", splitmix64(&mut mix)));
        state.player_rng = PlayerRngSet::new(splitmix64(&mut mix));
        Simulator::from_scenario(state, Arc::clone(&self.content))
    }
}

/// The pile's contents are the player's to see; the order is not, and
/// neither is which same-looking copy sits where. Sorting by the serialized
/// print puts both beyond reach, with the instance id as a tiebreak so the
/// erasure itself is deterministic.
fn sort_hidden_draw_order(combat: &mut CombatState) {
    combat.player.piles.draw.sort_by_cached_key(|card| {
        (
            serde_json::to_string(&card.fingerprint).expect("a fingerprint serializes"),
            card.id,
        )
    });
}

/// How one hidden remainder is rearranged: the erasure puts it into its one
/// canonical order, a sample deals it a fresh permutation. Both run the same
/// walk over the same remainders, which is what makes the erasure a
/// projection — a sample re-erased canonicalizes back to the very belief
/// state it was dealt from.
enum Rearrange<'a> {
    Canonical,
    Deal(&'a mut MegaRandom),
}

impl Rearrange<'_> {
    fn slice<T, K: Ord>(&mut self, values: &mut [T], key: impl Fn(&T) -> K) {
        match self {
            Self::Canonical => values.sort_by_cached_key(key),
            Self::Deal(rng) => shuffle(values, rng),
        }
    }
}

/// Every act-level hidden remainder, walked in one fixed order.
///
/// Only the current act is touched. Later acts are beyond the horizon and
/// carried; finished acts' remainders are dead state nothing ahead reads.
fn rearrange_act_remainder(run: &mut RunState, order: &mut Rearrange<'_>) {
    let act_index = run.current_act;
    if let Some(act) = run.acts.get_mut(act_index) {
        // The walked prefix of each pool is evidence and stays put; the
        // suffix is hidden order. The events cursor also counts entries the
        // act passed over without showing them — and the ancient the act
        // opened on — so those stay pinned too, a leak this model accepts:
        // they are dead until the cursor laps the list, which an act's worth
        // of rooms does not do.
        let normals = act
            .normal_encounters_visited
            .min(act.normal_encounters.len());
        order.slice(&mut act.normal_encounters[normals..], Clone::clone);
        let elites = act.elite_encounters_visited.min(act.elite_encounters.len());
        order.slice(&mut act.elite_encounters[elites..], Clone::clone);
        let events = act.events_visited.min(act.events.len());
        order.slice(&mut act.events[events..], Clone::clone);
    }
    // Definitions placed at unwalked map points are hidden; which unwalked
    // point of their type holds which is put beyond reach, and everything a
    // definition carries — enemies, pre-rolled rewards — rides with it. The
    // list itself is kept in coordinate order so that where an entry sits in
    // the vector says nothing either.
    let visited: BTreeSet<MapCoord> = run
        .visited
        .iter()
        .copied()
        .chain(run.current_coord)
        .collect();
    if let (Some(map), Some(rooms)) = (run.maps.get(act_index), run.rooms.get_mut(act_index)) {
        rooms.sort_by_key(|room| room.coord);
        for point_type in [
            MapPointType::Ancient,
            MapPointType::Monster,
            MapPointType::Elite,
            MapPointType::Unknown,
            MapPointType::Shop,
            MapPointType::RestSite,
            MapPointType::Treasure,
            MapPointType::Boss,
        ] {
            let group: Vec<usize> = rooms
                .iter()
                .enumerate()
                .filter(|(_, room)| {
                    !visited.contains(&room.coord)
                        && map.points.iter().any(|point| {
                            point.coord == room.coord && point.point_type == point_type
                        })
                })
                .map(|(index, _)| index)
                .collect();
            if group.len() < 2 {
                continue;
            }
            let mut definitions: Vec<RoomDefinition> = group
                .iter()
                .map(|&index| rooms[index].definition.clone())
                .collect();
            order.slice(&mut definitions, |definition| {
                serde_json::to_string(definition).expect("a room definition serializes")
            });
            for (&index, definition) in group.iter().zip(definitions) {
                rooms[index].definition = definition;
            }
        }
    }
    // A grab bag's remaining contents are derivable — the pools are pinned
    // content and every relic that left a bag was seen leaving — but the
    // order the front and back will deal in is not. The starter bucket never
    // holds anything, the same six as `RelicGrabBagState::remove` walks.
    for bag in [
        &mut run.shared_relic_grab_bag,
        &mut run.player_relic_grab_bag,
    ] {
        for rarity in [
            RelicRarity::Common,
            RelicRarity::Uncommon,
            RelicRarity::Rare,
            RelicRarity::Shop,
            RelicRarity::Event,
            RelicRarity::Ancient,
        ] {
            order.slice(bag.bucket_mut(rarity), Clone::clone);
        }
    }
}

/// Fisher–Yates under the analysis generator.
fn shuffle<T>(values: &mut [T], rng: &mut MegaRandom) {
    for hole in (1..values.len()).rev() {
        #[allow(
            clippy::cast_possible_truncation,
            reason = "pile sizes are far below u64"
        )]
        let pick = (rng.next_u64() % (hole as u64 + 1)) as usize;
        values.swap(hole, pick);
    }
}
