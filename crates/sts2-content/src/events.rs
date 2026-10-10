//! Event definitions of the pinned build.

use sts2_engine::{
    AncientRelicGroup, CardTag, EventAmount, EventCardFilter, EventCondition, EventDefinition,
    EventEffect, EventGeneration, EventOption,
};
use sts2_engine::{
    ChoiceDestination, ChoiceOperation as ChoiceOp, ChoiceOperationKind, ChoiceScope,
    ChoiceSemantics,
};

use crate::id;

fn group(relics: &[&str]) -> AncientRelicGroup {
    AncientRelicGroup {
        relics: relics.iter().map(|relic| id(relic)).collect(),
        only_in_act: None,
    }
}

/// A group only this act is offered.
fn act_group(act: usize, relics: &[&str]) -> AncientRelicGroup {
    AncientRelicGroup {
        only_in_act: Some(act),
        ..group(relics)
    }
}

/// Darv, the ancient who deals relics.
#[must_use]
pub fn darv() -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.DARV"),
        combat_layout: None,
        heals_on_entry: true,
        allowed: EventCondition::Always,
        vars: Vec::new(),
        generation: EventGeneration::AncientRelics {
            groups: vec![
                group(&["RELIC.ASTROLABE"]),
                group(&["RELIC.BLACK_STAR"]),
                group(&["RELIC.CALLING_BELL"]),
                group(&["RELIC.EMPTY_CAGE"]),
                // Pandora's Box is off the table when a modifier empties the
                // deck, which the supported baseline never does.
                group(&["RELIC.PANDORAS_BOX"]),
                group(&["RELIC.RUNIC_PYRAMID"]),
                group(&["RELIC.SNECKO_EYE"]),
                act_group(1, &["RELIC.ECTOPLASM", "RELIC.SOZU"]),
                act_group(2, &["RELIC.PHILOSOPHERS_STONE", "RELIC.VELVET_CHOKER"]),
            ],
            keepsake: id("RELIC.DUSTY_TOME"),
            // The tome offers the ancient cards of the character's own pool,
            // minus the ones the Archaic Tooth turns a starter into. The
            // Ironclad's pool holds two ancients and
            // the tooth claims one of them, so one is left — and the roll
            // between one still reads a die.
            keepsake_cards: vec![id("CARD.CORRUPTION")],
        },
    }
}

/// The Stone of All Time: drink a potion the stone picks and grow, or push it
/// over in blood for an enchantment.
#[must_use]
pub fn stone_of_all_time() -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.STONE_OF_ALL_TIME"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::All(vec![
            EventCondition::ActIndexIs(1),
            EventCondition::PotionsAtLeast(1),
        ]),
        vars: Vec::new(),
        generation: EventGeneration::Conditional(vec![
            sts2_engine::ConditionalOption {
                available: EventCondition::PotionsAtLeast(1),
                option: option(
                    "STONE_OF_ALL_TIME.pages.INITIAL.options.LIFT",
                    "Lift",
                    vec![
                        EventEffect::DiscardRolledPotion,
                        EventEffect::GainMaxHp(10),
                        // A last hundred-sided roll whose answer nothing
                        // reads.
                        EventEffect::ReadTheEventStream(100),
                    ],
                ),
                locked_option_id: "STONE_OF_ALL_TIME.pages.INITIAL.options.LIFT_LOCKED".to_owned(),
            },
            sts2_engine::ConditionalOption {
                // Vigorous only goes on attacks.
                available: EventCondition::DeckCardsAtLeast {
                    filter: EventCardFilter::EnchantableAttack,
                    count: 1,
                },
                option: option(
                    "STONE_OF_ALL_TIME.pages.INITIAL.options.PUSH",
                    "Push",
                    vec![
                        EventEffect::LoseHp(EventAmount::Fixed(6)),
                        // One card is picked out of the deck for a Vigorous
                        // of the event's own amount, and the enchant then runs
                        // over what comes back. The screen offers the attacks
                        // of the deck the enchantment can go on, in deck
                        // order, and takes the only one without asking.
                        EventEffect::Run(vec![sts2_engine::Command::CardSelect(
                            sts2_engine::CardSelectCmd::FromDeckForEnchantment {
                                semantics: ChoiceSemantics::known(vec![
                                    ChoiceOp::new(ChoiceOperationKind::Enchant, ChoiceScope::Run)
                                        .model(id("ENCHANTMENT.VIGOROUS"))
                                        .amount(8),
                                ]),
                                enchantment: id("ENCHANTMENT.VIGOROUS"),
                                amount: 8,
                                filter: sts2_engine::CardFilter::Any,
                                shuffled_on: None,
                                prefs: sts2_engine::SelectorPrefs::exactly(1),
                                then: vec![sts2_engine::Command::Card(
                                    sts2_engine::CardCmd::Enchant {
                                        cards: sts2_engine::CardSelector::Chosen,
                                        enchantment: id("ENCHANTMENT.VIGOROUS"),
                                        amount: sts2_engine::Amount::Fixed(8),
                                    },
                                )],
                            },
                        )]),
                        EventEffect::ReadTheEventStream(100),
                    ],
                ),
                locked_option_id: "STONE_OF_ALL_TIME.pages.INITIAL.options.PUSH_LOCKED".to_owned(),
            },
        ]),
    }
}

/// Ranwid the Elder, who trades a relic for two, a potion for one, or a
/// hundred gold for one.
#[must_use]
pub fn ranwid_the_elder() -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.RANWID_THE_ELDER"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::All(vec![
            EventCondition::ActIndexAbove(0),
            EventCondition::TradableRelicsAtLeast(1),
            EventCondition::GoldAtLeast(100),
            EventCondition::PotionsAtLeast(1),
        ]),
        // The page rolls the potion it would take first and the relic second,
        // in the order it lays the options out.
        vars: Vec::new(),
        generation: EventGeneration::Conditional(vec![
            sts2_engine::ConditionalOption {
                available: EventCondition::PotionsAtLeast(1),
                option: option(
                    "RANWID_THE_ELDER.pages.INITIAL.options.POTION",
                    "Potion",
                    vec![
                        EventEffect::DiscardRolledPotion,
                        EventEffect::ObtainRolledRelic,
                    ],
                ),
                locked_option_id: "RANWID_THE_ELDER.pages.INITIAL.options.POTION_LOCKED".to_owned(),
            },
            sts2_engine::ConditionalOption {
                available: EventCondition::Always,
                option: option(
                    "RANWID_THE_ELDER.pages.INITIAL.options.GOLD",
                    "Gold",
                    vec![
                        EventEffect::LoseGold(EventAmount::Fixed(100)),
                        EventEffect::ObtainRolledRelic,
                    ],
                ),
                locked_option_id: "RANWID_THE_ELDER.pages.INITIAL.options.GOLD".to_owned(),
            },
            sts2_engine::ConditionalOption {
                available: EventCondition::TradableRelicsAtLeast(1),
                option: option(
                    "RANWID_THE_ELDER.pages.INITIAL.options.RELIC",
                    "Relic",
                    vec![
                        EventEffect::RemoveRolledTradableRelic,
                        EventEffect::ObtainRolledRelic,
                        EventEffect::ObtainRolledRelic,
                    ],
                ),
                locked_option_id: "RANWID_THE_ELDER.pages.INITIAL.options.RELIC_LOCKED".to_owned(),
            },
        ]),
    }
}

/// Vakuu, the ancient who shuffles three pools and deals the top of each.
#[must_use]
pub fn vakuu() -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.VAKUU"),
        combat_layout: None,
        heals_on_entry: true,
        allowed: EventCondition::Always,
        vars: Vec::new(),
        generation: EventGeneration::ShuffledRelicPools {
            pools: vec![
                vec![
                    id("RELIC.BLOOD_SOAKED_ROSE"),
                    id("RELIC.WHISPERING_EARRING"),
                    id("RELIC.FIDDLE"),
                ],
                vec![
                    id("RELIC.PRESERVED_FOG"),
                    id("RELIC.SERE_TALON"),
                    id("RELIC.DISTINGUISHED_CAPE"),
                ],
                vec![
                    id("RELIC.CHOICES_PARADOX"),
                    id("RELIC.MUSIC_BOX"),
                    id("RELIC.LORDS_PARASOL"),
                    id("RELIC.JEWELED_MASK"),
                ],
            ],
        },
    }
}

/// The war historian, who is behind a cage the lantern key opens — or a chest,
/// if the key is spent on that instead.
///
/// The event is never pulled off an act's list — it is never allowed — and it
/// is only ever reached because a lantern key carried into the last act
/// replaces whatever the act would have offered. Either option spends the key
/// first, and a run holding a second one would then be offered the other; no
/// single-player run can hold two, so the page that would follow is the one
/// this baseline does not lay out.
#[must_use]
pub fn war_historian_repy() -> EventDefinition {
    let spend_the_key = || {
        EventEffect::Run(vec![sts2_engine::Command::CardPile(
            sts2_engine::CardPileCmd::RemoveFromDeck {
                cards: sts2_engine::CardSelector::FirstInDeckOfModel(id("CARD.LANTERN_KEY")),
            },
        )])
    };
    EventDefinition {
        model_id: id("EVENT.WAR_HISTORIAN_REPY"),
        combat_layout: None,
        heals_on_entry: false,
        // Never allowed: the act never offers it.
        allowed: EventCondition::Never,
        vars: Vec::new(),
        generation: EventGeneration::Fixed(vec![
            option(
                "WAR_HISTORIAN_REPY.pages.INITIAL.options.UNLOCK_CAGE",
                "Unlock the cage",
                vec![
                    spend_the_key(),
                    EventEffect::ObtainRelic(id("RELIC.HISTORY_COURSE")),
                ],
            ),
            option(
                "WAR_HISTORIAN_REPY.pages.INITIAL.options.UNLOCK_CHEST",
                "Unlock the chest",
                vec![
                    spend_the_key(),
                    EventEffect::Run(vec![
                        sts2_engine::Command::Rewards(sts2_engine::RewardsCmd::AddPotion),
                        sts2_engine::Command::Rewards(sts2_engine::RewardsCmd::AddPotion),
                        sts2_engine::Command::Rewards(sts2_engine::RewardsCmd::AddRelic),
                        sts2_engine::Command::Rewards(sts2_engine::RewardsCmd::AddRelic),
                        sts2_engine::Command::Rewards(sts2_engine::RewardsCmd::OfferAdded),
                    ]),
                ],
            ),
        ]),
    }
}

/// A brain leech: share what you know for one of five cards out of your own
/// pool, or let it rip five HP out of you for three colorless cards.
#[must_use]
pub fn brain_leech() -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.BRAIN_LEECH"),
        combat_layout: None,
        heals_on_entry: false,
        // Offered in the first two acts only.
        allowed: EventCondition::ActIndexBelow(2),
        vars: Vec::new(),
        generation: EventGeneration::Fixed(vec![
            option(
                "BRAIN_LEECH.pages.INITIAL.options.SHARE_KNOWLEDGE",
                "Share knowledge",
                // Five cards rolled out of the player's own pool at the
                // out-of-combat odds, laid out on a rewards grid that takes
                // one into the deck.
                vec![EventEffect::ChooseCreatedCards {
                    picks: 1,
                    creation: sts2_engine::CardCreation {
                        pools: vec![sts2_engine::CardPoolRef::OwnCharacter],
                        filter: sts2_engine::CardFilter::Any,
                        odds: sts2_engine::RarityOdds::RegularEncounter,
                        from_an_encounter: false,
                        count: 5,
                        flags: sts2_engine::CardCreationFlags {
                            no_upgrade_roll: true,
                            ..sts2_engine::CardCreationFlags::default()
                        },
                    },
                }],
            ),
            option(
                "BRAIN_LEECH.pages.INITIAL.options.RIP",
                "Rip",
                vec![
                    EventEffect::LoseHp(EventAmount::Fixed(5)),
                    EventEffect::OfferCreatedCardRewards {
                        creations: vec![sts2_engine::CardCreation {
                            pools: vec![sts2_engine::CardPoolRef::Named(id(
                                "CARD_POOL.COLORLESS_CARD_POOL",
                            ))],
                            filter: sts2_engine::CardFilter::Any,
                            odds: sts2_engine::RarityOdds::RegularEncounter,
                            from_an_encounter: false,
                            count: 3,
                            flags: sts2_engine::CardCreationFlags {
                                no_upgrade_roll: true,
                                no_card_pool_modifications: true,
                                ..sts2_engine::CardCreationFlags::default()
                            },
                        }],
                    },
                ],
            ),
        ]),
    }
}

