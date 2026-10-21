use std::collections::{BTreeMap, VecDeque};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sts2_core::{CardInstanceId, ChoiceId, CreatureCombatId, ModelId, OrbInstanceId, RewardSetId};
use sts2_rng::{PlayerRngSet, RunRngSet, SerializablePlayerRngSet, SerializableRunRngSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConformanceOrigin {
    Generation,
    PortableContinuation,
    Scenario,
    /// A possible world sampled from player-visible state by a belief model.
    /// Its gameplay is analysis, never fidelity evidence: nothing carrying
    /// this origin may be compared against a recording.
    Analysis,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSide {
    Player,
    Enemy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardType {
    Attack,
    Skill,
    Power,
    Status,
    Curse,
    /// A quest card: a run-long errand a deck carries, which is never played.
    Quest,
}

/// The rarities a card can have, in the build's own order, which is what a
/// screen sorting by rarity puts its cards in.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardRarity {
    // Generation rolls these three, in this order, and prices them by it.
    Common,
    Uncommon,
    Rare,
    /// What a character starts the run holding.
    Basic,
    /// What a relic or an event hands over outside the rolled rarities.
    Ancient,
    /// What only an event hands over. Generation never rolls it, and a
    /// generation that has to pick a card for a fight drops it.
    Event,
    /// What a card conjured mid-fight is: a token exists for the combat that
    /// made it and is never offered as a reward.
    Token,
    /// What an enemy forces into a deck.
    Status,
    /// What a curse card is, which is a rarity of its own and sorts behind
    /// every other card.
    Curse,
    /// What a quest card is: a run-long errand a card carries, which sorts
    /// behind even a curse.
    Quest,
}

impl CardRarity {
    /// Where this rarity stands in the build's own rarity order, which is the
    /// order a screen sorting by rarity puts its cards in. It is not the order
    /// generation walks, which is why it is a key of its own rather than the
    /// derived one.
    #[must_use]
    pub const fn sort_key(self) -> u8 {
        match self {
            Self::Basic => 1,
            Self::Common => 2,
            Self::Uncommon => 3,
            Self::Rare => 4,
            Self::Ancient => 5,
            Self::Event => 6,
            Self::Token => 7,
            Self::Status => 8,
            Self::Curse => 9,
            Self::Quest => 10,
        }
    }

    /// The wrap-around order generation walks when a rolled rarity has no
    /// card of the requested type.
    ///
    /// The rarities generation never rolls step to no rarity at all, which
    /// stocks no pool, so the walk ends there rather than continuing into a
    /// rarity it was never asked for.
    #[must_use]
    pub const fn next_highest(self) -> Option<Self> {
        match self {
            // Basic and Rare both fall to Common, from opposite ends of the
            // table: Basic climbs into the rolled rarities, Rare wraps.
            Self::Basic | Self::Rare => Some(Self::Common),
            Self::Common => Some(Self::Uncommon),
            Self::Uncommon => Some(Self::Rare),
            Self::Ancient
            | Self::Event
            | Self::Token
            | Self::Status
            | Self::Curse
            | Self::Quest => None,
        }
    }
}

/// Which base odds a card generation site rolls with.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RarityOdds {
    /// Out-of-combat uniform odds: every card the filter leaves is as likely
    /// as any other, and no rarity is rolled at all.
    Uniform,
    RegularEncounter,
    EliteEncounter,
    BossEncounter,
    Shop,
}

impl RarityOdds {
    /// The base odds, for the two rarities a roll reads: the rare threshold
    /// and the uncommon one above it.
    ///
    /// Scarcity halves the rare odds of every site that rolls one. It also
    /// raises the common odds of three of them, and nothing ever reads those:
    /// every rarity roll asks for the rare and uncommon thresholds alone and
    /// lets common be the remainder, so the ascended common numbers are
    /// written down beside the odds rather than in them.
    #[must_use]
    pub fn base(self, ascension: u8) -> (f32, f32) {
        let rare = |ascended: f32, baseline: f32| {
            crate::float_if_ascension(
                ascension,
                crate::AscensionLevel::Scarcity,
                ascended,
                baseline,
            )
        };
        match self {
            // Uniform odds roll no rarity; the pair is what a caller that asks
            // anyway would read, and nothing does.
            Self::Uniform => (0.0, 0.0),
            // The regular common odds are 0.615 under Scarcity, 0.6 below it.
            Self::RegularEncounter => (rare(0.0149, 0.03), 0.37),
            // The elite common odds are 0.549 under Scarcity, 0.5 below it.
            Self::EliteEncounter => (rare(0.05, 0.1), 0.4),
            Self::BossEncounter => (1.0, 0.0),
            // The shop common odds are 0.585 under Scarcity, 0.54 below it.
            Self::Shop => (rare(0.045, 0.09), 0.37),
        }
    }
}

/// Player-scoped odds that drift as a run generates rewards.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlayerOdds {
    /// Added to the rare threshold of every card rarity roll.
    pub card_rarity_offset_bits: u32,
    /// Chance that a won combat also offers a potion.
    pub potion_reward_bits: u32,
}

impl PlayerOdds {
    pub const CARD_RARITY_START: f32 = -0.05;
    pub const CARD_RARITY_MAX: f32 = 0.4;
    pub const POTION_REWARD_START: f32 = 0.4;
    pub const POTION_REWARD_STEP: f32 = 0.1;

    #[must_use]
    pub const fn start_of_run() -> Self {
        Self {
            card_rarity_offset_bits: Self::CARD_RARITY_START.to_bits(),
            potion_reward_bits: Self::POTION_REWARD_START.to_bits(),
        }
    }

    #[must_use]
    pub const fn card_rarity_offset(self) -> f32 {
        f32::from_bits(self.card_rarity_offset_bits)
    }

    #[must_use]
    pub const fn potion_reward(self) -> f32 {
        f32::from_bits(self.potion_reward_bits)
    }

    /// How far a roll that came up short pushes the next one, which Scarcity
    /// halves.
    #[must_use]
    pub fn card_rarity_growth(ascension: u8) -> f32 {
        crate::float_if_ascension(ascension, crate::AscensionLevel::Scarcity, 0.005, 0.01)
    }

    /// A rare result resets the offset; anything else nudges it up toward the
    /// cap.
    pub fn record_card_rarity(&mut self, rolled: CardRarity, ascension: u8) {
        let offset = if rolled == CardRarity::Rare {
            Self::CARD_RARITY_START
        } else {
            (self.card_rarity_offset() + Self::card_rarity_growth(ascension))
                .min(Self::CARD_RARITY_MAX)
        };
        self.card_rarity_offset_bits = offset.to_bits();
    }

    /// The pity system: a dropped potion lowers the next chance, a missed one
    /// raises it.
    pub fn record_potion_reward(&mut self, dropped: bool) {
        let step = if dropped {
            -Self::POTION_REWARD_STEP
        } else {
            Self::POTION_REWARD_STEP
        };
        self.potion_reward_bits = (self.potion_reward() + step).to_bits();
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetType {
    None,
    SelfTarget,
    /// Any player creature, which single-player always resolves to the one
    /// player but which is still named explicitly.
    AnyPlayer,
    AnyEnemy,
    /// Every enemy at once. The build performs no target selection for it.
    AllEnemies,
    /// One enemy the card itself rolls for. The build performs no target
    /// selection for it either.
    RandomEnemy,
    /// The thing aimed at is not a creature. A foul potion outside a fight is
    /// thrown at the merchant, which the belt treats as targeted and the
    /// effect resolves without a creature.
    TargetedNoCreature,
    /// Every ally at once, which single-player resolves to the one player. It
    /// is not a single target, so the build performs no target selection for
    /// it and it is untargeted here for the same reason `AllEnemies` is.
    AllAllies,
    /// One other player. A card that aims this way cannot be played wherever
    /// its owner has no living ally, which in a single-player run is always.
    /// Only the two multiplayer-only cards of the build aim this way.
    AnyAlly,
}

/// What a card's enchantment comes to when the card is written down, which is
/// what a recording projects beside the card.
///
/// The projection is an object rather than a bare model ID —
/// `{ model_id, amount, properties }` — because the amount is the
/// enchantment's own saved state and a second Goopy play is a different card
/// from the first.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EnchantmentFingerprint {
    pub model_id: ModelId,
    /// How much of the enchantment the card carries.
    pub amount: i32,
    /// The saved properties the enchantment declares. No pinned enchantment
    /// declares one, so this is empty for everything registered; it is carried
    /// because the projection carries it.
    #[serde(default)]
    pub properties: BTreeMap<String, Value>,
}

impl EnchantmentFingerprint {
    #[must_use]
    pub fn new(model_id: ModelId, amount: i32) -> Self {
        Self {
            model_id,
            amount,
            properties: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CardFingerprint {
    pub model_id: ModelId,
    pub upgrade_level: u8,
    pub floor_added_to_deck: Option<u32>,
    pub enchantment: Option<EnchantmentFingerprint>,
    pub properties: BTreeMap<String, Value>,
    /// Whether Bing Bong made this exact card object and has not yet seen it
    /// leave and rejoin the deck.
    ///
    /// Bing Bong remembers exactly which cards it has already made. The
    /// marker is not part of the recorded card fingerprint, but it is
    /// future-relevant: a copied card stolen by a Thieving Hopper must not be
    /// copied again when the hopper's reward returns it to the deck.
    #[serde(default)]
    pub bing_bong_skip_once: bool,
}

impl CardFingerprint {
    /// One saved property written onto the card, which is what a card created
    /// with a property already set hands over.
    #[must_use]
    pub fn with_property(mut self, name: &str, value: i32) -> Self {
        self.properties.insert(name.to_owned(), Value::from(value));
        self
    }

    /// A card of a model and nothing else, carrying none of the saved
    /// properties the model may declare.
    ///
    /// A card the engine hands the player comes off `ContentRegistry::
    /// fresh_card`, which writes those properties on. This is what a
    /// definition builds a card with when it is naming the properties
    /// itself.
    #[must_use]
    pub fn base(model_id: ModelId) -> Self {
        Self {
            model_id,
            upgrade_level: 0,
            floor_added_to_deck: None,
            enchantment: None,
            properties: BTreeMap::new(),
            bing_bong_skip_once: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag mirrors one independent property of the pinned card instance"
)]
pub struct CardInstance {
    pub id: CardInstanceId,
    pub fingerprint: CardFingerprint,
    pub energy_cost: i32,
    pub star_cost: i32,
    pub costs_x: bool,
    /// What the last play of this card spent on energy, which is what the
    /// card reads for X.
    #[serde(default)]
    pub captured_x: i32,
    /// How many stars the last play of this card paid. It is written for every
    /// play rather than only for an X-cost one, and it is what the X of a card
    /// whose star cost is X resolves through.
    #[serde(default)]
    pub last_stars_spent: i32,
    /// What has been put over this card's own energy cost for the rest of the
    /// turn. End-of-turn cleanup clears it.
    #[serde(default)]
    pub cost_this_turn: i32,
    /// The same, standing until the fight ends rather than until the turn
    /// does.
    #[serde(default)]
    pub cost_this_combat: i32,
    /// The run-deck card this one is the combat's copy of, named by where it
    /// stands in the deck. A card conjured mid-fight belongs to nobody, cannot
    /// be stolen, and has nothing to write through to.
    ///
    /// Cloning a card clears the link, and only standing the combat up sets
    /// it, so a card cloned mid-fight carries nothing however its original
    /// stood. Naming the deck card by its place rather than by identity is the
    /// same approximation `ListenerSource::DeckCard` makes: the deck is a list
    /// of fingerprints here rather than a list of instances.
    ///
    /// A place is only as good a name as the list is still: the link the build
    /// keeps is to the card itself, so a card leaving the deck moves nobody
    /// else's, and the places have to be carried across the gap to say the
    /// same. That is what `Simulator::take_deck_card` is for, and it is the
    /// only thing that takes a card out of the run deck.
    #[serde(alias = "from_deck")]
    pub deck_version: Option<usize>,
    /// The card this one was made a copy of, when it was. Being a clone is
    /// this being set, and it is what a card that reads the fight's history as
    /// it joins the combat asks before it acts — a copy made mid-fight is not
    /// owed what the original was.
    #[serde(default)]
    pub clone_of: Option<CardInstanceId>,
    /// How many extra times this card's own effect runs when it is played.
    #[serde(default)]
    pub replay_count: i32,
    /// Whether this copy burns up if it is still in hand when the turn ends.
    /// It starts as the model's own keyword, and the keyword can also be
    /// applied to a single copy mid-fight.
    #[serde(default)]
    pub ethereal: bool,
    /// The same for Retain: the keyword can be put on a copy, and that copy
    /// keeps itself out of a flushed hand as if the model declared it.
    #[serde(default)]
    pub retain: bool,
    /// And for Sly, which a Master Planner puts on every skill its owner plays.
    #[serde(default)]
    pub sly: bool,
    /// Sly given for this turn alone, which is what a Hand Trick hands one
    /// card of the hand. A card is Sly this turn if it has the keyword or
    /// this, and end-of-turn cleanup clears it.
    #[serde(default)]
    pub sly_this_turn: bool,
    /// The same for Retain: what a Well Laid Plans hands the cards it keeps
    /// back, cleared when the turn ends.
    #[serde(default)]
    pub retain_this_turn: bool,
    /// What this copy has written on itself for this fight — a Regret's count
    /// of the hand it was held in — which is neither saved with the run nor a
    /// saved property. Nothing projects it, and it goes with the combat.
    #[serde(default)]
    pub locals: BTreeMap<String, i32>,
    /// The one affliction a card can carry at a time.
    /// Nothing projects it; what it comes to is read by whatever put it there.
    #[serde(default)]
    pub affliction: Option<ModelId>,
    /// What has been added to this card's own damage number, which is what a
    /// card like Thrash grows. It belongs to the combat's copy of the card and
    /// nothing projects it.
    #[serde(default)]
    pub damage_bonus: i32,
    /// The player turn this copy was last played on, and nothing when it
    /// never has been. It stands in for the history entry a card that comes
    /// back to the hand looks itself up in.
    #[serde(default)]
    pub last_played_turn: Option<u32>,
    /// A copy made to be played once. It goes into no pile at all when the
    /// play is over, and it carries no Exhaust, because making the copy takes
    /// that keyword off.
    #[serde(default)]
    pub dupe: bool,
    /// The costs standing over this card's own, oldest first.
    ///
    /// They are a list because a card can carry more than one at a time, each
    /// with its own lifetime: a cost rolled for the whole fight, and a cost of
    /// nothing put over it for one turn. Each is read in the order it was put
    /// on, and each comes off on its own terms — so the one underneath is what
    /// the card goes back to costing.
    ///
    /// `energy_cost` is what the card costs now; these only say what put it
    /// there and when each of them stops standing.
    #[serde(default, alias = "temporary_cost", with = "one_or_many_costs")]
    pub temporary_costs: Vec<TemporaryCost>,
    /// The same for stars, which the build keeps as a list of its own. The
    /// two stack together — anything that makes a card free makes it free of
    /// both — and anything that only makes it cheaper touches the energy list
    /// alone.
    #[serde(default, alias = "temporary_star_cost", with = "one_or_many_costs")]
    pub temporary_star_costs: Vec<TemporaryCost>,
    /// What has been added to this card's own cost until the card is next
    /// played. It shifts the cost rather than standing over it, so it stacks
    /// with itself: a slumbering essence held through three turn ends is worth
    /// three.
    #[serde(default)]
    pub cost_until_played: i32,
    /// Whether the enchantment this copy carries still acts. It is neither
    /// saved with the run nor a saved property: nothing writes it down and a
    /// fresh combat copy starts at `Normal`.
    #[serde(default)]
    pub enchantment_status: EnchantmentStatus,
    /// What an enchantment writes on itself for the fight — the extra damage
    /// a Momentum has built up, whether a Glam has already been used. Like the
    /// status they are neither saved nor projected.
    #[serde(default)]
    pub enchantment_locals: BTreeMap<String, i32>,
}

impl CardInstance {
    /// Whether this card is the combat's copy of one in the run deck.
    #[must_use]
    pub const fn from_deck(&self) -> bool {
        self.deck_version.is_some()
    }
}

/// Whether the enchantment still acts.
///
/// A disabled enchantment is still on the card and still listens; what being
/// disabled costs it is whatever part of its behaviour tests the status.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnchantmentStatus {
    #[default]
    Normal,
    Disabled,
}

/// Reads a cost list that an older state may have written as a single cost or
/// as nothing at all.
mod one_or_many_costs {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::TemporaryCost;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        One(Option<TemporaryCost>),
        Many(Vec<TemporaryCost>),
    }

    pub(super) fn serialize<S: Serializer>(
        costs: &Vec<TemporaryCost>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        costs.serialize(serializer)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<TemporaryCost>, D::Error> {
        Ok(match OneOrMany::deserialize(deserializer)? {
            OneOrMany::One(None) => Vec::new(),
            OneOrMany::One(Some(cost)) => vec![cost],
            OneOrMany::Many(costs) => costs,
        })
    }
}

/// A cost put over a card's own, and what takes it back off. A card carries a
/// list of them and reads them in the order they were put on, each standing
/// over what the one before it left.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag is one independent term of a temporary card cost"
)]
pub struct TemporaryCost {
    /// What the card costs while this stands.
    pub cost: i32,
    /// Whether the end of the turn takes it back off.
    pub clears_when_turn_ends: bool,
    /// Whether playing the card takes it back off.
    pub clears_when_played: bool,
    /// The cost stands only where it would lower what the card costs, which is
    /// what a discount asked for as a reduction alone comes to.
    #[serde(default)]
    pub reduce_only: bool,
    /// Whether the card's star cost is put down with it.
    ///
    /// Setting a card's energy cost and making a card free are two different
    /// things. Setting the energy cost puts one entry on the energy list and
    /// leaves the star list alone; making the card free puts one on each. So
    /// anything that makes a card cheaper by a point touches energy alone, and
    /// anything that makes a card *free* makes it free of both.
    #[serde(default)]
    pub frees_the_star_cost: bool,
}

impl TemporaryCost {
    /// Gone when the card is played or when the turn ends, whichever comes
    /// first.
    #[must_use]
    pub const fn this_turn(cost: i32) -> Self {
        Self {
            cost,
            clears_when_turn_ends: true,
            clears_when_played: true,
            reduce_only: false,
            frees_the_star_cost: false,
        }
    }

    /// The same, standing only where it would make the card cheaper.
    #[must_use]
    pub const fn this_turn_reduce_only(cost: i32) -> Self {
        Self {
            reduce_only: true,
            ..Self::this_turn(cost)
        }
    }

    /// A cost set for the turn: it stands until the turn is over and playing
    /// the card does not take it off — which is the one thing that separates
    /// it from [`Self::this_turn`]. A flatten writes it on itself once the pet
    /// has swung.
    #[must_use]
    pub const fn set_this_turn(cost: i32) -> Self {
        Self {
            cost,
            clears_when_turn_ends: true,
            clears_when_played: false,
            reduce_only: false,
            frees_the_star_cost: false,
        }
    }

    /// Stands until the fight is over.
    #[must_use]
    pub const fn this_combat(cost: i32) -> Self {
        Self {
            cost,
            clears_when_turn_ends: false,
            clears_when_played: false,
            reduce_only: false,
            frees_the_star_cost: false,
        }
    }

    /// Free of both energy and stars until the card is played or the turn
    /// ends.
    #[must_use]
    pub const fn free_this_turn() -> Self {
        Self {
            frees_the_star_cost: true,
            ..Self::this_turn(0)
        }
    }

    /// The same, standing until the fight is over.
    #[must_use]
    pub const fn free_this_combat() -> Self {
        Self {
            frees_the_star_cost: true,
            ..Self::this_combat(0)
        }
    }

    /// The same, standing only where it would make the card cheaper.
    #[must_use]
    pub const fn this_combat_reduce_only(cost: i32) -> Self {
        Self {
            reduce_only: true,
            ..Self::this_combat(cost)
        }
    }
}

/// Where a played card is put down, for a power that answers it.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayedCardRule {
    /// Corruption: a skill of its owner's goes to the exhaust pile rather
    /// than wherever it would have gone.
    SkillsExhaust,
    /// Rebound: a card of its owner's that would have been discarded goes to
    /// the top of the draw pile instead.
    DiscardsToDrawPileTop,
    /// Nostalgia: the first few attack or skill plays of its owner's turn go
    /// to the top of the draw pile rather than to the discard. How many
    /// is the power's own amount, and the count is of the plays that had
    /// already started when this one's pile was settled — which is before this
    /// play is written into the history.
    EarlyAttackOrSkillPlaysToDrawPileTop,
    /// Feral: an attack of its owner's that spent no energy goes to the top
    /// of the *hand* rather than to the discard, up to the power's own
    /// amount of them. The count is the power's own counter rather than the
    /// history — the power keeps it across the plays of a turn, resets it as
    /// its owner's turn opens, and winds it on once it has actually changed
    /// where a card landed.
    ZeroCostAttacksToHandTop,
}

/// What a card costs while a power stands, for a power that answers it. The
/// pass runs over the whole listener list every time a cost is read, so what
/// it says is never written onto the card.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardCostRule {
    /// Free Attack: an attack of its owner's standing in the hand or in play
    /// costs nothing.
    FreeAttacksInHandOrPlay,
    /// Corruption: a skill of its owner's costs nothing.
    FreeSkills,
    /// Free Skill: a skill of its owner's standing in the hand or in play
    /// costs nothing.
    FreeSkillsInHandOrPlay,
    /// Tangled: an attack of its owner's costs the power's own amount more.
    /// The build hangs that on the Entangled affliction, which the power works
    /// onto every attack of its owner's as it lands and onto every attack that
    /// enters while it stands, and clears as it comes off — so while it stands
    /// there is no attack of its owner's that is not afflicted.
    AttacksCostMore,
    /// Spiked Gauntlets: a power of its owner's costs this much more.
    PowersCostMore,
    /// Curious: a power of its owner's costs this much less, never below
    /// nothing, and only where it costs something to begin with — a power
    /// already free is left alone rather than answered for.
    PowersCostLess,
    /// Borrowed Time: every card of its owner's costs the power's own amount
    /// more, whatever type it is.
    CardsCostMore,
    /// Brilliant Scarf: a card of its owner's standing in the hand or in play
    /// costs nothing.
    FreeInHandOrPlay,
    /// Void Form: a card of its owner's standing in the hand or in play costs
    /// nothing while they have played fewer cards this turn than the rule is
    /// worth.
    FreeInHandOrPlayUnderCount,
    /// Free Power: a power of its owner's standing in the hand or in play
    /// costs nothing.
    FreePowersInHandOrPlay,
    /// Veilpiercer: a card of its owner's that carries Ethereal and stands in
    /// the hand or in play costs nothing. The keyword is read off the copy as
    /// it stands, so a card made ethereal mid-fight is free too.
    FreeEtherealInHandOrPlay,
}

impl CardCostRule {
    /// Which of the build's two cost passes the rule answers. Everything the
    /// early pass says stands before anything the late pass says.
    #[must_use]
    pub const fn runs_late(self) -> bool {
        matches!(
            self,
            Self::FreeAttacksInHandOrPlay
                | Self::FreeSkills
                | Self::FreeSkillsInHandOrPlay
                | Self::FreeInHandOrPlay
                | Self::FreePowersInHandOrPlay
                | Self::FreeEtherealInHandOrPlay
                | Self::FreeInHandOrPlayUnderCount
        )
    }

    /// Whether the rule answers for star costs as well as energy ones.
    #[must_use]
    ///
    /// Only two rules of the pinned build do, and both say the same thing
    /// about stars as they do about energy: a card of their owner's standing
    /// in the hand or in play costs nothing of either. Every other cost rule
    /// is written for energy alone, so a power that makes its owner's attacks
    /// dearer leaves their star costs where they stand.
    pub const fn answers_the_star_pass(self) -> bool {
        matches!(
            self,
            Self::FreeInHandOrPlay | Self::FreeInHandOrPlayUnderCount
        )
    }
}

/// A cost rule as a relic answers it: what the rule does, how much it is
/// worth, and the count the relic has to be standing on for it to answer at
/// all.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RelicCostRule {
    pub rule: CardCostRule,
    #[serde(default)]
    pub amount: i32,
    /// The relic's own count of cards played this turn standing exactly here,
    /// which is what a brilliant scarf asks before it makes a card free.
    #[serde(default)]
    pub when_counter_is: Option<i32>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Piles {
    pub draw: Vec<CardInstance>,
    pub hand: Vec<CardInstance>,
    pub discard: Vec<CardInstance>,
    pub exhaust: Vec<CardInstance>,
    pub play: Vec<CardInstance>,
}

impl Piles {
    /// Every card in the combat, which is what rules that count cards by kind
    /// read: hand, draw, discard, exhaust, play, in that order.
    pub fn all_cards(&self) -> impl Iterator<Item = &CardInstance> {
        self.hand
            .iter()
            .chain(&self.draw)
            .chain(&self.discard)
            .chain(&self.exhaust)
            .chain(&self.play)
    }

    pub fn all_cards_mut(&mut self) -> impl Iterator<Item = &mut CardInstance> {
        self.hand
            .iter_mut()
            .chain(&mut self.draw)
            .chain(&mut self.discard)
            .chain(&mut self.exhaust)
            .chain(&mut self.play)
    }
}

/// One entry of the fight's record of finished card plays.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FinishedPlay {
    pub card_id: CardInstanceId,
    /// The card's type as the play finished, which is what anything filtering
    /// the history by kind reads.
    pub card_type: CardType,
    /// The player turn the play finished on, which is what "this turn" and
    /// "last player turn" are asked against.
    pub turn: u32,
    /// Whether the card played was a one-shot copy. Anything looking for a
    /// card to copy skips the copies it already made.
    #[serde(default)]
    pub was_dupe: bool,
    /// Whether the player chose the play rather than it being played
    /// automatically. A Pael's Eye counts only the plays they chose.
    #[serde(default = "yes")]
    pub was_chosen: bool,
    /// Whether the card carried Ethereal as the play finished. A banshee's
    /// cry and a pull from below both count
    /// the fight's ethereal plays, and the count is written down here because
    /// the card itself may be gone by the time anything reads it.
    #[serde(default)]
    pub was_ethereal: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlayerCombatState {
    pub slot: String,
    pub creature_id: CreatureCombatId,
    pub energy: i32,
    pub max_energy: i32,
    /// The Regent's second resource. It has no maximum, nothing modifies a
    /// maximum for it, and nothing resets it — nothing refills it as a turn
    /// opens, so what a fight's first turn is given stands until it is spent.
    /// It starts at nothing because the whole combat state is thrown away and
    /// built fresh as a fight is set up, which is also why stars never carry
    /// from one fight to the next.
    #[serde(default)]
    pub stars: i32,
    /// How many cards this player has conjured into this fight. The history
    /// is the combat's, so it is not cleared between turns.
    #[serde(default)]
    pub cards_generated_this_combat: u32,
    /// How many stars this player has gained since the turn opened — the
    /// gains alone, not the net of gains and spends. A Radiate counts its hits
    /// off exactly that.
    #[serde(default)]
    pub stars_gained_this_turn: i32,
    pub turn: u32,
    pub piles: Piles,
    /// Where inside the turn machine this player stands.
    pub phase: PlayerTurnPhase,
    /// Whether this player has said they are done acting for the turn. It is
    /// set from the moment the turn is ended until the next one opens.
    pub ready_to_end_turn: bool,
    /// How many card plays this player has started this turn.
    #[serde(default)]
    pub cards_played_this_turn: u32,
    /// Cards this player discarded this turn. The flush of the hand at the end
    /// of a turn is a move between piles rather than a discard, so it does not
    /// count.
    #[serde(default)]
    pub cards_discarded_this_turn: u32,
    /// Every card this player has drawn in the fight. The history is the
    /// combat's, so this is not cleared between turns.
    #[serde(default)]
    pub cards_drawn_this_combat: u32,
    /// The same, narrowed to this turn and to the draws the opening hand draw
    /// did not make, which is what a death march grows on.
    #[serde(default)]
    pub cards_drawn_this_turn_outside_hand_draw: u32,
    /// Whole swings this player's pet has made this turn. A swing is counted
    /// once, after its last hit and before anything reacts to the attack, so a
    /// swing of several hits counts once.
    #[serde(default)]
    pub pet_attacks_this_turn: u32,
    /// The type of each play this player has *begun* this turn, in order. It
    /// is one entry per *iteration* of a play rather than one per play, so a
    /// card that runs its effect three times writes three. A Lethality counts
    /// the attacks in it and a Smoggy asks whether it holds a skill.
    #[serde(default)]
    pub play_types_started_this_turn: Vec<CardType>,
    /// The models of the powers this player's own creature has worked onto
    /// anybody this turn, in the order they landed. It stands in for the
    /// power-received record a death's door looks a Doom up in — that record
    /// carries who applied the power, and the only thing that reads it asks
    /// for its own owner — and it is written where the build writes that
    /// record: on a fresh power that landed for something other than nothing,
    /// and on every stacking application whatever the offset came to.
    #[serde(default)]
    pub powers_applied_this_turn: Vec<ModelId>,
    /// How many card plays have finished in this combat, over every player.
    /// The history is the combat's, not the turn's, and is cleared with it.
    #[serde(default)]
    pub card_plays_finished: u32,
    /// Those finished plays themselves, in the order they finished: one entry
    /// each, which is what anything looking for the last card of a kind its
    /// owner played reads back.
    #[serde(default)]
    pub plays_finished: Vec<FinishedPlay>,
    /// How many attack or skill plays this player has started this turn. A
    /// play is counted after its own result pile has been settled, so a card
    /// asking the count does not count itself.
    #[serde(default)]
    pub attack_or_skill_plays_this_turn: u32,
    /// How many attack plays this player has finished this turn.
    #[serde(default)]
    pub attacks_played_this_turn: u32,
    /// How many of this player's cards have burned up this turn, which is what
    /// the cards that ask whether anything has exhausted read.
    #[serde(default)]
    pub cards_exhausted_this_turn: u32,
    /// One entry per powered block gain this turn. A card ID exempts gains
    /// within that card's latest play from Unmovable's count. Starting another
    /// play of the same card changes its earlier entries to `None`, preserving
    /// their count without exempting them again. Unattributed gains are also
    /// `None` and always count.
    #[serde(default)]
    pub block_gains_this_turn: Vec<Option<CardInstanceId>>,
    /// The orbs this player is holding and how many slots they have to hold
    /// them in.
    #[serde(default)]
    pub orbs: OrbQueue,
    /// How many plays this player has *begun* this turn, counting a play the
    /// player did not choose and counting a card that runs its effect several
    /// times once. An Echo Form caps itself on it.
    #[serde(default)]
    pub first_plays_started_this_turn: u32,
    /// The same, narrowed to attacks that spent no energy: what a Feral counts
    /// as it lands.
    #[serde(default)]
    pub zero_cost_attack_plays_this_turn: u32,
    /// How many status cards this player has drawn this turn, which is what an
    /// Iteration asks before it pays for a status.
    #[serde(default)]
    pub status_cards_drawn_this_turn: u32,
    /// What this player's plays have paid in energy since the turn opened. It
    /// is what a Helix Drill counts its hits off.
    #[serde(default)]
    pub energy_spent_this_turn: i32,
    /// One entry per orb this player has channelled, in order. The history is
    /// the combat's, so it is not cleared between turns, and a Voltaic counts
    /// the Lightning orbs in it.
    #[serde(default)]
    pub orbs_channeled_this_combat: Vec<ModelId>,
}

