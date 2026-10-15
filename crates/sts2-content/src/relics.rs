//! Relic definitions for the relics the covered run can hold.
//!
//! A relic that acts through the run listens on the shared hook surface, and
//! its listener bodies are runs of commands like any other model's. A relic
//! whose whole effect happens the moment it is obtained has no hooks; its grant
//! is modeled where it is obtained instead.

use sts2_engine::{
    Amount, CardCmd, CardFilter, CardOrigin, CardPileCmd, CardSelectCmd, CardSelector, CardTag,
    Command, Condition, CreatureCmd, CreatureSelector, HookCondition, HookEvent, HookTiming,
    ModifierCmd, ModifierHook, PlayerCmd, PowerCmd, RelicCmd, RelicDefinition, RelicRarity,
    RelicSpawnCondition, RestSiteOffer, RestSiteOption, RewardsCmd, SelectorPrefs, ValueProps,
};
use sts2_engine::{CardCreation, CardPoolRef, CardRarity};
use sts2_engine::{
    ChoiceDestination, ChoiceDuration, ChoiceOperation as ChoiceOp, ChoiceOperationKind,
    ChoiceScope, ChoiceSemantics, ChoiceSubject, ChoiceTiming,
};

use crate::id;

fn obtained_only(model_id: &str, rarity: RelicRarity) -> RelicDefinition {
    RelicDefinition {
        model_id: id(model_id),
        grants_an_extra_turn: None,
        map_rule: None,
        rarity,
        counter_property: None,
        counter_kind: None,
        counter_saved_only_when_set: false,
        keeps_hand: false,
        keeps_first_hand: false,
        on_obtained: Vec::new(),
        setup: None,
        removes_unknown_room_types: Vec::new(),
        grants_free_travel: false,
        hooks: Vec::new(),
        card_reward_alternative: None,
        has_upon_pickup_effect: false,
        spawns_pets: false,
        adds_pet: false,
        rolled_skin: None,
        is_stackable: false,
        allowed_in_shops: true,
        merchant_cost_override: None,
        modifies_x_value_by: 0,
        restocks_merchant_entries: false,
        adds_rest_site_option: None,
        keeps_rest_site_options_standing: false,
        keeps_energy_after_first_turn: false,
        keeps_block_up_to: None,
        forces_potion_reward_in_combat_rooms: false,
        refuses_potions: false,
        stops_draws_on_its_own_side_turn: false,
        card_cost_rule: None,
        cards_playable_each_turn: None,
        upgrades_cards_of_type: None,
        enchants_cards_it_meets: None,
        suppresses_treasure_while_second_counter_at_most: None,
        second_counter_property: None,
        latch_property: None,
        saved_card_array_property: None,
        prevents_owners_death_late: Vec::new(),
        adds_card_pool_to_rewards: None,
        adds_every_character_pool_to_rewards: false,
        initial_counter: 0,
        used_up: None,
        spawn_condition: None,
    }
}

/// A relic a merchant never stocks, which five of the shared pool are.
fn never_in_shops(mut definition: RelicDefinition) -> RelicDefinition {
    definition.allowed_in_shops = false;
    definition
}

/// Cards picked out of the deck for an enchantment, and the enchant that then
/// runs on whatever was picked — what every relic that puts an enchantment on
/// cards the player chooses comes to.
fn enchant_chosen_deck_cards(enchantment: &str, amount: i32, prefs: SelectorPrefs) -> Command {
    Command::CardSelect(CardSelectCmd::FromDeckForEnchantment {
        semantics: ChoiceSemantics::known(vec![
            ChoiceOp::new(ChoiceOperationKind::Enchant, ChoiceScope::Run)
                .model(id(enchantment))
                .amount(amount),
        ]),
        enchantment: id(enchantment),
        amount,
        filter: CardFilter::Any,
        shuffled_on: None,
        prefs,
        then: vec![Command::Card(CardCmd::Enchant {
            cards: CardSelector::Chosen,
            enchantment: id(enchantment),
            amount: Amount::Fixed(amount),
        })],
    })
}

/// Every card of the deck the enchantment can go on, enchanted without asking,
/// which is what a relic that enchants a whole set of the deck is.
fn enchant_every_deck_card(enchantment: &str, amount: i32, filter: CardFilter) -> Command {
    Command::Card(CardCmd::Enchant {
        cards: CardSelector::DeckCardsEnchantableBy {
            enchantment: id(enchantment),
            filter,
        },
        enchantment: id(enchantment),
        amount: Amount::Fixed(amount),
    })
}

/// A relic that does something the moment it is taken, which is one of the
/// things that keeps it from being traded away.
fn upon_pickup(mut definition: RelicDefinition) -> RelicDefinition {
    definition.has_upon_pickup_effect = true;
    definition
}

/// The five relics of the shared pool a merchant never stocks, whatever else
/// it may roll.
pub(crate) const NEVER_IN_SHOPS: [&str; 5] = [
    "RELIC.AMETHYST_AUBERGINE",
    "RELIC.BOWLER_HAT",
    "RELIC.LUCKY_FYSH",
    "RELIC.OLD_COIN",
    "RELIC.THE_COURIER",
];

/// The Necrobinder's own relic pool, whole, and her starter with them.
#[must_use]
#[allow(clippy::too_many_lines, reason = "one entry per relic of the pool")]
pub fn necrobinder_relics() -> Vec<RelicDefinition> {
    vec![
        // A bound phylactery summons one before the fight is announced, and one
        // more at every energy reset but the first turn's — the turn the
        // combat-start summon already answered for. It takes the late half of
        // that pass, which is the build's own note: a Friendship asking whether
        // the pet is standing is answered before this turn's pet arrives.
        RelicDefinition {
            spawns_pets: true,
            adds_pet: false,
            rolled_skin: None,
            hooks: vec![
                ModifierHook::every_turn(HookEvent::BeforeCombatStart, vec![summon(1)]),
                ModifierHook::every_turn(
                    HookEvent::AfterEnergyResetLate,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::Not(Box::new(Condition::PlayerTurnIs(1))),
                        then: vec![summon(1)],
                    }],
                ),
            ],
            ..obtained_only("RELIC.BOUND_PHYLACTERY", RelicRarity::Starter)
        },
        // A phylactery unbound is the other starter: five as the fight opens
        // and two more at the start of every turn its owner takes part in,
        // including the first — it answers `AfterSideTurnStart` rather than the
        // late half of the energy reset, so it has no turn to skip.
        //
        // No relic pool holds it and no run of this baseline starts with it;
        // the Necrobinder's own loadout is the bound one.
        RelicDefinition {
            spawns_pets: true,
            hooks: vec![
                ModifierHook::every_turn(HookEvent::BeforeCombatStart, vec![summon(5)]),
                ModifierHook::every_turn(HookEvent::AfterSideTurnStart, vec![summon(2)])
                    .when(HookCondition::OwnerIsParticipant),
            ],
            ..obtained_only("RELIC.PHYLACTERY_UNBOUND", RelicRarity::Starter)
        },
        // A bone flute answers every swing the owner's pet finishes with block
        // that nothing scales.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(HookEvent::AfterAttack, vec![own_block(2)])
                    .when(HookCondition::OwnersPetAttacked),
            ],
            ..obtained_only("RELIC.BONE_FLUTE", RelicRarity::Common)
        },
        // A book repair knife heals three for every creature but its owner a
        // doom kill took down, and only for the deaths that counted.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterDiedToDoom,
                vec![heal(Amount::per(
                    0,
                    3,
                    sts2_engine::Count::CreaturesThatDiedToDoom,
                ))],
            )],
            ..obtained_only("RELIC.BOOK_REPAIR_KNIFE", RelicRarity::Uncommon)
        },
        // A funerary mask shuffles three souls through the draw pile before the
        // opening hand is dealt, one at a time.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::BeforeHandDraw,
                    vec![Command::CardPile(CardPileCmd::AddGeneratedToCombat {
                        enchantment: None,
                        card: CardOrigin::Model(id("CARD.SOUL")),
                        grown_by: Amount::Fixed(0),
                        count: Amount::Fixed(3),
                        pile: sts2_engine::PileName::Draw,
                        upgraded: false,
                        ethereal: false,
                        retain: false,
                        temporary_cost: None,
                        position: sts2_engine::DrawPilePosition::Random,
                    })],
                )
                .timed(HookTiming::FirstTurnOnly),
            ],
            ..obtained_only("RELIC.FUNERARY_MASK", RelicRarity::Uncommon)
        },
        // A big hat deals two different ethereal cards of its owner's own pool
        // into the opening hand.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    vec![Command::CardPile(CardPileCmd::AddGeneratedFromPool {
                        pool: sts2_engine::CardPoolRef::OwnCharacter,
                        count: Amount::Fixed(2),
                        distinct: true,
                        filter: CardFilter::Ethereal,
                        pile: sts2_engine::PileName::Hand,
                        position: sts2_engine::DrawPilePosition::Bottom,
                        excludes_source: false,
                        upgraded: false,
                        temporary_cost: None,
                        ethereal: false,
                        rolled_one_at_a_time: false,
                    })],
                )
                .timed(HookTiming::FirstTurnOnly)
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..obtained_only("RELIC.BIG_HAT", RelicRarity::Rare)
        },
        // A bookmark takes a point off one of the cards that stayed in hand
        // through the flush, and only off one that still costs something.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterFlush,
                vec![Command::Card(CardCmd::AddCostUntilPlayed {
                    cards: CardSelector::RandomFromPile {
                        pile: sts2_engine::PileName::Hand,
                        filter: CardFilter::CostsEnergyNow,
                        fallback: None,
                    },
                    amount: Amount::Fixed(-1),
                })],
            )],
            ..obtained_only("RELIC.BOOKMARK", RelicRarity::Rare)
        },
        // An ivory tile hands a point back for every card its owner spends
        // three or more on.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardPlayed,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::PlayedCardEnergyAtLeast(3),
                        then: vec![gain_energy(1)],
                    }],
                )
                .when(HookCondition::OwnerPlayedTheCard),
            ],
            ..obtained_only("RELIC.IVORY_TILE", RelicRarity::Rare)
        },
        // An undying sigil halves every powered blow a doomed creature lands on
        // its owner. It does nothing else: the relic is described as making a
        // Doom fire at the start of the enemy turn while it stands, and the
        // Doom of the pinned build does no such thing.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::ModifyDamageMultiplicative,
                    vec![Command::Modify(ModifierCmd::Scale(Amount::Fixed(50)))],
                )
                .when(HookCondition::All(vec![
                    HookCondition::OwnerTakesPoweredDamage,
                    HookCondition::Not(Box::new(HookCondition::OwnerDealsPoweredDamage)),
                    HookCondition::DamageDealerIsDoomed(id("POWER.DOOM_POWER")),
                ])),
            ],
            ..obtained_only("RELIC.UNDYING_SIGIL", RelicRarity::Shop)
        },
    ]
}

/// A summon worth that much, for the relic's own owner.
fn summon(amount: i32) -> Command {
    Command::Osty(sts2_engine::OstyCmd::Summon {
        amount: Amount::Fixed(amount),
        owner: CreatureSelector::Owner,
    })
}

/// The Defect's own relic pool, whole, and the starter a refinement trades
/// his for.
#[must_use]
#[allow(clippy::too_many_lines, reason = "one entry per relic of the pool")]
pub fn defect_relics() -> Vec<RelicDefinition> {
    vec![
        // A cracked core channels one Lightning orb before the first turn of a
        // fight opens. `BeforeSideTurnStart` is ahead of the energy reset and
        // ahead of the draw, so what an orb evoked to make room pays out lands
        // before either.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::BeforeSideTurnStart,
                    vec![Command::Orb(sts2_engine::OrbCmd::Channel(
                        sts2_engine::OrbSource::Model(id("ORB.LIGHTNING_ORB")),
                    ))],
                )
                .when(HookCondition::OwnerIsParticipant)
                .timed(HookTiming::FirstTurnOnly),
            ],
            ..obtained_only("RELIC.CRACKED_CORE", RelicRarity::Starter)
        },
        // An infused core is the other starter, and no pool holds it: it is
        // what a touch of orobas refines a cracked core into. Three Lightning
        // orbs rather than one, channelled after the side's own listeners
        // rather than before them, and every Lightning orb of its owner's is
        // worth one more.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    vec![Command::Repeat {
                        times: Amount::Fixed(3),
                        then: vec![Command::Orb(sts2_engine::OrbCmd::Channel(
                            sts2_engine::OrbSource::Model(id("ORB.LIGHTNING_ORB")),
                        ))],
                    }],
                )
                .when(HookCondition::OwnerIsParticipant)
                .timed(HookTiming::FirstTurnOnly),
                ModifierHook::every_turn(
                    HookEvent::ModifyOrbValue,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::AnnouncedOrbIs(id("ORB.LIGHTNING_ORB")),
                        then: vec![Command::Modify(sts2_engine::ModifierCmd::Add(
                            Amount::Fixed(1),
                        ))],
                    }],
                )
                .when(HookCondition::OrbBelongsToOwner),
            ],
            ..obtained_only("RELIC.INFUSED_CORE", RelicRarity::Starter)
        },
        // A data disk works a Focus on as its owner walks into a fight. The
        // combat state is built before the room announces itself, so the power
        // lands on the creature the fight just stood up.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterRoomEntered,
                    vec![own_power("POWER.FOCUS_POWER", Amount::Fixed(1))],
                )
                .when(HookCondition::InACombatRoom),
            ],
            ..obtained_only("RELIC.DATA_DISK", RelicRarity::Common)
        },
        // Gold plated cables fire the orb at the front of the queue a second
        // time at every turn boundary. What it answers with is one more
        // trigger; the pass over whoever changed the count reaches it and
        // flashes the relic, which is presentation.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::ModifyOrbPassiveTriggerCount,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::AnnouncedOrbIsAtTheFront,
                        then: vec![Command::Modify(sts2_engine::ModifierCmd::Add(
                            Amount::Fixed(1),
                        ))],
                    }],
                )
                .when(HookCondition::OrbBelongsToOwner),
            ],
            ..obtained_only("RELIC.GOLD_PLATED_CABLES", RelicRarity::Uncommon)
        },
        // A symbiotic virus channels a Dark orb once the first turn of a fight
        // has opened.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    vec![Command::Orb(sts2_engine::OrbCmd::Channel(
                        sts2_engine::OrbSource::Model(id("ORB.DARK_ORB")),
                    ))],
                )
                .when(HookCondition::OwnerIsParticipant)
                .timed(HookTiming::FirstTurnOnly),
            ],
            ..obtained_only("RELIC.SYMBIOTIC_VIRUS", RelicRarity::Uncommon)
        },
        // An emotion chip fires every orb its owner is holding as their turn
        // opens, where a blow block did not stop whole landed on them since
        // their last turn began. What it does on taking one is set its own
        // display state, which is presentation.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterPlayerTurnStart,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::OwnerTookAnUnstoppedBlowLastTurn,
                    then: vec![Command::ForEachOrb {
                        orbs: sts2_engine::OrbSelector::All,
                        then: vec![Command::Orb(sts2_engine::OrbCmd::Passive {
                            orb: sts2_engine::OrbSelector::Bound,
                            target: None,
                        })],
                    }],
                }],
            )],
            ..obtained_only("RELIC.EMOTION_CHIP", RelicRarity::Rare)
        },
        // A power cell deals two of the draw pile's free cards into the hand
        // before the first turn of a fight opens. The pile is put in canonical
        // order and stably shuffled on the combat card-selection stream, and
        // the first two of what is left are taken.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::BeforeSideTurnStart,
                    vec![Command::CardPile(CardPileCmd::MoveToPile {
                        cards: CardSelector::StablyShuffledFromPile {
                            pile: sts2_engine::PileName::Draw,
                            filter: sts2_engine::CardFilter::CostsNothingNow,
                            fallback: None,
                            take: Amount::Fixed(2),
                            stream: sts2_engine::ShuffleStream::CombatCardSelection,
                        },
                        pile: sts2_engine::PileName::Hand,
                        position: sts2_engine::DrawPilePosition::Bottom,
                    })],
                )
                .when(HookCondition::OwnerIsParticipant)
                .timed(HookTiming::FirstTurnOnly),
            ],
            ..obtained_only("RELIC.POWER_CELL", RelicRarity::Rare)
        },
        // A metronome counts the orbs its owner channels in a fight and hits
        // everything for thirty on the seventh. The count is never saved, so
        // nothing of it survives a reload, and it is put back to nothing as a
        // combat room is entered and again as the fight ends.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterOrbChanneled,
                    vec![
                        Command::Relic(RelicCmd::ModifyListeningCounter(Amount::Fixed(1))),
                        Command::When {
                            otherwise: Vec::new(),
                            condition: Condition::ListeningRelicCounterIs(7),
                            then: vec![Command::Creature(CreatureCmd::Damage {
                                target: CreatureSelector::AllOpponents,
                                amount: Amount::Fixed(30),
                                props: ValueProps::UNPOWERED,
                            })],
                        },
                    ],
                ),
                ModifierHook::every_turn(
                    HookEvent::AfterRoomEntered,
                    vec![Command::Relic(RelicCmd::SetListeningCounter(
                        Amount::Fixed(0),
                    ))],
                )
                .when(HookCondition::InACombatRoom),
                ModifierHook::every_turn(
                    HookEvent::AfterCombatEnd,
                    vec![Command::Relic(RelicCmd::SetListeningCounter(
                        Amount::Fixed(0),
                    ))],
                ),
            ],
            ..obtained_only("RELIC.METRONOME", RelicRarity::Rare)
        },
        // A runic capacitor gives three orb slots once the first turn of a
        // fight has opened.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    vec![Command::Orb(sts2_engine::OrbCmd::AddSlots(Amount::Fixed(
                        3,
                    )))],
                )
                .when(HookCondition::OwnerIsParticipant)
                .timed(HookTiming::FirstTurnOnly),
            ],
            ..obtained_only("RELIC.RUNIC_CAPACITOR", RelicRarity::Shop)
        },
    ]
}

/// The Regent's own relic pool, whole, and the starter a refinement trades
/// his for.
#[must_use]
#[allow(clippy::too_many_lines, reason = "one entry per relic of the pool")]
pub fn regent_relics() -> Vec<RelicDefinition> {
    vec![
        // A divine right pays three stars as its owner walks into a fight. The
        // combat state is built before the room announces itself, so the pool
        // it lands in is the fresh one.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterRoomEntered,
                    vec![Command::Player(sts2_engine::PlayerCmd::GainStars(
                        Amount::Fixed(3),
                    ))],
                )
                .when(HookCondition::InACombatRoom),
            ],
            ..obtained_only("RELIC.DIVINE_RIGHT", RelicRarity::Starter)
        },
        // A black blood is the Ironclad's refined starter: twelve healed at the
        // end of a won fight rather than six. No pool holds it — it is what a
        // touch of orobas refines a burning blood into.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterCombatVictory,
                vec![Command::Creature(CreatureCmd::Heal {
                    target: CreatureSelector::Owner,
                    amount: Amount::Fixed(12),
                })],
            )],
            ..obtained_only("RELIC.BLACK_BLOOD", RelicRarity::Starter)
        },
        // A ring of the drake is the Silent's: two more cards on every hand of
        // the first three turns rather than on the opening one alone.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::ModifyHandDraw,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::PlayerTurnAtMost(3),
                    then: vec![Command::Modify(ModifierCmd::Add(Amount::Fixed(2)))],
                }],
            )],
            ..obtained_only("RELIC.RING_OF_THE_DRAKE", RelicRarity::Starter)
        },
        // A circlet is worth nothing at all: it is what a refinement hands over
        // where the starter it was pointed at has no refined form. It and the
        // deprecated relic are the build's only two stackable relics, and its
        // only two of no rarity at all.
        RelicDefinition {
            is_stackable: true,
            ..obtained_only("RELIC.CIRCLET", RelicRarity::None)
        },
        // The stand-in for a relic the game no longer has, which the run
        // history reads back. No pool holds it and nothing hands one over.
        RelicDefinition {
            is_stackable: true,
            ..obtained_only("RELIC.DEPRECATED_RELIC", RelicRarity::None)
        },
        // A divine destiny is the other starter: six stars on the first turn of
        // every fight rather than three as the room opens. No pool holds it —
        // it is what a touch of orobas refines a divine right into.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    vec![Command::Player(sts2_engine::PlayerCmd::GainStars(
                        Amount::Fixed(6),
                    ))],
                )
                .when(HookCondition::OwnerIsParticipant)
                .timed(HookTiming::FirstTurnOnly),
            ],
            ..obtained_only("RELIC.DIVINE_DESTINY", RelicRarity::Starter)
        },
        // A fencing manual forges ten on the first turn of every fight.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    vec![Command::Forge(sts2_engine::ForgeCmd::Forge(Amount::Fixed(
                        10,
                    )))],
                )
                .when(HookCondition::OwnerIsParticipant)
                .timed(HookTiming::FirstTurnOnly),
            ],
            ..obtained_only("RELIC.FENCING_MANUAL", RelicRarity::Common)
        },
        // Galactic dust counts the stars its owner spends and pays ten block
        // for every ten of them, keeping the remainder for next time. The count
        // is a saved property of its own, so it outlives the fight.
        RelicDefinition {
            counter_property: Some("StarsSpent".to_owned()),
            counter_kind: Some(sts2_engine::CounterKind::Cycle { wraps_at: 10 }),
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterStarsSpent,
                vec![
                    Command::Relic(RelicCmd::ModifyListeningCounter(Amount::counting(
                        sts2_engine::Count::AnnouncedAmount,
                    ))),
                    Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningRelicCounterAtLeast(10),
                        then: vec![
                            Command::Creature(CreatureCmd::GainBlock {
                                target: CreatureSelector::Owner,
                                amount: Amount::Product {
                                    left: Box::new(Amount::Quotient {
                                        of: Box::new(Amount::counting(
                                            sts2_engine::Count::ListeningRelicCounter,
                                        )),
                                        by: 10,
                                    }),
                                    right: Box::new(Amount::Fixed(10)),
                                },
                                props: ValueProps::UNPOWERED,
                            }),
                            Command::Relic(RelicCmd::SetListeningCounter(Amount::Remainder {
                                of: Box::new(Amount::counting(
                                    sts2_engine::Count::ListeningRelicCounter,
                                )),
                                by: 10,
                            })),
                        ],
                    },
                ],
            )],
            ..obtained_only("RELIC.GALACTIC_DUST", RelicRarity::Uncommon)
        },
        // A lunar pastry pays a star at the end of every turn its owner took
        // part in.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnEnd,
                    vec![Command::Player(sts2_engine::PlayerCmd::GainStars(
                        Amount::Fixed(1),
                    ))],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..obtained_only("RELIC.LUNAR_PASTRY", RelicRarity::Rare)
        },
        // A mini regent pays a strength the first time its owner spends stars
        // in a turn. The latch is never saved, so nothing of it survives a
        // reload.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterStarsSpent,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::Not(Box::new(Condition::ListeningRelicIsLatched)),
                        then: vec![
                            latch(true),
                            own_power("POWER.STRENGTH_POWER", Amount::Fixed(1)),
                        ],
                    }],
                ),
                ModifierHook::every_turn(HookEvent::BeforeSideTurnStart, vec![latch(false)])
                    .when(HookCondition::OwnerIsParticipant),
                ModifierHook::every_turn(HookEvent::AfterCombatEnd, vec![latch(false)]),
            ],
            ..obtained_only("RELIC.MINI_REGENT", RelicRarity::Rare)
        },
        // Orange dough deals two colorless cards into the hand on the first
        // turn of every fight.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    vec![Command::CardPile(CardPileCmd::AddGeneratedFromPool {
                        pool: sts2_engine::CardPoolRef::Named(id("CARD_POOL.COLORLESS_CARD_POOL")),
                        count: Amount::Fixed(2),
                        distinct: true,
                        filter: sts2_engine::CardFilter::Any,
                        pile: sts2_engine::PileName::Hand,
                        position: sts2_engine::DrawPilePosition::Bottom,
                        excludes_source: false,
                        upgraded: false,
                        temporary_cost: None,
                        ethereal: false,
                        rolled_one_at_a_time: false,
                    })],
                )
                .when(HookCondition::OwnerIsParticipant)
                .timed(HookTiming::FirstTurnOnly),
            ],
            ..obtained_only("RELIC.ORANGE_DOUGH", RelicRarity::Rare)
        },
        // A regalite pays two block for every card its owner has conjured into
        // the fight, and nothing scales it.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardGeneratedForCombat,
                    vec![own_block(2)],
                )
                .when(HookCondition::OwnerConjuredTheCard),
            ],
            ..obtained_only("RELIC.REGALITE", RelicRarity::Uncommon)
        },
        // A vitruvian minion doubles what a minion card deals and what it works
        // up, and makes no test on how the value was reached: a minion's dive
        // bomb hits twice as hard and its sacrifice is worth twice the block.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::ModifyDamageMultiplicative,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::DamageCardIsTagged(sts2_engine::CardTag::Minion),
                        then: vec![Command::Modify(ModifierCmd::Scale(Amount::Fixed(200)))],
                    }],
                ),
                ModifierHook::every_turn(
                    HookEvent::ModifyBlockMultiplicative,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::DamageCardIsTagged(sts2_engine::CardTag::Minion),
                        then: vec![Command::Modify(ModifierCmd::Scale(Amount::Fixed(200)))],
                    }],
                ),
            ],
            ..obtained_only("RELIC.VITRUVIAN_MINION", RelicRarity::Shop)
        },
    ]
}

/// Every relic this crate registers, whichever pool it stands in.
///
/// The relic pools bucket their members by rarity, and this is where a relic's
/// rarity is declared, so the bags read it back off the definitions rather than
/// out of a rarity table of their own.
#[must_use]
pub(crate) fn every_relic() -> Vec<RelicDefinition> {
    silent_relics()
        .into_iter()
        .chain(necrobinder_relics())
        .chain(regent_relics())
        .chain(defect_relics())
        .chain(ironclad_relics())
        .collect()
}

#[must_use]
pub fn ironclad_relics() -> Vec<RelicDefinition> {
    combat_opening_relics()
        .into_iter()
        .chain(other_relics())
        .chain(walked_relics())
        .chain(hive_relics())
        .chain(std::iter::once(music_box()))
        .chain(ancient_relics())
        .chain(neow_relics())
        .chain(std::iter::once(booming_conch()))
        .chain(std::iter::once(pear()))
        .chain(std::iter::once(stone_calendar()))
        .chain(std::iter::once(strike_dummy()))
        .chain(std::iter::once(dusty_tome()))
        .chain(std::iter::once(art_of_war()))
        .chain(std::iter::once(ringing_triangle()))
        .chain(std::iter::once(bronze_scales()))
        .chain(std::iter::once(vajra()))
        .chain(std::iter::once(pendulum()))
        .chain(std::iter::once(gorget()))
        .chain(std::iter::once(looming_fruit()))
        .chain(std::iter::once(paels_wing()))
        .chain(common_relics())
        .chain(uncommon_relics())
        .chain(rare_relics())
        .chain(shop_relics())
        .chain(event_relics())
        .collect()
}

