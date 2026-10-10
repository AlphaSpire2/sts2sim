//! The cards that belong to no character: statuses, curses, tokens, quests,
//! event cards, and the colorless pool.
//!
//! Each of them is its pinned model, base form and upgraded form where one is
//! defined, together with what the card declares about itself: what it
//! costs, what it counts as, what it may be aimed at, and which keywords it
//! carries. A card whose body needs a subsystem this baseline does not have is
//! registered for what it is and says so in its body, so that reaching it stops
//! rather than doing something the build does not.

use sts2_engine::{
    Amount, CardCmd, CardDefinition, CardFilter, CardPileCmd, CardPlayRestriction, CardRarity,
    CardSelector, CardTag, CardType, CardUpgrade, Command, Condition, Count, CreatureCmd,
    CreatureSelector, DrawPilePosition, PileName, PlayerCmd, PowerCmd, StunFollowUp, TargetType,
    ValueProps,
};
use sts2_engine::{
    ChoiceDestination, ChoiceDuration, ChoiceOperation as ChoiceOp, ChoiceOperationKind,
    ChoiceScope, ChoiceSemantics, ChoiceSubject,
};

use crate::cards::{
    apply_power, attack, block, buff, card, costing_x, debuff, draw, energy, eternal, ethereal,
    exhausting, held_at_turn_end, innate_when_upgraded, multi_attack, random_attack, rarity,
    striking, sweep, unplayable, upgrading, upgrading_cost, with_damage_var,
};
use crate::id;

/// The type and the rider a Tinker Time wrote onto a Mad Science, as the saved
/// properties the card carries them in. The build carries both as named
/// choices; they are written down here as the numbers it gives them.
pub(crate) const TINKER_TIME_TYPE: &str = "TinkerTimeType";
pub(crate) const TINKER_TIME_RIDER: &str = "TinkerTimeRider";

/// One rider a Tinker Time can hang off a Mad Science, by the number the build
/// gives it.
#[derive(Clone, Copy)]
pub(crate) enum Rider {
    Sapping = 1,
    Violence = 2,
    Choking = 3,
    Energized = 4,
    Wisdom = 5,
    Chaos = 6,
    Expertise = 7,
    Curious = 8,
    Improvement = 9,
}

/// The two saved properties a Mad Science carries, each starting at nothing:
/// the type at its None entry, which is zero, and the rider likewise. The minus
/// one the type carries in a save is the order the two are written in, not a
/// value. The rider is written only where it is set, and the tinker's table
/// always sets one.
fn tinkerable(mut definition: CardDefinition) -> CardDefinition {
    definition.saved_properties = vec![
        sts2_engine::SavedCardProperty::of(TINKER_TIME_TYPE, 0),
        sts2_engine::SavedCardProperty::of(TINKER_TIME_RIDER, 0),
    ];
    // A Mad Science's type is the property the tinker wrote, and what it can
    // be aimed at and whether it gains block both follow from that. Everything
    // that asks a Mad Science what it is reads the property rather than what
    // the card was made as.
    definition.type_from_property = Some(TINKER_TIME_TYPE.to_owned());
    definition
}

/// Tests one of the two properties the tinker writes against a value.
fn tinkered(name: &str, value: i32) -> Condition {
    Condition::SourceCardPropertyIs {
        name: name.to_owned(),
        value,
    }
}

fn is_rider(rider: Rider) -> Condition {
    tinkered(TINKER_TIME_RIDER, rider as i32)
}

/// A card type by the number the build gives it, which counts from a None
/// entry rather than from Attack.
const fn card_type_ordinal(card_type: CardType) -> i32 {
    match card_type {
        CardType::Attack => 1,
        CardType::Skill => 2,
        CardType::Power => 3,
        CardType::Status => 4,
        CardType::Curse => 5,
        CardType::Quest => 6,
    }
}

/// The test a Mad Science's body opens each of its branches with.
fn when(condition: Condition, then: Vec<Command>) -> Command {
    Command::When {
        condition,
        then,
        otherwise: Vec::new(),
    }
}

/// Mad Science, played: the base effect its type declares, and then the rider
/// the tinker's table hung off it.
///
/// The type switch comes first — an attack for twelve, eight block, or a power
/// that is nothing but its own rider — and the riders that are not folded into
/// one of those three run behind it. Violence is folded into the attack's own
/// hit count; Expertise, Curious and Improvement are the whole of what the
/// power branch does.
fn mad_science() -> Vec<Command> {
    let chaos = Command::CardPile(CardPileCmd::AddGeneratedFromPool {
        pool: sts2_engine::CardPoolRef::OwnCharacter,
        count: Amount::Fixed(1),
        distinct: true,
        filter: CardFilter::Any,
        pile: PileName::Hand,
        position: DrawPilePosition::Bottom,
        excludes_source: false,
        upgraded: false,
        temporary_cost: Some(sts2_engine::TemporaryCost::free_this_turn()),
        ethereal: false,
        rolled_one_at_a_time: false,
    });
    vec![
        when(
            tinkered(TINKER_TIME_TYPE, card_type_ordinal(CardType::Attack)),
            vec![multi_attack(
                Amount::Fixed(12),
                Amount::IfElse {
                    condition: is_rider(Rider::Violence),
                    then: Box::new(Amount::Fixed(3)),
                    otherwise: Box::new(Amount::Fixed(1)),
                },
            )],
        ),
        when(
            tinkered(TINKER_TIME_TYPE, card_type_ordinal(CardType::Skill)),
            vec![block(8)],
        ),
        when(
            tinkered(TINKER_TIME_TYPE, card_type_ordinal(CardType::Power)),
            vec![
                when(
                    is_rider(Rider::Expertise),
                    vec![
                        buff("POWER.STRENGTH_POWER", 2),
                        buff("POWER.DEXTERITY_POWER", 2),
                    ],
                ),
                when(
                    is_rider(Rider::Curious),
                    vec![buff("POWER.CURIOUS_POWER", 1)],
                ),
                when(
                    is_rider(Rider::Improvement),
                    vec![buff("POWER.IMPROVEMENT_POWER", 1)],
                ),
            ],
        ),
        // The four riders behind the type switch, and Sapping.
        when(
            is_rider(Rider::Sapping),
            vec![
                debuff("POWER.WEAK_POWER", 2),
                debuff("POWER.VULNERABLE_POWER", 2),
            ],
        ),
        when(
            is_rider(Rider::Choking),
            vec![debuff("POWER.STRANGLE_POWER", 6)],
        ),
        when(
            is_rider(Rider::Energized),
            vec![Command::Player(PlayerCmd::GainEnergy(Amount::Fixed(2)))],
        ),
        when(is_rider(Rider::Wisdom), vec![draw(3)]),
        when(is_rider(Rider::Chaos), vec![chaos]),
    ]
}

/// Shiv, played: one attack, thrown at everything while its owner has a fan
/// of knives and at whatever it was aimed at otherwise.
fn shiv(damage: i32) -> Vec<Command> {
    vec![Command::When {
        condition: sts2_engine::Condition::Not(Box::new(sts2_engine::Condition::OwnerLacksPower(
            id("POWER.FAN_OF_KNIVES_POWER"),
        ))),
        then: vec![crate::cards::sweep(damage, 1)],
        otherwise: vec![attack(damage)],
    }]
}

/// A card whose target type widens to the whole enemy side while its owner
/// carries a power.
fn widening_with(mut definition: CardDefinition, power: &str) -> CardDefinition {
    definition.target_widens_with = Some(id(power));
    definition
}

/// What a card counts as when a deck is sorted by rarity. The character pools
/// hand that out from the pool table; the shared families each have one rarity
/// of their own, except the colorless pool, which carries the rolled three.
/// The lantern key: a quest card that cannot be played and answers for the
/// last act. While it is in the deck there, every unknown map point is an
/// event, and the next event the act pulls is the war historian.
fn lantern_key() -> CardDefinition {
    CardDefinition {
        quest_hooks: Some(sts2_engine::QuestHooks {
            // `_gloryActIndex = 2`.
            act: 2,
            unknown_room_type: sts2_engine::ResolvedRoom::Event,
            next_event: id("EVENT.WAR_HISTORIAN_REPY"),
        }),
        ..quest("CARD.LANTERN_KEY", TargetType::SelfTarget)
    }
}

/// Retain: the card is not put down when the hand is flushed.
fn retaining(mut definition: CardDefinition) -> CardDefinition {
    definition.retain = true;
    definition
}

/// Innate: the card is in the opening hand of every fight.
fn innate(mut definition: CardDefinition) -> CardDefinition {
    definition.innate = true;
    definition
}

/// Damage a card works on its own holder that block cannot stop and that
/// nothing scales.
fn unblockable_self_damage(amount: Amount) -> Command {
    Command::Creature(CreatureCmd::Damage {
        target: CreatureSelector::Owner,
        amount,
        props: ValueProps::UNBLOCKABLE,
    })
}

/// Damage a status works on its own holder that block still stops. It is dealt
/// as the card's own damage value, which powers do not scale.
fn blockable_self_damage(amount: i32) -> Command {
    Command::Creature(CreatureCmd::Damage {
        target: CreatureSelector::Owner,
        amount: Amount::Fixed(amount),
        props: ValueProps::UNPOWERED,
    })
}

