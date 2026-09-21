//! Every monster of the pinned build, across all of its acts.
//!
//! Each definition mirrors the pinned build's move state machine: states, their
//! follow-ups, and the repeat rules a random branch applies to its roll log.
//! A move's body, and what the monster does as it joins the room, are runs of
//! commands over the shared surface.

use sts2_engine::{
    Amount, AscensionLevel, CardOrigin, CardPileCmd, CardRarity, CardSelectCmd, CardSelector,
    Command, Condition, ConditionalBranch, Count, CreatureCmd, CreatureSelector, DamageCmd,
    HookCondition, HookEvent, Intent, ModifierHook, MonsterDefinition, MonsterMove, MonsterState,
    MoveBranch, MoveCondition, MoveIntent, MoveRepeat, PileName, PowerCmd, PowerRef, SpawnSlot,
    StunFollowUp, ValueProps, value_if_ascension,
};

use crate::id;

/// A monster attack, aimed at everyone the monster is fighting.
fn strike(damage: i32, hits: i32) -> Command {
    Command::Damage(DamageCmd::Attack {
        amount: Amount::Fixed(damage),
        hits: Amount::Fixed(hits),
        target: CreatureSelector::AllOpponents,
        props: ValueProps::NONE,
    })
}

fn guard(amount: i32) -> Command {
    Command::Creature(CreatureCmd::GainBlock {
        target: CreatureSelector::Owner,
        amount: Amount::Fixed(amount),
        props: ValueProps::NONE,
    })
}

fn power_on(power: &str, target: CreatureSelector, amount: i32) -> Command {
    Command::Power(PowerCmd::Apply {
        marks: None,
        power: id(power),
        target,
        applier: Some(CreatureSelector::Owner),
        amount: Amount::Fixed(amount),
    })
}

/// A debuff worked on whoever the monster is fighting.
fn debuff(power: &str, amount: i32) -> Command {
    power_on(power, CreatureSelector::Target, amount)
}

/// A buff the monster works on itself.
fn self_power(power: &str, amount: i32) -> Command {
    power_on(power, CreatureSelector::Owner, amount)
}

/// The same for a stack count worked out where the monster is written rather
/// than declared as a literal.
fn self_power_amount(power: &str, amount: Amount) -> Command {
    Command::Power(PowerCmd::Apply {
        marks: None,
        power: id(power),
        target: CreatureSelector::Owner,
        applier: Some(CreatureSelector::Owner),
        amount,
    })
}

fn conjure(card: &str, count: i32, pile: PileName) -> Command {
    Command::CardPile(CardPileCmd::AddGeneratedToCombat {
        enchantment: None,
        card: CardOrigin::Model(id(card)),
        grown_by: Amount::Fixed(0),
        count: Amount::Fixed(count),
        pile,
        ethereal: false,
        retain: false,
        upgraded: false,
        temporary_cost: None,
        position: sts2_engine::DrawPilePosition::Bottom,
    })
}

fn attack(move_id: &str, damage: i32, follow_up: &str) -> MonsterState {
    multi_attack(move_id, damage, 1, follow_up)
}

fn multi_attack(move_id: &str, damage: i32, hits: i32, follow_up: &str) -> MonsterState {
    MonsterState::Move(MonsterMove {
        state_id: move_id.to_owned(),
        intent: Intent::new(
            move_id,
            vec![MoveIntent::attack(
                damage,
                u32::try_from(hits).expect("hit count is small"),
            )],
        ),
        commands: vec![strike(damage, hits)],
        follow_up: follow_up.to_owned(),
    })
}

/// A move that adds `count` Slimed cards to the player's discard pile.
fn slime_status(move_id: &str, count: i32, follow_up: &str) -> MonsterState {
    status_move(move_id, "CARD.SLIMED", count, follow_up)
}

/// A move that buries the player in status cards.
fn status_move(move_id: &str, card: &str, count: i32, follow_up: &str) -> MonsterState {
    MonsterState::Move(MonsterMove {
        state_id: move_id.to_owned(),
        intent: Intent::new(move_id, vec![MoveIntent::of("status_card")]),
        commands: vec![conjure(card, count, PileName::Discard)],
        follow_up: follow_up.to_owned(),
    })
}

/// A monster that puts a power on as it joins the room, which is what every
/// covered monster does on arrival.
fn joins_with(mut definition: MonsterDefinition, commands: Vec<Command>) -> MonsterDefinition {
    definition.on_added = commands;
    definition
}

/// An ascension branch bound to one run's ascension, which is how a monster
/// whose numbers branch is built.
fn branch_at(ascension: u8) -> impl Fn(AscensionLevel, i32, i32) -> i32 {
    move |level, ascension_value, fallback| {
        value_if_ascension(ascension, level, ascension_value, fallback)
    }
}

fn monster(
    model_id: &str,
    min_initial_hp: i32,
    max_initial_hp: i32,
    initial_state: &str,
    states: Vec<MonsterState>,
) -> MonsterDefinition {
    MonsterDefinition {
        model_id: id(model_id),
        min_initial_hp,
        max_initial_hp,
        initial_state: initial_state.to_owned(),
        states,
        on_added: Vec::new(),
        hooks: Vec::new(),
    }
}

fn shrinker_beetle_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    monster(
        "MONSTER.SHRINKER_BEETLE",
        asc(AscensionLevel::ToughEnemies, 40, 38),
        asc(AscensionLevel::ToughEnemies, 42, 40),
        "SHRINKER_MOVE",
        vec![
            MonsterState::Move(MonsterMove {
                state_id: "SHRINKER_MOVE".to_owned(),
                intent: Intent::new("SHRINKER_MOVE", vec![MoveIntent::of("debuff_strong")]),
                commands: vec![debuff("POWER.SHRINK_POWER", -1)],
                follow_up: "CHOMP_MOVE".to_owned(),
            }),
            attack(
                "CHOMP_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 8, 7),
                "STOMP_MOVE",
            ),
            attack(
                "STOMP_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 14, 13),
                "CHOMP_MOVE",
            ),
        ],
    )
}

/// The act's boss. Its followers are minions, so the fight ends with the priest.
fn kin_priest_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let hp = asc(AscensionLevel::ToughEnemies, 199, 190);
    monster(
        "MONSTER.KIN_PRIEST",
        hp,
        hp,
        "ORB_OF_FRAILTY_MOVE",
        vec![
            orb(
                "ORB_OF_FRAILTY_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 9, 8),
                "POWER.FRAIL_POWER",
                "ORB_OF_WEAKNESS_MOVE",
            ),
            orb(
                "ORB_OF_WEAKNESS_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 9, 8),
                "POWER.WEAK_POWER",
                "BEAM_MOVE",
            ),
            multi_attack(
                "BEAM_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 3, 3),
                3,
                "RITUAL_MOVE",
            ),
            self_buff(
                "RITUAL_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 3, 2),
                "ORB_OF_FRAILTY_MOVE",
            ),
        ],
    )
}

/// An orb both hits and leaves something behind.
fn orb(state_id: &str, damage: i32, power: &str, follow_up: &str) -> MonsterState {
    MonsterState::Move(MonsterMove {
        state_id: state_id.to_owned(),
        intent: Intent::new(
            state_id,
            vec![MoveIntent::attack(damage, 1), MoveIntent::of("debuff")],
        ),
        commands: vec![strike(damage, 1), debuff(power, 1)],
        follow_up: follow_up.to_owned(),
    })
}

/// A move whose only effect is making the mover stronger.
fn self_buff(state_id: &str, amount: i32, follow_up: &str) -> MonsterState {
    MonsterState::Move(MonsterMove {
        state_id: state_id.to_owned(),
        intent: Intent::new(state_id, vec![MoveIntent::of("buff")]),
        commands: vec![self_power("POWER.STRENGTH_POWER", amount)],
        follow_up: follow_up.to_owned(),
    })
}

/// A buried cube. It comes up with an artifact on, charges twice over, and
/// blasts, growing stronger with every blast it fires.
///
/// Its arrival also asks for thirteen block, and gets none: block gained
/// before the fight has been announced is dropped, and the fight is announced
/// only once every creature has joined. The block is written down here as the
/// build asks for it, and the engine drops it the way the build does.
fn cubex_construct_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let blast = asc(AscensionLevel::DeadlyEnemies, 8, 7);
    let expel = asc(AscensionLevel::DeadlyEnemies, 6, 5);
    let hp = asc(AscensionLevel::ToughEnemies, 70, 65);
    // A blast that hits and then makes the next one worth more.
    let repeater = |state_id: &str, follow_up: &str| {
        MonsterState::Move(MonsterMove {
            state_id: state_id.to_owned(),
            intent: Intent::new(
                state_id,
                vec![MoveIntent::attack(blast, 1), MoveIntent::of("buff")],
            ),
            commands: vec![strike(blast, 1), self_power("POWER.STRENGTH_POWER", 2)],
            follow_up: follow_up.to_owned(),
        })
    };
    joins_with(
        monster(
            "MONSTER.CUBEX_CONSTRUCT",
            hp,
            hp,
            "CHARGE_UP_MOVE",
            vec![
                MonsterState::Move(MonsterMove {
                    state_id: "CHARGE_UP_MOVE".to_owned(),
                    intent: Intent::new("CHARGE_UP_MOVE", vec![MoveIntent::of("buff")]),
                    commands: vec![self_power("POWER.STRENGTH_POWER", 2)],
                    follow_up: "REPEATER_BLAST_MOVE".to_owned(),
                }),
                repeater("REPEATER_BLAST_MOVE", "REPEATER_BLAST_MOVE_2"),
                repeater("REPEATER_BLAST_MOVE_2", "EXPEL_MOVE"),
                MonsterState::Move(MonsterMove {
                    state_id: "EXPEL_MOVE".to_owned(),
                    intent: Intent::new("EXPEL_MOVE", vec![MoveIntent::attack(expel, 2)]),
                    commands: vec![strike(expel, 2)],
                    follow_up: "REPEATER_BLAST_MOVE".to_owned(),
                }),
            ],
        ),
        vec![guard(13), self_power("POWER.ARTIFACT_POWER", 1)],
    )
}

/// The priest's guard. The encounter decides which one opens with the dance.
fn kin_follower_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    joins_with(
        monster(
            "MONSTER.KIN_FOLLOWER",
            asc(AscensionLevel::ToughEnemies, 62, 58),
            asc(AscensionLevel::ToughEnemies, 63, 59),
            "QUICK_SLASH_MOVE",
            vec![
                attack(
                    "QUICK_SLASH_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 5, 5),
                    "BOOMERANG_MOVE",
                ),
                multi_attack(
                    "BOOMERANG_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 2, 2),
                    2,
                    "POWER_DANCE_MOVE",
                ),
                self_buff(
                    "POWER_DANCE_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 3, 2),
                    "QUICK_SLASH_MOVE",
                ),
            ],
        ),
        vec![self_power("POWER.MINION_POWER", 1)],
    )
}

/// The Fogmog opens by conjuring an illusion, then alternates between swiping
/// itself stronger and heavier attacks.
fn fogmog_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let hp = asc(AscensionLevel::ToughEnemies, 78, 74);
    let swipe_damage = asc(AscensionLevel::DeadlyEnemies, 9, 8);
    monster(
        "MONSTER.FOGMOG",
        hp,
        hp,
        "ILLUSION_MOVE",
        vec![
            MonsterState::Move(MonsterMove {
                state_id: "ILLUSION_MOVE".to_owned(),
                intent: Intent::new("ILLUSION_MOVE", vec![MoveIntent::of("summon")]),
                commands: vec![Command::Creature(CreatureCmd::Add {
                    initial_state: None,
                    then: Vec::new(),
                    monster: id("MONSTER.EYE_WITH_TEETH"),
                    // The illusion arrives in the encounter's first slot, so
                    // the enemy side is reordered with it in front of the
                    // Fogmog. That order is the order the side takes its turns
                    // in and the order a random target is rolled over.
                    slot: sts2_engine::SpawnSlot::Named("illusion".to_owned()),
                })],
                follow_up: "SWIPE_MOVE".to_owned(),
            }),
            swipe("SWIPE_MOVE", swipe_damage, "BRANCH"),
            swipe("SWIPE_RANDOM_MOVE", swipe_damage, "HEADBUTT_MOVE"),
            attack(
                "HEADBUTT_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 16, 14),
                "SWIPE_MOVE",
            ),
            MonsterState::RandomBranch {
                state_id: "BRANCH".to_owned(),
                branches: vec![
                    MoveBranch::new("SWIPE_RANDOM_MOVE", MoveRepeat::CannotRepeat).with_weight(0.4),
                    MoveBranch::new("HEADBUTT_MOVE", MoveRepeat::CannotRepeat).with_weight(0.6),
                ],
            },
        ],
    )
}

/// A swipe both hits and feeds the swiper.
fn swipe(state_id: &str, damage: i32, follow_up: &str) -> MonsterState {
    MonsterState::Move(MonsterMove {
        state_id: state_id.to_owned(),
        intent: Intent::new(
            state_id,
            vec![MoveIntent::attack(damage, 1), MoveIntent::of("buff")],
        ),
        commands: vec![strike(damage, 1), self_power("POWER.STRENGTH_POWER", 1)],
        follow_up: follow_up.to_owned(),
    })
}

/// The Fogmog's illusion. It carries the two powers a summoned illusion has and
/// buries the player in Dazed.
#[must_use]
pub fn eye_with_teeth() -> MonsterDefinition {
    joins_with(
        monster(
            "MONSTER.EYE_WITH_TEETH",
            6,
            6,
            "DISTRACT_MOVE",
            vec![status_move(
                "DISTRACT_MOVE",
                "CARD.DAZED",
                3,
                "DISTRACT_MOVE",
            )],
        ),
        // Illusion brings the minion marker with it.
        vec![self_power("POWER.ILLUSION_POWER", 1)],
    )
}

/// An Inklet slips the first hit that lands on it. The encounter decides which
/// of its two openers each one uses.
fn inklet_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    joins_with(
        monster(
            "MONSTER.INKLET",
            asc(AscensionLevel::ToughEnemies, 12, 11),
            asc(AscensionLevel::ToughEnemies, 18, 17),
            "JAB_MOVE",
            vec![
                attack("JAB_MOVE", asc(AscensionLevel::DeadlyEnemies, 4, 3), "RAND"),
                multi_attack(
                    "WHIRLWIND_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 3, 2),
                    3,
                    "JAB_MOVE",
                ),
                attack(
                    "PIERCING_GAZE_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 11, 10),
                    "JAB_MOVE",
                ),
                MonsterState::RandomBranch {
                    state_id: "RAND".to_owned(),
                    branches: vec![
                        MoveBranch::new("PIERCING_GAZE_MOVE", MoveRepeat::CannotRepeat),
                        MoveBranch::new("WHIRLWIND_MOVE", MoveRepeat::CannotRepeat),
                    ],
                },
            ],
        ),
        vec![self_power("POWER.SLIPPERY_POWER", 1)],
    )
}

/// The elite Byrdonis carries Territorial into the fight, which grows its
/// Strength every time its side finishes a turn.
fn byrdonis_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    joins_with(
        monster(
            "MONSTER.BYRDONIS",
            asc(AscensionLevel::ToughEnemies, 90, 81),
            asc(AscensionLevel::ToughEnemies, 90, 84),
            "SWOOP_MOVE",
            vec![
                attack(
                    "SWOOP_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 19, 17),
                    "PECK_MOVE",
                ),
                multi_attack(
                    "PECK_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 4, 3),
                    asc(AscensionLevel::DeadlyEnemies, 3, 3),
                    "SWOOP_MOVE",
                ),
            ],
        ),
        vec![self_power("POWER.TERRITORIAL_POWER", 1)],
    )
}

/// The Flyconid opens on a branch that cannot pick its debuff-only move, and
/// its spore moves sit on cooldowns afterward.
fn flyconid_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let spore = asc(AscensionLevel::DeadlyEnemies, 9, 8);
    monster(
        "MONSTER.FLYCONID",
        asc(AscensionLevel::ToughEnemies, 51, 47),
        asc(AscensionLevel::ToughEnemies, 53, 49),
        "INITIAL",
        vec![
            MonsterState::Move(MonsterMove {
                state_id: "VULNERABLE_SPORES_MOVE".to_owned(),
                intent: Intent::new("VULNERABLE_SPORES_MOVE", vec![MoveIntent::of("debuff")]),
                commands: vec![debuff("POWER.VULNERABLE_POWER", 2)],
                follow_up: "RAND".to_owned(),
            }),
            MonsterState::Move(MonsterMove {
                state_id: "FRAIL_SPORES_MOVE".to_owned(),
                intent: Intent::new(
                    "FRAIL_SPORES_MOVE",
                    vec![MoveIntent::attack(spore, 1), MoveIntent::of("debuff")],
                ),
                commands: vec![strike(spore, 1), debuff("POWER.FRAIL_POWER", 2)],
                follow_up: "RAND".to_owned(),
            }),
            attack(
                "SMASH_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 12, 11),
                "RAND",
            ),
            MonsterState::RandomBranch {
                state_id: "RAND".to_owned(),
                branches: vec![
                    MoveBranch::new("VULNERABLE_SPORES_MOVE", MoveRepeat::CannotRepeat)
                        .with_cooldown(3),
                    MoveBranch::new("FRAIL_SPORES_MOVE", MoveRepeat::CannotRepeat).with_cooldown(2),
                    MoveBranch::new("SMASH_MOVE", MoveRepeat::CannotRepeat),
                ],
            },
            MonsterState::RandomBranch {
                state_id: "INITIAL".to_owned(),
                branches: vec![
                    MoveBranch::new("FRAIL_SPORES_MOVE", MoveRepeat::CannotRepeat).with_cooldown(2),
                    MoveBranch::new("SMASH_MOVE", MoveRepeat::CannotRepeat),
                ],
            },
        ],
    )
}

/// The Nibbit. Its opening state is a conditional branch on whether it stands
/// alone: alone, as the weak encounter spawns it, the branch resolves to Butt,
/// which is the opening here; paired, the encounter names each Nibbit's
/// opening on its spawn instead.
fn nibbit_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let slice = asc(AscensionLevel::DeadlyEnemies, 7, 6);
    monster(
        "MONSTER.NIBBIT",
        asc(AscensionLevel::ToughEnemies, 44, 42),
        asc(AscensionLevel::ToughEnemies, 48, 46),
        "BUTT_MOVE",
        vec![
            attack(
                "BUTT_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 13, 12),
                "SLICE_MOVE",
            ),
            MonsterState::Move(MonsterMove {
                state_id: "SLICE_MOVE".to_owned(),
                intent: Intent::new(
                    "SLICE_MOVE",
                    vec![MoveIntent::attack(slice, 1), MoveIntent::of("defend")],
                ),
                commands: vec![
                    strike(slice, 1),
                    guard(asc(AscensionLevel::ToughEnemies, 6, 5)),
                ],
                follow_up: "HISS_MOVE".to_owned(),
            }),
            MonsterState::Move(MonsterMove {
                state_id: "HISS_MOVE".to_owned(),
                intent: Intent::new("HISS_MOVE", vec![MoveIntent::of("buff")]),
                commands: vec![self_power(
                    "POWER.STRENGTH_POWER",
                    asc(AscensionLevel::DeadlyEnemies, 3, 2),
                )],
                follow_up: "BUTT_MOVE".to_owned(),
            }),
        ],
    )
}

fn twig_slime_s_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    monster(
        "MONSTER.TWIG_SLIME_S",
        asc(AscensionLevel::ToughEnemies, 8, 7),
        asc(AscensionLevel::ToughEnemies, 12, 11),
        "TACKLE_MOVE",
        vec![attack(
            "TACKLE_MOVE",
            asc(AscensionLevel::DeadlyEnemies, 5, 4),
            "TACKLE_MOVE",
        )],
    )
}

fn leaf_slime_s_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    monster(
        "MONSTER.LEAF_SLIME_S",
        asc(AscensionLevel::ToughEnemies, 12, 11),
        asc(AscensionLevel::ToughEnemies, 16, 15),
        "RAND",
        vec![
            attack(
                "TACKLE_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 4, 3),
                "RAND",
            ),
            slime_status("GOOP_MOVE", 1, "RAND"),
            MonsterState::RandomBranch {
                state_id: "RAND".to_owned(),
                branches: vec![
                    MoveBranch::new("TACKLE_MOVE", MoveRepeat::CannotRepeat),
                    MoveBranch::new("GOOP_MOVE", MoveRepeat::CannotRepeat),
                ],
            },
        ],
    )
}

fn leaf_slime_m_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    monster(
        "MONSTER.LEAF_SLIME_M",
        asc(AscensionLevel::ToughEnemies, 33, 32),
        asc(AscensionLevel::ToughEnemies, 36, 35),
        "STICKY_SHOT",
        vec![
            attack(
                "CLUMP_SHOT",
                asc(AscensionLevel::DeadlyEnemies, 9, 8),
                "STICKY_SHOT",
            ),
            slime_status("STICKY_SHOT", 2, "CLUMP_SHOT"),
        ],
    )
}

fn twig_slime_m_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    monster(
        "MONSTER.TWIG_SLIME_M",
        asc(AscensionLevel::ToughEnemies, 27, 26),
        asc(AscensionLevel::ToughEnemies, 29, 28),
        "STICKY_SHOT_MOVE",
        vec![
            attack(
                "POKEY_POUNCE_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 12, 11),
                "RAND",
            ),
            slime_status("STICKY_SHOT_MOVE", 1, "RAND"),
            MonsterState::RandomBranch {
                state_id: "RAND".to_owned(),
                branches: vec![
                    MoveBranch::new("POKEY_POUNCE_MOVE", MoveRepeat::CanRepeatTimes(2)),
                    MoveBranch::new("STICKY_SHOT_MOVE", MoveRepeat::CannotRepeat),
                ],
            },
        ],
    )
}

/// The theft: one card the player owns out of what is drawn or discarded,
/// preferring what the deck can least afford to lose. It leaves both the combat
/// and the deck, and the power the thief gains for it carries it.
fn thievery() -> Command {
    Command::Select {
        cards: CardSelector::StolenFromDeck {
            piles: vec![PileName::Draw, PileName::Discard],
            priorities: vec![
                vec![CardRarity::Uncommon],
                vec![CardRarity::Common, CardRarity::Rare],
                vec![CardRarity::Basic],
            ],
        },
        then: vec![
            Command::CardPile(CardPileCmd::RemoveFromCombat {
                cards: CardSelector::Chosen,
            }),
            Command::CardPile(CardPileCmd::RemoveFromDeck {
                cards: CardSelector::Chosen,
            }),
            // The theft marks whoever it took the card from, which is who the
            // loot goes back to.
            Command::Power(PowerCmd::Apply {
                power: id("POWER.SWIPE_POWER"),
                target: CreatureSelector::Owner,
                applier: Some(CreatureSelector::Owner),
                amount: Amount::Fixed(1),
                marks: Some(CreatureSelector::AllOpponents),
            }),
            Command::Power(PowerCmd::HoldCard {
                power: PowerRef::owned(id("POWER.SWIPE_POWER")),
                cards: CardSelector::Chosen,
            }),
        ],
    }
}

/// The Hive's thief. It takes a card, flutters out of reach, and leaves once it
/// has had enough.
fn thieving_hopper_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let hp = asc(AscensionLevel::ToughEnemies, 84, 79);
    let theft = asc(AscensionLevel::DeadlyEnemies, 19, 17);
    joins_with(
        monster(
            "MONSTER.THIEVING_HOPPER",
            hp,
            hp,
            "THIEVERY_MOVE",
            vec![
                MonsterState::Move(MonsterMove {
                    state_id: "THIEVERY_MOVE".to_owned(),
                    intent: Intent::new(
                        "THIEVERY_MOVE",
                        vec![MoveIntent::attack(theft, 1), MoveIntent::of("card_debuff")],
                    ),
                    commands: vec![thievery(), strike(theft, 1)],
                    follow_up: "FLUTTER_MOVE".to_owned(),
                }),
                MonsterState::Move(MonsterMove {
                    state_id: "FLUTTER_MOVE".to_owned(),
                    intent: Intent::new("FLUTTER_MOVE", vec![MoveIntent::of("buff")]),
                    commands: vec![self_power("POWER.FLUTTER_POWER", 5)],
                    follow_up: "HAT_TRICK_MOVE".to_owned(),
                }),
                attack(
                    "HAT_TRICK_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 23, 21),
                    "NAB_MOVE",
                ),
                attack(
                    "NAB_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 16, 14),
                    "ESCAPE_MOVE",
                ),
                // Once it has had enough it leaves, and it goes on leaving:
                // the escape's follow-up is itself.
                MonsterState::Move(MonsterMove {
                    state_id: "ESCAPE_MOVE".to_owned(),
                    intent: Intent::new("ESCAPE_MOVE", vec![MoveIntent::of("escape")]),
                    commands: vec![Command::Creature(CreatureCmd::Escape {
                        target: CreatureSelector::Owner,
                    })],
                    follow_up: "ESCAPE_MOVE".to_owned(),
                }),
            ],
        ),
        vec![self_power("POWER.ESCAPE_ARTIST_POWER", 5)],
    )
}

