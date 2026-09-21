//! The Regent's whole card pool, transcribed.
//!
//! Eighty-eight models: the four he starts with, the eighty a rolled rarity can
//! offer, the two ancients, and the two multiplayer-only cards no
//! single-player pool ever holds. Each is the run of commands the card issues
//! when it is played, in order, at both the levels it supports.
//!
//! Two things recur across the pool and are worth naming once. A star cost is
//! the second half of what a card costs — a card that declares no star cost has
//! none at all rather than a cost of zero — and a card that costs stars is
//! refused for stars the way any other is refused for energy. And a forge
//! stands a Sovereign Blade up where its owner holds none outside the exhaust
//! pile, and sharpens every blade they hold by the amount.

use sts2_engine::{
    Amount, CardCmd, CardDefinition, CardFilter, CardOrigin, CardPileCmd, CardPoolRef,
    CardSelectCmd, CardSelector, CardType, Command, Condition, Count, CreatureCmd,
    CreatureSelector, DamageCmd, DrawPilePosition, ForgeCmd, HookCondition, HookEvent,
    ModifierHook, PileName, PlayerCmd, SelectorPrefs, TargetType, ValueProps,
};

use crate::cards::{
    apply_power, attack, block, buff, card, costing_star_x, costing_stars, costing_x, debuff, draw,
    exhausting, innate_when_upgraded, kept_in_hand, multi_attack, rarity, striking, sweep,
    upgrading, upgrading_cost,
};
use crate::id;

/// A forge worth that much, for the card's own owner.
fn forge(amount: i32) -> Command {
    Command::Forge(ForgeCmd::Forge(Amount::Fixed(amount)))
}

/// That many stars for the card's own owner.
fn gain_stars(amount: i32) -> Command {
    Command::Player(PlayerCmd::GainStars(Amount::Fixed(amount)))
}

/// That much energy for the card's own owner.
fn gain_energy(amount: i32) -> Command {
    Command::Player(PlayerCmd::GainEnergy(Amount::Fixed(amount)))
}

fn ethereal(definition: CardDefinition) -> CardDefinition {
    crate::cards::ethereal(definition)
}

fn retaining_when_upgraded(mut definition: CardDefinition) -> CardDefinition {
    if let Some(upgrade) = definition.upgrade.as_mut() {
        upgrade.retain = true;
    }
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

/// A card that has an upgraded form which changes nothing about its own
/// commands: what the upgrade does is asked for inside the body, or comes from
/// a keyword the upgrade adds.
fn upgrading_in_place(definition: CardDefinition) -> CardDefinition {
    let effects = definition.effects.clone();
    upgrading(definition, effects)
}

/// The colorless pool, which six of his cards conjure out of by name.
fn colorless() -> CardPoolRef {
    CardPoolRef::Named(id("CARD_POOL.COLORLESS_CARD_POOL"))
}

/// That many distinct colorless cards rolled for the fight, each of them then
/// conjured into it.
fn colorless_into_hand(count: i32, upgraded: bool) -> Command {
    Command::CardPile(CardPileCmd::AddGeneratedFromPool {
        pool: colorless(),
        count: Amount::Fixed(count),
        distinct: true,
        filter: CardFilter::Any,
        pile: PileName::Hand,
        position: DrawPilePosition::Bottom,
        excludes_source: false,
        upgraded,
        temporary_cost: None,
        ethereal: false,
        rolled_one_at_a_time: false,
    })
}

/// Whatever the screen just handed back, put on top of the draw pile.
fn chosen_onto_the_draw_pile() -> Command {
    Command::CardPile(CardPileCmd::MoveToPile {
        cards: CardSelector::Chosen,
        pile: PileName::Draw,
        position: DrawPilePosition::Top,
    })
}

/// One card of the hand turned into a token: the token is made, upgraded if
/// the card that did it was upgraded, and then put in the card's place.
fn transform_from_hand(prefs: SelectorPrefs, into: &str, upgraded: bool) -> Command {
    Command::CardSelect(CardSelectCmd::FromHand {
        prefs,
        maximum: None,
        exact_count: None,
        filter: CardFilter::Any,
        then: vec![Command::Card(CardCmd::Transform {
            cards: CardSelector::Chosen,
            into: id(into),
            upgraded,
        })],
    })
}

/// The Weak and then the Vulnerable a starry attack lands, in the order the
/// bodies issue them rather than the order the vars are declared in.
fn weak_then_vulnerable(weak: i32, vulnerable: i32) -> Vec<Command> {
    vec![
        debuff("POWER.WEAK_POWER", weak),
        debuff("POWER.VULNERABLE_POWER", vulnerable),
    ]
}

/// Comet, Falling Star and Gamma Blast: one blow at whoever the play was aimed
/// at, and the two debuffs behind it.
fn starry_strike(damage: i32, weak: i32, vulnerable: i32) -> Vec<Command> {
    let mut body = vec![attack(damage)];
    body.extend(weak_then_vulnerable(weak, vulnerable));
    body
}

#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one literal per card model in the pool"
)]
pub(crate) fn regent_cards() -> Vec<CardDefinition> {
    let mut cards = basics();
    cards.extend(commons());
    cards.extend(uncommons());
    cards.extend(rares());
    cards.extend(ancients());
    cards.extend(multiplayer_only());
    cards
}

/// The four the Regent opens a run holding.
fn basics() -> Vec<CardDefinition> {
    vec![
        striking(upgrading(
            card(
                "CARD.STRIKE_REGENT",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![attack(6)],
            ),
            vec![attack(9)],
        )),
        crate::cards::defending(upgrading(
            card(
                "CARD.DEFEND_REGENT",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![block(5)],
            ),
            vec![block(8)],
        )),
        // The star the deck opens with: two stars for a blow and the two
        // debuffs behind it.
        costing_stars(
            upgrading(
                card(
                    "CARD.FALLING_STAR",
                    CardType::Attack,
                    TargetType::AnyEnemy,
                    0,
                    starry_strike(8, 1, 1),
                ),
                starry_strike(12, 1, 1),
            ),
            2,
        ),
        upgrading(
            card(
                "CARD.VENERATE",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![gain_stars(2)],
            ),
            vec![gain_stars(3)],
        ),
    ]
}