/// A player's orb slots and what is standing in them.
///
/// The queue is built with the combat state and is thrown away with it: it
/// starts empty and is given the character's own base orb slot count back, so
/// nothing about it outlives a fight.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrbQueue {
    /// How many orbs the queue can hold, which every request to add slots is
    /// held at ten.
    #[serde(default)]
    pub capacity: i32,
    /// The orbs held, front first. Evoking the next orb reaches the front and
    /// evoking the last one reaches the back; a channel past a full queue
    /// evokes the front to make room, and the new orb joins at the back.
    #[serde(default)]
    pub orbs: Vec<OrbInstance>,
    /// The identity the next orb channelled into this queue takes.
    #[serde(default)]
    pub next_orb_instance: u64,
}

impl OrbQueue {
    /// The most slots a queue may ever have; every request to add slots is
    /// trimmed to it.
    pub const MAX_CAPACITY: i32 = 10;

    /// Adds orb slots.
    pub const fn add_capacity(&mut self, amount: i32) {
        self.capacity += amount;
    }

    /// Takes orb slots away: the capacity floors at nothing, and any orb left
    /// standing past it comes off the back one at a time.
    ///
    /// An orb dropped this way is simply taken out of the queue without being
    /// marked as having left the combat, so it never runs an evoke and nothing
    /// that reacts to an orb leaving sees it.
    pub fn remove_capacity(&mut self, amount: i32) {
        self.capacity = (self.capacity - amount).max(0);
        while i32::try_from(self.orbs.len()).unwrap_or(i32::MAX) > self.capacity {
            self.orbs.pop();
        }
    }

    /// Where an orb stands in the queue, by identity.
    #[must_use]
    pub fn position(&self, orb: OrbInstanceId) -> Option<usize> {
        self.orbs.iter().position(|held| held.id == orb)
    }

    #[must_use]
    pub fn get(&self, orb: OrbInstanceId) -> Option<&OrbInstance> {
        self.orbs.iter().find(|held| held.id == orb)
    }

    pub fn get_mut(&mut self, orb: OrbInstanceId) -> Option<&mut OrbInstance> {
        self.orbs.iter_mut().find(|held| held.id == orb)
    }
}

/// One orb standing in a queue.
///
/// An orb grows its own numbers as it sits in the queue: a Dark orb's evoke
/// value climbs by its own passive every time the passive fires, and a Glass
/// orb's passive value falls by one every time it does. Neither is saved —
/// orbs are combat-only — so the projection carries them only as the `passive`
/// and `evoke` numbers they come to.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrbInstance {
    pub id: OrbInstanceId,
    pub model_id: ModelId,
    /// What the orb itself says its passive is worth before any listener
    /// speaks, which is the number a Glass orb wears down. An orb whose
    /// passive is a constant carries the constant.
    pub passive_base: i32,
    /// The same for the evoke, which is the number a Dark orb grows. An orb
    /// whose evoke is derived from its passive carries nothing here.
    pub evoke_base: i32,
    /// The passive and evoke values as they stand: the numbers above worked
    /// through everything that modifies orb values, where the orb asks for it.
    /// The build works them out at every read; here they are written back
    /// wherever they could have changed, because the recording carries the
    /// answer rather than the question.
    #[serde(default)]
    pub passive: i32,
    #[serde(default)]
    pub evoke: i32,
    /// Whether the orb has left the combat for good, which is set once an
    /// evoke that took it out of the queue has finished. It is the orb's half
    /// of the listener liveness test, and it outlives the orb leaving the
    /// queue.
    #[serde(default)]
    pub removed_from_state: bool,
}

/// Where a player stands inside their turn.
///
/// The pinned build carries this per player and the recorder projects it, so it
/// is gameplay state rather than presentation: what a listener may do, and
/// whether the player may act at all, depends on it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerTurnPhase {
    /// Not in a turn: combat is over, or the enemy side is acting.
    #[default]
    None,
    /// From the moment the side switches until the player may play cards.
    /// Block clearing, the energy refill, and the draw all happen here.
    Start,
    /// Start-of-turn auto-play effects.
    AutoPrePlay,
    /// The player is acting.
    Play,
    /// End-of-turn auto-play effects.
    AutoPostPlay,
    /// From the moment the turn is ended until the side switches.
    End,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerType {
    Buff,
    Debuff,
}

/// How a power carries its amount.
///
/// The build has a fourth, unset value that no power ever declares, so it is
/// not modelled here. The distinction is what a power shows and how a negative
/// amount reads: only a counter power that may go negative reports itself as a
/// debuff while it is down.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerStackType {
    /// The power carries no amount to speak of. Nothing is shown for it and
    /// the powers that work this way are applied once, for one.
    None,
    /// The amount is shown, and something has to work it up or down.
    Counter,
    /// The amount is hidden. Nothing enforces that it stays at one; the
    /// powers that work this way simply never apply more.
    Single,
}

/// Whether a second application joins the power a creature already carries or
/// stands up a new one.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerInstanceType {
    /// One instance per creature: a later application stacks onto it.
    None,
    /// Every application is its own instance, which is what a power that counts
    /// down on its own timer needs.
    Instanced,
    /// One instance per applier: a second application by the same creature
    /// stacks, one by anybody else stands up its own.
    InstancedPerApplier,
}

/// A power that lends a stat and takes it back.
///
/// There is one of these per stat, and each such power is that shared
/// behaviour plus the stat it lends: it works the amount — signed — of the
/// stat onto its owner as it lands and as it is stacked onto, and at the end
/// of a turn its owner takes part in it takes itself off and gives the same
/// amount back.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TemporaryPower {
    /// The stat power this one works onto its owner while it stands.
    pub internally_applied_power: ModelId,
    /// Which way the stat moves.
    pub positive: bool,
}

/// Which listener list a pass walks.
///
/// There are two, and each pass picks one. The combat list is every
/// creature's powers and what the player carries into the fight. The run list
/// puts the run deck's own cards in front of that list, and outside a combat
/// is the relics and potion slots on their own. Which one a pass uses is
/// fidelity, because it decides whether a deck card gets to speak before a
/// relic does.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListenerScope {
    /// The run deck's cards, then the combat list.
    Run,
    /// The run deck's cards, the relics, and the potion slots, with no combat
    /// list behind them even where a fight is standing. The reward passes are
    /// the ones that walk it, because a reward set belongs to the room rather
    /// than to the fight that filled it.
    RunWithoutCombat,
    /// The combat list alone.
    Combat,
}

/// The `true` a `#[serde(default)]` boolean needs when its absent value is yes.
pub(crate) const fn yes() -> bool {
    true
}

/// One pass over the listener list: a single moment at which everything that
/// can react is asked, in listener order.
///
/// These are the individual passes, not the moments that run them: ending a
/// side's turn runs three passes over the whole listener list, very early,
/// early, and then ordinary. Naming the passes keeps that ordering in the data
/// rather than in whoever wrote the listener, and lets a moment declare its
/// passes as a list.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    /// Before a side's turn opens, ahead of block clearing and the draw.
    BeforeSideTurnStart,
    /// After a creature's block has been cleared for the turn.
    AfterBlockCleared,
    /// The two passes run over the listener list once the player's energy has
    /// been refilled and before the draw. A listener that wants everything
    /// else to have had its say first takes the late one — a
    /// bound phylactery summons there so that a Friendship asking whether the
    /// pet is standing is answered before this turn's pet arrives.
    AfterEnergyReset,
    AfterEnergyResetLate,
    /// The three passes run over the listener list after the player's opening
    /// draw, still inside their turn setup, in this order.
    AfterPlayerTurnStartEarly,
    AfterPlayerTurnStart,
    AfterPlayerTurnStartLate,
    /// After every player on the side has finished setting up. The listener
    /// list is walked twice, the late pass second.
    AfterSideTurnStart,
    AfterSideTurnStartLate,
    /// The start-of-turn auto-play phase, between setup and player control.
    /// The build walks the listener list twice here, the early pass first: a
    /// Bombardment plays itself out of the exhaust pile there.
    AfterAutoPrePlayPhaseEnteredEarly,
    AfterAutoPrePlayPhaseEntered,
    /// The third pass of the same moment, which is for listeners that have to
    /// run after every other trigger of the phase.
    AfterAutoPrePlayPhaseEnteredLate,
    /// The end-of-turn auto-play phase, after the player stops acting.
    AfterAutoPostPlayPhaseEntered,
    /// Before a side's turn ends. Runs three passes, in this order.
    BeforeSideTurnEndVeryEarly,
    BeforeSideTurnEndEarly,
    BeforeSideTurnEnd,
    /// Before the hand is put down at the end of the player's turn. The late
    /// pass runs after the early one, so a listener that asks the player which
    /// cards to keep back sees what every other one has already done.
    BeforeFlush,
    BeforeFlushLate,
    /// After the hand has been put down, with what was kept and what was not.
    AfterFlush,
    /// After a side's turn has ended. Runs two passes, in this order.
    AfterSideTurnEnd,
    AfterSideTurnEndLate,
    /// The pass over how much energy a turn opens with.
    ModifyMaxEnergy,
    /// The two passes over how many cards the turn's hand draw is, in this
    /// order. The whole listener list is walked once for each.
    ModifyHandDraw,
    ModifyHandDrawLate,
    /// Once the hand-draw count is settled and before a card moves, which is
    /// where a power that stocks the hand first stocks it.
    BeforeHandDraw,
    /// The pass over exactly the listeners that answered where a played card
    /// goes with something other than what they were handed, which is how a
    /// rebound is spent for the card it sent back to the draw pile.
    AfterModifyingCardPlayResultPile,
    /// Around the whole of one swing rather than around each of its hits. They
    /// are what lets a power bind one swing, add itself to every hit of it,
    /// and be spent once for the lot.
    BeforeAttack,
    AfterAttack,
    /// Once a card has joined the combat state.
    AfterCardEnteredCombat,
    /// Once a card conjured into a fight is standing in its pile. It carries
    /// who conjured it, which is what a listener paying its owner for their
    /// own conjuring tests. A card the fight was laid out with is not one of
    /// these.
    AfterCardGeneratedForCombat,
    /// Once a play has paid its energy. It runs for every play, including one
    /// that paid nothing.
    AfterEnergySpent,
    /// The two passes run over the listener list once a card is standing in
    /// the hand, in this order.
    AfterCardDrawnEarly,
    AfterCardDrawn,
    /// After block is gained, which runs for every gain, including one the
    /// passes worked down to nothing. Each listener tests the amount itself.
    AfterBlockGained,
    /// Once a power has landed for something other than nothing — on a fresh
    /// power and on one stacked onto alike.
    AfterPowerAmountChanged,
    /// The chaining pass over how much energy a gain is worth. A listener
    /// answers with the value rather than with an offset.
    ModifyEnergyGain,
    /// The chaining pass over what a card costs in stars. Unlike the energy
    /// cost, which gets an early and a late pass, this is one pass; and a cost
    /// of less than nothing is answered before the walk begins, so nothing can
    /// give a star cost to a card that has none.
    ModifyStarCost,
    /// After stars are gained and after they are spent, each carrying how many
    /// moved. The gain pass runs for any amount at all; the spend pass runs
    /// only where something was actually paid.
    AfterStarsGained,
    AfterStarsSpent,
    /// Once a forge has stood a Sovereign Blade up and sharpened every one its
    /// owner holds. Nothing may ask the player a question there.
    AfterForge,
    /// Once an orb has actually joined the queue. It only runs where the orb
    /// was enqueued, so a channel into a queue with no capacity at all
    /// announces nothing.
    AfterOrbChanneled,
    /// Once an orb has been evoked, carrying the orb and whatever its evoke
    /// reached. It runs after the evoke itself and before a dequeued orb is
    /// marked gone, so the orb is still a live listener for its own
    /// announcement.
    AfterOrbEvoked,
    /// The chaining pass over what an orb's passive or evoke is worth. It is
    /// asked every time the value is read rather than once, so a Focus that
    /// lands between two reads changes the second.
    ModifyOrbValue,
    /// The chaining pass over how many times one orb's turn trigger fires, and
    /// then the pass over exactly the listeners that changed it.
    ModifyOrbPassiveTriggerCount,
    AfterModifyingOrbPassiveTriggerCount,
    /// The chaining pass over how many times a play runs the card's effect,
    /// and then the pass over whoever changed it.
    ModifyCardPlayCount,
    AfterModifyingCardPlayCount,
    /// After a card has burned up: the card is added to the exhaust pile, the
    /// burn is written into the combat history, and then this runs. There is
    /// no bulk form of it, and the reason there is none is that the listeners
    /// for one card have to finish before the next card starts burning — so a
    /// run of cards announces this once each, in order.
    AfterCardExhausted,
    /// A card discarded into the discard pile, once per card of the batch. The
    /// end-of-turn hand flush is not one of them: it is a move between piles
    /// rather than a discard.
    AfterCardDiscarded,
    /// Before a card is played, with whatever it was aimed at.
    BeforeCardPlayed,
    /// The two passes run over the listener list once a card has finished
    /// being played, in this order.
    AfterCardPlayed,
    AfterCardPlayedLate,
    /// Before a potion is drunk, with whatever it was aimed at.
    BeforePotionUsed,
    /// After the potion's own effect has finished, which is where a relic
    /// that pays for drinking one is reached.
    AfterPotionUsed,
    /// The chaining pass over how much gold a gain is worth, and then the
    /// pass over whoever changed it.
    ModifyGoldGained,
    AfterModifyingGoldGained,
    /// The additive pass over what resting is worth, and the announcement once
    /// the heal has happened.
    ModifyRestSiteHealAmount,
    AfterRestSiteHeal,
    /// The one pass over what resting hands over besides the heal. What the
    /// listeners put on the list is offered as a reward set of its own.
    TryModifyRestSiteHealRewards,
    /// The two passes over a room's reward set once the rewards it generated
    /// for itself have been rolled, and then the pass over exactly the
    /// listeners that put something on the list. What a listener adds is
    /// rolled after every reward that was already there, in the order it was
    /// added.
    TryModifyRewards,
    TryModifyRewardsLate,
    AfterModifyingRewards,
    /// The two passes over the cards a card reward has just been built out of,
    /// and then the pass over exactly the listeners that changed one. They run
    /// while the reward is still being built, so they see the offer before
    /// anybody else does.
    TryModifyCardRewardOptions,
    TryModifyCardRewardOptionsLate,
    AfterModifyingCardRewardOptions,
    /// Once a player's draw pile has been shuffled.
    AfterShuffle,
    /// The pass over the order a shuffle settled on, before it becomes the
    /// draw pile. A listener moves its own card within the list rather than
    /// answering with a value.
    ///
    /// The game distinguishes the shuffle that lays a fight's opening draw
    /// pile out from every later one. Only the later ones are announced here,
    /// because the combat listener list is not standing yet when a fight lays
    /// its opening draw pile out — and the one thing in the pinned build that
    /// answers this pass declines the opening shuffle anyway, so nothing of it
    /// is lost.
    ModifyShuffleOrder,
    /// The pass run once the two block passes have settled, over the whole
    /// listener list rather than over whoever answered. Each listener tests
    /// the settled amount itself, which is how a vambrace knows the doubling
    /// it asked for landed.
    AfterModifyingBlockAmount,
    /// The four passes run over each settled damage result once the blow has
    /// landed, in this order. Each has a condition of its own, read off the
    /// result rather than off the listeners, and each listener then makes its
    /// own tests on top:
    ///
    /// - `AfterBlockBroken` when block took some of the blow and ran out;
    /// - `AfterCurrentHpChanged` when the blow cost the target HP;
    /// - `AfterDamageGiven` for every result, in combat, whatever it came to;
    /// - `AfterDamageReceived` unless the blow killed the target and the target
    ///   is still dead.
    ///
    /// The block-broken pass also runs when block is lost some other way, and
    /// the HP-changed pass whenever anything else moves a creature's HP.
    AfterBlockBroken,
    AfterCurrentHpChanged,
    /// Damage given runs for every blow, not only for attacks, and over the
    /// whole listener list rather than over the dealer's own listeners: each
    /// listener tests the dealer, the target, and the result itself.
    AfterDamageGiven,
    /// After a creature has taken damage.
    AfterDamageReceived,
    /// After the damage has been worked out and before block takes any of it.
    BeforeDamageReceived,
    /// The two passes run after every creature has joined a fight and before
    /// the opening turn.
    BeforeCombatStart,
    BeforeCombatStartLate,
    /// Once the room the player walked into is standing.
    ///
    /// The build announces it from every room. This engine announces it from
    /// the combat room alone, because a listener pass outside a combat has no
    /// machinery yet; anything that answers it for another room is unsupported
    /// until it does.
    AfterRoomEntered,
    /// Once for each creature that has gone down, over the run's listeners.
    AfterDeath,
    /// Once for the whole list a doom kill took down. A doom kill takes the
    /// list down one creature at a time and then runs this single pass over
    /// it, so a listener counting the dead reads them all together rather than
    /// one at a time.
    AfterDiedToDoom,
    /// Once a creature that arrived mid-fight is standing. It is announced
    /// even while a fight is ending, because a creature added then can decide
    /// whether it really ends. The creature that arrived is the pass's
    /// subject.
    AfterCreatureAddedToCombat,
    /// The first thing that happens once the fight has stopped being in
    /// progress, over the run's listeners with the combat still standing. It
    /// is where a card that counts the fights a run has seen counts one.
    ///
    /// The build runs it for a lost fight too. Here a lost fight is a lost
    /// run, settled at the next boundary rebuild with the combat dropped, so
    /// the engine announces it only where a fight is finished.
    AfterCombatEnd,
    /// The two passes run once the fight is won, in this order.
    AfterCombatVictoryEarly,
    AfterCombatVictory,
    /// The hand is empty and nothing is mid-play, which is checked after a
    /// card play and after a potion.
    AfterHandEmptied,
    /// Once gold the player actually kept has landed.
    AfterGoldGained,
    /// Once something has been bought at a merchant.
    AfterItemPurchased,
    /// The chaining pass over what a shelf entry costs. It is asked every time
    /// a price is read and only where the room standing is a merchant's, so a
    /// fake merchant's shelf is untouched by it. Two relics answer it and both
    /// answer multiplicatively, which is why a shelf held by both is worth two
    /// fifths rather than three tenths off.
    ModifyMerchantPrice,
    /// Once a potion has reached or left a belt slot.
    AfterPotionProcured,
    AfterPotionDiscarded,
    /// The three passes over damage, in this order: everything additive, then
    /// everything multiplicative, then the caps. Nothing in the passes reads
    /// what kind of damage it is; each listener tests that itself, which is
    /// why a cap applies to unpowered damage and a scale does not.
    ModifyDamageAdditive,
    ModifyDamageMultiplicative,
    ModifyDamageCap,
    /// The two passes over block. There is no cap pass.
    ModifyBlockAdditive,
    ModifyBlockMultiplicative,
    /// The four passes over what block did not stop, two before the pet may
    /// take the blow instead and two after. Redirecting a blow happens between
    /// the pairs, so nothing can redirect one until the pet exists.
    ModifyHpLostBeforeOsty,
    ModifyHpLostBeforeOstyLate,
    ModifyHpLostAfterOsty,
    ModifyHpLostAfterOstyLate,
    /// The two passes over exactly the listeners that changed the HP a blow
    /// cost, one per phase, in listener order. The first runs between the two
    /// phases and the second after them, which is how a buffer is spent for
    /// the blow it turned away.
    AfterModifyingHpLostBeforeOsty,
    AfterModifyingHpLostAfterOsty,
    /// Who actually takes what block did not stop, chained through the
    /// listener list between the two HP-loss phases.
    ///
    /// Nothing is announced here: the one thing in the pinned build that
    /// redirects a blow declares that it does rather than answering a pass,
    /// and nothing else ever will. What the pass is here for is the list — it
    /// walks the combat listeners directly rather than stopping while a fight
    /// is ending, which is exactly what that pair of declarations comes to.
    ModifyUnblockedDamageTarget,
    /// Run for each player who is taking an extra turn, straight after the
    /// sides did not switch.
    AfterTakingExtraTurn,
    /// Run at the very start of either way a power can be applied, before
    /// anything is allowed to change the amount, and whether or not the
    /// applier is standing in the fight. It carries the card the application
    /// came from, which nothing else about a power application does: it is
    /// what arms a listener that then doubles what that card works.
    BeforePowerAmountChanged,
    /// What the creature working a power makes of how much of it they are
    /// handing over. It runs before the receiving pass, and only where the
    /// applier is standing in the combat.
    ModifyPowerAmountGiven,
    /// Runs only over the listeners that changed the amount in that pass.
    AfterModifyingPowerAmountGiven,
    /// What a power lands for on the creature it is being worked onto.
    ModifyPowerAmountReceived,
    /// Runs only over the listeners that changed the amount in the pass above,
    /// in listener order, which is how an absorber is spent for what it turned
    /// away.
    AfterModifyingPowerAmountReceived,
    /// What a power does as it lands, which is how one power brings another
    /// with it.
    AfterApplied,
    /// The two passes run whenever a card moves, is created into a pile, or
    /// leaves the combat: the ordinary one over the whole list, then the late
    /// one over the whole list again.
    AfterCardChangedPiles,
    AfterCardChangedPilesLate,
    /// The five things an enchantment answers for the card it is on.
    ///
    /// These are not passes over the listener list. The first four are asked
    /// of the enchantment on the card the damage or block came from, before
    /// any listener is asked; the fifth is asked of the enchantment on a card
    /// being played. They are named as events so that an enchantment is
    /// written over the same effect surface as everything else.
    EnchantDamageAdditive,
    EnchantDamageMultiplicative,
    EnchantBlockAdditive,
    EnchantBlockMultiplicative,
    /// Chaining, so the enchantment answers with the count rather than with an
    /// offset.
    EnchantPlayCount,
}

impl HookEvent {
    /// Whether the pass stops answering once the fight is over or ending.
    ///
    /// Resolution passes keep answering when they finish the action that
    /// ended combat, including Doom's announcement after the final kill.
    #[must_use]
    pub const fn guarded_while_ending(self) -> bool {
        !matches!(
            self,
            Self::AfterCardPlayed
                | Self::AfterBlockBroken
                | Self::AfterDamageGiven
                | Self::AfterDiedToDoom
                | Self::AfterCreatureAddedToCombat
                | Self::ModifyUnblockedDamageTarget
        )
    }

    /// Which listener list this pass walks.
    #[must_use]
    pub const fn scope(self) -> ListenerScope {
        match self {
            // The three damage passes walk the run list with the combat behind
            // it, and so do the HP-loss passes, the HP-changed and
            // damage-received passes, the potion passes, the combat-victory
            // passes, and the card-changed-piles passes.
            Self::ModifyDamageAdditive
            | Self::ModifyDamageMultiplicative
            | Self::ModifyDamageCap
            | Self::ModifyHpLostBeforeOsty
            | Self::ModifyHpLostBeforeOstyLate
            | Self::ModifyHpLostAfterOsty
            | Self::ModifyHpLostAfterOstyLate
            | Self::AfterModifyingHpLostBeforeOsty
            | Self::AfterModifyingHpLostAfterOsty
            | Self::AfterCurrentHpChanged
            | Self::AfterDamageReceived
            | Self::BeforeDamageReceived
            | Self::BeforePotionUsed
            | Self::AfterPotionUsed
            | Self::ModifyGoldGained
            | Self::AfterModifyingGoldGained
            | Self::ModifyRestSiteHealAmount
            | Self::AfterRestSiteHeal
            | Self::AfterCombatEnd
            | Self::AfterCombatVictoryEarly
            | Self::AfterCombatVictory
            | Self::AfterPotionProcured
            | Self::AfterPotionDiscarded
            | Self::AfterCardChangedPiles
            | Self::AfterCardChangedPilesLate
            // Combat start and room entry both walk the run list, the second
            // with no combat behind it at all.
            | Self::BeforeCombatStart
            | Self::BeforeCombatStartLate
            | Self::AfterDeath
            | Self::AfterRoomEntered => ListenerScope::Run,
            // The reward passes walk the run list with no combat behind it at
            // all, so a fight standing behind the reward screen takes no part
            // in them.
            Self::AfterGoldGained
            | Self::AfterItemPurchased
            | Self::ModifyMerchantPrice
            | Self::TryModifyRestSiteHealRewards
            | Self::TryModifyRewards
            | Self::TryModifyRewardsLate
            | Self::AfterModifyingRewards
            | Self::TryModifyCardRewardOptions
            | Self::TryModifyCardRewardOptionsLate
            | Self::AfterModifyingCardRewardOptions => ListenerScope::RunWithoutCombat,
            _ => ListenerScope::Combat,
        }
    }

