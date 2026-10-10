//! Power definitions: what each power declares, and what its listener bodies
//! do.
//!
//! A power is its stacking metadata plus a set of listener methods. How it
//! changes damage, block, and the HP a blow actually costs are listener methods
//! like any other — the `Modify*` family — so nothing about a power is a flag
//! the engine interprets by model ID.

use sts2_engine::{
    Amount, CardCmd, CardFilter, CardOrigin, CardPileCmd, CardSelectCmd, CardSelector, Command,
    Condition, Count, CreatureCmd, CreatureSelector, DrawPilePosition, HookCondition, HookEvent,
    HookTiming, ModifierCmd, ModifierHook, PileName, PlayerCmd, PowerCmd, PowerDefinition,
    PowerInstanceType, PowerRef, PowerStackType, PowerType, SelectorPrefs, StunFollowUp,
    TemporaryPower, ValueProps,
};
use sts2_engine::{
    ChoiceDestination, ChoiceDuration, ChoiceOperation as ChoiceOp, ChoiceOperationKind,
    ChoiceScope, ChoiceSemantics,
};

use crate::id;

/// A power with nothing declared past its type and how it carries its amount.
fn power(model_id: &str, power_type: PowerType, stack_type: PowerStackType) -> PowerDefinition {
    PowerDefinition {
        stops_combat_from_ending: false,
        must_perform_move_on_death: false,
        model_id: id(model_id),
        power_type,
        stack_type,
        instance_type: PowerInstanceType::None,
        allow_negative: false,
        owner_is_secondary_enemy: false,
        owner_removed_from_combat_on_death: true,
        temporary: None,
        play_restriction: None,
        owner_death_triggers_fatal: true,
        keeps_block: false,
        stops_draws_outside_hand_draw: false,
        keeps_hand: false,
        removed_when_applier_dies: false,
        keeps_owners_powers_on_death: false,
        survives_owner_death: false,
        redirects_owners_powered_attacks: false,
        holds_owner_where_it_stands: false,
        holds_owner_only_with_living_kin: false,
        move_on_death: None,
        on_removed: Vec::new(),
        card_cost_rule: None,
        played_card_rule: None,
        makes_owners_cards_ethereal: false,
        hooks: Vec::new(),
    }
}

/// A counter power, which is what all but the marker powers are.
fn counter(model_id: &str, power_type: PowerType) -> PowerDefinition {
    power(model_id, power_type, PowerStackType::Counter)
}

/// A power whose amount is hidden and which nothing ever applies twice.
fn single(model_id: &str, power_type: PowerType) -> PowerDefinition {
    power(model_id, power_type, PowerStackType::Single)
}

/// One value-modifying listener body: the pass it joins, the test it opens
/// with, and what it says about the value.
fn modifies(event: HookEvent, condition: HookCondition, commands: Vec<Command>) -> ModifierHook {
    ModifierHook::every_turn(event, commands).when(condition)
}

/// Debilitate's pass over the Vulnerable and Weak multipliers, which is the
/// same arithmetic said twice: both come to twice the multiplier less one, so
/// a three-quarters weakness becomes a half and a half-again vulnerability
/// becomes a double.
///
/// A Weak multiplier is asked of the dealer and a Vulnerable one of the
/// target, and either way the Debilitate that answers is the one sitting on
/// the same creature as the power being asked. So the test is the listening
/// power's own owner.
fn debilitated(multiplier: Amount) -> Amount {
    Amount::IfElse {
        condition: Condition::OwnerHasPower(id("POWER.DEBILITATE_POWER")),
        then: Box::new(Amount::Sum(vec![
            Amount::Product {
                left: Box::new(Amount::Fixed(2)),
                right: Box::new(multiplier.clone()),
            },
            Amount::Fixed(-100),
        ])),
        otherwise: Box::new(multiplier),
    }
}

/// The power's own current amount.
fn stacks() -> Amount {
    Amount::counting(Count::ListeningPowerAmount)
}

/// Ticks the power's own duration down.
fn tick_down() -> Command {
    Command::Power(PowerCmd::TickDownDuration {
        power: PowerRef::listening(),
    })
}

/// Takes one stack off the power itself.
fn decrement() -> Command {
    Command::Power(PowerCmd::Decrement {
        power: PowerRef::listening(),
    })
}

/// A slumbering beetle waking up, less the noise and the animation it makes:
/// the shell comes off, if the shell is still on.
fn wakes_up() -> Vec<Command> {
    vec![Command::When {
        otherwise: Vec::new(),
        condition: Condition::OwnerHasPower(id("POWER.PLATING_POWER")),
        then: vec![Command::Power(PowerCmd::Remove {
            power: PowerRef::owned(id("POWER.PLATING_POWER")),
        })],
    }]
}

/// Takes the power itself off its owner.
fn remove_self() -> Command {
    Command::Power(PowerCmd::Remove {
        power: PowerRef::listening(),
    })
}

/// Works another power on the listener's owner, as many stacks as `amount`.
fn applied_to(power: &str, target: CreatureSelector, amount: Amount) -> Command {
    Command::Power(PowerCmd::Apply {
        marks: None,
        power: id(power),
        target,
        applier: Some(CreatureSelector::Owner),
        amount,
    })
}

fn apply_to_owner(power: &str, applier: CreatureSelector, amount: Amount) -> Command {
    Command::Power(PowerCmd::Apply {
        marks: None,
        power: id(power),
        target: CreatureSelector::Owner,
        applier: Some(applier),
        amount,
    })
}

/// A timed debuff: it counts down as the enemy side's turn ends, whichever side
/// is carrying it.
fn wearing_off(mut definition: PowerDefinition) -> PowerDefinition {
    definition.hooks.push(
        ModifierHook::every_turn(HookEvent::AfterSideTurnEnd, vec![tick_down()])
            .when(HookCondition::EnemySideTurn),
    );
    definition
}

/// A power that only marks its owner as a secondary enemy.
fn minion_power(model_id: &str) -> PowerDefinition {
    PowerDefinition {
        owner_is_secondary_enemy: true,
        // A minion's death is not a fatal blow: killing a minion is not the
        // kill a Feed asks for.
        owner_death_triggers_fatal: false,
        ..single(model_id, PowerType::Buff)
    }
}

/// The powers whose whole behavior is a pass over a value.
fn damage_shaping_powers() -> Vec<PowerDefinition> {
    vec![
        // Dexterity adds to the block its owner works up. Block off a card
        // belongs to whoever played it, which is why the test is not simply
        // who is gaining it.
        PowerDefinition {
            allow_negative: true,
            hooks: vec![modifies(
                HookEvent::ModifyBlockAdditive,
                HookCondition::OwnerGrantsPoweredBlock,
                vec![Command::Modify(ModifierCmd::Add(stacks()))],
            )],
            ..counter("POWER.DEXTERITY_POWER", PowerType::Buff)
        },
        // Strength adds to what its owner deals, and shows as a debuff while it
        // is down.
        PowerDefinition {
            allow_negative: true,
            hooks: vec![modifies(
                HookEvent::ModifyDamageAdditive,
                HookCondition::OwnerDealsPoweredDamage,
                vec![Command::Modify(ModifierCmd::Add(stacks()))],
            )],
            ..counter("POWER.STRENGTH_POWER", PowerType::Buff)
        },
        // Shrink counts itself down at the end of a turn its owner took part
        // in, unless it was worked on backwards: a shrink applied for less
        // than nothing is the permanent kind and never wears off.
        PowerDefinition {
            allow_negative: true,
            // A shrink comes off when whoever worked it goes down.
            removed_when_applier_dies: true,
            hooks: vec![
                scales_damage_dealt(70),
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnEnd,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningPowerAmountAbove(0),
                        then: vec![decrement()],
                    }],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.SHRINK_POWER", PowerType::Debuff)
        },
        // Weak and Vulnerable wear off as the enemy side's turn ends,
        // whichever side is carrying them.
        //
        // Weak's own multiplier is three quarters, and the body looks up two
        // things that may take it down further before it answers: a paper krane
        // on whoever is taking the blow, worth fifteen hundredths, and a
        // Debilitate on whoever is swinging, which doubles what is left of the
        // reduction. Both are here.
        wearing_off(PowerDefinition {
            hooks: vec![modifies(
                HookEvent::ModifyDamageMultiplicative,
                HookCondition::OwnerDealsPoweredDamage,
                vec![Command::Modify(ModifierCmd::Scale(debilitated(
                    Amount::per(75, -15, Count::DamageTargetRelic(id("RELIC.PAPER_KRANE"))),
                )))],
            )],
            ..counter("POWER.WEAK_POWER", PowerType::Debuff)
        }),
        // Vulnerable's multiplier is not a constant: the power offers its own
        // increase to whatever wants to raise it, in order — a Paper Phrog and
        // a Cruelty on whoever is swinging, and a Debilitate on whoever is
        // taking it. Two of the three are here: a Cruelty adds its own stacks
        // as hundredths, and a Paper Phrog held by whoever is swinging adds a
        // quarter. Both are additive, so the order they are asked in does not
        // show.
        wearing_off(PowerDefinition {
            hooks: vec![modifies(
                HookEvent::ModifyDamageMultiplicative,
                HookCondition::OwnerTakesPoweredDamage,
                vec![Command::Modify(ModifierCmd::Scale(debilitated(
                    Amount::Sum(vec![
                        Amount::per(150, 1, Count::DamageDealerPower(id("POWER.CRUELTY_POWER"))),
                        Amount::per(0, 25, Count::DamageDealerRelic(id("RELIC.PAPER_PHROG"))),
                    ]),
                )))],
            )],
            ..counter("POWER.VULNERABLE_POWER", PowerType::Debuff)
        }),
    ]
}

/// The Kaiser Crab's rage: the half that outlives the other takes six strength
/// and ninety-nine block, and the rage is spent.
fn crab_rage_power() -> PowerDefinition {
    PowerDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterDeath,
                vec![
                    apply_to_owner(
                        "POWER.STRENGTH_POWER",
                        CreatureSelector::Owner,
                        Amount::Fixed(6),
                    ),
                    Command::Creature(CreatureCmd::GainBlock {
                        target: CreatureSelector::Owner,
                        amount: Amount::Fixed(99),
                        props: ValueProps::UNPOWERED,
                    }),
                    remove_self(),
                ],
            )
            .when(HookCondition::AllyDied),
        ],
        ..single("POWER.CRAB_RAGE_POWER", PowerType::Buff)
    }
}

/// A crimson mantle pays block at the start of every turn, and costs its owner
/// one HP for every time the card was laid down.
fn crimson_mantle_power() -> PowerDefinition {
    PowerDefinition {
        hooks: vec![ModifierHook::every_turn(
            HookEvent::AfterPlayerTurnStart,
            vec![
                Command::Creature(CreatureCmd::Damage {
                    target: CreatureSelector::Owner,
                    amount: Amount::counting(Count::ListeningPowerCounter),
                    props: ValueProps::UNBLOCKABLE,
                }),
                Command::Creature(CreatureCmd::GainBlock {
                    target: CreatureSelector::Owner,
                    amount: stacks(),
                    props: ValueProps::UNPOWERED,
                }),
            ],
        )],
        ..counter("POWER.CRIMSON_MANTLE_POWER", PowerType::Buff)
    }
}

/// A ritual makes its owner stronger at the end of every turn it takes part
/// in, from the turn after the one it landed on.
fn ritual_power() -> PowerDefinition {
    PowerDefinition {
        hooks: vec![
            // An enemy that put a ritual on itself skips the first payout,
            // which the power remembers as a mark of its own.
            ModifierHook::every_turn(
                HookEvent::AfterApplied,
                vec![Command::Power(PowerCmd::SetCounter {
                    power: PowerRef::listening(),
                    amount: Amount::Fixed(1),
                })],
            )
            .when(HookCondition::OwnerIsEnemy),
            ModifierHook::every_turn(
                HookEvent::AfterSideTurnEnd,
                // The skip is read before it is cleared, so the two halves of
                // the build's `if` cannot both run.
                vec![
                    Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::Not(Box::new(Condition::ListeningPowerCounterIs(1))),
                        then: vec![apply_to_owner(
                            "POWER.STRENGTH_POWER",
                            CreatureSelector::Owner,
                            stacks(),
                        )],
                    },
                    Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningPowerCounterIs(1),
                        then: vec![Command::Power(PowerCmd::SetCounter {
                            power: PowerRef::listening(),
                            amount: Amount::Fixed(0),
                        })],
                    },
                ],
            )
            .when(HookCondition::OwnerIsParticipant),
        ],
        ..counter("POWER.RITUAL_POWER", PowerType::Buff)
    }
}

/// Paper cuts take the player's maximum HP down whenever their owner's attack
/// costs the player any.
fn paper_cuts_power() -> PowerDefinition {
    PowerDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterDamageGiven,
                vec![Command::Creature(CreatureCmd::LoseMaxHp {
                    target: CreatureSelector::Target,
                    amount: stacks(),
                })],
            )
            .when(HookCondition::OwnerDealtPoweredHpLossToPlayer),
        ],
        ..counter("POWER.PAPER_CUTS_POWER", PowerType::Buff)
    }
}

/// A segment that goes down stays where it is, holds still, and reattaches
/// itself for twenty-five HP.
fn reattach_power() -> PowerDefinition {
    PowerDefinition {
        owner_removed_from_combat_on_death: false,
        survives_owner_death: true,
        holds_owner_only_with_living_kin: true,
        move_on_death: Some("DEAD_MOVE".to_owned()),
        ..single("POWER.REATTACH_POWER", PowerType::Buff)
    }
}

/// A burrowed creature keeps its block, and the blow that finally breaks it
/// leaves the creature dizzy: it is stunned, and the block it was standing
/// behind goes with the power.
fn burrowed_power() -> PowerDefinition {
    PowerDefinition {
        owner_death_triggers_fatal: true,
        keeps_block: true,
        stops_draws_outside_hand_draw: false,
        keeps_hand: false,
        hooks: vec![ModifierHook::every_turn(
            HookEvent::AfterBlockBroken,
            vec![
                Command::Creature(CreatureCmd::Stun {
                    target: CreatureSelector::Owner,
                    next_move: StunFollowUp::Named("BITE_MOVE".to_owned()),
                    stun_move: Vec::new(),
                }),
                remove_self(),
            ],
        )],
        on_removed: vec![Command::Creature(CreatureCmd::LoseBlock {
            target: CreatureSelector::Owner,
            amount: Amount::Fixed(999_999_999),
        })],
        ..single("POWER.BURROWED_POWER", PowerType::Buff)
    }
}

/// Thorns answer whoever struck their owner, before block takes any of the
/// blow, and what they answer with nothing scales.
fn thorns_power() -> PowerDefinition {
    PowerDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::BeforeDamageReceived,
                vec![Command::Creature(CreatureCmd::Damage {
                    target: CreatureSelector::DamageDealer,
                    amount: stacks(),
                    props: ValueProps::UNPOWERED,
                })],
            )
            .when(HookCondition::OwnerTakesPoweredDamageOrOmnislice),
        ],
        ..counter("POWER.THORNS_POWER", PowerType::Buff)
    }
}

/// The Ceremonial Beast's charge. Once its owner is worn down to what the
/// power is worth, the charge breaks: the strength it built up goes, the beast
/// is stunned into its second phase, and the power is spent.
fn plow_power() -> PowerDefinition {
    PowerDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterDamageReceived,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::OwnerHpAtMostListeningPowerAmount,
                    then: vec![
                        Command::Power(PowerCmd::Remove {
                            power: PowerRef::owned(id("POWER.STRENGTH_POWER")),
                        }),
                        // The ceremonial beast is stunned into its cry, and
                        // anything else carrying the power takes a plain stun
                        // that resumes whatever it last performed.
                        Command::When {
                            otherwise: Vec::new(),
                            condition: Condition::OwnerIsMonster(id("MONSTER.CEREMONIAL_BEAST")),
                            then: vec![Command::Creature(CreatureCmd::Stun {
                                target: CreatureSelector::Owner,
                                next_move: StunFollowUp::Named("BEAST_CRY_MOVE".to_owned()),
                                stun_move: Vec::new(),
                            })],
                        },
                        Command::When {
                            otherwise: Vec::new(),
                            condition: Condition::Not(Box::new(Condition::OwnerIsMonster(id(
                                "MONSTER.CEREMONIAL_BEAST",
                            )))),
                            then: vec![Command::Creature(CreatureCmd::Stun {
                                target: CreatureSelector::Owner,
                                next_move: StunFollowUp::TheLastMovePerformed,
                                stun_move: Vec::new(),
                            })],
                        },
                        remove_self(),
                    ],
                }],
            )
            .when(HookCondition::OwnerTookUnblockedDamage),
        ],
        ..counter("POWER.PLOW_POWER", PowerType::Debuff)
    }
}

/// The beast's cry. While it is on, its owner can only get one card played a
/// turn.
fn ringing_power() -> PowerDefinition {
    PowerDefinition {
        hooks: vec![
            ModifierHook::every_turn(HookEvent::AfterSideTurnEnd, vec![remove_self()])
                .when(HookCondition::OwnerIsParticipant),
        ],
        play_restriction: Some(sts2_engine::PlayRestriction::OneCardEachTurn),
        ..single("POWER.RINGING_POWER", PowerType::Debuff)
    }
}

/// Slowness counts the cards played while it is on and makes every powered
/// blow that lands on its owner ten percent heavier a card, starting over as
/// its owner's side takes its turn.
fn slow_power() -> PowerDefinition {
    PowerDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterCardPlayed,
                vec![Command::Power(PowerCmd::ModifyCounter {
                    power: PowerRef::listening(),
                    amount: Amount::Fixed(1),
                })],
            ),
            modifies(
                HookEvent::ModifyDamageMultiplicative,
                HookCondition::OwnerTakesPoweredDamage,
                vec![Command::Modify(ModifierCmd::Scale(Amount::per(
                    100,
                    10,
                    Count::ListeningPowerCounter,
                )))],
            ),
            ModifierHook::every_turn(
                HookEvent::AfterSideTurnStart,
                vec![Command::Power(PowerCmd::SetCounter {
                    power: PowerRef::listening(),
                    amount: Amount::Fixed(0),
                })],
            )
            .when(HookCondition::OwnerIsParticipant),
        ],
        ..counter("POWER.SLOW_POWER", PowerType::Debuff)
    }
}

/// Scales what the owner deals, as a percentage.
fn scales_damage_dealt(percent: i32) -> ModifierHook {
    modifies(
        HookEvent::ModifyDamageMultiplicative,
        HookCondition::OwnerDealsPoweredDamage,
        vec![Command::Modify(ModifierCmd::Scale(Amount::Fixed(percent)))],
    )
}

/// Scales what the owner takes, as a percentage.
fn scales_damage_taken(percent: i32) -> ModifierHook {
    modifies(
        HookEvent::ModifyDamageMultiplicative,
        HookCondition::OwnerTakesPoweredDamage,
        vec![Command::Modify(ModifierCmd::Scale(Amount::Fixed(percent)))],
    )
}

/// The two possession powers, which differ only in the stat they take: every
/// point their owner takes off a player is written into
/// the tally, and the whole of it goes back when their owner dies.
fn possession(model_id: &str, stat: &str) -> PowerDefinition {
    PowerDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterPowerAmountChanged,
                vec![Command::Power(PowerCmd::ModifyCounter {
                    power: PowerRef::listening(),
                    amount: Amount::counting(Count::AnnouncedAmount),
                })],
            )
            .when(HookCondition::OwnerTookPowerFromAPlayer(id(stat))),
            ModifierHook::every_turn(
                HookEvent::AfterDeath,
                vec![Command::Power(PowerCmd::Apply {
                    power: id(stat),
                    target: CreatureSelector::AllOpponents,
                    // The debuff is applied with no applier: nobody works
                    // it, so nothing takes it away again.
                    applier: None,
                    amount: Amount::Product {
                        left: Box::new(Amount::Fixed(-1)),
                        right: Box::new(Amount::counting(Count::ListeningPowerCounter)),
                    },
                    marks: None,
                })],
            )
            .when(HookCondition::OwnerDied),
        ],
        ..single(model_id, PowerType::Buff)
    }
}

