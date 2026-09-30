//! The command layer: the primitive vocabulary every content model is written
//! in.
//!
//! The pinned build composes cards, powers, relics, potions, events, and
//! monster moves out of a finite set of primitive effects. This module is
//! that surface, one enum per command family, one variant per gameplay
//! effect.
//! Anything an effect does that is only presentation — animation triggers,
//! VFX/SFX, camera shake, card previews, the timed waits that pace a beat —
//! has no variant, because none of it is observable through this simulator's
//! boundary.
//!
//! What each family leaves out, and why:
//!
//! - `CreatureCmd`: triggering an animation is presentation. Setting a
//!   creature's maximum HP outright is not reached by covered content.
//! - `CardCmd`: `AutoPlay` needs the play machinery for a card nobody chose;
//!   `Enchant`, `ClearEnchantment`, `Afflict`, `ClearAffliction`,
//!   `ApplyKeyword`, taking a keyword back off, and granting Sly for a single
//!   turn need the enchantment, affliction, and keyword subsystems, which do
//!   not exist; `Discard` on its own is unreached; previewing a card, and
//!   previewing a card joining a pile, are presentation. `Downgrade` is here,
//!   and so is the bulk form [`PowerCmd::DowngradeOwnersCards`] a Dampen
//!   writes, which has to note what it took before it takes it.
//! - `CardPileCmd`: putting a card into a pile as part of the player playing
//!   it is what playing a card does and belongs to the engine's own play path
//!   rather than to content. `ShuffleIfNecessary` runs inside `Draw` rather
//!   than being issued.
//! - `PlayerCmd`: setting energy or gold outright are compositions of the gain
//!   and lose pairs,
//!   `EndTurn` is a player decision rather than an effect, and completing a
//!   quest needs quest cards. `AddPet` is here; taking a pet away is not
//!   reached, because nothing in the pinned build takes a pet away — a pet
//!   that dies stays in the fight and the fight ending drops it.
//! - `PowerCmd`: finding an instance already standing to stack onto, and
//!   `ModifyAmount`, are the two halves of `Apply` — the search, and the
//!   stacking itself — rather than effects a model issues. Plating reaches
//!   `ModifyAmount` directly only on its multiplayer path.
//! - `RelicCmd`: the whole family is here.
//! - `PotionCmd`: drinking a potion is the potion model's own effect, not a
//!   command.
//! - `RewardsCmd`: the room-end reward screen is one the engine drives when a
//!   room finishes rather than content issuing it; generating that screen's
//!   rewards, or a custom set of them, is unreached.
//!
//! - `CardSelectCmd`: the whole screen surface is here. The automated-selector
//!   stack — pushing a selector, using one, resetting it, and suspending it —
//!   is how a selection screen is answered with nobody choosing, which is what
//!   the build's own automatic play leans on, and this simulator's decision
//!   boundary is that mechanism; deciding whether the local player is the one
//!   to pick, reporting a softlock, and logging a choice are multiplayer
//!   routing, telemetry, and logging. Four screens are shaped and fail as
//!   unsupported content because each needs a subsystem that does not exist:
//!   card creation results, transformation pools, enchantments, and bundles.
//!
//! - `OstyCmd`: the whole family is here, which is one effect.
//!
//! - `ForgeCmd`: `Forge` is the whole of the family's gameplay. Raising a
//!   Sovereign Blade's damage, and finding the blades to raise, are its own
//!   two halves rather than effects a model issues, and previewing a blade and
//!   the forge's combat-room flourish are presentation.
//!
//! - `OrbCmd`: eight effects, six variants. Channelling an orb named by its
//!   model and channelling one named by its kind are one and the same — an orb
//!   is named by its model either way — and `Evoke` is the shared body behind
//!   `EvokeNext` and `EvokeLast` rather than anything a model issues.

use serde::{Deserialize, Serialize};
use sts2_core::{CardInstanceId, CreatureCombatId, ModelId};

use crate::{CardFingerprint, CardRarity, CardTag, CardType, PileName, RewardFingerprint};

/// One call over the command surface.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Command {
    Creature(CreatureCmd),
    Damage(DamageCmd),
    Card(CardCmd),
    CardPile(CardPileCmd),
    CardSelect(CardSelectCmd),
    Player(PlayerCmd),
    Power(PowerCmd),
    Relic(RelicCmd),
    Potion(PotionCmd),
    Rewards(RewardsCmd),
    Osty(OstyCmd),
    Forge(ForgeCmd),
    Orb(OrbCmd),
    /// What a modifier listener contributes, expressed as a command. Outside
    /// a modifier pass it does nothing, because there is no value to work on.
    Modify(ModifierCmd),
    /// The condition a model body guards a run of commands with.
    When {
        condition: Condition,
        then: Vec<Command>,
        /// The other arm, for a body that has one: a Nemesis turning its
        /// intangibility on or off is a single choice with two outcomes.
        #[serde(default)]
        otherwise: Vec<Command>,
    },
    /// A resolved set of cards a model body names once, so that the commands
    /// that follow all act on the same cards rather than re-rolling the
    /// selection. Inside `then`, [`CardSelector::Chosen`] is that set.
    Select {
        cards: CardSelector,
        then: Vec<Command>,
    },
    /// A draw whose cards the commands that follow read.
    ///
    /// This is what a body names when it draws and then asks what it drew, the
    /// way an Escape Plan pays block only for a skill. Inside `then`,
    /// [`CardSelector::Chosen`] is what came up.
    Drawn {
        count: Amount,
        then: Vec<Command>,
    },
    /// An amount a body settles before it acts, the way a Sacrifice works out
    /// its block first: the amount is worked out once, and the commands after
    /// it read [`Count::Bound`] rather than working it out again against a
    /// board they have already changed.
    WithAmount {
        amount: Amount,
        then: Vec<Command>,
    },
    /// The body runs once, and buys itself another pass for every creature
    /// that pass killed, until a pass kills nothing. An Echoing Slash is the
    /// only thing shaped this way.
    RepeatPerKill {
        then: Vec<Command>,
    },
    /// What a body does when it does the same thing a counted number of
    /// times.
    Repeat {
        times: Amount,
        then: Vec<Command>,
    },
    /// The same repetition where the final pass differs: every pass but the
    /// last runs `then` and the last runs `last`. A Multi-Cast and a Quadcast
    /// behave that way — every evoke but the final one leaves the orb standing
    /// — and a count of nothing runs neither arm.
    RepeatWithLast {
        times: Amount,
        then: Vec<Command>,
        last: Vec<Command>,
    },
    /// A resolved set of creatures a model body names once, so that the
    /// commands after it act on the same creatures rather than resolving the
    /// selector again. Inside `then`, [`CreatureSelector::Bound`] is that set.
    /// It is the set of targets a Lightning orb settles before it both damages
    /// them and hands them back.
    WithCreatures {
        creatures: CreatureSelector,
        then: Vec<Command>,
    },
    /// What a Misery notes before it swings: a copy of every power on the
    /// play's target that counts as a debuff at the amount it is standing at,
    /// taken before the play does anything to that target, so the copies are
    /// unaffected by what the swing goes on to do. Inside `then`,
    /// [`PowerCmd::CopyBoundDebuffs`] is what reads them back.
    WithTargetsDebuffs {
        then: Vec<Command>,
    },
    /// What a body does when it does the same thing once per orb, with the
    /// pass's own orb bound to [`OrbSelector::Bound`] each time round. The set
    /// is resolved once, before the first pass, which is what the build's own
    /// filter-then-snapshot comes to: an orb that arrives or leaves partway
    /// through does not change what the remaining passes act on.
    ForEachOrb {
        orbs: OrbSelector,
        then: Vec<Command>,
    },
    /// What a body does when it does the same thing once per card, with the
    /// pass's own card bound to [`CardSelector::Chosen`] each time round. This
    /// is not `Select`, which names the whole set once: a Second Wind burns a
    /// card and works up block for it, one card at a time.
    ForEachCard {
        cards: CardSelector,
        then: Vec<Command>,
    },
    /// Damage dealt whose result the commands that follow read.
    ///
    /// This is what a body names when it damages something and then acts on
    /// what that blow came to, the way an Omnislice splashes the first hit
    /// onto everyone else. Inside `then`, the counts that read a settled blow
    /// are that first result.
    Deal {
        target: CreatureSelector,
        amount: Amount,
        props: ValueProps,
        then: Vec<Command>,
    },
    /// An attack whose per-hit results the commands that follow read.
    ///
    /// This is what a body names when it swings and then acts on what the
    /// swing came to, the way a Fisticuffs works up block worth the blow. The
    /// build settles one set of results per hit; what `then` is handed is all
    /// of them, in order, flattened into one run, which is what its bodies
    /// read.
    Strike {
        amount: Amount,
        hits: Amount,
        target: CreatureSelector,
        props: ValueProps,
        then: Vec<Command>,
    },
    /// A block gain whose result the commands that follow read.
    ///
    /// The gain settles at what the block came to once the passes had their
    /// say, and a body that works up block and then tells a power how much it
    /// was reads exactly that.
    Shield {
        target: CreatureSelector,
        amount: Amount,
        props: ValueProps,
        then: Vec<Command>,
    },
    /// The block-modifying passes asked for a number rather than for a gain.
    ///
    /// A Glitterstream works out what the block it is about to promise for next
    /// turn is worth *now* — through the same two passes an actual gain runs,
    /// and before it gains any block of its own — and then puts that number on
    /// the power. So a Dexterity standing as the card is played is worth
    /// something next turn as well, and one gained in between is not. What the
    /// passes settled on is [`Count::Bound`] inside `then`.
    WorkOutBlock {
        target: CreatureSelector,
        amount: Amount,
        props: ValueProps,
        then: Vec<Command>,
    },
    /// A body this baseline has not transcribed, named by the model that owns
    /// it. Running it fails as unsupported content.
    ///
    /// A model can be registered for what it is outside its body — what
    /// generation, an offer, a shop price and a deck projection read — before
    /// the body itself is written. This is what stands in the body's place
    /// until it is, so a run that reaches it stops rather than doing something
    /// this build does not do.
    Unsupported(ModelId),
}

/// A size a body reads off a die rather than out of its own fields.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RolledAmount {
    Fixed(i32),
    /// One roll on the run's `Niche` stream, inclusive of `min` and
    /// exclusive of `max`.
    OnNiche {
        min: i32,
        max: i32,
    },
}

/// Where a creature an add brings in stands.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum SpawnSlot {
    /// An add that names no slot at all.
    #[default]
    Unslotted,
    /// A slot the caller names outright.
    Named(String),
    /// The last of the encounter's slots nobody is standing in. Every slot
    /// taken means the add is not made at all.
    LastFree,
    /// Where the creature the body belongs to is standing, which is what a
    /// respawn takes over.
    SameAsOwner,
    /// The first of the encounter's slots nobody is standing in.
    FirstFree,
}

/// What a body does to a creature: bring one in, move it, heal or hurt it,
/// or change what it is standing at.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CreatureCmd {
    /// Brings a monster into the combat on the enemy side, in the encounter
    /// slot the caller names. A creature that arrives in a slot re-sorts the
    /// enemy side into slot order.
    Add {
        monster: ModelId,
        /// Which of the encounter's slots the arrival stands in.
        #[serde(default)]
        slot: SpawnSlot,
        /// The move state the arriving monster opens on, for a caller that
        /// brings it in already set a particular way — a Wriggler brought in
        /// stunned is the one this baseline reaches. `None` opens on the
        /// monster's own first move state.
        #[serde(default)]
        initial_state: Option<String>,
        /// What the body goes on to do with the creature the add brought in.
        /// These run with the caller as owner and the arrival as target, so a
        /// power worked here is worked by the caller onto what it just made.
        /// An add that put nothing down runs none of them.
        #[serde(default)]
        then: Vec<Command>,
    },
    /// The same, over a monster rolled out of a list on the monster-AI
    /// stream. The caller's own last roll is taken out of the list first — a
    /// fabricator never rolls the same monster twice running — and what this
    /// roll settles on becomes its new last.
    AddRolled {
        options: Vec<ModelId>,
        #[serde(default)]
        slot: SpawnSlot,
        #[serde(default)]
        then: Vec<Command>,
    },
    /// The creature's maximum becomes this and its current HP goes with it,
    /// which is how a hatchling comes out of its egg at a size of its own.
    SetMaxAndCurrentHp {
        target: CreatureSelector,
        amount: RolledAmount,
    },
    /// Deals damage to the target.
    Damage {
        target: CreatureSelector,
        amount: Amount,
        props: ValueProps,
    },
    /// The target gains block.
    GainBlock {
        target: CreatureSelector,
        amount: Amount,
        props: ValueProps,
    },
    /// What a decimillipede segment does as it joins the room: the HP the
    /// creature rolled is rounded up to an even number and then raised by two
    /// until no teammate carries the same, wrapping back to the monster's
    /// minimum when it passes its maximum, and then set as both maximum and
    /// current HP.
    SetEvenUniqueMaxHp,
    /// Raises the maximum and heals by as much. Outside a combat there is no
    /// creature list, and the player's own HP is the run's, so it lands
    /// there.
    GainMaxHp {
        target: CreatureSelector,
        amount: Amount,
    },
    /// Takes the maximum down, and the current HP with it where it has to.
    ///
    /// The current HP left standing over the new maximum comes off as a blow
    /// before the maximum moves — unblockable, scaled by nothing, dealt by
    /// nobody and behind no card — so a maximum-HP loss that costs real HP is
    /// in the turn's damage record and reaches the listeners that watch for
    /// damage. A creature already at or below its new maximum takes no blow.
    LoseMaxHp {
        target: CreatureSelector,
        amount: Amount,
    },
    /// The target loses block.
    LoseBlock {
        target: CreatureSelector,
        amount: Amount,
    },
    /// Heals the target.
    Heal {
        target: CreatureSelector,
        amount: Amount,
    },
    /// Sets the target's current HP.
    SetCurrentHp {
        target: CreatureSelector,
        amount: Amount,
    },
    /// The bare write the command above is built on. Nothing is announced
    /// and nothing is killed — a body that reaches for this instead, as a
    /// punch construct's arrival does, is setting the number rather than
    /// dealing anything.
    SetCurrentHpRaw {
        target: CreatureSelector,
        amount: Amount,
    },
    /// The monster is forced onto this move state and the intent it shows is
    /// republished, without a roll and without waiting for the creature's
    /// turn.
    SetMoveImmediate {
        target: CreatureSelector,
        state: String,
    },
    /// Puts the creature off balance, or takes it back on. The build has no
    /// command for this: a bowlbug rock's power sets it on the monster
    /// directly and the monster's own move clears it.
    SetOffBalance {
        target: CreatureSelector,
        value: bool,
    },
    /// Changes how the creature's HP reads. The build has no command for this
    /// either: a hardened shell and a waterfall giant set it on the creature
    /// directly.
    SetHpDisplay {
        target: CreatureSelector,
        display: crate::HpDisplay,
    },
    /// A number the creature writes down for itself and nothing outside it
    /// reads: the damage a pressure gun has worked up to, or the pressure a
    /// giant is holding as it goes down.
    SetLocal {
        target: CreatureSelector,
        name: String,
        amount: Amount,
    },
    /// The creature goes down outright, whatever it had left, and the death
    /// is settled right there.
    Kill { target: CreatureSelector },
    /// The same kill, as Doom deals it, and then one pass of the
    /// died-to-Doom listeners over the whole set. The build takes the set down
    /// a creature at a time so that a reviving power sees the rest of it go,
    /// and announces the deaths together afterwards.
    DoomKill { target: CreatureSelector },
    /// The creature stands through its next turn and picks its moves back up
    /// afterwards.
    ///
    /// `next_move` is the move state the stunned turn follows up into.
    Stun {
        target: CreatureSelector,
        next_move: StunFollowUp,
        /// What the stunned turn performs. Almost every caller hands over a
        /// body that does nothing, which is a creature standing there for the
        /// turn; a Slumber hands over the waking that takes the beetle's shell
        /// off.
        #[serde(default)]
        stun_move: Vec<Command>,
    },
    /// The creature leaves the fight on its own feet.
    /// Nothing about it is a death — it keeps the HP it had, no death listener
    /// runs, and no kill is counted — but it is out of the combat and the
    /// encounter remembers that it got away, because a fight pays out less
    /// gold for each of its monsters that did.
    ///
    /// A creature already down escapes nothing, and neither does one in a
    /// fight that is over or ending.
    Escape { target: CreatureSelector },
}

