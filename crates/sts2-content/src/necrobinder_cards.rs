//! The Necrobinder's whole card pool, transcribed.
//!
//! Eighty-eight models: the four she starts with, the eighty a rolled rarity
//! can offer, the two ancients, and the two multiplayer-only cards no
//! single-player pool ever holds. Each is the run of commands the card issues
//! when it is played, in order, at both the levels it supports.
//!
//! Three things recur across the pool and are worth naming once. A summon
//! stands the pet up, raises its maximum, or brings it back, all through the
//! one step. An Osty attack is a blow dealt by the pet rather than by its
//! owner, and every one of them is guarded on the pet standing — the shake a
//! missing pet plays is presentation, and a play with no pet does nothing at
//! all. And a Soul is a token
//! card her own cards conjure by the handful, always into a rolled place in the
//! draw pile unless the body says otherwise.

use sts2_engine::{
    Amount, CardCmd, CardDefinition, CardFilter, CardOrigin, CardPileCmd, CardSelectCmd,
    CardSelector, CardTag, CardType, Command, Condition, Count, CreatureCmd, CreatureSelector,
    DamageCmd, DrawPilePosition, HookEvent, ModifierHook, OstyCmd, PileName, PowerCmd,
    SelectorPrefs, TargetType, ValueProps,
};
use sts2_engine::{
    ChoiceCondition, ChoiceDestination, ChoiceDuration, ChoiceOperation as ChoiceOp,
    ChoiceOperationKind, ChoiceScope, ChoiceSemantics,
};

use crate::cards::{
    apply_power, attack, block, buff, card, costing_x, debuff, defending, draw, energy, exhausting,
    innate_when_upgraded, multi_attack, rarity, striking, sweep, upgrading, upgrading_cost,
    with_damage_var,
};
use crate::id;

/// A summon worth that much, for the card's own owner.
pub(crate) fn summon(amount: i32) -> Command {
    Command::Osty(OstyCmd::Summon {
        amount: Amount::Fixed(amount),
        owner: CreatureSelector::Owner,
    })
}

/// A blow dealt by the pet rather than by its owner, guarded on the pet
/// standing the way every one of these cards guards it.
fn osty_attack(
    amount: Amount,
    hits: Amount,
    target: CreatureSelector,
    then: Vec<Command>,
) -> Command {
    let mut body = vec![Command::Damage(DamageCmd::AttackFromPet {
        amount,
        hits,
        target,
        props: ValueProps::NONE,
    })];
    body.extend(then);
    Command::When {
        condition: Condition::PetIsStanding,
        then: body,
        otherwise: Vec::new(),
    }
}

/// The commonest shape of one: a flat blow at whoever the play was aimed at.
fn osty_strike(amount: i32) -> Command {
    osty_attack(
        Amount::Fixed(amount),
        Amount::Fixed(1),
        CreatureSelector::Target,
        Vec::new(),
    )
}

/// The same over the whole enemy side, aimed from the pet's side of the
/// fight.
fn osty_sweep_all(amount: i32) -> Command {
    osty_attack(
        Amount::Fixed(amount),
        Amount::Fixed(1),
        CreatureSelector::AllOpponents,
        Vec::new(),
    )
}

/// Rolled targets: one roll per hit, on the combat-targets stream the pet
/// reads through its owner.
pub(crate) fn osty_sweep(amount: i32) -> Command {
    osty_attack(
        Amount::Fixed(amount),
        Amount::Fixed(1),
        CreatureSelector::RandomOpponent,
        Vec::new(),
    )
}

/// Tags the card an Osty attack, which a Squeeze counts and a Flatten reads.
fn osty_tagged(mut definition: CardDefinition) -> CardDefinition {
    definition.tags.push(CardTag::OstyAttack);
    definition
}

/// A base amount plus an extra point of damage for each of something the pet
/// is worth, swung by the pet.
fn scaled_by_the_pet(base: i32, per: i32, count: Count) -> Command {
    osty_attack(
        Amount::Scaled { base, per, count },
        Amount::Fixed(1),
        CreatureSelector::Target,
        Vec::new(),
    )
}

/// Ethereal: the card burns up if the turn ends with it in hand.
fn ethereal(mut definition: CardDefinition) -> CardDefinition {
    definition.ethereal = true;
    definition
}

/// Retain: the card is not put down when the hand is flushed.
fn retaining(mut definition: CardDefinition) -> CardDefinition {
    definition.retain = true;
    definition
}

/// Eternal: the card can never leave the deck.
fn eternal(mut definition: CardDefinition) -> CardDefinition {
    definition.eternal = true;
    definition
}

/// An upgrade that adds Retain and changes nothing else.
fn retain_when_upgraded(mut definition: CardDefinition) -> CardDefinition {
    if let Some(upgrade) = definition.upgrade.as_mut() {
        upgrade.retain = true;
    }
    definition
}

/// An upgrade that takes Exhaust back off.
fn upgrade_drops_exhaust(mut definition: CardDefinition) -> CardDefinition {
    if let Some(upgrade) = definition.upgrade.as_mut() {
        upgrade.drops_exhaust = true;
    }
    definition
}

/// That many Souls made for the fight and put into the named pile at the named
/// position.
fn souls(count: i32, pile: PileName, position: DrawPilePosition, upgraded: bool) -> Command {
    Command::CardPile(CardPileCmd::AddGeneratedToCombat {
        enchantment: None,
        card: CardOrigin::Model(id("CARD.SOUL")),
        grown_by: Amount::Fixed(0),
        count: Amount::Fixed(count),
        pile,
        ethereal: false,
        retain: false,
        upgraded,
        temporary_cost: None,
        position,
    })
}

/// The same for a count the play works out, which is what an X-cost Dirge
/// hands over.
fn many_souls(count: Amount, upgraded: bool) -> Command {
    Command::CardPile(CardPileCmd::AddGeneratedToCombat {
        enchantment: None,
        card: CardOrigin::Model(id("CARD.SOUL")),
        grown_by: Amount::Fixed(0),
        count,
        pile: PileName::Draw,
        ethereal: false,
        retain: false,
        upgraded,
        temporary_cost: None,
        position: DrawPilePosition::Random,
    })
}

/// The power worked onto every enemy that can be hit, by the card's owner.
fn sweeping_power(power: &str, amount: i32) -> Command {
    apply_power(power, CreatureSelector::AllOpponents, Amount::Fixed(amount))
}

/// That much strength taken off the target.
fn lose_strength(target: CreatureSelector, amount: i32) -> Command {
    apply_power("POWER.STRENGTH_POWER", target, Amount::Fixed(-amount))
}

/// The Necrobinder's eighty-eight, in her pool's own order, with the two
/// tokens her cards conjure registered elsewhere.
#[must_use]
pub fn necrobinder_cards() -> Vec<CardDefinition> {
    let mut cards = the_pet_and_the_pool();
    cards.extend(the_plain_attacks());
    cards.extend(the_souls_and_the_screens());
    cards.extend(the_powers());
    cards.extend(the_doom_cards());
    cards.extend(the_rest_of_the_pet());
    cards.extend(the_cards_that_wait());
    cards
}

/// Doom: a countdown on the target's own life.
fn doom(amount: Amount) -> Command {
    Command::Power(PowerCmd::Apply {
        marks: None,
        power: id("POWER.DOOM_POWER"),
        target: CreatureSelector::Target,
        applier: Some(CreatureSelector::Owner),
        amount,
    })
}

/// The same over the whole enemy side, one at a time.
fn doom_everything(amount: i32) -> Command {
    apply_power(
        "POWER.DOOM_POWER",
        CreatureSelector::AllOpponents,
        Amount::Fixed(amount),
    )
}