/// The Hive's sleeper. It snores behind a shell until enough has got through to
/// wake it, and then rolls at whoever woke it, harder every time.
fn slumbering_beetle_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let hp = asc(AscensionLevel::ToughEnemies, 89, 86);
    let rollout = asc(AscensionLevel::DeadlyEnemies, 18, 16);
    let plating = asc(AscensionLevel::ToughEnemies, 18, 15);
    joins_with(
        monster(
            "MONSTER.SLUMBERING_BEETLE",
            hp,
            hp,
            "SNORE_MOVE",
            vec![
                // The snore does nothing at all; the branch after it is what
                // asks whether the sleep has run out.
                MonsterState::Move(MonsterMove {
                    state_id: "SNORE_MOVE".to_owned(),
                    intent: Intent::new("SNORE_MOVE", vec![MoveIntent::of("sleep")]),
                    commands: Vec::new(),
                    follow_up: "SNORE_NEXT".to_owned(),
                }),
                MonsterState::ConditionalBranch {
                    state_id: "SNORE_NEXT".to_owned(),
                    branches: vec![
                        ConditionalBranch::new(
                            "SNORE_MOVE",
                            MoveCondition::OwnerHoldsPower(id("POWER.SLUMBER_POWER")),
                        ),
                        ConditionalBranch::new(
                            "ROLL_OUT_MOVE",
                            MoveCondition::Not(Box::new(MoveCondition::OwnerHoldsPower(id(
                                "POWER.SLUMBER_POWER",
                            )))),
                        ),
                    ],
                },
                MonsterState::Move(MonsterMove {
                    state_id: "ROLL_OUT_MOVE".to_owned(),
                    intent: Intent::new(
                        "ROLL_OUT_MOVE",
                        vec![MoveIntent::attack(rollout, 1), MoveIntent::of("buff")],
                    ),
                    commands: vec![strike(rollout, 1), self_power("POWER.STRENGTH_POWER", 2)],
                    follow_up: "ROLL_OUT_MOVE".to_owned(),
                }),
            ],
        ),
        vec![
            self_power("POWER.PLATING_POWER", plating),
            self_power("POWER.SLUMBER_POWER", 3),
        ],
    )
}

/// One of the Hive's roaches. Which move it opens on is its place in the line.
fn exoskeleton_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    joins_with(
        monster(
            "MONSTER.EXOSKELETON",
            asc(AscensionLevel::ToughEnemies, 25, 24),
            asc(AscensionLevel::ToughEnemies, 29, 28),
            "SKITTER_MOVE",
            vec![
                multi_attack(
                    "SKITTER_MOVE",
                    1,
                    asc(AscensionLevel::DeadlyEnemies, 4, 3),
                    "RAND",
                ),
                attack(
                    "MANDIBLES_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 9, 8),
                    "ENRAGE_MOVE",
                ),
                self_buff("ENRAGE_MOVE", 2, "RAND"),
                MonsterState::RandomBranch {
                    state_id: "RAND".to_owned(),
                    branches: vec![
                        MoveBranch::new("SKITTER_MOVE", MoveRepeat::CannotRepeat),
                        MoveBranch::new("MANDIBLES_MOVE", MoveRepeat::CannotRepeat),
                    ],
                },
            ],
        ),
        vec![self_power("POWER.HARD_TO_KILL_POWER", 9)],
    )
}

/// The Test Subject, the act's second boss. It dies twice and comes back
/// bigger each time: its first form bites and bashes, its second claws harder
/// every turn, and its third works round a triple cut, a pounce, and a growl
/// that buries the deck in burns.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "three forms of one creature, which is the machine this is"
)]
fn test_subject_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let first_form = asc(AscensionLevel::ToughEnemies, 111, 100);
    let claw = asc(AscensionLevel::DeadlyEnemies, 11, 10);
    let bash = asc(AscensionLevel::DeadlyEnemies, 16, 14);
    let revives = |times, hp, then: Vec<Command>| Command::When {
        condition: Condition::OwnerEnteredStateTimes {
            state_id: "RESPAWN_MOVE".to_owned(),
            times,
        },
        then: {
            let mut body = vec![Command::Creature(CreatureCmd::SetMaxAndCurrentHp {
                target: CreatureSelector::Owner,
                amount: sts2_engine::RolledAmount::Fixed(hp),
            })];
            body.extend(then);
            body
        },
        otherwise: Vec::new(),
    };
    joins_with(
        monster(
            "MONSTER.TEST_SUBJECT",
            first_form,
            first_form,
            "BITE_MOVE",
            vec![
                // `Respawns++` and then the form it comes back as. The state
                // log is the count: standing on this move for the nth time is
                // the nth respawn.
                MonsterState::Move(MonsterMove {
                    state_id: "RESPAWN_MOVE".to_owned(),
                    intent: Intent::new(
                        "RESPAWN_MOVE",
                        vec![MoveIntent::of("heal"), MoveIntent::of("buff")],
                    ),
                    commands: vec![
                        revives(
                            1,
                            asc(AscensionLevel::ToughEnemies, 212, 200),
                            vec![self_power("POWER.PAINFUL_STABS_POWER", 1)],
                        ),
                        revives(
                            2,
                            asc(AscensionLevel::ToughEnemies, 313, 300),
                            vec![
                                self_power("POWER.NEMESIS_POWER", 1),
                                Command::Power(PowerCmd::Remove {
                                    power: PowerRef::owned(id("POWER.ADAPTABLE_POWER")),
                                }),
                                Command::Power(PowerCmd::Remove {
                                    power: PowerRef::owned(id("POWER.PAINFUL_STABS_POWER")),
                                }),
                            ],
                        ),
                    ],
                    follow_up: "REVIVE_BRANCH".to_owned(),
                }),
                attack(
                    "BITE_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 22, 20),
                    "SKULL_BASH_MOVE",
                ),
                MonsterState::Move(MonsterMove {
                    state_id: "SKULL_BASH_MOVE".to_owned(),
                    intent: Intent::new(
                        "SKULL_BASH_MOVE",
                        vec![MoveIntent::attack(bash, 1), MoveIntent::of("debuff")],
                    ),
                    commands: vec![strike(bash, 1), debuff("POWER.VULNERABLE_POWER", 1)],
                    follow_up: "BITE_MOVE".to_owned(),
                }),
                // The move counts itself up at the end: one more claw every
                // time it comes round again.
                MonsterState::Move(MonsterMove {
                    state_id: "MULTI_CLAW_MOVE".to_owned(),
                    intent: Intent::new(
                        "MULTI_CLAW_MOVE",
                        vec![MoveIntent::attack_growing(claw, 3, "MULTI_CLAW_MOVE")],
                    ),
                    commands: vec![Command::Damage(DamageCmd::Attack {
                        amount: Amount::Fixed(claw),
                        hits: Amount::Scaled {
                            base: 2,
                            per: 1,
                            count: sts2_engine::Count::TimesOwnerEnteredState(
                                "MULTI_CLAW_MOVE".to_owned(),
                            ),
                        },
                        target: CreatureSelector::AllOpponents,
                        props: ValueProps::NONE,
                    })],
                    follow_up: "MULTI_CLAW_MOVE".to_owned(),
                }),
                multi_attack(
                    "PHASE3_LACERATE_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 11, 10),
                    3,
                    "BIG_POUNCE",
                ),
                attack("BIG_POUNCE", 45, "BURNING_GROWL_MOVE"),
                MonsterState::Move(MonsterMove {
                    state_id: "BURNING_GROWL_MOVE".to_owned(),
                    intent: Intent::new(
                        "BURNING_GROWL_MOVE",
                        vec![MoveIntent::of("status_card"), MoveIntent::of("buff")],
                    ),
                    commands: vec![
                        conjure(
                            "CARD.BURN",
                            asc(AscensionLevel::DeadlyEnemies, 5, 3),
                            PileName::Discard,
                        ),
                        self_power(
                            "POWER.STRENGTH_POWER",
                            asc(AscensionLevel::DeadlyEnemies, 3, 2),
                        ),
                    ],
                    follow_up: "PHASE3_LACERATE_MOVE".to_owned(),
                }),
                MonsterState::ConditionalBranch {
                    state_id: "REVIVE_BRANCH".to_owned(),
                    branches: vec![
                        ConditionalBranch {
                            state_id: "MULTI_CLAW_MOVE".to_owned(),
                            condition: MoveCondition::Not(Box::new(
                                MoveCondition::EnteredStateAtLeast {
                                    state_id: "RESPAWN_MOVE".to_owned(),
                                    times: 2,
                                },
                            )),
                        },
                        ConditionalBranch {
                            state_id: "PHASE3_LACERATE_MOVE".to_owned(),
                            condition: MoveCondition::EnteredStateAtLeast {
                                state_id: "RESPAWN_MOVE".to_owned(),
                                times: 2,
                            },
                        },
                    ],
                },
            ],
        ),
        vec![
            self_power("POWER.ADAPTABLE_POWER", 1),
            self_power(
                "POWER.ENRAGE_POWER",
                asc(AscensionLevel::DeadlyEnemies, 3, 2),
            ),
        ],
    )
}

/// One of the Fabricator's little bots, which all share a size.
fn bot(
    ascension: u8,
    model_id: &str,
    initial_state: &str,
    states: Vec<MonsterState>,
) -> MonsterDefinition {
    let asc = branch_at(ascension);
    monster(
        model_id,
        asc(AscensionLevel::ToughEnemies, 19, 18),
        asc(AscensionLevel::ToughEnemies, 24, 23),
        initial_state,
        states,
    )
}

/// A Zapbot. It gathers charge every turn and fires the same shot forever.
fn zapbot_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    joins_with(
        bot(
            ascension,
            "MONSTER.ZAPBOT",
            "ZAP",
            vec![attack(
                "ZAP",
                asc(AscensionLevel::DeadlyEnemies, 15, 14),
                "ZAP",
            )],
        ),
        vec![self_power("POWER.HIGH_VOLTAGE_POWER", 2)],
    )
}

/// A Stabbot. One stab, and the frailty that comes with it.
fn stabbot_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let stab = asc(AscensionLevel::DeadlyEnemies, 12, 11);
    bot(
        ascension,
        "MONSTER.STABBOT",
        "STAB_MOVE",
        vec![MonsterState::Move(MonsterMove {
            state_id: "STAB_MOVE".to_owned(),
            intent: Intent::new(
                "STAB_MOVE",
                vec![MoveIntent::attack(stab, 1), MoveIntent::of("debuff")],
            ),
            commands: vec![strike(stab, 1), debuff("POWER.FRAIL_POWER", 1)],
            follow_up: "STAB_MOVE".to_owned(),
        })],
    )
}

/// A Guardbot. It puts fifteen block on every Fabricator it can see, and none
/// on itself.
fn guardbot_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    monster(
        "MONSTER.GUARDBOT",
        asc(AscensionLevel::ToughEnemies, 17, 16),
        asc(AscensionLevel::ToughEnemies, 21, 20),
        "GUARD_MOVE",
        vec![MonsterState::Move(MonsterMove {
            state_id: "GUARD_MOVE".to_owned(),
            intent: Intent::new("GUARD_MOVE", vec![MoveIntent::of("defend")]),
            commands: vec![Command::Creature(CreatureCmd::GainBlock {
                target: CreatureSelector::AlliesOfModel(id("MONSTER.FABRICATOR")),
                amount: Amount::Fixed(15),
                props: ValueProps::UNPOWERED,
            })],
            follow_up: "GUARD_MOVE".to_owned(),
        })],
    )
}

/// A Noisebot. Two Dazed a turn: one into the discard pile and one anywhere in
/// the draw pile.
fn noisebot_at(ascension: u8) -> MonsterDefinition {
    bot(
        ascension,
        "MONSTER.NOISEBOT",
        "NOISE_MOVE",
        vec![MonsterState::Move(MonsterMove {
            state_id: "NOISE_MOVE".to_owned(),
            intent: Intent::new("NOISE_MOVE", vec![MoveIntent::of("status_card")]),
            commands: vec![
                conjure("CARD.DAZED", 1, PileName::Discard),
                Command::CardPile(CardPileCmd::AddGeneratedToCombat {
                    enchantment: None,
                    card: CardOrigin::Model(id("CARD.DAZED")),
                    grown_by: Amount::Fixed(0),
                    count: Amount::Fixed(1),
                    pile: PileName::Draw,
                    ethereal: false,
                    retain: false,
                    upgraded: false,
                    temporary_cost: None,
                    position: sts2_engine::DrawPilePosition::Random,
                }),
            ],
            follow_up: "NOISE_MOVE".to_owned(),
        })],
    )
}

/// The Fabricator. It builds bots while it has room for them and cuts when it
/// does not, and every bot it makes is a minion of its own.
fn fabricator_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let strike_damage = asc(AscensionLevel::DeadlyEnemies, 21, 18);
    let spawn = |options: Vec<sts2_core::ModelId>| {
        Command::Creature(CreatureCmd::AddRolled {
            options,
            slot: sts2_engine::SpawnSlot::FirstFree,
            then: vec![Command::Power(PowerCmd::Apply {
                power: id("POWER.MINION_POWER"),
                target: CreatureSelector::Target,
                applier: Some(CreatureSelector::Owner),
                amount: Amount::Fixed(1),
                marks: None,
            })],
        })
    };
    let defensive = || spawn(vec![id("MONSTER.GUARDBOT"), id("MONSTER.NOISEBOT")]);
    let aggressive = || spawn(vec![id("MONSTER.ZAPBOT"), id("MONSTER.STABBOT")]);
    let hp = asc(AscensionLevel::ToughEnemies, 155, 150);
    monster(
        "MONSTER.FABRICATOR",
        hp,
        hp,
        "fabricateBranch",
        vec![
            MonsterState::Move(MonsterMove {
                state_id: "FABRICATE_MOVE".to_owned(),
                intent: Intent::new("FABRICATE_MOVE", vec![MoveIntent::of("summon")]),
                commands: vec![defensive(), aggressive()],
                follow_up: "fabricateBranch".to_owned(),
            }),
            MonsterState::Move(MonsterMove {
                state_id: "FABRICATING_STRIKE_MOVE".to_owned(),
                intent: Intent::new(
                    "FABRICATING_STRIKE_MOVE",
                    vec![
                        MoveIntent::attack(strike_damage, 1),
                        MoveIntent::of("summon"),
                    ],
                ),
                commands: vec![strike(strike_damage, 1), aggressive()],
                follow_up: "fabricateBranch".to_owned(),
            }),
            attack(
                "DISINTEGRATE_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 13, 11),
                "fabricateBranch",
            ),
            MonsterState::RandomBranch {
                state_id: "RAND".to_owned(),
                branches: vec![
                    MoveBranch::new("FABRICATE_MOVE", MoveRepeat::CanRepeatForever),
                    MoveBranch::new("FABRICATING_STRIKE_MOVE", MoveRepeat::CanRepeatForever),
                ],
            },
            MonsterState::ConditionalBranch {
                state_id: "fabricateBranch".to_owned(),
                branches: vec![
                    ConditionalBranch {
                        state_id: "RAND".to_owned(),
                        condition: MoveCondition::LivingTeammatesAtMost(3),
                    },
                    ConditionalBranch {
                        state_id: "DISINTEGRATE_MOVE".to_owned(),
                        condition: MoveCondition::Not(Box::new(
                            MoveCondition::LivingTeammatesAtMost(3),
                        )),
                    },
                ],
            },
        ],
    )
}

/// An Axebot. It swings and then works one-two, and what it is holding of
/// itself decides how it opens: the one the room stands up starts on its
/// uppercut, and every replacement boots up first, harder each time.
fn axebot_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let uppercut = asc(AscensionLevel::DeadlyEnemies, 14, 12);
    let one_two = asc(AscensionLevel::DeadlyEnemies, 10, 9);
    let strength_gain = asc(AscensionLevel::DeadlyEnemies, 4, 3);
    joins_with(
        monster(
            "MONSTER.AXEBOT",
            asc(AscensionLevel::ToughEnemies, 76, 70),
            asc(AscensionLevel::ToughEnemies, 86, 78),
            "HAMMER_UPPERCUT_MOVE",
            vec![
                MonsterState::Move(MonsterMove {
                    state_id: "BOOT_UP_MOVE".to_owned(),
                    intent: Intent::new(
                        "BOOT_UP_MOVE",
                        vec![MoveIntent::of("defend"), MoveIntent::of("buff")],
                    ),
                    commands: vec![
                        guard(asc(AscensionLevel::DeadlyEnemies, 15, 10)),
                        // Twice the boot-up strength, less one lot of it for
                        // every stock the bot is still holding of itself.
                        Command::Power(PowerCmd::Apply {
                            power: id("POWER.STRENGTH_POWER"),
                            target: CreatureSelector::Owner,
                            applier: Some(CreatureSelector::Owner),
                            amount: Amount::Scaled {
                                base: strength_gain * 2,
                                per: -strength_gain,
                                count: sts2_engine::Count::OwnerPower(id("POWER.STOCK_POWER")),
                            },
                            marks: None,
                        }),
                    ],
                    follow_up: "HAMMER_UPPERCUT_MOVE".to_owned(),
                }),
                MonsterState::Move(MonsterMove {
                    state_id: "HAMMER_UPPERCUT_MOVE".to_owned(),
                    intent: Intent::new(
                        "HAMMER_UPPERCUT_MOVE",
                        vec![MoveIntent::attack(uppercut, 1), MoveIntent::of("debuff")],
                    ),
                    commands: vec![
                        strike(uppercut, 1),
                        debuff("POWER.WEAK_POWER", 2),
                        debuff("POWER.FRAIL_POWER", 2),
                    ],
                    follow_up: "ONE_TWO_MOVE".to_owned(),
                }),
                multi_attack("ONE_TWO_MOVE", one_two, 2, "HAMMER_UPPERCUT_MOVE"),
            ],
        ),
        // On arrival, the bot the room stands up carries two of itself. A
        // replacement is built with a stock of its own, which is what its
        // opening move says, so this is the one that is not.
        vec![Command::When {
            otherwise: Vec::new(),
            condition: Condition::OwnerMoveStateIs("HAMMER_UPPERCUT_MOVE".to_owned()),
            then: vec![Command::Power(PowerCmd::Apply {
                power: id("POWER.STOCK_POWER"),
                target: CreatureSelector::Owner,
                applier: None,
                amount: Amount::Fixed(2),
                marks: None,
            })],
        }],
    )
}

/// The Lost. It takes two strength off whoever it is fighting and keeps it,
/// then fires, and the strength goes back when it dies.
fn the_lost_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let hp = asc(AscensionLevel::ToughEnemies, 99, 93);
    let steal = asc(AscensionLevel::DeadlyEnemies, 2, 2);
    joins_with(
        monster(
            "MONSTER.THE_LOST",
            hp,
            hp,
            "DEBILITATING_SMOG",
            vec![
                MonsterState::Move(MonsterMove {
                    state_id: "DEBILITATING_SMOG".to_owned(),
                    intent: Intent::new(
                        "DEBILITATING_SMOG",
                        vec![MoveIntent::of("debuff"), MoveIntent::of("buff")],
                    ),
                    commands: vec![
                        debuff("POWER.STRENGTH_POWER", -steal),
                        self_power("POWER.STRENGTH_POWER", steal),
                    ],
                    follow_up: "EYE_LASERS".to_owned(),
                }),
                multi_attack(
                    "EYE_LASERS",
                    asc(AscensionLevel::DeadlyEnemies, 5, 4),
                    2,
                    "DEBILITATING_SMOG",
                ),
            ],
        ),
        // Nobody is named as having worked the possession on, so nothing
        // takes it off by dying.
        vec![Command::Power(PowerCmd::Apply {
            power: id("POWER.POSSESS_STRENGTH_POWER"),
            target: CreatureSelector::Owner,
            applier: None,
            amount: Amount::Fixed(1),
            marks: None,
        })],
    )
}

/// The Forgotten. It takes two dexterity, guards behind eight, and then dreads
/// for more the more of it it is holding.
fn the_forgotten_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let hp = asc(AscensionLevel::ToughEnemies, 111, 106);
    let steal = asc(AscensionLevel::DeadlyEnemies, 2, 2);
    let dread = asc(AscensionLevel::DeadlyEnemies, 15, 13);
    joins_with(
        monster(
            "MONSTER.THE_FORGOTTEN",
            hp,
            hp,
            "MIASMA",
            vec![
                MonsterState::Move(MonsterMove {
                    state_id: "MIASMA".to_owned(),
                    intent: Intent::new(
                        "MIASMA",
                        vec![
                            MoveIntent::of("debuff"),
                            MoveIntent::of("defend"),
                            MoveIntent::of("buff"),
                        ],
                    ),
                    commands: vec![
                        debuff("POWER.DEXTERITY_POWER", -steal),
                        guard(8),
                        self_power("POWER.DEXTERITY_POWER", steal),
                    ],
                    follow_up: "DREAD".to_owned(),
                }),
                // The dread is worth its base plus whatever dexterity the
                // hound is holding, worked out afresh every time it is read.
                MonsterState::Move(MonsterMove {
                    state_id: "DREAD".to_owned(),
                    intent: Intent::new(
                        "DREAD",
                        vec![MoveIntent::attack_scaled(
                            dread,
                            id("POWER.DEXTERITY_POWER"),
                        )],
                    ),
                    commands: vec![Command::Damage(DamageCmd::Attack {
                        amount: Amount::Scaled {
                            base: dread,
                            per: 1,
                            count: sts2_engine::Count::OwnerPower(id("POWER.DEXTERITY_POWER")),
                        },
                        hits: Amount::Fixed(1),
                        target: CreatureSelector::AllOpponents,
                        props: ValueProps::NONE,
                    })],
                    follow_up: "MIASMA".to_owned(),
                }),
            ],
        ),
        vec![Command::Power(PowerCmd::Apply {
            power: id("POWER.POSSESS_SPEED_POWER"),
            target: CreatureSelector::Owner,
            applier: None,
            amount: Amount::Fixed(1),
            marks: None,
        })],
    )
}

/// The Insatiable. It liquifies the ground under the player once, and from
/// then on works round its thrash, its bite, and the salivation that makes
/// both worse.
///
/// The pit it opens is a power on itself that names the player and counts down
/// four enemy turns; when it runs out the player is eaten. Six Frantic Escapes
/// go into the piles with it — three into the draw pile and three into the
/// discard, each at a place rolled on the shuffle stream — and playing one
/// gives a turn back.
fn the_insatiable_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let thrash = asc(AscensionLevel::DeadlyEnemies, 9, 8);
    let hp = asc(AscensionLevel::ToughEnemies, 341, 321);
    let escape = |pile: PileName| {
        Command::CardPile(CardPileCmd::AddGeneratedToCombat {
            enchantment: None,
            card: CardOrigin::Model(id("CARD.FRANTIC_ESCAPE")),
            grown_by: Amount::Fixed(0),
            count: Amount::Fixed(3),
            pile,
            ethereal: false,
            retain: false,
            upgraded: false,
            temporary_cost: None,
            position: sts2_engine::DrawPilePosition::Random,
        })
    };
    monster(
        "MONSTER.THE_INSATIABLE",
        hp,
        hp,
        "LIQUIFY_GROUND_MOVE",
        vec![
            MonsterState::Move(MonsterMove {
                state_id: "LIQUIFY_GROUND_MOVE".to_owned(),
                intent: Intent::new(
                    "LIQUIFY_GROUND_MOVE",
                    vec![MoveIntent::of("buff"), MoveIntent::of("status_card")],
                ),
                commands: vec![
                    Command::Power(PowerCmd::Apply {
                        power: id("POWER.SANDPIT_POWER"),
                        target: CreatureSelector::Owner,
                        applier: Some(CreatureSelector::Owner),
                        amount: Amount::Fixed(4),
                        // The sandpit is marked with the player it opened
                        // under.
                        marks: Some(CreatureSelector::AllOpponents),
                    }),
                    escape(PileName::Draw),
                    escape(PileName::Discard),
                ],
                follow_up: "THRASH_MOVE".to_owned(),
            }),
            multi_attack("THRASH_MOVE", thrash, 2, "LUNGING_BITE_MOVE"),
            attack(
                "LUNGING_BITE_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 31, 28),
                "SALIVATE_MOVE",
            ),
            self_buff(
                "SALIVATE_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 3, 2),
                "THRASH_MOVE_2",
            ),
            multi_attack("THRASH_MOVE_2", thrash, 2, "THRASH_MOVE"),
        ],
    )
}