/// Which move state a stunned creature picks back up once its stunned turn
/// has been performed.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum StunFollowUp {
    /// The caller named none, so the creature takes up the move it last
    /// performed, which it goes on to perform again.
    TheLastMovePerformed,
    /// A state the caller named outright.
    Named(String),
    /// Not the last move but the one the creature would have gone on to,
    /// which is what a Flutter names as the blow that empties it drops its
    /// owner. Only move states are recorded, and a move state's follow-up is
    /// fixed, with no roll behind it.
    AfterTheLastMovePerformed,
}

/// A swing: damage dealt in a counted number of hits.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DamageCmd {
    /// An attack, carrying the gameplay settings it is built with: the damage
    /// each hit deals, how many hits it makes, who it is aimed at, and whether
    /// it skips the listeners that scale it.
    Attack {
        amount: Amount,
        hits: Amount,
        target: CreatureSelector,
        props: ValueProps,
    },
    /// The same swing dealt by the owner's pet rather than by the owner: the
    /// pet is the attacker, so the pet's own stats shape the blow and the pet
    /// is the dealer every listener answers about. The card is still the
    /// source.
    ///
    /// It is a variant of its own because the build treats a pet swing as a
    /// separate thing to build rather than as a flag on an ordinary one, and
    /// because a swing with no pet to swing is nothing at all — every card
    /// that issues it guards on the pet standing first.
    AttackFromPet {
        amount: Amount,
        hits: Amount,
        target: CreatureSelector,
        props: ValueProps,
    },
}

/// What a body does to a card itself: play it, change what it is, or put it
/// out of the fight.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CardCmd {
    /// Exhausts the named cards.
    Exhaust { cards: CardSelector },
    /// A card played without the player choosing it. It is put into the play
    /// pile first and then played, which is what the build's own automatic
    /// play does.
    AutoPlay {
        cards: CardSelector,
        /// Aims the play at the target the body was pointed at rather than at
        /// one rolled for the card. Only a card that aims at one enemy reads
        /// it.
        #[serde(default)]
        at_the_bodys_target: bool,
    },
    /// What a Whispering Earring does once the automatic pre-play phase has
    /// opened: its owner's whole hand played for them, up to this many cards.
    ///
    /// Each pass takes the first card of the hand that can be played, pays for
    /// it out of both pools, and plays it without the player choosing
    /// it — aimed at the leftmost enemy where the card aims at one, at a living
    /// ally rolled on the combat-targets stream where it aims at one of those,
    /// at its owner where it aims at a player, and at nobody otherwise. The
    /// loop stops early on a fight that is over or ending, on the player having
    /// said they are done, on the turn having moved, and on the hand holding
    /// nothing it can play.
    ///
    /// A selection screen a play of it opens is answered by taking the first
    /// cards offered, up to what the screen asks for. It is the one place in
    /// the pinned build where gameplay answers a selection screen for itself,
    /// and all that answer does is take from the front of what is offered.
    PlayTheHandAutomatically { limit: Amount },
    /// A copy of each card selected is made and played in its place. The copy
    /// is a dupe — it carries no Exhaust
    /// and goes nowhere once the play is over — and the card it was made from
    /// is left exactly where it stands.
    AutoPlayDupe { cards: CardSelector },
    /// The build has no command for this: a card carries its own replay
    /// count, which a Soldier's Stew raises, and a card's play effect runs one
    /// more time than that count says.
    ModifyReplayCount { cards: CardSelector, amount: Amount },
    /// Upgrades the named cards. A card with no upgraded form is passed
    /// over.
    Upgrade { cards: CardSelector },
    /// The upgrade comes back off, which a mirror does to a couple of deck
    /// cards.
    Downgrade { cards: CardSelector },
    /// The cards are put down, the draw happens, and
    /// only then do the discard listeners run — which is why it is one command
    /// and not a discard followed by a draw. An empty set of cards to discard
    /// returns before anything at all, the draw included.
    DiscardAndDraw { cards: CardSelector, count: Amount },
    /// A cost of nothing up to three, rolled on the `CombatEnergyCosts`
    /// stream: what a snecko oil does to every card of the hand that costs
    /// something and does not cost X, and what a Confused does to each card as
    /// it is drawn. One roll per card, in the order the selector names them.
    ///
    /// A card whose canonical cost is below nothing is passed over without a
    /// roll, which is the guard the drawn-card path opens with; the snecko
    /// oil's own filter has already ruled those out.
    RandomizeCost {
        cards: CardSelector,
        /// The cost the roll settles on stands for the rest of the fight
        /// rather than only for this turn or until the card is played.
        #[serde(default)]
        for_the_combat: bool,
    },
    /// A number the card itself carries, moved.
    ///
    /// The build changes these on the card directly — a Guilty counting the
    /// fights it has seen — and they are saved with it. They belong to the
    /// card rather than to the combat, so one written on a deck card outlives
    /// the fight, and it is what a recording projects alongside the card's
    /// model and upgrade level.
    ModifyProperty {
        cards: CardSelector,
        name: String,
        amount: Amount,
        /// The same is written onto the run-deck card the named card is the
        /// combat's copy of, so the growth outlives the fight.
        #[serde(default)]
        and_the_deck_version: bool,
    },
    /// A number a card writes down for itself and nothing outside it reads,
    /// the way a Regret notes the hand it is being held in so that its
    /// end-of-turn effect can read it back. It belongs to the combat's copy of
    /// the card and nothing projects it.
    SetLocal {
        cards: CardSelector,
        name: String,
        amount: Amount,
    },
    /// A transform over the run deck: the first deck card the table names
    /// becomes what it maps to, keeping the upgrade the original carried.
    /// A deck transform does not replace in place — the original leaves the
    /// deck and the replacement joins the end of it, on the floor the run is
    /// standing on.
    TransformInDeck { upgrades: Vec<(ModelId, ModelId)> },
    /// The affliction goes onto every named card that can take one — a card
    /// already carrying an affliction is passed over, since none of the
    /// covered ones stack.
    Afflict {
        cards: CardSelector,
        affliction: ModelId,
    },
    /// Takes the affliction back off every named card carrying it.
    ClearAffliction {
        cards: CardSelector,
        affliction: ModelId,
    },
    /// A cost put over the card's own for a while, which is what making a
    /// card free for a turn or until it is played comes to.
    SetTemporaryCost {
        cards: CardSelector,
        lifetime: crate::TemporaryCost,
    },
    /// Over the named cards: each goes to the discard pile,
    /// the discard is written into the combat history, and a card that was Sly
    /// as it was queued plays itself for free once the batch is down.
    Discard { cards: CardSelector },
    /// Raises the damage the named cards are written for: a Rampage grows its
    /// own damage, and a Maul grows every Maul in the fight.
    GrowDamage { cards: CardSelector, amount: Amount },
    /// A cost put over the named cards' own for the rest of the turn, on top
    /// of whatever else stands there.
    AddCostThisTurn { cards: CardSelector, amount: Amount },
    /// The same, for the rest of the fight.
    AddCostThisCombat { cards: CardSelector, amount: Amount },
    /// What a Thrash does: the damage the card the enclosing selection picked
    /// would deal, worked through the damage passes, is added to the damage
    /// the card that issued the command is written for. It goes straight onto
    /// that card's own damage, which belongs to the combat's copy and is not
    /// projected.
    AddChosenDamageToSource,
    /// Each card becomes a fresh card of `into`, in place.
    /// `upgraded` is what a body that upgrades the new card before transforming
    /// into it asks for, the way an upgraded Primal Force makes upgraded rocks.
    Transform {
        cards: CardSelector,
        into: ModelId,
        upgraded: bool,
    },
    /// The same transform in place, with the replacement rolled out of the
    /// card's own transformation options rather than named. An Entropy turns
    /// cards of the hand this way, one at a time, on the combat
    /// card-selection stream.
    TransformToRolled {
        cards: CardSelector,
        stream: crate::TransformStream,
    },
    /// The keyword is put on the copy of the card in front of the body rather
    /// than on the card everywhere, and stands for as long as that copy
    /// does.
    ApplyKeyword {
        cards: CardSelector,
        keyword: crate::CardKeyword,
    },
    /// The same keyword for this turn alone — Sly and Retain are the two that
    /// can be given this way. The end-of-turn cleanup clears it off every card
    /// its owner has, in every pile.
    ApplySingleTurnKeyword {
        cards: CardSelector,
        keyword: crate::CardKeyword,
    },
    /// Puts an enchantment on the named cards at the amount asked for.
    ///
    /// The build fails outright where the enchantment cannot go on the card,
    /// so a body that reaches this with such a card is content being wrong.
    /// Where
    /// the card already carries an enchantment of the same model the amounts
    /// add instead; nothing in the pinned build is stackable, so that path is
    /// only ever reached by a second application of the same model to a card
    /// the first one is still on.
    Enchant {
        cards: CardSelector,
        enchantment: ModelId,
        amount: Amount,
    },
    /// Takes the enchantment back off the named cards.
    ClearEnchantment { cards: CardSelector },
    /// Moves the amount of the enchantment the named cards carry, which is
    /// what Goopy does to itself and to its deck version as its card is
    /// played.
    ModifyEnchantmentAmount {
        cards: CardSelector,
        amount: Amount,
        /// Where the named card is the combat's copy of a run-deck card, the
        /// same is written onto that deck card too, so the growth outlives the
        /// fight.
        #[serde(default)]
        and_the_deck_version: bool,
    },
    /// Sets the status of the enchantment the named cards carry.
    SetEnchantmentStatus {
        cards: CardSelector,
        status: crate::EnchantmentStatus,
    },
    /// A number an enchantment writes down for itself and nothing outside it
    /// reads, the way Momentum keeps the damage it has worked up and Glam
    /// marks itself spent. It belongs to the fight and nothing projects it.
    ModifyEnchantmentLocal {
        cards: CardSelector,
        name: String,
        amount: Amount,
    },
    /// An amount added on top of the named cards' own cost that stands until
    /// each is next played.
    AddCostUntilPlayed { cards: CardSelector, amount: Amount },
    /// A rolled transform over the run deck.
    ///
    /// The build works out a replacement for every named card first, in the
    /// order they were named — which is the order the rolls come in — takes
    /// every original out of the deck, and only then adds the replacements,
    /// sorted by where their originals stood. A deck add appends, so the
    /// replacements end up at the end of the deck in that order, each one
    /// joining the way any other card joins the deck.
    TransformDeckCards {
        cards: CardSelector,
        replacement: CardTransformReplacement,
        /// Each card is transformed on its own rather than the whole set at
        /// once, which is what an astrolabe and a new leaf do. Each
        /// replacement joins the deck before the next card is worked out, so
        /// the deck takes them in the order the selection named rather than in
        /// the order their originals stood.
        #[serde(default)]
        one_at_a_time: bool,
    },
}

/// What a card being transformed turns into, which is the two shapes the
/// build works with: a replacement named outright, or one rolled out of the
/// card's own transformation options.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CardTransformReplacement {
    /// Nothing is named, so the replacement is rolled out of the card's
    /// default transformation options on the stream the caller named.
    Rolled {
        stream: TransformStream,
        /// What was rolled is upgraded before the transform, which is what an
        /// astrolabe does and a new leaf does not.
        #[serde(default)]
        upgraded: bool,
    },
    /// A named card, which consumes no roll at all.
    Named {
        model: ModelId,
        /// The replacement is upgraded where the card it stands in for was and
        /// the replacement can be upgraded at all.
        #[serde(default)]
        keeps_upgrade: bool,
        /// The replacement carries a copy of whatever enchantment the original
        /// wore, at the amount it wore it, where the enchantment can go on it.
        /// It is the body's own doing rather than the transform's, so only the
        /// body that asks for it gets it.
        #[serde(default)]
        keeps_enchantment: bool,
    },
}

/// Which stream a transform rolls its replacement on. Each place a transform
/// happens is pinned to one of these and they are not interchangeable.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransformStream {
    /// The run's niche stream.
    Niche,
    /// The player's own transformation stream.
    Transformations,
    /// The event's own stream, carried on from wherever its last page left it.
    Event,
    /// The combat card-selection stream, which is what a body transforming a
    /// card of the hand mid-fight rolls on.
    CombatCardSelection,
}

/// What a body does to a whole pile of cards: add to one, move cards between
/// them, draw from one, or shuffle one.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CardPileCmd {
    /// What Reflections: Shatter does: every card of the run deck cloned into
    /// the run deck, over the deck as it stood when the cloning opened.
    CloneTheDeckIntoItself,
    /// Cards that exist for this combat only and never reach the deck.
    AddGeneratedToCombat {
        /// An enchantment put on each card that was made, which is what a
        /// Blade of Ink does with the Shivs it made.
        #[serde(default)]
        enchantment: Option<Box<(ModelId, i32)>>,
        card: CardOrigin,
        count: Amount,
        pile: PileName,
        /// What was made is upgraded before it is handed over.
        #[serde(default)]
        upgraded: bool,
        /// Whether the body puts the Ethereal keyword on the copy it just
        /// made, the way a music box does to the card it is holding.
        #[serde(default)]
        ethereal: bool,
        /// The same for the Retain keyword. The build puts it on before the
        /// screen that offers the card, and the only card the screen hands
        /// anywhere is the one that is added, so the keyword rides on the
        /// add.
        #[serde(default)]
        retain: bool,
        /// A cost the body puts over the card's own before it hands it over,
        /// the way a colorless potion makes what it offers free for the turn.
        /// The build prepares the card and then adds it; this engine makes the
        /// card as it adds it, so what it was prepared with is said here.
        #[serde(default)]
        temporary_cost: Option<crate::TemporaryCost>,
        /// Where in the pile the card lands, which only the draw pile
        /// distinguishes. A random position is rolled on the combat
        /// card-selection stream, one roll per card.
        #[serde(default = "bottom_of_pile")]
        position: DrawPilePosition,
        /// What the card's own damage is grown by as it is made, which is how
        /// a monster brings a card it keeps growing up to where the ones
        /// already in the fight stand.
        #[serde(default = "no_growth")]
        grown_by: Amount,
    },
    /// A card added to the run deck, which is also what the curse draw below
    /// comes to.
    AddToDeck { card: CardOrigin },
    /// The curse draw two relics do for themselves: the curse pool's cards
    /// that modifiers are allowed to generate, ordered by model ID, drawn one
    /// at a time on the run's niche stream without replacement, each added to
    /// the deck as it is drawn.
    AddCursesToDeck { pool: ModelId, count: usize },
    /// Cards made the way a reward offer makes them, added straight to the
    /// deck instead of being offered: an arcane scroll creates its one card
    /// and puts it in the deck.
    ///
    /// The build makes as many cards as the creation asks for and adds the
    /// first of them; with one asked for those are the same set, and this adds
    /// every card that was made, in the order it made them.
    AddCreatedCardsToDeck { creation: CardCreation },
    /// What a Bing Bong does as a card joins the deck: a copy of that card is
    /// added to the bottom of the deck and marked as this listener's doing, so
    /// that the copy does not set the listener off again.
    CloneCardJoiningTheDeck,
    /// Cards taken off the draw pile and played without the player choosing
    /// them, each one exhausting after the play where the caller asks for
    /// it.
    AutoPlayFromDrawPile {
        count: Amount,
        position: DrawPilePosition,
        force_exhaust: bool,
    },
    /// Draws this many cards.
    Draw { count: Amount },
    /// The shuffle a draw makes for itself when the draw pile has run out,
    /// issued on its own by a body that has to look at the draw pile before it
    /// draws from it.
    ShuffleIfNecessary,
    /// The discard pile and the draw pile become one pile, put in canonical
    /// order and shuffled on the shuffle stream. It is not
    /// `ShuffleIfNecessary`, which only acts on an empty draw pile and which
    /// the draw runs itself.
    Shuffle,
    /// What a Pillage does: one card off the pile, and another for as long as
    /// the one just drawn is what the filter names and the hand is not full.
    /// The first card always comes off, whatever it turns out to be.
    DrawWhile { filter: CardFilter },
    /// Cards rolled out of a pool for this combat only, going into a pile.
    ///
    /// `distinct` is which of the two rolls the body asked for. The distinct
    /// form shuffles the whole filtered list on the combat card-generation
    /// stream and takes the first few, so it never hands the same card over
    /// twice; the other rolls each card on its own and can.
    AddGeneratedFromPool {
        pool: CardPoolRef,
        count: Amount,
        distinct: bool,
        #[serde(default = "any_card")]
        filter: CardFilter,
        pile: PileName,
        /// Where in the pile the cards land, which only the draw pile
        /// distinguishes.
        #[serde(default = "bottom_of_pile")]
        position: DrawPilePosition,
        /// The roll passes over the model of the card that issued it, which
        /// is what a card rolling out of a pool it is itself in has to say to
        /// keep from making copies of itself.
        #[serde(default)]
        excludes_source: bool,
        /// What was made is upgraded before it is handed over.
        #[serde(default)]
        upgraded: bool,
        #[serde(default)]
        temporary_cost: Option<crate::TemporaryCost>,
        /// The Ethereal keyword put on each card as it is made, which is what
        /// a Call of the Void marks its roll with.
        #[serde(default)]
        ethereal: bool,
        /// The distinct roll made once per card rather than once for the
        /// lot: asking for a single distinct card, over and over, shuffles the
        /// whole filtered pool again each time, so the rolls are independent
        /// of one another and the same card can come up twice.
        /// Only a Call of the Void is written this way, and the cards are still
        /// all rolled before any of them is added.
        #[serde(default)]
        rolled_one_at_a_time: bool,
    },
    /// Cards moved into a pile, at the end of it the caller names.
    MoveToPile {
        cards: CardSelector,
        pile: PileName,
        position: DrawPilePosition,
    },
    /// Takes the named cards out of the run deck.
    RemoveFromDeck { cards: CardSelector },
    /// Takes the named cards out of the combat altogether.
    RemoveFromCombat { cards: CardSelector },
}