/// The cards written around Doom.
#[allow(clippy::too_many_lines, reason = "one entry per card")]
fn the_doom_cards() -> Vec<CardDefinition> {
    vec![
        // A scourge is Doom and a card.
        upgrading(
            card(
                "CARD.SCOURGE",
                CardType::Skill,
                TargetType::AnyEnemy,
                1,
                vec![doom(Amount::Fixed(13)), draw(1)],
            ),
            vec![doom(Amount::Fixed(16)), draw(2)],
        ),
        // A negative pulse is block and Doom over everything.
        upgrading(
            card(
                "CARD.NEGATIVE_PULSE",
                CardType::Skill,
                TargetType::AllEnemies,
                1,
                vec![block(5), doom_everything(7)],
            ),
            vec![block(6), doom_everything(11)],
        ),
        // A deathbringer is Doom and a weakness over everything.
        upgrading(
            card(
                "CARD.DEATHBRINGER",
                CardType::Skill,
                TargetType::AllEnemies,
                2,
                vec![doom_everything(21), sweeping_power("POWER.WEAK_POWER", 1)],
            ),
            vec![doom_everything(26), sweeping_power("POWER.WEAK_POWER", 1)],
        ),
        // The end of days dooms everything and then collects on it at once,
        // rather than waiting for the turn to end.
        upgrading(
            card(
                "CARD.END_OF_DAYS",
                CardType::Skill,
                TargetType::AllEnemies,
                3,
                vec![doom_everything(29), doom_kill()],
            ),
            vec![doom_everything(37), doom_kill()],
        ),
        // No escape works out its Doom off what the target already carries:
        // ten, and five more for every whole ten of it.
        upgrading(
            card(
                "CARD.NO_ESCAPE",
                CardType::Skill,
                TargetType::AnyEnemy,
                1,
                vec![doom(no_escape_amount(10))],
            ),
            vec![doom(no_escape_amount(15))],
        ),
        // Time's up is worth exactly what the target's Doom is.
        exhausting(retain_when_upgraded(upgrading(
            card(
                "CARD.TIMES_UP",
                CardType::Attack,
                TargetType::AnyEnemy,
                2,
                vec![multi_attack(
                    Amount::counting(Count::TargetPower(id("POWER.DOOM_POWER"))),
                    Amount::Fixed(1),
                )],
            ),
            vec![multi_attack(
                Amount::counting(Count::TargetPower(id("POWER.DOOM_POWER"))),
                Amount::Fixed(1),
            )],
        ))),
        // A blight strike is a blow and as much Doom as the blow came to.
        striking(upgrading(
            card(
                "CARD.BLIGHT_STRIKE",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![blight_strike(8)],
            ),
            vec![blight_strike(10)],
        )),
        // A countdown dooms one enemy, rolled, every turn.
        upgrading(
            card(
                "CARD.COUNTDOWN",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.COUNTDOWN_POWER", 6)],
            ),
            vec![buff("POWER.COUNTDOWN_POWER", 9)],
        ),
        // A neurosurge pays three energy and two cards now and dooms its own
        // owner three every turn for it.
        upgrading(
            card(
                "CARD.NEUROSURGE",
                CardType::Power,
                TargetType::SelfTarget,
                0,
                vec![energy(3), draw(2), buff("POWER.NEUROSURGE_POWER", 3)],
            ),
            vec![energy(4), draw(2), buff("POWER.NEUROSURGE_POWER", 3)],
        ),
        // A shroud pays block every time its owner works Doom.
        upgrading(
            card(
                "CARD.SHROUD",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.SHROUD_POWER", 2)],
            ),
            vec![buff("POWER.SHROUD_POWER", 3)],
        ),
        // A danse macabre pays block before every card worth two or more.
        upgrading(
            card(
                "CARD.DANSE_MACABRE",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.DANSE_MACABRE_POWER", 4)],
            ),
            vec![buff("POWER.DANSE_MACABRE_POWER", 6)],
        ),
        // A spirit of ash pays block before every ethereal card.
        upgrading(
            card(
                "CARD.SPIRIT_OF_ASH",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.SPIRIT_OF_ASH_POWER", 4)],
            ),
            vec![buff("POWER.SPIRIT_OF_ASH_POWER", 5)],
        ),
        // Devouring life summons every time a Soul is played.
        upgrading(
            card(
                "CARD.DEVOUR_LIFE",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.DEVOUR_LIFE_POWER", 1)],
            ),
            vec![buff("POWER.DEVOUR_LIFE_POWER", 2)],
        ),
        // A haunt cuts an enemy every time a Soul is played.
        upgrading(
            card(
                "CARD.HAUNT",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.HAUNT_POWER", 6)],
            ),
            vec![buff("POWER.HAUNT_POWER", 8)],
        ),
    ]
}

/// Every doomed enemy that can be hit goes down where it stands rather than
/// waiting for the turn to end.
fn doom_kill() -> Command {
    Command::Creature(CreatureCmd::DoomKill {
        target: CreatureSelector::DoomedAmongOpponents(id("POWER.DOOM_POWER")),
    })
}

/// Ten, and five more for every whole ten of the target's own Doom, which the
/// build writes as a base plus an extra per ten.
fn no_escape_amount(base: i32) -> Amount {
    Amount::Sum(vec![
        Amount::Fixed(base),
        Amount::Product {
            left: Box::new(Amount::Fixed(5)),
            right: Box::new(Amount::Percent {
                percent: 10,
                count: Count::TargetPower(id("POWER.DOOM_POWER")),
            }),
        },
    ])
}

/// A blow, and then Doom worth exactly what the blow came to.
fn blight_strike(damage: i32) -> Command {
    Command::Deal {
        target: CreatureSelector::Target,
        amount: Amount::Fixed(damage),
        props: ValueProps::NONE,
        then: vec![doom(Amount::counting(Count::DealtDamageWithOverkill))],
    }
}

/// The rest of the pet's own cards.
#[allow(clippy::too_many_lines, reason = "one entry per card")]
fn the_rest_of_the_pet() -> Vec<CardDefinition> {
    vec![
        // A sacrifice puts the pet down and stands behind twice what it was
        // worth. The block is worked out before the kill, because after it
        // there is nothing to read.
        retaining(upgrading_cost(
            card(
                "CARD.SACRIFICE",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![Command::WithAmount {
                    amount: Amount::Scaled {
                        base: 0,
                        per: 2,
                        count: Count::PetMaxHp,
                    },
                    then: vec![Command::When {
                        condition: Condition::PetIsStanding,
                        then: vec![
                            Command::Creature(CreatureCmd::Kill {
                                target: CreatureSelector::Pet,
                            }),
                            Command::Creature(CreatureCmd::GainBlock {
                                target: CreatureSelector::Owner,
                                amount: Amount::counting(Count::Bound),
                                props: ValueProps::NONE,
                            }),
                        ],
                        otherwise: Vec::new(),
                    }],
                }],
            ),
            0,
        )),
        // A squeeze swings the pet for twenty-five and five more for every
        // other card of the fight that swings it too.
        osty_tagged(upgrading(
            card(
                "CARD.SQUEEZE",
                CardType::Attack,
                TargetType::AnyEnemy,
                3,
                vec![squeeze(25, 5)],
            ),
            vec![squeeze(30, 6)],
        )),
        // Sic 'em swings the pet and marks the target: the mark is worked on
        // whether or not the pet was there to swing.
        osty_tagged(upgrading(
            card(
                "CARD.SIC_EM",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![osty_strike(5), debuff("POWER.SIC_EM_POWER", 3)],
            ),
            vec![osty_strike(6), debuff("POWER.SIC_EM_POWER", 4)],
        )),
        // Necro mastery summons and then turns every point the pet loses into
        // unblockable damage over everything.
        upgrading(
            card(
                "CARD.NECRO_MASTERY",
                CardType::Power,
                TargetType::SelfTarget,
                2,
                vec![summon(5), buff("POWER.NECRO_MASTERY_POWER", 1)],
            ),
            vec![summon(8), buff("POWER.NECRO_MASTERY_POWER", 1)],
        ),
        // A reaper form turns everything its owner and its owner's pet deal
        // into as much Doom again.
        retain_when_upgraded(upgrading(
            card(
                "CARD.REAPER_FORM",
                CardType::Power,
                TargetType::SelfTarget,
                3,
                vec![buff("POWER.REAPER_FORM_POWER", 1)],
            ),
            vec![buff("POWER.REAPER_FORM_POWER", 1)],
        )),
    ]
}