    /// Whether a listener on this pass may open a selection screen, and under
    /// which of the three disciplines.
    ///
    /// Some passes can be interrupted by a player choice and some cannot: a
    /// pass that cannot gives its listeners nothing to pause with, so a
    /// listener that asked a question there would have nowhere to stop. Which
    /// of the three disciplines applies is settled by the moment that runs the
    /// pass rather than by the pass itself, because what pauses is not the
    /// same thing in each.
    #[must_use]
    pub const fn suspension(self) -> HookSuspension {
        match self {
            // The pass is run with one pause point and each listener is
            // finished before the next begins, so a screen stops the pass
            // where it stands and everything behind it waits for the answer.
            Self::AfterCardDrawnEarly
            | Self::AfterCardDrawn
            | Self::AfterCardPlayed
            | Self::AfterCardExhausted
            | Self::AfterPowerAmountChanged
            | Self::AfterDamageGiven
            | Self::AfterDamageReceived
            | Self::AfterPlayerTurnStart
            | Self::AfterAutoPrePlayPhaseEntered
            | Self::AfterAutoPrePlayPhaseEnteredLate
            | Self::AfterAutoPostPlayPhaseEntered
            | Self::AfterShuffle
            | Self::AfterHandEmptied
            | Self::BeforeHandDraw
            | Self::AfterFlush => HookSuspension::Blocking,
            // Each listener gets a pause point of its own and the pass only
            // waits for it to pause, not to finish. The pass carries on to the
            // next listener with the screen still owed, and the paused
            // listeners are waited for at the end of the moment that ran the
            // pass — ending a side's turn waits after all three of its passes,
            // the end-of-turn announcement after each of its two, and the
            // start of a side's turn never. Several screens can therefore be
            // outstanding at once. The engine carries this as
            // [`Effect::HookScreen`]: the pass banks each paused listener's
            // screen and runs on, and the wait opens them, in the order the
            // listeners paused.
            Self::BeforeSideTurnStart
            | Self::BeforeSideTurnEndVeryEarly
            | Self::BeforeSideTurnEndEarly
            | Self::BeforeSideTurnEnd
            | Self::BeforeFlush
            | Self::BeforeFlushLate
            | Self::AfterSideTurnEnd
            | Self::AfterSideTurnEndLate => HookSuspension::Deferred,
            // Each listener gets a pause point of its own, and nothing ever
            // waits for it: a body that asks a question is put on the player's
            // action queue as an action of its own and the pass runs on, and so
            // does the kill and the action the kill happened inside. The
            // question is put to the player when the queue reaches that action,
            // which is once the action ahead of it has finished. The engine
            // carries this as the combat's queue of hook screens.
            Self::AfterDeath | Self::AfterDiedToDoom => HookSuspension::Queued,
            _ => HookSuspension::None,
        }
    }

    /// Whether a listener on this pass may open a selection screen that this
    /// engine can put down and pick back up.
    #[must_use]
    pub const fn can_suspend(self) -> bool {
        matches!(self.suspension(), HookSuspension::Blocking)
    }

    /// Whether the listeners this moment deferred are waited for once this
    /// pass has run. Ending a side's turn waits once after the last of its
    /// three passes, flushing the hand once after its second, and the
    /// end-of-turn announcement after each of its two. A screen a deferred
    /// listener asked for is shown at that point, so this is where the engine
    /// opens the ones the pass has carried.
    #[must_use]
    pub const fn joins_deferred_bodies(self) -> bool {
        matches!(
            self,
            Self::BeforeSideTurnEnd
                | Self::BeforeFlushLate
                | Self::AfterSideTurnEnd
                | Self::AfterSideTurnEndLate
        )
    }

    /// Whether a screen a listener opens on this pass is carried to the
    /// waiting point rather than refused: every deferred pass that is waited
    /// for at all. The start of a side's turn defers its listeners and never
    /// waits for them — a paused listener sits on the action queue with
    /// nothing waiting on it — which is a discipline this engine still does
    /// not carry, so a listener that asked there still stops with the moment
    /// named.
    #[must_use]
    pub const fn defers_screens_to_a_join(self) -> bool {
        matches!(self.suspension(), HookSuspension::Deferred)
            && !matches!(self, Self::BeforeSideTurnStart)
    }

    /// Whether a screen a listener opens on this pass is queued behind the
    /// running action rather than shown where it was asked for.
    #[must_use]
    pub const fn queues_screens(self) -> bool {
        matches!(self.suspension(), HookSuspension::Queued)
    }
}

/// How a listener pass is paused for a player choice, which is not the same at
/// every moment that runs one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HookSuspension {
    /// One pause point for the whole pass, honoured listener by listener. A
    /// screen stops the pass, and whatever ran it waits.
    Blocking,
    /// One pause point per listener, honoured only until it pauses, and waited
    /// for at the end of the moment. A screen does not stop the pass.
    Deferred,
    /// One pause point per listener, honoured only until it pauses, and never
    /// waited for. A screen does not stop the pass or anything around it; it
    /// is shown once the action the pass ran inside has finished.
    Queued,
    /// The pass offers nothing to pause with, so a listener that asked a
    /// question would have nowhere to stop.
    None,
}

/// What a listener checks before acting: the tests a listener makes of what
/// the pass handed it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookCondition {
    /// Nothing is checked.
    Always,
    /// The listener's owner is one of the creatures whose turn is starting or
    /// ending.
    OwnerIsParticipant,
    /// The orb the pass is about belongs to the listener's owner, which is the
    /// first thing every orb listener asks.
    OrbBelongsToOwner,
    /// The turn starting or ending is the other side's, which is what a power
    /// that wears off between its owner's turns asks.
    OwnerIsNotParticipant,
    /// The hand is really going down, which is what a Well-Laid Plans asks
    /// before it offers to keep any of it back.
    TheHandWouldFlush,
    /// The side taking the turn is the player's.
    PlayerSideTurn,
    /// The side taking the turn is the enemy's. A timed debuff counts down here
    /// whichever side is carrying it.
    EnemySideTurn,
    /// The listener's owner is on the side that is currently acting.
    OwnerSideIsActing,
    /// The card that was played is an attack.
    PlayedCardIsAttack,
    /// The play is one the player chose. A listener that counts a turn's
    /// plays skips the ones nobody chose.
    PlayWasChosen,
    /// The card that was played is of this type.
    PlayedCardIsOfType(CardType),
    /// The play or the potion was aimed at somebody, and it belongs to the
    /// listener's owner. It is what a Surrounded asks before it turns its
    /// owner to face what they struck at.
    OwnerAimedAtSomebody,
    /// The same, and the blow got past block: the extra test an Envenom adds
    /// to the two below.
    OwnerDealtUnblockedPoweredDamage,
    /// The card whose play the pass is about belongs to the listener's owner,
    /// which is what a listener that answers for its owner's plays asks.
    OwnerPlayedTheCard,
    /// The same for whoever applied the power: a Strangle sits on the creature
    /// it is hurting and answers for the plays of the player who worked it on.
    ApplierPlayedTheCard,
    /// The listener's owner is dealing the damage, and the damage is a
    /// powered attack: the two tests Strength, Weak, and Shrink make.
    OwnerDealsPoweredDamage,
    /// The same for the owner's pet rather than the owner: the dealer is a
    /// pet, its owner is the listener's own, and the damage is a powered
    /// attack, which is what a Calcify asks before it adds to the blow.
    OwnersPetDealsPoweredDamage,
    /// The same, and the damage came off a card: the half of a Double Damage's
    /// test that lets a pet's blow count as its owner's.
    OwnersPetDealsPoweredDamageFromACard,
    /// The swing the pass is about came from the owner's own pet, powered or
    /// not, which is the pair a bone flute asks once the swing is done.
    OwnersPetAttacked,
    /// The health that just moved was the owner's own pet's, and it went down.
    /// It is the pair a necro mastery asks, and the reason the HP-change pass
    /// announces its creature rather than being about it.
    OwnersPetLostHp,
    /// The creature the blow came from is doomed — it has no more HP left than
    /// the named power's amount on it. It is how an undying sigil asks, and a
    /// dealer carrying none of the power answers no because nothing standing
    /// has less than no HP.
    DamageDealerIsDoomed(ModelId),
    /// The same, and the damage came off a card: the three tests a Double
    /// Damage makes, the last of which leaves damage that came off no card
    /// untouched. Its fourth — a pet of the owner counting as the owner — is
    /// not written, because nothing this baseline registers brings a pet.
    OwnerDealsPoweredDamageFromACard,
    /// The listener's owner is taking the damage, and it is a powered attack.
    OwnerTakesPoweredDamage,
    /// The listener's owner is on the enemy side.
    OwnerIsEnemy,
    /// The turn starting or ending is the player side's and the listener's
    /// owner is not a player: the two tests a Plating makes before a side's
    /// turn opens, so that an enemy wearing plating is already behind block
    /// when the player first looks at it.
    EnemyOwnerOnPlayerSideTurn,
    /// The creature the pass is about is somebody else on the listener's own
    /// side.
    AllyDied,
    /// The creature the pass is about is the listener's own owner, which is
    /// what a listener answering for its own death asks.
    OwnerDied,
    /// The creature the pass is about was on the other side from the
    /// listener's owner.
    OpponentDied,
    /// A Paper Cuts' test: the listener's owner dealt it, the player took it,
    /// it was a powered attack, and it cost the player HP.
    OwnerDealtPoweredHpLossToPlayer,
    /// A Thorns' test: somebody is dealing it, the listener's owner is taking
    /// it, and it is either a powered attack or the splash an Omnislice makes,
    /// which is named outright.
    OwnerTakesPoweredDamageOrOmnislice,
    /// The listener's owner is taking the damage, powered or not. A cap does
    /// not care how the damage was worked out.
    OwnerTakesDamage,
    /// The listener's owner took the damage and some of it got past block,
    /// which is what a Slippery asks. The pass runs for a blow block stopped
    /// whole too, so a listener that only answers a blow that cost something
    /// says so itself.
    OwnerTookUnblockedDamage,
    /// The same, and the blow was a powered attack: the three tests a Flutter
    /// makes.
    OwnerTookUnblockedPoweredDamage,
    /// The listener's owner took a powered attack somebody swung, which is
    /// what a hive answers. Block has no say in it — a blow stopped whole
    /// still came from whoever swung it.
    OwnerTookPoweredAttackFromSomebody,
    /// The same, and its owner's side is the one acting: the three tests an
    /// Inferno makes.
    OwnerTookUnblockedDamageOnOwnSideTurn,
    /// The listener's owner dealt the damage and block stopped all of it,
    /// which is what an Imbalanced asks.
    OwnerDealtFullyBlockedDamage,
    /// The owner's blow took the last of an opponent's block.
    OwnerBrokeAnOpponentsBlock,
    /// The block is going to the listener's owner, and it is powered block.
    OwnerGainsPoweredBlock,
    /// Dexterity's test: block off a card belongs to whoever owns the card,
    /// and block from anywhere else to whoever is gaining it — either way it
    /// has to be the listener's owner, and it has to be powered.
    OwnerGrantsPoweredBlock,
    /// The power being worked is being worked onto the listener's owner.
    PowerAppliedToOwner,
    /// The named power is being worked onto the listener's owner. This is the
    /// model check used by listeners such as Ruined Helmet, which only answers
    /// the first Strength application rather than the first power of any kind.
    PowerAppliedToOwnerIs(ModelId),
    /// The card that just burned up belongs to the listener's owner, which is
    /// what an exhaust listener asks.
    OwnersCardExhausted,
    /// The card that just burned up is the listening card itself, which is
    /// what a card answering for its own burn asks.
    ListeningCardExhausted,
    /// The card that just joined the fight is the listening card itself, which
    /// is what a Stomp asks.
    ListeningCardEnteredCombat,
    /// The card that was just drawn is the listening card itself, which is
    /// what a Void asks of a draw.
    ListeningCardDrawn,
    /// The listener's owner just gained block, and gained some rather than
    /// none.
    OwnerGainedBlock,
    /// The block the pass is about is going to the listener's owner, whatever
    /// it carries and wherever it came from: the one test a Shadowmeld makes
    /// before it doubles what its owner is working up.
    OwnerIsGainingBlock,
    /// The block is going to the listener's owner, it is powered, and it came
    /// off a card: the three tests a No Block makes, the last of which leaves
    /// block that came off no card untouched.
    OwnerGainsPoweredBlockFromACard,
    /// The block is going to the listener's owner, it is powered, and either
    /// nothing or a card carrying this tag is behind it, which is what a
    /// Fasten asks.
    OwnerGainsPoweredBlockNotFromAnUntagged(CardTag),
    /// The listener's owner just worked this power onto somebody, for more
    /// than nothing.
    OwnerWorkedPower(ModelId),
    /// The pass under way is over how much of this power the listener's owner
    /// is handing over, which is what a snecko skull asks.
    OwnerIsGivingPower(ModelId),
    /// A card the listener's owner owns was discarded while their own side was
    /// the one acting: the two tests a tingsha and a set of tough bandages both
    /// open with.
    OwnersCardDiscardedOnOwnSideTurn,
    /// The card whose play the pass is about carries this tag.
    PlayedCardIsTagged(CardTag),
    /// The card whose play the pass is about is this model, which is what a
    /// Devour Life and a Haunt both ask about a Soul.
    PlayedCardIs(ModelId),
    /// The card whose play the pass is about carries the Ethereal keyword,
    /// read off the copy as it stands.
    PlayedCardIsEthereal,
    /// The card whose play the pass is about costs at least this much, as its
    /// cost resolves at the moment it is read.
    PlayedCardEnergyAtLeast(i32),
    /// The card whose play the pass is about is standing in its owner's hand
    /// or in the play pile, which is what a veilpiercer asks both times. A
    /// card auto-played off the draw pile is in neither.
    PlayedCardIsInHandOrPlay,
    /// The power that just landed is a lasting debuff the listener's own owner
    /// worked onto an enemy: it landed for something, it reads as a debuff at
    /// that amount, its holder is an enemy, the listener's owner applied it,
    /// and it is not one of the powers that lend a stat for a turn. That is
    /// the whole of a sleight of flesh's test.
    OwnerWorkedALastingDebuffOnAnEnemy,
    /// A Lethality's whole test, which is a run of tests on the card the value
    /// came from rather than on whoever is dealing it: the damage came off a
    /// card, that card's owner is the listener's own owner, the card is not
    /// standing on a replay past its first, and no other attack play of the
    /// owner's has begun this turn — where the card is in the play pile its
    /// own play is already in the history and counts itself, and where it is
    /// anywhere else it is not.
    DamageCardIsTheOwnersFirstAttackPlay,
    /// The other way round: the listener's owner just took this power off a
    /// player — the listener's owner applied it, its holder is a player, it is
    /// this model, and the amount was negative. Those are the four tests a
    /// possession makes.
    OwnerTookPowerFromAPlayer(ModelId),
    /// The card that was just drawn belongs to the listener's owner and carries
    /// this tag.
    DrawnCardIsTagged(CardTag),
    /// The same, for the Ethereal keyword rather than a tag, which is what a
    /// Pagestorm asks of a draw — on the card as it stands rather than on its
    /// model.
    OwnersEtherealCardDrawn,
    /// A card the listener's owner drew, whatever it was, which is all a
    /// Corrosive Wave asks of a draw.
    OwnersCardDrawn,
    /// The same, for a card of one type, which is what an Iteration crosses
    /// its owner test with.
    OwnersCardOfTypeDrawn(CardType),
    /// The same, for a draw that is not the turn's own opening hand draw, and
    /// only while the owner's side is the one acting: the three tests a
    /// Speedster makes.
    OwnersCardDrawnMidTurn,
    /// The card just conjured into the fight is of this type, which a
    /// Smokestack and a Trash To Treasure both cross their owner test with.
    ConjuredCardIsOfType(CardType),
    /// The card that just entered the combat carries a tag, and belongs to
    /// the listener's owner. It is what a Phantom Blades asks before it makes
    /// a Shiv keep itself.
    EnteredCardIsTagged(CardTag),
    /// The card that joined the combat is this model, which is what a sword
    /// sage asks before it hands a blade another play.
    EnteredCardIs(ModelId),
    /// The card that joined the combat is of this type and belongs to the
    /// listener's owner, which is what a Smoggy asks of a skill.
    EnteredCardIsOfType(CardType),
    /// The card the pass is about was conjured into the fight by the
    /// listener's own owner. Only the player conjures cards here, so it comes
    /// to the listener's owner being the player.
    OwnerConjuredTheCard,
    /// The amount that just changed is the listening power's own, which is
    /// what a sword sage asks before it acts on it.
    ListeningPowerAmountChanged,
    /// A fight is in progress, which is what anything that only acts inside a
    /// live fight asks. A fight that has not been announced or has already
    /// ended is not one: the victory heal a starter relic pays lands after the
    /// fight has stopped being in progress, so a listener that asks this does
    /// not answer it.
    InACombat,
    /// The room standing is a merchant's, which is what a relic that only acts
    /// in a shop asks.
    InAMerchantRoom,
    /// The room standing is a combat room, which is what a relic that answers
    /// a room being entered asks when it only wants fights. It is not the same
    /// test as [`Self::InACombat`]: a combat room announces itself before the
    /// fight is in progress, so a relic that works strength in as the fight is
    /// walked into sees the room but not a fight in progress.
    InACombatRoom,
    /// The card the pass is about landed in the run deck, which is what a
    /// relic that answers for the deck growing asks.
    CardJoinedTheDeck,
    /// The card the value came off carries an enchantment, whichever one.
    DamageCardIsEnchanted,
    /// The damage source card belongs to the listener's owner, even when a
    /// pet deals its damage.
    DamageCardBelongsToOwner,
    /// Several tests that all have to hold, and one that has to fail. A
    /// listener usually opens with a run of tests it bails out on; where more
    /// than one of them is not already a condition of its own, they are named
    /// together here.
    All(Vec<HookCondition>),
    Not(Box<HookCondition>),
    /// The listening power was already standing when the turn opened, which is
    /// how a summon owed for next turn is not paid on the turn it was
    /// promised.
    ListeningPowerStoodAtTurnStart,
    /// The value being worked out is a powered attack, with no test of who is
    /// dealing it. An enchantment is asked about the card the value came off
    /// rather than over the listener list, so this is the only test it makes.
    ValueIsAPoweredAttack,
    /// The listening enchantment is not disabled.
    ListeningEnchantmentIsNormal,
    /// The card the pass is about is the very card the listening enchantment
    /// is on, which is what Glam, Goopy, and Vigorous each ask of a play and
    /// Slither asks of a draw.
    PlayedCardCarriesTheListeningEnchantment,
    /// The card the listening enchantment is on is standing in this pile,
    /// which is what Slither and Slumbering Essence each ask about the hand.
    EnchantedCardIsInPile(crate::PileName),
    /// The listening enchantment has written this down for the fight and it is
    /// not nothing, which is how a Glam knows it has already been used.
    ListeningEnchantmentLocalIsSet(String),
}

/// When in a combat a hook is eligible.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookTiming {
    EveryTurn,
    FirstTurnOnly,
    /// The second turn alone, which is the one turn a horn cleat braces.
    SecondTurnOnly,
    AfterFirstTurn,
}

impl HookTiming {
    /// Whether the hook is eligible on the player's `turn`, counted from one.
    #[must_use]
    pub const fn holds_on(self, turn: u32) -> bool {
        match self {
            Self::EveryTurn => true,
            Self::FirstTurnOnly => turn <= 1,
            Self::SecondTurnOnly => turn == 2,
            Self::AfterFirstTurn => turn > 1,
        }
    }
}

/// One listener: the pass it answers, the test it makes of what the pass hands
/// it, and the commands it issues.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModifierHook {
    pub event: HookEvent,
    pub timing: HookTiming,
    pub condition: HookCondition,
    pub commands: Vec<crate::Command>,
}

impl ModifierHook {
    /// A listener that acts every turn and checks nothing.
    #[must_use]
    pub const fn every_turn(event: HookEvent, commands: Vec<crate::Command>) -> Self {
        Self {
            event,
            timing: HookTiming::EveryTurn,
            condition: HookCondition::Always,
            commands,
        }
    }

    #[must_use]
    pub fn when(mut self, condition: HookCondition) -> Self {
        self.condition = condition;
        self
    }

    #[must_use]
    pub const fn timed(mut self, timing: HookTiming) -> Self {
        self.timing = timing;
        self
    }
}

/// A power: what it declares about itself and what it listens for.
///
/// Everything a power *does* is a listener over the command surface, including
/// how it changes damage, block, and the HP a blow actually costs: those are
/// passes like any other. What stays declared here is what the build settles
/// about a power up front rather than through behaviour.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag mirrors one independent property of the pinned power model"
)]
pub struct PowerDefinition {
    pub model_id: ModelId,
    /// Whether the power reads as a buff or a debuff, before the amount is
    /// taken into account.
    pub power_type: PowerType,
    /// How the power carries its amount.
    pub stack_type: PowerStackType,
    /// Whether a second application stacks or stands up its own.
    pub instance_type: PowerInstanceType,
    /// Whether the amount may go below nothing. A power that may go negative
    /// shows as a debuff while it is down and falls away at exactly zero
    /// rather than at zero or below.
    pub allow_negative: bool,
    /// Whether its owner counts as a secondary enemy. A combat ends when the
    /// primary enemies are gone, whether or not a secondary one still stands.
    pub owner_is_secondary_enemy: bool,
    /// Whether its owner leaves the fight when it dies. An illusion answers
    /// no: it holds its place, keeps its buffs, sheds what was worked on it,
    /// and waits on the revive move it is put on.
    pub owner_removed_from_combat_on_death: bool,
    /// Set where the power lends a stat and takes it back.
    pub temporary: Option<TemporaryPower>,
    /// The power comes off when whoever worked it dies, which is what Hex,
    /// Guarded and the magic bomb do. A power that does not say so stays.
    #[serde(default)]
    pub removed_when_applier_dies: bool,
    /// The power answers for every power its owner carries when that owner
    /// dies, which is what an illusion does: a debuff goes, and everything
    /// else stays.
    #[serde(default)]
    pub keeps_owners_powers_on_death: bool,
    /// This power stays on a dead owner even when the rest of them go.
    #[serde(default)]
    pub survives_owner_death: bool,
    /// What block did not stop is taken by this power's own owner — the pet —
    /// instead of by the player it belongs to.
    ///
    /// Die For You is the only power in the pinned build that redirects a
    /// blow, and nothing else ever will. Its three tests are the ones made
    /// here: the blow is aimed at the pet's owner, the pet is still standing,
    /// and the damage is a powered attack.
    #[serde(default)]
    pub redirects_owners_powered_attacks: bool,
    /// The power keeps a fight from ending: every listener is asked before a
    /// fight is called over, so a power that says so keeps the fight standing
    /// with nothing primary left alive.
    #[serde(default)]
    pub stops_combat_from_ending: bool,
    /// The power keeps its owner in the fight and puts it on no move at all,
    /// because it does nothing at all as its owner dies: a pet's corpse
    /// stands exactly where it stood, still showing the unset move it has shown
    /// all fight, until a summon brings it back.
    ///
    /// It is the difference between the two ways a power can hold a dead owner.
    /// An illusion answers `AfterDeath` and puts itself on a revive move; this
    /// one answers nothing and only declares that the creature is not removed.
    #[serde(default)]
    pub holds_owner_where_it_stands: bool,
    /// A Reattach's test: the owner is only put on its holding move while
    /// another creature carrying this same power is still standing.
    /// The last of them to die is not held, and the fight is over.
    #[serde(default)]
    pub holds_owner_only_with_living_kin: bool,
    /// The move its owner is put on when it dies while this power keeps it in
    /// the combat. `None` is the revive move a power builds for itself, which
    /// heals its owner whole; a named state is one the owner's own machine
    /// carries, which is what a Reattach puts a segment on.
    #[serde(default)]
    pub move_on_death: Option<String>,
    /// Whether the move a power puts its owner on has to be performed once
    /// before the machine moves past it, which is what makes a Test Subject's
    /// respawn happen at all.
    #[serde(default)]
    pub must_perform_move_on_death: bool,
    /// Whether this power's owner going down counts as a kill for the cards
    /// that ask for one.
    #[serde(default = "yes")]
    pub owner_death_triggers_fatal: bool,
    /// The block its owner is standing behind is not cleared as a turn opens.
    #[serde(default)]
    pub keeps_block: bool,
    /// Its owner draws nothing, except for the turn's own opening hand draw,
    /// which is let through.
    #[serde(default)]
    pub stops_draws_outside_hand_draw: bool,
    /// The whole hand stays where it is at the end of the turn.
    #[serde(default)]
    pub keeps_hand: bool,
    /// What the power does as it comes off, whether it was taken off outright
    /// or fell away at nothing.
    #[serde(default)]
    pub on_removed: Vec<crate::Command>,
    /// What the power refuses to let be played, when it refuses anything. What
    /// a power says here is a test rather than a run of commands.
    #[serde(default)]
    pub play_restriction: Option<PlayRestriction>,
    /// What this power says a card costs while it stands.
    #[serde(default)]
    pub card_cost_rule: Option<CardCostRule>,
    /// Where a card its owner played is put down, when this power says
    /// something about it.
    #[serde(default)]
    pub played_card_rule: Option<PlayedCardRule>,
    /// A Hex making its owner's cards Ethereal.
    ///
    /// The build hangs that on the card carrying the Hexed affliction, which
    /// the power works onto every card of its owner's in the combat as it
    /// lands and onto every card that enters while it stands, and clears as it
    /// comes off. So while the power stands there is no card of its owner's
    /// that is not afflicted, and what the affliction comes to at this
    /// baseline is exactly this. Ethereal from anywhere else sits on the card
    /// itself and is untouched either way.
    #[serde(default)]
    pub makes_owners_cards_ethereal: bool,
    pub hooks: Vec<ModifierHook>,
}

/// What a power refuses to let be played.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PlayRestriction {
    /// Chains of Binding: a card of its owner's carrying this affliction
    /// cannot be played once one already has been this turn, which the power
    /// latches for itself as a card play begins.
    OneAfflictedCardEachTurn(ModelId),
    /// Smoggy: a card of its owner's carrying this affliction cannot be played
    /// at all while it carries one.
    NoAfflictedCard(ModelId),
    /// Ringing: a card its owner is holding cannot be played once that owner
    /// has already started a card play this turn.
    ///
    /// The build carries this on the cards themselves — the power afflicts
    /// every card in the combat as it lands, and every card that enters while
    /// it is on, and clears them all as it comes off. Nothing projects a card's
    /// affliction, and while the power stands there is no card its owner holds
    /// that is not afflicted, so what the affliction comes to at this baseline
    /// is exactly this: one card a turn.
    OneCardEachTurn,
    /// Sloth: its owner may play as many cards in a turn as the power's own
    /// amount, and no more.
    CardsEachTurn,
    /// Smoggy: a card of this type cannot be played once one of that type
    /// already has been this turn.
    ///
    /// The build carries this on the cards as well: playing a skill afflicts
    /// every skill of its owner's, and every skill that enters afterwards, and
    /// the turn's end clears them all. Nothing projects a card's affliction, so
    /// what it comes to is exactly this.
    OneCardOfTypeEachTurn(CardType),
}

impl PowerDefinition {
    /// The listeners this power answers with, which for a power that lends a
    /// stat includes the shared one: at the end of a turn its owner took part
    /// in, it gives the stat back and takes itself off.
    #[must_use]
    pub fn listener_hooks(&self) -> Vec<ModifierHook> {
        let mut hooks = self.hooks.clone();
        if let Some(temporary) = &self.temporary {
            let sign = if temporary.positive { -1 } else { 1 };
            hooks.push(
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnEnd,
                    vec![
                        crate::Command::Power(crate::PowerCmd::Apply {
                            marks: None,
                            power: temporary.internally_applied_power.clone(),
                            target: crate::CreatureSelector::Owner,
                            applier: Some(crate::CreatureSelector::Owner),
                            amount: crate::Amount::Scaled {
                                base: 0,
                                per: sign,
                                count: crate::Count::ListeningPowerAmount,
                            },
                        }),
                        crate::Command::Power(crate::PowerCmd::Remove {
                            power: crate::PowerRef::listening(),
                        }),
                    ],
                )
                .when(HookCondition::OwnerIsParticipant),
            );
        }
        hooks
    }

    /// What the power reads as while it carries this much. A counter that may
    /// go negative is a debuff while it is down, and a debuff worked backwards
    /// is a buff.
    #[must_use]
    pub const fn type_for_amount(&self, amount: i32) -> PowerType {
        if matches!(self.stack_type, PowerStackType::Counter) && self.allow_negative && amount < 0 {
            return PowerType::Debuff;
        }
        if !self.allow_negative && matches!(self.power_type, PowerType::Debuff) && amount < 0 {
            return PowerType::Buff;
        }
        self.power_type
    }

    /// Whether the power falls away at this amount.
    #[must_use]
    pub const fn should_remove_due_to_amount(&self, amount: i32) -> bool {
        if self.allow_negative {
            amount == 0
        } else {
            amount <= 0
        }
    }
}