/// Powers that act at a turn boundary or react to what happens.
#[allow(
    clippy::too_many_lines,
    reason = "one entry per power, which is the table this is"
)]
fn reacting_powers() -> Vec<PowerDefinition> {
    vec![
        flex_potion_power(),
        // An illusion holds the field when it dies and comes back whole, and
        // marks its owner a minion as it lands.
        PowerDefinition {
            owner_removed_from_combat_on_death: false,
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterApplied,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::OwnerLacksPower(id("POWER.MINION_POWER")),
                    then: vec![Command::Power(PowerCmd::Apply {
                        marks: None,
                        power: id("POWER.MINION_POWER"),
                        target: CreatureSelector::Owner,
                        // Attached by nobody, so nothing takes it away by
                        // dying.
                        applier: None,
                        amount: Amount::Fixed(1),
                    })],
                }],
            )],
            keeps_owners_powers_on_death: true,
            ..single("POWER.ILLUSION_POWER", PowerType::Buff)
        },
        minion_power("POWER.MINION_POWER"),
        // While it holds, each hit only costs one HP, and landing a hit uses
        // one of its stacks.
        PowerDefinition {
            hooks: vec![
                modifies(
                    HookEvent::ModifyHpLostAfterOsty,
                    HookCondition::OwnerTakesDamage,
                    vec![Command::Modify(ModifierCmd::CapAt(Amount::Fixed(1)))],
                ),
                // Only a blow that got past block spends a stack: one block
                // stopped whole is still offered to the body, and costs it
                // nothing.
                ModifierHook::every_turn(HookEvent::AfterDamageReceived, vec![decrement()])
                    .when(HookCondition::OwnerTookUnblockedDamage),
            ],
            ..counter("POWER.SLIPPERY_POWER", PowerType::Buff)
        },
        // Rage pays block for every attack until the turn ends.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardPlayed,
                    vec![Command::Creature(CreatureCmd::GainBlock {
                        target: CreatureSelector::Owner,
                        amount: stacks(),
                        props: ValueProps::UNPOWERED,
                    })],
                )
                .when(HookCondition::PlayedCardIsAttack),
                ModifierHook::every_turn(HookEvent::AfterSideTurnEnd, vec![remove_self()])
                    .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.RAGE_POWER", PowerType::Buff)
        },
        // Infested owes four wrigglers: when its owner goes down they arrive
        // stunned, one to each of the encounter's wriggler slots. It never
        // takes itself off — it goes with the creature it was on — and while it
        // is still there it holds the fight open, which is what keeps the fight
        // alive in the window between the phrog dying and the wrigglers
        // standing.
        PowerDefinition {
            stops_combat_from_ending: true,
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterDeath,
                    (0..4)
                        .map(|slot| {
                            Command::Creature(CreatureCmd::Add {
                                monster: id("MONSTER.WRIGGLER"),
                                // The wriggler slots are named in order.
                                slot: sts2_engine::SpawnSlot::Named(format!(
                                    "wriggler{}",
                                    slot + 1
                                )),
                                // Each wriggler arrives stunned.
                                initial_state: Some("SPAWNED_MOVE".to_owned()),
                                then: Vec::new(),
                            })
                        })
                        .collect(),
                )
                .when(HookCondition::OwnerDied),
            ],
            ..single("POWER.INFESTED_POWER", PowerType::Buff)
        },
        // Adaptable: what a Test Subject dies behind. Its owner is not taken
        // out of the fight, goes onto its own respawn move, keeps this power
        // through the death, and holds the fight open while it stands.
        PowerDefinition {
            owner_removed_from_combat_on_death: false,
            survives_owner_death: true,
            stops_combat_from_ending: true,
            move_on_death: Some("RESPAWN_MOVE".to_owned()),
            must_perform_move_on_death: true,
            ..single("POWER.ADAPTABLE_POWER", PowerType::Buff)
        },
        // Enrage: a skill played in front of it makes its owner stronger.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardPlayed,
                    vec![apply_to_owner(
                        "POWER.STRENGTH_POWER",
                        CreatureSelector::Owner,
                        stacks(),
                    )],
                )
                .when(HookCondition::PlayedCardIsOfType(
                    sts2_engine::CardType::Skill,
                )),
            ],
            ..counter("POWER.ENRAGE_POWER", PowerType::Buff)
        },
        // Painful stabs: every blow of its owner's swing that gets past a
        // player's block buries that player in as many Wounds as it stands at.
        // It stays on a dead owner and keeps them in the fight, because the
        // Test Subject wears it across a revive.
        PowerDefinition {
            owner_removed_from_combat_on_death: false,
            survives_owner_death: true,
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterAttack,
                    vec![Command::CardPile(
                        sts2_engine::CardPileCmd::AddGeneratedToCombat {
                            enchantment: None,
                            card: sts2_engine::CardOrigin::Model(id("CARD.WOUND")),
                            grown_by: Amount::Fixed(0),
                            count: Amount::Product {
                                left: Box::new(stacks()),
                                right: Box::new(Amount::counting(Count::UnblockedHitsOnPlayers)),
                            },
                            pile: sts2_engine::PileName::Discard,
                            ethereal: false,
                            retain: false,
                            upgraded: false,
                            temporary_cost: None,
                            position: sts2_engine::DrawPilePosition::Bottom,
                        },
                    )],
                )
                .when(HookCondition::OwnerDealsPoweredDamage),
            ],
            ..counter("POWER.PAINFUL_STABS_POWER", PowerType::Buff)
        },
        // Nemesis: its owner is intangible every other turn, and the boolean it
        // flips is the latch.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnEnd,
                    vec![Command::When {
                        condition: Condition::ListeningPowerIsLatched,
                        // It was on, so it goes off and the intangibility with
                        // it.
                        then: vec![
                            Command::Power(PowerCmd::SetLatch(false)),
                            Command::Power(PowerCmd::Remove {
                                power: PowerRef::owned(id("POWER.INTANGIBLE_POWER")),
                            }),
                        ],
                        otherwise: vec![
                            Command::Power(PowerCmd::SetLatch(true)),
                            apply_to_owner(
                                "POWER.INTANGIBLE_POWER",
                                CreatureSelector::Owner,
                                Amount::Fixed(1),
                            ),
                        ],
                    }],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..single("POWER.NEMESIS_POWER", PowerType::Buff)
        },
        // High voltage: its owner takes as much strength as it is standing at,
        // at the end of every turn it took part in.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnEnd,
                    vec![apply_to_owner(
                        "POWER.STRENGTH_POWER",
                        CreatureSelector::Owner,
                        stacks(),
                    )],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.HIGH_VOLTAGE_POWER", PowerType::Buff)
        },
        // A stock: what an Axebot has left of itself. When one dies holding
        // any, another takes its place in the same slot, booting up rather than
        // swinging, and carrying one less. It holds the fight open while it
        // still owes one.
        PowerDefinition {
            stops_combat_from_ending: true,
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterDeath,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningPowerAmountAbove(0),
                        then: vec![Command::Creature(CreatureCmd::Add {
                            monster: id("MONSTER.AXEBOT"),
                            slot: sts2_engine::SpawnSlot::SameAsOwner,
                            // A bot built with a stock of its own opens on
                            // its boot-up.
                            initial_state: Some("BOOT_UP_MOVE".to_owned()),
                            // The bot arrives carrying one stock less than
                            // the power is worth, which it puts on itself as
                            // it lands.
                            then: vec![Command::Power(PowerCmd::Apply {
                                power: id("POWER.STOCK_POWER"),
                                target: CreatureSelector::Target,
                                applier: None,
                                amount: Amount::Scaled {
                                    base: -1,
                                    per: 1,
                                    count: Count::ListeningPowerAmount,
                                },
                                marks: None,
                            })],
                        })],
                    }],
                )
                .when(HookCondition::OwnerDied),
            ],
            ..counter("POWER.STOCK_POWER", PowerType::Buff)
        },
        // A possession: it keeps a tally of what its owner has taken off the
        // player, and gives every point of it back when its owner goes down.
        possession("POWER.POSSESS_STRENGTH_POWER", "POWER.STRENGTH_POWER"),
        possession("POWER.POSSESS_SPEED_POWER", "POWER.DEXTERITY_POWER"),
        // The Insatiable's sandpit. It sits on the boss, names the player it
        // swallowed, and counts down one at the start of every enemy turn.
        // When it comes off, whoever it named is eaten — unless one of them is
        // already down, which is the early return the body opens with.
        PowerDefinition {
            instance_type: PowerInstanceType::Instanced,
            hooks: vec![
                ModifierHook::every_turn(HookEvent::AfterSideTurnStartLate, vec![decrement()])
                    .when(HookCondition::EnemySideTurn),
            ],
            on_removed: vec![Command::When {
                otherwise: Vec::new(),
                condition: Condition::All(vec![
                    Condition::OwnerIsAlive,
                    Condition::ListeningPowerTargetIsAlive,
                ]),
                then: vec![Command::Creature(CreatureCmd::Kill {
                    target: CreatureSelector::ListeningPowerTarget,
                })],
            }],
            ..counter("POWER.SANDPIT_POWER", PowerType::Buff)
        },
        // A hatch counts down at the end of every turn its owner took part in.
        // Nothing reads it: what hatches the egg is its own machine, whose
        // first move is the hatch, and this is the count the player is shown.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(HookEvent::AfterSideTurnEnd, vec![decrement()])
                    .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.HATCH_POWER", PowerType::Buff)
        },
        // Constrict squeezes its owner for its own count at the end of every
        // turn they took part in, and comes off when whoever worked it dies.
        PowerDefinition {
            removed_when_applier_dies: true,
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnEnd,
                    vec![Command::Creature(CreatureCmd::Damage {
                        target: CreatureSelector::Owner,
                        amount: stacks(),
                        props: ValueProps::UNPOWERED,
                    })],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.CONSTRICT_POWER", PowerType::Debuff)
        },
        // Inferno burns its owner a little each turn and burns the enemy side
        // whenever its owner is hurt on their own turn.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterPlayerTurnStart,
                    vec![Command::Creature(CreatureCmd::Damage {
                        target: CreatureSelector::Owner,
                        amount: Amount::counting(Count::ListeningPowerCounter),
                        props: ValueProps::UNBLOCKABLE,
                    })],
                ),
                ModifierHook::every_turn(
                    HookEvent::AfterDamageReceived,
                    vec![Command::Creature(CreatureCmd::Damage {
                        target: CreatureSelector::AllOpponents,
                        amount: stacks(),
                        props: ValueProps::UNPOWERED,
                    })],
                )
                // Only a blow that lands on the owner, on the owner's own
                // side's turn, and gets through the block counts, so a blow
                // that cost its owner nothing burns nobody.
                .when(HookCondition::OwnerTookUnblockedDamageOnOwnSideTurn),
            ],
            ..counter("POWER.INFERNO_POWER", PowerType::Buff)
        },
    ]
}

/// Radiance pays energy every time the pool refills, until it runs out.
fn radiance_power() -> PowerDefinition {
    PowerDefinition {
        hooks: vec![ModifierHook::every_turn(
            HookEvent::AfterEnergyReset,
            vec![
                Command::Player(PlayerCmd::GainEnergy(Amount::Fixed(1))),
                decrement(),
            ],
        )],
        ..counter("POWER.RADIANCE_POWER", PowerType::Buff)
    }
}

/// Tenderized: every card played costs its owner strength and dexterity for the
/// rest of the turn, and gives it all back once the turn is over. Whoever
/// tenderized them is who works it, both ways.
fn tender_power() -> PowerDefinition {
    let lent = |power: &str| apply_to_owner(power, CreatureSelector::Applier, Amount::Fixed(-1));
    let returned = |power: &str| {
        apply_to_owner(
            power,
            CreatureSelector::Applier,
            Amount::counting(Count::ListeningPowerCounter),
        )
    };
    PowerDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterCardPlayed,
                vec![
                    Command::Power(PowerCmd::ModifyCounter {
                        power: PowerRef::listening(),
                        amount: Amount::Fixed(1),
                    }),
                    lent("POWER.STRENGTH_POWER"),
                    lent("POWER.DEXTERITY_POWER"),
                ],
            ),
            ModifierHook::every_turn(
                HookEvent::AfterSideTurnEnd,
                vec![
                    returned("POWER.STRENGTH_POWER"),
                    returned("POWER.DEXTERITY_POWER"),
                    Command::Power(PowerCmd::SetCounter {
                        power: PowerRef::listening(),
                        amount: Amount::Fixed(0),
                    }),
                ],
            )
            .when(HookCondition::OwnerIsParticipant),
        ],
        ..counter("POWER.TENDER_POWER", PowerType::Debuff)
    }
}

/// Strength lent for a turn. The whole of what it does is being a temporary
/// power: the strength goes on as it lands and comes back off at the end of a
/// turn its owner took part in.
fn flex_potion_power() -> PowerDefinition {
    PowerDefinition {
        temporary: Some(TemporaryPower {
            internally_applied_power: id("POWER.STRENGTH_POWER"),
            positive: true,
        }),
        ..counter("POWER.FLEX_POTION_POWER", PowerType::Buff)
    }
}

/// How far a wither has grown. The queen counts it herself, and this engine
/// reads it off how many times she has stood on the move that grows it.
fn wither_growth() -> Amount {
    Amount::per(
        0,
        3,
        Count::TimesOwnerPerformedState("INCREASING_INTENSITY_MOVE".to_owned()),
    )
}

/// Powers that monsters carry.
#[allow(clippy::too_many_lines, reason = "one entry per power")]
fn monster_powers() -> Vec<PowerDefinition> {
    let mut powers = monster_powers_of_the_docks();
    powers.extend(monster_powers_of_the_rest());
    powers
}

/// The powers the Underdocks' own monsters carry.
fn monster_powers_of_the_docks() -> Vec<PowerDefinition> {
    vec![
        // A suck grows the fossil stalker by three strength for every hit of
        // its swing that got through, counted a hit at a time: a blow a pet
        // took and then trampled through onto its owner settles twice and is
        // still one hit. The build also asks that the swing was aimed at the
        // other side, which is the one thing every attack a monster makes
        // already is.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterAttack,
                    // The guard on a positive amount is not written out: an
                    // application of nothing does nothing at all, which is the
                    // same thing.
                    vec![Command::Power(PowerCmd::Apply {
                        marks: None,
                        power: id("POWER.STRENGTH_POWER"),
                        target: CreatureSelector::Owner,
                        applier: Some(CreatureSelector::Owner),
                        amount: Amount::Product {
                            left: Box::new(stacks()),
                            right: Box::new(Amount::counting(Count::HitsThatGotThrough)),
                        },
                    })],
                )
                .when(HookCondition::OwnerDealsPoweredDamage),
            ],
            ..counter("POWER.SUCK_POWER", PowerType::Buff)
        },
        // A hardened shell holds what a turn can cost its owner down to twenty
        // in total, and keeps what the turn has cost so far in a count of its
        // own. It answers the late half of the before-pet pass rather than the
        // damage cap, which is what the build's own placement says.
        PowerDefinition {
            hooks: vec![
                modifies(
                    HookEvent::ModifyHpLostBeforeOstyLate,
                    HookCondition::OwnerTakesDamage,
                    vec![Command::Modify(ModifierCmd::CapAt(Amount::Sum(vec![
                        stacks(),
                        Amount::negating(Count::ListeningPowerCounter),
                    ])))],
                ),
                ModifierHook::every_turn(
                    HookEvent::AfterDamageReceived,
                    vec![
                        Command::Power(PowerCmd::ModifyCounter {
                            power: PowerRef::listening(),
                            amount: Amount::counting(Count::UnblockedDamage),
                        }),
                        // Once a turn's damage received reaches the amount,
                        // the owner's HP reads as infinite for the rest of
                        // that turn: a turn that has already cost its owner
                        // everything the shell holds back leaves the owner
                        // reading as infinite.
                        Command::When {
                            condition: Condition::Not(Box::new(
                                Condition::ListeningPowerCounterBelowAmount,
                            )),
                            then: vec![Command::Creature(CreatureCmd::SetHpDisplay {
                                target: CreatureSelector::Owner,
                                display: sts2_engine::HpDisplay::InfiniteWithNumbers,
                            })],
                            otherwise: Vec::new(),
                        },
                    ],
                )
                .when(HookCondition::OwnerTookUnblockedDamage),
                ModifierHook::every_turn(
                    HookEvent::BeforeSideTurnStart,
                    vec![
                        Command::Power(PowerCmd::SetCounter {
                            power: PowerRef::listening(),
                            amount: Amount::Fixed(0),
                        }),
                        Command::Creature(CreatureCmd::SetHpDisplay {
                            target: CreatureSelector::Owner,
                            display: sts2_engine::HpDisplay::Normal,
                        }),
                    ],
                ),
            ],
            ..counter("POWER.HARDENED_SHELL_POWER", PowerType::Buff)
        },
        // A shriek is the eel's own threshold: the first blow that gets through
        // and leaves it at or below what the shriek is worth knocks it out of
        // the fight for a turn, and the shriek is spent for it. The move the
        // machine picks back up on is the eel's terror.
        PowerDefinition {
            allow_negative: true,
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterDamageReceived,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::OwnerHpAtMostListeningPowerAmount,
                        then: vec![
                            Command::Creature(CreatureCmd::Stun {
                                target: CreatureSelector::Owner,
                                next_move: sts2_engine::StunFollowUp::Named(
                                    "TERROR_MOVE".to_owned(),
                                ),
                                stun_move: Vec::new(),
                            }),
                            remove_self(),
                        ],
                    }],
                )
                .when(HookCondition::OwnerTookUnblockedDamage),
            ],
            ..counter("POWER.SHRIEK_POWER", PowerType::Debuff)
        },
    ]
}

#[allow(clippy::too_many_lines, reason = "one entry per power")]
fn monster_powers_of_the_rest() -> Vec<PowerDefinition> {
    let galvanize = || {
        Command::Card(CardCmd::Afflict {
            cards: CardSelector::AllCombatCards {
                filter: sts2_engine::CardFilter::OfType(sts2_engine::CardType::Power),
            },
            affliction: id("AFFLICTION.GALVANIZED"),
        })
    };
    let mut powers = vec![
        // The queen's withering presence: every sixth card the player it is
        // about plays puts a wither in their hand. The build counts down from
        // six to zero and the count is kept the other way round here, because
        // the projection carries the power's amount rather than the number the
        // build shows.
        //
        // A wither made here arrives grown to where the ones already in the
        // fight stand, which is what `AfterCardGeneratedForCombat` does for
        // every wither the queen's fight makes.
        PowerDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterCardPlayed,
                vec![
                    Command::Power(PowerCmd::ModifyCounter {
                        power: PowerRef::listening(),
                        amount: Amount::Fixed(1),
                    }),
                    Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningPowerCounterIs(6),
                        then: vec![
                            Command::CardPile(sts2_engine::CardPileCmd::AddGeneratedToCombat {
                                enchantment: None,
                                card: sts2_engine::CardOrigin::Model(id("CARD.WITHER")),
                                grown_by: wither_growth(),
                                count: Amount::Fixed(1),
                                pile: sts2_engine::PileName::Hand,
                                ethereal: false,
                                retain: false,
                                upgraded: false,
                                temporary_cost: None,
                                position: sts2_engine::DrawPilePosition::Bottom,
                            }),
                            Command::Power(PowerCmd::SetCounter {
                                power: PowerRef::listening(),
                                amount: Amount::Fixed(0),
                            }),
                        ],
                    },
                ],
            )],
            ..counter("POWER.WITHERING_PRESENCE_POWER", PowerType::Buff)
        },
        // The pressure a waterfall giant builds. It holds the fight open, keeps
        // its owner in the room after the death that empties it, and puts the
        // giant on the move it blows up from.
        PowerDefinition {
            owner_removed_from_combat_on_death: false,
            survives_owner_death: true,
            stops_combat_from_ending: true,
            move_on_death: Some("ABOUT_TO_BLOW_MOVE".to_owned()),
            must_perform_move_on_death: true,
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterDeath,
                    // Maximum and current HP both go to 999999999 and the
                    // health bar reads as endless: the giant stands there with
                    // nothing able to take it down until it has blown up, and
                    // reads that way.
                    vec![
                        Command::Creature(CreatureCmd::SetMaxAndCurrentHp {
                            target: CreatureSelector::Owner,
                            amount: sts2_engine::RolledAmount::Fixed(999_999_999),
                        }),
                        Command::Creature(CreatureCmd::SetHpDisplay {
                            target: CreatureSelector::Owner,
                            display: sts2_engine::HpDisplay::InfiniteWithoutNumbers,
                        }),
                    ],
                )
                .when(HookCondition::OwnerDied),
            ],
            ..counter("POWER.STEAM_ERUPTION_POWER", PowerType::Buff)
        },
        // Smog: once its owner has played a skill, every skill they hold is
        // smogged and unplayable until the turn ends. Which cards carry the
        // affliction is not projected, so what it comes to is one skill a turn.
        PowerDefinition {
            play_restriction: Some(sts2_engine::PlayRestriction::NoAfflictedCard(id(
                "AFFLICTION.SMOG",
            ))),
            hooks: vec![
                // Every skill of its owner's that can take an affliction is
                // smogged once one of them has been played, the card just
                // played included: it is in the play pile, and the sweep over
                // every card in the fight reaches it.
                ModifierHook::every_turn(
                    HookEvent::AfterCardPlayed,
                    vec![Command::Card(CardCmd::Afflict {
                        cards: CardSelector::AllCombatCards {
                            filter: sts2_engine::CardFilter::OfType(sts2_engine::CardType::Skill),
                        },
                        affliction: id("AFFLICTION.SMOG"),
                    })],
                )
                .when(HookCondition::All(vec![
                    HookCondition::OwnerPlayedTheCard,
                    HookCondition::PlayedCardIsOfType(sts2_engine::CardType::Skill),
                ])),
                // A skill that joins the fight after a skill play has begun is
                // smogged as it arrives.
                ModifierHook::every_turn(
                    HookEvent::AfterCardEnteredCombat,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::OwnerStartedAPlayOfTypeThisTurn(
                            sts2_engine::CardType::Skill,
                        ),
                        then: vec![Command::Card(CardCmd::Afflict {
                            cards: CardSelector::Announced,
                            affliction: id("AFFLICTION.SMOG"),
                        })],
                    }],
                )
                .when(HookCondition::EnteredCardIsOfType(
                    sts2_engine::CardType::Skill,
                )),
                // The smog is off every card of the fight at the end of the
                // turn its owner took part in.
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnEnd,
                    vec![Command::Card(CardCmd::ClearAffliction {
                        cards: CardSelector::AllCombatCards {
                            filter: sts2_engine::CardFilter::Any,
                        },
                        affliction: id("AFFLICTION.SMOG"),
                    })],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..single("POWER.SMOGGY_POWER", PowerType::Debuff)
        },
        // A gardener is skittish: the first blow of a turn that gets past its
        // block puts as much block back up, and the turn the player takes ends
        // with it stretched out again.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterAttack,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::All(vec![
                            Condition::Not(Box::new(Condition::ListeningPowerIsLatched)),
                            Condition::DamageCameFromACard,
                            Condition::OwnerWasHitByTheAttack,
                        ]),
                        then: vec![
                            Command::Power(PowerCmd::SetLatch(true)),
                            Command::Creature(CreatureCmd::GainBlock {
                                target: CreatureSelector::Owner,
                                amount: stacks(),
                                props: ValueProps::UNPOWERED,
                            }),
                        ],
                    }],
                ),
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnEnd,
                    vec![Command::Power(PowerCmd::SetLatch(false))],
                )
                .when(HookCondition::OwnerIsNotParticipant),
            ],
            ..counter("POWER.SKITTISH_POWER", PowerType::Buff)
        },
        // A corpse slug's hunger: when one of its own goes down, it stops to
        // devour, which costs it the turn and leaves it stronger.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterDeath,
                    vec![
                        Command::Creature(CreatureCmd::Stun {
                            target: CreatureSelector::Owner,
                            // The stun names the move that wakes it and
                            // nothing after it, so the slug picks its turn up
                            // from whatever it last performed.
                            next_move: StunFollowUp::TheLastMovePerformed,
                            stun_move: Vec::new(),
                        }),
                        apply_to_owner("POWER.STRENGTH_POWER", CreatureSelector::Owner, stacks()),
                    ],
                )
                .when(HookCondition::AllyDied),
            ],
            ..counter("POWER.RAVENOUS_POWER", PowerType::Buff)
        },
        // A globe head's charge: every power card of the fight is galvanized,
        // the ones it opens with and any that join it, and playing one costs
        // its player what the charge is worth.
        //
        // The build afflicts each card on its own as it comes in; afflicting
        // the whole set instead comes to the same thing, because a card already
        // carrying an affliction is passed over.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(HookEvent::BeforeCombatStart, vec![galvanize()]),
                ModifierHook::every_turn(HookEvent::AfterCardEnteredCombat, vec![galvanize()]),
                ModifierHook::every_turn(
                    HookEvent::AfterCardPlayed,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::PlayedCardIsAfflicted(id("AFFLICTION.GALVANIZED")),
                        then: vec![Command::Creature(CreatureCmd::Damage {
                            // The damage lands on whoever played the card,
                            // which in a single-player fight is the one
                            // player.
                            target: CreatureSelector::AllOpponents,
                            amount: stacks(),
                            props: ValueProps::UNPOWERED,
                        })],
                    }],
                ),
            ],
            ..counter("POWER.GALVANIC_POWER", PowerType::Buff)
        },
        // A personal hive drops a dazed into the draw pile of whoever swings a
        // powered attack at its owner, one for every stack it is holding, each
        // at a place of its own.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterDamageReceived,
                    vec![Command::CardPile(
                        sts2_engine::CardPileCmd::AddGeneratedToCombat {
                            enchantment: None,
                            card: sts2_engine::CardOrigin::Model(id("CARD.DAZED")),
                            grown_by: Amount::Fixed(0),
                            count: stacks(),
                            pile: sts2_engine::PileName::Draw,
                            ethereal: false,
                            retain: false,
                            upgraded: false,
                            temporary_cost: None,
                            position: sts2_engine::DrawPilePosition::Random,
                        },
                    )],
                )
                .when(HookCondition::OwnerTookPoweredAttackFromSomebody),
            ],
            ..counter("POWER.PERSONAL_HIVE_POWER", PowerType::Buff)
        },
        // An artifact takes a debuff meant for its owner, and is spent doing
        // it: it says the power lands for nothing, and the pass over whoever
        // answered comes back to take a stack off it.
        PowerDefinition {
            hooks: vec![
                modifies(
                    HookEvent::ModifyPowerAmountReceived,
                    HookCondition::PowerAppliedToOwner,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::AppliedPowerIsDebuff,
                        then: vec![Command::Modify(ModifierCmd::Set(Amount::Fixed(0)))],
                    }],
                ),
                ModifierHook::every_turn(
                    HookEvent::AfterModifyingPowerAmountReceived,
                    vec![decrement()],
                ),
            ],
            ..counter("POWER.ARTIFACT_POWER", PowerType::Buff)
        },
        // A shell nothing can crack in one go: no single hit on its owner is
        // worth more than it has stacks, however the hit was worked out.
        PowerDefinition {
            hooks: vec![modifies(
                HookEvent::ModifyDamageCap,
                HookCondition::OwnerTakesDamage,
                vec![Command::Modify(ModifierCmd::CapAt(stacks()))],
            )],
            ..counter("POWER.HARD_TO_KILL_POWER", PowerType::Buff)
        },
        // A thief's timer: it counts down to the turn it leaves, stopping one
        // short so that the power stays on to say so.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnEnd,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningPowerAmountAbove(1),
                        then: vec![decrement()],
                    }],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.ESCAPE_ARTIST_POWER", PowerType::Buff)
        },
        // Out of reach: while it holds, an attack on the owner lands for half,
        // and every blow that gets past block costs it a stack. The blow that
        // takes the last one knocks the owner out of the air: it is stunned,
        // and what it picks back up is not the move it was standing on but the
        // one its machine would have gone on to.
        //
        // The test for the last stack is made after the decrement has already
        // taken the power off its owner, where its amount reads nothing.
        PowerDefinition {
            hooks: vec![
                scales_damage_taken(50),
                ModifierHook::every_turn(
                    HookEvent::AfterDamageReceived,
                    vec![
                        decrement(),
                        Command::When {
                            otherwise: Vec::new(),
                            condition: Condition::Not(Box::new(
                                Condition::ListeningPowerAmountAbove(0),
                            )),
                            then: vec![Command::Creature(CreatureCmd::Stun {
                                target: CreatureSelector::Owner,
                                next_move: StunFollowUp::AfterTheLastMovePerformed,
                                stun_move: Vec::new(),
                            })],
                        },
                    ],
                )
                .when(HookCondition::OwnerTookUnblockedPoweredDamage),
            ],
            ..counter("POWER.FLUTTER_POWER", PowerType::Buff)
        },
        // A beetle's sleep. Every blow that gets past its shell costs it a
        // count, and so does the end of every turn it stood through. When the
        // last one goes it wakes: the shell comes off, and the machine picks
        // the roll up.
        //
        // The two ways out of it are not the same. A blow that empties it stuns
        // the sleeper, so the waking is what the stunned turn performs and the
        // roll comes the turn after; the turn's own end runs the waking where
        // it stands, and the roll is telegraphed straight away.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterDamageReceived,
                    vec![
                        decrement(),
                        Command::When {
                            otherwise: Vec::new(),
                            condition: Condition::Not(Box::new(
                                Condition::ListeningPowerAmountAbove(0),
                            )),
                            then: vec![Command::Creature(CreatureCmd::Stun {
                                target: CreatureSelector::Owner,
                                next_move: StunFollowUp::Named("ROLL_OUT_MOVE".to_owned()),
                                stun_move: wakes_up(),
                            })],
                        },
                    ],
                )
                .when(HookCondition::OwnerTookUnblockedDamage),
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnEnd,
                    vec![
                        decrement(),
                        Command::When {
                            otherwise: Vec::new(),
                            condition: Condition::Not(Box::new(
                                Condition::ListeningPowerAmountAbove(0),
                            )),
                            then: wakes_up(),
                        },
                    ],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.SLUMBER_POWER", PowerType::Buff)
        },
        // The matriarch's sleep, which is a different sleep from a beetle's.
        // It counts one down at the end of every turn its owner stood through,
        // and takes the plating off the turn before the last one runs out, so
        // the shell is already gone when the sleep ends of its own accord.
        //
        // A blow that gets past the shell ends it outright instead: the plating
        // comes off where the blow landed, the matriarch is stunned into the
        // slash, and the sleep is spent. Waking itself is sound, animation and
        // a flag, so the stunned turn performs nothing.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterDamageReceived,
                    vec![
                        Command::When {
                            otherwise: Vec::new(),
                            condition: Condition::OwnerHasPower(id("POWER.PLATING_POWER")),
                            then: vec![Command::Power(PowerCmd::Remove {
                                power: PowerRef::owned(id("POWER.PLATING_POWER")),
                            })],
                        },
                        Command::Creature(CreatureCmd::Stun {
                            target: CreatureSelector::Owner,
                            next_move: StunFollowUp::Named("SLASH_MOVE".to_owned()),
                            stun_move: Vec::new(),
                        }),
                        remove_self(),
                    ],
                )
                .when(HookCondition::OwnerTookUnblockedDamage),
                // `BeforeSideTurnEndVeryEarly`, so the shell is off before the
                // plating's own early pass would put its block up again.
                ModifierHook::every_turn(
                    HookEvent::BeforeSideTurnEndVeryEarly,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::All(vec![
                            Condition::Not(Box::new(Condition::ListeningPowerAmountAbove(1))),
                            Condition::OwnerHasPower(id("POWER.PLATING_POWER")),
                        ]),
                        then: vec![Command::Power(PowerCmd::Remove {
                            power: PowerRef::owned(id("POWER.PLATING_POWER")),
                        })],
                    }],
                )
                .when(HookCondition::OwnerIsParticipant),
                // The waking the last count runs is sound and a flag, so the
                // count going is the whole of it.
                ModifierHook::every_turn(HookEvent::AfterSideTurnEnd, vec![decrement()])
                    .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.ASLEEP_POWER", PowerType::Buff)
        },
        // The three turns a battle friend stands for. Every turn its side
        // finishes takes one off, and when the last one goes it walks out of
        // the fight — which the event standing the fight up reads back as the
        // player having run out of time.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnEnd,
                    vec![Command::When {
                        condition: Condition::ListeningPowerAmountAbove(1),
                        then: vec![decrement()],
                        otherwise: vec![Command::Creature(CreatureCmd::Escape {
                            target: CreatureSelector::Owner,
                        })],
                    }],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.BATTLEWORN_DUMMY_TIME_LIMIT_POWER", PowerType::Buff)
        },
        // What a thief took, until it is killed for it. Every theft is its own
        // instance, because each carries its own card.
        PowerDefinition {
            instance_type: PowerInstanceType::Instanced,
            ..single("POWER.SWIPE_POWER", PowerType::Buff)
        },
    ];
    powers.extend(scaling_monster_powers());
    powers
}