/// `25 + 5 per other card of the combat carrying the pet's tag`, which is the
/// tagged-card count less this very card.
fn squeeze(base: i32, per: i32) -> Command {
    osty_attack(
        Amount::Sum(vec![
            Amount::Fixed(base),
            Amount::Product {
                left: Box::new(Amount::Fixed(per)),
                right: Box::new(Amount::Scaled {
                    base: -1,
                    per: 1,
                    count: Count::TaggedCards(CardTag::OstyAttack),
                }),
            },
        ]),
        Amount::Fixed(1),
        CreatureSelector::Target,
        Vec::new(),
    )
}

/// The cards whose bodies wait on machinery this baseline does not carry.
///
/// Each is registered for what it is — its cost, its type, its rarity, and its
/// place in the pool, which is everything a reward offer, a transform, and an
/// enchantment read off it — and playing one stops the run by name rather than
/// doing something the build does not.
/// Drain Power, played: the swing, and then that many upgradable cards of the
/// discard pile taken at random and upgraded.
fn drain_power(damage: i32, cards: i32) -> Vec<Command> {
    vec![
        attack(damage),
        Command::Card(CardCmd::Upgrade {
            cards: CardSelector::ShuffledFromPile {
                pile: PileName::Discard,
                filter: CardFilter::Upgradable,
                take: Amount::Fixed(cards),
            },
        }),
    ]
}

/// Melancholy: block, and a point off its own cost for the rest of the fight
/// every time anything goes down.
///
/// The body guards on the card standing in a combat pile, which is the only
/// place a card is a listener at all, and on the death not having been turned
/// away — a death that was prevented never reaches the pass.
fn melancholy() -> CardDefinition {
    let mut definition = upgrading(
        card(
            "CARD.MELANCHOLY",
            CardType::Skill,
            TargetType::SelfTarget,
            3,
            vec![block(13)],
        ),
        vec![block(17)],
    );
    definition.hooks.push(ModifierHook::every_turn(
        HookEvent::AfterDeath,
        vec![Command::Card(CardCmd::AddCostThisCombat {
            cards: CardSelector::Source,
            amount: Amount::Fixed(-1),
        })],
    ));
    definition
}

/// Each Ethereal play takes two energy off for the rest of the fight. A fresh
/// card entering combat also receives the discount from earlier plays; a clone
/// already carries the original card's cost changes.
fn banshees_cry() -> CardDefinition {
    let mut definition = upgrading_cost(
        card(
            "CARD.BANSHEES_CRY",
            CardType::Attack,
            TargetType::AllEnemies,
            9,
            vec![sweep(33, 1)],
        ),
        7,
    );
    definition.hooks = vec![
        ModifierHook::every_turn(
            HookEvent::AfterCardEnteredCombat,
            vec![Command::When {
                condition: Condition::Not(Box::new(Condition::SourceCardIsAClone)),
                then: vec![Command::Card(CardCmd::AddCostThisCombat {
                    cards: CardSelector::Source,
                    amount: Amount::per(0, -2, Count::EtherealPlaysThisCombat),
                })],
                otherwise: Vec::new(),
            }],
        )
        .when(sts2_engine::HookCondition::ListeningCardEnteredCombat),
        ModifierHook::every_turn(
            HookEvent::AfterCardPlayed,
            vec![Command::Card(CardCmd::AddCostThisCombat {
                cards: CardSelector::Source,
                amount: Amount::Fixed(-2),
            })],
        )
        .when(sts2_engine::HookCondition::All(vec![
            sts2_engine::HookCondition::OwnerPlayedTheCard,
            sts2_engine::HookCondition::PlayedCardIsEthereal,
        ])),
    ];
    definition
}

/// Eradicate, played: one swing per point of energy its own X swallowed.
fn eradicate(damage: i32) -> Command {
    multi_attack(Amount::Fixed(damage), Amount::counting(Count::EnergyX))
}

/// The Scythe, whose damage is a saved property: it starts at thirteen, grows
/// by the Increase var on every play, and the growth is written through to the
/// run-deck card so that it outlives the fight.
fn the_scythe() -> CardDefinition {
    let body = |increase: i32| {
        vec![
            // The swing is the damage the card is carrying, which it keeps
            // at thirteen plus everything its plays have added.
            multi_attack(
                Amount::counting(Count::SourceCardProperty("CurrentDamage".to_owned())),
                Amount::Fixed(1),
            ),
            // The play grows this card and the deck's own copy of it: the
            // growth is added and then carried into the damage.
            Command::Card(CardCmd::ModifyProperty {
                cards: CardSelector::Source,
                name: "IncreasedDamage".to_owned(),
                amount: Amount::Fixed(increase),
                and_the_deck_version: true,
            }),
            Command::Card(CardCmd::ModifyProperty {
                cards: CardSelector::Source,
                name: "CurrentDamage".to_owned(),
                amount: Amount::Fixed(increase),
                and_the_deck_version: true,
            }),
        ]
    };
    let mut definition = exhausting(upgrading(
        card(
            "CARD.THE_SCYTHE",
            CardType::Attack,
            TargetType::AnyEnemy,
            2,
            body(4),
        ),
        body(5),
    ));
    definition.saved_properties = vec![
        sts2_engine::SavedCardProperty::of("CurrentDamage", 13),
        sts2_engine::SavedCardProperty::of("IncreasedDamage", 0),
    ];
    definition
}

/// The base, plus that much more for each card drawn this turn that the hand
/// draw did not deal: the whole of a death march's damage.
fn death_march(base: i32, per: i32) -> Command {
    multi_attack(
        Amount::Scaled {
            base,
            per,
            count: Count::OwnerCardsDrawnThisTurnOutsideHandDraw,
        },
        Amount::Fixed(1),
    )
}

/// Death's Door, played: one gain of block, and the Repeat var's worth more
/// where the owner has worked a Doom onto anything this turn. Each gain is its
/// own gain of block, so each is scaled on its own.
fn deaths_door(block_amount: i32, repeat: i32) -> Vec<Command> {
    vec![Command::Repeat {
        times: Amount::IfElse {
            condition: Condition::OwnerAppliedPowerThisTurn(id("POWER.DOOM_POWER")),
            then: Box::new(Amount::Fixed(1 + repeat)),
            otherwise: Box::new(Amount::Fixed(1)),
        },
        then: vec![block(block_amount)],
    }]
}