/// A self-help book: one card of the deck enchanted, with the enchantment
/// deciding which cards qualify. Each option is locked where the deck holds
/// nothing it could go on.
#[must_use]
pub fn self_help_book() -> EventDefinition {
    let reading = |name: &str, enchantment: &str, card_type: sts2_engine::CardType| {
        sts2_engine::ConditionalOption {
            available: EventCondition::DeckCardsAtLeast {
                filter: EventCardFilter::EnchantableByOfType {
                    enchantment: id(enchantment),
                    card_type,
                },
                count: 1,
            },
            option: option(
                &format!("SELF_HELP_BOOK.pages.INITIAL.options.{name}"),
                name,
                vec![EventEffect::Run(vec![sts2_engine::Command::CardSelect(
                    sts2_engine::CardSelectCmd::FromDeckForEnchantment {
                        semantics: ChoiceSemantics::known(vec![
                            ChoiceOp::new(ChoiceOperationKind::Enchant, ChoiceScope::Run)
                                .model(id(enchantment))
                                .amount(2),
                        ]),
                        enchantment: id(enchantment),
                        amount: 2,
                        filter: sts2_engine::CardFilter::OfType(card_type),
                        shuffled_on: None,
                        prefs: sts2_engine::SelectorPrefs::exactly(1),
                        then: vec![sts2_engine::Command::Card(sts2_engine::CardCmd::Enchant {
                            cards: sts2_engine::CardSelector::Chosen,
                            enchantment: id(enchantment),
                            amount: sts2_engine::Amount::Fixed(2),
                        })],
                    },
                )])],
            ),
            locked_option_id: format!("SELF_HELP_BOOK.pages.INITIAL.options.{name}_LOCKED"),
        }
    };
    EventDefinition {
        model_id: id("EVENT.SELF_HELP_BOOK"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::Always,
        vars: Vec::new(),
        generation: EventGeneration::Conditional(vec![
            reading(
                "READ_THE_BACK",
                "ENCHANTMENT.SHARP",
                sts2_engine::CardType::Attack,
            ),
            reading(
                "READ_PASSAGE",
                "ENCHANTMENT.NIMBLE",
                sts2_engine::CardType::Skill,
            ),
            reading(
                "READ_ENTIRE_BOOK",
                "ENCHANTMENT.SWIFT",
                sts2_engine::CardType::Power,
            ),
        ]),
    }
}

/// A lost wisp: claim it for a relic and a decay, or search the place for gold
/// the event rolled on its own die as the page was built.
#[must_use]
pub fn lost_wisp() -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.LOST_WISP"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::Always,
        vars: Vec::new(),
        generation: EventGeneration::Fixed(vec![
            option(
                "LOST_WISP.pages.INITIAL.options.CLAIM",
                "Claim",
                vec![
                    EventEffect::AddCard(sts2_engine::CardFingerprint::base(id("CARD.DECAY"))),
                    EventEffect::ObtainRelic(id("RELIC.LOST_WISP")),
                ],
            ),
            option(
                "LOST_WISP.pages.INITIAL.options.SEARCH",
                "Search",
                // The gold is sixty shifted by a roll of −15 to 15, taken as
                // the page is built, so it is settled before the page is shown
                // and one roll is spent whichever option is taken.
                vec![EventEffect::GainGold(EventAmount::RolledOnto {
                    base: 60,
                    min: -15,
                    max: 16,
                })],
            ),
        ]),
    }
}

/// Dense vegetation: cut through it for gold and eight HP, or rest in it and
/// find out what was resting there already.
#[must_use]
pub fn dense_vegetation() -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.DENSE_VEGETATION"),
        combat_layout: None,
        heals_on_entry: false,
        // Single-player answers yes before the loop that tests every player.
        allowed: EventCondition::Always,
        vars: Vec::new(),
        generation: EventGeneration::Fixed(vec![
            option(
                "DENSE_VEGETATION.pages.INITIAL.options.TRUDGE_ON",
                "Trudge on",
                vec![
                    EventEffect::LoseHp(EventAmount::Fixed(8)),
                    // The gold is rolled as the page is built, whichever
                    // option is taken.
                    EventEffect::GainGold(EventAmount::Rolled { min: 61, max: 100 }),
                ],
            ),
            option(
                "DENSE_VEGETATION.pages.INITIAL.options.REST",
                "Rest",
                vec![
                    EventEffect::Run(vec![sts2_engine::Command::Player(
                        sts2_engine::PlayerCmd::MimicRestSiteHeal,
                    )]),
                    EventEffect::SetPage(vec![option(
                        "DENSE_VEGETATION.pages.REST.options.FIGHT",
                        "Fight",
                        vec![EventEffect::EnterCombatWithoutLeaving {
                            encounter: id("ENCOUNTER.DENSE_VEGETATION_EVENT_ENCOUNTER"),
                            extra_rewards: Vec::new(),
                            resumes_parent: false,
                            chosen_monster: None,
                            resume: Vec::new(),
                        }],
                    )]),
                ],
            ),
        ]),
    }
}

/// A symbiote: let it onto a card of the deck, or burn one of the deck's cards
/// into another card entirely.
///
/// The first option is offered locked when the deck holds nothing the
/// corruption can go on.
#[must_use]
pub fn symbiote() -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.SYMBIOTE"),
        combat_layout: None,
        heals_on_entry: false,
        // Not offered in act one.
        allowed: EventCondition::ActIndexAbove(0),
        vars: Vec::new(),
        generation: EventGeneration::Conditional(vec![
            sts2_engine::ConditionalOption {
                available: EventCondition::DeckCardsAtLeast {
                    filter: EventCardFilter::Enchantable,
                    count: 1,
                },
                option: option(
                    "SYMBIOTE.pages.INITIAL.options.APPROACH",
                    "Approach",
                    vec![EventEffect::Run(vec![sts2_engine::Command::CardSelect(
                        sts2_engine::CardSelectCmd::FromDeckForEnchantment {
                            semantics: ChoiceSemantics::known(vec![
                                ChoiceOp::new(ChoiceOperationKind::Enchant, ChoiceScope::Run)
                                    .model(id("ENCHANTMENT.CORRUPTED"))
                                    .amount(1),
                            ]),
                            enchantment: id("ENCHANTMENT.CORRUPTED"),
                            amount: 1,
                            filter: sts2_engine::CardFilter::Any,
                            shuffled_on: None,
                            prefs: sts2_engine::SelectorPrefs::exactly(1),
                            then: vec![sts2_engine::Command::Card(sts2_engine::CardCmd::Enchant {
                                cards: sts2_engine::CardSelector::Chosen,
                                enchantment: id("ENCHANTMENT.CORRUPTED"),
                                amount: sts2_engine::Amount::Fixed(1),
                            })],
                        },
                    )])],
                ),
                locked_option_id: "SYMBIOTE.pages.INITIAL.options.APPROACH_LOCKED".to_owned(),
            },
            sts2_engine::ConditionalOption {
                available: EventCondition::Always,
                option: option(
                    "SYMBIOTE.pages.INITIAL.options.KILL_WITH_FIRE",
                    "Kill it with fire",
                    vec![EventEffect::Run(vec![sts2_engine::Command::CardSelect(
                        sts2_engine::CardSelectCmd::FromDeckForTransformation {
                            semantics: ChoiceSemantics::known(vec![
                                ChoiceOp::new(ChoiceOperationKind::Transform, ChoiceScope::Run)
                                    .rolled(),
                            ]),
                            prefs: sts2_engine::SelectorPrefs::exactly(1),
                            then: vec![sts2_engine::Command::Card(
                                sts2_engine::CardCmd::TransformDeckCards {
                                    cards: sts2_engine::CardSelector::Chosen,
                                    replacement: sts2_engine::CardTransformReplacement::Rolled {
                                        stream: sts2_engine::TransformStream::Event,
                                        upgraded: false,
                                    },
                                    one_at_a_time: false,
                                },
                            )],
                        },
                    )])],
                ),
                locked_option_id: "SYMBIOTE.pages.INITIAL.options.KILL_WITH_FIRE".to_owned(),
            },
        ]),
    }
}

/// A spiraling whirlpool: watch it and a basic strike or defend spirals, or
/// drink it and a third of the player's own maximum comes back.
#[must_use]
pub fn spiraling_whirlpool() -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.SPIRALING_WHIRLPOOL"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::DeckCardsAtLeast {
            filter: EventCardFilter::EnchantableBasicStrikeOrDefend,
            count: 1,
        },
        vars: Vec::new(),
        generation: EventGeneration::Fixed(vec![
            option(
                "SPIRALING_WHIRLPOOL.pages.INITIAL.options.OBSERVE",
                "Observe",
                vec![EventEffect::Run(vec![sts2_engine::Command::CardSelect(
                    sts2_engine::CardSelectCmd::FromDeckForEnchantment {
                        semantics: ChoiceSemantics::known(vec![
                            ChoiceOp::new(ChoiceOperationKind::Enchant, ChoiceScope::Run)
                                .model(id("ENCHANTMENT.SPIRAL"))
                                .amount(1),
                        ]),
                        enchantment: id("ENCHANTMENT.SPIRAL"),
                        amount: 1,
                        // The extra filter is what Spiral can go on again,
                        // which the screen already applies.
                        filter: sts2_engine::CardFilter::Any,
                        shuffled_on: None,
                        prefs: sts2_engine::SelectorPrefs::exactly(1),
                        then: vec![sts2_engine::Command::Card(sts2_engine::CardCmd::Enchant {
                            cards: sts2_engine::CardSelector::Chosen,
                            enchantment: id("ENCHANTMENT.SPIRAL"),
                            amount: sts2_engine::Amount::Fixed(1),
                        })],
                    },
                )])],
            ),
            option(
                "SPIRALING_WHIRLPOOL.pages.INITIAL.options.DRINK",
                "Drink",
                // A third of maximum health, taken as a whole number.
                vec![EventEffect::Heal(EventAmount::MaxHpFraction {
                    numerator: 33,
                    denominator: 100,
                })],
            ),
        ]),
    }
}

/// The lantern key: hand it back for a hundred gold, or keep it and fight the
/// knight who wants it back.
///
/// Keeping it lays out a second page with one option on it, and that option
/// walks into the knight's encounter without leaving the event. The key itself
/// is offered with the fight's rewards as a card handed over rather than one
/// rolled. The event does not resume after the fight, so proceeding past the
/// rewards opens the map rather than going back to the page.
#[must_use]
pub fn the_lantern_key() -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.THE_LANTERN_KEY"),
        // The room is laid out as a fight against the knight, so he is
        // standing there from the moment it is entered.
        combat_layout: Some(id("ENCOUNTER.MYSTERIOUS_KNIGHT_EVENT_ENCOUNTER")),
        heals_on_entry: false,
        allowed: EventCondition::Always,
        vars: Vec::new(),
        generation: EventGeneration::Fixed(vec![
            option(
                "THE_LANTERN_KEY.pages.INITIAL.options.RETURN_THE_KEY",
                "Return the key",
                vec![EventEffect::GainGold(EventAmount::Fixed(100))],
            ),
            option(
                "THE_LANTERN_KEY.pages.INITIAL.options.KEEP_THE_KEY",
                "Keep the key",
                vec![EventEffect::SetPage(vec![option(
                    "THE_LANTERN_KEY.pages.KEEP_THE_KEY.options.FIGHT",
                    "Fight",
                    vec![EventEffect::EnterCombatWithoutLeaving {
                        encounter: id("ENCOUNTER.MYSTERIOUS_KNIGHT_EVENT_ENCOUNTER"),
                        extra_rewards: vec![
                            sts2_engine::RewardFingerprint::of("special_card").with_special_card(
                                sts2_engine::CardFingerprint::base(id("CARD.LANTERN_KEY")),
                            ),
                        ],
                        resumes_parent: false,
                        chosen_monster: None,
                        resume: Vec::new(),
                    }],
                )])],
            ),
        ]),
    }
}

/// The amalgamator, who forges two starter strikes into an Ultimate Strike, or
/// two starter defends into an Ultimate Defend.
///
/// The screen asks for exactly two, filtered to a basic card carrying the tag
/// that can still be removed, on top of the removability the
/// removal screen applies itself. What comes back leaves the deck, and the
/// forged card joins it.
#[must_use]
pub fn amalgamator() -> EventDefinition {
    let combine = |name: &str, tag, into: &str| {
        option(
            &format!("AMALGAMATOR.pages.INITIAL.options.COMBINE_{name}"),
            name,
            vec![EventEffect::Run(vec![sts2_engine::Command::CardSelect(
                sts2_engine::CardSelectCmd::FromDeckForRemoval {
                    semantics: ChoiceSemantics::known(vec![
                        ChoiceOp::new(ChoiceOperationKind::Remove, ChoiceScope::Run)
                            .to(ChoiceDestination::Removed),
                    ]),
                    prefs: sts2_engine::SelectorPrefs::exactly(2),
                    filter: sts2_engine::CardFilter::BasicOfTag(tag),
                    then: vec![
                        sts2_engine::Command::CardPile(sts2_engine::CardPileCmd::RemoveFromDeck {
                            cards: sts2_engine::CardSelector::Chosen,
                        }),
                        sts2_engine::Command::CardPile(sts2_engine::CardPileCmd::AddToDeck {
                            card: sts2_engine::CardOrigin::Model(id(into)),
                        }),
                    ],
                },
            )])],
        )
    };
    EventDefinition {
        model_id: id("EVENT.AMALGAMATOR"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::All(vec![
            EventCondition::DeckCardsAtLeast {
                filter: EventCardFilter::BasicRemovableTagged(CardTag::Strike),
                count: 2,
            },
            EventCondition::DeckCardsAtLeast {
                filter: EventCardFilter::BasicRemovableTagged(CardTag::Defend),
                count: 2,
            },
        ]),
        vars: Vec::new(),
        generation: EventGeneration::Fixed(vec![
            combine("STRIKES", CardTag::Strike, "CARD.ULTIMATE_STRIKE"),
            combine("DEFENDS", CardTag::Defend, "CARD.ULTIMATE_DEFEND"),
        ]),
    }
}

/// The colorful philosophers, who offer another character's cards: a common, an
/// uncommon and a rare out of the pool whose colour was picked, three of each.
///
/// The page is built in the game's own colour order — necrobinder, ironclad,
/// regent, silent, defect — over the pools the run has unlocked and is not
/// playing, and is then cut down to three by removing rolled entries. Every
/// character is
/// unlocked at this baseline, so an ironclad run lays four out and loses one.
///
/// Every character's pool is registered, so every colour but the walking
/// player's own is laid out and every one of them deals.
#[must_use]
pub fn colorful_philosophers(character: &sts2_core::ModelId) -> EventDefinition {
    let offer = |pool: &str| {
        let creation = |rarity| sts2_engine::CardCreation {
            pools: vec![sts2_engine::CardPoolRef::Named(id(pool))],
            filter: sts2_engine::CardFilter::OfRarity(rarity),
            odds: sts2_engine::RarityOdds::Uniform,
            from_an_encounter: false,
            count: 3,
            flags: sts2_engine::CardCreationFlags {
                is_card_reward: true,
                no_card_pool_modifications: true,
                no_upgrade_roll: false,
            },
        };
        vec![EventEffect::OfferCreatedCardRewards {
            creations: vec![
                creation(sts2_engine::CardRarity::Common),
                creation(sts2_engine::CardRarity::Uncommon),
                creation(sts2_engine::CardRarity::Rare),
            ],
        }]
    };
    let colour = |name: &str, pool: &str| sts2_engine::EventOption {
        option_id: format!("COLORFUL_PHILOSOPHERS.pages.INITIAL.options.{name}"),
        label: name.to_owned(),
        effects: offer(pool),
        is_proceed: false,
        was_chosen: false,
        locked: false,
    };
    // Every colour but the one being played, in the build's own order, and
    // only the pools the run has unlocked.
    let own = character.entry();
    let options = [
        ("NECROBINDER", "CARD_POOL.NECROBINDER_CARD_POOL"),
        ("IRONCLAD", "CARD_POOL.IRONCLAD_CARD_POOL"),
        ("REGENT", "CARD_POOL.REGENT_CARD_POOL"),
        ("SILENT", "CARD_POOL.SILENT_CARD_POOL"),
        ("DEFECT", "CARD_POOL.DEFECT_CARD_POOL"),
    ]
    .into_iter()
    .filter(|(name, _)| *name != own)
    .map(|(name, pool)| colour(name, pool))
    .collect();
    EventDefinition {
        model_id: id("EVENT.COLORFUL_PHILOSOPHERS"),
        combat_layout: None,
        heals_on_entry: false,
        // Only where every player has more than one card pool unlocked.
        allowed: EventCondition::MultipleCharacterPoolsUnlocked,
        vars: Vec::new(),
        generation: EventGeneration::RemovedDownTo { options, keep: 3 },
    }
}

/// Tezcatara, the ancient who rolls one relic out of each of three pools.
///
/// The first pool gains the soup for a deck that still holds a basic Strike —
/// a card of basic rarity carrying the Strike tag, with no word about whether
/// that card could be removed.
#[must_use]
pub fn tezcatara() -> EventDefinition {
    let always = |relic: &str| sts2_engine::ConditionalRelic {
        relic: id(relic),
        available: EventCondition::Always,
    };
    EventDefinition {
        model_id: id("EVENT.TEZCATARA"),
        combat_layout: None,
        heals_on_entry: true,
        allowed: EventCondition::Always,
        vars: Vec::new(),
        generation: EventGeneration::RolledRelicPools {
            pools: vec![
                vec![
                    always("RELIC.VERY_HOT_COCOA"),
                    always("RELIC.YUMMY_COOKIE"),
                    sts2_engine::ConditionalRelic {
                        relic: id("RELIC.NUTRITIOUS_SOUP"),
                        available: EventCondition::DeckCardsAtLeast {
                            filter: EventCardFilter::BasicTagged(CardTag::Strike),
                            count: 1,
                        },
                    },
                ],
                vec![
                    always("RELIC.BIIIG_HUG"),
                    always("RELIC.STORYBOOK"),
                    always("RELIC.TOASTY_MITTENS"),
                ],
                vec![
                    always("RELIC.GOLDEN_COMPASS"),
                    always("RELIC.PUMPKIN_CANDLE"),
                    always("RELIC.TOY_BOX"),
                    always("RELIC.SEAL_OF_GOLD"),
                ],
            ],
        },
    }
}

/// The Future of Potions: trade a potion for three upgraded cards of one
/// rolled type, at the rarity the potion was worth.
#[must_use]
pub fn the_future_of_potions(pool: Vec<sts2_engine::PoolCard>) -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.THE_FUTURE_OF_POTIONS"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::PotionsAtLeast(2),
        vars: Vec::new(),
        generation: EventGeneration::PotionTrade { pool },
    }
}