/// The monster powers that work a value at a turn boundary.
fn scaling_monster_powers() -> Vec<PowerDefinition> {
    vec![
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnEnd,
                    vec![apply_to_owner(
                        "POWER.STRENGTH_POWER",
                        CreatureSelector::Owner,
                        stacks(),
                    )],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.TERRITORIAL_POWER", PowerType::Buff)
        },
        wearing_off(PowerDefinition {
            hooks: vec![modifies(
                HookEvent::ModifyBlockMultiplicative,
                HookCondition::OwnerGainsPoweredBlock,
                vec![Command::Modify(ModifierCmd::Scale(Amount::Fixed(75)))],
            )],
            ..counter("POWER.FRAIL_POWER", PowerType::Debuff)
        }),
        PowerDefinition {
            hooks: vec![
                // An enemy that stands up with Plating stands up behind block
                // too: the round-one player turn start puts the block up before
                // the player has looked at it. The build counts rounds where
                // this counts the player's turns, which is the same number for
                // the one turn that matters.
                ModifierHook::every_turn(
                    HookEvent::BeforeSideTurnStart,
                    vec![Command::Creature(CreatureCmd::GainBlock {
                        target: CreatureSelector::Owner,
                        amount: stacks(),
                        props: ValueProps::UNPOWERED,
                    })],
                )
                .when(HookCondition::EnemyOwnerOnPlayerSideTurn)
                .timed(HookTiming::FirstTurnOnly),
                // Early, so the block is standing before end-of-turn damage.
                ModifierHook::every_turn(
                    HookEvent::BeforeSideTurnEndEarly,
                    vec![Command::Creature(CreatureCmd::GainBlock {
                        target: CreatureSelector::Owner,
                        amount: stacks(),
                        props: ValueProps::UNPOWERED,
                    })],
                )
                .when(HookCondition::OwnerIsParticipant),
                ModifierHook::every_turn(HookEvent::AfterSideTurnStart, vec![decrement()])
                    .when(HookCondition::OwnerIsParticipant)
                    .timed(HookTiming::AfterFirstTurn),
            ],
            ..counter("POWER.PLATING_POWER", PowerType::Buff)
        },
        tender_power(),
        // Any blow of its owner's that block stops outright leaves it off
        // balance. The build's body branches on who is carrying it: a rock bug
        // is marked off balance, and stuns itself at the end of the move it was
        // making; anything else is stunned where it stands.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterDamageGiven,
                    vec![
                        Command::When {
                            otherwise: Vec::new(),
                            condition: Condition::OwnerIsMonster(id("MONSTER.BOWLBUG_ROCK")),
                            then: vec![Command::Creature(CreatureCmd::SetOffBalance {
                                target: CreatureSelector::Owner,
                                value: true,
                            })],
                        },
                        Command::When {
                            otherwise: Vec::new(),
                            condition: Condition::Not(Box::new(Condition::OwnerIsMonster(id(
                                "MONSTER.BOWLBUG_ROCK",
                            )))),
                            then: vec![Command::Creature(CreatureCmd::Stun {
                                target: CreatureSelector::Owner,
                                // The stun names no next move, so the
                                // creature picks its turn up from whatever it
                                // last performed.
                                next_move: StunFollowUp::TheLastMovePerformed,
                                stun_move: Vec::new(),
                            })],
                        },
                    ],
                )
                .when(HookCondition::OwnerDealtFullyBlockedDamage),
            ],
            ..single("POWER.IMBALANCED_POWER", PowerType::Debuff)
        },
    ]
}

/// The powers the Kaiser Crab's two halves bring with them.
fn crab_powers() -> Vec<PowerDefinition> {
    // Which way the owner is facing, kept as the power's own variable: facing
    // right is nothing, facing left is one. Whoever carries the marker for the
    // other side is standing behind them.
    let struck_from_behind = |facing: i32, marker: &str| Command::When {
        otherwise: Vec::new(),
        condition: Condition::ListeningPowerCounterIs(facing),
        then: vec![Command::When {
            otherwise: Vec::new(),
            condition: Condition::DamageDealerHasPower(id(marker)),
            then: vec![Command::Modify(ModifierCmd::Scale(Amount::Fixed(150)))],
        }],
    };
    // Striking at whoever was behind you turns you to face them, which happens
    // before what was played lands and so never earns the blow itself.
    let turn_to_face = |facing: i32, marker: &str| Command::When {
        otherwise: Vec::new(),
        condition: Condition::ListeningPowerCounterIs(facing),
        then: vec![Command::When {
            otherwise: Vec::new(),
            condition: Condition::TargetHasPower(id(marker)),
            then: vec![Command::Power(PowerCmd::SetCounter {
                power: PowerRef::listening(),
                amount: Amount::Fixed(1 - facing),
            })],
        }],
    };
    // `AfterDeath`: when the enemies that are left all stand on one side, its
    // owner turns to face them.
    let turn_to_the_rest = |facing: i32, marker: &str| Command::When {
        otherwise: Vec::new(),
        condition: Condition::ListeningPowerCounterIs(facing),
        then: vec![Command::When {
            otherwise: Vec::new(),
            condition: Condition::AllOpponentsHavePower(id(marker)),
            then: vec![Command::Power(PowerCmd::SetCounter {
                power: PowerRef::listening(),
                amount: Amount::Fixed(1 - facing),
            })],
        }],
    };
    let facing = vec![
        turn_to_face(0, "POWER.BACK_ATTACK_LEFT_POWER"),
        turn_to_face(1, "POWER.BACK_ATTACK_RIGHT_POWER"),
    ];
    vec![
        // Surrounded: whoever the player is not facing strikes at their back
        // for half again as much, whether or not the blow is a powered one.
        PowerDefinition {
            hooks: vec![
                modifies(
                    HookEvent::ModifyDamageMultiplicative,
                    HookCondition::OwnerTakesDamage,
                    vec![
                        struck_from_behind(0, "POWER.BACK_ATTACK_LEFT_POWER"),
                        struck_from_behind(1, "POWER.BACK_ATTACK_RIGHT_POWER"),
                    ],
                ),
                modifies(
                    HookEvent::BeforeCardPlayed,
                    HookCondition::OwnerAimedAtSomebody,
                    facing.clone(),
                ),
                modifies(
                    HookEvent::BeforePotionUsed,
                    HookCondition::OwnerAimedAtSomebody,
                    facing,
                ),
                modifies(
                    HookEvent::AfterDeath,
                    HookCondition::OpponentDied,
                    vec![
                        turn_to_the_rest(0, "POWER.BACK_ATTACK_LEFT_POWER"),
                        turn_to_the_rest(1, "POWER.BACK_ATTACK_RIGHT_POWER"),
                    ],
                ),
            ],
            ..single("POWER.SURROUNDED_POWER", PowerType::Debuff)
        },
        // Markers for which side of the player a crab stands on. They do
        // nothing themselves; Surrounded reads them.
        single("POWER.BACK_ATTACK_LEFT_POWER", PowerType::Buff),
        single("POWER.BACK_ATTACK_RIGHT_POWER", PowerType::Buff),
        crab_rage_power(),
    ]
}

/// Every power the engine drives from data.
#[must_use]
/// The queen's chains: the first cards drawn each turn are bound, and only one
/// bound card can be played in a turn.
///
/// The build works the Bound affliction onto each card as it is drawn, up to
/// the power's amount, counting what it has bound this turn out of the combat
/// history; the count is kept on the power itself here. Which cards are bound
/// is not projected — what the affliction comes to is which plays are legal.
fn chains_of_binding_power() -> PowerDefinition {
    PowerDefinition {
        play_restriction: Some(sts2_engine::PlayRestriction::OneAfflictedCardEachTurn(id(
            "AFFLICTION.BOUND",
        ))),
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterCardDrawn,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::ListeningPowerCounterBelowAmount,
                    then: vec![
                        Command::Card(CardCmd::Afflict {
                            cards: CardSelector::Announced,
                            affliction: id("AFFLICTION.BOUND"),
                        }),
                        Command::Power(PowerCmd::ModifyCounter {
                            power: PowerRef::listening(),
                            amount: Amount::Fixed(1),
                        }),
                    ],
                }],
            )
            .when(HookCondition::OwnerSideIsActing),
            // The turn its owner took ends with the latch down, the count back
            // at nothing, and every binding cleared.
            ModifierHook::every_turn(
                HookEvent::BeforeSideTurnEnd,
                vec![
                    Command::Power(PowerCmd::SetLatch(false)),
                    Command::Power(PowerCmd::SetCounter {
                        power: PowerRef::listening(),
                        amount: Amount::Fixed(0),
                    }),
                    Command::Card(CardCmd::ClearAffliction {
                        cards: CardSelector::AllCombatCards {
                            filter: sts2_engine::CardFilter::Any,
                        },
                        affliction: id("AFFLICTION.BOUND"),
                    }),
                ],
            )
            .when(HookCondition::OwnerIsParticipant),
            // `BeforeCardPlayed` latches as soon as a bound card is played.
            ModifierHook::every_turn(
                HookEvent::BeforeCardPlayed,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::PlayedCardIsAfflicted(id("AFFLICTION.BOUND")),
                    then: vec![Command::Power(PowerCmd::SetLatch(true))],
                }],
            ),
        ],
        ..counter("POWER.CHAINS_OF_BINDING_POWER", PowerType::Debuff)
    }
}

/// A rampart puts its block up in front of the gunner as the player's turn
/// opens.
///
/// The build names the turret operator outright; the only encounter that
/// carries a rampart is the one with a living shield standing in front of one,
/// so the gunner is the other creature on its side.
fn rampart_power() -> PowerDefinition {
    PowerDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterSideTurnStart,
                vec![Command::Creature(CreatureCmd::GainBlock {
                    target: CreatureSelector::OtherAllies,
                    amount: stacks(),
                    props: ValueProps::UNPOWERED,
                })],
            )
            .when(HookCondition::PlayerSideTurn),
        ],
        ..counter("POWER.RAMPART_POWER", PowerType::Buff)
    }
}

/// Disintegration eats at whoever carries it at the end of every turn they
/// take part in.
fn disintegration_power() -> PowerDefinition {
    PowerDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterSideTurnEndLate,
                vec![Command::Creature(CreatureCmd::Damage {
                    target: CreatureSelector::Owner,
                    amount: stacks(),
                    props: ValueProps::UNPOWERED,
                })],
            )
            .when(HookCondition::OwnerIsParticipant),
        ],
        ..counter("POWER.DISINTEGRATION_POWER", PowerType::Debuff)
    }
}

/// Mind rot takes cards off the hand its owner draws.
fn mind_rot_power() -> PowerDefinition {
    PowerDefinition {
        hooks: vec![modifies(
            HookEvent::ModifyHandDraw,
            HookCondition::Always,
            vec![Command::Modify(ModifierCmd::Add(Amount::Scaled {
                base: 0,
                per: -1,
                count: Count::ListeningPowerAmount,
            }))],
        )],
        ..counter("POWER.MIND_ROT_POWER", PowerType::Debuff)
    }
}

/// Sloth holds its owner to as many cards a turn as it carries.
fn sloth_power() -> PowerDefinition {
    PowerDefinition {
        play_restriction: Some(sts2_engine::PlayRestriction::CardsEachTurn),
        ..counter("POWER.SLOTH_POWER", PowerType::Debuff)
    }
}

/// Wasting away takes energy off every turn its owner opens.
fn waste_away_power() -> PowerDefinition {
    PowerDefinition {
        hooks: vec![modifies(
            HookEvent::ModifyMaxEnergy,
            HookCondition::Always,
            vec![Command::Modify(ModifierCmd::Add(Amount::Scaled {
                base: 0,
                per: -1,
                count: Count::ListeningPowerAmount,
            }))],
        )],
        ..counter("POWER.WASTE_AWAY_POWER", PowerType::Debuff)
    }
}

/// Tainted: every powered blow that lands on its owner is heavier by as much,
/// and it goes as the enemy side's turn ends.
fn tainted_power() -> PowerDefinition {
    PowerDefinition {
        hooks: vec![
            modifies(
                HookEvent::ModifyDamageAdditive,
                HookCondition::OwnerTakesPoweredDamage,
                vec![Command::Modify(ModifierCmd::Add(stacks()))],
            ),
            ModifierHook::every_turn(HookEvent::AfterSideTurnEnd, vec![remove_self()])
                .when(HookCondition::EnemySideTurn),
        ],
        ..counter("POWER.TAINTED_POWER", PowerType::Debuff)
    }
}

/// A vital spark taints every skill its owner's opponent holds: playing one
/// leaves that player taking more from every blow until the round is out.
///
/// The build works the Tainted affliction onto every skill of the player's as
/// the fight opens, onto every skill that enters while it stands, and clears
/// them as it comes off — so while it stands there is no skill of theirs that is
/// not tainted, and what the affliction comes to is exactly this.
fn vital_spark_power() -> PowerDefinition {
    PowerDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterCardPlayed,
                vec![Command::Power(PowerCmd::Apply {
                    marks: None,
                    power: id("POWER.TAINTED_POWER"),
                    target: CreatureSelector::AllOpponents,
                    // Nobody is named as the applier.
                    applier: None,
                    amount: stacks(),
                })],
            )
            .when(HookCondition::PlayedCardIsOfType(
                sts2_engine::CardType::Skill,
            )),
        ],
        ..counter("POWER.VITAL_SPARK_POWER", PowerType::Buff)
    }
}

/// Curled up: the first card that gets through to its owner leaves it behind
/// its shell, and the shell is spent for it.
fn curl_up_power() -> PowerDefinition {
    PowerDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterDamageReceived,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::DamageCameFromACard,
                    then: vec![Command::Power(PowerCmd::SetLatch(true))],
                }],
            )
            .when(HookCondition::OwnerTakesPoweredDamage),
            ModifierHook::every_turn(
                HookEvent::AfterCardPlayed,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::ListeningPowerIsLatched,
                    then: vec![
                        Command::Creature(CreatureCmd::GainBlock {
                            target: CreatureSelector::Owner,
                            amount: stacks(),
                            props: ValueProps::UNPOWERED,
                        }),
                        remove_self(),
                    ],
                }],
            ),
        ],
        ..counter("POWER.CURL_UP_POWER", PowerType::Buff)
    }
}

/// Corruption: every skill is free and burns as it is played.
fn corruption_power() -> PowerDefinition {
    PowerDefinition {
        card_cost_rule: Some(sts2_engine::CardCostRule::FreeSkills),
        played_card_rule: Some(sts2_engine::PlayedCardRule::SkillsExhaust),
        ..single("POWER.CORRUPTION_POWER", PowerType::Buff)
    }
}

/// Tangled vines: every attack costs more until the turn is over.
fn tangled_power() -> PowerDefinition {
    PowerDefinition {
        card_cost_rule: Some(sts2_engine::CardCostRule::AttacksCostMore),
        hooks: vec![
            ModifierHook::every_turn(HookEvent::AfterSideTurnEnd, vec![remove_self()])
                .when(HookCondition::OwnerIsParticipant),
        ],
        ..counter("POWER.TANGLED_POWER", PowerType::Debuff)
    }
}

/// A free attack: the next attack out of the hand costs nothing, and playing
/// one spends the stack.
fn free_attack_power() -> PowerDefinition {
    PowerDefinition {
        card_cost_rule: Some(sts2_engine::CardCostRule::FreeAttacksInHandOrPlay),
        hooks: vec![
            ModifierHook::every_turn(HookEvent::BeforeCardPlayed, vec![decrement()]).when(
                HookCondition::PlayedCardIsOfType(sts2_engine::CardType::Attack),
            ),
        ],
        ..counter("POWER.FREE_ATTACK_POWER", PowerType::Buff)
    }
}

/// Regeneration heals its owner at the end of every turn it takes part in, and
/// counts itself down for it.
fn regen_power() -> PowerDefinition {
    PowerDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterSideTurnEnd,
                vec![
                    Command::Creature(CreatureCmd::Heal {
                        target: CreatureSelector::Owner,
                        amount: stacks(),
                    }),
                    decrement(),
                ],
            )
            .when(HookCondition::OwnerIsParticipant),
        ],
        ..counter("POWER.REGEN_POWER", PowerType::Buff)
    }
}