/// Dredge, played: as many cards of the discard pile as it names go back into
/// the hand, capped by the room the hand has left, and no screen at all where
/// that comes to nothing.
fn dredge(count: i32) -> Vec<Command> {
    vec![Command::CardSelect(CardSelectCmd::FromCombatPile {
        semantics: ChoiceSemantics::known(vec![
            ChoiceOp::new(ChoiceOperationKind::Move, ChoiceScope::Combat)
                .to(ChoiceDestination::Hand),
        ]),
        pile: PileName::Discard,
        prefs: SelectorPrefs::exactly(0),
        maximum: Some(Amount::Least(vec![
            Amount::Fixed(count),
            Amount::counting(Count::SpaceLeftInHand),
        ])),
        exact_count: Some(Amount::Least(vec![
            Amount::Fixed(count),
            Amount::counting(Count::SpaceLeftInHand),
        ])),
        restricted_to_chosen: false,
        filter: CardFilter::Any,
        then: vec![Command::CardPile(CardPileCmd::MoveToPile {
            cards: CardSelector::Chosen,
            pile: PileName::Hand,
            position: DrawPilePosition::Bottom,
        })],
    })]
}

/// Eidolon, played: the whole hand burns, one card at a time over the list as
/// it stood, and nine or more of them pay a point of Intangible. The count is
/// the local the build increments per burn, which is the hand's own size.
fn eidolon() -> Vec<Command> {
    vec![Command::WithAmount {
        amount: Amount::counting(Count::CardsInPile(PileName::Hand)),
        then: vec![
            Command::Card(CardCmd::Exhaust {
                cards: CardSelector::Pile {
                    pile: PileName::Hand,
                    filter: CardFilter::Any,
                },
            }),
            Command::When {
                condition: Condition::BoundAtLeast(9),
                then: vec![buff("POWER.INTANGIBLE_POWER", 1)],
                otherwise: Vec::new(),
            },
        ],
    }]
}

/// Fetch, played: the pet swings, and the play draws where no play of this
/// card has finished this turn.
fn fetch(damage: i32) -> Vec<Command> {
    vec![osty_attack(
        Amount::Fixed(damage),
        Amount::Fixed(1),
        CreatureSelector::Target,
        vec![Command::When {
            condition: Condition::Not(Box::new(Condition::SourceCardPlayedThisTurn)),
            then: vec![draw(1)],
            otherwise: Vec::new(),
        }],
    )]
}

/// A flatten makes the card free for the turn, which is a cost change that
/// expires when the turn ends rather than when the card is played.
fn flatten_is_free() -> Command {
    Command::Card(CardCmd::SetTemporaryCost {
        cards: CardSelector::Source,
        lifetime: sts2_engine::TemporaryCost::set_this_turn(0),
    })
}

/// Rattle's hit count: one swing, and one more for each the pet has already
/// made this turn.
fn rattle(damage: i32) -> Vec<Command> {
    vec![osty_attack(
        Amount::Fixed(damage),
        Amount::per(1, 1, Count::OwnersPetAttacksThisTurn),
        CreatureSelector::Target,
        Vec::new(),
    )]
}

/// Transfigure, played: one card of the hand costs a point more for the rest
/// of the fight — where its cost can be written at all — and takes another run
/// of its own body either way.
fn transfigure() -> Vec<Command> {
    vec![Command::CardSelect(CardSelectCmd::FromHand {
        semantics: ChoiceSemantics::known(vec![
            ChoiceOp::new(ChoiceOperationKind::AddCost, ChoiceScope::Combat)
                .amount(1)
                .lasting(ChoiceDuration::Combat)
                .when(ChoiceCondition::WritableEnergyCost),
            ChoiceOp::new(ChoiceOperationKind::AddReplay, ChoiceScope::Combat)
                .amount(1)
                .lasting(ChoiceDuration::Combat),
        ]),
        prefs: SelectorPrefs::exactly(1),
        maximum: None,
        exact_count: None,
        filter: CardFilter::Any,
        then: vec![
            Command::Card(CardCmd::AddCostThisCombat {
                cards: CardSelector::ChosenMatching {
                    filter: CardFilter::HasWritableEnergyCost,
                },
                amount: Amount::Fixed(1),
            }),
            Command::Card(CardCmd::ModifyReplayCount {
                cards: CardSelector::Chosen,
                amount: Amount::Fixed(1),
            }),
        ],
    })]
}

#[allow(clippy::too_many_lines, reason = "one entry per card")]
fn the_cards_that_wait() -> Vec<CardDefinition> {
    vec![
        // A death march grows with the cards drawn this turn that the hand
        // draw did not deal.
        upgrading(
            card(
                "CARD.DEATH_MARCH",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![death_march(8, 4)],
            ),
            vec![death_march(9, 6)],
        ),
        // A death's door gains its block three times over where its owner has
        // worked a Doom onto anything this turn, and once where they have not.
        gaining_block(upgrading(
            card(
                "CARD.DEATHS_DOOR",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                deaths_door(6, 2),
            ),
            deaths_door(7, 2),
        )),
        // A drain power swings and then upgrades a couple of cards of the
        // discard pile, taken at random on the combat card-selection stream.
        with_damage_var(
            upgrading(
                card(
                    "CARD.DRAIN_POWER",
                    CardType::Attack,
                    TargetType::AnyEnemy,
                    1,
                    drain_power(10, 2),
                ),
                drain_power(12, 3),
            ),
            10,
            12,
        ),
        // A dredge takes up to three cards back out of the discard pile, and
        // the count is what the hand has room for.
        exhausting(retain_when_upgraded(upgrading(
            card(
                "CARD.DREDGE",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                dredge(3),
            ),
            dredge(3),
        ))),
        // An eidolon burns the whole hand and pays intangible for nine.
        upgrading_cost(
            card(
                "CARD.EIDOLON",
                CardType::Skill,
                TargetType::SelfTarget,
                2,
                eidolon(),
            ),
            1,
        ),
        // An eradicate lands as many times as the energy its own X swallowed.
        retaining(costing_x(with_damage_var(
            upgrading(
                card(
                    "CARD.ERADICATE",
                    CardType::Attack,
                    TargetType::AnyEnemy,
                    0,
                    vec![eradicate(11)],
                ),
                vec![eradicate(14)],
            ),
            11,
            14,
        ))),
        // A fetch draws on its own first play of the turn.
        osty_tagged(upgrading(
            card(
                "CARD.FETCH",
                CardType::Attack,
                TargetType::AnyEnemy,
                0,
                fetch(3),
            ),
            fetch(6),
        )),
        // A flatten is free once the pet has swung this turn, which it says
        // twice: once as it enters the fight and once off every swing the pet
        // makes while it stands in a pile.
        osty_tagged(flatten()),
        // A lethality raises the turn's first attack, and which play that is
        // is read off the fight's own history of started plays.
        ethereal(upgrading(
            card(
                "CARD.LETHALITY",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.LETHALITY_POWER", 50)],
            ),
            vec![buff("POWER.LETHALITY_POWER", 75)],
        )),
        // A melancholy gets a point cheaper for the rest of the fight every
        // time anything dies, which is a listener the card carries while it
        // stands in a combat pile.
        melancholy(),
        // A misery copies every debuff the target was already carrying onto
        // everything else.
        retain_when_upgraded(upgrading(
            card(
                "CARD.MISERY",
                CardType::Attack,
                TargetType::AnyEnemy,
                0,
                misery(7),
            ),
            misery(9),
        )),
        // An oblivion marks the target and pays Doom for every card its
        // applier plays this turn.
        upgrading(
            card(
                "CARD.OBLIVION",
                CardType::Skill,
                TargetType::AnyEnemy,
                0,
                vec![oblivion(3)],
            ),
            vec![oblivion(4)],
        ),
        // A pull from below lands once for every ethereal card played this
        // combat.
        upgrading(
            card(
                "CARD.PULL_FROM_BELOW",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![multi_attack(
                    Amount::Fixed(5),
                    Amount::counting(Count::EtherealPlaysThisCombat),
                )],
            ),
            vec![multi_attack(
                Amount::Fixed(7),
                Amount::counting(Count::EtherealPlaysThisCombat),
            )],
        ),
        // A rattle lands once more for every swing the pet has already made
        // this turn.
        osty_tagged(upgrading(
            card(
                "CARD.RATTLE",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                rattle(7),
            ),
            rattle(9),
        )),
        // A right hand hand comes back to the hand whenever its owner spends
        // two energy on a play, which it answers from the discard pile.
        osty_tagged(right_hand_hand()),
        // A sleight of flesh cuts an enemy every time a lasting debuff its
        // owner works lands on it.
        upgrading(
            card(
                "CARD.SLEIGHT_OF_FLESH",
                CardType::Power,
                TargetType::SelfTarget,
                2,
                vec![buff("POWER.SLEIGHT_OF_FLESH_POWER", 9)],
            ),
            vec![buff("POWER.SLEIGHT_OF_FLESH_POWER", 13)],
        ),
        // A soul storm grows with the Souls burned this fight.
        upgrading(
            card(
                "CARD.SOUL_STORM",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![soul_storm(9, 2)],
            ),
            vec![soul_storm(9, 3)],
        ),
        // The scythe grows every time it is played and keeps the growth on the
        // deck's own card.
        the_scythe(),
        // A transfigure makes one card of the hand dearer and gives it another
        // run of its own body.
        exhausting(upgrade_drops_exhaust(upgrading(
            card(
                "CARD.TRANSFIGURE",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                transfigure(),
            ),
            transfigure(),
        ))),
        // A veilpiercer makes the next ethereal cards free, and the power that
        // does it is spent by the very play it pays for.
        upgrading(
            card(
                "CARD.VEILPIERCER",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![attack(10), buff("POWER.VEILPIERCER_POWER", 1)],
            ),
            vec![attack(13), buff("POWER.VEILPIERCER_POWER", 1)],
        ),
    ]
}

