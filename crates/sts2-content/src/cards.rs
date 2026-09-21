//! Ironclad card definitions.
//!
//! A card is the run of commands its play issues, in order. It supports its
//! base form, and its upgraded form where one is defined.

use sts2_engine::{
    Amount, CardCmd, CardDefinition, CardFilter, CardOrigin, CardPileCmd, CardSelector, CardTag,
    CardType, CardUpgrade, Command, Condition, Count, CreatureCmd, CreatureSelector, DamageCmd,
    PileName, PlayerCmd, PowerCmd, PowerRef, TargetType, ValueProps,
};

use crate::id;

pub(crate) fn card(
    model_id: &str,
    card_type: CardType,
    target: TargetType,
    energy_cost: i32,
    effects: Vec<Command>,
) -> CardDefinition {
    CardDefinition {
        model_id: id(model_id),
        // What a character starts with is basic; the pool sets the rest as the
        // registry is built.
        rarity: sts2_engine::CardRarity::Basic,
        ethereal: false,
        eternal: false,
        unplayable: false,
        end_of_turn_in_hand: Vec::new(),
        tags: Vec::new(),
        upgrade: None,
        supported_upgrade_levels: vec![0],
        allow_custom_properties: false,
        gains_block: false,
        saved_properties: Vec::new(),
        quest_hooks: None,
        map_hooks: None,
        type_from_property: None,
        card_type,
        target,
        target_widens_with: None,
        energy_cost,
        // A star cost of -1 is what a card that costs no stars carries, and
        // a card that does cost them says so where it is written.
        star_cost: -1,
        costs_star_x: false,
        returns_to_hand: false,
        costs_x: false,
        innate: false,
        retain: false,
        sly: false,
        exhausts: false,
        play_restriction: None,
        playable_when: None,
        damage_var: None,
        effects,
        // Which pool the card stands in, and which one a transform of it rolls
        // against, are both settled as the registry is built, where the pool
        // tables are.
        transform_pool: None,
        visual_pool: None,
        adds_rest_site_option: None,
        hooks: Vec::new(),
    }
}

/// Whether a card gains block, which every card declares for itself rather
/// than anything worked out from what it does: a card that says yes is one a
/// Nimble enchantment will go on, and nothing else asks.
///
/// It is a table here for the same reason a card's rarity is: the answer
/// belongs to the card, and gathering the ones that say yes in one place is
/// what makes the set checkable against the build.
///
/// Seventy-six cards of the build answer yes outright; two more — Mad Science
/// and Sovereign Blade — answer from the state of the fight, and neither is
/// registered here. What is listed is every one of the seventy-six this
/// baseline registers, which is all of them but five: a demonic shield, an
/// intercept, a lift, a mimic and a rally are multiplayer-only cards, and a
/// single-player run's pools never hold one.
pub(crate) fn gains_block(model_id: &str) -> bool {
    matches!(
        model_id,
        "CARD.ARMAMENTS"
            | "CARD.BACKFLIP"
            | "CARD.BLOOD_WALL"
            | "CARD.BLUR"
            | "CARD.BONE_SHARDS"
            | "CARD.BOOST_AWAY"
            | "CARD.BOOT_SEQUENCE"
            | "CARD.BULWARK"
            | "CARD.CHARGE_BATTERY"
            | "CARD.CLOAK_AND_DAGGER"
            | "CARD.CLOAK_OF_STARS"
            | "CARD.COLOSSUS"
            | "CARD.COMPACT"
            | "CARD.COSMIC_INDIFFERENCE"
            | "CARD.DASH"
            | "CARD.DEATHS_DOOR"
            | "CARD.DEFEND_DEFECT"
            | "CARD.DEFEND_IRONCLAD"
            | "CARD.DEFEND_NECROBINDER"
            | "CARD.DEFEND_REGENT"
            | "CARD.DEFEND_SILENT"
            | "CARD.DEFLECT"
            | "CARD.DEFY"
            | "CARD.DELAY"
            | "CARD.DODGE_AND_ROLL"
            | "CARD.ENTRENCH"
            | "CARD.EQUILIBRIUM"
            | "CARD.ESCAPE_PLAN"
            | "CARD.EVIL_EYE"
            | "CARD.FIGHT_THROUGH"
            | "CARD.FINESSE"
            | "CARD.FISTICUFFS"
            | "CARD.FLAME_BARRIER"
            | "CARD.GATHER_LIGHT"
            | "CARD.GENETIC_ALGORITHM"
            | "CARD.GLACIER"
            | "CARD.GLASSWORK"
            | "CARD.GLITTERSTREAM"
            | "CARD.GRAVE_WARDEN"
            | "CARD.HAND_TRICK"
            | "CARD.HOLOGRAM"
            | "CARD.IMPERVIOUS"
            | "CARD.IRON_WAVE"
            | "CARD.I_AM_INVINCIBLE"
            | "CARD.LEAP"
            | "CARD.LEG_SWEEP"
            | "CARD.LIGHTNING_ROD"
            | "CARD.MANIFEST_AUTHORITY"
            | "CARD.MELANCHOLY"
            | "CARD.MINION_SACRIFICE"
            | "CARD.MIRAGE"
            | "CARD.NEGATIVE_PULSE"
            | "CARD.PANIC_BUTTON"
            | "CARD.PARTICLE_WALL"
            | "CARD.PATTER"
            | "CARD.PULL_AGGRO"
            | "CARD.REFLECT"
            | "CARD.RELAX"
            | "CARD.SACRIFICE"
            | "CARD.SECOND_WIND"
            | "CARD.SHADOW_SHIELD"
            | "CARD.SHRUG_IT_OFF"
            | "CARD.STACK"
            | "CARD.SURVIVOR"
            | "CARD.TAUNT"
            | "CARD.THE_GAMBIT"
            | "CARD.TORIC_TOUGHNESS"
            | "CARD.TRUE_GRIT"
            | "CARD.ULTIMATE_DEFEND"
            | "CARD.UNDEATH"
            | "CARD.UNTOUCHABLE"
    )
}

/// Two hits, strength for its owner, and one strength for the target.
fn fight_me(damage: i32, strength: i32) -> Vec<Command> {
    vec![
        multi_attack(Amount::Fixed(damage), Amount::Fixed(2)),
        buff("POWER.STRENGTH_POWER", strength),
        apply_power(
            "POWER.STRENGTH_POWER",
            CreatureSelector::Target,
            Amount::Fixed(1),
        ),
    ]
}

/// Every attack in hand becomes a Giant Rock in its place.
fn into_rocks(upgraded: bool) -> Command {
    Command::Card(CardCmd::Transform {
        cards: CardSelector::Pile {
            pile: PileName::Hand,
            filter: CardFilter::OfType(CardType::Attack),
        },
        into: id("CARD.GIANT_ROCK"),
        upgraded,
    })
}

/// An attack on whoever the play was aimed at, landing once.
pub(crate) fn attack(amount: i32) -> Command {
    multi_attack(Amount::Fixed(amount), Amount::Fixed(1))
}

/// An attack whose damage or hit count is read off the state it lands in.
pub(crate) fn multi_attack(amount: Amount, hits: Amount) -> Command {
    Command::Damage(DamageCmd::Attack {
        amount,
        hits,
        target: CreatureSelector::Target,
        props: ValueProps::NONE,
    })
}

/// Block the card's owner works up, which listeners may scale.
pub(crate) fn block(amount: i32) -> Command {
    Command::Creature(CreatureCmd::GainBlock {
        target: CreatureSelector::Owner,
        amount: Amount::Fixed(amount),
        props: ValueProps::NONE,
    })
}

pub(crate) fn apply_power(power: &str, target: CreatureSelector, amount: Amount) -> Command {
    Command::Power(PowerCmd::Apply {
        marks: None,
        power: id(power),
        target,
        applier: Some(CreatureSelector::Owner),
        amount,
    })
}

