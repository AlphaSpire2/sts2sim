//! The Silent's whole card pool, transcribed.
//!
//! Eighty-eight models: the four she starts with, the eighty a rolled rarity
//! can offer, the two ancients, and the two multiplayer-only cards no
//! single-player pool ever holds. Each is the run of commands the card issues
//! when it is played, in order, at both the levels it supports.
//!
//! Three things recur across the pool and are worth naming once. A Shiv is a
//! token card the Silent's own cards conjure by the handful, and conjuring them
//! is one add to the hand whether one was asked for or four. Poison is a debuff
//! that costs its holder what it is worth as their turn opens and wears off as
//! it bites. And Sly is a keyword that belongs to the discard itself: a Sly
//! card put into the discard pile plays itself for free once the batch it went
//! down in is finished.

use sts2_engine::{
    Amount, CardCmd, CardDefinition, CardFilter, CardOrigin, CardPileCmd, CardSelectCmd,
    CardSelector, CardTag, CardType, Command, Condition, Count, CreatureCmd, CreatureSelector,
    DrawPilePosition, HookCondition, HookEvent, ModifierHook, PileName, PowerCmd, PowerRef,
    RewardsCmd, SelectorPrefs, TargetType, ValueProps,
};

use crate::cards::{
    apply_power, attack, block, buff, card, costing_x, debuff, defending, draw, energy, exhausting,
    innate_when_upgraded, multi_attack, rarity, striking, sweep, upgrading, upgrading_cost,
};
use crate::id;

/// Innate: the card opens the fight in hand.
fn innate(mut definition: CardDefinition) -> CardDefinition {
    definition.innate = true;
    definition
}

/// Retain: the card is not put down when the hand is flushed.
fn retaining(mut definition: CardDefinition) -> CardDefinition {
    definition.retain = true;
    definition
}

/// Sly: discarding the card plays it for free.
fn sly(mut definition: CardDefinition) -> CardDefinition {
    definition.sly = true;
    definition
}

/// The card gains block, which only what a Nimble can go on reads. It is
/// filled in from the table in `cards.rs` as the registry is built, so nothing
/// here has to say it twice.
///
/// An upgrade that takes the Exhaust keyword back off, which is what upgrading
/// a Scare does.
fn upgrade_drops_exhaust(mut definition: CardDefinition) -> CardDefinition {
    if let Some(upgrade) = definition.upgrade.as_mut() {
        upgrade.drops_exhaust = true;
    }
    definition
}

/// That many Shivs conjured into the owner's hand.
fn shivs(count: i32) -> Command {
    crate::powers::shivs_into_hand(Amount::Fixed(count), false)
}

/// The same, with each of them upgraded on the way in, which is what a Hidden
/// Daggers and a Storm of Steel do to what they just made.
fn upgraded_shivs(count: Amount) -> Command {
    crate::powers::shivs_into_hand(count, true)
}

/// That much poison on the target, worked by the card's own owner.
fn poison(amount: i32) -> Command {
    debuff("POWER.POISON_POWER", amount)
}

/// The same, worked onto every enemy still standing, one at a time.
fn poison_everything(amount: i32) -> Command {
    sweeping_power("POWER.POISON_POWER", amount)
}

/// The power worked onto every enemy that can be hit, one at a time.
fn sweeping_power(power: &str, amount: i32) -> Command {
    apply_power(power, CreatureSelector::AllOpponents, Amount::Fixed(amount))
}

/// Those cards discarded, with nothing drawn back.
fn discard(cards: CardSelector) -> Command {
    Command::Card(CardCmd::DiscardAndDraw {
        cards,
        count: Amount::Fixed(0),
    })
}

/// That many cards picked out of the hand and discarded, taken without a
/// screen when the
/// hand holds no more than that.
fn discard_from_hand(count: usize) -> Command {
    Command::CardSelect(CardSelectCmd::FromHandForDiscard {
        prefs: SelectorPrefs::exactly(count),
        maximum: None,
        exact_count: None,
        filter: CardFilter::Any,
        then: vec![discard(CardSelector::Chosen)],
    })
}

/// The whole hand, put down at once.
fn discard_the_hand() -> Command {
    discard(CardSelector::Pile {
        pile: PileName::Hand,
        filter: CardFilter::Any,
    })
}

/// The Silent's eighty-eight, in her pool's own order.
#[must_use]
pub fn silent_cards() -> Vec<CardDefinition> {
    let mut cards = opening_pool();
    cards.extend(daggers_and_blades());
    cards.extend(poisons());
    cards.extend(the_middle_pool());
    cards.extend(the_late_pool());
    cards
}