/// The Silent's own relic pool, whole. The pool is eight and the starter a
/// Silent run opens holding is one of them.
#[must_use]
pub fn silent_relics() -> Vec<RelicDefinition> {
    vec![
        // A ring of the snake deals two more cards on the opening hand, on the
        // same turn-number test a bag of preparation makes.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::ModifyHandDraw,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::PlayerTurnAtMost(1),
                    then: vec![Command::Modify(ModifierCmd::Add(Amount::Fixed(2)))],
                }],
            )],
            ..obtained_only("RELIC.RING_OF_THE_SNAKE", RelicRarity::Starter)
        },
        // A helical dart lends its owner a point of dexterity for the turn
        // every time they throw a Shiv.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardPlayed,
                    vec![Command::Power(sts2_engine::PowerCmd::Apply {
                        marks: None,
                        power: id("POWER.HELICAL_DART_POWER"),
                        target: CreatureSelector::Owner,
                        applier: Some(CreatureSelector::Owner),
                        amount: Amount::Fixed(1),
                    })],
                )
                .when(HookCondition::All(vec![
                    HookCondition::OwnerPlayedTheCard,
                    HookCondition::PlayedCardIsTagged(sts2_engine::CardTag::Shiv),
                ])),
            ],
            ..obtained_only("RELIC.HELICAL_DART", RelicRarity::Rare)
        },
        // A ninja scroll deals three Shivs before the opening hand.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::BeforeHandDraw,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::PlayerTurnAtMost(1),
                    then: vec![crate::powers::shivs_into_hand(Amount::Fixed(3), false)],
                }],
            )],
            ..obtained_only("RELIC.NINJA_SCROLL", RelicRarity::Shop)
        },
        // A paper krane does not listen for anything: Weak looks the relic up
        // on whoever is taking the blow and takes its own multiplier down by
        // fifteen hundredths for it. The whole of that lives in the power.
        obtained_only("RELIC.PAPER_KRANE", RelicRarity::Rare),
        // A snecko skull adds one to every point of poison its owner works.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::ModifyPowerAmountGiven,
                    vec![Command::Modify(ModifierCmd::Add(Amount::Fixed(1)))],
                )
                .when(HookCondition::OwnerIsGivingPower(id("POWER.POISON_POWER"))),
            ],
            ..obtained_only("RELIC.SNECKO_SKULL", RelicRarity::Common)
        },
        // A tingsha throws three at something for every card its owner puts
        // down on their own turn.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardDiscarded,
                    vec![Command::Creature(CreatureCmd::Damage {
                        target: CreatureSelector::RandomOpponent,
                        amount: Amount::Fixed(3),
                        props: sts2_engine::ValueProps::UNPOWERED,
                    })],
                )
                .when(HookCondition::OwnersCardDiscardedOnOwnSideTurn),
            ],
            ..obtained_only("RELIC.TINGSHA", RelicRarity::Uncommon)
        },
        // Tough bandages pay three block for the same thing.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardDiscarded,
                    vec![Command::Creature(CreatureCmd::GainBlock {
                        target: CreatureSelector::Owner,
                        amount: Amount::Fixed(3),
                        props: sts2_engine::ValueProps::UNPOWERED,
                    })],
                )
                .when(HookCondition::OwnersCardDiscardedOnOwnSideTurn),
            ],
            ..obtained_only("RELIC.TOUGH_BANDAGES", RelicRarity::Rare)
        },
        // A twisted funnel poisons everything as the first turn opens.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::BeforeSideTurnStart,
                    vec![Command::Power(sts2_engine::PowerCmd::Apply {
                        marks: None,
                        power: id("POWER.POISON_POWER"),
                        target: CreatureSelector::AllOpponents,
                        applier: Some(CreatureSelector::Owner),
                        amount: Amount::Fixed(4),
                    })],
                )
                .timed(sts2_engine::HookTiming::FirstTurnOnly)
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..obtained_only("RELIC.TWISTED_FUNNEL", RelicRarity::Uncommon)
        },
    ]
}

/// The relic stops being offered from the act-three chest on, which is why the
/// bags drop it there.
fn stops_late(mut definition: RelicDefinition) -> RelicDefinition {
    definition.spawn_condition = Some(RelicSpawnCondition::BeforeAct3TreasureChest);
    definition
}

/// A relic that acts the moment it is taken.
fn taken_effect(model_id: &str, rarity: RelicRarity, commands: Vec<Command>) -> RelicDefinition {
    upon_pickup(RelicDefinition {
        on_obtained: commands,
        ..obtained_only(model_id, rarity)
    })
}

/// One energy for whoever the listener belongs to.
fn gain_energy(amount: i32) -> Command {
    Command::Player(PlayerCmd::GainEnergy(Amount::Fixed(amount)))
}

/// Block for the listener's owner, which nothing scales.
fn own_block(amount: i32) -> Command {
    Command::Creature(CreatureCmd::GainBlock {
        target: CreatureSelector::Owner,
        amount: Amount::Fixed(amount),
        props: ValueProps::UNPOWERED,
    })
}

/// A power worked on the listener's owner by the listener's owner.
fn own_power(power: &str, amount: Amount) -> Command {
    Command::Power(PowerCmd::Apply {
        marks: None,
        power: id(power),
        target: CreatureSelector::Owner,
        applier: Some(CreatureSelector::Owner),
        amount,
    })
}

/// The same power, credited to no one: some relics hand their owner a power
/// without naming an applier, and the fight remembers it that way.
fn own_power_unattributed(power: &str, amount: Amount) -> Command {
    Command::Power(PowerCmd::Apply {
        marks: None,
        power: id(power),
        target: CreatureSelector::Owner,
        applier: None,
        amount,
    })
}

fn heal(amount: Amount) -> Command {
    Command::Creature(CreatureCmd::Heal {
        target: CreatureSelector::Owner,
        amount,
    })
}

fn draw(count: i32) -> Command {
    Command::CardPile(CardPileCmd::Draw {
        count: Amount::Fixed(count),
    })
}

fn latch(value: bool) -> Command {
    Command::Relic(RelicCmd::SetListeningLatch(value))
}

/// The card a relic binds for the fight, written on or cleared.
fn bind_card(bind: bool) -> Command {
    Command::Relic(RelicCmd::BindAnnouncedCardToTheRelic { bind })
}

/// The counter a relic keeps, wound on by one and wrapped at a threshold, which
/// is the `(n + 1) % k` several of them write.
fn counts_to(threshold: i32) -> Vec<Command> {
    vec![
        Command::Relic(RelicCmd::ModifyListeningCounter(Amount::Fixed(1))),
        Command::When {
            otherwise: Vec::new(),
            condition: Condition::ListeningRelicCounterIs(threshold),
            then: vec![Command::Relic(RelicCmd::SetListeningCounter(
                Amount::Fixed(0),
            ))],
        },
    ]
}

/// The two rooms a relic answers by looking at what room it is standing in as
/// the room is entered.
fn in_room(room: sts2_engine::ResolvedRoom, then: Vec<Command>) -> Command {
    Command::When {
        otherwise: Vec::new(),
        condition: Condition::CurrentRoomIs(room),
        then,
    }
}

/// The common relics of the shared pool, and the Ironclad's own common.
#[must_use]
#[allow(clippy::too_many_lines, reason = "one entry per relic of the rarity")]
fn common_relics() -> Vec<RelicDefinition> {
    vec![
        // A bag of preparation deals two more cards on the opening hand. The
        // build's own test is on the turn number rather than on a timing, and
        // the draw it swells is the one the opening turn makes.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::ModifyHandDraw,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::PlayerTurnAtMost(1),
                    then: vec![Command::Modify(ModifierCmd::Add(Amount::Fixed(2)))],
                }],
            )],
            ..obtained_only("RELIC.BAG_OF_PREPARATION", RelicRarity::Common)
        },
        // A centennial puzzle deals three the first time a fight costs its
        // owner HP, and holds its latch until the fight is over.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterDamageReceived,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::Not(Box::new(Condition::ListeningRelicIsLatched)),
                        then: vec![
                            latch(true),
                            // `for (i = 0; i < Cards; i++) Draw(1)`: three
                            // draws, not one draw of three.
                            Command::Repeat {
                                times: Amount::Fixed(3),
                                then: vec![draw(1)],
                            },
                        ],
                    }],
                )
                .when(HookCondition::OwnerTookUnblockedDamage),
                ModifierHook::every_turn(HookEvent::AfterCombatEnd, vec![latch(false)]),
            ],
            ..obtained_only("RELIC.CENTENNIAL_PUZZLE", RelicRarity::Common)
        },
        // A happy flower counts its owner's turns and pays an energy on every
        // third.
        RelicDefinition {
            counter_property: Some("TurnsSeen".to_owned()),
            counter_kind: Some(sts2_engine::CounterKind::Cycle { wraps_at: 3 }),
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    counts_to(3)
                        .into_iter()
                        .chain(std::iter::once(Command::When {
                            otherwise: Vec::new(),
                            condition: Condition::ListeningRelicCounterIs(0),
                            then: vec![gain_energy(1)],
                        }))
                        .collect(),
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..obtained_only("RELIC.HAPPY_FLOWER", RelicRarity::Common)
        },
        // A potion belt is two more slots the moment it is taken.
        taken_effect(
            "RELIC.POTION_BELT",
            RelicRarity::Common,
            vec![Command::Player(PlayerCmd::GainMaxPotionCount(
                Amount::Fixed(2),
            ))],
        ),
        red_skull(),
        // A regal pillow makes resting worth fifteen more.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::ModifyRestSiteHealAmount,
                vec![Command::Modify(ModifierCmd::Add(Amount::Fixed(15)))],
            )],
            ..obtained_only("RELIC.REGAL_PILLOW", RelicRarity::Common)
        },
        // A strawberry is seven more maximum HP the moment it is taken.
        taken_effect(
            "RELIC.STRAWBERRY",
            RelicRarity::Common,
            vec![Command::Creature(CreatureCmd::GainMaxHp {
                target: CreatureSelector::Owner,
                amount: Amount::Fixed(7),
            })],
        ),
        // A venerable tea set remembers a rest and pays two energy as the next
        // fight's energy is set.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterRoomEntered,
                    vec![in_room(
                        sts2_engine::ResolvedRoom::RestSite,
                        vec![latch(true)],
                    )],
                ),
                ModifierHook::every_turn(
                    HookEvent::AfterEnergyReset,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningRelicIsLatched,
                        then: vec![gain_energy(2), latch(false)],
                    }],
                ),
            ],
            latch_property: Some("GainEnergyInNextCombat".to_owned()),
            ..obtained_only("RELIC.VENERABLE_TEA_SET", RelicRarity::Common)
        },
        upgrades_two_of_the_deck("RELIC.WAR_PAINT", sts2_engine::CardType::Skill),
        upgrades_two_of_the_deck("RELIC.WHETSTONE", sts2_engine::CardType::Attack),
        // A meal ticket heals fifteen every time its owner walks into a shop.
        stops_late(RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterRoomEntered,
                vec![in_room(
                    sts2_engine::ResolvedRoom::Shop,
                    vec![heal(Amount::Fixed(15))],
                )],
            )],
            ..obtained_only("RELIC.MEAL_TICKET", RelicRarity::Common)
        }),
        // A book of five rings counts every card that joins the deck and heals
        // twenty on every fifth.
        stops_late(RelicDefinition {
            counter_property: Some("CardsAdded".to_owned()),
            counter_kind: Some(sts2_engine::CounterKind::Cycle { wraps_at: 5 }),
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardChangedPiles,
                    vec![
                        Command::Relic(RelicCmd::ModifyListeningCounter(Amount::Fixed(1))),
                        Command::When {
                            otherwise: Vec::new(),
                            condition: Condition::ListeningRelicCounterMultipleOf(5),
                            then: vec![heal(Amount::Fixed(20))],
                        },
                    ],
                )
                .when(HookCondition::CardJoinedTheDeck),
            ],
            ..obtained_only("RELIC.BOOK_OF_FIVE_RINGS", RelicRarity::Common)
        }),
        // An amethyst aubergine puts fifteen more gold on what a fight pays
        // out, in every combat room but the one the run ends in.
        never_in_shops(stops_late(RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::TryModifyRewards,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::All(vec![
                        Condition::CurrentRoomIsCombat,
                        Condition::Not(Box::new(Condition::All(vec![
                            Condition::CurrentRoomIs(sts2_engine::ResolvedRoom::Boss),
                            Condition::InTheLastAct,
                        ]))),
                    ]),
                    then: vec![Command::Rewards(sts2_engine::RewardsCmd::AddGold(
                        Amount::Fixed(15),
                    ))],
                }],
            )],
            ..obtained_only("RELIC.AMETHYST_AUBERGINE", RelicRarity::Common)
        })),
    ]
}

/// The uncommon relics of the shared pool, and the Ironclad's own two.
#[must_use]
#[allow(clippy::too_many_lines, reason = "one entry per relic of the rarity")]
fn uncommon_relics() -> Vec<RelicDefinition> {
    vec![
        // An akabeko is eight vigor as the opening turn settles.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    vec![own_power("POWER.VIGOR_POWER", Amount::Fixed(8))],
                )
                .when(HookCondition::OwnerIsParticipant)
                .timed(HookTiming::FirstTurnOnly),
            ],
            ..obtained_only("RELIC.AKABEKO", RelicRarity::Uncommon)
        },
        // A candelabra pays two energy as the second turn opens.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(HookEvent::AfterSideTurnStart, vec![gain_energy(2)])
                    .when(HookCondition::OwnerIsParticipant)
                    .timed(HookTiming::SecondTurnOnly),
            ],
            ..obtained_only("RELIC.CANDELABRA", RelicRarity::Uncommon)
        },
        // An eternal feather heals three for every five cards of the deck as
        // its owner walks into a rest site. The division comes first and what
        // is left of it is dropped.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterRoomEntered,
                vec![in_room(
                    sts2_engine::ResolvedRoom::RestSite,
                    vec![heal(Amount::Product {
                        left: Box::new(Amount::Fixed(3)),
                        right: Box::new(Amount::Percent {
                            percent: 20,
                            count: sts2_engine::Count::DeckSize,
                        }),
                    })],
                )],
            )],
            ..obtained_only("RELIC.ETERNAL_FEATHER", RelicRarity::Uncommon)
        },
        // A gremlin horn pays an energy and a card for every enemy that goes
        // down.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(HookEvent::AfterDeath, vec![gain_energy(1), draw(1)])
                    .when(HookCondition::OpponentDied),
            ],
            ..obtained_only("RELIC.GREMLIN_HORN", RelicRarity::Uncommon)
        },
        joss_paper(),
        kusarigama(),
        // A mercury hourglass cuts every enemy as its owner's turn opens.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterPlayerTurnStart,
                vec![Command::Creature(CreatureCmd::Damage {
                    target: CreatureSelector::AllOpponents,
                    amount: Amount::Fixed(3),
                    props: ValueProps::UNPOWERED,
                })],
            )],
            ..obtained_only("RELIC.MERCURY_HOURGLASS", RelicRarity::Uncommon)
        },
        orichalcum(),
        // A pantograph heals twenty-five as a boss fight opens.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::BeforeCombatStart,
                vec![in_room(
                    sts2_engine::ResolvedRoom::Boss,
                    vec![heal(Amount::Fixed(25))],
                )],
            )],
            ..obtained_only("RELIC.PANTOGRAPH", RelicRarity::Uncommon)
        },
        // A paper phrog has no listener body of its own. Vulnerable reads it
        // off whoever is swinging and raises its own multiplier by a quarter,
        // which is where the relic is transcribed.
        obtained_only("RELIC.PAPER_PHROG", RelicRarity::Uncommon),
        pen_nib(),
        // A permafrost pays seven block for the first power its owner plays in
        // a fight.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(HookEvent::AfterRoomEntered, vec![latch(false)])
                    .when(HookCondition::InACombatRoom),
                ModifierHook::every_turn(
                    HookEvent::AfterCardPlayed,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::Not(Box::new(Condition::ListeningRelicIsLatched)),
                        then: vec![own_block(7), latch(true)],
                    }],
                )
                .when(HookCondition::PlayedCardIsOfType(
                    sts2_engine::CardType::Power,
                )),
            ],
            ..obtained_only("RELIC.PERMAFROST", RelicRarity::Uncommon)
        },
        // A petrified toad hands over a rock as every fight opens.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::BeforeCombatStartLate,
                vec![Command::Potion(sts2_engine::PotionCmd::TryToProcure(
                    sts2_engine::PotionOrigin::Model(id("POTION.POTION_SHAPED_ROCK")),
                ))],
            )],
            ..obtained_only("RELIC.PETRIFIED_TOAD", RelicRarity::Uncommon)
        },
        // A reptile trinket pays three strength for the turn every time its
        // owner drinks something.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterPotionUsed,
                vec![own_power("POWER.REPTILE_TRINKET_POWER", Amount::Fixed(3))],
            )],
            ..obtained_only("RELIC.REPTILE_TRINKET", RelicRarity::Uncommon)
        },
        // Self-forming clay owes three block next turn for every blow that gets
        // through.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterDamageReceived,
                    vec![own_power("POWER.SELF_FORMING_CLAY_POWER", Amount::Fixed(3))],
                )
                .when(HookCondition::OwnerTookUnblockedDamage),
            ],
            ..obtained_only("RELIC.SELF_FORMING_CLAY", RelicRarity::Uncommon)
        },
        // A sparkling rouge pays a strength and a dexterity as the third turn
        // clears its owner's block.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterBlockCleared,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::PlayerTurnIs(3),
                    then: vec![
                        own_power("POWER.STRENGTH_POWER", Amount::Fixed(1)),
                        own_power("POWER.DEXTERITY_POWER", Amount::Fixed(1)),
                    ],
                }],
            )],
            ..obtained_only("RELIC.SPARKLING_ROUGE", RelicRarity::Uncommon)
        },
        // A stone cracker upgrades two of the draw pile as a fight is walked
        // into, stably shuffled on the combat card-selection stream.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterRoomEntered,
                    vec![Command::Card(sts2_engine::CardCmd::Upgrade {
                        cards: sts2_engine::CardSelector::StablyShuffledFromPile {
                            pile: sts2_engine::PileName::Draw,
                            filter: sts2_engine::CardFilter::Upgradable,
                            fallback: None,
                            take: Amount::Fixed(2),
                            stream: sts2_engine::ShuffleStream::CombatCardSelection,
                        },
                    })],
                )
                .when(HookCondition::InACombatRoom),
            ],
            ..obtained_only("RELIC.STONE_CRACKER", RelicRarity::Uncommon)
        },
        tuning_fork(),
        vambrace(),
        // A tiny mailbox puts two potions on the rest site's reward screen.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::TryModifyRestSiteHealRewards,
                vec![
                    Command::Rewards(sts2_engine::RewardsCmd::AddPotion),
                    Command::Rewards(sts2_engine::RewardsCmd::AddPotion),
                ],
            )],
            ..obtained_only("RELIC.TINY_MAILBOX", RelicRarity::Uncommon)
        },
        // A bowler hat is a quarter more gold, wherever the gold comes from.
        never_in_shops(stops_late(RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::ModifyGoldGained,
                vec![Command::Modify(ModifierCmd::Scale(Amount::Fixed(125)))],
            )],
            ..obtained_only("RELIC.BOWLER_HAT", RelicRarity::Uncommon)
        })),
        // A lucky fysh pays fifteen gold for every card that joins the deck.
        never_in_shops(stops_late(RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardChangedPiles,
                    vec![Command::Player(PlayerCmd::GainGold(Amount::Fixed(15)))],
                )
                .when(HookCondition::CardJoinedTheDeck),
            ],
            ..obtained_only("RELIC.LUCKY_FYSH", RelicRarity::Uncommon)
        })),
        // A planisphere heals five every time its owner walks onto an unknown
        // point, whatever the point turned out to be.
        stops_late(RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterRoomEntered,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::CurrentMapPointIsUnknown,
                    then: vec![heal(Amount::Fixed(5))],
                }],
            )],
            ..obtained_only("RELIC.PLANISPHERE", RelicRarity::Uncommon)
        }),
        lasting_candy(),
    ]
}

/// The relics an event hands over. None of them is rolled: they reach a player
/// only through the event that gives them, and most of those events are not
/// built yet. What each one does is transcribed and tested all the same.
#[must_use]
#[allow(clippy::too_many_lines, reason = "one entry per relic of the rarity")]
fn event_relics() -> Vec<RelicDefinition> {
    vec![
        // A big mushroom is twenty more maximum HP, and two fewer cards on
        // every fight's opening hand.
        upon_pickup(RelicDefinition {
            on_obtained: vec![Command::Creature(CreatureCmd::GainMaxHp {
                target: CreatureSelector::Owner,
                amount: Amount::Fixed(20),
            })],
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::ModifyHandDraw,
                    vec![Command::Modify(ModifierCmd::Add(Amount::Fixed(-2)))],
                )
                .timed(HookTiming::FirstTurnOnly),
            ],
            ..obtained_only("RELIC.BIG_MUSHROOM", RelicRarity::Event)
        }),
        // A bing bong copies every card that joins the deck, and does not
        // answer for the copies it made.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardChangedPiles,
                    vec![Command::CardPile(CardPileCmd::CloneCardJoiningTheDeck)],
                )
                .when(HookCondition::CardJoinedTheDeck),
            ],
            ..obtained_only("RELIC.BING_BONG", RelicRarity::Event)
        },
        // A bone tea upgrades the whole opening hand of the next fight, once.
        spends_itself(
            "RELIC.BONE_TEA",
            1,
            vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningRelicCounterAtLeast(1),
                        then: vec![
                            Command::Card(sts2_engine::CardCmd::Upgrade {
                                cards: sts2_engine::CardSelector::Pile {
                                    pile: sts2_engine::PileName::Hand,
                                    filter: sts2_engine::CardFilter::Any,
                                },
                            }),
                            Command::Relic(RelicCmd::ModifyListeningCounter(Amount::Fixed(-1))),
                        ],
                    }],
                )
                .when(HookCondition::OwnerIsParticipant)
                .timed(HookTiming::FirstTurnOnly),
            ],
        ),
        // A byrdpip brings a pet to every fight, and turns every byrdonis egg
        // its owner is carrying into a swoop as it is taken.
        upon_pickup(RelicDefinition {
            spawns_pets: true,
            adds_pet: true,
            rolled_skin: Some(skin_of_four()),
            on_obtained: vec![Command::Card(CardCmd::TransformDeckCards {
                cards: CardSelector::DeckCardsOfModel(id("CARD.BYRDONIS_EGG")),
                replacement: sts2_engine::CardTransformReplacement::Named {
                    model: id("CARD.BYRD_SWOOP"),
                    keeps_upgrade: false,
                    keeps_enchantment: false,
                },
                one_at_a_time: false,
            })],
            hooks: vec![ModifierHook::every_turn(
                HookEvent::BeforeCombatStart,
                vec![Command::Player(PlayerCmd::AddPet {
                    monster: id("MONSTER.BYRDPIP"),
                })],
            )],
            ..obtained_only("RELIC.BYRDPIP", RelicRarity::Event)
        }),
        // A chosen cheese is a maximum HP for every fight that ends.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterCombatEnd,
                vec![Command::Creature(CreatureCmd::GainMaxHp {
                    target: CreatureSelector::Owner,
                    amount: Amount::Fixed(1),
                })],
            )],
            ..obtained_only("RELIC.CHOSEN_CHEESE", RelicRarity::Event)
        },
        // A darkstone periapt is six more maximum HP for every curse that
        // joins the deck.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardChangedPiles,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::JoinedDeckCardIsOfType(sts2_engine::CardType::Curse),
                        then: vec![Command::Creature(CreatureCmd::GainMaxHp {
                            target: CreatureSelector::Owner,
                            amount: Amount::Fixed(6),
                        })],
                    }],
                )
                .when(HookCondition::CardJoinedTheDeck),
            ],
            ..obtained_only("RELIC.DARKSTONE_PERIAPT", RelicRarity::Event)
        },
        // A daughter of the wind pays a block for every attack its owner plays.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(HookEvent::AfterCardPlayed, vec![own_block(1)])
                    .when(HookCondition::PlayedCardIsAttack),
            ],
            ..obtained_only("RELIC.DAUGHTER_OF_THE_WIND", RelicRarity::Event)
        },
        // A dream catcher puts a card offer on the rest site's reward screen.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::TryModifyRestSiteHealRewards,
                vec![Command::Rewards(sts2_engine::RewardsCmd::AddCardOffer {
                    room: sts2_engine::ResolvedRoom::Monster,
                    count: 3,
                })],
            )],
            ..obtained_only("RELIC.DREAM_CATCHER", RelicRarity::Event)
        },
        // An ember tea works two strength in for each of the next five fights.
        spends_itself(
            "RELIC.EMBER_TEA",
            5,
            vec![
                ModifierHook::every_turn(
                    HookEvent::AfterRoomEntered,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningRelicCounterAtLeast(1),
                        then: vec![
                            own_power_unattributed("POWER.STRENGTH_POWER", Amount::Fixed(2)),
                            Command::Relic(RelicCmd::ModifyListeningCounter(Amount::Fixed(-1))),
                        ],
                    }],
                )
                .when(HookCondition::InACombatRoom),
            ],
        ),
        // A fake anchor is four block already standing as a fight opens.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::BeforeCombatStart,
                vec![own_block(4)],
            )],
            ..obtained_only("RELIC.FAKE_ANCHOR", RelicRarity::Event)
        },
        // A fake blood vial heals one as the opening turn settles.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterPlayerTurnStartLate,
                    vec![heal(Amount::Fixed(1))],
                )
                .timed(HookTiming::FirstTurnOnly),
            ],
            ..obtained_only("RELIC.FAKE_BLOOD_VIAL", RelicRarity::Event)
        },
        // A fake happy flower counts turns and pays an energy on every fifth.
        RelicDefinition {
            counter_property: Some("TurnsSeen".to_owned()),
            counter_kind: Some(sts2_engine::CounterKind::Cycle { wraps_at: 5 }),
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    counts_to(5)
                        .into_iter()
                        .chain(std::iter::once(Command::When {
                            otherwise: Vec::new(),
                            condition: Condition::ListeningRelicCounterIs(0),
                            then: vec![gain_energy(1)],
                        }))
                        .collect(),
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..obtained_only("RELIC.FAKE_HAPPY_FLOWER", RelicRarity::Event)
        },
        // A fake Lee's waffle heals a tenth of its owner's maximum HP.
        taken_effect(
            "RELIC.FAKE_LEES_WAFFLE",
            RelicRarity::Event,
            vec![heal(Amount::Percent {
                percent: 10,
                count: sts2_engine::Count::OwnerMaxHp,
            })],
        ),
        // A fake mango is three more maximum HP the moment it is taken.
        taken_effect(
            "RELIC.FAKE_MANGO",
            RelicRarity::Event,
            vec![Command::Creature(CreatureCmd::GainMaxHp {
                target: CreatureSelector::Owner,
                amount: Amount::Fixed(3),
            })],
        ),
        // A fake merchant's rug does nothing at all: it is a rarity and
        // nothing else.
        obtained_only("RELIC.FAKE_MERCHANTS_RUG", RelicRarity::Event),
        // A fake orichalcum pays three block at the end of a turn its owner
        // finished behind nothing, checked before a plating pays out.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::BeforeSideTurnEndVeryEarly,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::Not(Box::new(Condition::OwnerBlockAtLeast(1))),
                        then: vec![latch(true)],
                    }],
                )
                .when(HookCondition::OwnerIsParticipant),
                ModifierHook::every_turn(
                    HookEvent::BeforeSideTurnEnd,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningRelicIsLatched,
                        then: vec![latch(false), own_block(3)],
                    }],
                ),
                ModifierHook::every_turn(HookEvent::BeforeSideTurnStart, vec![latch(false)])
                    .when(HookCondition::OwnerIsParticipant),
            ],
            ..obtained_only("RELIC.FAKE_ORICHALCUM", RelicRarity::Event)
        },
        // A fake snecko eye confuses its owner as it is taken inside a fight,
        // and as every fight opens after.
        RelicDefinition {
            on_obtained: vec![Command::When {
                otherwise: Vec::new(),
                condition: Condition::InACombat,
                then: vec![own_power("POWER.CONFUSED_POWER", Amount::Fixed(1))],
            }],
            hooks: vec![ModifierHook::every_turn(
                HookEvent::BeforeCombatStart,
                vec![own_power("POWER.CONFUSED_POWER", Amount::Fixed(1))],
            )],
            // Priced at 50, which an event relic has to state for itself: its
            // rarity's own price is out of reach.
            merchant_cost_override: Some(50),
            ..obtained_only("RELIC.FAKE_SNECKO_EYE", RelicRarity::Event)
        },
        // A fake strike dummy pays one more on every Strike its owner throws.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::ModifyDamageAdditive,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::DamageCardIsTagged(sts2_engine::CardTag::Strike),
                        then: vec![Command::Modify(ModifierCmd::Add(Amount::Fixed(1)))],
                    }],
                )
                .when(HookCondition::OwnerDealsPoweredDamage),
            ],
            ..obtained_only("RELIC.FAKE_STRIKE_DUMMY", RelicRarity::Event)
        },
        // A fake venerable tea set remembers a rest and pays one energy as the
        // next fight's energy is set.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterRoomEntered,
                    vec![in_room(
                        sts2_engine::ResolvedRoom::RestSite,
                        vec![latch(true)],
                    )],
                ),
                ModifierHook::every_turn(
                    HookEvent::AfterEnergyReset,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningRelicIsLatched,
                        then: vec![gain_energy(1), latch(false)],
                    }],
                ),
            ],
            latch_property: Some("GainEnergyInNextCombat".to_owned()),
            ..obtained_only("RELIC.FAKE_VENERABLE_TEA_SET", RelicRarity::Event)
        },
        // A forgotten soul cuts one enemy it rolls for every time a card of
        // its owner's burns up.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardExhausted,
                    vec![Command::Creature(CreatureCmd::Damage {
                        target: CreatureSelector::RandomOpponent,
                        amount: Amount::Fixed(1),
                        props: ValueProps::UNPOWERED,
                    })],
                )
                .when(HookCondition::OwnersCardExhausted),
            ],
            ..obtained_only("RELIC.FORGOTTEN_SOUL", RelicRarity::Event)
        },
        // A fragrant mushroom costs fifteen HP and upgrades two of the deck.
        taken_effect(
            "RELIC.FRAGRANT_MUSHROOM",
            RelicRarity::Event,
            vec![
                Command::Creature(CreatureCmd::Damage {
                    target: CreatureSelector::Owner,
                    amount: Amount::Fixed(15),
                    props: ValueProps::UNBLOCKABLE,
                }),
                Command::Card(sts2_engine::CardCmd::Upgrade {
                    cards: sts2_engine::CardSelector::StablyShuffledFromDeck {
                        filter: sts2_engine::CardFilter::Upgradable,
                        take: Amount::Fixed(2),
                        stream: sts2_engine::ShuffleStream::Niche,
                    },
                }),
            ],
        ),
        // A fresnel lens puts two Nimble on every card it meets that will take
        // one: on a card reward's options, on what a merchant creates, and on
        // whatever joins the deck.
        RelicDefinition {
            enchants_cards_it_meets: Some(sts2_engine::EnchantmentGrant {
                enchantment: id("ENCHANTMENT.NIMBLE"),
                amount: 2,
            }),
            hooks: vec![ModifierHook::every_turn(
                HookEvent::TryModifyCardRewardOptionsLate,
                vec![Command::Rewards(RewardsCmd::EnchantCardOptions {
                    enchantment: id("ENCHANTMENT.NIMBLE"),
                    amount: Amount::Fixed(2),
                    one_rolled_on_the_niche_stream: false,
                })],
            )],
            ..obtained_only("RELIC.FRESNEL_LENS", RelicRarity::Event)
        },
        // A hand drill works two vulnerable into whoever its owner's blow took
        // the last of the block off.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterDamageGiven,
                    vec![Command::Power(PowerCmd::Apply {
                        marks: None,
                        power: id("POWER.VULNERABLE_POWER"),
                        target: CreatureSelector::Target,
                        applier: Some(CreatureSelector::Owner),
                        amount: Amount::Fixed(2),
                    })],
                )
                .when(HookCondition::OwnerBrokeAnOpponentsBlock),
            ],
            ..obtained_only("RELIC.HAND_DRILL", RelicRarity::Event)
        },
        // A history course replays the last attack or skill its owner played
        // the turn before, as a copy that goes nowhere afterwards. The first
        // turn of a fight has no turn before it.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterAutoPrePlayPhaseEntered,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::PlayerTurnAtLeast(2),
                    then: vec![Command::Card(sts2_engine::CardCmd::AutoPlayDupe {
                        cards: sts2_engine::CardSelector::LastPlayedLastTurn {
                            types: vec![
                                sts2_engine::CardType::Attack,
                                sts2_engine::CardType::Skill,
                            ],
                        },
                    })],
                }],
            )],
            ..obtained_only("RELIC.HISTORY_COURSE", RelicRarity::Event)
        },
        // A lost wisp cuts every enemy for every power its owner plays.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardPlayed,
                    vec![Command::Creature(CreatureCmd::Damage {
                        target: CreatureSelector::AllOpponents,
                        amount: Amount::Fixed(8),
                        props: ValueProps::UNPOWERED,
                    })],
                )
                .when(HookCondition::PlayedCardIsOfType(
                    sts2_engine::CardType::Power,
                )),
            ],
            ..obtained_only("RELIC.LOST_WISP", RelicRarity::Event)
        },
        maw_bank(),
        // Mr Struggles cuts every enemy for the number of the turn, as each of
        // its owner's turns opens.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterPlayerTurnStart,
                vec![Command::Creature(CreatureCmd::Damage {
                    target: CreatureSelector::AllOpponents,
                    amount: Amount::counting(sts2_engine::Count::PlayerTurn),
                    props: ValueProps::UNPOWERED,
                })],
            )],
            ..obtained_only("RELIC.MR_STRUGGLES", RelicRarity::Event)
        },
        pollinous_core(),
        royal_poison(),
        // A sword of jade works three strength in as a fight is walked into.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterRoomEntered,
                    vec![own_power_unattributed(
                        "POWER.STRENGTH_POWER",
                        Amount::Fixed(3),
                    )],
                )
                .when(HookCondition::InACombatRoom),
            ],
            ..obtained_only("RELIC.SWORD_OF_JADE", RelicRarity::Event)
        },
        // A sword of stone counts the elites its owner puts down and becomes a
        // sword of jade on the fifth.
        RelicDefinition {
            counter_property: Some("ElitesDefeated".to_owned()),
            counter_kind: Some(sts2_engine::CounterKind::Ledger),
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterCombatVictory,
                vec![in_room(
                    sts2_engine::ResolvedRoom::Elite,
                    vec![
                        Command::Relic(RelicCmd::ModifyListeningCounter(Amount::Fixed(1))),
                        Command::When {
                            otherwise: Vec::new(),
                            condition: Condition::ListeningRelicCounterAtLeast(5),
                            then: vec![Command::Relic(RelicCmd::Replace {
                                original: id("RELIC.SWORD_OF_STONE"),
                                replacement: id("RELIC.SWORD_OF_JADE"),
                            })],
                        },
                    ],
                )],
            )],
            ..obtained_only("RELIC.SWORD_OF_STONE", RelicRarity::Event)
        },
        // A tea of discourtesy puts two Dazed in the next fight's draw pile.
        spends_itself(
            "RELIC.TEA_OF_DISCOURTESY",
            1,
            vec![ModifierHook::every_turn(
                HookEvent::BeforeCombatStart,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::ListeningRelicCounterAtLeast(1),
                    then: vec![
                        Command::CardPile(CardPileCmd::AddGeneratedToCombat {
                            enchantment: None,
                            card: CardOrigin::Model(id("CARD.DAZED")),
                            grown_by: Amount::Fixed(0),
                            count: Amount::Fixed(2),
                            pile: sts2_engine::PileName::Draw,
                            upgraded: false,
                            ethereal: false,
                            retain: false,
                            temporary_cost: None,
                            position: sts2_engine::DrawPilePosition::Random,
                        }),
                        Command::Relic(RelicCmd::ModifyListeningCounter(Amount::Fixed(-1))),
                    ],
                }],
            )],
        ),
        // The boot holds every small blow its owner deals up to five.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::ModifyHpLostAfterOstyLate,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::All(vec![
                            Condition::ModifiedValueAtLeast(1),
                            Condition::Not(Box::new(Condition::ModifiedValueAtLeast(5))),
                        ]),
                        then: vec![Command::Modify(ModifierCmd::Set(Amount::Fixed(5)))],
                    }],
                )
                .when(HookCondition::OwnerDealsPoweredDamage),
            ],
            ..obtained_only("RELIC.THE_BOOT", RelicRarity::Event)
        },
        wongos_mystery_ticket(),
        // A Wongo customer appreciation badge does nothing at all: it is a
        // rarity and nothing else.
        obtained_only(
            "RELIC.WONGO_CUSTOMER_APPRECIATION_BADGE",
            RelicRarity::Event,
        ),
    ]
}