/// Orobas, the ancient who deals three relics: one out of each of its pools.
#[must_use]
pub fn orobas() -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.OROBAS"),
        combat_layout: None,
        heals_on_entry: true,
        allowed: EventCondition::Always,
        vars: Vec::new(),
        generation: EventGeneration::OrobasRelics {
            pool_one: vec![
                id("RELIC.ELECTRIC_SHRYMP"),
                id("RELIC.GLASS_EYE"),
                id("RELIC.SAND_CASTLE"),
            ],
            prismatic: id("RELIC.PRISMATIC_GEM"),
            sea_glass: id("RELIC.SEA_GLASS"),
            pool_two: vec![
                id("RELIC.ALCHEMICAL_COFFER"),
                id("RELIC.DRIFTWOOD"),
                id("RELIC.RADIANT_PEARL"),
            ],
            pool_three: vec![
                // The touch of Orobas is only offered where there is a
                // starter relic for it to refine, and the archaic tooth only
                // where there is a starter card for it to transcend.
                sts2_engine::ConditionalRelic {
                    relic: id("RELIC.TOUCH_OF_OROBAS"),
                    available: EventCondition::HoldsRelicOfRarity(
                        sts2_engine::RelicRarity::Starter,
                    ),
                },
                sts2_engine::ConditionalRelic {
                    relic: id("RELIC.ARCHAIC_TOOTH"),
                    available: EventCondition::DeckCardsAtLeast {
                        filter: EventCardFilter::Transcendable,
                        count: 1,
                    },
                },
            ],
            locked_option_id: "OROBAS.pages.INITIAL.options.OPTION_POOL_3_LOCKED".to_owned(),
        },
    }
}

fn option(option_id: &str, label: &str, effects: Vec<EventEffect>) -> EventOption {
    EventOption {
        option_id: option_id.to_owned(),
        label: label.to_owned(),
        effects,
        is_proceed: false,
        was_chosen: false,
        locked: false,
    }
}

/// The card is made with its act index already set to one, which is the act
/// whose map it bends. It is the only card of the pinned build that does
/// anything at all as it is created, and this is the whole of it.
fn spoils_map() -> sts2_engine::CardFingerprint {
    sts2_engine::CardFingerprint::base(id("CARD.SPOILS_MAP")).with_property("SpoilsActIndex", 1)
}

/// A trash heap. Dive in for a relic out of five and eight unblockable HP, or
/// grab what is on top for a hundred gold and one card out of ten.
#[must_use]
pub fn trash_heap() -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.TRASH_HEAP"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::CurrentHpAbove(5),
        vars: Vec::new(),
        generation: EventGeneration::Fixed(vec![
            option(
                "TRASH_HEAP.pages.INITIAL.options.DIVE_IN",
                "DiveIn",
                vec![
                    EventEffect::LoseHp(EventAmount::Fixed(8)),
                    EventEffect::ObtainRelicRolledFrom {
                        candidates: [
                            "RELIC.DARKSTONE_PERIAPT",
                            "RELIC.DREAM_CATCHER",
                            "RELIC.HAND_DRILL",
                            "RELIC.MAW_BANK",
                            "RELIC.THE_BOOT",
                        ]
                        .into_iter()
                        .map(id)
                        .collect(),
                    },
                ],
            ),
            option(
                "TRASH_HEAP.pages.INITIAL.options.GRAB",
                "Grab",
                vec![
                    EventEffect::GainGold(EventAmount::Fixed(100)),
                    EventEffect::AddCardRolledFrom {
                        candidates: [
                            "CARD.CALTROPS",
                            "CARD.CLASH",
                            "CARD.DISTRACTION",
                            "CARD.DUAL_WIELD",
                            "CARD.ENTRENCH",
                            "CARD.HELLO_WORLD",
                            "CARD.OUTMANEUVER",
                            "CARD.REBOUND",
                            "CARD.RIP_AND_TEAR",
                            "CARD.STACK",
                        ]
                        .into_iter()
                        .map(id)
                        .collect(),
                    },
                ],
            ),
        ]),
    }
}

/// The legends were true. Nab the map for a quest card, or find the way out
/// slowly for eight unblockable HP and a potion rolled off the belt's own
/// options.
#[must_use]
pub fn the_legends_were_true(potions: Vec<sts2_core::ModelId>) -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.THE_LEGENDS_WERE_TRUE"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::All(vec![
            EventCondition::ActIndexIs(0),
            EventCondition::DeckCardsAtLeast {
                filter: EventCardFilter::Any,
                count: 1,
            },
            EventCondition::CurrentHpAtLeast(10),
        ]),
        vars: Vec::new(),
        generation: EventGeneration::Fixed(vec![
            option(
                "THE_LEGENDS_WERE_TRUE.pages.INITIAL.options.NAB_THE_MAP",
                "NabTheMap",
                vec![EventEffect::AddCard(spoils_map())],
            ),
            option(
                "THE_LEGENDS_WERE_TRUE.pages.INITIAL.options.SLOWLY_FIND_AN_EXIT",
                "SlowlyFindAnExit",
                vec![
                    EventEffect::LoseHp(EventAmount::Fixed(8)),
                    EventEffect::OfferRolledPotion {
                        candidates: potions,
                    },
                ],
            ),
        ]),
    }
}

/// A spirit grafter. Let it in and be healed twenty-five with a Metamorphosis
/// in the deck for it, or turn it away and upgrade one card of the deck for
/// ten HP.
#[must_use]
pub fn spirit_grafter() -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.SPIRIT_GRAFTER"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::Always,
        vars: Vec::new(),
        generation: EventGeneration::Fixed(vec![
            option(
                "SPIRIT_GRAFTER.pages.INITIAL.options.LET_IT_IN",
                "Let it in",
                vec![
                    EventEffect::Heal(EventAmount::Fixed(25)),
                    EventEffect::AddCard(sts2_engine::CardFingerprint::base(id(
                        "CARD.METAMORPHOSIS",
                    ))),
                ],
            ),
            option(
                "SPIRIT_GRAFTER.pages.INITIAL.options.REJECTION",
                "Rejection",
                vec![
                    EventEffect::Run(vec![sts2_engine::Command::CardSelect(
                        sts2_engine::CardSelectCmd::FromDeckForUpgrade {
                            semantics: ChoiceSemantics::known(vec![ChoiceOp::new(
                                ChoiceOperationKind::Upgrade,
                                ChoiceScope::Run,
                            )]),
                            prefs: sts2_engine::SelectorPrefs::exactly(1),
                            then: vec![sts2_engine::Command::Card(sts2_engine::CardCmd::Upgrade {
                                cards: sts2_engine::CardSelector::Chosen,
                            })],
                        },
                    )]),
                    EventEffect::LoseHp(EventAmount::Fixed(10)),
                ],
            ),
        ]),
    }
}

/// A tea master. A bone tea for fifty gold, an ember tea for a hundred and
/// fifty, and a tea of discourtesy for nothing at all; each option is shown
/// locked where the gold is not there, and the third never is.
#[must_use]
pub fn tea_master() -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.TEA_MASTER"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::All(vec![
            EventCondition::ActIndexBelow(2),
            EventCondition::GoldAtLeast(150),
        ]),
        vars: Vec::new(),
        generation: EventGeneration::Conditional(vec![
            sts2_engine::ConditionalOption {
                available: EventCondition::GoldAtLeast(50),
                option: option(
                    "TEA_MASTER.pages.INITIAL.options.BONE_TEA",
                    "Bone tea",
                    vec![
                        EventEffect::LoseGold(EventAmount::Fixed(50)),
                        EventEffect::ObtainRelic(id("RELIC.BONE_TEA")),
                    ],
                ),
                locked_option_id: "TEA_MASTER.pages.INITIAL.options.BONE_TEA_LOCKED".to_owned(),
            },
            sts2_engine::ConditionalOption {
                available: EventCondition::GoldAtLeast(150),
                option: option(
                    "TEA_MASTER.pages.INITIAL.options.EMBER_TEA",
                    "Ember tea",
                    vec![
                        EventEffect::LoseGold(EventAmount::Fixed(150)),
                        EventEffect::ObtainRelic(id("RELIC.EMBER_TEA")),
                    ],
                ),
                locked_option_id: "TEA_MASTER.pages.INITIAL.options.EMBER_TEA_LOCKED".to_owned(),
            },
            sts2_engine::ConditionalOption {
                available: EventCondition::Always,
                option: option(
                    "TEA_MASTER.pages.INITIAL.options.TEA_OF_DISCOURTESY",
                    "Tea of discourtesy",
                    vec![EventEffect::ObtainRelic(id("RELIC.TEA_OF_DISCOURTESY"))],
                ),
                locked_option_id: String::new(),
            },
        ]),
    }
}

/// Two doors. The light one upgrades two cards of the deck, rolled on the
/// event's own stream; the dark one takes one away.
#[must_use]
pub fn doors_of_light_and_dark() -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.DOORS_OF_LIGHT_AND_DARK"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::Always,
        vars: Vec::new(),
        generation: EventGeneration::Fixed(vec![
            option(
                "DOORS_OF_LIGHT_AND_DARK.pages.INITIAL.options.LIGHT",
                "Light",
                vec![EventEffect::Run(vec![sts2_engine::Command::Card(
                    sts2_engine::CardCmd::Upgrade {
                        cards: sts2_engine::CardSelector::StablyShuffledFromDeck {
                            filter: sts2_engine::CardFilter::Upgradable,
                            take: sts2_engine::Amount::Fixed(2),
                            stream: sts2_engine::ShuffleStream::Event,
                        },
                    },
                )])],
            ),
            option(
                "DOORS_OF_LIGHT_AND_DARK.pages.INITIAL.options.DARK",
                "Dark",
                vec![EventEffect::RemoveDeckCardsForGold { count: 1, cost: 0 }],
            ),
        ]),
    }
}

/// A tea party. Sit down for it and be healed whole with something poisonous in
/// hand, or pick a fight and be paid a relic for the blood it costs.
#[must_use]
pub fn round_tea_party() -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.ROUND_TEA_PARTY"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::CurrentHpAtLeast(12),
        vars: Vec::new(),
        generation: EventGeneration::Fixed(vec![
            option(
                "ROUND_TEA_PARTY.pages.INITIAL.options.ENJOY_TEA",
                "Enjoy tea",
                vec![
                    EventEffect::ObtainRelic(id("RELIC.ROYAL_POISON")),
                    // A heal to full, which is the whole of it.
                    EventEffect::Heal(EventAmount::Fixed(9_999)),
                ],
            ),
            option(
                "ROUND_TEA_PARTY.pages.INITIAL.options.PICK_FIGHT",
                "Pick fight",
                vec![EventEffect::SetPage(vec![option(
                    "ROUND_TEA_PARTY.pages.PICK_FIGHT.options.CONTINUE_FIGHT",
                    "Continue fight",
                    vec![
                        EventEffect::LoseHp(EventAmount::Fixed(11)),
                        EventEffect::ObtainRolledRelic,
                    ],
                )])],
            ),
        ]),
    }
}