/// A speed potion lends dexterity for the turn, the way flex lends strength.
fn speed_potion_power() -> PowerDefinition {
    PowerDefinition {
        temporary: Some(TemporaryPower {
            internally_applied_power: id("POWER.DEXTERITY_POWER"),
            positive: true,
        }),
        ..counter("POWER.SPEED_POTION_POWER", PowerType::Buff)
    }
}

/// Soaring halves every powered blow that lands on the owl while it is up.
fn soar_power() -> PowerDefinition {
    PowerDefinition {
        hooks: vec![scales_damage_taken(50)],
        ..single("POWER.SOAR_POWER", PowerType::Buff)
    }
}

/// A hex makes every card its owner is holding burn up on the turn it is left
/// in hand, and it comes off with whoever cast it.
fn hex_power() -> PowerDefinition {
    PowerDefinition {
        makes_owners_cards_ethereal: true,
        removed_when_applier_dies: true,
        ..single("POWER.HEX_POWER", PowerType::Debuff)
    }
}

/// Dampen takes the upgrades off everything its owner brought to the fight and
/// gives them back when it goes.
///
/// The build keeps the set of creatures that cast it and only lets go when the
/// last of them is dead. A Magi Knight is the only thing that casts it, and the
/// only encounter with a Magi Knight has one, so the set has one member and its
/// applier is that member.
fn dampen_power() -> PowerDefinition {
    PowerDefinition {
        removed_when_applier_dies: true,
        on_removed: vec![Command::Power(PowerCmd::RestoreDowngradedCards)],
        hooks: vec![ModifierHook::every_turn(
            HookEvent::AfterApplied,
            vec![Command::Power(PowerCmd::DowngradeOwnersCards)],
        )],
        ..power(
            "POWER.DAMPEN_POWER",
            PowerType::Debuff,
            PowerStackType::None,
        )
    }
}

#[must_use]
pub fn ironclad_powers() -> Vec<PowerDefinition> {
    let mut powers = damage_shaping_powers();
    powers.extend(reacting_powers());
    powers.extend(monster_powers());
    powers.extend(crab_powers());
    powers.push(radiance_power());
    powers.push(slow_power());
    powers.push(plow_power());
    powers.push(ringing_power());
    powers.push(burrowed_power());
    powers.push(thorns_power());
    powers.push(ritual_power());
    powers.push(paper_cuts_power());
    powers.push(reattach_power());
    powers.push(crimson_mantle_power());
    powers.push(soar_power());
    powers.push(regen_power());
    powers.push(free_attack_power());
    powers.push(tangled_power());
    powers.push(corruption_power());
    powers.push(curl_up_power());
    powers.push(tainted_power());
    powers.push(vital_spark_power());
    powers.push(rampart_power());
    powers.push(disintegration_power());
    powers.push(mind_rot_power());
    powers.push(sloth_power());
    powers.push(waste_away_power());
    powers.push(chains_of_binding_power());
    powers.push(speed_potion_power());
    powers.push(hex_power());
    powers.push(dampen_power());
    powers.extend(ironclad_card_powers());
    powers
}

/// The powers the Ironclad's own cards lay down.
#[allow(clippy::too_many_lines, reason = "one entry per power")]
fn ironclad_card_powers() -> Vec<PowerDefinition> {
    vec![
        // Barricade stops its owner's block from clearing, and that is the
        // whole of it.
        PowerDefinition {
            owner_death_triggers_fatal: true,
            keeps_block: true,
            stops_draws_outside_hand_draw: false,
            keeps_hand: false,
            ..single("POWER.BARRICADE_POWER", PowerType::Buff)
        },
        // A colossus halves what a shaken attacker gets through, and one stack
        // of it is spent for every enemy turn that ends.
        PowerDefinition {
            hooks: vec![
                modifies(
                    HookEvent::ModifyDamageMultiplicative,
                    HookCondition::OwnerTakesPoweredDamage,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::DamageDealerHasPower(id("POWER.VULNERABLE_POWER")),
                        then: vec![Command::Modify(ModifierCmd::Scale(Amount::Fixed(50)))],
                    }],
                ),
                ModifierHook::every_turn(HookEvent::AfterSideTurnEnd, vec![decrement()])
                    .when(HookCondition::EnemySideTurn),
            ],
            ..counter("POWER.COLOSSUS_POWER", PowerType::Buff)
        },
        // A flame barrier burns whoever swings at its owner, and burns out when
        // the other side's turn ends.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterDamageReceived,
                    vec![Command::Creature(CreatureCmd::Damage {
                        target: CreatureSelector::DamageDealer,
                        amount: stacks(),
                        props: ValueProps::UNPOWERED,
                    })],
                )
                .when(HookCondition::OwnerTakesPoweredDamage),
                ModifierHook::every_turn(HookEvent::AfterSideTurnEnd, vec![remove_self()])
                    .when(HookCondition::OwnerIsNotParticipant),
            ],
            ..counter("POWER.FLAME_BARRIER_POWER", PowerType::Buff)
        },
        // A demon form hands its owner strength as their own turn opens.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    vec![apply_to_owner(
                        "POWER.STRENGTH_POWER",
                        CreatureSelector::Owner,
                        stacks(),
                    )],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.DEMON_FORM_POWER", PowerType::Buff)
        },
        // A pyre raises how much energy its owner's turn opens with.
        PowerDefinition {
            hooks: vec![modifies(
                HookEvent::ModifyMaxEnergy,
                HookCondition::Always,
                vec![Command::Modify(ModifierCmd::Add(stacks()))],
            )],
            ..counter("POWER.PYRE_POWER", PowerType::Buff)
        },
        // Cruelty carries nothing of its own: Vulnerable reads it off whoever
        // is swinging and pays out the heavier multiplier itself.
        counter("POWER.CRUELTY_POWER", PowerType::Buff),
        // The two temporary strengths the Ironclad's cards hand out: one up for
        // a Setup Strike, one down for a Mangle, both taken back at the end of
        // a turn their owner took part in.
        PowerDefinition {
            temporary: Some(TemporaryPower {
                internally_applied_power: id("POWER.STRENGTH_POWER"),
                positive: true,
            }),
            ..counter("POWER.SETUP_STRIKE_POWER", PowerType::Buff)
        },
        PowerDefinition {
            temporary: Some(TemporaryPower {
                internally_applied_power: id("POWER.STRENGTH_POWER"),
                positive: false,
            }),
            ..counter("POWER.MANGLE_POWER", PowerType::Debuff)
        },
        // Feeling no pain is block for every card of its owner's that burns,
        // and nothing scales it.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardExhausted,
                    vec![Command::Creature(CreatureCmd::GainBlock {
                        target: CreatureSelector::Owner,
                        amount: stacks(),
                        props: ValueProps::UNPOWERED,
                    })],
                )
                .when(HookCondition::OwnersCardExhausted),
            ],
            ..counter("POWER.FEEL_NO_PAIN_POWER", PowerType::Buff)
        },
        dark_embrace_power(),
        // Expecting a fight pays energy up front and takes the next gain away:
        // the power says a gain is worth nothing at all, and comes off at the
        // end of the turn its owner took part in.
        PowerDefinition {
            hooks: vec![
                modifies(
                    HookEvent::ModifyEnergyGain,
                    HookCondition::Always,
                    vec![Command::Modify(ModifierCmd::Set(Amount::Fixed(0)))],
                ),
                ModifierHook::every_turn(HookEvent::AfterSideTurnEnd, vec![remove_self()])
                    .when(HookCondition::OwnerIsParticipant),
            ],
            ..single("POWER.NO_ENERGY_GAIN_POWER", PowerType::Debuff)
        },
        // A battle trance draws deep and then stops the well: nothing its owner
        // does draws another card this turn, though the next turn's own opening
        // hand still comes.
        PowerDefinition {
            stops_draws_outside_hand_draw: true,
            hooks: vec![
                ModifierHook::every_turn(HookEvent::AfterSideTurnEnd, vec![remove_self()])
                    .when(HookCondition::OwnerIsParticipant),
            ],
            ..single("POWER.NO_DRAW_POWER", PowerType::Debuff)
        },
        // A juggernaut answers every block its owner works up by throwing what
        // it is worth at somebody, rolled on the combat-targets stream.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterBlockGained,
                    vec![Command::Creature(CreatureCmd::Damage {
                        target: CreatureSelector::RandomOpponent,
                        amount: stacks(),
                        props: ValueProps::UNPOWERED,
                    })],
                )
                .when(HookCondition::OwnerGainedBlock),
            ],
            ..counter("POWER.JUGGERNAUT_POWER", PowerType::Buff)
        },
        // Viciousness draws for every shaking its owner works onto anybody.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterPowerAmountChanged,
                    vec![Command::CardPile(CardPileCmd::Draw { count: stacks() })],
                )
                .when(HookCondition::OwnerWorkedPower(id(
                    "POWER.VULNERABLE_POWER",
                ))),
            ],
            ..counter("POWER.VICIOUS_POWER", PowerType::Buff)
        },
        // A one-two punch makes its owner's next attack play run twice, and one
        // stack is spent for each attack that does. What is left of it goes at
        // the end of a turn its owner took part in.
        PowerDefinition {
            hooks: vec![
                modifies(
                    HookEvent::ModifyCardPlayCount,
                    HookCondition::PlayedCardIsAttack,
                    vec![Command::Modify(ModifierCmd::Add(Amount::Fixed(1)))],
                ),
                ModifierHook::every_turn(HookEvent::AfterModifyingCardPlayCount, vec![decrement()]),
                ModifierHook::every_turn(HookEvent::AfterSideTurnEnd, vec![remove_self()])
                    .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.ONE_TWO_PUNCH_POWER", PowerType::Buff)
        },
        juggling_power(),
        stampede_power(),
        hellraiser_power(),
        aggression_power(),
        rupture_power(),
        unmovable_power(),
    ]
}

/// A rupture pays strength for HP its owner loses on their own turn. What a
/// card costs them is paid once that card's play is over, rather than in the
/// middle of it, so a card that bleeds its owner and then swings does not swing
/// harder for it.
///
/// The tally is one entry per in-flight play of its owner's, kept against the
/// card played: a card played inside another card's play runs up its own entry
/// and pays it out when it finishes, leaving
/// the outer play's untouched. A blow that came from no card of the owner's —
/// or from a play the guard refused — pays its strength on the spot instead.
fn rupture_power() -> PowerDefinition {
    PowerDefinition {
        hooks: vec![
            // The play is tallied, guarded on the card being the owner's and
            // on the owner's own side being the one acting.
            ModifierHook::every_turn(
                HookEvent::BeforeCardPlayed,
                vec![Command::When {
                    // The owner here is the player, so their own side acting is
                    // the enemy side not acting.
                    otherwise: vec![Command::Power(PowerCmd::OpenPlayTally { open: true })],
                    condition: Condition::EnemySideIsActing,
                    then: vec![Command::Power(PowerCmd::OpenPlayTally { open: false })],
                }],
            )
            .when(HookCondition::OwnerPlayedTheCard),
            ModifierHook::every_turn(
                HookEvent::AfterDamageReceived,
                vec![
                    Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::All(vec![
                            Condition::ListeningPowerPlayTallyIsOpen,
                            Condition::DamageCameFromACard,
                        ]),
                        then: vec![Command::Power(PowerCmd::AddToPlayTally(stacks()))],
                    },
                    Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::Not(Box::new(Condition::All(vec![
                            Condition::ListeningPowerPlayTallyIsOpen,
                            Condition::DamageCameFromACard,
                        ]))),
                        then: vec![apply_to_owner(
                            "POWER.STRENGTH_POWER",
                            CreatureSelector::Owner,
                            stacks(),
                        )],
                    },
                ],
            )
            .when(HookCondition::OwnerTookUnblockedDamageOnOwnSideTurn),
            // The tally is closed and the strength it owed paid. The entry is
            // taken off whether or not it was opened, because the build only
            // ever tallied the plays that opened one.
            ModifierHook::every_turn(
                HookEvent::AfterCardPlayed,
                vec![
                    Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningPowerPlayTallyIsOpen,
                        then: vec![apply_to_owner(
                            "POWER.STRENGTH_POWER",
                            CreatureSelector::Owner,
                            Amount::counting(Count::ListeningPowerPlayTally),
                        )],
                    },
                    Command::Power(PowerCmd::ClosePlayTally),
                ],
            )
            .when(HookCondition::OwnerPlayedTheCard),
        ],
        ..counter("POWER.RUPTURE_POWER", PowerType::Buff)
    }
}

/// Unmovable doubles the block its owner works up off a card or a move, for as
/// many separate gains a turn as it is worth. The gains the play now being
/// answered has already made do not count against it.
fn unmovable_power() -> PowerDefinition {
    PowerDefinition {
        hooks: vec![modifies(
            HookEvent::ModifyBlockMultiplicative,
            HookCondition::OwnerGrantsPoweredBlock,
            vec![Command::When {
                otherwise: Vec::new(),
                condition: Condition::BlockGainsFromOtherPlaysBelowListeningPowerAmount,
                then: vec![Command::Modify(ModifierCmd::Scale(Amount::Fixed(200)))],
            }],
        )],
        ..counter("POWER.UNMOVABLE_POWER", PowerType::Buff)
    }
}

/// Juggling counts the attacks its owner has played this turn — starting from
/// what the history already holds when it lands — and hands back copies of the
/// third one.
fn juggling_power() -> PowerDefinition {
    PowerDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterApplied,
                vec![Command::Power(PowerCmd::SetCounter {
                    power: PowerRef::listening(),
                    amount: Amount::counting(Count::OwnerAttacksPlayedThisTurn),
                })],
            ),
            ModifierHook::every_turn(
                HookEvent::AfterCardPlayed,
                vec![
                    Command::Power(PowerCmd::ModifyCounter {
                        power: PowerRef::listening(),
                        amount: Amount::Fixed(1),
                    }),
                    Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningPowerCounterIs(3),
                        then: vec![Command::CardPile(CardPileCmd::AddGeneratedToCombat {
                            enchantment: None,
                            card: CardOrigin::CloneOfPlayedCard,
                            grown_by: Amount::Fixed(0),
                            count: stacks(),
                            pile: sts2_engine::PileName::Hand,
                            ethereal: false,
                            retain: false,
                            upgraded: false,
                            temporary_cost: None,
                            position: sts2_engine::DrawPilePosition::Bottom,
                        })],
                    },
                ],
            )
            .when(HookCondition::PlayedCardIsAttack),
            ModifierHook::every_turn(
                HookEvent::AfterSideTurnEnd,
                vec![Command::Power(PowerCmd::SetCounter {
                    power: PowerRef::listening(),
                    amount: Amount::Fixed(0),
                })],
            )
            .when(HookCondition::OwnerIsParticipant),
        ],
        ..counter("POWER.JUGGLING_POWER", PowerType::Buff)
    }
}

/// A stampede plays that many attacks out of its owner's hand once they have
/// stopped acting, each one rolled off the shuffle stream.
fn stampede_power() -> PowerDefinition {
    PowerDefinition {
        hooks: vec![ModifierHook::every_turn(
            HookEvent::AfterAutoPostPlayPhaseEntered,
            vec![Command::Repeat {
                times: stacks(),
                then: vec![Command::Select {
                    cards: CardSelector::ShuffleRolledFromPile {
                        pile: sts2_engine::PileName::Hand,
                        filter: sts2_engine::CardFilter::PlayableOfType(
                            sts2_engine::CardType::Attack,
                        ),
                    },
                    then: vec![Command::Card(CardCmd::AutoPlay {
                        at_the_bodys_target: false,
                        cards: CardSelector::Chosen,
                    })],
                }],
            }],
        )],
        ..counter("POWER.STAMPEDE_POWER", PowerType::Buff)
    }
}

/// A hellraiser plays every Strike its owner draws, the moment it is drawn.
///
/// It plays nine of them a turn at the most while nothing its owner can hit
/// has HP that runs out. A hardened shell and a waterfall giant standing up to
/// blow are what put a field in that state here, and the cap is what keeps the
/// draw the play answers from feeding itself forever where no blow can end the
/// fight. A field with anything killable left in it clears the count instead,
/// so the cap costs an ordinary fight nothing.
///
/// The count belongs to the power, kept as its own counter, and the end of a
/// turn its owner took part in clears it.
fn hellraiser_power() -> PowerDefinition {
    // The most auto-plays one turn may take while no blow can end the fight.
    const INFINITE_AUTO_PLAY_CAP: i32 = 9;
    let count_it = || {
        Command::Power(PowerCmd::ModifyCounter {
            power: PowerRef::listening(),
            amount: Amount::Fixed(1),
        })
    };
    let play = || {
        Command::Card(CardCmd::AutoPlay {
            at_the_bodys_target: false,
            cards: CardSelector::Announced,
        })
    };
    PowerDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterCardDrawnEarly,
                vec![Command::When {
                    condition: Condition::EveryOpponentsHpDisplayIsInfinite,
                    // The count runs whether or not the play does, and the
                    // cap is read off what it stood at before this draw.
                    then: vec![Command::When {
                        condition: Condition::ListeningPowerCounterAtLeast(INFINITE_AUTO_PLAY_CAP),
                        then: vec![count_it()],
                        otherwise: vec![count_it(), play()],
                    }],
                    // A killable field clears the count outright.
                    otherwise: vec![
                        Command::Power(PowerCmd::SetCounter {
                            power: PowerRef::listening(),
                            amount: Amount::Fixed(0),
                        }),
                        play(),
                    ],
                }],
            )
            .when(HookCondition::DrawnCardIsTagged(
                sts2_engine::CardTag::Strike,
            )),
            ModifierHook::every_turn(
                HookEvent::AfterSideTurnEnd,
                vec![Command::Power(PowerCmd::SetCounter {
                    power: PowerRef::listening(),
                    amount: Amount::Fixed(0),
                })],
            )
            .when(HookCondition::OwnerIsParticipant),
        ],
        ..single("POWER.HELLRAISER_POWER", PowerType::Buff)
    }
}

/// Aggression takes attacks back out of the discard pile as its owner's turn
/// opens, shuffled, and upgrades each of them.
fn aggression_power() -> PowerDefinition {
    PowerDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::BeforeSideTurnStart,
                vec![Command::ForEachCard {
                    cards: CardSelector::ShuffledFromPile {
                        pile: sts2_engine::PileName::Discard,
                        filter: sts2_engine::CardFilter::OfType(sts2_engine::CardType::Attack),
                        take: stacks(),
                    },
                    then: vec![
                        Command::CardPile(CardPileCmd::MoveToPile {
                            cards: CardSelector::Chosen,
                            pile: sts2_engine::PileName::Hand,
                            position: sts2_engine::DrawPilePosition::Bottom,
                        }),
                        Command::Card(CardCmd::Upgrade {
                            cards: CardSelector::Chosen,
                        }),
                    ],
                }],
            )
            .when(HookCondition::OwnerIsParticipant),
        ],
        ..counter("POWER.AGGRESSION_POWER", PowerType::Buff)
    }
}

/// A dark embrace draws for every card of its owner's that burns — except for
/// the ones that burn up in hand as the turn ends, which it counts and pays for
/// once the hand has been put down.
///
/// The build says why in its own comment: STS1 could put those draws on a
/// queue, and this build has to defer them by hand. The count it defers on is
/// the power's own internal data, which is the instance's counter here.
fn dark_embrace_power() -> PowerDefinition {
    PowerDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::AfterCardExhausted,
                vec![
                    Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ExhaustCausedByEthereal,
                        then: vec![Command::Power(PowerCmd::ModifyCounter {
                            power: PowerRef::listening(),
                            amount: Amount::Fixed(1),
                        })],
                    },
                    Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::Not(Box::new(Condition::ExhaustCausedByEthereal)),
                        then: vec![Command::CardPile(CardPileCmd::Draw { count: stacks() })],
                    },
                ],
            )
            .when(HookCondition::OwnersCardExhausted),
            ModifierHook::every_turn(
                HookEvent::AfterSideTurnEnd,
                vec![
                    Command::CardPile(CardPileCmd::Draw {
                        count: Amount::Product {
                            left: Box::new(stacks()),
                            right: Box::new(Amount::counting(Count::ListeningPowerCounter)),
                        },
                    }),
                    Command::Power(PowerCmd::SetCounter {
                        power: PowerRef::listening(),
                        amount: Amount::Fixed(0),
                    }),
                ],
            )
            .when(HookCondition::OwnerIsParticipant),
        ],
        ..counter("POWER.DARK_EMBRACE_POWER", PowerType::Buff)
    }
}