/// A relic given a number of uses: the count it is taken holding is that
/// number, it is spent down by its own body, and it stops being tradable once
/// there is nothing left of it.
fn spends_itself(model_id: &str, uses: i32, hooks: Vec<ModifierHook>) -> RelicDefinition {
    RelicDefinition {
        counter_property: Some("CombatsLeft".to_owned()),
        counter_kind: Some(sts2_engine::CounterKind::Ledger),
        initial_counter: uses,
        used_up: Some(sts2_engine::UsedUp::CounterAtMost(0)),
        hooks,
        ..obtained_only(model_id, RelicRarity::Event)
    }
}

/// A maw bank pays twelve gold every time its owner walks into a room, and
/// stops the moment they buy anything.
fn maw_bank() -> RelicDefinition {
    RelicDefinition {
        used_up: Some(sts2_engine::UsedUp::Latched),
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterRoomEntered,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::Not(Box::new(Condition::ListeningRelicIsLatched)),
                    then: vec![Command::Player(PlayerCmd::GainGold(Amount::Fixed(12)))],
                }],
            ),
            ModifierHook::every_turn(HookEvent::AfterItemPurchased, vec![latch(true)]),
        ],
        latch_property: Some("HasItemBeenBought".to_owned()),
        ..obtained_only("RELIC.MAW_BANK", RelicRarity::Event)
    }
}

/// A pollinous core swells every fourth turn's hand by two.
///
/// The build counts the turns as each one opens and clears the count from the
/// pass over whoever changed the draw, which is why the count is cleared where
/// it is rather than when it is read.
fn pollinous_core() -> RelicDefinition {
    RelicDefinition {
        counter_property: Some("TurnsSeen".to_owned()),
        counter_kind: Some(sts2_engine::CounterKind::Cycle { wraps_at: 4 }),
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::BeforeSideTurnStart,
                vec![Command::Relic(RelicCmd::ModifyListeningCounter(
                    Amount::Fixed(1),
                ))],
            )
            .when(HookCondition::OwnerIsParticipant),
            ModifierHook::every_turn(
                HookEvent::ModifyHandDraw,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::ListeningRelicCounterIs(4),
                    then: vec![
                        Command::Modify(ModifierCmd::Add(Amount::Fixed(2))),
                        Command::Relic(RelicCmd::SetListeningCounter(Amount::Fixed(0))),
                    ],
                }],
            ),
        ],
        ..obtained_only("RELIC.POLLINOUS_CORE", RelicRarity::Event)
    }
}

/// A Wongo's mystery ticket counts five fights and then pays three relics on
/// the fifth one's reward screen, once.
fn wongos_mystery_ticket() -> RelicDefinition {
    RelicDefinition {
        counter_property: Some("CombatsFinished".to_owned()),
        counter_kind: Some(sts2_engine::CounterKind::Ledger),
        used_up: Some(sts2_engine::UsedUp::Latched),
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterCombatEnd,
                vec![Command::Relic(RelicCmd::ModifyListeningCounter(
                    Amount::Fixed(1),
                ))],
            ),
            ModifierHook::every_turn(
                HookEvent::TryModifyRewards,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::All(vec![
                        Condition::CurrentRoomIsCombat,
                        Condition::Not(Box::new(Condition::ListeningRelicIsLatched)),
                        Condition::ListeningRelicCounterAtLeast(5),
                    ]),
                    then: vec![
                        Command::Rewards(sts2_engine::RewardsCmd::AddRelic),
                        Command::Rewards(sts2_engine::RewardsCmd::AddRelic),
                        Command::Rewards(sts2_engine::RewardsCmd::AddRelic),
                    ],
                }],
            ),
            ModifierHook::every_turn(HookEvent::AfterModifyingRewards, vec![latch(true)]),
        ],
        latch_property: Some("GaveRelic".to_owned()),
        ..obtained_only("RELIC.WONGOS_MYSTERY_TICKET", RelicRarity::Event)
    }
}

/// The shop relics of the shared pool, and the Ironclad's own one.
///
/// The four shop relics that belong to the Silent, the Defect, the Necrobinder,
/// and the Regent land with those characters and are not registered here.
#[must_use]
#[allow(clippy::too_many_lines, reason = "one entry per relic of the rarity")]
fn shop_relics() -> Vec<RelicDefinition> {
    vec![
        belt_buckle(),
        // A bread pays an energy on every turn but the first, and takes two off
        // the first one.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::ModifyMaxEnergy,
                    vec![Command::Modify(ModifierCmd::Add(Amount::Fixed(1)))],
                )
                .timed(HookTiming::AfterFirstTurn),
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    vec![Command::Player(PlayerCmd::LoseEnergy(Amount::Fixed(2)))],
                )
                .when(HookCondition::OwnerIsParticipant)
                .timed(HookTiming::FirstTurnOnly),
            ],
            ..obtained_only("RELIC.BREAD", RelicRarity::Shop)
        },
        // A brimstone works two strength into its owner and one into everything
        // still standing against them, as every turn opens.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    vec![
                        own_power("POWER.STRENGTH_POWER", Amount::Fixed(2)),
                        Command::Power(PowerCmd::Apply {
                            marks: None,
                            power: id("POWER.STRENGTH_POWER"),
                            target: CreatureSelector::AllOpponents,
                            applier: None,
                            amount: Amount::Fixed(1),
                        }),
                    ],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..obtained_only("RELIC.BRIMSTONE", RelicRarity::Shop)
        },
        burning_sticks(),
        // A cauldron hands over five potions the moment it is taken.
        upon_pickup(RelicDefinition {
            on_obtained: vec![
                Command::Rewards(sts2_engine::RewardsCmd::AddPotion),
                Command::Rewards(sts2_engine::RewardsCmd::AddPotion),
                Command::Rewards(sts2_engine::RewardsCmd::AddPotion),
                Command::Rewards(sts2_engine::RewardsCmd::AddPotion),
                Command::Rewards(sts2_engine::RewardsCmd::AddPotion),
                Command::Rewards(sts2_engine::RewardsCmd::OfferAdded),
            ],
            ..obtained_only("RELIC.CAULDRON", RelicRarity::Shop)
        }),
        // Chemical X pays two more X on every X-cost play its owner makes, of
        // energy and of stars alike: the X pass never asks which kind of X
        // was wanted, and whether the play costs star X is read only for the
        // flash the relic plays as the card goes down.
        RelicDefinition {
            modifies_x_value_by: 2,
            ..obtained_only("RELIC.CHEMICAL_X", RelicRarity::Shop)
        },
        // A dingy rug puts the colorless pool behind the character's own on
        // every card reward.
        RelicDefinition {
            adds_card_pool_to_rewards: Some(id("CARD_POOL.COLORLESS_CARD_POOL")),
            adds_every_character_pool_to_rewards: false,
            ..obtained_only("RELIC.DINGY_RUG", RelicRarity::Shop)
        },
        // A dolly's mirror copies a chosen deck card into the deck.
        upon_pickup(RelicDefinition {
            on_obtained: vec![Command::CardSelect(
                sts2_engine::CardSelectCmd::FromDeckGeneric {
                    semantics: ChoiceSemantics::known(vec![
                        ChoiceOp::new(ChoiceOperationKind::Copy, ChoiceScope::Run)
                            .on(ChoiceSubject::Copy)
                            .to(ChoiceDestination::RunDeck),
                    ]),
                    prefs: sts2_engine::SelectorPrefs::exactly(1),
                    filter: sts2_engine::CardFilter::NotOfType(sts2_engine::CardType::Quest),
                    then: vec![Command::CardPile(CardPileCmd::AddToDeck {
                        card: CardOrigin::CloneOfChosen,
                    })],
                },
            )],
            ..obtained_only("RELIC.DOLLYS_MIRROR", RelicRarity::Shop)
        }),
        // A dragon fruit is a maximum HP for every gold its owner takes in.
        stops_late(RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterGoldGained,
                vec![Command::Creature(CreatureCmd::GainMaxHp {
                    target: CreatureSelector::Owner,
                    amount: Amount::Fixed(1),
                })],
            )],
            ..obtained_only("RELIC.DRAGON_FRUIT", RelicRarity::Shop)
        }),
        ghost_seed(),
        // A gnarled hammer puts three Sharp on up to three cards of the deck.
        upon_pickup(RelicDefinition {
            on_obtained: vec![enchant_chosen_deck_cards(
                "ENCHANTMENT.SHARP",
                3,
                SelectorPrefs::between(0, 3).confirmed(),
            )],
            ..obtained_only("RELIC.GNARLED_HAMMER", RelicRarity::Shop)
        }),
        // A kifuda puts three Adroit on up to three of them.
        upon_pickup(RelicDefinition {
            on_obtained: vec![enchant_chosen_deck_cards(
                "ENCHANTMENT.ADROIT",
                3,
                SelectorPrefs::between(0, 3).confirmed(),
            )],
            ..obtained_only("RELIC.KIFUDA", RelicRarity::Shop)
        }),
        // A punch dagger puts five Momentum on one.
        upon_pickup(RelicDefinition {
            on_obtained: vec![enchant_chosen_deck_cards(
                "ENCHANTMENT.MOMENTUM",
                5,
                SelectorPrefs::exactly(1),
            )],
            ..obtained_only("RELIC.PUNCH_DAGGER", RelicRarity::Shop)
        }),
        // A royal stamp makes one card of the deck innate and retained. The
        // build shuffles the candidates on the niche stream before it hands
        // them to the screen, and the screen sorts them straight back into deck
        // order, so what the shuffle comes to is the dice it read.
        upon_pickup(RelicDefinition {
            on_obtained: vec![Command::CardSelect(CardSelectCmd::FromDeckForEnchantment {
                semantics: ChoiceSemantics::known(vec![
                    ChoiceOp::new(ChoiceOperationKind::Enchant, ChoiceScope::Run)
                        .model(id("ENCHANTMENT.ROYALLY_APPROVED"))
                        .amount(1),
                ]),
                enchantment: id("ENCHANTMENT.ROYALLY_APPROVED"),
                amount: 1,
                filter: CardFilter::Any,
                shuffled_on: Some(sts2_engine::ShuffleStream::Niche),
                prefs: SelectorPrefs::exactly(1),
                then: vec![Command::Card(CardCmd::Enchant {
                    cards: CardSelector::Chosen,
                    enchantment: id("ENCHANTMENT.ROYALLY_APPROVED"),
                    amount: Amount::Fixed(1),
                })],
            })],
            ..obtained_only("RELIC.ROYAL_STAMP", RelicRarity::Shop)
        }),
        // A wing charm puts one Swift on one card of every card reward, rolled
        // on the niche stream out of the ones it can go on.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::TryModifyCardRewardOptionsLate,
                    vec![Command::Rewards(RewardsCmd::EnchantCardOptions {
                        enchantment: id("ENCHANTMENT.SWIFT"),
                        amount: Amount::Fixed(1),
                        one_rolled_on_the_niche_stream: true,
                    })],
                )
                .when(HookCondition::Always),
            ],
            ..obtained_only("RELIC.WING_CHARM", RelicRarity::Shop)
        },
        // A mystic lighter pays nine on a powered attack from an enchanted card
        // of its owner's.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::ModifyDamageAdditive,
                    vec![Command::Modify(ModifierCmd::Add(Amount::Fixed(9)))],
                )
                // The relic asks three things before it adds anything, in
                // this order: that the blow is a powered attack, that the card
                // dealing it carries an enchantment, and that the card is its
                // own owner's.
                .when(HookCondition::All(vec![
                    HookCondition::ValueIsAPoweredAttack,
                    HookCondition::DamageCardIsEnchanted,
                    HookCondition::DamageCardBelongsToOwner,
                ])),
            ],
            ..obtained_only("RELIC.MYSTIC_LIGHTER", RelicRarity::Shop)
        },
        lava_lamp(),
        // A Lee's waffle is seven more maximum HP and the HP to fill it.
        taken_effect(
            "RELIC.LEES_WAFFLE",
            RelicRarity::Shop,
            vec![
                Command::Creature(CreatureCmd::GainMaxHp {
                    target: CreatureSelector::Owner,
                    amount: Amount::Fixed(7),
                }),
                // The bar is filled rather than a fixed number healed.
                heal(Amount::Sum(vec![
                    Amount::counting(sts2_engine::Count::OwnerMaxHp),
                    Amount::Product {
                        left: Box::new(Amount::Fixed(-1)),
                        right: Box::new(Amount::counting(sts2_engine::Count::OwnerCurrentHp)),
                    },
                ])),
            ],
        ),
        // A membership card halves every merchant price. Its discount is the
        // share of the price that is kept rather than the share taken off, so
        // fifty means half the price rather than half off it.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::ModifyMerchantPrice,
                vec![Command::Modify(ModifierCmd::Scale(Amount::Fixed(50)))],
            )],
            ..obtained_only("RELIC.MEMBERSHIP_CARD", RelicRarity::Shop)
        },
        // A miniature tent leaves the rest site's other options standing for
        // its owner, so only the option that was taken comes off the list.
        RelicDefinition {
            keeps_rest_site_options_standing: true,
            ..obtained_only("RELIC.MINIATURE_TENT", RelicRarity::Shop)
        },
        // An orrery hands over five card offers the moment it is taken.
        upon_pickup(RelicDefinition {
            on_obtained: vec![
                Command::Rewards(sts2_engine::RewardsCmd::AddOwnCardOffer { count: 3 }),
                Command::Rewards(sts2_engine::RewardsCmd::AddOwnCardOffer { count: 3 }),
                Command::Rewards(sts2_engine::RewardsCmd::AddOwnCardOffer { count: 3 }),
                Command::Rewards(sts2_engine::RewardsCmd::AddOwnCardOffer { count: 3 }),
                Command::Rewards(sts2_engine::RewardsCmd::AddOwnCardOffer { count: 3 }),
                Command::Rewards(sts2_engine::RewardsCmd::OfferAdded),
            ],
            ..obtained_only("RELIC.ORRERY", RelicRarity::Shop)
        }),
        ringing_triangle(),
        // A screaming flagon cuts every enemy at the end of a turn its owner
        // finished holding nothing.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::BeforeSideTurnEnd,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::Not(Box::new(Condition::CardsInPileAtLeast {
                            pile: sts2_engine::PileName::Hand,
                            count: 1,
                        })),
                        then: vec![Command::Creature(CreatureCmd::Damage {
                            target: CreatureSelector::AllOpponents,
                            amount: Amount::Fixed(20),
                            props: ValueProps::UNPOWERED,
                        })],
                    }],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..obtained_only("RELIC.SCREAMING_FLAGON", RelicRarity::Shop)
        },
        // A sling of courage works two strength in as an elite is walked into.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterRoomEntered,
                vec![in_room(
                    sts2_engine::ResolvedRoom::Elite,
                    vec![own_power("POWER.STRENGTH_POWER", Amount::Fixed(2))],
                )],
            )],
            ..obtained_only("RELIC.SLING_OF_COURAGE", RelicRarity::Shop)
        },
        // An abacus pays six block every time its owner's pile is shuffled.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterShuffle,
                vec![own_block(6)],
            )],
            ..obtained_only("RELIC.THE_ABACUS", RelicRarity::Shop)
        },
        // A toolbox lays three colorless cards out before the opening hand is
        // drawn, and whichever is taken joins that hand.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::BeforeHandDraw,
                    vec![Command::CardSelect(
                        sts2_engine::CardSelectCmd::ChooseAGeneratedCard {
                            semantics: ChoiceSemantics::known(vec![
                                ChoiceOp::new(ChoiceOperationKind::Gain, ChoiceScope::Combat)
                                    .to(ChoiceDestination::Hand),
                            ]),
                            pool: sts2_engine::CardPoolRef::Named(id(
                                "CARD_POOL.COLORLESS_CARD_POOL",
                            )),
                            count: 3,
                            filter: sts2_engine::CardFilter::Any,
                            upgraded: false,
                            can_skip: true,
                            then: vec![Command::CardPile(CardPileCmd::AddGeneratedToCombat {
                                enchantment: None,
                                card: CardOrigin::Chosen,
                                grown_by: Amount::Fixed(0),
                                count: Amount::Fixed(1),
                                pile: sts2_engine::PileName::Hand,
                                upgraded: false,
                                ethereal: false,
                                retain: false,
                                temporary_cost: None,
                                position: sts2_engine::DrawPilePosition::Bottom,
                            })],
                        },
                    )],
                )
                .timed(HookTiming::FirstTurnOnly),
            ],
            ..obtained_only("RELIC.TOOLBOX", RelicRarity::Shop)
        },
    ]
}

/// A belt buckle lends two dexterity for as long as its owner is holding no
/// potion, and takes it back the moment they are.
///
/// The build remembers whether the loan is out; the relic's latch is that
/// note, and it is cleared as a fight opens and as one is won.
fn belt_buckle() -> RelicDefinition {
    let lend = || Command::When {
        otherwise: Vec::new(),
        condition: Condition::All(vec![
            Condition::HoldsNoPotions,
            Condition::Not(Box::new(Condition::ListeningRelicIsLatched)),
        ]),
        then: vec![
            latch(true),
            own_power_unattributed("POWER.DEXTERITY_POWER", Amount::Fixed(2)),
        ],
    };
    let take_back = || Command::When {
        otherwise: Vec::new(),
        condition: Condition::All(vec![
            Condition::Not(Box::new(Condition::HoldsNoPotions)),
            Condition::ListeningRelicIsLatched,
        ]),
        then: vec![
            latch(false),
            own_power_unattributed("POWER.DEXTERITY_POWER", Amount::Fixed(-2)),
        ],
    };
    let in_a_fight = |then: Vec<Command>| Command::When {
        otherwise: Vec::new(),
        condition: Condition::InACombat,
        then,
    };
    RelicDefinition {
        on_obtained: vec![in_a_fight(vec![lend()])],
        hooks: vec![
            ModifierHook::every_turn(HookEvent::BeforeCombatStart, vec![latch(false), lend()]),
            ModifierHook::every_turn(
                HookEvent::AfterPotionProcured,
                vec![in_a_fight(vec![take_back()])],
            ),
            ModifierHook::every_turn(
                HookEvent::AfterPotionDiscarded,
                vec![in_a_fight(vec![lend()])],
            ),
            ModifierHook::every_turn(HookEvent::AfterPotionUsed, vec![in_a_fight(vec![lend()])]),
            ModifierHook::every_turn(HookEvent::AfterCombatVictory, vec![latch(false)]),
        ],
        ..obtained_only("RELIC.BELT_BUCKLE", RelicRarity::Shop)
    }
}

/// Burning sticks copy the first skill of each fight that burns up back into
/// their owner's hand.
fn burning_sticks() -> RelicDefinition {
    RelicDefinition {
        hooks: vec![
            ModifierHook::every_turn(HookEvent::AfterRoomEntered, vec![latch(false)])
                .when(HookCondition::InACombatRoom),
            ModifierHook::every_turn(
                HookEvent::AfterCardExhausted,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::Not(Box::new(Condition::ListeningRelicIsLatched)),
                    then: vec![Command::Select {
                        cards: sts2_engine::CardSelector::Announced,
                        then: vec![
                            Command::CardPile(CardPileCmd::AddGeneratedToCombat {
                                enchantment: None,
                                card: CardOrigin::CloneOfChosen,
                                grown_by: Amount::Fixed(0),
                                count: Amount::Fixed(1),
                                pile: sts2_engine::PileName::Hand,
                                upgraded: false,
                                ethereal: false,
                                retain: false,
                                temporary_cost: None,
                                position: sts2_engine::DrawPilePosition::Bottom,
                            }),
                            latch(true),
                        ],
                    }],
                }],
            )
            .when(HookCondition::OwnersCardExhausted),
            ModifierHook::every_turn(HookEvent::AfterCombatEnd, vec![latch(false)]),
        ],
        ..obtained_only("RELIC.BURNING_STICKS", RelicRarity::Shop)
    }
}

/// A ghost seed makes every starter strike and defend of a fight ethereal, both
/// the ones the fight opens with and any that join it later.
fn ghost_seed() -> RelicDefinition {
    let make_ethereal = |selector: sts2_engine::CardSelector| {
        Command::Card(sts2_engine::CardCmd::ApplyKeyword {
            cards: selector,
            keyword: sts2_engine::CardKeyword::Ethereal,
        })
    };
    let whole_combat = |tag: sts2_engine::CardTag| {
        make_ethereal(sts2_engine::CardSelector::AllCombatCards {
            filter: sts2_engine::CardFilter::BasicOfTag(tag),
        })
    };
    RelicDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterRoomEntered,
                vec![
                    whole_combat(sts2_engine::CardTag::Strike),
                    whole_combat(sts2_engine::CardTag::Defend),
                ],
            )
            .when(HookCondition::InACombatRoom),
            ModifierHook::every_turn(
                HookEvent::AfterCardEnteredCombat,
                vec![
                    whole_combat(sts2_engine::CardTag::Strike),
                    whole_combat(sts2_engine::CardTag::Defend),
                ],
            ),
        ],
        ..obtained_only("RELIC.GHOST_SEED", RelicRarity::Shop)
    }
}