/// Nonupeipe, the ancient who deals three of nine — ten, for a deck that can
/// take the bracelet.
#[must_use]
pub fn nonupeipe() -> EventDefinition {
    let always = |relic: &str| sts2_engine::ConditionalRelic {
        relic: id(relic),
        available: EventCondition::Always,
    };
    EventDefinition {
        model_id: id("EVENT.NONUPEIPE"),
        combat_layout: None,
        heals_on_entry: true,
        allowed: EventCondition::Always,
        vars: Vec::new(),
        generation: EventGeneration::ShuffledRelicPool {
            relics: vec![
                always("RELIC.BLESSED_ANTLER"),
                always("RELIC.BRILLIANT_SCARF"),
                always("RELIC.DELICATE_FROND"),
                always("RELIC.DIAMOND_DIADEM"),
                always("RELIC.FUR_COAT"),
                always("RELIC.GLITTER"),
                always("RELIC.JEWELRY_BOX"),
                always("RELIC.LOOMING_FRUIT"),
                always("RELIC.SIGNET_RING"),
                // Swift goes on any enchantable card, and the bracelet joins
                // the list — at the end of it, before the shuffle — for a deck
                // with four cards it could take.
                sts2_engine::ConditionalRelic {
                    relic: id("RELIC.BEAUTIFUL_BRACELET"),
                    available: EventCondition::DeckCardsAtLeast {
                        filter: EventCardFilter::Enchantable,
                        count: 4,
                    },
                },
            ],
            offered: 3,
        },
    }
}

/// A flower with something at the bottom of it. Each reach costs blood and
/// raises what the prize is worth; the third reach pays a relic instead.
///
/// Every page is laid out as the option before it is taken, and the damage a
/// reach deals is the one for the dig it is making rather than the one it is
/// about to reach.
#[must_use]
pub fn colossal_flower() -> EventDefinition {
    let extract = |page: &str, dig: usize, gold: i32| {
        option(
            &format!("COLOSSAL_FLOWER.pages.{page}.options.EXTRACT_CURRENT_PRIZE_{dig}"),
            "Extract",
            vec![EventEffect::GainGold(EventAmount::Fixed(gold))],
        )
    };
    let second_page = vec![
        extract("REACH_DEEPER_1", 2, 75),
        option(
            "COLOSSAL_FLOWER.pages.REACH_DEEPER_1.options.REACH_DEEPER_2",
            "Reach deeper",
            vec![
                EventEffect::LoseHp(EventAmount::Fixed(6)),
                EventEffect::SetPage(vec![
                    option(
                        "COLOSSAL_FLOWER.pages.REACH_DEEPER_2.options.EXTRACT_INSTEAD",
                        "Extract instead",
                        vec![EventEffect::GainGold(EventAmount::Fixed(135))],
                    ),
                    option(
                        "COLOSSAL_FLOWER.pages.REACH_DEEPER_2.options.POLLINOUS_CORE",
                        "Pollinous core",
                        vec![
                            EventEffect::LoseHp(EventAmount::Fixed(7)),
                            EventEffect::ObtainRelic(id("RELIC.POLLINOUS_CORE")),
                        ],
                    ),
                ]),
            ],
        ),
    ];
    EventDefinition {
        model_id: id("EVENT.COLOSSAL_FLOWER"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::CurrentHpAtLeast(19),
        vars: Vec::new(),
        generation: EventGeneration::Fixed(vec![
            extract("INITIAL", 1, 35),
            option(
                "COLOSSAL_FLOWER.pages.INITIAL.options.REACH_DEEPER_1",
                "Reach deeper",
                vec![
                    EventEffect::LoseHp(EventAmount::Fixed(5)),
                    EventEffect::SetPage(second_page),
                ],
            ),
        ]),
    }
}

/// The Zen Weaver: two cards of enlightenment for a little gold, or a card off
/// the deck for a lot of it.
#[must_use]
pub fn zen_weaver() -> EventDefinition {
    let locked = "ZEN_WEAVER.pages.INITIAL.options.LOCKED";
    EventDefinition {
        model_id: id("EVENT.ZEN_WEAVER"),
        combat_layout: None,
        heals_on_entry: false,
        // The page is only pulled for a player who could afford the middle
        // option.
        allowed: EventCondition::GoldAtLeast(125),
        vars: Vec::new(),
        generation: EventGeneration::Conditional(vec![
            sts2_engine::ConditionalOption {
                available: EventCondition::Always,
                option: option(
                    "ZEN_WEAVER.pages.INITIAL.options.BREATHING_TECHNIQUES",
                    "Breathing techniques",
                    vec![
                        EventEffect::LoseGold(EventAmount::Fixed(50)),
                        EventEffect::AddCard(sts2_engine::CardFingerprint::base(id(
                            "CARD.ENLIGHTENMENT",
                        ))),
                        EventEffect::AddCard(sts2_engine::CardFingerprint::base(id(
                            "CARD.ENLIGHTENMENT",
                        ))),
                    ],
                ),
                locked_option_id: locked.to_owned(),
            },
            sts2_engine::ConditionalOption {
                available: EventCondition::GoldAtLeast(125),
                option: option(
                    "ZEN_WEAVER.pages.INITIAL.options.EMOTIONAL_AWARENESS",
                    "Emotional awareness",
                    vec![EventEffect::RemoveDeckCardsForGold {
                        count: 1,
                        cost: 125,
                    }],
                ),
                locked_option_id: locked.to_owned(),
            },
            sts2_engine::ConditionalOption {
                available: EventCondition::GoldAtLeast(250),
                option: option(
                    "ZEN_WEAVER.pages.INITIAL.options.ARACHNID_ACUPUNCTURE",
                    "Arachnid acupuncture",
                    vec![EventEffect::RemoveDeckCardsForGold {
                        count: 2,
                        cost: 250,
                    }],
                ),
                locked_option_id: locked.to_owned(),
            },
        ]),
    }
}

/// An automaton crawling with something. Studying it hands over a power;
/// touching the core hands over something that costs nothing.
#[must_use]
pub fn infested_automaton(pool: &[sts2_engine::PoolCard]) -> EventDefinition {
    let of_type = |card_type: sts2_engine::CardType| -> Vec<sts2_engine::PoolCard> {
        pool.iter()
            .filter(|card| card.card_type == card_type)
            .cloned()
            .collect()
    };
    // A card that costs nothing and does not cost X: what the card itself
    // costs, before any upgrade or modifier.
    let free: Vec<sts2_engine::PoolCard> = pool
        .iter()
        .filter(|card| card.energy_cost == 0 && !card.costs_x)
        .cloned()
        .collect();
    EventDefinition {
        model_id: id("EVENT.INFESTED_AUTOMATON"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::Always,
        vars: Vec::new(),
        generation: EventGeneration::Fixed(vec![
            option(
                "INFESTED_AUTOMATON.pages.INITIAL.options.STUDY",
                "Study",
                vec![EventEffect::AddGeneratedCard {
                    pool: of_type(sts2_engine::CardType::Power),
                }],
            ),
            option(
                "INFESTED_AUTOMATON.pages.INITIAL.options.TOUCH_CORE",
                "Touch core",
                vec![EventEffect::AddGeneratedCard { pool: free }],
            ),
        ]),
    }
}

/// Pael, the ancient who offers three of its own: one out of each of three
/// pools, where two of them take the entries the run qualifies for.
#[must_use]
pub fn pael() -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.PAEL"),
        combat_layout: None,
        heals_on_entry: true,
        allowed: EventCondition::Always,
        vars: Vec::new(),
        generation: EventGeneration::PaelRelics {
            pool_one: vec![
                id("RELIC.PAELS_FLESH"),
                id("RELIC.PAELS_HORN"),
                id("RELIC.PAELS_TEARS"),
            ],
            pool_two: vec![
                sts2_engine::ConditionalRelic {
                    relic: id("RELIC.PAELS_WING"),
                    available: EventCondition::Always,
                },
                // Goopy wants a Defend, and the claw wants three of them.
                sts2_engine::ConditionalRelic {
                    relic: id("RELIC.PAELS_CLAW"),
                    available: EventCondition::DeckCardsAtLeast {
                        filter: EventCardFilter::EnchantableDefend,
                        count: 3,
                    },
                },
                sts2_engine::ConditionalRelic {
                    relic: id("RELIC.PAELS_TOOTH"),
                    available: EventCondition::DeckCardsAtLeast {
                        filter: EventCardFilter::Removable,
                        count: 5,
                    },
                },
            ],
            doubled_last: id("RELIC.PAELS_GROWTH"),
            pool_three: vec![
                sts2_engine::ConditionalRelic {
                    relic: id("RELIC.PAELS_EYE"),
                    available: EventCondition::Always,
                },
                sts2_engine::ConditionalRelic {
                    relic: id("RELIC.PAELS_BLOOD"),
                    available: EventCondition::Always,
                },
                // The condition asks for the Necrobinder's Osty, which no run
                // this baseline generates can have.
                sts2_engine::ConditionalRelic {
                    relic: id("RELIC.PAELS_LEGION"),
                    available: EventCondition::Always,
                },
            ],
        },
    }
}

/// A statue holding a sword over water. Take the sword, or dive for the purse
/// under it and pay in blood.
#[must_use]
pub fn sunken_statue() -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.SUNKEN_STATUE"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::Always,
        vars: Vec::new(),
        generation: EventGeneration::Fixed(vec![
            option(
                "SUNKEN_STATUE.pages.INITIAL.options.GRAB_SWORD",
                "Grab sword",
                vec![EventEffect::ObtainRelic(id("RELIC.SWORD_OF_STONE"))],
            ),
            option(
                "SUNKEN_STATUE.pages.INITIAL.options.DIVE_INTO_WATER",
                "Dive into water",
                vec![
                    EventEffect::GainGold(EventAmount::Rolled { min: 101, max: 122 }),
                    EventEffect::LoseHp(EventAmount::Fixed(7)),
                ],
            ),
        ]),
    }
}

/// A maze and a stranger. Going alone pays better and hurts; going together
/// pays less and costs nothing.
///
/// The build shifts each purse by a fractional roll around it, in the order
/// the vars are declared, which is the order the options are laid out. The solo
/// option also shuffles a three-entry list of hit effects before
/// it strikes; that draws from the event's own generator and nothing in the
/// event reads it afterwards, so it is presentation and nothing else.
#[must_use]
pub fn jungle_maze_adventure() -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.JUNGLE_MAZE_ADVENTURE"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::Always,
        vars: Vec::new(),
        generation: EventGeneration::Fixed(vec![
            option(
                "JUNGLE_MAZE_ADVENTURE.pages.INITIAL.options.SOLO_QUEST",
                "Solo quest",
                vec![
                    EventEffect::GainGold(EventAmount::RolledAround {
                        base: 150,
                        spread: 15,
                    }),
                    EventEffect::LoseHp(EventAmount::Fixed(18)),
                ],
            ),
            option(
                "JUNGLE_MAZE_ADVENTURE.pages.INITIAL.options.JOIN_FORCES",
                "Join forces",
                vec![EventEffect::GainGold(EventAmount::RolledAround {
                    base: 50,
                    spread: 15,
                })],
            ),
        ]),
    }
}

/// Two chests. The plain one costs blood and pays gold; the ornate one pays a
/// relic and a curse.
#[must_use]
pub fn this_or_that() -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.THIS_OR_THAT"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::Always,
        vars: Vec::new(),
        generation: EventGeneration::Fixed(vec![
            option(
                "THIS_OR_THAT.pages.INITIAL.options.PLAIN",
                "Plain",
                vec![
                    EventEffect::LoseHp(EventAmount::Fixed(6)),
                    EventEffect::GainGold(EventAmount::Rolled { min: 41, max: 69 }),
                ],
            ),
            option(
                "THIS_OR_THAT.pages.INITIAL.options.ORNATE",
                "Ornate",
                vec![
                    EventEffect::ObtainRolledRelic,
                    EventEffect::AddCard(sts2_engine::CardFingerprint::base(id("CARD.CLUMSY"))),
                ],
            ),
        ]),
    }
}

/// A sapphire seed. Eat it and be healed nine and upgrade a card of the deck,
/// or plant it and sow one of them instead.
#[must_use]
pub fn sapphire_seed() -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.SAPPHIRE_SEED"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::Always,
        vars: Vec::new(),
        generation: EventGeneration::Fixed(vec![
            option(
                "SAPPHIRE_SEED.pages.INITIAL.options.EAT",
                "Eat",
                vec![
                    EventEffect::Heal(EventAmount::Fixed(9)),
                    EventEffect::Run(vec![sts2_engine::Command::CardSelect(
                        sts2_engine::CardSelectCmd::FromDeckForUpgrade {
                            semantics: ChoiceSemantics::known(vec![ChoiceOp::new(
                                ChoiceOperationKind::Upgrade,
                                ChoiceScope::Run,
                            )]),
                            prefs: sts2_engine::SelectorPrefs::exactly(1),
                            then: vec![sts2_engine::Command::Card(sts2_engine::CardCmd::Upgrade {
                                cards: sts2_engine::CardSelector::Chosen,
                            })],
                        },
                    )]),
                ],
            ),
            option(
                "SAPPHIRE_SEED.pages.INITIAL.options.PLANT",
                "Plant",
                vec![EventEffect::Run(vec![sts2_engine::Command::CardSelect(
                    sts2_engine::CardSelectCmd::FromDeckForEnchantment {
                        semantics: ChoiceSemantics::known(vec![
                            ChoiceOp::new(ChoiceOperationKind::Enchant, ChoiceScope::Run)
                                .model(id("ENCHANTMENT.SOWN"))
                                .amount(1),
                        ]),
                        enchantment: id("ENCHANTMENT.SOWN"),
                        amount: 1,
                        filter: sts2_engine::CardFilter::Any,
                        shuffled_on: None,
                        prefs: sts2_engine::SelectorPrefs::exactly(1),
                        then: vec![sts2_engine::Command::Card(sts2_engine::CardCmd::Enchant {
                            cards: sts2_engine::CardSelector::Chosen,
                            enchantment: id("ENCHANTMENT.SOWN"),
                            amount: sts2_engine::Amount::Fixed(1),
                        })],
                    },
                )])],
            ),
        ]),
    }
}