/// The twenty a common rarity offers.
#[allow(
    clippy::too_many_lines,
    reason = "one literal per card model in the pool"
)]
fn commons() -> Vec<CardDefinition> {
    vec![
        costing_stars(
            upgrading(
                card(
                    "CARD.ASTRAL_PULSE",
                    CardType::Attack,
                    TargetType::AllEnemies,
                    0,
                    vec![sweep(6, 2)],
                ),
                vec![sweep(8, 2)],
            ),
            3,
        ),
        // One card of the hand becomes a minion's strike, upgraded where the
        // card that made it was.
        upgrading(
            card(
                "CARD.BEGONE",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![transform_from_hand(
                    SelectorPrefs::exactly(1),
                    "CARD.MINION_STRIKE",
                    false,
                )],
            ),
            vec![transform_from_hand(
                SelectorPrefs::exactly(1),
                "CARD.MINION_STRIKE",
                true,
            )],
        ),
        upgrading(
            card(
                "CARD.CELESTIAL_MIGHT",
                CardType::Attack,
                TargetType::AnyEnemy,
                2,
                vec![multi_attack(Amount::Fixed(6), Amount::Fixed(3))],
            ),
            vec![multi_attack(Amount::Fixed(6), Amount::Fixed(4))],
        ),
        costing_stars(
            upgrading(
                card(
                    "CARD.CLOAK_OF_STARS",
                    CardType::Skill,
                    TargetType::SelfTarget,
                    0,
                    vec![block(7)],
                ),
                vec![block(10)],
            ),
            1,
        ),
        // A blow, and a piece of debris in the hand for having landed it.
        upgrading(
            card(
                "CARD.COLLISION_COURSE",
                CardType::Attack,
                TargetType::AnyEnemy,
                0,
                collision_course(11),
            ),
            collision_course(15),
        ),
        upgrading(
            card(
                "CARD.COSMIC_INDIFFERENCE",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                cosmic_indifference(6),
            ),
            cosmic_indifference(9),
        ),
        // The spear is worth eight and two more for every card of the fight
        // that costs stars at all, counted off the printed cost.
        costing_stars(
            upgrading(
                card(
                    "CARD.CRESCENT_SPEAR",
                    CardType::Attack,
                    TargetType::AnyEnemy,
                    1,
                    vec![crescent_spear(2)],
                ),
                vec![crescent_spear(3)],
            ),
            1,
        ),
        upgrading(
            card(
                "CARD.CRUSH_UNDER",
                CardType::Attack,
                TargetType::AllEnemies,
                1,
                crush_under(7, 1),
            ),
            crush_under(8, 2),
        ),
        upgrading(
            card(
                "CARD.GATHER_LIGHT",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![block(8), gain_stars(1)],
            ),
            vec![block(11), gain_stars(1)],
        ),
        upgrading(
            card(
                "CARD.GLITTERSTREAM",
                CardType::Skill,
                TargetType::SelfTarget,
                2,
                glitterstream(11, 5),
            ),
            glitterstream(13, 7),
        ),
        upgrading(
            card(
                "CARD.GLOW",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                glow(1),
            ),
            glow(2),
        ),
        costing_stars(
            upgrading(
                card(
                    "CARD.GUIDING_STAR",
                    CardType::Attack,
                    TargetType::AnyEnemy,
                    1,
                    vec![attack(12), draw(2)],
                ),
                vec![attack(13), draw(3)],
            ),
            2,
        ),
        upgrading(
            card(
                "CARD.HIDDEN_CACHE",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![gain_stars(1), buff("POWER.STAR_NEXT_TURN_POWER", 3)],
            ),
            vec![gain_stars(1), buff("POWER.STAR_NEXT_TURN_POWER", 4)],
        ),
        // The upgrade takes the burn off rather than changing what it does.
        upgrade_drops_exhaust(upgrading_in_place(exhausting(card(
            "CARD.KNOW_THY_PLACE",
            CardType::Skill,
            TargetType::AnyEnemy,
            0,
            weak_then_vulnerable(1, 1),
        )))),
        upgrading(
            card(
                "CARD.PATTER",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![block(8), buff("POWER.VIGOR_POWER", 2)],
            ),
            vec![block(10), buff("POWER.VIGOR_POWER", 3)],
        ),
        upgrading(
            card(
                "CARD.PHOTON_CUT",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                photon_cut(10, 1),
            ),
            photon_cut(13, 2),
        ),
        upgrading(
            card(
                "CARD.REFINE_BLADE",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![forge(9), buff("POWER.ENERGY_NEXT_TURN_POWER", 1)],
            ),
            vec![forge(13), buff("POWER.ENERGY_NEXT_TURN_POWER", 1)],
        ),
        striking(upgrading(
            card(
                "CARD.SOLAR_STRIKE",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![attack(9), gain_stars(1)],
            ),
            vec![attack(10), gain_stars(2)],
        )),
        upgrading(
            card(
                "CARD.SPOILS_OF_BATTLE",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![forge(5), draw(2)],
            ),
            vec![forge(8), draw(2)],
        ),
        upgrading(
            card(
                "CARD.WROUGHT_IN_WAR",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![attack(7), forge(7)],
            ),
            vec![attack(9), forge(9)],
        ),
    ]
}

/// Collision Course, played: the blow, and then a Debris conjured into the
/// hand.
fn collision_course(damage: i32) -> Vec<Command> {
    vec![
        attack(damage),
        Command::CardPile(CardPileCmd::AddGeneratedToCombat {
            enchantment: None,
            card: CardOrigin::Model(id("CARD.DEBRIS")),
            count: Amount::Fixed(1),
            pile: PileName::Hand,
            upgraded: false,
            ethereal: false,
            retain: false,
            temporary_cost: None,
            position: DrawPilePosition::Bottom,
            grown_by: Amount::Fixed(0),
        }),
    ]
}