/// Misery, played: the blow, and then every lasting debuff the target was
/// carrying before it copied onto everything else standing.
fn misery(damage: i32) -> Vec<Command> {
    vec![Command::WithTargetsDebuffs {
        then: vec![
            attack(damage),
            Command::Power(PowerCmd::CopyBoundDebuffs {
                targets: CreatureSelector::OtherOpponents,
            }),
        ],
    }]
}

/// Oblivion, played: the mark, worked by the play's owner onto whoever it was
/// aimed at.
fn oblivion(amount: i32) -> Command {
    Command::Power(PowerCmd::Apply {
        marks: None,
        power: id("POWER.OBLIVION_POWER"),
        target: CreatureSelector::Target,
        applier: Some(CreatureSelector::Owner),
        amount: Amount::Fixed(amount),
    })
}

/// A flatten: the pet's own heavy swing, and the two places the card writes
/// itself free for the turn once the pet has already swung.
fn flatten() -> CardDefinition {
    let body = |damage: i32| {
        vec![osty_attack(
            Amount::Fixed(damage),
            Amount::Fixed(1),
            CreatureSelector::Target,
            Vec::new(),
        )]
    };
    let mut definition = upgrading(
        card(
            "CARD.FLATTEN",
            CardType::Attack,
            TargetType::AnyEnemy,
            2,
            body(12),
        ),
        body(16),
    );
    definition.hooks = vec![
        // A flatten that joins the fight after the pet has swung is already
        // free, which is a card other than itself entering the fight.
        ModifierHook::every_turn(
            HookEvent::AfterCardEnteredCombat,
            vec![Command::When {
                condition: Condition::OwnersPetAttackedThisTurn,
                then: vec![flatten_is_free()],
                otherwise: Vec::new(),
            }],
        )
        .when(sts2_engine::HookCondition::ListeningCardEnteredCombat),
        // Any attack whose dealer is the owner's own pet, which is every
        // swing it makes wherever this card stands.
        ModifierHook::every_turn(HookEvent::AfterAttack, vec![flatten_is_free()])
            .when(sts2_engine::HookCondition::OwnersPetAttacked),
    ];
    definition
}

/// A right hand hand: the pet's swing, and the listener that calls the card
/// back out of the discard pile whenever its owner spends two on a play.
fn right_hand_hand() -> CardDefinition {
    let body = |damage: i32| {
        vec![osty_attack(
            Amount::Fixed(damage),
            Amount::Fixed(1),
            CreatureSelector::Target,
            Vec::new(),
        )]
    };
    let mut definition = upgrading(
        card(
            "CARD.RIGHT_HAND_HAND",
            CardType::Attack,
            TargetType::AnyEnemy,
            0,
            body(4),
        ),
        body(6),
    );
    definition.hooks = vec![
        ModifierHook::every_turn(
            HookEvent::AfterCardPlayedLate,
            vec![Command::When {
                condition: Condition::SourceCardIsInPile(PileName::Discard),
                then: vec![Command::CardPile(CardPileCmd::MoveToPile {
                    cards: CardSelector::Source,
                    pile: PileName::Hand,
                    position: DrawPilePosition::Bottom,
                })],
                otherwise: Vec::new(),
            }],
        )
        .when(sts2_engine::HookCondition::All(vec![
            sts2_engine::HookCondition::OwnerPlayedTheCard,
            sts2_engine::HookCondition::PlayedCardEnergyAtLeast(2),
        ])),
    ];
    definition
}

/// `9 + 2 per Soul in the exhaust pile`.
fn soul_storm(base: i32, per: i32) -> Command {
    multi_attack(
        Amount::Scaled {
            base,
            per,
            count: Count::CardsInPileOfModel {
                pile: PileName::Exhaust,
                model: id("CARD.SOUL"),
            },
        },
        Amount::Fixed(1),
    )
}

/// The card gains block, which the registry fills in from the table in
/// `cards.rs`; the wrapper marks such a card at its definition, so nothing has
/// to look it up twice.
const fn gaining_block(definition: CardDefinition) -> CardDefinition {
    definition
}