/// An aroma of chaos. Let go and one card of the deck becomes another; keep
/// hold and one card of the deck is upgraded.
#[must_use]
pub fn aroma_of_chaos() -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.AROMA_OF_CHAOS"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::Always,
        vars: Vec::new(),
        generation: EventGeneration::Fixed(vec![
            option(
                "AROMA_OF_CHAOS.pages.INITIAL.options.LET_GO",
                "Let go",
                vec![EventEffect::Run(vec![sts2_engine::Command::CardSelect(
                    sts2_engine::CardSelectCmd::FromDeckForTransformation {
                        semantics: ChoiceSemantics::known(vec![
                            ChoiceOp::new(ChoiceOperationKind::Transform, ChoiceScope::Run)
                                .rolled(),
                        ]),
                        prefs: sts2_engine::SelectorPrefs::exactly(1),
                        then: vec![sts2_engine::Command::Card(
                            sts2_engine::CardCmd::TransformDeckCards {
                                cards: sts2_engine::CardSelector::Chosen,
                                replacement: sts2_engine::CardTransformReplacement::Rolled {
                                    stream: sts2_engine::TransformStream::Event,
                                    upgraded: false,
                                },
                                one_at_a_time: false,
                            },
                        )],
                    },
                )])],
            ),
            option(
                "AROMA_OF_CHAOS.pages.INITIAL.options.MAINTAIN_CONTROL",
                "Maintain control",
                vec![EventEffect::Run(vec![sts2_engine::Command::CardSelect(
                    sts2_engine::CardSelectCmd::FromDeckForUpgrade {
                        semantics: ChoiceSemantics::known(vec![ChoiceOp::new(
                            ChoiceOperationKind::Upgrade,
                            ChoiceScope::Run,
                        )]),
                        prefs: sts2_engine::SelectorPrefs::exactly(1),
                        then: vec![sts2_engine::Command::Card(sts2_engine::CardCmd::Upgrade {
                            cards: sts2_engine::CardSelector::Chosen,
                        })],
                    },
                )])],
            ),
        ]),
    }
}

/// A potion courier. Grab the three foul potions it is carrying, or ransack it
/// for one uncommon potion rolled out of what the run could be offered.
#[must_use]
pub fn potion_courier(uncommon_potions: Vec<sts2_core::ModelId>) -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.POTION_COURIER"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::ActIndexAbove(0),
        vars: Vec::new(),
        generation: EventGeneration::Fixed(vec![
            option(
                "POTION_COURIER.pages.INITIAL.options.GRAB_POTIONS",
                "Grab potions",
                vec![EventEffect::Rewards(
                    std::iter::repeat_with(|| {
                        sts2_engine::RewardFingerprint::of("potion")
                            .with_model(id("POTION.FOUL_POTION"))
                    })
                    .take(3)
                    .collect(),
                )],
            ),
            option(
                "POTION_COURIER.pages.INITIAL.options.RANSACK",
                "Ransack",
                vec![EventEffect::OfferRolledPotion {
                    candidates: uncommon_potions,
                }],
            ),
        ]),
    }
}

/// A room of mirrors. Touch one and two upgrades come off the deck and four go
/// on; shatter them and the deck doubles and takes a curse for it.
#[must_use]
pub fn reflections() -> EventDefinition {
    let rolled = |filter, take| sts2_engine::CardSelector::RolledFromDeckDistinct {
        filter,
        stream: sts2_engine::ShuffleStream::Event,
        take,
    };
    EventDefinition {
        model_id: id("EVENT.REFLECTIONS"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::Always,
        vars: Vec::new(),
        generation: EventGeneration::Fixed(vec![
            option(
                "REFLECTIONS.pages.INITIAL.options.TOUCH_A_MIRROR",
                "Touch a mirror",
                // Two loops, and the second builds its list after the first has
                // run: a card the downgrade took an upgrade off is upgradable
                // again by the time the upgrades are handed out.
                vec![EventEffect::Run(vec![
                    sts2_engine::Command::Card(sts2_engine::CardCmd::Downgrade {
                        cards: rolled(
                            sts2_engine::CardFilter::Upgraded,
                            sts2_engine::Amount::Fixed(2),
                        ),
                    }),
                    sts2_engine::Command::Card(sts2_engine::CardCmd::Upgrade {
                        cards: rolled(
                            sts2_engine::CardFilter::Upgradable,
                            sts2_engine::Amount::Fixed(4),
                        ),
                    }),
                ])],
            ),
            option(
                "REFLECTIONS.pages.INITIAL.options.SHATTER",
                "Shatter",
                vec![
                    EventEffect::Run(vec![sts2_engine::Command::CardPile(
                        sts2_engine::CardPileCmd::CloneTheDeckIntoItself,
                    )]),
                    EventEffect::AddCard(sts2_engine::CardFingerprint::base(id("CARD.BAD_LUCK"))),
                ],
            ),
        ]),
    }
}

/// A trial. Accept it and one of three defendants is rolled; each pair of
/// verdicts costs a curse and pays something. Reject it and the page offers the
/// trial again beside a proceed that abandons the run.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "three defendants and two verdicts each, which is the page this is"
)]
pub fn trial(pool: &sts2_engine::GeneratedRewardSpec) -> EventDefinition {
    let curse = |model: &str| EventEffect::AddCard(sts2_engine::CardFingerprint::base(id(model)));
    let accept = |option_id: &str| {
        option(
            option_id,
            "Accept",
            vec![EventEffect::SetRolledPage(vec![
                // The first of the three pages rolled between: the merchant.
                vec![
                    option(
                        "TRIAL.pages.MERCHANT.options.GUILTY",
                        "Guilty",
                        vec![
                            curse("CARD.REGRET"),
                            EventEffect::ObtainRolledRelic,
                            EventEffect::ObtainRolledRelic,
                        ],
                    ),
                    option(
                        "TRIAL.pages.MERCHANT.options.INNOCENT",
                        "Innocent",
                        vec![
                            curse("CARD.SHAME"),
                            EventEffect::Run(vec![sts2_engine::Command::CardSelect(
                                sts2_engine::CardSelectCmd::FromDeckForUpgrade {
                                    semantics: ChoiceSemantics::known(vec![ChoiceOp::new(
                                        ChoiceOperationKind::Upgrade,
                                        ChoiceScope::Run,
                                    )]),
                                    prefs: sts2_engine::SelectorPrefs::exactly(2),
                                    then: vec![sts2_engine::Command::Card(
                                        sts2_engine::CardCmd::Upgrade {
                                            cards: sts2_engine::CardSelector::Chosen,
                                        },
                                    )],
                                },
                            )]),
                        ],
                    ),
                ],
                // Case 1: the noble.
                vec![
                    option(
                        "TRIAL.pages.NOBLE.options.GUILTY",
                        "Guilty",
                        vec![EventEffect::Heal(EventAmount::Fixed(10))],
                    ),
                    option(
                        "TRIAL.pages.NOBLE.options.INNOCENT",
                        "Innocent",
                        vec![
                            curse("CARD.REGRET"),
                            EventEffect::GainGold(EventAmount::Fixed(300)),
                        ],
                    ),
                ],
                // Case 2: the nondescript one.
                vec![
                    option(
                        "TRIAL.pages.NONDESCRIPT.options.GUILTY",
                        "Guilty",
                        vec![
                            curse("CARD.DOUBT"),
                            EventEffect::OfferCardRewards {
                                count: 2,
                                spec: Box::new(pool.clone()),
                            },
                        ],
                    ),
                    option(
                        "TRIAL.pages.NONDESCRIPT.options.INNOCENT",
                        "Innocent",
                        vec![
                            curse("CARD.DOUBT"),
                            EventEffect::Run(vec![sts2_engine::Command::CardSelect(
                                sts2_engine::CardSelectCmd::FromDeckForTransformation {
                                    semantics: ChoiceSemantics::known(vec![
                                        ChoiceOp::new(
                                            ChoiceOperationKind::Transform,
                                            ChoiceScope::Run,
                                        )
                                        .rolled(),
                                    ]),
                                    prefs: sts2_engine::SelectorPrefs::exactly(2),
                                    then: vec![sts2_engine::Command::Card(
                                        sts2_engine::CardCmd::TransformDeckCards {
                                            cards: sts2_engine::CardSelector::Chosen,
                                            replacement:
                                                sts2_engine::CardTransformReplacement::Rolled {
                                                    stream: sts2_engine::TransformStream::Event,
                                                    upgraded: false,
                                                },
                                            // One card at a time, in the
                                            // order the screen handed them
                                            // back.
                                            one_at_a_time: true,
                                        },
                                    )],
                                },
                            )]),
                        ],
                    ),
                ],
            ])],
        )
    };
    EventDefinition {
        model_id: id("EVENT.TRIAL"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::Always,
        vars: Vec::new(),
        generation: EventGeneration::Fixed(vec![
            accept("TRIAL.pages.INITIAL.options.ACCEPT"),
            option(
                "TRIAL.pages.INITIAL.options.REJECT",
                "Reject",
                vec![EventEffect::SetPage(vec![
                    accept("TRIAL.pages.REJECT.options.ACCEPT"),
                    // The proceed on this page opens the abandon-run
                    // confirmation, which is not a gameplay decision this
                    // simulator offers.
                    EventOption {
                        option_id: "TRIAL.pages.REJECT.options.DOUBLE_DOWN".to_owned(),
                        label: "Double down".to_owned(),
                        effects: vec![EventEffect::Unsupported(id("EVENT.TRIAL"))],
                        is_proceed: true,
                        was_chosen: false,
                        locked: false,
                    },
                ])],
            ),
        ]),
    }
}

/// A merchant who is not one. Its page carries no options at all, so walking
/// in is walking through, and the run leaves by the map.
///
/// What it does carry is a shelf: entering the room shuffles nine fake relics
/// and stocks six of them at fifty gold each, on a custom screen of its
/// own. The shelf is bought off the way a merchant's is — the same buy action,
/// each entry priced on the shops stream — except that the event owns it, so
/// nothing prices it through the merchant hook and a bought slot is never
/// restocked. That screen is not a merchant room, and no `merchant` choice
/// stands for the visit. A foul potion thrown at him buys the
/// rug, whatever is still stocked, and the fight he starts without leaving the
/// page, which the belt reaches wherever the page is the run's room. He turns
/// once — the page is handed off to that fight and not handed back.
#[must_use]
pub fn fake_merchant() -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.FAKE_MERCHANT"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::All(vec![
            EventCondition::ActIndexAbove(0),
            EventCondition::Any(vec![
                EventCondition::GoldAtLeast(100),
                EventCondition::HoldsPotion(id("POTION.FOUL_POTION")),
            ]),
        ]),
        vars: Vec::new(),
        generation: EventGeneration::MerchantShelf {
            relics: vec![
                id("RELIC.FAKE_ANCHOR"),
                id("RELIC.FAKE_BLOOD_VIAL"),
                id("RELIC.FAKE_HAPPY_FLOWER"),
                id("RELIC.FAKE_LEES_WAFFLE"),
                id("RELIC.FAKE_MANGO"),
                id("RELIC.FAKE_ORICHALCUM"),
                id("RELIC.FAKE_SNECKO_EYE"),
                id("RELIC.FAKE_STRIKE_DUMMY"),
                id("RELIC.FAKE_VENERABLE_TEA_SET"),
            ],
            offered: 6,
            // Every one of these relics sets its own merchant cost to the
            // same fifty, which the entry then prices around.
            cost: 50,
        },
    }
}

/// A tablet nobody can read. Smash it and be healed twenty, or decipher it and
/// pay maximum HP for an upgrade, five times over at a rising price.
///
/// The next price is set by how many times the tablet has already been paid —
/// 6, 12, 24, and then everything but one point — and each payment upgrades one
/// card of the deck it rolls for, except on the fifth, which upgrades the whole
/// deck. A price the player cannot pay takes them to one maximum HP and
/// kills them there.
#[must_use]
pub fn tablet_of_truth() -> EventDefinition {
    // The page closes and the room is done, which is what an option with
    // nothing in its body comes to.
    let give_up = || {
        option(
            "TABLET_OF_TRUTH.pages.DECIPHER.options.GIVE_UP",
            "Give up",
            Vec::new(),
        )
    };
    // The last decipher: the price is everything but one point, the whole deck
    // is upgraded, and the fifth payment finishes the event instead of laying
    // out another page.
    let mut page = vec![
        option(
            "TABLET_OF_TRUTH.pages.DECIPHER_4.options.DECIPHER",
            "Decipher",
            vec![EventEffect::LoseMaxHpAndUpgrade {
                amount: EventAmount::MaxHpLessOne,
                whole_deck: true,
            }],
        ),
        give_up(),
    ];
    // Deciphers four, three, and two, each laying out the page behind it.
    for (deciphered, cost) in [(3, 24), (2, 12), (1, 6)] {
        page = vec![
            option(
                &format!("TABLET_OF_TRUTH.pages.DECIPHER_{deciphered}.options.DECIPHER"),
                "Decipher",
                vec![
                    EventEffect::LoseMaxHpAndUpgrade {
                        amount: EventAmount::Fixed(cost),
                        whole_deck: false,
                    },
                    EventEffect::SetPage(page),
                ],
            ),
            give_up(),
        ];
    }
    EventDefinition {
        model_id: id("EVENT.TABLET_OF_TRUTH"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::Always,
        vars: Vec::new(),
        generation: EventGeneration::Fixed(vec![
            option(
                "TABLET_OF_TRUTH.pages.INITIAL.options.DECIPHER_1",
                "Decipher",
                vec![
                    EventEffect::LoseMaxHpAndUpgrade {
                        amount: EventAmount::Fixed(3),
                        whole_deck: false,
                    },
                    EventEffect::SetPage(page),
                ],
            ),
            option(
                "TABLET_OF_TRUTH.pages.INITIAL.options.SMASH",
                "Smash",
                vec![EventEffect::Heal(EventAmount::Fixed(20))],
            ),
        ]),
    }
}

/// A room of cheese: eat your fill, or search it and pay in blood.
#[must_use]
pub fn room_full_of_cheese(commons: Vec<sts2_core::ModelId>) -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.ROOM_FULL_OF_CHEESE"),
        combat_layout: None,
        heals_on_entry: false,
        // Offered in the first two acts only.
        allowed: EventCondition::ActIndexBelow(2),
        vars: Vec::new(),
        generation: EventGeneration::Fixed(vec![
            option(
                "ROOM_FULL_OF_CHEESE.pages.INITIAL.options.GORGE",
                "Gorge",
                vec![EventEffect::ChooseGeneratedCards {
                    count: 8,
                    picks: 2,
                    pool: commons,
                }],
            ),
            option(
                "ROOM_FULL_OF_CHEESE.pages.INITIAL.options.SEARCH",
                "Search",
                vec![
                    EventEffect::LoseHp(EventAmount::Fixed(14)),
                    EventEffect::ObtainRelic(id("RELIC.CHOSEN_CHEESE")),
                ],
            ),
        ]),
    }
}

