//! The twenty-three enchantments of the pinned build.
//!
//! An enchantment lives on a card. It says what cards it may go on, changes
//! the card as it lands, adds a body to the card's play, answers the five
//! enchantment value passes that stand in front of every other modifier, and
//! listens on the hook list right behind the card carrying it.
//!
//! The one enchantment left out is the game's free mock, which is test
//! scaffolding the build ships for its own tests and no run can reach.

use sts2_engine::{
    Amount, CardChange, CardCmd, CardKeyword, CardPileCmd, CardSelector, CardTag, CardType,
    Command, Condition, Count, CreatureCmd, CreatureSelector, DrawPilePosition, EnchantRestriction,
    EnchantmentDefinition, EnchantmentStatus, HookCondition, HookEvent, ModifierCmd, ModifierHook,
    PileName, PlayerCmd, PowerCmd, ValueProps,
};

use crate::id;

/// An enchantment with nothing declared but what it may go on.
fn enchantment(model_id: &str, restriction: EnchantRestriction) -> EnchantmentDefinition {
    EnchantmentDefinition {
        model_id: id(model_id),
        // No enchantment of the pinned build is stackable.
        stackable: false,
        restriction,
        starts_at_bottom_of_draw_pile: false,
        on_enchant: Vec::new(),
        on_play: Vec::new(),
        value_hooks: Vec::new(),
        hooks: Vec::new(),
    }
}

/// The card the running enchantment is attached to.
fn own_card() -> CardSelector {
    CardSelector::Source
}

/// An enchantment effect whose only condition is that the value it is changing
/// belongs to a powered attack.
fn powered(event: HookEvent, commands: Vec<Command>) -> ModifierHook {
    ModifierHook::every_turn(event, commands).when(HookCondition::ValueIsAPoweredAttack)
}

/// The amount the enchantment that is listening carries.
fn amount() -> Amount {
    Amount::counting(Count::ListeningEnchantmentAmount)
}

/// Adroit: block worth its amount, every time the card is played.
///
/// The enchantment's block value is its amount, and playing the card gains
/// that much. That value is a powered one, so the usual block passes scale it.
fn adroit() -> EnchantmentDefinition {
    EnchantmentDefinition {
        on_play: vec![Command::Creature(CreatureCmd::GainBlock {
            target: CreatureSelector::Owner,
            amount: amount(),
            props: ValueProps::NONE,
        })],
        ..enchantment("ENCHANTMENT.ADROIT", EnchantRestriction::Any)
    }
}

/// Clone: nothing of its own.
///
/// It does nothing but mark the card, and the rest site's clone option is what
/// reads the mark.
fn clone_enchantment() -> EnchantmentDefinition {
    enchantment("ENCHANTMENT.CLONE", EnchantRestriction::Any)
}

/// Corrupted: half again as much damage, and two unblockable to its owner
/// every time the card is played.
fn corrupted() -> EnchantmentDefinition {
    EnchantmentDefinition {
        value_hooks: vec![powered(
            HookEvent::EnchantDamageMultiplicative,
            vec![Command::Modify(ModifierCmd::Scale(Amount::Fixed(150)))],
        )],
        on_play: vec![Command::Creature(CreatureCmd::Damage {
            target: CreatureSelector::Owner,
            amount: Amount::Fixed(2),
            props: ValueProps::UNBLOCKABLE,
        })],
        ..enchantment(
            "ENCHANTMENT.CORRUPTED",
            EnchantRestriction::OfType(CardType::Attack),
        )
    }
}

/// The deprecated enchantment: what a save's enchantment becomes when the
/// build no longer has the one it names. It does nothing.
fn deprecated_enchantment() -> EnchantmentDefinition {
    enchantment(
        "ENCHANTMENT.DEPRECATED_ENCHANTMENT",
        EnchantRestriction::Any,
    )
}

/// Glam: the first play of the card each combat runs its body once more.
///
/// It adds one to the card's play count until the card has been played, and
/// the play then marks it spent for the fight and disables it. The count is
/// settled before the replays begin, so the play that spends the replay is the
/// play that gets it.
fn glam() -> EnchantmentDefinition {
    EnchantmentDefinition {
        value_hooks: vec![
            ModifierHook::every_turn(
                HookEvent::EnchantPlayCount,
                vec![Command::Modify(ModifierCmd::Add(Amount::Fixed(1)))],
            )
            .when(HookCondition::Not(Box::new(
                HookCondition::ListeningEnchantmentLocalIsSet("UsedThisCombat".to_owned()),
            ))),
        ],
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterCardPlayed,
                vec![
                    Command::Card(CardCmd::ModifyEnchantmentLocal {
                        cards: own_card(),
                        name: "UsedThisCombat".to_owned(),
                        amount: Amount::Fixed(1),
                    }),
                    Command::Card(CardCmd::SetEnchantmentStatus {
                        cards: own_card(),
                        status: EnchantmentStatus::Disabled,
                    }),
                ],
            )
            .when(HookCondition::All(vec![
                HookCondition::PlayedCardCarriesTheListeningEnchantment,
                HookCondition::Not(Box::new(HookCondition::ListeningEnchantmentLocalIsSet(
                    "UsedThisCombat".to_owned(),
                ))),
            ])),
        ],
        ..enchantment("ENCHANTMENT.GLAM", EnchantRestriction::Any)
    }
}