/// Cosmic Indifference, played: block, and one card of the discard pile put
/// back on top of the draw pile — but only if it is still standing in one of
/// the two piles it could be moved between.
fn cosmic_indifference(block_amount: i32) -> Vec<Command> {
    vec![
        block(block_amount),
        Command::CardSelect(CardSelectCmd::FromCombatPile {
            pile: PileName::Discard,
            prefs: SelectorPrefs::exactly(1),
            maximum: None,
            exact_count: None,
            restricted_to_chosen: false,
            filter: CardFilter::Any,
            then: vec![Command::When {
                condition: Condition::ChosenCardIsInDrawOrDiscard,
                then: vec![chosen_onto_the_draw_pile()],
                otherwise: Vec::new(),
            }],
        }),
    ]
}

/// Crescent Spear's damage: eight, and this much more for every card
/// of the fight whose model declares a star cost at all.
fn crescent_spear(per_card: i32) -> Command {
    multi_attack(
        Amount::Scaled {
            base: 8,
            per: per_card,
            count: Count::CardsThatCostStars,
        },
        Amount::Fixed(1),
    )
}

/// Crush Under, played: one blow over the whole enemy side, and then the
/// strength each of them loses for a turn.
fn crush_under(damage: i32, strength: i32) -> Vec<Command> {
    vec![
        sweep(damage, 1),
        apply_power(
            "POWER.CRUSH_UNDER_POWER",
            CreatureSelector::AllOpponents,
            Amount::Fixed(strength),
        ),
    ]
}

/// Glitterstream, played: what next turn's block is worth is worked out first,
/// through the block passes and before any block is gained, and only then does
/// this turn's block land and the power take the number.
fn glitterstream(now: i32, next_turn: i32) -> Vec<Command> {
    vec![Command::WorkOutBlock {
        target: CreatureSelector::Owner,
        amount: Amount::Fixed(next_turn),
        props: ValueProps::NONE,
        then: vec![
            block(now),
            apply_power(
                "POWER.BLOCK_NEXT_TURN_POWER",
                CreatureSelector::Owner,
                Amount::counting(Count::Bound),
            ),
        ],
    }]
}

/// Glow, played: a star, a card, and a card again next turn. The upgrade
/// raises the stars alone, so the draw stays where it is.
fn glow(stars: i32) -> Vec<Command> {
    vec![
        gain_stars(stars),
        draw(1),
        buff("POWER.DRAW_CARDS_NEXT_TURN_POWER", 1),
    ]
}

/// Photon Cut, played: a blow, a draw, and one card of the hand back on top of
/// the draw pile.
fn photon_cut(damage: i32, cards: i32) -> Vec<Command> {
    vec![
        attack(damage),
        draw(cards),
        crate::cards::choosing_from_hand(
            SelectorPrefs::exactly(1),
            vec![chosen_onto_the_draw_pile()],
        ),
    ]
}