/// What she starts with, and everything else that swings the pet or stands it
/// up.
#[allow(clippy::too_many_lines, reason = "one entry per card")]
fn the_pet_and_the_pool() -> Vec<CardDefinition> {
    vec![
        // Her Strike is every other character's, with her own portrait.
        striking(upgrading(
            card(
                "CARD.STRIKE_NECROBINDER",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![attack(6)],
            ),
            vec![attack(9)],
        )),
        // Her Defend likewise.
        defending(upgrading(
            card(
                "CARD.DEFEND_NECROBINDER",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![block(5)],
            ),
            vec![block(8)],
        )),
        // A bodyguard is a summon and nothing else.
        upgrading(
            card(
                "CARD.BODYGUARD",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![summon(5)],
            ),
            vec![summon(7)],
        ),
        // An unleash swings the pet for six and one more for every point of HP
        // the pet is standing on. The upgrade raises the six.
        osty_tagged(upgrading(
            card(
                "CARD.UNLEASH",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![scaled_by_the_pet(6, 1, Count::PetCurrentHp)],
            ),
            vec![scaled_by_the_pet(9, 1, Count::PetCurrentHp)],
        )),
        // A protector swings for the pet's whole size rather than what is left
        // of it, and the upgrade both raises the base and makes it free. It is
        // the Necrobinder's other ancient, outside the rolled rarities.
        crate::cards::ancient(osty_tagged(upgrading_cost_and_body(
            card(
                "CARD.PROTECTOR",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![scaled_by_the_pet(10, 1, Count::PetMaxHp)],
            ),
            0,
            vec![scaled_by_the_pet(15, 1, Count::PetMaxHp)],
        ))),
        // A poke is the plainest of the pet's swings.
        osty_tagged(upgrading(
            card(
                "CARD.POKE",
                CardType::Attack,
                TargetType::AnyEnemy,
                0,
                vec![osty_strike(6)],
            ),
            vec![osty_strike(9)],
        )),
        // An afterlife is a bigger summon that burns up for it.
        exhausting(upgrading(
            card(
                "CARD.AFTERLIFE",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![summon(6)],
            ),
            vec![summon(9)],
        )),
        // Pulling aggro is a summon and the block it buys.
        upgrading(
            card(
                "CARD.PULL_AGGRO",
                CardType::Skill,
                TargetType::SelfTarget,
                2,
                vec![summon(4), block(7)],
            ),
            vec![summon(5), block(9)],
        ),
        // A reanimation is the biggest summon there is, and it burns up.
        exhausting(upgrading(
            card(
                "CARD.REANIMATE",
                CardType::Skill,
                TargetType::SelfTarget,
                3,
                vec![summon(20)],
            ),
            vec![summon(25)],
        )),
        // A spur summons and then heals what it summoned.
        retaining(upgrading(
            card(
                "CARD.SPUR",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![summon(3), heal_the_pet(5)],
            ),
            vec![summon(5), heal_the_pet(7)],
        )),
        // A cleanse summons and then burns one card out of the draw pile.
        upgrading(
            card(
                "CARD.CLEANSE",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![summon(3), exhaust_one_of_the_draw_pile()],
            ),
            vec![summon(5), exhaust_one_of_the_draw_pile()],
        ),
        // A high five is the one card of the pool that cannot be played at all
        // without a pet: the card itself refuses the play where the pet is
        // missing, rather than being played and doing nothing.
        playable_only_with_a_pet(osty_tagged(upgrading(
            card(
                "CARD.HIGH_FIVE",
                CardType::Attack,
                TargetType::AllEnemies,
                2,
                vec![
                    osty_sweep_all(11),
                    sweeping_power("POWER.VULNERABLE_POWER", 2),
                ],
            ),
            vec![
                osty_sweep_all(13),
                sweeping_power("POWER.VULNERABLE_POWER", 3),
            ],
        ))),
        // Bone shards are the pet thrown at everything and then put down for
        // it: the block lands whether or not the pet was there to throw.
        osty_tagged(upgrading(
            card(
                "CARD.BONE_SHARDS",
                CardType::Attack,
                TargetType::AllEnemies,
                1,
                vec![osty_sweep_all(9), block(9), kill_the_pet()],
            ),
            vec![osty_sweep_all(12), block(12), kill_the_pet()],
        )),
        // A legion of bone summons for every living player, which in a
        // single-player run is one summon.
        rarity(
            exhausting(upgrading(
                card(
                    "CARD.LEGION_OF_BONE",
                    CardType::Skill,
                    TargetType::AllAllies,
                    2,
                    vec![summon(6)],
                ),
                vec![summon(8)],
            )),
            sts2_engine::CardRarity::Uncommon,
        ),
        // An invoke owes next turn a summon and two energy.
        upgrading(
            card(
                "CARD.INVOKE",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![
                    buff("POWER.SUMMON_NEXT_TURN_POWER", 2),
                    buff("POWER.ENERGY_NEXT_TURN_POWER", 2),
                ],
            ),
            vec![
                buff("POWER.SUMMON_NEXT_TURN_POWER", 3),
                buff("POWER.ENERGY_NEXT_TURN_POWER", 3),
            ],
        ),
        // A dirge summons three for every point of energy it was played with,
        // and then shuffles that many Souls in — upgraded ones, if it is.
        exhausting(costing_x(upgrading(
            card(
                "CARD.DIRGE",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![
                    Command::Repeat {
                        times: Amount::counting(Count::EnergyX),
                        then: vec![summon(3)],
                    },
                    many_souls(Amount::counting(Count::EnergyX), false),
                ],
            ),
            vec![
                Command::Repeat {
                    times: Amount::counting(Count::EnergyX),
                    then: vec![summon(4)],
                },
                many_souls(Amount::counting(Count::EnergyX), true),
            ],
        ))),
    ]
}