/// The status cards an enemy forces into a deck.
///
/// Four of them — Dazed, Slimed, Toxic, and Wound — arrived with the content
/// the recorded runs walk and live in `cards.rs` beside it.
pub(crate) fn status_cards() -> Vec<CardDefinition> {
    vec![
        // A beckon is playable and does nothing; what it costs its holder is
        // six HP if the turn ends with it still in hand, which block does not
        // stop.
        rarity(
            held_at_turn_end(
                status("CARD.BECKON", 1),
                vec![unblockable_self_damage(Amount::Fixed(6))],
            ),
            CardRarity::Status,
        ),
        // A burn cannot be played and burns its holder for two at the end of
        // the turn. Block stops it: the build passes the card's damage var.
        rarity(
            held_at_turn_end(
                unplayable(status("CARD.BURN", -1)),
                vec![blockable_self_damage(2)],
            ),
            CardRarity::Status,
        ),
        // Debris is a playable nothing that burns up for having been played.
        rarity(exhausting(status("CARD.DEBRIS", 1)), CardRarity::Status),
        // An infection is a burn worth three.
        rarity(
            held_at_turn_end(
                unplayable(status("CARD.INFECTION", -1)),
                vec![blockable_self_damage(3)],
            ),
            CardRarity::Status,
        ),
        // Wither is another, and opens on the same three — but the queen grows
        // every one of them by three each time she works her intensity up, so
        // what it deals is its own damage var rather than a written number.
        rarity(
            held_at_turn_end(
                with_damage_var(unplayable(status("CARD.WITHER", -1)), 3, 3),
                vec![Command::Creature(CreatureCmd::Damage {
                    target: CreatureSelector::Owner,
                    amount: Amount::counting(Count::SourceCardDamage),
                    props: ValueProps::UNPOWERED,
                })],
            ),
            CardRarity::Status,
        ),
        // Soot is a card and nothing else: no play, no keyword past being
        // unplayable, and no body at all.
        rarity(unplayable(status("CARD.SOOT", -1)), CardRarity::Status),
        // A void cannot be played, burns up if it is still held at the end of
        // the turn, and takes a point of energy the moment it is drawn.
        rarity(
            drawing_listener(
                ethereal(unplayable(status("CARD.VOID", -1))),
                vec![Command::Player(PlayerCmd::LoseEnergy(Amount::Fixed(1)))],
            ),
            CardRarity::Status,
        ),
        // A frantic escape feeds the sandpit that made it and grows dearer for
        // the rest of the fight.
        rarity(
            card(
                "CARD.FRANTIC_ESCAPE",
                CardType::Status,
                TargetType::SelfTarget,
                1,
                vec![
                    // The first enemy carrying a sandpit has it raised by
                    // one: the pit gives a turn back.
                    Command::Power(sts2_engine::PowerCmd::ModifyAmount {
                        power: sts2_engine::PowerRef {
                            owner: CreatureSelector::EnemyCarryingThePower,
                            power: sts2_engine::PowerName::Model(id("POWER.SANDPIT_POWER")),
                        },
                        amount: Amount::Fixed(1),
                    }),
                    // The card costs one more for the rest of the fight,
                    // whether or not it found a pit.
                    Command::Card(sts2_engine::CardCmd::AddCostThisCombat {
                        cards: CardSelector::Source,
                        amount: Amount::Fixed(1),
                    }),
                ],
            ),
            CardRarity::Status,
        ),
    ]
}

/// A status card, which is a card of that type with nothing else declared.
fn status(model_id: &str, energy_cost: i32) -> CardDefinition {
    card(
        model_id,
        CardType::Status,
        TargetType::None,
        energy_cost,
        Vec::new(),
    )
}

fn curse(model_id: &str, energy_cost: i32) -> CardDefinition {
    card(
        model_id,
        CardType::Curse,
        TargetType::None,
        energy_cost,
        Vec::new(),
    )
}

/// A card that answers for its own draw.
fn drawing_listener(mut definition: CardDefinition, commands: Vec<Command>) -> CardDefinition {
    definition.hooks.push(
        sts2_engine::ModifierHook::every_turn(sts2_engine::HookEvent::AfterCardDrawn, commands)
            .when(sts2_engine::HookCondition::ListeningCardDrawn),
    );
    definition
}

/// The curses an event, a relic, or an ascension charges a deck.
///
/// Clumsy and Greed arrived with the recorded runs and live in `cards.rs`.
#[allow(clippy::too_many_lines, reason = "one entry per curse")]
pub(crate) fn curse_cards() -> Vec<CardDefinition> {
    vec![
        // The ascender's bane cannot be played, cannot be removed, and burns up
        // if the turn ends with it still in hand.
        rarity(
            ethereal(eternal(unplayable(curse("CARD.ASCENDERS_BANE", -1)))),
            CardRarity::Curse,
        ),
        // Bad luck cannot be removed and costs thirteen HP that block does not
        // stop if the turn ends with it in hand.
        rarity(
            held_at_turn_end(
                eternal(unplayable(curse("CARD.BAD_LUCK", -1))),
                vec![unblockable_self_damage(Amount::Fixed(13))],
            ),
            CardRarity::Curse,
        ),
        // The curse of the bell is a card that does nothing at all and cannot
        // be taken out of the deck. What it is for is elsewhere.
        rarity(
            eternal(unplayable(curse("CARD.CURSE_OF_THE_BELL", -1))),
            CardRarity::Curse,
        ),
        // A debt takes ten gold at the end of a turn it is held through, or
        // whatever is left of the purse if there is less than that in it.
        rarity(
            held_at_turn_end(
                unplayable(curse("CARD.DEBT", -1)),
                vec![Command::Player(PlayerCmd::LoseGold(Amount::Fixed(10)))],
            ),
            CardRarity::Curse,
        ),
        // Decay is a burn that sorts as a curse.
        rarity(
            held_at_turn_end(
                unplayable(curse("CARD.DECAY", -1)),
                vec![blockable_self_damage(2)],
            ),
            CardRarity::Curse,
        ),
        // Doubt weakens its holder for a stack at the end of the turn, worked
        // by nobody, and the stack it stands up lasts through the turn it
        // landed on.
        rarity(
            held_at_turn_end(
                unplayable(curse("CARD.DOUBT", -1)),
                vec![unattributed_debuff("POWER.WEAK_POWER", 1)],
            ),
            CardRarity::Curse,
        ),
        // Enthralled is the only card its holder may choose to play while it is
        // in hand, and it can never be removed from the deck.
        rarity(
            restricted(
                eternal(curse("CARD.ENTHRALLED", 2)),
                CardPlayRestriction::OnlyItself,
            ),
            CardRarity::Curse,
        ),
        // A folly is dealt into the opening hand, cannot be played, burns up at
        // the end of the turn, and cannot be removed.
        rarity(
            innate(ethereal(eternal(unplayable(curse("CARD.FOLLY", -1))))),
            CardRarity::Curse,
        ),
        // Guilt counts the fights the run has seen and leaves the deck after
        // five of them.
        rarity(guilty(), CardRarity::Curse),
        // An injury is a card that does nothing.
        rarity(unplayable(curse("CARD.INJURY", -1)), CardRarity::Curse),
        // Normality allows three card plays in a turn and no more.
        rarity(
            restricted(
                unplayable(curse("CARD.NORMALITY", -1)),
                CardPlayRestriction::CardsEachTurn(3),
            ),
            CardRarity::Curse,
        ),
        // Poor sleep cannot be played and is never put down.
        rarity(
            retaining(unplayable(curse("CARD.POOR_SLEEP", -1))),
            CardRarity::Curse,
        ),
        // A regret costs its holder one HP for every card in the hand it was
        // held through, counted before the turn's end took anything out of it.
        rarity(regret(), CardRarity::Curse),
        // Shame is Doubt with frailty.
        rarity(
            held_at_turn_end(
                unplayable(curse("CARD.SHAME", -1)),
                vec![unattributed_debuff("POWER.FRAIL_POWER", 1)],
            ),
            CardRarity::Curse,
        ),
        // A spore mind is playable, does nothing, and burns up for having been
        // played. Exhaust is the whole of what it declares.
        rarity(exhausting(curse("CARD.SPORE_MIND", 1)), CardRarity::Curse),
        // A writhe is dealt into the opening hand and cannot be played.
        rarity(
            innate(unplayable(curse("CARD.WRITHE", -1))),
            CardRarity::Curse,
        ),
    ]
}

/// A debuff a card works on its own holder with no applier, which is the null
/// the build passes: nothing takes it away by dying, and the stack it stands up
/// skips the tick that would otherwise end it on the turn it landed.
fn unattributed_debuff(power: &str, amount: i32) -> Command {
    Command::Power(PowerCmd::Apply {
        marks: None,
        power: id(power),
        target: CreatureSelector::Owner,
        applier: None,
        amount: Amount::Fixed(amount),
    })
}

/// Splash, played: three attacks rolled out of every character pool but the
/// walking one's, one of them taken, and what is taken made free for the turn
/// and dealt into the hand.
///
/// The upgrade upgrades the three before the screen goes up rather than
/// upgrading whichever one is taken.
fn splash(upgraded: bool) -> Vec<Command> {
    vec![Command::CardSelect(
        sts2_engine::CardSelectCmd::ChooseAGeneratedCard {
            semantics: ChoiceSemantics::known(vec![
                ChoiceOp::new(ChoiceOperationKind::Gain, ChoiceScope::Combat)
                    .to(ChoiceDestination::Hand),
                ChoiceOp::new(ChoiceOperationKind::SetCost, ChoiceScope::Combat)
                    .amount(0)
                    .lasting(ChoiceDuration::UntilPlayedOrTurnEnds)
                    .including_stars(),
            ]),
            pool: sts2_engine::CardPoolRef::EveryOtherCharacter,
            count: 3,
            filter: CardFilter::OfType(CardType::Attack),
            upgraded,
            can_skip: true,
            then: vec![Command::CardPile(CardPileCmd::AddGeneratedToCombat {
                enchantment: None,
                card: sts2_engine::CardOrigin::Chosen,
                grown_by: Amount::Fixed(0),
                count: Amount::Fixed(1),
                pile: PileName::Hand,
                ethereal: false,
                retain: false,
                upgraded: false,
                temporary_cost: Some(sts2_engine::TemporaryCost::free_this_turn()),
                position: sts2_engine::DrawPilePosition::Bottom,
            })],
        },
    )]
}