/// A lava lamp upgrades every card of a fight's reward, where its owner walked
/// out of that fight without a blow having cost them anything.
fn lava_lamp() -> RelicDefinition {
    RelicDefinition {
        hooks: vec![
            ModifierHook::every_turn(HookEvent::AfterRoomEntered, vec![latch(false)]),
            ModifierHook::every_turn(
                HookEvent::AfterDamageReceived,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::All(vec![
                        Condition::CurrentRoomIsCombat,
                        Condition::Not(Box::new(Condition::DamageWasUnblockable)),
                    ]),
                    then: vec![latch(true)],
                }],
            )
            .when(HookCondition::OwnerTookUnblockedDamage),
            ModifierHook::every_turn(
                HookEvent::TryModifyCardRewardOptionsLate,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::All(vec![
                        Condition::CurrentRoomIsCombat,
                        Condition::Not(Box::new(Condition::ListeningRelicIsLatched)),
                    ]),
                    then: vec![Command::Rewards(
                        sts2_engine::RewardsCmd::UpgradeCardOptions(
                            sts2_engine::CardFilter::Upgradable,
                        ),
                    )],
                }],
            ),
        ],
        latch_property: Some("TookDamageThisCombat".to_owned()),
        ..obtained_only("RELIC.LAVA_LAMP", RelicRarity::Shop)
    }
}

/// The rare relics of the shared pool, and the Ironclad's own three.
///
/// The twelve rares that belong to the Silent, the Defect, the Necrobinder, and
/// the Regent land with those characters and are not registered here.
#[must_use]
#[allow(clippy::too_many_lines, reason = "one entry per relic of the rarity")]
fn rare_relics() -> Vec<RelicDefinition> {
    vec![
        // Bellows upgrades the whole opening hand.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterPlayerTurnStart,
                    vec![Command::Card(sts2_engine::CardCmd::Upgrade {
                        cards: sts2_engine::CardSelector::Pile {
                            pile: sts2_engine::PileName::Hand,
                            filter: sts2_engine::CardFilter::Any,
                        },
                    })],
                )
                .timed(HookTiming::FirstTurnOnly),
            ],
            ..obtained_only("RELIC.BELLOWS", RelicRarity::Rare)
        },
        beating_remnant(),
        // A captain's wheel braces the third turn as its block is cleared.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterBlockCleared,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::PlayerTurnIs(3),
                    then: vec![own_block(18)],
                }],
            )],
            ..obtained_only("RELIC.CAPTAINS_WHEEL", RelicRarity::Rare)
        },
        // Charon's ashes cut every enemy for every card its owner burns.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardExhausted,
                    vec![Command::Creature(CreatureCmd::Damage {
                        target: CreatureSelector::AllOpponents,
                        amount: Amount::Fixed(3),
                        props: ValueProps::UNPOWERED,
                    })],
                )
                .when(HookCondition::OwnersCardExhausted),
            ],
            ..obtained_only("RELIC.CHARONS_ASHES", RelicRarity::Rare)
        },
        // A chandelier pays three energy as the third turn opens.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::PlayerTurnIs(3),
                        then: vec![gain_energy(3)],
                    }],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..obtained_only("RELIC.CHANDELIER", RelicRarity::Rare)
        },
        // A cloak clasp pays a block for every card still held as the turn ends.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::BeforeSideTurnEnd,
                    vec![Command::Creature(CreatureCmd::GainBlock {
                        target: CreatureSelector::Owner,
                        amount: Amount::counting(sts2_engine::Count::CardsInPile(
                            sts2_engine::PileName::Hand,
                        )),
                        props: ValueProps::UNPOWERED,
                    })],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..obtained_only("RELIC.CLOAK_CLASP", RelicRarity::Rare)
        },
        demon_tongue(),
        // A gambling chip lays the opening hand out to be thrown away, and
        // deals back as many as were thrown.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterPlayerTurnStart,
                    vec![Command::CardSelect(
                        sts2_engine::CardSelectCmd::FromHandForDiscard {
                            semantics: ChoiceSemantics::known(vec![
                                ChoiceOp::new(ChoiceOperationKind::Discard, ChoiceScope::Combat)
                                    .to(ChoiceDestination::Discard),
                            ]),
                            maximum: None,
                            exact_count: None,
                            prefs: sts2_engine::SelectorPrefs::between(0, 999_999_999),
                            filter: sts2_engine::CardFilter::Any,
                            then: vec![Command::Card(sts2_engine::CardCmd::DiscardAndDraw {
                                cards: sts2_engine::CardSelector::Chosen,
                                count: Amount::counting(sts2_engine::Count::ChosenCards),
                            })],
                        },
                    )],
                )
                .timed(HookTiming::FirstTurnOnly),
            ],
            ..obtained_only("RELIC.GAMBLING_CHIP", RelicRarity::Rare)
        },
        // A game piece deals a card for every power its owner plays.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(HookEvent::AfterCardPlayed, vec![draw(1)]).when(
                    HookCondition::PlayedCardIsOfType(sts2_engine::CardType::Power),
                ),
            ],
            ..obtained_only("RELIC.GAME_PIECE", RelicRarity::Rare)
        },
        girya(),
        // An ice cream keeps whatever energy a turn did not spend: the refill
        // adds to what is standing instead of replacing it, on every turn but
        // the first.
        RelicDefinition {
            keeps_energy_after_first_turn: true,
            ..obtained_only("RELIC.ICE_CREAM", RelicRarity::Rare)
        },
        // An intimidating helmet pays four block for every card its owner
        // spends two or more energy on.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::BeforeCardPlayed,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::PlayedCardEnergyAtLeast(2),
                    then: vec![own_block(4)],
                }],
            )],
            ..obtained_only("RELIC.INTIMIDATING_HELMET", RelicRarity::Rare)
        },
        // A kunai pays a dexterity on every third attack of a turn.
        RelicDefinition {
            hooks: vec![
                counts_cards_each_turn(sts2_engine::CardType::Attack),
                third_card_of_the_turn(
                    sts2_engine::CardType::Attack,
                    own_power("POWER.DEXTERITY_POWER", Amount::Fixed(1)),
                ),
                clears_the_count(HookEvent::BeforeSideTurnStart),
                clears_the_count(HookEvent::AfterCombatEnd),
            ],
            ..obtained_only("RELIC.KUNAI", RelicRarity::Rare)
        },
        lizard_tail(),
        // A mango is fourteen more maximum HP the moment it is taken.
        taken_effect(
            "RELIC.MANGO",
            RelicRarity::Rare,
            vec![Command::Creature(CreatureCmd::GainMaxHp {
                target: CreatureSelector::Owner,
                amount: Amount::Fixed(14),
            })],
        ),
        // Meat on the bone heals twelve as a fight is won, if its owner walks
        // away from it at half their maximum HP or less.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterCombatVictoryEarly,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::Not(Box::new(Condition::OwnerHpAboveFraction(50))),
                    then: vec![heal(Amount::Fixed(12))],
                }],
            )],
            ..obtained_only("RELIC.MEAT_ON_THE_BONE", RelicRarity::Rare)
        },
        mummified_hand(),
        // An old coin is three hundred gold the moment it is taken.
        never_in_shops(stops_late(taken_effect(
            "RELIC.OLD_COIN",
            RelicRarity::Rare,
            vec![Command::Player(PlayerCmd::GainGold(Amount::Fixed(300)))],
        ))),
        // A prayer wheel puts a second card reward on every monster room.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::TryModifyRewards,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::CurrentRoomIs(sts2_engine::ResolvedRoom::Monster),
                    then: vec![Command::Rewards(sts2_engine::RewardsCmd::AddCardOffer {
                        room: sts2_engine::ResolvedRoom::Monster,
                        count: 3,
                    })],
                }],
            )],
            ..obtained_only("RELIC.PRAYER_WHEEL", RelicRarity::Rare)
        },
        rainbow_ring(),
        // A razor tooth upgrades every attack and skill its owner plays —
        // those two types and nothing else — and an upgrade passes over a card
        // that has no upgraded form.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardPlayed,
                    vec![Command::Card(sts2_engine::CardCmd::Upgrade {
                        cards: sts2_engine::CardSelector::PlayedCard,
                    })],
                )
                .when(HookCondition::PlayedCardIsOfType(
                    sts2_engine::CardType::Attack,
                )),
                ModifierHook::every_turn(
                    HookEvent::AfterCardPlayed,
                    vec![Command::Card(sts2_engine::CardCmd::Upgrade {
                        cards: sts2_engine::CardSelector::PlayedCard,
                    })],
                )
                .when(HookCondition::PlayedCardIsOfType(
                    sts2_engine::CardType::Skill,
                )),
            ],
            ..obtained_only("RELIC.RAZOR_TOOTH", RelicRarity::Rare)
        },
        pocketwatch(),
        ruined_helmet(),
        // A shovel puts a dig option on the rest site, which pulls a relic off
        // the front of its owner's own bag.
        stops_late(RelicDefinition {
            adds_rest_site_option: Some(RestSiteOffer::of(RestSiteOption::Dig)),
            ..obtained_only("RELIC.SHOVEL", RelicRarity::Rare)
        }),
        // A shuriken pays a strength on every third attack of a turn.
        RelicDefinition {
            hooks: vec![
                counts_cards_each_turn(sts2_engine::CardType::Attack),
                third_card_of_the_turn(
                    sts2_engine::CardType::Attack,
                    own_power("POWER.STRENGTH_POWER", Amount::Fixed(1)),
                ),
                clears_the_count(HookEvent::BeforeSideTurnStart),
                clears_the_count(HookEvent::AfterCombatEnd),
            ],
            ..obtained_only("RELIC.SHURIKEN", RelicRarity::Rare)
        },
        // A sturdy clamp keeps ten of its owner's block across the turn.
        RelicDefinition {
            keeps_block_up_to: Some(10),
            ..obtained_only("RELIC.STURDY_CLAMP", RelicRarity::Rare)
        },
        // The Courier takes a fifth off every merchant price and restocks each
        // shelf slot as it is bought from. Its body reads `originalPrice * (1 -
        // Discount / 100)`, which is the other way round from a membership
        // card's; a shelf held by both is worth two fifths.
        never_in_shops(RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::ModifyMerchantPrice,
                vec![Command::Modify(ModifierCmd::Scale(Amount::Fixed(80)))],
            )],
            restocks_merchant_entries: true,
            ..obtained_only("RELIC.THE_COURIER", RelicRarity::Rare)
        }),
        // An unceasing top deals a card whenever its owner's hand runs out.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterHandEmptied,
                vec![draw(1)],
            )],
            ..obtained_only("RELIC.UNCEASING_TOP", RelicRarity::Rare)
        },
        // An unsettling lamp doubles every lasting debuff the first card of a
        // fight to work one lands. Which card that is is bound in the pass that
        // runs before a power's amount is settled — the one pass about an
        // application that carries the card it came from — and the doubling
        // itself is what the giver's own multiplicative pass says. The binding
        // is spent as that card's play finishes, and the fight clears both.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::BeforeCombatStart,
                    vec![bind_card(false), latch(false)],
                ),
                ModifierHook::every_turn(
                    HookEvent::BeforePowerAmountChanged,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::All(vec![
                            Condition::ListeningRelicHasNoBoundCard,
                            Condition::Not(Box::new(Condition::ListeningRelicIsLatched)),
                            Condition::TheRelicsOwnerIsWorkingADebuffFromACard,
                        ]),
                        then: vec![bind_card(true)],
                    }],
                ),
                ModifierHook::every_turn(
                    HookEvent::ModifyPowerAmountGiven,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::All(vec![
                            Condition::AnnouncedCardIsTheRelicsBoundCard,
                            Condition::Not(Box::new(Condition::ListeningRelicIsLatched)),
                            // A temporary power the lamp has already doubled
                            // lends its owner a stat of its own, and doubling
                            // that as well would come out four times over.
                            Condition::Not(Box::new(
                                Condition::TheAnnouncedPowerIsLentByTheRelicsBoundPower,
                            )),
                            Condition::TheRelicsOwnerIsWorkingADebuffFromACard,
                        ]),
                        then: vec![Command::Modify(ModifierCmd::Scale(Amount::Fixed(200)))],
                    }],
                ),
                ModifierHook::every_turn(
                    HookEvent::AfterCardPlayed,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::All(vec![
                            Condition::PlayedCardIsTheRelicsBoundCard,
                            Condition::Not(Box::new(Condition::ListeningRelicIsLatched)),
                        ]),
                        then: vec![latch(true)],
                    }],
                ),
                ModifierHook::every_turn(
                    HookEvent::AfterCombatEnd,
                    vec![bind_card(false), latch(false)],
                ),
            ],
            ..obtained_only("RELIC.UNSETTLING_LAMP", RelicRarity::Rare)
        },
        // A tungsten rod takes one off every blow that costs its owner HP, and
        // never below nothing.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::ModifyHpLostAfterOsty,
                    vec![Command::Modify(ModifierCmd::Set(Amount::Greatest(vec![
                        Amount::Fixed(0),
                        Amount::Sum(vec![
                            Amount::counting(sts2_engine::Count::ModifiedValue),
                            Amount::Fixed(-1),
                        ]),
                    ])))],
                )
                .when(HookCondition::OwnerTakesDamage),
            ],
            ..obtained_only("RELIC.TUNGSTEN_ROD", RelicRarity::Rare)
        },
        // A vexing puzzlebox conjures one of its owner's own cards into the
        // opening hand, free for the turn.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterPlayerTurnStart,
                    vec![Command::CardPile(CardPileCmd::AddGeneratedFromPool {
                        pool: sts2_engine::CardPoolRef::OwnCharacter,
                        count: Amount::Fixed(1),
                        distinct: true,
                        filter: sts2_engine::CardFilter::Any,
                        pile: sts2_engine::PileName::Hand,
                        position: sts2_engine::DrawPilePosition::Bottom,
                        excludes_source: false,
                        upgraded: false,
                        temporary_cost: Some(sts2_engine::TemporaryCost::free_this_turn()),
                        ethereal: false,
                        rolled_one_at_a_time: false,
                    })],
                )
                .timed(HookTiming::FirstTurnOnly),
            ],
            ..obtained_only("RELIC.VEXING_PUZZLEBOX", RelicRarity::Rare)
        },
        // A white beast statue makes every fight pay a potion.
        stops_late(RelicDefinition {
            forces_potion_reward_in_combat_rooms: true,
            ..obtained_only("RELIC.WHITE_BEAST_STATUE", RelicRarity::Rare)
        }),
        // A white star puts a boss's card reward on every elite.
        stops_late(RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::TryModifyRewards,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::CurrentRoomIs(sts2_engine::ResolvedRoom::Elite),
                    then: vec![Command::Rewards(sts2_engine::RewardsCmd::AddCardOffer {
                        room: sts2_engine::ResolvedRoom::Boss,
                        count: 3,
                    })],
                }],
            )],
            ..obtained_only("RELIC.WHITE_STAR", RelicRarity::Rare)
        }),
        egg("RELIC.FROZEN_EGG", sts2_engine::CardType::Power, false),
        egg("RELIC.MOLTEN_EGG", sts2_engine::CardType::Attack, true),
        egg("RELIC.TOXIC_EGG", sts2_engine::CardType::Skill, false),
    ]
}

/// An egg upgrades every card of its type: on a card reward as it is built, on
/// a merchant's shelf as it is stocked, and on a card joining the deck.
///
/// `only_unupgraded` is Molten Egg's extra refusal, which it makes on the deck
/// path alone. The reward path also refuses a card that was made asking for no
/// upgrades, which nothing at this baseline ever asks for.
fn egg(model_id: &str, card_type: sts2_engine::CardType, only_unupgraded: bool) -> RelicDefinition {
    stops_late(RelicDefinition {
        upgrades_cards_of_type: Some((card_type, only_unupgraded)),
        hooks: vec![ModifierHook::every_turn(
            HookEvent::TryModifyCardRewardOptionsLate,
            vec![Command::Rewards(
                sts2_engine::RewardsCmd::UpgradeCardOptions(
                    sts2_engine::CardFilter::UpgradableOfType(card_type),
                ),
            )],
        )],
        ..obtained_only(model_id, RelicRarity::Rare)
    })
}

/// A beating remnant holds every turn's HP loss down to twenty in total.
///
/// The build remembers what the turn has cost so far and answers the HP-loss
/// pass with what is left of the twenty. It says why it is that pass rather
/// than the damage cap: a blow trampling through the pet has already been past
/// the damage passes once.
fn beating_remnant() -> RelicDefinition {
    RelicDefinition {
        // What the turn has cost so far is a number the relic keeps to itself:
        // it is not one of the properties a relic writes down, so nothing that
        // reads a relic's saved properties ever sees it.
        counter_property: None,
        counter_kind: None,
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::ModifyHpLostAfterOsty,
                vec![Command::Modify(ModifierCmd::CapAt(Amount::Sum(vec![
                    Amount::Fixed(20),
                    Amount::Product {
                        left: Box::new(Amount::Fixed(-1)),
                        right: Box::new(Amount::counting(
                            sts2_engine::Count::ListeningRelicCounter,
                        )),
                    },
                ])))],
            )
            .when(HookCondition::OwnerTakesDamage),
            // The counter takes the unblocked part of each blow, which adds
            // nothing for a blow block stopped whole.
            ModifierHook::every_turn(
                HookEvent::AfterDamageReceived,
                vec![Command::Relic(RelicCmd::ModifyListeningCounter(
                    Amount::counting(sts2_engine::Count::UnblockedDamage),
                ))],
            )
            .when(HookCondition::OwnerTookUnblockedDamage),
            ModifierHook::every_turn(
                HookEvent::BeforeSideTurnStart,
                vec![Command::Relic(RelicCmd::SetListeningCounter(
                    Amount::Fixed(0),
                ))],
            )
            .when(HookCondition::OwnerIsParticipant),
        ],
        ..obtained_only("RELIC.BEATING_REMNANT", RelicRarity::Rare)
    }
}

/// A demon tongue heals the first blow of each of its owner's own turns that
/// block did not stop.
fn demon_tongue() -> RelicDefinition {
    RelicDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterDamageReceived,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::Not(Box::new(Condition::ListeningRelicIsLatched)),
                    then: vec![
                        latch(true),
                        heal(Amount::counting(sts2_engine::Count::UnblockedDamage)),
                    ],
                }],
            )
            .when(HookCondition::OwnerTookUnblockedDamageOnOwnSideTurn),
            ModifierHook::every_turn(HookEvent::BeforeSideTurnStart, vec![latch(false)])
                .when(HookCondition::OwnerIsParticipant),
        ],
        ..obtained_only("RELIC.DEMON_TONGUE", RelicRarity::Rare)
    }
}

/// A girya works in a strength for every time it has been lifted as a fight is
/// walked into, and puts a lift option on the rest site until it has been
/// lifted its three times.
fn girya() -> RelicDefinition {
    stops_late(RelicDefinition {
        counter_property: Some("TimesLifted".to_owned()),
        counter_kind: Some(sts2_engine::CounterKind::Ledger),
        adds_rest_site_option: Some(RestSiteOffer::of(RestSiteOption::Lift).while_counter_below(3)),
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterRoomEntered,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::ListeningRelicCounterAtLeast(1),
                    then: vec![own_power(
                        "POWER.STRENGTH_POWER",
                        Amount::counting(sts2_engine::Count::ListeningRelicCounter),
                    )],
                }],
            )
            .when(HookCondition::InACombatRoom),
        ],
        ..obtained_only("RELIC.GIRYA", RelicRarity::Rare)
    })
}

/// A lizard tail holds its owner up the once and heals them half their maximum
/// HP. Turning a death away is not a surface this baseline has, so it carries
/// the spent flag and nothing else.
fn lizard_tail() -> RelicDefinition {
    RelicDefinition {
        used_up: Some(sts2_engine::UsedUp::Latched),
        // Half the maximum, never less than one, and the heal truncates what
        // it lands on, so half of an odd maximum is the lower half.
        prevents_owners_death_late: vec![
            Command::Relic(RelicCmd::SetListeningLatch(true)),
            Command::Creature(CreatureCmd::Heal {
                target: CreatureSelector::Owner,
                amount: Amount::Greatest(vec![
                    Amount::Fixed(1),
                    Amount::Percent {
                        percent: 50,
                        count: sts2_engine::Count::OwnerMaxHp,
                    },
                ]),
            }),
        ],
        latch_property: Some("WasUsed".to_owned()),
        ..obtained_only("RELIC.LIZARD_TAIL", RelicRarity::Rare)
    }
}

/// A mummified hand makes one card of the hand free for the turn every time its
/// owner plays a power.
///
/// The build rolls over four lists in turn and takes the first that has
/// anything in it: the held cards that cost something both as written and as
/// they stand, then those that cost something as they stand, then those that
/// cost something as written, then the whole hand. An empty list costs no
/// randomness. A card the hand already made free this turn costs nothing as
/// it stands, so a second power frees a different card while one is left that
/// still costs something.
fn mummified_hand() -> RelicDefinition {
    let free_this_turn = || {
        Command::Card(sts2_engine::CardCmd::SetTemporaryCost {
            cards: sts2_engine::CardSelector::Chosen,
            lifetime: sts2_engine::TemporaryCost::free_this_turn(),
        })
    };
    RelicDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterCardPlayed,
                vec![Command::Select {
                    cards: sts2_engine::CardSelector::RandomFromPile {
                        pile: sts2_engine::PileName::Hand,
                        filter: sts2_engine::CardFilter::CostsAnythingAsWrittenAndNow,
                        fallback: Some(sts2_engine::CardFilter::CostsAnythingNow),
                    },
                    then: vec![
                        Command::When {
                            otherwise: Vec::new(),
                            condition: Condition::Not(Box::new(Condition::NoCardWasChosen)),
                            then: vec![free_this_turn()],
                        },
                        Command::When {
                            otherwise: Vec::new(),
                            condition: Condition::NoCardWasChosen,
                            then: vec![Command::Select {
                                cards: sts2_engine::CardSelector::RandomFromPile {
                                    pile: sts2_engine::PileName::Hand,
                                    filter: sts2_engine::CardFilter::CostsAnythingAsWritten,
                                    fallback: Some(sts2_engine::CardFilter::Any),
                                },
                                then: vec![free_this_turn()],
                            }],
                        },
                    ],
                }],
            )
            .when(HookCondition::PlayedCardIsOfType(
                sts2_engine::CardType::Power,
            )),
        ],
        ..obtained_only("RELIC.MUMMIFIED_HAND", RelicRarity::Rare)
    }
}

/// A pocketwatch swells the hand by three on every turn that follows one where
/// its owner played three cards or fewer.
///
/// The build remembers this turn's count and last turn's; the relic's two
/// counters are those, and the turn's opening moves one into the other.
fn pocketwatch() -> RelicDefinition {
    RelicDefinition {
        // Neither count is ever saved: they live for the fight alone. They
        // drive the relic but do not appear in a run projection.
        counter_property: None,
        counter_kind: None,
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterCardPlayed,
                vec![Command::Relic(RelicCmd::ModifyListeningCounter(
                    Amount::Fixed(1),
                ))],
            ),
            ModifierHook::every_turn(
                HookEvent::BeforeSideTurnStart,
                vec![
                    Command::Relic(RelicCmd::SetListeningSecondCounter(Amount::counting(
                        sts2_engine::Count::ListeningRelicCounter,
                    ))),
                    Command::Relic(RelicCmd::SetListeningCounter(Amount::Fixed(0))),
                ],
            )
            .when(HookCondition::OwnerIsParticipant),
            ModifierHook::every_turn(
                HookEvent::ModifyHandDraw,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::Not(Box::new(
                        Condition::ListeningRelicSecondCounterAtLeast(4),
                    )),
                    then: vec![Command::Modify(ModifierCmd::Add(Amount::Fixed(3)))],
                }],
            )
            .timed(HookTiming::AfterFirstTurn),
            ModifierHook::every_turn(
                HookEvent::AfterCombatEnd,
                vec![
                    Command::Relic(RelicCmd::SetListeningCounter(Amount::Fixed(0))),
                    Command::Relic(RelicCmd::SetListeningSecondCounter(Amount::Fixed(0))),
                ],
            ),
        ],
        ..obtained_only("RELIC.POCKETWATCH", RelicRarity::Rare)
    }
}

/// A ruined helmet doubles the first strength its owner gains in a fight.
///
/// Only a gain counts: a strength loss passes through untouched and does not
/// spend the helmet, so the first gain after a drain is still the one doubled.
/// The build remembers, for the fight alone, whether it has fired, and forgets
/// it as the fight ends.
fn ruined_helmet() -> RelicDefinition {
    RelicDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::ModifyPowerAmountReceived,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::All(vec![
                        Condition::Not(Box::new(Condition::ListeningRelicIsLatched)),
                        Condition::ModifiedValueAtLeast(1),
                    ]),
                    then: vec![Command::Modify(ModifierCmd::Scale(Amount::Fixed(200)))],
                }],
            )
            .when(HookCondition::PowerAppliedToOwnerIs(id(
                "POWER.STRENGTH_POWER",
            ))),
            ModifierHook::every_turn(
                HookEvent::AfterModifyingPowerAmountReceived,
                vec![latch(true)],
            ),
            ModifierHook::every_turn(HookEvent::AfterCombatEnd, vec![latch(false)]),
        ],
        ..obtained_only("RELIC.RUINED_HELMET", RelicRarity::Rare)
    }
}

/// A rainbow ring pays a strength and a dexterity the first time in a turn that
/// its owner has played an attack, a skill, and a power.
///
/// The build remembers three counts and a fourth for having fired. They are
/// one number here: a bit apiece for the three types, and the fourth bit for
/// the firing, all cleared as the turn opens and as the fight ends.
fn rainbow_ring() -> RelicDefinition {
    const ATTACK: i32 = 1;
    const SKILL: i32 = 2;
    const POWER: i32 = 4;
    const FIRED: i32 = 8;
    let mark = |bit: i32, card_type: sts2_engine::CardType| {
        ModifierHook::every_turn(
            HookEvent::AfterCardPlayed,
            vec![Command::When {
                otherwise: Vec::new(),
                condition: Condition::Not(Box::new(Condition::ListeningRelicCounterHasBit(FIRED))),
                then: vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::Not(Box::new(Condition::ListeningRelicCounterHasBit(
                        bit,
                    ))),
                    then: vec![Command::Relic(RelicCmd::ModifyListeningCounter(
                        Amount::Fixed(bit),
                    ))],
                }],
            }],
        )
        .when(HookCondition::PlayedCardIsOfType(card_type))
    };
    let clear = |event: HookEvent| {
        ModifierHook::every_turn(
            event,
            vec![Command::Relic(RelicCmd::SetListeningCounter(
                Amount::Fixed(0),
            ))],
        )
    };
    RelicDefinition {
        hooks: vec![
            mark(ATTACK, sts2_engine::CardType::Attack),
            mark(SKILL, sts2_engine::CardType::Skill),
            mark(POWER, sts2_engine::CardType::Power),
            ModifierHook::every_turn(
                HookEvent::AfterCardPlayed,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::All(vec![
                        Condition::Not(Box::new(Condition::ListeningRelicCounterHasBit(FIRED))),
                        Condition::ListeningRelicCounterHasBit(ATTACK),
                        Condition::ListeningRelicCounterHasBit(SKILL),
                        Condition::ListeningRelicCounterHasBit(POWER),
                    ]),
                    then: vec![
                        own_power("POWER.STRENGTH_POWER", Amount::Fixed(1)),
                        own_power("POWER.DEXTERITY_POWER", Amount::Fixed(1)),
                        Command::Relic(RelicCmd::ModifyListeningCounter(Amount::Fixed(FIRED))),
                    ],
                }],
            ),
            clear(HookEvent::BeforeSideTurnStart).when(HookCondition::OwnerIsParticipant),
            clear(HookEvent::AfterCombatEnd),
        ],
        ..obtained_only("RELIC.RAINBOW_RING", RelicRarity::Rare)
    }
}