/// The powers the cards that belong to no character apply.
///
/// They land from a colorless card, an event card, or an ancient one, so they
/// belong to no pool either.
#[allow(clippy::too_many_lines, reason = "one entry per power")]
pub fn shared_powers() -> Vec<PowerDefinition> {
    vec![
        // A stratagem hands as many cards as it has stacks back out of the
        // draw pile every time that pile is shuffled.
        PowerDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterShuffle,
                vec![Command::CardSelect(CardSelectCmd::FromCombatPile {
                    semantics: ChoiceSemantics::known(vec![
                        ChoiceOp::new(ChoiceOperationKind::Move, ChoiceScope::Combat)
                            .to(ChoiceDestination::Hand),
                    ]),
                    pile: PileName::Draw,
                    prefs: SelectorPrefs::exactly(0),
                    maximum: None,
                    exact_count: Some(Amount::counting(Count::ListeningPowerAmount)),
                    restricted_to_chosen: false,
                    filter: CardFilter::Any,
                    then: vec![Command::CardPile(CardPileCmd::MoveToPile {
                        cards: CardSelector::Chosen,
                        pile: PileName::Hand,
                        position: DrawPilePosition::Bottom,
                    })],
                })],
            )],
            ..counter("POWER.STRATAGEM_POWER", PowerType::Buff)
        },
        // Intangible holds every blow that reaches its owner down to one, and
        // is spent for the enemy turn that ends.
        wearing_off(PowerDefinition {
            hooks: vec![
                // The HP-loss pass is what actually holds the blow down; the
                // cap pass duplicates it so that block loss and an advertised
                // intent read the same number.
                modifies(
                    HookEvent::ModifyHpLostAfterOsty,
                    HookCondition::OwnerTakesDamage,
                    vec![Command::Modify(ModifierCmd::CapAt(Amount::Fixed(1)))],
                ),
                modifies(
                    HookEvent::ModifyDamageCap,
                    HookCondition::OwnerTakesDamage,
                    vec![Command::Modify(ModifierCmd::CapAt(Amount::Fixed(1)))],
                ),
            ],
            ..counter("POWER.INTANGIBLE_POWER", PowerType::Buff)
        }),
        // A feeding frenzy lends strength for the turn, the way flex does.
        PowerDefinition {
            temporary: Some(TemporaryPower {
                internally_applied_power: id("POWER.STRENGTH_POWER"),
                positive: true,
            }),
            ..counter("POWER.FEEDING_FRENZY_POWER", PowerType::Buff)
        },
        // Energy owed for the next turn: it is paid the moment the pool refills
        // and comes off for having paid.
        PowerDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterEnergyReset,
                vec![
                    Command::Player(PlayerCmd::GainEnergy(stacks())),
                    remove_self(),
                ],
            )],
            ..counter("POWER.ENERGY_NEXT_TURN_POWER", PowerType::Buff)
        },
        // Cards owed for the next turn: the hand draw is that much bigger, and
        // the power comes off once the turn it paid for has opened.
        //
        // Both bodies are guarded on what the power stood at when the turn
        // opened, taken before anything of the turn runs. A stack applied after
        // that point pays nothing and is not taken off, so it stands to the
        // next turn; what the draw is raised by is the *live* amount, which is
        // asked separately from the gate.
        PowerDefinition {
            hooks: vec![
                modifies(
                    HookEvent::ModifyHandDraw,
                    HookCondition::ListeningPowerStoodAtTurnStart,
                    vec![Command::Modify(ModifierCmd::Add(stacks()))],
                ),
                ModifierHook::every_turn(HookEvent::AfterSideTurnStart, vec![remove_self()]).when(
                    HookCondition::All(vec![
                        HookCondition::OwnerIsParticipant,
                        HookCondition::ListeningPowerStoodAtTurnStart,
                    ]),
                ),
            ],
            ..counter("POWER.DRAW_CARDS_NEXT_TURN_POWER", PowerType::Buff)
        },
        // A rebound sends the next card its owner plays back to the top of the
        // draw pile rather than to the discard, and is spent for the card it
        // sent. What is left of it goes when the turn does.
        PowerDefinition {
            played_card_rule: Some(sts2_engine::PlayedCardRule::DiscardsToDrawPileTop),
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterModifyingCardPlayResultPile,
                    vec![decrement()],
                ),
                ModifierHook::every_turn(HookEvent::AfterSideTurnEnd, vec![remove_self()])
                    .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.REBOUND_POWER", PowerType::Buff)
        },
        // Toric toughness works its block up again every turn its owner's block
        // is cleared, for as many turns as it carries. What it is worth is the
        // block the card that applied it actually gained, which it keeps in its
        // own counter because the amount is the turns.
        PowerDefinition {
            instance_type: PowerInstanceType::Instanced,
            // A block clear is announced about one creature at a time and
            // only that creature's own listeners run, which is the owner test
            // the build's own body opens with.
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterBlockCleared,
                vec![
                    Command::Creature(CreatureCmd::GainBlock {
                        target: CreatureSelector::Owner,
                        amount: Amount::counting(Count::ListeningPowerCounter),
                        props: ValueProps::UNPOWERED,
                    }),
                    decrement(),
                ],
            )],
            ..counter("POWER.TORIC_TOUGHNESS_POWER", PowerType::Buff)
        },
        // Hello world stocks the hand with as many common cards of its owner's
        // own pool as it carries, before the turn's own draw.
        PowerDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::BeforeHandDraw,
                vec![Command::CardPile(CardPileCmd::AddGeneratedFromPool {
                    pool: sts2_engine::CardPoolRef::OwnCharacter,
                    count: stacks(),
                    distinct: true,
                    filter: sts2_engine::CardFilter::OfRarity(sts2_engine::CardRarity::Common),
                    pile: sts2_engine::PileName::Hand,
                    position: sts2_engine::DrawPilePosition::Bottom,
                    excludes_source: false,
                    upgraded: false,
                    temporary_cost: None,
                    ethereal: false,
                    rolled_one_at_a_time: false,
                })],
            )],
            ..counter("POWER.HELLO_WORLD_POWER", PowerType::Buff)
        },
    ]
}

/// The powers a potion or a relic works on, and nothing else does.
#[must_use]
#[allow(clippy::too_many_lines, reason = "one entry per power")]
pub fn item_powers() -> Vec<PowerDefinition> {
    vec![
        // A snecko eye's confusion: every card its owner draws is rolled a new
        // cost for the rest of the fight, and a card that cannot be played at
        // all is passed over without a roll.
        PowerDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterCardDrawn,
                vec![Command::Card(sts2_engine::CardCmd::RandomizeCost {
                    cards: CardSelector::Announced,
                    for_the_combat: true,
                })],
            )],
            ..single("POWER.CONFUSED_POWER", PowerType::Debuff)
        },
        // A diamond diadem halves what reaches its owner, and comes off at the
        // end of the enemy's turn.
        PowerDefinition {
            hooks: vec![
                modifies(
                    HookEvent::ModifyDamageMultiplicative,
                    HookCondition::OwnerTakesPoweredDamage,
                    vec![Command::Modify(ModifierCmd::Scale(Amount::Fixed(50)))],
                ),
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnEnd,
                    vec![Command::Power(PowerCmd::Remove {
                        power: PowerRef::listening(),
                    })],
                )
                .when(HookCondition::EnemySideTurn),
            ],
            ..single("POWER.DIAMOND_DIADEM_POWER", PowerType::Buff)
        },
        // Clarity is a bigger hand for each of the next few turns, one stack
        // spent per turn it paid for.
        PowerDefinition {
            hooks: vec![
                modifies(
                    HookEvent::ModifyHandDraw,
                    HookCondition::Always,
                    vec![Command::Modify(ModifierCmd::Add(Amount::Fixed(1)))],
                ),
                ModifierHook::every_turn(HookEvent::AfterSideTurnStart, vec![decrement()])
                    .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.CLARITY_POWER", PowerType::Buff)
        },
        // Duplication plays its owner's next card twice, spends a stack for the
        // play it doubled, and whatever is left goes at the end of the turn.
        PowerDefinition {
            hooks: vec![
                modifies(
                    HookEvent::ModifyCardPlayCount,
                    HookCondition::Always,
                    vec![Command::Modify(ModifierCmd::Add(Amount::Fixed(1)))],
                ),
                ModifierHook::every_turn(HookEvent::AfterModifyingCardPlayCount, vec![decrement()]),
                ModifierHook::every_turn(HookEvent::AfterSideTurnEnd, vec![remove_self()])
                    .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.DUPLICATION_POWER", PowerType::Buff)
        },
        gigantification_power(),
        // A reptile trinket's strength is the temporary-strength wrapper the
        // right way up: strength for the turn and back again after.
        PowerDefinition {
            temporary: Some(TemporaryPower {
                internally_applied_power: id("POWER.STRENGTH_POWER"),
                positive: true,
            }),
            ..counter("POWER.REPTILE_TRINKET_POWER", PowerType::Buff)
        },
        // Self-forming clay pays its block as its owner's block is cleared, and
        // is spent whole for it.
        PowerDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterBlockCleared,
                vec![
                    Command::Creature(CreatureCmd::GainBlock {
                        target: CreatureSelector::Owner,
                        amount: stacks(),
                        props: ValueProps::UNPOWERED,
                    }),
                    remove_self(),
                ],
            )],
            ..counter("POWER.SELF_FORMING_CLAY_POWER", PowerType::Buff)
        },
        // A buffer takes one blow whole: the HP it would have cost becomes
        // nothing, and the pass over whoever said so spends the stack.
        PowerDefinition {
            hooks: vec![
                modifies(
                    HookEvent::ModifyHpLostAfterOstyLate,
                    HookCondition::OwnerTakesDamage,
                    vec![Command::Modify(ModifierCmd::Set(Amount::Fixed(0)))],
                ),
                ModifierHook::every_turn(
                    HookEvent::AfterModifyingHpLostAfterOsty,
                    vec![decrement()],
                ),
            ],
            ..counter("POWER.BUFFER_POWER", PowerType::Buff)
        },
        // Demise costs its owner its whole amount as their side's turn ends,
        // and neither block nor anything that scales damage touches it.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnEnd,
                    vec![Command::Creature(CreatureCmd::Damage {
                        target: CreatureSelector::Owner,
                        amount: stacks(),
                        props: ValueProps::UNBLOCKABLE,
                    })],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.DEMISE_POWER", PowerType::Debuff)
        },
        // A shackling potion takes strength for the fight and gives it back
        // when the power goes.
        PowerDefinition {
            temporary: Some(TemporaryPower {
                internally_applied_power: id("POWER.STRENGTH_POWER"),
                positive: false,
            }),
            ..counter("POWER.SHACKLING_POTION_POWER", PowerType::Debuff)
        },
    ]
}

/// Gigantification trebles one attack of its owner's and is spent for it.
///
/// The build latches onto the first powered attack an attack card opens,
/// trebles the damage of the card that swing came from, and lets go when that
/// same swing closes. The latch here is that binding: it differs from the
/// build only if some other card of the owner's
/// deals powered damage between the swing opening and closing, which nothing at
/// this baseline does.
fn gigantification_power() -> PowerDefinition {
    PowerDefinition {
        hooks: vec![
            ModifierHook::every_turn(
                HookEvent::BeforeAttack,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::All(vec![
                        Condition::DamageCardIsOfType(sts2_engine::CardType::Attack),
                        Condition::Not(Box::new(Condition::ListeningPowerIsLatched)),
                    ]),
                    then: vec![Command::Power(PowerCmd::SetLatch(true))],
                }],
            )
            .when(HookCondition::OwnerDealsPoweredDamage),
            modifies(
                HookEvent::ModifyDamageMultiplicative,
                HookCondition::OwnerDealsPoweredDamage,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::DamageCameFromACard,
                    then: vec![Command::Modify(ModifierCmd::Scale(Amount::Fixed(300)))],
                }],
            ),
            ModifierHook::every_turn(
                HookEvent::AfterAttack,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::ListeningPowerIsLatched,
                    then: vec![Command::Power(PowerCmd::SetLatch(false)), decrement()],
                }],
            ),
        ],
        ..counter("POWER.GIGANTIFICATION_POWER", PowerType::Buff)
    }
}

/// The powers the colorless pool's cards apply.
#[allow(clippy::too_many_lines, reason = "one entry per power")]
pub fn colorless_powers() -> Vec<PowerDefinition> {
    vec![
        // Curious: a power of its owner's costs its own amount less, never
        // below nothing, and only where it costs something to begin with.
        PowerDefinition {
            card_cost_rule: Some(sts2_engine::CardCostRule::PowersCostLess),
            ..counter("POWER.CURIOUS_POWER", PowerType::Buff)
        },
        // Improvement upgrades that many cards of the deck once the fight is
        // over, each rolled off what the one before it left.
        PowerDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterCombatEnd,
                vec![Command::Card(sts2_engine::CardCmd::Upgrade {
                    cards: sts2_engine::CardSelector::RolledFromDeckDistinct {
                        filter: sts2_engine::CardFilter::Upgradable,
                        stream: sts2_engine::ShuffleStream::CombatCardSelection,
                        take: stacks(),
                    },
                })],
            )],
            ..counter("POWER.IMPROVEMENT_POWER", PowerType::Buff)
        },
        // Dark shackles takes strength for the turn and gives it back after.
        PowerDefinition {
            temporary: Some(TemporaryPower {
                internally_applied_power: id("POWER.STRENGTH_POWER"),
                positive: false,
            }),
            ..counter("POWER.DARK_SHACKLES_POWER", PowerType::Debuff)
        },
        // Equilibrium keeps the whole hand where it is, and is spent for the
        // turn it kept.
        PowerDefinition {
            keeps_hand: true,
            hooks: vec![
                ModifierHook::every_turn(HookEvent::AfterSideTurnEnd, vec![decrement()])
                    .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.RETAIN_HAND_POWER", PowerType::Buff)
        },
        // Fasten adds to the block a Defend works up, and to block that came off
        // no card at all.
        PowerDefinition {
            hooks: vec![modifies(
                HookEvent::ModifyBlockAdditive,
                HookCondition::OwnerGainsPoweredBlockNotFromAnUntagged(
                    sts2_engine::CardTag::Defend,
                ),
                vec![Command::Modify(ModifierCmd::Add(stacks()))],
            )],
            ..counter("POWER.FASTEN_POWER", PowerType::Buff)
        },
        // Mayhem plays what is on top of the draw pile as the turn's auto-play
        // phase opens, as many times as it carries.
        PowerDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterAutoPrePlayPhaseEntered,
                vec![Command::CardPile(CardPileCmd::AutoPlayFromDrawPile {
                    count: stacks(),
                    position: sts2_engine::DrawPilePosition::Top,
                    force_exhaust: false,
                })],
            )],
            ..counter("POWER.MAYHEM_POWER", PowerType::Buff)
        },
        // A panic button pays no block off a card at all while it stands, and
        // one stack of it goes with every enemy turn.
        PowerDefinition {
            hooks: vec![
                modifies(
                    HookEvent::ModifyBlockMultiplicative,
                    HookCondition::OwnerGainsPoweredBlockFromACard,
                    vec![Command::Modify(ModifierCmd::Scale(Amount::Fixed(0)))],
                ),
                ModifierHook::every_turn(HookEvent::AfterSideTurnEnd, vec![decrement()])
                    .when(HookCondition::EnemySideTurn),
            ],
            ..counter("POWER.NO_BLOCK_POWER", PowerType::Debuff)
        },
        // Block owed for the next turn: it lands the moment its owner's block is
        // cleared, and the power goes with it.
        PowerDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterBlockCleared,
                vec![
                    Command::Creature(CreatureCmd::GainBlock {
                        target: CreatureSelector::Owner,
                        amount: stacks(),
                        props: ValueProps::UNPOWERED,
                    }),
                    remove_self(),
                ],
            )],
            ..counter("POWER.BLOCK_NEXT_TURN_POWER", PowerType::Buff)
        },
        // Automation counts the cards its owner draws and pays energy at every
        // tenth one.
        PowerDefinition {
            instance_type: PowerInstanceType::Instanced,
            hooks: vec![
                // The count opens at ten; the instance's own counter is where
                // it is kept, because the amount is the energy.
                ModifierHook::every_turn(
                    HookEvent::AfterApplied,
                    vec![Command::Power(PowerCmd::SetCounter {
                        power: PowerRef::listening(),
                        amount: Amount::Fixed(10),
                    })],
                ),
                ModifierHook::every_turn(
                    HookEvent::AfterCardDrawn,
                    vec![
                        Command::Power(PowerCmd::ModifyCounter {
                            power: PowerRef::listening(),
                            amount: Amount::Fixed(-1),
                        }),
                        Command::When {
                            otherwise: Vec::new(),
                            condition: Condition::ListeningPowerCounterIs(0),
                            then: vec![
                                Command::Player(PlayerCmd::GainEnergy(stacks())),
                                Command::Power(PowerCmd::SetCounter {
                                    power: PowerRef::listening(),
                                    amount: Amount::Fixed(10),
                                }),
                            ],
                        },
                    ],
                ),
            ],
            ..counter("POWER.AUTOMATION_POWER", PowerType::Buff)
        },
        // Panache counts five card plays and then hits everything, and the count
        // goes back to five as the turn ends. The card that applied it does not
        // count towards its own five, which is the latch.
        PowerDefinition {
            instance_type: PowerInstanceType::Instanced,
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterApplied,
                    vec![Command::Power(PowerCmd::SetCounter {
                        power: PowerRef::listening(),
                        amount: Amount::Fixed(5),
                    })],
                ),
                ModifierHook::every_turn(
                    HookEvent::AfterCardPlayed,
                    vec![
                        Command::When {
                            otherwise: Vec::new(),
                            condition: Condition::ListeningPowerIsLatched,
                            then: vec![
                                Command::Power(PowerCmd::ModifyCounter {
                                    power: PowerRef::listening(),
                                    amount: Amount::Fixed(-1),
                                }),
                                Command::When {
                                    otherwise: Vec::new(),
                                    condition: Condition::ListeningPowerCounterIs(0),
                                    then: vec![
                                        Command::Creature(CreatureCmd::Damage {
                                            target: CreatureSelector::AllOpponents,
                                            amount: stacks(),
                                            props: ValueProps::UNPOWERED,
                                        }),
                                        Command::Power(PowerCmd::SetCounter {
                                            power: PowerRef::listening(),
                                            amount: Amount::Fixed(5),
                                        }),
                                    ],
                                },
                            ],
                        },
                        Command::Power(PowerCmd::SetLatch(true)),
                    ],
                ),
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnEnd,
                    vec![Command::Power(PowerCmd::SetCounter {
                        power: PowerRef::listening(),
                        amount: Amount::Fixed(5),
                    })],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.PANACHE_POWER", PowerType::Buff)
        },
        // A rolling boulder rolls over everything as its owner's turn opens, and
        // rolls harder every turn.
        PowerDefinition {
            instance_type: PowerInstanceType::Instanced,
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterPlayerTurnStart,
                vec![
                    Command::Creature(CreatureCmd::Damage {
                        target: CreatureSelector::AllOpponents,
                        amount: stacks(),
                        props: ValueProps::UNPOWERED,
                    }),
                    Command::Power(PowerCmd::SetAmount {
                        power: PowerRef::listening(),
                        amount: Amount::Scaled {
                            base: 5,
                            per: 1,
                            count: Count::ListeningPowerAmount,
                        },
                    }),
                ],
            )],
            ..counter("POWER.ROLLING_BOULDER_POWER", PowerType::Buff)
        },
        // The bomb counts turns down and then goes off over everything.
        PowerDefinition {
            instance_type: PowerInstanceType::Instanced,
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::BeforeSideTurnEnd,
                    // Counting down and going off are one test with two arms:
                    // a bomb with more than one turn left only ticks. Written
                    // as two tests the second would be asked after the first
                    // had already taken the power away, which reads the next
                    // bomb rather than nothing.
                    vec![Command::When {
                        condition: Condition::ListeningPowerAmountAbove(1),
                        then: vec![decrement()],
                        otherwise: vec![
                            Command::Creature(CreatureCmd::Damage {
                                target: CreatureSelector::AllOpponents,
                                amount: Amount::counting(Count::ListeningPowerCounter),
                                props: ValueProps::UNPOWERED,
                            }),
                            remove_self(),
                        ],
                    }],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.THE_BOMB_POWER", PowerType::Buff)
        },
        // Prep time works vigor up as its owner's side turn opens.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    vec![apply_to_owner(
                        "POWER.VIGOR_POWER",
                        CreatureSelector::Owner,
                        stacks(),
                    )],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.PREP_TIME_POWER", PowerType::Buff)
        },
        // Calamity deals out attacks of its owner's own pool for every attack
        // they play.
        //
        // The build writes the card down as its play opens and pays out for it
        // as the same play finishes, so a calamity that landed inside an
        // attack's own play was not standing when that play opened and owes it
        // nothing. What is paid out is the amount as the play *finishes*, which
        // is the live one rather than the one written down.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardPlayed,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningPowerAmountBeforeThisPlayAtLeast(0),
                        then: vec![Command::CardPile(CardPileCmd::AddGeneratedFromPool {
                            pool: sts2_engine::CardPoolRef::OwnCharacter,
                            count: stacks(),
                            distinct: false,
                            filter: sts2_engine::CardFilter::OfType(sts2_engine::CardType::Attack),
                            pile: sts2_engine::PileName::Hand,
                            position: sts2_engine::DrawPilePosition::Bottom,
                            excludes_source: false,
                            upgraded: false,
                            temporary_cost: None,
                            ethereal: false,
                            rolled_one_at_a_time: false,
                        })],
                    }],
                )
                .when(HookCondition::All(vec![
                    HookCondition::OwnerPlayedTheCard,
                    HookCondition::PlayedCardIsAttack,
                ])),
            ],
            ..counter("POWER.CALAMITY_POWER", PowerType::Buff)
        },
        // Nostalgia sends the first few attacks and skills of a turn back on
        // top of the draw pile instead of to the discard.
        PowerDefinition {
            played_card_rule: Some(
                sts2_engine::PlayedCardRule::EarlyAttackOrSkillPlaysToDrawPileTop,
            ),
            ..counter("POWER.NOSTALGIA_POWER", PowerType::Buff)
        },
        // The gambit is paid for by the first blow that gets through: its owner
        // goes down outright.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterDamageReceived,
                    vec![
                        remove_self(),
                        Command::Creature(CreatureCmd::Kill {
                            target: CreatureSelector::Owner,
                        }),
                    ],
                )
                .when(HookCondition::OwnerTookUnblockedPoweredDamage),
            ],
            ..single("POWER.THE_GAMBIT_POWER", PowerType::Debuff)
        },
        // Vigor binds one swing of its owner's, adds itself to every hit of it,
        // and is spent whole for it. The binding is never cleared: a power that
        // survives the swing it bound — one stacked mid-swing does — goes on
        // adding to attacks from that same card and refuses every other, and
        // spends nothing more, because what it owed was written down as the
        // swing opened and is cleared with the payout.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::BeforeAttack,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::Not(Box::new(Condition::ListeningPowerIsLatched)),
                        then: vec![
                            Command::Power(PowerCmd::SetLatch(true)),
                            Command::Power(PowerCmd::BindDamageCard),
                            Command::Power(PowerCmd::SetCounter {
                                power: PowerRef::listening(),
                                amount: stacks(),
                            }),
                        ],
                    }],
                )
                .when(HookCondition::OwnerDealsPoweredDamage),
                modifies(
                    HookEvent::ModifyDamageAdditive,
                    HookCondition::OwnerDealsPoweredDamage,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::DamageCardMatchesTheListeningPowersBinding,
                        then: vec![Command::Modify(ModifierCmd::Add(stacks()))],
                    }],
                ),
                ModifierHook::every_turn(
                    HookEvent::AfterAttack,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningPowerIsLatched,
                        then: vec![
                            Command::Repeat {
                                times: Amount::counting(Count::ListeningPowerCounter),
                                then: vec![decrement()],
                            },
                            Command::Power(PowerCmd::SetCounter {
                                power: PowerRef::listening(),
                                amount: Amount::Fixed(0),
                            }),
                        ],
                    }],
                ),
            ],
            ..counter("POWER.VIGOR_POWER", PowerType::Buff)
        },
    ]
}

/// A power that does nothing of its own: something else reads it off whoever
/// is carrying it.
fn marker(model_id: &str, stack_type: PowerStackType) -> PowerDefinition {
    power(model_id, PowerType::Buff, stack_type)
}

/// A power taken off outright as its owner's side finishes a turn.
fn spent_at_the_end_of_the_turn(mut definition: PowerDefinition) -> PowerDefinition {
    definition.hooks.push(
        ModifierHook::every_turn(HookEvent::AfterSideTurnEnd, vec![remove_self()])
            .when(HookCondition::OwnerIsParticipant),
    );
    definition
}

/// Poisons every enemy that can be hit, applied by the power's own owner.
fn poison_everything(amount: Amount) -> Command {
    Command::Power(PowerCmd::Apply {
        marks: None,
        power: id("POWER.POISON_POWER"),
        target: CreatureSelector::AllOpponents,
        applier: Some(CreatureSelector::Owner),
        amount,
    })
}