/// The thirty-six an uncommon rarity offers, less the one a single-player run
/// never sees.
#[allow(
    clippy::too_many_lines,
    reason = "one literal per card model in the pool"
)]
fn uncommons() -> Vec<CardDefinition> {
    vec![
        costing_stars(
            upgrading(
                card(
                    "CARD.ALIGNMENT",
                    CardType::Skill,
                    TargetType::SelfTarget,
                    0,
                    vec![gain_energy(2)],
                ),
                vec![gain_energy(3)],
            ),
            3,
        ),
        upgrading(
            card(
                "CARD.BLACK_HOLE",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.BLACK_HOLE_POWER", 3)],
            ),
            vec![buff("POWER.BLACK_HOLE_POWER", 4)],
        ),
        upgrading(
            card(
                "CARD.BULWARK",
                CardType::Skill,
                TargetType::SelfTarget,
                2,
                vec![block(12), forge(10)],
            ),
            vec![block(15), forge(13)],
        ),
        // Two cards of the draw pile become a minion's dive bomb, upgraded
        // where the card that made them was.
        upgrading(
            card(
                "CARD.CHARGE",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![charge(false)],
            ),
            vec![charge(true)],
        ),
        // A pillar of creation pays block for every card its owner conjures.
        upgrading(
            card(
                "CARD.PILLAR_OF_CREATION",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.PILLAR_OF_CREATION_POWER", 3)],
            ),
            vec![buff("POWER.PILLAR_OF_CREATION_POWER", 4)],
        ),
        upgrading(
            card(
                "CARD.CHILD_OF_THE_STARS",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.CHILD_OF_THE_STARS_POWER", 2)],
            ),
            vec![buff("POWER.CHILD_OF_THE_STARS_POWER", 3)],
        ),
        upgrading(
            card(
                "CARD.CONQUEROR",
                CardType::Skill,
                TargetType::AnyEnemy,
                1,
                vec![forge(3), debuff("POWER.CONQUEROR_POWER", 1)],
            ),
            vec![forge(5), debuff("POWER.CONQUEROR_POWER", 1)],
        ),
        upgrading(
            card(
                "CARD.CONVERGENCE",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                convergence(1),
            ),
            convergence(2),
        ),
        costing_stars(
            upgrading(
                card(
                    "CARD.DEVASTATE",
                    CardType::Attack,
                    TargetType::AnyEnemy,
                    1,
                    vec![attack(30)],
                ),
                vec![attack(40)],
            ),
            4,
        ),
        upgrading(
            card(
                "CARD.FURNACE",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.FURNACE_POWER", 5)],
            ),
            vec![buff("POWER.FURNACE_POWER", 7)],
        ),
        costing_stars(
            upgrading(
                card(
                    "CARD.GAMMA_BLAST",
                    CardType::Attack,
                    TargetType::AnyEnemy,
                    0,
                    starry_strike(13, 2, 2),
                ),
                starry_strike(18, 2, 2),
            ),
            3,
        ),
        upgrading(
            card(
                "CARD.GLIMMER",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                glimmer(3),
            ),
            glimmer(4),
        ),
        upgrading(
            card(
                "CARD.HEGEMONY",
                CardType::Attack,
                TargetType::AnyEnemy,
                2,
                vec![attack(15), buff("POWER.ENERGY_NEXT_TURN_POWER", 2)],
            ),
            vec![attack(18), buff("POWER.ENERGY_NEXT_TURN_POWER", 3)],
        ),
        // The kick is cheaper for every time it has been drawn this fight.
        cheaper_when_drawn(upgrading(
            card(
                "CARD.KINGLY_KICK",
                CardType::Attack,
                TargetType::AnyEnemy,
                4,
                vec![attack(27)],
            ),
            vec![attack(35)],
        )),
        // The punch is heavier for every time it has been drawn this fight,
        // and the growth is written onto the fight's own copy of it.
        grows_when_drawn(
            crate::cards::with_damage_var(
                upgrading(
                    card(
                        "CARD.KINGLY_PUNCH",
                        CardType::Attack,
                        TargetType::AnyEnemy,
                        1,
                        vec![kingly_punch()],
                    ),
                    vec![kingly_punch()],
                ),
                8,
                10,
            ),
            4,
            6,
        ),
        upgrading(
            card(
                "CARD.KNOCKOUT_BLOW",
                CardType::Attack,
                TargetType::AnyEnemy,
                3,
                knockout_blow(30),
            ),
            knockout_blow(38),
        ),
        upgrading(
            card(
                "CARD.LUNAR_BLAST",
                CardType::Attack,
                TargetType::AnyEnemy,
                0,
                vec![lunar_blast(4)],
            ),
            vec![lunar_blast(5)],
        ),
        upgrading(
            card(
                "CARD.MANIFEST_AUTHORITY",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![block(7), colorless_into_hand(1, false)],
            ),
            vec![block(8), colorless_into_hand(1, true)],
        ),
        // The monologue's power stands up an instance of its own each time and
        // pays its owner a strength for every card played while it is up.
        retaining_when_upgraded(upgrading_in_place(card(
            "CARD.MONOLOGUE",
            CardType::Skill,
            TargetType::SelfTarget,
            0,
            vec![buff("POWER.MONOLOGUE_POWER", 1)],
        ))),
        upgrading_cost(
            card(
                "CARD.ORBIT",
                CardType::Power,
                TargetType::SelfTarget,
                2,
                vec![buff("POWER.ORBIT_POWER", 1)],
            ),
            1,
        ),
        upgrading(
            card(
                "CARD.PALE_BLUE_DOT",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.PALE_BLUE_DOT_POWER", 1)],
            ),
            vec![buff("POWER.PALE_BLUE_DOT_POWER", 2)],
        ),
        upgrading(
            card(
                "CARD.PARRY",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.PARRY_POWER", 10)],
            ),
            vec![buff("POWER.PARRY_POWER", 14)],
        ),
        // The wall stays in the hand rather than being discarded.
        costing_stars(
            kept_in_hand(upgrading(
                card(
                    "CARD.PARTICLE_WALL",
                    CardType::Skill,
                    TargetType::SelfTarget,
                    0,
                    vec![block(9)],
                ),
                vec![block(12)],
            )),
            2,
        ),
        upgrading(
            card(
                "CARD.PROPHESIZE",
                CardType::Skill,
                TargetType::SelfTarget,
                2,
                vec![draw(6)],
            ),
            vec![draw(9)],
        ),
        // Three colorless cards laid out, and whichever is taken joins the
        // hand. The upgrade upgrades the three before the screen goes up.
        costing_stars(
            upgrading(
                card(
                    "CARD.QUASAR",
                    CardType::Skill,
                    TargetType::SelfTarget,
                    0,
                    vec![quasar(false)],
                ),
                vec![quasar(true)],
            ),
            2,
        ),
        upgrading(
            card(
                "CARD.RADIATE",
                CardType::Attack,
                TargetType::AllEnemies,
                0,
                vec![radiate(3)],
            ),
            vec![radiate(4)],
        ),
        costing_stars(
            upgrading(
                card(
                    "CARD.REFLECT",
                    CardType::Skill,
                    TargetType::SelfTarget,
                    1,
                    vec![block(15), buff("POWER.REFLECT_POWER", 1)],
                ),
                vec![block(20), buff("POWER.REFLECT_POWER", 1)],
            ),
            3,
        ),
        costing_stars(
            upgrading(
                card(
                    "CARD.RESONANCE",
                    CardType::Skill,
                    TargetType::AllEnemies,
                    1,
                    resonance(1),
                ),
                resonance(2),
            ),
            3,
        ),
        // Five stars for nine, and the upgrade keeps it in hand.
        costing_stars(
            retaining_when_upgraded(upgrading_in_place(exhausting(card(
                "CARD.ROYAL_GAMBLE",
                CardType::Skill,
                TargetType::SelfTarget,
                0,
                vec![gain_stars(9)],
            )))),
            5,
        ),
        striking(upgrading(
            card(
                "CARD.SHINING_STRIKE",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                shining_strike(8),
            ),
            shining_strike(11),
        )),
        upgrading_cost(
            card(
                "CARD.SPECTRUM_SHIFT",
                CardType::Power,
                TargetType::SelfTarget,
                2,
                vec![buff("POWER.SPECTRUM_SHIFT_POWER", 1)],
            ),
            1,
        ),
        // The one card of the build whose cost is X stars: the play spends the
        // whole pool and the body reads back what it spent.
        costing_star_x(upgrading(
            card(
                "CARD.STARDUST",
                CardType::Attack,
                TargetType::RandomEnemy,
                0,
                vec![stardust(5)],
            ),
            vec![stardust(7)],
        )),
        upgrading(
            card(
                "CARD.SUMMON_FORTH",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                summon_forth(8),
            ),
            summon_forth(11),
        ),
        upgrading(
            card(
                "CARD.SUPERMASSIVE",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                vec![supermassive(3)],
            ),
            vec![supermassive(4)],
        ),
        upgrading(
            card(
                "CARD.TERRAFORMING",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.VIGOR_POWER", 6)],
            ),
            vec![buff("POWER.VIGOR_POWER", 8)],
        ),
    ]
}

/// Convergence, played: the hand is kept, and an energy and this many stars
/// are owed next turn.
fn convergence(stars: i32) -> Vec<Command> {
    vec![
        buff("POWER.RETAIN_HAND_POWER", 1),
        buff("POWER.ENERGY_NEXT_TURN_POWER", 1),
        buff("POWER.STAR_NEXT_TURN_POWER", stars),
    ]
}

