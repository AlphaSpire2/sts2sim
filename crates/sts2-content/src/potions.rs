//! Potion definitions: what each potion is, and what drinking it does.
//!
//! Drinking one is a run of commands, the same vocabulary a card is written in.
//! A potion also declares where it may be drunk, whether a roll inside a fight
//! is allowed to conjure it, and what it answers on the shared listener surface
//! while it sits in the belt.

use sts2_engine::{
    Amount, CardCmd, CardFilter, CardOrigin, CardPileCmd, CardSelectCmd, CardSelector, CardType,
    Command, Count, CreatureCmd, CreatureSelector, DrawPilePosition, PileName, PlayerCmd,
    PotionCmd, PotionDefinition, PotionOrigin, PotionRarity, PotionUsage, PowerCmd, SelectorPrefs,
    TargetType, TemporaryCost, ValueProps,
};

use crate::id;

/// A potion that may only be drunk in a fight and that any roll may conjure,
/// which is what all but a handful of them are.
fn potion(
    model_id: &str,
    rarity: PotionRarity,
    target: TargetType,
    effects: Vec<Command>,
) -> PotionDefinition {
    PotionDefinition {
        model_id: id(model_id),
        target,
        rarity,
        usage: PotionUsage::CombatOnly,
        can_be_generated_in_combat: true,
        prevents_owners_death: false,
        hooks: Vec::new(),
        effects,
        thrown_at_a_merchant: None,
    }
}

/// A potion that can be drunk anywhere, a fight included.
fn any_time(mut definition: PotionDefinition) -> PotionDefinition {
    definition.usage = PotionUsage::AnyTime;
    definition
}

/// A potion that heals or resurrects is kept out of the in-combat roll and out
/// of that roll alone.
fn not_conjured_in_combat(mut definition: PotionDefinition) -> PotionDefinition {
    definition.can_be_generated_in_combat = false;
    definition
}

/// A power the potion works on whoever it was aimed at.
fn applied(power: &str, amount: i32) -> Command {
    Command::Power(PowerCmd::Apply {
        marks: None,
        power: id(power),
        target: CreatureSelector::Target,
        applier: Some(CreatureSelector::Owner),
        amount: Amount::Fixed(amount),
    })
}

/// A power the potion works on whoever drank it, which is the handful that
/// hit the drinker rather than the target they were aimed at.
fn applied_to_drinker(power: &str, amount: i32) -> Command {
    Command::Power(PowerCmd::Apply {
        marks: None,
        power: id(power),
        target: CreatureSelector::Owner,
        applier: Some(CreatureSelector::Owner),
        amount: Amount::Fixed(amount),
    })
}

/// A power the potion works on every enemy still standing and able to be hit.
/// The applier is whoever drank it.
fn applied_to_all(power: &str, amount: i32) -> Command {
    Command::Power(PowerCmd::Apply {
        marks: None,
        power: id(power),
        target: CreatureSelector::AllOpponents,
        applier: Some(CreatureSelector::Owner),
        amount: Amount::Fixed(amount),
    })
}

/// Draws that many cards.
fn draw(count: i32) -> Command {
    Command::CardPile(CardPileCmd::Draw {
        count: Amount::Fixed(count),
    })
}

/// The three type potions: three cards of one type rolled out of the drinker's
/// own pool, one of them taken free for the turn and put straight into hand.
///
/// The three are rolled by shuffling the whole filtered pool on the combat
/// card-generation stream and taking the first three, and the offer may be
/// skipped.
fn offers_a_card_of_type(model_id: &str, card_type: CardType) -> PotionDefinition {
    offers_a_card(
        model_id,
        sts2_engine::CardPoolRef::OwnCharacter,
        CardFilter::OfType(card_type),
    )
}

fn offers_a_card(
    model_id: &str,
    pool: sts2_engine::CardPoolRef,
    filter: CardFilter,
) -> PotionDefinition {
    potion(
        model_id,
        PotionRarity::Common,
        TargetType::SelfTarget,
        vec![Command::CardSelect(CardSelectCmd::ChooseAGeneratedCard {
            pool,
            count: 3,
            filter,
            upgraded: false,
            can_skip: true,
            then: vec![Command::CardPile(CardPileCmd::AddGeneratedToCombat {
                enchantment: None,
                card: CardOrigin::Chosen,
                grown_by: Amount::Fixed(0),
                count: Amount::Fixed(1),
                pile: PileName::Hand,
                ethereal: false,
                retain: false,
                upgraded: false,
                // The card is made free for the turn after the roll settles
                // it and before it is handed over.
                temporary_cost: Some(TemporaryCost::free_this_turn()),
                position: sts2_engine::DrawPilePosition::Bottom,
            })],
        })],
    )
}

/// Every potion this baseline registers, in no particular order: the registry
/// keys them by model and the generation pool names its own order.
#[must_use]
pub fn all_potions() -> Vec<PotionDefinition> {
    shared_potions()
        .into_iter()
        .chain(ironclad_potions())
        .chain(event_potions())
        .chain(other_potions())
        .collect()
}