/// The Silent's own powers, and the ones only her cards apply.
#[must_use]
#[allow(clippy::too_many_lines, reason = "one entry per power")]
pub fn silent_powers() -> Vec<PowerDefinition> {
    let powers = vec![
        // A shadow step is spent as its owner's turn opens, and what it was
        // worth is handed over as double damage.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    vec![
                        apply_to_owner(
                            "POWER.DOUBLE_DAMAGE_POWER",
                            CreatureSelector::Owner,
                            stacks(),
                        ),
                        remove_self(),
                    ],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.SHADOW_STEP_POWER", PowerType::Buff)
        },
        // Double damage doubles what its owner — or a pet of its owner's —
        // swings off a card, and one of it is spent as their side's turn ends.
        // The either-or is said the long way round, because the surface carries
        // `All` and `Not` and no `Any`.
        PowerDefinition {
            hooks: vec![
                modifies(
                    HookEvent::ModifyDamageMultiplicative,
                    HookCondition::Not(Box::new(HookCondition::All(vec![
                        HookCondition::Not(Box::new(
                            HookCondition::OwnerDealsPoweredDamageFromACard,
                        )),
                        HookCondition::Not(Box::new(
                            HookCondition::OwnersPetDealsPoweredDamageFromACard,
                        )),
                    ]))),
                    vec![Command::Modify(ModifierCmd::Scale(Amount::Fixed(200)))],
                ),
                ModifierHook::every_turn(HookEvent::AfterSideTurnEnd, vec![decrement()])
                    .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.DOUBLE_DAMAGE_POWER", PowerType::Buff)
        },
        // Poison costs its owner what it is worth as their turn opens, and one
        // of it wears off for every time it bit. An accelerant on the other
        // side buys extra bites, and the amount comes down between them, so the
        // second is worth one less than the first.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    vec![Command::Repeat {
                        times: Amount::Least(vec![
                            stacks(),
                            Amount::Sum(vec![
                                Amount::Fixed(1),
                                Amount::counting(Count::LivingOpponentsPower(id(
                                    "POWER.ACCELERANT_POWER",
                                ))),
                            ]),
                        ]),
                        then: vec![
                            Command::Creature(CreatureCmd::Damage {
                                target: CreatureSelector::Owner,
                                amount: stacks(),
                                props: ValueProps::UNBLOCKABLE,
                            }),
                            // The stack only comes off if the owner survived
                            // the blow.
                            Command::When {
                                otherwise: Vec::new(),
                                condition: Condition::OwnerIsAlive,
                                then: vec![decrement()],
                            },
                        ],
                    }],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.POISON_POWER", PowerType::Debuff)
        },
        // An accelerant does nothing itself: poison looks for it on the other
        // side and bites once more for each of it.
        marker("POWER.ACCELERANT_POWER", PowerStackType::Counter),
        // A fan of knives does nothing itself either: the Shiv reads it and
        // throws itself at everything instead of at one thing.
        marker("POWER.FAN_OF_KNIVES_POWER", PowerStackType::Single),
        // The hunt's power is the mark that says the card paid out; the card
        // itself is where the paying happens.
        marker("POWER.THE_HUNT_POWER", PowerStackType::Counter),
        // Accuracy pays out on a Shiv and on nothing else.
        PowerDefinition {
            hooks: vec![modifies(
                HookEvent::ModifyDamageAdditive,
                HookCondition::OwnerDealsPoweredDamageFromACard,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::DamageCardIsTagged(sts2_engine::CardTag::Shiv),
                    then: vec![Command::Modify(ModifierCmd::Add(stacks()))],
                }],
            )],
            ..counter("POWER.ACCURACY_POWER", PowerType::Buff)
        },
        // An afterimage is block for every card its owner puts down. It writes
        // what it was worth down as each play begins, so playing the card that
        // laid it is not itself worth anything.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardPlayed,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningPowerAmountBeforeThisPlayAtLeast(1),
                        then: vec![Command::Creature(CreatureCmd::GainBlock {
                            target: CreatureSelector::Owner,
                            amount: Amount::counting(Count::ListeningPowerAmountBeforeThisPlay),
                            props: ValueProps::UNPOWERED,
                        })],
                    }],
                )
                .when(HookCondition::OwnerPlayedTheCard),
            ],
            ..counter("POWER.AFTERIMAGE_POWER", PowerType::Buff)
        },
        // Anticipating is dexterity lent for the turn.
        PowerDefinition {
            temporary: Some(TemporaryPower {
                internally_applied_power: id("POWER.DEXTERITY_POWER"),
                positive: true,
            }),
            ..counter("POWER.ANTICIPATE_POWER", PowerType::Buff)
        },
        // A helical dart lends dexterity for the turn, the way a flex lends
        // strength.
        PowerDefinition {
            temporary: Some(TemporaryPower {
                internally_applied_power: id("POWER.DEXTERITY_POWER"),
                positive: true,
            }),
            ..counter("POWER.HELICAL_DART_POWER", PowerType::Buff)
        },
        // A piercing wail takes strength for the turn and hands it back.
        PowerDefinition {
            temporary: Some(TemporaryPower {
                internally_applied_power: id("POWER.STRENGTH_POWER"),
                positive: false,
            }),
            ..counter("POWER.PIERCING_WAIL_POWER", PowerType::Debuff)
        },
        // A blur keeps the block its owner is standing behind, and one of it is
        // spent as their turn opens.
        PowerDefinition {
            keeps_block: true,
            hooks: vec![
                ModifierHook::every_turn(HookEvent::AfterSideTurnStart, vec![decrement()])
                    .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.BLUR_POWER", PowerType::Buff)
        },
        // A burst plays the next skill twice, and is spent doing it.
        spent_at_the_end_of_the_turn(PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::ModifyCardPlayCount,
                    vec![Command::Modify(ModifierCmd::Add(Amount::Fixed(1)))],
                )
                .when(HookCondition::All(vec![
                    HookCondition::OwnerPlayedTheCard,
                    HookCondition::PlayedCardIsOfType(sts2_engine::CardType::Skill),
                ])),
                ModifierHook::every_turn(HookEvent::AfterModifyingCardPlayCount, vec![decrement()]),
            ],
            ..counter("POWER.BURST_POWER", PowerType::Buff)
        }),
        // A corrosive wave poisons everything every time its owner draws, and
        // washes out at the end of the turn.
        spent_at_the_end_of_the_turn(PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardDrawn,
                    vec![poison_everything(stacks())],
                )
                .when(HookCondition::OwnersCardDrawn),
            ],
            ..counter("POWER.CORROSIVE_WAVE_POWER", PowerType::Buff)
        }),
        // Envenom poisons whatever its owner's attack takes HP off.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterDamageGiven,
                    vec![Command::Power(PowerCmd::Apply {
                        marks: None,
                        power: id("POWER.POISON_POWER"),
                        target: CreatureSelector::Target,
                        applier: Some(CreatureSelector::Owner),
                        amount: stacks(),
                    })],
                )
                .when(HookCondition::OwnerDealtUnblockedPoweredDamage),
            ],
            ..counter("POWER.ENVENOM_POWER", PowerType::Buff)
        },
        // Flanking makes everyone but whoever worked it hit its owner harder,
        // for one turn.
        spent_at_the_end_of_the_turn(PowerDefinition {
            instance_type: PowerInstanceType::Instanced,
            hooks: vec![modifies(
                HookEvent::ModifyDamageMultiplicative,
                HookCondition::OwnerTakesPoweredDamage,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::Not(Box::new(Condition::DamageDealerIsTheApplier)),
                    then: vec![Command::Modify(ModifierCmd::Scale(Amount::per(
                        0,
                        100,
                        Count::ListeningPowerAmount,
                    )))],
                }],
            )],
            ..counter("POWER.FLANKING_POWER", PowerType::Debuff)
        }),
        // A free skill is one skill of the hand costing nothing, spent as the
        // skill is played.
        PowerDefinition {
            card_cost_rule: Some(sts2_engine::CardCostRule::FreeSkillsInHandOrPlay),
            hooks: vec![
                ModifierHook::every_turn(HookEvent::BeforeCardPlayed, vec![decrement()]).when(
                    HookCondition::All(vec![
                        HookCondition::OwnerPlayedTheCard,
                        HookCondition::PlayedCardIsOfType(sts2_engine::CardType::Skill),
                    ]),
                ),
            ],
            ..counter("POWER.FREE_SKILL_POWER", PowerType::Buff)
        },
        // Infinite blades deals its owner a Shiv before every hand.
        PowerDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::BeforeHandDraw,
                vec![shivs_into_hand(stacks(), false)],
            )],
            ..counter("POWER.INFINITE_BLADES_POWER", PowerType::Buff)
        },
        // A master planner makes every skill its owner plays Sly, for good.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardPlayed,
                    vec![Command::Card(CardCmd::ApplyKeyword {
                        cards: CardSelector::PlayedCard,
                        keyword: sts2_engine::CardKeyword::Sly,
                    })],
                )
                .when(HookCondition::All(vec![
                    HookCondition::OwnerPlayedTheCard,
                    HookCondition::PlayedCardIsOfType(sts2_engine::CardType::Skill),
                ])),
            ],
            ..single("POWER.MASTER_PLANNER_POWER", PowerType::Buff)
        },
        // A nightmare holds the card it was pointed at and deals that many
        // copies of it into the next hand, once.
        PowerDefinition {
            instance_type: PowerInstanceType::Instanced,
            hooks: vec![ModifierHook::every_turn(
                HookEvent::BeforeHandDraw,
                vec![
                    Command::CardPile(CardPileCmd::AddGeneratedToCombat {
                        enchantment: None,
                        card: CardOrigin::CloneOfHeldByListeningPower,
                        grown_by: Amount::Fixed(0),
                        count: stacks(),
                        pile: PileName::Hand,
                        ethereal: false,
                        retain: false,
                        upgraded: false,
                        temporary_cost: None,
                        position: DrawPilePosition::Bottom,
                    }),
                    remove_self(),
                ],
            )],
            ..counter("POWER.NIGHTMARE_POWER", PowerType::Buff)
        },
        // Noxious fumes poisons everything as its owner's turn opens.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    vec![poison_everything(stacks())],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.NOXIOUS_FUMES_POWER", PowerType::Buff)
        },
        // An outbreak counts the times its owner has worked poison on anything,
        // and every third one it burns everything for what it is worth. The
        // count is the power's own, and it is kept modulo three.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterPowerAmountChanged,
                    vec![
                        Command::Power(PowerCmd::ModifyCounter {
                            power: PowerRef::listening(),
                            amount: Amount::Fixed(1),
                        }),
                        Command::When {
                            otherwise: Vec::new(),
                            condition: Condition::ListeningPowerCounterAtLeast(3),
                            then: vec![
                                Command::Creature(CreatureCmd::Damage {
                                    target: CreatureSelector::AllOpponents,
                                    amount: stacks(),
                                    props: ValueProps::UNPOWERED,
                                }),
                                Command::Power(PowerCmd::ModifyCounter {
                                    power: PowerRef::listening(),
                                    amount: Amount::Fixed(-3),
                                }),
                            ],
                        },
                    ],
                )
                .when(HookCondition::OwnerWorkedPower(id("POWER.POISON_POWER"))),
            ],
            ..counter("POWER.OUTBREAK_POWER", PowerType::Buff)
        },
        // Phantom blades make every Shiv keep itself, and put their whole worth
        // behind the first Shiv of a turn and nothing behind the rest.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardEnteredCombat,
                    vec![Command::Card(CardCmd::ApplyKeyword {
                        cards: CardSelector::Announced,
                        keyword: sts2_engine::CardKeyword::Retain,
                    })],
                )
                .when(HookCondition::EnteredCardIsTagged(
                    sts2_engine::CardTag::Shiv,
                )),
                ModifierHook::every_turn(
                    HookEvent::AfterApplied,
                    vec![Command::Card(CardCmd::ApplyKeyword {
                        cards: CardSelector::AllCombatCards {
                            filter: CardFilter::Tagged(sts2_engine::CardTag::Shiv),
                        },
                        keyword: sts2_engine::CardKeyword::Retain,
                    })],
                )
                .when(HookCondition::PowerAppliedToOwner),
                modifies(
                    HookEvent::ModifyDamageAdditive,
                    HookCondition::OwnerDealsPoweredDamageFromACard,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::All(vec![
                            Condition::DamageCardIsTagged(sts2_engine::CardTag::Shiv),
                            Condition::Not(Box::new(Condition::OwnerPlayedATaggedCardThisTurn(
                                sts2_engine::CardTag::Shiv,
                            ))),
                        ]),
                        then: vec![Command::Modify(ModifierCmd::Add(stacks()))],
                    }],
                ),
            ],
            ..counter("POWER.PHANTOM_BLADES_POWER", PowerType::Buff)
        },
        // A serpent form throws what it is worth at something on the other side
        // for every card its owner puts down, the same way an afterimage pays
        // block.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardPlayed,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningPowerAmountBeforeThisPlayAtLeast(1),
                        then: vec![Command::Creature(CreatureCmd::Damage {
                            target: CreatureSelector::RandomOpponent,
                            amount: Amount::counting(Count::ListeningPowerAmountBeforeThisPlay),
                            props: ValueProps::UNPOWERED,
                        })],
                    }],
                )
                .when(HookCondition::OwnerPlayedTheCard),
            ],
            ..counter("POWER.SERPENT_FORM_POWER", PowerType::Buff)
        },
        // A shadowmeld doubles every block its owner works up, once per stack,
        // and is gone at the end of the turn.
        spent_at_the_end_of_the_turn(PowerDefinition {
            hooks: vec![modifies(
                HookEvent::ModifyBlockMultiplicative,
                HookCondition::OwnerIsGainingBlock,
                vec![Command::Modify(ModifierCmd::Scale(Amount::Product {
                    left: Box::new(Amount::Fixed(100)),
                    right: Box::new(Amount::TwoToThe(Box::new(stacks()))),
                }))],
            )],
            ..counter("POWER.SHADOWMELD_POWER", PowerType::Buff)
        }),
        // Being sneaky is block whenever somebody else plays an attack, which
        // in a single-player run is nobody: the card is multiplayer-only and no
        // single-player pool holds it.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardPlayed,
                    vec![Command::Creature(CreatureCmd::GainBlock {
                        target: CreatureSelector::Owner,
                        amount: stacks(),
                        props: ValueProps::UNPOWERED,
                    })],
                )
                .when(HookCondition::All(vec![
                    HookCondition::PlayedCardIsAttack,
                    HookCondition::Not(Box::new(HookCondition::OwnerPlayedTheCard)),
                ])),
            ],
            ..counter("POWER.SNEAKY_POWER", PowerType::Buff)
        },
        // A speedster cuts everything every time its owner draws a card outside
        // the turn's own opening hand, and only while their side is acting.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardDrawn,
                    vec![Command::Creature(CreatureCmd::Damage {
                        target: CreatureSelector::AllOpponents,
                        amount: stacks(),
                        props: ValueProps::UNPOWERED,
                    })],
                )
                .when(HookCondition::OwnersCardDrawnMidTurn),
            ],
            ..counter("POWER.SPEEDSTER_POWER", PowerType::Buff)
        },
        // A strangle costs whoever is carrying it what it is worth for every
        // card the player who worked it puts down. Unlike its two siblings it
        // has no test on the amount, so a strangle worth nothing still deals a
        // blow worth nothing.
        spent_at_the_end_of_the_turn(PowerDefinition {
            instance_type: PowerInstanceType::InstancedPerApplier,
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardPlayed,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningPowerAmountBeforeThisPlayAtLeast(0),
                        then: vec![Command::Creature(CreatureCmd::Damage {
                            target: CreatureSelector::Owner,
                            amount: Amount::counting(Count::ListeningPowerAmountBeforeThisPlay),
                            props: ValueProps::UNBLOCKABLE,
                        })],
                    }],
                )
                .when(HookCondition::ApplierPlayedTheCard),
            ],
            ..counter("POWER.STRANGLE_POWER", PowerType::Debuff)
        }),
        // Tools of the trade draw one more card a turn and ask for one back.
        PowerDefinition {
            hooks: vec![
                modifies(
                    HookEvent::ModifyHandDraw,
                    HookCondition::Always,
                    vec![Command::Modify(ModifierCmd::Add(stacks()))],
                ),
                ModifierHook::every_turn(
                    HookEvent::AfterPlayerTurnStart,
                    vec![Command::CardSelect(CardSelectCmd::FromHandForDiscard {
                        semantics: ChoiceSemantics::known(vec![
                            ChoiceOp::new(ChoiceOperationKind::Discard, ChoiceScope::Combat)
                                .to(ChoiceDestination::Discard),
                        ]),
                        prefs: SelectorPrefs::exactly(0),
                        maximum: None,
                        exact_count: Some(stacks()),
                        filter: CardFilter::Any,
                        then: vec![Command::Card(CardCmd::DiscardAndDraw {
                            cards: CardSelector::Chosen,
                            count: Amount::Fixed(0),
                        })],
                    })],
                ),
            ],
            ..counter("POWER.TOOLS_OF_THE_TRADE_POWER", PowerType::Buff)
        },
        // Tracking multiplies what its owner's cards deal to anything shaken.
        PowerDefinition {
            hooks: vec![modifies(
                HookEvent::ModifyDamageMultiplicative,
                HookCondition::OwnerDealsPoweredDamageFromACard,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::TargetHasPower(id("POWER.WEAK_POWER")),
                    then: vec![Command::Modify(ModifierCmd::Scale(Amount::per(
                        0,
                        100,
                        Count::ListeningPowerAmount,
                    )))],
                }],
            )],
            ..counter("POWER.TRACKING_POWER", PowerType::Buff)
        },
        // Well laid plans keeps back as many cards of the hand as it is worth.
        // It asks on `BeforeFlushLate` — deliberately the late pass, so that
        // the player sees what every other before-flush effect has done to the
        // hand before choosing what to keep. Every listener on that pass gets
        // to act before any of them finishes, so the engine banks the screen
        // and opens it once the whole before-flush pass has run and before the
        // hand goes down.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::BeforeFlushLate,
                    vec![Command::CardSelect(CardSelectCmd::FromHand {
                        semantics: ChoiceSemantics::known(vec![
                            ChoiceOp::new(ChoiceOperationKind::GrantKeyword, ChoiceScope::Combat)
                                .keyword(sts2_engine::CardKeyword::Retain)
                                .lasting(ChoiceDuration::Turn),
                        ]),
                        prefs: SelectorPrefs::unbounded_range(),
                        maximum: Some(stacks()),
                        exact_count: None,
                        filter: CardFilter::NotRetainingThisTurn,
                        then: vec![Command::Card(CardCmd::ApplySingleTurnKeyword {
                            cards: CardSelector::Chosen,
                            keyword: sts2_engine::CardKeyword::Retain,
                        })],
                    })],
                )
                // Only a hand that is about to be discarded is asked; a hand
                // a pyramid keeps up is not.
                .when(HookCondition::TheHandWouldFlush),
            ],
            ..counter("POWER.WELL_LAID_PLANS_POWER", PowerType::Buff)
        },
        // A wraith form costs its owner dexterity every turn, for good: it
        // never takes itself off.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    vec![apply_to_owner(
                        "POWER.DEXTERITY_POWER",
                        CreatureSelector::Owner,
                        Amount::negating(Count::ListeningPowerAmount),
                    )],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.WRAITH_FORM_POWER", PowerType::Debuff)
        },
    ];
    powers
}

/// The Necrobinder's own powers.
///
/// Her pet's is here rather than with her cards' because nothing plays it: a
/// summon puts it on the pet as the pet arrives, and what it declares is the
/// whole of the pet's relationship with its owner.
#[must_use]
#[allow(clippy::too_many_lines, reason = "one entry per power")]
/// What the power is worth, dealt over the whole enemy side, unpowered.
fn sweeping_damage() -> Command {
    Command::Creature(CreatureCmd::Damage {
        target: CreatureSelector::AllOpponents,
        amount: stacks(),
        props: ValueProps::UNPOWERED,
    })
}