/// A debuff worked onto whoever the play was aimed at.
pub(crate) fn debuff(power: &str, amount: i32) -> Command {
    apply_power(power, CreatureSelector::Target, Amount::Fixed(amount))
}

/// A buff the card's owner works on themselves.
pub(crate) fn buff(power: &str, amount: i32) -> Command {
    apply_power(power, CreatureSelector::Owner, Amount::Fixed(amount))
}

pub(crate) fn draw(count: i32) -> Command {
    Command::CardPile(CardPileCmd::Draw {
        count: Amount::Fixed(count),
    })
}

pub(crate) fn exhausting(mut definition: CardDefinition) -> CardDefinition {
    definition.exhausts = true;
    definition
}

/// An upgrade that makes the card cheaper rather than stronger.
pub(crate) fn upgrading_cost(mut definition: CardDefinition, energy_cost: i32) -> CardDefinition {
    definition.supported_upgrade_levels.push(1);
    let effects = definition.effects.clone();
    definition.upgrade = Some(CardUpgrade {
        energy_cost: Some(energy_cost),
        innate: false,
        retain: false,
        drops_exhaust: false,
        drops_ethereal: false,
        damage_var: None,
        effects,
    });
    definition
}

/// Gives a card its upgraded form, which changes what it does rather than
/// simply scaling it.
pub(crate) fn upgrading(mut definition: CardDefinition, effects: Vec<Command>) -> CardDefinition {
    definition.supported_upgrade_levels.push(1);
    definition.upgrade = Some(CardUpgrade {
        energy_cost: None,
        innate: false,
        retain: false,
        drops_exhaust: false,
        drops_ethereal: false,
        damage_var: None,
        effects,
    });
    definition
}

/// What the card costs of the Regent's own second resource, which is -1 on
/// every card that costs none.
pub(crate) fn costing_stars(mut definition: CardDefinition, stars: i32) -> CardDefinition {
    definition.star_cost = stars;
    definition
}

/// The card's star cost is X, so playing it spends the whole pool and what it
/// does is worked out from how much was spent.
pub(crate) fn costing_star_x(mut definition: CardDefinition) -> CardDefinition {
    definition.costs_star_x = true;
    definition
}

/// A play that would send the card to the discard pile leaves it in the hand
/// instead.
pub(crate) fn kept_in_hand(mut definition: CardDefinition) -> CardDefinition {
    definition.returns_to_hand = true;
    definition
}

/// The card's cost is X. What it is priced at is zero whatever the card says,
/// so it can always be played, and the play spends every point of energy its
/// owner has left.
pub(crate) fn costing_x(mut definition: CardDefinition) -> CardDefinition {
    definition.costs_x = true;
    definition
}

/// A card whose whole upgrade is becoming innate.
pub(crate) fn innate_when_upgraded(mut definition: CardDefinition) -> CardDefinition {
    if definition.upgrade.is_none() {
        definition.supported_upgrade_levels.push(1);
        definition.upgrade = Some(CardUpgrade {
            energy_cost: None,
            innate: true,
            retain: false,
            drops_exhaust: false,
            drops_ethereal: false,
            damage_var: None,
            effects: definition.effects.clone(),
        });
    } else if let Some(upgrade) = definition.upgrade.as_mut() {
        upgrade.innate = true;
    }
    definition
}

/// Cards the Strike-counting rules look for.
pub(crate) fn striking(mut definition: CardDefinition) -> CardDefinition {
    definition.tags.push(CardTag::Strike);
    definition
}

/// Cards the Defend-counting rules look for.
pub(crate) fn defending(mut definition: CardDefinition) -> CardDefinition {
    definition.tags.push(CardTag::Defend);
    definition
}

#[must_use]
pub fn ironclad_cards() -> Vec<CardDefinition> {
    let mut cards = starting_cards();
    cards.extend(later_attacks());
    cards.extend(hive_attacks());
    cards.extend(later_skills());
    cards.extend(status_cards());
    cards.extend(overgrowth_pickups());
    cards.extend(pool_attacks());
    cards.extend(pool_skills());
    cards.extend(pool_powers());
    cards.extend(burning_cards());
    cards.extend(x_cost_cards());
    cards.extend(pool_listeners_and_piles());
    cards.extend(pool_remainder());
    cards
}

/// A card the play puts a screen up for: the selection is a first-class
/// decision, and the commands after it act on what came back.
pub(crate) fn choosing_from_hand(prefs: sts2_engine::SelectorPrefs, then: Vec<Command>) -> Command {
    Command::CardSelect(sts2_engine::CardSelectCmd::FromHand {
        maximum: None,
        exact_count: None,
        prefs,
        filter: CardFilter::Any,
        then,
    })
}

/// A pick of exactly one card of the hand and the burn that follows it: the
/// card is taken without a screen when there is only one to take.
fn exhaust_one_of_hand() -> Command {
    choosing_from_hand(
        sts2_engine::SelectorPrefs::exactly(1),
        vec![Command::Card(CardCmd::Exhaust {
            cards: CardSelector::Chosen,
        })],
    )
}

/// An attack aimed at everything on the other side.
pub(crate) fn sweep(amount: i32, hits: i32) -> Command {
    Command::Damage(DamageCmd::Attack {
        amount: Amount::Fixed(amount),
        hits: Amount::Fixed(hits),
        target: CreatureSelector::AllOpponents,
        props: ValueProps::NONE,
    })
}

/// The attacks of the Ironclad's pool that this baseline transcribes in full.
fn pool_attacks() -> Vec<CardDefinition> {
    vec![
        // A breakthrough costs its owner a point of their own before it lands
        // on everything in front of them.
        upgrading(
            card(
                "CARD.BREAKTHROUGH",
                CardType::Attack,
                TargetType::AllEnemies,
                1,
                vec![self_hp_loss(1), sweep(9, 1)],
            ),
            vec![self_hp_loss(1), sweep(13, 1)],
        ),
        // Small blows, four of them, over everything: upgrading buys a fifth
        // rather than a heavier one.
        upgrading(
            card(
                "CARD.CONFLAGRATION",
                CardType::Attack,
                TargetType::AllEnemies,
                1,
                vec![sweep(2, 4)],
            ),
            vec![sweep(2, 5)],
        ),
        // A thunderclap lands on everything and leaves everything shaken.
        upgrading(
            card(
                "CARD.THUNDERCLAP",
                CardType::Attack,
                TargetType::AllEnemies,
                1,
                thunderclap(4),
            ),
            thunderclap(7),
        ),
        // The whole hand burns, and the blow lands once for every card that
        // did. The count is taken before the burning, which is why an empty
        // hand still swings nothing.
        exhausting(upgrading(
            card(
                "CARD.FIEND_FIRE",
                CardType::Attack,
                TargetType::AnyEnemy,
                2,
                fiend_fire(7),
            ),
            fiend_fire(10),
        )),
        // A mangling leaves the target weaker for the turn: the power hands the
        // strength back as the turn ends.
        upgrading(
            card(
                "CARD.MANGLE",
                CardType::Attack,
                TargetType::AnyEnemy,
                3,
                vec![attack(15), debuff("POWER.MANGLE_POWER", 10)],
            ),
            vec![attack(20), debuff("POWER.MANGLE_POWER", 15)],
        ),
        // A setup strike leaves its owner stronger for the turn.
        striking(upgrading(
            card(
                "CARD.SETUP_STRIKE",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![attack(7), buff("POWER.SETUP_STRIKE_POWER", 2)],
            ),
            vec![attack(9), buff("POWER.SETUP_STRIKE_POWER", 3)],
        )),
        // A pact's end asks what has burned up: three cards in the exhaust pile
        // and it lands, fewer and it does nothing at all.
        upgrading(
            card(
                "CARD.PACTS_END",
                CardType::Attack,
                TargetType::AllEnemies,
                0,
                pacts_end(17),
            ),
            pacts_end(23),
        ),
    ]
}