/// A Tough Egg. It sits through its hatch and then nibbles, over and over.
///
/// The hatch takes off everything worked on it but the minion mark, and then
/// sets its size afresh off the run's niche stream — the hatchling is a bigger
/// creature than the egg was.
fn tough_egg_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    joins_with(
        monster(
            "MONSTER.TOUGH_EGG",
            asc(AscensionLevel::ToughEnemies, 15, 14),
            asc(AscensionLevel::ToughEnemies, 19, 18),
            "HATCH_MOVE",
            vec![
                MonsterState::Move(MonsterMove {
                    state_id: "HATCH_MOVE".to_owned(),
                    intent: Intent::new("HATCH_MOVE", vec![MoveIntent::of("summon")]),
                    commands: vec![
                        Command::Power(PowerCmd::RemoveOwnersOtherPowers {
                            keep: vec![id("POWER.MINION_POWER")],
                        }),
                        Command::Creature(CreatureCmd::SetMaxAndCurrentHp {
                            target: CreatureSelector::Owner,
                            amount: sts2_engine::RolledAmount::OnNiche {
                                min: asc(AscensionLevel::ToughEnemies, 20, 19),
                                max: asc(AscensionLevel::ToughEnemies, 23, 22),
                            },
                        }),
                    ],
                    follow_up: "NIBBLE_MOVE".to_owned(),
                }),
                attack(
                    "NIBBLE_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 5, 4),
                    "NIBBLE_MOVE",
                ),
            ],
        ),
        // On arrival: an egg laid on the enemy's own turn has two ticks to
        // sit through and one laid on the player's turn has one.
        vec![Command::When {
            otherwise: Vec::new(),
            condition: Condition::EnemySideIsActing,
            then: vec![self_power("POWER.HATCH_POWER", 2)],
        }],
    )
}

/// An Ovicopter. It lays three eggs into the last free slots it can find,
/// smashes, tenderizes, and then lays again — or, with a full nest, feeds
/// itself instead.
fn ovicopter_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let smash = asc(AscensionLevel::DeadlyEnemies, 17, 16);
    let tenderizer = asc(AscensionLevel::DeadlyEnemies, 8, 7);
    let lay_one = || {
        Command::Creature(CreatureCmd::Add {
            monster: id("MONSTER.TOUGH_EGG"),
            slot: sts2_engine::SpawnSlot::LastFree,
            initial_state: None,
            then: vec![Command::Power(PowerCmd::Apply {
                marks: None,
                power: id("POWER.MINION_POWER"),
                target: CreatureSelector::Target,
                applier: Some(CreatureSelector::Owner),
                amount: Amount::Fixed(1),
            })],
        })
    };
    monster(
        "MONSTER.OVICOPTER",
        asc(AscensionLevel::ToughEnemies, 126, 124),
        asc(AscensionLevel::ToughEnemies, 132, 130),
        "LAY_EGGS_MOVE",
        vec![
            MonsterState::Move(MonsterMove {
                state_id: "LAY_EGGS_MOVE".to_owned(),
                intent: Intent::new("LAY_EGGS_MOVE", vec![MoveIntent::of("summon")]),
                commands: vec![lay_one(), lay_one(), lay_one()],
                follow_up: "SMASH_MOVE".to_owned(),
            }),
            attack("SMASH_MOVE", smash, "TENDERIZER_MOVE"),
            MonsterState::Move(MonsterMove {
                state_id: "TENDERIZER_MOVE".to_owned(),
                intent: Intent::new(
                    "TENDERIZER_MOVE",
                    vec![MoveIntent::attack(tenderizer, 1), MoveIntent::of("debuff")],
                ),
                commands: vec![strike(tenderizer, 1), debuff("POWER.VULNERABLE_POWER", 2)],
                follow_up: "SUMMON_BRANCH_STATE".to_owned(),
            }),
            MonsterState::Move(MonsterMove {
                state_id: "NUTRITIONAL_PASTE_MOVE".to_owned(),
                intent: Intent::new("NUTRITIONAL_PASTE_MOVE", vec![MoveIntent::of("buff")]),
                commands: vec![self_power(
                    "POWER.STRENGTH_POWER",
                    asc(AscensionLevel::DeadlyEnemies, 4, 3),
                )],
                follow_up: "SMASH_MOVE".to_owned(),
            }),
            MonsterState::ConditionalBranch {
                state_id: "SUMMON_BRANCH_STATE".to_owned(),
                branches: vec![
                    ConditionalBranch {
                        state_id: "LAY_EGGS_MOVE".to_owned(),
                        condition: MoveCondition::LivingTeammatesAtMost(3),
                    },
                    ConditionalBranch {
                        state_id: "NUTRITIONAL_PASTE_MOVE".to_owned(),
                        condition: MoveCondition::Not(Box::new(
                            MoveCondition::LivingTeammatesAtMost(3),
                        )),
                    },
                ],
            },
        ],
    )
}

/// A Wriggler. Where it stands decides what it opens with: the odd slots bite,
/// the even ones wriggle, and one spawned by a dying phrog spends its first
/// turn doing nothing at all.
fn wriggler_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    monster(
        "MONSTER.WRIGGLER",
        asc(AscensionLevel::ToughEnemies, 18, 17),
        asc(AscensionLevel::ToughEnemies, 22, 21),
        "INIT_MOVE",
        vec![
            MonsterState::Move(MonsterMove {
                state_id: "SPAWNED_MOVE".to_owned(),
                intent: Intent::new("SPAWNED_MOVE", vec![MoveIntent::of("stun")]),
                commands: Vec::new(),
                follow_up: "INIT_MOVE".to_owned(),
            }),
            attack(
                "NASTY_BITE_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 7, 6),
                "WRIGGLE_MOVE",
            ),
            MonsterState::Move(MonsterMove {
                state_id: "WRIGGLE_MOVE".to_owned(),
                intent: Intent::new(
                    "WRIGGLE_MOVE",
                    vec![MoveIntent::of("buff"), MoveIntent::of("status_card")],
                ),
                commands: vec![
                    conjure("CARD.INFECTION", 1, PileName::Discard),
                    self_power("POWER.STRENGTH_POWER", 2),
                ],
                follow_up: "NASTY_BITE_MOVE".to_owned(),
            }),
            // The opener branches on which named slot the wriggler stands in,
            // in registration order: wriggler1 and wriggler3 bite, wriggler2
            // and wriggler4
            // wriggle. Which slot index each name is depends on the encounter,
            // and both encounters that stand wrigglers up lay the four names
            // out in the same order.
            MonsterState::ConditionalBranch {
                state_id: "INIT_MOVE".to_owned(),
                branches: vec![
                    ConditionalBranch {
                        state_id: "NASTY_BITE_MOVE".to_owned(),
                        condition: MoveCondition::Any(vec![
                            MoveCondition::OwnerIsInSlotNamed("wriggler1".to_owned()),
                            MoveCondition::OwnerIsInSlotNamed("wriggler3".to_owned()),
                        ]),
                    },
                    ConditionalBranch {
                        state_id: "WRIGGLE_MOVE".to_owned(),
                        condition: MoveCondition::Any(vec![
                            MoveCondition::OwnerIsInSlotNamed("wriggler2".to_owned()),
                            MoveCondition::OwnerIsInSlotNamed("wriggler4".to_owned()),
                        ]),
                    },
                ],
            },
        ],
    )
}

/// A Phrog Parasite. It rolls between infecting the deck and a fourfold lash,
/// and it carries the wrigglers it lets out when it dies.
fn phrog_parasite_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let lash = asc(AscensionLevel::DeadlyEnemies, 5, 4);
    joins_with(
        monster(
            "MONSTER.PHROG_PARASITE",
            asc(AscensionLevel::ToughEnemies, 66, 61),
            asc(AscensionLevel::ToughEnemies, 68, 64),
            "INFECT_MOVE",
            vec![
                MonsterState::Move(MonsterMove {
                    state_id: "INFECT_MOVE".to_owned(),
                    intent: Intent::new("INFECT_MOVE", vec![MoveIntent::of("status_card")]),
                    commands: vec![conjure("CARD.INFECTION", 3, PileName::Discard)],
                    follow_up: "LASH_MOVE".to_owned(),
                }),
                multi_attack("LASH_MOVE", lash, 4, "INFECT_MOVE"),
                MonsterState::RandomBranch {
                    state_id: "RAND".to_owned(),
                    branches: vec![
                        MoveBranch::new("INFECT_MOVE", MoveRepeat::CannotRepeat),
                        MoveBranch::new("LASH_MOVE", MoveRepeat::CannotRepeat),
                    ],
                },
            ],
        ),
        vec![self_power("POWER.INFESTED_POWER", 4)],
    )
}

/// A Slithering Strangler. It constricts first, then rolls between a thwack it
/// guards behind and a heavier lash, going back to the constriction each time.
fn slithering_strangler_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let thwack = asc(AscensionLevel::DeadlyEnemies, 8, 7);
    monster(
        "MONSTER.SLITHERING_STRANGLER",
        asc(AscensionLevel::ToughEnemies, 54, 53),
        asc(AscensionLevel::ToughEnemies, 56, 55),
        "CONSTRICT",
        vec![
            MonsterState::Move(MonsterMove {
                state_id: "CONSTRICT".to_owned(),
                intent: Intent::new("CONSTRICT", vec![MoveIntent::of("debuff")]),
                commands: vec![debuff("POWER.CONSTRICT_POWER", 3)],
                follow_up: "rand".to_owned(),
            }),
            MonsterState::Move(MonsterMove {
                state_id: "THWACK".to_owned(),
                intent: Intent::new(
                    "THWACK",
                    vec![MoveIntent::attack(thwack, 1), MoveIntent::of("defend")],
                ),
                commands: vec![strike(thwack, 1), guard(5)],
                follow_up: "CONSTRICT".to_owned(),
            }),
            attack(
                "LASH",
                asc(AscensionLevel::DeadlyEnemies, 13, 12),
                "CONSTRICT",
            ),
            MonsterState::RandomBranch {
                state_id: "rand".to_owned(),
                branches: vec![
                    MoveBranch::new("THWACK", MoveRepeat::CanRepeatForever),
                    MoveBranch::new("LASH", MoveRepeat::CanRepeatForever),
                ],
            },
        ],
    )
}

/// A Mawler. It opens on its claws and then works between a heavy rip, one
/// roar it only ever makes once, and those claws again.
fn mawler_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let hp = asc(AscensionLevel::ToughEnemies, 76, 72);
    monster(
        "MONSTER.MAWLER",
        hp,
        hp,
        "CLAW_MOVE",
        vec![
            attack(
                "RIP_AND_TEAR_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 16, 14),
                "RAND",
            ),
            MonsterState::Move(MonsterMove {
                state_id: "ROAR_MOVE".to_owned(),
                intent: Intent::new("ROAR_MOVE", vec![MoveIntent::of("debuff")]),
                commands: vec![debuff("POWER.VULNERABLE_POWER", 3)],
                follow_up: "RAND".to_owned(),
            }),
            multi_attack(
                "CLAW_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 5, 4),
                2,
                "RAND",
            ),
            MonsterState::RandomBranch {
                state_id: "RAND".to_owned(),
                branches: vec![
                    MoveBranch::new("RIP_AND_TEAR_MOVE", MoveRepeat::CannotRepeat),
                    MoveBranch::new("ROAR_MOVE", MoveRepeat::UseOnlyOnce),
                    MoveBranch::new("CLAW_MOVE", MoveRepeat::CannotRepeat),
                ],
            },
        ],
    )
}

/// The Hive's hunter. It tenderizes first, then works between a bite and a
/// triple puncture.
fn hunter_killer_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let hp = asc(AscensionLevel::ToughEnemies, 126, 121);
    monster(
        "MONSTER.HUNTER_KILLER",
        hp,
        hp,
        "TENDERIZING_GOOP_MOVE",
        vec![
            MonsterState::Move(MonsterMove {
                state_id: "TENDERIZING_GOOP_MOVE".to_owned(),
                intent: Intent::new("TENDERIZING_GOOP_MOVE", vec![MoveIntent::of("debuff")]),
                commands: vec![debuff("POWER.TENDER_POWER", 1)],
                follow_up: "RAND".to_owned(),
            }),
            attack(
                "BITE_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 19, 17),
                "RAND",
            ),
            multi_attack(
                "PUNCTURE_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 8, 7),
                3,
                "RAND",
            ),
            MonsterState::RandomBranch {
                state_id: "RAND".to_owned(),
                branches: vec![
                    MoveBranch::new("BITE_MOVE", MoveRepeat::CannotRepeat),
                    MoveBranch::new("PUNCTURE_MOVE", MoveRepeat::CanRepeatTimes(2)),
                ],
            },
        ],
    )
}

/// A Chomper. It clamps, then screeches the player's deck full of Dazed.
fn chomper_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    joins_with(
        monster(
            "MONSTER.CHOMPER",
            asc(AscensionLevel::ToughEnemies, 63, 60),
            asc(AscensionLevel::ToughEnemies, 67, 64),
            "CLAMP_MOVE",
            vec![
                multi_attack(
                    "CLAMP_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 9, 8),
                    2,
                    "SCREECH_MOVE",
                ),
                status_move("SCREECH_MOVE", "CARD.DAZED", 3, "CLAMP_MOVE"),
            ],
        ),
        vec![self_power("POWER.ARTIFACT_POWER", 2)],
    )
}

/// The rock bowlbug: one headbutt, over and over, until a blow of its own
/// bounces off block and leaves it dizzy.
fn bowlbug_rock_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    joins_with(
        monster(
            "MONSTER.BOWLBUG_ROCK",
            asc(AscensionLevel::ToughEnemies, 46, 45),
            asc(AscensionLevel::ToughEnemies, 49, 48),
            "HEADBUTT_MOVE",
            vec![MonsterState::Move(MonsterMove {
                state_id: "HEADBUTT_MOVE".to_owned(),
                intent: Intent::new(
                    "HEADBUTT_MOVE",
                    vec![MoveIntent::attack(
                        asc(AscensionLevel::DeadlyEnemies, 16, 15),
                        1,
                    )],
                ),
                commands: vec![
                    strike(asc(AscensionLevel::DeadlyEnemies, 16, 15), 1),
                    // The move ends by stunning itself if the blow it just
                    // made bounced off block.
                    Command::When {
                        otherwise: Vec::new(),
                        condition: Condition::OwnerIsOffBalance,
                        then: vec![Command::Creature(CreatureCmd::Stun {
                            target: CreatureSelector::Owner,
                            // The stun names the dizzy move but nothing to
                            // follow it, so the interrupted Headbutt is
                            // restored after the stunned turn.
                            next_move: StunFollowUp::TheLastMovePerformed,
                            stun_move: vec![Command::Creature(CreatureCmd::SetOffBalance {
                                target: CreatureSelector::Owner,
                                value: false,
                            })],
                        })],
                    },
                ],
                follow_up: "HEADBUTT_MOVE".to_owned(),
            })],
        ),
        vec![self_power("POWER.IMBALANCED_POWER", 1)],
    )
}

/// The silk bug, which spits weakness between thrashings.
fn bowlbug_silk_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    monster(
        "MONSTER.BOWLBUG_SILK",
        asc(AscensionLevel::ToughEnemies, 41, 40),
        asc(AscensionLevel::ToughEnemies, 44, 43),
        "TOXIC_SPIT_MOVE",
        vec![
            multi_attack(
                "THRASH_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 5, 4),
                2,
                "TOXIC_SPIT_MOVE",
            ),
            MonsterState::Move(MonsterMove {
                state_id: "TOXIC_SPIT_MOVE".to_owned(),
                intent: Intent::new("TOXIC_SPIT_MOVE", vec![MoveIntent::of("debuff")]),
                commands: vec![debuff("POWER.WEAK_POWER", 1)],
                follow_up: "THRASH_MOVE".to_owned(),
            }),
        ],
    )
}

/// The nectar bug, which works itself up to a huge blow.
fn bowlbug_nectar_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    monster(
        "MONSTER.BOWLBUG_NECTAR",
        asc(AscensionLevel::ToughEnemies, 36, 35),
        asc(AscensionLevel::ToughEnemies, 39, 38),
        "THRASH_MOVE",
        vec![
            attack("THRASH_MOVE", 3, "BUFF_MOVE"),
            self_buff(
                "BUFF_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 16, 15),
                "THRASH2_MOVE",
            ),
            attack("THRASH2_MOVE", 3, "THRASH2_MOVE"),
        ],
    )
}

/// A Myte. Which move it opens on is its place in the pair.
fn myte_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let suck = asc(AscensionLevel::DeadlyEnemies, 6, 4);
    monster(
        "MONSTER.MYTE",
        asc(AscensionLevel::ToughEnemies, 64, 61),
        asc(AscensionLevel::ToughEnemies, 69, 67),
        "TOXIC_MOVE",
        vec![
            MonsterState::Move(MonsterMove {
                state_id: "TOXIC_MOVE".to_owned(),
                intent: Intent::new("TOXIC_MOVE", vec![MoveIntent::of("status_card")]),
                commands: vec![conjure("CARD.TOXIC", 2, PileName::Hand)],
                follow_up: "BITE_MOVE".to_owned(),
            }),
            attack(
                "BITE_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 15, 13),
                "SUCK_MOVE",
            ),
            MonsterState::Move(MonsterMove {
                state_id: "SUCK_MOVE".to_owned(),
                intent: Intent::new(
                    "SUCK_MOVE",
                    vec![MoveIntent::attack(suck, 1), MoveIntent::of("buff")],
                ),
                commands: vec![
                    strike(suck, 1),
                    self_power(
                        "POWER.STRENGTH_POWER",
                        asc(AscensionLevel::DeadlyEnemies, 3, 2),
                    ),
                ],
                follow_up: "TOXIC_MOVE".to_owned(),
            }),
        ],
    )
}

/// The Kaiser Crab's left half, which works itself up and guards.
fn crusher_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let hp = asc(AscensionLevel::ToughEnemies, 219, 209);
    let sting = asc(AscensionLevel::DeadlyEnemies, 7, 6);
    let guarded = asc(AscensionLevel::DeadlyEnemies, 14, 12);
    joins_with(
        monster(
            "MONSTER.CRUSHER",
            hp,
            hp,
            "THRASH_MOVE",
            vec![
                attack(
                    "THRASH_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 14, 12),
                    "ENLARGING_STRIKE_MOVE",
                ),
                attack(
                    "ENLARGING_STRIKE_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 4, 4),
                    "BUG_STING_MOVE",
                ),
                MonsterState::Move(MonsterMove {
                    state_id: "BUG_STING_MOVE".to_owned(),
                    intent: Intent::new(
                        "BUG_STING_MOVE",
                        vec![MoveIntent::attack(sting, 2), MoveIntent::of("debuff")],
                    ),
                    commands: vec![
                        strike(sting, 2),
                        debuff("POWER.WEAK_POWER", 2),
                        debuff("POWER.FRAIL_POWER", 2),
                    ],
                    follow_up: "ADAPT_MOVE".to_owned(),
                }),
                self_buff(
                    "ADAPT_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 3, 2),
                    "GUARDED_STRIKE_MOVE",
                ),
                MonsterState::Move(MonsterMove {
                    state_id: "GUARDED_STRIKE_MOVE".to_owned(),
                    intent: Intent::new(
                        "GUARDED_STRIKE_MOVE",
                        vec![MoveIntent::attack(guarded, 1), MoveIntent::of("defend")],
                    ),
                    commands: vec![strike(guarded, 1), guard(18)],
                    follow_up: "THRASH_MOVE".to_owned(),
                }),
            ],
        ),
        vec![
            self_power("POWER.BACK_ATTACK_LEFT_POWER", 1),
            self_power("POWER.CRAB_RAGE_POWER", 1),
        ],
    )
}

/// The Kaiser Crab's right half, which charges up to a laser and recharges.
fn rocket_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let hp = asc(AscensionLevel::ToughEnemies, 209, 199);
    joins_with(
        monster(
            "MONSTER.ROCKET",
            hp,
            hp,
            "TARGETING_RETICLE_MOVE",
            vec![
                attack(
                    "TARGETING_RETICLE_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 4, 3),
                    "PRECISION_BEAM_MOVE",
                ),
                attack(
                    "PRECISION_BEAM_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 20, 18),
                    "CHARGE_UP_MOVE",
                ),
                self_buff(
                    "CHARGE_UP_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 3, 2),
                    "LASER_MOVE",
                ),
                attack(
                    "LASER_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 35, 31),
                    "RECHARGE_MOVE",
                ),
                MonsterState::Move(MonsterMove {
                    state_id: "RECHARGE_MOVE".to_owned(),
                    intent: Intent::new("RECHARGE_MOVE", vec![MoveIntent::of("sleep")]),
                    commands: Vec::new(),
                    follow_up: "TARGETING_RETICLE_MOVE".to_owned(),
                }),
            ],
        ),
        vec![
            // The rocket is what surrounds whoever it fights.
            debuff("POWER.SURROUNDED_POWER", 1),
            self_power("POWER.BACK_ATTACK_RIGHT_POWER", 1),
            self_power("POWER.CRAB_RAGE_POWER", 1),
        ],
    )
}

/// A crawler that spits acid, breathes in, and spits twice more. Inhaling is
/// what makes the second spit hurt: it is seven strength, kept for the rest of
/// the fight.
fn fuzzy_wurm_crawler_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let acid_goop = asc(AscensionLevel::DeadlyEnemies, 6, 4);
    monster(
        "MONSTER.FUZZY_WURM_CRAWLER",
        asc(AscensionLevel::ToughEnemies, 58, 55),
        asc(AscensionLevel::ToughEnemies, 59, 57),
        "FIRST_ACID_GOOP",
        vec![
            attack("FIRST_ACID_GOOP", acid_goop, "INHALE"),
            MonsterState::Move(MonsterMove {
                state_id: "INHALE".to_owned(),
                intent: Intent::new("INHALE", vec![MoveIntent::of("buff")]),
                commands: vec![self_power("POWER.STRENGTH_POWER", 7)],
                follow_up: "ACID_GOOP".to_owned(),
            }),
            attack("ACID_GOOP", acid_goop, "FIRST_ACID_GOOP"),
        ],
    )
}

/// An effigy that sleeps through the opening of the fight, wakes up ten
/// strength heavier, and slashes from then on. It carries its own slowness in.
fn bygone_effigy_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let slash = asc(AscensionLevel::DeadlyEnemies, 15, 13);
    let hp = asc(AscensionLevel::ToughEnemies, 132, 127);
    joins_with(
        monster(
            "MONSTER.BYGONE_EFFIGY",
            hp,
            hp,
            "SLEEP_MOVE",
            vec![
                sleeping("SLEEP_MOVE", "WAKE_MOVE"),
                MonsterState::Move(MonsterMove {
                    state_id: "WAKE_MOVE".to_owned(),
                    intent: Intent::new("WAKE_MOVE", vec![MoveIntent::of("buff")]),
                    commands: vec![self_power("POWER.STRENGTH_POWER", 10)],
                    follow_up: "SLASHES_MOVE".to_owned(),
                }),
                // The second sleep is a state the machine can be put into
                // rather than one it walks to on its own.
                sleeping("SLEEP_MOVE_2", "SLASHES_MOVE"),
                attack("SLASHES_MOVE", slash, "SLASHES_MOVE"),
            ],
        ),
        vec![self_power("POWER.SLOW_POWER", 1)],
    )
}

/// A move that does nothing at all, which is what an effigy asleep does.
fn sleeping(state_id: &str, follow_up: &str) -> MonsterState {
    MonsterState::Move(MonsterMove {
        state_id: state_id.to_owned(),
        intent: Intent::new(state_id, vec![MoveIntent::of("sleep")]),
        commands: Vec::new(),
        follow_up: follow_up.to_owned(),
    })
}