/// Goopy: the Defend it is on burns up, and works up one more block every
/// time it is played.
///
/// The amount starts where the enchant put it and grows by one per play; the
/// block it adds is `Amount - 1`, so an enchant of one is worth nothing on the
/// first play. The growth is written onto the run-deck card as well, so it
/// outlives the fight.
fn goopy() -> EnchantmentDefinition {
    EnchantmentDefinition {
        on_enchant: vec![CardChange::AddKeyword(CardKeyword::Exhaust)],
        value_hooks: vec![ModifierHook::every_turn(
            HookEvent::EnchantBlockAdditive,
            vec![Command::Modify(ModifierCmd::Add(Amount::Scaled {
                base: -1,
                per: 1,
                count: Count::ListeningEnchantmentAmount,
            }))],
        )],
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterCardPlayed,
                vec![Command::Card(CardCmd::ModifyEnchantmentAmount {
                    cards: own_card(),
                    amount: Amount::Fixed(1),
                    and_the_deck_version: true,
                })],
            )
            .when(HookCondition::PlayedCardCarriesTheListeningEnchantment),
        ],
        ..enchantment(
            "ENCHANTMENT.GOOPY",
            EnchantRestriction::Tagged(CardTag::Defend),
        )
    }
}

/// Imbued: the skill it is on starts at the bottom of the draw pile and plays
/// itself on the opening turn.
fn imbued() -> EnchantmentDefinition {
    EnchantmentDefinition {
        starts_at_bottom_of_draw_pile: true,
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterAutoPrePlayPhaseEntered,
                vec![Command::Card(CardCmd::AutoPlay {
                    cards: own_card(),
                    at_the_bodys_target: false,
                })],
            )
            .timed(sts2_engine::HookTiming::FirstTurnOnly),
        ],
        ..enchantment(
            "ENCHANTMENT.IMBUED",
            EnchantRestriction::OfType(CardType::Skill),
        )
    }
}

/// Inky: one more damage, and one Weak on whatever the play reached.
///
/// The debuff goes on the play's own target unless the card aims at the whole
/// enemy side, in which case it goes on every enemy that can be hit.
fn inky() -> EnchantmentDefinition {
    let weak = |target: CreatureSelector| {
        Command::Power(PowerCmd::Apply {
            marks: None,
            power: id("POWER.WEAK_POWER"),
            target,
            applier: Some(CreatureSelector::Owner),
            amount: Amount::Fixed(1),
        })
    };
    EnchantmentDefinition {
        value_hooks: vec![powered(
            HookEvent::EnchantDamageAdditive,
            vec![Command::Modify(ModifierCmd::Add(Amount::Fixed(1)))],
        )],
        on_play: vec![
            Command::When {
                otherwise: Vec::new(),
                condition: Condition::SourceCardTargetsAllEnemies,
                then: vec![weak(CreatureSelector::AllOpponents)],
            },
            Command::When {
                otherwise: Vec::new(),
                condition: Condition::Not(Box::new(Condition::SourceCardTargetsAllEnemies)),
                then: vec![weak(CreatureSelector::Target)],
            },
        ],
        ..enchantment("ENCHANTMENT.INKY", EnchantRestriction::Any)
    }
}

/// Instinct: twice the damage.
fn instinct() -> EnchantmentDefinition {
    EnchantmentDefinition {
        value_hooks: vec![powered(
            HookEvent::EnchantDamageMultiplicative,
            vec![Command::Modify(ModifierCmd::Scale(Amount::Fixed(200)))],
        )],
        ..enchantment(
            "ENCHANTMENT.INSTINCT",
            EnchantRestriction::OfType(CardType::Attack),
        )
    }
}

