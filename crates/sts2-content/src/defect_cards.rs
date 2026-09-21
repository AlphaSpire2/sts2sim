//! The Defect's whole card pool, transcribed.
//!
//! Eighty-eight models: the four he starts with, the eighty a rolled rarity can
//! offer, the two ancients, and the two multiplayer-only cards no
//! single-player pool ever holds. Each is the run of commands the card issues
//! when it is played, in order, at both the levels it supports.
//!
//! Three things recur across the pool and are worth naming once. Channelling
//! evokes the front of a full queue to make room before the new orb joins the
//! back of it. Evoking takes the next orb, and every card but three takes that
//! orb out of the queue; the three that do not evoke the same orb twice and
//! take it out on the last of them. And a card that reads how many orbs its
//! owner is holding reads either the plain count or the number of distinct
//! kinds, and the two are different questions.

use sts2_engine::{
    Amount, CardCmd, CardDefinition, CardFilter, CardOrigin, CardPileCmd, CardPoolRef,
    CardSelectCmd, CardSelector, CardType, Command, Condition, Count, CreatureCmd,
    CreatureSelector, DamageCmd, DrawPilePosition, OrbCmd, OrbSelector, OrbSource, PileName,
    PlayerCmd, SelectorPrefs, ShuffleStream, TargetType, TemporaryCost, ValueProps,
};

use crate::cards::{
    apply_power, attack, block, buff, card, costing_x, debuff, draw, exhausting, multi_attack,
    rarity, striking, sweep, upgrading, upgrading_cost, with_damage_var,
};
use crate::id;

const LIGHTNING: &str = "ORB.LIGHTNING_ORB";
const FROST: &str = "ORB.FROST_ORB";
const DARK: &str = "ORB.DARK_ORB";
const PLASMA: &str = "ORB.PLASMA_ORB";
const GLASS: &str = "ORB.GLASS_ORB";

/// That orb channelled by the card's own owner.
fn channel(orb: &str) -> Command {
    Command::Orb(OrbCmd::Channel(OrbSource::Model(id(orb))))
}

/// The same, a counted number of times, which every card that channels more
/// than one writes as its own loop.
fn channel_times(orb: &str, times: i32) -> Command {
    Command::Repeat {
        times: Amount::Fixed(times),
        then: vec![channel(orb)],
    }
}

/// The next orb evoked and taken out of the queue.
const fn evoke_next() -> Command {
    Command::Orb(OrbCmd::EvokeNext { dequeue: true })
}

/// The same, leaving the orb standing.
const fn evoke_next_keeping() -> Command {
    Command::Orb(OrbCmd::EvokeNext { dequeue: false })
}

fn defending(definition: CardDefinition) -> CardDefinition {
    crate::cards::defending(definition)
}

fn gains_block(definition: CardDefinition) -> CardDefinition {
    definition
}

fn ancient(definition: CardDefinition) -> CardDefinition {
    crate::cards::ancient(definition)
}

fn ethereal(definition: CardDefinition) -> CardDefinition {
    crate::cards::ethereal(definition)
}

fn innate(mut definition: CardDefinition) -> CardDefinition {
    definition.innate = true;
    definition
}

fn upgrade_drops_exhaust(mut definition: CardDefinition) -> CardDefinition {
    if let Some(upgrade) = definition.upgrade.as_mut() {
        upgrade.drops_exhaust = true;
    }
    definition
}

fn upgrade_drops_ethereal(mut definition: CardDefinition) -> CardDefinition {
    if let Some(upgrade) = definition.upgrade.as_mut() {
        upgrade.drops_ethereal = true;
    }
    definition
}

fn innate_when_upgraded(definition: CardDefinition) -> CardDefinition {
    crate::cards::innate_when_upgraded(definition)
}

/// A card whose upgraded form runs the same commands: what the upgrade does is
/// asked for inside the body, or comes from a keyword the upgrade adds.
fn upgrading_in_place(definition: CardDefinition) -> CardDefinition {
    let effects = definition.effects.clone();
    upgrading(definition, effects)
}

/// A status card made for the fight and put into the pile named.
fn conjure(model_id: &str, count: i32, pile: PileName) -> Command {
    Command::CardPile(CardPileCmd::AddGeneratedToCombat {
        enchantment: None,
        card: CardOrigin::Model(id(model_id)),
        grown_by: Amount::Fixed(0),
        count: Amount::Fixed(count),
        pile,
        ethereal: false,
        retain: false,
        upgraded: false,
        temporary_cost: None,
        position: DrawPilePosition::Bottom,
    })
}

fn energy(amount: i32) -> Command {
    Command::Player(PlayerCmd::GainEnergy(Amount::Fixed(amount)))
}

#[must_use]
pub fn defect_cards() -> Vec<CardDefinition> {
    let mut cards = starters();
    cards.extend(commons());
    cards.extend(uncommons());
    cards.extend(rares());
    cards.extend(ancients_and_multiplayer());
    cards
}

/// What the Defect opens a run with: four Strikes, four Defends, a Zap, and a
/// Dualcast.
fn starters() -> Vec<CardDefinition> {
    vec![
        striking(with_damage_var(
            upgrading(
                card(
                    "CARD.STRIKE_DEFECT",
                    CardType::Attack,
                    TargetType::AnyEnemy,
                    1,
                    vec![attack(6)],
                ),
                vec![attack(9)],
            ),
            6,
            9,
        )),
        defending(gains_block(upgrading(
            card(
                "CARD.DEFEND_DEFECT",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![block(5)],
            ),
            vec![block(8)],
        ))),
        upgrading_cost(
            card(
                "CARD.ZAP",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![channel(LIGHTNING)],
            ),
            0,
        ),
        // The guard is on the queue holding an orb rather than on the evoke,
        // so a Dualcast with nothing standing does nothing at all — and
        // the first of its two evokes leaves the orb where it is.
        upgrading_cost(
            card(
                "CARD.DUALCAST",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![Command::When {
                    condition: Condition::CountAtLeast {
                        count: Count::OrbsHeld,
                        at_least: 1,
                    },
                    then: vec![evoke_next_keeping(), evoke_next()],
                    otherwise: Vec::new(),
                }],
            ),
            0,
        ),
    ]
}