/// The act's boss. It charges up behind a plow that breaks when it is worn down
/// far enough, and fights a second way once it has been.
fn ceremonial_beast_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let hp = asc(AscensionLevel::ToughEnemies, 262, 252);
    let plow_amount = asc(AscensionLevel::DeadlyEnemies, 160, 150);
    let plow_damage = asc(AscensionLevel::DeadlyEnemies, 20, 18);
    let stomp_damage = asc(AscensionLevel::DeadlyEnemies, 17, 15);
    let crush_damage = asc(AscensionLevel::DeadlyEnemies, 19, 17);
    let crush_strength = asc(AscensionLevel::DeadlyEnemies, 4, 3);
    monster(
        "MONSTER.CEREMONIAL_BEAST",
        hp,
        hp,
        "STAMP_MOVE",
        vec![
            MonsterState::Move(MonsterMove {
                state_id: "STAMP_MOVE".to_owned(),
                intent: Intent::new("STAMP_MOVE", vec![MoveIntent::of("buff")]),
                commands: vec![self_power("POWER.PLOW_POWER", plow_amount)],
                follow_up: "PLOW_MOVE".to_owned(),
            }),
            // The charge repeats until the plow breaks.
            MonsterState::Move(MonsterMove {
                state_id: "PLOW_MOVE".to_owned(),
                intent: Intent::new(
                    "PLOW_MOVE",
                    vec![MoveIntent::attack(plow_damage, 1), MoveIntent::of("buff")],
                ),
                commands: vec![
                    strike(plow_damage, 1),
                    self_power("POWER.STRENGTH_POWER", 2),
                ],
                follow_up: "PLOW_MOVE".to_owned(),
            }),
            // Nothing walks into the beast's own stun state: the plow
            // breaking is what stuns it, and that stun names the cry as what
            // follows.
            MonsterState::Move(MonsterMove {
                state_id: "STUN_MOVE".to_owned(),
                intent: Intent::new("STUN_MOVE", vec![MoveIntent::of("stun")]),
                commands: Vec::new(),
                follow_up: "BEAST_CRY_MOVE".to_owned(),
            }),
            MonsterState::Move(MonsterMove {
                state_id: "BEAST_CRY_MOVE".to_owned(),
                intent: Intent::new("BEAST_CRY_MOVE", vec![MoveIntent::of("debuff")]),
                commands: vec![Command::Power(PowerCmd::Apply {
                    marks: None,
                    power: id("POWER.RINGING_POWER"),
                    target: CreatureSelector::AllOpponents,
                    applier: Some(CreatureSelector::Owner),
                    amount: Amount::Fixed(1),
                })],
                follow_up: "STOMP_MOVE".to_owned(),
            }),
            attack("STOMP_MOVE", stomp_damage, "CRUSH_MOVE"),
            MonsterState::Move(MonsterMove {
                state_id: "CRUSH_MOVE".to_owned(),
                intent: Intent::new(
                    "CRUSH_MOVE",
                    vec![MoveIntent::attack(crush_damage, 1), MoveIntent::of("buff")],
                ),
                commands: vec![
                    strike(crush_damage, 1),
                    self_power("POWER.STRENGTH_POWER", crush_strength),
                ],
                follow_up: "BEAST_CRY_MOVE".to_owned(),
            }),
        ],
    )
}

/// A tunneler bites, burrows behind a wall of block, and strikes from below
/// until the wall is broken.
fn tunneler_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let hp = asc(AscensionLevel::ToughEnemies, 92, 87);
    let bite = asc(AscensionLevel::DeadlyEnemies, 15, 13);
    let block = asc(AscensionLevel::ToughEnemies, 37, 32);
    let below = asc(AscensionLevel::DeadlyEnemies, 26, 23);
    monster(
        "MONSTER.TUNNELER",
        hp,
        hp,
        "BITE_MOVE",
        vec![
            attack("BITE_MOVE", bite, "BURROW_MOVE"),
            MonsterState::Move(MonsterMove {
                state_id: "BURROW_MOVE".to_owned(),
                intent: Intent::new(
                    "BURROW_MOVE",
                    vec![MoveIntent::of("buff"), MoveIntent::of("defend")],
                ),
                commands: vec![
                    self_power("POWER.BURROWED_POWER", 1),
                    Command::Creature(CreatureCmd::GainBlock {
                        target: CreatureSelector::Owner,
                        amount: Amount::Fixed(block),
                        props: ValueProps::NONE,
                    }),
                ],
                follow_up: "BELOW_MOVE".to_owned(),
            }),
            attack("BELOW_MOVE", below, "BELOW_MOVE"),
            // Nothing walks into the dizzy state: breaking the burrow is what
            // stuns the tunneler, and that names the bite as what follows.
            MonsterState::Move(MonsterMove {
                state_id: "DIZZY_MOVE".to_owned(),
                intent: Intent::new("DIZZY_MOVE", vec![MoveIntent::of("stun")]),
                commands: Vec::new(),
                follow_up: "BITE_MOVE".to_owned(),
            }),
        ],
    )
}

/// A toad that raises its spikes, blows them off, and lashes.
fn spiny_toad_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let lash = asc(AscensionLevel::DeadlyEnemies, 19, 17);
    let explosion = asc(AscensionLevel::DeadlyEnemies, 25, 23);
    monster(
        "MONSTER.SPINY_TOAD",
        asc(AscensionLevel::ToughEnemies, 121, 116),
        asc(AscensionLevel::ToughEnemies, 124, 119),
        "PROTRUDING_SPIKES_MOVE",
        vec![
            MonsterState::Move(MonsterMove {
                state_id: "PROTRUDING_SPIKES_MOVE".to_owned(),
                intent: Intent::new("PROTRUDING_SPIKES_MOVE", vec![MoveIntent::of("buff")]),
                commands: vec![self_power("POWER.THORNS_POWER", 5)],
                follow_up: "SPIKE_EXPLOSION_MOVE".to_owned(),
            }),
            // The explosion spends the spikes it threw.
            MonsterState::Move(MonsterMove {
                state_id: "SPIKE_EXPLOSION_MOVE".to_owned(),
                intent: Intent::new(
                    "SPIKE_EXPLOSION_MOVE",
                    vec![MoveIntent::attack(explosion, 1)],
                ),
                commands: vec![strike(explosion, 1), self_power("POWER.THORNS_POWER", -5)],
                follow_up: "TONGUE_LASH_MOVE".to_owned(),
            }),
            attack("TONGUE_LASH_MOVE", lash, "PROTRUDING_SPIKES_MOVE"),
        ],
    )
}

/// A sculptor that speaks its incantation once and then savages whoever is in
/// front of it, growing stronger every turn from the ritual it started.
fn devoted_sculptor_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let hp = asc(AscensionLevel::ToughEnemies, 172, 162);
    let savage = asc(AscensionLevel::DeadlyEnemies, 15, 12);
    monster(
        "MONSTER.DEVOTED_SCULPTOR",
        hp,
        hp,
        "FORBIDDEN_INCANTATION_MOVE",
        vec![
            MonsterState::Move(MonsterMove {
                state_id: "FORBIDDEN_INCANTATION_MOVE".to_owned(),
                intent: Intent::new("FORBIDDEN_INCANTATION_MOVE", vec![MoveIntent::of("buff")]),
                // The ritual is worked by nobody: the build passes a null
                // applier.
                commands: vec![Command::Power(PowerCmd::Apply {
                    marks: None,
                    power: id("POWER.RITUAL_POWER"),
                    target: CreatureSelector::Owner,
                    applier: None,
                    amount: Amount::Fixed(9),
                })],
                follow_up: "SAVAGE_MOVE".to_owned(),
            }),
            attack("SAVAGE_MOVE", savage, "SAVAGE_MOVE"),
        ],
    )
}

/// A scroll that bites, chews twice, and grows more teeth. Which move it opens
/// on is the encounter's to say.
fn scroll_of_biting_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let chomp = asc(AscensionLevel::DeadlyEnemies, 16, 14);
    let chew = asc(AscensionLevel::DeadlyEnemies, 6, 5);
    joins_with(
        monster(
            "MONSTER.SCROLL_OF_BITING",
            asc(AscensionLevel::ToughEnemies, 33, 30),
            asc(AscensionLevel::ToughEnemies, 39, 37),
            "CHOMP",
            vec![
                attack("CHOMP", chomp, "MORE_TEETH"),
                multi_attack("CHEW", chew, 2, "rand"),
                self_buff("MORE_TEETH", 2, "CHEW"),
                MonsterState::RandomBranch {
                    state_id: "rand".to_owned(),
                    branches: vec![
                        MoveBranch::new("CHOMP", MoveRepeat::CannotRepeat),
                        MoveBranch::new("CHEW", MoveRepeat::CanRepeatTimes(2)),
                    ],
                },
            ],
        ),
        vec![self_power("POWER.PAPER_CUTS_POWER", 2)],
    )
}

/// One segment of the Decimillipede. All three are the same machine; only the
/// model each is drawn as differs, and which move it opens on.
fn decimillipede_segment(ascension: u8, model_id: &str) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let writhe = asc(AscensionLevel::DeadlyEnemies, 6, 5);
    let constrict = asc(AscensionLevel::DeadlyEnemies, 9, 8);
    let bulk = asc(AscensionLevel::DeadlyEnemies, 7, 6);
    joins_with(
        monster(
            model_id,
            asc(AscensionLevel::ToughEnemies, 46, 40),
            asc(AscensionLevel::ToughEnemies, 52, 46),
            "WRITHE_MOVE",
            vec![
                multi_attack("WRITHE_MOVE", writhe, 2, "CONSTRICT_MOVE"),
                MonsterState::Move(MonsterMove {
                    state_id: "BULK_MOVE".to_owned(),
                    intent: Intent::new(
                        "BULK_MOVE",
                        vec![MoveIntent::attack(bulk, 1), MoveIntent::of("buff")],
                    ),
                    commands: vec![strike(bulk, 1), self_power("POWER.STRENGTH_POWER", 2)],
                    follow_up: "WRITHE_MOVE".to_owned(),
                }),
                MonsterState::Move(MonsterMove {
                    state_id: "CONSTRICT_MOVE".to_owned(),
                    intent: Intent::new(
                        "CONSTRICT_MOVE",
                        vec![MoveIntent::attack(constrict, 1), MoveIntent::of("debuff")],
                    ),
                    commands: vec![strike(constrict, 1), debuff("POWER.WEAK_POWER", 1)],
                    follow_up: "BULK_MOVE".to_owned(),
                }),
                // A segment that goes down holds still for a turn and then
                // pulls itself back on. The reattach heals what the power is
                // worth: the build asks first whether every other segment is
                // dead, and a segment cannot reach this move when they are,
                // because the fight would already be over.
                MonsterState::Move(MonsterMove {
                    state_id: "DEAD_MOVE".to_owned(),
                    intent: Intent::new("DEAD_MOVE", Vec::new()),
                    commands: Vec::new(),
                    follow_up: "REATTACH_MOVE".to_owned(),
                }),
                MonsterState::Move(MonsterMove {
                    state_id: "REATTACH_MOVE".to_owned(),
                    intent: Intent::new("REATTACH_MOVE", vec![MoveIntent::of("heal")]),
                    commands: vec![Command::Creature(CreatureCmd::Heal {
                        target: CreatureSelector::Owner,
                        amount: Amount::Fixed(25),
                    })],
                    follow_up: "RAND".to_owned(),
                }),
                MonsterState::RandomBranch {
                    state_id: "RAND".to_owned(),
                    branches: vec![
                        MoveBranch::new("WRITHE_MOVE", MoveRepeat::CannotRepeat),
                        MoveBranch::new("BULK_MOVE", MoveRepeat::CannotRepeat),
                        MoveBranch::new("CONSTRICT_MOVE", MoveRepeat::CannotRepeat),
                    ],
                },
            ],
        ),
        vec![
            Command::Creature(CreatureCmd::SetEvenUniqueMaxHp),
            self_power("POWER.REATTACH_POWER", 25),
        ],
    )
}

fn decimillipede_segment_front_at(ascension: u8) -> MonsterDefinition {
    decimillipede_segment(ascension, "MONSTER.DECIMILLIPEDE_SEGMENT_FRONT")
}

fn decimillipede_segment_middle_at(ascension: u8) -> MonsterDefinition {
    decimillipede_segment(ascension, "MONSTER.DECIMILLIPEDE_SEGMENT_MIDDLE")
}

fn decimillipede_segment_back_at(ascension: u8) -> MonsterDefinition {
    decimillipede_segment(ascension, "MONSTER.DECIMILLIPEDE_SEGMENT_BACK")
}

/// Glory's knight. It opens on the tongue, works its way round to the buff, and
/// then asks once — and only once — whether it is hurt enough to charge.
fn frog_knight_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    joins_with(
        monster(
            "MONSTER.FROG_KNIGHT",
            asc(AscensionLevel::ToughEnemies, 199, 191),
            asc(AscensionLevel::ToughEnemies, 199, 191),
            "TONGUE_LASH",
            vec![
                self_buff("FOR_THE_QUEEN", 5, "HALF_HEALTH"),
                attack(
                    "STRIKE_DOWN_EVIL",
                    asc(AscensionLevel::DeadlyEnemies, 23, 21),
                    "FOR_THE_QUEEN",
                ),
                MonsterState::Move(MonsterMove {
                    state_id: "TONGUE_LASH".to_owned(),
                    intent: Intent::new(
                        "TONGUE_LASH",
                        vec![
                            MoveIntent::attack(asc(AscensionLevel::DeadlyEnemies, 14, 13), 1),
                            MoveIntent::of("debuff"),
                        ],
                    ),
                    commands: vec![
                        strike(asc(AscensionLevel::DeadlyEnemies, 14, 13), 1),
                        debuff("POWER.FRAIL_POWER", 2),
                    ],
                    follow_up: "STRIKE_DOWN_EVIL".to_owned(),
                }),
                attack(
                    "BEETLE_CHARGE",
                    asc(AscensionLevel::DeadlyEnemies, 40, 35),
                    "TONGUE_LASH",
                ),
                // Whether the beetle has already charged is a mark the charge
                // leaves on itself, so the state log answers for it.
                MonsterState::ConditionalBranch {
                    state_id: "HALF_HEALTH".to_owned(),
                    branches: vec![
                        ConditionalBranch::new(
                            "TONGUE_LASH",
                            MoveCondition::Any(vec![
                                MoveCondition::HasEnteredState("BEETLE_CHARGE".to_owned()),
                                MoveCondition::Not(Box::new(
                                    MoveCondition::CurrentHpBelowHalfOfMax,
                                )),
                            ]),
                        ),
                        ConditionalBranch::new(
                            "BEETLE_CHARGE",
                            MoveCondition::All(vec![
                                MoveCondition::Not(Box::new(MoveCondition::HasEnteredState(
                                    "BEETLE_CHARGE".to_owned(),
                                ))),
                                MoveCondition::CurrentHpBelowHalfOfMax,
                            ]),
                        ),
                    ],
                },
            ],
        ),
        vec![self_power(
            "POWER.PLATING_POWER",
            asc(AscensionLevel::ToughEnemies, 19, 15),
        )],
    )
}

/// The flail knight: it opens on the ram and then rolls between chanting
/// itself stronger and its two attacks.
fn flail_knight_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    monster(
        "MONSTER.FLAIL_KNIGHT",
        asc(AscensionLevel::ToughEnemies, 108, 101),
        asc(AscensionLevel::ToughEnemies, 108, 101),
        "RAM_MOVE",
        vec![
            self_buff("WAR_CHANT", 3, "RAND"),
            multi_attack(
                "FLAIL_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 10, 9),
                2,
                "RAND",
            ),
            attack(
                "RAM_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 17, 15),
                "RAND",
            ),
            MonsterState::RandomBranch {
                state_id: "RAND".to_owned(),
                branches: vec![
                    MoveBranch::new("WAR_CHANT", MoveRepeat::CannotRepeat),
                    MoveBranch::new("FLAIL_MOVE", MoveRepeat::CanRepeatTimes(2)),
                    MoveBranch::new("RAM_MOVE", MoveRepeat::CanRepeatTimes(2)),
                ],
            },
        ],
    )
}

/// The entomancer: bees, a spear, and a spit that grows the hive it stands up
/// with until it is holding three of it, and works strength in either way.
fn entomancer_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let hp = asc(AscensionLevel::ToughEnemies, 155, 145);
    joins_with(
        monster(
            "MONSTER.ENTOMANCER",
            hp,
            hp,
            "BEES_MOVE",
            vec![
                MonsterState::Move(MonsterMove {
                    state_id: "PHEROMONE_SPIT_MOVE".to_owned(),
                    intent: Intent::new("PHEROMONE_SPIT_MOVE", vec![MoveIntent::of("buff")]),
                    commands: vec![Command::When {
                        condition: Condition::OwnerPowerBelow {
                            power: id("POWER.PERSONAL_HIVE_POWER"),
                            amount: 3,
                        },
                        then: vec![
                            self_power("POWER.PERSONAL_HIVE_POWER", 1),
                            self_power("POWER.STRENGTH_POWER", 1),
                        ],
                        otherwise: vec![self_power("POWER.STRENGTH_POWER", 2)],
                    }],
                    follow_up: "BEES_MOVE".to_owned(),
                }),
                multi_attack(
                    "BEES_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 3, 3),
                    asc(AscensionLevel::DeadlyEnemies, 8, 7),
                    "SPEAR_MOVE",
                ),
                attack(
                    "SPEAR_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 20, 18),
                    "PHEROMONE_SPIT_MOVE",
                ),
            ],
        ),
        vec![self_power("POWER.PERSONAL_HIVE_POWER", 1)],
    )
}

/// Aeonglass, the queen of the last act: a blow it puts its own wall up behind,
/// two lasers, and an intensity that grows every wither in the fight, makes
/// another, and works more strength in each time than the time before.
///
/// It stands up with a withering presence about each of its opponents and three
/// artifact.
fn aeonglass_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let hp = asc(AscensionLevel::ToughEnemies, 535, 512);
    let ebb = asc(AscensionLevel::DeadlyEnemies, 32, 26);
    let lasers = asc(AscensionLevel::DeadlyEnemies, 12, 11);
    let wither_amount = asc(AscensionLevel::DeadlyEnemies, 2, 1);
    joins_with(
        monster(
            "MONSTER.AEONGLASS",
            hp,
            hp,
            "EBB_MOVE",
            vec![
                MonsterState::Move(MonsterMove {
                    state_id: "EBB_MOVE".to_owned(),
                    intent: Intent::new(
                        "EBB_MOVE",
                        vec![MoveIntent::attack(ebb, 1), MoveIntent::of("defend")],
                    ),
                    commands: vec![
                        strike(ebb, 1),
                        // A flat thirty-three block.
                        Command::Creature(CreatureCmd::GainBlock {
                            target: CreatureSelector::Owner,
                            amount: Amount::Fixed(33),
                            props: ValueProps::NONE,
                        }),
                    ],
                    follow_up: "EYE_LASERS_MOVE".to_owned(),
                }),
                multi_attack("EYE_LASERS_MOVE", lasers, 2, "INCREASING_INTENSITY_MOVE"),
                MonsterState::Move(MonsterMove {
                    state_id: "INCREASING_INTENSITY_MOVE".to_owned(),
                    intent: Intent::new(
                        "INCREASING_INTENSITY_MOVE",
                        vec![MoveIntent::of("status_card"), MoveIntent::of("buff")],
                    ),
                    commands: vec![
                        // Every wither already in the fight grows by three, and
                        // the one this move makes arrives grown to match.
                        Command::Card(sts2_engine::CardCmd::GrowDamage {
                            cards: sts2_engine::CardSelector::CombatCardsOfModel(id("CARD.WITHER")),
                            amount: Amount::Fixed(3),
                        }),
                        Command::CardPile(CardPileCmd::AddGeneratedToCombat {
                            enchantment: None,
                            card: CardOrigin::Model(id("CARD.WITHER")),
                            grown_by: Amount::per(
                                0,
                                3,
                                sts2_engine::Count::TimesOwnerPerformedState(
                                    "INCREASING_INTENSITY_MOVE".to_owned(),
                                ),
                            ),
                            count: Amount::Fixed(wither_amount),
                            pile: PileName::Discard,
                            ethereal: false,
                            retain: false,
                            upgraded: false,
                            temporary_cost: None,
                            position: sts2_engine::DrawPilePosition::Bottom,
                        }),
                        // A base strength plus an additional one, and the
                        // additional one is raised after every intensity:
                        // three the first time, one more each time after it.
                        Command::Power(PowerCmd::Apply {
                            marks: None,
                            power: id("POWER.STRENGTH_POWER"),
                            target: CreatureSelector::Owner,
                            applier: Some(CreatureSelector::Owner),
                            amount: Amount::per(
                                asc(AscensionLevel::DeadlyEnemies, 3, 2),
                                1,
                                sts2_engine::Count::TimesOwnerPerformedState(
                                    "INCREASING_INTENSITY_MOVE".to_owned(),
                                ),
                            ),
                        }),
                    ],
                    follow_up: "EBB_MOVE".to_owned(),
                }),
            ],
        ),
        vec![
            Command::Power(PowerCmd::Apply {
                // The power is marked with its target before it is applied,
                // one instance per opponent.
                marks: Some(CreatureSelector::AllOpponents),
                power: id("POWER.WITHERING_PRESENCE_POWER"),
                target: CreatureSelector::Owner,
                applier: Some(CreatureSelector::Owner),
                amount: Amount::Fixed(6),
            }),
            self_power("POWER.ARTIFACT_POWER", 3),
        ],
    )
}

/// The waterfall giant: every move it makes builds the pressure it is holding,
/// and when it goes down the pressure blows it up in the player's face.
///
/// The pressure the gun is holding and the `SteamEruptionDamage` the eruption
/// will deal are numbers the build carries on the monster; they are written
/// down on the creature here, which is the same thing.
#[must_use]
#[allow(clippy::too_many_lines, reason = "one entry per move of the machine")]
fn waterfall_giant_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let hp = asc(AscensionLevel::ToughEnemies, 250, 240);
    let pressurize = asc(AscensionLevel::DeadlyEnemies, 20, 15);
    let stomp = asc(AscensionLevel::DeadlyEnemies, 16, 15);
    let ram = asc(AscensionLevel::DeadlyEnemies, 11, 10);
    let pressure_up = asc(AscensionLevel::DeadlyEnemies, 14, 13);
    let gun_base = asc(AscensionLevel::DeadlyEnemies, 23, 20);
    let siphon = asc(AscensionLevel::ToughEnemies, 15, 10);
    let build_pressure = |amount: i32| self_power("POWER.STEAM_ERUPTION_POWER", amount);
    joins_with(
        monster(
            "MONSTER.WATERFALL_GIANT",
            hp,
            hp,
            "PRESSURIZE_MOVE",
            vec![
                MonsterState::Move(MonsterMove {
                    state_id: "PRESSURIZE_MOVE".to_owned(),
                    intent: Intent::new("PRESSURIZE_MOVE", vec![MoveIntent::of("buff")]),
                    commands: vec![build_pressure(pressurize)],
                    follow_up: "STOMP_MOVE".to_owned(),
                }),
                MonsterState::Move(MonsterMove {
                    state_id: "STOMP_MOVE".to_owned(),
                    intent: Intent::new(
                        "STOMP_MOVE",
                        vec![
                            MoveIntent::attack(stomp, 1),
                            MoveIntent::of("debuff"),
                            MoveIntent::of("buff"),
                        ],
                    ),
                    commands: vec![
                        strike(stomp, 1),
                        debuff("POWER.WEAK_POWER", 1),
                        build_pressure(3),
                    ],
                    follow_up: "RAM_MOVE".to_owned(),
                }),
                MonsterState::Move(MonsterMove {
                    state_id: "RAM_MOVE".to_owned(),
                    intent: Intent::new(
                        "RAM_MOVE",
                        vec![MoveIntent::attack(ram, 1), MoveIntent::of("buff")],
                    ),
                    commands: vec![strike(ram, 1), build_pressure(3)],
                    follow_up: "SIPHON_MOVE".to_owned(),
                }),
                MonsterState::Move(MonsterMove {
                    state_id: "SIPHON_MOVE".to_owned(),
                    intent: Intent::new(
                        "SIPHON_MOVE",
                        vec![MoveIntent::of("heal"), MoveIntent::of("buff")],
                    ),
                    // The siphon heals once per player, which single-player
                    // is once.
                    commands: vec![
                        Command::Creature(CreatureCmd::Heal {
                            target: CreatureSelector::Owner,
                            amount: Amount::Fixed(siphon),
                        }),
                        build_pressure(3),
                    ],
                    follow_up: "PRESSURE_GUN_MOVE".to_owned(),
                }),
                MonsterState::Move(MonsterMove {
                    state_id: "PRESSURE_GUN_MOVE".to_owned(),
                    intent: Intent::new(
                        "PRESSURE_GUN_MOVE",
                        vec![
                            MoveIntent {
                                damage_from_local: Some("CurrentPressureGunDamage".to_owned()),
                                ..MoveIntent::attack(gun_base, 1)
                            },
                            MoveIntent::of("buff"),
                        ],
                    ),
                    commands: vec![
                        Command::Damage(DamageCmd::Attack {
                            amount: Amount::counting(sts2_engine::Count::OwnerLocal(
                                "CurrentPressureGunDamage".to_owned(),
                            )),
                            hits: Amount::Fixed(1),
                            target: CreatureSelector::AllOpponents,
                            props: ValueProps::NONE,
                        }),
                        // The gun's damage goes up by its own increase.
                        Command::Creature(CreatureCmd::SetLocal {
                            target: CreatureSelector::Owner,
                            name: "CurrentPressureGunDamage".to_owned(),
                            amount: Amount::per(
                                5,
                                1,
                                sts2_engine::Count::OwnerLocal(
                                    "CurrentPressureGunDamage".to_owned(),
                                ),
                            ),
                        }),
                        build_pressure(3),
                    ],
                    follow_up: "PRESSURE_UP_MOVE".to_owned(),
                }),
                MonsterState::Move(MonsterMove {
                    state_id: "PRESSURE_UP_MOVE".to_owned(),
                    intent: Intent::new(
                        "PRESSURE_UP_MOVE",
                        vec![MoveIntent::attack(pressure_up, 1), MoveIntent::of("buff")],
                    ),
                    commands: vec![strike(pressure_up, 1), build_pressure(3)],
                    follow_up: "STOMP_MOVE".to_owned(),
                }),
                // What the giant is put on as it goes down: it writes down what
                // it was holding, lets the pressure go, and blows up next.
                MonsterState::Move(MonsterMove {
                    state_id: "ABOUT_TO_BLOW_MOVE".to_owned(),
                    intent: Intent::new("ABOUT_TO_BLOW_MOVE", vec![MoveIntent::of("stun")]),
                    commands: vec![
                        Command::Creature(CreatureCmd::SetLocal {
                            target: CreatureSelector::Owner,
                            name: "SteamEruptionDamage".to_owned(),
                            amount: Amount::counting(sts2_engine::Count::OwnerPower(id(
                                "POWER.STEAM_ERUPTION_POWER",
                            ))),
                        }),
                        Command::Power(PowerCmd::Remove {
                            power: PowerRef::owned(id("POWER.STEAM_ERUPTION_POWER")),
                        }),
                    ],
                    follow_up: "EXPLODE_MOVE".to_owned(),
                }),
                MonsterState::Move(MonsterMove {
                    state_id: "EXPLODE_MOVE".to_owned(),
                    intent: Intent::new(
                        "EXPLODE_MOVE",
                        vec![MoveIntent {
                            damage_from_local: Some("SteamEruptionDamage".to_owned()),
                            ..MoveIntent::death_blow(0)
                        }],
                    ),
                    commands: vec![
                        Command::Damage(DamageCmd::Attack {
                            amount: Amount::counting(sts2_engine::Count::OwnerLocal(
                                "SteamEruptionDamage".to_owned(),
                            )),
                            hits: Amount::Fixed(1),
                            target: CreatureSelector::AllOpponents,
                            props: ValueProps::NONE,
                        }),
                        Command::Creature(CreatureCmd::Kill {
                            target: CreatureSelector::Owner,
                        }),
                    ],
                    follow_up: "EXPLODE_MOVE".to_owned(),
                }),
            ],
        ),
        // The gun opens at its base damage.
        vec![Command::Creature(CreatureCmd::SetLocal {
            target: CreatureSelector::Owner,
            name: "CurrentPressureGunDamage".to_owned(),
            amount: Amount::Fixed(gun_base),
        })],
    )
}