/// Which end of the draw pile a card is taken from, or lands at.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DrawPilePosition {
    Top,
    Bottom,
    /// A place anywhere in the pile, rolled on the shuffle stream over every
    /// gap the pile has, both ends included.
    Random,
}

/// How many cards a selection screen asks for and how it may be answered.
///
/// A screen also carries a prompt and a handful of purely visual settings —
/// whether the cards are laid out for comparison, whether previews are shown
/// unpowered, whether unplayable cards are offered as if they could be played,
/// whether the confirm button glows — none of which cross this boundary. What
/// is left is the count range, whether the screen may be backed out of, and
/// whether it comes up at all when there is nothing to choose between.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SelectorPrefs {
    /// The fewest cards the screen will settle for.
    pub minimum: usize,
    /// The most it will take.
    pub maximum: usize,
    /// Whether the screen may be backed out of without choosing.
    pub cancelable: bool,
    /// Whether the player has to confirm what they picked. A screen built from
    /// a range takes it on where the two ends differ, and a caller that wants
    /// confirmation even for an exact count asks for it. When it is clear and
    /// the candidates are no more than the minimum, the build takes all of
    /// them without asking.
    pub require_manual_confirmation: bool,
}

impl SelectorPrefs {
    /// Exactly this many, and no screen when there are no more than that to
    /// pick from.
    #[must_use]
    pub const fn exactly(count: usize) -> Self {
        Self {
            minimum: count,
            maximum: count,
            cancelable: false,
            require_manual_confirmation: false,
        }
    }

    /// A range with no declared ceiling, for a screen whose only cap is the
    /// one its body works out where the screen opens.
    #[must_use]
    pub const fn unbounded_range() -> Self {
        Self::between(0, usize::MAX)
    }

    /// A range, which needs confirmation wherever its two ends differ.
    #[must_use]
    pub const fn between(minimum: usize, maximum: usize) -> Self {
        Self {
            minimum,
            maximum,
            cancelable: false,
            require_manual_confirmation: minimum != maximum,
        }
    }

    #[must_use]
    pub const fn cancelable(mut self) -> Self {
        self.cancelable = true;
        self
    }

    #[must_use]
    pub const fn confirmed(mut self) -> Self {
        self.require_manual_confirmation = true;
        self
    }
}

/// The card-selection screen surface.
///
/// Every variant suspends the queue on a first-class decision and then runs
/// `then` with what was picked bound to [`CardSelector::Chosen`], which is
/// where the build picks the body back up once the screen has been answered. A
/// screen with nothing to offer hands back nothing and runs `then` with an
/// empty binding rather than coming up empty: every one of these gives up
/// before the screen when there is nothing to pick from.
///
/// What has no variant: the automated-selector stack — pushing a selector,
/// using one, resetting it, and suspending it — is how a screen is answered
/// with nobody choosing, which is what the build's own automatic play leans
/// on, and this simulator's decision boundary is that mechanism; deciding
/// whether the local player is the one to pick, reporting a softlock, and
/// logging a choice are multiplayer routing, telemetry, and logging.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CardSelectCmd {
    /// A screen over the hand.
    FromHand {
        prefs: SelectorPrefs,
        /// The cap a body works out where the screen opens rather than
        /// declaring, which stands over `prefs.maximum`.
        #[serde(default)]
        maximum: Option<Amount>,
        /// A count worked out where the screen opens rather than declared,
        /// which sets the minimum and the maximum alike.
        #[serde(default)]
        exact_count: Option<Amount>,
        filter: CardFilter,
        then: Vec<Command>,
    },
    /// The same screen over the hand with a gold glow on the cards worth
    /// discarding, which is presentation only.
    FromHandForDiscard {
        prefs: SelectorPrefs,
        #[serde(default)]
        maximum: Option<Amount>,
        #[serde(default)]
        exact_count: Option<Amount>,
        filter: CardFilter,
        then: Vec<Command>,
    },
    /// Exactly one upgradable card of the hand, taken without asking when
    /// there is only one.
    FromHandForUpgrade { then: Vec<Command> },
    /// A screen over one of the combat piles. The pile must be a combat pile;
    /// a screen over the draw pile is presented in a stable order of the
    /// build's own rather than in pile order, so it leaks nothing about the
    /// draw.
    FromCombatPile {
        pile: PileName,
        prefs: SelectorPrefs,
        /// The cap a body works out before it opens the screen, where it does:
        /// Neow's Fury asks for no more cards than the hand has room for. It
        /// stands over `prefs.maximum`, and a cap of nothing opens no screen
        /// at all.
        #[serde(default)]
        maximum: Option<Amount>,
        /// A count worked out where the screen opens rather than declared: a
        /// Stratagem asks for as many cards as the power has stacks. It sets
        /// the minimum and the maximum alike.
        #[serde(default)]
        exact_count: Option<Amount>,
        /// The screen offers only the cards the enclosing selection already
        /// named, the way a Seeker Strike narrows its own screen: the pile is
        /// still what the cards come from, and the screen still presents it in
        /// the pile's own screen order.
        #[serde(default)]
        restricted_to_chosen: bool,
        filter: CardFilter,
        then: Vec<Command>,
    },
    /// A screen over the upgradable cards of the run deck.
    FromDeckForUpgrade {
        prefs: SelectorPrefs,
        then: Vec<Command>,
    },
    /// A screen over the removable cards of the run deck, with a filter on
    /// top of the removability the screen applies itself.
    FromDeckForRemoval {
        prefs: SelectorPrefs,
        filter: CardFilter,
        then: Vec<Command>,
    },
    /// A screen over the run deck with no purpose of its own attached.
    FromDeckGeneric {
        prefs: SelectorPrefs,
        filter: CardFilter,
        then: Vec<Command>,
    },
    /// One of at most three cards that stand in no pile, with an optional
    /// skip.
    ChooseACard {
        cards: Vec<CardFingerprint>,
        can_skip: bool,
        then: Vec<Command>,
    },
    /// A grid over distinct cards rolled for this combat, rather than the
    /// three-card screen, which is what a body laying out five of them
    /// needs.
    SimpleGridOfGeneratedCards {
        pool: CardPoolRef,
        count: usize,
        #[serde(default = "any_card")]
        filter: CardFilter,
        prefs: SelectorPrefs,
        then: Vec<Command>,
    },
    /// The same screen, over cards rolled for the combat rather than named.
    /// The distinct roll keeps what a pool allows to be generated in a combat,
    /// shuffles what is left on the combat card-generation stream, and takes
    /// the first `count`.
    ChooseAGeneratedCard {
        pool: CardPoolRef,
        count: usize,
        /// A narrowing a body puts over the pool before the roll, which is how
        /// an attack potion offers only attacks.
        #[serde(default = "any_card")]
        filter: CardFilter,
        /// What the roll made is upgraded before the screen goes up: a
        /// Quasar's upgrade offers three upgraded cards rather than upgrading
        /// whichever one is taken.
        #[serde(default)]
        upgraded: bool,
        can_skip: bool,
        then: Vec<Command>,
    },
    /// A grid over a list of cards that stand in no pile.
    SimpleGrid {
        cards: Vec<CardFingerprint>,
        prefs: SelectorPrefs,
        then: Vec<Command>,
    },
    /// The same grid, over cards a reward creation has just made.
    ///
    /// What the screen shows is each card as the passes over the creation left
    /// it; what the card started as is kept beside it only so that the relic
    /// which changed it can flash over the card, which is presentation. So
    /// what crosses this boundary is the created cards themselves.
    ///
    /// `creations` is one entry per creation the body makes, concatenated in
    /// the order the body concatenates them: each creation carries a blacklist
    /// of its own, so the same card can stand twice in a grid built from
    /// two.
    SimpleGridForRewards {
        creations: Vec<CardCreation>,
        prefs: SelectorPrefs,
        then: Vec<Command>,
    },
    /// The three-card screen over cards a reward creation has just made.
    ChooseACreatedCard {
        creation: CardCreation,
        can_skip: bool,
        then: Vec<Command>,
    },
    /// The transformable cards of the run deck, shown with what each would
    /// become. Working out what a card would turn into needs the card pools a
    /// transform rolls against, which is not a subsystem this baseline has.
    FromDeckForTransformation {
        prefs: SelectorPrefs,
        then: Vec<Command>,
    },
    /// The cards of the run deck the enchantment can go on, in deck order,
    /// shown with the enchantment over them.
    ///
    /// The screen applies the enchantment to nothing itself: `amount` is what
    /// the preview shows, and the body that opened the screen is what enchants
    /// what it hands back. `filter` is the extra narrowing two callers put
    /// over the offer, on top of the cards the enchantment can go on at all.
    ///
    /// This screen alone takes every card without asking when it has no more
    /// than the minimum, whatever the caller asked about confirmation — the
    /// other screens take that shortcut only where no confirmation was asked
    /// for. Every caller in the pinned build asks for one count rather than a
    /// range, which makes the two the same thing, so the shared shortcut
    /// stands here.
    FromDeckForEnchantment {
        enchantment: ModelId,
        /// What the preview over each card shows the enchantment as being
        /// worth. Nothing acts on it.
        amount: i32,
        #[serde(default = "any_card")]
        filter: CardFilter,
        /// The form where the caller hands over the candidates itself, and
        /// shuffles them first. The screen sorts what it is given back into
        /// deck order, and the shortcut it takes when it has no more cards
        /// than the minimum is over a set of one, so the shuffle is dice being
        /// read and nothing else. Which stream it reads is the caller's, and
        /// it is fidelity.
        #[serde(default)]
        shuffled_on: Option<ShuffleStream>,
        prefs: SelectorPrefs,
        then: Vec<Command>,
    },
    /// A bundle screen over bundles rolled for it: `count` bundles of
    /// `commons` common cards and `uncommons` uncommon ones, drawn off the
    /// pools one at a time on the rewards stream, with every card of every
    /// bundle unique.
    ///
    /// Before each bundle there is a one-in-a-hundred chance of three Claws
    /// instead. It is guarded on the owner being the Defect, and that guard is
    /// tested first, so a run of any other character reads no dice for it and
    /// reaches no such bundle.
    ChooseAGeneratedBundle {
        pools: Vec<CardPoolRef>,
        count: usize,
        commons: usize,
        uncommons: usize,
        then: Vec<Command>,
    },
}

/// What a body does to the player themselves: their energy, gold, stars,
/// potion belt, and pet.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PlayerCmd {
    /// The player gains this much energy.
    GainEnergy(Amount),
    /// The player loses this much energy, which never takes the pool below
    /// nothing.
    LoseEnergy(Amount),
    /// The player is made ready to end their turn from inside a body rather
    /// than by choosing to, and cannot back out of it.
    ///
    /// Nothing waits on it — the handover starts behind the play that asked
    /// for it — so what is left of the play runs first and the turn hands over
    /// once the action has settled. Only a Void Form issues it.
    EndTurn,
    /// The Regent's second resource, worked onto the pool and then announced
    /// to the stars-gained listeners. Nothing caps it, so a gain is floored at
    /// nothing and nothing else. A pass asks first whether the gain should
    /// happen at all; no model of the pinned build answers it.
    GainStars(Amount),
    /// Takes stars off, announcing nothing: the only spend with a pass behind
    /// it is the one a card play makes.
    LoseStars(Amount),
    /// Whichever of the two above closes the gap, and nothing at all where the
    /// pool already stands where it was asked to.
    SetStars(Amount),
    /// The player gains this much gold.
    GainGold(Amount),
    /// The player loses this much gold.
    LoseGold(Amount),
    /// Heals as much as a rest site would.
    MimicRestSiteHeal,
    /// The potion belt grows by this many empty slots.
    GainMaxPotionCount(Amount),
    /// A creature of this monster model joins the player's own side of the
    /// fight as their pet.
    ///
    /// It is the whole of what the build does: the creature is made on the
    /// player's side with no slot, so it skips the unique-HP roll and the
    /// multiplayer scaling an enemy gets and opens at the model's own initial
    /// HP; the player's combat state writes itself on it as its owner; and then
    /// it joins the room the way any creature does. Two relics issue it
    /// directly — the ones that bring a bird and a legion — while the
    /// Necrobinder's own pet arrives through [`OstyCmd::Summon`], which
    /// wraps this in the max-HP and revive rules Osty has and those two do
    /// not.
    AddPet { monster: ModelId },
}

/// The Necrobinder's pet.
///
/// It is one variant, because everything else about the pet is an ordinary
/// creature: it takes damage, carries powers, answers listeners, and stands on
/// its owner's side of the fight.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum OstyCmd {
    /// The pet arrives with this many HP, or, if it is already standing, its
    /// maximum goes up by this much and it heals by as much again.
    ///
    /// The one effect has three shapes, and they are the build's own. A living
    /// pet gains that much maximum HP. A pet that is down — the corpse stays
    /// in the fight, because the power it carries keeps it there — has its
    /// maximum set to the amount outright and is healed to it. And where there
    /// is no pet at all, one is made, given the power that keeps its corpse in
    /// the fight, and then set up the same way. An amount of nothing does none
    /// of it.
    ///
    /// `owner` is the player the pet belongs to, which everywhere but one is
    /// the creature that issued the command. A Sic 'Em names the marking
    /// player instead — the summon comes from a listener standing on the
    /// marked enemy and points at that enemy's pet owner — so the owner is a
    /// selector rather than an assumption.
    Summon {
        amount: Amount,
        #[serde(default = "CreatureSelector::owner")]
        owner: CreatureSelector,
    },
}