/// Glimmer, played: a draw, and one card of the hand back on top of the draw
/// pile.
fn glimmer(cards: i32) -> Vec<Command> {
    vec![
        draw(cards),
        crate::cards::choosing_from_hand(
            SelectorPrefs::exactly(1),
            vec![chosen_onto_the_draw_pile()],
        ),
    ]
}

/// Kingly Kick, as it is drawn: the card takes a point off its own cost for the
/// rest of the fight every time it is drawn.
fn cheaper_when_drawn(mut definition: CardDefinition) -> CardDefinition {
    definition.hooks.push(
        ModifierHook::every_turn(
            HookEvent::AfterCardDrawn,
            vec![Command::Card(CardCmd::AddCostThisCombat {
                cards: CardSelector::Source,
                amount: Amount::Fixed(-1),
            })],
        )
        .when(HookCondition::ListeningCardDrawn),
    );
    definition
}

/// Kingly Punch, as it is drawn: the card grows its own damage var every time
/// it is drawn, and the growth belongs to the fight's copy of it. The upgraded
/// form grows by more, which the body reads off the form it is in.
fn grows_when_drawn(mut definition: CardDefinition, by: i32, upgraded_by: i32) -> CardDefinition {
    let grow = |amount: i32| {
        Command::Card(CardCmd::GrowDamage {
            cards: CardSelector::Source,
            amount: Amount::Fixed(amount),
        })
    };
    definition.hooks.push(
        ModifierHook::every_turn(
            HookEvent::AfterCardDrawn,
            vec![Command::When {
                condition: Condition::SourceCardIsUpgraded,
                then: vec![grow(upgraded_by)],
                otherwise: vec![grow(by)],
            }],
        )
        .when(HookCondition::ListeningCardDrawn),
    );
    definition
}

/// Kingly Punch, played: what the card's own damage var has grown to.
fn kingly_punch() -> Command {
    multi_attack(Amount::counting(Count::SourceCardDamage), Amount::Fixed(1))
}

/// Knockout Blow, played: the blow, and five stars if it put something down.
fn knockout_blow(damage: i32) -> Vec<Command> {
    vec![
        attack(damage),
        Command::When {
            condition: Condition::AFatalKillHappenedThisPlay,
            then: vec![gain_stars(5)],
            otherwise: Vec::new(),
        },
    ]
}

/// Lunar Blast's hit count: one hit for every skill play its owner has
/// finished this turn.
fn lunar_blast(damage: i32) -> Command {
    multi_attack(
        Amount::Fixed(damage),
        Amount::Scaled {
            base: 0,
            per: 1,
            count: Count::OwnerCardsOfTypePlayedThisTurn(CardType::Skill),
        },
    )
}

/// Charge, played: two cards named off the draw pile, each made into a
/// minion's dive bomb where it stood.
fn charge(upgraded: bool) -> Command {
    Command::CardSelect(CardSelectCmd::FromCombatPile {
        pile: PileName::Draw,
        prefs: SelectorPrefs::exactly(2),
        maximum: None,
        exact_count: None,
        restricted_to_chosen: false,
        filter: CardFilter::Any,
        then: vec![Command::Card(CardCmd::Transform {
            cards: CardSelector::Chosen,
            into: id("CARD.MINION_DIVE_BOMB"),
            upgraded,
        })],
    })
}

/// Quasar, played: three colorless cards laid out, and whichever is taken
/// joins the hand.
fn quasar(upgraded: bool) -> Command {
    Command::CardSelect(CardSelectCmd::ChooseAGeneratedCard {
        pool: colorless(),
        count: 3,
        filter: CardFilter::Any,
        upgraded,
        can_skip: true,
        then: vec![Command::CardPile(CardPileCmd::AddGeneratedToCombat {
            enchantment: None,
            card: CardOrigin::Chosen,
            count: Amount::Fixed(1),
            pile: PileName::Hand,
            upgraded: false,
            ethereal: false,
            retain: false,
            temporary_cost: None,
            position: DrawPilePosition::Bottom,
            grown_by: Amount::Fixed(0),
        })],
    })
}

/// Radiate's hit count: one hit for every star its owner has gained this
/// turn.
fn radiate(damage: i32) -> Command {
    Command::Damage(DamageCmd::Attack {
        amount: Amount::Fixed(damage),
        hits: Amount::Scaled {
            base: 0,
            per: 1,
            count: Count::StarsGainedThisTurn,
        },
        target: CreatureSelector::AllOpponents,
        props: ValueProps::NONE,
    })
}

/// Resonance, played: strength for its owner, and a point off every enemy
/// still standing. The upgrade raises what its owner gains alone.
fn resonance(strength: i32) -> Vec<Command> {
    vec![
        buff("POWER.STRENGTH_POWER", strength),
        apply_power(
            "POWER.STRENGTH_POWER",
            CreatureSelector::AllOpponents,
            Amount::Fixed(-1),
        ),
    ]
}

/// Shining Strike, played: the blow, two stars, and the card itself back on top
/// of the draw pile unless it is about to burn up.
fn shining_strike(damage: i32) -> Vec<Command> {
    vec![
        attack(damage),
        gain_stars(2),
        Command::When {
            condition: Condition::Not(Box::new(Condition::SourceCardBurnsUp)),
            then: vec![Command::CardPile(CardPileCmd::MoveToPile {
                cards: CardSelector::Source,
                pile: PileName::Draw,
                position: DrawPilePosition::Top,
            })],
            otherwise: Vec::new(),
        },
    ]
}

/// Stardust, played: one hit at a rolled enemy for every star the play spent,
/// counted after the X passes have had their say.
fn stardust(damage: i32) -> Command {
    Command::Damage(DamageCmd::Attack {
        amount: Amount::Fixed(damage),
        hits: Amount::counting(Count::StarX),
        target: CreatureSelector::RandomOpponent,
        props: ValueProps::NONE,
    })
}