/// A seapunk: a kick, a spinning kick, and a burp it puts block and strength up
/// behind.
fn seapunk_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    monster(
        "MONSTER.SEAPUNK",
        asc(AscensionLevel::ToughEnemies, 47, 44),
        asc(AscensionLevel::ToughEnemies, 49, 46),
        "SEA_KICK_MOVE",
        vec![
            attack(
                "SEA_KICK_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 13, 11),
                "SPINNING_KICK_MOVE",
            ),
            multi_attack("SPINNING_KICK_MOVE", 2, 4, "BUBBLE_BURP_MOVE"),
            MonsterState::Move(MonsterMove {
                state_id: "BUBBLE_BURP_MOVE".to_owned(),
                intent: Intent::new(
                    "BUBBLE_BURP_MOVE",
                    vec![MoveIntent::of("buff"), MoveIntent::of("defend")],
                ),
                commands: vec![
                    guard(asc(AscensionLevel::ToughEnemies, 8, 7)),
                    self_power(
                        "POWER.STRENGTH_POWER",
                        asc(AscensionLevel::DeadlyEnemies, 2, 1),
                    ),
                ],
                follow_up: "SEA_KICK_MOVE".to_owned(),
            }),
        ],
    )
}

/// A haunted ship: it opens with a haunt that weakens and buries the player in
/// dazed cards, and then walks a swipe and a stomp for the rest of the fight.
fn haunted_ship_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let hp = asc(AscensionLevel::ToughEnemies, 67, 63);
    monster(
        "MONSTER.HAUNTED_SHIP",
        hp,
        hp,
        "HAUNT_MOVE",
        vec![
            MonsterState::Move(MonsterMove {
                state_id: "HAUNT_MOVE".to_owned(),
                intent: Intent::new(
                    "HAUNT_MOVE",
                    vec![MoveIntent::of("debuff"), MoveIntent::of("status_card")],
                ),
                commands: vec![
                    debuff("POWER.WEAK_POWER", 3),
                    conjure("CARD.DAZED", 5, PileName::Discard),
                ],
                follow_up: "SWIPE_MOVE".to_owned(),
            }),
            attack(
                "SWIPE_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 14, 13),
                "STOMP_MOVE",
            ),
            multi_attack(
                "STOMP_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 5, 4),
                3,
                "SWIPE_MOVE",
            ),
        ],
    )
}

/// A gas bomb: it goes off once and dies of it.
fn gas_bomb_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    joins_with(
        monster(
            "MONSTER.GAS_BOMB",
            asc(AscensionLevel::ToughEnemies, 8, 7),
            asc(AscensionLevel::ToughEnemies, 8, 7),
            "EXPLODE_MOVE",
            vec![MonsterState::Move(MonsterMove {
                state_id: "EXPLODE_MOVE".to_owned(),
                intent: Intent::new(
                    "EXPLODE_MOVE",
                    vec![MoveIntent::death_blow(asc(
                        AscensionLevel::DeadlyEnemies,
                        9,
                        8,
                    ))],
                ),
                commands: vec![
                    strike(asc(AscensionLevel::DeadlyEnemies, 9, 8), 1),
                    Command::Creature(CreatureCmd::Kill {
                        target: CreatureSelector::Owner,
                    }),
                ],
                follow_up: "EXPLODE_MOVE".to_owned(),
            })],
        ),
        vec![self_power("POWER.MINION_POWER", 1)],
    )
}

/// A living fog: gas that smogs whoever breathes it, a bloat that leaves a bomb
/// standing, and a blast. After the first gas it only ever bloats and blasts.
fn living_fog_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let gas = asc(AscensionLevel::DeadlyEnemies, 9, 8);
    let bloat = asc(AscensionLevel::DeadlyEnemies, 6, 5);
    let hp = asc(AscensionLevel::ToughEnemies, 82, 80);
    monster(
        "MONSTER.LIVING_FOG",
        hp,
        hp,
        "ADVANCED_GAS_MOVE",
        vec![
            MonsterState::Move(MonsterMove {
                state_id: "ADVANCED_GAS_MOVE".to_owned(),
                intent: Intent::new(
                    "ADVANCED_GAS_MOVE",
                    vec![MoveIntent::attack(gas, 1), MoveIntent::of("card_debuff")],
                ),
                commands: vec![strike(gas, 1), debuff("POWER.SMOGGY_POWER", 1)],
                follow_up: "BLOAT_MOVE".to_owned(),
            }),
            MonsterState::Move(MonsterMove {
                state_id: "BLOAT_MOVE".to_owned(),
                intent: Intent::new(
                    "BLOAT_MOVE",
                    vec![MoveIntent::attack(bloat, 1), MoveIntent::of("summon")],
                ),
                // The bomb is stood up first, in the first slot nobody holds,
                // and the swing follows it.
                commands: vec![
                    Command::Creature(CreatureCmd::Add {
                        monster: id("MONSTER.GAS_BOMB"),
                        slot: sts2_engine::SpawnSlot::FirstFree,
                        initial_state: None,
                        then: Vec::new(),
                    }),
                    strike(bloat, 1),
                ],
                follow_up: "SUPER_GAS_BLAST_MOVE".to_owned(),
            }),
            attack(
                "SUPER_GAS_BLAST_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 9, 8),
                "BLOAT_MOVE",
            ),
        ],
    )
}

/// A phantasmal gardener: a bite, a lash, a flail, and an enlargement, walked
/// round in that order. Which of the four it opens on is its slot.
fn phantasmal_gardener_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    joins_with(
        monster(
            "MONSTER.PHANTASMAL_GARDENER",
            asc(AscensionLevel::ToughEnemies, 27, 26),
            asc(AscensionLevel::ToughEnemies, 32, 31),
            "BITE_MOVE",
            vec![
                attack("BITE_MOVE", 5, "LASH_MOVE"),
                attack("LASH_MOVE", 7, "FLAIL_MOVE"),
                multi_attack("FLAIL_MOVE", 1, 3, "ENLARGE_MOVE"),
                self_buff(
                    "ENLARGE_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 3, 2),
                    "BITE_MOVE",
                ),
            ],
        ),
        vec![self_power(
            "POWER.SKITTISH_POWER",
            asc(AscensionLevel::ToughEnemies, 7, 6),
        )],
    )
}

/// A cultist: one incantation, and then the same dark strike for the rest of
/// the fight. The two of them differ only in their numbers.
fn cultist(
    model_id: &str,
    min_hp: i32,
    max_hp: i32,
    ritual: i32,
    strike_damage: i32,
) -> MonsterDefinition {
    monster(
        model_id,
        min_hp,
        max_hp,
        "INCANTATION_MOVE",
        vec![
            MonsterState::Move(MonsterMove {
                state_id: "INCANTATION_MOVE".to_owned(),
                intent: Intent::new("INCANTATION_MOVE", vec![MoveIntent::of("buff")]),
                commands: vec![self_power("POWER.RITUAL_POWER", ritual)],
                follow_up: "DARK_STRIKE_MOVE".to_owned(),
            }),
            attack("DARK_STRIKE_MOVE", strike_damage, "DARK_STRIKE_MOVE"),
        ],
    )
}

/// The calcified cultist: the smaller ritual, the heavier strike.
fn calcified_cultist_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    cultist(
        "MONSTER.CALCIFIED_CULTIST",
        asc(AscensionLevel::ToughEnemies, 39, 38),
        asc(AscensionLevel::ToughEnemies, 42, 41),
        2,
        asc(AscensionLevel::DeadlyEnemies, 11, 9),
    )
}

/// The damp cultist: the larger ritual, and a strike worth almost nothing.
fn damp_cultist_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    cultist(
        "MONSTER.DAMP_CULTIST",
        asc(AscensionLevel::ToughEnemies, 52, 51),
        asc(AscensionLevel::ToughEnemies, 54, 53),
        asc(AscensionLevel::DeadlyEnemies, 6, 5),
        asc(AscensionLevel::DeadlyEnemies, 3, 1),
    )
}

/// A sludge spinner: a spray that weakens, a slam, and a rage that leaves it
/// stronger. It opens on the spray and then rolls between the three, never the
/// same one twice.
fn sludge_spinner_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let spray = asc(AscensionLevel::DeadlyEnemies, 9, 8);
    let rage = asc(AscensionLevel::DeadlyEnemies, 7, 6);
    monster(
        "MONSTER.SLUDGE_SPINNER",
        asc(AscensionLevel::ToughEnemies, 41, 37),
        asc(AscensionLevel::ToughEnemies, 42, 39),
        "OIL_SPRAY_MOVE",
        vec![
            MonsterState::Move(MonsterMove {
                state_id: "OIL_SPRAY_MOVE".to_owned(),
                intent: Intent::new(
                    "OIL_SPRAY_MOVE",
                    vec![MoveIntent::attack(spray, 1), MoveIntent::of("debuff")],
                ),
                commands: vec![strike(spray, 1), debuff("POWER.WEAK_POWER", 1)],
                follow_up: "RAND".to_owned(),
            }),
            attack(
                "SLAM_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 12, 11),
                "RAND",
            ),
            MonsterState::Move(MonsterMove {
                state_id: "RAGE_MOVE".to_owned(),
                intent: Intent::new(
                    "RAGE_MOVE",
                    vec![MoveIntent::attack(rage, 1), MoveIntent::of("buff")],
                ),
                commands: vec![strike(rage, 1), self_power("POWER.STRENGTH_POWER", 3)],
                follow_up: "RAND".to_owned(),
            }),
            MonsterState::RandomBranch {
                state_id: "RAND".to_owned(),
                branches: vec![
                    MoveBranch::new("OIL_SPRAY_MOVE", MoveRepeat::CannotRepeat),
                    MoveBranch::new("SLAM_MOVE", MoveRepeat::CannotRepeat),
                    MoveBranch::new("RAGE_MOVE", MoveRepeat::CannotRepeat),
                ],
            },
        ],
    )
}

/// A corpse slug: two whips, a glomp, and a goop that leaves its target frail.
/// It stands up hungry, and eating one of its own costs it a turn.
fn corpse_slug_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    joins_with(
        monster(
            "MONSTER.CORPSE_SLUG",
            asc(AscensionLevel::ToughEnemies, 27, 25),
            asc(AscensionLevel::ToughEnemies, 29, 27),
            "WHIP_SLAP_MOVE",
            vec![
                multi_attack("WHIP_SLAP_MOVE", 3, 2, "GLOMP_MOVE"),
                attack(
                    "GLOMP_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 9, 8),
                    "GOOP_MOVE",
                ),
                MonsterState::Move(MonsterMove {
                    state_id: "GOOP_MOVE".to_owned(),
                    intent: Intent::new("GOOP_MOVE", vec![MoveIntent::of("debuff")]),
                    commands: vec![debuff("POWER.FRAIL_POWER", 2)],
                    follow_up: "WHIP_SLAP_MOVE".to_owned(),
                }),
            ],
        ),
        vec![self_power(
            "POWER.RAVENOUS_POWER",
            asc(AscensionLevel::DeadlyEnemies, 5, 4),
        )],
    )
}

/// A toadpole: spikes up, spits them, and whirls. Which of the three it opens
/// on is a mark the encounter writes on each spawn, so the opener is named
/// where the spawn is placed rather than branched here.
fn toadpole_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    monster(
        "MONSTER.TOADPOLE",
        asc(AscensionLevel::ToughEnemies, 22, 21),
        asc(AscensionLevel::ToughEnemies, 26, 25),
        "SPIKEN_MOVE",
        vec![
            MonsterState::Move(MonsterMove {
                state_id: "SPIKEN_MOVE".to_owned(),
                intent: Intent::new("SPIKEN_MOVE", vec![MoveIntent::of("buff")]),
                commands: vec![self_power("POWER.THORNS_POWER", 2)],
                follow_up: "SPIKE_SPIT_MOVE".to_owned(),
            }),
            MonsterState::Move(MonsterMove {
                state_id: "SPIKE_SPIT_MOVE".to_owned(),
                intent: Intent::new(
                    "SPIKE_SPIT_MOVE",
                    vec![MoveIntent::attack(
                        asc(AscensionLevel::DeadlyEnemies, 4, 3),
                        3,
                    )],
                ),
                // The spikes come off before the spitting.
                commands: vec![
                    self_power("POWER.THORNS_POWER", -2),
                    strike(asc(AscensionLevel::DeadlyEnemies, 4, 3), 3),
                ],
                follow_up: "WHIRL_MOVE".to_owned(),
            }),
            attack(
                "WHIRL_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 8, 7),
                "SPIKEN_MOVE",
            ),
        ],
    )
}

/// The globe head: a slap that leaves its target frail, three bolts, and a
/// burst that works its own strength in. It stands up already charged.
fn globe_head_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let hp = asc(AscensionLevel::ToughEnemies, 158, 148);
    joins_with(
        monster(
            "MONSTER.GLOBE_HEAD",
            hp,
            hp,
            "SHOCKING_SLAP",
            vec![
                MonsterState::Move(MonsterMove {
                    state_id: "SHOCKING_SLAP".to_owned(),
                    intent: Intent::new(
                        "SHOCKING_SLAP",
                        vec![
                            MoveIntent::attack(asc(AscensionLevel::DeadlyEnemies, 14, 13), 1),
                            MoveIntent::of("debuff"),
                        ],
                    ),
                    commands: vec![
                        strike(asc(AscensionLevel::DeadlyEnemies, 14, 13), 1),
                        debuff("POWER.FRAIL_POWER", 2),
                    ],
                    follow_up: "THUNDER_STRIKE".to_owned(),
                }),
                multi_attack(
                    "THUNDER_STRIKE",
                    asc(AscensionLevel::DeadlyEnemies, 7, 6),
                    3,
                    "GALVANIC_BURST",
                ),
                MonsterState::Move(MonsterMove {
                    state_id: "GALVANIC_BURST".to_owned(),
                    intent: Intent::new(
                        "GALVANIC_BURST",
                        vec![
                            MoveIntent::attack(asc(AscensionLevel::DeadlyEnemies, 17, 16), 1),
                            MoveIntent::of("buff"),
                        ],
                    ),
                    commands: vec![
                        strike(asc(AscensionLevel::DeadlyEnemies, 17, 16), 1),
                        self_power("POWER.STRENGTH_POWER", 2),
                    ],
                    follow_up: "SHOCKING_SLAP".to_owned(),
                }),
            ],
        ),
        vec![self_power("POWER.GALVANIC_POWER", 6)],
    )
}

/// The mysterious knight: a flail knight that stands up already six stronger
/// and six plated.
///
/// Its arrival runs the plain flail knight's — which is nothing — and then
/// works the two powers on itself.
fn mysterious_knight_at(ascension: u8) -> MonsterDefinition {
    joins_with(
        MonsterDefinition {
            model_id: id("MONSTER.MYSTERIOUS_KNIGHT"),
            ..flail_knight_at(ascension)
        },
        vec![
            self_power("POWER.STRENGTH_POWER", 6),
            self_power("POWER.PLATING_POWER", 6),
        ],
    )
}

/// The spectral knight: the hex first, then the sword, then a roll between the
/// sword again and the flames.
fn spectral_knight_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    monster(
        "MONSTER.SPECTRAL_KNIGHT",
        asc(AscensionLevel::ToughEnemies, 97, 93),
        asc(AscensionLevel::ToughEnemies, 97, 93),
        "HEX",
        vec![
            MonsterState::Move(MonsterMove {
                state_id: "HEX".to_owned(),
                intent: Intent::new("HEX", vec![MoveIntent::of("debuff")]),
                commands: vec![debuff("POWER.HEX_POWER", 2)],
                follow_up: "SOUL_SLASH".to_owned(),
            }),
            attack(
                "SOUL_SLASH",
                asc(AscensionLevel::DeadlyEnemies, 17, 15),
                "RAND",
            ),
            multi_attack(
                "SOUL_FLAME",
                asc(AscensionLevel::DeadlyEnemies, 4, 3),
                3,
                "RAND",
            ),
            MonsterState::RandomBranch {
                state_id: "RAND".to_owned(),
                branches: vec![
                    MoveBranch::new("SOUL_SLASH", MoveRepeat::CanRepeatTimes(2)),
                    MoveBranch::new("SOUL_FLAME", MoveRepeat::CannotRepeat),
                ],
            },
        ],
    )
}

/// The magi knight: one fixed round of shield, dampen and spear, and then it
/// winds up a bomb between every spear from there on.
fn magi_knight_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let block = asc(AscensionLevel::ToughEnemies, 9, 5);
    monster(
        "MONSTER.MAGI_KNIGHT",
        asc(AscensionLevel::ToughEnemies, 89, 82),
        asc(AscensionLevel::ToughEnemies, 89, 82),
        "POWER_SHIELD_MOVE",
        vec![
            MonsterState::Move(MonsterMove {
                state_id: "POWER_SHIELD_MOVE".to_owned(),
                intent: Intent::new(
                    "POWER_SHIELD_MOVE",
                    vec![
                        MoveIntent::attack(asc(AscensionLevel::DeadlyEnemies, 7, 6), 1),
                        MoveIntent::of("defend"),
                    ],
                ),
                commands: vec![
                    strike(asc(AscensionLevel::DeadlyEnemies, 7, 6), 1),
                    guard(block),
                ],
                follow_up: "DAMPEN_MOVE".to_owned(),
            }),
            MonsterState::Move(MonsterMove {
                state_id: "DAMPEN_MOVE".to_owned(),
                intent: Intent::new("DAMPEN_MOVE", vec![MoveIntent::of("debuff")]),
                commands: vec![debuff("POWER.DAMPEN_POWER", 1)],
                follow_up: "RAM_MOVE".to_owned(),
            }),
            attack(
                "RAM_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 11, 10),
                "PREP_MOVE",
            ),
            MonsterState::Move(MonsterMove {
                state_id: "PREP_MOVE".to_owned(),
                intent: Intent::new("PREP_MOVE", vec![MoveIntent::of("defend")]),
                commands: vec![guard(block)],
                follow_up: "MAGIC_BOMB".to_owned(),
            }),
            attack(
                "MAGIC_BOMB",
                asc(AscensionLevel::DeadlyEnemies, 40, 35),
                "RAM_MOVE",
            ),
        ],
    )
}

/// The magistrate: scrutiny, a flurry of pecks, then it takes to the air and
/// comes down on the verdict.
fn owl_magistrate_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    monster(
        "MONSTER.OWL_MAGISTRATE",
        asc(AscensionLevel::ToughEnemies, 247, 231),
        asc(AscensionLevel::ToughEnemies, 247, 231),
        "MAGISTRATE_SCRUTINY",
        vec![
            attack(
                "MAGISTRATE_SCRUTINY",
                asc(AscensionLevel::DeadlyEnemies, 17, 16),
                "PECK_ASSAULT",
            ),
            multi_attack(
                "PECK_ASSAULT",
                asc(AscensionLevel::DeadlyEnemies, 4, 4),
                6,
                "JUDICIAL_FLIGHT",
            ),
            MonsterState::Move(MonsterMove {
                state_id: "JUDICIAL_FLIGHT".to_owned(),
                intent: Intent::new("JUDICIAL_FLIGHT", vec![MoveIntent::of("buff")]),
                commands: vec![self_power("POWER.SOAR_POWER", 1)],
                follow_up: "VERDICT".to_owned(),
            }),
            MonsterState::Move(MonsterMove {
                state_id: "VERDICT".to_owned(),
                intent: Intent::new(
                    "VERDICT",
                    vec![
                        MoveIntent::attack(asc(AscensionLevel::DeadlyEnemies, 36, 33), 1),
                        MoveIntent::of("debuff"),
                    ],
                ),
                commands: vec![
                    strike(asc(AscensionLevel::DeadlyEnemies, 36, 33), 1),
                    debuff("POWER.VULNERABLE_POWER", 4),
                    Command::Power(PowerCmd::Remove {
                        power: PowerRef::owned(id("POWER.SOAR_POWER")),
                    }),
                ],
                follow_up: "MAGISTRATE_SCRUTINY".to_owned(),
            }),
        ],
    )
}

/// The soul nexus: one burn, one maelstrom, one draining blow, and a roll that
/// never takes the same one twice running.
fn soul_nexus_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    monster(
        "MONSTER.SOUL_NEXUS",
        asc(AscensionLevel::ToughEnemies, 254, 234),
        asc(AscensionLevel::ToughEnemies, 254, 234),
        "SOUL_BURN_MOVE",
        vec![
            attack(
                "SOUL_BURN_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 31, 29),
                "RAND",
            ),
            multi_attack(
                "MAELSTROM_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 7, 6),
                asc(AscensionLevel::DeadlyEnemies, 4, 4),
                "RAND",
            ),
            MonsterState::Move(MonsterMove {
                state_id: "DRAIN_LIFE_MOVE".to_owned(),
                intent: Intent::new(
                    "DRAIN_LIFE_MOVE",
                    vec![
                        MoveIntent::attack(asc(AscensionLevel::DeadlyEnemies, 19, 18), 1),
                        MoveIntent::of("debuff_strong"),
                    ],
                ),
                commands: vec![
                    strike(asc(AscensionLevel::DeadlyEnemies, 19, 18), 1),
                    debuff("POWER.VULNERABLE_POWER", 2),
                    debuff("POWER.WEAK_POWER", 2),
                ],
                follow_up: "RAND".to_owned(),
            }),
            MonsterState::RandomBranch {
                state_id: "RAND".to_owned(),
                branches: vec![
                    MoveBranch::new("SOUL_BURN_MOVE", MoveRepeat::CannotRepeat),
                    MoveBranch::new("MAELSTROM_MOVE", MoveRepeat::CannotRepeat),
                    MoveBranch::new("DRAIN_LIFE_MOVE", MoveRepeat::CannotRepeat),
                ],
            },
        ],
    )
}