/// A potion the player can hold and drink.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PotionDefinition {
    pub model_id: ModelId,
    pub target: TargetType,
    /// The potion's rarity, which decides what a trade for it is worth.
    pub rarity: PotionRarity,
    /// When the belt lets it be drunk at all.
    #[serde(default = "combat_only")]
    pub usage: PotionUsage,
    /// Whether the potion may be generated during a fight. A potion that heals
    /// or resurrects is kept out of the in-combat generation roll, and only
    /// out of that one: a reward, a shop, and an out-of-combat roll can all
    /// still hand it over.
    #[serde(default = "yes")]
    pub can_be_generated_in_combat: bool,
    /// The potion refuses to let its owner die, which is Fairy in a Bottle and
    /// nothing else. A held potion is on the listener list, so the refusal is
    /// asked of the belt in listener order like any other pass, and the potion
    /// drinks itself where it stands.
    #[serde(default)]
    pub prevents_owners_death: bool,
    /// What the potion answers on the shared listener surface while it is
    /// held. A potion in a slot is a listener, sitting behind the relics.
    #[serde(default)]
    pub hooks: Vec<ModifierHook>,
    /// What drinking it does, as commands, which is the same vocabulary a
    /// card is written in.
    pub effects: Vec<crate::Command>,
    /// A foul potion's other half: what it is outside a fight.
    ///
    /// Both what it aims at and whether it may be used at all are worked out
    /// rather than fixed. Out of a fight it aims at something that is not a
    /// creature — a merchant — and it can only be used at all where there is
    /// one to throw it at.
    #[serde(default)]
    pub thrown_at_a_merchant: Option<MerchantThrow>,
}

/// The two things throwing a foul potion at a merchant can come to.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MerchantThrow {
    /// The gold paid where the room is a real merchant's. Whether anything is
    /// standing there to be splashed changes only what is shown, so the gold
    /// is paid either way.
    pub gold: i32,
    /// The event whose merchant it may also be thrown at.
    pub event: ModelId,
    /// The fight that merchant turns into, which the event pushes without
    /// leaving and does not come back from.
    pub encounter: ModelId,
    /// The rug the merchant leaves behind, offered beside every relic still
    /// stocked on the shelf he was holding.
    pub reward_relic: ModelId,
}

const fn combat_only() -> PotionUsage {
    PotionUsage::CombatOnly
}

/// How rare a potion is.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PotionRarity {
    /// What a potion the game no longer has carries. Generation never rolls it,
    /// because it is in no pool generation reads.
    None,
    Common,
    Uncommon,
    Rare,
    Event,
    Token,
}

/// Where a potion may be drunk.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PotionUsage {
    /// Only inside a fight.
    CombatOnly,
    /// Anywhere, a fight included.
    AnyTime,
    /// Never by the player: something else drinks it. Fairy in a Bottle is the
    /// only one, and what drinks it is the death it prevents.
    Automatic,
}

/// A saved string a relic rolls for itself as it is obtained.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RolledSkin {
    /// The name the saved property is written under.
    pub property: String,
    /// The skins it picks between, in the order they are listed.
    pub options: Vec<String>,
}

/// A relic the player can hold. Relics listen on the same surface powers do;
/// a relic whose whole effect happens when it is obtained listens for
/// nothing.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag mirrors one independent property of the pinned relic model"
)]
pub struct RelicDefinition {
    pub model_id: ModelId,
    pub rarity: crate::RelicRarity,
    /// The name this relic's counter is saved under, for relics that keep
    /// one.
    pub counter_property: Option<String>,
    /// How that saved count carries between fights — a cycle that wraps at
    /// a threshold, or a ledger of something the run did — which is what a
    /// setup that does not say the count is read against. Every relic that
    /// saves a count classifies it; the content audit refuses one that does
    /// not.
    #[serde(default)]
    pub counter_kind: Option<crate::CounterKind>,
    /// Whether the counter is written down only once it is something. One
    /// relic of the build works that way — a joss paper, whose count of burnt
    /// cards goes back to nothing every fifth one — and every other saved
    /// counter is written down from the start and carries its zero.
    #[serde(default)]
    pub counter_saved_only_when_set: bool,
    /// Whether its owner holds on to their hand when their turn ends.
    pub keeps_hand: bool,
    /// The hand is kept on the opening turn alone, which is what a ringing
    /// triangle does.
    #[serde(default)]
    pub keeps_first_hand: bool,
    /// What the relic does once it is the player's. It is the one listener
    /// that runs where a relic is picked up rather than where a fight is, so
    /// it is the path by which a selection screen opens outside a combat.
    #[serde(default)]
    pub on_obtained: Vec<crate::Command>,
    pub hooks: Vec<ModifierHook>,
    /// The setup an offer runs on a fresh relic before it is shown: it works
    /// out what the relic would do to this run and saves it as the relic's own
    /// properties, which a recording projects.
    ///
    /// This engine creates the relic when it is obtained rather than when it is
    /// offered, and nothing can change the deck between an ancient building its
    /// options and the player answering, so the setup runs there.
    #[serde(default)]
    pub setup: Option<RelicSetup>,
    /// Room types this relic takes off the table before an unknown map point
    /// is rolled. What a relic says here is a set of removals rather than a
    /// run of commands, so this is the set it removes; a type taken off takes
    /// no part in the roll and no part in the drift that follows it.
    #[serde(default)]
    pub removes_unknown_room_types: Vec<crate::ResolvedRoom>,
    /// While this relic has a use left, the map lets its owner step onto any
    /// point of the next row rather than only
    /// onto the children of the point they stand on. The one relic that says so
    /// spends a use on a step that was not a child's.
    #[serde(default)]
    pub grants_free_travel: bool,
    /// What this relic puts on a card reward screen besides the cards and the
    /// skip.
    #[serde(default)]
    pub card_reward_alternative: Option<CardRewardAlternative>,
    /// The relic does something the moment it is taken, which is one of the
    /// things that stops it being traded away. A relic whose grant is written
    /// where it is obtained rather than on the relic still declares this,
    /// because the flag belongs to the relic rather than to where the grant is
    /// written.
    #[serde(default)]
    pub has_upon_pickup_effect: bool,
    /// The relic spawns pets, which is another of the things that stops it
    /// being traded away.
    #[serde(default)]
    pub spawns_pets: bool,
    /// The other half of the same refusal: a relic that brings a creature with
    /// it is never traded away either.
    #[serde(default)]
    pub adds_pet: bool,
    /// A saved string rolled once as the relic is obtained, off a die local to
    /// the relic and its owner. The two relics that bring a pet each roll
    /// which of four skins it wears. Nothing reads the answer; the projection
    /// carries it.
    #[serde(default)]
    pub rolled_skin: Option<RolledSkin>,
    /// A relic that may be held more than once, which is why obtaining it does
    /// not take it out of either grab bag.
    #[serde(default)]
    pub is_stackable: bool,
    /// Whether a merchant may stock it, which defaults to yes.
    #[serde(default = "yes")]
    pub allowed_in_shops: bool,
    /// A merchant price the relic sets outright rather than taking from its
    /// rarity. The nine relics the fake merchant stocks all say fifty, which
    /// they have to: their rarity is Event, and an event relic's rarity price
    /// is out of reach.
    #[serde(default)]
    pub merchant_cost_override: Option<i32>,
    /// What this relic adds to the X an X-cost play resolves to.
    ///
    /// The X of a play is chained through the combat listeners, and the pinned
    /// build has exactly one thing that answers it, additively. A declared
    /// amount summed over the listeners is that pass; anything that wrote the
    /// value outright would need the pass itself, and nothing does.
    #[serde(default)]
    pub modifies_x_value_by: i32,
    /// The shelf restocks whatever this relic's owner buys off it rather than
    /// clearing the slot. Which entry was bought makes no difference, so all
    /// that is left is who owns the relic.
    #[serde(default)]
    pub restocks_merchant_entries: bool,
    /// What this relic puts on the list a rest site generates.
    #[serde(default)]
    pub adds_rest_site_option: Option<RestSiteOffer>,
    /// The rest of the rest site's options stay standing once one has been
    /// taken, rather than the whole list being cleared. The first listener to
    /// say so settles it, and all such a listener does is test who owns it.
    #[serde(default)]
    pub keeps_rest_site_options_standing: bool,
    /// The energy refill is refused on every turn but the first, so whatever
    /// the turn did not spend carries over.
    #[serde(default)]
    pub keeps_energy_after_first_turn: bool,
    /// The owner's block is not cleared for the turn, and anything over this
    /// is lost instead. Refusing the clear and trimming what is kept are one
    /// thing said once.
    #[serde(default)]
    pub keeps_block_up_to: Option<i32>,
    /// Every combat room pays a potion, whatever the odds rolled.
    #[serde(default)]
    pub forces_potion_reward_in_combat_rooms: bool,
    /// No potion ever reaches its owner's belt, whoever offered it.
    #[serde(default)]
    pub refuses_potions: bool,
    /// While its owner's own side turn is standing, a draw that is not the
    /// turn's opening hand draw is refused. A draw on any other side's turn is
    /// let through, which is what sets it apart from the power that refuses
    /// every one.
    #[serde(default)]
    pub stops_draws_on_its_own_side_turn: bool,
    /// What this relic says a card costs, for the two relics that say
    /// anything.
    #[serde(default)]
    pub card_cost_rule: Option<RelicCostRule>,
    /// Its owner may play as many cards in a turn as this, and no more. The
    /// count is the relic's own, because the build's counts every play rather
    /// than only the ones a player chose.
    #[serde(default)]
    pub cards_playable_each_turn: Option<i32>,
    /// A card of this type joining the deck, or standing on a merchant's
    /// shelf, is replaced by an upgraded clone of itself. The flag is a Molten
    /// Egg's extra refusal to touch a card that is already upgraded, which it
    /// only makes for the deck.
    #[serde(default)]
    pub upgrades_cards_of_type: Option<(crate::CardType, bool)>,
    /// A Fresnel lens: the same two paths, said once. Every card this relic's
    /// owner is offered at a merchant and every card joining their deck is
    /// replaced by a clone carrying this enchantment, where it can go on. The
    /// card is in no pile at either point, so the one test an enchantment
    /// makes about the deck does not apply.
    #[serde(default)]
    pub enchants_cards_it_meets: Option<EnchantmentGrant>,
    /// A lizard tail turning its owner's death away while it has not been
    /// used, and what it then does.
    ///
    /// The two halves are one thing: the relic turns the death away while it
    /// has not been used, and what it then runs is what latches it and puts
    /// its owner back up. The whole listener list is asked whether a creature
    /// should die before it is asked again in the late pass, so a potion that
    /// turns the death away is asked first whatever order the two are held in.
    #[serde(default)]
    pub prevents_owners_death_late: Vec<crate::Command>,
    /// A treasure room hands over nothing at all while the second count the
    /// relic keeps is no more than this — no relic and no gold, and neither is
    /// rolled for.
    #[serde(default)]
    pub suppresses_treasure_while_second_counter_at_most: Option<i32>,
    /// The name this relic's second count is saved under, for the one relic
    /// that keeps two.
    #[serde(default)]
    pub second_counter_property: Option<String>,
    /// The name this relic's boolean is saved under, for the relics whose
    /// latch is saved with the run rather than kept only for the session.
    ///
    /// A saved property is written down whatever it holds, so a relic that
    /// declares one writes it down from the moment it is obtained, `false` and
    /// all. A relic whose boolean is not saved carries none of this and writes
    /// nothing.
    #[serde(default)]
    pub latch_property: Option<String>,
    /// The name this relic's whole list of cards is saved under, for the one
    /// relic that keeps cards away from the deck. A relic that saves cards
    /// one at a time under names of their own carries none of this.
    #[serde(default)]
    pub saved_card_array_property: Option<String>,
    /// A pool this relic puts behind the character's own on every card
    /// reward, unless the offer already draws from it.
    #[serde(default)]
    pub adds_card_pool_to_rewards: Option<ModelId>,
    /// Every character pool the player has unlocked, put in front of the
    /// creation's own: the unlocked pools keep their order and whatever of the
    /// creation's own is new is appended, so the colours come first. It is
    /// skipped where every pool the creation already names is a colourless
    /// one.
    #[serde(default)]
    pub adds_every_character_pool_to_rewards: bool,
    /// What the relic's own count is standing at when it is obtained, which
    /// for a relic given a number of uses is that number.
    #[serde(default)]
    pub initial_counter: i32,
    /// How this relic knows it is used up: a relic with a limited number of
    /// uses that have all been spent. Ten relics answer it, each off a count
    /// or a boolean they keep for themselves, which is what this says against
    /// the counter and the latch the relic carries. It is asked of the relic
    /// the player is holding rather than of the definition.
    #[serde(default)]
    pub used_up: Option<UsedUp>,
    /// Pael's Eye: while this relic has not been spent this fight and its
    /// owner has chosen no card play this turn, its owner takes another turn
    /// instead of handing over to the enemy side. The relic's own latch is
    /// what "spent" means, and taking the extra turn sets it.
    ///
    /// The "played a card" test also answers yes on the opening turn where its
    /// owner holds the relic named here, which is a cross-reference the build
    /// makes outright to the one relic that plays that turn for them.
    #[serde(default)]
    pub grants_an_extra_turn: Option<ExtraTurnRule>,
    /// What this relic does to an act's map as it is generated, and what it
    /// then does in the rooms it marked.
    #[serde(default)]
    pub map_rule: Option<RelicMapRule>,
    /// Whether the relic is allowed in this run at all, which defaults to
    /// yes. A bag drops every relic whose answer is no before it hands one
    /// out, so a relic that stops being
    /// allowed leaves every bucket at the next pull rather than at the moment
    /// it stopped.
    #[serde(default)]
    pub spawn_condition: Option<RelicSpawnCondition>,
}

/// An enchantment a relic puts on a card, and what it is worth.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EnchantmentGrant {
    pub enchantment: ModelId,
    pub amount: i32,
}

/// An option something the player holds adds to a rest site's list.
///
/// Everything in the pinned build that adds one tests who owns it and adds a
/// single option, with Girya alone making a second test against the count it
/// keeps, so what is left of them all is the option and that limit.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RestSiteOffer {
    pub option: crate::RestSiteOption,
    /// Girya: the option is added only while the relic's own count stands
    /// below this.
    #[serde(default)]
    pub while_counter_below: Option<i32>,
}

impl RestSiteOffer {
    #[must_use]
    pub const fn of(option: crate::RestSiteOption) -> Self {
        Self {
            option,
            while_counter_below: None,
        }
    }

    #[must_use]
    pub const fn while_counter_below(mut self, limit: i32) -> Self {
        self.while_counter_below = Some(limit);
        self
    }
}

/// An extra line a relic puts on a card reward screen, and what taking it
/// does.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardRewardAlternative {
    /// Pael's Wing: the reward is thrown away, the relic counts it, and every
    /// `every`th one pays a relic pulled from the front of the player's bag on
    /// a rolled rarity. The selection ends and the reward is completed either
    /// way.
    Sacrifice { every: i32 },
}

/// What a relic does to the map of the act it was taken in.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelicMapRule {
    /// A fur coat: this many of the act's Monster and Elite points are
    /// marked,
    /// shuffled on a stream the relic builds for itself, and every enemy of a
    /// marked room is standing on one HP. The coords are saved, so a map
    /// generated again in the same act carries the same marks; a saved coord
    /// that is no longer one of those two kinds of point re-rolls the whole
    /// set.
    MarksCombatRooms { count: usize },
    /// A golden compass: the act it was taken in is laid out as the golden
    /// path
    /// instead of its own map, generated again the moment the relic is taken,
    /// and every unknown point of that act is an event.
    GoldenPath,
}

/// What an extra turn depends on besides the relic's own state.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExtraTurnRule {
    /// The relic whose opening-turn auto-play counts as its owner having played
    /// a card, whether or not any play finished.
    pub counts_as_played_on_the_first_turn: ModelId,
}

/// How a relic knows it is used up, said against what it keeps for itself.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsedUp {
    /// The count the relic was given has run down to this or below.
    CounterAtMost(i32),
    /// The count has run up to what the relic was given.
    CounterAtLeast(i32),
    /// The relic's own boolean, which this engine keeps as the listening
    /// latch.
    Latched,
    /// The one relic whose used-up test is made of both counts it keeps.
    BothCountsAtLeast { counter: i32, second_counter: i32 },
}

/// What keeps a relic out of a run's bags altogether.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelicSpawnCondition {
    /// Before the act-three treasure chest, which in a single-player run is
    /// floor forty-one. The relics that read it are the ones whose value runs
    /// out late.
    BeforeAct3TreasureChest,
    /// A single-player run, which two ancients ask for. This baseline is
    /// single-player, so it always holds.
    SinglePlayerOnly,
    /// A multiplayer run, which one ancient asks for. This baseline is
    /// single-player, so it never holds and the relic leaves every bag.
    MultiplayerOnly,
}

/// What a relic works out about the run and saves as it is set up.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RelicSetup {
    /// An Archaic Tooth's setup: the deck's starter card and the ancient card
    /// it would become, saved as `StarterCard` and `AncientCard`. The ancient
    /// card is the one the relic makes, which has joined no deck yet.
    Transcendence { upgrades: Vec<(ModelId, ModelId)> },
    /// A Touch of Orobas' setup: the starter relic the player is holding and
    /// the refined form it would be replaced by, saved as `StarterRelic` and
    /// `UpgradedRelic`. A starter with no refined form of its own is worth the
    /// fallback, which is a circlet.
    Refinement {
        upgrades: Vec<(ModelId, ModelId)>,
        fallback: ModelId,
    },
}

impl RelicDefinition {
    /// Whether an event may take this relic back off the player.
    ///
    /// Five things are refused: a relic whose limited uses are spent, one that
    /// does its work the moment it is picked up, one that has been melted, one
    /// that brings a pet, and the three rarities outside the rolled ones —
    /// Starter, Event, and Ancient. Melting and being used up both belong to
    /// the relic the player is holding rather than to the definition, so both
    /// are asked of it.
    #[must_use]
    pub fn is_tradable(&self, instance: &RelicInstance) -> bool {
        !self.is_used_up(instance)
            && !self.has_upon_pickup_effect
            && !instance.melted
            && !self.spawns_pets
            && !matches!(
                self.rarity,
                crate::RelicRarity::Starter
                    | crate::RelicRarity::Event
                    | crate::RelicRarity::Ancient
            )
    }

    /// Whether this relic the player is holding is used up.
    #[must_use]
    pub fn is_used_up(&self, instance: &RelicInstance) -> bool {
        match self.used_up {
            None => false,
            Some(UsedUp::CounterAtMost(bound)) => instance.counter <= bound,
            Some(UsedUp::CounterAtLeast(bound)) => instance.counter >= bound,
            Some(UsedUp::Latched) => instance.latched,
            Some(UsedUp::BothCountsAtLeast {
                counter,
                second_counter,
            }) => instance.counter >= counter && instance.second_counter >= second_counter,
        }
    }

    /// Whether the relic is letting its owner step off the map's own edges,
    /// which it does while it still has a use left.
    #[must_use]
    pub fn allows_free_travel(&self, instance: &RelicInstance) -> bool {
        self.grants_free_travel && !self.is_used_up(instance)
    }

    /// What a shop charges for it, by rarity, unless the relic sets a price of
    /// its own. The three rarities a shop never stocks are priced out of reach
    /// rather than forbidden. The rarity that costs one belongs to the
    /// deprecated and fallback stand-ins that this baseline does not
    /// register.
    #[must_use]
    pub const fn merchant_cost(&self) -> i32 {
        if let Some(override_cost) = self.merchant_cost_override {
            return override_cost;
        }
        match self.rarity {
            crate::RelicRarity::Common => 175,
            crate::RelicRarity::Uncommon => 225,
            crate::RelicRarity::Rare => 275,
            crate::RelicRarity::Shop => 200,
            crate::RelicRarity::Ancient
            | crate::RelicRarity::Starter
            | crate::RelicRarity::Event
            | crate::RelicRarity::None => 999_999_999,
        }
    }
}

/// A relic the player is holding, with the per-run state some relics count in.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RelicInstance {
    pub model_id: ModelId,
    pub counter: i32,
    /// A second count a relic keeps beside the first, which is a joss paper
    /// holding the burns it has to defer until the turn ends.
    #[serde(default)]
    pub second_counter: i32,
    /// The boolean a relic keeps for itself, which is the once-a-fight latch a
    /// centennial puzzle, a permafrost, and an orichalcum all keep.
    #[serde(default)]
    pub latched: bool,
    /// The act a relic that bends a map was taken in, which both such relics
    /// save under a name of their own.
    #[serde(default)]
    pub map_act: Option<usize>,
    /// The points this relic marked on that act's map. The build saves the
    /// columns and the rows as two lists; here they are pairs.
    #[serde(default)]
    pub marked_coords: Vec<crate::MapCoord>,
    /// The card an unsettling lamp bound for the fight, which is not saved
    /// with the run and goes with the combat. It is what the lamp doubles for
    /// and nothing else.
    #[serde(default)]
    pub bound_card: Option<CardInstanceId>,
    /// The power the binding armed on, which is the only one ever held — a
    /// lamp with a card already bound arms on nothing more, so nothing is
    /// added after the first. It is read to keep the doubling off the stat a
    /// power that lends one works, which would otherwise be doubled a second
    /// time and come out four times over.
    #[serde(default)]
    pub bound_power: Option<ModelId>,
    /// The relic is still in the inventory and does nothing at all. It stops
    /// listening and stops being tradable.
    #[serde(default)]
    pub melted: bool,
    /// The floor the relic was obtained on.
    pub floor_added: Option<u32>,
    /// The cards a relic that was set up for this player saved, in the order
    /// the projection lists them, which is by name.
    #[serde(default)]
    pub saved_cards: Vec<(String, CardFingerprint)>,
    /// The models a relic that was set up for this player saved, the same way.
    /// A dusty tome's ancient card is the one this baseline reaches.
    #[serde(default)]
    pub saved_models: Vec<(String, ModelId)>,
    /// A relic a toy box handed over, which that toy box melts again after
    /// enough fights. It is nothing but a saved boolean; what reads it is
    /// whatever melts one.
    #[serde(default)]
    pub wax: bool,
    /// The strings a relic saved, by name. The two relics that bring a pet
    /// each roll which of four skins it wears as they are obtained, and the
    /// answer is saved with the run — nothing else reads it, but the
    /// projection carries it.
    #[serde(default)]
    pub saved_strings: Vec<(String, String)>,
}

impl RelicInstance {
    #[must_use]
    pub const fn new(model_id: ModelId) -> Self {
        Self {
            model_id,
            counter: 0,
            second_counter: 0,
            latched: false,
            map_act: None,
            marked_coords: Vec::new(),
            bound_card: None,
            bound_power: None,
            melted: false,
            floor_added: None,
            saved_cards: Vec::new(),
            saved_models: Vec::new(),
            saved_strings: Vec::new(),
            wax: false,
        }
    }

    #[must_use]
    pub const fn obtained_on(mut self, floor: u32) -> Self {
        self.floor_added = Some(floor);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PowerInstance {
    pub model_id: ModelId,
    /// Which instance of the model this is, and the only thing that tells two
    /// of them apart.
    ///
    /// A power that winds itself down winds down the very one whose listener
    /// is running rather than the first one its owner happens to carry. A
    /// creature can carry several of an instanced power — two bombs on
    /// different fuses is the case that shows it — so the listener list names
    /// the instance and every command that acts on "this power" acts on that
    /// one. It is handed out per combat and is not projected.
    #[serde(default)]
    pub instance: u64,
    /// A card this power is holding, which is how a thief carries what it took.
    pub held_card: Option<CardFingerprint>,
    /// The card the one swing this power bound came from, which is what a
    /// Vigor keeps.
    ///
    /// The build binds the swing itself and reads two things off it — the card
    /// it came from and who is swinging — and never clears the binding, so a
    /// power that survives the swing it bound goes on adding to attacks from
    /// that same card and refuses every other. The card is the whole of what
    /// the refusal reads, because who is swinging is already covered by the
    /// listener's own test that its owner is dealing the damage.
    #[serde(default)]
    pub bound_damage_card: Option<CardInstanceId>,
    pub amount: i32,
    /// Per-instance state a few powers count separately from their stacks.
    pub counter: i32,
    pub amount_on_turn_start: i32,
    /// What this power was worth when each card play now running began, one
    /// entry per play, innermost last.
    ///
    /// It is what an Afterimage, a Serpent Form and a Strangle each keep per
    /// card being played: the entry is written as the play begins and taken
    /// back out as it finishes, so a power applied while a play was already
    /// running is owed nothing for it, and a second application mid-play does
    /// not pay twice. Plays nest last-in first-out — a Sly card discarded by a
    /// play is played inside it — which is what makes a stack the right shape
    /// for it.
    #[serde(default)]
    pub amounts_at_play_start: Vec<i32>,
    /// What one in-flight play of its owner's has run up so far, one entry per
    /// play, which is what a Rupture keeps.
    ///
    /// The entry is opened by the power itself as the play begins rather than
    /// by the pass, and `None` is a play the power declined — nothing is
    /// written down for it, and a play with nothing written down is one the
    /// power finds nothing for when the play ends. Plays nest last-in
    /// first-out, so a stack is the right shape for it.
    #[serde(default)]
    pub play_tallies: Vec<Option<i32>>,
    /// Set when a debuff lands on a player-side creature, and consumed by the
    /// next duration tick. It is why a debuff applied on the turn it would wear
    /// off lasts through the following one instead.
    pub skip_next_duration_tick: bool,
    pub applier: Option<CreatureCombatId>,
    /// A creature the power is about that is not the one carrying it. The
    /// Insatiable's sandpit is the one this baseline reaches —
    /// it sits on the boss and counts down to eating the player.
    #[serde(default)]
    pub target: Option<CreatureCombatId>,
    pub power_type: PowerType,
    /// The cards a Dampen took the upgrades off, and the level each was
    /// standing on, in the order it took them.
    #[serde(default)]
    pub downgraded_cards: Vec<(CardInstanceId, u8)>,
    /// The boolean a power keeps for itself, which at this baseline is a
    /// Chains of Binding remembering that its one card has been played.
    #[serde(default)]
    pub latched: bool,
}

/// One line of a monster's advertised move. A move can advertise several: an
/// attack that also defends shows both.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MoveIntent {
    pub intent_type: String,
    pub damage: Option<i32>,
    pub repeats: Option<u32>,
    /// A power of the mover's own whose amount is part of the damage before
    /// anything modifies it. Such an intent is worked out afresh every time it
    /// is shown, and what it reads is a written number plus a power the
    /// creature is holding.
    #[serde(default)]
    pub scaled_by: Option<ModelId>,
    /// One more blow for every time past the first the mover has stood on this
    /// state, which is what a Test Subject's multi-claw shows.
    #[serde(default)]
    pub extra_repeats_per_entry_of: Option<String>,
    /// A number the mover wrote down for itself that the intent reads instead
    /// of a written damage — a pressure gun's current damage is the one — and
    /// which is read afresh every time the intent is shown.
    #[serde(default)]
    pub damage_from_local: Option<String>,
}

impl MoveIntent {
    #[must_use]
    pub fn attack(damage: i32, repeats: u32) -> Self {
        Self {
            intent_type: "attack".to_owned(),
            damage: Some(damage),
            repeats: Some(repeats),
            scaled_by: None,
            extra_repeats_per_entry_of: None,
            damage_from_local: None,
        }
    }

    /// A blow that is the last thing the mover does, which is shown as its own
    /// kind of intent.
    #[must_use]
    pub fn death_blow(damage: i32) -> Self {
        Self {
            intent_type: "death_blow".to_owned(),
            damage: Some(damage),
            repeats: Some(1),
            scaled_by: None,
            extra_repeats_per_entry_of: None,
            damage_from_local: None,
        }
    }

    /// A blow that lands once more every time its state comes round again.
    #[must_use]
    pub fn attack_growing(damage: i32, repeats: u32, state_id: &str) -> Self {
        Self {
            extra_repeats_per_entry_of: Some(state_id.to_owned()),
            ..Self::attack(damage, repeats)
        }
    }

    /// One blow whose own damage grows with a power the mover is holding.
    #[must_use]
    pub fn attack_scaled(damage: i32, power: ModelId) -> Self {
        Self {
            scaled_by: Some(power),
            ..Self::attack(damage, 1)
        }
    }