/// A slippery bridge. One card of the deck is rolled as the page is built:
/// let it go and it leaves the deck, or hold on for three — one more for every
/// hold before it — and another card is rolled in its place.
///
/// It is offered past floor six, and only to a deck holding a removable card,
/// which every deck of this baseline does.
#[must_use]
pub fn slippery_bridge() -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.SLIPPERY_BRIDGE"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::All(vec![
            EventCondition::TotalFloorAbove(6),
            EventCondition::DeckCardsAtLeast {
                filter: sts2_engine::EventCardFilter::Removable,
                count: 1,
            },
        ]),
        vars: Vec::new(),
        generation: EventGeneration::SlipperyBridge,
    }
}

/// A crystal sphere. Pay it fifty gold and a little more for three divinations,
/// or take a Debt for six, and then clear the fog off an eleven-by-eleven board
/// with fifteen things buried under it.
///
/// The price is shifted by a whole number rolled onto it as the room is
/// entered, which is one read of the event's own stream; the board's fifteen
/// placements are the next fifteen.
#[must_use]
pub fn crystal_sphere() -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.CRYSTAL_SPHERE"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::All(vec![
            EventCondition::GoldAtLeast(100),
            EventCondition::ActIndexAbove(0),
        ]),
        vars: Vec::new(),
        generation: EventGeneration::Fixed(vec![
            option(
                "CRYSTAL_SPHERE.pages.INITIAL.options.UNCOVER_FUTURE",
                "Uncover the future",
                vec![
                    EventEffect::LoseGold(EventAmount::RolledOnto {
                        base: 50,
                        min: 1,
                        max: 50,
                    }),
                    EventEffect::PlayTheCrystalSphere { divinations: 3 },
                ],
            ),
            option(
                "CRYSTAL_SPHERE.pages.INITIAL.options.PAYMENT_PLAN",
                "Payment plan",
                vec![
                    EventEffect::AddCard(sts2_engine::CardFingerprint::base(id("CARD.DEBT"))),
                    EventEffect::PlayTheCrystalSphere { divinations: 6 },
                ],
            ),
        ]),
    }
}

/// A whispering hollow. Pay it thirty-five gold or so and be offered two
/// potions, or hug it: one card of the deck becomes another, and nine damage
/// nothing blocks and nothing scales.
///
/// The gold is shifted by a whole number rolled around it as the room is
/// entered, which is one read of the event's own stream.
#[must_use]
pub fn whispering_hollow() -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.WHISPERING_HOLLOW"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::GoldAtLeast(44),
        vars: Vec::new(),
        generation: EventGeneration::Fixed(vec![
            option(
                "WHISPERING_HOLLOW.pages.INITIAL.options.GOLD",
                "Gold",
                vec![
                    EventEffect::LoseGold(EventAmount::RolledOnto {
                        base: 35,
                        min: -9,
                        max: 10,
                    }),
                    // A custom reward set of two potions, both rolled as the
                    // set is settled.
                    EventEffect::Run(vec![
                        sts2_engine::Command::Rewards(sts2_engine::RewardsCmd::AddPotion),
                        sts2_engine::Command::Rewards(sts2_engine::RewardsCmd::AddPotion),
                        sts2_engine::Command::Rewards(sts2_engine::RewardsCmd::OfferAdded),
                    ]),
                ],
            ),
            option(
                "WHISPERING_HOLLOW.pages.INITIAL.options.HUG",
                "Hug",
                vec![EventEffect::Run(vec![
                    sts2_engine::Command::CardSelect(
                        sts2_engine::CardSelectCmd::FromDeckForTransformation {
                            semantics: ChoiceSemantics::known(vec![
                                ChoiceOp::new(ChoiceOperationKind::Transform, ChoiceScope::Run)
                                    .rolled(),
                            ]),
                            prefs: sts2_engine::SelectorPrefs::exactly(1),
                            then: vec![sts2_engine::Command::Card(
                                sts2_engine::CardCmd::TransformDeckCards {
                                    cards: sts2_engine::CardSelector::Chosen,
                                    replacement: sts2_engine::CardTransformReplacement::Rolled {
                                        stream: sts2_engine::TransformStream::Event,
                                        upgraded: false,
                                    },
                                    // Each card is finished before the next
                                    // one is looked at.
                                    one_at_a_time: true,
                                },
                            )],
                        },
                    ),
                    sts2_engine::Command::Creature(sts2_engine::CreatureCmd::Damage {
                        target: sts2_engine::CreatureSelector::Owner,
                        amount: sts2_engine::Amount::Fixed(9),
                        props: sts2_engine::ValueProps::UNBLOCKABLE,
                    }),
                ])],
            ),
        ]),
    }
}

/// An event whose page never varies, heals nobody on the way in, and rolls
/// nothing before it is built.
fn plain(model_id: &str, allowed: EventCondition, options: Vec<EventOption>) -> EventDefinition {
    EventDefinition {
        model_id: id(model_id),
        combat_layout: None,
        heals_on_entry: false,
        allowed,
        vars: Vec::new(),
        generation: EventGeneration::Fixed(options),
    }
}

/// The same over a page whose options each stand behind a test.
fn gated(
    model_id: &str,
    allowed: EventCondition,
    options: Vec<sts2_engine::ConditionalOption>,
) -> EventDefinition {
    EventDefinition {
        model_id: id(model_id),
        combat_layout: None,
        heals_on_entry: false,
        allowed,
        vars: Vec::new(),
        generation: EventGeneration::Conditional(options),
    }
}

/// One option of a gated page that nothing actually gates: the build writes it
/// out unconditionally, so the locked form it would show is its own id.
fn ungated(option: EventOption) -> sts2_engine::ConditionalOption {
    sts2_engine::ConditionalOption {
        available: EventCondition::Always,
        locked_option_id: option.option_id.clone(),
        option,
    }
}

/// One card of the deck taken away and another put in its place, which is the
/// removal screen an event opens and the curse it hands over behind it.
fn remove_for_a_curse(count: usize, curse: &str) -> EventEffect {
    EventEffect::Run(vec![sts2_engine::Command::CardSelect(
        sts2_engine::CardSelectCmd::FromDeckForRemoval {
            semantics: ChoiceSemantics::known(vec![
                ChoiceOp::new(ChoiceOperationKind::Remove, ChoiceScope::Run)
                    .to(ChoiceDestination::Removed),
            ]),
            prefs: sts2_engine::SelectorPrefs::exactly(count),
            filter: sts2_engine::CardFilter::Any,
            then: vec![
                sts2_engine::Command::CardPile(sts2_engine::CardPileCmd::RemoveFromDeck {
                    cards: sts2_engine::CardSelector::Chosen,
                }),
                sts2_engine::Command::CardPile(sts2_engine::CardPileCmd::AddToDeck {
                    card: sts2_engine::CardOrigin::Model(id(curse)),
                }),
            ],
        },
    )])
}

/// Picks that many cards out of the deck and works the enchantment onto every
/// one that came back.
fn enchant_deck_cards(enchantment: &str, picks: usize) -> EventEffect {
    EventEffect::Run(vec![sts2_engine::Command::CardSelect(
        sts2_engine::CardSelectCmd::FromDeckForEnchantment {
            semantics: ChoiceSemantics::known(vec![
                ChoiceOp::new(ChoiceOperationKind::Enchant, ChoiceScope::Run)
                    .model(id(enchantment))
                    .amount(1),
            ]),
            enchantment: id(enchantment),
            amount: 1,
            filter: sts2_engine::CardFilter::Any,
            shuffled_on: None,
            prefs: sts2_engine::SelectorPrefs::exactly(picks),
            then: vec![sts2_engine::Command::Card(sts2_engine::CardCmd::Enchant {
                cards: sts2_engine::CardSelector::Chosen,
                enchantment: id(enchantment),
                amount: sts2_engine::Amount::Fixed(1),
            })],
        },
    )])
}

/// Hungry for mushrooms: a big one for twenty maximum HP and a bigger hand, or
/// a fragrant one that costs fifteen and upgrades two cards.
///
/// Both options are relic options, so each is named for the relic it hands
/// over and does nothing but obtain it: the fifteen the second one advertises
/// is what that relic itself does once obtained.
#[must_use]
pub fn hungry_for_mushrooms() -> EventDefinition {
    plain(
        "EVENT.HUNGRY_FOR_MUSHROOMS",
        EventCondition::Always,
        vec![
            option(
                "HUNGRY_FOR_MUSHROOMS.pages.INITIAL.options.BIG_MUSHROOM",
                "BIG_MUSHROOM",
                vec![EventEffect::ObtainRelic(id("RELIC.BIG_MUSHROOM"))],
            ),
            option(
                "HUNGRY_FOR_MUSHROOMS.pages.INITIAL.options.FRAGRANT_MUSHROOM",
                "FRAGRANT_MUSHROOM",
                vec![EventEffect::ObtainRelic(id("RELIC.FRAGRANT_MUSHROOM"))],
            ),
        ],
    )
}

/// The bugslayer, who hands over one of two cards.
#[must_use]
pub fn bugslayer() -> EventDefinition {
    plain(
        "EVENT.BUGSLAYER",
        EventCondition::Always,
        vec![
            option(
                "BUGSLAYER.pages.INITIAL.options.EXTERMINATION",
                "Extermination",
                vec![EventEffect::AddCard(sts2_engine::CardFingerprint::base(
                    id("CARD.EXTERMINATE"),
                ))],
            ),
            option(
                "BUGSLAYER.pages.INITIAL.options.SQUASH",
                "Squash",
                vec![EventEffect::AddCard(sts2_engine::CardFingerprint::base(
                    id("CARD.SQUASH"),
                ))],
            ),
        ],
    )
}

/// A byrdonis nest: eat the egg for seven maximum HP, or take it as a card.
///
/// It is only offered to a player with no event pet, which is what keeps a run
/// that already carries an egg or a pet from being offered another.
#[must_use]
pub fn byrdonis_nest() -> EventDefinition {
    plain(
        "EVENT.BYRDONIS_NEST",
        EventCondition::NoEventPet,
        vec![
            option(
                "BYRDONIS_NEST.pages.INITIAL.options.EAT",
                "Eat",
                vec![EventEffect::GainMaxHp(7)],
            ),
            option(
                "BYRDONIS_NEST.pages.INITIAL.options.TAKE",
                "Take",
                vec![EventEffect::AddCard(sts2_engine::CardFingerprint::base(
                    id("CARD.BYRDONIS_EGG"),
                ))],
            ),
        ],
    )
}

/// A wellspring: bottle it for a potion rolled off the belt's own options, or
/// bathe in it and pay one card of the deck for a Guilty.
#[must_use]
pub fn wellspring(potions: Vec<sts2_core::ModelId>) -> EventDefinition {
    plain(
        "EVENT.WELLSPRING",
        EventCondition::Always,
        vec![
            option(
                "WELLSPRING.pages.INITIAL.options.BOTTLE",
                "Bottle",
                vec![EventEffect::OfferRolledPotion {
                    candidates: potions,
                }],
            ),
            option(
                "WELLSPRING.pages.INITIAL.options.BATHE",
                "Bathe",
                vec![remove_for_a_curse(1, "CARD.GUILTY")],
            ),
        ],
    )
}

/// A drowning beacon: bottle the water for a Glowwater Potion, or climb the
/// lighthouse for thirteen maximum HP and the lens at the top of it.
#[must_use]
pub fn drowning_beacon() -> EventDefinition {
    plain(
        "EVENT.DROWNING_BEACON",
        EventCondition::Always,
        vec![
            option(
                "DROWNING_BEACON.pages.INITIAL.options.BOTTLE",
                "Bottle",
                // The potion is named rather than rolled, so nothing is read.
                vec![EventEffect::Rewards(vec![
                    sts2_engine::RewardFingerprint::of("potion")
                        .with_model(id("POTION.GLOWWATER_POTION")),
                ])],
            ),
            option(
                "DROWNING_BEACON.pages.INITIAL.options.CLIMB",
                "Climb",
                vec![
                    EventEffect::LoseMaxHp(EventAmount::Fixed(13)),
                    EventEffect::ObtainRelic(id("RELIC.FRESNEL_LENS")),
                ],
            ),
        ],
    )
}