/// The queen's puppet. It is a minion, so the fight ends with the queen.
fn torch_head_amalgam_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    joins_with(
        monster(
            "MONSTER.TORCH_HEAD_AMALGAM",
            asc(AscensionLevel::ToughEnemies, 211, 199),
            asc(AscensionLevel::ToughEnemies, 211, 199),
            "TACKLE_MOVE",
            vec![
                attack(
                    "TACKLE_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 19, 18),
                    "TACKLE_2_MOVE",
                ),
                attack(
                    "TACKLE_2_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 19, 18),
                    "BEAM_MOVE",
                ),
                multi_attack(
                    "BEAM_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 8, 8),
                    3,
                    "TACKLE_3_MOVE",
                ),
                attack(
                    "TACKLE_3_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 15, 14),
                    "TACKLE_4_MOVE",
                ),
                attack(
                    "TACKLE_4_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 15, 14),
                    "BEAM_MOVE",
                ),
            ],
        ),
        vec![self_power("POWER.MINION_POWER", 1)],
    )
}

/// Glory's queen. She binds the deck, drowns the player in debuffs, and feeds
/// her puppet until it goes down — and the moment it does, if she was about to
/// feed it again, she is enraged instead.
fn queen_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let mut definition = monster(
        "MONSTER.QUEEN",
        asc(AscensionLevel::ToughEnemies, 419, 400),
        asc(AscensionLevel::ToughEnemies, 419, 400),
        "PUPPET_STRINGS_MOVE",
        vec![
            MonsterState::Move(MonsterMove {
                state_id: "PUPPET_STRINGS_MOVE".to_owned(),
                intent: Intent::new("PUPPET_STRINGS_MOVE", vec![MoveIntent::of("card_debuff")]),
                commands: vec![debuff("POWER.CHAINS_OF_BINDING_POWER", 3)],
                follow_up: "YOU_ARE_MINE_MOVE".to_owned(),
            }),
            MonsterState::Move(MonsterMove {
                state_id: "YOU_ARE_MINE_MOVE".to_owned(),
                intent: Intent::new("YOU_ARE_MINE_MOVE", vec![MoveIntent::of("debuff")]),
                commands: vec![
                    debuff("POWER.FRAIL_POWER", 99),
                    debuff("POWER.WEAK_POWER", 99),
                    debuff("POWER.VULNERABLE_POWER", 99),
                ],
                follow_up: "YOURE_MINE_NOW_BRANCH".to_owned(),
            }),
            MonsterState::Move(MonsterMove {
                state_id: "BURN_BRIGHT_FOR_ME_MOVE".to_owned(),
                intent: Intent::new(
                    "BURN_BRIGHT_FOR_ME_MOVE",
                    vec![MoveIntent::of("buff"), MoveIntent::of("defend")],
                ),
                commands: vec![
                    Command::Power(PowerCmd::Apply {
                        marks: None,
                        power: id("POWER.STRENGTH_POWER"),
                        target: CreatureSelector::OtherAllies,
                        applier: Some(CreatureSelector::Owner),
                        amount: Amount::Fixed(asc(AscensionLevel::DeadlyEnemies, 1, 1)),
                    }),
                    guard(20),
                ],
                follow_up: "BURN_BRIGHT_FOR_ME_BRANCH".to_owned(),
            }),
            multi_attack(
                "OFF_WITH_YOUR_HEAD_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 4, 3),
                5,
                "EXECUTION_MOVE",
            ),
            attack(
                "EXECUTION_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 18, 15),
                "ENRAGE_MOVE",
            ),
            self_buff("ENRAGE_MOVE", 2, "OFF_WITH_YOUR_HEAD_MOVE"),
            amalgam_branch("YOURE_MINE_NOW_BRANCH"),
            amalgam_branch("BURN_BRIGHT_FOR_ME_BRANCH"),
        ],
    );
    // `AfterDeath`: when the puppet goes down and the queen is still standing,
    // a queen who was about to feed it is enraged where she stands instead.
    definition.hooks = vec![
        ModifierHook::every_turn(
            HookEvent::AfterDeath,
            vec![Command::When {
                otherwise: Vec::new(),
                condition: Condition::AnnouncedCreatureIsMonster(id("MONSTER.TORCH_HEAD_AMALGAM")),
                then: vec![Command::When {
                    otherwise: Vec::new(),
                    condition: Condition::OwnerMoveStateIs("BURN_BRIGHT_FOR_ME_MOVE".to_owned()),
                    then: vec![Command::Creature(CreatureCmd::SetMoveImmediate {
                        target: CreatureSelector::Owner,
                        state: "ENRAGE_MOVE".to_owned(),
                    })],
                }],
            }],
        )
        .when(HookCondition::AllyDied),
    ];
    definition
}

/// The two branches the queen asks the same question at: while her puppet is
/// standing she feeds it, and once it is gone she starts swinging.
fn amalgam_branch(state_id: &str) -> MonsterState {
    MonsterState::ConditionalBranch {
        state_id: state_id.to_owned(),
        branches: vec![
            ConditionalBranch::new(
                "BURN_BRIGHT_FOR_ME_MOVE",
                MoveCondition::Not(Box::new(MoveCondition::TeammateOfModelIsGone(id(
                    "MONSTER.TORCH_HEAD_AMALGAM",
                )))),
            ),
            ConditionalBranch::new(
                "OFF_WITH_YOUR_HEAD_MOVE",
                MoveCondition::TeammateOfModelIsGone(id("MONSTER.TORCH_HEAD_AMALGAM")),
            ),
        ],
    }
}

/// The egg bowlbug: one bite, and block behind it, forever.
fn bowlbug_egg_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    monster(
        "MONSTER.BOWLBUG_EGG",
        asc(AscensionLevel::ToughEnemies, 23, 21),
        asc(AscensionLevel::ToughEnemies, 24, 22),
        "BITE_MOVE",
        vec![MonsterState::Move(MonsterMove {
            state_id: "BITE_MOVE".to_owned(),
            intent: Intent::new(
                "BITE_MOVE",
                vec![
                    MoveIntent::attack(asc(AscensionLevel::DeadlyEnemies, 8, 7), 1),
                    MoveIntent::of("defend"),
                ],
            ),
            commands: vec![
                strike(asc(AscensionLevel::DeadlyEnemies, 8, 7), 1),
                guard(asc(AscensionLevel::DeadlyEnemies, 8, 7)),
            ],
            follow_up: "BITE_MOVE".to_owned(),
        })],
    )
}

/// The Obscura, which opens by conjuring its own fear and then rolls between
/// three ways of pressing it.
fn the_obscura_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    monster(
        "MONSTER.THE_OBSCURA",
        asc(AscensionLevel::ToughEnemies, 129, 123),
        asc(AscensionLevel::ToughEnemies, 129, 123),
        "ILLUSION_MOVE",
        vec![
            MonsterState::Move(MonsterMove {
                state_id: "ILLUSION_MOVE".to_owned(),
                intent: Intent::new("ILLUSION_MOVE", vec![MoveIntent::of("summon")]),
                commands: vec![Command::Creature(CreatureCmd::Add {
                    initial_state: None,
                    then: Vec::new(),
                    monster: id("MONSTER.PARAFRIGHT"),
                    // The fear stands in the first of the encounter's slots,
                    // so it takes its turn before the Obscura does.
                    slot: sts2_engine::SpawnSlot::Named("illusion".to_owned()),
                })],
                follow_up: "RAND".to_owned(),
            }),
            attack(
                "PIERCING_GAZE_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 11, 10),
                "RAND",
            ),
            // The wail is worked on everyone still standing beside it, itself
            // included.
            MonsterState::Move(MonsterMove {
                state_id: "SAIL_MOVE".to_owned(),
                intent: Intent::new("SAIL_MOVE", vec![MoveIntent::of("buff")]),
                commands: vec![power_on(
                    "POWER.STRENGTH_POWER",
                    CreatureSelector::AllAllies,
                    3,
                )],
                follow_up: "RAND".to_owned(),
            }),
            MonsterState::Move(MonsterMove {
                state_id: "HARDENING_STRIKE_MOVE".to_owned(),
                intent: Intent::new(
                    "HARDENING_STRIKE_MOVE",
                    vec![
                        MoveIntent::attack(asc(AscensionLevel::DeadlyEnemies, 7, 6), 1),
                        MoveIntent::of("defend"),
                    ],
                ),
                commands: vec![
                    strike(asc(AscensionLevel::DeadlyEnemies, 7, 6), 1),
                    guard(asc(AscensionLevel::DeadlyEnemies, 7, 6)),
                ],
                follow_up: "RAND".to_owned(),
            }),
            MonsterState::RandomBranch {
                state_id: "RAND".to_owned(),
                branches: vec![
                    MoveBranch::new("PIERCING_GAZE_MOVE", MoveRepeat::CannotRepeat),
                    MoveBranch::new("SAIL_MOVE", MoveRepeat::CannotRepeat),
                    MoveBranch::new("HARDENING_STRIKE_MOVE", MoveRepeat::CannotRepeat),
                ],
            },
        ],
    )
}

/// The fear the Obscura conjures: an illusion that only ever slams.
fn parafright_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    joins_with(
        monster(
            "MONSTER.PARAFRIGHT",
            21,
            21,
            "SLAM_MOVE",
            vec![attack(
                "SLAM_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 17, 16),
                "SLAM_MOVE",
            )],
        ),
        vec![self_power("POWER.ILLUSION_POWER", 1)],
    )
}

/// The snapping jaxfruit, which does nothing but throw an orb and grow.
fn snapping_jaxfruit_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    monster(
        "MONSTER.SNAPPING_JAXFRUIT",
        asc(AscensionLevel::ToughEnemies, 34, 31),
        asc(AscensionLevel::ToughEnemies, 36, 33),
        "ENERGY_ORB_MOVE",
        vec![MonsterState::Move(MonsterMove {
            state_id: "ENERGY_ORB_MOVE".to_owned(),
            intent: Intent::new(
                "ENERGY_ORB_MOVE",
                vec![
                    MoveIntent::attack(asc(AscensionLevel::DeadlyEnemies, 4, 3), 1),
                    MoveIntent::of("buff"),
                ],
            ),
            commands: vec![
                strike(asc(AscensionLevel::DeadlyEnemies, 4, 3), 1),
                self_power("POWER.STRENGTH_POWER", 2),
            ],
            follow_up: "ENERGY_ORB_MOVE".to_owned(),
        })],
    )
}

/// The vine shambler: a swipe, then vines that tangle the deck, then a chomp.
fn vine_shambler_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    monster(
        "MONSTER.VINE_SHAMBLER",
        asc(AscensionLevel::ToughEnemies, 64, 61),
        asc(AscensionLevel::ToughEnemies, 64, 61),
        "SWIPE_MOVE",
        vec![
            MonsterState::Move(MonsterMove {
                state_id: "GRASPING_VINES_MOVE".to_owned(),
                intent: Intent::new(
                    "GRASPING_VINES_MOVE",
                    vec![
                        MoveIntent::attack(asc(AscensionLevel::DeadlyEnemies, 9, 8), 1),
                        MoveIntent::of("card_debuff"),
                    ],
                ),
                commands: vec![
                    strike(asc(AscensionLevel::DeadlyEnemies, 9, 8), 1),
                    debuff("POWER.TANGLED_POWER", 1),
                ],
                follow_up: "CHOMP_MOVE".to_owned(),
            }),
            multi_attack(
                "SWIPE_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 7, 6),
                2,
                "GRASPING_VINES_MOVE",
            ),
            attack(
                "CHOMP_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 18, 16),
                "SWIPE_MOVE",
            ),
        ],
    )
}

/// Act one's Vantom: a blot, a lance, a dismembering that buries the discard
/// pile in wounds, and then it draws itself up again.
fn vantom_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    joins_with(
        monster(
            "MONSTER.VANTOM",
            asc(AscensionLevel::ToughEnemies, 183, 173),
            asc(AscensionLevel::ToughEnemies, 183, 173),
            "INK_BLOT_MOVE",
            vec![
                attack(
                    "INK_BLOT_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 8, 7),
                    "INKY_LANCE_MOVE",
                ),
                multi_attack(
                    "INKY_LANCE_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 7, 6),
                    2,
                    "DISMEMBER_MOVE",
                ),
                MonsterState::Move(MonsterMove {
                    state_id: "DISMEMBER_MOVE".to_owned(),
                    intent: Intent::new(
                        "DISMEMBER_MOVE",
                        vec![
                            MoveIntent::attack(asc(AscensionLevel::DeadlyEnemies, 30, 26), 1),
                            MoveIntent::of("status_card"),
                        ],
                    ),
                    commands: vec![
                        strike(asc(AscensionLevel::DeadlyEnemies, 30, 26), 1),
                        conjure("CARD.WOUND", 3, PileName::Discard),
                    ],
                    follow_up: "PREPARE_MOVE".to_owned(),
                }),
                self_buff("PREPARE_MOVE", 2, "INK_BLOT_MOVE"),
            ],
        ),
        vec![self_power(
            "POWER.SLIPPERY_POWER",
            asc(AscensionLevel::ToughEnemies, 9, 8),
        )],
    )
}

/// The louse progenitor: a web, then it curls up and grows, then it pounces.
fn louse_progenitor_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let curl_block = asc(AscensionLevel::ToughEnemies, 18, 14);
    joins_with(
        monster(
            "MONSTER.LOUSE_PROGENITOR",
            asc(AscensionLevel::ToughEnemies, 138, 134),
            asc(AscensionLevel::ToughEnemies, 141, 136),
            "WEB_CANNON_MOVE",
            vec![
                MonsterState::Move(MonsterMove {
                    state_id: "WEB_CANNON_MOVE".to_owned(),
                    intent: Intent::new(
                        "WEB_CANNON_MOVE",
                        vec![
                            MoveIntent::attack(asc(AscensionLevel::DeadlyEnemies, 10, 9), 1),
                            MoveIntent::of("debuff"),
                        ],
                    ),
                    commands: vec![
                        strike(asc(AscensionLevel::DeadlyEnemies, 10, 9), 1),
                        debuff("POWER.FRAIL_POWER", 2),
                    ],
                    follow_up: "CURL_AND_GROW_MOVE".to_owned(),
                }),
                MonsterState::Move(MonsterMove {
                    state_id: "CURL_AND_GROW_MOVE".to_owned(),
                    intent: Intent::new(
                        "CURL_AND_GROW_MOVE",
                        vec![MoveIntent::of("defend"), MoveIntent::of("buff")],
                    ),
                    commands: vec![guard(curl_block), self_power("POWER.STRENGTH_POWER", 5)],
                    follow_up: "POUNCE_MOVE".to_owned(),
                }),
                attack(
                    "POUNCE_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 16, 14),
                    "WEB_CANNON_MOVE",
                ),
            ],
        ),
        vec![self_power("POWER.CURL_UP_POWER", curl_block)],
    )
}

/// The infested prism: a jab, a radiating strike, a whirlwind, and a pulse that
/// puts more of its own spark up.
fn infested_prism_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let spark = asc(AscensionLevel::DeadlyEnemies, 3, 2);
    joins_with(
        monster(
            "MONSTER.INFESTED_PRISM",
            asc(AscensionLevel::ToughEnemies, 171, 161),
            asc(AscensionLevel::ToughEnemies, 171, 161),
            "JAB_MOVE",
            vec![
                attack(
                    "JAB_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 17, 15),
                    "RADIATE_MOVE",
                ),
                MonsterState::Move(MonsterMove {
                    state_id: "RADIATE_MOVE".to_owned(),
                    intent: Intent::new(
                        "RADIATE_MOVE",
                        vec![
                            MoveIntent::attack(asc(AscensionLevel::DeadlyEnemies, 13, 11), 1),
                            MoveIntent::of("defend"),
                        ],
                    ),
                    commands: vec![
                        strike(asc(AscensionLevel::DeadlyEnemies, 13, 11), 1),
                        guard(asc(AscensionLevel::DeadlyEnemies, 13, 11)),
                    ],
                    follow_up: "WHIRLWIND_MOVE".to_owned(),
                }),
                multi_attack(
                    "WHIRLWIND_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 6, 5),
                    3,
                    "PULSATE_MOVE",
                ),
                MonsterState::Move(MonsterMove {
                    state_id: "PULSATE_MOVE".to_owned(),
                    intent: Intent::new(
                        "PULSATE_MOVE",
                        vec![
                            MoveIntent::attack(asc(AscensionLevel::DeadlyEnemies, 10, 8), 1),
                            MoveIntent::of("buff"),
                            MoveIntent::of("defend"),
                        ],
                    ),
                    commands: vec![
                        strike(asc(AscensionLevel::DeadlyEnemies, 10, 8), 1),
                        guard(asc(AscensionLevel::ToughEnemies, 22, 20)),
                        self_power("POWER.VITAL_SPARK_POWER", spark),
                    ],
                    follow_up: "JAB_MOVE".to_owned(),
                }),
            ],
        ),
        vec![self_power("POWER.VITAL_SPARK_POWER", spark)],
    )
}

/// Act two's Knowledge Demon: three rounds of it, and each one opens with a
/// curse the player has to choose between.
///
/// Which pair is offered, and how much the disintegration is worth, is the
/// count of curses it has laid so far. The build keeps that count on the
/// monster; the state log carries the same number, since a curse is logged as
/// the machine stands on it and before the move runs.
fn knowledge_demon_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    monster(
        "MONSTER.KNOWLEDGE_DEMON",
        asc(AscensionLevel::ToughEnemies, 399, 379),
        asc(AscensionLevel::ToughEnemies, 399, 379),
        "CURSE_OF_KNOWLEDGE_MOVE",
        vec![
            MonsterState::Move(MonsterMove {
                state_id: "CURSE_OF_KNOWLEDGE_MOVE".to_owned(),
                intent: Intent::new("CURSE_OF_KNOWLEDGE_MOVE", vec![MoveIntent::of("debuff")]),
                commands: vec![
                    curse_of_knowledge(1, "CARD.MIND_ROT", "POWER.MIND_ROT_POWER", 1, 6),
                    curse_of_knowledge(2, "CARD.SLOTH", "POWER.SLOTH_POWER", 3, 7),
                    curse_of_knowledge(3, "CARD.WASTE_AWAY", "POWER.WASTE_AWAY_POWER", 1, 8),
                ],
                follow_up: "SLAP_MOVE".to_owned(),
            }),
            attack(
                "SLAP_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 18, 17),
                "KNOWLEDGE_OVERWHELMING_MOVE",
            ),
            multi_attack(
                "KNOWLEDGE_OVERWHELMING_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 9, 8),
                3,
                "PONDER_MOVE",
            ),
            MonsterState::Move(MonsterMove {
                state_id: "PONDER_MOVE".to_owned(),
                intent: Intent::new(
                    "PONDER_MOVE",
                    vec![
                        MoveIntent::attack(asc(AscensionLevel::DeadlyEnemies, 13, 11), 1),
                        MoveIntent::of("heal"),
                        MoveIntent::of("buff"),
                    ],
                ),
                commands: vec![
                    strike(asc(AscensionLevel::DeadlyEnemies, 13, 11), 1),
                    // Thirty per player, which is thirty in a single-player
                    // run.
                    Command::Creature(CreatureCmd::Heal {
                        target: CreatureSelector::Owner,
                        amount: Amount::Fixed(30),
                    }),
                    self_power(
                        "POWER.STRENGTH_POWER",
                        asc(AscensionLevel::DeadlyEnemies, 3, 2),
                    ),
                ],
                follow_up: "CurseOfKnowledgeBranch".to_owned(),
            }),
            MonsterState::ConditionalBranch {
                state_id: "CurseOfKnowledgeBranch".to_owned(),
                branches: vec![
                    ConditionalBranch::new(
                        "CURSE_OF_KNOWLEDGE_MOVE",
                        MoveCondition::Not(Box::new(MoveCondition::EnteredStateAtLeast {
                            state_id: "CURSE_OF_KNOWLEDGE_MOVE".to_owned(),
                            times: 3,
                        })),
                    ),
                    ConditionalBranch::new(
                        "SLAP_MOVE",
                        MoveCondition::EnteredStateAtLeast {
                            state_id: "CURSE_OF_KNOWLEDGE_MOVE".to_owned(),
                            times: 3,
                        },
                    ),
                ],
            },
        ],
    )
}

/// One round of the curse: two cards laid out, and whichever is taken works its
/// own power onto whoever took it.
fn curse_of_knowledge(
    round: usize,
    other: &str,
    other_power: &str,
    other_amount: i32,
    disintegration: i32,
) -> Command {
    let take = |card: &str, power: &str, amount: i32| Command::When {
        otherwise: Vec::new(),
        condition: Condition::ChosenCardIs(id(card)),
        then: vec![Command::Power(PowerCmd::Apply {
            marks: None,
            power: id(power),
            target: CreatureSelector::Target,
            applier: Some(CreatureSelector::Target),
            amount: Amount::Fixed(amount),
        })],
    };
    Command::When {
        otherwise: Vec::new(),
        condition: Condition::OwnerEnteredStateTimes {
            state_id: "CURSE_OF_KNOWLEDGE_MOVE".to_owned(),
            times: round,
        },
        then: vec![Command::CardSelect(CardSelectCmd::ChooseACard {
            cards: vec![
                sts2_engine::CardFingerprint::base(id("CARD.DISINTEGRATION")),
                sts2_engine::CardFingerprint::base(id(other)),
            ],
            can_skip: false,
            then: vec![
                take(
                    "CARD.DISINTEGRATION",
                    "POWER.DISINTEGRATION_POWER",
                    disintegration,
                ),
                take(other, other_power, other_amount),
            ],
        })],
    }
}

/// The turret operator: two bursts, then it reloads.
fn turret_operator_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let fire = asc(AscensionLevel::DeadlyEnemies, 4, 3);
    monster(
        "MONSTER.TURRET_OPERATOR",
        asc(AscensionLevel::ToughEnemies, 51, 41),
        asc(AscensionLevel::ToughEnemies, 51, 41),
        "UNLOAD_MOVE",
        vec![
            multi_attack("UNLOAD_MOVE", fire, 5, "UNLOAD_MOVE_2"),
            multi_attack("UNLOAD_MOVE_2", fire, 5, "RELOAD_MOVE"),
            self_buff("RELOAD_MOVE", 1, "UNLOAD_MOVE"),
        ],
    )
}

/// The living shield: it slams while anything is standing behind it, and swings
/// for itself once nothing is.
fn living_shield_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    joins_with(
        monster(
            "MONSTER.LIVING_SHIELD",
            asc(AscensionLevel::ToughEnemies, 65, 55),
            asc(AscensionLevel::ToughEnemies, 65, 55),
            "SHIELD_SLAM_MOVE",
            vec![
                attack("SHIELD_SLAM_MOVE", 6, "SHIELD_SLAM_BRANCH"),
                MonsterState::Move(MonsterMove {
                    state_id: "SMASH_MOVE".to_owned(),
                    intent: Intent::new(
                        "SMASH_MOVE",
                        vec![
                            MoveIntent::attack(asc(AscensionLevel::DeadlyEnemies, 18, 16), 1),
                            MoveIntent::of("buff"),
                        ],
                    ),
                    commands: vec![
                        strike(asc(AscensionLevel::DeadlyEnemies, 18, 16), 1),
                        self_power("POWER.STRENGTH_POWER", 3),
                    ],
                    follow_up: "SMASH_MOVE".to_owned(),
                }),
                MonsterState::ConditionalBranch {
                    state_id: "SHIELD_SLAM_BRANCH".to_owned(),
                    branches: vec![
                        ConditionalBranch::new("SHIELD_SLAM_MOVE", MoveCondition::HasLivingAlly),
                        ConditionalBranch::new(
                            "SMASH_MOVE",
                            MoveCondition::Not(Box::new(MoveCondition::HasLivingAlly)),
                        ),
                    ],
                },
            ],
        ),
        vec![self_power("POWER.RAMPART_POWER", 25)],
    )
}

/// The Necrobinder's pet.
///
/// It is a monster model like any other, and almost none of that model is used.
/// Its machine is one move that does nothing and follows up into itself, and
/// nothing ever rolls it: a pet stands on the player's side, so the enemy turn
/// never reaches it, and it would be refused a turn even if it did. What the
/// model is for is the HP it opens at — one, whatever the summon then sets —
/// and the creature the pet is.
#[must_use]
pub fn osty() -> MonsterDefinition {
    monster(
        "MONSTER.OSTY",
        1,
        1,
        "NOTHING_MOVE",
        vec![MonsterState::Move(MonsterMove {
            state_id: "NOTHING_MOVE".to_owned(),
            intent: Intent::new("NOTHING_MOVE", Vec::new()),
            commands: Vec::new(),
            follow_up: "NOTHING_MOVE".to_owned(),
        })],
    )
}