/// The Regent's forge.
///
/// It is one gameplay effect. Raising a Sovereign Blade's damage, and finding
/// the blades to raise, are that one effect's own halves rather than anything a
/// model issues, and previewing a blade and the forge's combat-room flourish
/// are presentation: the hammer that lands on the card and the sword that
/// orbits its owner.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ForgeCmd {
    /// A Sovereign Blade is made where the forger holds none outside the
    /// exhaust pile, and then every blade they hold is sharpened by this much.
    ///
    /// Three things about it are the build's and are load-bearing. Nothing
    /// happens at all once the fight is over or ending. The
    /// search that decides whether to make a blade passes over the exhaust
    /// pile, while the sharpening that follows does not — a blade that burned
    /// up still grows, and a fresh one is made beside it. And the amount is a
    /// signed decimal: a Beat Into Shape can hand over less than nothing.
    Forge(Amount),
}

/// The Defect's orbs.
///
/// The build has eight orb effects and six of them are ones a model issues.
/// Channelling an orb named by its model and channelling one named by its kind
/// are one variant: the second is the first with the model looked up first,
/// and an orb is named by its model either way. Evoking is the shared body
/// behind `EvokeNext` and `EvokeLast`, so nothing issues it directly.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum OrbCmd {
    /// Orb slots, held at ten. Nothing happens at all once the fight is over
    /// or ending, and the amount is trimmed to what is left under the ceiling
    /// — a negative trim would take slots away, but nothing in the pinned
    /// build asks for slots while ten are standing.
    AddSlots(Amount),
    /// Slots off the back, orbs and all. An orb dropped this way is simply
    /// taken out of the queue — no evoke runs, and nothing else happens to it
    /// — so it is not marked removed from the state.
    RemoveSlots(Amount),
    /// An orb of this model joins the back of the queue.
    ///
    /// Four things happen in order and each is the build's. A player whose
    /// character has no base slots at all and whose queue has never been given
    /// one is given one first, which is what lets a character other than the
    /// Defect hold an orb. The orb is stamped with its owner. A queue already
    /// full evokes its front to make room. And only if the orb actually
    /// joined the queue — it does not where the capacity is still nothing — is
    /// the channel written into the combat history and announced to the
    /// orb-channelled listeners.
    Channel(OrbSource),
    /// Evokes the orb at the front of the queue. Nothing at all happens where
    /// the queue is empty.
    EvokeNext {
        /// Whether the orb comes out of the queue. Every caller in the pinned
        /// build but one passes true; a Recursion evokes without dequeuing so
        /// that the orb keeps standing.
        #[serde(default = "default_true")]
        dequeue: bool,
    },
    /// Evokes the orb at the back of the queue.
    EvokeLast {
        #[serde(default = "default_true")]
        dequeue: bool,
    },
    /// One orb's passive fired outside its turn trigger, which is what a
    /// Loop, a Darkness, a Tesla Coil and an emotion chip ask for. The target
    /// is handed to the orb itself and only a Lightning orb reads one — giving
    /// one to any other orb fails outright — so a body naming a target names
    /// it for a Lightning orb.
    Passive {
        orb: OrbSelector,
        #[serde(default)]
        target: Option<CreatureSelector>,
    },
    /// One of an orb's own two numbers, written by that orb: the evoke
    /// amount a Dark orb grows by its own passive, and the passive amount a
    /// Glass orb wears down by one. The build has no command for it — it is
    /// the same thing [`CardCmd::SetLocal`] and
    /// [`CardCmd::ModifyEnchantmentLocal`] are for cards and enchantments —
    /// and it belongs to the fight, because an orb is combat-only.
    SetValue {
        orb: OrbSelector,
        field: OrbField,
        amount: Amount,
    },
    /// What an evoke hands back, expressed as a command: the creatures the
    /// evoke reached, which the orb-evoked listeners are then told about. It
    /// is the same shape [`ModifierCmd`] is — a result written as a command —
    /// and outside an evoke body it has nothing to answer for and does
    /// nothing.
    Reached(CreatureSelector),
}

/// `dequeue` defaults to what every caller but two passes.
fn default_true() -> bool {
    true
}

/// Which of an orb's two numbers a body is writing.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrbField {
    Passive,
    Evoke,
}

/// Which orb a `Channel` stands up.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum OrbSource {
    /// The orb model named outright.
    Model(ModelId),
    /// One index over the five orbs that can be rolled, in their fixed order —
    /// lightning, frost, dark, plasma, glass — rolled on the run's
    /// `combat_orbs` stream. Both places this happens read that stream.
    RandomOnCombatOrbs,
}

/// Which orb of a queue a command acts on.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum OrbSelector {
    /// The orb at the front of the queue.
    Front,
    /// The orb a [`Command::ForEachOrb`] is on this time round.
    Bound,
    /// Every orb of the queue, in queue order.
    All,
    /// Every orb of this model in the queue, in queue order: the Dark orbs a
    /// Darkness walks and the Lightning orbs a Tesla Coil does.
    OfModel(ModelId),
}

/// What a body does to a power: put one on, take one off, or move what one
/// is standing at.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PowerCmd {
    /// Puts a power on a creature, stacking onto an instance already standing
    /// where the power allows it. What the power counts as comes from the
    /// power itself rather than from what put it there. `applier` is `None`
    /// where nobody is named, which is how a power that brings another with it
    /// leaves the second one attached by nobody.
    Apply {
        power: ModelId,
        target: CreatureSelector,
        applier: Option<CreatureSelector>,
        amount: Amount,
        /// A creature the caller marks the power with before it goes on. It is
        /// a creature the power is about rather than the one carrying it, and
        /// only an instanced power has one.
        #[serde(default)]
        marks: Option<CreatureSelector>,
    },
    /// The card the swing now opening came from, written onto the listening
    /// power so that the power can answer about it later, the way a Vigor
    /// does. A swing from no card at all binds nothing.
    BindDamageCard,
    /// An entry for the play now opening, written on the listening power's
    /// own tally, the way a Rupture keeps one. `open` is whether the body's
    /// guard passed — no entry is written at all where it did not, and a play
    /// with no entry pays nothing when it finishes.
    OpenPlayTally { open: bool },
    /// Adds to the tally entry the innermost play opened. An entry the guard
    /// refused takes nothing.
    AddToPlayTally(Amount),
    /// The entry the innermost play opened is taken back off the tally. What
    /// it came to is read with [`Count::ListeningPowerPlayTally`] before this
    /// runs.
    ClosePlayTally,
    /// What a Misery does: every debuff the enclosing
    /// [`Command::WithTargetsDebuffs`] noted, copied onto other creatures.
    ///
    /// Each named creature is offered each noted debuff in turn, with the
    /// amount and the applier the original carried and the copying card as the
    /// source. Where an instance is already standing to stack onto, the amount
    /// joins it as an offset; where it is not, a fresh instance stands up. A
    /// temporary power copied this way has its next instance ignored, which is
    /// the build's own guard against lending the stat behind it twice, so only
    /// the wrapper lands.
    CopyBoundDebuffs { targets: CreatureSelector },
    /// Takes one off the power's amount.
    Decrement { power: PowerRef },
    /// Ticks the power's duration down by one.
    TickDownDuration { power: PowerRef },
    /// Takes the power off its carrier.
    Remove { power: PowerRef },
    /// Every power its owner carries but the ones named, removed one at a time
    /// in the order the owner holds them, which is what a hatching egg does to
    /// everything but the power that makes it a minion.
    RemoveOwnersOtherPowers { keep: Vec<ModelId> },
    /// Adjusts a power instance's own stored variable. The build has no
    /// command for this: a power keeps its variables itself and changes them
    /// where it needs to, the way an Inferno grows the damage it deals itself
    /// and a Tender counts the cards played this turn.
    ModifyCounter { power: PowerRef, amount: Amount },
    /// Sets that variable outright, which is how a per-turn count is cleared.
    SetCounter { power: PowerRef, amount: Amount },
    /// The amount written outright rather than worked through the application
    /// passes, which is what a power that grows itself every turn does to
    /// itself.
    SetAmount { power: PowerRef, amount: Amount },
    /// The amount moves by this much, which is what a Frantic Escape does to
    /// the sandpit that made it.
    ModifyAmount { power: PowerRef, amount: Amount },
    /// Sets the yes-or-no the listening power keeps for itself, the way Chains
    /// of Binding notes that the card it is bound to has been played.
    SetLatch(bool),
    /// Attaches a card to a power instance, which is how a thief carries what
    /// it stole until it is killed for it.
    HoldCard {
        power: PowerRef,
        cards: CardSelector,
    },
    /// What a Dampen does as it goes on: every upgraded card of its owner's is
    /// downgraded, and the level each was on is written down on the power so
    /// that it can be put back. That record is the instance's own
    /// `downgraded_cards`.
    DowngradeOwnersCards,
    /// What a Dampen does as it comes off: each card the power wrote down is
    /// upgraded back to the level it was on, in the order it was written
    /// down.
    RestoreDowngradedCards,
}

/// What a body does to a relic: take one, lose one, or move what one is
/// holding on to.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RelicCmd {
    /// The relic joins the player's inventory at the end or at a named index,
    /// leaves both grab bags unless it is stackable, writes down the floor it
    /// was taken on, and then whatever it does on being obtained runs and is
    /// waited on.
    Obtain(ModelId),
    /// A relic pulled off the front of the player's own bag on a rarity rolled
    /// on the rewards stream, obtained where it stands rather than offered.
    ObtainFromTheBag,
    /// The same at a named place in the inventory, which is what a replacement
    /// takes: the index the relic it replaced stood at.
    ObtainAt {
        relic: ModelId,
        index: usize,
    },
    /// The relic leaves the player. The build also notes it in the map
    /// point's history, which nothing projects, and waits on whatever the
    /// relic does on being removed — which no relic of the pinned build
    /// defines, so there is nothing to run.
    Remove(ModelId),
    /// The first relic is removed and the second obtained where it stood.
    Replace {
        original: ModelId,
        replacement: ModelId,
    },
    /// What a Touch of Orobas does once it is taken: the starter relic its
    /// owner is holding is replaced by the refined form the table names, or by
    /// the fallback where the table has no entry for it. It goes by the
    /// starter it noted as it was set up; nothing can change the inventory
    /// between the two, so the table is read again here.
    RefineStarter {
        upgrades: Vec<(ModelId, ModelId)>,
        fallback: ModelId,
    },
    /// The relic stays in the inventory and does nothing. It stops listening,
    /// stops being tradable, and whatever it does on being removed runs and is
    /// waited on, the same way a removal's does.
    Melt(ModelId),
    /// Melts the first wax relic of the inventory that has not melted already,
    /// which is what a toy box takes back every third fight.
    MeltFirstWaxRelic,
    /// Sets the listening relic's saved counter outright, which is how a
    /// per-turn count is cleared.
    SetListeningCounter(Amount),
    /// Adjusts the listening relic's saved counter. The build has no command
    /// for this either: a relic keeps its counter itself, the way a nunchaku
    /// counts the attacks played.
    ModifyListeningCounter(Amount),
    /// The same pair for the second count a relic keeps beside the first, which
    /// is a joss paper holding the burns it has to defer until the turn ends.
    SetListeningSecondCounter(Amount),
    ModifyListeningSecondCounter(Amount),
    /// What a Lord's Parasol does: every item still on the merchant's shelf
    /// bought for nothing, in the inventory's own order — the character cards,
    /// then the colourless ones, then the relics, then the potions — and then
    /// its card removal, which is not cancelable.
    ///
    /// Each purchase goes through the ordinary buying path with the cost
    /// ignored, so no gold moves at all and the item-purchased listeners are
    /// told about a purchase with nothing spent. A slot the shelf refills is
    /// refilled, because that too is what any purchase does.
    BuyTheWholeShelfForNothing,
    /// The card the pass now running came from, written onto the listening
    /// relic, the way an Unsettling Lamp notes what set it off. Passing
    /// nothing clears it, which is what the build's own reset comes to.
    BindAnnouncedCardToTheRelic {
        bind: bool,
    },
    /// What a Pael's Tooth does once it is taken: the cards the enclosing
    /// selection picked are copied onto the listening relic, ordered by model
    /// ID, and leave the deck. The relic keeps them until it hands them back.
    KeepChosenCards,
    /// What a Pael's Tooth does when a fight ends: one of the cards the
    /// listening relic is keeping, rolled on the player's rewards stream,
    /// upgraded where it can be and put back in the deck, and the relic keeps
    /// it no longer.
    HandBackAKeptCard,
    /// Sets the yes-or-no the listening relic keeps for itself, which is the
    /// once-a-fight latch a centennial puzzle and a permafrost keep and the
    /// once-a-turn one an orichalcum keeps.
    SetListeningLatch(bool),
}

/// What a body does to the potion belt: put a potion in it, or take one out.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PotionCmd {
    /// Takes the first free slot, and fails when the belt is full.
    TryToProcure(PotionOrigin),
    /// The potion in the named slot leaves the belt without being drunk.
    Discard { slot: usize },
    /// A potion rolled out of combat into every free slot of the belt, one
    /// roll at a time, stopping at the first procure that fails. It is not a
    /// counted batch: each roll stands on its own, and a refused procure ends
    /// it rather than rolling on.
    FillTheBelt,
    /// A batch of potions rolled out of combat, each one procured after the
    /// last.
    ///
    /// The whole batch is rolled before any of it is procured, and a batch
    /// never rolls the same potion twice — which is why it is one command
    /// rather than a repeated single procure.
    ProcureRolledBatch {
        count: Amount,
        /// The potions go into the slots the belt has just grown rather than
        /// into the first free slot, which is the last `count` slots of the
        /// belt.
        #[serde(default)]
        into_the_new_slots: bool,
    },
}

/// What a procure command puts in the belt.
///
/// The potion is worked out before it reaches the belt; the two ways a
/// content body works one out are naming it and rolling for it, so those are
/// the two shapes here.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PotionOrigin {
    /// A named potion.
    Model(ModelId),
    /// A roll over the player's pools that passes over what may not be
    /// generated inside a fight, on the combat potion-generation stream.
    RandomInCombat,
    /// The same roll with nothing filtered out. The build reads the same
    /// stream for it wherever a card, relic, or potion asks; a reward or a shop
    /// rolls on its own stream instead.
    RandomOutOfCombat,
}