fn restricted(mut definition: CardDefinition, restriction: CardPlayRestriction) -> CardDefinition {
    definition.play_restriction = Some(restriction);
    definition
}

/// Guilty, as a fight ends: the deck's own card counts that fight, and leaves
/// the deck once it has counted five. The fight's copy of it counts nothing,
/// because the count is only taken for a card standing in the deck.
fn guilty() -> CardDefinition {
    let mut definition = unplayable(curse("CARD.GUILTY", -1));
    definition.saved_properties = vec![sts2_engine::SavedCardProperty::of("CombatsSeen", 0)];
    definition.hooks.push(sts2_engine::ModifierHook::every_turn(
        sts2_engine::HookEvent::AfterCombatEnd,
        vec![Command::When {
            otherwise: Vec::new(),
            condition: Condition::SourceCardIsInTheDeck,
            then: vec![
                Command::Card(CardCmd::ModifyProperty {
                    cards: CardSelector::Source,
                    name: "CombatsSeen".to_owned(),
                    amount: Amount::Fixed(1),
                    and_the_deck_version: false,
                }),
                Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::SourceCardPropertyAtLeast {
                        name: "CombatsSeen".to_owned(),
                        value: 5,
                    },
                    then: vec![Command::CardPile(CardPileCmd::RemoveFromDeck {
                        cards: CardSelector::Source,
                    })],
                },
            ],
        }],
    ));
    definition
}

/// Regret: the hand it was held in is counted as the turn begins to end, and
/// that is what it costs its holder — read before the hand acts, because by the
/// time a regret's own body runs it has already left the hand for the play
/// pile.
fn regret() -> CardDefinition {
    let mut definition = held_at_turn_end(
        unplayable(curse("CARD.REGRET", -1)),
        vec![
            unblockable_self_damage(Amount::counting(Count::SourceCardLocal(
                "CardsInHand".to_owned(),
            ))),
            Command::Card(CardCmd::SetLocal {
                cards: CardSelector::Source,
                name: "CardsInHand".to_owned(),
                amount: Amount::Fixed(0),
            }),
        ],
    );
    definition.hooks.push(
        sts2_engine::ModifierHook::every_turn(
            sts2_engine::HookEvent::BeforeSideTurnEnd,
            vec![Command::When {
                otherwise: Vec::new(),
                condition: Condition::SourceCardIsInPile(PileName::Hand),
                then: vec![Command::Card(CardCmd::SetLocal {
                    cards: CardSelector::Source,
                    name: "CardsInHand".to_owned(),
                    amount: Amount::counting(Count::CardsInPile(PileName::Hand)),
                })],
            }],
        )
        .when(sts2_engine::HookCondition::OwnerIsParticipant),
    );
    definition
}

/// The cards a body conjures into a fight and nothing else ever holds.
///
/// Disintegration, Giant Rock, Mind Rot, Sloth, and Waste Away arrived with the
/// content the recorded runs walk and live in `cards.rs`.
#[allow(clippy::too_many_lines, reason = "one entry per token")]
pub(crate) fn token_cards() -> Vec<CardDefinition> {
    vec![
        // Fuel is a point of energy and a card, and burns for having been
        // played.
        rarity(
            upgrading(
                exhausting(card(
                    "CARD.FUEL",
                    CardType::Skill,
                    TargetType::SelfTarget,
                    0,
                    vec![energy(1), draw(1)],
                )),
                vec![energy(1), draw(2)],
            ),
            CardRarity::Token,
        ),
        // Luminesce is two points of energy, and it is never put down.
        rarity(
            upgrading(
                retaining(exhausting(card(
                    "CARD.LUMINESCE",
                    CardType::Skill,
                    TargetType::SelfTarget,
                    0,
                    vec![energy(2)],
                ))),
                vec![energy(3)],
            ),
            CardRarity::Token,
        ),
        // The three cards a summoned minion leaves behind.
        rarity(
            tagged(
                upgrading(
                    exhausting(card(
                        "CARD.MINION_DIVE_BOMB",
                        CardType::Attack,
                        TargetType::AnyEnemy,
                        0,
                        vec![attack(13)],
                    )),
                    vec![attack(16)],
                ),
                &[CardTag::Minion],
            ),
            CardRarity::Token,
        ),
        rarity(
            tagged(
                upgrading(
                    exhausting(card(
                        "CARD.MINION_SACRIFICE",
                        CardType::Skill,
                        TargetType::SelfTarget,
                        0,
                        vec![block(8)],
                    )),
                    vec![block(11)],
                ),
                &[CardTag::Minion],
            ),
            CardRarity::Token,
        ),
        rarity(
            tagged(
                upgrading(
                    exhausting(card(
                        "CARD.MINION_STRIKE",
                        CardType::Attack,
                        TargetType::AnyEnemy,
                        0,
                        vec![attack(6), draw(1)],
                    )),
                    vec![attack(9), draw(1)],
                ),
                &[CardTag::Strike, CardTag::Minion],
            ),
            CardRarity::Token,
        ),
        // A shiv is four damage and gone. Its target type widens to the whole
        // enemy side while its owner carries a fan of knives, and its body
        // branches the same way: it builds one attack and points it at
        // everything or at the one thing it was aimed at.
        rarity(
            widening_with(
                tagged(
                    upgrading(
                        exhausting(card(
                            "CARD.SHIV",
                            CardType::Attack,
                            TargetType::AnyEnemy,
                            0,
                            shiv(4),
                        )),
                        shiv(6),
                    ),
                    &[CardTag::Shiv],
                ),
                "POWER.FAN_OF_KNIVES_POWER",
            ),
            CardRarity::Token,
        ),
        // A soul is two cards and gone.
        rarity(
            upgrading(
                exhausting(card(
                    "CARD.SOUL",
                    CardType::Skill,
                    TargetType::SelfTarget,
                    0,
                    vec![draw(2)],
                )),
                vec![draw(3)],
            ),
            CardRarity::Token,
        ),
        // The Regent's blade, which the forge stands up and sharpens. It swings
        // for what its own damage var has been grown to, and works up block
        // behind the blow for as much parry as its owner is standing on. Its
        // target type is worked out rather than declared: a seeking edge sends
        // it at the whole enemy side.
        rarity(
            widening_with(
                with_damage_var(
                    upgrading_cost(
                        retaining(card(
                            "CARD.SOVEREIGN_BLADE",
                            CardType::Attack,
                            TargetType::AnyEnemy,
                            2,
                            crate::regent_cards::sovereign_blade(),
                        )),
                        1,
                    ),
                    10,
                    10,
                ),
                "POWER.SEEKING_EDGE_POWER",
            ),
            CardRarity::Token,
        ),
        // The Necrobinder's pet swings this one, at whoever the roll picks.
        rarity(
            tagged(
                upgrading(
                    ethereal(exhausting(card(
                        "CARD.SWEEPING_GAZE",
                        CardType::Attack,
                        TargetType::RandomEnemy,
                        0,
                        vec![crate::necrobinder_cards::osty_sweep(10)],
                    ))),
                    vec![crate::necrobinder_cards::osty_sweep(15)],
                ),
                &[CardTag::OstyAttack],
            ),
            CardRarity::Token,
        ),
        // The placeholder the loader hands back for a card it cannot find: a
        // card, and then the deck card it stood for taken out of the deck.
        rarity(
            exhausting(card(
                "CARD.DEPRECATED_CARD",
                CardType::Status,
                TargetType::None,
                0,
                vec![
                    draw(1),
                    Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::SourceCardCameFromTheDeck,
                        then: vec![Command::CardPile(CardPileCmd::RemoveFromDeck {
                            cards: CardSelector::Source,
                        })],
                    },
                ],
            )),
            CardRarity::Status,
        ),
    ]
}

/// The quest cards a run carries as errands rather than as plays.
///
/// All three are unplayable cards whose whole behaviour is outside a combat:
/// one adds an option to a rest site, one bends the last act's map and the
/// event it leads to, and one plants a treasure on a map it has re-generated.
/// Only the first of those surfaces exists here, so the other two are
/// registered for what they are and carry nothing else.
pub(crate) fn quest_cards() -> Vec<CardDefinition> {
    let mut spoils_map = quest("CARD.SPOILS_MAP", TargetType::SelfTarget);
    // A spoils map is made with its act index already set to one, rather than
    // with the unset value the property would otherwise start at.
    spoils_map.saved_properties = vec![sts2_engine::SavedCardProperty::of("SpoilsActIndex", 1)];
    // While it stands in the deck the act it names is laid out as the spoils
    // map instead of its own, the first treasure point of that map is marked
    // for it, and walking onto that point pays six hundred and takes the card
    // out of the deck.
    spoils_map.map_hooks = Some(sts2_engine::CardMapHooks {
        act_property: "SpoilsActIndex".to_owned(),
        replaces_the_map_with: sts2_engine::SpecialMap::Spoils,
        marks_the_first_treasure: true,
        gold_for_the_marked_point: 600,
    });
    let mut egg = quest("CARD.BYRDONIS_EGG", TargetType::None);
    // The egg adds a hatch option to a rest site, which hands its owner the
    // byrdpip the egg was holding.
    egg.adds_rest_site_option = Some(sts2_engine::RestSiteOffer::of(
        sts2_engine::RestSiteOption::Hatch,
    ));
    vec![
        rarity(egg, CardRarity::Quest),
        rarity(lantern_key(), CardRarity::Quest),
        rarity(spoils_map, CardRarity::Quest),
    ]
}

fn quest(model_id: &str, target: TargetType) -> CardDefinition {
    unplayable(card(model_id, CardType::Quest, target, -1, Vec::new()))
}

fn tagged(mut definition: CardDefinition, tags: &[CardTag]) -> CardDefinition {
    definition.tags = tags.to_vec();
    definition
}