/// Thunderclap: the sweep, and then the shaking worked onto everything still
/// standing on the other side.
fn thunderclap(damage: i32) -> Vec<Command> {
    vec![
        sweep(damage, 1),
        apply_power(
            "POWER.VULNERABLE_POWER",
            CreatureSelector::AllOpponents,
            Amount::Fixed(1),
        ),
    ]
}

/// Fiend Fire: the hand is taken whole, every card of it burns, and the
/// attack lands once per card that was taken.
fn fiend_fire(damage: i32) -> Vec<Command> {
    vec![Command::Select {
        cards: CardSelector::Pile {
            pile: PileName::Hand,
            filter: CardFilter::Any,
        },
        then: vec![
            Command::Card(CardCmd::Exhaust {
                cards: CardSelector::Chosen,
            }),
            Command::Damage(DamageCmd::Attack {
                amount: Amount::Fixed(damage),
                hits: Amount::counting(Count::ChosenCards),
                target: CreatureSelector::Target,
                props: ValueProps::NONE,
            }),
        ],
    }]
}

/// Pact's End only deals its damage when the exhaust pile holds as many cards
/// as the card's own card count, which is three at either level.
fn pacts_end(damage: i32) -> Vec<Command> {
    vec![Command::When {
        otherwise: Vec::new(),
        condition: Condition::CardsInPileAtLeast {
            pile: PileName::Exhaust,
            count: 3,
        },
        then: vec![sweep(damage, 1)],
    }]
}

/// The skills of the Ironclad's pool that this baseline transcribes in full.
#[allow(clippy::too_many_lines, reason = "one entry per card")]
fn pool_skills() -> Vec<CardDefinition> {
    vec![
        // Block, and then an upgrade: one card of the hand picked out of it, or
        // every upgradable card in it once the armaments themselves are
        // upgraded.
        upgrading(
            card(
                "CARD.ARMAMENTS",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![
                    block(5),
                    Command::CardSelect(sts2_engine::CardSelectCmd::FromHandForUpgrade {
                        then: vec![Command::Card(CardCmd::Upgrade {
                            cards: CardSelector::Chosen,
                        })],
                    }),
                ],
            ),
            vec![
                block(5),
                Command::Card(CardCmd::Upgrade {
                    cards: CardSelector::Pile {
                        pile: PileName::Hand,
                        filter: CardFilter::Upgradable,
                    },
                }),
            ],
        ),
        // Two of its owner's own HP, and then the wall.
        upgrading(
            card(
                "CARD.BLOOD_WALL",
                CardType::Skill,
                TargetType::SelfTarget,
                2,
                vec![self_hp_loss(2), block(16)],
            ),
            vec![self_hp_loss(2), block(20)],
        ),
        // Three HP for two energy, three once upgraded.
        upgrading(
            card(
                "CARD.BLOODLETTING",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![self_hp_loss(3), energy(2)],
            ),
            vec![self_hp_loss(3), energy(3)],
        ),
        // A point of blood, a card off the hand, and the strength it buys.
        upgrading(
            card(
                "CARD.BRAND",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![
                    self_hp_loss(1),
                    exhaust_one_of_hand(),
                    buff("POWER.STRENGTH_POWER", 1),
                ],
            ),
            vec![
                self_hp_loss(1),
                exhaust_one_of_hand(),
                buff("POWER.STRENGTH_POWER", 2),
            ],
        ),
        // A card off the hand paid for cards off the pile.
        upgrading(
            card(
                "CARD.BURNING_PACT",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![exhaust_one_of_hand(), draw(2)],
            ),
            vec![exhaust_one_of_hand(), draw(3)],
        ),
        exhausting(upgrading(
            card(
                "CARD.IMPERVIOUS",
                CardType::Skill,
                TargetType::SelfTarget,
                2,
                vec![block(30)],
            ),
            vec![block(40)],
        )),
        exhausting(upgrading(
            card(
                "CARD.NOT_YET",
                CardType::Skill,
                TargetType::SelfTarget,
                2,
                vec![heal(10)],
            ),
            vec![heal(13)],
        )),
        upgrading(
            card(
                "CARD.SHRUG_IT_OFF",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![block(8), draw(1)],
            ),
            vec![block(11), draw(1)],
        ),
        // True grit burns a card of the hand: whichever one the roll picks, or
        // whichever one the player picks once it is upgraded.
        upgrading(
            card(
                "CARD.TRUE_GRIT",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![
                    block(7),
                    Command::Card(CardCmd::Exhaust {
                        cards: CardSelector::RandomFromPile {
                            pile: PileName::Hand,
                            filter: CardFilter::Any,
                            fallback: None,
                        },
                    }),
                ],
            ),
            vec![block(9), exhaust_one_of_hand()],
        ),
        // A colossus stands behind block and halves what a shaken attacker gets
        // through it, for one enemy turn.
        upgrading(
            card(
                "CARD.COLOSSUS",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![block(5), buff("POWER.COLOSSUS_POWER", 1)],
            ),
            vec![block(8), buff("POWER.COLOSSUS_POWER", 1)],
        ),
        // A barrier of block that burns whoever swings through it.
        upgrading(
            card(
                "CARD.FLAME_BARRIER",
                CardType::Skill,
                TargetType::SelfTarget,
                2,
                vec![block(12), buff("POWER.FLAME_BARRIER_POWER", 4)],
            ),
            vec![block(16), buff("POWER.FLAME_BARRIER_POWER", 6)],
        ),
    ]
}

/// The powers of the Ironclad's pool that this baseline transcribes in full.
fn pool_powers() -> Vec<CardDefinition> {
    vec![
        // A barricade keeps the block its owner is standing behind, and
        // upgrading only makes it cheaper.
        upgrading_cost(
            card(
                "CARD.BARRICADE",
                CardType::Power,
                TargetType::SelfTarget,
                3,
                vec![buff("POWER.BARRICADE_POWER", 1)],
            ),
            2,
        ),
        // Cruelty does nothing of its own: it makes every shaking its owner
        // works count for a quarter more, which Vulnerable reads off whoever
        // is swinging.
        upgrading(
            card(
                "CARD.CRUELTY",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.CRUELTY_POWER", 25)],
            ),
            vec![buff("POWER.CRUELTY_POWER", 50)],
        ),
        upgrading(
            card(
                "CARD.DEMON_FORM",
                CardType::Power,
                TargetType::SelfTarget,
                3,
                vec![buff("POWER.DEMON_FORM_POWER", 2)],
            ),
            vec![buff("POWER.DEMON_FORM_POWER", 3)],
        ),
        upgrading(
            card(
                "CARD.PYRE",
                CardType::Power,
                TargetType::SelfTarget,
                2,
                vec![buff("POWER.PYRE_POWER", 1)],
            ),
            vec![buff("POWER.PYRE_POWER", 2)],
        ),
        upgrading(
            card(
                "CARD.FEEL_NO_PAIN",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.FEEL_NO_PAIN_POWER", 3)],
            ),
            vec![buff("POWER.FEEL_NO_PAIN_POWER", 4)],
        ),
        // Upgrading a dark embrace only makes it cheaper.
        upgrading_cost(
            card(
                "CARD.DARK_EMBRACE",
                CardType::Power,
                TargetType::SelfTarget,
                2,
                vec![buff("POWER.DARK_EMBRACE_POWER", 1)],
            ),
            1,
        ),
    ]
}

