use serde::{Deserialize, Serialize};
use sts2_core::{ModelId, PINNED_COMPATIBILITY_ID};
use sts2_rng::canonicalize_seed;

use crate::{
    CardFingerprint, EngineError, ErrorCode, RewardFingerprint, RewardSet, SuspendedChoice,
};

pub const FIXTURE_UNLOCK_PRESET: &str = sts2_core::FIXTURE_UNLOCK_PRESET;

/// The lobby the run was started from. `Custom` is the one workflow where the
/// game accepts a chosen seed — a played-back decision script records as it —
/// and generates exactly as `Standard` does; the mode is carried because the
/// build carries it and the projection compares it.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunMode {
    Standard,
    Custom,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RunPlayerConfig {
    pub slot: String,
    pub character: ModelId,
}

/// The lobby's act-one setting: which act one a run opens in.
///
/// The lobby offers it as one of three choices before the run starts, and it
/// is run-defining twice over: `overgrowth` and `underdocks` name the act
/// outright, while `random` leaves standing whatever the act-selection stream
/// rolled. It is not derivable from the seed, so it belongs in the config.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActOneSetting {
    /// The rolled act stands. This is what the lobby starts at, and the build
    /// hides the dropdown that would change it, so it is what a recorded
    /// single-player run of this baseline used.
    #[default]
    Random,
    Overgrowth,
    Underdocks,
}

impl ActOneSetting {
    /// The act this setting names outright, if it names one at all.
    #[must_use]
    pub fn overridden_act(self) -> Option<&'static str> {
        match self {
            Self::Random => None,
            Self::Overgrowth => Some("ACT.OVERGROWTH"),
            Self::Underdocks => Some("ACT.UNDERDOCKS"),
        }
    }
}

/// Every generation-affecting input required by the fidelity contract.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RunConfig {
    pub seed: String,
    pub compatibility_id: String,
    pub mode: RunMode,
    pub players: Vec<RunPlayerConfig>,
    /// The run's ascension level, zero through
    /// [`MAX_ASCENSION`](crate::MAX_ASCENSION). The level is a run-defining
    /// input the seed does not carry, and it is where every "does this run sit
    /// at ascension N or above?" question in the run is answered from.
    pub ascension: u8,
    pub modifiers: Vec<ModelId>,
    /// Which act one the lobby asked for. Act two and act three have one
    /// candidate each at this baseline, so only act one has a setting.
    pub act_one: ActOneSetting,
    pub unlock_preset: String,
    /// Explicit generation inputs, including profiles supplied by a replay.
    /// Older configurations resolve `unlock_preset` through the registry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unlock_profile: Option<sts2_core::UnlockPresetManifest>,
}

impl RunConfig {
    #[must_use]
    pub fn fixture_baseline(seed: impl Into<String>) -> Self {
        Self {
            seed: seed.into(),
            compatibility_id: PINNED_COMPATIBILITY_ID.to_owned(),
            mode: RunMode::Standard,
            players: vec![RunPlayerConfig {
                slot: "p0".to_owned(),
                character: "CHARACTER.IRONCLAD".parse().expect("static character ID"),
            }],
            ascension: 0,
            modifiers: Vec::new(),
            act_one: ActOneSetting::Random,
            unlock_preset: FIXTURE_UNLOCK_PRESET.to_owned(),
            unlock_profile: None,
        }
    }

    /// The same configuration generated at an ascension level.
    #[must_use]
    pub fn at_ascension(mut self, ascension: u8) -> Self {
        self.ascension = ascension;
        self
    }