#[allow(
    clippy::too_many_lines,
    reason = "one entry per card the pool declares"
)]
fn commons() -> Vec<CardDefinition> {
    vec![
        upgrading(
            card(
                "CARD.BALL_LIGHTNING",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![attack(7), channel(LIGHTNING)],
            ),
            vec![attack(10), channel(LIGHTNING)],
        ),
        // One hit per orb its owner is holding, counted before the swing
        // opens.
        upgrading(
            card(
                "CARD.BARRAGE",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![multi_attack(
                    Amount::Fixed(5),
                    Amount::counting(Count::OrbsHeld),
                )],
            ),
            vec![multi_attack(
                Amount::Fixed(7),
                Amount::counting(Count::OrbsHeld),
            )],
        ),
        upgrading(
            card(
                "CARD.BEAM_CELL",
                CardType::Attack,
                TargetType::AnyEnemy,
                0,
                vec![attack(3), debuff("POWER.VULNERABLE_POWER", 1)],
            ),
            vec![attack(4), debuff("POWER.VULNERABLE_POWER", 2)],
        ),
        gains_block(upgrading(
            card(
                "CARD.BOOST_AWAY",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![block(6), conjure("CARD.DAZED", 1, PileName::Discard)],
            ),
            vec![block(9), conjure("CARD.DAZED", 1, PileName::Discard)],
        )),
        gains_block(upgrading(
            card(
                "CARD.CHARGE_BATTERY",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![block(7), buff("POWER.ENERGY_NEXT_TURN_POWER", 1)],
            ),
            vec![block(10), buff("POWER.ENERGY_NEXT_TURN_POWER", 1)],
        )),
        // Every Claw of the fight grows, the one that was played included,
        // and the growth belongs to the combat copy.
        with_damage_var(
            upgrading(
                card(
                    "CARD.CLAW",
                    CardType::Attack,
                    TargetType::AnyEnemy,
                    0,
                    claw(2),
                ),
                claw(3),
            ),
            3,
            4,
        ),
        upgrading(
            card(
                "CARD.COLD_SNAP",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![attack(6), channel(FROST)],
            ),
            vec![attack(9), channel(FROST)],
        ),
        // One card per distinct kind of orb held rather than one per orb.
        upgrading(
            card(
                "CARD.COMPILE_DRIVER",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![
                    attack(7),
                    Command::CardPile(CardPileCmd::Draw {
                        count: Amount::counting(Count::DistinctOrbModelsHeld),
                    }),
                ],
            ),
            vec![
                attack(10),
                Command::CardPile(CardPileCmd::Draw {
                    count: Amount::counting(Count::DistinctOrbModelsHeld),
                }),
            ],
        ),
        upgrading(
            card(
                "CARD.COOLHEADED",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![channel(FROST), draw(1)],
            ),
            vec![channel(FROST), draw(2)],
        ),
        striking(upgrading(
            card(
                "CARD.FOCUSED_STRIKE",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![attack(9), buff("POWER.FOCUSED_STRIKE_POWER", 1)],
            ),
            vec![attack(11), buff("POWER.FOCUSED_STRIKE_POWER", 2)],
        )),
        // The debuff lands only where the target is showing an attack, which
        // is asked after the blow rather than before it.
        upgrading(
            card(
                "CARD.GO_FOR_THE_EYES",
                CardType::Attack,
                TargetType::AnyEnemy,
                0,
                vec![attack(3), weaken_an_attacker(1)],
            ),
            vec![attack(4), weaken_an_attacker(2)],
        ),
        upgrading(
            card(
                "CARD.GUNK_UP",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![
                    multi_attack(Amount::Fixed(4), Amount::Fixed(3)),
                    conjure("CARD.SLIMED", 1, PileName::Discard),
                ],
            ),
            vec![
                multi_attack(Amount::Fixed(5), Amount::Fixed(3)),
                conjure("CARD.SLIMED", 1, PileName::Discard),
            ],
        ),
        gains_block(upgrade_drops_exhaust(exhausting(upgrading(
            card(
                "CARD.HOLOGRAM",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![block(3), one_card_of_the_discard_back()],
            ),
            vec![block(5), one_card_of_the_discard_back()],
        )))),
        upgrade_drops_exhaust(exhausting(upgrading_in_place(card(
            "CARD.HOTFIX",
            CardType::Skill,
            TargetType::SelfTarget,
            0,
            vec![buff("POWER.HOTFIX_POWER", 2)],
        )))),
        gains_block(upgrading(
            card(
                "CARD.LEAP",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![block(9)],
            ),
            vec![block(12)],
        )),
        gains_block(upgrading(
            card(
                "CARD.LIGHTNING_ROD",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![block(4), buff("POWER.LIGHTNING_ROD_POWER", 2)],
            ),
            vec![block(7), buff("POWER.LIGHTNING_ROD_POWER", 2)],
        )),
        // The card makes itself free for the rest of the fight once it has
        // swung.
        striking(upgrading(
            card(
                "CARD.MOMENTUM_STRIKE",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![attack(10), free_for_the_combat()],
            ),
            vec![attack(13), free_for_the_combat()],
        )),
        upgrading(
            card(
                "CARD.SWEEPING_BEAM",
                CardType::Attack,
                TargetType::AllEnemies,
                1,
                vec![sweep(6, 1), draw(1)],
            ),
            vec![sweep(9, 1), draw(1)],
        ),
        upgrading(
            card(
                "CARD.TURBO",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![energy(2), conjure("CARD.VOID", 1, PileName::Discard)],
            ),
            vec![energy(3), conjure("CARD.VOID", 1, PileName::Discard)],
        ),
        upgrading(
            card(
                "CARD.UPROAR",
                CardType::Attack,
                TargetType::AnyEnemy,
                2,
                vec![
                    multi_attack(Amount::Fixed(6), Amount::Fixed(2)),
                    play_an_attack_off_the_draw_pile(),
                ],
            ),
            vec![
                multi_attack(Amount::Fixed(8), Amount::Fixed(2)),
                play_an_attack_off_the_draw_pile(),
            ],
        ),
    ]
}