/// Abrasive through Calculated Gamble.
#[allow(clippy::too_many_lines, reason = "one entry per card")]
fn opening_pool() -> Vec<CardDefinition> {
    vec![
        // An abrasive works dexterity on first and thorns after it; only the
        // thorns grow with the upgrade.
        sly(upgrading(
            card(
                "CARD.ABRASIVE",
                CardType::Power,
                TargetType::SelfTarget,
                3,
                vec![
                    buff("POWER.DEXTERITY_POWER", 1),
                    buff("POWER.THORNS_POWER", 4),
                ],
            ),
            vec![
                buff("POWER.DEXTERITY_POWER", 1),
                buff("POWER.THORNS_POWER", 6),
            ],
        )),
        upgrading(
            card(
                "CARD.ACCELERANT",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.ACCELERANT_POWER", 1)],
            ),
            vec![buff("POWER.ACCELERANT_POWER", 2)],
        ),
        upgrading(
            card(
                "CARD.ACCURACY",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.ACCURACY_POWER", 4)],
            ),
            vec![buff("POWER.ACCURACY_POWER", 6)],
        ),
        // Acrobatics draws deep and asks for one card back.
        upgrading(
            card(
                "CARD.ACROBATICS",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![draw(3), discard_from_hand(1)],
            ),
            vec![draw(4), discard_from_hand(1)],
        ),
        exhausting(upgrading(
            card(
                "CARD.ADRENALINE",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![energy(1), draw(2)],
            ),
            vec![energy(2), draw(2)],
        )),
        // Upgrading an afterimage makes it innate rather than stronger.
        innate_when_upgraded(upgrading(
            card(
                "CARD.AFTERIMAGE",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.AFTERIMAGE_POWER", 1)],
            ),
            vec![buff("POWER.AFTERIMAGE_POWER", 1)],
        )),
        // Anticipating lends its owner dexterity for the turn. The card's own
        // var points at dexterity, which is what its text reads; the power it
        // actually works is its own.
        upgrading(
            card(
                "CARD.ANTICIPATE",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![buff("POWER.ANTICIPATE_POWER", 2)],
            ),
            vec![buff("POWER.ANTICIPATE_POWER", 3)],
        ),
        // An assassination opens the fight in hand, burns up, and leaves what
        // it hit open — whether or not the blow left anything standing.
        innate(exhausting(upgrading(
            card(
                "CARD.ASSASSINATE",
                CardType::Attack,
                TargetType::AnyEnemy,
                0,
                vec![attack(10), debuff("POWER.VULNERABLE_POWER", 1)],
            ),
            vec![attack(13), debuff("POWER.VULNERABLE_POWER", 2)],
        ))),
        upgrading(
            card(
                "CARD.BACKFLIP",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![block(5), draw(2)],
            ),
            vec![block(8), draw(2)],
        ),
        // A backstab opens the fight in hand and burns up for one heavy swing.
        innate(exhausting(upgrading(
            card(
                "CARD.BACKSTAB",
                CardType::Attack,
                TargetType::AnyEnemy,
                0,
                vec![attack(11)],
            ),
            vec![attack(15)],
        ))),
        upgrading(
            card(
                "CARD.BLUR",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![block(5), buff("POWER.BLUR_POWER", 1)],
            ),
            vec![block(8), buff("POWER.BLUR_POWER", 1)],
        ),
        // A bouncing flask rolls its target fresh for every throw, so it can
        // land on the same thing twice.
        upgrading(
            card(
                "CARD.BOUNCING_FLASK",
                CardType::Skill,
                TargetType::RandomEnemy,
                2,
                vec![bouncing_flask(3)],
            ),
            vec![bouncing_flask(4)],
        ),
        // A bubble does nothing at all to something that is not already
        // poisoned.
        upgrading(
            card(
                "CARD.BUBBLE_BUBBLE",
                CardType::Skill,
                TargetType::AnyEnemy,
                1,
                bubble(9),
            ),
            bubble(12),
        ),
        // Bullet time makes the hand free for the turn and stops the well.
        // A card whose cost is X is passed over.
        upgrading_cost(
            card(
                "CARD.BULLET_TIME",
                CardType::Skill,
                TargetType::SelfTarget,
                3,
                vec![
                    Command::Card(CardCmd::SetTemporaryCost {
                        cards: CardSelector::Pile {
                            pile: PileName::Hand,
                            filter: CardFilter::HasWritableEnergyCost,
                        },
                        lifetime: sts2_engine::TemporaryCost::free_this_turn(),
                    }),
                    buff("POWER.NO_DRAW_POWER", 1),
                ],
            ),
            2,
        ),
        upgrading(
            card(
                "CARD.BURST",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.BURST_POWER", 1)],
            ),
            vec![buff("POWER.BURST_POWER", 2)],
        ),
        // A calculated gamble puts the whole hand down and draws that many
        // back. The count is taken before anything is discarded, and upgrading
        // makes the card keep itself rather than changing what it does.
        retaining_when_upgraded(exhausting(card(
            "CARD.CALCULATED_GAMBLE",
            CardType::Skill,
            TargetType::SelfTarget,
            0,
            vec![Command::Card(CardCmd::DiscardAndDraw {
                cards: CardSelector::Pile {
                    pile: PileName::Hand,
                    filter: CardFilter::Any,
                },
                count: Amount::counting(Count::CardsInPile(PileName::Hand)),
            })],
        ))),
    ]
}