/// The shared potion pool, in the pool's own order. Generation reads this
/// order, so it is fidelity rather than presentation.
#[must_use]
pub fn shared_potion_pool() -> Vec<sts2_core::ModelId> {
    shared_potions()
        .into_iter()
        .map(|definition| definition.model_id)
        .collect()
}

/// The generation options split by rarity, keeping the pool's own order, which
/// is what a roll for one rarity is offered.
///
/// A reward and a merchant roll the same three lists a card's mid-fight roll
/// does; the only difference is the stream they read. Deriving them here is
/// what keeps the three paths one table rather than three.
#[must_use]
pub(crate) fn generation_pool_by_rarity(
    character: &sts2_core::ModelId,
) -> [Vec<sts2_core::ModelId>; 3] {
    let mut split = [Vec::new(), Vec::new(), Vec::new()];
    for definition in character_potions(character)
        .into_iter()
        .chain(shared_potions())
    {
        let bucket = match definition.rarity {
            PotionRarity::Common => 0,
            PotionRarity::Uncommon => 1,
            PotionRarity::Rare => 2,
            PotionRarity::None | PotionRarity::Event | PotionRarity::Token => continue,
        };
        split[bucket].push(definition.model_id);
    }
    split
}

/// The uncommon potions of the character's pool and the shared pool, in that
/// order, which is what an effect rolling one uncommon potion reads.
#[must_use]
pub fn uncommon_generation_potions(character: &sts2_core::ModelId) -> Vec<sts2_core::ModelId> {
    let [_, uncommon, _] = generation_pool_by_rarity(character);
    uncommon
}

/// The three potions a character carries, in the game's own order. A character
/// whose pool is not registered has none.
fn character_potions(character: &sts2_core::ModelId) -> Vec<PotionDefinition> {
    match character.to_string().as_str() {
        "CHARACTER.IRONCLAD" => ironclad_potions(),
        "CHARACTER.SILENT" => silent_potions(),
        "CHARACTER.NECROBINDER" => necrobinder_potions(),
        "CHARACTER.REGENT" => regent_potions(),
        "CHARACTER.DEFECT" => defect_potions(),
        _ => Vec::new(),
    }
}

/// The character's own pool as a potion roll reads it, followed by nothing:
/// the shared pool is chained on at registration.
#[must_use]
pub fn character_potion_pool(character: &sts2_core::ModelId) -> Vec<sts2_core::ModelId> {
    character_potions(character)
        .into_iter()
        .map(|definition| definition.model_id)
        .collect()
}

/// The potion pool that belongs to a character.
fn character_potion_pool_id(character: &sts2_core::ModelId) -> Option<&'static str> {
    match character.to_string().as_str() {
        "CHARACTER.IRONCLAD" => Some("POTION_POOL.IRONCLAD_POTION_POOL"),
        "CHARACTER.SILENT" => Some("POTION_POOL.SILENT_POTION_POOL"),
        "CHARACTER.NECROBINDER" => Some("POTION_POOL.NECROBINDER_POTION_POOL"),
        "CHARACTER.REGENT" => Some("POTION_POOL.REGENT_POTION_POOL"),
        "CHARACTER.DEFECT" => Some("POTION_POOL.DEFECT_POTION_POOL"),
        _ => None,
    }
}

/// The event potion pool, in its own order: the two an event hands over.
/// Neither carries a rolled rarity, so no generation reads them.
const EVENT_POTION_POOL: [&str; 2] = ["POTION.FOUL_POTION", "POTION.GLOWWATER_POTION"];

/// The token pool, whole: the one potion an effect conjures that a reward or a
/// shop must never offer.
const TOKEN_POTION_POOL: [&str; 1] = ["POTION.POTION_SHAPED_ROCK"];

/// The deprecated pool, whole: the stand-in a save reads back where the potion
/// it names is gone.
const DEPRECATED_POTION_POOL: [&str; 1] = ["POTION.DEPRECATED_POTION"];

/// Every potion pool the game defines, registered with the membership the game
/// gives it, and the two pools each character's roll walks.
///
/// The game's mock pool is not among them: the one potion it holds is test
/// scaffolding the build ships for its own tests, and nothing in a run reaches
/// it.
pub(crate) fn register_potion_pools(content: &mut sts2_engine::ContentRegistry) {
    let shared = crate::id("POTION_POOL.SHARED_POTION_POOL");
    content.add_potion_pool(shared.clone(), shared_potion_pool());
    for (model_id, members) in [
        (
            "POTION_POOL.DEPRECATED_POTION_POOL",
            DEPRECATED_POTION_POOL.as_slice(),
        ),
        (
            "POTION_POOL.EVENT_POTION_POOL",
            EVENT_POTION_POOL.as_slice(),
        ),
        (
            "POTION_POOL.TOKEN_POTION_POOL",
            TOKEN_POTION_POOL.as_slice(),
        ),
    ] {
        content.add_potion_pool(
            crate::id(model_id),
            members.iter().copied().map(crate::id).collect(),
        );
    }
    for character in crate::rewards::IMPLEMENTED_CHARACTERS {
        let character = crate::id(character);
        let Some(pool) = character_potion_pool_id(&character) else {
            continue;
        };
        let pool = crate::id(pool);
        content.add_potion_pool(pool.clone(), character_potion_pool(&character));
        // The character's own three first, then the shared forty-five, and
        // every roll reads that one order.
        content.set_character_potion_pools(character, vec![pool, shared.clone()]);
    }
}