/// A sunken treasury: a small chest of gold, or a large one with a Greed in it.
///
/// Both purses are rolled as the room is entered — the small one first — so
/// both are settled whichever chest is opened.
#[must_use]
pub fn sunken_treasury() -> EventDefinition {
    plain(
        "EVENT.SUNKEN_TREASURY",
        EventCondition::Always,
        vec![
            option(
                "SUNKEN_TREASURY.pages.INITIAL.options.FIRST_CHEST",
                "First chest",
                // Sixty, shifted by a roll of −8 to 7.
                vec![EventEffect::GainGold(EventAmount::RolledOnto {
                    base: 60,
                    min: -8,
                    max: 8,
                })],
            ),
            option(
                "SUNKEN_TREASURY.pages.INITIAL.options.SECOND_CHEST",
                "Second chest",
                // Three hundred and thirty-three, shifted by −30 to 30.
                vec![
                    EventEffect::GainGold(EventAmount::RolledOnto {
                        base: 333,
                        min: -30,
                        max: 31,
                    }),
                    EventEffect::AddCard(sts2_engine::CardFingerprint::base(id("CARD.GREED"))),
                ],
            ),
        ],
    )
}

/// A morphic grove: hand over every coin and two cards of the deck become two
/// others, or walk on alone for five maximum HP.
#[must_use]
pub fn morphic_grove() -> EventDefinition {
    plain(
        "EVENT.MORPHIC_GROVE",
        EventCondition::All(vec![
            EventCondition::GoldAtLeast(100),
            EventCondition::DeckCardsAtLeast {
                filter: EventCardFilter::Transformable,
                count: 2,
            },
        ]),
        vec![
            option(
                "MORPHIC_GROVE.pages.INITIAL.options.GROUP",
                "Group",
                vec![EventEffect::Run(vec![
                    // Every coin the player is carrying.
                    sts2_engine::Command::Player(sts2_engine::PlayerCmd::LoseGold(
                        sts2_engine::Amount::counting(sts2_engine::Count::PlayerGold),
                    )),
                    sts2_engine::Command::CardSelect(
                        sts2_engine::CardSelectCmd::FromDeckForTransformation {
                            semantics: ChoiceSemantics::known(vec![
                                ChoiceOp::new(ChoiceOperationKind::Transform, ChoiceScope::Run)
                                    .rolled(),
                            ]),
                            prefs: sts2_engine::SelectorPrefs::exactly(2),
                            then: vec![sts2_engine::Command::Card(
                                sts2_engine::CardCmd::TransformDeckCards {
                                    cards: sts2_engine::CardSelector::Chosen,
                                    replacement: sts2_engine::CardTransformReplacement::Rolled {
                                        stream: sts2_engine::TransformStream::Event,
                                        upgraded: false,
                                    },
                                    // One card at a time, in the order they
                                    // came back.
                                    one_at_a_time: true,
                                },
                            )],
                        },
                    ),
                ])],
            ),
            option(
                "MORPHIC_GROVE.pages.INITIAL.options.LONER",
                "Loner",
                vec![EventEffect::GainMaxHp(5)],
            ),
        ],
    )
}

/// An unrest site: sleep badly and wake up whole, or put the thing down for
/// eight maximum HP and a relic off the front of the bag.
///
/// The heal is everything the player is missing, read off the player rather
/// than rolled.
#[must_use]
pub fn unrest_site() -> EventDefinition {
    plain(
        "EVENT.UNREST_SITE",
        EventCondition::CurrentHpAtMostFraction {
            numerator: 70,
            denominator: 100,
        },
        vec![
            option(
                "UNREST_SITE.pages.INITIAL.options.REST",
                "Rest",
                vec![
                    EventEffect::Heal(EventAmount::MissingHp),
                    EventEffect::AddCard(sts2_engine::CardFingerprint::base(id("CARD.POOR_SLEEP"))),
                ],
            ),
            option(
                "UNREST_SITE.pages.INITIAL.options.KILL",
                "Kill",
                vec![
                    EventEffect::LoseMaxHp(EventAmount::Fixed(8)),
                    EventEffect::ObtainRolledRelic,
                ],
            ),
        ],
    )
}

/// A field of man-sized holes: resist and pay two cards for a Normality, or
/// climb in and one card of the deck fits perfectly from then on.
#[must_use]
pub fn field_of_man_sized_holes() -> EventDefinition {
    plain(
        "EVENT.FIELD_OF_MAN_SIZED_HOLES",
        EventCondition::DeckCardsAtLeast {
            filter: EventCardFilter::Enchantable,
            count: 1,
        },
        vec![
            option(
                "FIELD_OF_MAN_SIZED_HOLES.pages.INITIAL.options.RESIST",
                "Resist",
                vec![remove_for_a_curse(2, "CARD.NORMALITY")],
            ),
            option(
                "FIELD_OF_MAN_SIZED_HOLES.pages.INITIAL.options.ENTER_YOUR_HOLE",
                "Enter your hole",
                vec![enchant_deck_cards("ENCHANTMENT.PERFECT_FIT", 1)],
            ),
        ],
    )
}

/// A grave of the forgotten: confront it for a Decay and a card that burns
/// itself out harder, or accept the soul it is offering.
///
/// The first option is laid out locked where the deck holds nothing the
/// enchantment can go on — which the event already refuses to be offered for,
/// so the act never lands a run there.
#[must_use]
pub fn grave_of_the_forgotten() -> EventDefinition {
    gated(
        "EVENT.GRAVE_OF_THE_FORGOTTEN",
        EventCondition::DeckCardsAtLeast {
            filter: EventCardFilter::EnchantableWithExhaust,
            count: 1,
        },
        vec![
            sts2_engine::ConditionalOption {
                available: EventCondition::DeckCardsAtLeast {
                    filter: EventCardFilter::EnchantableWithExhaust,
                    count: 1,
                },
                option: option(
                    "GRAVE_OF_THE_FORGOTTEN.pages.INITIAL.options.CONFRONT",
                    "Confront",
                    vec![
                        EventEffect::AddCard(sts2_engine::CardFingerprint::base(id("CARD.DECAY"))),
                        enchant_deck_cards("ENCHANTMENT.SOULS_POWER", 1),
                    ],
                ),
                locked_option_id: "GRAVE_OF_THE_FORGOTTEN.pages.INITIAL.options.CONFRONT_LOCKED"
                    .to_owned(),
            },
            ungated(option(
                "GRAVE_OF_THE_FORGOTTEN.pages.INITIAL.options.ACCEPT",
                "Accept",
                vec![EventEffect::ObtainRelic(id("RELIC.FORGOTTEN_SOUL"))],
            )),
        ],
    )
}

/// Wood carvings: a starter becomes a Peck, a card of the deck learns to
/// slither, or a starter becomes Toric Toughness.
///
/// The snake is laid out locked where nothing in the deck can take the
/// enchantment. The event is offered for any basic removable card, so a deck of
/// nothing but unplayable starters would reach the locked form.
#[must_use]
pub fn wood_carvings() -> EventDefinition {
    let carve = |name: &str, into: &str| {
        option(
            &format!("WOOD_CARVINGS.pages.INITIAL.options.{name}"),
            name,
            vec![EventEffect::Run(vec![sts2_engine::Command::CardSelect(
                sts2_engine::CardSelectCmd::FromDeckGeneric {
                    semantics: ChoiceSemantics::known(vec![
                        ChoiceOp::new(ChoiceOperationKind::Transform, ChoiceScope::Run)
                            .model(id(into))
                            .inheriting(true, false),
                    ]),
                    prefs: sts2_engine::SelectorPrefs::exactly(1),
                    filter: sts2_engine::CardFilter::BasicTransformable,
                    then: vec![sts2_engine::Command::Card(
                        sts2_engine::CardCmd::TransformDeckCards {
                            cards: sts2_engine::CardSelector::Chosen,
                            // The replacement is named, so nothing is rolled
                            // for it.
                            replacement: sts2_engine::CardTransformReplacement::Named {
                                model: id(into),
                                keeps_upgrade: true,
                                keeps_enchantment: false,
                            },
                            one_at_a_time: false,
                        },
                    )],
                },
            )])],
        )
    };
    gated(
        "EVENT.WOOD_CARVINGS",
        EventCondition::DeckCardsAtLeast {
            filter: EventCardFilter::BasicRemovable,
            count: 1,
        },
        vec![
            ungated(carve("BIRD", "CARD.PECK")),
            sts2_engine::ConditionalOption {
                available: EventCondition::DeckCardsAtLeast {
                    filter: EventCardFilter::EnchantableBy(id("ENCHANTMENT.SLITHER")),
                    count: 1,
                },
                option: option(
                    "WOOD_CARVINGS.pages.INITIAL.options.SNAKE",
                    "Snake",
                    vec![enchant_deck_cards("ENCHANTMENT.SLITHER", 1)],
                ),
                locked_option_id: "WOOD_CARVINGS.pages.INITIAL.options.SNAKE_LOCKED".to_owned(),
            },
            ungated(carve("TORUS", "CARD.TORIC_TOUGHNESS")),
        ],
    )
}

/// A waterlogged scriptorium: bloody ink for six maximum HP, a tentacle quill
/// for fifty-five gold and a steady card, or a prickly sponge for ninety-nine
/// and two of them.
#[must_use]
pub fn waterlogged_scriptorium() -> EventDefinition {
    gated(
        "EVENT.WATERLOGGED_SCRIPTORIUM",
        EventCondition::GoldAtLeast(55),
        vec![
            ungated(option(
                "WATERLOGGED_SCRIPTORIUM.pages.INITIAL.options.BLOODY_INK",
                "Bloody ink",
                vec![EventEffect::GainMaxHp(6)],
            )),
            sts2_engine::ConditionalOption {
                available: EventCondition::GoldAtLeast(55),
                option: option(
                    "WATERLOGGED_SCRIPTORIUM.pages.INITIAL.options.TENTACLE_QUILL",
                    "Tentacle quill",
                    vec![
                        EventEffect::LoseGold(EventAmount::Fixed(55)),
                        enchant_deck_cards("ENCHANTMENT.STEADY", 1),
                    ],
                ),
                locked_option_id:
                    "WATERLOGGED_SCRIPTORIUM.pages.INITIAL.options.TENTACLE_QUILL_LOCKED".to_owned(),
            },
            sts2_engine::ConditionalOption {
                available: EventCondition::GoldAtLeast(99),
                option: option(
                    "WATERLOGGED_SCRIPTORIUM.pages.INITIAL.options.PRICKLY_SPONGE",
                    "Prickly sponge",
                    vec![
                        EventEffect::LoseGold(EventAmount::Fixed(99)),
                        enchant_deck_cards("ENCHANTMENT.STEADY", 2),
                    ],
                ),
                locked_option_id:
                    "WATERLOGGED_SCRIPTORIUM.pages.INITIAL.options.PRICKLY_SPONGE_LOCKED".to_owned(),
            },
        ],
    )
}

/// A luminous choir: reach into the flesh and pay two cards for a Spore Mind,
/// or offer the tribute it asks for and take a relic off the front of the bag.
///
/// The hundred and forty-nine the tribute costs is discounted by a whole
/// number rolled as the room is entered, and the option is laid out locked
/// where the player cannot meet the discounted price. Whether the event may be
/// offered at all is decided on the undiscounted number, because the act asks
/// that before the room's own vars are settled.
#[must_use]
pub fn luminous_choir() -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.LUMINOUS_CHOIR"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::All(vec![
            EventCondition::GoldAtLeast(149),
            EventCondition::RelicGrabBagHasRelics,
        ]),
        // A hundred and forty-nine, less a roll of 0 to 49.
        vars: vec![EventAmount::RolledOffOf {
            base: 149,
            min: 0,
            max: 50,
        }],
        generation: EventGeneration::Conditional(vec![
            ungated(option(
                "LUMINOUS_CHOIR.pages.INITIAL.options.REACH_INTO_THE_FLESH",
                "Reach into the flesh",
                vec![remove_for_a_curse(2, "CARD.SPORE_MIND")],
            )),
            sts2_engine::ConditionalOption {
                available: EventCondition::GoldAtLeastVar(0),
                option: option(
                    "LUMINOUS_CHOIR.pages.INITIAL.options.OFFER_TRIBUTE",
                    "Offer tribute",
                    vec![
                        EventEffect::LoseGold(EventAmount::Var(0)),
                        EventEffect::ObtainRolledRelic,
                    ],
                ),
                locked_option_id: "LUMINOUS_CHOIR.pages.INITIAL.options.OFFER_TRIBUTE_LOCKED"
                    .to_owned(),
            },
        ]),
    }
}

/// The abyssal baths: immerse for two maximum HP and three that nothing
/// blocks, and then linger as long as the water is worth it — every dip pays
/// two more maximum HP and takes one more than the last.
#[must_use]
pub fn abyssal_baths() -> EventDefinition {
    plain(
        "EVENT.ABYSSAL_BATHS",
        EventCondition::Always,
        vec![
            option(
                "ABYSSAL_BATHS.pages.INITIAL.options.IMMERSE",
                "Immerse",
                vec![EventEffect::ImmerseInTheBaths { damage: 3 }],
            ),
            option(
                "ABYSSAL_BATHS.pages.INITIAL.options.ABSTAIN",
                "Abstain",
                vec![EventEffect::Heal(EventAmount::Fixed(10))],
            ),
        ],
    )
}

