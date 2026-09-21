//! The five orbs.
//!
//! An orb has two numbers and two effects. What each number is worth is either
//! a constant, a constant that orb-value effects such as Focus can raise, or a
//! value the orb keeps for itself; what each effect does is a run of commands
//! like anything else. Which of the two turn triggers an orb answers to is its
//! own: four of them fire as the turn ends and the Plasma orb fires as the next
//! one opens.
//!
//! Nothing here reacts to anything but its own trigger. The orb queue does sit
//! in the combat's listener order, between the potion belt and the piles, but
//! no orb in the pinned build answers any of those events, so an orb only ever
//! answers for itself.

use sts2_engine::{
    Amount, Command, Condition, Count, CreatureCmd, CreatureSelector, OrbCmd, OrbDefinition,
    OrbField, OrbSelector, OrbTrigger, OrbValue, PlayerCmd, ValueProps,
};

use crate::id;

/// An orb's passive and evoke numbers, read where the build reads them: inside
/// the effect that is running.
fn passive_value() -> Amount {
    Amount::counting(Count::OrbPassiveValue)
}

fn evoke_value() -> Amount {
    Amount::counting(Count::OrbEvokeValue)
}

/// Every orb the pinned build has, in the game's own order — which is the
/// order a rolled orb indexes into.
#[must_use]
pub fn orbs() -> Vec<OrbDefinition> {
    vec![
        lightning_orb(),
        frost_orb(),
        dark_orb(),
        plasma_orb(),
        glass_orb(),
    ]
}

/// The Lightning orb. Its passive and its evoke do the same thing with a
/// different number and a different target: the passive hits whoever the
/// effect that ran it named and the evoke never names anyone, so the evoke
/// always rolls its target.
///
/// The build settles the target list before it damages anything and hands that
/// same list back, which is what a Thunder then swings at. Where no opponent
/// can be hit it hands back an empty list without ever looking at the target
/// it was given, so nothing is rolled and nothing is announced there.
fn lightning_orb() -> OrbDefinition {
    OrbDefinition {
        model_id: id("ORB.LIGHTNING_ORB"),
        passive_value: OrbValue::Modified(3),
        evoke_value: OrbValue::Modified(8),
        turn_trigger: OrbTrigger::BeforeTurnEnd,
        passive: lightning_damage(passive_value()),
        evoke: lightning_damage(evoke_value()),
    }
}

/// The Lightning orb's blow, which its passive and its evoke both deal.
fn lightning_damage(amount: Amount) -> Vec<Command> {
    vec![Command::WithCreatures {
        creatures: CreatureSelector::TargetOrRandomOpponent,
        then: vec![
            Command::Creature(CreatureCmd::Damage {
                target: CreatureSelector::Bound,
                amount,
                props: ValueProps::UNPOWERED,
            }),
            Command::Orb(OrbCmd::Reached(CreatureSelector::Bound)),
        ],
    }]
}

/// The Frost orb: block for its owner, both ways, and the owner is what the
/// evoke hands back.
fn frost_orb() -> OrbDefinition {
    OrbDefinition {
        model_id: id("ORB.FROST_ORB"),
        passive_value: OrbValue::Modified(2),
        evoke_value: OrbValue::Modified(5),
        turn_trigger: OrbTrigger::BeforeTurnEnd,
        passive: vec![Command::Creature(CreatureCmd::GainBlock {
            target: CreatureSelector::Owner,
            amount: passive_value(),
            props: ValueProps::UNPOWERED,
        })],
        evoke: vec![
            Command::Creature(CreatureCmd::GainBlock {
                target: CreatureSelector::Owner,
                amount: evoke_value(),
                props: ValueProps::UNPOWERED,
            }),
            Command::Orb(OrbCmd::Reached(CreatureSelector::Owner)),
        ],
    }
}