    pub fn validate(&self) -> Result<(), EngineError> {
        if self.compatibility_id != PINNED_COMPATIBILITY_ID {
            return Err(EngineError::new(ErrorCode::UnsupportedBuild)
                .with_detail("compatibility_id", self.compatibility_id.clone()));
        }
        if self.seed.is_empty() || canonicalize_seed(&self.seed) != self.seed {
            return Err(EngineError::new(ErrorCode::InvalidConfig).with_detail("field", "seed"));
        }
        // A lobby that asked for a random character is a legal input: which
        // character it becomes is rolled at run start, and a run that rolls
        // one this baseline has no content for fails as unsupported content
        // rather than as a malformed config.
        let accepted_characters: [ModelId; 6] = [
            "CHARACTER.IRONCLAD".parse().expect("static character ID"),
            "CHARACTER.SILENT".parse().expect("static character ID"),
            "CHARACTER.REGENT".parse().expect("static character ID"),
            "CHARACTER.NECROBINDER"
                .parse()
                .expect("static character ID"),
            "CHARACTER.DEFECT".parse().expect("static character ID"),
            "CHARACTER.RANDOM_CHARACTER"
                .parse()
                .expect("static character ID"),
        ];
        if self.players.len() != 1
            || self.players[0].slot != "p0"
            || !accepted_characters.contains(&self.players[0].character)
        {
            return Err(EngineError::new(ErrorCode::InvalidConfig).with_detail("field", "players"));
        }
        // Ten is the highest ascension the game plays at, and the character
        // select screen offers nothing above it, so eleven is not a harder run
        // this simulator declines to generate — it is not a run.
        if self.ascension > crate::MAX_ASCENSION {
            return Err(EngineError::new(ErrorCode::InvalidConfig)
                .with_detail("field", "ascension")
                .with_detail("value", self.ascension));
        }
        if !self.modifiers.is_empty() {
            return Err(
                EngineError::new(ErrorCode::InvalidConfig).with_detail("field", "modifiers")
            );
        }
        // An explicit profile carries its own inputs. A name alone must still
        // resolve through the registry; it never implies a default profile.
        let valid_profile = self.unlock_profile.as_ref().map_or_else(
            || sts2_core::UnlockPresetManifest::with_id(&self.unlock_preset).is_ok(),
            |profile| {
                profile.schema_version == 1
                    && profile.compatibility_id == self.compatibility_id
                    && profile.preset_id == self.unlock_preset
            },
        );
        if !valid_profile {
            return Err(EngineError::new(ErrorCode::InvalidConfig)
                .with_detail("field", "unlock_preset")
                .with_detail("value", self.unlock_preset.clone()));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct MapCoord {
    pub col: u8,
    pub row: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapPointType {
    Ancient,
    Monster,
    Elite,
    Unknown,
    Shop,
    RestSite,
    Treasure,
    Boss,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MapPoint {
    pub coord: MapCoord,
    pub point_type: MapPointType,
    pub children: Vec<MapCoord>,
    pub can_modify: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActMap {
    pub width: u8,
    pub height: u8,
    pub start: MapCoord,
    pub boss: MapCoord,
    /// A second boss standing a row behind the first, which the last act of a
    /// Double Boss run lays out and nothing below that level does.
    pub second_boss: Option<MapCoord>,
    pub points: Vec<MapPoint>,
}

impl ActMap {
    #[must_use]
    pub fn destinations(&self, current: Option<MapCoord>) -> &[MapCoord] {
        let Some(current) = current else {
            // Stepping onto a fresh act means stepping onto its start point.
            return std::slice::from_ref(&self.start);
        };
        self.points
            .iter()
            .find(|point| point.coord == current)
            .map_or(&[], |point| point.children.as_slice())
    }

    pub fn validate(&self) -> Result<(), EngineError> {
        if self.width == 0
            || self.height == 0
            || !self.points.iter().any(|point| point.coord == self.start)
        {
            return Err(EngineError::new(ErrorCode::InternalInvariant)
                .with_detail("map", "invalid dimensions or start"));
        }
        // The map grid runs to the act's own length; the boss stands one row
        // past it and a second boss one row past the boss, so the last row a
        // point may stand on is whichever of those the act laid out.
        let last_row = self.second_boss.map_or(self.height, |coord| coord.row);
        for point in &self.points {
            if point.coord.col >= self.width || point.coord.row > last_row {
                return Err(EngineError::new(ErrorCode::InternalInvariant).with_detail(
                    "map_point",
                    format!("{},{}", point.coord.col, point.coord.row),
                ));
            }
            for child in &point.children {
                if !self.points.iter().any(|point| point.coord == *child)
                    || child.row != point.coord.row + 1
                {
                    return Err(EngineError::new(ErrorCode::InternalInvariant)
                        .with_detail("map_edge", format!("{point:?} -> {child:?}")));
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActRoomPools {
    pub act_model: ModelId,
    pub ancient: ModelId,
    pub boss: ModelId,
    /// A second boss for acts that hold one. No act at this baseline does.
    pub second_boss: Option<ModelId>,
    pub normal_encounters: Vec<ModelId>,
    pub elite_encounters: Vec<ModelId>,
    pub events: Vec<ModelId>,
    /// How far into each list this act has walked. These belong to the act
    /// rather than the run: a finished act keeps its totals while later acts
    /// start their own lists from the front.
    pub normal_encounters_visited: usize,
    pub elite_encounters_visited: usize,
    pub events_visited: usize,
    pub boss_encounters_visited: usize,
}

/// Relic rarities that take part in grab-bag generation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelicRarity {
    /// A rarity no pool holds and generation never rolls. Only the circlet a
    /// refinement falls back to carries it.
    None,
    /// What a character starts the run holding. Generation never rolls it.
    Starter,
    // Generation rolls and prices these four, in this order.
    Common,
    Uncommon,
    Rare,
    Shop,
    /// What an event hands out.
    Event,
    /// What only a boss or an ancient gives.
    Ancient,
}

impl RelicRarity {
    /// Where this rarity stands among the four a merchant prices, which is the
    /// order a merchant's price list runs in. A rarity a merchant never stocks
    /// has no place there.
    #[must_use]
    pub const fn shop_price_index(self) -> Option<usize> {
        match self {
            Self::Common => Some(0),
            Self::Uncommon => Some(1),
            Self::Rare => Some(2),
            Self::Shop => Some(3),
            Self::None | Self::Starter | Self::Event | Self::Ancient => None,
        }
    }

    /// The rarity a pull falls back to when its own bucket has nothing usable.
    #[must_use]
    pub const fn next_fallback(self) -> Option<Self> {
        match self {
            Self::Shop => Some(Self::Common),
            Self::Common => Some(Self::Uncommon),
            Self::Uncommon => Some(Self::Rare),
            Self::None | Self::Starter | Self::Rare | Self::Event | Self::Ancient => None,
        }
    }
}

/// Ordered relic pools whose order affects every future relic reward.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RelicGrabBagState {
    /// Always empty: generation never rolls a starter relic.
    #[serde(default)]
    pub starter: Vec<ModelId>,
    pub common: Vec<ModelId>,
    pub uncommon: Vec<ModelId>,
    pub rare: Vec<ModelId>,
    pub shop: Vec<ModelId>,
    pub event: Vec<ModelId>,
    pub ancient: Vec<ModelId>,
}

impl RelicGrabBagState {
    /// A starter relic is never in a bag, and neither is a relic of no rarity
    /// at all: nothing rolls either, so the bucket a pull would ask for is the
    /// empty one they share with every other rarity the bags do not carry.
    #[must_use]
    pub const fn bucket(&self, rarity: RelicRarity) -> &Vec<ModelId> {
        match rarity {
            RelicRarity::None | RelicRarity::Starter => &self.starter,
            RelicRarity::Common => &self.common,
            RelicRarity::Uncommon => &self.uncommon,
            RelicRarity::Rare => &self.rare,
            RelicRarity::Shop => &self.shop,
            RelicRarity::Event => &self.event,
            RelicRarity::Ancient => &self.ancient,
        }
    }

    #[must_use]
    pub const fn bucket_mut(&mut self, rarity: RelicRarity) -> &mut Vec<ModelId> {
        match rarity {
            RelicRarity::None | RelicRarity::Starter => &mut self.starter,
            RelicRarity::Common => &mut self.common,
            RelicRarity::Uncommon => &mut self.uncommon,
            RelicRarity::Rare => &mut self.rare,
            RelicRarity::Shop => &mut self.shop,
            RelicRarity::Event => &mut self.event,
            RelicRarity::Ancient => &mut self.ancient,
        }
    }

    /// Whether any rarity still has a relic left to give.
    #[must_use]
    pub fn has_relics(&self) -> bool {
        [
            RelicRarity::Starter,
            RelicRarity::Common,
            RelicRarity::Uncommon,
            RelicRarity::Rare,
            RelicRarity::Shop,
            RelicRarity::Event,
            RelicRarity::Ancient,
        ]
        .iter()
        .any(|rarity| !self.bucket(*rarity).is_empty())
    }

    /// Takes the first usable relic of `rarity`. Reward sources pull from the
    /// front; merchants pull from the back.
    pub fn pull_from_front(
        &mut self,
        rarity: RelicRarity,
        excluded: &[ModelId],
    ) -> Option<ModelId> {
        let mut rarity = Some(rarity);
        while let Some(current) = rarity {
            let bucket = self.bucket_mut(current);
            if let Some(index) = bucket
                .iter()
                .position(|model_id| !excluded.contains(model_id))
            {
                return Some(bucket.remove(index));
            }
            rarity = current.next_fallback();
        }
        None
    }

    /// Takes the last usable relic of `rarity`, walking the fallback chain when
    /// a bucket has nothing the filter allows. Shops pull from the back so they
    /// never compete with the reward sources that pull from the front.
    pub fn pull_from_back(&mut self, rarity: RelicRarity, excluded: &[ModelId]) -> Option<ModelId> {
        let mut rarity = Some(rarity);
        while let Some(current) = rarity {
            let bucket = self.bucket_mut(current);
            if let Some(index) = bucket
                .iter()
                .rposition(|model_id| !excluded.contains(model_id))
            {
                return Some(bucket.remove(index));
            }
            rarity = current.next_fallback();
        }
        None
    }

    /// Drops one relic from every bucket, as taking a relic does to the pool it
    /// is no longer available in.
    pub fn remove(&mut self, model_id: &ModelId) {
        for bucket in [
            &mut self.common,
            &mut self.uncommon,
            &mut self.rare,
            &mut self.shop,
            &mut self.event,
            &mut self.ancient,
        ] {
            bucket.retain(|entry| entry != model_id);
        }
    }
}

/// What an unknown map point turned into.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolvedRoom {
    Monster,
    /// The act's boss, which is a room family a relic asks about even though an
    /// unknown point never becomes one.
    Boss,
    /// A rest site, likewise.
    RestSite,
    /// Carried by the weight table but never rolled at this baseline: its
    /// weight is negative, and the roll skips negative weights.
    Elite,
    Treasure,
    Shop,
    Event,
}

/// Run-level odds for what an unknown map point becomes. Each roll resets the
/// type it produced to its base weight and raises every other type by its base
/// weight, and the whole set resets between acts.
///
/// Elite is in the table with a base weight of -1. A negative weight is never
/// rolled, and because it is never the rolled type it only ever takes the
/// raise, so it drifts further negative as the act goes on. It is carried
/// rather than dropped because the recording projects it and modifiers can set
/// a non-negative base for it.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UnknownRoomOdds {
    pub monster_bits: u32,
    pub elite_bits: u32,
    pub treasure_bits: u32,
    pub shop_bits: u32,
}

impl UnknownRoomOdds {
    pub const BASE: [f32; 4] = [0.1, -1.0, 0.02, 0.03];
    /// The order the roll walks, which is the order the pinned build's weight
    /// table is declared in.
    pub const ORDER: [ResolvedRoom; 4] = [
        ResolvedRoom::Monster,
        ResolvedRoom::Elite,
        ResolvedRoom::Treasure,
        ResolvedRoom::Shop,
    ];

    #[must_use]
    pub const fn start_of_act() -> Self {
        Self {
            monster_bits: Self::BASE[0].to_bits(),
            elite_bits: Self::BASE[1].to_bits(),
            treasure_bits: Self::BASE[2].to_bits(),
            shop_bits: Self::BASE[3].to_bits(),
        }
    }

    #[must_use]
    pub const fn weights(self) -> [f32; 4] {
        [
            f32::from_bits(self.monster_bits),
            f32::from_bits(self.elite_bits),
            f32::from_bits(self.treasure_bits),
            f32::from_bits(self.shop_bits),
        ]
    }

    fn set_weights(&mut self, weights: [f32; 4]) {
        self.monster_bits = weights[0].to_bits();
        self.elite_bits = weights[1].to_bits();
        self.treasure_bits = weights[2].to_bits();
        self.shop_bits = weights[3].to_bits();
    }

    /// Applies the drift that follows a roll. A type that was not on the table
    /// takes no part.
    pub fn record(&mut self, rolled: ResolvedRoom, blocked: &[ResolvedRoom]) {
        let mut weights = self.weights();
        for (index, kind) in Self::ORDER.into_iter().enumerate() {
            if blocked.contains(&kind) {
                continue;
            }
            weights[index] = if kind == rolled {
                Self::BASE[index]
            } else {
                weights[index] + Self::BASE[index]
            };
        }
        self.set_weights(weights);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunPhase {
    Map,
    Room,
    Rewards,
    ActTransition,
}

/// One relic on a page that offers a choice between relics, with the name the
/// option is keyed by.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RelicOnOffer {
    pub model_id: ModelId,
    /// The relic's displayed name, which is the option's key on such a page.
    pub name: String,
}

/// The rest of a purchase, waiting on the reward screen the item itself opened.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OwedPurchase {
    pub offer_index: usize,
    pub on_a_merchants_shelf: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag is one thing the run still owes or has already settled"
)]
pub struct RunState {
    pub config: RunConfig,
    pub current_act: usize,
    pub floor: u32,
    /// Where the player is standing. Between acts they stand nowhere: the new
    /// act's map opens with its start point as the only place to go.
    pub current_coord: Option<MapCoord>,
    pub maps: Vec<ActMap>,
    pub rooms: Vec<Vec<RoomAt>>,
    /// Map points walked onto in this act, in the order they were walked.
    /// The recording keeps traversal order, not sorted order, and clears this
    /// when a new act opens.
    pub visited: Vec<MapCoord>,
    pub acts: Vec<ActRoomPools>,
    pub phase: RunPhase,
    pub current_room_model: Option<ModelId>,
    /// The id the next room built will take: every room takes it and raises
    /// it as it is built, and it resets as a map point is walked onto. Almost
    /// every room is
    /// the first at its point and so is room zero; a room the run stands up
    /// without moving — the fight an event pushes, or the Architect's room
    /// behind the last boss — is the next one along.
    #[serde(default)]
    pub next_room_id: u32,
    /// The id the current room took.
    #[serde(default)]
    pub current_room_id: u32,
    /// What the current room turned out to be. An unknown map point only
    /// becomes a real family when the player walks onto it.
    pub current_room_type: Option<MapPointType>,
    /// What the room before it turned out to be, which an unknown point reads
    /// when deciding what it can become.
    pub previous_room_type: Option<MapPointType>,
    /// Ordinary events walked into so far, run-wide. An act's ancient is not
    /// one of them, and the recording keeps this sorted rather than in visit
    /// order.
    pub visited_events: std::collections::BTreeSet<ModelId>,
    /// The event the run last began, with the page it is standing on.
    ///
    /// The run keeps the event it began until the next event begins, so an
    /// event that never reached its finish is still there — and still
    /// projected — after its room has been left. An event that finished is
    /// standing on a page with no options, which is what the empty list means
    /// here.
    #[serde(default)]
    pub begun_event: Option<BegunEvent>,
    pub active_room: Option<ActiveRoom>,
    /// A room standing above the map point's own: the fight an event walks
    /// into without leaving the event, which stands on top of the event's room
    /// rather than in place of it.
    ///
    /// It stays on the stack until the next map point is walked onto, which is
    /// where the whole stack is dropped at once. Only a combat room is ever
    /// pushed at this baseline.
    #[serde(default)]
    pub pushed_room: Option<PushedRoom>,
    pub shared_relic_grab_bag: RelicGrabBagState,
    pub player_relic_grab_bag: RelicGrabBagState,
    /// The map point a quest card of the deck marked as the act's map was
    /// generated. The mark is not saved with the run: it belongs to the
    /// session and is worked out again whenever the map is.
    #[serde(default)]
    pub marked_map_coord: Option<MapCoord>,
    pub reward_set: Option<RewardSet>,
    /// A reward set the player has walked away from, which the *projection*
    /// still carries. Proceeding from a terminal rewards screen only opens
    /// the map: the set stays stacked and unreachable until the next map point
    /// is walked onto and the old room is finally left. Nothing reads it —
    /// `reward_set` is what "a set is still owed an answer" means, and this
    /// one is not — so it is kept apart from it.
    #[serde(default)]
    pub dismissed_reward_set: Option<RewardSet>,
    /// A rest-site option taken whose trim the *projection* has not caught up
    /// with. The list is trimmed at once — that is what decides whether another
    /// option may be taken — but the recorder's rest observation comes from the
    /// option-chosen event, which the build raises before the trim, and holds
    /// that value until the next thing the player does is observed.
    #[serde(default)]
    pub rest_option_owed: Option<usize>,
    /// The option indices that were standing when `rest_option_owed` was
    /// chosen. The recorder holds precisely this pre-trim list on screen;
    /// keeping the snapshot matters when Miniature Tent has already spent an
    /// earlier option.
    #[serde(default)]
    pub rest_options_owed: Vec<usize>,
    pub suspended_choice: Option<SuspendedChoice>,
    /// What a command run outside a combat still owes once a selection screen
    /// has interrupted it. Out here there is no combat to hold a queue, and
    /// nothing out of combat defers work into one either: every out-of-combat
    /// selection blocks on its own screen, so whatever the interrupted effect
    /// had left to do simply waits for the answer.
    #[serde(default)]
    pub pending: std::collections::VecDeque<crate::Effect>,
    pub next_choice_id: u64,
    pub next_reward_set_id: u64,
    pub finish_event_after_rewards: bool,
    /// An event page whose chosen option stopped on a screen. Finishing the
    /// event is the last thing such an option does — an ancient's relic option
    /// is the case that reaches this — so the page stays standing while the
    /// screen is up and the room finishes once the rest of the option has run
    /// out.
    #[serde(default)]
    pub finish_event_after_pending: bool,
    /// A purchase whose own effect put a reward set up. The shelf is cleared
    /// and the sale announced only once the purchase itself has finished, and
    /// an orrery's offer of five card rewards happens inside the purchase, so
    /// the entry stands with its item on it for as long as the screen does.
    #[serde(default)]
    pub purchase_owed_by_rewards: Option<OwedPurchase>,
    /// The rest of an effect that put a reward set up and waits on it, where
    /// the set is one the player may walk away from. An offer does not come
    /// back until the set is done with, taken or skipped, so whatever the
    /// effect had after the offer runs then — but the queue is not held for
    /// such a set,
    /// so the rest is carried here rather than on `pending`, and joins it as
    /// the set goes. A grab off an endless conveyor whose plate put a potion
    /// on the table is what reaches this.
    #[serde(default)]
    pub owed_by_rewards: std::collections::VecDeque<crate::Effect>,
    /// Card removals bought at merchants so far. Each one raises the next
    /// merchant's removal price.
    pub card_removals_used: i32,
    /// The purchases a lord's parasol owes the shelf, oldest first: offer
    /// indexes in the order the game buys them, with whether the card
    /// removal it forces afterwards is still owed. The game makes them after
    /// the room is entered, one at a time, each as a purchase of its own;
    /// here each is made as the step that names it, or all of them before
    /// whatever else is stepped, so the shelf still stands full as the room
    /// is entered.
    #[serde(default)]
    pub owed_purchases: std::collections::VecDeque<usize>,
    #[serde(default)]
    pub owed_removal: bool,
    pub unknown_room_odds: UnknownRoomOdds,
    /// The normalized unlock inputs this run was generated against. Not
    /// profile decoration: the epoch and encounter lists decide what is
    /// eligible at all, and a profile with no finished runs forces its first
    /// unknown map points. Past a first run the count changes no generation
    /// and is carried for the projection alone.
    pub unlocks: sts2_core::UnlockState,
    /// Every map point walked, act by act, with what each turned out to be.
    /// The pinned build reads this back: an unknown map point rolled by a
    /// profile with no finished runs counts the unknown points in here.
    pub map_history: Vec<ActHistory>,
    /// Whether the run opened on Neow. Every standard run at this baseline
    /// does; a run resumed from a checkpoint would not.
    pub started_with_neow: bool,
    /// How many times this run has been loaded back off its save file. The
    /// count rises as the load begins, so a run that was picked up once
    /// carries one.
    #[serde(default)]
    pub reloads: i32,
}

impl RunState {
    /// What the room the player is standing in is, which is the top of the
    /// room stack rather than the map point below it. A relic that only pays
    /// out in a monster room reads this, so a fight an event walked into
    /// counts as the monster room it is.
    #[must_use]
    pub fn standing_room_type(&self) -> Option<MapPointType> {
        self.pushed_room
            .as_ref()
            .map(|pushed| pushed.room_type)
            .or(self.current_room_type)
    }

    #[must_use]
    pub fn map(&self) -> &ActMap {
        &self.maps[self.current_act]
    }

    #[must_use]
    pub fn act(&self) -> &ActRoomPools {
        &self.acts[self.current_act]
    }

    pub fn act_mut(&mut self) -> &mut ActRoomPools {
        &mut self.acts[self.current_act]
    }

    /// The point being stood on, as the history records it.
    pub fn visit_mut(&mut self) -> Option<&mut MapPointVisit> {
        self.map_history.last_mut()?.points.last_mut()
    }
}

/// One act's walked points, in the order they were walked.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActHistory {
    pub act: usize,
    pub points: Vec<MapPointVisit>,
}

/// A map point the player stood on.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MapPointVisit {
    pub floor: u32,
    /// What the map laid out here, which stays `Unknown` for an unknown point
    /// even after the room it became is known.
    pub point_type: MapPointType,
    pub rooms: Vec<RoomVisit>,
}

/// The event the run last began, and the page it stands on.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BegunEvent {
    pub model_id: ModelId,
    pub options: Vec<EventOption>,
}

/// A room standing on top of the one the map point put up.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PushedRoom {
    pub model_id: ModelId,
    /// The id this room took off the run's own counter.
    #[serde(default)]
    pub id: u32,
    /// What the pushed room itself is, rather than what the map point below
    /// it says.
    pub room_type: MapPointType,
    /// Whether proceeding past the fight's rewards goes back to the event
    /// that pushed it or opens the map.
    pub resumes_parent: bool,
    /// What the event picks back up with once the fight it pushed is done
    /// with, which is what pays out for the fight. Empty for a fight nothing
    /// resumes.
    #[serde(default)]
    pub resume: Vec<EventEffect>,
}

/// A room entered while standing on a point.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RoomVisit {
    pub index: usize,
    pub model_id: Option<ModelId>,
    /// What the room turned out to be, which is the resolved family rather
    /// than the map point's own type.
    pub room_type: MapPointType,
    /// Every monster that turned up in the fight, in the order it joined.
    /// Kept by combat identity because an encounter can field several of the
    /// same monster, and a monster that dies stays on the record.
    pub monsters: Vec<MonsterVisit>,
    pub turns_taken: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MonsterVisit {
    pub combat_id: sts2_core::CreatureCombatId,
    pub model_id: ModelId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EnemySpawn {
    pub model_id: ModelId,
    /// Fixed maximum HP for rooms that replay a recorded spawn. `None` rolls
    /// the definition's unique initial HP from the niche stream.
    pub fixed_hp: Option<i32>,
    /// Opening move for this slot. The pinned machines pick their opener from a
    /// condition on where the monster stands, and an encounter that places a
    /// monster in a known spot names the resulting opener directly.
    pub initial_state: Option<String>,
    /// Which of the encounter's named slots this monster stands in. `None` is
    /// a monster the encounter names no slot for; looking such a name up in
    /// the slot list finds nothing, which counts as slot −1.
    #[serde(default)]
    pub slot_name: Option<String>,
    /// A number the encounter writes onto the monster before the fight stands
    /// up, which the monster then reads back off itself. The starting-HP
    /// reduction written onto a punching construct is the one this baseline
    /// reaches.
    #[serde(default)]
    pub locals: Vec<(String, i32)>,
    /// The same, where the encounter rolls the number on its own floor-local
    /// RNG rather than writing it down: one `next_int_range(minimum, maximum)`
    /// draw per spawn, in spawn order, whose result is moved into `locals` as
    /// the room is built.
    #[serde(default)]
    pub rolled_locals: Vec<RolledLocal>,
}

/// One number an encounter rolls onto a spawn as it generates its monsters.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RolledLocal {
    pub name: String,
    /// The draw is `next_int_range(minimum, maximum)`, whose upper bound is
    /// exclusive.
    pub minimum: i32,
    pub maximum: i32,
}

impl EnemySpawn {
    #[must_use]
    pub const fn rolled(model_id: ModelId) -> Self {
        Self {
            model_id,
            fixed_hp: None,
            initial_state: None,
            slot_name: None,
            locals: Vec::new(),
            rolled_locals: Vec::new(),
        }
    }

    #[must_use]
    pub fn opening(mut self, state: &str) -> Self {
        self.initial_state = Some(state.to_owned());
        self
    }

    /// A number the encounter rolls onto this spawn as it generates it.
    #[must_use]
    pub fn rolling(mut self, name: &str, minimum: i32, maximum: i32) -> Self {
        self.rolled_locals.push(RolledLocal {
            name: name.to_owned(),
            minimum,
            maximum,
        });
        self
    }
}

/// How an encounter picks the monsters it spawns.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum MonsterGeneration {
    /// Always the same monsters, consuming no encounter randomness.
    Fixed(Vec<ModelId>),
    /// Always the same spawns, where the encounter also fixes their openers.
    Placed(Vec<EnemySpawn>),
    /// Every monster starts on one of its own openers, and one roll decides
    /// which opener the first takes: the rest follow it round the list, which
    /// is what one starting-move index rolled once and offset per spawn comes
    /// to.
    RotatingOpeners { spawns: Vec<RotatingSpawn> },
    /// One `next_bool` draw decides between two arrangements of the same
    /// monsters, which is what an encounter that shuffles two of its spawns
    /// does with a single coin.
    Coin {
        heads: Vec<ModelId>,
        tails: Vec<ModelId>,
    },
    /// One even draw over the arrangements an encounter can take, and then
    /// whatever that arrangement rolls for itself, with the monster the
    /// encounter is named for appended after it.
    ///
    /// One roll picks the branch, and each entry of the branch is either a
    /// monster written down or one rolled out of a list, in the order the
    /// branch names them.
    Branched {
        branches: Vec<Vec<MonsterPick>>,
        tail: Vec<ModelId>,
    },
    /// One monster out of a list, where which one is a setting the event that
    /// stands the fight up writes on the encounter rather than anything the
    /// encounter rolls. No map point can ever be such an encounter, so
    /// generating the room off the map is unsupported content.
    ChosenByTheEvent { options: Vec<ModelId> },
    /// Draws from candidate pools without replacement within a pool, in the
    /// order the game's generator draws them, then spawns `spawn_order`.
    Draws {
        pools: Vec<Vec<ModelId>>,
        /// Pool index for each draw, in exact draw order.
        draws: Vec<usize>,
        /// What stands in each spawn slot, in spawn order.
        spawn_order: Vec<MonsterSlot>,
    },
}

/// One monster of a branch: written down, or rolled out of a list.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum MonsterPick {
    Fixed(ModelId),
    Rolled(Vec<ModelId>),
}