    #[must_use]
    pub fn of(intent_type: &str) -> Self {
        Self {
            intent_type: intent_type.to_owned(),
            damage: None,
            repeats: None,
            scaled_by: None,
            extra_repeats_per_entry_of: None,
            damage_from_local: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Intent {
    pub move_id: String,
    pub intents: Vec<MoveIntent>,
}

impl Intent {
    #[must_use]
    pub fn new(move_id: &str, intents: Vec<MoveIntent>) -> Self {
        Self {
            move_id: move_id.to_owned(),
            intents,
        }
    }

    /// The damage the move advertises, which is the first attacking line.
    #[must_use]
    pub fn damage(&self) -> Option<i32> {
        self.intents
            .iter()
            .find(|intent| intent.damage.is_some())
            .and_then(|intent| intent.damage)
    }
}

/// How a creature's HP reads, and whether it reads as infinite.
///
/// The difference between the two infinite kinds is what the bar shows, and
/// nothing in the rules reads it. What the rules read is whether the HP is
/// infinite at all, which both of them are: a hellraiser caps its automatic
/// plays against a creature whose HP reads that way.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HpDisplay {
    #[default]
    Normal,
    InfiniteWithNumbers,
    InfiniteWithoutNumbers,
}

impl HpDisplay {
    /// Whether the HP reads as infinite; both infinite kinds answer yes.
    #[must_use]
    pub const fn is_infinite(self) -> bool {
        matches!(
            self,
            Self::InfiniteWithNumbers | Self::InfiniteWithoutNumbers
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each is one of the build's own per-creature flags"
)]
pub struct Creature {
    pub id: CreatureCombatId,
    pub model_id: ModelId,
    pub side: CombatSide,
    pub current_hp: i32,
    pub max_hp: i32,
    pub block: i32,
    /// Whether this creature's HP reads as a number or as infinite. A
    /// hardened shell writes it once a turn has cost its owner
    /// everything the shell holds back, and a waterfall giant writes it as it
    /// stands up to blow.
    #[serde(default)]
    pub hp_display: HpDisplay,
    /// How many blows this creature has taken in this combat that block did
    /// not stop whole, which is what a Tear Asunder reads out of the combat
    /// history.
    #[serde(default)]
    pub unblocked_blows_taken: u32,
    pub powers: Vec<PowerInstance>,
    pub intent: Option<Intent>,
    /// Current state machine node, which is a move state whenever the creature
    /// is showing an intent.
    pub move_state: String,
    /// True once the creature has performed a move, matching the state
    /// machine's first-move transition guard.
    pub performed_first_move: bool,
    /// The creature arrived during the side turn now running. A monster that
    /// did is skipped when its side acts, and the flag is cleared as the sides
    /// switch, so an arrival never acts on the turn it
    /// arrived in and always acts on the next one.
    #[serde(default)]
    pub spawned_this_turn: bool,
    /// Whether the creature has already been through the death its holding
    /// power answered: the death is announced once, where the creature went
    /// down.
    #[serde(default)]
    pub death_handled: bool,
    /// The state a stun told the creature to pick its machine back up on.
    /// `None` means the last move it performed.
    #[serde(default)]
    pub stunned_follow_up: Option<String>,
    /// What the stunned turn performs. It is held on the creature because the
    /// stun writes it and the turn after performs it.
    #[serde(default)]
    pub stunned_move: Vec<crate::Command>,
    /// Which of its encounter's named slots the creature stands in, and
    /// `None` for one the encounter named no slot for.
    #[serde(default)]
    pub slot_name: Option<String>,
    /// Where that name stands in the encounter's slot list, worked out as the
    /// creature joins. It is the order the side takes its turns in and the
    /// order a random target is rolled over: the enemy list is sorted by it
    /// every time a creature with a slot joins, and −1 — where the encounter
    /// does not list the name at all — sorts in front of all of them.
    #[serde(default)]
    pub slot: i32,
    /// The player whose pet this creature is, named by that player's own
    /// creature. It is what being a pet comes to, it is written once as the
    /// pet joins and never again, and it is `None` for everything that is not
    /// a pet — every player and every enemy.
    ///
    /// A pet stands on its owner's side of the fight and is a creature in
    /// every other respect: it takes damage, carries powers, and answers every
    /// listener pass. What it is not is a player, so the projection's `player`
    /// is nobody for it and this is what the projection's `pet_owner` names
    /// instead.
    #[serde(default)]
    pub pet_owner: Option<CreatureCombatId>,
    /// Whether a bowlbug rock is off balance, which a power writes and the
    /// monster's own move reads and clears.
    #[serde(default)]
    pub off_balance: bool,
    /// The monster a fabricator's last rolled spawn settled on, which the next
    /// one leaves out of its list.
    #[serde(default)]
    pub last_spawned: Option<ModelId>,
    /// What this creature has written down for the fight — the damage a
    /// pressure gun has worked up to, the pressure a giant is about to blow
    /// with — none of which is saved or projected.
    #[serde(default)]
    pub locals: std::collections::BTreeMap<String, i32>,
    /// Loggable states in roll order. Repeat and cooldown weights read it.
    pub state_log: Vec<String>,
    /// The move states this creature has actually performed, in order. A move
    /// is entered when the machine settles on it — which is a turn before it
    /// runs — so anything counting how many times it has done something
    /// reads
    /// this rather than the log of what it stands on.
    #[serde(default)]
    pub performed_log: Vec<String>,
}

impl Creature {
    #[must_use]
    pub const fn alive(&self) -> bool {
        self.current_hp > 0
    }

    /// Whether this creature is somebody's pet, which is whether it has an
    /// owner.
    #[must_use]
    pub const fn is_pet(&self) -> bool {
        self.pet_owner.is_some()
    }

    #[must_use]
    pub fn power_amount(&self, model_id: &str) -> i32 {
        self.powers
            .iter()
            .filter(|power| power.model_id.to_string() == model_id)
            .map(|power| power.amount)
            .sum()
    }
}

/// How many characters a run could be offered another character's keepsake
/// for: every character but the one being played, with every character epoch
/// revealed.
pub const CHARACTERS_BESIDES_THE_RUNS: usize = 4;

/// Tags a card carries for rules that count cards by kind.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardTag {
    Strike,
    Defend,
    /// What a card a summoned minion leaves behind is, which a relic that pays
    /// for them looks up.
    Minion,
    Shiv,
    /// What one of the Necrobinder's pet attacks is.
    OstyAttack,
}

/// What a card itself refuses to let be played while it is held.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardPlayRestriction {
    /// Enthralled: while it is in its owner's hand, the only card of that
    /// owner's the player may choose to play is another Enthralled. A card
    /// played by something other than the player is let through.
    OnlyItself,
    /// Normality: while it is in its owner's hand, that owner may start this
    /// many card plays in a turn and no more.
    CardsEachTurn(i32),
}

/// What changes when a card is upgraded.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag is one keyword an upgrade adds or removes"
)]
pub struct CardUpgrade {
    pub energy_cost: Option<i32>,
    /// Upgrading adds Innate, which is the whole of what upgrading some cards
    /// does.
    #[serde(default)]
    pub innate: bool,
    /// Upgrading adds Retain.
    #[serde(default)]
    pub retain: bool,
    /// Upgrading removes a keyword the base form carries. Upgrades only ever
    /// drop Exhaust or Ethereal, and each is a keyword of the card's own
    /// rather than something that can be put back.
    #[serde(default)]
    pub drops_exhaust: bool,
    #[serde(default)]
    pub drops_ethereal: bool,
    /// What the upgraded form's own `Damage` var is worth, where the card
    /// declares one.
    #[serde(default)]
    pub damage_var: Option<i32>,
    pub effects: Vec<crate::Command>,
}

/// What a quest card in the deck says about the act it is carried through.
///
/// The Lantern Key is the whole of it: while the run is walking the act it
/// names, every unknown map point is the room type it names, and the next
/// event the act pulls is the event it names.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct QuestHooks {
    /// The act index the card answers for.
    pub act: usize,
    /// The only room type an unknown map point may become while the card is
    /// carried. The roll is still read — the die is taken before the table is
    /// walked — and a table with nothing left on it leaves the answer where it
    /// started, so the drift does not move either.
    pub unknown_room_type: crate::ResolvedRoom,
    /// What the act's next event becomes.
    pub next_event: ModelId,
}

/// What a quest card standing in the run deck does to an act's map.
///
/// The Spoils Map is the one card of the pinned build shaped this way, and
/// every one of the four things it does makes the same pair of tests first:
/// the act being generated is the one the card's saved property names, and the
/// card is standing in the deck rather than in a fight.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CardMapHooks {
    /// The saved property holding the act index the card answers for.
    pub act_property: String,
    /// The map the card lays out in place of the one the act would have.
    pub replaces_the_map_with: SpecialMap,
    /// The first treasure point of that map is remembered and marked for this
    /// card once the map is laid out.
    pub marks_the_first_treasure: bool,
    /// What walking onto the marked point pays, on top of what the treasure
    /// room itself pays. The card leaves the deck for it.
    pub gold_for_the_marked_point: i32,
}

/// The two maps the build lays out from scratch rather than by bending the
/// act's own, as the engine reaches them: the content crate owns the
/// generation and the registry carries it, the way it carries every other
/// thing a model names.
#[derive(Clone, Copy)]
pub struct SpecialMapBuilders {
    /// The spoils map, which reads a stream of its own.
    pub spoils: fn(&str, usize, u8) -> crate::ActMap,
    /// The golden path, which reads no dice at all.
    pub golden_path: fn() -> crate::ActMap,
}

impl std::fmt::Debug for SpecialMapBuilders {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SpecialMapBuilders")
    }
}

/// A map the build lays out from scratch rather than by bending the act's own.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecialMap {
    /// The hourglass every path runs through one centred treasure point of,
    /// on a stream of its own.
    Spoils,
    /// One column of sixteen fixed points, which reads no dice at all.
    GoldenPath,
}

/// What a card costs in stars when it says nothing about stars at all.
pub(crate) const fn no_star_cost() -> i32 {
    -1
}

/// One saved property a card declares: its name and what a freshly made card
/// of that model holds in it.
///
/// Every declared property is written down whatever it holds, so a card that
/// has never been played still carries its starting values. A card of a model
/// that declares one is therefore never written down without it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SavedCardProperty {
    pub name: String,
    pub initial: i32,
}

impl SavedCardProperty {
    #[must_use]
    pub fn of(name: &str, initial: i32) -> Self {
        Self {
            name: name.to_owned(),
            initial,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag mirrors one independent property of the pinned card model"
)]
pub struct CardDefinition {
    pub model_id: ModelId,
    /// Whether the card burns up if it is still in hand when the turn ends.
    pub ethereal: bool,
    /// Whether the card can never leave the deck. A card carrying Eternal is
    /// not removable, and a removal screen hides every card that is not.
    pub eternal: bool,
    /// Whether the card cannot be played at all, which is what a status or a
    /// curse usually is. Such a card carries a cost of -1.
    pub unplayable: bool,
    /// What the card does to its holder if the turn ends with it still in hand.
    pub end_of_turn_in_hand: Vec<crate::Command>,
    /// What the card counts as when something sorts a deck by rarity, such as
    /// a thief choosing what to take.
    pub rarity: CardRarity,
    pub tags: Vec<CardTag>,
    /// The card's upgraded form, when it has one.
    pub upgrade: Option<CardUpgrade>,
    pub supported_upgrade_levels: Vec<u8>,
    pub allow_custom_properties: bool,
    /// Whether the card gains block, which the card declares about itself
    /// rather than anything worked out from what it does. Only the Nimble
    /// enchantment reads it.
    #[serde(default)]
    pub gains_block: bool,
    /// The saved properties the card declares. A fingerprint may carry these
    /// and nothing else: a property no model declares is a variant this
    /// baseline does not support.
    #[serde(default)]
    pub saved_properties: Vec<SavedCardProperty>,
    /// What the one quest card that steers an act says about its unknown map
    /// points and its next event. A card in the run deck answers both wherever
    /// it stands, which is how a key kept out of an event turns the last act's
    /// unknown points into the rooms it wants.
    #[serde(default)]
    pub quest_hooks: Option<QuestHooks>,
    /// The whole of what one quest card does to an act's map while it stands
    /// in the run deck.
    #[serde(default)]
    pub map_hooks: Option<CardMapHooks>,
    pub card_type: CardType,
    /// A Mad Science: the card's type is read off one of its own saved
    /// properties rather than declared, by the number that property holds,
    /// counted from no type at all.
    ///
    /// A card that works this way reads two more things off the same answer:
    /// what it aims at, which is one enemy for an attack and itself otherwise,
    /// and whether it gains block, which it does only as a skill. A property
    /// holding none of the three playable numbers reads here as the type the
    /// card was defined with, which is what an untinkered card does nothing
    /// as.
    #[serde(default)]
    pub type_from_property: Option<String>,
    pub target: TargetType,
    /// A Shiv: the card aims at the whole enemy side instead of at one of them
    /// while its owner is carrying this power. It is the only card of the
    /// build whose target is worked out rather than fixed.
    #[serde(default)]
    pub target_widens_with: Option<ModelId>,
    pub energy_cost: i32,
    /// What the card costs of the Regent's own second resource. Nothing is
    /// `-1` rather than zero, and the two are different things: a card at `-1`
    /// costs no stars at all, shows none, and is passed over by the star-cost
    /// pass, which answers a negative cost without asking anybody. Nothing in
    /// the pinned build moves it on an upgrade, so it belongs to the card
    /// rather than to the form the card is in.
    #[serde(default = "no_star_cost")]
    pub star_cost: i32,
    /// The card keeps itself: a play that would put it in the discard pile
    /// leaves it in the hand instead, and any other answer — a burn, a power
    /// leaving the combat — stands. One card of the pinned build works this
    /// way.
    #[serde(default)]
    pub returns_to_hand: bool,
    /// The card's star cost is X. Playing it spends every star its owner has
    /// left and writes what it spent onto the card, and its cost reads as the
    /// pool itself without the cost pass running at all, so nothing ever
    /// refuses the play for stars and no cost rule can move it.
    #[serde(default)]
    pub costs_star_x: bool,
    /// Innate: the card is put on top of the draw pile before the opening hand
    /// is drawn, and the opening draw is at least as many cards as there are
    /// innate ones.
    #[serde(default)]
    pub innate: bool,
    /// Retain: the card is not put down when the hand is flushed at the end of
    /// the player's turn.
    #[serde(default)]
    pub retain: bool,
    /// Sly: discarding the card plays it for free, once every card of the
    /// batch it was discarded in has been put down and whatever draw came with
    /// them has happened.
    #[serde(default)]
    pub sly: bool,
    /// The card's energy cost is X. Such a card's canonical cost is zero
    /// whatever it was defined with, so it can always be played; playing it
    /// spends every point of energy its owner has left and captures what was
    /// spent.
    #[serde(default)]
    pub costs_x: bool,
    pub exhausts: bool,
    /// What the card itself refuses to let be played, when it refuses
    /// anything. Both cards of the build that refuse anything say it as a test
    /// rather than as a run of commands, and both ask that the listening card
    /// is standing in its owner's hand.
    #[serde(default)]
    pub play_restriction: Option<CardPlayRestriction>,
    /// The extra condition the card itself puts on its own play, which a Clash
    /// writes as every card in hand being an attack. A card that fails it is
    /// unplayable for that reason and no other.
    #[serde(default)]
    pub playable_when: Option<crate::Condition>,
    /// What the card's own damage number is worth before anything grows it.
    /// Only the cards that read another card's damage need it, and only the
    /// cards that declare one carry it.
    #[serde(default)]
    pub damage_var: Option<i32>,
    pub effects: Vec<crate::Command>,
    /// The pool a transform of this card rolls its replacement against.
    ///
    /// The build works it out where the transform happens — the card's own
    /// pool, unless it is a quest card or carries the Event, Ancient, or Token
    /// rarity, all of which roll against the colourless pool instead. It
    /// depends on nothing that changes during a run, so it is settled where
    /// the card is registered. A card whose pool this baseline does not
    /// register carries nothing here, and transforming it fails as unsupported
    /// content.
    #[serde(default)]
    pub transform_pool: Option<ModelId>,
    /// The pool the card is read as belonging to, which is the first pool
    /// whose membership holds it — except for the ten event cards that name a
    /// character's pool instead. It is not the transform pool: a quest card,
    /// an ancient, an event card, and a token all transform against the
    /// colourless pool while standing in a pool of their own.
    ///
    /// All that is read off it is whether the pool is colourless, which is the
    /// whole of what a filter asking for a colourless card tests.
    #[serde(default)]
    pub visual_pool: Option<ModelId>,
    /// What this card puts on the list a rest site generates. One card in the
    /// pinned build puts anything there.
    #[serde(default)]
    pub adds_rest_site_option: Option<RestSiteOffer>,
    /// What the card answers as a listener. A card is a listener wherever it
    /// stands: the build puts every card of every combat pile on the combat
    /// list and every card of the run deck in front of the run list, and its
    /// only liveness guard is whether the card has been removed from the state.
    pub hooks: Vec<ModifierHook>,
}

impl CardDefinition {
    /// Whether the form at this upgrade level burns up when it is played,
    /// which is the card's own keyword unless upgrading removes it.
    #[must_use]
    pub fn exhausts_at(&self, upgrade_level: u8) -> bool {
        if upgrade_level > 0
            && self
                .upgrade
                .as_ref()
                .is_some_and(|upgrade| upgrade.drops_exhaust)
        {
            return false;
        }
        self.exhausts
    }

    /// Whether the form at this upgrade level burns up if it is still held when
    /// the turn ends.
    #[must_use]
    pub fn ethereal_at(&self, upgrade_level: u8) -> bool {
        if upgrade_level > 0
            && self
                .upgrade
                .as_ref()
                .is_some_and(|upgrade| upgrade.drops_ethereal)
        {
            return false;
        }
        self.ethereal
    }

    /// Whether the form at this upgrade level is kept in hand when the turn
    /// ends.
    #[must_use]
    pub fn retains_at(&self, upgrade_level: u8) -> bool {
        if upgrade_level > 0 && self.upgrade.as_ref().is_some_and(|upgrade| upgrade.retain) {
            return true;
        }
        self.retain
    }
}

/// One orb model.
///
/// An orb says what its passive and its evoke are worth, which of the two turn
/// triggers it answers, and what each of the two does. It is a combat listener
/// in its own right, but no orb in the pinned build listens for anything, so
/// the definition carries no listeners.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrbDefinition {
    pub model_id: ModelId,
    /// What the orb's passive is worth.
    pub passive_value: OrbValue,
    /// What the orb's evoke is worth.
    pub evoke_value: OrbValue,
    /// Which of the two turn triggers the orb answers. Every orb in the pinned
    /// build answers exactly one, and each of them answers it by firing its
    /// own passive.
    pub turn_trigger: OrbTrigger,
    /// What the passive does.
    pub passive: Vec<crate::Command>,
    /// What the evoke does. What it reaches is what the evoke announcement
    /// then carries.
    pub evoke: Vec<crate::Command>,
}

/// Where an orb's own number comes from.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrbValue {
    /// The orb's own number returned outright, with no pass over it: a Plasma
    /// orb's one and two, and the number a Dark orb grows. An orb whose number
    /// is a constant and one whose number is a value it writes are the same
    /// shape — the constant is simply a value nothing writes.
    Fixed(i32),
    /// The same number, put through the pass that modifies orb values.
    Modified(i32),
    /// A multiple of the passive, worked out after the passive's own pass
    /// rather than before, which is what a Glass orb's evoke of twice its
    /// passive comes to.
    PassiveTimes(i32),
}

impl OrbValue {
    /// What the orb is stood up holding, before anything has written to it.
    #[must_use]
    pub const fn base(self) -> i32 {
        match self {
            Self::Fixed(value) | Self::Modified(value) => value,
            Self::PassiveTimes(_) => 0,
        }
    }
}

/// Which turn boundary an orb's passive fires at on its own.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrbTrigger {
    /// At the very top of the turn's end — before the hand's own end-of-turn
    /// effects and before the flush.
    BeforeTurnEnd,
    /// At the start of the turn, after the side's turn has been announced and
    /// before the auto-pre-play phase, so a Plasma orb's energy lands after
    /// the reset that refilled it.
    AfterTurnStart,
}

/// One enchantment model.
///
/// An enchantment lives on a card: it says what cards it may go on, changes
/// the card as it lands, adds an effect to the card's play, answers the five
/// questions that stand in front of every other modifier, and listens right
/// behind the card carrying it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EnchantmentDefinition {
    pub model_id: ModelId,
    /// Whether a card may carry more than one. Nothing in the pinned build
    /// may, so a card that already carries any enchantment refuses every one.
    #[serde(default)]
    pub stackable: bool,
    /// What this enchantment adds on top of the test every enchantment shares.
    pub restriction: EnchantRestriction,
    /// The card is sorted to the bottom of the draw pile as the combat opens.
    #[serde(default)]
    pub starts_at_bottom_of_draw_pile: bool,
    /// What the enchantment does to the card it lands on. The build works it
    /// out again whenever the card's values are rebuilt — after a downgrade,
    /// after a load — so it is derived from the fingerprint here rather than
    /// written into the card once.
    #[serde(default)]
    pub on_enchant: Vec<CardChange>,
    /// What runs after the card's own effect, once per play of the card,
    /// before the affliction's own effect and before the card-played
    /// announcement.
    #[serde(default)]
    pub on_play: Vec<crate::Command>,
    /// The five things an enchantment answers about its card: damage
    /// additively and multiplicatively, block the same two ways, and the play
    /// count.
    ///
    /// These are not passes over the listener list. They are asked of the
    /// enchantment on the card the value came from, in front of every
    /// listener, which is why they are held apart from `hooks`.
    #[serde(default)]
    pub value_hooks: Vec<ModifierHook>,
    /// What the enchantment answers as a listener. It stands on the combat
    /// listener list right behind the card it is on, and on the run list right
    /// behind that card's deck version.
    #[serde(default)]
    pub hooks: Vec<ModifierHook>,
}

/// What an enchantment adds to the test every enchantment shares.
///
/// That shared test is the same for all of them — no status, curse, or quest
/// card; no unplayable card that is standing in the run deck; no card that
/// already carries an enchantment — so only the extra test each enchantment
/// makes is named here.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnchantRestriction {
    /// Nothing beyond the shared test.
    Any,
    /// One card type, which is what Sharp, Instinct, Corrupted, Momentum, and
    /// Vigorous ask for attacks and Imbued for skills.
    OfType(CardType),
    /// An attack or a skill, which is what Royally Approved asks.
    AttackOrSkill,
    /// The card carries the tag, which is what Goopy asks.
    Tagged(CardTag),
    /// The card gains block, which is what Nimble asks — and which a card
    /// declares about itself rather than anything worked out from what it
    /// does.
    GainsBlock,
    /// The card's own keywords hold Exhaust, which is what Souls asks. It is
    /// the card's own set, not one a power lent it.
    LocallyExhausts,
    /// A basic card tagged Strike or Defend, which is what Spiral asks.
    BasicStrikeOrDefend,
    /// Not unplayable wherever it stands, and its cost is not X, which is what
    /// Slither asks.
    PlayableWithAFixedCost,
}

/// One change an enchantment makes to the card it lands on.
///
/// The build applies these once, as the enchantment lands, and again whenever
/// the card's values are rebuilt. Since they depend on nothing but the
/// enchantment, this engine reads them off the fingerprint wherever the card's
/// keywords or cost are asked for, which comes to the same thing and cannot
/// fall out of step.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardChange {
    /// Puts a keyword on the card.
    AddKeyword(CardKeyword),
    /// Takes a keyword off the card.
    RemoveKeyword(CardKeyword),
    /// The card's own cost becomes nothing. An upgrade cannot take a cost
    /// below nothing, so a later upgrade cannot take it further.
    CostBecomesFree,
}

/// The card keywords something moves: an enchantment puts one on or takes one
/// off, and a keyword can also be applied to a single copy rather than to
/// every card of the model.
///
/// Only the keywords something covered actually moves are here; a keyword a
/// card merely declares is a flag on the card definition.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardKeyword {
    Exhaust,
    Innate,
    Retain,
    Eternal,
    /// The card burns up if it is still held when the turn ends.
    Ethereal,
    /// Discarding the card plays it for free once the whole batch it was
    /// discarded in has been put down, and whatever draw came with them has
    /// happened.
    Sly,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MonsterMove {
    pub state_id: String,
    pub intent: Intent,
    /// What performing the move does, as commands.
    pub commands: Vec<crate::Command>,
    /// State entered after this move performs. An empty follow-up returns to
    /// the machine's initial state.
    pub follow_up: String,
}

/// How often a random branch may select the same state.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum MoveRepeat {
    CanRepeatForever,
    CannotRepeat,
    CanRepeatTimes(usize),
    UseOnlyOnce,
}

/// A branch weight the build works out fresh every time the branch is weighed.
///
/// A branch may carry a weight that is worked out rather than fixed, and every
/// covered monster's is one question about the fight with a weight for each
/// answer. It is written here as that question and the weight for a no; the
/// branch's own `weight_bits` is the weight for a yes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConditionalWeight {
    pub condition: MoveCondition,
    /// As `f32` bits, so state comparison stays exact.
    pub otherwise_bits: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MoveBranch {
    pub state_id: String,
    pub repeat: MoveRepeat,
    pub cooldown: usize,
    /// Selection weight as `f32` bits so state comparison stays exact.
    pub weight_bits: u32,
    /// The weight this branch is weighed with where the weight is worked out
    /// rather than fixed. `None` is a fixed weight, which is what almost every
    /// branch carries.
    #[serde(default)]
    pub weight_when: Option<ConditionalWeight>,
}

impl MoveBranch {
    #[must_use]
    pub fn new(state_id: impl Into<String>, repeat: MoveRepeat) -> Self {
        Self {
            state_id: state_id.into(),
            repeat,
            cooldown: 0,
            weight_bits: 1.0_f32.to_bits(),
            weight_when: None,
        }
    }

    #[must_use]
    pub fn with_cooldown(mut self, cooldown: usize) -> Self {
        self.cooldown = cooldown;
        self
    }

    #[must_use]
    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight_bits = weight.to_bits();
        self
    }

    /// The two-way form: `weight` while the condition holds, `otherwise` when
    /// it does not.
    #[must_use]
    pub fn with_weight_when(
        mut self,
        condition: MoveCondition,
        weight: f32,
        otherwise: f32,
    ) -> Self {
        self.weight_bits = weight.to_bits();
        self.weight_when = Some(ConditionalWeight {
            condition,
            otherwise_bits: otherwise.to_bits(),
        });
        self
    }

    #[must_use]
    pub fn weight(&self) -> f32 {
        f32::from_bits(self.weight_bits)
    }

    /// The weight the branch is worth, given how its own question was
    /// answered. A branch with a fixed weight ignores the answer.
    #[must_use]
    pub fn weight_given(&self, condition_holds: bool) -> f32 {
        match &self.weight_when {
            Some(conditional) if !condition_holds => f32::from_bits(conditional.otherwise_bits),
            _ => self.weight(),
        }
    }
}

/// What a branch asks before it is taken. Each variant is one question a
/// monster asks about itself or about the fight.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum MoveCondition {
    /// A branch that asks nothing at all.
    Always,
    /// The owner has already stood on this move state. It stands in for the
    /// "have I done this yet" flag a monster sets while it is performing a
    /// move: the state log carries the same answer, one roll earlier.
    HasEnteredState(String),
    /// The owner is below half its maximum HP, with the halving done in whole
    /// numbers.
    CurrentHpBelowHalfOfMax,
    /// The owner is marked off balance.
    OwnerIsOffBalance,
    /// The owner is standing in the slot of this name, which is what a machine
    /// that opens differently depending on where its owner is standing
    /// branches on.
    OwnerIsInSlotNamed(String),
    /// Somebody else on the owner's own side is still standing.
    HasLivingAlly,
    /// At most this many creatures on the owner's side are still standing,
    /// counting the owner among them: what an Ovicopter asks before it
    /// lays.
    LivingTeammatesAtMost(usize),
    /// The owner has stood on this move state at least this many times.
    EnteredStateAtLeast {
        state_id: String,
        times: usize,
    },
    /// No creature of this model is standing beside the owner any more. It
    /// stands in for the hidden "has my minion died yet" flag a monster sets
    /// when it loses one: the encounter that reaches it starts with exactly one
    /// of them, so the two answers are the same.
    TeammateOfModelIsGone(ModelId),
    /// The owner is carrying this power, which is what a machine that goes on
    /// doing one thing until a power of its own has run out branches on.
    OwnerHoldsPower(ModelId),
    /// A number the owner has written down for itself is at least this. It is
    /// how a machine reads a countdown its own moves keep — the turns a
    /// rat has left before it may call for backup, and how many calls its side
    /// has already made.
    OwnerLocalAtLeast {
        name: String,
        at_least: i32,
    },
    /// Somebody else on the owner's own side is intending this move, which a
    /// monster asks before doing something only one of its side should do at a
    /// time.
    AnAllyIntends(String),
    /// One of the encounter's slots has nobody standing in it. An encounter
    /// that names no slots at all has none free.
    EncounterHasAFreeSlot,
    Not(Box<MoveCondition>),
    All(Vec<MoveCondition>),
    Any(Vec<MoveCondition>),
}

/// One branch of a conditional transition, in the order the branches are
/// listed.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConditionalBranch {
    pub state_id: String,
    pub condition: MoveCondition,
}