/// Momentum: the attack it is on gains its amount in damage every time it is
/// played, for the rest of the fight.
///
/// The damage it has worked up is combat-local rather than part of its amount,
/// so it starts over with each combat while the amount does not. Each play adds
/// to the running total before the next play reads it, and the play that adds
/// is not the play that gets it: that play's damage was already settled.
fn momentum() -> EnchantmentDefinition {
    EnchantmentDefinition {
        value_hooks: vec![powered(
            HookEvent::EnchantDamageAdditive,
            vec![Command::Modify(ModifierCmd::Add(Amount::counting(
                Count::ListeningEnchantmentLocal("ExtraDamage".to_owned()),
            )))],
        )],
        on_play: vec![Command::Card(CardCmd::ModifyEnchantmentLocal {
            cards: own_card(),
            name: "ExtraDamage".to_owned(),
            amount: amount(),
        })],
        ..enchantment(
            "ENCHANTMENT.MOMENTUM",
            EnchantRestriction::OfType(CardType::Attack),
        )
    }
}

/// Nimble: the card's block is worth its amount more.
fn nimble() -> EnchantmentDefinition {
    EnchantmentDefinition {
        value_hooks: vec![ModifierHook::every_turn(
            HookEvent::EnchantBlockAdditive,
            vec![Command::Modify(ModifierCmd::Add(amount()))],
        )],
        ..enchantment("ENCHANTMENT.NIMBLE", EnchantRestriction::GainsBlock)
    }
}

/// Perfect Fit: the card is put on top of the draw pile after every shuffle
/// but the one a combat opens with.
fn perfect_fit() -> EnchantmentDefinition {
    EnchantmentDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::ModifyShuffleOrder,
                vec![Command::CardPile(CardPileCmd::MoveToPile {
                    cards: own_card(),
                    pile: PileName::Draw,
                    position: DrawPilePosition::Top,
                })],
            )
            // The cards being shuffled are the draw pile, so the test is
            // whether the enchanted card is standing in it.
            .when(HookCondition::EnchantedCardIsInPile(PileName::Draw)),
        ],
        ..enchantment("ENCHANTMENT.PERFECT_FIT", EnchantRestriction::Any)
    }
}

/// Royally Approved: the attack or skill it is on is Innate and is retained.
fn royally_approved() -> EnchantmentDefinition {
    EnchantmentDefinition {
        on_enchant: vec![
            CardChange::AddKeyword(CardKeyword::Innate),
            CardChange::AddKeyword(CardKeyword::Retain),
        ],
        ..enchantment(
            "ENCHANTMENT.ROYALLY_APPROVED",
            EnchantRestriction::AttackOrSkill,
        )
    }
}

/// Sharp: the attack deals its amount more.
fn sharp() -> EnchantmentDefinition {
    EnchantmentDefinition {
        value_hooks: vec![powered(
            HookEvent::EnchantDamageAdditive,
            vec![Command::Modify(ModifierCmd::Add(amount()))],
        )],
        ..enchantment(
            "ENCHANTMENT.SHARP",
            EnchantRestriction::OfType(CardType::Attack),
        )
    }
}

/// Slither: the card takes a cost rolled for the rest of the fight every time
/// it is drawn into the hand.
fn slither() -> EnchantmentDefinition {
    EnchantmentDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterCardDrawn,
                vec![Command::Card(CardCmd::RandomizeCost {
                    cards: own_card(),
                    for_the_combat: true,
                })],
            )
            .when(HookCondition::All(vec![
                HookCondition::PlayedCardCarriesTheListeningEnchantment,
                HookCondition::EnchantedCardIsInPile(PileName::Hand),
            ])),
        ],
        ..enchantment(
            "ENCHANTMENT.SLITHER",
            EnchantRestriction::PlayableWithAFixedCost,
        )
    }
}

/// Slumbering Essence: the card costs one less until it is played, for every
/// turn it was still in hand as the hand went down.
fn slumbering_essence() -> EnchantmentDefinition {
    EnchantmentDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::BeforeFlush,
                vec![Command::Card(CardCmd::AddCostUntilPlayed {
                    cards: own_card(),
                    amount: Amount::Fixed(-1),
                })],
            )
            .when(HookCondition::EnchantedCardIsInPile(PileName::Hand)),
        ],
        ..enchantment("ENCHANTMENT.SLUMBERING_ESSENCE", EnchantRestriction::Any)
    }
}

/// Souls Power: the card stops burning up.
///
/// It can only go on a card whose own keywords hold Exhaust, so it always has
/// one to take away.
fn souls_power() -> EnchantmentDefinition {
    EnchantmentDefinition {
        on_enchant: vec![CardChange::RemoveKeyword(CardKeyword::Exhaust)],
        ..enchantment(
            "ENCHANTMENT.SOULS_POWER",
            EnchantRestriction::LocallyExhausts,
        )
    }
}