/// One spawn whose opener is decided by the encounter's single roll.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RotatingSpawn {
    pub model_id: ModelId,
    /// The openers the rolled starting-move index picks between, in the
    /// build's order.
    pub openers: Vec<String>,
    /// The starting-move index this spawn is given outright, where the
    /// encounter names one rather than rotating it off the roll. It is what a
    /// fourth scroll standing beside three rotating ones carries.
    #[serde(default)]
    pub fixed_opener: Option<usize>,
}

/// One spawn slot of a generated encounter.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum MonsterSlot {
    /// The monster produced by the numbered draw.
    Drawn(usize),
    /// A monster the encounter always spawns.
    Fixed(ModelId),
}

/// One card a merchant can stock, with everything merchant generation filters
/// and prices on.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MerchantCard {
    pub model_id: ModelId,
    pub card_type: crate::CardType,
    pub rarity: crate::CardRarity,
}

/// Everything a merchant needs to stock and price its inventory.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MerchantSpec {
    /// The character's card pool, in declaration order.
    pub character_pool: Vec<MerchantCard>,
    /// The colorless pool, in declaration order.
    pub colorless_pool: Vec<MerchantCard>,
    pub common_potions: Vec<ModelId>,
    pub uncommon_potions: Vec<ModelId>,
    pub rare_potions: Vec<ModelId>,
    /// Base card price by rarity, before the colorless surcharge and the price
    /// roll.
    pub card_cost: [i32; 3],
    /// Base potion price by rarity.
    pub potion_cost: [i32; 3],
    /// Base relic price by rarity, ordered common, uncommon, rare, shop.
    pub relic_cost: [i32; 4],
    /// Relics the merchant never stocks.
    pub relics_banned_from_shops: Vec<ModelId>,
    pub removal_base_cost: i32,
    pub removal_price_increase: i32,
}