impl ConditionalBranch {
    #[must_use]
    pub fn new(state_id: impl Into<String>, condition: MoveCondition) -> Self {
        Self {
            state_id: state_id.into(),
            condition,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum MonsterState {
    Move(MonsterMove),
    /// Weighted random transition. Branch states are never logged themselves.
    RandomBranch {
        state_id: String,
        branches: Vec<MoveBranch>,
    },
    /// The first branch whose condition answers yes, with no roll of any
    /// kind. A machine that reaches such a transition with no branch holding
    /// is an error rather than a no-op.
    ConditionalBranch {
        state_id: String,
        branches: Vec<ConditionalBranch>,
    },
}

impl MonsterState {
    #[must_use]
    pub fn state_id(&self) -> &str {
        match self {
            Self::Move(monster_move) => &monster_move.state_id,
            Self::RandomBranch { state_id, .. } | Self::ConditionalBranch { state_id, .. } => {
                state_id
            }
        }
    }

    #[must_use]
    pub const fn is_move(&self) -> bool {
        matches!(self, Self::Move(_))
    }

    /// Only move states appear in the log the repeat weights read.
    #[must_use]
    pub const fn logged(&self) -> bool {
        self.is_move()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MonsterDefinition {
    pub model_id: ModelId,
    pub min_initial_hp: i32,
    pub max_initial_hp: i32,
    pub initial_state: String,
    pub states: Vec<MonsterState>,
    /// What the monster does as it joins the combat, which for every covered
    /// monster is putting a power on itself or on whoever it is fighting.
    pub on_added: Vec<crate::Command>,
    /// What the monster itself listens for. A monster is on the combat
    /// listener list in its own right, right behind the powers of the creature
    /// carrying it.
    #[serde(default)]
    pub hooks: Vec<ModifierHook>,
}

impl MonsterDefinition {
    #[must_use]
    pub fn state(&self, state_id: &str) -> Option<&MonsterState> {
        self.states
            .iter()
            .find(|state| state.state_id() == state_id)
    }

    #[must_use]
    pub fn monster_move(&self, state_id: &str) -> Option<&MonsterMove> {
        match self.state(state_id)? {
            MonsterState::Move(monster_move) => Some(monster_move),
            MonsterState::RandomBranch { .. } | MonsterState::ConditionalBranch { .. } => None,
        }
    }
}

/// Where a card goes once it finishes resolving.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayedCardDestination {
    Discard,
    Exhaust,
    /// The top of the draw pile, which is where a rebound sends a card that
    /// would otherwise have been discarded.
    DrawPileTop,
    /// The hand, where the card would otherwise have been discarded — which is
    /// what a Particle Wall does. The card says it of itself rather than
    /// anything a listener saying it, so it is settled before the pass runs.
    Hand,
    /// The front of the hand rather than the back of it, which is where a
    /// Feral sends a free attack.
    HandTop,
    /// Powers leave the combat entirely once they resolve.
    Removed,
}

/// One entry in the ordered listener list a pass walks.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Listener {
    /// The creature the listening model belongs to.
    pub owner: CreatureCombatId,
    pub source: ListenerSource,
    pub model_id: ModelId,
    /// For a power, which instance of the model is listening. Two instances of
    /// one power on one creature are two listeners, and each answers for
    /// itself.
    #[serde(default)]
    pub power_instance: Option<u64>,
}

/// What kind of thing is listening. The build has powers, monsters, relics,
/// potions, orbs, cards, afflictions, enchantments, modifiers, and badges; only
/// the ones this baseline carries are here.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListenerSource {
    Power,
    /// The monster model a creature is, which follows that creature's powers on
    /// the combat listener list.
    Monster,
    Relic,
    /// A card standing in one of the combat piles, named by its combat
    /// instance. All that is asked of it is whether it has left the combat,
    /// never which pile it is in, so a card that moved piles mid-pass keeps
    /// answering.
    CombatCard(CardInstanceId),
    /// A card in the run deck, named by where it stands in it.
    DeckCard(usize),
    /// The enchantment a combat card carries. It stands on the combat
    /// listener list immediately behind that card, and it listens exactly
    /// while that card does: the card is still there, it has not left the
    /// combat, and its owner is still taking part.
    CombatCardEnchantment(CardInstanceId),
    /// The enchantment a run-deck card carries, immediately behind that card on
    /// the run listener list.
    DeckCardEnchantment(usize),
    /// A potion the player is holding, named by the slot it sits in. The build
    /// puts the potion belt on the listener list right behind the relics, and a
    /// potion listens for as long as it is held.
    PotionSlot(usize),
    /// An orb standing in the player's orb queue. Orbs are combat listeners,
    /// and the whole queue stands right behind the potion belt and in front of
    /// the piles. An orb listens while it has not left the combat and its
    /// owner is still taking part.
    Orb(OrbInstanceId),
}

/// What one instance of damage came to.
///
/// The whole of it is handed to every listener the settled blow is offered to,
/// and each reads it rather than working anything out again. It is also what
/// decides which of the four passes over a settled blow happen at all.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "damage properties, block outcomes and death are independent facts"
)]
pub struct DamageResult {
    /// How much of the blow block took.
    pub blocked: i32,
    /// The HP the target actually lost, measured against what it had before
    /// the blow — so a blow worth more than the target had left counts only
    /// what it took.
    pub unblocked: i32,
    /// What a killing blow was worth beyond the target's remaining HP, and
    /// zero for a blow that did not kill.
    pub overkill: i32,
    /// Block the target was still standing behind once the blow was stopped.
    pub block_left: i32,
    /// Whether block was allowed to stop it at all.
    pub unblockable: bool,
    /// Whether the original target fully blocked the blow, measured after
    /// HP-loss modifiers and before a redirected blow spills back onto it.
    /// A redirect recipient carries no block outcome of its own.
    pub was_fully_blocked: bool,
    /// Whether the original target's block broke. Only its result carries
    /// this flag, even when another creature absorbed the HP loss.
    pub was_block_broken: bool,
    /// The blow found the target standing and was worth at least what it had
    /// left. It stays true even if something stood the target back up
    /// afterwards, which is why the pass that skips a killed target asks about
    /// the creature as well.
    pub killed: bool,
}

impl DamageResult {
    /// Whether block stopped the blow whole. Note what this is not: a blow
    /// worth nothing against a creature with no block is not fully blocked,
    /// and one worth nothing against a creature that has some is.
    #[must_use]
    pub const fn fully_blocked(self) -> bool {
        self.was_fully_blocked
    }

    /// Whether block took some of the blow and had nothing left afterwards. A
    /// blow that lands on a creature with no block does not break it, and one
    /// block swallows whole without running out does not either.
    #[must_use]
    pub const fn block_broken(self) -> bool {
        self.was_block_broken
    }
}

/// What every pass over a value carries: who is dealing, who is taking, how
/// the value was worked out, and what card it came from. A power application
/// pass carries the power being worked instead of a dealer.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ValueContext {
    /// Whoever is dealing the damage, or whoever the play belongs to.
    pub dealer: Option<CreatureCombatId>,
    /// Whoever is taking the damage, gaining the block, or being aimed at.
    pub target: Option<CreatureCombatId>,
    pub props: crate::ValueProps,
    /// The card the value came from.
    pub card: Option<CardInstanceId>,
    /// The power a power-received pass is carrying, and how much of it is on
    /// its way.
    pub power: Option<(ModelId, i32)>,
    /// Whoever worked the power the pass is about.
    #[serde(default)]
    pub applier: Option<CreatureCombatId>,
    /// What the value the pass is about came to, which is what a block or
    /// power listener tests before it acts.
    #[serde(default)]
    pub amount: i32,
    /// What an instance of damage came to, which is what the damage-lifecycle
    /// listeners are handed once it is settled.
    pub damage: Option<DamageResult>,
}

/// What a pass knows about what just happened, which is what the listeners
/// test against.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag is one thing a pass can carry about what happened"
)]
pub struct HookContext {
    /// The creature it happened to, when it names one. Only that creature's
    /// own listeners run, which stands in for the test each listener would
    /// otherwise make that it is about its own owner.
    pub subject: Option<CreatureCombatId>,
    /// A creature the pass names, offered to every listener rather than
    /// restricting the pass to that creature's own. A death is announced this
    /// way, and each listener tests it for itself.
    #[serde(default)]
    pub announced_creature: Option<CreatureCombatId>,
    /// The side whose turn is starting or ending.
    pub side: Option<CombatSide>,
    /// What a whole swing settled and who took each of them, which the
    /// after-attack pass hands its listeners.
    ///
    /// The results are grouped by hit, and the grouping is load-bearing: a hit
    /// a pet took for its owner settles twice inside one group, and anything
    /// counting the hits that got through has to count that once. So each
    /// entry carries which hit of the swing it belongs to.
    #[serde(default)]
    pub attack_results: Vec<(usize, CreatureCombatId, DamageResult)>,
    /// Whether the card that was just played is an attack.
    pub played_attack: bool,
    /// What type the card that was just played is.
    #[serde(default)]
    pub played_card_type: Option<CardType>,
    /// The card that was just played.
    #[serde(default)]
    pub played_card: Option<CardInstanceId>,
    /// The card that just changed piles, and the pile it came from.
    pub moved_card: Option<CardInstanceId>,
    pub moved_from: Option<crate::PileName>,
    /// The card that changed piles landed in the run deck. The deck is not
    /// one of the combat piles, so this is said here rather than in
    /// `moved_from`.
    #[serde(default)]
    pub moved_into_deck: bool,
    /// Where in the run deck the card that just joined it stands. A deck card
    /// is not a combat instance, so it is named by its place rather than by an
    /// identifier.
    #[serde(default)]
    pub joined_deck_card: Option<usize>,
    /// The card was put in the deck by a listener rather than by the run,
    /// which is what stops a listener answering for its own copy.
    #[serde(default)]
    pub deck_card_cloned_by_a_listener: bool,
    /// Whether the burn was a card burning up in hand at the end of a turn
    /// rather than one something asked for.
    #[serde(default)]
    pub exhaust_caused_by_ethereal: bool,
    /// Whether the draw was the turn's own opening hand draw rather than one
    /// something asked for.
    #[serde(default)]
    pub drawn_from_hand_draw: bool,
    /// How much the announced creature's health moved, negative where it lost
    /// some. Every listener there is sees the pass and tests the creature for
    /// itself, which is why the creature is announced rather than made the
    /// pass's subject.
    #[serde(default)]
    pub hp_delta: i32,
    /// The creatures a doom kill took down whose deaths counted, written down
    /// before the kill because the fight no longer holds them afterwards.
    #[serde(default)]
    pub died_to_doom: Vec<CreatureCombatId>,
    /// The orb a pass is about: what the channel, evoke, value and
    /// trigger-count passes are all handed.
    #[serde(default)]
    pub announced_orb: Option<OrbInstanceId>,
    /// The model of that orb, kept beside it because an evoke announces an orb
    /// the queue no longer holds.
    #[serde(default)]
    pub announced_orb_model: Option<ModelId>,
    /// What an evoke reached.
    #[serde(default)]
    pub evoked_orb_targets: Vec<CreatureCombatId>,
    /// What a pass over a value knows about that value.
    pub value: ValueContext,
}

/// A listener pass a selection screen interrupted: the list it snapshotted
/// before it began, how far along it was, and what it was dispatching.
///
/// A paused pass is mid-way through a list of listeners, so the frame has to
/// remember exactly how far it got and what it had already handed them. Here it
/// has to be state, because the screen is a decision boundary and the pass has
/// to survive the player thinking about it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SuspendedHookPass {
    pub event: HookEvent,
    /// The list as it stood before the pass began. A listener added while the
    /// screen was up is not in it and does not act until the next pass.
    pub listeners: Vec<Listener>,
    /// The entry to check next. Everything before it has already acted.
    pub next: usize,
    pub context: HookContext,
}

/// A run of commands a selection screen interrupted, and the context it was
/// running in.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct QueuedCommands {
    pub commands: Vec<crate::Command>,
    pub context: crate::CommandContext,
}

/// The screen a deferred listener asked for, waiting for the pass to be waited
/// for.
///
/// A deferred listener that asks a question is put on the action queue and
/// paused there, while the pass carries on to the next listener. Here the entry
/// carries the request itself — the screen is stood back up over the piles as
/// they stand when it is reached, which is when the real screen would read
/// them, because such a screen takes the filter rather than a fixed list of
/// cards — and everything the paused listener still owes behind the answer.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SuspendedHookScreen {
    /// The choice id reserved when the listener asked, which is when the real
    /// game reserves its own.
    pub id: ChoiceId,
    pub source: SelectionSource,
    pub prefs: crate::SelectorPrefs,
    pub filters: Vec<crate::CardFilter>,
    /// The explicit candidate list, where the caller passed one.
    #[serde(default)]
    pub only: Option<Vec<CardInstanceId>>,
    #[serde(default)]
    pub bundles: Vec<Vec<usize>>,
    pub purpose: ChoicePurpose,
    /// Public meaning retained independently of the execution continuation.
    #[serde(default)]
    pub semantics: crate::ChoiceSemantics,
    /// The semantic screen family used by recorder-compatible consumers.
    #[serde(default)]
    pub screen_kind: ChoiceScreen,
    /// A name for the screen, for the diagnostics.
    pub screen: String,
    /// What the paused listener still owes once the answer lands, in resume
    /// order.
    pub owed: Vec<Effect>,
}

/// One entry of the combat's effect queue.
///
/// A card, potion, or listener is a run of commands, and each of them queues as
/// one entry carrying the context it was issued in. The other two entries are
/// the engine's own: they have no command behind them because the build does
/// them as part of playing a card rather than as something a card asked for.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Effect {
    /// One command, with what it knows about where it came from.
    Run(Box<QueuedCommand>),
    /// Moves the played card to wherever it ends up. Where the card goes was
    /// settled before its own effect ran — it is asked the moment the card
    /// reaches the play pile, ahead of the play being announced — so a power a
    /// card applies to itself has no say over that card.
    MovePlayedCard {
        card_id: CardInstanceId,
        destination: PlayedCardDestination,
    },
    /// Runs the listeners at a point in the turn cycle, in queue order, so that
    /// what a card does lands before anything that reacts to the card.
    DispatchModifierHooks(HookEvent),
    /// Announces a card play, which happens inside the play loop rather than
    /// in front of it: a card whose effect runs twice announces itself twice,
    /// and a power that writes down what it was worth when a play began writes
    /// an entry per pass.
    AnnounceCardPlay {
        card_id: CardInstanceId,
        target: Option<CreatureCombatId>,
        /// Which pass of the play loop this announcement opens. A Lethality
        /// reads it to refuse a replay past the first.
        #[serde(default)]
        play_index: i32,
    },
    /// The rest of an effect a selection screen interrupted, run where it
    /// stands rather than one command per entry, because that is what the rest
    /// of an effect is.
    RunRemaining(Box<QueuedCommands>),
    /// The rest of a listener pass a selection screen interrupted.
    ResumeHookPass(Box<SuspendedHookPass>),
    /// Completes a run relic reward after its acquisition effects and choices.
    /// Earlier queue entries belong to the claim; later entries belong to the
    /// body waiting for the reward set.
    FinishRelicReward {
        set_id: RewardSetId,
        reward_index: usize,
    },
    /// A deferred listener's screen waiting to be shown, which stands on the
    /// action queue. The pass that deferred it has already run on past it;
    /// opening it is what waiting for the pass waits for.
    HookScreen(Box<SuspendedHookScreen>),
    /// The rest of a purchase behind a screen the purchase itself opened: the
    /// purchase is finished — a relic's obtain effect and any screen it puts
    /// up included — before the shelf is cleared or restocked and the sale
    /// announced, so what follows the screen is state the answer picks back
    /// up.
    SettlePurchase {
        offer_index: usize,
        /// Whether the shelf is a real merchant's: an event's shelf neither
        /// restocks nor asks anything about restocking.
        on_a_merchants_shelf: bool,
    },
    /// The rest of a grab off an endless conveyor behind a screen the plate
    /// itself opened: the belt turns once the plate has been eaten, so the roll
    /// that decides what comes round next reads its die after whatever the
    /// plate's own effect read.
    SettleConveyorGrab {
        /// The dish just taken, which the belt will not serve twice running.
        taken: String,
        /// How many plates had been taken before this one.
        grabs: usize,
    },
    /// The rest of a parasol's walk down a merchant's shelf, behind a screen
    /// one of its purchases opened: each purchase is finished before the next
    /// begins, so the entries still to be bought — and the removal that ends
    /// the walk — wait for the answer.
    BuyTheRestOfTheShelf {
        /// The offers still to be bought, by index, in the inventory's order.
        remaining: Vec<usize>,
    },
    /// The rest of one of the engine's own frames — the turn handover, an
    /// attack between its hits, a command landing on several creatures — picked
    /// back up at the step it was interrupted on.
    Resume(Box<ResumeFrame>),
    /// The empty-hand check and the pass it runs, together, which a potion
    /// queues behind its own listeners.
    CheckForEmptyHand,
    /// The card's own effect, which its play runs once per play, ahead of its
    /// enchantment's.
    ///
    /// It is an entry of its own rather than the card's commands queued up
    /// front, for the reason the enchantment's is: the card's own numbers are
    /// read at the moment this is reached, and a pass of the play loop that
    /// has already run may have changed them. A Razor Tooth upgrades what was
    /// played as soon as the play is announced, inside the loop, so the second
    /// pass of a Strike a Soldier's Stew doubled resolves the upgraded form.
    RunCardBody {
        card_id: CardInstanceId,
        target: Option<CreatureCombatId>,
    },
    /// The enchantment's own effect, which the card's play runs once per play
    /// after the card's own and before the play is announced as finished.
    ///
    /// It is an entry of its own rather than the enchantment's commands queued
    /// up front, because which enchantment the card carries is read at the
    /// moment this is reached and something that has already run may have
    /// changed it.
    RunEnchantmentOnPlay {
        card_id: CardInstanceId,
        target: Option<CreatureCombatId>,
    },
}

/// A step of the fight's own sequence that a selection screen can interrupt.
///
/// A paused step is mid-way through something with an order to it, so the frame
/// has to remember which statement it had reached and where it stood in
/// whatever loop it was running. Here it has to be state, because the screen is
/// a decision boundary and the frame has to survive the player thinking about
/// it. Each variant names the stretch of the fight it stands for.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ResumeFrame {
    /// Everything between one player turn and the next: the two halves of the
    /// player's turn end, the enemy side's whole turn, and the opening of the
    /// player's next one.
    TurnHandover(TurnHandoverStep),
    /// The first half of ending the player's turn.
    EndPlayerTurnPhaseOne(EndTurnPhaseOneStep),
    /// The second half of ending the player's turn.
    EndPlayerTurnPhaseTwo(EndTurnPhaseTwoStep),
    /// The cards that were still in hand as the turn ended.
    CardsStillHeld {
        /// The cards, listed before the loop began.
        cards: Vec<CardInstanceId>,
        /// Which of them is being acted on.
        index: usize,
        /// How far into that card's own end-of-turn effect the run got.
        command: usize,
    },
    /// The enemy side's turn.
    EnemyTurn {
        /// The enemies, listed before the loop began. Each entry is checked
        /// for still being in the combat as it comes up.
        enemies: Vec<CreatureCombatId>,
        index: usize,
    },
    /// One monster performing its move: the move captured before it was
    /// performed, and how far into it the run got.
    MonsterMove {
        enemy: CreatureCombatId,
        move_state: String,
        command: usize,
    },
    /// Setting the player's turn up, and the auto-pre-play phase behind it.
    /// The two run as one stretch and the turn start carries on past a pause,
    /// so this frame outlives the turn start that began it.
    SetupPlayerTurn(SetupPlayerTurnStep),
    /// One attack's per-hit loop.
    Attack(Box<AttackFrame>),
    /// The damage calls inside one manually grouped attack. A damage listener
    /// may suspend either call, but the after-attack pass must see both hits.
    SplashAttack {
        context: Box<crate::CommandContext>,
        results: Vec<(usize, CreatureCombatId, DamageResult)>,
        splash_pending: bool,
    },
    /// Damage over the creatures one command reaches.
    Damage(Box<DamageFrame>),
    /// A maximum-HP loss past the blow it deals for the current HP left
    /// standing over the new maximum: that creature's maximum still has to
    /// come down, and the creatures the command had not reached still owe the
    /// whole pass.
    LoseMaxHp {
        /// The creature whose blow opened the screen, and the maximum its own
        /// call worked out before the blow — which is what its maximum becomes
        /// when the screen is answered.
        settling: Option<(CreatureCombatId, i32)>,
        /// The creatures the loop had not begun.
        rest: Vec<CreatureCombatId>,
        /// The maximum HP each of those still loses.
        amount: i32,
    },
    /// An automatic play: what the play still owes once a screen it opened
    /// has been answered. A play the player chose queues its whole tail up
    /// front, but an automatic one is finished inside whatever asked for it,
    /// so its tail is held here instead.
    AutoPlay(Box<AutoPlayFrame>),
    /// The walk over the orbs a turn boundary fires.
    OrbTurnPass(Box<OrbTurnFrame>),
    /// A loop inside an effect, between two of its rounds. The loop is held as
    /// one command here, so the frame names the command it stands in.
    Loop(Box<LoopFrame>),
    /// A draw between two of its cards: the shuffle it ran opened a screen —
    /// a Stratagem asking which cards to take back — and the cards still to
    /// be drawn wait behind the answer.
    Draw {
        /// How many cards the draw still owes.
        remaining: usize,
        /// Whether the draw is the turn's own opening hand draw.
        from_hand_draw: bool,
    },
}

/// An orb turn pass between two of its orbs, or between two triggers of one
/// orb.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrbTurnFrame {
    pub trigger: OrbTrigger,
    /// The orbs, listed before the loop opened. An orb that left the queue
    /// during the pass is still walked to and nothing checks whether it is
    /// still there — only whether its owner is still in a fight.
    pub orbs: Vec<OrbInstanceId>,
    /// Which of them the loop is on.
    pub index: usize,
    /// How many triggers of that orb are still owed, and nothing where the
    /// count pass for it has not run yet.
    #[serde(default)]
    pub triggers_left: Option<i32>,
}

/// The rounds of an effect's loop a selection screen has not reached.
///
/// An effect that stops inside a loop puts down two things to pick back up,
/// and they are ordered the way the game would resume them: the rest of the
/// interrupted round's statements first, then this — the rounds the loop had
/// not begun. Whatever the frame names is checked again as it comes up, the
/// way every other suspended frame checks: a fight that ended behind the
/// screen owes no rounds at all, and a creature or orb that is gone is found
/// gone where it is read.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum LoopFrame {
    /// A plain repeat: the rounds not yet begun.
    Repeat {
        remaining: i32,
        then: Vec<crate::Command>,
        context: crate::CommandContext,
    },
    /// A repeat whose very last round runs its own arm: the rounds not yet
    /// begun.
    RepeatWithLast {
        remaining: i32,
        then: Vec<crate::Command>,
        last: Vec<crate::Command>,
        context: crate::CommandContext,
    },
    /// `Command::RepeatPerKill`: the rounds already owed, and the enemies that
    /// stood when the interrupted round began — its kills have not been
    /// counted yet, and the count belongs to the continuation.
    RepeatPerKill {
        owed: usize,
        standing: Vec<CreatureCombatId>,
        then: Vec<crate::Command>,
        context: crate::CommandContext,
    },
    /// A walk over orbs: the ones it has not reached. Each is looked up again
    /// as it comes up, exactly as an uninterrupted walk looks it up.
    ForEachOrb {
        rest: Vec<OrbInstanceId>,
        then: Vec<crate::Command>,
        context: crate::CommandContext,
    },
    /// A walk over cards: the ones it has not reached.
    ForEachCard {
        rest: Vec<crate::ChosenCard>,
        then: Vec<crate::Command>,
        context: crate::CommandContext,
    },
    /// A passive fired over several orbs: the ones the pass has not reached.
    /// The target was settled before the pass began, which is why the frame
    /// carries it settled.
    OrbPassive {
        rest: Vec<OrbInstanceId>,
        target: Option<CreatureCombatId>,
        context: crate::CommandContext,
    },
    /// A channel past the evoke it made room with: putting the orb in the
    /// queue and announcing it are what is left, and neither reads anything
    /// from whoever asked for the channel.
    Channel { model_id: ModelId },
    /// Several cards played automatically, as themselves or as one-shot
    /// copies: the ones the loop has not played. A copy is made as each card
    /// is reached, so the frame carries the originals.
    AutoPlayCards {
        rest: Vec<CardInstanceId>,
        aimed_at: Option<CreatureCombatId>,
        dupe: bool,
    },
    /// Cards taken off the draw pile to be played automatically, all of them
    /// put on the play pile before the first was played: the ones a screen
    /// opened by an earlier one left waiting there.
    AutoPlayTaken {
        rest: Vec<CardInstanceId>,
        force_exhaust: bool,
    },
    /// A run of cards burning up: the ones not yet burned. The run is walked
    /// as one and each burn can be interrupted, so a screen a listener opens
    /// on one card's burn is answered with the rest of the run still owed.
    ExhaustRun {
        rest: Vec<CardInstanceId>,
        caused_by_ethereal: bool,
    },
}

/// What an automatic play still owes: the passes of its effect it has not run,
/// where in the interrupted pass it stopped, and where the card goes once they
/// are done.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AutoPlayFrame {
    pub card_id: CardInstanceId,
    /// What the play was aimed at, which the announcement of it names.
    pub target: CreatureCombatId,
    /// How many passes the settled play count still owes, the interrupted one
    /// included.
    pub plays_left: i32,
    /// What the first of those passes still has to run.
    #[serde(default)]
    pub step: AutoPlayStep,
    pub destination: PlayedCardDestination,
}

/// The three parts of one pass of an automatic play's repeat loop, in order:
/// the announcement that the card is being played and the card's own body,
/// its enchantment's body, and the announcement that it has been played.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoPlayStep {
    #[default]
    Body,
    Enchantment,
    Announcement,
}

/// The points at which the turn handover can stop.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnHandoverStep {
    /// The first half of ending the player's turn.
    PhaseOne,
    /// The second half of it, once every player is ready for the enemy turn.
    PhaseTwo,
    /// The sides switch and the enemy turn opens.
    EnemyTurnStart,
    /// The enemy turn itself.
    EnemyTurn,
    /// The enemy turn ending.
    EnemyTurnEnd,
    /// The sides switch back and the player's next turn stands up.
    PlayerTurnStart,
}

impl TurnHandoverStep {
    /// The stage after this one, or nothing when the handover is over.
    #[must_use]
    pub const fn next(self) -> Option<Self> {
        match self {
            Self::PhaseOne => Some(Self::PhaseTwo),
            Self::PhaseTwo => Some(Self::EnemyTurnStart),
            Self::EnemyTurnStart => Some(Self::EnemyTurn),
            Self::EnemyTurn => Some(Self::EnemyTurnEnd),
            Self::EnemyTurnEnd => Some(Self::PlayerTurnStart),
            Self::PlayerTurnStart => None,
        }
    }
}

/// The points at which the first half of ending a player's turn can stop.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndTurnPhaseOneStep {
    /// The auto-post-play phase and the announcement that it has opened.
    AutoPostPlay,
    /// The player's phase becomes `End`, and the before-turn-end pass runs its
    /// three passes.
    BeforeTurnEnd,
    /// The very first thing the turn end does: the orbs that fire as a turn
    /// ends. It stands ahead of everything the hand does, and a fight the orbs
    /// finished skips the rest of the turn end outright.
    OrbsBeforeTurnEnd,
    /// The rest of the turn end: what the hand does to whoever is still
    /// holding it.
    CardsStillHeld,
    /// The pass before the hand is flushed.
    BeforeFlush,
}

impl EndTurnPhaseOneStep {
    #[must_use]
    pub const fn next(self) -> Option<Self> {
        match self {
            Self::AutoPostPlay => Some(Self::BeforeTurnEnd),
            Self::BeforeTurnEnd => Some(Self::OrbsBeforeTurnEnd),
            Self::OrbsBeforeTurnEnd => Some(Self::CardsStillHeld),
            Self::CardsStillHeld => Some(Self::BeforeFlush),
            Self::BeforeFlush => None,
        }
    }
}

/// The points at which the second half of ending a player's turn can stop.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndTurnPhaseTwoStep {
    /// The hand goes down and the after-flush pass runs.
    Flush,
    /// Both after-turn-end passes.
    AfterTurnEnd,
}

impl EndTurnPhaseTwoStep {
    #[must_use]
    pub const fn next(self) -> Option<Self> {
        match self {
            Self::Flush => Some(Self::AfterTurnEnd),
            Self::AfterTurnEnd => None,
        }
    }
}