/// Summon Forth, played: every Sovereign Blade standing anywhere but the hand
/// is called back into it, and then the forge sharpens them all.
fn summon_forth(amount: i32) -> Vec<Command> {
    vec![
        Command::CardPile(CardPileCmd::MoveToPile {
            cards: CardSelector::CombatCardsOfModelOutside {
                model: id("CARD.SOVEREIGN_BLADE"),
                pile: PileName::Hand,
            },
            pile: PileName::Hand,
            position: DrawPilePosition::Bottom,
        }),
        forge(amount),
    ]
}

/// Supermassive's damage: five, and this much more for every card its
/// owner has had conjured into the fight.
fn supermassive(per_card: i32) -> Command {
    multi_attack(
        Amount::Scaled {
            base: 5,
            per: per_card,
            count: Count::CardsGeneratedThisCombat,
        },
        Amount::Fixed(1),
    )
}

/// The twenty-six a rare rarity offers, less the one a single-player run never
/// sees.
#[allow(
    clippy::too_many_lines,
    reason = "one literal per card model in the pool"
)]
fn rares() -> Vec<CardDefinition> {
    vec![
        innate_when_upgraded(upgrading_in_place(card(
            "CARD.ARSENAL",
            CardType::Power,
            TargetType::SelfTarget,
            1,
            vec![buff("POWER.ARSENAL_POWER", 1)],
        ))),
        upgrading(
            card(
                "CARD.BEAT_INTO_SHAPE",
                CardType::Attack,
                TargetType::AnyEnemy,
                1,
                beat_into_shape(5, 5, 5),
            ),
            beat_into_shape(7, 7, 7),
        ),
        innate_when_upgraded(upgrading_in_place(exhausting(card(
            "CARD.BIG_BANG",
            CardType::Skill,
            TargetType::SelfTarget,
            0,
            vec![draw(1), gain_stars(1), gain_energy(1), forge(5)],
        )))),
        // A bombardment that burned up plays itself again as the turn opens.
        plays_itself_out_of_the_exhaust_pile(upgrading(
            exhausting(card(
                "CARD.BOMBARDMENT",
                CardType::Attack,
                TargetType::AnyEnemy,
                3,
                vec![attack(18)],
            )),
            vec![attack(24)],
        )),
        upgrading(
            exhausting(card(
                "CARD.BUNDLE_OF_JOY",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![colorless_into_hand(3, false)],
            )),
            vec![colorless_into_hand(4, false)],
        ),
        costing_stars(
            upgrading(
                card(
                    "CARD.COMET",
                    CardType::Attack,
                    TargetType::AnyEnemy,
                    0,
                    starry_strike(33, 3, 3),
                ),
                starry_strike(44, 3, 3),
            ),
            5,
        ),
        upgrading(
            card(
                "CARD.CRASH_LANDING",
                CardType::Attack,
                TargetType::AllEnemies,
                1,
                crash_landing(21),
            ),
            crash_landing(26),
        ),
        // Six stars to draw three and play one of them three times over.
        costing_stars(
            upgrading(
                exhausting(card(
                    "CARD.DECISIONS_DECISIONS",
                    CardType::Skill,
                    TargetType::SelfTarget,
                    0,
                    decisions_decisions(3),
                )),
                decisions_decisions(5),
            ),
            6,
        ),
        costing_stars(
            ethereal(upgrading(
                card(
                    "CARD.DYING_STAR",
                    CardType::Attack,
                    TargetType::AllEnemies,
                    1,
                    dying_star(9, 9),
                ),
                dying_star(11, 11),
            )),
            3,
        ),
        upgrading(
            card(
                "CARD.FOREGONE_CONCLUSION",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.FOREGONE_CONCLUSION_POWER", 2)],
            ),
            vec![buff("POWER.FOREGONE_CONCLUSION_POWER", 3)],
        ),
        upgrading(
            card(
                "CARD.GENESIS",
                CardType::Power,
                TargetType::SelfTarget,
                2,
                vec![buff("POWER.GENESIS_POWER", 2)],
            ),
            vec![buff("POWER.GENESIS_POWER", 3)],
        ),
        // As many cards of the hand as its owner cares to name become a
        // minion's sacrifice, upgraded where the card that made them was.
        upgrading(
            exhausting(card(
                "CARD.GUARDS",
                CardType::Skill,
                TargetType::SelfTarget,
                2,
                vec![transform_from_hand(
                    SelectorPrefs::unbounded_range(),
                    "CARD.MINION_SACRIFICE",
                    false,
                )],
            )),
            vec![transform_from_hand(
                SelectorPrefs::unbounded_range(),
                "CARD.MINION_SACRIFICE",
                true,
            )],
        ),
        // The drill hits once for every point of energy the play spent, and
        // twice as often where that was four or more.
        costing_x(upgrading(
            card(
                "CARD.HEAVENLY_DRILL",
                CardType::Attack,
                TargetType::AnyEnemy,
                0,
                vec![heavenly_drill(8)],
            ),
            vec![heavenly_drill(10)],
        )),
        upgrading(
            card(
                "CARD.HEIRLOOM_HAMMER",
                CardType::Attack,
                TargetType::AnyEnemy,
                2,
                heirloom_hammer(20),
            ),
            heirloom_hammer(25),
        ),
        // The card plays itself off the top of the draw pile as the turn's
        // acting ends.
        plays_itself_off_the_draw_pile(upgrading(
            card(
                "CARD.I_AM_INVINCIBLE",
                CardType::Skill,
                TargetType::SelfTarget,
                1,
                vec![block(10)],
            ),
            vec![block(13)],
        )),
        // Every third skill play of the turn calls it back into the hand.
        returns_every_third_skill(upgrading(
            card(
                "CARD.MAKE_IT_SO",
                CardType::Attack,
                TargetType::AnyEnemy,
                0,
                vec![attack(6)],
            ),
            vec![attack(9)],
        )),
        upgrading_cost(
            card(
                "CARD.MONARCHS_GAZE",
                CardType::Power,
                TargetType::SelfTarget,
                2,
                vec![buff("POWER.MONARCHS_GAZE_POWER", 1)],
            ),
            1,
        ),
        costing_stars(
            upgrading(
                card(
                    "CARD.NEUTRON_AEGIS",
                    CardType::Power,
                    TargetType::SelfTarget,
                    1,
                    vec![buff("POWER.PLATING_POWER", 8)],
                ),
                vec![buff("POWER.PLATING_POWER", 11)],
            ),
            5,
        ),
        upgrading(
            card(
                "CARD.ROYALTIES",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.ROYALTIES_POWER", 30)],
            ),
            vec![buff("POWER.ROYALTIES_POWER", 40)],
        ),
        upgrading(
            card(
                "CARD.SEEKING_EDGE",
                CardType::Power,
                TargetType::SelfTarget,
                1,
                vec![buff("POWER.SEEKING_EDGE_POWER", 1), forge(7)],
            ),
            vec![buff("POWER.SEEKING_EDGE_POWER", 1), forge(11)],
        ),
        costing_stars(
            upgrading_cost(
                card(
                    "CARD.SEVEN_STARS",
                    CardType::Attack,
                    TargetType::AllEnemies,
                    2,
                    vec![sweep(7, 7)],
                ),
                1,
            ),
            7,
        ),
        upgrading_cost(
            card(
                "CARD.SWORD_SAGE",
                CardType::Power,
                TargetType::SelfTarget,
                2,
                vec![buff("POWER.SWORD_SAGE_POWER", 1)],
            ),
            1,
        ),
        costing_stars(
            upgrading(
                card(
                    "CARD.THE_SMITH",
                    CardType::Skill,
                    TargetType::SelfTarget,
                    1,
                    vec![forge(30)],
                ),
                vec![forge(40)],
            ),
            4,
        ),
        innate_when_upgraded(upgrading_in_place(card(
            "CARD.TYRANNY",
            CardType::Power,
            TargetType::SelfTarget,
            1,
            vec![buff("POWER.TYRANNY_POWER", 1)],
        ))),
        // The void form ends the turn the moment it lands.
        upgrade_drops_ethereal(ethereal(upgrading_in_place(card(
            "CARD.VOID_FORM",
            CardType::Power,
            TargetType::SelfTarget,
            3,
            vec![
                buff("POWER.VOID_FORM_POWER", 2),
                Command::Player(PlayerCmd::EndTurn),
            ],
        )))),
    ]
}