/// A combat room the run generator can place on any map point of its type.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EncounterDefinition {
    pub model_id: ModelId,
    /// The names this encounter's creatures can stand in, in the order the
    /// side is sorted into by slot. An encounter that names none leaves every
    /// creature at −1, which is where a name the list does not carry places
    /// its creature.
    #[serde(default)]
    pub slots: Vec<String>,
    /// The slot name generation hands each spawn, in spawn order. Each spawn
    /// comes out paired with its own slot name, so the names are positional
    /// over whatever the generation shape produced. An encounter that names
    /// none — most of them — leaves this empty, and one whose spawn count
    /// varies names only the spawns it always makes.
    #[serde(default)]
    pub spawn_slot_names: Vec<String>,
    pub generation: MonsterGeneration,
    pub rewards: Option<GeneratedCombatRewardSpec>,
    /// Whether the room lays out a reward set as the fight ends. Almost every
    /// encounter does, even one whose set turns out empty; the battleworn
    /// dummy's does not — the event pays for that fight — and a fight that
    /// pays nothing builds no set, so it takes no reward-set identifier
    /// either.
    #[serde(default = "gives_rewards_by_default")]
    pub gives_rewards: bool,
}

const fn gives_rewards_by_default() -> bool {
    true
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RoomAt {
    pub coord: MapCoord,
    pub definition: RoomDefinition,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CombatRoomDefinition {
    pub encounter_model: ModelId,
    pub enemies: Vec<EnemySpawn>,
    pub rewards: Vec<RewardFingerprint>,
    pub generated_rewards: Option<GeneratedCombatRewardSpec>,
}

/// An event the run can walk into. What its page offers is rolled on the
/// event's own local RNG as the room is entered.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EventDefinition {
    pub model_id: ModelId,
    /// Set for an event laid out in front of a fight, naming the encounter it
    /// stands in front of.
    ///
    /// An event laid out this way makes its encounter's creatures as its room
    /// is entered — right after the page is built — rather than when the fight
    /// starts, so their rolled health is spent there. The fight the event
    /// walks into stands those same creatures up rather than making new
    /// ones.
    #[serde(default)]
    pub combat_layout: Option<ModelId>,
    /// An ancient restores whoever walks in before it says anything.
    pub heals_on_entry: bool,
    /// Whether the act may pull this event right now. An event that answers
    /// no is counted as visited and passed over, and the act moves on to the
    /// next candidate in its list.
    #[serde(default = "EventCondition::always")]
    pub allowed: EventCondition,
    /// The amounts the event settles onto its own declared vars, worked out
    /// once as the room is entered and before the page is built, in the order
    /// the event settles them. A page that
    /// rolls one spends the roll whether or not any option goes on to read it,
    /// and an amount or a condition that does read one names its index with
    /// [`EventAmount::Var`] or [`EventCondition::GoldAtLeastVar`].
    #[serde(default)]
    pub vars: Vec<EventAmount>,
    pub generation: EventGeneration,
}

/// What an event asks about the run before the act may pull it.
///
/// Each variant is one test a pinned event makes. Single-player is a constant
/// here, so a test every player has to pass is the one player's answer.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EventCondition {
    /// The event asks nothing and is always eligible.
    Always,
    /// The event is never pulled at this baseline.
    Never,
    /// Every test has to pass, in the order the event makes them.
    All(Vec<EventCondition>),
    /// Any one of them is enough.
    Any(Vec<EventCondition>),
    /// The run is past the act at this index.
    ActIndexAbove(usize),
    /// The run has not yet reached the act at this index.
    ActIndexBelow(usize),
    /// The run is in the act at this index.
    ActIndexIs(usize),
    /// The run stands on this floor or a later one, counted run-wide.
    TotalFloorAtLeast(u32),
    /// The run stands past this floor, counted run-wide.
    TotalFloorAbove(u32),
    /// The player is carrying at least this much gold.
    GoldAtLeast(i32),
    /// The player is on at least this much current HP.
    CurrentHpAtLeast(i32),
    /// The player is above this much current HP.
    CurrentHpAbove(i32),
    /// The player is down to this fraction of maximum HP or below, worked out
    /// as `max * numerator / denominator` in integers.
    CurrentHpAtMostFraction { numerator: i32, denominator: i32 },
    /// The player is holding at least this many potions, counting only the
    /// slots that actually have one in them.
    PotionsAtLeast(usize),
    /// At least this many deck cards pass the filter.
    DeckCardsAtLeast {
        filter: EventCardFilter,
        count: usize,
    },
    /// At least this many relics the player holds can be traded away.
    TradableRelicsAtLeast(usize),
    /// The player's relic grab bag still has a relic left to give.
    RelicGrabBagHasRelics,
    /// The profile has more than one character's card pool unlocked.
    MultipleCharacterPoolsUnlocked,
    /// No event pet is following the player.
    NoEventPet,
    /// The player holds a relic of this rarity, which is how Touch of Orobas
    /// finds the starter relic it refines.
    HoldsRelicOfRarity(RelicRarity),
    /// The player is holding this potion.
    HoldsPotion(ModelId),
    /// The potion belt has room for another.
    HasOpenPotionSlot,
    /// The player is off full HP — carrying a wound — which is what a dish
    /// that heals asks before it goes on the belt.
    Hurt,
    /// The player is carrying at least as much gold as one of the event's own
    /// vars holds: the same test as [`Self::GoldAtLeast`] over a number the
    /// room settled rather than one written down ahead of time.
    GoldAtLeastVar(usize),
}