/// The last of the pool: the cards that read the fight's own history, roll
/// cards out of the character's pool, or grow as they are played.
#[allow(clippy::too_many_lines, reason = "one entry per card")]
fn pool_remainder() -> Vec<CardDefinition> {
    vec![
        // A feed grows its owner if the blow kills, and asks first whether that
        // death would count — a minion's does not.
        exhausting(upgrading(
            card(
                "CARD.FEED",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                feed(10, 3),
            ),
            feed(12, 4),
        )),
        // A tear asunder lands once, and once more for every blow its owner has
        // already taken this fight that block did not stop whole.
        upgrading(
            card(
                "CARD.TEAR_ASUNDER",
                CardType::Attack,
                TargetType::AnyEnemy,
                2,
                vec![multi_attack(
                    Amount::Fixed(5),
                    Amount::per(1, 1, Count::OwnerUnblockedBlowsTaken),
                )],
            ),
            vec![multi_attack(
                Amount::Fixed(7),
                Amount::per(1, 1, Count::OwnerUnblockedBlowsTaken),
            )],
        ),
        // A rampage grows itself every time it is played.
        with_damage_var(
            upgrading(
                card(
                    "CARD.RAMPAGE",
                    CardType::Attack,
                    TargetType::AnyEnemy,
                    1,
                    rampage(5),
                ),
                rampage(9),
            ),
            9,
            9,
        ),
        // A stomp costs one less for every attack its owner has already played
        // this turn, and one less again for each one played while it is held.
        stomp(),
        upgrading(
            card(
                "CARD.RUPTURE",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.RUPTURE_POWER", 1)],
            ),
            vec![buff("POWER.RUPTURE_POWER", 2)],
        ),
        upgrading_cost(
            card(
                "CARD.UNMOVABLE",
                CardType::Power,
                TargetType::SelfTarget,
                2,
                vec![buff("POWER.UNMOVABLE_POWER", 1)],
            ),
            1,
        ),
        // An infernal blade rolls one attack of the character's own pool and
        // hands it over free for the turn.
        exhausting(upgrading_cost(
            card(
                "CARD.INFERNAL_BLADE",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![Command::CardPile(CardPileCmd::AddGeneratedFromPool {
                    pool: sts2_engine::CardPoolRef::OwnCharacter,
                    count: Amount::Fixed(1),
                    distinct: true,
                    filter: CardFilter::OfType(CardType::Attack),
                    pile: PileName::Hand,
                    position: sts2_engine::DrawPilePosition::Bottom,
                    excludes_source: false,
                    upgraded: false,
                    temporary_cost: Some(sts2_engine::TemporaryCost::free_this_turn()),
                    ethereal: false,
                    rolled_one_at_a_time: false,
                })],
            ),
            0,
        )),
        // A stoke burns the whole hand and rolls that many cards of the
        // character's own pool back into it, upgraded if the stoke is.
        upgrading(
            card(
                "CARD.STOKE",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![stoke(false)],
            ),
            vec![stoke(true)],
        ),
    ]
}

/// Feed: the swing, and then the growth if the blow killed something whose
/// death counts. The two are separate steps, which is why a Feed that ends the
/// fight still grows its owner: gaining maximum HP is not held back by a
/// combat that is ending.
fn feed(damage: i32, max_hp: i32) -> Vec<Command> {
    vec![
        attack(damage),
        Command::When {
            otherwise: Vec::new(),
            condition: Condition::AFatalKillHappenedThisPlay,
            then: vec![Command::Creature(CreatureCmd::GainMaxHp {
                target: CreatureSelector::Owner,
                amount: Amount::Fixed(max_hp),
            })],
        },
    ]
}

/// Rampage: the swing is what the card's damage var has grown to, and then
/// the var grows again.
fn rampage(increase: i32) -> Vec<Command> {
    vec![
        multi_attack(Amount::counting(Count::SourceCardDamage), Amount::Fixed(1)),
        Command::Card(CardCmd::GrowDamage {
            cards: CardSelector::Source,
            amount: Amount::Fixed(increase),
        }),
    ]
}

/// Stomp: the sweep is what playing it does; what it costs is its own two
/// listeners. It counts the attacks already played this turn as it joins the
/// fight, and takes one more off for every attack played while it is held.
fn stomp() -> CardDefinition {
    let mut definition = upgrading(
        card(
            "CARD.STOMP",
            CardType::Attack,
            TargetType::AllEnemies,
            3,
            vec![sweep(12, 1)],
        ),
        vec![sweep(15, 1)],
    );
    definition.hooks.push(
        sts2_engine::ModifierHook::every_turn(
            sts2_engine::HookEvent::AfterCardEnteredCombat,
            // A copy made mid-fight is owed nothing for the attacks played
            // before it existed.
            vec![Command::When {
                otherwise: Vec::new(),
                condition: Condition::Not(Box::new(Condition::SourceCardIsAClone)),
                then: vec![Command::Card(CardCmd::AddCostThisTurn {
                    cards: CardSelector::Source,
                    amount: Amount::negating(Count::OwnerAttacksPlayedThisTurn),
                })],
            }],
        )
        .when(sts2_engine::HookCondition::ListeningCardEnteredCombat),
    );
    definition.hooks.push(
        sts2_engine::ModifierHook::every_turn(
            sts2_engine::HookEvent::BeforeCardPlayed,
            vec![Command::Card(CardCmd::AddCostThisTurn {
                cards: CardSelector::Source,
                amount: Amount::Fixed(-1),
            })],
        )
        .when(sts2_engine::HookCondition::PlayedCardIsAttack),
    );
    definition
}

/// Stoke: the whole hand burns, and that many cards of the character's own
/// pool are rolled one at a time and handed over.
fn stoke(upgraded: bool) -> Command {
    Command::Select {
        cards: CardSelector::Pile {
            pile: PileName::Hand,
            filter: CardFilter::Any,
        },
        then: vec![
            Command::Card(CardCmd::Exhaust {
                cards: CardSelector::Chosen,
            }),
            Command::CardPile(CardPileCmd::AddGeneratedFromPool {
                pool: sts2_engine::CardPoolRef::OwnCharacter,
                count: Amount::counting(Count::ChosenCards),
                distinct: false,
                filter: CardFilter::Any,
                pile: PileName::Hand,
                position: sts2_engine::DrawPilePosition::Bottom,
                excludes_source: false,
                upgraded,
                temporary_cost: None,
                ethereal: false,
                rolled_one_at_a_time: false,
            }),
        ],
    }
}