/// The points at which setting a player's turn up, and the auto-pre-play phase
/// behind it, can stop.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SetupPlayerTurnStep {
    /// Energy back, then the pass that follows the refill.
    EnergyReset,
    /// The pass before the hand draw, with the hand still as the turn end left
    /// it. It is a step of its own because a listener may ask the player
    /// something here, and the draw behind it must not run twice for the
    /// answer.
    BeforeHandDraw,
    /// The draw itself.
    Draw,
    /// The pass that follows the player's turn opening.
    PlayerTurnStart,
    /// The phase becomes auto-pre-play and the pass that follows it runs.
    AutoPrePlay,
    /// The phase becomes `Play` and the player takes control.
    Play,
}

impl SetupPlayerTurnStep {
    #[must_use]
    pub const fn next(self) -> Option<Self> {
        match self {
            Self::EnergyReset => Some(Self::BeforeHandDraw),
            Self::BeforeHandDraw => Some(Self::Draw),
            Self::Draw => Some(Self::PlayerTurnStart),
            Self::PlayerTurnStart => Some(Self::AutoPrePlay),
            Self::AutoPrePlay => Some(Self::Play),
            Self::Play => None,
        }
    }
}

/// One attack between its hits.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AttackFrame {
    pub amount: crate::Amount,
    pub props: crate::ValueProps,
    pub target: crate::CreatureSelector,
    pub context: crate::CommandContext,
    /// Hits still owed after the one that was interrupted. The hit count is
    /// settled before the loop begins, so it is never worked out again.
    pub hits_left: i32,
    /// What every hit so far settled, flattened, in the order the results are
    /// walked. Only an attack something has bound keeps them.
    #[serde(default)]
    pub results: Vec<DamageResult>,
    /// What still runs after the swing, for whatever bound what its swing came
    /// to. An attack nothing binds owes nothing after it.
    #[serde(default)]
    pub then: Vec<crate::Command>,
}

/// Damage between the creatures one command reaches.
///
/// There are two loops: the first works every target's damage out and settles
/// it against block and HP, the second hands each settled result to the
/// listeners. Only the second can be interrupted, because only the
/// damage-given and damage-received passes can ask the player anything.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DamageFrame {
    /// The targets, listed before the first loop began.
    pub targets: Vec<CreatureCombatId>,
    pub dealer: Option<CreatureCombatId>,
    pub card: Option<CardInstanceId>,
    pub props: crate::ValueProps,
    /// The value the caller worked out once, before the call.
    pub amount: i32,
    /// What the first loop settled, in target order.
    pub results: Vec<(CreatureCombatId, DamageResult)>,
    /// How far the second loop got through them: how many have been announced
    /// to both listener passes.
    pub announced: usize,
    /// Whether the damage-given pass has already run for
    /// `results[announced]`, so that a screen opened by it comes back to the
    /// damage-received pass rather than skipping it.
    pub given: bool,
}

/// A command waiting on the queue, with what it knows about where it came from.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct QueuedCommand {
    pub command: crate::Command,
    pub context: crate::CommandContext,
}

impl QueuedCommand {
    /// One queue entry for a command issued in `context`.
    #[must_use]
    pub fn queued(command: crate::Command, context: crate::CommandContext) -> Effect {
        Effect::Run(Box::new(Self { command, context }))
    }
}

/// How the cards on offer are named, which is what makes an answer portable.
///
/// An answer takes one of three shapes, one per screen family: the cards
/// themselves for a screen over a combat pile, the cards themselves for one
/// over the run deck, and bare indices for one over a list of cards that are
/// only being offered and stand in no pile at all.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChoiceIdentity {
    /// A card standing in one of the combat piles: its combat instance names
    /// it, and its index is where it stands in the offered list.
    CombatCard,
    /// A card in the run deck: its position in the deck names it.
    DeckCard,
    /// A card that exists only as an offer: its position in the offer names it.
    Offered,
}

/// The concrete game screen carrying a card choice.
///
/// Positional `choice.index` records are verified against the recorder's
/// screen-specific offer context, so producers must retain this distinction
/// even where two screens expose the same cards and selection counts.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChoiceScreen {
    /// A snapshot that names no screen family.
    #[default]
    Unknown,
    Hand,
    CombatPile,
    Deck,
    ChooseACard,
    CardGrid,
    RewardGrid,
    CardBundle,
    CardReward,
}

/// Where a selection screen's cards come from, which is also what names them
/// in an answer.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionSource {
    /// One of the combat piles.
    CombatPile(crate::PileName),
    /// The run deck.
    Deck,
    /// A list of cards that stand in no pile at all, which is what a generator
    /// lays out to be chosen between.
    Offered(Vec<CardFingerprint>),
}

impl SelectionSource {
    #[must_use]
    pub const fn identity(&self) -> ChoiceIdentity {
        match self {
            Self::CombatPile(_) => ChoiceIdentity::CombatCard,
            Self::Deck => ChoiceIdentity::DeckCard,
            Self::Offered(_) => ChoiceIdentity::Offered,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SuspendedChoice {
    pub id: ChoiceId,
    /// Where the cards on offer came from.
    pub identity: ChoiceIdentity,
    /// Which concrete game screen presents the candidates.
    #[serde(default)]
    pub screen: ChoiceScreen,
    pub candidates: Vec<CardHandle>,
    pub minimum: usize,
    pub maximum: usize,
    /// Whether the screen can be backed out of with nothing taken, even
    /// though it asks for a minimum.
    pub cancelable: bool,
    /// What the screen offers besides its cards and the skip, which is what a
    /// relic puts on a card reward.
    #[serde(default)]
    pub alternatives: Vec<ChoiceAlternative>,
    /// A choose-a-bundle screen: the candidates grouped into bundles, each
    /// named by the places its cards stand at in the candidate
    /// list. A screen with bundles is answered with one whole bundle and
    /// nothing else; every other screen has none.
    #[serde(default)]
    pub bundles: Vec<Vec<usize>>,
    pub purpose: ChoicePurpose,
    /// Public meaning retained independently of the execution continuation.
    #[serde(default)]
    pub semantics: crate::ChoiceSemantics,
    /// What the player has clicked into the selection so far. The screen is
    /// worked one card at a time — a click toggles a card in or out, and the
    /// selection completes once it holds enough — so a screen with more
    /// answers than
    /// the action list can lay out is
    /// answered the same way, and this is the selection standing between the
    /// clicks. A screen small enough to enumerate never holds one.
    #[serde(default)]
    pub picked: Vec<CardHandle>,
    /// The filter a screen over the hand was opened with. The build's hand
    /// screen is handed the filter rather than a list, and offers whatever
    /// stands in the hand while it is up, so the candidates are read again
    /// where something joins the hand in front of the answer. No other screen
    /// keeps one.
    #[serde(default)]
    pub hand_filters: Option<Vec<crate::CardFilter>>,
}

/// One extra line on a card reward screen: a button that is not a card, and
/// the relic that put it there.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChoiceAlternative {
    pub option_id: String,
    /// The relic that put the button there, where one did. The reroll is the
    /// card reward's own rather than a listener's, so it names none.
    pub relic: Option<ModelId>,
}

impl SuspendedChoice {
    /// How few cards an answer may name. A cancelable screen also accepts
    /// nothing at all, which is what backing out of it hands over.
    #[must_use]
    pub const fn least_answer(&self) -> usize {
        if self.cancelable { 0 } else { self.minimum }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[allow(
    clippy::large_enum_variant,
    reason = "a suspended screen carries the body it owes, which is as big as that body"
)]
pub enum ChoicePurpose {
    /// What the selection screen's caller does with what it picked: the
    /// commands after the screen, with the picked cards bound to them.
    RunCommands {
        commands: Vec<crate::Command>,
        context: crate::CommandContext,
    },
    /// Whatever is chosen off an event's table joins the deck.
    AddChosenCardsToDeck,
    AddRewardCard {
        set_id: RewardSetId,
        reward_index: usize,
    },
    /// Whichever deck card is chosen is bought off the deck at a merchant.
    RemoveDeckCardAtShop {
        /// Whether the removal is paid for at the merchant's price. A parasol
        /// asks for the same removal with the cost waived, and the answer
        /// charges nothing and needs no gold in hand.
        paid: bool,
    },
    /// The rest site's smith: what was chosen is upgraded, and having upgraded
    /// something is what takes the option off the list. Backing out leaves the
    /// site exactly as it stood, the option included.
    RestSmith { index: usize },
    /// The rest site's cook: the two cards chosen leave the deck and the
    /// maximum rises by nine. Backing out leaves the site as it stood, the
    /// same way the smith does.
    RestCook { index: usize },
    /// A Zen weaver's removal: what was chosen leaves the deck, the gold is
    /// paid after it, and the page is done.
    RemoveDeckCardsForGold { cost: i32 },
    /// A removal offered as a reward: what was chosen leaves the deck and the
    /// reward is taken. Backing out leaves the reward on the table for the
    /// player to click again.
    RemoveDeckCardForReward {
        set_id: RewardSetId,
        reward_index: usize,
    },
}

/// A card a selection handed back.
///
/// [`CommandContext::chosen`] carries these whole rather than as identifiers, so
/// that a command after the one that moved or removed them can still read what
/// they were. Which shape a card takes is where the screen found it: the build
/// makes no distinction between a deck card and a combat card, but a run-deck
/// card has no combat instance here, and a card that is only being offered
/// stands in no pile at all.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[allow(
    clippy::large_enum_variant,
    reason = "a chosen combat card is carried whole on purpose, which is as big as a combat card"
)]
pub enum ChosenCard {
    Combat(CardInstance),
    Deck {
        index: usize,
        fingerprint: CardFingerprint,
    },
    Offered {
        index: usize,
        fingerprint: CardFingerprint,
    },
}

impl ChosenCard {
    #[must_use]
    pub const fn fingerprint(&self) -> &CardFingerprint {
        match self {
            Self::Combat(card) => &card.fingerprint,
            Self::Deck { fingerprint, .. } | Self::Offered { fingerprint, .. } => fingerprint,
        }
    }

    #[must_use]
    pub const fn combat_id(&self) -> Option<CardInstanceId> {
        match self {
            Self::Combat(card) => Some(card.id),
            Self::Deck { .. } | Self::Offered { .. } => None,
        }
    }

    #[must_use]
    pub const fn deck_index(&self) -> Option<usize> {
        match self {
            Self::Deck { index, .. } => Some(*index),
            Self::Combat(_) | Self::Offered { .. } => None,
        }
    }

    /// The run-deck card this choice names, whether it was picked off the
    /// deck itself or off a combat card cloned from one. A
    /// card conjured mid-fight names none, and neither does one only being
    /// offered.
    #[must_use]
    pub const fn deck_link(&self) -> Option<usize> {
        match self {
            Self::Deck { index, .. } => Some(*index),
            Self::Combat(card) => card.deck_version,
            Self::Offered { .. } => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "independent reward properties, not mutually exclusive states"
)]
pub struct RewardFingerprint {
    pub reward_type: String,
    pub model_id: Option<ModelId>,
    pub gold_amount: i32,
    pub offered_cards: Vec<CardFingerprint>,
    /// One card handed over rather than chosen between, which is how loot taken
    /// off a thief comes back.
    pub special_card: Option<CardFingerprint>,
    /// The pools a card reward draws from.
    pub card_pools: Vec<ModelId>,
    /// How many cards were offered to begin with. `offered_cards` shrinks as
    /// one is taken, so the original width is kept separately.
    pub option_count: usize,
    /// Which base odds a card reward rolled with.
    pub rarity_odds: Option<RarityOdds>,
    /// What produced the reward.
    pub source: RewardSource,
    /// The encounter a reward describes itself by, which is how loot handed
    /// back names the thief it came off.
    pub custom_description_encounter_source_id: Option<ModelId>,
    pub was_gold_stolen_back: bool,
    /// What the offer was built with, kept because the reward is the only
    /// thing that knows: a reroll builds a fresh offer with the very same
    /// options.
    #[serde(default)]
    pub creation: Option<crate::CardCreation>,
    /// Upgrade each upgradable card after generation, including on a reroll.
    /// The Future of Potions attaches this to the reward's `AfterGenerated`
    /// callback, after the ordinary card-reward modifier passes.
    #[serde(default)]
    pub upgrade_after_generation: bool,
    /// Whether the offer may be rerolled, which one relic sets on every card
    /// offer of a set. It is what puts the reroll button on the screen, and
    /// taking it clears the flag, so a reroll is offered once.
    #[serde(default)]
    pub can_reroll: bool,
    /// Whether the relic this reward is holding is waxed: a toy box marks each
    /// of the four it pulls before it offers them, so the relic arrives
    /// waxed.
    #[serde(default)]
    pub relic_is_wax: bool,
}

/// Where a reward came from.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RewardSource {
    #[default]
    None,
    Encounter,
    Other,
}

impl RewardSource {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Encounter => "encounter",
            Self::Other => "other",
        }
    }
}

impl RarityOdds {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Uniform => "uniform",
            Self::RegularEncounter => "regular_encounter",
            Self::EliteEncounter => "elite_encounter",
            Self::BossEncounter => "boss_encounter",
            Self::Shop => "shop",
        }
    }
}

impl RewardFingerprint {
    /// A reward of a kind, with no card-reward metadata on it.
    #[must_use]
    pub fn of(reward_type: &str) -> Self {
        Self {
            reward_type: reward_type.to_owned(),
            model_id: None,
            gold_amount: 0,
            offered_cards: Vec::new(),
            special_card: None,
            card_pools: Vec::new(),
            option_count: 0,
            rarity_odds: None,
            creation: None,
            upgrade_after_generation: false,
            can_reroll: false,
            relic_is_wax: false,
            source: RewardSource::None,
            custom_description_encounter_source_id: None,
            was_gold_stolen_back: false,
        }
    }

    /// A card handed over as it stands rather than one rolled out of a
    /// pool.
    #[must_use]
    pub fn with_special_card(mut self, card: CardFingerprint) -> Self {
        self.special_card = Some(card);
        self
    }

    #[must_use]
    pub fn with_model(mut self, model_id: ModelId) -> Self {
        self.model_id = Some(model_id);
        self
    }

    #[must_use]
    pub const fn with_gold(mut self, gold_amount: i32) -> Self {
        self.gold_amount = gold_amount;
        self
    }

    /// The offered cards, the pools they came from, and the odds they rolled
    /// with.
    #[must_use]
    pub fn with_cards(
        mut self,
        offered_cards: Vec<CardFingerprint>,
        card_pools: Vec<ModelId>,
        rarity_odds: RarityOdds,
    ) -> Self {
        // An offer handed a list of cards is as wide as the list. An offer
        // that rolled its own is as wide as the roll asked for, which
        // `from_creation` sets.
        self.option_count = offered_cards.len();
        self.offered_cards = offered_cards;
        self.card_pools = card_pools;
        self.rarity_odds = Some(rarity_odds);
        self
    }

    /// The options the offer was built with, which is what a reroll re-runs.
    #[must_use]
    pub fn from_creation(mut self, creation: crate::CardCreation) -> Self {
        // A card reward keeps the count it was built for. Whatever widens the
        // offer afterwards — a lasting candy putting a fourth card on it —
        // leaves that count where it was, so the screen still says how wide
        // the offer started.
        self.option_count = creation.count;
        self.creation = Some(creation);
        self
    }

    /// Whether the save schema can write this reward down.
    ///
    /// An offer whose pool is narrowed by a filter cannot be written down,
    /// and neither can one carrying a creation flag beyond the reward marker.
    /// So a recording of one carries the whole save-derived block — the pools,
    /// the width, the odds, the source — as nothing at all, and only the cards
    /// standing on the screen describe it.
    /// A colourful philosopher offers three such rewards, one per rarity.
    ///
    /// An offer handed a ready-made list of cards keeps no creation here, and
    /// is written down: it is described by the options such an offer builds for
    /// itself, which name no pool and roll no odds. A kaleidoscope's three are
    /// those.
    ///
    /// Every other kind of reward writes itself down whole.
    #[must_use]
    pub fn has_save_representation(&self) -> bool {
        if self.reward_type != "card" {
            return true;
        }
        self.creation.as_ref().is_none_or(|creation| {
            creation.filter == crate::CardFilter::Any
                && creation.flags
                    == crate::CardCreationFlags {
                        is_card_reward: true,
                        ..crate::CardCreationFlags::default()
                    }
        })
    }

    #[must_use]
    pub const fn from_source(mut self, source: RewardSource) -> Self {
        self.source = source;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RewardItem {
    pub index: usize,
    pub fingerprint: RewardFingerprint,
    pub selected: bool,
    /// Whether the player answered this reward with a button that completed it
    /// rather than with a card off it — a sacrifice. The line is selected
    /// either way; this only says which way it went. Nothing projects it.
    #[serde(default)]
    pub passed_over: bool,
    /// Whether this reward's pick screen has been opened and skipped. A rolled
    /// card offer is changed by a relic taken beside it until its screen is
    /// answered; the skip answers it without taking it, leaving the offer on
    /// the table and deaf from then on however many times it is opened again.
    /// Nothing projects it.
    #[serde(default)]
    pub opened: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RewardSet {
    pub id: RewardSetId,
    pub player_slot: String,
    pub rewards: Vec<RewardItem>,
    pub disallow_skipping: bool,
}

/// One line of an open reward screen, as the screen draws it.
///
/// A deliberate projection of [`RewardItem`] rather than the item itself:
/// [`RewardFingerprint`] also carries what the offer was *built* from — the
/// pools it rolled out of, the odds it rolled under, and the `creation` a
/// reroll would re-run — and none of that is on the screen. Two screens
/// showing the same three cards look the same however they were rolled, so
/// carrying the recipe would split observation keys a player cannot split,
/// which is the same fairness fault in the other direction.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag is one independent mark the reward screen draws"
)]
pub struct VisibleReward {
    pub index: usize,
    /// Selection opportunity associated with this reward line.
    #[serde(default)]
    pub choice: Option<crate::ChoiceOpportunity>,
    /// Which kind of reward line this is — gold, a card offer, a relic, a
    /// potion — under the name the fingerprint carries.
    pub reward_type: String,
    /// The relic or potion on the line, where the line is one thing rather
    /// than a choice between several.
    pub model_id: Option<ModelId>,
    pub gold_amount: i32,
    /// The cards a card reward is offering, as its screen lays them out.
    pub offered_cards: Vec<CardFingerprint>,
    /// A card handed over rather than chosen between.
    pub special_card: Option<CardFingerprint>,
    /// How wide the offer started, which stays on the screen as cards are
    /// taken out of it.
    pub option_count: usize,
    /// Whether the reroll button is on the screen.
    pub can_reroll: bool,
    /// Whether the relic this line is holding is waxed — a waxed relic is
    /// drawn as one.
    pub relic_is_wax: bool,
    /// Whether the line has been taken, and whether it has been passed over.
    /// Both change what the screen shows and what it will still accept.
    pub selected: bool,
    pub passed_over: bool,
}

impl VisibleReward {
    #[must_use]
    pub fn of(item: &RewardItem) -> Self {
        Self {
            index: item.index,
            choice: None,
            reward_type: item.fingerprint.reward_type.clone(),
            model_id: item.fingerprint.model_id.clone(),
            gold_amount: item.fingerprint.gold_amount,
            offered_cards: item.fingerprint.offered_cards.clone(),
            special_card: item.fingerprint.special_card.clone(),
            option_count: item.fingerprint.option_count,
            can_reroll: item.fingerprint.can_reroll,
            relic_is_wax: item.fingerprint.relic_is_wax,
            selected: item.selected,
            passed_over: item.passed_over,
        }
    }

    /// The lines of an open reward set, in screen order.
    #[must_use]
    pub fn of_set(set: &RewardSet) -> Vec<Self> {
        set.rewards.iter().map(Self::of).collect()
    }
}

/// One point of the current act's map, as the map screen draws it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VisibleMapPoint {
    pub coord: crate::MapCoord,
    pub point_type: crate::MapPointType,
    /// The points this one has paths to, one row up.
    pub children: Vec<crate::MapCoord>,
    /// Whether the walk has already stood here. The path drawn on the screen
    /// is exactly this set.
    pub visited: bool,
}

/// The current act's map, as the map screen shows it.
///
/// Everything here is on that screen the moment the act opens: the grid, the
/// icon on every point, the paths between them, the path already walked — and
/// the act's boss, whose own art is drawn on the boss point, and its ancient,
/// whose icon is drawn on the ancient point. What sits *behind* an unwalked
/// icon — the
/// encounter, the event, the rewards already rolled for it — is not here and
/// never will be: the belief layer erases exactly that.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VisibleMap {
    /// Which act's map this is, zero-based, matching `current_act`.
    pub act: usize,
    pub width: u8,
    pub height: u8,
    pub start: crate::MapCoord,
    pub boss_coord: crate::MapCoord,
    pub second_boss_coord: Option<crate::MapCoord>,
    pub points: Vec<VisibleMapPoint>,
    /// The act's previewed boss, drawn on the boss point.
    pub boss: ModelId,
    pub second_boss: Option<ModelId>,
    /// The act's ancient, drawn on the ancient point.
    pub ancient: ModelId,
}

/// The live counts the game's own card and relic text shows the player.
///
/// Every one of them is a tally of things that happened on the screen, so a
/// player who watched has it; the cards that read them draw the number.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct VisibleTallies {
    /// Plays begun this turn, which an Echo Form caps itself on.
    pub first_plays_started_this_turn: u32,
    /// Zero-cost attack plays this turn, which a Feral counts.
    pub zero_cost_attack_plays_this_turn: u32,
    /// Status cards drawn this turn, which an Iteration asks about.
    pub status_cards_drawn_this_turn: u32,
    /// Energy spent this turn, which a Helix Drill counts its hits off.
    pub energy_spent_this_turn: i32,
    /// The orbs channelled this combat, in order, which a Voltaic counts.
    pub orbs_channeled_this_combat: Vec<ModelId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag is one independent marker the pinned build carries and the recorder projects"
)]
pub struct CombatState {
    /// Whether a creature whose death counts as a kill has gone down since
    /// the play now running began. It is whether a target was killed, asked
    /// after the swing and crossed with the fatal test made before it.
    #[serde(default)]
    pub fatal_kill_this_play: bool,
    pub encounter_model: ModelId,
    /// Whether the player is barred from acting. It is set for the whole of the
    /// turn handover, in both directions, and cleared once their turn opens.
    pub player_actions_disabled: bool,
    /// Whether the card whose listeners are running was an attack.
    pub played_card_was_attack: bool,
    /// Whether the play running right now is one the player chose. A listener
    /// that counts a turn's plays skips the ones nobody chose.
    #[serde(default)]
    pub played_card_was_chosen: bool,
    /// What type the card that was just played is, which a listener that counts
    /// one kind of card reads.
    #[serde(default)]
    pub played_card_type: Option<CardType>,
    /// Which card was just played.
    #[serde(default)]
    pub played_card: Option<CardInstanceId>,
    /// What that card's play paid in energy, written as it was paid. The
    /// card's own cost can move while its play is under way — a card that
    /// makes itself free once the pet has swung does it inside its own play —
    /// so what a play cost is not read back off the card afterwards.
    #[serde(default)]
    pub played_card_energy_spent: i32,
    /// This run's one player is taking an extra turn: the sides do not
    /// switch, the round does not wind on, and the enemy side does not act at
    /// all. It is settled where the sides would otherwise have switched, and
    /// cleared there next time.
    #[serde(default)]
    pub taking_extra_turn: bool,
    /// Which pass of the play loop the play now running is on. It is cleared
    /// to nothing between plays, so outside a play it reads as the first
    /// pass.
    #[serde(default)]
    pub played_card_play_index: i32,
    /// Whether something has made the player ready to end their turn without
    /// them choosing to. Ending a turn this way does not block, so the
    /// handover runs once whatever reached it has settled rather than inside
    /// it.
    #[serde(default)]
    pub turn_end_is_owed: bool,
    /// Who the player has landed a powered blow on this turn, one entry per
    /// blow, which is what a Beat Into Shape counts. It is cleared as the
    /// player's turn opens.
    #[serde(default)]
    pub powered_blows_this_turn: Vec<CreatureCombatId>,
    /// Whether the player has been hurt this turn, which a few cards read.
    pub player_hurt_this_turn: bool,
    /// A blow block did not stop whole has landed on the player since their
    /// own turn opened. It is not the same as losing HP — a blow worth nothing
    /// against a player standing behind no block is not fully blocked either.
    #[serde(default)]
    pub player_took_an_unstopped_blow_this_turn: bool,
    /// The same carried across the turn boundary, which is what "last player
    /// turn" covers: the player's own last turn and the enemy turn that
    /// followed it.
    #[serde(default)]
    pub player_took_an_unstopped_blow_last_turn: bool,
    pub round: u32,
    /// The identity the next power instance takes. Two instances of one power
    /// on one creature are two listeners and two things an effect can act on,
    /// and
    /// this is what tells them apart.
    #[serde(default)]
    pub next_power_instance: u64,
    /// The id the next creature to join the fight takes. It only ever goes
    /// up, so a creature that arrives after another has died and been cleared
    /// does not take the dead one's number.
    #[serde(default)]
    pub next_creature_id: u64,
    /// The fight has been announced and has not ended.
    ///
    /// Every creature joins and does whatever it does on arrival before the
    /// fight is in progress, and a whole family of commands — gaining and
    /// losing block among them — does nothing at all while a fight is over or
    /// ending. So block a monster gains as it joins the room is silently
    /// dropped, which is why a Cubex Construct that asks for thirteen has
    /// none. The flag is cleared before the end of the fight is announced, so
    /// a listener that answers an HP change only while a fight is in progress
    /// does not answer the victory heal.
    #[serde(default)]
    pub in_progress: bool,
    /// The fight has been announced and has since ended. It is the other half
    /// of being over or ending without still starting: a combat-scope pass
    /// reaches nobody at all from here on, while a fight that has not been
    /// announced yet is still starting and its passes run.
    #[serde(default)]
    pub has_ended: bool,
    pub side: CombatSide,
    pub player: PlayerCombatState,
    pub creatures: Vec<Creature>,
    pub queue: VecDeque<Effect>,
    pub suspended_choice: Option<SuspendedChoice>,
    /// The screens death listeners asked for, in the order they asked, each
    /// with everything its paused listener still owes behind the answer.
    ///
    /// The build puts each one on the player's action queue as an action of
    /// its own, behind the action that was running when it was asked for, so
    /// it is shown only once that action has finished and the fight has been
    /// checked for a win. A fight that ends first cancels them, and what they
    /// owed never runs.
    #[serde(default, skip_serializing_if = "VecDeque::is_empty")]
    pub queued_hook_screens: VecDeque<SuspendedHookScreen>,
    pub victory_rewards: Vec<RewardFingerprint>,
    pub generated_victory_rewards: Option<crate::GeneratedCombatRewardSpec>,
    pub reward_set: Option<RewardSet>,
    /// The fight is won and its reward screen is up. This outlives the reward
    /// set, which is dropped once everything on it has been taken, and is what
    /// keeps the room waiting to be left rather than back at combat priority.
    pub rewards_open: bool,
    /// A fight that pays no rewards and was won by killing the last enemy is
    /// left standing exactly as the blow left it — piles, powers, phase and
    /// all — until the room is proceeded from, which is when the game ends
    /// it. This says that ending is still owed.
    #[serde(default)]
    pub finish_owed: bool,
    pub next_choice_id: u64,
    pub next_reward_set_id: u64,
    /// What left the fight on its own feet rather than dying, named by model.
    /// A fight pays out less gold for each
    /// of the encounter's monsters that got away.
    #[serde(default)]
    pub escaped_creatures: Vec<ModelId>,
    /// Gold carried by creatures that escaped. The build keeps it on the
    /// encounter itself; keeping the amount here preserves the same outcome
    /// after the escaping creature and its powers leave the combat.
    /// Gremlin Merc reads whether the escaped Fat Gremlin carried any stolen
    /// gold when calculating the room's base-gold proportion.
    #[serde(default)]
    pub escaped_stolen_gold: i32,
    /// Every monster model that has stood on the enemy side of this fight,
    /// whether it was there at the start or was summoned into it. There is one
    /// entry per model however many creatures of it there were.
    #[serde(default)]
    pub spawned_enemy_models: Vec<ModelId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VisibleHistoryEntry {
    pub sequence: u64,
    pub event: String,
}

/// What the run has been watched doing, bounded: the newest entries verbatim
/// and a running digest of everything older. A run emits events for its whole
/// life, and an unbounded log made every state clone, observation, and
/// canonical key grow with run age. The digest keeps the information-set
/// property the observation key needs — two runs whose visible pasts differ
/// anywhere keep different digests even after the differing entries have been
/// folded out of the tail — at O(1) size.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct VisibleHistory {
    /// FNV-1a chain over every entry folded out of `recent`, oldest first.
    /// Zero until the first fold; a pure function of the folded prefix, so
    /// two states with the same visible past always agree.
    pub folded_digest: u64,
    /// How many entries the digest has absorbed.
    pub folded_count: u64,
    /// The newest entries, oldest first, at most [`Self::RECENT_CAP`].
    pub recent: std::collections::VecDeque<VisibleHistoryEntry>,
}