/// Beat Into Shape, played: the blow, and then a forge worth the base plus one
/// helping for every powered blow its owner has landed on this target this
/// turn, less one helping for the hit the attack just made.
fn beat_into_shape(damage: i32, base: i32, extra: i32) -> Vec<Command> {
    vec![
        attack(damage),
        Command::Forge(ForgeCmd::Forge(Amount::Sum(vec![
            Amount::Scaled {
                base,
                per: extra,
                count: Count::PoweredBlowsOnTargetThisTurn,
            },
            // One helping comes off for each hit the attack made, and it
            // makes one.
            Amount::Fixed(-extra),
        ]))),
    ]
}

/// A copy standing in the exhaust pile plays itself again as the turn's
/// automatic plays open.
fn plays_itself_out_of_the_exhaust_pile(mut definition: CardDefinition) -> CardDefinition {
    definition.hooks.push(ModifierHook::every_turn(
        HookEvent::AfterAutoPrePlayPhaseEnteredEarly,
        vec![Command::When {
            condition: Condition::SourceCardIsInPile(PileName::Exhaust),
            then: vec![Command::Card(CardCmd::AutoPlay {
                cards: CardSelector::Source,
                at_the_bodys_target: false,
            })],
            otherwise: Vec::new(),
        }],
    ));
    definition
}

/// A skill play of its owner's that leaves the turn's skill count a multiple
/// of three calls the card back into the hand, and only where it is not already
/// there.
fn returns_every_third_skill(mut definition: CardDefinition) -> CardDefinition {
    definition.hooks.push(
        ModifierHook::every_turn(
            HookEvent::AfterCardPlayedLate,
            vec![Command::When {
                condition: Condition::All(vec![
                    Condition::Not(Box::new(Condition::SourceCardIsInPile(PileName::Hand))),
                    Condition::CountIsAMultipleOf {
                        count: Count::OwnerCardsOfTypePlayedThisTurn(CardType::Skill),
                        of: 3,
                    },
                ]),
                then: vec![Command::CardPile(CardPileCmd::MoveToPile {
                    cards: CardSelector::Source,
                    pile: PileName::Hand,
                    position: DrawPilePosition::Bottom,
                })],
                otherwise: Vec::new(),
            }],
        )
        .when(HookCondition::All(vec![
            HookCondition::OwnerPlayedTheCard,
            HookCondition::PlayedCardIsOfType(CardType::Skill),
        ])),
    );
    definition
}

/// Crash Landing, played: one blow over the enemy side, and then as much debris
/// as the hand has room for.
fn crash_landing(damage: i32) -> Vec<Command> {
    vec![
        sweep(damage, 1),
        Command::CardPile(CardPileCmd::AddGeneratedToCombat {
            enchantment: None,
            card: CardOrigin::Model(id("CARD.DEBRIS")),
            count: Amount::counting(Count::SpaceLeftInHand),
            pile: PileName::Hand,
            upgraded: false,
            ethereal: false,
            retain: false,
            temporary_cost: None,
            position: DrawPilePosition::Bottom,
            grown_by: Amount::Fixed(0),
        }),
    ]
}

/// Decisions, Decisions, played: a draw, and then one playable skill of the
/// hand played three times over.
fn decisions_decisions(cards: i32) -> Vec<Command> {
    vec![
        draw(cards),
        Command::CardSelect(CardSelectCmd::FromHand {
            prefs: SelectorPrefs::exactly(1),
            maximum: None,
            exact_count: None,
            filter: CardFilter::PlayableOfType(CardType::Skill),
            then: vec![Command::Repeat {
                times: Amount::Fixed(3),
                then: vec![Command::Card(CardCmd::AutoPlay {
                    cards: CardSelector::Chosen,
                    at_the_bodys_target: false,
                })],
            }],
        }),
    ]
}