/// The cards whose bodies lean on a listener the pool's powers carry, and the
/// ones that move cards between piles.
#[allow(clippy::too_many_lines, reason = "one entry per card")]
fn pool_listeners_and_piles() -> Vec<CardDefinition> {
    vec![
        // A headbutt swings and then puts one card of the discard pile back on
        // top of the draw pile.
        upgrading(
            card(
                "CARD.HEADBUTT",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                headbutt(9),
            ),
            headbutt(12),
        ),
        // A pillage swings, then draws for as long as it keeps turning up
        // attacks and the hand has room.
        upgrading(
            card(
                "CARD.PILLAGE",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![
                    attack(6),
                    Command::CardPile(CardPileCmd::DrawWhile {
                        filter: CardFilter::OfType(CardType::Attack),
                    }),
                ],
            ),
            vec![
                attack(9),
                Command::CardPile(CardPileCmd::DrawWhile {
                    filter: CardFilter::OfType(CardType::Attack),
                }),
            ],
        ),
        // A second wind burns every card of the hand that is not an attack, and
        // works up block for each of them as it goes.
        upgrading(
            card(
                "CARD.SECOND_WIND",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![second_wind(5)],
            ),
            vec![second_wind(7)],
        ),
        // Expecting a fight pays a point of energy for every attack in hand and
        // then stops the next gain outright.
        upgrading_cost(
            card(
                "CARD.EXPECT_A_FIGHT",
                CardType::Skill,
                TargetType::SelfTarget,
                2,
                vec![
                    Command::Player(PlayerCmd::GainEnergy(Amount::counting(
                        Count::CardsInPileOfType {
                            pile: PileName::Hand,
                            card_type: CardType::Attack,
                        },
                    ))),
                    buff("POWER.NO_ENERGY_GAIN_POWER", 1),
                ],
            ),
            1,
        ),
        // A battle trance draws deep and stops the well for the turn.
        upgrading(
            card(
                "CARD.BATTLE_TRANCE",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![draw(3), buff("POWER.NO_DRAW_POWER", 1)],
            ),
            vec![draw(4), buff("POWER.NO_DRAW_POWER", 1)],
        ),
        // A howl lands on everything, and answers for itself: once its owner
        // has stopped acting, a howl standing in the exhaust pile plays again.
        howl_from_beyond(),
        upgrading(
            card(
                "CARD.JUGGERNAUT",
                CardType::Power,
                TargetType::SelfTarget,
                2,
                vec![buff("POWER.JUGGERNAUT_POWER", 6)],
            ),
            vec![buff("POWER.JUGGERNAUT_POWER", 8)],
        ),
        upgrading(
            card(
                "CARD.VICIOUS",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.VICIOUS_POWER", 1)],
            ),
            vec![buff("POWER.VICIOUS_POWER", 2)],
        ),
        upgrading(
            card(
                "CARD.ONE_TWO_PUNCH",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.ONE_TWO_PUNCH_POWER", 1)],
            ),
            vec![buff("POWER.ONE_TWO_PUNCH_POWER", 2)],
        ),
        // Upgrading a juggling makes it innate rather than stronger.
        innate_when_upgraded(upgrading(
            card(
                "CARD.JUGGLING",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.JUGGLING_POWER", 1)],
            ),
            vec![buff("POWER.JUGGLING_POWER", 1)],
        )),
        innate_when_upgraded(upgrading(
            card(
                "CARD.AGGRESSION",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.AGGRESSION_POWER", 1)],
            ),
            vec![buff("POWER.AGGRESSION_POWER", 1)],
        )),
        upgrading_cost(
            card(
                "CARD.STAMPEDE",
                CardType::Power,
                TargetType::SelfTarget,
                2,
                vec![buff("POWER.STAMPEDE_POWER", 1)],
            ),
            1,
        ),
        upgrading_cost(
            card(
                "CARD.HELLRAISER",
                CardType::Power,
                TargetType::SelfTarget,
                2,
                vec![buff("POWER.HELLRAISER_POWER", 1)],
            ),
            1,
        ),
    ]
}

/// Headbutt: the swing, and then a screen over the discard pile whose pick
/// goes on top of the draw pile.
fn headbutt(damage: i32) -> Vec<Command> {
    vec![
        attack(damage),
        Command::CardSelect(sts2_engine::CardSelectCmd::FromCombatPile {
            exact_count: None,
            pile: PileName::Discard,
            prefs: sts2_engine::SelectorPrefs::exactly(1),
            maximum: None,
            restricted_to_chosen: false,
            filter: CardFilter::Any,
            then: vec![Command::CardPile(CardPileCmd::MoveToPile {
                cards: CardSelector::Chosen,
                pile: PileName::Draw,
                position: sts2_engine::DrawPilePosition::Top,
            })],
        }),
    ]
}

/// Second Wind: the non-attacks of the hand, taken together, and then one
/// burn and one block for each of them in turn.
fn second_wind(block_amount: i32) -> Command {
    Command::ForEachCard {
        cards: CardSelector::Pile {
            pile: PileName::Hand,
            filter: CardFilter::NotOfType(CardType::Attack),
        },
        then: vec![
            Command::Card(CardCmd::Exhaust {
                cards: CardSelector::Chosen,
            }),
            block(block_amount),
        ],
    }
}

/// Howl from Beyond: the sweep is what playing it does; playing itself again
/// out of the exhaust pile once its owner has stopped acting is its own
/// listener.
fn howl_from_beyond() -> CardDefinition {
    let mut definition = upgrading(
        card(
            "CARD.HOWL_FROM_BEYOND",
            CardType::Attack,
            TargetType::AllEnemies,
            3,
            vec![sweep(16, 1)],
        ),
        vec![sweep(21, 1)],
    );
    definition.hooks.push(sts2_engine::ModifierHook::every_turn(
        sts2_engine::HookEvent::AfterAutoPostPlayPhaseEntered,
        vec![Command::When {
            otherwise: Vec::new(),
            condition: Condition::SourceCardIsInPile(PileName::Exhaust),
            then: vec![Command::Card(CardCmd::AutoPlay {
                at_the_bodys_target: false,
                cards: CardSelector::Source,
            })],
        }],
    ));
    definition
}

/// The two cards of the pool whose cost is X.
fn x_cost_cards() -> Vec<CardDefinition> {
    vec![
        // A whirlwind lands once for every point of energy it swallowed. At no
        // energy it swallows nothing and lands nothing.
        costing_x(upgrading(
            card(
                "CARD.WHIRLWIND",
                CardType::Attack,
                TargetType::AllEnemies,
                0,
                vec![whirlwind(5)],
            ),
            vec![whirlwind(8)],
        )),
        // A cascade plays that many cards off the top of the draw pile, and one
        // more once it is upgraded. Nothing it plays is forced to burn.
        costing_x(upgrading(
            card(
                "CARD.CASCADE",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![cascade(Amount::counting(Count::EnergyX))],
            ),
            vec![cascade(Amount::per(1, 1, Count::EnergyX))],
        )),
    ]
}

fn whirlwind(damage: i32) -> Command {
    Command::Damage(DamageCmd::Attack {
        amount: Amount::Fixed(damage),
        hits: Amount::counting(Count::EnergyX),
        target: CreatureSelector::AllOpponents,
        props: ValueProps::NONE,
    })
}

fn cascade(count: Amount) -> Command {
    Command::CardPile(CardPileCmd::AutoPlayFromDrawPile {
        count,
        position: sts2_engine::DrawPilePosition::Top,
        force_exhaust: false,
    })
}

/// The cards that ask whether anything has burned this turn, and the one that
/// pays out when it burns itself.
fn burning_cards() -> Vec<CardDefinition> {
    vec![
        // An evil eye works up its block once, and twice over if something of
        // its owner's has already burned this turn.
        upgrading(
            card(
                "CARD.EVIL_EYE",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                evil_eye(8),
            ),
            evil_eye(11),
        ),
        // A forgotten ritual pays nothing at all unless something has already
        // burned this turn. It burns itself for having been played, which is
        // after its body has asked.
        exhausting(upgrading(
            card(
                "CARD.FORGOTTEN_RITUAL",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                forgotten_ritual(3),
            ),
            forgotten_ritual(4),
        )),
        // A drum of battle draws when it is played and pays energy when it
        // burns, however it comes to burn. The energy is its own var, so the
        // upgraded copy pays one more.
        drum_of_battle(),
    ]
}

/// Evil Eye works its block up once, or twice when something has burned this
/// turn, and the two are separate gains.
fn evil_eye(block_amount: i32) -> Vec<Command> {
    vec![
        block(block_amount),
        Command::When {
            otherwise: Vec::new(),
            condition: Condition::OwnerExhaustedACardThisTurn,
            then: vec![block(block_amount)],
        },
    ]
}

fn forgotten_ritual(amount: i32) -> Vec<Command> {
    vec![Command::When {
        otherwise: Vec::new(),
        condition: Condition::OwnerExhaustedACardThisTurn,
        then: vec![energy(amount)],
    }]
}