/// A byrdpip, the pet a Byrdonis egg hatches into. Like every event pet it is
/// nine thousand nine hundred and ninety-nine health that no bar ever shows,
/// and one hidden move that does nothing and follows itself.
#[must_use]
pub fn byrdpip() -> MonsterDefinition {
    pet_monster("MONSTER.BYRDPIP")
}

/// Pael's legion, the pet the ancient's relic brings, on the same model.
#[must_use]
pub fn paels_legion() -> MonsterDefinition {
    pet_monster("MONSTER.PAELS_LEGION")
}

/// The shape both event pets are: a creature to stand there, and nothing else.
fn pet_monster(model_id: &str) -> MonsterDefinition {
    monster(
        model_id,
        9999,
        9999,
        "NOTHING_MOVE",
        vec![MonsterState::Move(MonsterMove {
            state_id: "NOTHING_MOVE".to_owned(),
            intent: Intent::new("NOTHING_MOVE", Vec::new()),
            commands: Vec::new(),
            follow_up: "NOTHING_MOVE".to_owned(),
        })],
    )
}

/// The Architect, who stands in his own room behind the last act's boss and
/// never acts: one hidden `NOTHING` move that follows itself, and a health
/// range with a single value in it — so the roll his creation spends still
/// reads a die and still lands on 9999.
#[must_use]
pub fn architect() -> MonsterDefinition {
    monster(
        "MONSTER.ARCHITECT",
        9999,
        9999,
        "NOTHING_MOVE",
        vec![MonsterState::Move(MonsterMove {
            state_id: "NOTHING_MOVE".to_owned(),
            intent: Intent::new("NOTHING_MOVE", Vec::new()),
            commands: Vec::new(),
            follow_up: "NOTHING_MOVE".to_owned(),
        })],
    )
}

/// A fossil stalker: a tackle that leaves its target frail, a latch, and a
/// double lash. It opens on the latch and rolls between the three from then on,
/// and none of them can come up three times running.
///
/// On arrival it works three Suck onto itself, which is the power that turns
/// every hit of its own that gets through into three strength.
fn fossil_stalker_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let tackle = asc(AscensionLevel::DeadlyEnemies, 11, 9);
    joins_with(
        monster(
            "MONSTER.FOSSIL_STALKER",
            asc(AscensionLevel::ToughEnemies, 54, 51),
            asc(AscensionLevel::ToughEnemies, 56, 53),
            "LATCH_MOVE",
            vec![
                MonsterState::RandomBranch {
                    state_id: "RAND".to_owned(),
                    branches: vec![
                        MoveBranch::new("LATCH_MOVE", MoveRepeat::CanRepeatTimes(2)),
                        MoveBranch::new("TACKLE_MOVE", MoveRepeat::CanRepeatTimes(2)),
                        MoveBranch::new("LASH_MOVE", MoveRepeat::CanRepeatTimes(2)),
                    ],
                },
                MonsterState::Move(MonsterMove {
                    state_id: "TACKLE_MOVE".to_owned(),
                    intent: Intent::new(
                        "TACKLE_MOVE",
                        vec![MoveIntent::attack(tackle, 1), MoveIntent::of("debuff")],
                    ),
                    commands: vec![strike(tackle, 1), debuff("POWER.FRAIL_POWER", 1)],
                    follow_up: "RAND".to_owned(),
                }),
                attack(
                    "LATCH_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 14, 12),
                    "RAND",
                ),
                multi_attack(
                    "LASH_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 4, 3),
                    2,
                    "RAND",
                ),
            ],
        ),
        vec![self_power_amount("POWER.SUCK_POWER", Amount::Fixed(3))],
    )
}

/// The mecha knight: one charge, and then it works round the flamethrower that
/// fills the hand with burns, the wind-up it guards and grows behind, and the
/// cleave the wind-up was for. It stands up behind three artifact.
///
/// Whether it is wound up is a mark the wind-up raises and the cleave lowers,
/// and nothing but the animation ever answers to it.
fn mecha_knight_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let hp = asc(AscensionLevel::ToughEnemies, 320, 300);
    joins_with(
        monster(
            "MONSTER.MECHA_KNIGHT",
            hp,
            hp,
            "CHARGE_MOVE",
            vec![
                attack(
                    "CHARGE_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 30, 25),
                    "FLAMETHROWER_MOVE",
                ),
                MonsterState::Move(MonsterMove {
                    state_id: "FLAMETHROWER_MOVE".to_owned(),
                    intent: Intent::new("FLAMETHROWER_MOVE", vec![MoveIntent::of("status_card")]),
                    commands: vec![conjure("CARD.BURN", 4, PileName::Hand)],
                    follow_up: "WINDUP_MOVE".to_owned(),
                }),
                MonsterState::Move(MonsterMove {
                    state_id: "WINDUP_MOVE".to_owned(),
                    intent: Intent::new(
                        "WINDUP_MOVE",
                        vec![MoveIntent::of("defend"), MoveIntent::of("buff")],
                    ),
                    commands: vec![guard(15), self_power("POWER.STRENGTH_POWER", 5)],
                    follow_up: "HEAVY_CLEAVE_MOVE".to_owned(),
                }),
                attack(
                    "HEAVY_CLEAVE_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 40, 35),
                    "FLAMETHROWER_MOVE",
                ),
            ],
        ),
        vec![self_power("POWER.ARTIFACT_POWER", 3)],
    )
}

/// A punch construct: it readies behind ten block, works a fast one-two that
/// leaves its target frail, and lands one heavy punch, over and over. It stands
/// up behind one artifact.
///
/// The opener it takes, and how much is knocked off the size it rolled, are
/// both marks an encounter writes on each spawn. The Punch Construct room sets
/// neither, so it opens on the ready at whatever size it rolled; the punch-off
/// event's room writes both. The reduction drops the health outright, so
/// nothing is announced and it stops at a floor of one.
fn punch_construct_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let fast = asc(AscensionLevel::DeadlyEnemies, 6, 5);
    let hp = asc(AscensionLevel::ToughEnemies, 60, 55);
    joins_with(
        monster(
            "MONSTER.PUNCH_CONSTRUCT",
            hp,
            hp,
            "READY_MOVE",
            vec![
                MonsterState::Move(MonsterMove {
                    state_id: "READY_MOVE".to_owned(),
                    intent: Intent::new("READY_MOVE", vec![MoveIntent::of("defend")]),
                    commands: vec![guard(10)],
                    follow_up: "FAST_PUNCH_MOVE".to_owned(),
                }),
                MonsterState::Move(MonsterMove {
                    state_id: "FAST_PUNCH_MOVE".to_owned(),
                    intent: Intent::new(
                        "FAST_PUNCH_MOVE",
                        vec![MoveIntent::attack(fast, 2), MoveIntent::of("debuff")],
                    ),
                    commands: vec![strike(fast, 2), debuff("POWER.FRAIL_POWER", 1)],
                    follow_up: "STRONG_PUNCH_MOVE".to_owned(),
                }),
                attack(
                    "STRONG_PUNCH_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 16, 14),
                    "READY_MOVE",
                ),
            ],
        ),
        vec![
            self_power("POWER.ARTIFACT_POWER", 1),
            Command::When {
                otherwise: Vec::new(),
                condition: Condition::CountAtLeast {
                    count: Count::OwnerLocal("StartingHpReduction".to_owned()),
                    at_least: 1,
                },
                then: vec![Command::Creature(CreatureCmd::SetCurrentHpRaw {
                    target: CreatureSelector::Owner,
                    amount: Amount::Greatest(vec![
                        Amount::Fixed(1),
                        Amount::Sum(vec![
                            Amount::counting(Count::OwnerCurrentHp),
                            Amount::negating(Count::OwnerLocal("StartingHpReduction".to_owned())),
                        ]),
                    ]),
                })],
            },
        ],
    )
}

/// A skulking colony: two zooms, a thrust it grows off, and a pair of piercing
/// stabs, walked round in that order.
///
/// On arrival it also works twenty Hardened Shell onto itself, which caps
/// what any one turn can take off it. Hardened Shell is not registered here,
/// so that command is not written.
fn skulking_colony_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let inertia = asc(AscensionLevel::DeadlyEnemies, 11, 9);
    let zoom = asc(AscensionLevel::DeadlyEnemies, 16, 14);
    let hp = asc(AscensionLevel::ToughEnemies, 80, 75);
    joins_with(
        monster(
            "MONSTER.SKULKING_COLONY",
            hp,
            hp,
            "ZOOM_MOVE",
            vec![
                attack("ZOOM_MOVE", zoom, "ZOOM_MOVE_2"),
                attack("ZOOM_MOVE_2", zoom, "INERTIA_MOVE"),
                MonsterState::Move(MonsterMove {
                    state_id: "INERTIA_MOVE".to_owned(),
                    intent: Intent::new(
                        "INERTIA_MOVE",
                        vec![MoveIntent::attack(inertia, 1), MoveIntent::of("buff")],
                    ),
                    commands: vec![
                        strike(inertia, 1),
                        self_power(
                            "POWER.STRENGTH_POWER",
                            asc(AscensionLevel::DeadlyEnemies, 4, 2),
                        ),
                    ],
                    follow_up: "PIERCING_STABS_MOVE".to_owned(),
                }),
                multi_attack(
                    "PIERCING_STABS_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 8, 7),
                    2,
                    "ZOOM_MOVE",
                ),
            ],
        ),
        vec![self_power_amount(
            "POWER.HARDENED_SHELL_POWER",
            Amount::Fixed(20),
        )],
    )
}

/// A terror eel: it crashes and it thrashes, and every thrashing leaves it six
/// vigor heavier.
///
/// The stun and the terror are a pair nothing in the machine ever walks into.
/// The shriek the eel stands up with is what puts it there: once a blow gets
/// through and leaves the eel at or under what the shriek is worth, the shriek
/// stuns it and names the terror as what follows, and then comes off. The
/// terror is ninety-nine vulnerable on whoever the eel is fighting.
///
/// Shriek is not registered here, so the command that stands the eel up with
/// it is not written, and nothing reaches the two states it leads to.
fn terror_eel_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let thrash = asc(AscensionLevel::DeadlyEnemies, 4, 3);
    let hp = asc(AscensionLevel::ToughEnemies, 150, 140);
    joins_with(
        monster(
            "MONSTER.TERROR_EEL",
            hp,
            hp,
            "CRASH_MOVE",
            vec![
                attack(
                    "CRASH_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 18, 16),
                    "THRASH_MOVE",
                ),
                MonsterState::Move(MonsterMove {
                    state_id: "THRASH_MOVE".to_owned(),
                    intent: Intent::new(
                        "THRASH_MOVE",
                        vec![MoveIntent::attack(thrash, 3), MoveIntent::of("buff")],
                    ),
                    commands: vec![strike(thrash, 3), self_power("POWER.VIGOR_POWER", 6)],
                    follow_up: "CRASH_MOVE".to_owned(),
                }),
                MonsterState::Move(MonsterMove {
                    state_id: "STUN_MOVE".to_owned(),
                    intent: Intent::new("STUN_MOVE", vec![MoveIntent::of("stun")]),
                    commands: Vec::new(),
                    follow_up: "TERROR_MOVE".to_owned(),
                }),
                MonsterState::Move(MonsterMove {
                    state_id: "TERROR_MOVE".to_owned(),
                    intent: Intent::new("TERROR_MOVE", vec![MoveIntent::of("debuff")]),
                    commands: vec![debuff("POWER.VULNERABLE_POWER", 99)],
                    follow_up: "CRASH_MOVE".to_owned(),
                }),
            ],
        ),
        vec![self_power_amount(
            "POWER.SHRIEK_POWER",
            Amount::Fixed(asc(AscensionLevel::ToughEnemies, 75, 70)),
        )],
    )
}

/// A sewer clam: a shell it stands up behind, and a jet it alternates with the
/// pressure that makes the next one heavier.
fn sewer_clam_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let jet = asc(AscensionLevel::DeadlyEnemies, 11, 10);
    joins_with(
        monster(
            "MONSTER.SEWER_CLAM",
            asc(AscensionLevel::ToughEnemies, 58, 56),
            asc(AscensionLevel::ToughEnemies, 58, 56),
            "JET_MOVE",
            vec![
                attack("JET_MOVE", jet, "PRESSURIZE_MOVE"),
                MonsterState::Move(MonsterMove {
                    state_id: "PRESSURIZE_MOVE".to_owned(),
                    intent: Intent::new("PRESSURIZE_MOVE", vec![MoveIntent::of("buff")]),
                    commands: vec![self_power("POWER.STRENGTH_POWER", 4)],
                    follow_up: "JET_MOVE".to_owned(),
                }),
            ],
        ),
        vec![self_power(
            "POWER.PLATING_POWER",
            asc(AscensionLevel::ToughEnemies, 9, 8),
        )],
    )
}

/// A gremlin merc: three moves, each of them a swing and then a hand in the
/// player's purse. It walks in already holding a Surprise — the two gremlins
/// it lets out when it goes down — and a Thievery, which is the purse.
fn gremlin_merc_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let gimme = asc(AscensionLevel::ToughEnemies, 8, 7);
    let smash = asc(AscensionLevel::ToughEnemies, 7, 6);
    let hehe = asc(AscensionLevel::ToughEnemies, 9, 8);
    joins_with(
        monster(
            "MONSTER.GREMLIN_MERC",
            asc(AscensionLevel::ToughEnemies, 51, 47),
            asc(AscensionLevel::ToughEnemies, 53, 49),
            "GIMME_MOVE",
            vec![
                MonsterState::Move(MonsterMove {
                    state_id: "GIMME_MOVE".to_owned(),
                    intent: Intent::new("GIMME_MOVE", vec![MoveIntent::attack(gimme, 2)]),
                    commands: vec![strike(gimme, 2), steal()],
                    follow_up: "DOUBLE_SMASH_MOVE".to_owned(),
                }),
                MonsterState::Move(MonsterMove {
                    state_id: "DOUBLE_SMASH_MOVE".to_owned(),
                    intent: Intent::new(
                        "DOUBLE_SMASH_MOVE",
                        vec![MoveIntent::attack(smash, 2), MoveIntent::of("debuff")],
                    ),
                    commands: vec![strike(smash, 2), steal(), debuff("POWER.WEAK_POWER", 2)],
                    follow_up: "HEHE_MOVE".to_owned(),
                }),
                MonsterState::Move(MonsterMove {
                    state_id: "HEHE_MOVE".to_owned(),
                    intent: Intent::new(
                        "HEHE_MOVE",
                        vec![MoveIntent::attack(hehe, 1), MoveIntent::of("buff")],
                    ),
                    commands: vec![
                        strike(hehe, 1),
                        steal(),
                        self_power("POWER.STRENGTH_POWER", 2),
                    ],
                    follow_up: "GIMME_MOVE".to_owned(),
                }),
            ],
        ),
        vec![
            self_power("POWER.SURPRISE_POWER", 1),
            // One Thievery per player, each marking the purse it takes from.
            Command::Power(PowerCmd::Apply {
                marks: Some(CreatureSelector::AllOpponents),
                power: id("POWER.THIEVERY_POWER"),
                target: CreatureSelector::Owner,
                applier: Some(CreatureSelector::Owner),
                amount: Amount::Fixed(20),
            }),
        ],
    )
}

/// A thievery's theft: as much of the player's purse as the power is worth,
/// taken and written down on the power itself so the fat gremlin can carry it
/// off.
fn steal() -> Command {
    Command::WithAmount {
        amount: Amount::Least(vec![
            Amount::counting(sts2_engine::Count::OwnerPower(id("POWER.THIEVERY_POWER"))),
            Amount::counting(sts2_engine::Count::PlayerGold),
        ]),
        then: vec![
            Command::Player(sts2_engine::PlayerCmd::LoseGold(Amount::counting(
                sts2_engine::Count::Bound,
            ))),
            Command::Power(PowerCmd::ModifyCounter {
                power: sts2_engine::PowerRef::owned(id("POWER.THIEVERY_POWER")),
                amount: Amount::counting(sts2_engine::Count::Bound),
            }),
        ],
    }
}

/// A fat gremlin: it wakes up on the turn it was let out, and runs on the next
/// one with whatever the merc had taken.
fn fat_gremlin_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    monster(
        "MONSTER.FAT_GREMLIN",
        asc(AscensionLevel::ToughEnemies, 14, 13),
        asc(AscensionLevel::ToughEnemies, 18, 17),
        "SPAWNED_MOVE",
        vec![
            MonsterState::Move(MonsterMove {
                state_id: "SPAWNED_MOVE".to_owned(),
                intent: Intent::new("SPAWNED_MOVE", vec![MoveIntent::of("stun")]),
                commands: Vec::new(),
                follow_up: "FLEE_MOVE".to_owned(),
            }),
            MonsterState::Move(MonsterMove {
                state_id: "FLEE_MOVE".to_owned(),
                intent: Intent::new("FLEE_MOVE", vec![MoveIntent::of("escape")]),
                commands: vec![Command::Creature(CreatureCmd::Escape {
                    target: CreatureSelector::Owner,
                })],
                follow_up: "FLEE_MOVE".to_owned(),
            }),
        ],
    )
}

/// A sneaky gremlin: the same wake-up, and then a tackle every turn.
fn sneaky_gremlin_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    monster(
        "MONSTER.SNEAKY_GREMLIN",
        asc(AscensionLevel::ToughEnemies, 11, 10),
        asc(AscensionLevel::ToughEnemies, 15, 14),
        "SPAWNED_MOVE",
        vec![
            MonsterState::Move(MonsterMove {
                state_id: "SPAWNED_MOVE".to_owned(),
                intent: Intent::new("SPAWNED_MOVE", vec![MoveIntent::of("stun")]),
                commands: Vec::new(),
                follow_up: "TACKLE_MOVE".to_owned(),
            }),
            attack(
                "TACKLE_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 10, 9),
                "TACKLE_MOVE",
            ),
        ],
    )
}

/// The soul fysh: the Underdocks' second boss, on a five-move round. It buries
/// its opponent in Beckons, fades out of reach for a turn, and screams its way
/// back in.
fn soul_fysh_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let hp = asc(AscensionLevel::ToughEnemies, 221, 211);
    let gaze = asc(AscensionLevel::DeadlyEnemies, 8, 7);
    let scream = asc(AscensionLevel::DeadlyEnemies, 15, 13);
    monster(
        "MONSTER.SOUL_FYSH",
        hp,
        hp,
        "BECKON_MOVE",
        vec![
            // One Beckon shuffled into the draw pile and one on the discard.
            MonsterState::Move(MonsterMove {
                state_id: "BECKON_MOVE".to_owned(),
                intent: Intent::new("BECKON_MOVE", vec![MoveIntent::of("status_card")]),
                commands: vec![
                    Command::CardPile(CardPileCmd::AddGeneratedToCombat {
                        enchantment: None,
                        card: CardOrigin::Model(id("CARD.BECKON")),
                        grown_by: Amount::Fixed(0),
                        count: Amount::Fixed(1),
                        pile: PileName::Draw,
                        ethereal: false,
                        retain: false,
                        upgraded: false,
                        temporary_cost: None,
                        position: sts2_engine::DrawPilePosition::Random,
                    }),
                    conjure("CARD.BECKON", 1, PileName::Discard),
                ],
                follow_up: "DE_GAS_MOVE".to_owned(),
            }),
            attack(
                "DE_GAS_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 17, 16),
                "GAZE_MOVE",
            ),
            MonsterState::Move(MonsterMove {
                state_id: "GAZE_MOVE".to_owned(),
                intent: Intent::new(
                    "GAZE_MOVE",
                    vec![MoveIntent::attack(gaze, 1), MoveIntent::of("status_card")],
                ),
                commands: vec![
                    strike(gaze, 1),
                    conjure("CARD.BECKON", 1, PileName::Discard),
                ],
                follow_up: "FADE_MOVE".to_owned(),
            }),
            MonsterState::Move(MonsterMove {
                state_id: "FADE_MOVE".to_owned(),
                intent: Intent::new("FADE_MOVE", vec![MoveIntent::of("buff")]),
                commands: vec![self_power("POWER.INTANGIBLE_POWER", 2)],
                follow_up: "SCREAM_MOVE".to_owned(),
            }),
            MonsterState::Move(MonsterMove {
                state_id: "SCREAM_MOVE".to_owned(),
                intent: Intent::new(
                    "SCREAM_MOVE",
                    vec![MoveIntent::attack(scream, 1), MoveIntent::of("debuff")],
                ),
                commands: vec![strike(scream, 1), debuff("POWER.VULNERABLE_POWER", 3)],
                follow_up: "BECKON_MOVE".to_owned(),
            }),
        ],
    )
}

/// A move that both hits and puts block up behind the swing, which is what an
/// axe raider's two swings are.
fn swing_and_guard(state_id: &str, damage: i32, block: i32, follow_up: &str) -> MonsterState {
    MonsterState::Move(MonsterMove {
        state_id: state_id.to_owned(),
        intent: Intent::new(
            state_id,
            vec![MoveIntent::attack(damage, 1), MoveIntent::of("defend")],
        ),
        commands: vec![strike(damage, 1), guard(block)],
        follow_up: follow_up.to_owned(),
    })
}

/// The assassin raider: one killshot, over and over.
fn assassin_ruby_raider_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    monster(
        "MONSTER.ASSASSIN_RUBY_RAIDER",
        asc(AscensionLevel::ToughEnemies, 19, 18),
        asc(AscensionLevel::ToughEnemies, 24, 23),
        "KILLSHOT_MOVE",
        vec![attack(
            "KILLSHOT_MOVE",
            asc(AscensionLevel::DeadlyEnemies, 11, 10),
            "KILLSHOT_MOVE",
        )],
    )
}

/// The axe raider: two swings it guards behind, and then one heavier swing it
/// does not.
fn axe_ruby_raider_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let swing = asc(AscensionLevel::DeadlyEnemies, 6, 5);
    let block = asc(AscensionLevel::DeadlyEnemies, 6, 5);
    monster(
        "MONSTER.AXE_RUBY_RAIDER",
        asc(AscensionLevel::ToughEnemies, 21, 20),
        asc(AscensionLevel::ToughEnemies, 23, 22),
        "SWING_1",
        vec![
            swing_and_guard("SWING_1", swing, block, "SWING_2"),
            swing_and_guard("SWING_2", swing, block, "BIG_SWING"),
            attack(
                "BIG_SWING",
                asc(AscensionLevel::DeadlyEnemies, 13, 12),
                "SWING_1",
            ),
        ],
    )
}

/// The brute raider: a beating, then a roar that leaves it three stronger.
fn brute_ruby_raider_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    monster(
        "MONSTER.BRUTE_RUBY_RAIDER",
        asc(AscensionLevel::ToughEnemies, 31, 30),
        asc(AscensionLevel::ToughEnemies, 34, 33),
        "BEAT_MOVE",
        vec![
            attack(
                "BEAT_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 8, 7),
                "ROAR_MOVE",
            ),
            self_buff("ROAR_MOVE", 3, "BEAT_MOVE"),
        ],
    )
}

/// The crossbow raider: it opens by loading, and then alternates the shot with
/// loading the next one.
///
/// Whether the crossbow is loaded is a mark the two moves write and nothing
/// but the animation answers to.
fn crossbow_ruby_raider_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    monster(
        "MONSTER.CROSSBOW_RUBY_RAIDER",
        asc(AscensionLevel::ToughEnemies, 19, 18),
        asc(AscensionLevel::ToughEnemies, 22, 21),
        // The reload is where the machine opens, not the shot.
        "RELOAD_MOVE",
        vec![
            MonsterState::Move(MonsterMove {
                state_id: "RELOAD_MOVE".to_owned(),
                intent: Intent::new("RELOAD_MOVE", vec![MoveIntent::of("defend")]),
                commands: vec![guard(3)],
                follow_up: "FIRE_MOVE".to_owned(),
            }),
            attack(
                "FIRE_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 16, 14),
                "RELOAD_MOVE",
            ),
        ],
    )
}

/// The tracker raider: it marks its quarry frail once, and then sets the hounds
/// on it for the rest of the fight.
fn tracker_ruby_raider_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let hounds = asc(AscensionLevel::DeadlyEnemies, 1, 1);
    let repeat = asc(AscensionLevel::DeadlyEnemies, 9, 8);
    monster(
        "MONSTER.TRACKER_RUBY_RAIDER",
        asc(AscensionLevel::ToughEnemies, 22, 21),
        asc(AscensionLevel::ToughEnemies, 26, 25),
        "TRACK_MOVE",
        vec![
            MonsterState::Move(MonsterMove {
                state_id: "TRACK_MOVE".to_owned(),
                intent: Intent::new("TRACK_MOVE", vec![MoveIntent::of("debuff")]),
                commands: vec![debuff("POWER.FRAIL_POWER", 2)],
                follow_up: "HOUNDS_MOVE".to_owned(),
            }),
            multi_attack("HOUNDS_MOVE", hounds, repeat, "HOUNDS_MOVE"),
        ],
    )
}