/// A Claw: it swings for whatever its own damage var has grown to, and then
/// every Claw of the fight — the one that swung included — grows by the
/// Increase var.
fn claw(increase: i32) -> Vec<Command> {
    vec![
        multi_attack(Amount::counting(Count::SourceCardDamage), Amount::Fixed(1)),
        Command::Card(CardCmd::GrowDamage {
            cards: CardSelector::CombatCardsOfSourceModel,
            amount: Amount::Fixed(increase),
        }),
    ]
}

/// Weak on the target, but only where it is showing an attack.
fn weaken_an_attacker(amount: i32) -> Command {
    Command::When {
        condition: Condition::TargetIntendsToAttack,
        then: vec![debuff("POWER.WEAK_POWER", amount)],
        otherwise: Vec::new(),
    }
}

/// The card that issued it becomes free for the rest of the fight.
fn free_for_the_combat() -> Command {
    Command::Card(CardCmd::SetTemporaryCost {
        cards: CardSelector::Source,
        lifetime: TemporaryCost::this_combat(0),
    })
}

/// A Hologram's screen: one card of the discard pile, back to the hand. The
/// screen takes the card without asking where the pile holds a single one, and
/// an empty pile hands nothing back.
fn one_card_of_the_discard_back() -> Command {
    Command::CardSelect(CardSelectCmd::FromCombatPile {
        pile: PileName::Discard,
        prefs: SelectorPrefs::exactly(1),
        maximum: None,
        exact_count: None,
        restricted_to_chosen: false,
        filter: CardFilter::Any,
        then: vec![Command::CardPile(CardPileCmd::MoveToPile {
            cards: CardSelector::Chosen,
            pile: PileName::Hand,
            position: DrawPilePosition::Bottom,
        })],
    })
}