/// Drum of Battle: the draw is what playing it does, and the energy is its own
/// `AfterCardExhausted` listener answering for its own burn — once for every
/// time a play of it would run.
fn drum_of_battle() -> CardDefinition {
    let mut definition = upgrading(
        card(
            "CARD.DRUM_OF_BATTLE",
            CardType::Skill,
            TargetType::SelfTarget,
            1,
            vec![draw(2)],
        ),
        vec![draw(2)],
    );
    definition.hooks.push(
        sts2_engine::ModifierHook::every_turn(
            sts2_engine::HookEvent::AfterCardExhausted,
            vec![Command::Player(PlayerCmd::GainEnergy(Amount::Product {
                left: Box::new(Amount::IfElse {
                    condition: Condition::SourceCardIsUpgraded,
                    then: Box::new(Amount::Fixed(3)),
                    otherwise: Box::new(Amount::Fixed(2)),
                }),
                right: Box::new(Amount::counting(Count::SourceCardPlayCount)),
            }))],
        )
        .when(sts2_engine::HookCondition::ListeningCardExhausted),
    );
    definition
}

/// Heals the card's own owner.
pub(crate) fn heal(amount: i32) -> Command {
    Command::Creature(CreatureCmd::Heal {
        target: CreatureSelector::Owner,
        amount: Amount::Fixed(amount),
    })
}
#[allow(clippy::too_many_lines, reason = "one entry per card")]
fn starting_cards() -> Vec<CardDefinition> {
    vec![
        striking(upgrading(
            card(
                "CARD.STRIKE_IRONCLAD",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![attack(6)],
            ),
            vec![attack(9)],
        )),
        defending(upgrading(
            card(
                "CARD.DEFEND_IRONCLAD",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![block(5)],
            ),
            vec![block(8)],
        )),
        upgrading(
            card(
                "CARD.BASH",
                CardType::Attack,
                TargetType::AnyEnemy,
                2,
                vec![attack(8), debuff("POWER.VULNERABLE_POWER", 2)],
            ),
            vec![attack(10), debuff("POWER.VULNERABLE_POWER", 3)],
        ),
        upgrading(
            card(
                "CARD.BULLY",
                CardType::Attack,
                TargetType::AnyEnemy,
                0,
                vec![multi_attack(
                    Amount::per(4, 2, Count::TargetPower(id("POWER.VULNERABLE_POWER"))),
                    Amount::Fixed(1),
                )],
            ),
            // Upgrading pays more per stack, not more up front.
            vec![multi_attack(
                Amount::per(4, 3, Count::TargetPower(id("POWER.VULNERABLE_POWER"))),
                Amount::Fixed(1),
            )],
        ),
        // Upgraded, the rocks it makes are upgraded too.
        upgrading(
            card(
                "CARD.PRIMAL_FORCE",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![into_rocks(false)],
            ),
            vec![into_rocks(true)],
        ),
        rarity(
            upgrading(
                card(
                    "CARD.GIANT_ROCK",
                    CardType::Attack,
                    TargetType::AnyEnemy,
                    1,
                    vec![attack(16)],
                ),
                vec![attack(20)],
            ),
            sts2_engine::CardRarity::Token,
        ),
        upgrading(
            card(
                "CARD.TAUNT",
                CardType::Skill,
                TargetType::AnyEnemy,
                1,
                vec![block(7), debuff("POWER.VULNERABLE_POWER", 1)],
            ),
            vec![block(8), debuff("POWER.VULNERABLE_POWER", 2)],
        ),
        // The one ancient card the Ironclad's own pool carries that a starter
        // does not become.
        ancient(upgrading_cost(
            card(
                "CARD.CORRUPTION",
                CardType::Power,
                TargetType::SelfTarget,
                3,
                vec![buff("POWER.CORRUPTION_POWER", 1)],
            ),
            2,
        )),
        // Two hits, and then it eats an attack out of the hand and keeps what
        // that attack would have dealt.
        with_damage_var(
            upgrading(
                card(
                    "CARD.THRASH",
                    CardType::Attack,
                    TargetType::AnyEnemy,
                    1,
                    thrash(),
                ),
                thrash(),
            ),
            4,
            6,
        ),
        upgrading(
            card(
                "CARD.BLUDGEON",
                CardType::Attack,
                TargetType::AnyEnemy,
                3,
                vec![attack(32)],
            ),
            vec![attack(42)],
        ),
        // The attack leaves its owner owed a free one.
        upgrading(
            card(
                "CARD.UNRELENTING",
                CardType::Attack,
                TargetType::AnyEnemy,
                2,
                vec![attack(14), buff("POWER.FREE_ATTACK_POWER", 1)],
            ),
            vec![attack(20), buff("POWER.FREE_ATTACK_POWER", 1)],
        ),
        // Six HP for two energy and three cards, four upgraded.
        exhausting(upgrading(
            card(
                "CARD.OFFERING",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![self_hp_loss(6), energy(2), draw(3)],
            ),
            vec![self_hp_loss(6), energy(2), draw(5)],
        )),
        upgrading(
            card(
                "CARD.INFLAME",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.STRENGTH_POWER", 2)],
            ),
            vec![buff("POWER.STRENGTH_POWER", 3)],
        ),
        // The build works the hit count out before it makes the attack, and
        // twice is what it comes to against a target already shaken.
        upgrading(
            card(
                "CARD.DISMANTLE",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                dismantle(8),
            ),
            dismantle(10),
        ),
        // A colorless power the run only ever meets out of a potion.
        upgrading(
            card(
                "CARD.ETERNAL_ARMOR",
                CardType::Power,
                TargetType::SelfTarget,
                3,
                vec![buff("POWER.PLATING_POWER", 9)],
            ),
            vec![buff("POWER.PLATING_POWER", 12)],
        ),
        upgrading(
            card(
                "CARD.STONE_ARMOR",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.PLATING_POWER", 4)],
            ),
            vec![buff("POWER.PLATING_POWER", 6)],
        ),
    ]
}

/// A card that burns up if it is still held when the turn ends.
pub(crate) fn ethereal(mut definition: CardDefinition) -> CardDefinition {
    definition.ethereal = true;
    definition
}

/// A card that does something to whoever still holds it when the turn ends.
pub(crate) fn held_at_turn_end(
    mut definition: CardDefinition,
    effects: Vec<Command>,
) -> CardDefinition {
    definition.end_of_turn_in_hand = effects;
    definition
}

/// A card that cannot be played at all. The pinned build prices those at -1.
pub(crate) fn unplayable(mut definition: CardDefinition) -> CardDefinition {
    definition.unplayable = true;
    definition.energy_cost = -1;
    definition
}

/// A card that can never be removed from the deck.
pub(crate) fn eternal(mut definition: CardDefinition) -> CardDefinition {
    definition.eternal = true;
    definition
}

/// The attacks the covered run picks up as it goes.
fn later_attacks() -> Vec<CardDefinition> {
    vec![upgrading(
        card(
            "CARD.ANGER",
            CardType::Attack,
            TargetType::AnyEnemy,
            0,
            anger(6),
        ),
        anger(8),
    )]
}

/// A swing, and a copy of the card into the discard pile for this combat only.
fn anger(damage: i32) -> Vec<Command> {
    vec![
        attack(damage),
        // The copy joins this combat only; it never reaches the deck.
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
            position: sts2_engine::DrawPilePosition::Bottom,
        }),
    ]
}

/// What Cinder burns is a card out of the hand, not itself: one card off the
/// hand, rolled on the combat card-selection stream.
fn burn_a_card_from_hand() -> Command {
    Command::Select {
        cards: CardSelector::RandomFromPile {
            pile: PileName::Hand,
            filter: CardFilter::Any,
            fallback: None,
        },
        then: vec![Command::Card(CardCmd::Exhaust {
            cards: CardSelector::Chosen,
        })],
    }
}