/// The cards only an event hands over, plus the ancient-rarity cards among
/// them.
///
/// The build's event pool holds twenty-seven cards, eight of which carry the
/// ancient rarity rather than the event one. Neither rarity is ever rolled:
/// both are filtered out of any roll made in a fight, so nothing in a fight can
/// conjure one of these.
#[allow(clippy::too_many_lines, reason = "one entry per event card")]
pub(crate) fn event_cards() -> Vec<CardDefinition> {
    vec![
        // An apotheosis upgrades every other card of the fight and burns.
        rarity(
            upgrading_cost(
                innate(exhausting(card(
                    "CARD.APOTHEOSIS",
                    CardType::Skill,
                    TargetType::SelfTarget,
                    2,
                    vec![Command::Card(CardCmd::Upgrade {
                        cards: CardSelector::OtherCombatCards {
                            filter: CardFilter::Upgradable,
                        },
                    })],
                ))),
                1,
            ),
            CardRarity::Ancient,
        ),
        // An apparition holds every blow down to one for a turn. Upgrading
        // stops it burning up in hand.
        rarity(
            keeps_ethereal_when_upgraded(ethereal(exhausting(card(
                "CARD.APPARITION",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.INTANGIBLE_POWER", 1)],
            )))),
            CardRarity::Ancient,
        ),
        // The brightest flame pays a point of maximum HP for energy and cards.
        rarity(
            upgrading(
                card(
                    "CARD.BRIGHTEST_FLAME",
                    CardType::Skill,
                    TargetType::SelfTarget,
                    0,
                    brightest_flame(2, 2),
                ),
                brightest_flame(3, 3),
            ),
            CardRarity::Ancient,
        ),
        // A byrd's swoop is one swing and nothing else.
        rarity(
            upgrading(
                card(
                    "CARD.BYRD_SWOOP",
                    CardType::Attack,
                    TargetType::AnyEnemy,
                    0,
                    vec![attack(14)],
                ),
                vec![attack(18)],
            ),
            CardRarity::Event,
        ),
        // Caltrops are thorns.
        rarity(
            upgrading(
                card(
                    "CARD.CALTROPS",
                    CardType::Power,
                    TargetType::SelfTarget,
                    1,
                    vec![buff("POWER.THORNS_POWER", 3)],
                ),
                vec![buff("POWER.THORNS_POWER", 5)],
            ),
            CardRarity::Event,
        ),
        // A clash may only be played out of a hand of nothing but attacks.
        rarity(
            playable_when(
                upgrading(
                    card(
                        "CARD.CLASH",
                        CardType::Attack,
                        TargetType::AnyEnemy,
                        0,
                        vec![attack(14)],
                    ),
                    vec![attack(18)],
                ),
                Condition::EveryCardInPileIsOfType {
                    pile: PileName::Hand,
                    card_type: CardType::Attack,
                },
            ),
            CardRarity::Event,
        ),
        // A distraction rolls one skill of its owner's own pool into the hand,
        // free for the turn.
        rarity(
            upgrading_cost(
                exhausting(card(
                    "CARD.DISTRACTION",
                    CardType::Skill,
                    TargetType::SelfTarget,
                    1,
                    vec![Command::CardPile(CardPileCmd::AddGeneratedFromPool {
                        pool: sts2_engine::CardPoolRef::OwnCharacter,
                        count: Amount::Fixed(1),
                        distinct: true,
                        filter: CardFilter::OfType(CardType::Skill),
                        pile: PileName::Hand,
                        position: DrawPilePosition::Bottom,
                        excludes_source: false,
                        upgraded: false,
                        temporary_cost: Some(sts2_engine::TemporaryCost::free_this_turn()),
                        ethereal: false,
                        rolled_one_at_a_time: false,
                    })],
                )),
                0,
            ),
            CardRarity::Event,
        ),
        // A dual wield copies an attack or a power out of the hand.
        rarity(
            upgrading(
                card(
                    "CARD.DUAL_WIELD",
                    CardType::Skill,
                    TargetType::SelfTarget,
                    1,
                    dual_wield(1),
                ),
                dual_wield(2),
            ),
            CardRarity::Event,
        ),
        // Enlightenment makes the hand cost one, this turn or this fight.
        rarity(
            upgrading(
                exhausting(card(
                    "CARD.ENLIGHTENMENT",
                    CardType::Skill,
                    TargetType::SelfTarget,
                    0,
                    vec![Command::Card(CardCmd::SetTemporaryCost {
                        cards: CardSelector::Pile {
                            pile: PileName::Hand,
                            filter: CardFilter::Any,
                        },
                        lifetime: sts2_engine::TemporaryCost::this_turn_reduce_only(1),
                    })],
                )),
                vec![Command::Card(CardCmd::SetTemporaryCost {
                    cards: CardSelector::Pile {
                        pile: PileName::Hand,
                        filter: CardFilter::Any,
                    },
                    lifetime: sts2_engine::TemporaryCost::this_combat_reduce_only(1),
                })],
            ),
            CardRarity::Event,
        ),
        // An entrench doubles the block its owner is already standing behind.
        rarity(
            upgrading_cost(
                card(
                    "CARD.ENTRENCH",
                    CardType::Skill,
                    TargetType::SelfTarget,
                    2,
                    vec![Command::Creature(CreatureCmd::GainBlock {
                        target: CreatureSelector::Owner,
                        amount: Amount::counting(Count::OwnerBlock),
                        props: ValueProps::UNPOWERED,
                    })],
                ),
                1,
            ),
            CardRarity::Event,
        ),
        // Exterminate sweeps four times.
        rarity(
            upgrading(
                card(
                    "CARD.EXTERMINATE",
                    CardType::Attack,
                    TargetType::AllEnemies,
                    1,
                    vec![sweep(3, 4)],
                ),
                vec![sweep(4, 4)],
            ),
            CardRarity::Event,
        ),
        // A feeding frenzy lends strength for the turn.
        rarity(
            upgrading(
                card(
                    "CARD.FEEDING_FRENZY",
                    CardType::Skill,
                    TargetType::SelfTarget,
                    0,
                    vec![buff("POWER.FEEDING_FRENZY_POWER", 5)],
                ),
                vec![buff("POWER.FEEDING_FRENZY_POWER", 7)],
            ),
            CardRarity::Event,
        ),
        // Hello world stocks the hand with a common card every turn.
        rarity(
            innate_when_upgraded(card(
                "CARD.HELLO_WORLD",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.HELLO_WORLD_POWER", 1)],
            )),
            CardRarity::Event,
        ),
        // Mad science is whatever the tinker's table made of it.
        rarity(
            tinkerable(innate_when_upgraded(card(
                "CARD.MAD_SCIENCE",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                mad_science(),
            ))),
            CardRarity::Event,
        ),
        // A maul grows every maul in the fight, itself included.
        rarity(
            with_damage_var(
                upgrading(
                    card(
                        "CARD.MAUL",
                        CardType::Attack,
                        TargetType::AnyEnemy,
                        1,
                        maul(1),
                    ),
                    maul(2),
                ),
                5,
                6,
            ),
            CardRarity::Ancient,
        ),
        // A metamorphosis shuffles free attacks of its owner's own pool into
        // the draw pile.
        rarity(
            upgrading(
                exhausting(card(
                    "CARD.METAMORPHOSIS",
                    CardType::Skill,
                    TargetType::SelfTarget,
                    2,
                    vec![metamorphosis(3)],
                )),
                vec![metamorphosis(5)],
            ),
            CardRarity::Event,
        ),
        // Neow's fury swings and then takes back what the hand has room for.
        rarity(
            upgrading(
                exhausting(card(
                    "CARD.NEOWS_FURY",
                    CardType::Attack,
                    TargetType::AnyEnemy,
                    1,
                    neows_fury(10, 2),
                )),
                neows_fury(14, 3),
            ),
            CardRarity::Ancient,
        ),
        // An outmaneuver is energy owed next turn.
        rarity(
            upgrading(
                card(
                    "CARD.OUTMANEUVER",
                    CardType::Skill,
                    TargetType::SelfTarget,
                    1,
                    vec![buff("POWER.ENERGY_NEXT_TURN_POWER", 2)],
                ),
                vec![buff("POWER.ENERGY_NEXT_TURN_POWER", 3)],
            ),
            CardRarity::Event,
        ),
        // A peck is three small hits, and a fourth when upgraded.
        rarity(
            upgrading(
                card(
                    "CARD.PECK",
                    CardType::Attack,
                    TargetType::AnyEnemy,
                    1,
                    vec![multi_attack(Amount::Fixed(2), Amount::Fixed(3))],
                ),
                vec![multi_attack(Amount::Fixed(2), Amount::Fixed(4))],
            ),
            CardRarity::Event,
        ),
        // A rebound sends the next card played back on top of the draw pile.
        rarity(
            upgrading(
                card(
                    "CARD.REBOUND",
                    CardType::Attack,
                    TargetType::AnyEnemy,
                    1,
                    vec![attack(9), buff("POWER.REBOUND_POWER", 1)],
                ),
                vec![attack(12), buff("POWER.REBOUND_POWER", 1)],
            ),
            CardRarity::Event,
        ),
        // Relax is block now and cards and energy next turn.
        rarity(
            upgrading(
                exhausting(card(
                    "CARD.RELAX",
                    CardType::Skill,
                    TargetType::SelfTarget,
                    3,
                    relax(15, 2, 2),
                )),
                relax(17, 3, 3),
            ),
            CardRarity::Ancient,
        ),
        // Rip and tear lands twice on whoever the roll finds.
        rarity(
            upgrading(
                card(
                    "CARD.RIP_AND_TEAR",
                    CardType::Attack,
                    TargetType::RandomEnemy,
                    1,
                    vec![random_attack(7, 2)],
                ),
                vec![random_attack(9, 2)],
            ),
            CardRarity::Event,
        ),
        // A squash swings and leaves its target open.
        rarity(
            upgrading(
                card(
                    "CARD.SQUASH",
                    CardType::Attack,
                    TargetType::AnyEnemy,
                    1,
                    vec![attack(10), debuff("POWER.VULNERABLE_POWER", 2)],
                ),
                vec![attack(12), debuff("POWER.VULNERABLE_POWER", 3)],
            ),
            CardRarity::Event,
        ),
        // A stack is worth the discard pile.
        rarity(
            upgrading(
                card(
                    "CARD.STACK",
                    CardType::Skill,
                    TargetType::SelfTarget,
                    1,
                    vec![stack(0)],
                ),
                vec![stack(3)],
            ),
            CardRarity::Event,
        ),
        // Toric toughness works its block up again for two turns.
        rarity(
            upgrading(
                card(
                    "CARD.TORIC_TOUGHNESS",
                    CardType::Skill,
                    TargetType::SelfTarget,
                    2,
                    toric_toughness(5),
                ),
                toric_toughness(7),
            ),
            CardRarity::Event,
        ),
        // A wish takes any one card out of the draw pile.
        rarity(
            retain_when_upgraded(exhausting(card(
                "CARD.WISH",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![Command::CardSelect(
                    sts2_engine::CardSelectCmd::FromCombatPile {
                        semantics: ChoiceSemantics::known(vec![
                            ChoiceOp::new(ChoiceOperationKind::Move, ChoiceScope::Combat)
                                .to(ChoiceDestination::Hand),
                        ]),
                        exact_count: None,
                        pile: PileName::Draw,
                        prefs: sts2_engine::SelectorPrefs::exactly(1),
                        maximum: None,
                        restricted_to_chosen: false,
                        filter: CardFilter::Any,
                        then: vec![Command::CardPile(CardPileCmd::MoveToPile {
                            cards: CardSelector::Chosen,
                            pile: PileName::Hand,
                            position: DrawPilePosition::Bottom,
                        })],
                    },
                )],
            ))),
            CardRarity::Ancient,
        ),
        // A whistle is one enormous blow that also stuns.
        rarity(
            upgrading(
                exhausting(card(
                    "CARD.WHISTLE",
                    CardType::Attack,
                    TargetType::AnyEnemy,
                    3,
                    vec![attack(33), stun_target()],
                )),
                vec![attack(44), stun_target()],
            ),
            CardRarity::Ancient,
        ),
    ]
}