impl EventCondition {
    /// The default an event that asks nothing about the run carries.
    #[must_use]
    pub const fn always() -> Self {
        Self::Always
    }
}

/// Which deck cards an event's eligibility test counts.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EventCardFilter {
    /// Any card at all.
    Any,
    /// A card that may be removed from the deck.
    Removable,
    /// A basic card that is also removable.
    BasicRemovable,
    /// A card that may be transformed, which for a deck card is the same set
    /// as the removable ones.
    Transformable,
    /// A card carrying this tag, of basic rarity and removable, which is what
    /// the Amalgamator accepts.
    BasicRemovableTagged(crate::CardTag),
    /// A card carrying this tag and of basic rarity, which is the test
    /// Tezcatara makes of the deck: removability does not come into it.
    BasicTagged(crate::CardTag),
    /// A card the Archaic Tooth has an ancient form for, which is what it
    /// looks for in the deck as it is set up.
    Transcendable,
    /// A card any enchantment accepts before it adds tests of its own: not a
    /// status, a curse or a quest card, not unplayable while it sits in the
    /// deck, and not already enchanted.
    Enchantable,
    /// The same, and an attack, which is the restriction the Vigorous
    /// enchantment adds.
    EnchantableAttack,
    /// The same, and carrying the Exhaust keyword, which is the restriction
    /// the Souls enchantment adds.
    EnchantableWithExhaust,
    /// The same, and tagged Defend, which is the restriction the Goopy
    /// enchantment adds.
    EnchantableDefend,
    /// The same, and a basic card tagged Strike or Defend, which is the
    /// restriction the Spiral enchantment adds.
    EnchantableBasicStrikeOrDefend,
    /// A card the named enchantment accepts, with nothing added on top: the
    /// plain "does any deck card take this enchantment?" a wood carving and an
    /// ancient both ask.
    EnchantableBy(ModelId),
    /// A card the named enchantment accepts that is also of this type: both
    /// tests together, which is what a self-help book asks of the deck once
    /// per enchantment it offers.
    EnchantableByOfType {
        enchantment: ModelId,
        card_type: crate::CardType,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EventGeneration {
    /// A page that never varies.
    Fixed(Vec<EventOption>),
    /// A fake merchant's shelf rather than a page, stocked before the event
    /// starts. The relics are shuffled on the event's own RNG, the first few
    /// are stocked, and each one prices itself on the player's shops stream
    /// the way any merchant's stock does. The page itself carries no options
    /// at all.
    MerchantShelf {
        relics: Vec<ModelId>,
        offered: usize,
        /// The merchant cost these relics carry, which is the same number
        /// for every one of them.
        cost: i32,
    },
    /// A page whose options each stand behind a test, and show a locked option
    /// in place of themselves when it fails.
    Conditional(Vec<ConditionalOption>),
    /// An event registered for what the act needs to know about it — whether it
    /// may be pulled at all — whose page is not transcribed. Walking into one
    /// is unsupported content.
    Unsupported,
    /// One relic offered out of each group this run is eligible for, shuffled;
    /// then three of them, or two and the keepsake.
    AncientRelics {
        groups: Vec<AncientRelicGroup>,
        keepsake: ModelId,
        /// The cards the keepsake picks between as it is set up, in pool
        /// order. Empty for a keepsake that sets nothing up.
        #[serde(default)]
        keepsake_cards: Vec<ModelId>,
    },
    /// Vakuu's page: each pool is shuffled and the first relic of each is
    /// offered.
    ShuffledRelicPools { pools: Vec<Vec<ModelId>> },
    /// The Colorful Philosophers' page: a list laid out in the build's own
    /// order, cut down to at most `keep` by removing one entry at a rolled
    /// index at a time. Each removal reads one value, so a list already short
    /// enough reads nothing.
    ///
    /// The options are the ones this baseline's unlock state leaves standing:
    /// every character card pool but the one the run is playing.
    RemovedDownTo {
        options: Vec<EventOption>,
        keep: usize,
    },
    /// Tezcatara's page: one relic rolled evenly out of each pool, over the
    /// entries this run qualifies for. A roll is not a shuffle: it reads one
    /// value per pool rather than a whole permutation.
    RolledRelicPools { pools: Vec<Vec<ConditionalRelic>> },
    /// The Architect's page: the ancient's dialogues are cut down to the ones
    /// valid for the character and one of what is left is rolled, and the page
    /// then walks that dialogue's lines one option at a time before the option
    /// that ends the run.
    ///
    /// Each entry is one candidate dialogue, already laid out as the first
    /// page it puts up; the pages after it hang off that page's own
    /// [`EventEffect::SetPage`], each line replacing the page with the next
    /// line's option. The roll is spent even over a single candidate.
    AncientDialogue { dialogues: Vec<Vec<EventOption>> },
    /// Nonupeipe's page: one list, with the entries this run qualifies for
    /// added to the end of it, shuffled whole, and the first few offered.
    ShuffledRelicPool {
        relics: Vec<ConditionalRelic>,
        offered: usize,
    },
    /// The Slippery Bridge's page: a card of the deck is rolled as the page
    /// is built and the page offers letting it go or holding on.
    ///
    /// The first roll is over the deck's non-basic removable cards; every roll
    /// after it drops the card already offered, everything of that card's own
    /// model, and everything a hold-on has passed over, and falls back to the
    /// whole removable deck where that leaves nothing. Holding on costs three
    /// and one more for every hold before it, and the page it puts up names the
    /// hold it is on — `HOLD_ON_0` through `HOLD_ON_6`, and `HOLD_ON_LOOP`
    /// from the seventh on.
    SlipperyBridge,
    /// The Future of Potions' page: one option for each of the first three
    /// potions the player is holding. Which card type each would
    /// trade for is rolled as the page is built, over every held potion in
    /// order, so a page built for three potions reads three dice.
    PotionTrade { pool: Vec<PoolCard> },
    /// Orobas' page: one relic out of each of three pools.
    ///
    /// The first pool gains a fourth entry before it is rolled — the prismatic
    /// gem on a roll under a third, and otherwise the sea glass of a character
    /// the run is not playing, which is itself rolled first. The third pool
    /// holds only the relics the player can actually be set up for, and offers
    /// a locked option when it holds none.
    OrobasRelics {
        pool_one: Vec<ModelId>,
        prismatic: ModelId,
        sea_glass: ModelId,
        pool_two: Vec<ModelId>,
        pool_three: Vec<ConditionalRelic>,
        locked_option_id: String,
    },
    /// The relic trader's two lists, both settled before the page is built:
    /// the tradable relics the player holds, sorted and then shuffled on the
    /// event's own stream and cut to three; and three relics pulled off the
    /// front of the player's own bag, each on a rarity rolled on the rewards
    /// stream. The page lays out one option per pair — `TOP`, `MIDDLE`,
    /// `BOTTOM` — and a lone `PROCEED` where there is no pair at all.
    RelicTrader,
    /// Welcome to Wongos' page: a rare relic a merchant would be allowed to
    /// stock, pulled off the front of the bag as the page is
    /// built, and three prices the purse is measured against, then a fourth
    /// option that leaves and takes an upgrade back off a rolled deck card.
    Wongos {
        bargain_bin: i32,
        featured: i32,
        mystery_box: i32,
        /// The relic the mystery box hands over rather than pulling.
        ticket: ModelId,
    },
    /// The endless conveyor's dish roll and the page it builds around
    /// whatever came up. Every fifth roll is the forced dish and reads no die;
    /// every other
    /// roll is a weighted pick over the dishes this run qualifies for, less
    /// whatever was on the belt last.
    EndlessConveyor {
        /// What a grab costs, unless the dish is free.
        cost: i32,
        dishes: Vec<ConveyorDish>,
        /// Every nth grab, counted from the first, takes the forced dish
        /// instead of a rolled one.
        forced_every: usize,
        forced: ConveyorDish,
    },
    /// Pael's page: one relic out of each of three pools, where the second
    /// and third gain the entries this run qualifies for before they are
    /// rolled.
    ///
    /// The second pool has every one of its entries listed twice before its
    /// last entry is added, so that entry is worth half of any of the
    /// others.
    PaelRelics {
        pool_one: Vec<ModelId>,
        pool_two: Vec<ConditionalRelic>,
        doubled_last: ModelId,
        pool_three: Vec<ConditionalRelic>,
    },
}

/// One option of a page that locks options the run cannot take.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConditionalOption {
    pub available: EventCondition,
    pub option: EventOption,
    /// What the page shows in its place: an option with no effects, which
    /// cannot be taken.
    pub locked_option_id: String,
}