/// The attacks and skills that ask nothing of the pet.
#[allow(clippy::too_many_lines, reason = "one entry per card")]
fn the_plain_attacks() -> Vec<CardDefinition> {
    vec![
        // A defile is a plain blow that burns up if it is held.
        ethereal(upgrading(
            card(
                "CARD.DEFILE",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![attack(13)],
            ),
            vec![attack(17)],
        )),
        // A defy is block and a weakness, and it burns up if it is held.
        ethereal(upgrading(
            card(
                "CARD.DEFY",
                CardType::Skill,
                TargetType::AnyEnemy,
                1,
                vec![block(6), debuff("POWER.WEAK_POWER", 1)],
            ),
            vec![block(9), debuff("POWER.WEAK_POWER", 1)],
        )),
        // A bury is one enormous swing.
        upgrading(
            card(
                "CARD.BURY",
                CardType::Attack,
                TargetType::AnyEnemy,
                4,
                vec![attack(52)],
            ),
            vec![attack(63)],
        ),
        // A reap is a big one that keeps itself in hand.
        retaining(upgrading(
            card(
                "CARD.REAP",
                CardType::Attack,
                TargetType::AnyEnemy,
                3,
                vec![attack(27)],
            ),
            vec![attack(33)],
        )),
        // A sow lands on everything and keeps itself in hand.
        retaining(upgrading(
            card(
                "CARD.SOW",
                CardType::Attack,
                TargetType::AllEnemies,
                1,
                vec![sweep(8, 1)],
            ),
            vec![sweep(11, 1)],
        )),
        banshees_cry(),
        // A fear is a small blow and the vulnerability behind it.
        ethereal(upgrading(
            card(
                "CARD.FEAR",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![attack(7), debuff("POWER.VULNERABLE_POWER", 1)],
            ),
            vec![attack(8), debuff("POWER.VULNERABLE_POWER", 2)],
        )),
        // A putrefy is both debuffs at once and burns up for it.
        exhausting(upgrading(
            card(
                "CARD.PUTREFY",
                CardType::Skill,
                TargetType::AnyEnemy,
                1,
                vec![
                    debuff("POWER.WEAK_POWER", 2),
                    debuff("POWER.VULNERABLE_POWER", 2),
                ],
            ),
            vec![
                debuff("POWER.WEAK_POWER", 3),
                debuff("POWER.VULNERABLE_POWER", 3),
            ],
        )),
        // A debilitate is a blow and the power that doubles what a weakness and
        // a vulnerability are worth on whoever took it.
        upgrading(
            card(
                "CARD.DEBILITATE",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![attack(10), debuff("POWER.DEBILITATE_POWER", 2)],
            ),
            vec![attack(12), debuff("POWER.DEBILITATE_POWER", 3)],
        ),
        // An enfeebling touch lends the enemy eight strength the wrong way for
        // the turn, and burns up if it is held.
        ethereal(upgrading(
            card(
                "CARD.ENFEEBLING_TOUCH",
                CardType::Skill,
                TargetType::AnyEnemy,
                1,
                vec![debuff("POWER.ENFEEBLING_TOUCH_POWER", 8)],
            ),
            vec![debuff("POWER.ENFEEBLING_TOUCH_POWER", 11)],
        )),
        // A shared fate costs both sides the same two points of strength.
        exhausting(upgrading(
            card(
                "CARD.SHARED_FATE",
                CardType::Skill,
                TargetType::AnyEnemy,
                0,
                vec![
                    lose_strength(CreatureSelector::Owner, 2),
                    lose_strength(CreatureSelector::Target, 2),
                ],
            ),
            vec![
                lose_strength(CreatureSelector::Owner, 2),
                lose_strength(CreatureSelector::Target, 3),
            ],
        )),
        // A hang is a blow and a mark that doubles every hang after it: the
        // mark it leaves is the larger of two and what the target already
        // carries, so it goes two, four, eight.
        upgrading(
            card(
                "CARD.HANG",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![attack(10), hang_mark()],
            ),
            vec![attack(13), hang_mark()],
        ),
        // A capture spirit is a small unblockable cut and three Souls shuffled
        // in behind it.
        upgrading(
            card(
                "CARD.CAPTURE_SPIRIT",
                CardType::Skill,
                TargetType::AnyEnemy,
                1,
                vec![
                    unblockable_hp_loss(3),
                    souls(3, PileName::Draw, DrawPilePosition::Random, false),
                ],
            ),
            vec![
                unblockable_hp_loss(4),
                souls(4, PileName::Draw, DrawPilePosition::Random, false),
            ],
        ),
        // A grave warden is block and one Soul.
        upgrading(
            card(
                "CARD.GRAVE_WARDEN",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![
                    block(8),
                    souls(1, PileName::Draw, DrawPilePosition::Random, false),
                ],
            ),
            vec![
                block(11),
                souls(1, PileName::Draw, DrawPilePosition::Random, false),
            ],
        ),
        // A reave is a blow and one Soul, upgraded with the card.
        upgrading(
            card(
                "CARD.REAVE",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![
                    attack(9),
                    souls(1, PileName::Draw, DrawPilePosition::Random, false),
                ],
            ),
            vec![
                attack(11),
                souls(1, PileName::Draw, DrawPilePosition::Random, true),
            ],
        ),
        // A severance is a blow and three Souls, one to each of three piles.
        upgrading(
            card(
                "CARD.SEVERANCE",
                CardType::Attack,
                TargetType::AnyEnemy,
                2,
                vec![attack(13), severance_souls()],
            ),
            vec![attack(18), severance_souls()],
        ),
        // A glimpse beyond hands every living player three Souls, which in a
        // single-player run is three.
        rarity(
            exhausting(upgrading(
                card(
                    "CARD.GLIMPSE_BEYOND",
                    CardType::Skill,
                    TargetType::AllAllies,
                    1,
                    vec![souls(3, PileName::Draw, DrawPilePosition::Random, false)],
                ),
                vec![souls(4, PileName::Draw, DrawPilePosition::Random, false)],
            )),
            sts2_engine::CardRarity::Rare,
        ),
        // A delay is block and an energy owed next turn.
        upgrading(
            card(
                "CARD.DELAY",
                CardType::Skill,
                TargetType::SelfTarget,
                2,
                vec![block(11), buff("POWER.ENERGY_NEXT_TURN_POWER", 1)],
            ),
            vec![block(13), buff("POWER.ENERGY_NEXT_TURN_POWER", 2)],
        ),
        // A parse is three cards and burns up if it is held.
        ethereal(upgrading(
            card(
                "CARD.PARSE",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![draw(3)],
            ),
            vec![draw(4)],
        )),
        // A wisp is an energy and gone.
        exhausting(retain_when_upgraded(upgrading(
            card(
                "CARD.WISP",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![energy(1)],
            ),
            vec![energy(1)],
        ))),
        // Borrowed time is four energy now and every card of the turn costing
        // one more for it.
        upgrading(
            card(
                "CARD.BORROWED_TIME",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![energy(4), buff("POWER.BORROWED_TIME_POWER", 1)],
            ),
            vec![energy(6), buff("POWER.BORROWED_TIME_POWER", 1)],
        ),
        // An undeath is block and another of itself in the discard pile.
        upgrading(
            card(
                "CARD.UNDEATH",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![block(7), a_copy_of_itself()],
            ),
            vec![block(9), a_copy_of_itself()],
        ),
    ]
}

/// The cards that put a screen up, and the ones written around a Soul.
#[allow(clippy::too_many_lines, reason = "one entry per card")]
fn the_souls_and_the_screens() -> Vec<CardDefinition> {
    vec![
        // A séance turns one card of the draw pile into a Soul.
        upgrading_cost(
            ethereal(card(
                "CARD.SEANCE",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![Command::CardSelect(CardSelectCmd::FromCombatPile {
                    semantics: ChoiceSemantics::known(vec![
                        ChoiceOp::new(ChoiceOperationKind::Transform, ChoiceScope::Combat)
                            .model(id("CARD.SOUL")),
                    ]),
                    pile: PileName::Draw,
                    prefs: SelectorPrefs::exactly(1),
                    maximum: None,
                    exact_count: None,
                    filter: CardFilter::Any,
                    restricted_to_chosen: false,
                    then: vec![Command::Card(CardCmd::Transform {
                        cards: CardSelector::Chosen,
                        into: id("CARD.SOUL"),
                        upgraded: false,
                    })],
                })],
            )),
            0,
        ),
        // A graveblast is a small blow and one card back out of the discard
        // pile; upgrading takes the burn off it.
        upgrade_drops_exhaust(upgrading(
            exhausting(card(
                "CARD.GRAVEBLAST",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![attack(4), one_card_back_from_the_discard()],
            )),
            vec![attack(6), one_card_back_from_the_discard()],
        )),
        // A sculpting strike is a blow and one card of the hand made ethereal.
        striking(upgrading(
            card(
                "CARD.SCULPTING_STRIKE",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![attack(9), make_a_hand_card_ethereal()],
            ),
            vec![attack(12), make_a_hand_card_ethereal()],
        )),
        // A snap is the pet's blow and one card of the hand kept: the keyword
        // is put on whether or not the pet was there to swing.
        osty_tagged(upgrading(
            card(
                "CARD.SNAP",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![osty_strike(7), make_a_hand_card_retain()],
            ),
            vec![osty_strike(10), make_a_hand_card_retain()],
        )),
    ]
}

/// The powers she plays, which are one application each.
fn the_powers() -> Vec<CardDefinition> {
    vec![
        // A demesne is a card and an energy every turn, and it burns up if it
        // is held.
        upgrading_cost(
            ethereal(card(
                "CARD.DEMESNE",
                CardType::Power,
                TargetType::SelfTarget,
                3,
                vec![buff("POWER.DEMESNE_POWER", 1)],
            )),
            2,
        ),
        // A friendship trades two points of strength for the energy.
        upgrading(
            card(
                "CARD.FRIENDSHIP",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![
                    lose_strength(CreatureSelector::Owner, 2),
                    buff("POWER.FRIENDSHIP_POWER", 1),
                ],
            ),
            vec![
                lose_strength(CreatureSelector::Owner, 1),
                buff("POWER.FRIENDSHIP_POWER", 1),
            ],
        ),
        // A pagestorm draws another card for every ethereal one drawn.
        upgrading_cost(
            card(
                "CARD.PAGESTORM",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.PAGESTORM_POWER", 1)],
            ),
            0,
        ),
        // A sentry mode deals a Sweeping Gaze with every hand.
        upgrading_cost(
            card(
                "CARD.SENTRY_MODE",
                CardType::Power,
                TargetType::SelfTarget,
                2,
                vec![buff("POWER.SENTRY_MODE_POWER", 1)],
            ),
            1,
        ),
        // A calcify adds four to every blow the pet lands.
        upgrading(
            card(
                "CARD.CALCIFY",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.CALCIFY_POWER", 4)],
            ),
            vec![buff("POWER.CALCIFY_POWER", 6)],
        ),
        // A call of the void deals a random ethereal card of her own pool with
        // every hand. Which cards those are is a creation against the character
        // pool, and its upgrade only makes the card innate.
        innate_when_upgraded(upgrading(
            card(
                "CARD.CALL_OF_THE_VOID",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.CALL_OF_THE_VOID_POWER", 1)],
            ),
            vec![buff("POWER.CALL_OF_THE_VOID_POWER", 1)],
        )),
        // A forbidden grimoire pays a card removal at the end of every fight,
        // and can never leave the deck. It is one of the Necrobinder's two
        // ancients: outside the rolled rarities, and never generated in a
        // fight, because an ancient is filtered out of any roll made there.
        crate::cards::ancient(eternal(upgrading_cost(
            card(
                "CARD.FORBIDDEN_GRIMOIRE",
                CardType::Power,
                TargetType::SelfTarget,
                2,
                vec![buff("POWER.FORBIDDEN_GRIMOIRE_POWER", 1)],
            ),
            1,
        ))),
    ]
}