/// A lasting candy puts a fourth card, a power, on every other fight's card
/// reward.
///
/// Its count of fights seen is wound on as each fight ends, and the fight whose
/// count came out even is the one whose reward is swelled — which is the fight
/// after the one that started the count, and every second one after that.
///
/// The relic is also refused to a profile that has never finished a run. This
/// baseline is pinned to profiles with runs behind them and refuses one without
/// them outright, so that clause is written down as the act-three cutoff
/// alone.
fn lasting_candy() -> RelicDefinition {
    stops_late(RelicDefinition {
        counter_property: Some("CombatsSeen".to_owned()),
        counter_kind: Some(sts2_engine::CounterKind::Cycle { wraps_at: 2 }),
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterCombatEnd,
                vec![Command::Relic(RelicCmd::ModifyListeningCounter(
                    Amount::Fixed(1),
                ))],
            ),
            ModifierHook::every_turn(
                HookEvent::TryModifyCardRewardOptions,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::All(vec![
                        Condition::CardOfferCameFromAnEncounter,
                        Condition::ListeningRelicCounterAtLeast(1),
                        Condition::ListeningRelicCounterMultipleOf(2),
                    ]),
                    then: vec![Command::Rewards(sts2_engine::RewardsCmd::AddCardOption {
                        card_type: sts2_engine::CardType::Power,
                    })],
                }],
            ),
        ],
        ..obtained_only("RELIC.LASTING_CANDY", RelicRarity::Uncommon)
    })
}

/// A joss paper deals a card for every five cards its owner burns. A card that
/// burned up in hand at the end of a turn is held back and counted once the
/// turn has finished, so the draw it pays for lands after the flush.
fn joss_paper() -> RelicDefinition {
    let draw_if_threshold_met = || Command::When {
        otherwise: Vec::new(),
        condition: Condition::ListeningRelicCounterAtLeast(5),
        then: vec![
            Command::CardPile(CardPileCmd::Draw {
                count: Amount::Percent {
                    percent: 20,
                    count: sts2_engine::Count::ListeningRelicCounter,
                },
            }),
            // The count wraps at five, said as what is left after the fives are
            // taken out.
            Command::Relic(RelicCmd::SetListeningCounter(Amount::Sum(vec![
                Amount::counting(sts2_engine::Count::ListeningRelicCounter),
                Amount::Product {
                    left: Box::new(Amount::Fixed(-5)),
                    right: Box::new(Amount::Percent {
                        percent: 20,
                        count: sts2_engine::Count::ListeningRelicCounter,
                    }),
                },
            ]))),
        ],
    };
    RelicDefinition {
        counter_property: Some("CardsExhausted".to_owned()),
        counter_kind: Some(sts2_engine::CounterKind::Cycle { wraps_at: 5 }),
        // The one counter of the build that is saved only once it is set: it
        // is written down only when it is something, and it goes back to
        // nothing every fifth card.
        counter_saved_only_when_set: true,
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterCardExhausted,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::ExhaustCausedByEthereal,
                    then: vec![Command::Relic(RelicCmd::ModifyListeningSecondCounter(
                        Amount::Fixed(1),
                    ))],
                }],
            )
            .when(HookCondition::OwnersCardExhausted),
            ModifierHook::every_turn(
                HookEvent::AfterCardExhausted,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::Not(Box::new(Condition::ExhaustCausedByEthereal)),
                    then: vec![
                        Command::Relic(RelicCmd::ModifyListeningCounter(Amount::Fixed(1))),
                        draw_if_threshold_met(),
                    ],
                }],
            )
            .when(HookCondition::OwnersCardExhausted),
            ModifierHook::every_turn(
                HookEvent::AfterSideTurnEnd,
                vec![
                    Command::Relic(RelicCmd::ModifyListeningCounter(Amount::counting(
                        sts2_engine::Count::ListeningRelicSecondCounter,
                    ))),
                    Command::Relic(RelicCmd::SetListeningSecondCounter(Amount::Fixed(0))),
                    draw_if_threshold_met(),
                ],
            )
            .when(HookCondition::OwnerIsParticipant),
            ModifierHook::every_turn(
                HookEvent::AfterCombatEnd,
                vec![Command::Relic(RelicCmd::SetListeningSecondCounter(
                    Amount::Fixed(0),
                ))],
            ),
        ],
        ..obtained_only("RELIC.JOSS_PAPER", RelicRarity::Uncommon)
    }
}

/// A kusarigama cuts one enemy it rolls for on every third attack of a turn.
fn kusarigama() -> RelicDefinition {
    RelicDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::BeforeCombatStart,
                vec![Command::Relic(RelicCmd::SetListeningCounter(
                    Amount::Fixed(0),
                ))],
            ),
            ModifierHook::every_turn(
                HookEvent::AfterSideTurnEnd,
                vec![Command::Relic(RelicCmd::SetListeningCounter(
                    Amount::Fixed(0),
                ))],
            )
            .when(HookCondition::OwnerIsParticipant),
            ModifierHook::every_turn(
                HookEvent::AfterCardPlayed,
                vec![
                    Command::Relic(RelicCmd::ModifyListeningCounter(Amount::Fixed(1))),
                    Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningRelicCounterMultipleOf(3),
                        then: vec![Command::Creature(CreatureCmd::Damage {
                            target: CreatureSelector::RandomOpponent,
                            amount: Amount::Fixed(6),
                            props: ValueProps::UNPOWERED,
                        })],
                    },
                ],
            )
            .when(HookCondition::PlayedCardIsAttack),
        ],
        ..obtained_only("RELIC.KUSARIGAMA", RelicRarity::Uncommon)
    }
}

/// An orichalcum pays six block at the end of a turn its owner finished behind
/// nothing.
///
/// The check is made on the very early pass, before a plating pays out, and the
/// block is worked up on the base one, which is why it is two bodies and a
/// latch rather than one test.
fn orichalcum() -> RelicDefinition {
    RelicDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::BeforeSideTurnEndVeryEarly,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::Not(Box::new(Condition::OwnerBlockAtLeast(1))),
                    then: vec![latch(true)],
                }],
            )
            .when(HookCondition::OwnerIsParticipant),
            ModifierHook::every_turn(
                HookEvent::BeforeSideTurnEnd,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::ListeningRelicIsLatched,
                    then: vec![latch(false), own_block(6)],
                }],
            ),
            ModifierHook::every_turn(HookEvent::BeforeSideTurnStart, vec![latch(false)])
                .when(HookCondition::OwnerIsParticipant),
        ],
        ..obtained_only("RELIC.ORICHALCUM", RelicRarity::Uncommon)
    }
}

/// A pen nib doubles every tenth attack its owner plays.
fn pen_nib() -> RelicDefinition {
    RelicDefinition {
        counter_property: Some("AttacksPlayed".to_owned()),
        counter_kind: Some(sts2_engine::CounterKind::Cycle { wraps_at: 10 }),
        hooks: vec![
            // The count is wound on before the card's own body runs, and the
            // tenth is the one that is doubled.
            ModifierHook::every_turn(
                HookEvent::BeforeCardPlayed,
                counts_to(10)
                    .into_iter()
                    .chain(std::iter::once(Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningRelicCounterIs(0),
                        then: vec![latch(true)],
                    }))
                    .collect(),
            )
            .when(HookCondition::PlayedCardIsOfType(
                sts2_engine::CardType::Attack,
            )),
            ModifierHook::every_turn(
                HookEvent::ModifyDamageMultiplicative,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::All(vec![
                        Condition::ListeningRelicIsLatched,
                        Condition::DamageCameFromACard,
                    ]),
                    then: vec![Command::Modify(ModifierCmd::Scale(Amount::Fixed(200)))],
                }],
            )
            // The blow is its owner's or their pet's: a card that swings the
            // pet is doubled like any other attack.
            .when(HookCondition::Not(Box::new(HookCondition::All(vec![
                HookCondition::Not(Box::new(HookCondition::OwnerDealsPoweredDamage)),
                HookCondition::Not(Box::new(HookCondition::OwnersPetDealsPoweredDamage)),
            ])))),
            ModifierHook::every_turn(
                HookEvent::AfterCardPlayed,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::ListeningRelicIsLatched,
                    then: vec![latch(false)],
                }],
            ),
        ],
        ..obtained_only("RELIC.PEN_NIB", RelicRarity::Uncommon)
    }
}

/// A tuning fork pays seven block on every tenth skill its owner plays, and the
/// count carries across turns and fights.
fn tuning_fork() -> RelicDefinition {
    RelicDefinition {
        counter_property: Some("SkillsPlayed".to_owned()),
        counter_kind: Some(sts2_engine::CounterKind::Cycle { wraps_at: 10 }),
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterCardPlayed,
                vec![
                    Command::Relic(RelicCmd::ModifyListeningCounter(Amount::Fixed(1))),
                    Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningRelicCounterAtLeast(10),
                        then: vec![
                            own_block(7),
                            Command::Relic(RelicCmd::ModifyListeningCounter(Amount::Fixed(-10))),
                        ],
                    },
                ],
            )
            .when(HookCondition::PlayedCardIsOfType(
                sts2_engine::CardType::Skill,
            )),
        ],
        ..obtained_only("RELIC.TUNING_FORK", RelicRarity::Uncommon)
    }
}

/// A vambrace doubles the first block a card of its owner's works up in a
/// fight, and is spent for the play that did it.
///
/// The build remembers the card that triggered it as well as that it has been
/// spent, so that a card working block up twice has both halves doubled; the
/// second count here is that same "it has fired during this play" mark, and the
/// play's end is where it is set.
fn vambrace() -> RelicDefinition {
    let reset = || {
        vec![
            latch(false),
            Command::Relic(RelicCmd::SetListeningSecondCounter(Amount::Fixed(0))),
        ]
    };
    RelicDefinition {
        hooks: vec![
            ModifierHook::every_turn(HookEvent::BeforeCombatStart, reset()),
            ModifierHook::every_turn(HookEvent::AfterCombatEnd, reset()),
            ModifierHook::every_turn(
                HookEvent::ModifyBlockMultiplicative,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::All(vec![
                        Condition::Not(Box::new(Condition::ListeningRelicIsLatched)),
                        Condition::DamageCameFromACard,
                    ]),
                    then: vec![Command::Modify(ModifierCmd::Scale(Amount::Fixed(200)))],
                }],
            )
            .when(HookCondition::OwnerGrantsPoweredBlock),
            // A play that worked no block up at all leaves nothing to be spent
            // for, so the mark is only made once there is block to show for it.
            ModifierHook::every_turn(
                HookEvent::AfterModifyingBlockAmount,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::CountAtLeast {
                        count: sts2_engine::Count::AnnouncedAmount,
                        at_least: 1,
                    },
                    then: vec![Command::Relic(RelicCmd::SetListeningSecondCounter(
                        Amount::Fixed(1),
                    ))],
                }],
            ),
            ModifierHook::every_turn(
                HookEvent::AfterCardPlayed,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::ListeningRelicSecondCounterIsSet,
                    then: vec![
                        latch(true),
                        Command::Relic(RelicCmd::SetListeningSecondCounter(Amount::Fixed(0))),
                    ],
                }],
            ),
        ],
        ..obtained_only("RELIC.VAMBRACE", RelicRarity::Uncommon)
    }
}

/// A red skull pays three strength while its owner is under half their maximum
/// HP, and takes it back when they are not.
///
/// The build remembers whether the strength is on, checks that as a combat
/// room is entered and every time its owner's HP moves, and clears it
/// when the fight ends.
fn red_skull() -> RelicDefinition {
    let check = || {
        vec![
            Command::When {
                otherwise: Vec::new(),
                condition: Condition::All(vec![
                    Condition::OwnerHpAboveFraction(50),
                    Condition::ListeningRelicIsLatched,
                ]),
                then: vec![
                    own_power("POWER.STRENGTH_POWER", Amount::Fixed(-3)),
                    latch(false),
                ],
            },
            Command::When {
                otherwise: Vec::new(),
                condition: Condition::All(vec![
                    Condition::Not(Box::new(Condition::OwnerHpAboveFraction(50))),
                    Condition::Not(Box::new(Condition::ListeningRelicIsLatched)),
                ]),
                then: vec![
                    own_power("POWER.STRENGTH_POWER", Amount::Fixed(3)),
                    latch(true),
                ],
            },
        ]
    };
    RelicDefinition {
        hooks: vec![
            ModifierHook::every_turn(HookEvent::AfterRoomEntered, check())
                .when(HookCondition::InACombatRoom),
            ModifierHook::every_turn(HookEvent::AfterCurrentHpChanged, check())
                .when(HookCondition::InACombat),
            ModifierHook::every_turn(HookEvent::AfterCombatEnd, vec![latch(false)]),
        ],
        ..obtained_only("RELIC.RED_SKULL", RelicRarity::Common)
    }
}

/// A war paint and a whetstone are the same relic over two card types: two of
/// the deck's upgradable cards of that type, stably shuffled on the niche
/// stream, upgraded the moment the relic is taken.
fn upgrades_two_of_the_deck(model_id: &str, card_type: sts2_engine::CardType) -> RelicDefinition {
    taken_effect(
        model_id,
        RelicRarity::Common,
        vec![Command::Card(sts2_engine::CardCmd::Upgrade {
            cards: sts2_engine::CardSelector::StablyShuffledFromDeck {
                filter: sts2_engine::CardFilter::UpgradableOfType(card_type),
                take: Amount::Fixed(2),
                stream: sts2_engine::ShuffleStream::Niche,
            },
        })],
    )
}

/// The relics that act as a combat opens or closes.
#[allow(clippy::too_many_lines, reason = "one entry per relic")]
fn combat_opening_relics() -> Vec<RelicDefinition> {
    vec![
        RelicDefinition {
            model_id: id("RELIC.BURNING_BLOOD"),
            // A starter rarity, which generation never rolls and no event
            // trades for.
            grants_an_extra_turn: None,
            map_rule: None,
            rarity: RelicRarity::Starter,
            counter_property: None,
            counter_kind: None,
            counter_saved_only_when_set: false,
            keeps_hand: false,
            keeps_first_hand: false,
            on_obtained: Vec::new(),
            setup: None,
            removes_unknown_room_types: Vec::new(),
            grants_free_travel: false,
            card_reward_alternative: None,
            has_upon_pickup_effect: false,
            spawns_pets: false,
            adds_pet: false,
            rolled_skin: None,
            is_stackable: false,
            allowed_in_shops: true,
            merchant_cost_override: None,
            modifies_x_value_by: 0,
            restocks_merchant_entries: false,
            adds_rest_site_option: None,
            keeps_rest_site_options_standing: false,
            keeps_energy_after_first_turn: false,
            keeps_block_up_to: None,
            forces_potion_reward_in_combat_rooms: false,
            refuses_potions: false,
            stops_draws_on_its_own_side_turn: false,
            card_cost_rule: None,
            cards_playable_each_turn: None,
            upgrades_cards_of_type: None,
            enchants_cards_it_meets: None,
            suppresses_treasure_while_second_counter_at_most: None,
            second_counter_property: None,
            latch_property: None,
            saved_card_array_property: None,
            prevents_owners_death_late: Vec::new(),
            adds_card_pool_to_rewards: None,
            adds_every_character_pool_to_rewards: false,
            initial_counter: 0,
            used_up: None,
            spawn_condition: None,
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterCombatVictory,
                vec![Command::Creature(CreatureCmd::Heal {
                    target: CreatureSelector::Owner,
                    amount: Amount::Fixed(6),
                })],
            )],
        },
        RelicDefinition {
            model_id: id("RELIC.RED_MASK"),
            grants_an_extra_turn: None,
            map_rule: None,
            rarity: RelicRarity::Common,
            counter_property: None,
            counter_kind: None,
            counter_saved_only_when_set: false,
            keeps_hand: false,
            keeps_first_hand: false,
            on_obtained: Vec::new(),
            setup: None,
            removes_unknown_room_types: Vec::new(),
            grants_free_travel: false,
            card_reward_alternative: None,
            has_upon_pickup_effect: false,
            spawns_pets: false,
            adds_pet: false,
            rolled_skin: None,
            is_stackable: false,
            allowed_in_shops: true,
            merchant_cost_override: None,
            modifies_x_value_by: 0,
            restocks_merchant_entries: false,
            adds_rest_site_option: None,
            keeps_rest_site_options_standing: false,
            keeps_energy_after_first_turn: false,
            keeps_block_up_to: None,
            forces_potion_reward_in_combat_rooms: false,
            refuses_potions: false,
            stops_draws_on_its_own_side_turn: false,
            card_cost_rule: None,
            cards_playable_each_turn: None,
            upgrades_cards_of_type: None,
            enchants_cards_it_meets: None,
            suppresses_treasure_while_second_counter_at_most: None,
            second_counter_property: None,
            latch_property: None,
            saved_card_array_property: None,
            prevents_owners_death_late: Vec::new(),
            adds_card_pool_to_rewards: None,
            adds_every_character_pool_to_rewards: false,
            initial_counter: 0,
            used_up: None,
            spawn_condition: None,
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::BeforeSideTurnStart,
                    vec![Command::Power(PowerCmd::Apply {
                        marks: None,
                        power: id("POWER.WEAK_POWER"),
                        target: CreatureSelector::AllOpponents,
                        applier: Some(CreatureSelector::Owner),
                        amount: Amount::Fixed(1),
                    })],
                )
                .when(HookCondition::PlayerSideTurn)
                .timed(HookTiming::FirstTurnOnly),
            ],
        },
        // A popper goes off as the first turn opens, on every enemy at once.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterPlayerTurnStart,
                    vec![Command::Creature(CreatureCmd::Damage {
                        target: CreatureSelector::AllOpponents,
                        amount: Amount::Fixed(9),
                        props: ValueProps::UNPOWERED,
                    })],
                )
                .timed(HookTiming::FirstTurnOnly),
            ],
            ..obtained_only("RELIC.FESTIVE_POPPER", RelicRarity::Common)
        },
        RelicDefinition {
            model_id: id("RELIC.LANTERN"),
            grants_an_extra_turn: None,
            map_rule: None,
            rarity: RelicRarity::Common,
            counter_property: None,
            counter_kind: None,
            counter_saved_only_when_set: false,
            keeps_hand: false,
            keeps_first_hand: false,
            on_obtained: Vec::new(),
            setup: None,
            removes_unknown_room_types: Vec::new(),
            grants_free_travel: false,
            card_reward_alternative: None,
            has_upon_pickup_effect: false,
            spawns_pets: false,
            adds_pet: false,
            rolled_skin: None,
            is_stackable: false,
            allowed_in_shops: true,
            merchant_cost_override: None,
            modifies_x_value_by: 0,
            restocks_merchant_entries: false,
            adds_rest_site_option: None,
            keeps_rest_site_options_standing: false,
            keeps_energy_after_first_turn: false,
            keeps_block_up_to: None,
            forces_potion_reward_in_combat_rooms: false,
            refuses_potions: false,
            stops_draws_on_its_own_side_turn: false,
            card_cost_rule: None,
            cards_playable_each_turn: None,
            upgrades_cards_of_type: None,
            enchants_cards_it_meets: None,
            suppresses_treasure_while_second_counter_at_most: None,
            second_counter_property: None,
            latch_property: None,
            saved_card_array_property: None,
            prevents_owners_death_late: Vec::new(),
            adds_card_pool_to_rewards: None,
            adds_every_character_pool_to_rewards: false,
            initial_counter: 0,
            used_up: None,
            spawn_condition: None,
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    vec![Command::Player(PlayerCmd::GainEnergy(Amount::Fixed(1)))],
                )
                .when(HookCondition::OwnerIsParticipant)
                .timed(HookTiming::FirstTurnOnly),
            ],
        },
    ]
}

/// Pael's Wing counts the card rewards its owner has thrown away, and pays a
/// relic for every second one.
///
/// The count it keeps is projected, so the relic carries it, and the relic the
/// second sacrifice pays comes off the front of the player's own bag.
fn paels_wing() -> RelicDefinition {
    RelicDefinition {
        counter_property: Some("RewardsSacrificed".to_owned()),
        counter_kind: Some(sts2_engine::CounterKind::Cycle { wraps_at: 2 }),
        card_reward_alternative: Some(sts2_engine::CardRewardAlternative::Sacrifice { every: 2 }),
        ..obtained_only("RELIC.PAELS_WING", RelicRarity::Ancient)
    }
}

/// Royal poison costs its owner four HP as every fight's first turn opens.
fn royal_poison() -> RelicDefinition {
    RelicDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterPlayerTurnStart,
                vec![Command::Creature(CreatureCmd::Damage {
                    target: CreatureSelector::Owner,
                    amount: Amount::Fixed(4),
                    props: ValueProps::UNBLOCKABLE,
                })],
            )
            .timed(HookTiming::FirstTurnOnly),
        ],
        ..obtained_only("RELIC.ROYAL_POISON", RelicRarity::Event)
    }
}

/// A looming fruit is thirty-one more maximum HP the moment it is taken.
fn looming_fruit() -> RelicDefinition {
    upon_pickup(RelicDefinition {
        on_obtained: vec![Command::Creature(CreatureCmd::GainMaxHp {
            target: CreatureSelector::Owner,
            amount: Amount::Fixed(31),
        })],
        ..obtained_only("RELIC.LOOMING_FRUIT", RelicRarity::Ancient)
    })
}

/// A gorget works its plating in as a fight is walked into.
fn gorget() -> RelicDefinition {
    RelicDefinition {
        hooks: vec![ModifierHook::every_turn(
            HookEvent::AfterRoomEntered,
            vec![Command::Power(PowerCmd::Apply {
                marks: None,
                power: id("POWER.PLATING_POWER"),
                target: CreatureSelector::Owner,
                applier: Some(CreatureSelector::Owner),
                amount: Amount::Fixed(4),
            })],
        )],
        ..obtained_only("RELIC.GORGET", RelicRarity::Common)
    }
}

/// A pendulum counts the turns and pays a card on every third.
fn pendulum() -> RelicDefinition {
    RelicDefinition {
        counter_property: Some("TurnsSeen".to_owned()),
        counter_kind: Some(sts2_engine::CounterKind::Cycle { wraps_at: 3 }),
        hooks: vec![ModifierHook::every_turn(
            HookEvent::AfterPlayerTurnStart,
            vec![
                // `TurnsSeen = (TurnsSeen + 1) % 3`, and the draw where it
                // comes back round to nothing.
                Command::Relic(RelicCmd::ModifyListeningCounter(Amount::Fixed(1))),
                Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::ListeningRelicCounterIs(3),
                    then: vec![Command::Relic(RelicCmd::SetListeningCounter(
                        Amount::Fixed(0),
                    ))],
                },
                Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::ListeningRelicCounterIs(0),
                    then: vec![Command::CardPile(CardPileCmd::Draw {
                        count: Amount::Fixed(1),
                    })],
                },
            ],
        )],
        ..obtained_only("RELIC.PENDULUM", RelicRarity::Common)
    }
}

/// A vajra works its strength in as a fight is walked into.
fn vajra() -> RelicDefinition {
    RelicDefinition {
        hooks: vec![ModifierHook::every_turn(
            HookEvent::AfterRoomEntered,
            vec![Command::Power(PowerCmd::Apply {
                marks: None,
                power: id("POWER.STRENGTH_POWER"),
                target: CreatureSelector::Owner,
                applier: Some(CreatureSelector::Owner),
                amount: Amount::Fixed(1),
            })],
        )],
        ..obtained_only("RELIC.VAJRA", RelicRarity::Common)
    }
}

/// Bronze scales work their thorns in as a fight is walked into.
fn bronze_scales() -> RelicDefinition {
    RelicDefinition {
        hooks: vec![ModifierHook::every_turn(
            HookEvent::AfterRoomEntered,
            vec![Command::Power(PowerCmd::Apply {
                marks: None,
                power: id("POWER.THORNS_POWER"),
                target: CreatureSelector::Owner,
                applier: Some(CreatureSelector::Owner),
                amount: Amount::Fixed(3),
            })],
        )],
        ..obtained_only("RELIC.BRONZE_SCALES", RelicRarity::Common)
    }
}

/// A ringing triangle keeps the opening hand.
fn ringing_triangle() -> RelicDefinition {
    RelicDefinition {
        keeps_first_hand: true,
        ..obtained_only("RELIC.RINGING_TRIANGLE", RelicRarity::Shop)
    }
}

/// The art of war: an energy at the start of a turn that follows one where no
/// attack was played.
///
/// The build remembers two things, whether an attack was played this turn and
/// whether one was played last turn. The relic's own counter carries both:
/// nothing at all, an attack played this turn with none last turn, or an attack
/// played last turn.
fn art_of_war() -> RelicDefinition {
    const NOTHING: i32 = 0;
    const PLAYED_THIS_TURN: i32 = 1;
    const PLAYED_LAST_TURN: i32 = 2;
    RelicDefinition {
        hooks: vec![
            // An attack played does nothing where one was already played last
            // turn, so only the quiet state moves.
            ModifierHook::every_turn(
                HookEvent::AfterCardPlayed,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::ListeningRelicCounterIs(NOTHING),
                    then: vec![Command::Relic(RelicCmd::SetListeningCounter(
                        Amount::Fixed(PLAYED_THIS_TURN),
                    ))],
                }],
            )
            .when(HookCondition::PlayedCardIsOfType(
                sts2_engine::CardType::Attack,
            )),
            // As the turn ends, this turn's answer becomes last turn's, and
            // it is read before it is cleared.
            ModifierHook::every_turn(
                HookEvent::AfterSideTurnEnd,
                vec![
                    Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningRelicCounterIs(PLAYED_LAST_TURN),
                        then: vec![Command::Relic(RelicCmd::SetListeningCounter(
                            Amount::Fixed(NOTHING),
                        ))],
                    },
                    Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningRelicCounterIs(PLAYED_THIS_TURN),
                        then: vec![Command::Relic(RelicCmd::SetListeningCounter(
                            Amount::Fixed(PLAYED_LAST_TURN),
                        ))],
                    },
                ],
            )
            .when(HookCondition::OwnerIsParticipant),
            ModifierHook::every_turn(
                HookEvent::AfterEnergyReset,
                vec![
                    Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::Not(Box::new(Condition::ListeningRelicCounterIs(
                            PLAYED_LAST_TURN,
                        ))),
                        then: vec![Command::Player(PlayerCmd::GainEnergy(Amount::Fixed(1)))],
                    },
                    Command::Relic(RelicCmd::SetListeningCounter(Amount::Fixed(NOTHING))),
                ],
            )
            .timed(HookTiming::AfterFirstTurn),
            // Both private booleans return to false when a fight ends. The
            // packed counter must do the same or an attack from the previous
            // combat can suppress tracking in the next one.
            ModifierHook::every_turn(
                HookEvent::AfterCombatEnd,
                vec![Command::Relic(RelicCmd::SetListeningCounter(
                    Amount::Fixed(NOTHING),
                ))],
            ),
        ],
        ..obtained_only("RELIC.ART_OF_WAR", RelicRarity::Rare)
    }
}

/// A dusty tome carries the ancient card it was set up with; taking it puts an
/// upgraded copy of that card in the deck.
fn dusty_tome() -> RelicDefinition {
    obtained_only("RELIC.DUSTY_TOME", RelicRarity::Ancient)
}

/// A strike dummy pays out on every Strike its owner throws.
fn strike_dummy() -> RelicDefinition {
    RelicDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::ModifyDamageAdditive,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::DamageCardIsTagged(sts2_engine::CardTag::Strike),
                    then: vec![Command::Modify(ModifierCmd::Add(Amount::Fixed(3)))],
                }],
            )
            .when(HookCondition::OwnerDealsPoweredDamage),
        ],
        ..obtained_only("RELIC.STRIKE_DUMMY", RelicRarity::Common)
    }
}

/// A stone calendar counts the turns of every fight, and lands on the seventh.
fn stone_calendar() -> RelicDefinition {
    RelicDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::BeforeSideTurnEnd,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::PlayerTurnIs(7),
                    then: vec![Command::Creature(CreatureCmd::Damage {
                        target: CreatureSelector::AllOpponents,
                        amount: Amount::Fixed(52),
                        props: ValueProps::UNPOWERED,
                    })],
                }],
            )
            .when(HookCondition::OwnerIsParticipant),
        ],
        ..obtained_only("RELIC.STONE_CALENDAR", RelicRarity::Rare)
    }
}

/// A pear is ten more maximum HP, and the HP to go with it, the moment it is
/// taken.
fn pear() -> RelicDefinition {
    upon_pickup(RelicDefinition {
        on_obtained: vec![Command::Creature(CreatureCmd::GainMaxHp {
            target: CreatureSelector::Owner,
            amount: Amount::Fixed(10),
        })],
        ..obtained_only("RELIC.PEAR", RelicRarity::Uncommon)
    })
}