/// One card of a pool an event generates from, with what a filter reads off it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PoolCard {
    pub model_id: ModelId,
    pub card_type: crate::CardType,
    pub rarity: crate::CardRarity,
    /// The cost the card is defined with, before any upgrade. An X-cost
    /// card's canonical cost is nothing at all.
    #[serde(default)]
    pub energy_cost: i32,
    /// Whether the card's cost is X rather than a number.
    #[serde(default)]
    pub costs_x: bool,
    /// Whether the card may be generated during a combat. A card that answers
    /// no is not among what a combat generation effect rolls; it can still be
    /// offered, bought, and taken as a reward.
    #[serde(default)]
    pub can_be_generated_in_combat: bool,
    /// Whether a modifier may generate the card, which defaults to yes. Eight
    /// of the eighteen curses answer no, and it is what an effect drawing a
    /// curse out of the curse pool filters on.
    #[serde(default = "crate::model::yes")]
    pub can_be_generated_by_modifiers: bool,
}

/// A card pool, as the registry holds one.
///
/// A pool settles two things a run reads: which cards stand in it, in the
/// order they are listed, and whether the pool is a colourless one. Everything
/// else a pool carries — a title, a frame, a card back — is presentation. The
/// membership is what a draw indexes into and what a card's own pool is worked
/// out from, so both the list and its order are fidelity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardPoolDefinition {
    pub model_id: ModelId,
    /// Whether the pool is a colourless one: false for a pool that belongs to
    /// one character, and for the curse, quest, and status families; true for
    /// the colourless, event, token, and deprecated ones.
    pub is_colorless: bool,
    /// The pool's unlocked single-player cards as this baseline reads them:
    /// every epoch is revealed, so the epoch filter prunes nothing, and the
    /// multiplayer-only cards are already left out.
    pub cards: Vec<PoolCard>,
}

/// One dish an endless conveyor can put on its belt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConveyorDish {
    /// The name the option is keyed under, which is the dish's own id.
    pub id: String,
    /// The weights are fractional in the build and every one of them is a
    /// whole number, so the sum and the threshold the roll is measured against
    /// are exact either way.
    pub weight: i32,
    /// The test made of the run before the dish joins the list.
    pub available: EventCondition,
    /// How many grabs have to have happened before the dish joins the list at
    /// all: the golden fysh only joins it from the second roll on.
    #[serde(default)]
    pub after_grabs: usize,
    /// Whether the dish costs nothing. The golden fysh is the one dish a grab
    /// does not pay for.
    #[serde(default)]
    pub free: bool,
    pub effects: Vec<EventEffect>,
}

/// One relic an ancient offers only when the run can be set up for it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConditionalRelic {
    pub relic: ModelId,
    /// Whether the relic can set itself up for this player, said as a test of
    /// the run.
    pub available: EventCondition,
}

/// What an amount on an event's page is worth: either written down, or rolled
/// on the event's own RNG as the room is entered.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EventAmount {
    Fixed(i32),
    /// One less than the player's maximum HP, which is what the Tablet of
    /// Truth asks for on the last decipher and what it falls back to when the
    /// cost is more than the player has.
    MaxHpLessOne,
    /// A fraction of the player's own maximum, worked out where it is read
    /// and truncated to a whole number, which is what a pool that heals a
    /// third of what its drinker has pays out.
    MaxHpFraction {
        numerator: i32,
        denominator: i32,
    },
    /// Rolled once, inclusive of `min` and exclusive of `max`.
    Rolled {
        min: i32,
        max: i32,
    },
    /// A base the event declares, shifted up by a whole number rolled over
    /// `min..max` as the room settles its vars, which is what a lost wisp's
    /// gold is.
    RolledOnto {
        base: i32,
        min: i32,
        max: i32,
    },
    /// A base the event declares, shifted by a fractional amount rolled over
    /// `-spread..spread` as the room settles its vars. What lands is the
    /// whole-number part: the gold gain truncates towards zero.
    RolledAround {
        base: i32,
        spread: i32,
    },
    /// A base the event declares, with a whole number rolled over `min..max`
    /// taken *off* it, which is what a luminous choir discounts its tribute
    /// by. The draw is the same one and over the same range as
    /// [`Self::RolledOnto`]; only the sign of what is done with the answer
    /// differs.
    RolledOffOf {
        base: i32,
        min: i32,
        max: i32,
    },
    /// Everything the player is missing off their maximum, which is what an
    /// unrest site settles onto its heal as the room is entered.
    MissingHp,
    /// One of the event's own settled vars, by the index it stands at in
    /// [`EventDefinition::vars`]. Generation replaces it with what the room
    /// settled.
    Var(usize),
}

impl EventAmount {
    #[must_use]
    pub const fn value(self) -> i32 {
        match self {
            Self::Fixed(amount) => amount,
            // An unrolled amount is worth nothing until the room rolls it, and
            // one read off the player is worth nothing without the player.
            Self::MaxHpLessOne
            | Self::MaxHpFraction { .. }
            | Self::MissingHp
            | Self::Rolled { .. }
            | Self::RolledOnto { .. }
            | Self::RolledOffOf { .. }
            | Self::RolledAround { .. }
            | Self::Var(_) => 0,
        }
    }
}