/// An Uproar's second half: the draw pile's playable attacks, stably shuffled
/// on the run's shuffle stream, and the first of them played for free. The
/// build writes a second list where the first is empty — every attack, playable
/// or not — and a shuffle over an empty list reads no dice, so the fallback
/// costs nothing of its own.
fn play_an_attack_off_the_draw_pile() -> Command {
    Command::Select {
        cards: CardSelector::StablyShuffledFromPile {
            pile: PileName::Draw,
            filter: CardFilter::PlayableOfType(CardType::Attack),
            fallback: Some(CardFilter::OfType(CardType::Attack)),
            take: Amount::Fixed(1),
            stream: ShuffleStream::Shuffle,
        },
        then: vec![Command::Card(CardCmd::AutoPlay {
            cards: CardSelector::Chosen,
            at_the_bodys_target: false,
        })],
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "one entry per card the pool declares"
)]
fn uncommons() -> Vec<CardDefinition> {
    let uncommon =
        |definition: CardDefinition| rarity(definition, sts2_engine::CardRarity::Uncommon);
    vec![
        uncommon(gains_block(innate(exhausting(upgrading(
            card(
                "CARD.BOOT_SEQUENCE",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![block(10)],
            ),
            vec![block(13)],
        ))))),
        // The slot goes first and the two stats follow: a Bulk Up pays a slot
        // for them, and the slot goes whether or not an orb was standing in
        // it.
        uncommon(upgrading(
            card(
                "CARD.BULK_UP",
                CardType::Power,
                TargetType::SelfTarget,
                2,
                vec![
                    Command::Orb(OrbCmd::RemoveSlots(Amount::Fixed(1))),
                    buff("POWER.STRENGTH_POWER", 2),
                    buff("POWER.DEXTERITY_POWER", 2),
                ],
            ),
            vec![
                Command::Orb(OrbCmd::RemoveSlots(Amount::Fixed(1))),
                buff("POWER.STRENGTH_POWER", 3),
                buff("POWER.DEXTERITY_POWER", 3),
            ],
        )),
        uncommon(upgrading(
            card(
                "CARD.CAPACITOR",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![Command::Orb(OrbCmd::AddSlots(Amount::Fixed(2)))],
            ),
            vec![Command::Orb(OrbCmd::AddSlots(Amount::Fixed(3)))],
        )),
        // A rolled orb is one index over the five there are, rolled on the
        // combat orb-generation stream once per orb asked for.
        uncommon(upgrading(
            card(
                "CARD.CHAOS",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![Command::Repeat {
                    times: Amount::Fixed(1),
                    then: vec![Command::Orb(OrbCmd::Channel(OrbSource::RandomOnCombatOrbs))],
                }],
            ),
            vec![Command::Repeat {
                times: Amount::Fixed(2),
                then: vec![Command::Orb(OrbCmd::Channel(OrbSource::RandomOnCombatOrbs))],
            }],
        )),
        // One Frost orb per hittable enemy, walked over the list the body took
        // before it channelled anything.
        uncommon(upgrade_drops_exhaust(exhausting(upgrading_in_place(card(
            "CARD.CHILL",
            CardType::Skill,
            TargetType::SelfTarget,
            0,
            vec![Command::Repeat {
                times: Amount::counting(Count::LivingOpponents),
                then: vec![channel(FROST)],
            }],
        ))))),
        // Every status card of the hand becomes a Fuel, upgraded where the
        // Compact that did it was.
        uncommon(gains_block(upgrading(
            card(
                "CARD.COMPACT",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![block(6), statuses_of_the_hand_into_fuel(false)],
            ),
            vec![block(7), statuses_of_the_hand_into_fuel(true)],
        ))),
        // A Dark orb channelled and then every Dark orb standing fired, once
        // for the base form and twice for the upgraded one.
        uncommon(upgrading(
            card(
                "CARD.DARKNESS",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![channel(DARK), fire_every_dark_orb(1)],
            ),
            vec![channel(DARK), fire_every_dark_orb(2)],
        )),
        uncommon(upgrading_cost(
            exhausting(card(
                "CARD.DOUBLE_ENERGY",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![Command::Player(PlayerCmd::GainEnergy(Amount::counting(
                    Count::PlayerEnergy,
                )))],
            )),
            0,
        )),
        uncommon(upgrading_cost(
            card(
                "CARD.FERAL",
                CardType::Power,
                TargetType::SelfTarget,
                2,
                vec![buff("POWER.FERAL_POWER", 1)],
            ),
            1,
        )),
        uncommon(gains_block(upgrading(
            card(
                "CARD.FIGHT_THROUGH",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![block(13), conjure("CARD.WOUND", 2, PileName::Discard)],
            ),
            vec![block(17), conjure("CARD.WOUND", 2, PileName::Discard)],
        ))),
        // The draw is guarded on the same count the card's own glow reads: how
        // many plays of its owner's have finished this turn.
        uncommon(upgrading(
            card(
                "CARD.FTL",
                CardType::Attack,
                TargetType::AnyEnemy,
                0,
                vec![attack(5), draw_under_a_play_cap(3, 1)],
            ),
            vec![attack(6), draw_under_a_play_cap(4, 1)],
        )),
        uncommon(upgrade_drops_exhaust(exhausting(upgrading_in_place(card(
            "CARD.FUSION",
            CardType::Skill,
            TargetType::SelfTarget,
            1,
            vec![channel(PLASMA)],
        ))))),
        uncommon(gains_block(upgrading(
            card(
                "CARD.GLACIER",
                CardType::Skill,
                TargetType::SelfTarget,
                2,
                vec![block(6), channel_times(FROST, 2)],
            ),
            vec![block(9), channel_times(FROST, 2)],
        ))),
        uncommon(gains_block(upgrading(
            card(
                "CARD.GLASSWORK",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![block(5), channel(GLASS)],
            ),
            vec![block(8), channel(GLASS)],
        ))),
        uncommon(upgrading(
            card(
                "CARD.HAILSTORM",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.HAILSTORM_POWER", 6)],
            ),
            vec![buff("POWER.HAILSTORM_POWER", 8)],
        )),
        uncommon(upgrading(
            card(
                "CARD.ITERATION",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.ITERATION_POWER", 2)],
            ),
            vec![buff("POWER.ITERATION_POWER", 3)],
        )),
        uncommon(upgrading(
            card(
                "CARD.LOOP",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.LOOP_POWER", 1)],
            ),
            vec![buff("POWER.LOOP_POWER", 2)],
        )),
        // The card gets dearer every time it is played, and the raise stands
        // for the rest of the fight.
        uncommon(upgrading(
            card(
                "CARD.MODDED",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![
                    Command::Orb(OrbCmd::AddSlots(Amount::Fixed(1))),
                    draw(1),
                    dearer_for_the_combat(1),
                ],
            ),
            vec![
                Command::Orb(OrbCmd::AddSlots(Amount::Fixed(1))),
                draw(2),
                dearer_for_the_combat(1),
            ],
        )),
        uncommon(upgrading(
            card(
                "CARD.NULL",
                CardType::Attack,
                TargetType::AnyEnemy,
                2,
                vec![attack(10), debuff("POWER.WEAK_POWER", 2), channel(DARK)],
            ),
            vec![attack(13), debuff("POWER.WEAK_POWER", 3), channel(DARK)],
        )),
        uncommon(upgrading(
            card(
                "CARD.OVERCLOCK",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![draw(2), conjure("CARD.BURN", 1, PileName::Discard)],
            ),
            vec![draw(3), conjure("CARD.BURN", 1, PileName::Discard)],
        )),
        // Two hits and then two Glass orbs, whatever the level.
        uncommon(upgrading(
            card(
                "CARD.REFRACT",
                CardType::Attack,
                TargetType::AnyEnemy,
                3,
                vec![
                    multi_attack(Amount::Fixed(9), Amount::Fixed(2)),
                    channel_times(GLASS, 2),
                ],
            ),
            vec![
                multi_attack(Amount::Fixed(12), Amount::Fixed(2)),
                channel_times(GLASS, 2),
            ],
        )),
        uncommon(upgrading(
            card(
                "CARD.ROCKET_PUNCH",
                CardType::Attack,
                TargetType::AnyEnemy,
                2,
                vec![attack(13), draw(1)],
            ),
            vec![attack(14), draw(2)],
        )),
        uncommon(upgrading(
            card(
                "CARD.SCAVENGE",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![
                    burn_one_card_of_the_hand(),
                    buff("POWER.ENERGY_NEXT_TURN_POWER", 2),
                ],
            ),
            vec![
                burn_one_card_of_the_hand(),
                buff("POWER.ENERGY_NEXT_TURN_POWER", 3),
            ],
        )),
        // The draw is settled first and then the cards among it that still
        // cost something are discarded.
        uncommon(upgrading(
            card(
                "CARD.SCRAPE",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![attack(7), draw_and_discard_what_costs_something(4)],
            ),
            vec![attack(10), draw_and_discard_what_costs_something(5)],
        )),
        uncommon(gains_block(upgrading(
            card(
                "CARD.SHADOW_SHIELD",
                CardType::Skill,
                TargetType::SelfTarget,
                2,
                vec![block(11), channel(DARK)],
            ),
            vec![block(15), channel(DARK)],
        ))),
        uncommon(upgrading(
            card(
                "CARD.SKIM",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![draw(3)],
            ),
            vec![draw(4)],
        )),
        uncommon(upgrading(
            card(
                "CARD.SMOKESTACK",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.SMOKESTACK_POWER", 5)],
            ),
            vec![buff("POWER.SMOKESTACK_POWER", 7)],
        )),
        uncommon(upgrading(
            card(
                "CARD.STORM",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.STORM_POWER", 1)],
            ),
            vec![buff("POWER.STORM_POWER", 2)],
        )),
        uncommon(upgrading_cost(
            card(
                "CARD.SUBROUTINE",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.SUBROUTINE_POWER", 1)],
            ),
            0,
        )),
        uncommon(upgrading(
            card(
                "CARD.SUNDER",
                CardType::Attack,
                TargetType::AnyEnemy,
                3,
                vec![energy_for_a_kill(24, 3)],
            ),
            vec![energy_for_a_kill(32, 3)],
        )),
        // Focus worth two for every distinct kind of orb its owner is
        // holding, as a power that gives it back at the end of the turn.
        uncommon(upgrade_drops_exhaust(exhausting(upgrading_in_place(card(
            "CARD.SYNCHRONIZE",
            CardType::Skill,
            TargetType::SelfTarget,
            1,
            vec![apply_power(
                "POWER.SYNCHRONIZE_POWER",
                CreatureSelector::Owner,
                Amount::per(0, 2, Count::DistinctOrbModelsHeld),
            )],
        ))))),
        uncommon(upgrading(
            card(
                "CARD.SYNTHESIS",
                CardType::Attack,
                TargetType::AnyEnemy,
                2,
                vec![attack(14), buff("POWER.FREE_POWER_POWER", 1)],
            ),
            vec![attack(20), buff("POWER.FREE_POWER_POWER", 1)],
        )),
        // A cost of X: the play spends every point of energy its owner has left
        // and the body reads back what it spent, one Lightning orb each, and
        // one more for the upgraded form.
        uncommon(costing_x(upgrading(
            card(
                "CARD.TEMPEST",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![Command::Repeat {
                    times: Amount::counting(Count::EnergyX),
                    then: vec![channel(LIGHTNING)],
                }],
            ),
            vec![Command::Repeat {
                times: Amount::Scaled {
                    base: 1,
                    per: 1,
                    count: Count::EnergyX,
                },
                then: vec![channel(LIGHTNING)],
            }],
        ))),
        // Every Lightning orb standing fires at whoever the play was aimed at,
        // once for the base form and twice for the upgraded one.
        uncommon(upgrading(
            card(
                "CARD.TESLA_COIL",
                CardType::Attack,
                TargetType::AnyEnemy,
                0,
                vec![attack(3), fire_every_lightning_orb_at_the_target(1)],
            ),
            vec![attack(4), fire_every_lightning_orb_at_the_target(2)],
        )),
        uncommon(upgrading(
            card(
                "CARD.THUNDER",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.THUNDER_POWER", 6)],
            ),
            vec![buff("POWER.THUNDER_POWER", 8)],
        )),
        uncommon(upgrading_cost(
            exhausting(card(
                "CARD.WHITE_NOISE",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![a_free_power_card_into_the_hand()],
            )),
            0,
        )),
    ]
}