/// A conch is worth two cards and an energy, but only on the opening turn of an
/// elite fight.
fn booming_conch() -> RelicDefinition {
    let only_in_an_elite_opening = |then: Vec<Command>| Command::When {
        otherwise: Vec::new(),
        condition: Condition::PlayerTurnAtMost(1),
        then: vec![Command::When {
            otherwise: Vec::new(),
            condition: Condition::CurrentRoomIs(sts2_engine::ResolvedRoom::Elite),
            then,
        }],
    };
    RelicDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::ModifyHandDraw,
                vec![only_in_an_elite_opening(vec![Command::Modify(
                    ModifierCmd::Add(Amount::Fixed(2)),
                )])],
            ),
            ModifierHook::every_turn(
                HookEvent::AfterSideTurnStart,
                vec![only_in_an_elite_opening(vec![Command::Player(
                    PlayerCmd::GainEnergy(Amount::Fixed(1)),
                )])],
            )
            .when(HookCondition::OwnerIsParticipant),
        ],
        ..obtained_only("RELIC.BOOMING_CONCH", RelicRarity::Ancient)
    }
}

/// The relics that act while a combat is under way, or on the hand itself.
fn other_relics() -> Vec<RelicDefinition> {
    vec![
        // Nunchaku counts every attack the run plays, and pays out on each
        // tenth. The count is the relic's own and carries across combats.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardPlayed,
                    vec![
                        Command::Relic(RelicCmd::ModifyListeningCounter(Amount::Fixed(1))),
                        // The count is kept whatever happens; the energy is
                        // paid only while the fight is still in progress, so an
                        // attack that ends the fight counts and pays nothing.
                        Command::When {
                            otherwise: Vec::new(),
                            condition: Condition::All(vec![
                                Condition::InACombat,
                                Condition::ListeningRelicCounterMultipleOf(10),
                            ]),
                            then: vec![Command::Player(PlayerCmd::GainEnergy(Amount::Fixed(1)))],
                        },
                    ],
                )
                .when(HookCondition::PlayedCardIsAttack),
            ],
            counter_property: Some("AttacksPlayed".to_owned()),
            counter_kind: Some(sts2_engine::CounterKind::Cycle { wraps_at: 10 }),
            ..obtained_only("RELIC.NUNCHAKU", RelicRarity::Uncommon)
        },
        // A pyramid keeps what its owner is holding.
        RelicDefinition {
            keeps_hand: true,
            ..obtained_only("RELIC.RUNIC_PYRAMID", RelicRarity::Ancient)
        },
        // Miniature Cannon pays out on upgraded cards only, joining the
        // additive pass like any other listener.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::ModifyDamageAdditive,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::DamageCardIsUpgraded,
                        then: vec![Command::Modify(ModifierCmd::Add(Amount::Fixed(3)))],
                    }],
                )
                .when(HookCondition::OwnerDealsPoweredDamage),
            ],
            ..obtained_only("RELIC.MINIATURE_CANNON", RelicRarity::Uncommon)
        },
    ]
}

/// The relics this run picks up as it walks: each one is its listener bodies.
fn walked_relics() -> Vec<RelicDefinition> {
    vec![
        // A blood vial heals as the opening turn settles, on the last of the
        // three turn-start passes.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterPlayerTurnStartLate,
                    vec![Command::Creature(CreatureCmd::Heal {
                        target: CreatureSelector::Owner,
                        amount: Amount::Fixed(2),
                    })],
                )
                .timed(HookTiming::FirstTurnOnly),
            ],
            ..obtained_only("RELIC.BLOOD_VIAL", RelicRarity::Common)
        },
        // The stone works its dexterity in as the combat room is entered,
        // before the fight itself starts.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterRoomEntered,
                vec![Command::Power(PowerCmd::Apply {
                    marks: None,
                    power: id("POWER.DEXTERITY_POWER"),
                    target: CreatureSelector::Owner,
                    applier: Some(CreatureSelector::Owner),
                    amount: Amount::Fixed(1),
                })],
            )],
            ..obtained_only("RELIC.ODDLY_SMOOTH_STONE", RelicRarity::Common)
        },
        // An anchor is block that is already there when the fight opens, and
        // nothing scales it.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::BeforeCombatStart,
                vec![Command::Creature(CreatureCmd::GainBlock {
                    target: CreatureSelector::Owner,
                    amount: Amount::Fixed(10),
                    props: ValueProps::UNPOWERED,
                })],
            )],
            ..obtained_only("RELIC.ANCHOR", RelicRarity::Common)
        },
        // A juzu bracelet keeps a monster off an unknown map point, and stops
        // being offered once there is little map left for it to work on.
        RelicDefinition {
            removes_unknown_room_types: vec![sts2_engine::ResolvedRoom::Monster],
            spawn_condition: Some(RelicSpawnCondition::BeforeAct3TreasureChest),
            ..obtained_only("RELIC.JUZU_BRACELET", RelicRarity::Common)
        },
        // Marbles land on everyone still standing as the first turn opens.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::BeforeSideTurnStart,
                    vec![Command::Power(PowerCmd::Apply {
                        marks: None,
                        power: id("POWER.VULNERABLE_POWER"),
                        target: CreatureSelector::AllOpponents,
                        applier: Some(CreatureSelector::Owner),
                        amount: Amount::Fixed(1),
                    })],
                )
                .when(HookCondition::OwnerIsParticipant)
                .timed(HookTiming::FirstTurnOnly),
            ],
            ..obtained_only("RELIC.BAG_OF_MARBLES", RelicRarity::Common)
        },
    ]
}

/// The relics this run buys in the Hive and later, each one its listener
/// bodies.
fn hive_relics() -> Vec<RelicDefinition> {
    vec![
        // A cleat braces the second turn as its block is cleared.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterBlockCleared,
                    vec![Command::Creature(CreatureCmd::GainBlock {
                        target: CreatureSelector::Owner,
                        amount: Amount::Fixed(14),
                        props: ValueProps::UNPOWERED,
                    })],
                )
                .timed(HookTiming::SecondTurnOnly),
            ],
            ..obtained_only("RELIC.HORN_CLEAT", RelicRarity::Uncommon)
        },
        // A fan pays block on every third attack of a turn, and the count
        // starts over as its owner's turn opens.
        RelicDefinition {
            hooks: vec![
                counts_cards_each_turn(sts2_engine::CardType::Attack),
                third_card_of_the_turn(
                    sts2_engine::CardType::Attack,
                    Command::Creature(CreatureCmd::GainBlock {
                        target: CreatureSelector::Owner,
                        amount: Amount::Fixed(4),
                        props: ValueProps::UNPOWERED,
                    }),
                ),
                clears_the_count(HookEvent::BeforeSideTurnStart),
            ],
            ..obtained_only("RELIC.ORNAMENTAL_FAN", RelicRarity::Uncommon)
        },
        // A letter opener cuts every enemy on every third skill of a turn. Its
        // count starts over as the fight opens and as each later turn opens.
        RelicDefinition {
            hooks: vec![
                counts_cards_each_turn(sts2_engine::CardType::Skill),
                third_card_of_the_turn(
                    sts2_engine::CardType::Skill,
                    Command::Creature(CreatureCmd::Damage {
                        target: CreatureSelector::AllOpponents,
                        amount: Amount::Fixed(5),
                        props: ValueProps::UNPOWERED,
                    }),
                ),
                clears_the_count(HookEvent::BeforeCombatStart),
                clears_the_count(HookEvent::AfterSideTurnStart).timed(HookTiming::AfterFirstTurn),
            ],
            ..obtained_only("RELIC.LETTER_OPENER", RelicRarity::Uncommon)
        },
        // A basin pays block at the end of a turn its owner played no attack
        // in. The count of attacks is the fan's count kept for the basin.
        RelicDefinition {
            hooks: vec![
                counts_cards_each_turn(sts2_engine::CardType::Attack),
                ModifierHook::every_turn(
                    HookEvent::BeforeSideTurnEnd,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningRelicCounterIs(0),
                        then: vec![Command::Creature(CreatureCmd::GainBlock {
                            target: CreatureSelector::Owner,
                            amount: Amount::Fixed(4),
                            props: ValueProps::UNPOWERED,
                        })],
                    }],
                )
                .when(HookCondition::OwnerIsParticipant),
                clears_the_count(HookEvent::BeforeSideTurnStart),
            ],
            ..obtained_only("RELIC.RIPPLE_BASIN", RelicRarity::Uncommon)
        },
        // A shield answers the end of a turn its owner finished behind ten
        // block, striking one enemy it rolls for.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnEnd,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::OwnerBlockAtLeast(10),
                        then: vec![Command::Creature(CreatureCmd::Damage {
                            target: CreatureSelector::RandomOpponent,
                            amount: Amount::Fixed(6),
                            props: ValueProps::UNPOWERED,
                        })],
                    }],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..obtained_only("RELIC.PARRYING_SHIELD", RelicRarity::Uncommon)
        },
    ]
}

/// Counts the cards of one type its owner plays, which is the per-turn count
/// these relics keep for themselves.
fn counts_cards_each_turn(card_type: sts2_engine::CardType) -> ModifierHook {
    ModifierHook::every_turn(
        HookEvent::AfterCardPlayed,
        vec![Command::Relic(RelicCmd::ModifyListeningCounter(
            Amount::Fixed(1),
        ))],
    )
    .when(HookCondition::PlayedCardIsOfType(card_type))
}

/// What the relic does when the count comes round to a third card.
fn third_card_of_the_turn(card_type: sts2_engine::CardType, effect: Command) -> ModifierHook {
    ModifierHook::every_turn(
        HookEvent::AfterCardPlayed,
        vec![Command::When {
            otherwise: Vec::new(),
            condition: Condition::ListeningRelicCounterMultipleOf(3),
            then: vec![effect],
        }],
    )
    .when(HookCondition::PlayedCardIsOfType(card_type))
}

/// Starts the count over.
fn clears_the_count(event: HookEvent) -> ModifierHook {
    ModifierHook::every_turn(
        event,
        vec![Command::Relic(RelicCmd::SetListeningCounter(
            Amount::Fixed(0),
        ))],
    )
    .when(HookCondition::OwnerIsParticipant)
}

/// A music box copies the first attack of every turn into its owner's hand,
/// and the copy burns up if it is still held when the turn ends.
fn music_box() -> RelicDefinition {
    RelicDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterCardPlayed,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::ListeningRelicCounterIs(0),
                    then: vec![
                        Command::CardPile(sts2_engine::CardPileCmd::AddGeneratedToCombat {
                            enchantment: None,
                            card: sts2_engine::CardOrigin::CloneOfPlayedCard,
                            grown_by: Amount::Fixed(0),
                            count: Amount::Fixed(1),
                            pile: sts2_engine::PileName::Hand,
                            ethereal: true,
                            retain: false,
                            upgraded: false,
                            temporary_cost: None,
                            position: sts2_engine::DrawPilePosition::Bottom,
                        }),
                        Command::Relic(RelicCmd::SetListeningCounter(Amount::Fixed(1))),
                    ],
                }],
            )
            .when(HookCondition::PlayedCardIsAttack),
            ModifierHook::every_turn(
                HookEvent::BeforeSideTurnStart,
                vec![Command::Relic(RelicCmd::SetListeningCounter(
                    Amount::Fixed(0),
                ))],
            )
            .when(HookCondition::OwnerIsParticipant),
        ],
        ..obtained_only("RELIC.MUSIC_BOX", RelicRarity::Ancient)
    }
}

/// Each character's starter relic and the refined form a touch of Orobas
/// trades it for.
fn refinement_upgrades() -> Vec<(sts2_core::ModelId, sts2_core::ModelId)> {
    vec![
        (id("RELIC.BURNING_BLOOD"), id("RELIC.BLACK_BLOOD")),
        (id("RELIC.RING_OF_THE_SNAKE"), id("RELIC.RING_OF_THE_DRAKE")),
        (id("RELIC.DIVINE_RIGHT"), id("RELIC.DIVINE_DESTINY")),
        (id("RELIC.BOUND_PHYLACTERY"), id("RELIC.PHYLACTERY_UNBOUND")),
        (id("RELIC.CRACKED_CORE"), id("RELIC.INFUSED_CORE")),
    ]
}

/// The relics an ancient deals. Only what a recorded run took carries a body;
/// the rest are registered for the offer, and taking one stops.
#[allow(clippy::too_many_lines, reason = "one entry per relic")]
fn ancient_relics() -> Vec<RelicDefinition> {
    let mut relics = vec![
        // An archaic tooth, once taken, turns the deck's starter card into
        // its ancient form. The whole table is written down; only the
        // Ironclad's pair is reachable at this baseline.
        RelicDefinition {
            setup: Some(sts2_engine::RelicSetup::Transcendence {
                upgrades: transcendence_upgrades(),
            }),
            on_obtained: vec![Command::Card(sts2_engine::CardCmd::TransformInDeck {
                upgrades: transcendence_upgrades(),
            })],
            ..obtained_only("RELIC.ARCHAIC_TOOTH", RelicRarity::Ancient)
        },
    ];
    relics.extend(transforming_relics());
    relics.extend(given_relics());
    relics.extend(ancient_combat_relics());
    relics.extend(reward_relics());
    relics.extend(potion_relics());
    relics.extend(counting_relics());
    relics.extend(cost_relics());
    // A fur coat marks seven of the act's combats as it is taken, and every
    // enemy of a marked room stands on one HP: whoever is there as the fight
    // opens, and whoever joins it afterwards.
    relics.push(RelicDefinition {
        map_rule: Some(sts2_engine::RelicMapRule::MarksCombatRooms { count: 7 }),
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::BeforeCombatStart,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::ListeningRelicMarkedThisRoom,
                    then: vec![Command::Creature(CreatureCmd::SetCurrentHp {
                        target: CreatureSelector::AllOpponents,
                        amount: Amount::Fixed(1),
                    })],
                }],
            ),
            ModifierHook::every_turn(
                HookEvent::AfterCreatureAddedToCombat,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::ListeningRelicMarkedThisRoom,
                    then: vec![Command::Creature(CreatureCmd::SetCurrentHp {
                        target: CreatureSelector::Announced,
                        amount: Amount::Fixed(1),
                    })],
                }],
            )
            .when(HookCondition::OwnerIsEnemy),
        ],
        ..obtained_only("RELIC.FUR_COAT", RelicRarity::Ancient)
    });
    // A golden compass lays the act it was taken in out as one column and
    // makes every unknown point of it an event.
    relics.push(RelicDefinition {
        map_rule: Some(sts2_engine::RelicMapRule::GoldenPath),
        ..upon_pickup(obtained_only("RELIC.GOLDEN_COMPASS", RelicRarity::Ancient))
    });
    // The Lord's parasol buys the whole shelf the moment its owner walks into a
    // merchant, and pays nothing for any of it.
    relics.push(RelicDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterRoomEntered,
                vec![Command::Relic(RelicCmd::BuyTheWholeShelfForNothing)],
            )
            .when(HookCondition::InAMerchantRoom),
        ],
        ..obtained_only("RELIC.LORDS_PARASOL", RelicRarity::Ancient)
    });
    // Pael's eye: one more turn a fight, where its owner played nothing they
    // chose. The hand they were holding burns first, and the eye is spent as
    // the turn it bought opens.
    relics.push(RelicDefinition {
        grants_an_extra_turn: Some(sts2_engine::ExtraTurnRule {
            counts_as_played_on_the_first_turn: id("RELIC.WHISPERING_EARRING"),
        }),
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::BeforeSideTurnEndEarly,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::ListeningRelicsExtraTurnIsOwed,
                    then: vec![Command::Card(CardCmd::Exhaust {
                        cards: CardSelector::Pile {
                            pile: sts2_engine::PileName::Hand,
                            filter: CardFilter::Any,
                        },
                    })],
                }],
            )
            .when(HookCondition::OwnerIsParticipant),
            ModifierHook::every_turn(HookEvent::AfterTakingExtraTurn, vec![latch(true)]),
            ModifierHook::every_turn(HookEvent::AfterCombatEnd, vec![latch(false)]),
        ],
        ..obtained_only("RELIC.PAELS_EYE", RelicRarity::Ancient)
    });
    // Neow's bones deal two of Neow's own options and charge a curse for them.
    // The offer is one set the player may not walk away from, and the curse is
    // added once it is done with.
    relics.push(RelicDefinition {
        on_obtained: vec![
            Command::Rewards(sts2_engine::RewardsCmd::OfferShuffledRelics {
                relics: neow_option_relics(),
                count: 2,
            }),
            Command::CardPile(sts2_engine::CardPileCmd::AddCursesToDeck {
                pool: id("CARD_POOL.CURSE_CARD_POOL"),
                count: 1,
            }),
        ],
        ..obtained_only("RELIC.NEOWS_BONES", RelicRarity::Ancient)
    });
    // A whispering earring pays a point of energy and plays its owner's whole
    // opening turn for them, thirteen cards at most. It answers the last of the
    // three auto-pre-play passes so that everything else of the phase — an
    // Imbued playing the card it is on, above all — has already run.
    relics.push(RelicDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::ModifyMaxEnergy,
                vec![Command::Modify(ModifierCmd::Add(Amount::Fixed(1)))],
            ),
            ModifierHook::every_turn(
                HookEvent::AfterAutoPrePlayPhaseEnteredLate,
                vec![Command::Card(CardCmd::PlayTheHandAutomatically {
                    limit: Amount::Fixed(13),
                })],
            )
            .timed(HookTiming::FirstTurnOnly),
        ],
        ..obtained_only("RELIC.WHISPERING_EARRING", RelicRarity::Ancient)
    });
    // A prismatic gem pays a point of energy and puts every character's pool
    // behind every card reward its owner is offered. Nothing happens as it is
    // taken, so the refused on-obtain body the offered-only default carries is
    // replaced by an empty one: Orobas really lays the gem in front of a
    // single-player act-two run, and taking it has to be a step the engine
    // accepts.
    relics.push(RelicDefinition {
        adds_every_character_pool_to_rewards: true,
        on_obtained: Vec::new(),
        hooks: vec![ModifierHook::every_turn(
            HookEvent::ModifyMaxEnergy,
            vec![Command::Modify(ModifierCmd::Add(Amount::Fixed(1)))],
        )],
        ..offered_only("RELIC.PRISMATIC_GEM")
    });
    // A touch of orobas replaces the starter relic its owner walked in with by
    // the refined form of it, and saves both as it is offered.
    relics.push(RelicDefinition {
        setup: Some(sts2_engine::RelicSetup::Refinement {
            upgrades: refinement_upgrades(),
            fallback: id("RELIC.CIRCLET"),
        }),
        on_obtained: vec![Command::Relic(sts2_engine::RelicCmd::RefineStarter {
            upgrades: refinement_upgrades(),
            fallback: id("RELIC.CIRCLET"),
        })],
        ..offered_only("RELIC.TOUCH_OF_OROBAS")
    });
    relics.extend(enchanting_ancients());
    // The offered-only ancients that still declare something outside their
    // bodies: what a bag may hold, what stops the relic answering, whether it
    // acts the moment it is taken, and whether it brings a pet.
    relics.extend([
        // A sea glass belongs to another character, and the ancient that offers
        // one names whose as it lays the option out. What it deals is the
        // event's own effect, so the relic itself does nothing as it is taken.
        upon_pickup(obtained_only("RELIC.SEA_GLASS", RelicRarity::Ancient)),
        // A kaleidoscope lays out two card offers as it is taken, each of them
        // one card from each of three colours other than the one being played.
        upon_pickup(RelicDefinition {
            on_obtained: vec![
                Command::Rewards(sts2_engine::RewardsCmd::AddCardOfferOfOtherColours {
                    colours: 3,
                }),
                Command::Rewards(sts2_engine::RewardsCmd::AddCardOfferOfOtherColours {
                    colours: 3,
                }),
                Command::Rewards(sts2_engine::RewardsCmd::OfferAdded),
            ],
            ..offered_only("RELIC.KALEIDOSCOPE")
        }),
        // Pael's tooth takes five upgradable cards out of the deck as it is
        // taken and hands one of them back, upgraded, at the end of every
        // fight. The screen asks for exactly five and needs no confirming, so a
        // deck with no more than that many upgradable cards loses all of them
        // without a question. What it keeps is ordered by model ID, which is
        // the order the build writes them down in.
        upon_pickup(RelicDefinition {
            saved_card_array_property: Some("SerializableCards".to_owned()),
            on_obtained: vec![Command::CardSelect(CardSelectCmd::FromDeckForRemoval {
                semantics: ChoiceSemantics::known(vec![
                    ChoiceOp::new(ChoiceOperationKind::Store, ChoiceScope::Run)
                        .to(ChoiceDestination::RelicStorage),
                    ChoiceOp::new(ChoiceOperationKind::Upgrade, ChoiceScope::Run)
                        .at(ChoiceTiming::AfterEachCombat),
                    ChoiceOp::new(ChoiceOperationKind::Move, ChoiceScope::Run)
                        .to(ChoiceDestination::RunDeck)
                        .at(ChoiceTiming::AfterEachCombat),
                ]),
                prefs: SelectorPrefs::exactly(5),
                filter: CardFilter::Upgradable,
                then: vec![Command::Relic(RelicCmd::KeepChosenCards)],
            })],
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterCombatEnd,
                vec![Command::Relic(RelicCmd::HandBackAKeptCard)],
            )],
            ..obtained_only("RELIC.PAELS_TOOTH", RelicRarity::Ancient)
        }),
        // Pael's legion brings a pet, and doubles the block of one card play
        // every three turns. The build notes which play the doubling answered
        // as it changes the block, and spends that note when that same play
        // finishes; here the latch is that note, and the count it then sets
        // runs down as its owner's turn opens.
        RelicDefinition {
            spawns_pets: true,
            adds_pet: true,
            rolled_skin: Some(sts2_engine::RolledSkin {
                property: "Skin".to_owned(),
                options: ["eyes", "horns", "spikes", "wings"]
                    .map(ToOwned::to_owned)
                    .to_vec(),
            }),
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::BeforeCombatStart,
                    vec![Command::Player(PlayerCmd::AddPet {
                        monster: id("MONSTER.PAELS_LEGION"),
                    })],
                ),
                // A cooldown still running answers with no change, and the
                // three tests in front of it admit block worked up by a card of
                // its owner's, and nothing else.
                ModifierHook::every_turn(
                    HookEvent::ModifyBlockMultiplicative,
                    vec![Command::Modify(ModifierCmd::Scale(Amount::IfElse {
                        condition: Condition::ListeningRelicCounterIs(0),
                        then: Box::new(Amount::Fixed(200)),
                        otherwise: Box::new(Amount::Fixed(100)),
                    }))],
                )
                .when(HookCondition::OwnerGainsPoweredBlockFromACard),
                // This pass runs over exactly the listeners that changed the
                // value, so reaching it at all is the relic having doubled this
                // play's block — but as with a vambrace, a play that worked no
                // block up leaves the relic nothing to have answered for.
                ModifierHook::every_turn(
                    HookEvent::AfterModifyingBlockAmount,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::CountAtLeast {
                            count: sts2_engine::Count::AnnouncedAmount,
                            at_least: 1,
                        },
                        then: vec![latch(true)],
                    }],
                ),
                ModifierHook::every_turn(
                    HookEvent::AfterCardPlayed,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningRelicIsLatched,
                        then: vec![
                            Command::Relic(RelicCmd::SetListeningCounter(Amount::Fixed(2))),
                            latch(false),
                        ],
                    }],
                ),
                // `Cooldown--` every turn its owner takes part in, which is
                // what makes the doubling a three-turn cycle.
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::Not(Box::new(Condition::ListeningRelicCounterIs(0))),
                        then: vec![Command::Relic(RelicCmd::ModifyListeningCounter(
                            Amount::Fixed(-1),
                        ))],
                    }],
                )
                .when(HookCondition::OwnerIsParticipant),
                ModifierHook::every_turn(
                    HookEvent::AfterCombatEnd,
                    vec![
                        Command::Relic(RelicCmd::SetListeningCounter(Amount::Fixed(0))),
                        latch(false),
                    ],
                ),
            ],
            ..obtained_only("RELIC.PAELS_LEGION", RelicRarity::Ancient)
        },
        // Both are gated on the number of players: a scroll only a
        // multiplayer run may hold, and a crucible only a single-player one.
        RelicDefinition {
            spawn_condition: Some(RelicSpawnCondition::MultiplayerOnly),
            ..offered_only("RELIC.MASSIVE_SCROLL")
        },
        // A silver crucible upgrades every card of the next three card rewards
        // it can, counts a use whether or not any of them could be, and
        // suppresses the treasure of the first chest walked into after it.
        //
        // Its used-up test is made of both counts it keeps, and the room count
        // rises on the way in — before the chest asks whether to generate
        // anything — so the chest the relic was taken to is the first one
        // suppressed and the second one pays out.
        RelicDefinition {
            spawn_condition: Some(RelicSpawnCondition::SinglePlayerOnly),
            counter_property: Some("TimesUsed".to_owned()),
            second_counter_property: Some("TreasureRoomsEntered".to_owned()),
            counter_kind: Some(sts2_engine::CounterKind::Ledger),
            latch_property: None,
            used_up: Some(sts2_engine::UsedUp::BothCountsAtLeast {
                counter: 3,
                second_counter: 1,
            }),
            suppresses_treasure_while_second_counter_at_most: Some(1),
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::TryModifyCardRewardOptionsLate,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::All(vec![
                            Condition::Not(Box::new(Condition::ListeningRelicCounterAtLeast(3))),
                            Condition::CardOfferIsACardReward,
                        ]),
                        // The offer shows an upgraded copy of the card, and
                        // the relic counts the use whether or not anything was
                        // upgradable.
                        then: vec![Command::Rewards(RewardsCmd::UpgradeCardOptions(
                            CardFilter::Upgradable,
                        ))],
                    }],
                ),
                ModifierHook::every_turn(
                    HookEvent::AfterModifyingCardRewardOptions,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::Not(Box::new(
                            Condition::ListeningRelicCounterAtLeast(3),
                        )),
                        then: vec![Command::Relic(RelicCmd::ModifyListeningCounter(
                            Amount::Fixed(1),
                        ))],
                    }],
                ),
                ModifierHook::every_turn(
                    HookEvent::AfterRoomEntered,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::CurrentRoomIs(sts2_engine::ResolvedRoom::Treasure),
                        then: vec![Command::Relic(RelicCmd::ModifyListeningSecondCounter(
                            Amount::Fixed(1),
                        ))],
                    }],
                ),
            ],
            ..obtained_only("RELIC.SILVER_CRUCIBLE", RelicRarity::Ancient)
        },
        // It is used up at three uses, which is what stops it being traded away
        // once its three free walks are gone.
        RelicDefinition {
            spawn_condition: Some(RelicSpawnCondition::SinglePlayerOnly),
            used_up: Some(sts2_engine::UsedUp::CounterAtLeast(3)),
            counter_property: Some("TimesUsed".to_owned()),
            counter_kind: Some(sts2_engine::CounterKind::Ledger),
            grants_free_travel: true,
            ..obtained_only("RELIC.WINGED_BOOTS", RelicRarity::Ancient)
        },
        // A toy box hands over four relics off the front of its owner's own
        // bag, each marked wax, and melts one of them again at the end of every
        // third fight. It is used up once every wax relic has melted, which is
        // twelve fights: four wax relics, one melted every three.
        upon_pickup(RelicDefinition {
            counter_property: Some("CombatsSeen".to_owned()),
            counter_kind: Some(sts2_engine::CounterKind::Ledger),
            used_up: Some(sts2_engine::UsedUp::CounterAtLeast(12)),
            on_obtained: vec![
                Command::Rewards(RewardsCmd::AddWaxRelic),
                Command::Rewards(RewardsCmd::AddWaxRelic),
                Command::Rewards(RewardsCmd::AddWaxRelic),
                Command::Rewards(RewardsCmd::AddWaxRelic),
                Command::Rewards(RewardsCmd::OfferAdded),
            ],
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterCombatEnd,
                vec![
                    Command::Relic(RelicCmd::ModifyListeningCounter(Amount::Fixed(1))),
                    Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningRelicCounterMultipleOf(3),
                        then: vec![Command::Relic(RelicCmd::MeltFirstWaxRelic)],
                    },
                ],
            )],
            ..obtained_only("RELIC.TOY_BOX", RelicRarity::Ancient)
        }),
    ]);
    relics
}

/// A card of this model joining the run deck.
fn joins_the_deck(model_id: &str) -> Command {
    Command::CardPile(CardPileCmd::AddToDeck {
        card: CardOrigin::Model(id(model_id)),
    })
}