/// Relics that share one slot in an ancient's offer.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AncientRelicGroup {
    pub relics: Vec<ModelId>,
    /// Some groups belong to one act only.
    pub only_in_act: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EventOption {
    pub option_id: String,
    pub label: String,
    /// What choosing the option does, in order.
    pub effects: Vec<EventEffect>,
    /// An option that only leaves the event. No event walked by the registered
    /// fixture has one.
    pub is_proceed: bool,
    /// An option that has already been taken. Every option of the pinned
    /// build disables itself once chosen, so an option clicked twice runs
    /// once and the second click does nothing at all. It is state rather than
    /// definition: a page put up in place of this one carries fresh
    /// options.
    #[serde(default)]
    pub was_chosen: bool,
    /// An option the page shows but the player cannot take: the reading a
    /// deck with no card of the right type is shown in place of, the relic a
    /// purse cannot pay for. It has no body at all, and it is on the page so
    /// that the page reads as the build lays it out, not so that it can be
    /// chosen.
    #[serde(default)]
    pub locked: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GeneratedRewardSpec {
    pub common_cards: Vec<ModelId>,
    pub uncommon_cards: Vec<ModelId>,
    pub rare_cards: Vec<ModelId>,
    pub common_potions: Vec<ModelId>,
    pub uncommon_potions: Vec<ModelId>,
    pub rare_potions: Vec<ModelId>,
    pub card_option_count: usize,
    /// The pools a card reward drawn against this spec names as its source.
    pub card_pools: Vec<ModelId>,
}

/// What a combat room pays out, less the card and potion pools: those belong
/// to the player, not to the encounter. A card reward generated for a room
/// draws out of the player's character pool and takes only the rarity odds
/// from the room, so an encounter carries the room's half and the character's
/// own spec is looked up as the rewards are generated.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GeneratedCombatRewardSpec {
    pub minimum_gold: i32,
    pub maximum_gold: i32,
    /// Whether the purse above is the room type's own, which is the range a
    /// room type hands its encounters and what Poverty takes three quarters
    /// of.
    ///
    /// An encounter may name its own purse instead, and the one encounter of
    /// the pinned build that does — the fake merchant's — sets its range
    /// outright and takes nothing from the room type. Such a purse is paid in
    /// full at every ascension, and this is the flag that says which kind a
    /// purse is.
    pub scales_with_poverty: bool,
    /// Which base odds this room's card reward rolls with.
    pub rarity_odds: crate::RarityOdds,
    /// Elites also hand over a relic.
    pub offers_relic: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EventEffect {
    GainGold(EventAmount),
    /// HP the option costs outright, which nothing can block.
    LoseHp(EventAmount),
    ObtainRelic(ModelId),
    /// A relic an ancient set up as it offered it, and the card that setup
    /// rolled: a dusty tome picks its card when the page is built, and the
    /// relic carries it from then on.
    ObtainRelicWithCard {
        relic: ModelId,
        card: ModelId,
    },
    /// A sea glass the ancient set a character on as it laid the option out,
    /// and the fifteen cards of that character's
    /// pool it deals as it is taken — five commons, five uncommons and five
    /// rares on one grid, and whatever is picked off it joins the deck.
    ObtainSeaGlass {
        relic: ModelId,
        character: ModelId,
        pool: ModelId,
    },
    /// A relic of a rolled rarity, taken from the front of the player's bag.
    ObtainRolledRelic,
    /// Lays out generated cards and takes however many of them the player picks.
    ChooseGeneratedCards {
        count: usize,
        picks: usize,
        pool: Vec<ModelId>,
    },
    /// The same over a card creation rather than a flat pool: the cards are
    /// rolled the way a reward's are, laid out on a grid, and what is picked
    /// joins the deck. A brain leech sharing what it knows is the one that
    /// does it.
    ChooseCreatedCards {
        creation: crate::CardCreation,
        picks: usize,
    },
    Heal(EventAmount),
    /// Raises the maximum HP and heals by as much.
    GainMaxHp(i32),
    /// Takes the maximum HP down and the current HP with it where it has to.
    /// Unlike [`Self::LoseMaxHpAndUpgrade`] nothing follows it, and a cost the
    /// player cannot pay is floored rather than fatal.
    LoseMaxHp(EventAmount),
    /// The Tablet of Truth's decipher: the maximum comes down by this much,
    /// and then one upgradable card of the deck — rolled on the event's own
    /// RNG — is upgraded, or on the last decipher every one of them.
    ///
    /// A cost the player cannot pay is the other half of the same effect: the
    /// maximum goes to one, the player is killed, and the upgrade never
    /// happens because nothing after the death runs.
    LoseMaxHpAndUpgrade {
        amount: EventAmount,
        /// The fourth decipher, which upgrades the whole deck rather than one
        /// rolled card and reads no dice doing it.
        #[serde(default)]
        whole_deck: bool,
    },
    /// The Future of Potions' trade: the potion in this slot goes, and three
    /// cards of one rolled type and the rarity the potion was worth are
    /// generated, upgraded, and offered as a reward.
    TradePotionForUpgradedCards {
        slot: usize,
        rarity: crate::CardRarity,
        card_type: crate::CardType,
        pool: Vec<PoolCard>,
    },
    /// Gold the option costs.
    LoseGold(EventAmount),
    /// A Zen weaver's bargain: a removal screen for this many deck cards, and
    /// the gold paid once they are gone.
    RemoveDeckCardsForGold {
        count: usize,
        cost: i32,
    },
    /// Removes the tradable relic an event rolled as the room was entered.
    /// Generation replaces it with the relic the roll settled on.
    RemoveRolledTradableRelic,
    RemoveRelic(ModelId),
    /// Discards the potion an event rolled as the room was entered.
    /// Generation replaces it with the slot the roll settled on.
    DiscardRolledPotion,
    DiscardPotion {
        slot: usize,
    },
    /// The page is replaced with another set of options, and the room stays
    /// open for them.
    SetPage(Vec<EventOption>),
    /// The same over a random few: the options are shuffled whole on the
    /// event's own stream and the first few are put up. The shuffle is the
    /// unstable one, so the order the options were written in is what it
    /// starts from.
    SetShuffledPage {
        options: Vec<EventOption>,
        take: usize,
    },
    /// The same, over a page rolled on the event's own RNG. Accepting a
    /// Trial rolls one of three pages.
    SetRolledPage(Vec<Vec<EventOption>>),
    /// An option that is a run of commands over the shared surface, in order.
    /// Most event options are exactly that — a few ordinary commands — and the
    /// named effects here are the ones that need something the command surface
    /// does not carry.
    Run(Vec<crate::Command>),
    AddCard(CardFingerprint),
    /// One card generated the way a reward's is and put straight into the
    /// deck: a rarity rolled on the base odds, bumped to the next rarity the
    /// pool stocks, one card taken evenly out of that rarity, and an upgrade
    /// rolled for it. The pool is the character's, cut to what the page's
    /// filter allows.
    AddGeneratedCard {
        pool: Vec<PoolCard>,
    },
    ObtainRelicAndRandomCard {
        relic: ModelId,
        candidates: Vec<ModelId>,
    },
    /// One relic of a list the event names, rolled on the event's own RNG
    /// rather than pulled from a bag.
    ObtainRelicRolledFrom {
        candidates: Vec<ModelId>,
    },
    /// One card of a list the event names, rolled the same way and added to
    /// the deck.
    AddCardRolledFrom {
        candidates: Vec<ModelId>,
    },
    ObtainRelicAndGeneratedRewards {
        relic: ModelId,
        spec: GeneratedRewardSpec,
    },
    Rewards(Vec<RewardFingerprint>),
    /// The card the page rolled leaves the deck.
    ///
    /// `card` is that card as the page shows it, written on when the roll
    /// settles. It names the option rather than resolving it: the roll is kept
    /// on the room as a deck index and that index is what the removal reads,
    /// because a deck can hold two cards of one fingerprint. A page that rolled
    /// nothing names nothing.
    RemoveTheRolledCard {
        card: Option<CardFingerprint>,
    },
    /// The price unless the dish is free, the dish's own body, then the next
    /// dish rolled and the page put back up around it. Which dish is on the
    /// belt and how many grabs have happened are carried here because the
    /// option keys are the dish's rather than the page's.
    ///
    /// `body` and `cost` are what taking it does and what it charges, written
    /// on as the page is built — the dish the belt is showing, spelled out
    /// rather than left as an index into a list only the registry holds. They
    /// are what the grab pays and applies; the index is kept because the next
    /// roll has to know which dish it is rolling away from.
    GrabOffTheBelt {
        dish: usize,
        grabs: usize,
        #[serde(default)]
        body: Vec<EventEffect>,
        #[serde(default)]
        cost: i32,
    },
    /// One upgradable card of the deck, rolled evenly on the event's own
    /// stream, with the upgrade worked onto whichever it lands on. A deck with
    /// nothing upgradable in it is checked first and reads no die.
    UpgradeARolledCard,
    /// What a battleworn dummy pays out only where the fight was won rather
    /// than timed out. The dummy's timer marks the encounter as having run out
    /// of time as it walks out of the fight, so the guard here is whether
    /// anything escaped it.
    UnlessTheFightTimedOut(Vec<EventEffect>),
    /// This many upgradable cards of the deck, upgraded one at a time: the
    /// deck's upgradable cards sorted, then stably shuffled on the event's own
    /// stream, and the first few taken.
    UpgradeShuffledDeckCards {
        count: usize,
    },
    /// A relic trader's trade: the relic the page put in this slot leaves the
    /// player, and the one it was paired with is obtained where it stood.
    TradeRelic {
        given: ModelId,
        taken: ModelId,
    },
    /// A Wongos purchase, bargain bin or featured item: the price, then a
    /// relic of this rarity pulled off the front of the bag over the relics a
    /// merchant would be allowed to stock. A featured item was already pulled
    /// as the page was built, and names itself instead.
    BuyFromTheBag {
        cost: i32,
        rarity: RelicRarity,
    },
    /// The price of leaving Wongos: one upgraded card of the deck, rolled on
    /// the event's own stream, has its upgrade taken back off. A deck with
    /// nothing upgraded in it reads no die, the way an even roll over an empty
    /// list does not.
    DowngradeARolledCard,
    /// The crystal sphere's board, uncovered outright or on a payment plan:
    /// the eleven-by-eleven fog board is stood up and the player is given this
    /// many divinations to spend on it. The page stays standing under the
    /// board, and the event finishes only once the rewards the board pays out
    /// are done with.
    PlayTheCrystalSphere {
        divinations: i32,
    },
    /// The damage the page advertises, unblockable and unpowered, then another
    /// card rolled and the next page put up.
    ///
    /// `cost` is that advertised number — three for the first hold and one more
    /// for every hold before it. Written on the option so that reading what a
    /// hold costs is reading the page, rather than recovering the hold count by
    /// parsing an option id.
    HoldOnToTheBridge {
        cost: i32,
    },
    /// One immersion in the abyssal baths: two maximum HP, then the damage
    /// the bath is standing at — unblockable and unpowered — and the page put
    /// back up with the next one written on it. The damage is carried on the
    /// option rather than counted, because the bath grows its own number while
    /// the option keys never change: every page after the first lays out the
    /// same linger-or-leave pair.
    ImmerseInTheBaths {
        damage: i32,
    },
    /// A doll room's offer: the relics are sorted and then stably shuffled on
    /// the event's own stream, and the first few are laid out as a page of one
    /// option each.
    ///
    /// An option built this way is keyed by the relic's own displayed name
    /// rather than by a page key, which is why the name travels with the relic
    /// here instead of being spelled out of the model ID.
    OfferShuffledRelics {
        relics: Vec<RelicOnOffer>,
        take: usize,
    },
    /// One potion rolled out of a list on the player's rewards stream and
    /// offered as a reward. An empty list rolls nothing and offers nothing at
    /// all.
    OfferRolledPotion {
        candidates: Vec<ModelId>,
    },
    /// A run of card rewards offered together, each one generated out of the
    /// character's own pool on the default out-of-combat odds. Nothing else is
    /// offered with them.
    OfferCardRewards {
        count: usize,
        spec: Box<GeneratedRewardSpec>,
    },
    /// One card reward per creation the event names, offered together in that
    /// order. It is what an event that offers a card of each
    /// rarity out of one pool builds: three creations over the same pool, each
    /// filtered to a rarity of its own.
    OfferCreatedCardRewards {
        creations: Vec<crate::CardCreation>,
    },
    /// A fight pushed on top of the event's own room without leaving the
    /// event. The event's page stays standing under it, the
    /// fight's rewards carry whatever the option handed over as well, and the
    /// map point below it counts one room more.
    EnterCombatWithoutLeaving {
        encounter: ModelId,
        /// Extra rewards added to the fight's room, offered with its own.
        extra_rewards: Vec<RewardFingerprint>,
        /// Whether proceeding past the fight goes back to the event.
        resumes_parent: bool,
        /// A setting the event wrote on the encounter before standing it up,
        /// as an index into the encounter's own
        /// [`crate::MonsterGeneration::ChosenByTheEvent`] list.
        #[serde(default)]
        chosen_monster: Option<usize>,
        /// What the event picks back up with once the fight is over, which
        /// only a fight that resumes its parent ever reaches.
        #[serde(default)]
        resume: Vec<EventEffect>,
    },
    /// A read of the event's own stream whose answer is thrown away: one draw
    /// over `n` at the end of two of the Stone of All Time's options. The dice
    /// move and nothing acts on them.
    ReadTheEventStream(i32),
    Unsupported(ModelId),
    Proceed,
    /// The Architect ends the run: the run is over and won, and the act
    /// change behind it carries the win through.
    WinRun,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ShopOffer {
    pub index: usize,
    pub item: ShopItem,
    /// Which kind of shelf entry this is, which is what a restock needs: a
    /// card slot remembers the card type or the rarity it was built for, and a
    /// relic or potion slot refills from its own generator.
    #[serde(default)]
    pub slot: ShopSlot,
    /// What the entry priced itself at, before anything that modifies prices
    /// has had a say.
    #[serde(default)]
    pub base_price: i32,
    /// The base price worked through everything that modifies merchant prices
    /// and truncated. The build works it out afresh on every read; here it is
    /// worked out again at every boundary, which is what a relic bought off
    /// this very shelf needs.
    pub price: i32,
    pub sold: bool,
}

/// What a shelf entry refills itself as. A card entry is built either for a
/// card type, off the character's pool, or for a rarity, off the colourless
/// one; the other entries are built without an argument.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShopSlot {
    #[default]
    CharacterCard,
    /// A slot that refills from the character's pool with a card of this type.
    CharacterCardOfType(crate::CardType),
    ColorlessCard(crate::CardRarity),
    Relic,
    Potion,
}

/// The crystal sphere's minigame: an eleven-by-eleven board under fog,
/// fifteen items hidden on it, and a count of divinations to spend clearing
/// the fog with.
///
/// The board is built with its four corners cleared and grown twice, the items
/// are placed on what is left, and every divination clears a patch. An item
/// whose every cell is clear is revealed, and what the revealed items are worth
/// is offered as one rewards screen when the last divination is spent.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CrystalSphereGame {
    /// The board, row-major by `x` — `cells[x * WIDTH + y]` — over the
    /// eleven-by-eleven grid the board is laid out on.
    pub cells: Vec<CrystalSphereCell>,
    /// The items in the order the board placed them.
    pub items: Vec<CrystalSphereItem>,
    /// Divinations left to spend; the minigame is finished at nothing.
    pub divinations: i32,
    /// The items revealed so far, named by where they stand in `items`, in the
    /// order they were revealed.
    pub revealed: Vec<usize>,
    /// Whether the last of the ten placement passes seated every item. The
    /// board is kept and played either way.
    pub placed_all_items: bool,
}