fn stun_target() -> Command {
    Command::Creature(CreatureCmd::Stun {
        target: CreatureSelector::Target,
        // The stun names no move to come back on, so the creature comes back
        // to whatever it last performed.
        next_move: StunFollowUp::TheLastMovePerformed,
        stun_move: Vec::new(),
    })
}

/// Brightest Flame, played: energy, then cards, then a point of maximum HP.
fn brightest_flame(energy_gained: i32, cards: i32) -> Vec<Command> {
    vec![
        energy(energy_gained),
        draw(cards),
        Command::Creature(CreatureCmd::LoseMaxHp {
            target: CreatureSelector::Owner,
            amount: Amount::Fixed(1),
        }),
    ]
}

/// Dual Wield, played: one attack or power of the hand, and that many copies of
/// it into the hand.
fn dual_wield(copies: i32) -> Vec<Command> {
    vec![Command::CardSelect(sts2_engine::CardSelectCmd::FromHand {
        semantics: ChoiceSemantics::known(vec![
            ChoiceOp::new(ChoiceOperationKind::Copy, ChoiceScope::Combat)
                .on(ChoiceSubject::Copy)
                .count(copies)
                .to(ChoiceDestination::Hand),
        ]),
        maximum: None,
        exact_count: None,
        prefs: sts2_engine::SelectorPrefs::exactly(1),
        filter: CardFilter::AttackOrPower,
        then: vec![Command::CardPile(CardPileCmd::AddGeneratedToCombat {
            enchantment: None,
            card: sts2_engine::CardOrigin::CloneOfChosen,
            grown_by: Amount::Fixed(0),
            count: Amount::Fixed(copies),
            pile: PileName::Hand,
            ethereal: false,
            retain: false,
            upgraded: false,
            temporary_cost: None,
            position: sts2_engine::DrawPilePosition::Bottom,
        })],
    })]
}

/// Maul, played: two hits of what this maul has grown to, and then every maul
/// in the fight grows, this one included.
fn maul(increase: i32) -> Vec<Command> {
    vec![
        multi_attack(Amount::counting(Count::SourceCardDamage), Amount::Fixed(2)),
        Command::Card(CardCmd::GrowDamage {
            cards: CardSelector::CombatCardsOfSourceModel,
            amount: Amount::Fixed(increase),
        }),
    ]
}

/// Metamorphosis, played: attacks of the owner's own pool, rolled one at a
/// time so the same card can come up twice, free for the fight, shuffled into
/// the draw pile.
fn metamorphosis(count: i32) -> Command {
    Command::CardPile(CardPileCmd::AddGeneratedFromPool {
        pool: sts2_engine::CardPoolRef::OwnCharacter,
        count: Amount::Fixed(count),
        distinct: false,
        filter: CardFilter::OfType(CardType::Attack),
        pile: PileName::Draw,
        position: DrawPilePosition::Random,
        excludes_source: false,
        upgraded: false,
        temporary_cost: Some(sts2_engine::TemporaryCost::free_this_combat()),
        ethereal: false,
        rolled_one_at_a_time: false,
    })
}

/// Neow's Fury, played: the swing, and then as many cards off the discard pile
/// as the hand has room for, up to what the card offers. No room is no screen.
fn neows_fury(damage: i32, cards: i32) -> Vec<Command> {
    vec![
        attack(damage),
        Command::CardSelect(sts2_engine::CardSelectCmd::FromCombatPile {
            semantics: ChoiceSemantics::known(vec![
                ChoiceOp::new(ChoiceOperationKind::Move, ChoiceScope::Combat)
                    .to(ChoiceDestination::Hand),
            ]),
            exact_count: None,
            pile: PileName::Discard,
            prefs: sts2_engine::SelectorPrefs::between(0, usize::try_from(cards).unwrap_or(0)),
            maximum: Some(Amount::Least(vec![
                Amount::Fixed(cards),
                Amount::counting(Count::SpaceLeftInHand),
            ])),
            restricted_to_chosen: false,
            filter: CardFilter::Any,
            then: vec![Command::CardPile(CardPileCmd::MoveToPile {
                cards: CardSelector::Chosen,
                pile: PileName::Hand,
                position: DrawPilePosition::Bottom,
            })],
        }),
    ]
}

/// Relax, played: block now, cards and energy next turn.
fn relax(block_amount: i32, cards: i32, energy_next: i32) -> Vec<Command> {
    vec![
        block(block_amount),
        buff("POWER.DRAW_CARDS_NEXT_TURN_POWER", cards),
        buff("POWER.ENERGY_NEXT_TURN_POWER", energy_next),
    ]
}

/// Stack, played: block worth the discard pile, plus whatever the upgrade adds
/// to the base.
fn stack(base: i32) -> Command {
    Command::Creature(CreatureCmd::GainBlock {
        target: CreatureSelector::Owner,
        amount: Amount::per(base, 1, Count::CardsInPile(PileName::Discard)),
        props: ValueProps::NONE,
    })
}

/// Toric Toughness, played: the block it works up now, and a power that works
/// up the same amount again for two more turns.
fn toric_toughness(block_amount: i32) -> Vec<Command> {
    vec![Command::Shield {
        target: CreatureSelector::Owner,
        amount: Amount::Fixed(block_amount),
        props: ValueProps::NONE,
        then: vec![
            buff("POWER.TORIC_TOUGHNESS_POWER", 2),
            Command::Power(PowerCmd::SetCounter {
                power: sts2_engine::PowerRef {
                    owner: CreatureSelector::Owner,
                    power: sts2_engine::PowerName::JustApplied,
                },
                amount: Amount::counting(Count::GainedBlock),
            }),
        ],
    }]
}

/// A card whose whole upgrade is adding Retain.
fn retain_when_upgraded(mut definition: CardDefinition) -> CardDefinition {
    definition.supported_upgrade_levels.push(1);
    let effects = definition.effects.clone();
    definition.upgrade = Some(CardUpgrade {
        energy_cost: None,
        innate: false,
        retain: true,
        drops_exhaust: false,
        drops_ethereal: false,
        damage_var: None,
        effects,
    });
    definition
}

/// A card whose whole upgrade is dropping the Exhaust keyword.
fn keeps_itself_when_upgraded(mut definition: CardDefinition) -> CardDefinition {
    definition.supported_upgrade_levels.push(1);
    let effects = definition.effects.clone();
    definition.upgrade = Some(CardUpgrade {
        energy_cost: None,
        innate: false,
        retain: false,
        drops_exhaust: true,
        drops_ethereal: false,
        damage_var: None,
        effects,
    });
    definition
}

/// A card whose whole upgrade is dropping the Ethereal keyword.
fn keeps_ethereal_when_upgraded(mut definition: CardDefinition) -> CardDefinition {
    definition.supported_upgrade_levels.push(1);
    let effects = definition.effects.clone();
    definition.upgrade = Some(CardUpgrade {
        energy_cost: None,
        innate: false,
        retain: false,
        drops_exhaust: false,
        drops_ethereal: true,
        damage_var: None,
        effects,
    });
    definition
}

/// What the card itself asks before it may be played.
fn playable_when(mut definition: CardDefinition, condition: Condition) -> CardDefinition {
    definition.playable_when = Some(condition);
    definition
}