/// That exact number of cards picked out of the deck for removal, and what is
/// then done with what came back.
fn removes_from_the_deck(count: usize, then: Vec<Command>) -> Command {
    Command::CardSelect(sts2_engine::CardSelectCmd::FromDeckForRemoval {
        semantics: ChoiceSemantics::known(vec![
            ChoiceOp::new(ChoiceOperationKind::Remove, ChoiceScope::Run)
                .to(ChoiceDestination::Removed),
        ]),
        prefs: sts2_engine::SelectorPrefs::exactly(count),
        filter: sts2_engine::CardFilter::Any,
        then,
    })
}

/// Whatever the enclosing selection picked leaves the deck.
fn removes_the_chosen() -> Command {
    Command::CardPile(CardPileCmd::RemoveFromDeck {
        cards: sts2_engine::CardSelector::Chosen,
    })
}

/// That exact number of cards picked out of the deck for upgrade, and the
/// upgrade that then runs on what came back.
fn upgrades_from_the_deck(count: usize) -> Command {
    Command::CardSelect(sts2_engine::CardSelectCmd::FromDeckForUpgrade {
        semantics: ChoiceSemantics::known(vec![ChoiceOp::new(
            ChoiceOperationKind::Upgrade,
            ChoiceScope::Run,
        )]),
        prefs: sts2_engine::SelectorPrefs::exactly(count),
        then: vec![Command::Card(sts2_engine::CardCmd::Upgrade {
            cards: sts2_engine::CardSelector::Chosen,
        })],
    })
}

/// One more energy for the relic's owner, which is what most of the ancients
/// that raise the energy give.
fn pays_an_energy(amount: i32) -> ModifierHook {
    ModifierHook::every_turn(
        HookEvent::ModifyMaxEnergy,
        vec![Command::Modify(ModifierCmd::Add(Amount::Fixed(amount)))],
    )
}

/// The ancients whose whole effect happens where they are taken.
#[allow(clippy::too_many_lines, reason = "one entry per relic")]
fn given_relics() -> Vec<RelicDefinition> {
    vec![
        // Gold, and nothing else.
        upon_pickup(RelicDefinition {
            on_obtained: vec![Command::Player(PlayerCmd::GainGold(Amount::Fixed(150)))],
            ..obtained_only("RELIC.GOLDEN_PEARL", RelicRarity::Ancient)
        }),
        RelicDefinition {
            on_obtained: vec![Command::Player(PlayerCmd::GainGold(Amount::Fixed(999)))],
            ..obtained_only("RELIC.SIGNET_RING", RelicRarity::Ancient)
        },
        // Eleven more maximum HP, and the HP to go with it.
        upon_pickup(RelicDefinition {
            on_obtained: vec![Command::Creature(CreatureCmd::GainMaxHp {
                target: CreatureSelector::Owner,
                amount: Amount::Fixed(11),
            })],
            ..obtained_only("RELIC.NUTRITIOUS_OYSTER", RelicRarity::Ancient)
        }),
        // One card into the deck, and that is the whole of it.
        upon_pickup(RelicDefinition {
            on_obtained: vec![joins_the_deck("CARD.APOTHEOSIS")],
            ..obtained_only("RELIC.JEWELRY_BOX", RelicRarity::Ancient)
        }),
        upon_pickup(RelicDefinition {
            on_obtained: vec![joins_the_deck("CARD.NEOWS_FURY")],
            ..obtained_only("RELIC.NEOWS_TORMENT", RelicRarity::Ancient)
        }),
        RelicDefinition {
            on_obtained: vec![joins_the_deck("CARD.BRIGHTEST_FLAME")],
            ..obtained_only("RELIC.STORYBOOK", RelicRarity::Ancient)
        },
        RelicDefinition {
            on_obtained: vec![joins_the_deck("CARD.WHISTLE")],
            ..obtained_only("RELIC.TANXS_WHISTLE", RelicRarity::Ancient)
        },
        RelicDefinition {
            on_obtained: vec![joins_the_deck("CARD.RELAX"), joins_the_deck("CARD.RELAX")],
            ..obtained_only("RELIC.PAELS_HORN", RelicRarity::Ancient)
        },
        // A curse for an energy: the curse joins where the relic is taken and
        // the energy is a pass the relic answers for the rest of the run.
        upon_pickup(RelicDefinition {
            on_obtained: vec![joins_the_deck("CARD.ENTHRALLED")],
            hooks: vec![pays_an_energy(1)],
            ..obtained_only("RELIC.BLOOD_SOAKED_ROSE", RelicRarity::Ancient)
        }),
        // Nine maximum HP for three apparitions.
        upon_pickup(RelicDefinition {
            on_obtained: vec![
                Command::Creature(CreatureCmd::LoseMaxHp {
                    target: CreatureSelector::Owner,
                    amount: Amount::Fixed(9),
                }),
                joins_the_deck("CARD.APPARITION"),
                joins_the_deck("CARD.APPARITION"),
                joins_the_deck("CARD.APPARITION"),
            ],
            ..obtained_only("RELIC.DISTINGUISHED_CAPE", RelicRarity::Ancient)
        }),
        // The newest starter strike and the newest starter defend of the deck,
        // each upgraded. The build reads them off the back of the deck.
        upon_pickup(RelicDefinition {
            on_obtained: vec![Command::Card(sts2_engine::CardCmd::Upgrade {
                cards: sts2_engine::CardSelector::LastInDeckOfEach {
                    filters: vec![
                        sts2_engine::CardFilter::BasicOfTag(sts2_engine::CardTag::Strike),
                        sts2_engine::CardFilter::BasicOfTag(sts2_engine::CardTag::Defend),
                    ],
                },
            })],
            ..obtained_only("RELIC.NEOWS_TALISMAN", RelicRarity::Ancient)
        }),
        // The scissors and the cage take cards out of the deck and nothing
        // else. The count each asks for is the whole of the difference.
        upon_pickup(RelicDefinition {
            on_obtained: vec![removes_from_the_deck(1, vec![removes_the_chosen()])],
            ..obtained_only("RELIC.PRECISE_SCISSORS", RelicRarity::Ancient)
        }),
        upon_pickup(RelicDefinition {
            on_obtained: vec![removes_from_the_deck(2, vec![removes_the_chosen()])],
            ..obtained_only("RELIC.EMPTY_CAGE", RelicRarity::Ancient)
        }),
        // Three cards out for a folly in.
        RelicDefinition {
            on_obtained: vec![removes_from_the_deck(
                3,
                vec![removes_the_chosen(), joins_the_deck("CARD.FOLLY")],
            )],
            ..obtained_only("RELIC.PRESERVED_FOG", RelicRarity::Ancient)
        },
        // Two cards out, and sixteen damage nothing scales for them.
        upon_pickup(RelicDefinition {
            on_obtained: vec![removes_from_the_deck(
                2,
                vec![
                    removes_the_chosen(),
                    Command::Creature(CreatureCmd::Damage {
                        target: CreatureSelector::Owner,
                        amount: Amount::Fixed(16),
                        props: ValueProps::UNPOWERED,
                    }),
                ],
            )],
            ..obtained_only("RELIC.PRECARIOUS_SHEARS", RelicRarity::Ancient)
        }),
        // Four cards out of the deck, and a soot into the draw pile on every
        // shuffle from then on.
        RelicDefinition {
            on_obtained: vec![removes_from_the_deck(4, vec![removes_the_chosen()])],
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterShuffle,
                vec![Command::CardPile(CardPileCmd::AddGeneratedToCombat {
                    enchantment: None,
                    card: CardOrigin::Model(id("CARD.SOOT")),
                    grown_by: Amount::Fixed(0),
                    count: Amount::Fixed(1),
                    pile: sts2_engine::PileName::Draw,
                    ethereal: false,
                    retain: false,
                    upgraded: false,
                    temporary_cost: None,
                    position: sts2_engine::DrawPilePosition::Random,
                })],
            )],
            ..obtained_only("RELIC.BIIIG_HUG", RelicRarity::Ancient)
        },
        // The two relics that upgrade what their owner picks.
        RelicDefinition {
            on_obtained: vec![upgrades_from_the_deck(1)],
            ..obtained_only("RELIC.POMANDER", RelicRarity::Ancient)
        },
        upon_pickup(RelicDefinition {
            on_obtained: vec![upgrades_from_the_deck(4)],
            ..obtained_only("RELIC.YUMMY_COOKIE", RelicRarity::Ancient)
        }),
        // A sand castle upgrades six of the deck's upgradable cards, shuffled
        // on the run's niche stream rather than chosen.
        upon_pickup(RelicDefinition {
            on_obtained: vec![Command::Card(sts2_engine::CardCmd::Upgrade {
                cards: sts2_engine::CardSelector::StablyShuffledFromDeck {
                    filter: sts2_engine::CardFilter::Upgradable,
                    take: Amount::Fixed(6),
                    stream: sts2_engine::ShuffleStream::Niche,
                },
            })],
            ..obtained_only("RELIC.SAND_CASTLE", RelicRarity::Ancient)
        }),
    ]
}

/// The ancients that change what a card costs, what may be played, or what is
/// drawn.
#[allow(clippy::too_many_lines, reason = "one entry per relic")]
fn cost_relics() -> Vec<RelicDefinition> {
    // The count of cards played this turn that three of them keep for
    // themselves, cleared as each turn opens and as a fight ends.
    // The count is wound on as each play finishes. A scarf ignores a play it
    // did not choose and a choker does not, so which plays are counted is the
    // relic's own business.
    let counts_the_turns_plays = |only_chosen: bool| {
        vec![
            ModifierHook::every_turn(
                HookEvent::AfterCardPlayed,
                vec![Command::Relic(RelicCmd::ModifyListeningCounter(
                    Amount::Fixed(1),
                ))],
            )
            .when(if only_chosen {
                HookCondition::PlayWasChosen
            } else {
                HookCondition::Always
            }),
            ModifierHook::every_turn(
                HookEvent::BeforeSideTurnStart,
                vec![Command::Relic(RelicCmd::SetListeningCounter(
                    Amount::Fixed(0),
                ))],
            )
            .when(HookCondition::OwnerIsParticipant),
            ModifierHook::every_turn(
                HookEvent::AfterCombatEnd,
                vec![Command::Relic(RelicCmd::SetListeningCounter(
                    Amount::Fixed(0),
                ))],
            ),
        ]
    };
    vec![
        // A snecko eye is two more cards a turn and a confusion that never
        // wears off: the power lands as the relic is taken if a fight is
        // standing, and as every fight opens after that.
        RelicDefinition {
            on_obtained: vec![Command::When {
                otherwise: Vec::new(),
                condition: Condition::InACombat,
                then: vec![own_power("POWER.CONFUSED_POWER", Amount::Fixed(1))],
            }],
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::BeforeCombatStart,
                    vec![own_power("POWER.CONFUSED_POWER", Amount::Fixed(1))],
                ),
                ModifierHook::every_turn(
                    HookEvent::ModifyHandDraw,
                    vec![Command::Modify(ModifierCmd::Add(Amount::Fixed(2)))],
                ),
            ],
            ..obtained_only("RELIC.SNECKO_EYE", RelicRarity::Ancient)
        },
        // Spiked gauntlets pay an energy and charge one more for every power.
        RelicDefinition {
            card_cost_rule: Some(sts2_engine::RelicCostRule {
                rule: sts2_engine::CardCostRule::PowersCostMore,
                amount: 1,
                when_counter_is: None,
            }),
            hooks: vec![pays_an_energy(1)],
            ..obtained_only("RELIC.SPIKED_GAUNTLETS", RelicRarity::Ancient)
        },
        // A brilliant scarf makes the fifth card of every turn free, which is
        // the count standing at four when the cost is worked out.
        RelicDefinition {
            card_cost_rule: Some(sts2_engine::RelicCostRule {
                rule: sts2_engine::CardCostRule::FreeInHandOrPlay,
                amount: 0,
                when_counter_is: Some(4),
            }),
            hooks: counts_the_turns_plays(true),
            ..obtained_only("RELIC.BRILLIANT_SCARF", RelicRarity::Ancient)
        },
        // A velvet choker pays an energy and allows six cards a turn.
        RelicDefinition {
            cards_playable_each_turn: Some(6),
            hooks: {
                let mut hooks = counts_the_turns_plays(false);
                hooks.push(pays_an_energy(1));
                // The choker starts its count over as a fight's room is
                // entered as well.
                hooks.push(ModifierHook::every_turn(
                    HookEvent::AfterRoomEntered,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::CurrentRoomIsCombat,
                        then: vec![Command::Relic(RelicCmd::SetListeningCounter(
                            Amount::Fixed(0),
                        ))],
                    }],
                ));
                hooks
            },
            ..obtained_only("RELIC.VELVET_CHOKER", RelicRarity::Ancient)
        },
        // A fiddle draws two more on every hand draw, on the late pass, and
        // refuses every other draw its owner's own turn would make.
        RelicDefinition {
            stops_draws_on_its_own_side_turn: true,
            hooks: vec![ModifierHook::every_turn(
                HookEvent::ModifyHandDrawLate,
                vec![Command::Modify(ModifierCmd::Add(Amount::Fixed(2)))],
            )],
            ..obtained_only("RELIC.FIDDLE", RelicRarity::Ancient)
        },
        // A diamond diadem answers a quiet turn: two cards or fewer played, and
        // the halving power lands as the turn ends.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardPlayed,
                    vec![Command::Relic(RelicCmd::ModifyListeningCounter(
                        Amount::Fixed(1),
                    ))],
                ),
                ModifierHook::every_turn(
                    HookEvent::BeforeSideTurnEnd,
                    vec![
                        Command::When {
                            otherwise: Vec::new(),
                            condition: Condition::Not(Box::new(
                                Condition::ListeningRelicCounterAtLeast(3),
                            )),
                            then: vec![own_power("POWER.DIAMOND_DIADEM_POWER", Amount::Fixed(1))],
                        },
                        Command::Relic(RelicCmd::SetListeningCounter(Amount::Fixed(0))),
                    ],
                )
                .when(HookCondition::OwnerIsParticipant),
                ModifierHook::every_turn(
                    HookEvent::AfterCombatEnd,
                    vec![Command::Relic(RelicCmd::SetListeningCounter(
                        Amount::Fixed(0),
                    ))],
                ),
            ],
            ..obtained_only("RELIC.DIAMOND_DIADEM", RelicRarity::Ancient)
        },
        // Toasty mittens burn a card off the draw pile before every hand draw
        // and pay a strength for it. On the opening turn the card burned is
        // the first that is not innate, and otherwise the first there is.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::BeforeHandDraw,
                vec![
                    Command::CardPile(CardPileCmd::ShuffleIfNecessary),
                    Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::PlayerTurnIs(1),
                        then: vec![Command::Select {
                            cards: sts2_engine::CardSelector::FirstInPilePreferring {
                                pile: sts2_engine::PileName::Draw,
                                preferred: sts2_engine::CardFilter::NotInnate,
                            },
                            then: vec![Command::Card(sts2_engine::CardCmd::Exhaust {
                                cards: sts2_engine::CardSelector::Chosen,
                            })],
                        }],
                    },
                    Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::Not(Box::new(Condition::PlayerTurnIs(1))),
                        then: vec![Command::Select {
                            cards: sts2_engine::CardSelector::FirstInPilePreferring {
                                pile: sts2_engine::PileName::Draw,
                                preferred: sts2_engine::CardFilter::Any,
                            },
                            then: vec![Command::Card(sts2_engine::CardCmd::Exhaust {
                                cards: sts2_engine::CardSelector::Chosen,
                            })],
                        }],
                    },
                    own_power("POWER.STRENGTH_POWER", Amount::Fixed(1)),
                ],
            )],
            ..obtained_only("RELIC.TOASTY_MITTENS", RelicRarity::Ancient)
        },
    ]
}

/// The ancients that put something on a reward screen, or hand a set over
/// where they are taken.
#[allow(clippy::too_many_lines, reason = "one entry per relic")]
fn reward_relics() -> Vec<RelicDefinition> {
    vec![
        // Every elite pays a relic on top of what it already pays.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::TryModifyRewards,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::CurrentRoomIs(sts2_engine::ResolvedRoom::Elite),
                    then: vec![Command::Rewards(sts2_engine::RewardsCmd::AddRelic)],
                }],
            )],
            ..obtained_only("RELIC.BLACK_STAR", RelicRarity::Ancient)
        },
        // Two relics on the first act's boss, once in the run. The boolean the
        // relic writes down is what stops it happening twice.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::TryModifyRewards,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::All(vec![
                        Condition::CurrentRoomIs(sts2_engine::ResolvedRoom::Boss),
                        Condition::CurrentActIs(0),
                        Condition::Not(Box::new(Condition::ListeningRelicIsLatched)),
                    ]),
                    then: vec![
                        Command::Rewards(sts2_engine::RewardsCmd::AddRelic),
                        Command::Rewards(sts2_engine::RewardsCmd::AddRelic),
                        latch(true),
                    ],
                }],
            )],
            latch_property: Some("HasTriggered".to_owned()),
            ..obtained_only("RELIC.LAVA_ROCK", RelicRarity::Ancient)
        },
        // A capsule hands over one relic reward of its own.
        RelicDefinition {
            on_obtained: vec![
                Command::Rewards(sts2_engine::RewardsCmd::AddRelic),
                Command::Rewards(sts2_engine::RewardsCmd::OfferAdded),
            ],
            ..obtained_only("RELIC.SMALL_CAPSULE", RelicRarity::Ancient)
        },
        // A bell is a curse for one relic of each rolled rarity. The rarities
        // are named rather than rolled, so nothing is rolled for them.
        upon_pickup(RelicDefinition {
            on_obtained: vec![
                joins_the_deck("CARD.CURSE_OF_THE_BELL"),
                Command::Rewards(sts2_engine::RewardsCmd::AddRelicOfRarity(
                    RelicRarity::Common,
                )),
                Command::Rewards(sts2_engine::RewardsCmd::AddRelicOfRarity(
                    RelicRarity::Uncommon,
                )),
                Command::Rewards(sts2_engine::RewardsCmd::AddRelicOfRarity(RelicRarity::Rare)),
                Command::Rewards(sts2_engine::RewardsCmd::OfferAdded),
            ],
            ..obtained_only("RELIC.CALLING_BELL", RelicRarity::Ancient)
        }),
        // A large capsule takes two relics off the front of the bag outright,
        // and puts a starter strike and a starter defend back in the deck.
    ]
}

/// The relics whose bodies name something the walking character's own model
/// answers, which is why they are built for a character rather than declared
/// once.
pub(crate) fn character_relics(character: &sts2_core::ModelId) -> Vec<RelicDefinition> {
    vec![
        // A large capsule hands over two relics off the front of its owner's
        // own bag, and puts the character's own basic strike and basic defend
        // back in the deck — the first basic card of that pool carrying the
        // tag, which for every character of the build is the one card of that
        // name.
        RelicDefinition {
            on_obtained: vec![
                Command::Relic(RelicCmd::ObtainFromTheBag),
                Command::Relic(RelicCmd::ObtainFromTheBag),
                joins_the_deck(&character_basic(character, "STRIKE")),
                joins_the_deck(&character_basic(character, "DEFEND")),
            ],
            ..obtained_only("RELIC.LARGE_CAPSULE", RelicRarity::Ancient)
        },
    ]
}

/// The one basic card of a character's own pool carrying this tag, named the
/// way the build's pools name it.
fn character_basic(character: &sts2_core::ModelId, tag: &str) -> String {
    format!("CARD.{tag}_{}", character.entry())
}

/// The ancients that answer the potion belt.
fn potion_relics() -> Vec<RelicDefinition> {
    vec![
        // One more slot and two potions, each taking the first free slot.
        upon_pickup(RelicDefinition {
            on_obtained: vec![
                Command::Player(PlayerCmd::GainMaxPotionCount(Amount::Fixed(1))),
                Command::Potion(sts2_engine::PotionCmd::ProcureRolledBatch {
                    count: Amount::Fixed(2),
                    into_the_new_slots: false,
                }),
            ],
            ..obtained_only("RELIC.PHIAL_HOLSTER", RelicRarity::Ancient)
        }),
        // Four more slots and four potions, and each of those goes into one of
        // the slots the belt has just grown rather than into the first free
        // one.
        upon_pickup(RelicDefinition {
            on_obtained: vec![
                Command::Player(PlayerCmd::GainMaxPotionCount(Amount::Fixed(4))),
                Command::Potion(sts2_engine::PotionCmd::ProcureRolledBatch {
                    count: Amount::Fixed(4),
                    into_the_new_slots: true,
                }),
            ],
            ..obtained_only("RELIC.ALCHEMICAL_COFFER", RelicRarity::Ancient)
        }),
        // A frond fills whatever the belt has room for as every fight opens.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::BeforeCombatStart,
                vec![Command::Potion(sts2_engine::PotionCmd::FillTheBelt)],
            )],
            ..obtained_only("RELIC.DELICATE_FROND", RelicRarity::Ancient)
        },
        // A sozu pays an energy, and no potion ever reaches the belt again.
        RelicDefinition {
            refuses_potions: true,
            hooks: vec![pays_an_energy(1)],
            ..obtained_only("RELIC.SOZU", RelicRarity::Ancient)
        },
    ]
}

/// The ancients that keep a count of their own across a run or a fight.
#[allow(clippy::too_many_lines, reason = "one entry per relic")]
fn counting_relics() -> Vec<RelicDefinition> {
    vec![
        // A pumpkin candle is kindled for five fights the moment it is taken
        // and again every time its owner kindles it at a rest site, pays an
        // energy while it is lit, and burns one fight of kindling down at the
        // end of every combat.
        //
        // The build also turns the relic's status light off once the count has
        // run out, which is presentation: a dimmed relic is on the listener
        // list all the same.
        RelicDefinition {
            counter_property: Some("KindleCount".to_owned()),
            counter_kind: Some(sts2_engine::CounterKind::Ledger),
            on_obtained: vec![Command::Relic(RelicCmd::ModifyListeningCounter(
                Amount::Fixed(5),
            ))],
            adds_rest_site_option: Some(RestSiteOffer::of(RestSiteOption::Kindle)),
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::ModifyMaxEnergy,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningRelicCounterAtLeast(1),
                        then: vec![Command::Modify(ModifierCmd::Add(Amount::Fixed(1)))],
                    }],
                ),
                // The count stops at nothing rather than running below it.
                ModifierHook::every_turn(
                    HookEvent::AfterCombatEnd,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningRelicCounterAtLeast(1),
                        then: vec![Command::Relic(RelicCmd::ModifyListeningCounter(
                            Amount::Fixed(-1),
                        ))],
                    }],
                ),
            ],
            ..obtained_only("RELIC.PUMPKIN_CANDLE", RelicRarity::Ancient)
        },
        // A meat cleaver puts a cook option on the rest site: two cards out of
        // the deck for nine more maximum HP.
        RelicDefinition {
            adds_rest_site_option: Some(RestSiteOffer::of(RestSiteOption::Cook)),
            ..obtained_only("RELIC.MEAT_CLEAVER", RelicRarity::Ancient)
        },
        // An iron club counts every card its owner plays and draws on every
        // fourth. The count is the relic's own and carries across fights.
        RelicDefinition {
            counter_property: Some("CardsPlayed".to_owned()),
            counter_kind: Some(sts2_engine::CounterKind::Cycle { wraps_at: 4 }),
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterCardPlayed,
                vec![
                    Command::Relic(RelicCmd::ModifyListeningCounter(Amount::Fixed(1))),
                    Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningRelicCounterMultipleOf(4),
                        then: vec![draw(1)],
                    },
                ],
            )],
            ..obtained_only("RELIC.IRON_CLUB", RelicRarity::Ancient)
        },
        // A throwing axe plays the first card of every fight twice. Its
        // boolean is set by the pass over whoever changed the count, which is
        // what stops it answering a second time.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterRoomEntered,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::CurrentRoomIsCombat,
                        then: vec![latch(false)],
                    }],
                ),
                ModifierHook::every_turn(
                    HookEvent::ModifyCardPlayCount,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::Not(Box::new(Condition::ListeningRelicIsLatched)),
                        then: vec![Command::Modify(ModifierCmd::Add(Amount::Fixed(1)))],
                    }],
                ),
                ModifierHook::every_turn(HookEvent::AfterModifyingCardPlayCount, vec![latch(true)]),
                ModifierHook::every_turn(HookEvent::AfterCombatEnd, vec![latch(false)]),
            ],
            ..obtained_only("RELIC.THROWING_AXE", RelicRarity::Ancient)
        },
        // A war hammer upgrades four of the deck's upgradable cards every time
        // an elite goes down.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterCombatVictory,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::CurrentRoomIs(sts2_engine::ResolvedRoom::Elite),
                    then: vec![Command::Card(sts2_engine::CardCmd::Upgrade {
                        cards: sts2_engine::CardSelector::StablyShuffledFromDeck {
                            filter: sts2_engine::CardFilter::Upgradable,
                            take: Amount::Fixed(4),
                            stream: sts2_engine::ShuffleStream::Niche,
                        },
                    })],
                }],
            )],
            ..obtained_only("RELIC.WAR_HAMMER", RelicRarity::Ancient)
        },
        // A fishing rod counts the monster rooms alone, and upgrades one card
        // it rolls for on every third.
        RelicDefinition {
            counter_property: Some("CombatsSeen".to_owned()),
            counter_kind: Some(sts2_engine::CounterKind::Cycle { wraps_at: 3 }),
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterCombatEnd,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::CurrentRoomIs(sts2_engine::ResolvedRoom::Monster),
                    then: vec![
                        Command::Relic(RelicCmd::ModifyListeningCounter(Amount::Fixed(1))),
                        Command::When {
                            otherwise: Vec::new(),
                            condition: Condition::ListeningRelicCounterMultipleOf(3),
                            then: vec![Command::Card(sts2_engine::CardCmd::Upgrade {
                                cards: sts2_engine::CardSelector::RolledFromDeck {
                                    filter: sts2_engine::CardFilter::Upgradable,
                                    stream: sts2_engine::ShuffleStream::Niche,
                                },
                            })],
                        },
                    ],
                }],
            )],
            ..obtained_only("RELIC.FISHING_ROD", RelicRarity::Ancient)
        },
        // A humidifier is five more maximum HP every time its owner rests.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterRestSiteHeal,
                vec![Command::Creature(CreatureCmd::GainMaxHp {
                    target: CreatureSelector::Owner,
                    amount: Amount::Fixed(5),
                })],
            )],
            ..obtained_only("RELIC.STONE_HUMIDIFIER", RelicRarity::Ancient)
        },
        // A crossbow conjures one of the character's own attacks into the hand
        // as every turn opens, free for that turn.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    vec![Command::CardPile(CardPileCmd::AddGeneratedFromPool {
                        pool: sts2_engine::CardPoolRef::OwnCharacter,
                        count: Amount::Fixed(1),
                        distinct: true,
                        filter: sts2_engine::CardFilter::OfType(sts2_engine::CardType::Attack),
                        pile: sts2_engine::PileName::Hand,
                        position: sts2_engine::DrawPilePosition::Bottom,
                        excludes_source: false,
                        upgraded: false,
                        temporary_cost: Some(sts2_engine::TemporaryCost::free_this_turn()),
                        ethereal: false,
                        rolled_one_at_a_time: false,
                    })],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..obtained_only("RELIC.CROSSBOW", RelicRarity::Ancient)
        },
        // A jeweled mask pulls one power out of the draw pile into the opening
        // hand, free for the turn.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::BeforeHandDraw,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::PlayerTurnAtMost(1),
                    then: vec![Command::Select {
                        cards: sts2_engine::CardSelector::RandomFromPile {
                            pile: sts2_engine::PileName::Draw,
                            filter: sts2_engine::CardFilter::OfType(sts2_engine::CardType::Power),
                            fallback: None,
                        },
                        then: vec![
                            Command::Card(sts2_engine::CardCmd::SetTemporaryCost {
                                cards: sts2_engine::CardSelector::Chosen,
                                lifetime: sts2_engine::TemporaryCost::free_this_turn(),
                            }),
                            Command::CardPile(CardPileCmd::MoveToPile {
                                cards: sts2_engine::CardSelector::Chosen,
                                pile: sts2_engine::PileName::Hand,
                                position: sts2_engine::DrawPilePosition::Bottom,
                            }),
                        ],
                    }],
                }],
            )],
            ..obtained_only("RELIC.JEWELED_MASK", RelicRarity::Ancient)
        },
        // A philosopher's stone pays an energy and arms the other side: every
        // enemy standing as the fight's room is entered, and every one that
        // arrives after.
        RelicDefinition {
            hooks: vec![
                pays_an_energy(1),
                ModifierHook::every_turn(
                    HookEvent::AfterRoomEntered,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::CurrentRoomIsCombat,
                        then: vec![Command::Power(PowerCmd::Apply {
                            marks: None,
                            power: id("POWER.STRENGTH_POWER"),
                            target: CreatureSelector::AllOpponents,
                            applier: None,
                            amount: Amount::Fixed(1),
                        })],
                    }],
                ),
                ModifierHook::every_turn(
                    HookEvent::AfterCreatureAddedToCombat,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::AnnouncedCreatureIsAnOpponent,
                        then: vec![Command::Power(PowerCmd::Apply {
                            marks: None,
                            power: id("POWER.STRENGTH_POWER"),
                            target: CreatureSelector::Announced,
                            applier: None,
                            amount: Amount::Fixed(1),
                        })],
                    }],
                ),
            ],
            ..obtained_only("RELIC.PHILOSOPHERS_STONE", RelicRarity::Ancient)
        },
    ]
}