impl CrystalSphereGame {
    /// The board is always eleven cells on a side.
    pub const WIDTH: i32 = 11;
    pub const HEIGHT: i32 = 11;

    #[must_use]
    pub fn cell(&self, x: i32, y: i32) -> Option<&CrystalSphereCell> {
        self.cells.get(Self::index(x, y)?)
    }

    #[must_use]
    pub fn index(x: i32, y: i32) -> Option<usize> {
        if !(0..Self::WIDTH).contains(&x) || !(0..Self::HEIGHT).contains(&y) {
            return None;
        }
        usize::try_from(x * Self::HEIGHT + y).ok()
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct CrystalSphereCell {
    pub hidden: bool,
    /// Where the item standing on this cell is in the board's item list.
    pub item: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CrystalSphereItem {
    pub kind: CrystalSphereItemKind,
    pub x: i32,
    pub y: i32,
}

impl CrystalSphereItem {
    /// How many cells the item covers, as a width and a height.
    #[must_use]
    pub const fn size(&self) -> (i32, i32) {
        match self.kind {
            CrystalSphereItemKind::Relic => (4, 4),
            CrystalSphereItemKind::Potion(crate::PotionRarity::Rare)
            | CrystalSphereItemKind::CardReward(_)
            | CrystalSphereItemKind::Curse => (2, 2),
            CrystalSphereItemKind::Potion(_) => (1, 3),
            CrystalSphereItemKind::Gold { big: true } => (2, 1),
            CrystalSphereItemKind::Gold { big: false } => (1, 1),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrystalSphereItemKind {
    /// A relic off the player's own bag.
    Relic,
    /// One potion of this rarity, rolled on the event's own RNG out of the
    /// potions of that rarity.
    Potion(crate::PotionRarity),
    /// Three cards of this rarity out of the character's pool, rolled evenly.
    CardReward(crate::CardRarity),
    /// A Doubt, straight into the deck as it is uncovered rather than offered.
    Curse,
    /// Ten gold, or thirty for the wide one.
    Gold { big: bool },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ShopItem {
    Card(CardFingerprint),
    Potion(ModelId),
    Relic(ModelId),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RoomDefinition {
    Event {
        model_id: ModelId,
        options: Vec<EventOption>,
        /// Where the event's own RNG stands once its page has been built.
        ///
        /// An event has exactly one stream, opened as the event is set up and
        /// read from then on, so a page that rolled as it was built leaves its
        /// counter where a chosen option picks it up.
        #[serde(default)]
        rng_counter: u64,
        /// What an event that stocks a shelf of its own put on it. Projection
        /// v1 describes a merchant room's inventories and nothing else, so a
        /// shelf an event owns is state this simulator keeps and the recording
        /// cannot see.
        #[serde(default)]
        shelf: Vec<ShopOffer>,
        /// The fight a combat-layout event already stood up as its room was
        /// entered, creatures and rolled health and all. The option that walks
        /// into the fight stands these up rather than making new ones.
        #[serde(default)]
        standing_fight: Option<Box<CombatRoomDefinition>>,
        /// The deck card the page rolled as it was built, where the event rolls
        /// one.
        #[serde(default)]
        rolled_card: Option<usize>,
    },
    Combat(CombatRoomDefinition),
    /// A merchant. Merchants have no room model of their own, so an event that
    /// stands one up names itself here instead.
    Shop {
        model_id: Option<ModelId>,
        offers: Vec<ShopOffer>,
        removal_price: i32,
    },
    RestSite,
    Treasure {
        /// The relics the chest already holds, which is how a scenario places
        /// one.
        relics: Vec<ModelId>,
        /// Whether the chest is the run's own and still to be filled. A
        /// treasure room fills itself as part of being entered, after
        /// everything that reacts to a room being entered has run, so a relic
        /// that suppresses the chest has counted the room it is standing in
        /// before the chest asks whether to generate anything.
        #[serde(default)]
        fills_itself: bool,
    },
    Unsupported {
        model_id: ModelId,
    },
}

impl RoomDefinition {
    #[must_use]
    pub fn model_id(&self) -> Option<&ModelId> {
        match self {
            Self::Event { model_id, .. } | Self::Unsupported { model_id } => Some(model_id),
            Self::Shop { model_id, .. } => model_id.as_ref(),
            Self::Combat(definition) => Some(&definition.encounter_model),
            Self::RestSite | Self::Treasure { .. } => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ActiveRoom {
    Event {
        model_id: ModelId,
        options: Vec<EventOption>,
        /// Where the event's own RNG stands, carried from the page that was
        /// built as the room was entered and advanced by every chosen option
        /// that reads it.
        #[serde(default)]
        rng_counter: u64,
        /// The shelf an event that owns one stocked as the room was entered.
        #[serde(default)]
        shelf: Vec<ShopOffer>,
        /// The fight a combat-layout event stood up as the room was entered.
        #[serde(default)]
        standing_fight: Option<Box<CombatRoomDefinition>>,
        /// The card the Slippery Bridge rolled and the cards it has already
        /// passed over: the deck card the page is offering to take, and every
        /// card a hold-on has skipped. Both are named by where the card stands
        /// in the deck, and nothing this event does moves a card until the
        /// removal itself.
        #[serde(default)]
        rolled_card: Option<usize>,
        #[serde(default)]
        skipped_cards: Vec<usize>,
        /// The crystal sphere board an option of this page stood up, while it
        /// is still being played.
        #[serde(default)]
        sphere: Option<Box<CrystalSphereGame>>,
    },
    Shop {
        model_id: Option<ModelId>,
        offers: Vec<ShopOffer>,
        /// The removal entry's undiscounted price. Everything that modifies
        /// merchant prices is read live, so the displayed price is rebuilt
        /// from this value at every boundary just like each shelf entry is
        /// rebuilt from its `base_price`.
        #[serde(default)]
        removal_base_price: Option<i32>,
        removal_price: i32,
        removal_used: bool,
    },
    RestSite {
        /// The options the rest site generated as the room was entered, in
        /// their order.
        options: Vec<RestSiteOptionState>,
        /// Whether an option has been taken successfully, which is what enables
        /// the proceed button: a rest site cannot be walked out of until one
        /// has, and a failed option leaves it disabled.
        left_open: bool,
    },
    Treasure {
        relics: Vec<ModelId>,
    },
}

/// One entry of the list a rest site generates when it is entered.
///
/// The build takes the entry that was taken off the list, or clears the whole
/// list where something the player holds disables the rest of it. The list is
/// kept whole here with the taken entries marked instead, because the screen's
/// own indices are what the recording projects and what an action names, and
/// because a removal would renumber the entries behind it.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RestSiteOptionState {
    pub option: RestSiteOption,
    /// Whether the option is still on the list.
    pub standing: bool,
}

/// One rest-site option of the pinned build, named by the id the screen keys
/// it under.
///
/// Which of them a rest site offers is content data — the base two plus
/// whatever the player is holding adds — but what each one does is behaviour
/// and is spelled out here.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestSiteOption {
    /// Rest, always the first entry.
    Heal,
    /// Smith, always the second.
    Smith,
    /// Cook, which Meat Cleaver adds.
    Cook,
    /// Clone, which Pael's Growth adds.
    Clone,
    /// Dig, which the Shovel adds.
    Dig,
    /// Hatch, which a Byrdonis Egg in the deck adds.
    Hatch,
    /// Kindle, which the Pumpkin Candle adds.
    Kindle,
    /// Lift, which Girya adds while it has lifts left.
    Lift,
}

impl RestSiteOption {
    /// The id the option is keyed under, which is what the recording names an
    /// option by and what the run's history writes down.
    #[must_use]
    pub const fn option_id(self) -> &'static str {
        match self {
            Self::Heal => "HEAL",
            Self::Smith => "SMITH",
            Self::Cook => "COOK",
            Self::Clone => "CLONE",
            Self::Dig => "DIG",
            Self::Hatch => "HATCH",
            Self::Kindle => "KINDLE",
            Self::Lift => "LIFT",
        }
    }
}