/// An upgrade that takes that much off the cost alongside a body that also
/// changes.
fn upgrading_cost_and_body(
    definition: CardDefinition,
    energy_cost: i32,
    effects: Vec<Command>,
) -> CardDefinition {
    let mut definition = upgrading(definition, effects);
    if let Some(upgrade) = definition.upgrade.as_mut() {
        upgrade.energy_cost = Some(energy_cost);
    }
    definition
}

/// The card can only be played while its owner's pet is standing.
fn playable_only_with_a_pet(mut definition: CardDefinition) -> CardDefinition {
    definition.playable_when = Some(Condition::PetIsStanding);
    definition
}

/// That much healing for the owner's own pet.
fn heal_the_pet(amount: i32) -> Command {
    Command::Creature(CreatureCmd::Heal {
        target: CreatureSelector::Pet,
        amount: Amount::Fixed(amount),
    })
}

/// The owner's own pet is put down, where it is still standing.
fn kill_the_pet() -> Command {
    Command::When {
        condition: Condition::PetIsStanding,
        then: vec![Command::Creature(CreatureCmd::Kill {
            target: CreatureSelector::Pet,
        })],
        otherwise: Vec::new(),
    }
}

/// Unblockable, unpowered damage: HP straight off, with nothing to scale it
/// and nothing to stop it.
fn unblockable_hp_loss(amount: i32) -> Command {
    Command::Creature(CreatureCmd::Damage {
        target: CreatureSelector::Target,
        amount: Amount::Fixed(amount),
        props: ValueProps::UNBLOCKABLE,
    })
}

/// A hang mark on the target, worth two or what it is already carrying,
/// whichever is more.
fn hang_mark() -> Command {
    Command::Power(PowerCmd::Apply {
        marks: None,
        power: id("POWER.HANG_POWER"),
        target: CreatureSelector::Target,
        applier: Some(CreatureSelector::Owner),
        amount: Amount::Greatest(vec![
            Amount::Fixed(2),
            Amount::counting(Count::TargetPower(id("POWER.HANG_POWER"))),
        ]),
    })
}

/// Three Souls, one to the draw pile, one to the discard and one to the hand —
/// three separate adds rather than one.
fn severance_souls() -> Command {
    Command::Repeat {
        times: Amount::Fixed(1),
        then: vec![
            souls(1, PileName::Draw, DrawPilePosition::Random, false),
            souls(1, PileName::Discard, DrawPilePosition::Bottom, false),
            souls(1, PileName::Hand, DrawPilePosition::Bottom, false),
        ],
    }
}

/// One card picked out of the draw pile, and the burn behind it.
fn exhaust_one_of_the_draw_pile() -> Command {
    Command::CardSelect(CardSelectCmd::FromCombatPile {
        semantics: ChoiceSemantics::known(vec![
            ChoiceOp::new(ChoiceOperationKind::Exhaust, ChoiceScope::Combat)
                .to(ChoiceDestination::Exhaust),
        ]),
        pile: PileName::Draw,
        prefs: SelectorPrefs::exactly(1),
        maximum: None,
        exact_count: None,
        filter: CardFilter::Any,
        restricted_to_chosen: false,
        then: vec![Command::Card(CardCmd::Exhaust {
            cards: CardSelector::Chosen,
        })],
    })
}

/// One card picked out of the discard pile, and taken back into the hand.
fn one_card_back_from_the_discard() -> Command {
    Command::CardSelect(CardSelectCmd::FromCombatPile {
        semantics: ChoiceSemantics::known(vec![
            ChoiceOp::new(ChoiceOperationKind::Move, ChoiceScope::Combat)
                .to(ChoiceDestination::Hand),
        ]),
        pile: PileName::Discard,
        prefs: SelectorPrefs::exactly(1),
        maximum: None,
        exact_count: None,
        filter: CardFilter::Any,
        restricted_to_chosen: false,
        then: vec![Command::CardPile(CardPileCmd::MoveToPile {
            cards: CardSelector::Chosen,
            pile: PileName::Hand,
            position: DrawPilePosition::Bottom,
        })],
    })
}

/// One card of the hand that is not already ethereal, made ethereal.
fn make_a_hand_card_ethereal() -> Command {
    Command::CardSelect(CardSelectCmd::FromHand {
        semantics: ChoiceSemantics::known(vec![
            ChoiceOp::new(ChoiceOperationKind::GrantKeyword, ChoiceScope::Combat)
                .keyword(sts2_engine::CardKeyword::Ethereal)
                .lasting(ChoiceDuration::Combat),
        ]),
        prefs: SelectorPrefs::exactly(1),
        maximum: None,
        exact_count: None,
        filter: CardFilter::NotEthereal,
        then: vec![Command::Card(CardCmd::ApplyKeyword {
            cards: CardSelector::Chosen,
            keyword: sts2_engine::CardKeyword::Ethereal,
        })],
    })
}

/// One card of the hand that is not already kept, given Retain.
fn make_a_hand_card_retain() -> Command {
    Command::CardSelect(CardSelectCmd::FromHand {
        semantics: ChoiceSemantics::known(vec![
            ChoiceOp::new(ChoiceOperationKind::GrantKeyword, ChoiceScope::Combat)
                .keyword(sts2_engine::CardKeyword::Retain)
                .lasting(ChoiceDuration::Combat),
        ]),
        prefs: SelectorPrefs::exactly(1),
        maximum: None,
        exact_count: None,
        filter: CardFilter::NotRetained,
        then: vec![Command::Card(CardCmd::ApplyKeyword {
            cards: CardSelector::Chosen,
            keyword: sts2_engine::CardKeyword::Retain,
        })],
    })
}

/// A copy of the card put into the discard pile, which is what an undeath
/// leaves behind.
fn a_copy_of_itself() -> Command {
    Command::CardPile(CardPileCmd::AddGeneratedToCombat {
        enchantment: None,
        card: CardOrigin::CloneOfSource,
        grown_by: Amount::Fixed(0),
        count: Amount::Fixed(1),
        pile: PileName::Discard,
        ethereal: false,
        retain: false,
        upgraded: false,
        temporary_cost: None,
        position: DrawPilePosition::Bottom,
    })
}

/// A blow that lands once and reads no state: kept here so the multi-attack
/// helper is not imported for a single use.
#[allow(dead_code, reason = "kept beside the pool's other shapes")]
fn plain_multi_attack(amount: i32, hits: i32) -> Command {
    multi_attack(Amount::Fixed(amount), Amount::Fixed(hits))
}