/// The attacks the Hive hands it.
fn hive_attacks() -> Vec<CardDefinition> {
    vec![
        upgrading(
            card(
                "CARD.CINDER",
                CardType::Attack,
                TargetType::AnyEnemy,
                2,
                vec![attack(18), burn_a_card_from_hand()],
            ),
            vec![attack(24), burn_a_card_from_hand()],
        ),
        upgrading_cost(
            card(
                "CARD.BODY_SLAM",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                // A slam is worth what its owner is standing behind.
                vec![multi_attack(
                    Amount::counting(Count::OwnerBlock),
                    Amount::Fixed(1),
                )],
            ),
            0,
        ),
        striking(upgrading(
            card(
                "CARD.POMMEL_STRIKE",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![attack(9), draw(1)],
            ),
            vec![attack(10), draw(2)],
        )),
        upgrading(
            card(
                "CARD.SPITE",
                CardType::Attack,
                TargetType::AnyEnemy,
                0,
                vec![multi_attack(Amount::Fixed(5), spite_hits(2))],
            ),
            vec![multi_attack(Amount::Fixed(5), spite_hits(3))],
        ),
        exhausting(upgrading(
            card(
                "CARD.DOMINATE",
                CardType::Skill,
                TargetType::AnyEnemy,
                1,
                dominate(1),
            ),
            dominate(2),
        )),
    ]
}

/// Spite lands its full run of hits only on an owner who has already bled this
/// turn.
fn spite_hits(repeat: i32) -> Amount {
    Amount::IfElse {
        condition: Condition::OwnerLostHpThisTurn,
        then: Box::new(Amount::Fixed(repeat)),
        otherwise: Box::new(Amount::Fixed(1)),
    }
}

/// Dominate leaves the target vulnerable and takes as much strength as the
/// target is then carrying vulnerability.
fn dominate(vulnerable: i32) -> Vec<Command> {
    vec![
        debuff("POWER.VULNERABLE_POWER", vulnerable),
        apply_power(
            "POWER.STRENGTH_POWER",
            CreatureSelector::Owner,
            Amount::counting(Count::TargetPower(id("POWER.VULNERABLE_POWER"))),
        ),
    ]
}

/// The skills and powers it picks up alongside them.
fn later_skills() -> Vec<CardDefinition> {
    vec![
        upgrading(
            card(
                "CARD.RAGE",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![buff("POWER.RAGE_POWER", 3)],
            ),
            vec![buff("POWER.RAGE_POWER", 5)],
        ),
        upgrading(
            card(
                "CARD.INFERNO",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![
                    buff("POWER.INFERNO_POWER", 6),
                    // The power burns its owner one harder for every time it
                    // has been laid down.
                    Command::Power(PowerCmd::ModifyCounter {
                        power: PowerRef::owned(id("POWER.INFERNO_POWER")),
                        amount: Amount::Fixed(1),
                    }),
                ],
            ),
            vec![
                buff("POWER.INFERNO_POWER", 9),
                Command::Power(PowerCmd::ModifyCounter {
                    power: PowerRef::owned(id("POWER.INFERNO_POWER")),
                    amount: Amount::Fixed(1),
                }),
            ],
        ),
        // Upgrading pays one more damage a hit and one more strength to its
        // owner; what the target is handed does not move.
        upgrading(
            card(
                "CARD.FIGHT_ME",
                CardType::Attack,
                TargetType::AnyEnemy,
                2,
                fight_me(5, 3),
            ),
            fight_me(6, 4),
        ),
        striking(upgrading(
            card(
                "CARD.TWIN_STRIKE",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![multi_attack(Amount::Fixed(5), Amount::Fixed(2))],
            ),
            vec![multi_attack(Amount::Fixed(7), Amount::Fixed(2))],
        )),
        // Upgrading pays one more per Strike rather than more up front.
        striking(upgrading(
            card(
                "CARD.PERFECTED_STRIKE",
                CardType::Attack,
                TargetType::AnyEnemy,
                2,
                vec![multi_attack(
                    Amount::per(6, 2, Count::TaggedCards(CardTag::Strike)),
                    Amount::Fixed(1),
                )],
            ),
            vec![multi_attack(
                Amount::per(6, 3, Count::TaggedCards(CardTag::Strike)),
                Amount::Fixed(1),
            )],
        )),
        exhausting(upgrading(
            card(
                "CARD.TREMBLE",
                CardType::Skill,
                TargetType::AnyEnemy,
                1,
                vec![debuff("POWER.VULNERABLE_POWER", 3)],
            ),
            vec![debuff("POWER.VULNERABLE_POWER", 4)],
        )),
    ]
}

/// The status cards enemies force into the deck, and the curses events do.
fn status_cards() -> Vec<CardDefinition> {
    vec![
        // Clumsy cannot be played, and burns up if it is still held when the
        // turn ends.
        unplayable(ethereal(card(
            "CARD.CLUMSY",
            CardType::Curse,
            TargetType::None,
            0,
            Vec::new(),
        ))),
        // The four cards the Knowledge Demon offers. None of them is ever in a
        // pile: the choice is made off a screen and what it does happens there.
        unplayable(card(
            "CARD.DISINTEGRATION",
            CardType::Status,
            TargetType::None,
            0,
            Vec::new(),
        )),
        unplayable(card(
            "CARD.MIND_ROT",
            CardType::Status,
            TargetType::None,
            0,
            Vec::new(),
        )),
        unplayable(card(
            "CARD.SLOTH",
            CardType::Status,
            TargetType::None,
            0,
            Vec::new(),
        )),
        unplayable(card(
            "CARD.WASTE_AWAY",
            CardType::Status,
            TargetType::None,
            0,
            Vec::new(),
        )),
        unplayable(card(
            "CARD.WOUND",
            CardType::Status,
            TargetType::None,
            0,
            Vec::new(),
        )),
        unplayable(ethereal(card(
            "CARD.DAZED",
            CardType::Status,
            TargetType::None,
            0,
            Vec::new(),
        ))),
        // Toxic burns its holder at the end of the turn, and exhausts when it
        // is played away. Block still stops it; nothing scales it.
        held_at_turn_end(
            exhausting(card(
                "CARD.TOXIC",
                CardType::Status,
                TargetType::None,
                1,
                Vec::new(),
            )),
            vec![Command::Creature(CreatureCmd::Damage {
                target: CreatureSelector::Owner,
                amount: Amount::Fixed(5),
                props: ValueProps::UNPOWERED,
            })],
        ),
        // Greed is what Cursed Pearl charges for its gold: unplayable, and
        // eternal, so no removal screen ever offers it.
        eternal(unplayable(card(
            "CARD.GREED",
            CardType::Curse,
            TargetType::None,
            0,
            Vec::new(),
        ))),
        exhausting(card(
            "CARD.SLIMED",
            CardType::Status,
            TargetType::None,
            1,
            vec![draw(1)],
        )),
    ]
}