/// The Silent's own three, in the game's own order: poison, ghost, cunning.
/// The potions themselves are registered with every other potion; this is the
/// pool's membership and order.
fn silent_potions() -> Vec<PotionDefinition> {
    let named = |model_id: &str| {
        all_potions()
            .into_iter()
            .find(|definition| definition.model_id == id(model_id))
            .expect("the Silent's own potions are registered")
    };
    vec![
        named("POTION.POISON_POTION"),
        named("POTION.GHOST_IN_A_JAR"),
        named("POTION.CUNNING_POTION"),
    ]
}

/// The Defect's own three, in the game's own order: focus, essence of
/// darkness, capacity.
fn defect_potions() -> Vec<PotionDefinition> {
    let named = |model_id: &str| {
        all_potions()
            .into_iter()
            .find(|definition| definition.model_id == id(model_id))
            .expect("the Defect's own potions are registered")
    };
    vec![
        named("POTION.FOCUS_POTION"),
        named("POTION.ESSENCE_OF_DARKNESS"),
        named("POTION.POTION_OF_CAPACITY"),
    ]
}

/// The Regent's own three, in the game's own order: star, concoction,
/// courage.
fn regent_potions() -> Vec<PotionDefinition> {
    let named = |model_id: &str| {
        all_potions()
            .into_iter()
            .find(|definition| definition.model_id == id(model_id))
            .expect("the Regent's own potions are registered")
    };
    vec![
        named("POTION.STAR_POTION"),
        named("POTION.COSMIC_CONCOCTION"),
        named("POTION.KINGS_COURAGE"),
    ]
}

/// The Necrobinder's own three, in the game's own order: doom, ghouls, brew.
fn necrobinder_potions() -> Vec<PotionDefinition> {
    let named = |model_id: &str| {
        all_potions()
            .into_iter()
            .find(|definition| definition.model_id == id(model_id))
            .expect("the Necrobinder's own potions are registered")
    };
    vec![
        named("POTION.POTION_OF_DOOM"),
        named("POTION.POT_OF_GHOULS"),
        named("POTION.BONE_BREW"),
    ]
}

/// The Ironclad's own three.
#[must_use]
fn ironclad_potions() -> Vec<PotionDefinition> {
    vec![
        // A fifth of whoever drinks it, which the build works out as a decimal
        // and hands to the heal whole.
        any_time(potion(
            "POTION.BLOOD_POTION",
            PotionRarity::Common,
            TargetType::AnyPlayer,
            vec![Command::Creature(CreatureCmd::Heal {
                target: CreatureSelector::Target,
                amount: Amount::Percent {
                    percent: 20,
                    count: Count::TargetMaxHp,
                },
            })],
        )),
        // A soldier's stew makes every Strike in the fight play twice.
        potion(
            "POTION.SOLDIERS_STEW",
            PotionRarity::Rare,
            TargetType::AnyPlayer,
            vec![Command::Card(CardCmd::ModifyReplayCount {
                cards: CardSelector::AllCombatCards {
                    filter: CardFilter::Tagged(sts2_engine::CardTag::Strike),
                },
                amount: Amount::Fixed(1),
            })],
        ),
        // Ashwater lays the hand out and burns whatever is picked out of it.
        potion(
            "POTION.ASHWATER",
            PotionRarity::Uncommon,
            TargetType::SelfTarget,
            vec![Command::CardSelect(CardSelectCmd::FromHand {
                maximum: None,
                exact_count: None,
                // As many as the hand holds, none of them if that is what the
                // player wants.
                prefs: SelectorPrefs::between(0, 999_999_999),
                filter: CardFilter::Any,
                then: vec![Command::Card(CardCmd::Exhaust {
                    cards: CardSelector::Chosen,
                })],
            })],
        ),
    ]
}