/// An upgrade whose whole effect is to add Retain.
fn retaining_when_upgraded(mut definition: CardDefinition) -> CardDefinition {
    definition.supported_upgrade_levels.push(1);
    let effects = definition.effects.clone();
    definition.upgrade = Some(sts2_engine::CardUpgrade {
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

/// Bouncing Flask, played: one roll on the combat-target stream per throw, and
/// a roll that finds nothing skips that throw rather than ending the loop.
fn bouncing_flask(throws: i32) -> Command {
    Command::Repeat {
        times: Amount::Fixed(throws),
        then: vec![apply_power(
            "POWER.POISON_POWER",
            CreatureSelector::RandomOpponent,
            Amount::Fixed(3),
        )],
    }
}

/// Bubble Bubble, played: the poison lands only on something already carrying
/// some.
fn bubble(amount: i32) -> Vec<Command> {
    vec![Command::When {
        otherwise: Vec::new(),
        condition: Condition::TargetHasPower(id("POWER.POISON_POWER")),
        then: vec![poison(amount)],
    }]
}

/// The cards that make Shivs, and the ones that read them.
#[allow(clippy::too_many_lines, reason = "one entry per card")]
fn daggers_and_blades() -> Vec<CardDefinition> {
    vec![
        // A blade of ink makes Shivs and inks each of them.
        upgrading(
            card(
                "CARD.BLADE_OF_INK",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![inked_shivs(2)],
            ),
            vec![inked_shivs(3)],
        ),
        exhausting(upgrading(
            card(
                "CARD.BLADE_DANCE",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![shivs(3)],
            ),
            vec![shivs(4)],
        )),
        upgrading(
            card(
                "CARD.CLOAK_AND_DAGGER",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![block(6), shivs(1)],
            ),
            vec![block(6), shivs(2)],
        ),
        // A fan of knives changes what every Shiv is aimed at, and deals four
        // of them to start with.
        upgrading(
            card(
                "CARD.FAN_OF_KNIVES",
                CardType::Power,
                TargetType::SelfTarget,
                2,
                vec![buff("POWER.FAN_OF_KNIVES_POWER", 1), shivs(4)],
            ),
            vec![buff("POWER.FAN_OF_KNIVES_POWER", 1), shivs(5)],
        ),
        // Hidden daggers pay two cards of the hand for two Shivs, upgraded when
        // the card is. It declares no upgrade of its own: the branch is inside
        // the body.
        upgrading(
            card(
                "CARD.HIDDEN_DAGGERS",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![discard_from_hand(2), shivs(2)],
            ),
            vec![discard_from_hand(2), upgraded_shivs(Amount::Fixed(2))],
        ),
        upgrading(
            card(
                "CARD.INFINITE_BLADES",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.INFINITE_BLADES_POWER", 1)],
            ),
            vec![buff("POWER.INFINITE_BLADES_POWER", 1)],
        ),
        // A knife trap plays every Shiv that has burned up this fight, at
        // whatever the trap was pointed at. Upgraded, each of them is upgraded
        // first.
        upgrading(
            card(
                "CARD.KNIFE_TRAP",
                CardType::Skill,
                TargetType::AnyEnemy,
                2,
                knife_trap(false),
            ),
            knife_trap(true),
        ),
        striking(upgrading(
            card(
                "CARD.LEADING_STRIKE",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![attack(3), shivs(2)],
            ),
            vec![attack(6), shivs(2)],
        )),
        upgrading(
            card(
                "CARD.PHANTOM_BLADES",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.PHANTOM_BLADES_POWER", 9)],
            ),
            vec![buff("POWER.PHANTOM_BLADES_POWER", 12)],
        ),
        // A storm of steel counts the hand, puts all of it down, and hands back
        // that many Shivs. The count is taken before the discard, so a storm
        // still in hand counts itself.
        upgrading(
            card(
                "CARD.STORM_OF_STEEL",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![storm_of_steel(false)],
            ),
            vec![storm_of_steel(true)],
        ),
        // Up my sleeve deals three Shivs and costs one less for the rest of the
        // fight every time it is played.
        upgrading(
            card(
                "CARD.UP_MY_SLEEVE",
                CardType::Skill,
                TargetType::SelfTarget,
                2,
                vec![shivs(3), cheaper_for_the_fight()],
            ),
            vec![shivs(4), cheaper_for_the_fight()],
        ),
    ]
}

/// That many Shivs, each conjured carrying an Inky worth one.
fn inked_shivs(count: i32) -> Command {
    Command::CardPile(CardPileCmd::AddGeneratedToCombat {
        enchantment: Some(Box::new((id("ENCHANTMENT.INKY"), 1))),
        card: CardOrigin::Model(id("CARD.SHIV")),
        grown_by: Amount::Fixed(0),
        count: Amount::Fixed(count),
        pile: PileName::Hand,
        ethereal: false,
        retain: false,
        upgraded: false,
        temporary_cost: None,
        position: DrawPilePosition::Bottom,
    })
}

/// Knife Trap, played: the Shivs of the exhaust pile, taken as a list before
/// anything moves, each upgraded where the trap is and then played at the
/// trap's own target.
fn knife_trap(upgraded: bool) -> Vec<Command> {
    let mut each = Vec::new();
    if upgraded {
        each.push(Command::Card(CardCmd::Upgrade {
            cards: CardSelector::Chosen,
        }));
    }
    each.push(Command::Card(CardCmd::AutoPlay {
        cards: CardSelector::Chosen,
        at_the_bodys_target: true,
    }));
    vec![Command::ForEachCard {
        cards: CardSelector::Pile {
            pile: PileName::Exhaust,
            filter: CardFilter::Tagged(CardTag::Shiv),
        },
        then: each,
    }]
}

/// Storm of Steel, played: the hand is taken as a list, all of it goes down,
/// and the list's own length is how many Shivs come back.
fn storm_of_steel(upgraded: bool) -> Command {
    Command::Select {
        cards: CardSelector::Pile {
            pile: PileName::Hand,
            filter: CardFilter::Any,
        },
        then: vec![
            discard(CardSelector::Chosen),
            crate::powers::shivs_into_hand(Amount::counting(Count::ChosenCards), upgraded),
        ],
    }
}

/// The card itself costs one less for the rest of the fight.
fn cheaper_for_the_fight() -> Command {
    Command::Card(CardCmd::AddCostThisCombat {
        cards: CardSelector::Source,
        amount: Amount::Fixed(-1),
    })
}

/// The cards that work poison, and the ones that read how much of it is out
/// there.
#[allow(clippy::too_many_lines, reason = "one entry per card")]
fn poisons() -> Vec<CardDefinition> {
    vec![
        upgrading(
            card(
                "CARD.CORROSIVE_WAVE",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.CORROSIVE_WAVE_POWER", 2)],
            ),
            vec![buff("POWER.CORROSIVE_WAVE_POWER", 3)],
        ),
        upgrading(
            card(
                "CARD.DEADLY_POISON",
                CardType::Skill,
                TargetType::AnyEnemy,
                1,
                vec![poison(5)],
            ),
            vec![poison(7)],
        ),
        upgrading(
            card(
                "CARD.ENVENOM",
                CardType::Power,
                TargetType::SelfTarget,
                2,
                vec![buff("POWER.ENVENOM_POWER", 1)],
            ),
            vec![buff("POWER.ENVENOM_POWER", 2)],
        ),
        sly(upgrading(
            card(
                "CARD.HAZE",
                CardType::Skill,
                TargetType::AllEnemies,
                3,
                vec![poison_everything(4)],
            ),
            vec![poison_everything(6)],
        )),
        // A mirage is worth every point of poison the other side is carrying.
        exhausting(upgrading_cost(
            card(
                "CARD.MIRAGE",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![Command::Creature(CreatureCmd::GainBlock {
                    target: CreatureSelector::Owner,
                    amount: Amount::counting(Count::LivingOpponentsPower(id("POWER.POISON_POWER"))),
                    props: ValueProps::NONE,
                })],
            ),
            0,
        )),
        upgrading(
            card(
                "CARD.NOXIOUS_FUMES",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.NOXIOUS_FUMES_POWER", 2)],
            ),
            vec![buff("POWER.NOXIOUS_FUMES_POWER", 3)],
        ),
        // An outbreak burns everything every third time its owner poisons
        // anything. The three is the power's own; the card's repeat var is
        // description.
        upgrading(
            card(
                "CARD.OUTBREAK",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.OUTBREAK_POWER", 11)],
            ),
            vec![buff("POWER.OUTBREAK_POWER", 15)],
        ),
        upgrading(
            card(
                "CARD.POISONED_STAB",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![attack(6), poison(3)],
            ),
            vec![attack(8), poison(4)],
        ),
        retaining(upgrading(
            card(
                "CARD.SNAKEBITE",
                CardType::Skill,
                TargetType::AnyEnemy,
                2,
                vec![poison(7)],
            ),
            vec![poison(10)],
        )),
    ]
}

/// Dagger Spray through Precise Cut: the swings, the draws, and the cards that
/// count what has already happened this turn.
#[allow(clippy::too_many_lines, reason = "one entry per card")]
fn the_middle_pool() -> Vec<CardDefinition> {
    vec![
        // Four to everything, twice over.
        upgrading(
            card(
                "CARD.DAGGER_SPRAY",
                CardType::Attack,
                TargetType::AllEnemies,
                1,
                vec![sweep(4, 2)],
            ),
            vec![sweep(6, 2)],
        ),
        // A dagger throw swings, draws, and asks for a card back.
        upgrading(
            card(
                "CARD.DAGGER_THROW",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![attack(9), draw(1), discard_from_hand(1)],
            ),
            vec![attack(12), draw(1), discard_from_hand(1)],
        ),
        // A dash works its block up before it swings.
        upgrading(
            card(
                "CARD.DASH",
                CardType::Attack,
                TargetType::AnyEnemy,
                2,
                vec![block(10), attack(10)],
            ),
            vec![block(13), attack(13)],
        ),
        defending(upgrading(
            card(
                "CARD.DEFEND_SILENT",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![block(5)],
            ),
            vec![block(8)],
        )),
        upgrading(
            card(
                "CARD.DEFLECT",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![block(4)],
            ),
            vec![block(7)],
        ),
        // A dodge and roll works the same block up again next turn, and what it
        // owes is what the block actually came to.
        upgrading(
            card(
                "CARD.DODGE_AND_ROLL",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![dodge_and_roll(4)],
            ),
            vec![dodge_and_roll(6)],
        ),
        // An echoing slash lands on everything, and buys itself another pass
        // for every creature it kills.
        upgrading(
            card(
                "CARD.ECHOING_SLASH",
                CardType::Attack,
                TargetType::AllEnemies,
                1,
                vec![echoing_slash(10)],
            ),
            vec![echoing_slash(13)],
        ),
        // An escape plan draws one card and pays block only for a skill.
        upgrading(
            card(
                "CARD.ESCAPE_PLAN",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![escape_plan(3)],
            ),
            vec![escape_plan(5)],
        ),
        // Expertise draws the hand back up to six cards, and nothing at all
        // when it already holds that many.
        upgrading(
            card(
                "CARD.EXPERTISE",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![expertise(6)],
            ),
            vec![expertise(7)],
        ),
        // Expose takes the block off what it is aimed at, takes an artifact
        // away, and leaves it open.
        exhausting(upgrading(
            card(
                "CARD.EXPOSE",
                CardType::Skill,
                TargetType::AnyEnemy,
                0,
                expose(2),
            ),
            expose(3),
        )),
        // A finisher swings once for every attack its owner has already
        // finished playing this turn.
        upgrading(
            card(
                "CARD.FINISHER",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![multi_attack(
                    Amount::Fixed(6),
                    Amount::counting(Count::OwnerAttacksPlayedThisTurn),
                )],
            ),
            vec![multi_attack(
                Amount::Fixed(8),
                Amount::counting(Count::OwnerAttacksPlayedThisTurn),
            )],
        ),
        // Flanking makes everyone but the player who threw it hit its target
        // harder. It is multiplayer-only, which keeps it out of every
        // single-player pool, so no run of this baseline holds one.
        rarity(
            upgrading_cost(
                card(
                    "CARD.FLANKING",
                    CardType::Skill,
                    TargetType::AnyEnemy,
                    2,
                    vec![debuff("POWER.FLANKING_POWER", 2)],
                ),
                1,
            ),
            sts2_engine::CardRarity::Uncommon,
        ),
        // Flechettes swing once for every skill in the hand.
        upgrading(
            card(
                "CARD.FLECHETTES",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![multi_attack(Amount::Fixed(5), skills_in_hand())],
            ),
            vec![multi_attack(Amount::Fixed(7), skills_in_hand())],
        ),
        sly(upgrading(
            card(
                "CARD.FLICK_FLACK",
                CardType::Attack,
                TargetType::AllEnemies,
                1,
                vec![sweep(6, 1)],
            ),
            vec![sweep(8, 1)],
        )),
        // A scare shakes everything, and upgrading stops it burning up.
        upgrade_drops_exhaust(exhausting(upgrading(
            card(
                "CARD.SCARE",
                CardType::Skill,
                TargetType::AllEnemies,
                0,
                vec![sweeping_power("POWER.WEAK_POWER", 1)],
            ),
            vec![sweeping_power("POWER.WEAK_POWER", 1)],
        ))),
        upgrading(
            card(
                "CARD.FOOTWORK",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.DEXTERITY_POWER", 2)],
            ),
            vec![buff("POWER.DEXTERITY_POWER", 3)],
        ),
        // A grand finale can only be played on an empty draw pile.
        playable_on_an_empty_draw_pile(upgrading(
            card(
                "CARD.GRAND_FINALE",
                CardType::Attack,
                TargetType::AllEnemies,
                0,
                vec![sweep(60, 1)],
            ),
            vec![sweep(75, 1)],
        )),
        // A hand trick makes one skill of the hand Sly for the turn.
        upgrading(
            card(
                "CARD.HAND_TRICK",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![block(7), hand_trick()],
            ),
            vec![block(10), hand_trick()],
        ),
        upgrading(
            card(
                "CARD.LEG_SWEEP",
                CardType::Skill,
                TargetType::AnyEnemy,
                2,
                vec![block(11), debuff("POWER.WEAK_POWER", 2)],
            ),
            vec![block(14), debuff("POWER.WEAK_POWER", 3)],
        ),
        // A malaise costs X and takes that much strength away and leaves that
        // much weakness. Upgrading is one more of each, worked into the body.
        exhausting(costing_x(upgrading(
            card(
                "CARD.MALAISE",
                CardType::Skill,
                TargetType::AnyEnemy,
                0,
                malaise(0),
            ),
            malaise(1),
        ))),
        upgrading_cost(
            card(
                "CARD.MASTER_PLANNER",
                CardType::Power,
                TargetType::SelfTarget,
                2,
                vec![buff("POWER.MASTER_PLANNER_POWER", 1)],
            ),
            1,
        ),
        // A memento mori is worth four more for every card put down this turn.
        upgrading(
            card(
                "CARD.MEMENTO_MORI",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![multi_attack(
                    Amount::per(9, 4, Count::OwnerCardsDiscardedThisTurn),
                    Amount::Fixed(1),
                )],
            ),
            vec![multi_attack(
                Amount::per(11, 5, Count::OwnerCardsDiscardedThisTurn),
                Amount::Fixed(1),
            )],
        ),
        // A murder is worth one more for every card drawn this fight.
        upgrading_cost(
            card(
                "CARD.MURDER",
                CardType::Attack,
                TargetType::AnyEnemy,
                3,
                vec![multi_attack(
                    Amount::per(1, 1, Count::OwnerCardsDrawnThisCombat),
                    Amount::Fixed(1),
                )],
            ),
            2,
        ),
        upgrading(
            card(
                "CARD.NEUTRALIZE",
                CardType::Attack,
                TargetType::AnyEnemy,
                0,
                vec![attack(3), debuff("POWER.WEAK_POWER", 1)],
            ),
            vec![attack(4), debuff("POWER.WEAK_POWER", 2)],
        ),
        // A nightmare holds a card of the hand and deals three copies of it
        // into the next hand.
        exhausting(upgrading_cost(
            card(
                "CARD.NIGHTMARE",
                CardType::Skill,
                TargetType::SelfTarget,
                3,
                vec![nightmare()],
            ),
            2,
        )),
        exhausting(upgrading(
            card(
                "CARD.PIERCING_WAIL",
                CardType::Skill,
                TargetType::AllEnemies,
                1,
                vec![sweeping_power("POWER.PIERCING_WAIL_POWER", 6)],
            ),
            vec![sweeping_power("POWER.PIERCING_WAIL_POWER", 8)],
        )),
        // A pinpoint costs one less for every skill its owner has played this
        // turn, and one less again for each one played while it is held.
        pinpoint(),
        upgrading(
            card(
                "CARD.POUNCE",
                CardType::Attack,
                TargetType::AnyEnemy,
                2,
                vec![attack(14), buff("POWER.FREE_SKILL_POWER", 1)],
            ),
            vec![attack(20), buff("POWER.FREE_SKILL_POWER", 1)],
        ),
        // A precise cut is worth two less for every other card in the hand.
        upgrading(
            card(
                "CARD.PRECISE_CUT",
                CardType::Attack,
                TargetType::AnyEnemy,
                0,
                vec![precise_cut(13)],
            ),
            vec![precise_cut(16)],
        ),
    ]
}

/// Predator through Wraith Form.
#[allow(clippy::too_many_lines, reason = "one entry per card")]
fn the_late_pool() -> Vec<CardDefinition> {
    vec![
        upgrading(
            card(
                "CARD.PREDATOR",
                CardType::Attack,
                TargetType::AnyEnemy,
                2,
                vec![attack(15), buff("POWER.DRAW_CARDS_NEXT_TURN_POWER", 2)],
            ),
            vec![attack(20), buff("POWER.DRAW_CARDS_NEXT_TURN_POWER", 2)],
        ),
        // Prepared draws and puts back as many.
        upgrading(
            card(
                "CARD.PREPARED",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![draw(1), discard_from_hand(1)],
            ),
            vec![draw(2), discard_from_hand(2)],
        ),
        sly(upgrading(
            card(
                "CARD.REFLEX",
                CardType::Skill,
                TargetType::SelfTarget,
                3,
                vec![draw(2)],
            ),
            vec![draw(3)],
        )),
        // A ricochet picks each of its targets fresh.
        sly(upgrading(
            card(
                "CARD.RICOCHET",
                CardType::Attack,
                TargetType::RandomEnemy,
                2,
                vec![crate::cards::random_attack(3, 4)],
            ),
            vec![crate::cards::random_attack(3, 5)],
        )),
        upgrading(
            card(
                "CARD.SERPENT_FORM",
                CardType::Power,
                TargetType::SelfTarget,
                3,
                vec![buff("POWER.SERPENT_FORM_POWER", 4)],
            ),
            vec![buff("POWER.SERPENT_FORM_POWER", 6)],
        ),
        // A shadow step puts the whole hand down and doubles what the next turn
        // swings off a card.
        upgrading_cost(
            card(
                "CARD.SHADOW_STEP",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![discard_the_hand(), buff("POWER.SHADOW_STEP_POWER", 1)],
            ),
            0,
        ),
        upgrading_cost(
            card(
                "CARD.SHADOWMELD",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.SHADOWMELD_POWER", 1)],
            ),
            0,
        ),
        // A skewer costs X and swings that many times.
        costing_x(upgrading(
            card(
                "CARD.SKEWER",
                CardType::Attack,
                TargetType::AnyEnemy,
                0,
                vec![multi_attack(
                    Amount::Fixed(8),
                    Amount::counting(Count::EnergyX),
                )],
            ),
            vec![multi_attack(
                Amount::Fixed(11),
                Amount::counting(Count::EnergyX),
            )],
        )),
        upgrading(
            card(
                "CARD.SLICE",
                CardType::Attack,
                TargetType::AnyEnemy,
                0,
                vec![attack(6)],
            ),
            vec![attack(9)],
        ),
        // Being sneaky pays block whenever somebody else plays an attack. It
        // is multiplayer-only, which keeps it out of every single-player
        // pool.
        rarity(
            sly(upgrading(
                card(
                    "CARD.SNEAKY",
                    CardType::Power,
                    TargetType::SelfTarget,
                    2,
                    vec![buff("POWER.SNEAKY_POWER", 1)],
                ),
                vec![buff("POWER.SNEAKY_POWER", 2)],
            )),
            sts2_engine::CardRarity::Rare,
        ),
        // Upgrading a speedster makes it innate rather than stronger.
        innate_when_upgraded(upgrading(
            card(
                "CARD.SPEEDSTER",
                CardType::Power,
                TargetType::SelfTarget,
                2,
                vec![buff("POWER.SPEEDSTER_POWER", 2)],
            ),
            vec![buff("POWER.SPEEDSTER_POWER", 2)],
        )),
        upgrading(
            card(
                "CARD.STRANGLE",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![attack(8), debuff("POWER.STRANGLE_POWER", 2)],
            ),
            vec![attack(10), debuff("POWER.STRANGLE_POWER", 3)],
        ),
        striking(upgrading(
            card(
                "CARD.STRIKE_SILENT",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![attack(6)],
            ),
            vec![attack(9)],
        )),
        // A sucker punch swings and leaves the target weakened.
        upgrading(
            card(
                "CARD.SUCKER_PUNCH",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![attack(8), debuff("POWER.WEAK_POWER", 1)],
            ),
            vec![attack(10), debuff("POWER.WEAK_POWER", 2)],
        ),
        // Suppress is one of the Silent's two ancients: outside the rolled
        // rarities, and only an ancient's transcendence deals it.
        crate::cards::ancient(innate(upgrading(
            card(
                "CARD.SUPPRESS",
                CardType::Attack,
                TargetType::AnyEnemy,
                0,
                vec![attack(11), debuff("POWER.WEAK_POWER", 3)],
            ),
            vec![attack(17), debuff("POWER.WEAK_POWER", 5)],
        ))),
        upgrading(
            card(
                "CARD.SURVIVOR",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![block(8), discard_from_hand(1)],
            ),
            vec![block(11), discard_from_hand(1)],
        ),
        sly(upgrading(
            card(
                "CARD.TACTICIAN",
                CardType::Skill,
                TargetType::SelfTarget,
                3,
                vec![energy(1)],
            ),
            vec![energy(2)],
        )),
        // The hunt pays a card reward for a kill that counts, and asks whether
        // the kill would count before it swings.
        exhausting(upgrading(
            card(
                "CARD.THE_HUNT",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                the_hunt(10),
            ),
            the_hunt(15),
        )),
        upgrading_cost(
            card(
                "CARD.TOOLS_OF_THE_TRADE",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.TOOLS_OF_THE_TRADE_POWER", 1)],
            ),
            0,
        ),
        // Tracking stacks two the first time and one every time after.
        upgrading_cost(
            card(
                "CARD.TRACKING",
                CardType::Power,
                TargetType::SelfTarget,
                2,
                vec![Command::When {
                    condition: Condition::OwnerLacksPower(id("POWER.TRACKING_POWER")),
                    then: vec![buff("POWER.TRACKING_POWER", 2)],
                    otherwise: vec![buff("POWER.TRACKING_POWER", 1)],
                }],
            ),
            1,
        ),
        sly(upgrading(
            card(
                "CARD.UNTOUCHABLE",
                CardType::Skill,
                TargetType::SelfTarget,
                2,
                vec![block(6)],
            ),
            vec![block(9)],
        )),
        upgrading(
            card(
                "CARD.WELL_LAID_PLANS",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.WELL_LAID_PLANS_POWER", 1)],
            ),
            vec![buff("POWER.WELL_LAID_PLANS_POWER", 2)],
        ),
        // A wraith form is the Silent's other ancient: intangibility bought
        // with a dexterity that never stops being paid.
        crate::cards::ancient(upgrading(
            card(
                "CARD.WRAITH_FORM",
                CardType::Power,
                TargetType::SelfTarget,
                3,
                vec![
                    buff("POWER.INTANGIBLE_POWER", 2),
                    buff("POWER.WRAITH_FORM_POWER", 1),
                ],
            ),
            vec![
                buff("POWER.INTANGIBLE_POWER", 3),
                buff("POWER.WRAITH_FORM_POWER", 1),
            ],
        )),
    ]
}

/// Dodge and Roll, played: the block, and then the same amount owed for next
/// turn — what the block actually came to once the passes had their say.
fn dodge_and_roll(amount: i32) -> Command {
    Command::Shield {
        target: CreatureSelector::Owner,
        amount: Amount::Fixed(amount),
        props: ValueProps::NONE,
        then: vec![apply_power(
            "POWER.BLOCK_NEXT_TURN_POWER",
            CreatureSelector::Owner,
            Amount::counting(Count::GainedBlock),
        )],
    }
}

/// Echoing Slash, played: one blow over everything hittable, and another pass
/// for every creature that blow killed.
fn echoing_slash(damage: i32) -> Command {
    Command::RepeatPerKill {
        then: vec![Command::Creature(CreatureCmd::Damage {
            target: CreatureSelector::AllOpponents,
            amount: Amount::Fixed(damage),
            props: ValueProps::NONE,
        })],
    }
}

/// Escape Plan, played: one card drawn, and block only where it turned out to
/// be a skill.
fn escape_plan(amount: i32) -> Command {
    Command::Drawn {
        count: Amount::Fixed(1),
        then: vec![Command::When {
            otherwise: Vec::new(),
            condition: Condition::ChosenCardIsOfType(CardType::Skill),
            then: vec![block(amount)],
        }],
    }
}

/// Expertise, played: cards drawn up to the number it names, counting what the
/// hand already holds, and nothing at all where the hand already holds that
/// many.
fn expertise(up_to: i32) -> Command {
    Command::CardPile(CardPileCmd::Draw {
        count: Amount::Greatest(vec![
            Amount::Fixed(0),
            Amount::per(up_to, -1, Count::CardsInPile(PileName::Hand)),
        ]),
    })
}

/// Expose, played: the block goes, an artifact goes with it, and the target is
/// left open.
fn expose(vulnerable: i32) -> Vec<Command> {
    vec![
        Command::Creature(CreatureCmd::LoseBlock {
            target: CreatureSelector::Target,
            amount: Amount::counting(Count::TargetBlock),
        }),
        Command::When {
            otherwise: Vec::new(),
            condition: Condition::TargetHasPower(id("POWER.ARTIFACT_POWER")),
            then: vec![Command::Power(PowerCmd::Remove {
                power: sts2_engine::PowerRef {
                    owner: CreatureSelector::Target,
                    power: sts2_engine::PowerName::Model(id("POWER.ARTIFACT_POWER")),
                },
            })],
        },
        debuff("POWER.VULNERABLE_POWER", vulnerable),
    ]
}

/// Flechettes' hit count: one per skill of the hand.
fn skills_in_hand() -> Amount {
    Amount::counting(Count::CardsInPileOfType {
        pile: PileName::Hand,
        card_type: CardType::Skill,
    })
}

/// Grand Finale can only be played on an empty draw pile.
fn playable_on_an_empty_draw_pile(mut definition: CardDefinition) -> CardDefinition {
    definition.playable_when = Some(Condition::Not(Box::new(Condition::CardsInPileAtLeast {
        pile: PileName::Draw,
        count: 1,
    })));
    definition
}

/// Hand Trick, played: one skill of the hand that is not already Sly, made Sly
/// for this turn.
fn hand_trick() -> Command {
    Command::CardSelect(CardSelectCmd::FromHand {
        prefs: SelectorPrefs::exactly(1),
        maximum: None,
        exact_count: None,
        filter: CardFilter::NotSlyOfType(CardType::Skill),
        then: vec![Command::Card(CardCmd::ApplySingleTurnKeyword {
            cards: CardSelector::Chosen,
            keyword: sts2_engine::CardKeyword::Sly,
        })],
    })
}

/// Malaise, played: what X came to, plus one where the card is upgraded, taken
/// off the target's strength and worked onto it as weakness.
fn malaise(bonus: i32) -> Vec<Command> {
    let amount = Amount::per(bonus, 1, Count::EnergyX);
    vec![
        Command::Power(PowerCmd::Apply {
            marks: None,
            power: id("POWER.STRENGTH_POWER"),
            target: CreatureSelector::Target,
            applier: Some(CreatureSelector::Owner),
            amount: Amount::Product {
                left: Box::new(Amount::Fixed(-1)),
                right: Box::new(amount.clone()),
            },
        }),
        Command::Power(PowerCmd::Apply {
            marks: None,
            power: id("POWER.WEAK_POWER"),
            target: CreatureSelector::Target,
            applier: Some(CreatureSelector::Owner),
            amount,
        }),
    ]
}

/// Nightmare, played: one card of the hand picked out, and a power that holds
/// a copy of it and deals three of them into the next hand.
fn nightmare() -> Command {
    Command::CardSelect(CardSelectCmd::FromHand {
        prefs: SelectorPrefs::exactly(1),
        maximum: None,
        exact_count: None,
        filter: CardFilter::Any,
        then: vec![
            buff("POWER.NIGHTMARE_POWER", 3),
            Command::Power(PowerCmd::HoldCard {
                power: PowerRef::owned(id("POWER.NIGHTMARE_POWER")),
                cards: CardSelector::Chosen,
            }),
        ],
    })
}

/// Pinpoint: the swing is what it does when played; what it costs is the two
/// things it listens for. It reads the skills already played this turn as it
/// joins the fight, and
/// takes one more off for every skill played while it is held.
fn pinpoint() -> CardDefinition {
    let mut definition = upgrading(
        card(
            "CARD.PINPOINT",
            CardType::Attack,
            TargetType::AnyEnemy,
            3,
            vec![attack(15)],
        ),
        vec![attack(19)],
    );
    definition.hooks.push(
        ModifierHook::every_turn(
            HookEvent::AfterCardEnteredCombat,
            // A copy made mid-fight is owed nothing for the skills played
            // before it existed.
            vec![Command::When {
                otherwise: Vec::new(),
                condition: Condition::Not(Box::new(Condition::SourceCardIsAClone)),
                then: vec![Command::Card(CardCmd::AddCostThisTurn {
                    cards: CardSelector::Source,
                    amount: Amount::negating(Count::OwnerCardsOfTypePlayedThisTurn(
                        CardType::Skill,
                    )),
                })],
            }],
        )
        .when(HookCondition::ListeningCardEnteredCombat),
    );
    definition.hooks.push(
        ModifierHook::every_turn(
            HookEvent::AfterCardPlayed,
            vec![Command::Card(CardCmd::AddCostThisTurn {
                cards: CardSelector::Source,
                amount: Amount::Fixed(-1),
            })],
        )
        .when(HookCondition::PlayedCardIsOfType(CardType::Skill)),
    );
    definition
}

/// Precise Cut's damage: the base, less two for every card of the hand that is
/// not the cut itself.
fn precise_cut(base: i32) -> Command {
    multi_attack(
        Amount::Sum(vec![
            Amount::Fixed(base),
            Amount::Product {
                left: Box::new(Amount::Fixed(-2)),
                right: Box::new(Amount::Sum(vec![
                    Amount::counting(Count::CardsInPile(PileName::Hand)),
                    Amount::IfElse {
                        condition: Condition::SourceCardIsInPile(PileName::Hand),
                        then: Box::new(Amount::Fixed(-1)),
                        otherwise: Box::new(Amount::Fixed(0)),
                    },
                ])),
            },
        ]),
        Amount::Fixed(1),
    )
}

/// The Hunt, played: nothing at all outside a combat room. Whether the target's
/// death would count is asked before the swing, and the reward and the mark
/// only follow a kill that would.
fn the_hunt(damage: i32) -> Vec<Command> {
    vec![Command::When {
        otherwise: Vec::new(),
        condition: Condition::CurrentRoomIsCombat,
        then: vec![Command::When {
            // The capture the build makes before the attack: every power the
            // target carries has to allow its death to count.
            condition: Condition::TargetsDeathWouldTriggerFatal,
            then: vec![
                attack(damage),
                Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::AFatalKillHappenedThisPlay,
                    then: vec![
                        Command::Rewards(RewardsCmd::AddOwnCardOfferToTheRoom { count: 3 }),
                        buff("POWER.THE_HUNT_POWER", 1),
                    ],
                },
            ],
            otherwise: vec![attack(damage)],
        }],
    }]
}