/// The cards this run picks up in Overgrowth and carries on.
#[allow(clippy::too_many_lines, reason = "one entry per card the run picks up")]
fn overgrowth_pickups() -> Vec<CardDefinition> {
    vec![
        // Iron Wave works up its block before it swings.
        upgrading(
            card(
                "CARD.IRON_WAVE",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![block(5), attack(5)],
            ),
            vec![block(7), attack(7)],
        ),
        // Hemokinesis pays two HP of its own, which nothing blocks and nothing
        // scales, before it swings.
        upgrading(
            card(
                "CARD.HEMOKINESIS",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![self_hp_loss(2), attack(15)],
            ),
            vec![self_hp_loss(2), attack(20)],
        ),
        // Sword Boomerang picks each of its targets fresh.
        upgrading(
            card(
                "CARD.SWORD_BOOMERANG",
                CardType::Attack,
                TargetType::RandomEnemy,
                1,
                vec![random_attack(3, 3)],
            ),
            vec![random_attack(3, 4)],
        ),
        // An uppercut leaves the target weakened and vulnerable, one of each,
        // or two of each once upgraded.
        upgrading(
            card(
                "CARD.UPPERCUT",
                CardType::Attack,
                TargetType::AnyEnemy,
                2,
                vec![
                    attack(13),
                    debuff("POWER.WEAK_POWER", 1),
                    debuff("POWER.VULNERABLE_POWER", 1),
                ],
            ),
            vec![
                attack(13),
                debuff("POWER.WEAK_POWER", 2),
                debuff("POWER.VULNERABLE_POWER", 2),
            ],
        ),
        // Molten Fist doubles what vulnerability the target is already under,
        // and asks only of a target still standing.
        exhausting(upgrading(
            card(
                "CARD.MOLTEN_FIST",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![attack(10), double_target_vulnerability()],
            ),
            vec![attack(14), double_target_vulnerability()],
        )),
        // Ashen Strike counts what has burned up: six damage, and three more
        // for every card in the exhaust pile.
        striking(upgrading(
            card(
                "CARD.ASHEN_STRIKE",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![multi_attack(
                    Amount::per(6, 3, Count::CardsInPile(PileName::Exhaust)),
                    Amount::Fixed(1),
                )],
            ),
            vec![multi_attack(
                Amount::per(6, 4, Count::CardsInPile(PileName::Exhaust)),
                Amount::Fixed(1),
            )],
        )),
        // Omnislice hits one enemy, then splashes what that blow came to —
        // overkill included — onto everyone else, unpowered.
        upgrading(
            card(
                "CARD.OMNISLICE",
                CardType::Attack,
                TargetType::AnyEnemy,
                0,
                vec![omnislice(8)],
            ),
            vec![omnislice(11)],
        ),
        // Havoc plays the top card of the draw pile for free and burns it. The
        // burning is the auto-played card's; Havoc itself carries no keyword.
        upgrading_cost(
            card(
                "CARD.HAVOC",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![Command::CardPile(CardPileCmd::AutoPlayFromDrawPile {
                    count: Amount::Fixed(1),
                    position: sts2_engine::DrawPilePosition::Top,
                    force_exhaust: true,
                })],
            ),
            0,
        ),
        // A crimson mantle stacks block, and each laying-down costs one more
        // HP a turn, which the power counts up itself.
        upgrading(
            card(
                "CARD.CRIMSON_MANTLE",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                crimson_mantle(8),
            ),
            crimson_mantle(10),
        ),
        // Break is the Archaic Tooth's card, outside the rolled rarities.
        ancient(upgrading(
            card(
                "CARD.BREAK",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![attack(20), debuff("POWER.VULNERABLE_POWER", 5)],
            ),
            vec![attack(30), debuff("POWER.VULNERABLE_POWER", 7)],
        )),
    ]
}

/// HP the card's own owner pays: no block stops it and nothing scales it.
/// Energy the card hands its owner.
pub(crate) fn energy(amount: i32) -> Command {
    Command::Player(PlayerCmd::GainEnergy(Amount::Fixed(amount)))
}

pub(crate) fn self_hp_loss(amount: i32) -> Command {
    Command::Creature(CreatureCmd::Damage {
        target: CreatureSelector::Owner,
        amount: Amount::Fixed(amount),
        props: ValueProps::UNBLOCKABLE,
    })
}

/// An attack that rolls its target for each hit it makes.
pub(crate) fn random_attack(damage: i32, hits: i32) -> Command {
    Command::Damage(DamageCmd::Attack {
        amount: Amount::Fixed(damage),
        hits: Amount::Fixed(hits),
        target: CreatureSelector::RandomOpponent,
        props: ValueProps::NONE,
    })
}

/// Thrash: two hits of whatever the card's damage var has grown to, and then
/// an attack out of the hand is eaten for what it would have dealt.
fn thrash() -> Vec<Command> {
    vec![
        multi_attack(Amount::counting(Count::SourceCardDamage), Amount::Fixed(2)),
        Command::Select {
            cards: CardSelector::RandomFromPile {
                pile: PileName::Hand,
                filter: CardFilter::OfType(CardType::Attack),
                fallback: None,
            },
            then: vec![
                Command::Card(CardCmd::AddChosenDamageToSource),
                Command::Card(CardCmd::Exhaust {
                    cards: CardSelector::Chosen,
                }),
            ],
        },
    ]
}

/// A card whose own damage var another card can read, at each level.
pub(crate) fn with_damage_var(
    mut definition: CardDefinition,
    damage: i32,
    upgraded: i32,
) -> CardDefinition {
    definition.damage_var = Some(damage);
    if let Some(upgrade) = definition.upgrade.as_mut() {
        upgrade.damage_var = Some(upgraded);
    }
    definition
}

/// One attack whose hit count is two against a target carrying Vulnerable and
/// one against anything else.
fn dismantle(damage: i32) -> Vec<Command> {
    vec![
        Command::When {
            otherwise: Vec::new(),
            condition: Condition::TargetHasPower(id("POWER.VULNERABLE_POWER")),
            then: vec![multi_attack(Amount::Fixed(damage), Amount::Fixed(2))],
        },
        Command::When {
            otherwise: Vec::new(),
            condition: Condition::Not(Box::new(Condition::TargetHasPower(id(
                "POWER.VULNERABLE_POWER",
            )))),
            then: vec![multi_attack(Amount::Fixed(damage), Amount::Fixed(1))],
        },
    ]
}

/// Works as much vulnerability onto the target as it already carries, asked of
/// a target still standing and never for nothing.
fn double_target_vulnerability() -> Command {
    Command::When {
        otherwise: Vec::new(),
        condition: Condition::TargetIsAlive,
        then: vec![Command::When {
            otherwise: Vec::new(),
            condition: Condition::TargetHasPower(id("POWER.VULNERABLE_POWER")),
            then: vec![apply_power(
                "POWER.VULNERABLE_POWER",
                CreatureSelector::Target,
                Amount::counting(Count::TargetPower(id("POWER.VULNERABLE_POWER"))),
            )],
        }],
    }
}

/// A card outside the rolled rarities, which a relic or an event hands over.
pub(crate) fn ancient(definition: CardDefinition) -> CardDefinition {
    rarity(definition, sts2_engine::CardRarity::Ancient)
}

/// The rarity a card declares for itself where its pool does not hand it one.
///
/// A character pool's rolled three come off the pool table, and every other
/// card carries the rarity it names for itself: an ancient, a token, a
/// status, a curse, a quest card, and the multiplayer-only cards a
/// single-player pool never holds but that still count as what they declare.
pub(crate) fn rarity(
    mut definition: CardDefinition,
    rarity: sts2_engine::CardRarity,
) -> CardDefinition {
    definition.rarity = rarity;
    definition
}

/// One blow on the target, and the same blow again on everyone else: what the
/// first came to, overkill and all, dealt unpowered.
fn omnislice(damage: i32) -> Command {
    Command::Deal {
        target: CreatureSelector::Target,
        amount: Amount::Fixed(damage),
        props: ValueProps::NONE,
        then: vec![Command::Creature(CreatureCmd::Damage {
            target: CreatureSelector::OtherOpponents,
            amount: Amount::counting(Count::DealtDamageWithOverkill),
            props: ValueProps::UNPOWERED,
        })],
    }
}

/// The mantle's block, and the one more HP a turn it will cost.
fn crimson_mantle(block: i32) -> Vec<Command> {
    vec![
        buff("POWER.CRIMSON_MANTLE_POWER", block),
        Command::Power(PowerCmd::ModifyCounter {
            power: PowerRef::owned(id("POWER.CRIMSON_MANTLE_POWER")),
            amount: Amount::Fixed(1),
        }),
    ]
}