/// The Defect's own powers: what a Focus is worth to an orb, the two cards that
/// bank a stat for a turn, and the bodies his pool applies.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one literal per power model the pool applies"
)]
pub fn defect_powers() -> Vec<PowerDefinition> {
    let channel = |orb: &str| {
        Command::Orb(sts2_engine::OrbCmd::Channel(sts2_engine::OrbSource::Model(
            id(orb),
        )))
    };
    vec![
        // Focus is what every orb that offers a number to `ModifyOrbValue`
        // reads: the stacks are added and the answer is floored at nothing, so a
        // Focus worked below zero never turns an orb negative. It may go below
        // zero itself, which is what a Hyperbeam and a Biased Cognition do.
        PowerDefinition {
            allow_negative: true,
            hooks: vec![modifies(
                HookEvent::ModifyOrbValue,
                HookCondition::OrbBelongsToOwner,
                vec![Command::Modify(sts2_engine::ModifierCmd::Set(
                    Amount::Greatest(vec![
                        Amount::Fixed(0),
                        Amount::Sum(vec![Amount::counting(Count::ModifiedValue), stacks()]),
                    ]),
                ))],
            )],
            ..counter("POWER.FOCUS_POWER", PowerType::Buff)
        },
        // The three temporary-Focus wrappers: each lends Focus as it lands
        // and gives it back at the end of a turn its owner took part in.
        temporary_focus("POWER.FOCUSED_STRIKE_POWER"),
        temporary_focus("POWER.HOTFIX_POWER"),
        temporary_focus("POWER.SYNCHRONIZE_POWER"),
        // A lightning rod channels one orb as the energy is reset rather than
        // as the turn opens, so what an orb evoked to make room pays out is not
        // lost to the reset or to the block clear. One stack goes with it.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterEnergyReset,
                    vec![channel("ORB.LIGHTNING_ORB"), decrement()],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.LIGHTNING_ROD_POWER", PowerType::Buff)
        },
        // A hailstorm hits everything as the turn ends, where at least one
        // Frost orb is standing.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::BeforeSideTurnEnd,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::CountAtLeast {
                            count: Count::OrbsHeldOfModel(id("ORB.FROST_ORB")),
                            at_least: 1,
                        },
                        then: vec![Command::Creature(CreatureCmd::Damage {
                            target: CreatureSelector::AllOpponents,
                            amount: stacks(),
                            props: ValueProps::UNPOWERED,
                        })],
                    }],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.HAILSTORM_POWER", PowerType::Buff)
        },
        // A thunder swings at whatever a Lightning orb's evoke reached, less
        // whatever it killed.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterOrbEvoked,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::AnnouncedOrbIs(id("ORB.LIGHTNING_ORB")),
                        then: vec![Command::Creature(CreatureCmd::Damage {
                            target: CreatureSelector::EvokedOrbTargets,
                            amount: stacks(),
                            props: ValueProps::UNPOWERED,
                        })],
                    }],
                )
                .when(HookCondition::OrbBelongsToOwner),
            ],
            ..counter("POWER.THUNDER_POWER", PowerType::Buff)
        },
        // Trash to treasure turns a status its owner conjured into orbs, one
        // rolled orb per stack.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardGeneratedForCombat,
                    vec![Command::Repeat {
                        times: stacks(),
                        then: vec![Command::Orb(sts2_engine::OrbCmd::Channel(
                            sts2_engine::OrbSource::RandomOnCombatOrbs,
                        ))],
                    }],
                )
                .when(HookCondition::All(vec![
                    HookCondition::OwnerConjuredTheCard,
                    HookCondition::ConjuredCardIsOfType(sts2_engine::CardType::Status),
                ])),
            ],
            ..counter("POWER.TRASH_TO_TREASURE_POWER", PowerType::Buff)
        },
        // A smokestack hits everything for its stacks instead.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardGeneratedForCombat,
                    vec![Command::Creature(CreatureCmd::Damage {
                        target: CreatureSelector::AllOpponents,
                        amount: stacks(),
                        props: ValueProps::UNPOWERED,
                    })],
                )
                .when(HookCondition::All(vec![
                    HookCondition::OwnerConjuredTheCard,
                    HookCondition::ConjuredCardIsOfType(sts2_engine::CardType::Status),
                ])),
            ],
            ..counter("POWER.SMOKESTACK_POWER", PowerType::Buff)
        },
        // A spinner channels its stacks in Glass orbs as the energy is reset.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterEnergyReset,
                    vec![Command::Repeat {
                        times: stacks(),
                        then: vec![channel("ORB.GLASS_ORB")],
                    }],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.SPINNER_POWER", PowerType::Buff)
        },
        // A storm channels a Lightning orb for every stack it was standing at
        // when a power card of its owner's began being played. A Storm applied
        // by the very play it would answer for was worth nothing then, so it
        // pays nothing for that play.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardPlayed,
                    vec![Command::Repeat {
                        times: Amount::counting(Count::ListeningPowerAmountBeforeThisPlay),
                        then: vec![channel("ORB.LIGHTNING_ORB")],
                    }],
                )
                .when(HookCondition::All(vec![
                    HookCondition::OwnerPlayedTheCard,
                    HookCondition::PlayedCardIsOfType(sts2_engine::CardType::Power),
                ])),
            ],
            ..counter("POWER.STORM_POWER", PowerType::Buff)
        },
        // A subroutine is the same shape, paying energy a point at a time.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardPlayed,
                    vec![Command::Repeat {
                        times: Amount::counting(Count::ListeningPowerAmountBeforeThisPlay),
                        then: vec![Command::Player(sts2_engine::PlayerCmd::GainEnergy(
                            Amount::Fixed(1),
                        ))],
                    }],
                )
                .when(HookCondition::All(vec![
                    HookCondition::OwnerPlayedTheCard,
                    HookCondition::PlayedCardIsOfType(sts2_engine::CardType::Power),
                ])),
            ],
            ..counter("POWER.SUBROUTINE_POWER", PowerType::Buff)
        },
        // A loop fires the orb at the front of the queue as its owner's turn
        // opens, once per stack. The guard is on the queue rather than on the
        // orb, so an empty queue does nothing at all.
        PowerDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterPlayerTurnStart,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::CountAtLeast {
                        count: Count::OrbsHeld,
                        at_least: 1,
                    },
                    then: vec![Command::Repeat {
                        times: stacks(),
                        then: vec![Command::Orb(sts2_engine::OrbCmd::Passive {
                            orb: sts2_engine::OrbSelector::Front,
                            target: None,
                        })],
                    }],
                }],
            )],
            ..counter("POWER.LOOP_POWER", PowerType::Buff)
        },
        // An iteration pays for the first status its owner draws in a turn and
        // for nothing after it: the count includes the card being answered for,
        // so `<= 1` is the first of them.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardDrawn,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::Not(Box::new(Condition::CountAtLeast {
                            count: Count::OwnerStatusCardsDrawnThisTurn,
                            at_least: 2,
                        })),
                        then: vec![Command::CardPile(CardPileCmd::Draw { count: stacks() })],
                    }],
                )
                .when(HookCondition::All(vec![
                    HookCondition::OwnersCardDrawn,
                    HookCondition::OwnersCardOfTypeDrawn(sts2_engine::CardType::Status),
                ])),
            ],
            ..counter("POWER.ITERATION_POWER", PowerType::Buff)
        },
        // Machine learning raises the count the turn's own hand draw settles
        // on, which is a chaining pass like every other `Modify*`.
        PowerDefinition {
            hooks: vec![modifies(
                HookEvent::ModifyHandDraw,
                HookCondition::Always,
                vec![Command::Modify(sts2_engine::ModifierCmd::Add(stacks()))],
            )],
            ..counter("POWER.MACHINE_LEARNING_POWER", PowerType::Buff)
        },
        // A creative AI deals a power card out of its owner's own pool into the
        // hand before the turn's count is settled, once per stack.
        PowerDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::BeforeHandDraw,
                vec![Command::Repeat {
                    times: stacks(),
                    then: vec![Command::CardPile(CardPileCmd::AddGeneratedFromPool {
                        pool: sts2_engine::CardPoolRef::OwnCharacter,
                        count: Amount::Fixed(1),
                        distinct: true,
                        filter: CardFilter::OfType(sts2_engine::CardType::Power),
                        pile: PileName::Hand,
                        position: sts2_engine::DrawPilePosition::Bottom,
                        excludes_source: false,
                        upgraded: false,
                        temporary_cost: None,
                        ethereal: false,
                        rolled_one_at_a_time: false,
                    })],
                }],
            )],
            ..counter("POWER.CREATIVE_AI_POWER", PowerType::Buff)
        },
        // An echo form runs its owner's body a second time for the first few
        // plays of a turn. The cap counts the plays that had already begun, so
        // the play being answered for is not one of them.
        PowerDefinition {
            hooks: vec![modifies(
                HookEvent::ModifyCardPlayCount,
                HookCondition::OwnerPlayedTheCard,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::CountBelowListeningPowerAmount(
                        Count::OwnerFirstPlaysStartedThisTurn,
                    ),
                    then: vec![Command::Modify(sts2_engine::ModifierCmd::Add(
                        Amount::Fixed(1),
                    ))],
                }],
            )],
            ..counter("POWER.ECHO_FORM_POWER", PowerType::Buff)
        },
        // A signal boost runs one power card twice and is spent for having done
        // so.
        PowerDefinition {
            hooks: vec![
                modifies(
                    HookEvent::ModifyCardPlayCount,
                    HookCondition::All(vec![
                        HookCondition::OwnerPlayedTheCard,
                        HookCondition::PlayedCardIsOfType(sts2_engine::CardType::Power),
                    ]),
                    vec![Command::Modify(sts2_engine::ModifierCmd::Add(
                        Amount::Fixed(1),
                    ))],
                ),
                ModifierHook::every_turn(HookEvent::AfterModifyingCardPlayCount, vec![decrement()]),
            ],
            ..counter("POWER.SIGNAL_BOOST_POWER", PowerType::Buff)
        },
        // A free power makes its owner's power cards cost nothing and is spent
        // when one of them is played.
        PowerDefinition {
            card_cost_rule: Some(sts2_engine::CardCostRule::FreePowersInHandOrPlay),
            hooks: vec![
                ModifierHook::every_turn(HookEvent::BeforeCardPlayed, vec![decrement()]).when(
                    HookCondition::All(vec![
                        HookCondition::OwnerPlayedTheCard,
                        HookCondition::PlayedCardIsOfType(sts2_engine::CardType::Power),
                    ]),
                ),
            ],
            ..counter("POWER.FREE_POWER_POWER", PowerType::Buff)
        },
        // A consuming shadow evokes the back of the queue as the turn ends,
        // once per stack, where anything is standing.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnEnd,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::CountAtLeast {
                            count: Count::OrbsHeld,
                            at_least: 1,
                        },
                        then: vec![Command::Repeat {
                            times: stacks(),
                            then: vec![Command::Orb(sts2_engine::OrbCmd::EvokeLast {
                                dequeue: true,
                            })],
                        }],
                    }],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.CONSUMING_SHADOW_POWER", PowerType::Buff)
        },
        // A coolant pays block for every distinct orb model its owner holds as
        // their turn opens.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    vec![Command::Creature(CreatureCmd::GainBlock {
                        target: CreatureSelector::Owner,
                        amount: Amount::Product {
                            left: Box::new(Amount::counting(Count::DistinctOrbModelsHeld)),
                            right: Box::new(stacks()),
                        },
                        props: ValueProps::UNPOWERED,
                    })],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.COOLANT_POWER", PowerType::Buff)
        },
        // A biased cognition takes its own worth in Focus off every turn.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    vec![applied_to(
                        "POWER.FOCUS_POWER",
                        CreatureSelector::Owner,
                        Amount::negating(Count::ListeningPowerAmount),
                    )],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.BIASED_COGNITION_POWER", PowerType::Debuff)
        },
        // A feral sends its owner's free attacks back to the top of the hand,
        // as many of them per turn as it is worth. The count is the power's own
        // counter: it is put back to what the turn has already spent as the
        // power lands, wound on by the pass over whoever changed the answer,
        // and reset as its owner's turn opens.
        PowerDefinition {
            played_card_rule: Some(sts2_engine::PlayedCardRule::ZeroCostAttacksToHandTop),
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterApplied,
                    vec![Command::Power(PowerCmd::SetCounter {
                        power: PowerRef::listening(),
                        amount: Amount::counting(Count::OwnerZeroCostAttackPlaysThisTurn),
                    })],
                ),
                ModifierHook::every_turn(
                    HookEvent::AfterModifyingCardPlayResultPile,
                    vec![Command::Power(PowerCmd::ModifyCounter {
                        power: PowerRef::listening(),
                        amount: Amount::Fixed(1),
                    })],
                ),
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    vec![Command::Power(PowerCmd::SetCounter {
                        power: PowerRef::listening(),
                        amount: Amount::Fixed(0),
                    })],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.FERAL_POWER", PowerType::Buff)
        },
    ]
}

/// One of the temporary-Focus powers: a wrapper that works Focus onto its
/// owner as it lands and gives the same amount back at the end of a turn they
/// took part in.
fn temporary_focus(model_id: &str) -> PowerDefinition {
    PowerDefinition {
        temporary: Some(sts2_engine::TemporaryPower {
            internally_applied_power: id("POWER.FOCUS_POWER"),
            positive: true,
        }),
        ..counter(model_id, PowerType::Buff)
    }
}

/// The Regent's own powers: the stars his cards hand out, the forge they feed,
/// and the two markers his Sovereign Blade reads off its owner.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one literal per power model the pool applies"
)]
/// The three powers the gremlin merc's fight is built out of.
///
/// A Thievery is the purse it keeps: the amount is what one hand in it is
/// worth, and the counter is what it has taken so far. A Surprise is what lets
/// two more gremlins out when the merc goes down and what keeps the fight
/// standing until they are. And a Heist is what the fat gremlin carries off:
/// killing it before it flees hands the whole of the stolen gold back.
fn gremlin_merc_powers() -> Vec<PowerDefinition> {
    vec![
        PowerDefinition {
            instance_type: PowerInstanceType::Instanced,
            ..counter("POWER.THIEVERY_POWER", PowerType::Buff)
        },
        PowerDefinition {
            stops_combat_from_ending: true,
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterDeath,
                    vec![
                        Command::Creature(CreatureCmd::Add {
                            monster: id("MONSTER.SNEAKY_GREMLIN"),
                            slot: sts2_engine::SpawnSlot::Named("sneaky".to_owned()),
                            initial_state: None,
                            then: Vec::new(),
                        }),
                        Command::Creature(CreatureCmd::Add {
                            monster: id("MONSTER.FAT_GREMLIN"),
                            slot: sts2_engine::SpawnSlot::Named("fat".to_owned()),
                            initial_state: None,
                            then: vec![Command::Power(PowerCmd::Apply {
                                // The heist is aimed at the same purse the
                                // thievery was, which is what it hands back.
                                marks: Some(CreatureSelector::AllOpponents),
                                power: id("POWER.HEIST_POWER"),
                                target: CreatureSelector::Target,
                                applier: Some(CreatureSelector::Owner),
                                amount: Amount::counting(Count::OwnerPowerCounter(id(
                                    "POWER.THIEVERY_POWER",
                                ))),
                            })],
                        }),
                    ],
                )
                .when(HookCondition::OwnerDied),
            ],
            ..single("POWER.SURPRISE_POWER", PowerType::Buff)
        },
        PowerDefinition {
            instance_type: PowerInstanceType::Instanced,
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterDeath,
                    vec![Command::Rewards(
                        sts2_engine::RewardsCmd::AddStolenGoldToTheRoom(stacks()),
                    )],
                )
                .when(HookCondition::OwnerDied),
            ],
            ..counter("POWER.HEIST_POWER", PowerType::Buff)
        },
    ]
}

/// The powers the Underdocks' own monsters carry, and the one an Entropy puts
/// on its player.
#[must_use]
pub fn underdocks_powers() -> Vec<PowerDefinition> {
    let mut powers = gremlin_merc_powers();
    // An entropy turns as many cards of the hand into other cards as it is
    // worth, every turn, as its owner's turn opens.
    powers.push(PowerDefinition {
        hooks: vec![ModifierHook::every_turn(
            HookEvent::AfterPlayerTurnStart,
            vec![Command::CardSelect(CardSelectCmd::FromHand {
                semantics: ChoiceSemantics::known(vec![
                    ChoiceOp::new(ChoiceOperationKind::Transform, ChoiceScope::Combat).rolled(),
                ]),
                prefs: SelectorPrefs::exactly(0),
                maximum: None,
                exact_count: Some(stacks()),
                filter: CardFilter::Any,
                then: vec![Command::Card(CardCmd::TransformToRolled {
                    cards: CardSelector::Chosen,
                    stream: sts2_engine::TransformStream::CombatCardSelection,
                })],
            })],
        )],
        ..counter("POWER.ENTROPY_POWER", PowerType::Buff)
    });
    powers
}

#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one literal per power model the pool applies"
)]
pub fn regent_powers() -> Vec<PowerDefinition> {
    vec![
        // An arsenal pays its owner strength for every card conjured into the
        // fight by them.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardGeneratedForCombat,
                    vec![applied_to(
                        "POWER.STRENGTH_POWER",
                        CreatureSelector::Owner,
                        stacks(),
                    )],
                )
                .when(HookCondition::OwnerConjuredTheCard),
            ],
            ..counter("POWER.ARSENAL_POWER", PowerType::Buff)
        },
        // A black hole hurts the whole enemy side whenever its owner's stars
        // move, and the two halves are not the same site: a gain answers as it
        // lands, while a spend is answered off the play that paid, because
        // stars come off at the very start of a play and the build wants the
        // blow after it.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterStarsGained,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::CountAtLeast {
                            count: Count::AnnouncedAmount,
                            at_least: 1,
                        },
                        then: vec![sweeping_damage()],
                    }],
                ),
                ModifierHook::every_turn(
                    HookEvent::AfterCardPlayed,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::PlayedCardSpentStars,
                        then: vec![sweeping_damage()],
                    }],
                )
                .when(HookCondition::OwnerPlayedTheCard),
            ],
            ..counter("POWER.BLACK_HOLE_POWER", PowerType::Buff)
        },
        // A child of the stars works up block for every star its owner spends,
        // as much per star as it is worth, and nothing scales it.
        PowerDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterStarsSpent,
                vec![Command::Creature(CreatureCmd::GainBlock {
                    target: CreatureSelector::Owner,
                    amount: Amount::Product {
                        left: Box::new(stacks()),
                        right: Box::new(Amount::counting(Count::AnnouncedAmount)),
                    },
                    props: ValueProps::UNPOWERED,
                })],
            )],
            ..counter("POWER.CHILD_OF_THE_STARS_POWER", PowerType::Buff)
        },
        // A conqueror doubles what a Sovereign Blade deals to whoever is
        // carrying it, and counts itself down at the end of a turn its owner
        // took part in.
        PowerDefinition {
            hooks: vec![
                modifies(
                    HookEvent::ModifyDamageMultiplicative,
                    HookCondition::OwnerTakesPoweredDamage,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::DamageCameFromCard(id("CARD.SOVEREIGN_BLADE")),
                        then: vec![Command::Modify(ModifierCmd::Scale(Amount::Fixed(200)))],
                    }],
                ),
                ModifierHook::every_turn(HookEvent::AfterSideTurnEnd, vec![decrement()])
                    .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.CONQUEROR_POWER", PowerType::Debuff)
        },
        // The two strengths the Regent's cards take away for a turn: a crush
        // and a dying star, both wrappers that work the strength off as they
        // land and hand it back at the end of a turn their owner took part in.
        PowerDefinition {
            temporary: Some(TemporaryPower {
                internally_applied_power: id("POWER.STRENGTH_POWER"),
                positive: false,
            }),
            ..counter("POWER.CRUSH_UNDER_POWER", PowerType::Debuff)
        },
        PowerDefinition {
            temporary: Some(TemporaryPower {
                internally_applied_power: id("POWER.STRENGTH_POWER"),
                positive: false,
            }),
            ..counter("POWER.DYING_STAR_POWER", PowerType::Debuff)
        },
        PowerDefinition {
            temporary: Some(TemporaryPower {
                internally_applied_power: id("POWER.STRENGTH_POWER"),
                positive: false,
            }),
            ..counter("POWER.MONARCHS_GAZE_STRENGTH_DOWN_POWER", PowerType::Debuff)
        },
        // A foregone conclusion picks cards out of the draw pile before the
        // hand is dealt, and is spent for it.
        PowerDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::BeforeHandDraw,
                vec![
                    Command::CardPile(CardPileCmd::ShuffleIfNecessary),
                    Command::CardSelect(CardSelectCmd::FromCombatPile {
                        semantics: ChoiceSemantics::known(vec![
                            ChoiceOp::new(ChoiceOperationKind::Move, ChoiceScope::Combat)
                                .to(ChoiceDestination::Hand),
                        ]),
                        pile: PileName::Draw,
                        prefs: SelectorPrefs::exactly(0),
                        maximum: None,
                        exact_count: Some(stacks()),
                        restricted_to_chosen: false,
                        filter: CardFilter::Any,
                        then: vec![Command::CardPile(CardPileCmd::MoveToPile {
                            cards: CardSelector::Chosen,
                            pile: PileName::Hand,
                            position: DrawPilePosition::Bottom,
                        })],
                    }),
                    remove_self(),
                ],
            )],
            ..counter("POWER.FOREGONE_CONCLUSION_POWER", PowerType::Buff)
        },
        // A furnace forges as its owner's turn opens.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    vec![Command::Forge(sts2_engine::ForgeCmd::Forge(stacks()))],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.FURNACE_POWER", PowerType::Buff)
        },
        // A genesis pays stars off the energy refill, which is where the build
        // hangs everything a turn owes its owner.
        PowerDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterEnergyReset,
                vec![Command::Player(PlayerCmd::GainStars(stacks()))],
            )],
            ..counter("POWER.GENESIS_POWER", PowerType::Buff)
        },
        // A hammer time mirrors every forge onto the other players. No
        // single-player run has one, so nothing it declares ever answers.
        single("POWER.HAMMER_TIME_POWER", PowerType::Buff),
        // A monarch's gaze takes strength off whatever its owner's powered
        // blows land on, for the turn.
        PowerDefinition {
            hooks: vec![modifies(
                HookEvent::AfterDamageGiven,
                HookCondition::OwnerDealsPoweredDamage,
                vec![applied_to(
                    "POWER.MONARCHS_GAZE_STRENGTH_DOWN_POWER",
                    CreatureSelector::DamageTarget,
                    stacks(),
                )],
            )],
            ..counter("POWER.MONARCHS_GAZE_POWER", PowerType::Buff)
        },
        // A monologue pays its owner strength for every card they play while it
        // stands, and gives the whole of it back at the end of a turn they took
        // part in. Each play is worth what the power was worth as that play
        // began, which is written down as the play opens and read back as it
        // closes; the running total is kept on the power itself, because the
        // strength it hands back is not its amount.
        PowerDefinition {
            instance_type: PowerInstanceType::Instanced,
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardPlayed,
                    // A play the power was not standing for has no entry to
                    // take out, and nothing at all is paid for it. The play
                    // that put the power there is the one that has none,
                    // because the entry is written as the play opens and the
                    // power only lands once that play is under way.
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningPowerAmountBeforeThisPlayAtLeast(1),
                        then: vec![
                            applied_to(
                                "POWER.STRENGTH_POWER",
                                CreatureSelector::Owner,
                                Amount::counting(Count::ListeningPowerAmountBeforeThisPlay),
                            ),
                            Command::Power(PowerCmd::ModifyCounter {
                                power: PowerRef::listening(),
                                amount: Amount::counting(Count::ListeningPowerAmount),
                            }),
                        ],
                    }],
                )
                .when(HookCondition::OwnerPlayedTheCard),
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnEnd,
                    // The build takes the power off and then hands the whole
                    // tally back off the object it is still holding, so the
                    // amount is worked out before the removal rather than read
                    // back off a power that is no longer there.
                    vec![Command::WithAmount {
                        amount: Amount::Product {
                            left: Box::new(Amount::Fixed(-1)),
                            right: Box::new(Amount::counting(Count::ListeningPowerCounter)),
                        },
                        then: vec![
                            remove_self(),
                            applied_to(
                                "POWER.STRENGTH_POWER",
                                CreatureSelector::Owner,
                                Amount::counting(Count::Bound),
                            ),
                        ],
                    }],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.MONOLOGUE_POWER", PowerType::Buff)
        },
        // An orbit pays an energy for every four its owner spends, counted
        // across the whole fight rather than reset each turn.
        PowerDefinition {
            instance_type: PowerInstanceType::Instanced,
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterEnergySpent,
                    vec![
                        Command::Power(PowerCmd::ModifyCounter {
                            power: PowerRef::listening(),
                            amount: Amount::counting(Count::AnnouncedAmount),
                        }),
                        Command::When {
                            otherwise: Vec::new(),
                            condition: Condition::CountAtLeast {
                                count: Count::ListeningPowerCounter,
                                at_least: 4,
                            },
                            then: vec![
                                Command::Player(PlayerCmd::GainEnergy(Amount::Product {
                                    left: Box::new(stacks()),
                                    right: Box::new(Amount::Fixed(1)),
                                })),
                                Command::Power(PowerCmd::ModifyCounter {
                                    power: PowerRef::listening(),
                                    amount: Amount::Fixed(-4),
                                }),
                            ],
                        },
                    ],
                )
                .when(HookCondition::OwnerPlayedTheCard),
            ],
            ..counter("POWER.ORBIT_POWER", PowerType::Buff)
        },
        // A pale blue dot deals its owner more cards, but only on a turn that
        // follows one they played at least five on.
        PowerDefinition {
            hooks: vec![modifies(
                HookEvent::ModifyHandDraw,
                HookCondition::Always,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::CountAtLeast {
                        count: Count::OwnerCardPlaysLastTurn,
                        at_least: 5,
                    },
                    then: vec![Command::Modify(ModifierCmd::Add(stacks()))],
                }],
            )],
            ..counter("POWER.PALE_BLUE_DOT_POWER", PowerType::Buff)
        },
        // A parry has no body at all: the Sovereign Blade reads the amount off
        // its owner and works up that much block behind its own blow.
        counter("POWER.PARRY_POWER", PowerType::Buff),
        // A pillar of creation pays block for every card conjured into the
        // fight by its owner, and nothing scales it.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardGeneratedForCombat,
                    vec![Command::Creature(CreatureCmd::GainBlock {
                        target: CreatureSelector::Owner,
                        amount: stacks(),
                        props: ValueProps::UNPOWERED,
                    })],
                )
                .when(HookCondition::OwnerConjuredTheCard),
            ],
            ..counter("POWER.PILLAR_OF_CREATION_POWER", PowerType::Buff)
        },
        // A reflect deals back whatever its owner's block stopped of a powered
        // blow, and counts down as their turn opens.
        PowerDefinition {
            hooks: vec![
                modifies(
                    HookEvent::AfterDamageReceived,
                    HookCondition::OwnerTookPoweredAttackFromSomebody,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::CountAtLeast {
                            count: Count::BlockedDamageOfTheBlow,
                            at_least: 1,
                        },
                        then: vec![Command::Creature(CreatureCmd::Damage {
                            target: CreatureSelector::DamageDealer,
                            amount: Amount::counting(Count::BlockedDamageOfTheBlow),
                            props: ValueProps::UNPOWERED,
                        })],
                    }],
                ),
                ModifierHook::every_turn(HookEvent::AfterSideTurnStart, vec![decrement()])
                    .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.REFLECT_POWER", PowerType::Buff)
        },
        // Royalties put gold on the fight's reward set as the fight ends.
        PowerDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterCombatEnd,
                vec![Command::Rewards(sts2_engine::RewardsCmd::AddGoldToTheRoom(
                    stacks(),
                ))],
            )],
            ..counter("POWER.ROYALTIES_POWER", PowerType::Buff)
        },
        // A seeking edge has no body either: the blade reads it off its owner
        // and aims at the whole enemy side instead of one of them.
        single("POWER.SEEKING_EDGE_POWER", PowerType::Buff),
        // A spectrum shift deals its owner colorless cards before the hand is
        // dealt.
        PowerDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::BeforeHandDraw,
                vec![Command::CardPile(CardPileCmd::AddGeneratedFromPool {
                    pool: sts2_engine::CardPoolRef::Named(id("CARD_POOL.COLORLESS_CARD_POOL")),
                    count: stacks(),
                    distinct: true,
                    filter: CardFilter::Any,
                    pile: PileName::Hand,
                    position: DrawPilePosition::Bottom,
                    excludes_source: false,
                    upgraded: false,
                    temporary_cost: None,
                    ethereal: false,
                    rolled_one_at_a_time: false,
                })],
            )],
            ..counter("POWER.SPECTRUM_SHIFT_POWER", PowerType::Buff)
        },
        // A star next turn pays its stars off the energy refill and is spent
        // for it.
        PowerDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterEnergyReset,
                vec![
                    Command::Player(PlayerCmd::GainStars(stacks())),
                    remove_self(),
                ],
            )],
            ..counter("POWER.STAR_NEXT_TURN_POWER", PowerType::Buff)
        },
        // A sword sage gives every Sovereign Blade its owner holds another
        // play, and gives one to each blade that joins the fight later.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterPowerAmountChanged,
                    vec![Command::Card(CardCmd::ModifyReplayCount {
                        cards: CardSelector::CombatCardsOfModel(id("CARD.SOVEREIGN_BLADE")),
                        amount: Amount::counting(Count::AnnouncedAmount),
                    })],
                )
                .when(HookCondition::ListeningPowerAmountChanged),
                ModifierHook::every_turn(
                    HookEvent::AfterCardEnteredCombat,
                    vec![Command::Card(CardCmd::ModifyReplayCount {
                        cards: CardSelector::Announced,
                        amount: stacks(),
                    })],
                )
                .when(HookCondition::EnteredCardIs(id("CARD.SOVEREIGN_BLADE"))),
            ],
            ..counter("POWER.SWORD_SAGE_POWER", PowerType::Buff)
        },
        // A sealed throne pays its owner stars before every card they play.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::BeforeCardPlayed,
                    vec![Command::Player(PlayerCmd::GainStars(stacks()))],
                )
                .when(HookCondition::OwnerPlayedTheCard),
            ],
            ..counter("POWER.THE_SEALED_THRONE_POWER", PowerType::Buff)
        },
        // A tyranny deals its owner more cards and makes them burn as many as
        // it is worth once the hand is in front of them.
        PowerDefinition {
            hooks: vec![
                modifies(
                    HookEvent::ModifyHandDraw,
                    HookCondition::Always,
                    vec![Command::Modify(ModifierCmd::Add(stacks()))],
                ),
                ModifierHook::every_turn(
                    HookEvent::AfterPlayerTurnStart,
                    vec![Command::CardSelect(CardSelectCmd::FromHand {
                        semantics: ChoiceSemantics::known(vec![
                            ChoiceOp::new(ChoiceOperationKind::Exhaust, ChoiceScope::Combat)
                                .to(ChoiceDestination::Exhaust),
                        ]),
                        prefs: SelectorPrefs::exactly(0),
                        maximum: None,
                        exact_count: Some(stacks()),
                        filter: CardFilter::Any,
                        then: vec![Command::Card(CardCmd::Exhaust {
                            cards: CardSelector::Chosen,
                        })],
                    })],
                ),
            ],
            ..counter("POWER.TYRANNY_POWER", PowerType::Buff)
        },
        // A void form makes the first cards of every turn free, of energy and
        // of stars alike, and counts what its owner has played itself.
        PowerDefinition {
            card_cost_rule: Some(sts2_engine::CardCostRule::FreeInHandOrPlayUnderCount),
            ..counter("POWER.VOID_FORM_POWER", PowerType::Buff)
        },
    ]
}