/// The colorless pool's cards, less those in `offered_colorless_cards` and the
/// eleven the build marks multiplayer-only.
///
/// A multiplayer-only card is filtered out of every pool a single-player run
/// looks at, so nothing can offer, hold, or play one, and none is registered.
#[allow(clippy::too_many_lines, reason = "one entry per colorless card")]
pub(crate) fn colorless_cards() -> Vec<CardDefinition> {
    vec![
        // Alchemize brews a potion out of nothing: one rolled on the combat
        // potion-generation stream, which passes over what may not be conjured
        // inside a fight, and the belt takes it if it has room.
        upgrading_cost(
            exhausting(card(
                "CARD.ALCHEMIZE",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![Command::Potion(sts2_engine::PotionCmd::TryToProcure(
                    sts2_engine::PotionOrigin::RandomInCombat,
                ))],
            )),
            0,
        ),
        // Automation pays a point of energy for every ten cards drawn.
        upgrading_cost(
            card(
                "CARD.AUTOMATION",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.AUTOMATION_POWER", 1)],
            ),
            0,
        ),
        // A catastrophe plays cards off the draw pile without choosing them.
        upgrading(
            card(
                "CARD.CATASTROPHE",
                CardType::Skill,
                TargetType::SelfTarget,
                2,
                vec![catastrophe(2)],
            ),
            vec![catastrophe(3)],
        ),
        // Dark shackles takes an enemy's strength for the turn.
        upgrading(
            exhausting(card(
                "CARD.DARK_SHACKLES",
                CardType::Skill,
                TargetType::AnyEnemy,
                0,
                vec![debuff("POWER.DARK_SHACKLES_POWER", 9)],
            )),
            vec![debuff("POWER.DARK_SHACKLES_POWER", 15)],
        ),
        // A discovery lays three of the character's own cards out to choose
        // between, and what is picked is free for the turn.
        keeps_itself_when_upgraded(exhausting(card(
            "CARD.DISCOVERY",
            CardType::Skill,
            TargetType::SelfTarget,
            1,
            vec![Command::CardSelect(
                sts2_engine::CardSelectCmd::ChooseAGeneratedCard {
                    semantics: ChoiceSemantics::known(vec![
                        ChoiceOp::new(ChoiceOperationKind::Gain, ChoiceScope::Combat)
                            .to(ChoiceDestination::Hand),
                        ChoiceOp::new(ChoiceOperationKind::SetCost, ChoiceScope::Combat)
                            .amount(0)
                            .lasting(ChoiceDuration::UntilPlayedOrTurnEnds)
                            .including_stars(),
                    ]),
                    pool: sts2_engine::CardPoolRef::OwnCharacter,
                    count: 3,
                    filter: CardFilter::Any,
                    upgraded: false,
                    can_skip: true,
                    then: vec![Command::CardPile(CardPileCmd::AddGeneratedToCombat {
                        enchantment: None,
                        card: sts2_engine::CardOrigin::Chosen,
                        grown_by: Amount::Fixed(0),
                        count: Amount::Fixed(1),
                        pile: PileName::Hand,
                        ethereal: false,
                        retain: false,
                        upgraded: false,
                        temporary_cost: Some(sts2_engine::TemporaryCost::free_this_turn()),
                        position: sts2_engine::DrawPilePosition::Bottom,
                    })],
                },
            )],
        ))),
        // Entropy turns a card of the hand into another every turn.
        innate_when_upgraded(card(
            "CARD.ENTROPY",
            CardType::Power,
            TargetType::SelfTarget,
            1,
            vec![buff("POWER.ENTROPY_POWER", 1)],
        )),
        // Equilibrium keeps the whole hand for a turn.
        upgrading(
            card(
                "CARD.EQUILIBRIUM",
                CardType::Skill,
                TargetType::SelfTarget,
                2,
                vec![block(13), buff("POWER.RETAIN_HAND_POWER", 1)],
            ),
            vec![block(16), buff("POWER.RETAIN_HAND_POWER", 1)],
        ),
        // Fasten makes every Defend worth more.
        upgrading(
            card(
                "CARD.FASTEN",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.FASTEN_POWER", 4)],
            ),
            vec![buff("POWER.FASTEN_POWER", 6)],
        ),
        // Finesse is a little block and a card.
        upgrading(
            card(
                "CARD.FINESSE",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![block(4), draw(1)],
            ),
            vec![block(7), draw(1)],
        ),
        // A flash of steel is a little swing and a card.
        upgrading(
            card(
                "CARD.FLASH_OF_STEEL",
                CardType::Attack,
                TargetType::AnyEnemy,
                0,
                vec![attack(5), draw(1)],
            ),
            vec![attack(8), draw(1)],
        ),
        // A gold axe is worth every card play the fight has seen.
        retain_when_upgraded(card(
            "CARD.GOLD_AXE",
            CardType::Attack,
            TargetType::AnyEnemy,
            1,
            vec![multi_attack(
                Amount::per(0, 1, Count::CardPlaysFinishedThisCombat),
                Amount::Fixed(1),
            )],
        )),
        // A hand of greed pays gold for a kill that counts.
        upgrading(
            card(
                "CARD.HAND_OF_GREED",
                CardType::Attack,
                TargetType::AnyEnemy,
                2,
                hand_of_greed(20, 20),
            ),
            hand_of_greed(25, 25),
        ),
        // A hidden gem makes one card of the draw pile play twice more.
        upgrading(
            card(
                "CARD.HIDDEN_GEM",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![hidden_gem(2)],
            ),
            vec![hidden_gem(3)],
        ),
        // Impatience pays out only when the hand holds no attack at all.
        upgrading(
            card(
                "CARD.IMPATIENCE",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![impatience(2)],
            ),
            vec![impatience(3)],
        ),
        // A jack of all trades rolls colorless cards other than itself.
        upgrading(
            exhausting(card(
                "CARD.JACK_OF_ALL_TRADES",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![jack_of_all_trades(1)],
            )),
            vec![jack_of_all_trades(2)],
        ),
        // A jackpot swings and then deals out free cards of the owner's pool.
        upgrading(
            card(
                "CARD.JACKPOT",
                CardType::Attack,
                TargetType::AnyEnemy,
                3,
                jackpot(25, false),
            ),
            jackpot(30, true),
        ),
        // A master of strategy is three cards and gone.
        upgrading(
            exhausting(card(
                "CARD.MASTER_OF_STRATEGY",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![draw(3)],
            )),
            vec![draw(4)],
        ),
        // Mayhem plays the top of the draw pile every turn.
        upgrading_cost(
            card(
                "CARD.MAYHEM",
                CardType::Power,
                TargetType::SelfTarget,
                2,
                vec![buff("POWER.MAYHEM_POWER", 1)],
            ),
            1,
        ),
        // A mind blast is worth the draw pile, and is dealt into the opening
        // hand.
        upgrading_cost(
            innate(card(
                "CARD.MIND_BLAST",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![multi_attack(
                    Amount::per(0, 1, Count::CardsInPile(PileName::Draw)),
                    Amount::Fixed(1),
                )],
            )),
            0,
        ),
        // Panache hits everything for every fifth card played.
        upgrading(
            card(
                "CARD.PANACHE",
                CardType::Power,
                TargetType::SelfTarget,
                0,
                vec![buff("POWER.PANACHE_POWER", 10)],
            ),
            vec![buff("POWER.PANACHE_POWER", 14)],
        ),
        // A panic button is a wall of block and two turns of none at all.
        upgrading(
            exhausting(card(
                "CARD.PANIC_BUTTON",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![block(30), buff("POWER.NO_BLOCK_POWER", 2)],
            )),
            vec![block(40), buff("POWER.NO_BLOCK_POWER", 2)],
        ),
        // Prep time works vigor up every turn.
        upgrading(
            card(
                "CARD.PREP_TIME",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.PREP_TIME_POWER", 4)],
            ),
            vec![buff("POWER.PREP_TIME_POWER", 6)],
        ),
        // A prolong carries the block its owner is standing behind into the
        // next turn.
        keeps_itself_when_upgraded(exhausting(card(
            "CARD.PROLONG",
            CardType::Skill,
            TargetType::SelfTarget,
            0,
            vec![apply_power(
                "POWER.BLOCK_NEXT_TURN_POWER",
                CreatureSelector::Owner,
                Amount::counting(Count::OwnerBlock),
            )],
        ))),
        // Prowess is a point of each.
        upgrading(
            card(
                "CARD.PROWESS",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![
                    buff("POWER.STRENGTH_POWER", 1),
                    buff("POWER.DEXTERITY_POWER", 1),
                ],
            ),
            vec![
                buff("POWER.STRENGTH_POWER", 2),
                buff("POWER.DEXTERITY_POWER", 2),
            ],
        ),
        // Restlessness pays out only when nothing else is held.
        upgrading(
            retaining(card(
                "CARD.RESTLESSNESS",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![restlessness(2, 2)],
            )),
            vec![restlessness(3, 3)],
        ),
        // A rolling boulder rolls harder every turn.
        upgrading(
            card(
                "CARD.ROLLING_BOULDER",
                CardType::Power,
                TargetType::SelfTarget,
                3,
                vec![buff("POWER.ROLLING_BOULDER_POWER", 5)],
            ),
            vec![buff("POWER.ROLLING_BOULDER_POWER", 10)],
        ),
        // A scrawl fills the hand.
        retain_when_upgraded(exhausting(card(
            "CARD.SCRAWL",
            CardType::Skill,
            TargetType::SelfTarget,
            1,
            vec![Command::CardPile(CardPileCmd::Draw {
                count: Amount::counting(Count::SpaceLeftInHand),
            })],
        ))),
        // A secret weapon takes an attack out of the draw pile.
        keeps_itself_when_upgraded(exhausting(card(
            "CARD.SECRET_WEAPON",
            CardType::Skill,
            TargetType::SelfTarget,
            0,
            vec![Command::CardSelect(
                sts2_engine::CardSelectCmd::FromCombatPile {
                    semantics: ChoiceSemantics::known(vec![
                        ChoiceOp::new(ChoiceOperationKind::Move, ChoiceScope::Combat)
                            .to(ChoiceDestination::Hand),
                    ]),
                    exact_count: None,
                    pile: PileName::Draw,
                    prefs: sts2_engine::SelectorPrefs::exactly(1),
                    maximum: None,
                    restricted_to_chosen: false,
                    filter: CardFilter::OfType(CardType::Attack),
                    then: vec![Command::CardPile(CardPileCmd::MoveToPile {
                        cards: CardSelector::Chosen,
                        pile: PileName::Hand,
                        position: DrawPilePosition::Bottom,
                    })],
                },
            )],
        ))),
        // A seeker strike swings and then offers three of the draw pile.
        striking(upgrading(
            card(
                "CARD.SEEKER_STRIKE",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                seeker_strike(9),
            ),
            seeker_strike(12),
        )),
        // A shockwave leaves everything shaken and open.
        upgrading(
            exhausting(card(
                "CARD.SHOCKWAVE",
                CardType::Skill,
                TargetType::AllEnemies,
                2,
                shockwave(3),
            )),
            shockwave(5),
        ),
        // A stratagem hands cards back off every shuffle.
        upgrading_cost(
            card(
                "CARD.STRATAGEM",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![Command::Power(PowerCmd::Apply {
                    marks: None,
                    power: id("POWER.STRATAGEM_POWER"),
                    target: CreatureSelector::Owner,
                    applier: Some(CreatureSelector::Owner),
                    amount: Amount::Fixed(1),
                })],
            ),
            0,
        ),
        // The bomb goes off three turns later.
        upgrading(
            card(
                "CARD.THE_BOMB",
                CardType::Skill,
                TargetType::SelfTarget,
                2,
                the_bomb(40),
            ),
            the_bomb(50),
        ),
        // Thinking ahead draws two and puts one back on top.
        keeps_itself_when_upgraded(exhausting(card(
            "CARD.THINKING_AHEAD",
            CardType::Skill,
            TargetType::SelfTarget,
            0,
            vec![
                draw(2),
                Command::CardSelect(sts2_engine::CardSelectCmd::FromHand {
                    semantics: ChoiceSemantics::known(vec![
                        ChoiceOp::new(ChoiceOperationKind::Move, ChoiceScope::Combat)
                            .to(ChoiceDestination::DrawTop),
                    ]),
                    maximum: None,
                    exact_count: None,
                    prefs: sts2_engine::SelectorPrefs::exactly(1),
                    filter: CardFilter::Any,
                    then: vec![Command::CardPile(CardPileCmd::MoveToPile {
                        cards: CardSelector::Chosen,
                        pile: PileName::Draw,
                        position: DrawPilePosition::Top,
                    })],
                }),
            ],
        ))),
        // A volley lands once for every point of energy it swallowed.
        costing_x(upgrading(
            card(
                "CARD.VOLLEY",
                CardType::Attack,
                TargetType::RandomEnemy,
                0,
                vec![volley(10)],
            ),
            vec![volley(14)],
        )),
    ]
}