/// What a body puts on the table at the end of a room, or adds to what is
/// already there.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RewardsCmd {
    /// What Neow's Bones does once it is taken: the relics of a named list,
    /// shuffled on the player's own rewards stream, and the first few of them
    /// offered as one reward set the player may not skip.
    OfferShuffledRelics { relics: Vec<ModelId>, count: usize },
    /// A set of rewards a relic or event hands over, which leaves the room
    /// open until they have been taken.
    OfferCustom(Vec<RewardFingerprint>),
    /// A fixed sum of gold added to a reward set being settled, which is
    /// settled as asked without being rolled for.
    AddGold(Amount),
    /// Gold put on the room's own reward set rather than offered as a screen
    /// of its own, which is what a body answering for a fight that has ended
    /// hands over.
    AddGoldToTheRoom(Amount),
    /// A card removal put on the room's own reward set. Taking it opens the
    /// deck's removal screen, and backing out of that screen leaves the reward
    /// on the table: a removal that was not gone through with is a reward that
    /// was not taken.
    AddCardRemovalToTheRoom,
    /// The same, marked as gold a thief took and its own death gave back,
    /// which the reward says so about.
    AddStolenGoldToTheRoom(Amount),
    /// A potion rolled out of combat when the set is settled.
    AddPotion,
    /// A relic pulled off the front of the player's own bag on a rolled
    /// rarity, like any other relic reward.
    AddRelic,
    /// The same pull with the rarity named rather than rolled, which is what a
    /// calling bell offers.
    AddRelicOfRarity(crate::RelicRarity),
    /// Another card offer, built the way the named room builds one.
    AddCardOffer {
        room: crate::ResolvedRoom,
        count: i32,
    },
    /// What the egg relics do, and what a lava lamp does for itself: every
    /// card the offer holds that the filter accepts is replaced by an upgraded
    /// copy of itself.
    UpgradeCardOptions(CardFilter),
    /// Every offered card the enchantment can go on is replaced by a copy
    /// carrying it. What was rolled is kept beside it as what the offer
    /// started as; what the offer shows is the copy.
    EnchantCardOptions {
        enchantment: ModelId,
        amount: Amount,
        /// One of the cards it could go on rather than all of them, rolled on
        /// the run's niche stream. A body that says no enchants every
        /// candidate and reads no dice.
        #[serde(default)]
        one_rolled_on_the_niche_stream: bool,
    },
    /// What a Lasting Candy does to an offer: one more option, rolled out of
    /// the offer's own pool narrowed to a card type and, where it can be, to
    /// what the offer does not already hold.
    AddCardOption { card_type: crate::CardType },
    /// An offer off the character's own pool that no room paid for, so it
    /// rolls the base odds and leaves the run's drifting ones where they
    /// stand.
    AddOwnCardOffer { count: i32 },
    /// An offer added to what the room will pay when the fight is won, rather
    /// than to a reward set being settled now. The Hunt is the one card that
    /// does it.
    AddOwnCardOfferToTheRoom { count: i32 },
    /// An offer whose creation a body built itself: a glass eye's five offers
    /// of one named rarity each.
    AddCreatedCardOffer { creation: CardCreation },
    /// What a Kaleidoscope does once it is taken: one card offer laid out from
    /// this many character colours other than the owner's.
    ///
    /// The colours are rolled per offer: every unlocked character's pool but
    /// the owner's, sorted by model ID and then shuffled on the run's niche
    /// stream, and the first few taken. Each is its own one-card creation with
    /// no pool modifications allowed, so each runs its own passes over what it
    /// made and none of them blacklists the others' cards; the cards are
    /// concatenated into one offer in the order the pools came up.
    AddCardOfferOfOtherColours { colours: usize },
    /// What a Driftwood does to a reward set: every card offer of the set may
    /// be rerolled once.
    AllowRerollingCardOffers,
    /// A relic pulled off the front of the player's own bag and marked wax on
    /// the way, which is what a toy box hands over four of.
    AddWaxRelic,
    /// Whatever this body has added so far is rolled, in the order it was
    /// added, and offered as a set of its own.
    OfferAdded,
}

/// What a card creation draws from, how it rolls, and which of the passes over
/// it are allowed to speak.
///
/// A creation carries a pool list or a card list of its own, a filter, what
/// asked for it, an odds type, a set of things it forbids, and sometimes a
/// stream to roll on. What crosses this boundary is what changes the dice or
/// the result. Every filter the pinned build puts on one is a rarity test, so
/// the filter is the same vocabulary a screen narrows a pile with. Nothing
/// here is an encounter's own offer, which is what makes the rarity roll read
/// the base odds rather than the run's drifting ones and leave them where they
/// stand. The one creation that rolls on a stream of its own is the crystal
/// sphere's, an event, so nothing names one yet.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CardCreation {
    /// The pools the cards are drawn from.
    pub pools: Vec<CardPoolRef>,
    /// A narrowing over those pools, applied before anything is rolled.
    #[serde(default = "any_card")]
    pub filter: CardFilter,
    /// Which rarity odds the roll reads. Uniform rolls no rarity at all and
    /// takes one card out of everything the filter left, which is why a body
    /// that names a single rarity has to use it.
    pub odds: crate::RarityOdds,
    /// Whether the offer is an encounter's own, which is what decides whether
    /// the rarity roll reads and moves the run's drifting odds: an encounter's
    /// offer rolls with them, and everything else rolls the base odds and
    /// leaves them where they stand.
    #[serde(default)]
    pub from_an_encounter: bool,
    /// How many cards are asked for. A reward creation blacklists each card it
    /// makes for the draws after it, so one creation never repeats a card.
    pub count: usize,
    /// What this creation forbids.
    #[serde(default)]
    pub flags: CardCreationFlags,
}

/// What a creation forbids.
///
/// Only the three the covered paths read are named. The others gate listeners
/// and changes this baseline either has nothing shipped to answer or settles
/// where the creation is written: forbidding rarity changes and forbidding
/// listener-driven upgrades have nothing in the pinned build that would do
/// either anyway, forbidding modifier listeners matters only to a body
/// generating a card inside a modifier pass and nothing covered does, and
/// forbidding card changes guards the tutorial's hand-set offers.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct CardCreationFlags {
    /// No roll is made for whether the card comes out upgraded. That roll
    /// draws its float before it tests whether the card can be upgraded at
    /// all, so allowing it costs a draw per card whatever comes of it.
    #[serde(default)]
    pub no_upgrade_roll: bool,
    /// Whether the creation is a card reward. It is what the relics that widen
    /// a pool or upgrade an offer gate on, so a creation that is not a reward
    /// is left alone by them.
    #[serde(default)]
    pub is_card_reward: bool,
    /// Nothing may put another pool behind the ones named here.
    #[serde(default)]
    pub no_card_pool_modifications: bool,
}

/// Which pool a creation draws from.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CardPoolRef {
    /// The walking character's own pool.
    OwnCharacter,
    /// A pool named by model ID, which is how the colorless pool is asked for.
    Named(ModelId),
    /// Every unlocked character's pool but the walking character's, in the
    /// order the characters are listed — ironclad, silent, regent,
    /// necrobinder, defect. Every character is unlocked at this baseline, so
    /// this is always the other four. A body that asks for it and finds only
    /// one pool at all keeps that one rather than coming away with none.
    EveryOtherCharacter,
}

/// What a modifier listener does to the value its pass is working on.
///
/// A modifier listener answers with a number: an additive one with what to
/// add, a multiplicative one with what to multiply by, a cap one with the most
/// the value may be, and a chaining one with the value itself. This is that
/// answer, said as a command so that a listener body stays a run of commands.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ModifierCmd {
    /// What an additive listener contributes.
    Add(Amount),
    /// What a multiplicative listener contributes, as a percentage. A pass
    /// gathers every factor and works the whole product out once, so nothing
    /// is lost between two of them.
    Scale(Amount),
    /// The most the value may come to, and the shape of a chaining listener
    /// that answers with the smaller of the value and its own limit. Caps
    /// combine by taking the least.
    CapAt(Amount),
    /// The value becomes this, which is what a listener that turns an
    /// application away answers with.
    Set(Amount),
}

/// Which creatures a command acts on.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CreatureSelector {
    /// Whoever issued the command: the card's owner, the monster making the
    /// move, or the creature the listening model belongs to.
    Owner,
    /// Whoever the play or the move was aimed at.
    Target,
    /// Every living creature on the side opposite the owner's except whoever
    /// the play was aimed at, over the creatures that can be hit at all.
    OtherOpponents,
    /// Every living creature on the side opposite the owner's that can be
    /// hit.
    AllOpponents,
    /// One of them, rolled on the combat-targets stream. It covers both a card
    /// that aims at a random opponent and a power or relic that rolls one for
    /// itself. An empty field rolls nothing and consumes nothing.
    RandomOpponent,
    /// Every living creature on the owner's own side, the owner included.
    AllAllies,
    /// The same set with the owner taken out of it.
    OtherAllies,
    /// Every creature in the fight, both sides, less the pets, which is what a
    /// foul potion splashes.
    EveryCreature,

    /// The creature the pass is about, which is the one a body answering an
    /// arrival is being told about.
    Announced,
    /// Whoever is dealing the damage the pass is about.
    DamageDealer,
    /// Whoever the damage the pass is about landed on: what a reaper form
    /// dooms. A power pass names its receiver in the same place, so a body
    /// answering for a power that just landed names the creature it landed on
    /// this way — which is what a sleight of flesh cuts.
    DamageTarget,
    /// Whoever worked the listening power onto its owner, or the owner when
    /// nobody did.
    Applier,
    /// The creature the listening power is about, which is the creature it was
    /// marked with. A power that names none resolves to nobody.
    ListeningPowerTarget,
    /// Every living creature of this monster model on the owner's own side,
    /// which is how a guard bot names the fabricators standing with it.
    AlliesOfModel(ModelId),
    /// The first creature on the enemy side carrying the power the reference
    /// names, and nobody when none does. Only a `PowerRef` resolves it.
    EnemyCarryingThePower,
    /// Every creature on the owner's own side carrying at least as much Doom
    /// as it has HP left, in side order. It is the set the first doomed
    /// creature's own listener kills, all of them at once.
    DoomedOnOwnersSide,
    /// The same set read off the other side rather than the owner's own, which
    /// is what a card doing the killing names. The power is named here because
    /// a card has no listening power to read it off.
    DoomedAmongOpponents(ModelId),
    /// The pet of whoever worked the listening power on, which is what a Sic
    /// 'Em summons for: the marking player's own pet rather than the marked
    /// creature's.
    AppliersPet,
    /// The owner's pet: the first of the owner's pets that is an Osty. It
    /// resolves to nobody where the owner has none, and it resolves to the
    /// corpse where the pet is down, because a downed Osty is still the
    /// owner's pet — every body that must not act on one checks for itself
    /// whether the pet is standing.
    Pet,
    /// The creatures the enclosing [`Command::WithCreatures`] resolved.
    Bound,
    /// Whoever the caller named, and one rolled over the hittable opponents on
    /// the combat-targets stream where the caller named nobody. A Lightning
    /// orb's passive and evoke are the same body and differ only in whether a
    /// target was handed in. Where no opponent can be hit it names nobody
    /// whoever the caller named, because the build gives up before it looks at
    /// what it was handed.
    TargetOrRandomOpponent,
    /// What the evoke the pass is announcing reached, less whatever it killed.
    /// It is what a Thunder swings at.
    EvokedOrbTargets,
    /// The opponent standing on the least health, which is what a Dark orb's
    /// evoke picks out. The first of a tie is the one kept, so the earliest
    /// such creature in side order is the one hit.
    WeakestOpponent,
}

impl CreatureSelector {
    /// The default owner of a summon, which is whoever issued it.
    #[must_use]
    pub const fn owner() -> Self {
        Self::Owner
    }
}

/// Which power a `PowerCmd` acts on.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PowerRef {
    /// Who is carrying it.
    pub owner: CreatureSelector,
    pub power: PowerName,
}

impl PowerRef {
    /// The power whose listener issued the command.
    #[must_use]
    pub const fn listening() -> Self {
        Self {
            owner: CreatureSelector::Owner,
            power: PowerName::Listening,
        }
    }