#[allow(
    clippy::too_many_lines,
    reason = "one literal per power model the pool applies"
)]
pub fn necrobinder_powers() -> Vec<PowerDefinition> {
    vec![
        // What a summoned pet carries. It declares four things and runs no body
        // at all: a blow aimed at its owner lands on it instead while it is
        // standing and the blow is a powered attack; its owner's death is not
        // its own removal, so the corpse stays in the fight to be revived; the
        // power stays on that corpse, which is what makes the next summon a
        // revive rather than a fresh pet; and its amount is hidden, because
        // nothing ever stacks a second one.
        PowerDefinition {
            redirects_owners_powered_attacks: true,
            owner_removed_from_combat_on_death: false,
            holds_owner_where_it_stands: true,
            survives_owner_death: true,
            ..single("POWER.DIE_FOR_YOU_POWER", PowerType::Buff)
        },
        // Borrowed time: every card of its owner's costs its own amount more,
        // and it is gone at the end of the turn its owner spent.
        PowerDefinition {
            card_cost_rule: Some(sts2_engine::CardCostRule::CardsCostMore),
            hooks: vec![
                ModifierHook::every_turn(HookEvent::AfterSideTurnEnd, vec![remove_self()])
                    .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.BORROWED_TIME_POWER", PowerType::Debuff)
        },
        // Debilitate has no listener body of its own. Weak and Vulnerable both
        // ask whether it is standing while they work out their own multiplier
        // — Weak off whoever is swinging, Vulnerable off whoever is taking the
        // blow — and both are transcribed on those powers. What is left here is
        // the count and how it comes down: one at the end of every turn its
        // owner takes part in.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(HookEvent::AfterSideTurnEnd, vec![decrement()])
                    .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.DEBILITATE_POWER", PowerType::Debuff)
        },
        // A demesne is a card and an energy every turn, for as long as it
        // stands.
        PowerDefinition {
            hooks: vec![
                modifies(
                    HookEvent::ModifyHandDraw,
                    HookCondition::Always,
                    vec![Command::Modify(ModifierCmd::Add(stacks()))],
                ),
                modifies(
                    HookEvent::ModifyMaxEnergy,
                    HookCondition::Always,
                    vec![Command::Modify(ModifierCmd::Add(stacks()))],
                ),
            ],
            ..counter("POWER.DEMESNE_POWER", PowerType::Buff)
        },
        // A friendship is the energy half of a demesne on its own.
        PowerDefinition {
            hooks: vec![modifies(
                HookEvent::ModifyMaxEnergy,
                HookCondition::Always,
                vec![Command::Modify(ModifierCmd::Add(stacks()))],
            )],
            ..counter("POWER.FRIENDSHIP_POWER", PowerType::Buff)
        },
        // An enfeebling touch is strength lent the wrong way for the turn: the
        // wrapper works the strength onto its owner as it lands and gives it
        // back at the end of a turn its owner took part in.
        PowerDefinition {
            temporary: Some(sts2_engine::TemporaryPower {
                internally_applied_power: id("POWER.STRENGTH_POWER"),
                positive: false,
            }),
            ..counter("POWER.ENFEEBLING_TOUCH_POWER", PowerType::Debuff)
        },
        // A hang multiplies the next hang by what it has grown to: the
        // multiplier is the power's own amount rather than a percentage of it,
        // and it answers only for the card that put it there.
        PowerDefinition {
            hooks: vec![modifies(
                HookEvent::ModifyDamageMultiplicative,
                HookCondition::OwnerTakesDamage,
                vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::DamageCameFromCard(id("CARD.HANG")),
                    then: vec![Command::Modify(ModifierCmd::Scale(Amount::Product {
                        left: Box::new(Amount::Fixed(100)),
                        right: Box::new(stacks()),
                    }))],
                }],
            )],
            ..counter("POWER.HANG_POWER", PowerType::Debuff)
        },
        // A pagestorm draws another card every time an ethereal one is drawn,
        // which is the base pass rather than the early one.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardDrawn,
                    vec![Command::CardPile(CardPileCmd::Draw { count: stacks() })],
                )
                .when(HookCondition::OwnersEtherealCardDrawn),
            ],
            ..counter("POWER.PAGESTORM_POWER", PowerType::Buff)
        },
        // A calcify adds its own amount to every powered blow the owner's pet
        // lands.
        PowerDefinition {
            hooks: vec![modifies(
                HookEvent::ModifyDamageAdditive,
                HookCondition::OwnersPetDealsPoweredDamage,
                vec![Command::Modify(ModifierCmd::Add(stacks()))],
            )],
            ..counter("POWER.CALCIFY_POWER", PowerType::Buff)
        },
        // A sentry mode deals a Sweeping Gaze into the hand for every stack,
        // one at a time, before the turn's own hand is drawn.
        PowerDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::BeforeHandDraw,
                vec![Command::CardPile(CardPileCmd::AddGeneratedToCombat {
                    enchantment: None,
                    card: CardOrigin::Model(id("CARD.SWEEPING_GAZE")),
                    grown_by: Amount::Fixed(0),
                    count: stacks(),
                    pile: PileName::Hand,
                    ethereal: false,
                    retain: false,
                    upgraded: false,
                    temporary_cost: None,
                    position: DrawPilePosition::Bottom,
                })],
            )],
            ..counter("POWER.SENTRY_MODE_POWER", PowerType::Buff)
        },
        // Doom is a countdown on somebody else's life: as the side it is on
        // ends its turn, every creature there carrying at least as much of it
        // as it has HP left goes down, and the whole list goes down from the
        // first of them rather than one at a time.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::BeforeSideTurnEnd,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::All(vec![
                            Condition::OwnerIsAlive,
                            Condition::OwnerIsTheFirstDoomedOnItsSide,
                        ]),
                        then: vec![Command::Creature(CreatureCmd::DoomKill {
                            target: CreatureSelector::DoomedOnOwnersSide,
                        })],
                    }],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.DOOM_POWER", PowerType::Debuff)
        },
        // A countdown works its own amount of Doom onto one enemy, rolled on
        // the combat-targets stream, as its owner's side turn opens.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    vec![applied_to(
                        "POWER.DOOM_POWER",
                        CreatureSelector::RandomOpponent,
                        stacks(),
                    )],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.COUNTDOWN_POWER", PowerType::Buff)
        },
        // A neurosurge works its own amount of Doom onto its owner as their
        // side turn opens, which is the price of the energy the card paid out.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterSideTurnStart,
                    vec![applied_to(
                        "POWER.DOOM_POWER",
                        CreatureSelector::Owner,
                        stacks(),
                    )],
                )
                .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.NEUROSURGE_POWER", PowerType::Debuff)
        },
        // A shroud pays block every time its owner works Doom onto anything.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterPowerAmountChanged,
                    vec![Command::Creature(CreatureCmd::GainBlock {
                        target: CreatureSelector::Owner,
                        amount: stacks(),
                        props: ValueProps::UNPOWERED,
                    })],
                )
                .when(HookCondition::OwnerWorkedPower(id("POWER.DOOM_POWER"))),
            ],
            ..counter("POWER.SHROUD_POWER", PowerType::Buff)
        },
        // Devouring life summons every time its owner plays a Soul.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardPlayed,
                    vec![Command::Osty(sts2_engine::OstyCmd::Summon {
                        amount: stacks(),
                        owner: CreatureSelector::Owner,
                    })],
                )
                .when(HookCondition::All(vec![
                    HookCondition::OwnerPlayedTheCard,
                    HookCondition::PlayedCardIs(id("CARD.SOUL")),
                ])),
            ],
            ..counter("POWER.DEVOUR_LIFE_POWER", PowerType::Buff)
        },
        // A haunt cuts one enemy, rolled, every time its owner plays a Soul.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardPlayed,
                    vec![Command::Creature(CreatureCmd::Damage {
                        target: CreatureSelector::RandomOpponent,
                        amount: stacks(),
                        props: ValueProps::UNBLOCKABLE,
                    })],
                )
                .when(HookCondition::All(vec![
                    HookCondition::OwnerPlayedTheCard,
                    HookCondition::PlayedCardIs(id("CARD.SOUL")),
                ])),
            ],
            ..counter("POWER.HAUNT_POWER", PowerType::Buff)
        },
        // A danse macabre pays block before every card of its owner's that
        // costs two or more.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::BeforeCardPlayed,
                    vec![Command::Creature(CreatureCmd::GainBlock {
                        target: CreatureSelector::Owner,
                        amount: stacks(),
                        props: ValueProps::UNPOWERED,
                    })],
                )
                .when(HookCondition::All(vec![
                    HookCondition::OwnerPlayedTheCard,
                    HookCondition::PlayedCardEnergyAtLeast(2),
                ])),
            ],
            ..counter("POWER.DANSE_MACABRE_POWER", PowerType::Buff)
        },
        // A spirit of ash pays block before every ethereal card its owner
        // plays.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::BeforeCardPlayed,
                    vec![Command::Creature(CreatureCmd::GainBlock {
                        target: CreatureSelector::Owner,
                        amount: stacks(),
                        props: ValueProps::UNPOWERED,
                    })],
                )
                .when(HookCondition::All(vec![
                    HookCondition::OwnerPlayedTheCard,
                    HookCondition::PlayedCardIsEthereal,
                ])),
            ],
            ..counter("POWER.SPIRIT_OF_ASH_POWER", PowerType::Buff)
        },
        // A necro mastery answers every point of HP its owner's pet loses with
        // that many times what the mastery is worth, thrown at the whole enemy
        // side at once, unblockable and unscaled.
        //
        // None of that can be said here yet.
        // A necro mastery turns every point its owner's pet loses into that
        // many times its own amount, dealt unblockably to the whole enemy side
        // by its owner. The health-changed pass reaches every listener in the
        // fight and names the creature whose health moved and by how much, so
        // a mastery standing on the player answers for a blow that landed on
        // the pet.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCurrentHpChanged,
                    vec![Command::Creature(CreatureCmd::Damage {
                        target: CreatureSelector::AllOpponents,
                        amount: Amount::Product {
                            left: Box::new(Amount::counting(Count::HpTheAnnouncedCreatureLost)),
                            right: Box::new(stacks()),
                        },
                        props: ValueProps::UNBLOCKABLE,
                    })],
                )
                .when(HookCondition::OwnersPetLostHp),
            ],
            ..counter("POWER.NECRO_MASTERY_POWER", PowerType::Buff)
        },
        // A reaper form dooms whatever its owner or its owner's pet lands a
        // powered attack on, for the whole blow times what the form is worth.
        // The whole blow is counted — the half block stopped and the half that
        // got through — and a blow worth nothing dooms nothing.
        //
        // Who swung is said as an either-or the long way round, because the
        // surface carries `All` and `Not` and no `Any`.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterDamageGiven,
                    vec![Command::Power(PowerCmd::Apply {
                        marks: None,
                        power: id("POWER.DOOM_POWER"),
                        target: CreatureSelector::DamageTarget,
                        applier: Some(CreatureSelector::Owner),
                        amount: Amount::Product {
                            left: Box::new(Amount::counting(Count::TotalDamageOfTheBlow)),
                            right: Box::new(stacks()),
                        },
                    })],
                )
                .when(HookCondition::Not(Box::new(HookCondition::All(vec![
                    HookCondition::Not(Box::new(HookCondition::OwnerDealsPoweredDamage)),
                    HookCondition::Not(Box::new(HookCondition::OwnersPetDealsPoweredDamage)),
                ])))),
            ],
            ..counter("POWER.REAPER_FORM_POWER", PowerType::Buff)
        },
        // Sic 'Em marks one enemy: a blow the marking player's own pet lands on
        // it summons for what the mark is worth, and the mark is gone at the
        // end of a turn its holder took part in.
        //
        // The summon is for the player whose pet swung, which the guard has
        // just established is the mark's applier, rather
        // than for the creature the listener stands on, which is the enemy.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterDamageGiven,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::DamageDealerIsTheAppliersPet,
                        then: vec![Command::Osty(sts2_engine::OstyCmd::Summon {
                            amount: stacks(),
                            owner: CreatureSelector::Applier,
                        })],
                    }],
                )
                .when(HookCondition::OwnerTakesDamage),
                ModifierHook::every_turn(HookEvent::AfterSideTurnEnd, vec![remove_self()])
                    .when(HookCondition::OwnerIsParticipant),
            ],
            ..counter("POWER.SIC_EM_POWER", PowerType::Debuff)
        },
        // A summon owed for the next turn is paid as that turn opens, and the
        // power is spent for it.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterPlayerTurnStart,
                    vec![
                        Command::Osty(sts2_engine::OstyCmd::Summon {
                            amount: stacks(),
                            owner: CreatureSelector::Owner,
                        }),
                        remove_self(),
                    ],
                )
                .when(HookCondition::ListeningPowerStoodAtTurnStart),
            ],
            ..counter("POWER.SUMMON_NEXT_TURN_POWER", PowerType::Buff)
        },
        // A call of the void rolls a card out of its owner's own pool into
        // the hand before every draw and makes it ethereal. Each card is its
        // own separate roll of one, so the rolls are independent and the same
        // card can come up twice; the filter is the one every combat roll from
        // a pool applies.
        PowerDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::BeforeHandDraw,
                vec![Command::CardPile(CardPileCmd::AddGeneratedFromPool {
                    pool: sts2_engine::CardPoolRef::OwnCharacter,
                    count: stacks(),
                    distinct: true,
                    filter: sts2_engine::CardFilter::Any,
                    pile: sts2_engine::PileName::Hand,
                    position: sts2_engine::DrawPilePosition::Bottom,
                    excludes_source: false,
                    upgraded: false,
                    temporary_cost: None,
                    ethereal: true,
                    rolled_one_at_a_time: true,
                })],
            )],
            ..counter("POWER.CALL_OF_THE_VOID_POWER", PowerType::Buff)
        },
        // A forbidden grimoire puts a card removal on the room's own reward
        // set for every stack it carries, as the fight ends.
        PowerDefinition {
            hooks: vec![ModifierHook::every_turn(
                HookEvent::AfterCombatEnd,
                vec![Command::Repeat {
                    times: stacks(),
                    then: vec![Command::Rewards(
                        sts2_engine::RewardsCmd::AddCardRemovalToTheRoom,
                    )],
                }],
            )],
            ..counter("POWER.FORBIDDEN_GRIMOIRE_POWER", PowerType::Buff)
        },
        // A lethality raises the turn's first attack by its own percentage.
        // Which play that is is the whole of its guard: the damage came off a
        // card of its owner's, that card is not on a replay past its first, and
        // no other attack play has begun this turn.
        PowerDefinition {
            hooks: vec![modifies(
                HookEvent::ModifyDamageMultiplicative,
                HookCondition::DamageCardIsTheOwnersFirstAttackPlay,
                vec![Command::Modify(ModifierCmd::Scale(Amount::per(
                    100,
                    1,
                    Count::ListeningPowerAmount,
                )))],
            )],
            ..counter("POWER.LETHALITY_POWER", PowerType::Buff)
        },
        // An oblivion marks its owner and pays Doom for every card its applier
        // plays this turn. What it pays is what the mark was worth when that
        // play *began*, written down as the play opens and read back as it
        // closes — so the play that lands the mark pays nothing for itself. It
        // goes at the end of the player's turn.
        PowerDefinition {
            instance_type: PowerInstanceType::InstancedPerApplier,
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterCardPlayed,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::ListeningPowerAmountBeforeThisPlayAtLeast(0),
                        then: vec![Command::Power(PowerCmd::Apply {
                            marks: None,
                            power: id("POWER.DOOM_POWER"),
                            target: CreatureSelector::Owner,
                            applier: Some(CreatureSelector::Applier),
                            amount: Amount::counting(Count::ListeningPowerAmountBeforeThisPlay),
                        })],
                    }],
                )
                .when(HookCondition::ApplierPlayedTheCard),
                ModifierHook::every_turn(HookEvent::AfterSideTurnEnd, vec![remove_self()])
                    .when(HookCondition::PlayerSideTurn),
            ],
            ..counter("POWER.OBLIVION_POWER", PowerType::Debuff)
        },
        // A sleight of flesh cuts an enemy every time a lasting debuff its own
        // owner works lands on it. The blow is unpowered and comes from the
        // owner, and what it is worth is the power's own amount.
        PowerDefinition {
            hooks: vec![
                ModifierHook::every_turn(
                    HookEvent::AfterPowerAmountChanged,
                    vec![Command::Creature(CreatureCmd::Damage {
                        target: CreatureSelector::DamageTarget,
                        amount: stacks(),
                        props: ValueProps::UNPOWERED,
                    })],
                )
                .when(HookCondition::OwnerWorkedALastingDebuffOnAnEnemy),
            ],
            ..counter("POWER.SLEIGHT_OF_FLESH_POWER", PowerType::Buff)
        },
        // A veilpiercer makes its owner's ethereal cards free while they stand
        // in the hand or in play, and one stack is spent by the very play it
        // pays for — before that play's own cost is read again.
        PowerDefinition {
            card_cost_rule: Some(sts2_engine::CardCostRule::FreeEtherealInHandOrPlay),
            hooks: vec![
                ModifierHook::every_turn(HookEvent::BeforeCardPlayed, vec![decrement()]).when(
                    HookCondition::All(vec![
                        HookCondition::OwnerPlayedTheCard,
                        HookCondition::PlayedCardIsEthereal,
                        HookCondition::PlayedCardIsInHandOrPlay,
                    ]),
                ),
            ],
            ..counter("POWER.VEILPIERCER_POWER", PowerType::Buff)
        },
    ]
}

/// That many fresh Shivs, made and put into the hand together.
pub(crate) fn shivs_into_hand(count: Amount, upgraded: bool) -> Command {
    Command::CardPile(CardPileCmd::AddGeneratedToCombat {
        enchantment: None,
        card: CardOrigin::Model(id("CARD.SHIV")),
        grown_by: Amount::Fixed(0),
        count,
        pile: PileName::Hand,
        ethereal: false,
        retain: false,
        upgraded,
        temporary_cost: None,
        position: DrawPilePosition::Bottom,
    })
}