/// Every transformable status card of the hand, each turned into a Fuel that
/// is upgraded where the card that did it was.
fn statuses_of_the_hand_into_fuel(upgraded: bool) -> Command {
    Command::Card(CardCmd::Transform {
        cards: CardSelector::Pile {
            pile: PileName::Hand,
            filter: CardFilter::OfType(CardType::Status),
        },
        into: id("CARD.FUEL"),
        upgraded,
    })
}

/// Every Dark orb held fires its passive, that many times each.
fn fire_every_dark_orb(times: i32) -> Command {
    Command::ForEachOrb {
        orbs: OrbSelector::OfModel(id(DARK)),
        then: vec![Command::Repeat {
            times: Amount::Fixed(times),
            then: vec![Command::Orb(OrbCmd::Passive {
                orb: OrbSelector::Bound,
                target: None,
            })],
        }],
    }
}

/// Every Lightning orb held fires its passive, that many times each, at
/// whoever the play was aimed at.
fn fire_every_lightning_orb_at_the_target(times: i32) -> Command {
    Command::ForEachOrb {
        orbs: OrbSelector::OfModel(id(LIGHTNING)),
        then: vec![Command::Repeat {
            times: Amount::Fixed(times),
            then: vec![Command::Orb(OrbCmd::Passive {
                orb: OrbSelector::Bound,
                target: Some(CreatureSelector::Target),
            })],
        }],
    }
}

/// The draw happens only under a cap on how many plays of the owner's have
/// finished this turn.
fn draw_under_a_play_cap(cap: i32, cards: i32) -> Command {
    Command::When {
        condition: Condition::Not(Box::new(Condition::CountAtLeast {
            count: Count::OwnerCardPlaysFinishedThisTurn,
            at_least: cap,
        })),
        then: vec![draw(cards)],
        otherwise: Vec::new(),
    }
}