/// The rest of the colorless pool, registered alongside `colorless_cards`:
/// Anointed, Beat Down, Bolas, Calamity, Dramatic Entrance, Fisticuffs,
/// Nostalgia, Production, Purity, Rend, Salvo, Secret Technique, Splash, The
/// Gambit, Thrumming Hatchet, Ultimate Defend and Ultimate Strike.
#[allow(clippy::too_many_lines, reason = "one entry per colorless card")]
pub(crate) fn offered_colorless_cards() -> Vec<CardDefinition> {
    vec![
        // An anointed takes every rare card the hand has room for out of the
        // draw pile.
        retain_when_upgraded(exhausting(card(
            "CARD.ANOINTED",
            CardType::Skill,
            TargetType::SelfTarget,
            1,
            vec![Command::Select {
                cards: CardSelector::ShuffledFromPile {
                    pile: PileName::Draw,
                    filter: CardFilter::OfRarity(CardRarity::Rare),
                    take: Amount::counting(Count::SpaceLeftInHand),
                },
                then: vec![Command::CardPile(CardPileCmd::MoveToPile {
                    cards: CardSelector::Chosen,
                    pile: PileName::Hand,
                    position: DrawPilePosition::Bottom,
                })],
            }],
        ))),
        // A beat down plays attacks off the discard pile that nobody chose.
        upgrading(
            card(
                "CARD.BEAT_DOWN",
                CardType::Skill,
                TargetType::RandomEnemy,
                3,
                vec![beat_down(3)],
            ),
            vec![beat_down(4)],
        ),
        // A bolas is a small swing that comes back to the hand next turn.
        returns_to_hand(upgrading(
            card(
                "CARD.BOLAS",
                CardType::Attack,
                TargetType::AnyEnemy,
                0,
                vec![attack(3)],
            ),
            vec![attack(4)],
        )),
        // A calamity deals out attacks for every attack its owner plays.
        upgrading_cost(
            card(
                "CARD.CALAMITY",
                CardType::Power,
                TargetType::SelfTarget,
                3,
                vec![buff("POWER.CALAMITY_POWER", 1)],
            ),
            2,
        ),
        // A dramatic entrance opens the fight in hand and sweeps once.
        upgrading(
            innate(exhausting(card(
                "CARD.DRAMATIC_ENTRANCE",
                CardType::Attack,
                TargetType::AllEnemies,
                0,
                vec![sweep(11, 1)],
            ))),
            vec![sweep(15, 1)],
        ),
        // Fisticuffs works up block worth whatever its swing came to.
        upgrading(
            card(
                "CARD.FISTICUFFS",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![fisticuffs(7)],
            ),
            vec![fisticuffs(9)],
        ),
        // Nostalgia sends the first attack or skill of a turn back on top of
        // the draw pile.
        upgrading_cost(
            card(
                "CARD.NOSTALGIA",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.NOSTALGIA_POWER", 1)],
            ),
            0,
        ),
        // Production is two points of energy and gone.
        upgrading(
            exhausting(card(
                "CARD.PRODUCTION",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![energy(2)],
            )),
            vec![energy(3)],
        ),
        // Purity burns up to three cards of the hand, and is never put down.
        upgrading(
            retaining(exhausting(card(
                "CARD.PURITY",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![purity(3)],
            ))),
            vec![purity(5)],
        ),
        // A rend is worth more for every lasting debuff its target carries.
        upgrading(
            card(
                "CARD.REND",
                CardType::Attack,
                TargetType::AnyEnemy,
                2,
                vec![rend(15, 5)],
            ),
            vec![rend(18, 8)],
        ),
        // A salvo swings and keeps the hand for a turn.
        upgrading(
            card(
                "CARD.SALVO",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![attack(12), buff("POWER.RETAIN_HAND_POWER", 1)],
            ),
            vec![attack(16), buff("POWER.RETAIN_HAND_POWER", 1)],
        ),
        // A secret technique takes a skill out of the draw pile.
        keeps_itself_when_upgraded(exhausting(card(
            "CARD.SECRET_TECHNIQUE",
            CardType::Skill,
            TargetType::SelfTarget,
            0,
            vec![Command::CardSelect(
                sts2_engine::CardSelectCmd::FromCombatPile {
                    semantics: ChoiceSemantics::known(vec![
                        ChoiceOp::new(ChoiceOperationKind::Move, ChoiceScope::Combat)
                            .to(ChoiceDestination::Hand),
                    ]),
                    exact_count: None,
                    pile: PileName::Draw,
                    prefs: sts2_engine::SelectorPrefs::exactly(1),
                    maximum: None,
                    restricted_to_chosen: false,
                    filter: CardFilter::OfType(CardType::Skill),
                    then: vec![Command::CardPile(CardPileCmd::MoveToPile {
                        cards: CardSelector::Chosen,
                        pile: PileName::Hand,
                        position: DrawPilePosition::Bottom,
                    })],
                },
            )],
        ))),
        // A splash offers three attacks out of the other characters' pools,
        // and whichever is taken is free for the turn it lands on.
        upgrading(
            card(
                "CARD.SPLASH",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                splash(false),
            ),
            splash(true),
        ),
        // The gambit is a wall of block that the next blow through it kills its
        // owner for.
        upgrading(
            card(
                "CARD.THE_GAMBIT",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![block(50), buff("POWER.THE_GAMBIT_POWER", 1)],
            ),
            vec![block(75), buff("POWER.THE_GAMBIT_POWER", 1)],
        ),
        // A thrumming hatchet comes back to the hand next turn.
        returns_to_hand(upgrading(
            card(
                "CARD.THRUMMING_HATCHET",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![attack(11)],
            ),
            vec![attack(14)],
        )),
        // The two ultimates are the basic cards writ large, and each counts as
        // what it is named after.
        tagged(
            upgrading(
                card(
                    "CARD.ULTIMATE_DEFEND",
                    CardType::Skill,
                    TargetType::SelfTarget,
                    1,
                    vec![block(11)],
                ),
                vec![block(15)],
            ),
            &[CardTag::Defend],
        ),
        tagged(
            upgrading(
                card(
                    "CARD.ULTIMATE_STRIKE",
                    CardType::Attack,
                    TargetType::AnyEnemy,
                    1,
                    vec![attack(14)],
                ),
                vec![attack(20)],
            ),
            &[CardTag::Strike],
        ),
    ]
}

/// Beat Down, played: playable attacks of the discard pile, put in canonical
/// order and shuffled on the shuffle stream, played one after another without
/// being chosen.
fn beat_down(count: i32) -> Command {
    Command::Select {
        cards: CardSelector::StablyShuffledFromPile {
            pile: PileName::Discard,
            filter: CardFilter::PlayableOfType(CardType::Attack),
            fallback: None,
            take: Amount::Fixed(count),
            stream: sts2_engine::ShuffleStream::Shuffle,
        },
        then: vec![Command::Card(CardCmd::AutoPlay {
            at_the_bodys_target: false,
            cards: CardSelector::Chosen,
        })],
    }
}