/// Every potion of the shared pool, written out in the pool's order.
#[must_use]
#[allow(clippy::too_many_lines, reason = "one entry per potion of the pool")]
fn shared_potions() -> Vec<PotionDefinition> {
    vec![
        offers_a_card_of_type("POTION.ATTACK_POTION", CardType::Attack),
        // Beetle juice shrinks what it is thrown at. Its effect reads the
        // repeat var; the damage-decrease var it also carries is text only.
        potion(
            "POTION.BEETLE_JUICE",
            PotionRarity::Rare,
            TargetType::AnyEnemy,
            vec![applied("POWER.SHRINK_POWER", 4)],
        ),
        potion(
            "POTION.BLESSING_OF_THE_FORGE",
            PotionRarity::Uncommon,
            TargetType::SelfTarget,
            vec![Command::Card(CardCmd::Upgrade {
                cards: CardSelector::Pile {
                    pile: PileName::Hand,
                    filter: CardFilter::Upgradable,
                },
            })],
        ),
        potion(
            "POTION.BLOCK_POTION",
            PotionRarity::Common,
            TargetType::AnyPlayer,
            // Twelve block, which nothing scales.
            vec![Command::Creature(CreatureCmd::GainBlock {
                target: CreatureSelector::Target,
                amount: Amount::Fixed(12),
                props: ValueProps::UNPOWERED,
            })],
        ),
        // Bottled potential puts the hand back into the draw pile, shuffles the
        // lot, and deals a fresh five.
        potion(
            "POTION.BOTTLED_POTENTIAL",
            PotionRarity::Rare,
            TargetType::AnyPlayer,
            vec![
                Command::CardPile(CardPileCmd::MoveToPile {
                    cards: CardSelector::Pile {
                        pile: PileName::Hand,
                        filter: CardFilter::Any,
                    },
                    pile: PileName::Draw,
                    position: DrawPilePosition::Bottom,
                }),
                Command::CardPile(CardPileCmd::Shuffle),
                draw(5),
            ],
        ),
        potion(
            "POTION.CLARITY",
            PotionRarity::Uncommon,
            TargetType::AnyPlayer,
            vec![draw(1), applied("POWER.CLARITY_POWER", 3)],
        ),
        // Three colorless cards rolled for the fight, one of them free for the
        // turn and put straight into the hand.
        offers_a_card(
            "POTION.COLORLESS_POTION",
            sts2_engine::CardPoolRef::Named(id("CARD_POOL.COLORLESS_CARD_POOL")),
            CardFilter::Any,
        ),
        potion(
            "POTION.CURE_ALL",
            PotionRarity::Uncommon,
            TargetType::AnyPlayer,
            vec![
                Command::Player(PlayerCmd::GainEnergy(Amount::Fixed(1))),
                draw(2),
            ],
        ),
        potion(
            "POTION.DEXTERITY_POTION",
            PotionRarity::Common,
            TargetType::AnyPlayer,
            vec![applied("POWER.DEXTERITY_POWER", 2)],
        ),
        // Distilled chaos plays the top three cards of the draw pile, each one
        // going wherever a played card of its own would go.
        potion(
            "POTION.DISTILLED_CHAOS",
            PotionRarity::Rare,
            TargetType::SelfTarget,
            vec![Command::CardPile(CardPileCmd::AutoPlayFromDrawPile {
                count: Amount::Fixed(3),
                position: DrawPilePosition::Top,
                force_exhaust: false,
            })],
        ),
        // A droplet lays the draw pile out and takes one card of it into hand.
        potion(
            "POTION.DROPLET_OF_PRECOGNITION",
            PotionRarity::Rare,
            TargetType::SelfTarget,
            vec![Command::CardSelect(CardSelectCmd::FromCombatPile {
                exact_count: None,
                pile: PileName::Draw,
                prefs: SelectorPrefs::exactly(1),
                maximum: None,
                restricted_to_chosen: false,
                filter: CardFilter::Any,
                then: vec![Command::CardPile(CardPileCmd::MoveToPile {
                    cards: CardSelector::Chosen,
                    pile: PileName::Hand,
                    position: DrawPilePosition::Bottom,
                })],
            })],
        ),
        // A duplicator works its power on whoever drank it, not on a target.
        potion(
            "POTION.DUPLICATOR",
            PotionRarity::Uncommon,
            TargetType::SelfTarget,
            vec![applied_to_drinker("POWER.DUPLICATION_POWER", 1)],
        ),
        potion(
            "POTION.ENERGY_POTION",
            PotionRarity::Common,
            TargetType::AnyPlayer,
            vec![Command::Player(PlayerCmd::GainEnergy(Amount::Fixed(2)))],
        ),
        // An entropic brew fills whatever is left of the belt, one rolled potion
        // at a time, and stops when a procure fails. Every slot that was free
        // therefore costs exactly one roll.
        any_time(potion(
            "POTION.ENTROPIC_BREW",
            PotionRarity::Rare,
            TargetType::SelfTarget,
            vec![Command::Repeat {
                times: Amount::counting(Count::OpenPotionSlots),
                then: vec![Command::Potion(PotionCmd::TryToProcure(
                    PotionOrigin::RandomOutOfCombat,
                ))],
            }],
        )),
        potion(
            "POTION.EXPLOSIVE_AMPOULE",
            PotionRarity::Common,
            TargetType::AllEnemies,
            vec![Command::Creature(CreatureCmd::Damage {
                target: CreatureSelector::AllOpponents,
                amount: Amount::Fixed(10),
                props: ValueProps::UNPOWERED,
            })],
        ),
        // A fairy is never drunk on purpose. It refuses the death of its own
        // owner and nobody else, and having done so it is drunk where it
        // stands: the potion leaves the belt and heals whoever it saved by
        // three tenths of their maximum, never less than one.
        fairy_in_a_bottle(),
        // A fire potion's damage is its own: nothing scales it.
        potion(
            "POTION.FIRE_POTION",
            PotionRarity::Common,
            TargetType::AnyEnemy,
            vec![Command::Creature(CreatureCmd::Damage {
                target: CreatureSelector::Target,
                amount: Amount::Fixed(20),
                props: ValueProps::UNPOWERED,
            })],
        ),
        potion(
            "POTION.FLEX_POTION",
            PotionRarity::Common,
            TargetType::AnyPlayer,
            // The potion works the temporary power on and nothing else; the
            // strength comes from the power itself as it lands.
            vec![applied("POWER.FLEX_POTION_POWER", 5)],
        ),
        // A fortifier doubles whatever block its target is already standing
        // behind, and nothing scales the gain.
        potion(
            "POTION.FORTIFIER",
            PotionRarity::Uncommon,
            TargetType::AnyPlayer,
            vec![Command::Creature(CreatureCmd::GainBlock {
                target: CreatureSelector::Target,
                amount: Amount::per(0, 2, Count::TargetBlock),
                props: ValueProps::UNPOWERED,
            })],
        ),
        not_conjured_in_combat(any_time(potion(
            "POTION.FRUIT_JUICE",
            PotionRarity::Rare,
            TargetType::AnyPlayer,
            vec![Command::Creature(CreatureCmd::GainMaxHp {
                target: CreatureSelector::Target,
                amount: Amount::Fixed(5),
            })],
        ))),
        potion(
            "POTION.FYSH_OIL",
            PotionRarity::Uncommon,
            TargetType::AnyPlayer,
            vec![
                applied("POWER.STRENGTH_POWER", 1),
                applied("POWER.DEXTERITY_POWER", 1),
            ],
        ),
        // A gambler's brew lays the hand out, puts down whatever is picked, and
        // deals back exactly as many.
        potion(
            "POTION.GAMBLERS_BREW",
            PotionRarity::Uncommon,
            TargetType::SelfTarget,
            vec![Command::CardSelect(CardSelectCmd::FromHandForDiscard {
                maximum: None,
                exact_count: None,
                prefs: SelectorPrefs::between(0, 999_999_999),
                filter: CardFilter::Any,
                then: vec![Command::Card(CardCmd::DiscardAndDraw {
                    cards: CardSelector::Chosen,
                    count: Amount::counting(Count::ChosenCards),
                })],
            })],
        ),
        potion(
            "POTION.GIGANTIFICATION_POTION",
            PotionRarity::Rare,
            TargetType::AnyPlayer,
            vec![applied("POWER.GIGANTIFICATION_POWER", 1)],
        ),
        potion(
            "POTION.HEART_OF_IRON",
            PotionRarity::Uncommon,
            TargetType::AnyPlayer,
            vec![applied("POWER.PLATING_POWER", 7)],
        ),
        // Liquid bronze is thorns for whoever drinks it.
        potion(
            "POTION.LIQUID_BRONZE",
            PotionRarity::Uncommon,
            TargetType::AnyPlayer,
            vec![applied("POWER.THORNS_POWER", 3)],
        ),
        // Liquid memories lays the discard pile out and takes one card back,
        // free for the turn.
        potion(
            "POTION.LIQUID_MEMORIES",
            PotionRarity::Rare,
            TargetType::SelfTarget,
            vec![Command::CardSelect(CardSelectCmd::FromCombatPile {
                exact_count: None,
                pile: PileName::Discard,
                prefs: SelectorPrefs::exactly(1),
                maximum: None,
                restricted_to_chosen: false,
                filter: CardFilter::Any,
                then: vec![
                    Command::Card(CardCmd::SetTemporaryCost {
                        cards: CardSelector::Chosen,
                        lifetime: TemporaryCost::free_this_turn(),
                    }),
                    Command::CardPile(CardPileCmd::MoveToPile {
                        cards: CardSelector::Chosen,
                        pile: PileName::Hand,
                        position: DrawPilePosition::Bottom,
                    }),
                ],
            })],
        ),
        potion(
            "POTION.LUCKY_TONIC",
            PotionRarity::Rare,
            TargetType::AnyPlayer,
            vec![applied("POWER.BUFFER_POWER", 1)],
        ),
        potion(
            "POTION.MAZALETHS_GIFT",
            PotionRarity::Rare,
            TargetType::AnyPlayer,
            vec![applied("POWER.RITUAL_POWER", 1)],
        ),
        // Orobic acid rolls one attack, one skill, and one power out of the
        // drinker's own pool, each free for the turn.
        potion(
            "POTION.OROBIC_ACID",
            PotionRarity::Rare,
            TargetType::SelfTarget,
            [CardType::Attack, CardType::Skill, CardType::Power]
                .into_iter()
                .map(|card_type| {
                    Command::CardPile(CardPileCmd::AddGeneratedFromPool {
                        pool: sts2_engine::CardPoolRef::OwnCharacter,
                        count: Amount::Fixed(1),
                        distinct: true,
                        filter: CardFilter::OfType(card_type),
                        pile: PileName::Hand,
                        position: DrawPilePosition::Bottom,
                        excludes_source: false,
                        upgraded: false,
                        temporary_cost: Some(TemporaryCost::free_this_turn()),
                        ethereal: false,
                        rolled_one_at_a_time: false,
                    })
                })
                .collect(),
        ),
        // Binding lands on everyone still standing. The build reads its two
        // vars crosswise — the Weak application takes the Vulnerable var and
        // the Vulnerable application the Weak one — which is invisible because
        // both are one; the numbers are transcribed as the build reads them.
        potion(
            "POTION.POTION_OF_BINDING",
            PotionRarity::Uncommon,
            TargetType::AllEnemies,
            vec![
                applied_to_all("POWER.WEAK_POWER", 1),
                applied_to_all("POWER.VULNERABLE_POWER", 1),
            ],
        ),
        potion(
            "POTION.POWDERED_DEMISE",
            PotionRarity::Uncommon,
            TargetType::AnyEnemy,
            vec![applied("POWER.DEMISE_POWER", 9)],
        ),
        offers_a_card_of_type("POTION.POWER_POTION", CardType::Power),
        potion(
            "POTION.RADIANT_TINCTURE",
            PotionRarity::Uncommon,
            TargetType::AnyPlayer,
            vec![
                Command::Player(PlayerCmd::GainEnergy(Amount::Fixed(1))),
                applied("POWER.RADIANCE_POWER", 3),
            ],
        ),
        not_conjured_in_combat(potion(
            "POTION.REGEN_POTION",
            PotionRarity::Uncommon,
            TargetType::AnyPlayer,
            vec![applied("POWER.REGEN_POWER", 5)],
        )),
        // A shackling potion takes strength off everyone still standing until
        // the fight is over, which is the temporary power doing the work.
        potion(
            "POTION.SHACKLING_POTION",
            PotionRarity::Rare,
            TargetType::AllEnemies,
            vec![applied_to_all("POWER.SHACKLING_POTION_POWER", 7)],
        ),
        // A ship in a bottle is block now and the same again next turn.
        potion(
            "POTION.SHIP_IN_A_BOTTLE",
            PotionRarity::Rare,
            TargetType::AnyPlayer,
            vec![
                Command::Creature(CreatureCmd::GainBlock {
                    target: CreatureSelector::Target,
                    amount: Amount::Fixed(10),
                    props: ValueProps::UNPOWERED,
                }),
                applied("POWER.BLOCK_NEXT_TURN_POWER", 10),
            ],
        ),
        offers_a_card_of_type("POTION.SKILL_POTION", CardType::Skill),
        // Snecko oil deals seven and then rolls a fresh cost, nought to three,
        // for every card of the hand that has one to write over. One roll per
        // card, in hand order, on the combat energy-cost stream.
        potion(
            "POTION.SNECKO_OIL",
            PotionRarity::Rare,
            TargetType::AnyPlayer,
            vec![
                draw(7),
                Command::Card(CardCmd::RandomizeCost {
                    cards: CardSelector::Pile {
                        pile: PileName::Hand,
                        filter: CardFilter::HasWritableEnergyCost,
                    },
                    for_the_combat: false,
                }),
            ],
        ),
        potion(
            "POTION.SPEED_POTION",
            PotionRarity::Common,
            TargetType::AnyPlayer,
            vec![applied("POWER.SPEED_POTION_POWER", 5)],
        ),
        potion(
            "POTION.STABLE_SERUM",
            PotionRarity::Uncommon,
            TargetType::AnyPlayer,
            vec![applied("POWER.RETAIN_HAND_POWER", 2)],
        ),
        potion(
            "POTION.STRENGTH_POTION",
            PotionRarity::Common,
            TargetType::AnyPlayer,
            vec![applied("POWER.STRENGTH_POWER", 2)],
        ),
        potion(
            "POTION.SWIFT_POTION",
            PotionRarity::Common,
            TargetType::AnyPlayer,
            vec![draw(3)],
        ),
        // One card of the hand that costs anything is free for the rest of the
        // fight.
        potion(
            "POTION.TOUCH_OF_INSANITY",
            PotionRarity::Uncommon,
            TargetType::SelfTarget,
            vec![Command::CardSelect(CardSelectCmd::FromHand {
                maximum: None,
                exact_count: None,
                prefs: SelectorPrefs::between(1, 1),
                filter: CardFilter::CostsAnything,
                then: vec![Command::Card(CardCmd::SetTemporaryCost {
                    cards: CardSelector::Chosen,
                    lifetime: TemporaryCost::free_this_combat(),
                })],
            })],
        ),
        potion(
            "POTION.VULNERABLE_POTION",
            PotionRarity::Common,
            TargetType::AnyEnemy,
            vec![applied("POWER.VULNERABLE_POWER", 3)],
        ),
        potion(
            "POTION.WEAK_POTION",
            PotionRarity::Common,
            TargetType::AnyEnemy,
            vec![applied("POWER.WEAK_POWER", 3)],
        ),
    ]
}