/// The Dark orb: the passive does nothing but grow the evoke, and the evoke
/// hits whoever is standing on the least health.
///
/// Its passive is worth 6 and orb-value effects can raise it, while its evoke
/// is whatever has been banked into it, so a Focus raises what each tick banks
/// and leaves what has already been banked alone. The weakest opponent is the
/// first of a tie, and a field with nothing on it names nobody, damages
/// nobody, and hands nothing back.
fn dark_orb() -> OrbDefinition {
    OrbDefinition {
        model_id: id("ORB.DARK_ORB"),
        passive_value: OrbValue::Modified(6),
        evoke_value: OrbValue::Fixed(6),
        turn_trigger: OrbTrigger::BeforeTurnEnd,
        passive: vec![Command::Orb(OrbCmd::SetValue {
            orb: OrbSelector::Bound,
            field: OrbField::Evoke,
            amount: Amount::Sum(vec![
                Amount::counting(Count::OrbEvokeField),
                passive_value(),
            ]),
        })],
        evoke: vec![Command::WithCreatures {
            creatures: CreatureSelector::WeakestOpponent,
            then: vec![
                Command::Creature(CreatureCmd::Damage {
                    target: CreatureSelector::Bound,
                    amount: evoke_value(),
                    props: ValueProps::UNPOWERED,
                }),
                Command::Orb(OrbCmd::Reached(CreatureSelector::Bound)),
            ],
        }],
    }
}

/// The Plasma orb: energy, and the one orb whose trigger is the turn opening
/// rather than the turn ending. Neither of its numbers can be raised by an
/// orb-value effect, so a Focus is worth nothing to it.
fn plasma_orb() -> OrbDefinition {
    OrbDefinition {
        model_id: id("ORB.PLASMA_ORB"),
        passive_value: OrbValue::Fixed(1),
        evoke_value: OrbValue::Fixed(2),
        turn_trigger: OrbTrigger::AfterTurnStart,
        passive: vec![Command::Player(PlayerCmd::GainEnergy(passive_value()))],
        evoke: vec![
            Command::Player(PlayerCmd::GainEnergy(evoke_value())),
            Command::Orb(OrbCmd::Reached(CreatureSelector::Owner)),
        ],
    }
}

/// The Glass orb: a passive that hits everything and wears itself down by one
/// every time it does, and an evoke worth double whatever the passive is worth
/// now.
///
/// Three things about the passive are the build's own order and are
/// load-bearing. The targets and the value are both worked out before anything
/// happens. The guard is on that captured value rather than on the orb's
/// passive value, so a Focus that holds the value above nothing keeps a
/// worn-out orb firing. And the passive value is worn down *before* the blow
/// lands rather than after it, which is what anything answering that blow
/// reads back.
fn glass_orb() -> OrbDefinition {
    OrbDefinition {
        model_id: id("ORB.GLASS_ORB"),
        passive_value: OrbValue::Modified(4),
        evoke_value: OrbValue::PassiveTimes(2),
        turn_trigger: OrbTrigger::BeforeTurnEnd,
        passive: vec![Command::WithCreatures {
            creatures: CreatureSelector::AllOpponents,
            then: vec![Command::WithAmount {
                amount: passive_value(),
                then: vec![Command::When {
                    condition: Condition::CountAtLeast {
                        count: Count::Bound,
                        at_least: 1,
                    },
                    then: vec![
                        Command::Orb(OrbCmd::SetValue {
                            orb: OrbSelector::Bound,
                            field: OrbField::Passive,
                            amount: Amount::Greatest(vec![
                                Amount::Fixed(0),
                                Amount::Sum(vec![
                                    Amount::counting(Count::OrbPassiveField),
                                    Amount::Fixed(-1),
                                ]),
                            ]),
                        }),
                        Command::Creature(CreatureCmd::Damage {
                            target: CreatureSelector::Bound,
                            amount: Amount::counting(Count::Bound),
                            props: ValueProps::UNPOWERED,
                        }),
                    ],
                    otherwise: Vec::new(),
                }],
            }],
        }],
        evoke: vec![Command::WithCreatures {
            creatures: CreatureSelector::AllOpponents,
            then: vec![Command::When {
                condition: Condition::CountAtLeast {
                    count: Count::OrbEvokeValue,
                    at_least: 1,
                },
                then: vec![
                    Command::Creature(CreatureCmd::Damage {
                        target: CreatureSelector::Bound,
                        amount: evoke_value(),
                        props: ValueProps::UNPOWERED,
                    }),
                    Command::Orb(OrbCmd::Reached(CreatureSelector::Bound)),
                ],
                otherwise: Vec::new(),
            }],
        }],
    }
}