/// The card that issued it gets that much dearer for the rest of the fight.
fn dearer_for_the_combat(amount: i32) -> Command {
    Command::Card(CardCmd::AddCostThisCombat {
        cards: CardSelector::Source,
        amount: Amount::Fixed(amount),
    })
}

/// A Scavenge's screen: one card of the hand, burned.
fn burn_one_card_of_the_hand() -> Command {
    Command::CardSelect(CardSelectCmd::FromHand {
        prefs: SelectorPrefs::exactly(1),
        maximum: None,
        exact_count: None,
        filter: CardFilter::Any,
        then: vec![Command::Card(CardCmd::Exhaust {
            cards: CardSelector::Chosen,
        })],
    })
}

/// That many cards drawn, and every one of them that still costs something,
/// or costs X, discarded.
fn draw_and_discard_what_costs_something(count: i32) -> Command {
    Command::Drawn {
        count: Amount::Fixed(count),
        then: vec![Command::Card(CardCmd::Discard {
            cards: CardSelector::ChosenMatching {
                filter: CardFilter::CostsSomethingNowOrX,
            },
        })],
    }
}

/// A Sunder: the swing, and the energy only where something it hit went down.
fn energy_for_a_kill(damage: i32, gain: i32) -> Command {
    Command::Strike {
        amount: Amount::Fixed(damage),
        hits: Amount::Fixed(1),
        target: CreatureSelector::Target,
        props: ValueProps::NONE,
        then: vec![Command::When {
            condition: Condition::TheSwingKilledSomething,
            then: vec![energy(gain)],
            otherwise: Vec::new(),
        }],
    }
}

/// One distinct Power card drawn from the owner's own pool off the combat
/// card-generation stream, made free for the turn and put in the hand.
fn a_free_power_card_into_the_hand() -> Command {
    Command::CardPile(CardPileCmd::AddGeneratedFromPool {
        pool: CardPoolRef::OwnCharacter,
        count: Amount::Fixed(1),
        distinct: true,
        filter: CardFilter::OfType(CardType::Power),
        pile: PileName::Hand,
        position: DrawPilePosition::Bottom,
        excludes_source: false,
        upgraded: false,
        temporary_cost: Some(TemporaryCost::free_this_turn()),
        ethereal: false,
        rolled_one_at_a_time: false,
    })
}