impl VisibleHistory {
    /// How many entries stay verbatim before the digest absorbs the oldest.
    pub const RECENT_CAP: usize = 32;

    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

    pub fn push(&mut self, entry: VisibleHistoryEntry) {
        self.recent.push_back(entry);
        while self.recent.len() > Self::RECENT_CAP {
            let folded = self.recent.pop_front().expect("longer than the cap");
            self.fold(&folded);
        }
    }

    fn fold(&mut self, entry: &VisibleHistoryEntry) {
        let mut hash = if self.folded_count == 0 {
            Self::FNV_OFFSET
        } else {
            self.folded_digest
        };
        let mut absorb = |byte: u8| {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(Self::FNV_PRIME);
        };
        for byte in entry.sequence.to_le_bytes() {
            absorb(byte);
        }
        for byte in entry.event.as_bytes() {
            absorb(*byte);
        }
        // A terminator so "ab" then "c" cannot collide with "a" then "bc".
        absorb(0xff);
        self.folded_digest = hash;
        self.folded_count += 1;
    }

    /// Every entry the run has emitted, folded and verbatim together.
    #[must_use]
    pub fn len(&self) -> u64 {
        self.folded_count + self.recent.len() as u64
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.folded_count == 0 && self.recent.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RunPlayerState {
    pub slot: String,
    pub current_hp: i32,
    pub max_hp: i32,
    /// Energy the player starts each combat turn with.
    pub max_energy: i32,
    pub gold: i32,
    pub deck: Vec<CardFingerprint>,
    pub potions: Vec<Option<ModelId>>,
    /// Relics the player is holding, in the order they were obtained.
    pub relics: Vec<RelicInstance>,
    /// How many orb slots this player's character opens every fight with. It
    /// is nothing for four of the five characters and three for the Defect,
    /// and it is what the orb queue's capacity starts at as a fight is set
    /// up.
    #[serde(default)]
    pub base_orb_slots: i32,
    /// Rarity and potion odds that drift across the whole run.
    pub odds: PlayerOdds,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GameState {
    pub compatibility_id: String,
    pub origin: ConformanceOrigin,
    pub rng: RunRngSet,
    pub player_rng: PlayerRngSet,
    pub run_player: RunPlayerState,
    pub run: Option<crate::RunState>,
    pub combat: Option<CombatState>,
    pub terminal: Option<RunResult>,
    pub visible_history: VisibleHistory,
    pub event_sequence: u64,
    pub next_card_instance_id: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct PortableGameState {
    compatibility_id: String,
    origin: ConformanceOrigin,
    rng: SerializableRunRngSet,
    player_rng: SerializablePlayerRngSet,
    run_player: RunPlayerState,
    run: Option<crate::RunState>,
    combat: Option<CombatState>,
    terminal: Option<RunResult>,
    visible_history: VisibleHistory,
    event_sequence: u64,
    next_card_instance_id: u64,
}

/// `PortableGameState` serialized by reference: the same fields in the same
/// order, borrowing the state instead of cloning it, for the paths that only
/// serialize — projection and the canonical state key.
#[derive(Serialize)]
pub(crate) struct PortableGameStateRef<'a> {
    compatibility_id: &'a str,
    origin: ConformanceOrigin,
    rng: SerializableRunRngSet,
    player_rng: SerializablePlayerRngSet,
    run_player: &'a RunPlayerState,
    run: Option<&'a crate::RunState>,
    combat: Option<&'a CombatState>,
    terminal: Option<RunResult>,
    visible_history: &'a VisibleHistory,
    event_sequence: u64,
    next_card_instance_id: u64,
}

impl GameState {
    pub(crate) fn portable_ref(&self) -> PortableGameStateRef<'_> {
        PortableGameStateRef {
            compatibility_id: &self.compatibility_id,
            origin: self.origin,
            rng: self.rng.to_serializable(),
            player_rng: self.player_rng.to_serializable(),
            run_player: &self.run_player,
            run: self.run.as_ref(),
            combat: self.combat.as_ref(),
            terminal: self.terminal,
            visible_history: &self.visible_history,
            event_sequence: self.event_sequence,
            next_card_instance_id: self.next_card_instance_id,
        }
    }

    pub(crate) fn to_value(&self) -> Result<Value, EngineError> {
        serde_json::to_value(self.portable_ref()).map_err(EngineError::serialization)
    }

    pub(crate) fn from_value(value: Value) -> Result<Self, EngineError> {
        let portable: PortableGameState =
            serde_json::from_value(value).map_err(EngineError::serialization)?;
        Ok(Self {
            compatibility_id: portable.compatibility_id,
            origin: ConformanceOrigin::PortableContinuation,
            rng: RunRngSet::from_serializable(&portable.rng),
            player_rng: PlayerRngSet::from_serializable(&portable.player_rng),
            run_player: portable.run_player,
            run: portable.run,
            combat: portable.combat,
            terminal: portable.terminal,
            visible_history: portable.visible_history,
            event_sequence: portable.event_sequence,
            next_card_instance_id: portable.next_card_instance_id,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunResult {
    Victory,
    Defeat,
}

/// The displayed per-copy state of a combat card: everything the card's
/// face shows beyond its print. Something that changed a copy — a stew's
/// replay mark, a granted keyword, a grown damage number, a cost put up or
/// down — changed what the player sees, so it belongs to the observation.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag is one independent keyword the card's face displays"
)]
pub struct CardFace {
    /// The card's own energy cost as it stands now.
    pub energy_cost: i32,
    /// What has been put over that cost for the turn, the fight, or until the
    /// next play — the displayed number is the sum.
    pub cost_this_turn: i32,
    pub cost_this_combat: i32,
    pub cost_until_played: i32,
    pub costs_x: bool,
    pub star_cost: i32,
    /// Extra times this copy's effect runs per play: a Soldier's Stew or a
    /// Hidden Gem shows as exactly this.
    pub replay_count: i32,
    /// The keywords as displayed, granted ones and this-turn grants folded
    /// in.
    pub ethereal: bool,
    pub retain: bool,
    pub sly: bool,
    pub affliction: Option<ModelId>,
    /// What has grown this copy's own damage, shown on the face.
    pub damage_bonus: i32,
    /// What the copy's enchantment has grown its damage by, which the damage
    /// number on the card shows: a Momentum's extra damage, added to the value
    /// before any listener speaks and drawn on the card because the
    /// enchantment shows its amount. What the enchantment keeps for itself
    /// stays hidden; what the face carries is the number the screen draws, the
    /// same resolution rule the shown intent damage follows.
    #[serde(default)]
    pub enchantment_damage_bonus: i32,
    /// Whether the enchantment still acts, as the card shows it: a Glam that
    /// has spent its replay greys out on the card, because using it disables
    /// the enchantment for the rest of the fight. What the enchantment keeps
    /// for itself stays hidden; this is what it shows as.
    #[serde(default)]
    pub enchantment_status: EnchantmentStatus,
}

/// A Momentum's extra damage, under the name the enchantment keeps it under
/// (`sts2-content/src/enchantments.rs`, `momentum`). It is the one card-side
/// number an enchantment keeps that reaches the screen, so it is the one the
/// face resolves; the other two in the registry are a Glam's record of having
/// been used, which shows as [`CardFace::enchantment_status`], and a Regret's
/// count of the hand it was held in, which shows nowhere at all.
const DISPLAYED_ENCHANTMENT_DAMAGE_LOCAL: &str = "ExtraDamage";

impl CardFace {
    #[must_use]
    pub fn of(card: &CardInstance) -> Self {
        Self {
            energy_cost: card.energy_cost,
            cost_this_turn: card.cost_this_turn,
            cost_this_combat: card.cost_this_combat,
            cost_until_played: card.cost_until_played,
            costs_x: card.costs_x,
            star_cost: card.star_cost,
            replay_count: card.replay_count,
            ethereal: card.ethereal,
            retain: card.retain || card.retain_this_turn,
            sly: card.sly || card.sly_this_turn,
            affliction: card.affliction.clone(),
            damage_bonus: card.damage_bonus,
            enchantment_damage_bonus: card
                .enchantment_locals
                .get(DISPLAYED_ENCHANTMENT_DAMAGE_LOCAL)
                .copied()
                .unwrap_or(0),
            enchantment_status: card.enchantment_status,
        }
    }
}

/// One combat card as a hidden-order pile shows it: the print and the face,
/// without the instance identity a player cannot localize.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PileCard {
    pub fingerprint: CardFingerprint,
    pub face: CardFace,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CardHandle {
    pub index: usize,
    pub card_id: CardInstanceId,
    pub fingerprint: CardFingerprint,
    /// The displayed per-copy state beside the print.
    #[serde(default)]
    pub face: CardFace,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TargetHandle {
    pub combat_id: CreatureCombatId,
    pub model_id: ModelId,
}

/// What a card in hand draws while it is aimed at one creature.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AimedDamage {
    pub combat_id: CreatureCombatId,
    pub damage: i32,
}

/// The numbers a combat card's face shows, resolved rather than printed.
///
/// A card does not draw its base: each number on its face is worked out by
/// running the whole pass over that number rather than by reading the number
/// itself — the damage passes for damage, the block passes for block. So a
/// Strike in a strengthened hand reads nine on its own face, and reads more
/// again while it is aimed at something Vulnerable. This is the same "what the
/// screen shows" rule the advertised intent damage already follows.
///
/// Those passes run for a card standing in the hand or the play pile and
/// nowhere else, so a card in the draw, discard or exhaust pile draws its
/// print and carries no preview here.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CardPreview {
    /// Which card in hand this belongs to. The list runs in hand order beside
    /// [`AgentObservation::hand`]; the id is what lets one entry stand alone.
    pub card_id: CardInstanceId,
    /// The damage the card draws with nothing aimed, which is how a card
    /// resting in the hand is drawn. The base the pass runs over is this
    /// copy's own damage number as it stands — what a Rampage has grown it to,
    /// and not what its print says. Nothing where the card carries no damage
    /// number at all.
    pub damage: Option<i32>,
    /// The block the card draws, on the same footing. The creature the block
    /// passes are asked about is the one gaining the block, which is always
    /// the card's own owner whatever the card is pointed at, so aiming never
    /// moves it.
    pub block: Option<i32>,
    /// What the card draws while it is aimed, one entry per enemy the play
    /// can name — the same enemies the `Action::PlayCard` targets enumerate,
    /// in the combat's own creature order. Empty for a card that names no
    /// enemy to aim at, and empty as well for one that resolves no damage var
    /// to draw.
    pub aimed_damage: Vec<AimedDamage>,
    /// What a card that lands on all or a random one of them draws while it
    /// is held up: the number is worked out against every hittable enemy and
    /// shown only where they all agree, falling back to the unaimed pass where
    /// they do not.
    /// Nothing for every other target type.
    pub multi_target_damage: Option<i32>,
}

impl CardPreview {
    /// A card with no number on its face.
    #[must_use]
    pub const fn blank(card_id: CardInstanceId) -> Self {
        Self {
            card_id,
            damage: None,
            block: None,
            aimed_damage: Vec::new(),
            multi_target_damage: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DecisionContext {
    MapNavigation {
        current: Option<crate::MapCoord>,
        destinations: Vec<crate::MapCoord>,
    },
    Event {
        model_id: ModelId,
        options: Vec<crate::EventOption>,
    },
    Shop {
        model_id: Option<ModelId>,
        offers: Vec<crate::ShopOffer>,
        removal_price: i32,
        removal_used: bool,
        removal_choice: crate::ChoiceOpportunity,
    },
    RestSite {
        options: Vec<crate::RestSiteOptionState>,
        left_open: bool,
    },
    Treasure {
        relics: Vec<ModelId>,
    },
    /// The fog board an event stood up, with this many divinations left to
    /// spend on it. The event's page is still standing
    /// under it and nothing else can be answered while it is up.
    CrystalSphere {
        divinations: i32,
    },
    ActTransition {
        next_act: usize,
    },
    CombatPriority {
        player_slot: String,
        turn: u32,
    },
    ChooseCards {
        #[serde(default)]
        semantics: crate::ChoiceSemantics,
        choice_id: ChoiceId,
        /// How the candidates are named, so that a caller knows whether an
        /// index is a combat-pile position, a deck position, or a place in a
        /// list of cards that stand nowhere.
        identity: ChoiceIdentity,
        /// The concrete screen family, retained for exact script offer
        /// encoding.
        screen: ChoiceScreen,
        minimum: usize,
        maximum: usize,
        /// Whether the screen accepts nothing at all despite its minimum.
        cancelable: bool,
        candidates: Vec<CardHandle>,
        /// Buttons exposed beside the cards on a card-reward screen.
        #[serde(default)]
        alternatives: Vec<ChoiceAlternative>,
        /// The candidates grouped into bundles, where the screen is one of the
        /// bundle screens: an answer names one whole bundle and nothing else.
        #[serde(default)]
        bundles: Vec<Vec<usize>>,
        /// What the answer being built already holds, on a screen with more
        /// answers than the action list can lay out: such a screen is worked
        /// the way the build's own is, one card toggled in at a time, and this
        /// is the selection standing between the clicks. Empty on every screen
        /// small enough to enumerate whole answers.
        #[serde(default)]
        picked: Vec<CardHandle>,
    },
    Rewards {
        set_id: RewardSetId,
        player_slot: String,
        /// What the screen is offering. A set id names which screen this is,
        /// not what is on it, and two reward screens with different gold or
        /// different cards are two sights a player tells apart at a glance —
        /// so the offers ride the decision, and the observation key moves
        /// with them.
        #[serde(default)]
        offers: Vec<VisibleReward>,
    },
    RoomProceed,
    Terminal {
        result: RunResult,
    },
}

/// The two divination tools the crystal sphere's board offers: the big one
/// clears the eight cells around the one clicked as well, the small one clears
/// only it.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DivinationTool {
    Big,
    Small,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Action {
    ChooseMap {
        destination: crate::MapCoord,
    },
    ChooseEvent {
        /// Where the option stands on the page. An event may lay out two
        /// options that share an id — the potion trade lays out one per potion
        /// — so the place on the page is what names one.
        index: usize,
        option_id: String,
    },
    BuyShopItem {
        offer_index: usize,
        fingerprint: crate::ShopItem,
    },
    /// Opens the merchant's card removal, which is settled by the card choice
    /// it puts up. Nothing is paid until a card is actually chosen.
    BuyCardRemoval,
    UsePotion {
        slot: usize,
        model_id: ModelId,
        target: Option<TargetHandle>,
    },
    /// The potion in this slot is thrown away to make room. It is available
    /// out of a fight and, inside one, only while the player is free to play
    /// cards — so the belt answers wherever the player may act and nowhere a
    /// screen is standing.
    DiscardPotion {
        slot: usize,
        model_id: ModelId,
    },
    /// One entry of the list the rest site generated, named by where it
    /// stands in that list and by what it is. An
    /// option that opens a screen — the smith, the cook — is settled by the
    /// card choice it puts up, and nothing is paid until a card is chosen.
    RestOption {
        index: usize,
        option: crate::RestSiteOption,
    },
    TakeTreasure {
        relic: ModelId,
    },
    /// One divination spent on this cell, with the tool the screen's own two
    /// buttons pick between. A cell the fog
    /// has already come off is not clickable, so only a hidden one is offered.
    UncoverCrystalSphere {
        x: i32,
        y: i32,
        tool: DivinationTool,
    },
    AdvanceAct,
    PlayCard {
        card: CardHandle,
        target: Option<TargetHandle>,
    },
    EndTurn {
        turn: u32,
    },
    ChooseCards {
        choice_id: ChoiceId,
        cards: Vec<CardHandle>,
    },
    /// Answering a screen with one of the buttons that is not a card, which a
    /// relic put there.
    ChooseAlternative {
        choice_id: ChoiceId,
        option_id: String,
    },
    ClaimReward {
        set_id: RewardSetId,
        reward_index: usize,
        fingerprint: RewardFingerprint,
    },
    Proceed,
}

impl Action {
    #[must_use]
    pub const fn family(&self) -> &'static str {
        match self {
            Self::ChooseMap { .. } => "map_navigation",
            Self::ChooseEvent { .. } => "event",
            Self::BuyShopItem { .. } | Self::BuyCardRemoval => "shop",
            Self::RestOption { .. } => "rest_site",
            Self::TakeTreasure { .. } => "treasure",
            Self::UncoverCrystalSphere { .. } => "crystal_sphere",
            Self::UsePotion { .. } | Self::DiscardPotion { .. } => "potion",
            Self::AdvanceAct => "act_transition",
            Self::PlayCard { .. } | Self::EndTurn { .. } => "combat_priority",
            Self::ChooseCards { .. } => "choose_cards",
            Self::ChooseAlternative { .. } => "choose_alternative",
            Self::ClaimReward { .. } => "rewards",
            Self::Proceed => "room_proceed",
        }
    }
}

/// How far back a [`MoveHistory`] keeps the two logs in order.
///
/// Every repeat and cooldown rule in the registry reads at most three entries
/// back — `with_cooldown(3)` is the deepest, `CanRepeatTimes(2)` the widest —
/// so four entries is the whole of what a machine's own weights can ask about,
/// with one to spare. The tallies below carry the rest of the history, so
/// nothing a branch condition reads is lost to the cut.
pub const MOVE_HISTORY_TAIL: usize = 4;

/// What a player who watched the fight knows about a creature's move machine.
///
/// The engine keeps two logs per creature — the move states entered, in roll
/// order, and the move states actually performed (`Creature::state_log`,
/// `Creature::performed_log`) — and both are *watchable*: a state is entered
/// exactly when the creature starts showing that move's intent, and it is
/// performed exactly when the move runs. So a digest of them is derivable by
/// anyone who sat through the fight, which is the fairness test, and it is not
/// the numbers the monster keeps to itself (`Creature::locals`), which stay
/// hidden.
///
/// What reads it: the `EnteredStateAtLeast` and `HasEnteredState` branch
/// conditions — the Knowledge Demon's count of the curses it has dealt is one
/// — the `TimesOwnerEnteredState` and `TimesOwnerPerformedState` counts, the
/// growing repeat counts an intent advertises, and the cooldown and no-repeat
/// weights of every random machine.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct MoveHistory {
    /// The last [`MOVE_HISTORY_TAIL`] move states entered, oldest first. The
    /// last entry is the state the creature stands on, which is the move its
    /// intent advertises.
    #[serde(default)]
    pub recent_states: Vec<String>,
    /// The last [`MOVE_HISTORY_TAIL`] move states performed, oldest first.
    #[serde(default)]
    pub recent_performed: Vec<String>,
    /// How many times each move state has been entered, over the whole fight.
    #[serde(default)]
    pub entered_counts: std::collections::BTreeMap<String, u32>,
    /// How many times each move state has been performed, over the whole
    /// fight. A state is entered a turn before it runs, so the two tallies
    /// differ by exactly the move now advertised.
    #[serde(default)]
    pub performed_counts: std::collections::BTreeMap<String, u32>,
}

impl MoveHistory {
    /// The digest of one creature's two logs.
    #[must_use]
    pub fn of(creature: &Creature) -> Self {
        fn tail(log: &[String]) -> Vec<String> {
            log[log.len().saturating_sub(MOVE_HISTORY_TAIL)..].to_vec()
        }
        fn tally(log: &[String]) -> std::collections::BTreeMap<String, u32> {
            let mut counts = std::collections::BTreeMap::new();
            for entry in log {
                *counts.entry(entry.clone()).or_insert(0) += 1;
            }
            counts
        }
        Self {
            recent_states: tail(&creature.state_log),
            recent_performed: tail(&creature.performed_log),
            entered_counts: tally(&creature.state_log),
            performed_counts: tally(&creature.performed_log),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VisibleCreature {
    pub combat_id: CreatureCombatId,
    pub model_id: ModelId,
    /// Which side the creature fights on, plainly visible on the screen.
    pub side: CombatSide,
    pub current_hp: i32,
    pub max_hp: i32,
    pub block: i32,
    /// How many blows this creature has taken that block did not stop whole —
    /// every one of them landed on the screen, so a player counted them.
    #[serde(default)]
    pub unblocked_blows_taken: u32,
    pub powers: Vec<PowerInstance>,
    pub intent: Option<Intent>,
    /// What the creature's move machine has done so far, as far as watching
    /// it says.
    #[serde(default)]
    pub move_history: MoveHistory,
}

/// The version of [`AgentObservation`]'s shape. Anything added, removed, or
/// re-ordered moves every `observation_key`, so the number moves with it and
/// a persisted key names the version it was hashed under.
///
/// Version 2 added the draw pile's contents (order-erased), the exhaust and
/// play piles, max energy, stars, the turn counter, and the orb queue.
/// Version 3 added each creature's side. Version 4 carries relics as
/// instances rather than bare model ids: a relic's counters and its
/// once-a-fight latch. Version 5 puts the card's face on every card view —
/// its live cost, its replay marks, its added keywords, its grown damage.
/// Version 6 adds an open reward screen's offer rather than only its set id,
/// the current act's map — points, types, edges, walked path, previewed boss
/// and ancient — a digest of each creature's move history, what the card's
/// enchantment shows, and the run's ascension, drift odds and live card-text
/// tallies. Version 7 bounds `visible_history`: a fixed tail of recent
/// entries plus a digest of the older prefix, instead of the whole event
/// log. Version 8 carries the resolved numbers the cards in hand display —
/// [`CardPreview`], with the aimed number against each enemy the play can
/// name. Version 10 adds public selection semantics and macro opportunities.
/// Version 9 retains the concrete card-selection screen and its
/// non-card alternatives, so a consumer can reproduce the exact offer
/// context and positional indexes.
pub const AGENT_OBSERVATION_VERSION: u32 = 10;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentObservation {
    pub player_slot: Option<String>,
    pub current_hp: Option<i32>,
    pub max_hp: Option<i32>,
    pub block: Option<i32>,
    pub energy: Option<i32>,
    /// The energy the turn opens with, which the player reads off the orb.
    pub max_energy: Option<i32>,
    /// The Regent's second resource, off the counter the screen shows.
    pub stars: Option<i32>,
    pub gold: Option<i32>,
    pub deck: Vec<CardFingerprint>,
    pub potions: Vec<Option<ModelId>>,
    /// The relics as held: identity and the visible state each keeps — its
    /// counters and its latch are shown on the screen.
    pub relics: Vec<RelicInstance>,
    pub current_act: Option<usize>,
    pub floor: Option<u32>,
    pub map_coord: Option<crate::MapCoord>,
    /// The ascension level the run was started at, which its setup screen
    /// named.
    #[serde(default)]
    pub ascension: Option<u8>,
    /// The current act's map, whole. Nothing outside a run has one.
    #[serde(default)]
    pub map: Option<VisibleMap>,
    /// The rarity and potion odds this run has drifted to. Not printed
    /// anywhere, but derivable by anyone who counted the rewards they were
    /// offered — the same standing the belief layer already gives
    /// `unknown_room_odds`.
    #[serde(default)]
    pub odds: Option<PlayerOdds>,
    /// The act's pity state for unknown map points, on the same footing.
    #[serde(default)]
    pub unknown_room_odds: Option<crate::UnknownRoomOdds>,
    /// The player's turn counter, which the fight displays.
    pub turn: Option<u32>,
    /// The live counts the game's own card text shows.
    #[serde(default)]
    pub tallies: VisibleTallies,
    pub hand: Vec<CardHandle>,
    /// What each card in hand displays, one entry per [`Self::hand`] slot in
    /// the same order. Nothing outside a combat has a hand to draw.
    #[serde(default)]
    pub hand_previews: Vec<CardPreview>,
    pub draw_pile_size: usize,
    /// The draw pile's contents in a canonical order — the view screen's
    /// sorted list, not the pile's. What the pile holds is the player's to
    /// see; the order it will come off in is not, and neither are the
    /// instance identities of same-looking copies, so this is prints and
    /// faces sorted by their serialized form rather than handles in pile
    /// order.
    pub draw_pile: Vec<PileCard>,
    pub discard: Vec<CardHandle>,
    /// The exhaust pile, which the player can open at any time.
    pub exhaust: Vec<CardHandle>,
    /// Cards standing mid-resolution, visible on the table.
    pub play: Vec<CardHandle>,
    /// The orb slots and what stands in them, all of it on screen.
    pub orbs: Option<OrbQueue>,
    pub creatures: Vec<VisibleCreature>,
    pub decision: DecisionContext,
    pub visible_history: VisibleHistory,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SemanticEvent {
    pub sequence: u64,
    pub kind: String,
    pub details: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TransitionCheckpoint {
    pub context: String,
    pub state_key: StateKey,
    pub projection: PortableState,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Transition {
    pub accepted_action: Action,
    pub next_decision: DecisionContext,
    pub events: Vec<SemanticEvent>,
    pub checkpoints: Vec<TransitionCheckpoint>,
    pub rng_before: SerializableRunRngSet,
    pub rng_after: SerializableRunRngSet,
    pub player_rng_before: SerializablePlayerRngSet,
    pub player_rng_after: SerializablePlayerRngSet,
    pub state_key: StateKey,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionScope {
    RunStartCore,
    RunCore,
    CombatCore,
}

impl ProjectionScope {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::RunStartCore => "run_start.core",
            Self::RunCore => "run.core",
            Self::CombatCore => "combat.core",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PortableState {
    pub projection_version: u32,
    pub scope: ProjectionScope,
    pub state: Value,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StateKey(pub [u8; 32]);

impl Display for StateKey {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// A canonical key over what a policy can see: `agent_observation`, hashed.
///
/// Two states the player cannot tell apart share this key while their
/// `StateKey`s differ — a draw pile in a different hidden order, an RNG
/// stream at a different position. It is what an information-set
/// transposition table keys on; keying one by `StateKey` would let a search
/// distinguish states no player could.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ObservationKey(pub [u8; 32]);

impl Display for ObservationKey {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    UnsupportedBuild,
    UnsupportedContent,
    InvalidConfig,
    InvalidReplay,
    WrongDecisionContext,
    IllegalAction,
    InternalInvariant,
}

#[derive(Debug)]
pub struct EngineError {
    pub code: ErrorCode,
    pub decision: Option<String>,
    pub details: BTreeMap<String, Value>,
    source: Option<Box<dyn Error + Send + Sync>>,
}

impl EngineError {
    #[must_use]
    pub fn new(code: ErrorCode) -> Self {
        Self {
            code,
            decision: None,
            details: BTreeMap::new(),
            source: None,
        }
    }

    #[must_use]
    pub fn with_detail(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }

    pub(crate) fn serialization(error: serde_json::Error) -> Self {
        Self {
            code: ErrorCode::InternalInvariant,
            decision: None,
            details: BTreeMap::new(),
            source: Some(Box::new(error)),
        }
    }
}

impl Display for EngineError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}", self.code)?;
        // Details carry the model ID and run location that make an unsupported
        // or illegal outcome actionable, so they belong in the message.
        for (index, (key, value)) in self.details.iter().enumerate() {
            let separator = if index == 0 { ": " } else { ", " };
            write!(formatter, "{separator}{key}={value}")?;
        }
        Ok(())
    }
}

impl Error for EngineError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source
            .as_deref()
            .map(|source| source as &(dyn Error + 'static))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvariantViolation {
    DuplicateCardId(CardInstanceId),
    DuplicateCreatureId(CreatureCombatId),
    InvalidPlayerCreature(CreatureCombatId),
    StaleTarget(CreatureCombatId),
    ChoiceMismatch,
    UnsettledQueue,
}

impl ChoicePurpose {
    /// Explicit engine-owned purposes share metadata with their public offers.
    #[must_use]
    pub fn semantics(&self) -> crate::ChoiceSemantics {
        match self {
            Self::RunCommands { .. } => crate::ChoiceSemantics::default(),
            Self::AddChosenCardsToDeck | Self::AddRewardCard { .. } => {
                crate::ChoiceSemantics::deck_gain()
            }
            Self::RestSmith { .. } => crate::ChoiceSemantics::deck_upgrade(),
            Self::RestCook { .. }
            | Self::RemoveDeckCardsForGold { .. }
            | Self::RemoveDeckCardForReward { .. }
            | Self::RemoveDeckCardAtShop { .. } => crate::ChoiceSemantics::deck_removal(),
        }
    }
}