/// A fairy in a bottle: never offered by the belt, and refusing the death of
/// its own owner alone, which is the only such refusal anything in this
/// baseline makes.
fn fairy_in_a_bottle() -> PotionDefinition {
    PotionDefinition {
        usage: PotionUsage::Automatic,
        can_be_generated_in_combat: false,
        prevents_owners_death: true,
        // Three tenths of the maximum, and never less than one.
        ..potion(
            "POTION.FAIRY_IN_A_BOTTLE",
            PotionRarity::Rare,
            TargetType::SelfTarget,
            vec![Command::Creature(CreatureCmd::Heal {
                target: CreatureSelector::Target,
                amount: Amount::Greatest(vec![
                    Amount::Percent {
                        percent: 30,
                        count: Count::TargetMaxHp,
                    },
                    Amount::Fixed(1),
                ]),
            })],
        )
    }
}

/// The event potions: the two an event hands over. Neither is ever rolled.
fn event_potions() -> Vec<PotionDefinition> {
    vec![
        // A foul potion splashes everyone in the fight but the pets, the
        // drinker included: both sides at once.
        PotionDefinition {
            // Outside a fight it aims at no creature and can only be used
            // where there is a merchant to throw it at: a real
            // one pays a hundred gold for it, and the fake one turns on the
            // player for it.
            thrown_at_a_merchant: Some(sts2_engine::MerchantThrow {
                gold: 100,
                event: id("EVENT.FAKE_MERCHANT"),
                encounter: id("ENCOUNTER.FAKE_MERCHANT_EVENT_ENCOUNTER"),
                reward_relic: id("RELIC.FAKE_MERCHANTS_RUG"),
            }),
            ..any_time(potion(
                "POTION.FOUL_POTION",
                PotionRarity::Event,
                TargetType::AllEnemies,
                vec![Command::Creature(CreatureCmd::Damage {
                    target: CreatureSelector::EveryCreature,
                    amount: Amount::Fixed(12),
                    props: ValueProps::UNPOWERED,
                })],
            ))
        },
        // Glowwater burns the whole hand and deals ten fresh cards.
        potion(
            "POTION.GLOWWATER_POTION",
            PotionRarity::Event,
            TargetType::SelfTarget,
            vec![
                Command::Card(CardCmd::Exhaust {
                    cards: CardSelector::Pile {
                        pile: PileName::Hand,
                        filter: CardFilter::Any,
                    },
                }),
                draw(10),
            ],
        ),
    ]
}