#[allow(
    clippy::too_many_lines,
    reason = "one entry per card the pool declares"
)]
fn rares() -> Vec<CardDefinition> {
    let rare = |definition: CardDefinition| rarity(definition, sts2_engine::CardRarity::Rare);
    vec![
        // A copy of itself, free for the fight, into the discard pile.
        rare(striking(upgrading(
            card(
                "CARD.ADAPTIVE_STRIKE",
                CardType::Attack,
                TargetType::AnyEnemy,
                2,
                vec![attack(18), a_free_copy_of_itself()],
            ),
            vec![attack(23), a_free_copy_of_itself()],
        ))),
        // Every card of the discard pile that costs nothing right now and is an
        // attack, a skill, or a power, back to the hand.
        rare(upgrading(
            card(
                "CARD.ALL_FOR_ONE",
                CardType::Attack,
                TargetType::AnyEnemy,
                2,
                vec![attack(10), free_cards_of_the_discard_back()],
            ),
            vec![attack(14), free_cards_of_the_discard_back()],
        )),
        rare(upgrading(
            card(
                "CARD.BUFFER",
                CardType::Power,
                TargetType::SelfTarget,
                2,
                vec![buff("POWER.BUFFER_POWER", 1)],
            ),
            vec![buff("POWER.BUFFER_POWER", 2)],
        )),
        rare(upgrading(
            card(
                "CARD.CONSUMING_SHADOW",
                CardType::Power,
                TargetType::SelfTarget,
                2,
                vec![
                    channel_times(DARK, 2),
                    buff("POWER.CONSUMING_SHADOW_POWER", 1),
                ],
            ),
            vec![
                channel_times(DARK, 3),
                buff("POWER.CONSUMING_SHADOW_POWER", 1),
            ],
        )),
        rare(upgrading(
            card(
                "CARD.COOLANT",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.COOLANT_POWER", 2)],
            ),
            vec![buff("POWER.COOLANT_POWER", 3)],
        )),
        rare(upgrading_cost(
            card(
                "CARD.CREATIVE_AI",
                CardType::Power,
                TargetType::SelfTarget,
                3,
                vec![buff("POWER.CREATIVE_AI_POWER", 1)],
            ),
            2,
        )),
        rare(upgrading(
            card(
                "CARD.DEFRAGMENT",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.FOCUS_POWER", 1)],
            ),
            vec![buff("POWER.FOCUS_POWER", 2)],
        )),
        rare(upgrade_drops_ethereal(ethereal(upgrading_in_place(card(
            "CARD.ECHO_FORM",
            CardType::Power,
            TargetType::SelfTarget,
            3,
            vec![buff("POWER.ECHO_FORM_POWER", 1)],
        ))))),
        // Every status card outside the exhaust pile is burned, and the swing
        // lands once for each of them — the count is taken before the burns.
        rare(upgrading(
            card(
                "CARD.FLAK_CANNON",
                CardType::Attack,
                TargetType::RandomEnemy,
                2,
                vec![burn_the_statuses_and_swing(8)],
            ),
            vec![burn_the_statuses_and_swing(11)],
        )),
        // A Genetic Algorithm's block is a saved property that grows with every
        // play and outlives the fight.
        rare(genetic_algorithm()),
        // One hit per point of energy the owner has spent this turn, less what
        // this play itself paid.
        rare(upgrading(
            card(
                "CARD.HELIX_DRILL",
                CardType::Attack,
                TargetType::AnyEnemy,
                0,
                vec![multi_attack(
                    Amount::Fixed(3),
                    energy_spent_before_this_play(),
                )],
            ),
            vec![multi_attack(
                Amount::Fixed(5),
                energy_spent_before_this_play(),
            )],
        )),
        rare(upgrading(
            card(
                "CARD.HYPERBEAM",
                CardType::Attack,
                TargetType::AllEnemies,
                2,
                vec![
                    sweep(28, 1),
                    apply_power(
                        "POWER.FOCUS_POWER",
                        CreatureSelector::Owner,
                        Amount::Fixed(-3),
                    ),
                ],
            ),
            vec![
                sweep(36, 1),
                apply_power(
                    "POWER.FOCUS_POWER",
                    CreatureSelector::Owner,
                    Amount::Fixed(-3),
                ),
            ],
        )),
        rare(upgrading(
            card(
                "CARD.ICE_LANCE",
                CardType::Attack,
                TargetType::AnyEnemy,
                3,
                vec![attack(19), channel_times(FROST, 3)],
            ),
            vec![attack(24), channel_times(FROST, 3)],
        )),
        rare(innate_when_upgraded(upgrading_in_place(card(
            "CARD.MACHINE_LEARNING",
            CardType::Power,
            TargetType::SelfTarget,
            1,
            vec![buff("POWER.MACHINE_LEARNING_POWER", 1)],
        )))),
        rare(striking(upgrading(
            card(
                "CARD.METEOR_STRIKE",
                CardType::Attack,
                TargetType::AnyEnemy,
                5,
                vec![attack(24), channel_times(PLASMA, 3)],
            ),
            vec![attack(30), channel_times(PLASMA, 3)],
        ))),
        // The hand goes back to the draw pile, the pile is shuffled, and a new
        // hand is dealt.
        rare(exhausting(upgrading(
            card(
                "CARD.REBOOT",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![reshuffle_the_hand(), draw(4)],
            ),
            vec![reshuffle_the_hand(), draw(6)],
        ))),
        // Every orb evoked twice, front first, and taken out of the queue on
        // the second of the two. The count is taken before the loop opens.
        rare(upgrading(
            card(
                "CARD.SHATTER",
                CardType::Attack,
                TargetType::AllEnemies,
                1,
                vec![sweep(7, 1), evoke_every_orb_twice()],
            ),
            vec![sweep(11, 1), evoke_every_orb_twice()],
        )),
        rare(exhausting(upgrading_cost(
            card(
                "CARD.SIGNAL_BOOST",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.SIGNAL_BOOST_POWER", 1)],
            ),
            0,
        ))),
        // The upgraded form channels one Glass orb as it lands, before the
        // power it applies starts channelling one every turn.
        rare(upgrading(
            card(
                "CARD.SPINNER",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.SPINNER_POWER", 1)],
            ),
            vec![channel(GLASS), buff("POWER.SPINNER_POWER", 1)],
        )),
        rare(exhausting(upgrading(
            card(
                "CARD.SUPERCRITICAL",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![energy(4)],
            ),
            vec![energy(6)],
        ))),
        rare(innate_when_upgraded(upgrading_in_place(card(
            "CARD.TRASH_TO_TREASURE",
            CardType::Power,
            TargetType::SelfTarget,
            1,
            vec![buff("POWER.TRASH_TO_TREASURE_POWER", 1)],
        )))),
        // One Lightning orb for every Lightning orb its owner has already
        // channelled in this fight.
        rare(upgrade_drops_exhaust(exhausting(upgrading_in_place(card(
            "CARD.VOLTAIC",
            CardType::Skill,
            TargetType::SelfTarget,
            3,
            vec![Command::Repeat {
                times: Amount::counting(Count::OrbsChanneledThisCombat(id(LIGHTNING))),
                then: vec![channel(LIGHTNING)],
            }],
        ))))),
        rare(upgrade_drops_exhaust(exhausting(upgrading_in_place(card(
            "CARD.RAINBOW",
            CardType::Skill,
            TargetType::SelfTarget,
            2,
            vec![channel(LIGHTNING), channel(FROST), channel(DARK)],
        ))))),
    ]
}

/// A copy of the card, free for the rest of the fight, into the discard
/// pile.
fn a_free_copy_of_itself() -> Command {
    Command::CardPile(CardPileCmd::AddGeneratedToCombat {
        enchantment: None,
        card: CardOrigin::CloneOfSource,
        grown_by: Amount::Fixed(0),
        count: Amount::Fixed(1),
        pile: PileName::Discard,
        ethereal: false,
        retain: false,
        upgraded: false,
        temporary_cost: Some(TemporaryCost::this_combat(0)),
        position: DrawPilePosition::Bottom,
    })
}

/// Every card of the discard pile that costs nothing now, moved to the
/// hand.
fn free_cards_of_the_discard_back() -> Command {
    Command::CardPile(CardPileCmd::MoveToPile {
        cards: CardSelector::Pile {
            pile: PileName::Discard,
            filter: CardFilter::CostsNothingNow,
        },
        pile: PileName::Hand,
        position: DrawPilePosition::Bottom,
    })
}

/// A Flak Cannon: the statuses are gathered, the hit count is worked out off
/// that list, every one of them burns, and only then does the swing land.
fn burn_the_statuses_and_swing(damage: i32) -> Command {
    Command::Select {
        cards: CardSelector::AllCombatCardsOutside {
            filter: CardFilter::OfType(CardType::Status),
            pile: PileName::Exhaust,
        },
        then: vec![Command::WithAmount {
            amount: Amount::counting(Count::ChosenCards),
            then: vec![
                Command::Card(CardCmd::Exhaust {
                    cards: CardSelector::Chosen,
                }),
                Command::Damage(DamageCmd::Attack {
                    amount: Amount::Fixed(damage),
                    hits: Amount::counting(Count::Bound),
                    target: CreatureSelector::RandomOpponent,
                    props: ValueProps::NONE,
                }),
            ],
        }],
    }
}