/// Dying Star, played: one blow over the enemy side, and the strength each of
/// them loses for the turn.
fn dying_star(damage: i32, strength: i32) -> Vec<Command> {
    vec![
        sweep(damage, 1),
        apply_power(
            "POWER.DYING_STAR_POWER",
            CreatureSelector::AllOpponents,
            Amount::Fixed(strength),
        ),
    ]
}

/// Heavenly Drill, played: one hit for every point of energy the play spent,
/// and twice as many where that was four or more.
fn heavenly_drill(damage: i32) -> Command {
    multi_attack(
        Amount::Fixed(damage),
        Amount::IfElse {
            condition: Condition::CountAtLeast {
                count: Count::EnergyX,
                at_least: 4,
            },
            then: Box::new(Amount::Scaled {
                base: 0,
                per: 2,
                count: Count::EnergyX,
            }),
            otherwise: Box::new(Amount::counting(Count::EnergyX)),
        },
    )
}

/// Heirloom Hammer, played: the blow, and a copy of one colorless card of the
/// hand back into it.
fn heirloom_hammer(damage: i32) -> Vec<Command> {
    vec![
        attack(damage),
        Command::CardSelect(CardSelectCmd::FromHand {
            prefs: SelectorPrefs::exactly(1),
            maximum: None,
            exact_count: None,
            filter: CardFilter::Colorless,
            then: vec![Command::CardPile(CardPileCmd::AddGeneratedToCombat {
                enchantment: None,
                card: CardOrigin::CloneOfChosen,
                count: Amount::Fixed(1),
                pile: PileName::Hand,
                upgraded: false,
                ethereal: false,
                retain: false,
                temporary_cost: None,
                position: DrawPilePosition::Bottom,
                grown_by: Amount::Fixed(0),
            })],
        }),
    ]
}

/// A copy standing on top of the draw pile plays itself as the turn's acting
/// ends.
fn plays_itself_off_the_draw_pile(mut definition: CardDefinition) -> CardDefinition {
    definition.hooks.push(ModifierHook::every_turn(
        HookEvent::AfterAutoPostPlayPhaseEntered,
        vec![Command::When {
            condition: Condition::SourceCardIsOnTopOfTheDrawPile,
            then: vec![Command::CardPile(CardPileCmd::AutoPlayFromDrawPile {
                count: Amount::Fixed(1),
                position: DrawPilePosition::Top,
                force_exhaust: false,
            })],
            otherwise: Vec::new(),
        }],
    ));
    definition
}

/// The two ancients scattered through his pool.
fn ancients() -> Vec<CardDefinition> {
    vec![
        rarity(
            costing_stars(
                upgrading(
                    card(
                        "CARD.METEOR_SHOWER",
                        CardType::Attack,
                        TargetType::AllEnemies,
                        0,
                        meteor_shower(14),
                    ),
                    meteor_shower(21),
                ),
                2,
            ),
            sts2_engine::CardRarity::Ancient,
        ),
        rarity(
            costing_stars(
                upgrading_cost(
                    card(
                        "CARD.THE_SEALED_THRONE",
                        CardType::Power,
                        TargetType::SelfTarget,
                        1,
                        vec![buff("POWER.THE_SEALED_THRONE_POWER", 1)],
                    ),
                    0,
                ),
                3,
            ),
            sts2_engine::CardRarity::Ancient,
        ),
    ]
}

/// Meteor Shower, played: one blow over the enemy side, and then the two
/// debuffs over whoever is still standing.
fn meteor_shower(damage: i32) -> Vec<Command> {
    vec![
        sweep(damage, 1),
        apply_power(
            "POWER.WEAK_POWER",
            CreatureSelector::AllOpponents,
            Amount::Fixed(2),
        ),
        apply_power(
            "POWER.VULNERABLE_POWER",
            CreatureSelector::AllOpponents,
            Amount::Fixed(2),
        ),
    ]
}

/// The two multiplayer-only cards his pool holds, which no single-player run
/// is ever offered. They are registered for what a
/// transform of one rolls against and for the pool's own membership.
fn multiplayer_only() -> Vec<CardDefinition> {
    vec![
        rarity(
            upgrading_cost(
                card(
                    "CARD.HAMMER_TIME",
                    CardType::Power,
                    TargetType::SelfTarget,
                    2,
                    vec![buff("POWER.HAMMER_TIME_POWER", 1)],
                ),
                1,
            ),
            sts2_engine::CardRarity::Rare,
        ),
        // A colorless card for whoever it was aimed at, upgraded where the
        // card that gave it was.
        rarity(
            upgrading_in_place(card(
                "CARD.LARGESSE",
                CardType::Skill,
                TargetType::AnyAlly,
                0,
                vec![colorless_into_hand(1, false)],
            )),
            sts2_engine::CardRarity::Uncommon,
        ),
    ]
}

/// Sovereign Blade, played: the blade swings for what the forge has grown it
/// to, as many times as it has been given, at whoever the play was aimed at —
/// or at the whole enemy side while its owner carries a seeking edge — and
/// works up block behind it for as much parry as they are standing on.
pub(crate) fn sovereign_blade() -> Vec<Command> {
    vec![
        Command::When {
            condition: Condition::OwnerHasPower(id("POWER.SEEKING_EDGE_POWER")),
            then: vec![Command::Damage(DamageCmd::Attack {
                amount: Amount::counting(Count::SourceCardDamage),
                hits: Amount::Fixed(1),
                target: CreatureSelector::AllOpponents,
                props: ValueProps::NONE,
            })],
            otherwise: vec![multi_attack(
                Amount::counting(Count::SourceCardDamage),
                Amount::Fixed(1),
            )],
        },
        // The block is the parry, and a blade standing behind none works up
        // nothing at all.
        Command::When {
            condition: Condition::OwnerHasPower(id("POWER.PARRY_POWER")),
            then: vec![Command::Creature(CreatureCmd::GainBlock {
                target: CreatureSelector::Owner,
                amount: Amount::counting(Count::OwnerPower(id("POWER.PARRY_POWER"))),
                props: ValueProps::NONE,
            })],
            otherwise: Vec::new(),
        },
    ]
}