/// The Glory's slimed berserker: it vomits the deck full of slime, pummels
/// four times over, hugs the player weak and itself stronger, and smothers.
fn slimed_berserker_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let hp = asc(AscensionLevel::ToughEnemies, 281, 261);
    monster(
        "MONSTER.SLIMED_BERSERKER",
        hp,
        hp,
        "VOMIT_ICHOR_MOVE",
        vec![
            slime_status("VOMIT_ICHOR_MOVE", 10, "FURIOUS_PUMMELING_MOVE"),
            multi_attack(
                "FURIOUS_PUMMELING_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 5, 4),
                4,
                "LEECHING_HUG_MOVE",
            ),
            MonsterState::Move(MonsterMove {
                state_id: "LEECHING_HUG_MOVE".to_owned(),
                intent: Intent::new(
                    "LEECHING_HUG_MOVE",
                    vec![MoveIntent::of("debuff"), MoveIntent::of("buff")],
                ),
                commands: vec![
                    // The hug names no applier, where the strength it works
                    // on itself does.
                    Command::Power(PowerCmd::Apply {
                        marks: None,
                        power: id("POWER.WEAK_POWER"),
                        target: CreatureSelector::Target,
                        applier: None,
                        amount: Amount::Fixed(3),
                    }),
                    self_power("POWER.STRENGTH_POWER", 3),
                ],
                follow_up: "SMOTHER_MOVE".to_owned(),
            }),
            attack(
                "SMOTHER_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 33, 30),
                "VOMIT_ICHOR_MOVE",
            ),
        ],
    )
}

/// Whether a rat can call for backup, as the branch's own weights ask it.
fn a_rat_can_summon() -> MoveCondition {
    MoveCondition::All(vec![
        // A `TurnsUntilSummonable` of one or more answers no.
        MoveCondition::Not(Box::new(MoveCondition::OwnerLocalAtLeast {
            name: TURNS_UNTIL_SUMMONABLE.to_owned(),
            at_least: 1,
        })),
        // A `CallForBackupCount` of three or more answers no.
        MoveCondition::Not(Box::new(MoveCondition::OwnerLocalAtLeast {
            name: CALL_FOR_BACKUP_COUNT.to_owned(),
            at_least: 3,
        })),
        // The encounter has a free slot to stand one in.
        MoveCondition::EncounterHasAFreeSlot,
        // And no teammate is already about to call.
        MoveCondition::Not(Box::new(MoveCondition::AnAllyIntends(
            "CALL_FOR_BACKUP_MOVE".to_owned(),
        ))),
    ])
}

const TURNS_UNTIL_SUMMONABLE: &str = "TurnsUntilSummonable";
const CALL_FOR_BACKUP_COUNT: &str = "CallForBackupCount";

/// One off `TurnsUntilSummonable`, which every move of a rat's but the call
/// makes.
fn one_turn_closer_to_summoning() -> Command {
    Command::Creature(CreatureCmd::SetLocal {
        target: CreatureSelector::Owner,
        name: TURNS_UNTIL_SUMMONABLE.to_owned(),
        amount: Amount::per(-1, 1, Count::OwnerLocal(TURNS_UNTIL_SUMMONABLE.to_owned())),
    })
}

/// A two-tailed rat: a scratch, a diseased bite, a screech that leaves its
/// quarry frail, and a call for backup that brings another rat into the room.
///
/// `TurnsUntilSummonable` and `CallForBackupCount` are numbers the build
/// carries on the monster; they are written on the creature here, which is the
/// same thing. The first opens at two and every move but the call takes one
/// off it; the second is a count the whole side shares, because every call
/// writes the new highest onto every rat still standing.
///
/// The branch after each move is where all of that is asked. Three of its four
/// weights are a twelfth once a call is possible and one when it is not, and
/// the fourth is three quarters once a call is possible and nothing when it is
/// not, so once a call is possible it is three parts in four and the three
/// attacks share the rest.
fn two_tailed_rat_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let scratch = asc(AscensionLevel::DeadlyEnemies, 9, 8);
    let bite = asc(AscensionLevel::DeadlyEnemies, 7, 6);
    let can_summon = a_rat_can_summon;
    // A twelfth once a call is possible and one when it is not, which is what
    // all three attacks weigh.
    let attacking = |branch: MoveBranch| branch.with_weight_when(can_summon(), 1.0 / 12.0, 1.0);
    joins_with(
        monster(
            "MONSTER.TWO_TAILED_RAT",
            asc(AscensionLevel::ToughEnemies, 18, 17),
            asc(AscensionLevel::ToughEnemies, 22, 21),
            // With no opener named, a rat opens on the branch. The room that
            // stands three of them up names an opener for each, and a rat
            // called into the fight carries none, so it rolls its first move.
            "RAND",
            vec![
                MonsterState::Move(MonsterMove {
                    state_id: "SCRATCH_MOVE".to_owned(),
                    intent: Intent::new("SCRATCH_MOVE", vec![MoveIntent::attack(scratch, 1)]),
                    commands: vec![one_turn_closer_to_summoning(), strike(scratch, 1)],
                    follow_up: "RAND".to_owned(),
                }),
                MonsterState::Move(MonsterMove {
                    state_id: "DISEASE_BITE_MOVE".to_owned(),
                    intent: Intent::new("DISEASE_BITE_MOVE", vec![MoveIntent::attack(bite, 1)]),
                    commands: vec![one_turn_closer_to_summoning(), strike(bite, 1)],
                    follow_up: "RAND".to_owned(),
                }),
                MonsterState::Move(MonsterMove {
                    state_id: "SCREECH_MOVE".to_owned(),
                    intent: Intent::new("SCREECH_MOVE", vec![MoveIntent::of("debuff")]),
                    commands: vec![
                        one_turn_closer_to_summoning(),
                        debuff("POWER.FRAIL_POWER", 1),
                    ],
                    follow_up: "RAND".to_owned(),
                }),
                MonsterState::Move(MonsterMove {
                    state_id: "CALL_FOR_BACKUP_MOVE".to_owned(),
                    intent: Intent::new("CALL_FOR_BACKUP_MOVE", vec![MoveIntent::of("summon")]),
                    commands: vec![
                        // The last slot nothing is standing in, and no free
                        // slot at all is a call that brings nobody.
                        Command::Creature(CreatureCmd::Add {
                            monster: id("MONSTER.TWO_TAILED_RAT"),
                            slot: SpawnSlot::LastFree,
                            initial_state: None,
                            then: Vec::new(),
                        }),
                        // The highest `CallForBackupCount` on the side plus
                        // one, written onto every rat standing, the new one
                        // included. They all carry the same count, so the
                        // highest is the caller's own and one more. It is
                        // written whether or not the call found a slot to put
                        // anybody in.
                        Command::Creature(CreatureCmd::SetLocal {
                            target: CreatureSelector::AlliesOfModel(id("MONSTER.TWO_TAILED_RAT")),
                            name: CALL_FOR_BACKUP_COUNT.to_owned(),
                            amount: Amount::per(
                                1,
                                1,
                                Count::OwnerLocal(CALL_FOR_BACKUP_COUNT.to_owned()),
                            ),
                        }),
                    ],
                    follow_up: "RAND".to_owned(),
                }),
                MonsterState::RandomBranch {
                    state_id: "RAND".to_owned(),
                    branches: vec![
                        attacking(MoveBranch::new("SCRATCH_MOVE", MoveRepeat::CannotRepeat)),
                        attacking(MoveBranch::new(
                            "DISEASE_BITE_MOVE",
                            MoveRepeat::CannotRepeat,
                        )),
                        attacking(
                            MoveBranch::new("SCREECH_MOVE", MoveRepeat::CannotRepeat)
                                .with_cooldown(3),
                        ),
                        MoveBranch::new("CALL_FOR_BACKUP_MOVE", MoveRepeat::UseOnlyOnce)
                            .with_weight_when(can_summon(), 0.75, 0.0),
                    ],
                },
            ],
        ),
        // `TurnsUntilSummonable` opens at two.
        vec![Command::Creature(CreatureCmd::SetLocal {
            target: CreatureSelector::Owner,
            name: TURNS_UNTIL_SUMMONABLE.to_owned(),
            amount: Amount::Fixed(2),
        })],
    )
}

/// The Lagavulin Matriarch, the Underdocks' third boss. It sleeps behind twelve
/// plating until something gets through it, and then works round a slash, a
/// twofold disembowelling, a second slash it guards behind, and a siphon that
/// takes two strength and two dexterity off the player and puts two strength on
/// itself.
///
/// Its arrival is the sleep: twelve Plating and three Asleep. The branch
/// after the sleeping move asks whether the Asleep is still standing, so the
/// matriarch goes on sleeping until it has run out or a blow has taken it off.
///
/// Being awake, and the shell being awake, are the animation's business; what
/// the monster itself answers `AfterDamageReceived` with is opening its eyes
/// at half health, and nothing else.
fn lagavulin_matriarch_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let hp = asc(AscensionLevel::ToughEnemies, 233, 222);
    let slash2 = asc(AscensionLevel::DeadlyEnemies, 14, 12);
    joins_with(
        monster(
            "MONSTER.LAGAVULIN_MATRIARCH",
            hp,
            hp,
            "SLEEP_MOVE",
            vec![
                MonsterState::Move(MonsterMove {
                    state_id: "SLEEP_MOVE".to_owned(),
                    intent: Intent::new("SLEEP_MOVE", vec![MoveIntent::of("sleep")]),
                    commands: Vec::new(),
                    follow_up: "SLEEP_BRANCH".to_owned(),
                }),
                MonsterState::ConditionalBranch {
                    state_id: "SLEEP_BRANCH".to_owned(),
                    branches: vec![
                        ConditionalBranch::new(
                            "SLEEP_MOVE",
                            MoveCondition::OwnerHoldsPower(id("POWER.ASLEEP_POWER")),
                        ),
                        ConditionalBranch::new(
                            "SLASH_MOVE",
                            MoveCondition::Not(Box::new(MoveCondition::OwnerHoldsPower(id(
                                "POWER.ASLEEP_POWER",
                            )))),
                        ),
                    ],
                },
                attack(
                    "SLASH_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 21, 19),
                    "DISEMBOWEL_MOVE",
                ),
                multi_attack(
                    "DISEMBOWEL_MOVE",
                    asc(AscensionLevel::DeadlyEnemies, 10, 9),
                    2,
                    "SLASH2_MOVE",
                ),
                swing_and_guard(
                    "SLASH2_MOVE",
                    slash2,
                    asc(AscensionLevel::ToughEnemies, 14, 12),
                    "SOUL_SIPHON_MOVE",
                ),
                MonsterState::Move(MonsterMove {
                    state_id: "SOUL_SIPHON_MOVE".to_owned(),
                    intent: Intent::new(
                        "SOUL_SIPHON_MOVE",
                        vec![MoveIntent::of("debuff"), MoveIntent::of("buff")],
                    ),
                    commands: vec![
                        debuff("POWER.STRENGTH_POWER", -2),
                        debuff("POWER.DEXTERITY_POWER", -2),
                        self_power("POWER.STRENGTH_POWER", 2),
                    ],
                    follow_up: "SLASH_MOVE".to_owned(),
                }),
            ],
        ),
        // The sleep it stands up in.
        vec![
            self_power("POWER.PLATING_POWER", 12),
            self_power("POWER.ASLEEP_POWER", 3),
        ],
    )
}

/// The fake merchant, which an event turns on the player: a swipe, eight coins
/// spat two at a time, a relic thrown that leaves its target frail, and a rage.
///
/// It rolls between the four after every move but the throw, and after a throw
/// it rolls between the three attacks alone. The rage sits on a cooldown of
/// three moves.
fn fake_merchant_monster_at(ascension: u8) -> MonsterDefinition {
    let asc = branch_at(ascension);
    let hp = asc(AscensionLevel::ToughEnemies, 175, 165);
    let throw = asc(AscensionLevel::DeadlyEnemies, 10, 9);
    let attacks = || {
        vec![
            MoveBranch::new("SWIPE_MOVE", MoveRepeat::CannotRepeat),
            MoveBranch::new("SPEW_COINS_MOVE", MoveRepeat::CannotRepeat),
            MoveBranch::new("THROW_RELIC_MOVE", MoveRepeat::CannotRepeat),
        ]
    };
    let mut everything = attacks();
    everything.push(MoveBranch::new("ENRAGE_MOVE", MoveRepeat::CannotRepeat).with_cooldown(3));
    monster(
        "MONSTER.FAKE_MERCHANT_MONSTER",
        hp,
        hp,
        "SWIPE_MOVE",
        vec![
            attack(
                "SWIPE_MOVE",
                asc(AscensionLevel::DeadlyEnemies, 15, 13),
                "RAND_MOVE",
            ),
            multi_attack("SPEW_COINS_MOVE", 2, 8, "RAND_MOVE"),
            MonsterState::Move(MonsterMove {
                state_id: "THROW_RELIC_MOVE".to_owned(),
                intent: Intent::new(
                    "THROW_RELIC_MOVE",
                    vec![MoveIntent::attack(throw, 1), MoveIntent::of("debuff")],
                ),
                commands: vec![strike(throw, 1), debuff("POWER.FRAIL_POWER", 1)],
                // A throw is the one move that rolls between the attacks alone.
                follow_up: "RAND_ATTACK_MOVE".to_owned(),
            }),
            self_buff("ENRAGE_MOVE", 2, "RAND_MOVE"),
            MonsterState::RandomBranch {
                state_id: "RAND_MOVE".to_owned(),
                branches: everything,
            },
            MonsterState::RandomBranch {
                state_id: "RAND_ATTACK_MOVE".to_owned(),
                branches: attacks(),
            },
        ],
    )
}

/// A battle friend: one of the three dummies the Battleworn Dummy event stands
/// up. It never acts — one hidden move that does nothing and follows itself —
/// and what it is for is the three turns its arrival puts on it.
fn battle_friend(model_id: &str, hp: i32) -> MonsterDefinition {
    joins_with(
        monster(
            model_id,
            hp,
            hp,
            "NOTHING_MOVE",
            vec![MonsterState::Move(MonsterMove {
                state_id: "NOTHING_MOVE".to_owned(),
                intent: Intent::new("NOTHING_MOVE", Vec::new()),
                commands: Vec::new(),
                follow_up: "NOTHING_MOVE".to_owned(),
            })],
        ),
        vec![Command::Power(PowerCmd::Apply {
            marks: None,
            power: id("POWER.BATTLEWORN_DUMMY_TIME_LIMIT_POWER"),
            target: CreatureSelector::Owner,
            // The event applies the timer without an applier, even though the
            // target is the arriving monster.
            applier: None,
            amount: Amount::Fixed(3),
        })],
    )
}

/// The event's first setting: seventy-five health.
#[must_use]
pub fn battle_friend_v1() -> MonsterDefinition {
    battle_friend("MONSTER.BATTLE_FRIEND_V1", 75)
}

/// Its second: a hundred and fifty.
#[must_use]
pub fn battle_friend_v2() -> MonsterDefinition {
    battle_friend("MONSTER.BATTLE_FRIEND_V2", 150)
}

/// Its third: three hundred.
#[must_use]
pub fn battle_friend_v3() -> MonsterDefinition {
    battle_friend("MONSTER.BATTLE_FRIEND_V3", 300)
}

/// One Adversary, on the machine all three of them share: a blow, a heavier
/// blow, and a barrage that leaves it stronger. They differ in size, in what
/// each of the three is worth, and in the artifact each stands up behind.
///
/// No encounter of the pinned build fields one, so nothing in a run ever stands
/// one up.
fn adversary(
    model_id: &str,
    hp: i32,
    artifact: i32,
    first: (&str, i32),
    second: (&str, i32),
    barrage: (i32, i32),
) -> MonsterDefinition {
    let (barrage_damage, barrage_strength) = barrage;
    joins_with(
        monster(
            model_id,
            hp,
            hp,
            first.0,
            vec![
                attack(first.0, first.1, second.0),
                attack(second.0, second.1, "BARRAGE_MOVE"),
                MonsterState::Move(MonsterMove {
                    state_id: "BARRAGE_MOVE".to_owned(),
                    intent: Intent::new(
                        "BARRAGE_MOVE",
                        vec![
                            MoveIntent::attack(barrage_damage, 2),
                            MoveIntent::of("buff"),
                        ],
                    ),
                    commands: vec![
                        strike(barrage_damage, 2),
                        self_power("POWER.STRENGTH_POWER", barrage_strength),
                    ],
                    follow_up: first.0.to_owned(),
                }),
            ],
        ),
        vec![self_power("POWER.ARTIFACT_POWER", artifact)],
    )
}

/// The Adversary Mk One: a hundred health, no artifact at all, and a smash, a
/// beam and a barrage worth two strength.
#[must_use]
pub fn the_adversary_mk_one() -> MonsterDefinition {
    adversary(
        "MONSTER.THE_ADVERSARY_MK_ONE",
        100,
        // Artifact worked on with no stacks at all.
        0,
        ("SMASH_MOVE", 12),
        ("BEAM_MOVE", 15),
        (8, 2),
    )
}

/// The Mk Two: two hundred health, one artifact, and heavier all round.
///
/// The flame beam's status count is a number the model declares and nothing
/// in a fight ever asks for.
fn the_adversary_mk_two_at(ascension: u8) -> MonsterDefinition {
    adversary(
        "MONSTER.THE_ADVERSARY_MK_TWO",
        // The branch is kept as the build has it, and both of its numbers are
        // two hundred.
        branch_at(ascension)(AscensionLevel::ToughEnemies, 200, 200),
        1,
        ("BASH_MOVE", 13),
        ("FLAME_BEAM_MOVE", 16),
        (9, 3),
    )
}

/// The Mk Three: three hundred health, two artifact, and heavier again.
fn the_adversary_mk_three_at(ascension: u8) -> MonsterDefinition {
    adversary(
        "MONSTER.THE_ADVERSARY_MK_THREE",
        branch_at(ascension)(AscensionLevel::ToughEnemies, 300, 300),
        2,
        ("CRASH_MOVE", 15),
        ("FLAME_BEAM_MOVE", 18),
        (10, 4),
    )
}

/// A monster whose whole machine is one hidden move that does nothing and
/// follows itself, which is what the build's own sizing dummies are.
fn sizing_dummy(model_id: &str, hp: i32) -> MonsterDefinition {
    monster(
        model_id,
        hp,
        hp,
        "NOTHING",
        vec![MonsterState::Move(MonsterMove {
            state_id: "NOTHING".to_owned(),
            intent: Intent::new("NOTHING", Vec::new()),
            commands: Vec::new(),
            follow_up: "NOTHING".to_owned(),
        })],
    )
}

/// The big dummy: nine thousand nine hundred and ninety-nine health and nothing
/// else. Only the build's own test rooms stand it up.
#[must_use]
pub fn big_dummy() -> MonsterDefinition {
    sizing_dummy("MONSTER.BIG_DUMMY", 9999)
}

/// The one-health dummy.
#[must_use]
pub fn one_hp_monster() -> MonsterDefinition {
    sizing_dummy("MONSTER.ONE_HP_MONSTER", 1)
}

/// The ten-health dummy.
#[must_use]
pub fn ten_hp_monster() -> MonsterDefinition {
    sizing_dummy("MONSTER.TEN_HP_MONSTER", 10)
}

/// A dummy that pokes for one, over and over.
#[must_use]
pub fn single_attack_move_monster() -> MonsterDefinition {
    monster(
        "MONSTER.SINGLE_ATTACK_MOVE_MONSTER",
        999,
        999,
        "POKE",
        vec![attack("POKE", 1, "POKE")],
    )
}

/// The same, five pokes at a time.
#[must_use]
pub fn multi_attack_move_monster() -> MonsterDefinition {
    monster(
        "MONSTER.MULTI_ATTACK_MOVE_MONSTER",
        999,
        999,
        "POKE",
        vec![multi_attack("POKE", 1, 5, "POKE")],
    )
}

/// Every monster this baseline registers, built for a run at `ascension`.
///
/// Which side of a branch a monster's numbers take is decided by the run
/// standing it up rather than by the model. That is what this list is: the
/// registry's
/// monsters built at the level the run is played at, so a model that branches
/// reads the side its own run is on. Fourteen of them carry no branch at all
/// and take no level, which is what the two forms below say.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one line per monster this baseline registers"
)]
pub fn monsters_at_ascension(ascension: u8) -> Vec<MonsterDefinition> {
    vec![
        osty(),
        architect(),
        byrdpip(),
        paels_legion(),
        fossil_stalker_at(ascension),
        mecha_knight_at(ascension),
        punch_construct_at(ascension),
        skulking_colony_at(ascension),
        terror_eel_at(ascension),
        shrinker_beetle_at(ascension),
        twig_slime_s_at(ascension),
        twig_slime_m_at(ascension),
        leaf_slime_s_at(ascension),
        leaf_slime_m_at(ascension),
        nibbit_at(ascension),
        flyconid_at(ascension),
        byrdonis_at(ascension),
        inklet_at(ascension),
        fogmog_at(ascension),
        eye_with_teeth(),
        cubex_construct_at(ascension),
        mawler_at(ascension),
        slithering_strangler_at(ascension),
        phrog_parasite_at(ascension),
        wriggler_at(ascension),
        ovicopter_at(ascension),
        tough_egg_at(ascension),
        the_insatiable_at(ascension),
        axebot_at(ascension),
        fabricator_at(ascension),
        test_subject_at(ascension),
        zapbot_at(ascension),
        stabbot_at(ascension),
        guardbot_at(ascension),
        noisebot_at(ascension),
        the_lost_at(ascension),
        the_forgotten_at(ascension),
        kin_priest_at(ascension),
        kin_follower_at(ascension),
        thieving_hopper_at(ascension),
        slumbering_beetle_at(ascension),
        exoskeleton_at(ascension),
        hunter_killer_at(ascension),
        chomper_at(ascension),
        bowlbug_rock_at(ascension),
        bowlbug_egg_at(ascension),
        snapping_jaxfruit_at(ascension),
        vine_shambler_at(ascension),
        vantom_at(ascension),
        louse_progenitor_at(ascension),
        infested_prism_at(ascension),
        knowledge_demon_at(ascension),
        turret_operator_at(ascension),
        living_shield_at(ascension),
        the_obscura_at(ascension),
        parafright_at(ascension),
        bowlbug_silk_at(ascension),
        bowlbug_nectar_at(ascension),
        myte_at(ascension),
        crusher_at(ascension),
        rocket_at(ascension),
        fuzzy_wurm_crawler_at(ascension),
        bygone_effigy_at(ascension),
        ceremonial_beast_at(ascension),
        tunneler_at(ascension),
        spiny_toad_at(ascension),
        frog_knight_at(ascension),
        flail_knight_at(ascension),
        mysterious_knight_at(ascension),
        entomancer_at(ascension),
        globe_head_at(ascension),
        aeonglass_at(ascension),
        toadpole_at(ascension),
        corpse_slug_at(ascension),
        sludge_spinner_at(ascension),
        calcified_cultist_at(ascension),
        damp_cultist_at(ascension),
        phantasmal_gardener_at(ascension),
        living_fog_at(ascension),
        gas_bomb_at(ascension),
        haunted_ship_at(ascension),
        seapunk_at(ascension),
        sewer_clam_at(ascension),
        gremlin_merc_at(ascension),
        fat_gremlin_at(ascension),
        sneaky_gremlin_at(ascension),
        soul_fysh_at(ascension),
        waterfall_giant_at(ascension),
        spectral_knight_at(ascension),
        magi_knight_at(ascension),
        owl_magistrate_at(ascension),
        soul_nexus_at(ascension),
        torch_head_amalgam_at(ascension),
        queen_at(ascension),
        devoted_sculptor_at(ascension),
        scroll_of_biting_at(ascension),
        decimillipede_segment_front_at(ascension),
        decimillipede_segment_middle_at(ascension),
        decimillipede_segment_back_at(ascension),
        assassin_ruby_raider_at(ascension),
        axe_ruby_raider_at(ascension),
        brute_ruby_raider_at(ascension),
        crossbow_ruby_raider_at(ascension),
        tracker_ruby_raider_at(ascension),
        slimed_berserker_at(ascension),
        two_tailed_rat_at(ascension),
        lagavulin_matriarch_at(ascension),
        fake_merchant_monster_at(ascension),
        battle_friend_v1(),
        battle_friend_v2(),
        battle_friend_v3(),
        the_adversary_mk_one(),
        the_adversary_mk_two_at(ascension),
        the_adversary_mk_three_at(ascension),
        big_dummy(),
        one_hp_monster(),
        ten_hp_monster(),
        single_attack_move_monster(),
        multi_attack_move_monster(),
    ]
}