/// The ancients that answer inside a fight and need nothing beyond the passes
/// this baseline already runs.
#[allow(clippy::too_many_lines, reason = "one entry per relic")]
fn ancient_combat_relics() -> Vec<RelicDefinition> {
    vec![
        // Seven block as its owner's turn opens, and nothing scales it.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(HookEvent::AfterSideTurnStart, vec![own_block(7)])
                    .when(HookCondition::OwnerIsParticipant),
            ],
            ..obtained_only("RELIC.SAI", RelicRarity::Ancient)
        },
        // Four energy on the opening turn alone.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::PlayerTurnAtMost(1),
                        then: vec![gain_energy(4)],
                    }],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..obtained_only("RELIC.VERY_HOT_COCOA", RelicRarity::Ancient)
        },
        // Five gold buys an energy as every turn opens, while there is gold to
        // spend.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::PlayerGoldAtLeast(5),
                        then: vec![
                            gain_energy(1),
                            Command::Player(PlayerCmd::LoseGold(Amount::Fixed(5))),
                        ],
                    }],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..obtained_only("RELIC.SEAL_OF_GOLD", RelicRarity::Ancient)
        },
        // A turn that ended with energy left over pays two on the next one.
        // What the relic writes down is its own boolean, and a fight ending
        // clears it.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::BeforeSideTurnEnd,
                    vec![
                        latch(false),
                        Command::When {
                            otherwise: Vec::new(),
                            condition: Condition::PlayerHasEnergy,
                            then: vec![latch(true)],
                        },
                    ],
                )
                .when(HookCondition::OwnerIsParticipant),
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningRelicIsLatched,
                        then: vec![gain_energy(2)],
                    }],
                )
                .when(HookCondition::OwnerIsParticipant),
                ModifierHook::every_turn(HookEvent::AfterCombatEnd, vec![latch(false)]),
            ],
            ..obtained_only("RELIC.PAELS_TEARS", RelicRarity::Ancient)
        },
        // One more card on every hand draw.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::ModifyHandDraw,
                vec![Command::Modify(ModifierCmd::Add(Amount::Fixed(1)))],
            )],
            ..obtained_only("RELIC.PAELS_BLOOD", RelicRarity::Ancient)
        },
        // A luminesce in the opening hand, put there before the hand is drawn.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::BeforeHandDraw,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::PlayerTurnIs(1),
                    then: vec![Command::CardPile(CardPileCmd::AddGeneratedToCombat {
                        enchantment: None,
                        card: CardOrigin::Model(id("CARD.LUMINESCE")),
                        grown_by: Amount::Fixed(0),
                        count: Amount::Fixed(1),
                        pile: sts2_engine::PileName::Hand,
                        ethereal: false,
                        retain: false,
                        upgraded: false,
                        temporary_cost: None,
                        position: sts2_engine::DrawPilePosition::Bottom,
                    })],
                }],
            )],
            ..obtained_only("RELIC.RADIANT_PEARL", RelicRarity::Ancient)
        },
        // An energy for three dazed, which are shuffled into the draw pile
        // before the opening hand is drawn.
        RelicDefinition {
            hooks: vec![
                pays_an_energy(1),
                ModifierHook::every_turn(
                    HookEvent::BeforeHandDraw,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::PlayerTurnIs(1),
                        then: vec![Command::CardPile(CardPileCmd::AddGeneratedToCombat {
                            enchantment: None,
                            card: CardOrigin::Model(id("CARD.DAZED")),
                            grown_by: Amount::Fixed(0),
                            count: Amount::Fixed(3),
                            pile: sts2_engine::PileName::Draw,
                            ethereal: false,
                            retain: false,
                            upgraded: false,
                            temporary_cost: None,
                            position: sts2_engine::DrawPilePosition::Random,
                        })],
                    }],
                ),
            ],
            ..obtained_only("RELIC.BLESSED_ANTLER", RelicRarity::Ancient)
        },
        // An energy, and no gold ever again: the gold pass answers with nothing
        // whatever it was handed.
        RelicDefinition {
            hooks: vec![
                pays_an_energy(1),
                ModifierHook::every_turn(
                    HookEvent::ModifyGoldGained,
                    vec![Command::Modify(ModifierCmd::Set(Amount::Fixed(0)))],
                ),
            ],
            ..obtained_only("RELIC.ECTOPLASM", RelicRarity::Ancient)
        },
        // An energy from the third turn of every fight on.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::ModifyMaxEnergy,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::PlayerTurnAtLeast(3),
                    then: vec![Command::Modify(ModifierCmd::Add(Amount::Fixed(1)))],
                }],
            )],
            ..obtained_only("RELIC.PAELS_FLESH", RelicRarity::Ancient)
        },
    ]
}

/// The exact number of cards a relic asks to transform: the screen takes that
/// many and needs no confirming, because the minimum and the maximum it is
/// given are the same number.
fn transforms_from_the_deck(
    count: usize,
    semantics: ChoiceSemantics,
    then: Vec<Command>,
) -> Command {
    Command::CardSelect(sts2_engine::CardSelectCmd::FromDeckForTransformation {
        semantics,
        prefs: sts2_engine::SelectorPrefs::exactly(count),
        then,
    })
}

/// Whatever the enclosing selection picked is transformed, with the replacement
/// rolled out of each card's own options.
fn rolls_a_replacement(
    stream: sts2_engine::TransformStream,
    upgraded: bool,
    one_at_a_time: bool,
) -> Command {
    Command::Card(sts2_engine::CardCmd::TransformDeckCards {
        cards: sts2_engine::CardSelector::Chosen,
        replacement: sts2_engine::CardTransformReplacement::Rolled { stream, upgraded },
        one_at_a_time,
    })
}

/// The ancients that transform deck cards against a pool.
fn transforming_relics() -> Vec<RelicDefinition> {
    vec![
        // An astrolabe transforms three chosen cards and upgrades what each one
        // became. The build works one card out and transforms it before it
        // looks at the next, so the deck takes the replacements in the order
        // the screen named them.
        upon_pickup(RelicDefinition {
            on_obtained: vec![transforms_from_the_deck(
                3,
                ChoiceSemantics::known(vec![
                    ChoiceOp::new(ChoiceOperationKind::Transform, ChoiceScope::Run).rolled(),
                    ChoiceOp::new(ChoiceOperationKind::Upgrade, ChoiceScope::Run)
                        .on(ChoiceSubject::Replacement),
                ]),
                vec![rolls_a_replacement(
                    sts2_engine::TransformStream::Niche,
                    true,
                    true,
                )],
            )],
            ..obtained_only("RELIC.ASTROLABE", RelicRarity::Ancient)
        }),
        // A new leaf transforms one chosen card and leaves it as it was rolled.
        upon_pickup(RelicDefinition {
            on_obtained: vec![transforms_from_the_deck(
                1,
                ChoiceSemantics::known(vec![
                    ChoiceOp::new(ChoiceOperationKind::Transform, ChoiceScope::Run).rolled(),
                ]),
                vec![rolls_a_replacement(
                    sts2_engine::TransformStream::Niche,
                    false,
                    true,
                )],
            )],
            ..obtained_only("RELIC.NEW_LEAF", RelicRarity::Ancient)
        }),
        // A Pandora's box turns every starter strike and defend of the deck
        // into a rolled card. Nothing is chosen, so the whole set is one
        // transform: the replacements join in the order their originals stood.
        upon_pickup(RelicDefinition {
            on_obtained: vec![Command::Card(sts2_engine::CardCmd::TransformDeckCards {
                cards: sts2_engine::CardSelector::DeckCards {
                    filters: vec![
                        sts2_engine::CardFilter::BasicStrikeOrDefend,
                        sts2_engine::CardFilter::Removable,
                    ],
                },
                replacement: sts2_engine::CardTransformReplacement::Rolled {
                    stream: sts2_engine::TransformStream::Niche,
                    upgraded: false,
                },
                one_at_a_time: false,
            })],
            ..obtained_only("RELIC.PANDORAS_BOX", RelicRarity::Ancient)
        }),
        // A leafy poultice costs twelve maximum HP and turns the deck's first
        // starter strike and first starter defend into rolled cards, on the
        // player's own transformation stream.
        RelicDefinition {
            on_obtained: vec![
                Command::Creature(CreatureCmd::LoseMaxHp {
                    target: CreatureSelector::Owner,
                    amount: Amount::Fixed(12),
                }),
                Command::Card(sts2_engine::CardCmd::TransformDeckCards {
                    cards: sts2_engine::CardSelector::FirstInDeckOfEach {
                        filters: vec![
                            sts2_engine::CardFilter::BasicOfTag(sts2_engine::CardTag::Strike),
                            sts2_engine::CardFilter::BasicOfTag(sts2_engine::CardTag::Defend),
                        ],
                    },
                    replacement: sts2_engine::CardTransformReplacement::Rolled {
                        stream: sts2_engine::TransformStream::Transformations,
                        upgraded: false,
                    },
                    one_at_a_time: false,
                }),
            ],
            ..obtained_only("RELIC.LEAFY_POULTICE", RelicRarity::Ancient)
        },
        // Claws turns up to six chosen cards into Mauls, each one upgraded
        // where the card it stands in for was and carrying whatever enchantment
        // it wore, where the enchantment will go on a Maul. The replacement is
        // named, so the transform rolls nothing at all.
        RelicDefinition {
            on_obtained: vec![Command::CardSelect(
                sts2_engine::CardSelectCmd::FromDeckForTransformation {
                    semantics: ChoiceSemantics::known(vec![
                        ChoiceOp::new(ChoiceOperationKind::Transform, ChoiceScope::Run)
                            .model(id("CARD.MAUL"))
                            .inheriting(true, true),
                    ]),
                    prefs: sts2_engine::SelectorPrefs::between(0, 6),
                    then: vec![Command::Card(sts2_engine::CardCmd::TransformDeckCards {
                        cards: sts2_engine::CardSelector::Chosen,
                        replacement: sts2_engine::CardTransformReplacement::Named {
                            model: id("CARD.MAUL"),
                            keeps_upgrade: true,
                            keeps_enchantment: true,
                        },
                        one_at_a_time: false,
                    })],
                },
            )],
            ..obtained_only("RELIC.CLAWS", RelicRarity::Ancient)
        },
    ]
}

/// One starter card per character, and the ancient card an archaic tooth turns
/// it into. Only the Ironclad's pair is reachable at this baseline.
fn transcendence_upgrades() -> Vec<(sts2_core::ModelId, sts2_core::ModelId)> {
    vec![
        (id("CARD.BASH"), id("CARD.BREAK")),
        (id("CARD.NEUTRALIZE"), id("CARD.SUPPRESS")),
        (id("CARD.UNLEASH"), id("CARD.PROTECTOR")),
        (id("CARD.FALLING_STAR"), id("CARD.METEOR_SHOWER")),
        (id("CARD.DUALCAST"), id("CARD.QUADCAST")),
    ]
}

/// A relic registered for the offer alone: its rarity is the build's, and what
/// it does as it is taken is not written down, so taking it stops.
/// The four skins a byrdpip is rolled from.
fn skin_of_four() -> sts2_engine::RolledSkin {
    sts2_engine::RolledSkin {
        property: "Skin".to_owned(),
        options: ["version1", "version2", "version3", "version4"]
            .map(ToOwned::to_owned)
            .to_vec(),
    }
}

/// The relics behind Neow's options, in the order the event lays them out,
/// less the two a single-player run is never offered there: the massive scroll,
/// which only a multiplayer run may hold, and the bones themselves, which the
/// event drops by name.
fn neow_option_relics() -> Vec<sts2_core::ModelId> {
    [
        "RELIC.CURSED_PEARL",
        "RELIC.HEFTY_TABLET",
        "RELIC.LARGE_CAPSULE",
        "RELIC.LEAFY_POULTICE",
        "RELIC.PRECARIOUS_SHEARS",
        "RELIC.SILKEN_TRESS",
        "RELIC.SILVER_CRUCIBLE",
        "RELIC.ARCANE_SCROLL",
        "RELIC.BOOMING_CONCH",
        "RELIC.FISHING_ROD",
        "RELIC.GOLDEN_PEARL",
        "RELIC.KALEIDOSCOPE",
        "RELIC.LEAD_PAPERWEIGHT",
        "RELIC.LOST_COFFER",
        "RELIC.NEOWS_TORMENT",
        "RELIC.NEW_LEAF",
        "RELIC.PHIAL_HOLSTER",
        "RELIC.PRECISE_SCISSORS",
        "RELIC.SCROLL_BOXES",
        "RELIC.WINGED_BOOTS",
        "RELIC.LAVA_ROCK",
        "RELIC.NEOWS_TALISMAN",
        "RELIC.NUTRITIOUS_OYSTER",
        "RELIC.POMANDER",
        "RELIC.SMALL_CAPSULE",
        "RELIC.STONE_HUMIDIFIER",
    ]
    .into_iter()
    .map(id)
    .collect()
}

fn offered_only(model_id: &str) -> RelicDefinition {
    RelicDefinition {
        on_obtained: vec![Command::Unsupported(id(model_id))],
        ..obtained_only(model_id, RelicRarity::Ancient)
    }
}

/// The relics Neow offers, whose whole effect is what they do as they are
/// taken.
#[allow(clippy::too_many_lines, reason = "one entry per relic")]
fn neow_relics() -> Vec<RelicDefinition> {
    vec![
        // Cursed Pearl pays 333 gold for a Greed curse, both as it is
        // obtained, in that order.
        RelicDefinition {
            on_obtained: vec![
                Command::CardPile(CardPileCmd::AddToDeck {
                    card: CardOrigin::Model(id("CARD.GREED")),
                }),
                Command::Player(PlayerCmd::GainGold(Amount::Fixed(333))),
            ],
            ..obtained_only("RELIC.CURSED_PEARL", RelicRarity::Ancient)
        },
        // A lost coffer hands over a card offer of three off its owner's own
        // pool and a potion. It adds the card offer first and the set is shown
        // in the build's own reward order, so the potion comes first.
        upon_pickup(RelicDefinition {
            on_obtained: vec![
                Command::Rewards(RewardsCmd::AddOwnCardOffer { count: 3 }),
                Command::Rewards(RewardsCmd::AddPotion),
                Command::Rewards(RewardsCmd::OfferAdded),
            ],
            ..obtained_only("RELIC.LOST_COFFER", RelicRarity::Ancient)
        }),
        // An arcane scroll makes one rare card of its owner's own pool and puts
        // it straight in the deck. Uniform odds over a pool the filter has cut
        // to one rarity means one draw and nothing else: no rarity is rolled,
        // asking for no upgrade roll spares the upgrade draw.
        RelicDefinition {
            on_obtained: vec![Command::CardPile(CardPileCmd::AddCreatedCardsToDeck {
                creation: creates(1, CardRarity::Rare),
            })],
            ..obtained_only("RELIC.ARCANE_SCROLL", RelicRarity::Ancient)
        },
        // A choices paradox lays five cards of its owner's own pool out on the
        // opening turn of every fight and puts whichever is picked into the
        // hand, keeping itself there. The build puts Retain on all five before
        // the screen and only the picked one reaches a pile, so the keyword
        // rides on the add.
        RelicDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterPlayerTurnStart,
                    vec![Command::CardSelect(
                        CardSelectCmd::SimpleGridOfGeneratedCards {
                            semantics: ChoiceSemantics::known(vec![
                                ChoiceOp::new(ChoiceOperationKind::Gain, ChoiceScope::Combat)
                                    .to(ChoiceDestination::Hand),
                                ChoiceOp::new(
                                    ChoiceOperationKind::GrantKeyword,
                                    ChoiceScope::Combat,
                                )
                                .keyword(sts2_engine::CardKeyword::Retain)
                                .lasting(ChoiceDuration::Combat),
                            ]),
                            pool: sts2_engine::CardPoolRef::OwnCharacter,
                            count: 5,
                            filter: CardFilter::Any,
                            prefs: SelectorPrefs::exactly(1),
                            then: vec![Command::CardPile(CardPileCmd::AddGeneratedToCombat {
                                enchantment: None,
                                card: CardOrigin::Chosen,
                                grown_by: Amount::Fixed(0),
                                count: Amount::Fixed(1),
                                pile: sts2_engine::PileName::Hand,
                                ethereal: false,
                                retain: true,
                                upgraded: false,
                                temporary_cost: None,
                                position: sts2_engine::DrawPilePosition::Bottom,
                            })],
                        },
                    )],
                )
                .timed(HookTiming::FirstTurnOnly),
            ],
            ..obtained_only("RELIC.CHOICES_PARADOX", RelicRarity::Ancient)
        },
        // A sere talon charges two curses and hands over three wishes.
        upon_pickup(RelicDefinition {
            on_obtained: vec![
                Command::CardPile(CardPileCmd::AddCursesToDeck {
                    pool: id("CARD_POOL.CURSE_CARD_POOL"),
                    count: 2,
                }),
                Command::CardPile(CardPileCmd::AddToDeck {
                    card: CardOrigin::Model(id("CARD.WISH")),
                }),
                Command::CardPile(CardPileCmd::AddToDeck {
                    card: CardOrigin::Model(id("CARD.WISH")),
                }),
                Command::CardPile(CardPileCmd::AddToDeck {
                    card: CardOrigin::Model(id("CARD.WISH")),
                }),
            ],
            ..obtained_only("RELIC.SERE_TALON", RelicRarity::Ancient)
        }),
        // Scroll boxes lay out two bundles of three — two commons and an
        // uncommon each, all six different — and whichever bundle is picked
        // joins the deck whole.
        RelicDefinition {
            on_obtained: vec![Command::CardSelect(CardSelectCmd::ChooseAGeneratedBundle {
                semantics: ChoiceSemantics::known(vec![
                    ChoiceOp::new(ChoiceOperationKind::Gain, ChoiceScope::Run)
                        .to(ChoiceDestination::RunDeck),
                ]),
                pools: vec![CardPoolRef::OwnCharacter],
                count: 2,
                commons: 2,
                uncommons: 1,
                then: vec![Command::CardPile(CardPileCmd::AddToDeck {
                    card: CardOrigin::Chosen,
                })],
            })],
            ..obtained_only("RELIC.SCROLL_BOXES", RelicRarity::Ancient)
        },
        // A driftwood lets every card offer of a set be rerolled once. The
        // body answers true for its owner whether or not the set held one.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::TryModifyRewardsLate,
                vec![Command::Rewards(RewardsCmd::AllowRerollingCardOffers)],
            )],
            ..obtained_only("RELIC.DRIFTWOOD", RelicRarity::Ancient)
        },
        // A glass eye hands over five card offers of three at a time, one
        // rarity each: two common, two uncommon, one rare.
        RelicDefinition {
            on_obtained: [
                CardRarity::Common,
                CardRarity::Common,
                CardRarity::Uncommon,
                CardRarity::Uncommon,
                CardRarity::Rare,
            ]
            .into_iter()
            .map(|rarity| {
                Command::Rewards(RewardsCmd::AddCreatedCardOffer {
                    creation: CardCreation {
                        // A card reward marks whatever it was built with as
                        // one.
                        flags: sts2_engine::CardCreationFlags {
                            is_card_reward: true,
                            ..creates(3, rarity).flags
                        },
                        ..creates(3, rarity)
                    },
                })
            })
            .chain(std::iter::once(Command::Rewards(RewardsCmd::OfferAdded)))
            .collect(),
            ..obtained_only("RELIC.GLASS_EYE", RelicRarity::Ancient)
        },
        // A hefty tablet lays out three rares to pick one of, and charges an
        // injury for it whether or not one is picked.
        upon_pickup(RelicDefinition {
            on_obtained: vec![Command::CardSelect(CardSelectCmd::ChooseACreatedCard {
                semantics: ChoiceSemantics::known(vec![
                    ChoiceOp::new(ChoiceOperationKind::Gain, ChoiceScope::Run)
                        .to(ChoiceDestination::RunDeck),
                ]),
                creation: creates(3, CardRarity::Rare),
                can_skip: true,
                then: vec![
                    // The card picked is put ahead of the injury and both
                    // join the deck in one go, so skipping leaves the injury
                    // alone.
                    Command::CardPile(CardPileCmd::AddToDeck {
                        card: CardOrigin::Chosen,
                    }),
                    Command::CardPile(CardPileCmd::AddToDeck {
                        card: CardOrigin::Model(id("CARD.INJURY")),
                    }),
                ],
            })],
            ..obtained_only("RELIC.HEFTY_TABLET", RelicRarity::Ancient)
        }),
        // A lead paperweight lays out two colorless cards to pick one of.
        // Nothing spares its upgrade roll, so each of the two costs a draw for
        // the rarity, one for the pick, and one for the upgrade.
        RelicDefinition {
            on_obtained: vec![Command::CardSelect(CardSelectCmd::ChooseACreatedCard {
                semantics: ChoiceSemantics::known(vec![
                    ChoiceOp::new(ChoiceOperationKind::Gain, ChoiceScope::Run)
                        .to(ChoiceDestination::RunDeck),
                ]),
                creation: CardCreation {
                    pools: vec![CardPoolRef::Named(id("CARD_POOL.COLORLESS_CARD_POOL"))],
                    filter: CardFilter::Any,
                    odds: sts2_engine::RarityOdds::RegularEncounter,
                    from_an_encounter: false,
                    count: 2,
                    flags: sts2_engine::CardCreationFlags::default(),
                },
                can_skip: true,
                then: vec![Command::CardPile(CardPileCmd::AddToDeck {
                    card: CardOrigin::Chosen,
                })],
            })],
            ..obtained_only("RELIC.LEAD_PAPERWEIGHT", RelicRarity::Ancient)
        },
    ]
}

/// A non-combat creation with uniform odds over the owner's own pool at one
/// rarity: the shape three of the ancients build a creation with, which is one
/// draw per card and no rarity roll at all.
fn creates(count: usize, rarity: CardRarity) -> CardCreation {
    CardCreation {
        pools: vec![CardPoolRef::OwnCharacter],
        filter: CardFilter::OfRarity(rarity),
        odds: sts2_engine::RarityOdds::Uniform,
        from_an_encounter: false,
        count,
        flags: sts2_engine::CardCreationFlags {
            no_upgrade_roll: true,
            ..sts2_engine::CardCreationFlags::default()
        },
    }
}

/// The ancients whose whole body is an enchantment.
///
/// Each of them either asks the player which deck cards to put one on, puts one
/// on every deck card it can, or puts one over a card reward.
fn enchanting_ancients() -> Vec<RelicDefinition> {
    vec![
        // A beautiful bracelet puts three Swift on three cards of the deck.
        RelicDefinition {
            on_obtained: vec![enchant_chosen_deck_cards(
                "ENCHANTMENT.SWIFT",
                3,
                SelectorPrefs::exactly(3),
            )],
            ..obtained_only("RELIC.BEAUTIFUL_BRACELET", RelicRarity::Ancient)
        },
        // An electric shrymp puts Imbued on one.
        upon_pickup(RelicDefinition {
            on_obtained: vec![enchant_chosen_deck_cards(
                "ENCHANTMENT.IMBUED",
                1,
                SelectorPrefs::exactly(1),
            )],
            ..obtained_only("RELIC.ELECTRIC_SHRYMP", RelicRarity::Ancient)
        }),
        // A tri-boomerang puts Instinct on three.
        RelicDefinition {
            on_obtained: vec![enchant_chosen_deck_cards(
                "ENCHANTMENT.INSTINCT",
                1,
                SelectorPrefs::exactly(3),
            )],
            ..obtained_only("RELIC.TRI_BOOMERANG", RelicRarity::Ancient)
        },
        // Nutritious soup puts Tezcatara's Ember on every basic Strike of the
        // deck, without asking.
        RelicDefinition {
            on_obtained: vec![enchant_every_deck_card(
                "ENCHANTMENT.TEZCATARAS_EMBER",
                1,
                CardFilter::BasicOfTag(CardTag::Strike),
            )],
            ..obtained_only("RELIC.NUTRITIOUS_SOUP", RelicRarity::Ancient)
        },
        // Pael's claw puts Goopy on every deck card it can go on, which is
        // every Defend.
        RelicDefinition {
            on_obtained: vec![enchant_every_deck_card(
                "ENCHANTMENT.GOOPY",
                1,
                CardFilter::Any,
            )],
            ..obtained_only("RELIC.PAELS_CLAW", RelicRarity::Ancient)
        },
        // Glitter puts a Glam over every card of every card reward it can go
        // on.
        RelicDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::TryModifyCardRewardOptionsLate,
                vec![Command::Rewards(RewardsCmd::EnchantCardOptions {
                    enchantment: id("ENCHANTMENT.GLAM"),
                    amount: Amount::Fixed(1),
                    one_rolled_on_the_niche_stream: false,
                })],
            )],
            ..obtained_only("RELIC.GLITTER", RelicRarity::Ancient)
        },
        // Pael's growth marks one deck card as a Clone, and offers a rest-site
        // option that duplicates every card so marked.
        //
        // The screen previews the enchantment as being worth one and the
        // enchant that follows it puts four on, which is what the build
        // does.
        RelicDefinition {
            adds_rest_site_option: Some(RestSiteOffer::of(RestSiteOption::Clone)),
            on_obtained: vec![Command::CardSelect(CardSelectCmd::FromDeckForEnchantment {
                semantics: ChoiceSemantics::known(vec![
                    ChoiceOp::new(ChoiceOperationKind::Enchant, ChoiceScope::Run)
                        .model(id("ENCHANTMENT.CLONE"))
                        .amount(4),
                ]),
                enchantment: id("ENCHANTMENT.CLONE"),
                amount: 1,
                filter: CardFilter::Any,
                shuffled_on: None,
                prefs: SelectorPrefs::exactly(1),
                then: vec![Command::Card(CardCmd::Enchant {
                    cards: CardSelector::Chosen,
                    enchantment: id("ENCHANTMENT.CLONE"),
                    amount: Amount::Fixed(4),
                })],
            })],
            ..obtained_only("RELIC.PAELS_GROWTH", RelicRarity::Ancient)
        },
        // A silken tress costs its owner every coin they hold and then does
        // Glitter's trick once, for one card reward, and is used up.
        RelicDefinition {
            on_obtained: vec![Command::Player(PlayerCmd::LoseGold(Amount::counting(
                sts2_engine::Count::PlayerGold,
            )))],
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::TryModifyCardRewardOptionsLate,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::Not(Box::new(Condition::ListeningRelicIsLatched)),
                        then: vec![Command::Rewards(RewardsCmd::EnchantCardOptions {
                            enchantment: id("ENCHANTMENT.GLAM"),
                            amount: Amount::Fixed(1),
                            one_rolled_on_the_niche_stream: false,
                        })],
                    }],
                ),
                // `AfterModifyingCardRewardOptions` runs over exactly the
                // listeners that changed an option, and the first time it is
                // reached the relic is spent.
                ModifierHook::every_turn(
                    HookEvent::AfterModifyingCardRewardOptions,
                    vec![Command::Relic(RelicCmd::SetListeningLatch(true))],
                ),
            ],
            latch_property: Some("IsUsed".to_owned()),
            ..obtained_only("RELIC.SILKEN_TRESS", RelicRarity::Ancient)
        },
    ]
}