/// The doll room: take one at random, pay five to be shown two, or pay fifteen
/// to be shown all three.
///
/// The random one is rolled over the dolls in the order the array declares
/// them; the two shown pages sort first and then shuffle, so the order the
/// options are laid out in does not depend on how the list happened to be
/// written.
#[must_use]
pub fn doll_room() -> EventDefinition {
    // A doll's option is keyed by the relic's own name rather than by anything
    // spelled out of the page, so the three names are carried here.
    let doll = |model_id: &str, name: &str| sts2_engine::RelicOnOffer {
        model_id: id(model_id),
        name: name.to_owned(),
    };
    let dolls = || {
        vec![
            doll("RELIC.DAUGHTER_OF_THE_WIND", "Daughter of the Wind"),
            doll("RELIC.MR_STRUGGLES", "Mr. Struggles"),
            doll("RELIC.BING_BONG", "Bing Bong"),
        ]
    };
    plain(
        "EVENT.DOLL_ROOM",
        EventCondition::ActIndexIs(1),
        vec![
            option(
                "DOLL_ROOM.pages.INITIAL.options.RANDOM",
                "Random",
                vec![EventEffect::ObtainRelicRolledFrom {
                    candidates: dolls().into_iter().map(|doll| doll.model_id).collect(),
                }],
            ),
            option(
                "DOLL_ROOM.pages.INITIAL.options.TAKE_SOME_TIME",
                "Take some time",
                vec![
                    EventEffect::LoseHp(EventAmount::Fixed(5)),
                    EventEffect::OfferShuffledRelics {
                        relics: dolls(),
                        take: 2,
                    },
                ],
            ),
            option(
                "DOLL_ROOM.pages.INITIAL.options.EXAMINE",
                "Examine",
                vec![
                    EventEffect::LoseHp(EventAmount::Fixed(15)),
                    EventEffect::OfferShuffledRelics {
                        relics: dolls(),
                        take: 3,
                    },
                ],
            ),
        ],
    )
}

/// The relic trader: three of the player's own relics against three off the
/// front of the bag, one pair per option.
#[must_use]
pub fn relic_trader() -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.RELIC_TRADER"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::All(vec![
            EventCondition::ActIndexAbove(0),
            EventCondition::TradableRelicsAtLeast(5),
        ]),
        vars: Vec::new(),
        generation: EventGeneration::RelicTrader,
    }
}

/// Welcome to Wongo's: a bargain bin, the item in the window, a mystery box,
/// or walk out and lose an upgrade on the way.
#[must_use]
pub fn welcome_to_wongos() -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.WELCOME_TO_WONGOS"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::All(vec![
            EventCondition::ActIndexIs(1),
            EventCondition::GoldAtLeast(100),
        ]),
        vars: Vec::new(),
        generation: EventGeneration::Wongos {
            bargain_bin: 100,
            featured: 200,
            mystery_box: 300,
            ticket: id("RELIC.WONGOS_MYSTERY_TICKET"),
        },
    }
}

/// A punch-off: nab what the two of them dropped for an Injury and a relic, or
/// tell them you can take them both and then do it.
///
/// The event is laid out as a fight, so the two of them are standing there from
/// the moment the room is entered and the fight the second option walks into
/// stands those same creatures up. A purse is rolled that the page shows and
/// neither option pays out.
#[must_use]
pub fn punch_off() -> EventDefinition {
    EventDefinition {
        model_id: id("EVENT.PUNCH_OFF"),
        combat_layout: Some(id("ENCOUNTER.PUNCH_OFF_EVENT_ENCOUNTER")),
        heals_on_entry: false,
        allowed: EventCondition::TotalFloorAtLeast(6),
        // A purse of 91 to 98, which only the page's text reads.
        vars: vec![EventAmount::Rolled { min: 91, max: 99 }],
        generation: EventGeneration::Fixed(vec![
            option(
                "PUNCH_OFF.pages.INITIAL.options.NAB",
                "Nab",
                vec![
                    EventEffect::AddCard(sts2_engine::CardFingerprint::base(id("CARD.INJURY"))),
                    EventEffect::Rewards(vec![sts2_engine::RewardFingerprint::of("relic")]),
                ],
            ),
            option(
                "PUNCH_OFF.pages.INITIAL.options.I_CAN_TAKE_THEM",
                "I can take them",
                vec![EventEffect::SetPage(vec![option(
                    "PUNCH_OFF.pages.I_CAN_TAKE_THEM.options.FIGHT",
                    "Fight",
                    vec![EventEffect::EnterCombatWithoutLeaving {
                        encounter: id("ENCOUNTER.PUNCH_OFF_EVENT_ENCOUNTER"),
                        extra_rewards: vec![
                            sts2_engine::RewardFingerprint::of("relic"),
                            sts2_engine::RewardFingerprint::of("potion"),
                        ],
                        resumes_parent: false,
                        chosen_monster: None,
                        resume: Vec::new(),
                    }],
                )])],
            ),
        ]),
    }
}

/// Tanx, the ancient who lays out nine weapons and one more for a deck that can
/// carry three instincts, shuffles the lot, and offers three.
#[must_use]
pub fn tanx() -> EventDefinition {
    let always = |relic: &str| sts2_engine::ConditionalRelic {
        relic: id(relic),
        available: EventCondition::Always,
    };
    let mut relics: Vec<sts2_engine::ConditionalRelic> = [
        "RELIC.CLAWS",
        "RELIC.CROSSBOW",
        "RELIC.IRON_CLUB",
        "RELIC.MEAT_CLEAVER",
        "RELIC.SAI",
        "RELIC.SPIKED_GAUNTLETS",
        "RELIC.TANXS_WHISTLE",
        "RELIC.THROWING_AXE",
        "RELIC.WAR_HAMMER",
    ]
    .into_iter()
    .map(always)
    .collect();
    relics.push(sts2_engine::ConditionalRelic {
        relic: id("RELIC.TRI_BOOMERANG"),
        available: EventCondition::DeckCardsAtLeast {
            filter: EventCardFilter::EnchantableBy(id("ENCHANTMENT.INSTINCT")),
            count: 3,
        },
    });
    EventDefinition {
        model_id: id("EVENT.TANX"),
        combat_layout: None,
        heals_on_entry: true,
        allowed: EventCondition::Always,
        vars: Vec::new(),
        generation: EventGeneration::ShuffledRelicPool { relics, offered: 3 },
    }
}

/// The battleworn dummy: pick a setting and fight the battle friend it stands
/// up, on a three-turn clock the friend carries itself.
///
/// The fight is pushed on top of the event's own room and the page stays
/// standing under it, so proceeding past the fight's rewards goes back to the
/// event rather than to the map. What the event then pays depends on the
/// setting, and pays nothing at all where the clock ran out — which is what the
/// friend walking out of the fight comes to.
#[must_use]
pub fn battleworn_dummy(potions: Vec<sts2_core::ModelId>) -> EventDefinition {
    let setting = |index: usize, resume: Vec<EventEffect>| {
        option(
            &format!(
                "BATTLEWORN_DUMMY.pages.INITIAL.options.SETTING_{}",
                index + 1
            ),
            &format!("Setting {}", index + 1),
            vec![EventEffect::EnterCombatWithoutLeaving {
                encounter: id("ENCOUNTER.BATTLEWORN_DUMMY_EVENT_ENCOUNTER"),
                extra_rewards: Vec::new(),
                resumes_parent: true,
                chosen_monster: Some(index),
                resume: vec![EventEffect::UnlessTheFightTimedOut(resume)],
            }],
        )
    };
    EventDefinition {
        model_id: id("EVENT.BATTLEWORN_DUMMY"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::Always,
        vars: Vec::new(),
        generation: EventGeneration::Fixed(vec![
            setting(
                0,
                vec![EventEffect::OfferRolledPotion {
                    candidates: potions,
                }],
            ),
            setting(1, vec![EventEffect::UpgradeShuffledDeckCards { count: 2 }]),
            setting(2, vec![EventEffect::ObtainRolledRelic]),
        ]),
    }
}

/// An endless conveyor: forty gold a plate, whatever is on the belt when you
/// reach for it, and another plate right behind it.
///
/// The dish is rolled as the room is entered and again after every grab, over
/// the dishes the run qualifies for less whatever was just eaten. Every fifth
/// roll is the seapunk salad and reads no die at all. The golden fysh joins the
/// list from the second roll on and is the one plate that costs nothing —
/// though the option is still locked where the purse cannot pay the forty.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one event with its explicit selection effect"
)]
pub fn endless_conveyor(potions: Vec<sts2_core::ModelId>) -> EventDefinition {
    let dish = |id: &str, weight: i32, effects: Vec<EventEffect>| sts2_engine::ConveyorDish {
        id: id.to_owned(),
        weight,
        available: EventCondition::Always,
        after_grabs: 0,
        free: false,
        effects,
    };
    EventDefinition {
        model_id: id("EVENT.ENDLESS_CONVEYOR"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::GoldAtLeast(120),
        vars: Vec::new(),
        generation: EventGeneration::EndlessConveyor {
            cost: 40,
            dishes: vec![
                dish("CAVIAR", 6, vec![EventEffect::GainMaxHp(4)]),
                dish("SPICY_SNAPPY", 3, vec![EventEffect::UpgradeARolledCard]),
                dish(
                    "JELLY_LIVER",
                    3,
                    vec![EventEffect::Run(vec![sts2_engine::Command::CardSelect(
                        sts2_engine::CardSelectCmd::FromDeckForTransformation {
                            semantics: ChoiceSemantics::known(vec![
                                ChoiceOp::new(ChoiceOperationKind::Transform, ChoiceScope::Run)
                                    .rolled(),
                            ]),
                            prefs: sts2_engine::SelectorPrefs::exactly(1),
                            then: vec![sts2_engine::Command::Card(
                                sts2_engine::CardCmd::TransformDeckCards {
                                    cards: sts2_engine::CardSelector::Chosen,
                                    replacement: sts2_engine::CardTransformReplacement::Rolled {
                                        stream: sts2_engine::TransformStream::Event,
                                        upgraded: false,
                                    },
                                    one_at_a_time: false,
                                },
                            )],
                        },
                    )])],
                ),
                dish(
                    "FRIED_EEL",
                    3,
                    vec![EventEffect::Run(vec![sts2_engine::Command::CardPile(
                        sts2_engine::CardPileCmd::AddCreatedCardsToDeck {
                            creation: sts2_engine::CardCreation {
                                pools: vec![sts2_engine::CardPoolRef::Named(id(
                                    "CARD_POOL.COLORLESS_CARD_POOL",
                                ))],
                                filter: sts2_engine::CardFilter::Any,
                                odds: sts2_engine::RarityOdds::RegularEncounter,
                                from_an_encounter: false,
                                count: 1,
                                // The plate is not a card reward and spares the
                                // upgrade draw, which is a read of the rewards
                                // stream whether or not anything comes of it.
                                flags: sts2_engine::CardCreationFlags {
                                    no_upgrade_roll: true,
                                    ..sts2_engine::CardCreationFlags::default()
                                },
                            },
                        },
                    )])],
                ),
                sts2_engine::ConveyorDish {
                    available: EventCondition::HasOpenPotionSlot,
                    ..dish(
                        "SUSPICIOUS_CONDIMENT",
                        3,
                        vec![EventEffect::OfferRolledPotion {
                            candidates: potions,
                        }],
                    )
                },
                sts2_engine::ConveyorDish {
                    available: EventCondition::Hurt,
                    ..dish(
                        "CLAM_ROLL",
                        6,
                        vec![EventEffect::Heal(EventAmount::Fixed(10))],
                    )
                },
                sts2_engine::ConveyorDish {
                    after_grabs: 1,
                    free: true,
                    ..dish(
                        "GOLDEN_FYSH",
                        1,
                        vec![EventEffect::GainGold(EventAmount::Fixed(75))],
                    )
                },
            ],
            forced_every: 5,
            forced: dish(
                "SEAPUNK_SALAD",
                0,
                vec![EventEffect::AddCard(sts2_engine::CardFingerprint::base(
                    id("CARD.FEEDING_FRENZY"),
                ))],
            ),
        },
    }
}

/// Tinker time: a Mad Science built to order. One click opens the type page,
/// which is two of the three types shuffled; picking one opens the rider page,
/// which is two of that type's three riders shuffled; and picking a rider makes
/// the card and puts it in the deck.
///
/// Both cuts take two at random — an unstable shuffle over the list and the
/// first two of what comes back — so each spends two reads of the event's own
/// stream.
#[must_use]
pub fn tinker_time() -> EventDefinition {
    // The build carries both as named choices; they are written down here as
    // the numbers it gives them, each counting from a None entry — the card
    // type from None, and the rider from None through the nine.
    let rider = |card_type: i32, name: &str, ordinal: i32| {
        option(
            &format!("TINKER_TIME.pages.CHOOSE_RIDER.options.{name}"),
            name,
            vec![EventEffect::AddCard(
                sts2_engine::CardFingerprint::base(id("CARD.MAD_SCIENCE"))
                    .with_property("TinkerTimeType", card_type)
                    .with_property("TinkerTimeRider", ordinal),
            )],
        )
    };
    let card_type = |name: &str, riders: Vec<EventOption>| {
        option(
            &format!("TINKER_TIME.pages.CHOOSE_CARD_TYPE.options.{name}"),
            name,
            vec![EventEffect::SetShuffledPage {
                options: riders,
                take: 2,
            }],
        )
    };
    EventDefinition {
        model_id: id("EVENT.TINKER_TIME"),
        combat_layout: None,
        heals_on_entry: false,
        allowed: EventCondition::Always,
        vars: Vec::new(),
        generation: EventGeneration::Fixed(vec![option(
            "TINKER_TIME.pages.INITIAL.options.CHOOSE_CARD_TYPE",
            "Choose a card type",
            vec![EventEffect::SetShuffledPage {
                options: vec![
                    card_type(
                        "ATTACK",
                        vec![
                            rider(1, "SAPPING", 1),
                            rider(1, "VIOLENCE", 2),
                            rider(1, "CHOKING", 3),
                        ],
                    ),
                    card_type(
                        "SKILL",
                        vec![
                            rider(2, "ENERGIZED", 4),
                            rider(2, "WISDOM", 5),
                            rider(2, "CHAOS", 6),
                        ],
                    ),
                    card_type(
                        "POWER",
                        vec![
                            rider(3, "EXPERTISE", 7),
                            rider(3, "CURIOUS", 8),
                            rider(3, "IMPROVEMENT", 9),
                        ],
                    ),
                ],
                take: 2,
            }],
        )]),
    }
}