    /// A named power on whoever issued the command.
    #[must_use]
    pub const fn owned(power: ModelId) -> Self {
        Self {
            owner: CreatureSelector::Owner,
            power: PowerName::Model(power),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PowerName {
    Model(ModelId),
    /// The power whose listener issued the command.
    Listening,
    /// The instance the enclosing body's own [`PowerCmd::Apply`] just settled
    /// on.
    ///
    /// Applying a power settles on an instance — a fresh one, or the one a
    /// stacking application joined — and a body that writes on what it just
    /// applied writes on exactly that instance. A bomb setting its own fuse
    /// while another bomb is already ticking is the case that shows it.
    JustApplied,
}

/// Which cards a command acts on.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CardSelector {
    /// The card that issued the command.
    Source,
    /// Every card of a combat pile that passes the filter.
    Pile { pile: PileName, filter: CardFilter },
    /// Every card of the run deck this enchantment can go on, which is what a
    /// body names before it enchants a whole set. `filter` is whatever else
    /// that body narrows by.
    DeckCardsEnchantableBy {
        enchantment: ModelId,
        #[serde(default = "any_card")]
        filter: CardFilter,
    },
    /// Every card anywhere in the combat that passes the filter, whichever
    /// pile it stands in.
    AllCombatCards { filter: CardFilter },
    /// The same, less the card that issued the command, which is what an
    /// Apotheosis leaves out of its own reach.
    OtherCombatCards { filter: CardFilter },
    /// Every card of the combat that is the same model as the one that issued
    /// the command, itself included, which is how a card reaches its own
    /// kind.
    CombatCardsOfSourceModel,
    /// The cards of a combat pile that pass the filter, shuffled on the combat
    /// card-selection stream, and the first few of what is left. The shuffle
    /// is over the pile in the order it happens to stand in.
    ShuffledFromPile {
        pile: PileName,
        #[serde(default = "any_card")]
        filter: CardFilter,
        take: Amount,
    },
    /// The cards of a combat pile that pass the filter, put in canonical
    /// order and then shuffled, and the first few of what is left. The sort in
    /// front of the shuffle is what makes the result independent of the order
    /// the pile happened to be in. Which stream it reads is the caller's, and
    /// it is fidelity.
    StablyShuffledFromPile {
        pile: PileName,
        #[serde(default = "any_card")]
        filter: CardFilter,
        /// The filter the body falls back on when the first leaves nothing,
        /// which is the second set a body names for itself. A shuffle over an
        /// empty set reads no dice, so choosing the set before the shuffle
        /// comes to the same run of them as shuffling both.
        #[serde(default)]
        fallback: Option<CardFilter>,
        take: Amount,
        #[serde(default = "shuffle_stream")]
        stream: ShuffleStream,
    },
    /// Every card of the run deck that passes all of the filters, in deck
    /// order, which is what a relic acting on the whole deck as it is taken
    /// reaches for.
    DeckCards {
        #[serde(default)]
        filters: Vec<CardFilter>,
    },
    /// The first card of the run deck that passes each filter, one card per
    /// filter, in the order the filters are named. A filter nothing matches
    /// contributes nothing at all, which is what a body writing down one card
    /// of each kind comes away with for a kind the deck does not hold.
    FirstInDeckOfEach {
        #[serde(default)]
        filters: Vec<CardFilter>,
    },
    /// The last card of one of these types the owner finished playing on the
    /// turn before this one, and which was not itself a copy. It selects
    /// nothing when the card has since left every combat pile, because a copy
    /// can only be made of a card that still stands in one.
    LastPlayedLastTurn { types: Vec<crate::CardType> },
    /// Every card of the fight of this model, whichever pile it stands in,
    /// which is what a monster growing the cards it made reaches for.
    CombatCardsOfModel(ModelId),
    /// Every combat card of one model that is standing anywhere but the named
    /// pile, which is how a Summon Forth recalls the Sovereign Blades that are
    /// not already in hand. A card standing in no pile at all counts as
    /// outside it.
    CombatCardsOfModelOutside { model: ModelId, pile: PileName },
    /// Every card of the run deck of this model, which is what a body acting
    /// on all of one card reaches for.
    DeckCardsOfModel(ModelId),
    /// The first card of the run deck of this model, which is what a body
    /// looking for one particular card reaches for. A deck holding none of it
    /// selects nothing.
    FirstInDeckOfModel(ModelId),
    /// The same read from the back, which is what a relic that upgrades the
    /// newest starter card of each kind reaches for.
    LastInDeckOfEach {
        #[serde(default)]
        filters: Vec<CardFilter>,
    },
    /// The same over the run deck rather than a combat pile, which is what a
    /// relic that upgrades a couple of cards as it is taken reads.
    StablyShuffledFromDeck {
        #[serde(default = "any_card")]
        filter: CardFilter,
        take: Amount,
        #[serde(default = "shuffle_stream")]
        stream: ShuffleStream,
    },
    /// The first card of a combat pile that passes the preferred filter, or
    /// the first card of the pile when none does.
    FirstInPilePreferring {
        pile: PileName,
        #[serde(default = "any_card")]
        preferred: CardFilter,
    },
    /// One card of the run deck that passes the filter, rolled on the stream
    /// the caller names, which is what a relic that upgrades one card of a
    /// deck reaches for.
    RolledFromDeck {
        #[serde(default = "any_card")]
        filter: CardFilter,
        #[serde(default = "shuffle_stream")]
        stream: ShuffleStream,
    },
    /// Several of them, each rolled off what is left once the one before it
    /// was taken out, so the same card never comes up twice. A set that runs
    /// out early stops the rolling there rather than rolling over nothing.
    RolledFromDeckDistinct {
        #[serde(default = "any_card")]
        filter: CardFilter,
        #[serde(default = "shuffle_stream")]
        stream: ShuffleStream,
        take: Amount,
    },
    /// One card of a combat pile that passes the filter, rolled on the
    /// shuffle stream, which a handful of bodies read instead of the combat
    /// card-selection stream.
    ShuffleRolledFromPile {
        pile: PileName,
        #[serde(default = "any_card")]
        filter: CardFilter,
    },
    /// One card of a combat pile that passes the filter, rolled on the combat
    /// card-selection stream.
    RandomFromPile {
        pile: PileName,
        #[serde(default = "any_card")]
        filter: CardFilter,
        /// The filter the body falls back on when the first leaves nothing.
        /// The set is settled before anything is rolled, so the fallback costs
        /// no dice of its own.
        #[serde(default)]
        fallback: Option<CardFilter>,
    },
    /// One card the player owns out of the named combat piles, preferring the
    /// first rarity group that has any and rolled on the combat card-generation
    /// stream. This is how a thief picks what a deck can least afford to lose.
    StolenFromDeck {
        piles: Vec<PileName>,
        priorities: Vec<Vec<CardRarity>>,
    },
    /// Whatever the enclosing [`Command::Select`] or selection screen picked.
    Chosen,
    /// The same, narrowed by a filter, the way a Scrape acts on only some of
    /// what it just drew.
    ChosenMatching {
        #[serde(default = "any_card")]
        filter: CardFilter,
    },
    /// Every card of the fight that passes the filter and is standing
    /// anywhere but the named pile, which is how a Flak Cannon gathers the
    /// statuses that have not burned up.
    AllCombatCardsOutside {
        #[serde(default = "any_card")]
        filter: CardFilter,
        pile: PileName,
    },
    /// The card the listener pass is about, which is the card a listener for
    /// something happening to a card — a draw, say — is being told about.
    Announced,
    /// The card the play the pass is about was made of.
    PlayedCard,
}

/// Which stream a stable shuffle reads. Each place a shuffle happens is pinned
/// to one of these and they are not interchangeable.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShuffleStream {
    /// The shuffle stream.
    Shuffle,
    /// The combat card-selection stream.
    CombatCardSelection,
    /// The run's niche stream, which is what the two relics that upgrade a
    /// couple of deck cards as they are taken read.
    Niche,
    /// The event's own stream.
    Event,
}

const fn shuffle_stream() -> ShuffleStream {
    ShuffleStream::Shuffle
}

const fn no_growth() -> Amount {
    Amount::Fixed(0)
}

fn bottom_of_pile() -> DrawPilePosition {
    DrawPilePosition::Bottom
}

fn any_card() -> CardFilter {
    CardFilter::Any
}

/// The test a selection screen or a pile command narrows its cards with.
///
/// A screen applies its own filter as well as its caller's: the removal screen
/// hides what cannot be removed before a caller's filter is ever consulted, and
/// the upgrade screen hides what is already at its maximum.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardFilter {
    Any,
    OfType(CardType),
    /// Every card that is not of this type, which is what a Second Wind burns
    /// through.
    NotOfType(CardType),
    /// A card of this type that is not Unplayable.
    PlayableOfType(CardType),
    /// A card that can still take an upgrade.
    Upgradable,
    /// A card that is already upgraded, which is what a mirror looks for
    /// before it takes an upgrade back off.
    Upgraded,
    /// A card of this type that can still take an upgrade, which is what a
    /// relic that upgrades a couple of deck cards narrows by.
    UpgradableOfType(CardType),
    /// A basic card carrying this tag: the pair a ghost seed tests, which
    /// names the starter strikes and defends and nothing else that shares
    /// their tag.
    BasicOfTag(CardTag),
    /// A basic card carrying either the Strike tag or the Defend tag, which is
    /// what a Pandora's Box works on.
    BasicStrikeOrDefend,
    /// The card's rarity is this one.
    OfRarity(CardRarity),
    /// The rarity and type a potion trades for in The Future of Potions.
    OfRarityAndType(CardRarity, CardType),
    /// The card is an attack or a power, which is the two-type test a Dual
    /// Wield's screen filter writes out.
    AttackOrPower,
    /// The card carries no Unplayable keyword, which is what a body that plays
    /// a card nobody chose asks of what it is about to play.
    Playable,
    /// A Hidden Gem's preferred list: a playable attack, skill, or power that
    /// no enchantment has already given a replay to.
    CanTakeAReplay,
    /// A card that is not Unplayable and is neither a curse nor a quest card:
    /// the wider set a body that prefers an attack, a skill, or a power falls
    /// back on, which a playable status is in.
    ///
    /// The build additionally drops a card whose enchantment already grants it
    /// a replay, from this set and from the narrow one alike; that test goes
    /// through the enchantment's own say over the card's replay count, which a
    /// filter here cannot ask for.
    PlayableAndNotACurse,
    /// The card costs nothing and its cost is not X, which is what a Jackpot
    /// rolls against.
    Free,
    /// A card of this type that is not already Sly this turn, which is what a
    /// Hand Trick narrows its screen by.
    NotSlyOfType(CardType),
    /// A card carrying this tag.
    Tagged(CardTag),
    /// A card that can be removed from the deck at all.
    Removable,
    /// A card that is not already keeping itself this turn, which is what a
    /// Well Laid Plans offers.
    NotRetainingThisTurn,
    /// A card of the hand that is not already ethereal in its own right, which
    /// is what a sculpting strike offers. A card made ethereal by something
    /// else still counts as not ethereal here.
    NotEthereal,
    /// A card of the hand that is not already kept, which is what a snap
    /// offers.
    NotRetained,
    /// The card carries no Innate keyword, which is what a body looking for a
    /// card of the draw pile worth burning prefers on the opening turn.
    NotInnate,
    /// The card has an energy cost that can be written over: a cost that is
    /// not below nothing before anything stands on it, and not X. That is what
    /// a snecko oil asks before it rolls a new one.
    HasWritableEnergyCost,
    /// A card that costs energy or stars, either with everything standing
    /// over its cost or with only what was put on the card itself, which is
    /// what a Touch of Insanity asks. A card made free for the turn costs
    /// nothing either way and is not offered.
    CostsAnything,
    /// A card that costs energy or stars with everything standing over its
    /// cost taken into account.
    CostsAnythingNow,
    /// A card whose cost as written, upgrade included but nothing standing
    /// over it, is energy or stars.
    CostsAnythingAsWritten,
    /// Both of the last two at once: the first of the four lists a mummified
    /// hand rolls over.
    CostsAnythingAsWrittenAndNow,
    /// The card is ethereal as its model is written, rather than as a
    /// particular copy of it stands: the pool cards a big hat can conjure.
    Ethereal,
    /// A card of the colorless pool, which is what an heirloom hammer copies
    /// out of the hand.
    Colorless,
    /// The card costs energy right now, with whatever was put on the card
    /// itself taken into account and nothing a power or relic says about every
    /// card, and does not cost X. That is a bookmark's filter, and it is
    /// answered off a copy because only a copy carries what stands over its
    /// cost.
    CostsEnergyNow,
    /// The mirror a Scrape puts down: the card either costs something other
    /// than nothing right now or costs X. A card whose cost has been written
    /// to nothing is kept and everything else goes, and an X-cost card goes
    /// whatever the nothing written on it says.
    CostsSomethingNowOrX,
    /// The card costs nothing right now, with everything that stands over its
    /// cost taken into account, and does not cost X. That is a wider question
    /// than the one a bookmark asks, which reads only what the fight put
    /// there.
    CostsNothingNow,
    /// An attack, a skill, or a power, which is how the build says "not a
    /// status, a curse, or a quest card" where it means the three playable
    /// families.
    AttackSkillOrPower,
    /// A card that can leave its pile and is neither a quest card nor
    /// otherwise fixed in place.
    Transformable,
    /// The same, and basic: the pair a wood carving asks of the deck before it
    /// turns a starter into something else.
    BasicTransformable,
}

/// What a pile command puts into a pile.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CardOrigin {
    /// A copy of the card the listener pass was about, which is what a music
    /// box makes of the attack that was just played.
    CloneOfPlayedCard,
    /// A fresh card of this model.
    Model(ModelId),
    /// A card already described in full, which is how a room hands one over.
    Card(CardFingerprint),
    /// A copy of the card that issued the command, fingerprint and all.
    CloneOfSource,
    /// The card the enclosing selection picked, which for a screen over cards
    /// that stand in no pile is a card the roll made and nothing has taken
    /// hold of yet.
    Chosen,
    /// A copy of the card the enclosing selection picked, which is what a
    /// dual wield makes of the attack it was pointed at. The copy carries
    /// everything the original instance stood with, and is marked a clone.
    CloneOfChosen,
    /// A copy of the card the power whose listener issued the command is
    /// carrying, which is what a Nightmare deals out of the card it was
    /// pointed at.
    CloneOfHeldByListeningPower,
}

/// A value a command reads out of the state it runs in.
///
/// `Scaled` is the shape a damage value that grows takes: a base plus an extra
/// value for each of something counted.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Amount {
    Fixed(i32),
    Scaled {
        base: i32,
        per: i32,
        count: Count,
    },
    /// A part of something counted: the product is worked out first, and then
    /// divided by a hundred with the remainder dropped.
    Percent {
        percent: i32,
        count: Count,
    },
    /// One of two values, chosen by a condition, which is how a body works out
    /// a value that depends on the board.
    IfElse {
        condition: Condition,
        then: Box<Amount>,
        otherwise: Box<Amount>,
    },
    /// Two values multiplied, which is what a body works out when it owes one
    /// thing per something else.
    /// The smaller of what a body works out two ways.
    Least(Vec<Amount>),
    /// The larger of them, which is the floor a body puts under a value it
    /// works out as a fraction.
    Greatest(Vec<Amount>),
    /// Two raised to this power, the only exponent anything covered raises,
    /// which is what a Shadowmeld doubles block by.
    TwoToThe(Box<Amount>),
    /// Several values added together, which is what a body that gathers a
    /// number out of more than one place works out.
    Sum(Vec<Amount>),
    Product {
        left: Box<Amount>,
        right: Box<Amount>,
    },
    /// How many whole times a batch goes into a value, rounded down, which is
    /// what a relic that pays out per completed batch reads.
    Quotient {
        of: Box<Amount>,
        by: i32,
    },
    /// What is left of a value once the whole batches have been taken out of
    /// it.
    Remainder {
        of: Box<Amount>,
        by: i32,
    },
}

impl Amount {
    #[must_use]
    pub const fn per(base: i32, per: i32, count: Count) -> Self {
        Self::Scaled { base, per, count }
    }

    /// As many as there are of the counted thing.
    #[must_use]
    pub const fn counting(count: Count) -> Self {
        Self::Scaled {
            base: 0,
            per: 1,
            count,
        }
    }

    /// As many as there are, taken away rather than given.
    #[must_use]
    pub const fn negating(count: Count) -> Self {
        Self::Scaled {
            base: 0,
            per: -1,
            count,
        }
    }
}