/// Fisticuffs, played: the swing, and then block worth every point it dealt,
/// overkill included.
fn fisticuffs(damage: i32) -> Command {
    Command::Strike {
        amount: Amount::Fixed(damage),
        hits: Amount::Fixed(1),
        target: CreatureSelector::Target,
        props: ValueProps::NONE,
        then: vec![Command::Creature(CreatureCmd::GainBlock {
            target: CreatureSelector::Owner,
            amount: Amount::counting(Count::TotalDealtDamageWithOverkill),
            props: ValueProps::NONE,
        })],
    }
}

/// Purity, played: up to this many cards of the hand, burned.
fn purity(cards: i32) -> Command {
    Command::CardSelect(sts2_engine::CardSelectCmd::FromHand {
        semantics: ChoiceSemantics::known(vec![
            ChoiceOp::new(ChoiceOperationKind::Exhaust, ChoiceScope::Combat)
                .to(ChoiceDestination::Exhaust),
        ]),
        maximum: None,
        exact_count: None,
        prefs: sts2_engine::SelectorPrefs::between(0, usize::try_from(cards).unwrap_or(0)),
        filter: CardFilter::Any,
        then: vec![Command::Card(CardCmd::Exhaust {
            cards: CardSelector::Chosen,
        })],
    })
}

/// Rend, played: a base swing, and more for every lasting debuff its target is
/// carrying.
fn rend(base: i32, per_debuff: i32) -> Command {
    multi_attack(
        Amount::per(base, per_debuff, Count::TargetLastingDebuffs),
        Amount::Fixed(1),
    )
}

/// A card that puts itself back in the hand as the next turn's draw opens, if
/// it was played on the turn before and is not already held.
fn returns_to_hand(mut definition: CardDefinition) -> CardDefinition {
    definition.hooks.push(sts2_engine::ModifierHook::every_turn(
        sts2_engine::HookEvent::BeforeHandDraw,
        vec![Command::When {
            otherwise: Vec::new(),
            condition: Condition::All(vec![
                Condition::SourceCardPlayedLastTurn,
                Condition::Not(Box::new(Condition::SourceCardIsInPile(PileName::Hand))),
            ]),
            then: vec![Command::CardPile(CardPileCmd::MoveToPile {
                cards: CardSelector::Source,
                pile: PileName::Hand,
                position: DrawPilePosition::Bottom,
            })],
        }],
    ));
    definition
}

/// Catastrophe, played: a card of the draw pile, played without being chosen,
/// as many times as the card says. The build takes the playable cards, puts
/// them in canonical order, shuffles on the shuffle stream and takes the first;
/// only where there is no playable card at all does it fall back to the whole
/// pile.
fn catastrophe(times: i32) -> Command {
    Command::Repeat {
        times: Amount::Fixed(times),
        then: vec![
            Command::When {
                otherwise: Vec::new(),
                condition: Condition::Not(Box::new(Condition::CardsInPileAtLeast {
                    pile: PileName::Draw,
                    count: 1,
                })),
                then: Vec::new(),
            },
            Command::Select {
                // The whole draw pile is what the build falls back on where
                // it holds no playable card at all.
                cards: CardSelector::StablyShuffledFromPile {
                    pile: PileName::Draw,
                    filter: CardFilter::Playable,
                    fallback: Some(CardFilter::Any),
                    take: Amount::Fixed(1),
                    stream: sts2_engine::ShuffleStream::Shuffle,
                },
                then: vec![Command::Card(CardCmd::AutoPlay {
                    at_the_bodys_target: false,
                    cards: CardSelector::Chosen,
                })],
            },
        ],
    }
}

/// Hand of Greed, played: the swing, and the gold if it killed something whose
/// death counts.
fn hand_of_greed(damage: i32, gold: i32) -> Vec<Command> {
    vec![
        attack(damage),
        Command::When {
            otherwise: Vec::new(),
            condition: Condition::AFatalKillHappenedThisPlay,
            then: vec![Command::Player(PlayerCmd::GainGold(Amount::Fixed(gold)))],
        },
    ]
}

/// Hidden Gem, played: one card of the draw pile, rolled on the combat
/// card-selection stream, plays that many extra times.
fn hidden_gem(replays: i32) -> Command {
    Command::Select {
        // The build writes two lists and rolls over whichever has anything in
        // it: the cards a replay may go on, and inside that the attacks, skills
        // and powers it prefers. A playable status is only reached when the
        // preferred list is empty.
        cards: CardSelector::RandomFromPile {
            pile: PileName::Draw,
            filter: CardFilter::CanTakeAReplay,
            fallback: Some(CardFilter::PlayableAndNotACurse),
        },
        then: vec![Command::Card(CardCmd::ModifyReplayCount {
            cards: CardSelector::Chosen,
            amount: Amount::Fixed(replays),
        })],
    }
}

/// Impatience, played: cards, but only out of a hand holding no attack.
fn impatience(cards: i32) -> Command {
    Command::When {
        otherwise: Vec::new(),
        condition: Condition::Not(Box::new(Condition::CardsInPileOfTypeAtLeast {
            pile: PileName::Hand,
            card_type: CardType::Attack,
            count: 1,
        })),
        then: vec![draw(cards)],
    }
}

/// Jack of All Trades, played: colorless cards other than itself, into the
/// hand.
fn jack_of_all_trades(count: i32) -> Command {
    Command::CardPile(CardPileCmd::AddGeneratedFromPool {
        pool: sts2_engine::CardPoolRef::Named(id("CARD_POOL.COLORLESS_CARD_POOL")),
        count: Amount::Fixed(count),
        distinct: true,
        filter: CardFilter::Any,
        pile: PileName::Hand,
        position: DrawPilePosition::Bottom,
        excludes_source: true,
        upgraded: false,
        temporary_cost: None,
        ethereal: false,
        rolled_one_at_a_time: false,
    })
}

/// Jackpot, played: the swing, and then free-cost cards of the owner's own
/// pool, upgraded if the jackpot is.
fn jackpot(damage: i32, upgraded: bool) -> Vec<Command> {
    vec![
        attack(damage),
        Command::CardPile(CardPileCmd::AddGeneratedFromPool {
            pool: sts2_engine::CardPoolRef::OwnCharacter,
            count: Amount::Fixed(3),
            distinct: false,
            filter: CardFilter::Free,
            pile: PileName::Hand,
            position: DrawPilePosition::Bottom,
            excludes_source: false,
            upgraded,
            temporary_cost: None,
            ethereal: false,
            rolled_one_at_a_time: false,
        }),
    ]
}

/// Restlessness, played: cards and energy, but only when nothing else is held.
fn restlessness(cards: i32, energy_gained: i32) -> Command {
    Command::When {
        otherwise: Vec::new(),
        condition: Condition::Not(Box::new(Condition::CardsInPileAtLeast {
            pile: PileName::Hand,
            count: 1,
        })),
        then: vec![
            // The build draws one card at a time rather than asking for the
            // whole hand at once.
            Command::Repeat {
                times: Amount::Fixed(cards),
                then: vec![draw(1)],
            },
            energy(energy_gained),
        ],
    }
}

/// Seeker Strike, played: the swing, and then a choice between three of the
/// draw pile, shuffled on the combat card-selection stream.
fn seeker_strike(damage: i32) -> Vec<Command> {
    vec![
        attack(damage),
        Command::Select {
            cards: CardSelector::StablyShuffledFromPile {
                pile: PileName::Draw,
                filter: CardFilter::Any,
                fallback: None,
                take: Amount::Fixed(3),
                stream: sts2_engine::ShuffleStream::CombatCardSelection,
            },
            then: vec![Command::CardSelect(
                sts2_engine::CardSelectCmd::FromCombatPile {
                    semantics: ChoiceSemantics::known(vec![
                        ChoiceOp::new(ChoiceOperationKind::Move, ChoiceScope::Combat)
                            .to(ChoiceDestination::Hand),
                    ]),
                    exact_count: None,
                    pile: PileName::Draw,
                    prefs: sts2_engine::SelectorPrefs::exactly(1),
                    maximum: None,
                    restricted_to_chosen: true,
                    filter: CardFilter::Any,
                    then: vec![Command::CardPile(CardPileCmd::MoveToPile {
                        cards: CardSelector::Chosen,
                        pile: PileName::Hand,
                        position: DrawPilePosition::Bottom,
                    })],
                },
            )],
        },
    ]
}

/// Shockwave, played: every enemy shaken and opened, one at a time.
fn shockwave(amount: i32) -> Vec<Command> {
    vec![
        apply_power(
            "POWER.WEAK_POWER",
            CreatureSelector::AllOpponents,
            Amount::Fixed(amount),
        ),
        apply_power(
            "POWER.VULNERABLE_POWER",
            CreatureSelector::AllOpponents,
            Amount::Fixed(amount),
        ),
    ]
}

/// The Bomb, played: three turns, and the damage the power keeps for itself.
fn the_bomb(damage: i32) -> Vec<Command> {
    vec![
        buff("POWER.THE_BOMB_POWER", 3),
        // The fuse is written on the power this play just applied, not on
        // whichever bomb the owner happens to be carrying first.
        Command::Power(PowerCmd::SetCounter {
            power: sts2_engine::PowerRef {
                owner: CreatureSelector::Owner,
                power: sts2_engine::PowerName::JustApplied,
            },
            amount: Amount::Fixed(damage),
        }),
    ]
}

/// Volley, played: one hit for every point of energy the play swallowed.
fn volley(damage: i32) -> Command {
    Command::Damage(sts2_engine::DamageCmd::Attack {
        amount: Amount::Fixed(damage),
        hits: Amount::counting(Count::EnergyX),
        target: CreatureSelector::RandomOpponent,
        props: ValueProps::NONE,
    })
}

/// Everything this module registers, in one list.
pub(crate) fn shared_cards() -> Vec<CardDefinition> {
    let mut cards = status_cards();
    cards.extend(curse_cards());
    cards.extend(token_cards());
    cards.extend(quest_cards());
    cards.extend(event_cards());
    cards.extend(colorless_cards());
    cards.extend(offered_colorless_cards());
    cards
}