/// Sown: the first play of the card pays its amount in energy, once ever.
///
/// The status is the enchantment's own and nothing puts it back, so this is a
/// once-per-run payout rather than a once-per-combat one — but that status is
/// not saved, so the combat copy starts each fight from the deck card's, which
/// nothing ever disables.
fn sown() -> EnchantmentDefinition {
    EnchantmentDefinition {
        on_play: vec![Command::When {
            otherwise: Vec::new(),
            condition: Condition::ListeningEnchantmentIsNormal,
            then: vec![
                Command::Player(PlayerCmd::GainEnergy(amount())),
                Command::Card(CardCmd::SetEnchantmentStatus {
                    cards: own_card(),
                    status: EnchantmentStatus::Disabled,
                }),
            ],
        }],
        ..enchantment("ENCHANTMENT.SOWN", EnchantRestriction::Any)
    }
}

/// Spiral: the basic Strike or Defend it is on runs its body once more, every
/// time.
fn spiral() -> EnchantmentDefinition {
    EnchantmentDefinition {
        value_hooks: vec![ModifierHook::every_turn(
            HookEvent::EnchantPlayCount,
            vec![Command::Modify(ModifierCmd::Add(Amount::Fixed(1)))],
        )],
        ..enchantment(
            "ENCHANTMENT.SPIRAL",
            EnchantRestriction::BasicStrikeOrDefend,
        )
    }
}

/// Steady: the card is retained.
fn steady() -> EnchantmentDefinition {
    EnchantmentDefinition {
        on_enchant: vec![CardChange::AddKeyword(CardKeyword::Retain)],
        ..enchantment("ENCHANTMENT.STEADY", EnchantRestriction::Any)
    }
}

/// Swift: the first play of the card draws its amount, once ever.
fn swift() -> EnchantmentDefinition {
    EnchantmentDefinition {
        on_play: vec![Command::When {
            otherwise: Vec::new(),
            condition: Condition::ListeningEnchantmentIsNormal,
            then: vec![
                Command::CardPile(CardPileCmd::Draw { count: amount() }),
                Command::Card(CardCmd::SetEnchantmentStatus {
                    cards: own_card(),
                    status: EnchantmentStatus::Disabled,
                }),
            ],
        }],
        ..enchantment("ENCHANTMENT.SWIFT", EnchantRestriction::Any)
    }
}

/// Tezcatara's Ember: the card becomes free, can never leave the deck, and
/// deals three more.
fn tezcataras_ember() -> EnchantmentDefinition {
    EnchantmentDefinition {
        on_enchant: vec![
            CardChange::CostBecomesFree,
            CardChange::AddKeyword(CardKeyword::Eternal),
        ],
        value_hooks: vec![powered(
            HookEvent::EnchantDamageAdditive,
            vec![Command::Modify(ModifierCmd::Add(Amount::Fixed(3)))],
        )],
        ..enchantment("ENCHANTMENT.TEZCATARAS_EMBER", EnchantRestriction::Any)
    }
}

/// Vigorous: the attack deals its amount more, once per fight.
///
/// The bonus is read as the damage is worked out and the play that got it
/// disables the enchantment afterwards, so a card played twice in one fight is
/// only ever worth it once.
fn vigorous() -> EnchantmentDefinition {
    EnchantmentDefinition {
        value_hooks: vec![
            ModifierHook::every_turn(
                HookEvent::EnchantDamageAdditive,
                vec![Command::Modify(ModifierCmd::Add(amount()))],
            )
            .when(HookCondition::All(vec![
                HookCondition::ListeningEnchantmentIsNormal,
                HookCondition::ValueIsAPoweredAttack,
            ])),
        ],
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterCardPlayed,
                vec![Command::Card(CardCmd::SetEnchantmentStatus {
                    cards: own_card(),
                    status: EnchantmentStatus::Disabled,
                })],
            )
            .when(HookCondition::PlayedCardCarriesTheListeningEnchantment),
        ],
        ..enchantment(
            "ENCHANTMENT.VIGOROUS",
            EnchantRestriction::OfType(CardType::Attack),
        )
    }
}

/// Every enchantment the pinned build declares, less the mock.
#[must_use]
pub fn all_enchantments() -> Vec<EnchantmentDefinition> {
    vec![
        adroit(),
        clone_enchantment(),
        corrupted(),
        deprecated_enchantment(),
        glam(),
        goopy(),
        imbued(),
        inky(),
        instinct(),
        momentum(),
        nimble(),
        perfect_fit(),
        royally_approved(),
        sharp(),
        slither(),
        slumbering_essence(),
        souls_power(),
        sown(),
        spiral(),
        steady(),
        swift(),
        tezcataras_ember(),
        vigorous(),
    ]
}