/// The energy spent this turn, less what this play itself paid: the card's own
/// cost is taken back off while the card is standing in the play pile, which it
/// always is while its body runs.
fn energy_spent_before_this_play() -> Amount {
    Amount::Sum(vec![
        Amount::counting(Count::OwnerEnergySpentThisTurn),
        Amount::negating(Count::SourceCardEnergyCost),
    ])
}

/// The Genetic Algorithm, whose block is a saved property: it starts at one,
/// grows by the Increase var on every play, and the growth is written through
/// to the run-deck card so that it outlives the fight.
fn genetic_algorithm() -> CardDefinition {
    let body = |increase: i32| {
        vec![
            // The block is the amount the card is carrying, which it keeps
            // at one plus everything its plays have added.
            Command::Creature(CreatureCmd::GainBlock {
                target: CreatureSelector::Owner,
                amount: Amount::counting(Count::SourceCardProperty("CurrentBlock".to_owned())),
                props: ValueProps::NONE,
            }),
            // The play grows this card and the deck's own copy of it: the
            // growth is added and then carried into the block.
            Command::Card(CardCmd::ModifyProperty {
                cards: CardSelector::Source,
                name: "IncreasedBlock".to_owned(),
                amount: Amount::Fixed(increase),
                and_the_deck_version: true,
            }),
            Command::Card(CardCmd::ModifyProperty {
                cards: CardSelector::Source,
                name: "CurrentBlock".to_owned(),
                amount: Amount::Fixed(increase),
                and_the_deck_version: true,
            }),
        ]
    };
    let mut definition = exhausting(upgrading(
        card(
            "CARD.GENETIC_ALGORITHM",
            CardType::Skill,
            TargetType::SelfTarget,
            1,
            body(3),
        ),
        body(4),
    ));
    definition.saved_properties = vec![
        sts2_engine::SavedCardProperty::of("CurrentBlock", 1),
        sts2_engine::SavedCardProperty::of("IncreasedBlock", 0),
    ];
    definition
}

/// Every card in hand goes to the draw pile one at a time, in hand order, and
/// then the pile is shuffled.
fn reshuffle_the_hand() -> Command {
    Command::CardPile(CardPileCmd::MoveToPile {
        cards: CardSelector::Pile {
            pile: PileName::Hand,
            filter: CardFilter::Any,
        },
        pile: PileName::Draw,
        position: DrawPilePosition::Bottom,
    })
}

/// Once per orb held, the next orb is evoked twice: once leaving it standing
/// and once taking it out of the queue.
fn evoke_every_orb_twice() -> Command {
    Command::Repeat {
        times: Amount::counting(Count::OrbsHeld),
        then: vec![evoke_next_keeping(), evoke_next()],
    }
}

fn ancients_and_multiplayer() -> Vec<CardDefinition> {
    vec![
        ancient(upgrading(
            card(
                "CARD.BIASED_COGNITION",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![
                    buff("POWER.FOCUS_POWER", 4),
                    buff("POWER.BIASED_COGNITION_POWER", 1),
                ],
            ),
            vec![
                buff("POWER.FOCUS_POWER", 5),
                buff("POWER.BIASED_COGNITION_POWER", 1),
            ],
        )),
        // Every evoke but the last leaves the orb standing.
        ancient(upgrading_cost(
            card(
                "CARD.QUADCAST",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![Command::When {
                    condition: Condition::CountAtLeast {
                        count: Count::OrbsHeld,
                        at_least: 1,
                    },
                    then: vec![Command::RepeatWithLast {
                        times: Amount::Fixed(4),
                        then: vec![evoke_next_keeping()],
                        last: vec![evoke_next()],
                    }],
                    otherwise: Vec::new(),
                }],
            ),
            0,
        )),
        // A cost of X: as many evokes as the play spent energy, plus one for
        // the upgraded form, and only the last of them dequeues.
        rarity(
            costing_x(upgrading(
                card(
                    "CARD.MULTI_CAST",
                    CardType::Skill,
                    TargetType::SelfTarget,
                    0,
                    vec![Command::RepeatWithLast {
                        times: Amount::counting(Count::EnergyX),
                        then: vec![evoke_next_keeping()],
                        last: vec![evoke_next()],
                    }],
                ),
                vec![Command::RepeatWithLast {
                    times: Amount::Scaled {
                        base: 1,
                        per: 1,
                        count: Count::EnergyX,
                    },
                    then: vec![evoke_next_keeping()],
                    last: vec![evoke_next()],
                }],
            )),
            sts2_engine::CardRarity::Rare,
        ),
        // The two multiplayer-only cards: registered
        // for what they are, because the pool holds them, and never offered to
        // a single-player run.
        rarity(
            upgrade_drops_exhaust(exhausting(upgrading_in_place(card(
                "CARD.IGNITION",
                CardType::Skill,
                TargetType::AnyAlly,
                1,
                vec![channel(PLASMA)],
            )))),
            sts2_engine::CardRarity::Rare,
        ),
        rarity(
            exhausting(upgrading(
                card(
                    "CARD.ENERGY_SURGE",
                    CardType::Skill,
                    TargetType::AllAllies,
                    1,
                    vec![energy(2)],
                ),
                vec![energy(3)],
            )),
            sts2_engine::CardRarity::Uncommon,
        ),
    ]
}