/// The potions in no pool a run reads: the token a relic hands over, the stand-in
/// for a potion the game no longer has, and the other four characters' twelve.
#[allow(
    clippy::too_many_lines,
    reason = "one entry per potion outside the pools"
)]
fn other_potions() -> Vec<PotionDefinition> {
    let mut potions = vec![
        potion(
            "POTION.POTION_SHAPED_ROCK",
            PotionRarity::Token,
            TargetType::AnyEnemy,
            vec![Command::Creature(CreatureCmd::Damage {
                target: CreatureSelector::Target,
                amount: Amount::Fixed(15),
                props: ValueProps::UNPOWERED,
            })],
        ),
        // The deprecated potion does nothing at all when drunk, which is what
        // a potion the game has removed comes to when a saved run still names
        // it.
        potion(
            "POTION.DEPRECATED_POTION",
            PotionRarity::None,
            TargetType::AnyEnemy,
            Vec::new(),
        ),
        // A ghost in a jar is one turn of intangibility, and nothing about it
        // is the Silent's: it is registered whole.
        potion(
            "POTION.GHOST_IN_A_JAR",
            PotionRarity::Rare,
            TargetType::AnyPlayer,
            vec![applied("POWER.INTANGIBLE_POWER", 1)],
        ),
        // A star potion is three stars.
        potion(
            "POTION.STAR_POTION",
            PotionRarity::Common,
            TargetType::SelfTarget,
            vec![Command::Player(sts2_engine::PlayerCmd::GainStars(
                Amount::Fixed(3),
            ))],
        ),
        // A king's courage forges fifteen for whoever it was poured over,
        // which in a single-player run is the player who drank it.
        potion(
            "POTION.KINGS_COURAGE",
            PotionRarity::Uncommon,
            TargetType::AnyPlayer,
            vec![Command::Forge(sts2_engine::ForgeCmd::Forge(Amount::Fixed(
                15,
            )))],
        ),
        // A cosmic concoction rolls three colorless cards, upgrades each, and
        // hands them over.
        potion(
            "POTION.COSMIC_CONCOCTION",
            PotionRarity::Rare,
            TargetType::SelfTarget,
            vec![Command::CardPile(CardPileCmd::AddGeneratedFromPool {
                pool: sts2_engine::CardPoolRef::Named(id("CARD_POOL.COLORLESS_CARD_POOL")),
                count: Amount::Fixed(3),
                distinct: true,
                filter: CardFilter::Any,
                pile: PileName::Hand,
                position: DrawPilePosition::Bottom,
                excludes_source: false,
                upgraded: true,
                temporary_cost: None,
                ethereal: false,
                rolled_one_at_a_time: false,
            })],
        ),
        // Three Shivs conjured straight into hand, each of them upgraded.
        potion(
            "POTION.CUNNING_POTION",
            PotionRarity::Uncommon,
            TargetType::SelfTarget,
            vec![Command::CardPile(CardPileCmd::AddGeneratedToCombat {
                enchantment: None,
                card: CardOrigin::Model(id("CARD.SHIV")),
                grown_by: Amount::Fixed(0),
                count: Amount::Fixed(3),
                pile: PileName::Hand,
                ethereal: false,
                retain: false,
                upgraded: true,
                temporary_cost: None,
                position: sts2_engine::DrawPilePosition::Bottom,
            })],
        ),
        // Six poison worked onto whatever it is thrown at.
        potion(
            "POTION.POISON_POTION",
            PotionRarity::Common,
            TargetType::AnyEnemy,
            vec![Command::Power(sts2_engine::PowerCmd::Apply {
                marks: None,
                power: id("POWER.POISON_POWER"),
                target: CreatureSelector::Target,
                applier: Some(CreatureSelector::Owner),
                amount: Amount::Fixed(6),
            })],
        ),
        // Thirty-three Doom worked onto whatever it is thrown at, which is a
        // countdown longer than anything at this baseline is standing on.
        potion(
            "POTION.POTION_OF_DOOM",
            PotionRarity::Common,
            TargetType::AnyEnemy,
            vec![Command::Power(sts2_engine::PowerCmd::Apply {
                marks: None,
                power: id("POWER.DOOM_POWER"),
                target: CreatureSelector::Target,
                applier: Some(CreatureSelector::Owner),
                amount: Amount::Fixed(33),
            })],
        ),
        // A bone brew is a summon for fifteen and nothing else.
        potion(
            "POTION.BONE_BREW",
            PotionRarity::Uncommon,
            TargetType::SelfTarget,
            vec![crate::necrobinder_cards::summon(15)],
        ),
        // Two Souls conjured straight into hand.
        potion(
            "POTION.POT_OF_GHOULS",
            PotionRarity::Rare,
            TargetType::SelfTarget,
            vec![Command::CardPile(CardPileCmd::AddGeneratedToCombat {
                enchantment: None,
                card: CardOrigin::Model(id("CARD.SOUL")),
                grown_by: Amount::Fixed(0),
                count: Amount::Fixed(2),
                pile: PileName::Hand,
                ethereal: false,
                retain: false,
                upgraded: false,
                temporary_cost: None,
                position: sts2_engine::DrawPilePosition::Bottom,
            })],
        ),
    ];
    // The Defect's three.
    potions.extend([
        // A Dark orb into every slot the queue has, filled or not: the count is
        // the capacity, taken before anything is channelled, so a full queue
        // evokes its way through itself.
        potion(
            "POTION.ESSENCE_OF_DARKNESS",
            PotionRarity::Rare,
            TargetType::SelfTarget,
            vec![Command::Repeat {
                times: Amount::counting(sts2_engine::Count::OrbSlots),
                then: vec![Command::Orb(sts2_engine::OrbCmd::Channel(
                    sts2_engine::OrbSource::Model(id("ORB.DARK_ORB")),
                ))],
            }],
        ),
        potion(
            "POTION.FOCUS_POTION",
            PotionRarity::Common,
            TargetType::SelfTarget,
            vec![Command::Power(PowerCmd::Apply {
                marks: None,
                power: id("POWER.FOCUS_POWER"),
                target: CreatureSelector::Owner,
                applier: Some(CreatureSelector::Owner),
                amount: Amount::Fixed(2),
            })],
        ),
        potion(
            "POTION.POTION_OF_CAPACITY",
            PotionRarity::Uncommon,
            TargetType::SelfTarget,
            vec![Command::Orb(sts2_engine::OrbCmd::AddSlots(Amount::Fixed(
                2,
            )))],
        ),
    ]);
    potions
}