/// What an [`Amount`] counts.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Count {
    /// Block the owner is standing behind.
    OwnerBlock,
    /// Block whoever the play or the potion was aimed at is standing behind.
    TargetBlock,
    /// Stacks of a power the target carries.
    TargetPower(ModelId),
    /// The stored variable of a named power the listener's own owner carries,
    /// which is what a Surprise reads off the Thievery beside it.
    OwnerPowerCounter(ModelId),
    /// A number the listener's own owner wrote down for itself.
    OwnerLocal(String),
    /// How much of a power the listener's own owner is holding, which is what a
    /// blow that grows with the mover's own stat reads.
    OwnerPower(ModelId),
    /// Stacks of a power whoever is dealing the damage under modification
    /// carries, which is what a value-shaping body reads off the other side of
    /// the blow.
    DamageDealerPower(ModelId),
    /// One if the player dealing the damage under modification holds this
    /// relic, nothing otherwise. It is what a power that lets a relic raise
    /// its own multiplier reads.
    DamageDealerRelic(ModelId),
    /// The mirror: one if the player *taking* the damage holds this relic. It
    /// is what a Weak reads before it settles its own multiplier.
    DamageTargetRelic(ModelId),
    /// How many cards the run deck holds.
    DeckSize,
    /// Cards anywhere in the combat that carry a tag.
    TaggedCards(CardTag),
    /// Stacks of the power whose listener issued the command.
    ListeningPowerAmount,
    /// What that power was worth when the card play now finishing began, and
    /// nothing at all when the power was not standing then. It is what an
    /// Afterimage notes as a play opens and reads back as it finishes.
    ListeningPowerAmountBeforeThisPlay,
    /// The saved counter of the relic whose listener issued the command.
    ListeningRelicCounter,
    /// The second count that relic keeps beside the first.
    ListeningRelicSecondCounter,
    /// The stored variable of the power whose listener issued the command.
    ListeningPowerCounter,
    /// What the entry the innermost in-flight play opened on the listening
    /// power's tally has run up to, and nothing where the play opened none.
    ListeningPowerPlayTally,
    /// How many cards the enclosing selection picked.
    ChosenCards,
    /// How many cards stand in one of the owner's combat piles.
    CardsInPile(PileName),
    /// How many cards of one type stand in one of the owner's combat piles.
    CardsInPileOfType {
        pile: PileName,
        card_type: CardType,
    },
    /// How many more cards the hand has room for, never below nothing.
    SpaceLeftInHand,
    /// Where the modifier pass this body is running in has the value standing
    /// right now, which is what the listener is being asked about.
    ModifiedValue,
    /// How many orbs the owner is holding.
    OrbsHeld,
    /// How many of them are of this model, which is what a Hailstorm reads.
    OrbsHeldOfModel(ModelId),
    /// How many different orb models the owner is holding, which is what a
    /// Compile Driver and a Synchronize both read.
    DistinctOrbModelsHeld,
    /// How many orb slots the owner has, filled or not.
    OrbSlots,
    /// The passive and evoke amounts of the orb this body is about, worked out
    /// fresh: the orb's own stored amount put through the orb-value passes
    /// where the orb asks for it, and left alone where it does not.
    OrbPassiveValue,
    OrbEvokeValue,
    /// The orb's own stored amounts behind those two, with no pass over them.
    /// A body writing its own number reads the stored amount rather than the
    /// answer: a Glass orb wears its stored passive amount down by one
    /// whatever a Focus is making of it.
    OrbPassiveField,
    OrbEvokeField,
    /// How many orbs of this model the owner has channelled in this fight,
    /// which is what a Voltaic reads.
    OrbsChanneledThisCombat(ModelId),
    /// How many opponents of the owner's are standing and can be hit, which is
    /// what a Chill channels one Frost orb for each of.
    LivingOpponents,
    /// The energy the player is standing on right now, which is what a Double
    /// Energy hands back to them.
    PlayerEnergy,
    /// One of the numbers the card that issued the command carries with it,
    /// which is what a Genetic Algorithm reads its own grown block off.
    SourceCardProperty(String),
    /// How many plays the owner has begun this turn, counting only the first
    /// of a series.
    OwnerFirstPlaysStartedThisTurn,
    /// The same narrowed to zero-energy attack plays, which is what a Feral
    /// counts as it lands.
    OwnerZeroCostAttackPlaysThisTurn,
    /// How many status cards the owner has drawn this turn.
    OwnerStatusCardsDrawnThisTurn,
    /// How many plays of the owner's have finished this turn.
    OwnerCardPlaysFinishedThisTurn,
    /// How much energy the owner has spent this turn, added up over the
    /// turn.
    OwnerEnergySpentThisTurn,
    /// What the card that issued the command costs in energy right now, with
    /// everything standing over its own cost taken into account.
    SourceCardEnergyCost,
    /// The amount the pass the listener is answering carries, which is what a
    /// body answering a power's amount having changed reads.
    AnnouncedAmount,
    /// How many times the listener's owner has stood on this move state, which
    /// is the count a repeat weight reads.
    TimesOwnerEnteredState(String),
    /// How many times the listener's owner has performed this move, which is
    /// the count a monster keeping "how many times have I done this" for
    /// itself reads. A move is stood on a turn before it is performed, so
    /// the two counts are not the same: inside the move's own body this one has
    /// already counted it.
    TimesOwnerPerformedState(String),
    /// How many hits of the swing just finished got past the listener's own
    /// owner's block, which is what a power answering for the blow its owner
    /// just took reads off the attack's results.
    UnblockedHitsOnOwner,
    /// How many hits of the swing just finished got past a player's block,
    /// which is what a Painful Stabs reads off the attack's results.
    UnblockedHitsOnPlayers,
    /// How many hits of the swing just finished got past somebody's block,
    /// counted a hit at a time rather than a result at a time.
    ///
    /// This is how a Suck reads the attack's results: for each hit, the
    /// owner's row is dropped where the same hit also reached that owner's pet
    /// — a blow the pet took and then trampled through settles twice and is
    /// one hit — and the hit is counted if anything of it got through at
    /// all.
    HitsThatGotThrough,
    /// The listener's owner's maximum and current HP.
    OwnerMaxHp,
    OwnerCurrentHp,
    /// The owner's pet's maximum and current HP, which an Unleash, a
    /// Protector and a Sacrifice each read. A pet that is missing or down
    /// counts for nothing.
    PetMaxHp,
    PetCurrentHp,
    /// How many slots of the belt are free. An entropic brew fills them one at
    /// a time and stops when the procure fails, which comes to the same run of
    /// rolls as counting first.
    OpenPotionSlots,
    /// What the enclosing [`Command::Shield`] worked up once the block passes
    /// had their say, which is what the gain itself settles at.
    GainedBlock,
    /// Every blow the enclosing binding settled, added up over both what
    /// landed and what was overkilled.
    TotalDealtDamageWithOverkill,
    /// How many lasting debuffs whoever the play was aimed at is carrying: a
    /// debuff that gives itself back at the end of the turn does not count,
    /// and a counter power worked backwards does.
    TargetLastingDebuffs,
    /// How many attack plays the owner has finished this turn.
    OwnerAttacksPlayedThisTurn,
    /// How many card plays have finished in the whole combat.
    CardPlaysFinishedThisCombat,
    /// How many of the owner's plays finished with the card carrying Ethereal,
    /// over the whole fight, which is what a banshee's cry is cheaper for and
    /// what a pull from below lands once per.
    EtherealPlaysThisCombat,
    /// How many cards of one model stand in one of the owner's combat piles,
    /// which is what a soul storm grows on.
    CardsInPileOfModel {
        pile: PileName,
        model: ModelId,
    },
    /// How many creatures the doom kill the pass is about took down whose
    /// deaths counted, leaving out the listener's own creature. The set is
    /// worked out while the doomed are still standing, because a creature the
    /// kill removed can no longer be asked.
    CreaturesThatDiedToDoom,
    /// What the announced creature just lost off its HP, and nothing where it
    /// gained instead.
    HpTheAnnouncedCreatureLost,
    /// The whole of one blow, which is the half block stopped plus the half
    /// that got through. It is what a reaper form dooms for and is not the
    /// same as the unblocked half on its own.
    TotalDamageOfTheBlow,
    /// What the enclosing [`Command::WithAmount`] worked out before it ran its
    /// body, which is what a body settles first when the value it is about to
    /// act on would not survive what the body does before it gets there.
    Bound,
    /// How many plays of a card of this type have finished this turn.
    OwnerCardsOfTypePlayedThisTurn(CardType),
    /// Cards a discard has put down this turn. The hand flush is not one of
    /// them: it moves the cards into the pile without the fight recording a
    /// discard.
    OwnerCardsDiscardedThisTurn,
    /// Cards the owner has drawn in this fight. It is read off the combat's
    /// own record, so this is not a turn's count.
    OwnerCardsDrawnThisCombat,
    /// The same narrowed to this turn and to the draws the turn's own opening
    /// hand draw did not make, which is what a death march grows on.
    OwnerCardsDrawnThisTurnOutsideHandDraw,
    /// Whole swings the owner's pet has made this turn. A swing is recorded
    /// once however many hits it makes, so a swing of several hits counts
    /// once. A rattle lands one more time for each of them.
    OwnersPetAttacksThisTurn,
    /// How many attack plays of the owner's have *begun* this turn, which is
    /// what a Lethality measures the turn's first attack by.
    OwnerAttackPlaysStartedThisTurn,
    /// Cards of a pile carrying a tag, which is what a Knife Trap counts as it
    /// looks for the Shivs that have burned up.
    CardsInPileTagged {
        pile: PileName,
        tag: CardTag,
    },
    /// Every stack of a power the living creatures on the other side are
    /// carrying, added up, which is how a Mirage knows how much poison is out
    /// there.
    LivingOpponentsPower(ModelId),
    /// What the blow the enclosing [`Command::Deal`] settled came to, what
    /// landed and what was overkilled together.
    DealtDamageWithOverkill,
    /// The maximum HP of whoever the play or the move was aimed at.
    TargetMaxHp,
    /// What the card that issued the command has grown its own damage to: the
    /// value the card is written for, plus everything a body has added to it.
    SourceCardDamage,
    /// A number the card that issued the command wrote down for itself earlier
    /// in the fight.
    SourceCardLocal(String),
    /// How many blows the owner has taken this combat that block did not stop
    /// whole.
    OwnerUnblockedBlowsTaken,
    /// What the blow the pass is about cost its target once block had taken
    /// what it could.
    UnblockedDamage,
    /// Which of the player's turns the fight is on.
    PlayerTurn,
    /// How many times the card that issued the command runs its body for one
    /// play, which is its replay count and one.
    SourceCardPlayCount,
    /// What the play of an X-cost card spent, which is every point of energy
    /// its owner had, put through the X-value passes.
    EnergyX,
    /// The same for a card whose star cost is X, read off what the play wrote
    /// down as spent. Both go through the one set of X-value passes, which is
    /// why a Chemical X is worth two more hits of a Stardust as readily as two
    /// more of an X-cost attack.
    StarX,
    /// How many stars the listener's owner is standing on.
    OwnerStars,
    /// How many stars the owner has gained since their turn opened, which is
    /// what a Radiate counts its hits off: only the gains this turn, not the
    /// spends.
    StarsGainedThisTurn,
    /// How many cards the owner has had conjured into this fight, over the
    /// whole of it. A Supermassive grows on it.
    CardsGeneratedThisCombat,
    /// How many powered blows the owner has landed on whoever the play is
    /// aimed at this turn. It is a Beat Into Shape's whole arithmetic.
    PoweredBlowsOnTargetThisTurn,
    /// How much of the blow the target's block took, which is what a reflect
    /// deals back to whoever swung.
    BlockedDamageOfTheBlow,
    /// How many cards the listener's owner finished playing on their previous
    /// turn, which is the threshold a pale blue dot asks about.
    OwnerCardPlaysLastTurn,
    /// How many of the owner's cards anywhere in the fight cost stars at all,
    /// an X star cost included. It is the *printed* cost the count reads, so
    /// nothing that moves what a card costs right now moves this.
    CardsThatCostStars,
    /// Every coin the player is holding, which is what a body that takes the
    /// whole of it away names.
    PlayerGold,
    /// What the listening enchantment is standing at.
    ListeningEnchantmentAmount,
    /// One of the numbers the listening enchantment writes down for itself for
    /// the fight, which is the extra damage a Momentum has worked up.
    ListeningEnchantmentLocal(String),
}

/// A test a model body makes before acting.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Condition {
    /// The owner has lost HP this turn.
    OwnerLostHpThisTurn,
    /// A blow block did not stop whole landed on the listener's owner during
    /// their own last turn or the enemy turn after it. The build treats it as
    /// having lost HP, but what it actually asks is whether the blow was fully
    /// blocked — so a blow worth nothing against a player standing behind no
    /// block passes it too.
    OwnerTookAnUnstoppedBlowLastTurn,
    /// The owner has a pet and it is standing. Every card that swings the pet
    /// opens with it, and a body that fails it issues nothing at all.
    PetIsStanding,
    /// The listening power has more stacks than this.
    ListeningPowerAmountAbove(i32),
    /// The power was standing when the card play now finishing began, and
    /// what it was worth then was at least this. A power that was not standing
    /// then has no entry at all, which is what the whole of a body like an
    /// Afterimage's is guarded on; the amount test on top of that is what two
    /// of the three add.
    ListeningPowerAmountBeforeThisPlayAtLeast(i32),
    /// The listening power's owner has *begun* a play of a card of this type
    /// this turn, which is what a Smoggy asks before it smogs a skill that has
    /// just joined the fight.
    OwnerStartedAPlayOfTypeThisTurn(CardType),
    /// The damage under modification came from the card the listening power
    /// bound its one swing to, or the power has bound nothing, or the damage
    /// came from no card at all. It is the pair of tests a Vigor makes before
    /// it adds itself.
    DamageCardMatchesTheListeningPowersBinding,
    /// The listening power's stored variable is exactly this.
    ListeningPowerCounterIs(i32),
    /// Nothing the listening model's owner can hit has HP that runs out. A
    /// field with nothing standing in it answers yes.
    EveryOpponentsHpDisplayIsInfinite,
    /// The innermost in-flight play opened an entry on the listening power's
    /// tally.
    ListeningPowerPlayTallyIsOpen,
    /// The room the run is standing in is one the listening relic marked.
    ListeningRelicMarkedThisRoom,
    /// The listening relic has not been spent this fight and its owner has
    /// chosen no card play this turn. It is what guards the hand a Pael's Eye
    /// burns before it takes the turn.
    ListeningRelicsExtraTurnIsOwed,
    /// The listening relic has bound no card yet, which is what a lamp's
    /// arming pass gives up on.
    ListeningRelicHasNoBoundCard,
    /// The card the pass now running is about is the one the listening relic
    /// bound. A pass with no card and a relic with no binding both answer no.
    AnnouncedCardIsTheRelicsBoundCard,
    /// The power the pass now running is about is the stat lent by the power
    /// the listening relic armed on.
    TheAnnouncedPowerIsLentByTheRelicsBoundPower,
    /// The card whose play the pass is about is the one the listening relic
    /// bound.
    PlayedCardIsTheRelicsBoundCard,
    /// The power the pass now running is about would land as a lasting debuff
    /// on the other side from the listening relic's owner, worked by that owner
    /// off a card: the run of early returns an unsettling lamp arms on.
    TheRelicsOwnerIsWorkingADebuffFromACard,
    /// The listening relic's counter has come round to a multiple of this.
    ListeningRelicCounterMultipleOf(i32),
    /// The listener's owner does not already carry this power.
    OwnerLacksPower(ModelId),
    /// The listener's owner carries less of this power than the amount named,
    /// which is what a body that grows a power up to a cap asks. A power the
    /// owner does not carry at all counts as none of it.
    OwnerPowerBelow { power: ModelId, amount: i32 },
    /// The enchantment whose body is running has not been spent or otherwise
    /// moved off its ordinary standing, which is the guard Sown and Swift open
    /// their once-ever payout with.
    ListeningEnchantmentIsNormal,
    /// The card the running body belongs to aims at every enemy at once, which
    /// is what Inky asks before it decides whether to work its debuff on the
    /// play's target or on the whole enemy side.
    SourceCardTargetsAllEnemies,
    /// The test the body makes the other way round.
    Not(Box<Condition>),
    /// Every test has to pass.
    All(Vec<Condition>),
    /// The listener's owner is this monster.
    OwnerIsMonster(ModelId),
    /// The listener's owner has no more HP left than the listening power's
    /// amount, which is what a power measuring its owner against itself asks.
    OwnerHpAtMostListeningPowerAmount,
    /// The listener's owner is standing behind at least this much block.
    OwnerBlockAtLeast(i32),
    /// At least this many cards stand in one of the owner's combat piles,
    /// which a body checks before it acts at all.
    CardsInPileAtLeast { pile: PileName, count: i32 },
    /// At least this many cards of one type stand in one of the owner's combat
    /// piles.
    CardsInPileOfTypeAtLeast {
        pile: PileName,
        card_type: CardType,
        count: i32,
    },
    /// Every card of a pile is of this type, which is what a Clash asks of the
    /// hand before it lets itself be played. An empty pile answers yes.
    EveryCardInPileIsOfType { pile: PileName, card_type: CardType },
    /// A card of the owner's has burned up this turn, which an Evil Eye and a
    /// Forgotten Ritual both read out of the combat history.
    OwnerExhaustedACardThisTurn,
    /// The burn the pass is about was a card burning up in hand at the end of
    /// a turn because it was ethereal.
    ExhaustCausedByEthereal,
    /// The card that issued the command is upgraded, which is what a body
    /// reading one of its own card's values comes to: an upgrade raises the
    /// value, and the two levels are the two answers.
    SourceCardIsUpgraded,
    /// The card that issued the command was played on the turn before the one
    /// standing now, which is what a card that comes back to the hand asks of
    /// the fight's history.
    SourceCardPlayedLastTurn,
    /// A play of the card that issued the command has *finished* on the turn
    /// standing now, which is what a fetch asks of the fight's history. The
    /// play the card is running right now has not finished, so a card asking
    /// this inside its own body does not count itself.
    SourceCardPlayedThisTurn,
    /// The listener's own owner has worked this power onto anybody this turn,
    /// which is what a death's door asks about a Doom.
    OwnerAppliedPowerThisTurn(ModelId),
    /// The listener's owner's pet has swung at least once this turn, which is
    /// what a flatten reads.
    OwnersPetAttackedThisTurn,
    /// What the enclosing [`Command::WithAmount`] worked out is at least this,
    /// which is what a body asks about a count it took before it changed the
    /// board the count was read off.
    BoundAtLeast(i32),
    /// The card that issued the command is standing in this pile, which is
    /// what a card answering for where it stands opens with.
    SourceCardIsInPile(PileName),
    /// The card that issued the command was made a copy of another. A card
    /// whose entered-combat listener pays it for what the fight has already
    /// seen refuses to pay a copy.
    SourceCardIsAClone,
    /// The card that issued the command is the combat's copy of a card in the
    /// run deck rather than one conjured mid-fight, which is what a card that
    /// takes itself out of the deck as it is played asks before it tries.
    SourceCardCameFromTheDeck,
    /// The card that issued the command is the run deck's own card rather than
    /// a combat's copy of it. A body that counts something across a run opens
    /// with this, because the copy in the fight would otherwise count it a
    /// second time.
    SourceCardIsInTheDeck,
    /// One of the numbers the card that issued the command carries with it has
    /// reached at least this value.
    SourceCardPropertyAtLeast { name: String, value: i32 },
    /// The same read for equality, which is what a body that branches on a
    /// number standing for one of a fixed set of things asks: a Mad Science is
    /// whatever type and whatever rider the tinker's table wrote onto it.
    SourceCardPropertyIs { name: String, value: i32 },
    /// Something whose death counts has gone down since this play began: one
    /// of the swing's hits killed its target, and the target was one whose
    /// death the body had already decided would count.
    AFatalKillHappenedThisPlay,
    /// Every power the target carries says its death would count as a kill.
    /// The two that say otherwise are a minion, which never counts, and a
    /// segment of a chain, which counts only once the rest of it is down.
    TargetsDeathWouldTriggerFatal,
    /// How many blows the owner has taken this combat that block did not stop
    /// whole is at least this.
    OwnerUnblockedBlowsTakenAtLeast(i32),
    /// At least one hit of the swing just finished got past the listener's own
    /// owner's block, which is what a power answering for what its owner just
    /// took tests.
    OwnerWasHitByTheAttack,
    /// The block gains this turn that came off a card or a monster move, other
    /// than the play now working one up, are fewer than the listening power's
    /// amount.
    BlockGainsFromOtherPlaysBelowListeningPowerAmount,
    /// The listener's owner has more HP left than this many hundredths of
    /// their maximum, which is what a relic that watches a health bar opens
    /// with.
    OwnerHpAboveFraction(i32),
    /// The play the pass is about spent at least this much energy.
    PlayedCardEnergyAtLeast(i32),
    /// The listening relic's second count is at least this, which is what a
    /// relic asks about what it wrote down last turn.
    ListeningRelicSecondCounterAtLeast(i32),
    /// One bit of the listening relic's count is set. A relic with several
    /// yes-or-nos to remember keeps them as bits of the one count it has.
    ListeningRelicCounterHasBit(i32),
    /// The enclosing selection picked nothing at all, which is what a body
    /// tests before it tries the next set.
    NoCardWasChosen,
    /// A count standing at this much or more.
    CountAtLeast { count: Count, at_least: i32 },
    /// The same measured against the listening power's own amount rather than
    /// against a written number, which is what an Echo Form caps itself with.
    CountBelowListeningPowerAmount(Count),
    /// A count that has come round to a multiple of this, which is what a Make
    /// It So asks of the turn's skill plays.
    CountIsAMultipleOf { count: Count, of: i32 },
    /// The play the pass is about paid stars for itself.
    PlayedCardSpentStars,
    /// The card the enclosing screen handed back is still standing in one of
    /// the two piles a Cosmic Indifference may move it between, the draw pile
    /// or the discard pile.
    ChosenCardIsInDrawOrDiscard,
    /// The card that issued the command is the first card of its owner's draw
    /// pile, which is what an I Am Invincible asks before it plays itself off
    /// the top of it.
    SourceCardIsOnTopOfTheDrawPile,
    /// The card that issued the command is about to burn up, either because
    /// it carries Exhaust or because something has set it to burn on its next
    /// play. A Shining Strike puts itself back on the draw pile only where it
    /// would otherwise be discarded.
    SourceCardBurnsUp,
    /// The card that has just joined the run deck is of this type, which is
    /// what a relic answering a deck addition tests.
    JoinedDeckCardIsOfType(CardType),
    /// Where the enclosing modifier pass has the value standing right now is
    /// at least this, which a body tests before it answers.
    ModifiedValueAtLeast(i32),
    /// A fight has been announced and has not ended. A body a listener can
    /// reach after a fight is over — a relic paying out for the tenth attack,
    /// where the tenth killed the last enemy — asks this before it pays.
    InACombat,
    /// The blow the pass is about was one block had no say over.
    DamageWasUnblockable,
    /// The player is holding no potion at all.
    HoldsNoPotions,
    /// The map point the run is standing on is an unknown one, whatever it
    /// turned out to be.
    CurrentMapPointIsUnknown,
    /// The room the run is standing in is a monster, elite, or boss room.
    CurrentRoomIsCombat,
    /// The act the run is walking is the last one it has.
    InTheLastAct,
    /// The card offer the pass is working on is the one a fight paid out.
    CardOfferCameFromAnEncounter,
    /// The creation the pass is working on is a card reward rather than cards
    /// a body made for itself.
    CardOfferIsACardReward,
    /// The listening relic's counter is exactly this.
    ListeningRelicCounterIs(i32),
    /// The listening relic's counter has reached at least this.
    ListeningRelicCounterAtLeast(i32),
    /// The yes-or-no the listening relic keeps is set.
    ListeningRelicIsLatched,
    /// The second count the listening relic keeps is not nothing, which is how
    /// a relic that marks something within one play reads its own mark back.
    ListeningRelicSecondCounterIsSet,
    /// Whoever the play or the move was aimed at is still alive.
    TargetIsAlive,
    /// Every living creature on the side opposite the owner's carries this
    /// power, and there is at least one of them.
    AllOpponentsHavePower(ModelId),
    /// Whoever the play or the move was aimed at carries this power.
    TargetHasPower(ModelId),
    /// Whoever issued the command carries this power, which is what a waking
    /// beetle asks before it takes its own shell off.
    OwnerHasPower(ModelId),
    /// Whoever is dealing the damage under modification carries this power.
    DamageDealerHasPower(ModelId),
    /// The blow came from the pet of whoever worked the listening power on,
    /// which is what a Sic 'Em asks of the mark it is sitting on.
    DamageDealerIsTheAppliersPet,
    /// The power being worked counts as a debuff at the amount it carries,
    /// which can differ from what the power counts as in general.
    AppliedPowerIsDebuff,
    /// The card the enclosing selection picked is this one.
    ChosenCardIs(ModelId),
    /// The card the enclosing selection picked is of this type, which is what
    /// an Escape Plan asks of the card it drew.
    ChosenCardIsOfType(CardType),
    /// Whoever is dealing the damage the pass is about is the creature that
    /// worked the listening power on, which is what a Flanking asks before it
    /// decides not to raise the blow.
    DamageDealerIsTheApplier,
    /// The stored count of the listening power has reached this.
    ListeningPowerCounterAtLeast(i32),
    /// A card carrying this tag finished playing this turn, for the listener's
    /// owner. It is what a Phantom Blades asks so that only the turn's first
    /// Shiv is paid for.
    OwnerPlayedATaggedCardThisTurn(CardTag),
    /// The listener's owner has stood on this move state this many times,
    /// which is what a monster counting its own move reads off the count it
    /// keeps for itself.
    OwnerEnteredStateTimes { state_id: String, times: usize },
    /// The damage the pass is about came from a card at all.
    DamageCameFromACard,
    /// The card the damage under modification came from is upgraded.
    DamageCardIsUpgraded,
    /// The card the damage under modification came from is of this type, which
    /// is what a body that only answers for attacks opens with.
    DamageCardIsOfType(CardType),
    /// The card the damage under modification came from carries this tag.
    DamageCardIsTagged(CardTag),
    /// The card the damage under modification came from is this model, which
    /// is what a power that only raises the blow of the card that put it there
    /// opens with.
    DamageCameFromCard(ModelId),
    /// The listening power's own counter has not reached its amount, which is
    /// what Chains of Binding asks as a card is drawn.
    ListeningPowerCounterBelowAmount,
    /// The yes-or-no the listening power keeps is set.
    ListeningPowerIsLatched,
    /// The card that was just played carries this affliction.
    PlayedCardIsAfflicted(ModelId),
    /// The creature the pass is about is this monster.
    AnnouncedCreatureIsMonster(ModelId),
    /// The creature the pass is about stands on the side opposite the
    /// listener's owner, which is what a body answering an arrival opens
    /// with.
    AnnouncedCreatureIsAnOpponent,
    /// The listener's owner is still standing.
    OwnerIsAlive,
    /// The listener's owner is doomed — it has no more HP left than the
    /// listening power is worth — and is the first such creature on its own
    /// side. Doom gathers the whole doomed set as the side's turn ends and
    /// kills it from that one listener, so every other doomed creature's own
    /// listener finds itself already down.
    OwnerIsTheFirstDoomedOnItsSide,
    /// The creature the listening power is about is still standing, which is
    /// half of what a sandpit checks as it comes off.
    ListeningPowerTargetIsAlive,
    /// The enemy side is the one acting, which is how an egg laid on the
    /// layer's own turn knows it has a whole player turn to sit through.
    EnemySideIsActing,
    /// The listener's owner is marked off balance.
    OwnerIsOffBalance,
    /// The listener's owner is standing on this move state, which for a
    /// monster showing an intent is the move that intent belongs to.
    OwnerMoveStateIs(String),
    /// The room the run is standing in is of this family.
    CurrentRoomIs(crate::ResolvedRoom),
    /// The player can afford what the body is about to spend.
    PlayerGoldAtLeast(i32),
    /// The player has energy left, which is what a relic that pays for a
    /// frugal turn writes down as the turn ends.
    PlayerHasEnergy,
    /// What the play was aimed at is showing an attack, which is what a Go For
    /// The Eyes asks before it works its debuff on.
    TargetIntendsToAttack,
    /// The swing the enclosing [`Command::Strike`] just made killed something,
    /// on any of its hits.
    TheSwingKilledSomething,
    /// The orb the pass is about is this model, which is what a Thunder opens
    /// with.
    AnnouncedOrbIs(ModelId),
    /// The orb the pass is about is the one at the front of the queue.
    AnnouncedOrbIsAtTheFront,
    /// The run is walking this act.
    CurrentActIs(usize),
    /// The player's turn number is at least this, which is the turn a relic
    /// starts paying out on.
    PlayerTurnAtLeast(u32),
    /// The player's turn number is at most this, which is what a relic that
    /// only acts on the opening turn asks.
    PlayerTurnAtMost(u32),
    /// The player's turn number is exactly this.
    PlayerTurnIs(u32),
}

/// The settings on a value that change how it resolves. The purely visual
/// ones are not modelled.
///
/// A value is scaled by the listeners that raise it only where it is a powered
/// one: worked out by a card or a monster move, and not marked unpowered.
/// Everything else — a power's own damage, a relic's — is unpowered by
/// construction. Every value this surface carries is one a card or a move
/// worked out, so `unpowered` alone answers the question and being a move's
/// value needs no field of its own.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ValueProps {
    /// Block does not stop it.
    pub unblockable: bool,
    /// The listeners that scale it do not apply to it. A listener that caps it
    /// still does, because a cap makes no such test.
    pub unpowered: bool,
}

impl ValueProps {
    /// What a card's attack and its block carry.
    pub const NONE: Self = Self {
        unblockable: false,
        unpowered: false,
    };
    /// What a power's own damage and block carry: nothing scales them.
    pub const UNPOWERED: Self = Self {
        unblockable: false,
        unpowered: true,
    };
    /// Damage nothing scales and nothing stops.
    pub const UNBLOCKABLE: Self = Self {
        unblockable: true,
        unpowered: true,
    };
}

/// One copy [`Command::WithTargetsDebuffs`] took, kept as the three things a
/// re-application reads off it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BoundDebuff {
    pub model_id: ModelId,
    /// What the power stood at as the copy was taken.
    pub amount: i32,
    /// Who had worked the original on, which the copy is worked with rather
    /// than by whoever is copying it.
    pub applier: Option<CreatureCombatId>,
}

/// What a command knows about where it came from.
///
/// These stand for what a model body has to hand about itself as it issues a
/// command: the owner, the play's target, the card, and the power or relic
/// whose listener is running.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct CommandContext {
    /// The creature that issued the command.
    pub owner: Option<CreatureCombatId>,
    /// The creature the play or the move was aimed at.
    pub target: Option<CreatureCombatId>,
    /// The card the command came from.
    pub card: Option<CardInstanceId>,
    /// The power whose listener issued the command.
    pub power: Option<ModelId>,
    /// What the enclosing [`Command::WithAmount`] worked out, bound for the
    /// commands inside it.
    #[serde(default)]
    pub bound_amount: Option<i32>,
    /// What the enclosing [`Command::WithTargetsDebuffs`] copied, bound for the
    /// commands inside it: one entry per debuff instance, in the order its
    /// owner held them.
    #[serde(default)]
    pub bound_debuffs: Vec<BoundDebuff>,
    /// Which instance of that power, where the listener is a power at all.
    /// `PowerName::Listening` means this instance rather than the first one the
    /// owner carries of the model.
    #[serde(default)]
    pub power_instance: Option<u64>,
    /// The relic whose listener issued the command.
    pub relic: Option<ModelId>,
    /// The belt slot the potion whose listener issued the command sits in.
    #[serde(default)]
    pub potion_slot: Option<usize>,
    /// The card the play that reached this body was made of. It is not the
    /// card that issued the command: a relic answering a play is not a card.
    pub played_card: Option<CardInstanceId>,
    /// The card a pass that is about a card other than a play names, which is
    /// the card a listener for something like a draw is told about.
    #[serde(default)]
    pub announced_card: Option<CardInstanceId>,
    /// Where in the run deck the card whose listener issued the command
    /// stands, when the listener is a deck card rather than a combat one. It
    /// is the deck's counterpart to `card`: a card is the same kind of thing
    /// either way, so a deck card's body names itself the same way a combat
    /// card's does.
    #[serde(default)]
    pub deck_card: Option<usize>,
    /// The cards the enclosing selection picked, carried whole so that a
    /// command after the one that moved them can still read what they were.
    pub chosen: Vec<crate::ChosenCard>,
    /// What the enclosing [`Command::Deal`] or [`Command::Strike`] settled.
    ///
    /// Damage dealt settles one result per creature it reached, in target
    /// order, and an attack settles one such set per hit; a body that reads
    /// them reads either the first or the whole run, so what is bound is the
    /// run, flattened hit by hit in order.
    #[serde(default)]
    pub dealt: Vec<crate::DamageResult>,
    /// What the enclosing [`Command::Shield`] worked up once the block passes
    /// had their say, which is what the gain itself settled at.
    #[serde(default)]
    pub gained_block: i32,
    /// The enchantment whose body issued the command. The card it is on is
    /// named by `card` or by `deck_card` alongside it, which is how an
    /// enchantment's body reaches the card it sits on.
    #[serde(default)]
    pub enchantment: Option<ModelId>,
    /// The orb this body is about: the one an enclosing
    /// [`Command::ForEachOrb`] is on, the one whose own passive or evoke body
    /// is running, or the one the pass that reached this listener was told
    /// about.
    #[serde(default)]
    pub orb: Option<sts2_core::OrbInstanceId>,
    /// That orb's model, kept beside it because an evoke runs its body with the
    /// orb already out of the queue.
    #[serde(default)]
    pub orb_model: Option<ModelId>,
    /// The orb's passive and evoke amounts as they stood when the body opened,
    /// and the two stored amounts behind them. They are carried rather than
    /// looked up because an evoke's own body reads its numbers off an orb the
    /// queue no longer holds.
    #[serde(default)]
    pub orb_passive: i32,
    #[serde(default)]
    pub orb_evoke: i32,
    #[serde(default)]
    pub orb_passive_field: i32,
    #[serde(default)]
    pub orb_evoke_field: i32,
    /// The creatures the enclosing [`Command::WithCreatures`] resolved.
    #[serde(default)]
    pub bound_creatures: Vec<CreatureCombatId>,
}

impl CommandContext {
    /// A command issued by a creature with nothing aimed at.
    #[must_use]
    pub fn by(owner: CreatureCombatId) -> Self {
        Self {
            owner: Some(owner),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn at(mut self, target: CreatureCombatId) -> Self {
        self.target = Some(target);
        self
    }

    #[must_use]
    pub fn from_card(mut self, card: CardInstanceId) -> Self {
        self.card = Some(card);
        self
    }

    #[must_use]
    pub fn from_power(mut self, power: ModelId) -> Self {
        self.power = Some(power);
        self
    }

    #[must_use]
    pub fn from_relic(mut self, relic: ModelId) -> Self {
        self.relic = Some(relic);
        self
    }
}
