//! Powers as behavior: stacking and instance rules, temporary powers, the
//! ordered `Modify*` passes, the caps, and random targeting.

use std::sync::Arc;

use sts2_core::ModelId;
use sts2_engine::{
    Action, Amount, CardDefinition, CardFingerprint, CardHandle, CardRarity, CardType, Command,
    ContentRegistry, CreatureCmd, CreatureSelector, DamageCmd, HookCondition, HookEvent, Intent,
    ModifierCmd, ModifierHook, MonsterDefinition, MonsterMove, MonsterState, MoveIntent, PileName,
    PowerCmd, PowerDefinition, PowerInstance, PowerInstanceType, PowerStackType, PowerType,
    RelicCmd, RelicDefinition, RelicRarity, ScenarioBuilder, Simulator, TargetType, TemporaryPower,
    ValueProps,
};
use sts2_rng::{RunRngSet, RunStream};

const SEED: &str = "POWERTEST";

fn id(value: &str) -> ModelId {
    value.parse().expect("static model ID")
}

fn power(model_id: &str, power_type: PowerType, hooks: Vec<ModifierHook>) -> PowerDefinition {
    PowerDefinition {
        stops_combat_from_ending: false,
        must_perform_move_on_death: false,
        model_id: id(model_id),
        power_type,
        stack_type: PowerStackType::Counter,
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
        hooks,
    }
}

fn modifies(event: HookEvent, condition: HookCondition, command: ModifierCmd) -> ModifierHook {
    ModifierHook::every_turn(event, vec![Command::Modify(command)]).when(condition)
}

fn card(
    model_id: &str,
    card_type: CardType,
    target: TargetType,
    effects: Vec<Command>,
) -> CardDefinition {
    CardDefinition {
        model_id: id(model_id),
        rarity: CardRarity::Basic,
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
        energy_cost: 0,
        star_cost: -1,
        returns_to_hand: false,
        costs_star_x: false,
        costs_x: false,
        innate: false,
        retain: false,
        sly: false,
        exhausts: false,
        play_restriction: None,
        playable_when: None,
        damage_var: None,
        effects,
        transform_pool: None,
        visual_pool: None,
        adds_rest_site_option: None,
        hooks: Vec::new(),
    }
}

fn attack_card(model_id: &str, damage: i32, hits: i32, target: CreatureSelector) -> CardDefinition {
    card(
        model_id,
        CardType::Attack,
        TargetType::AnyEnemy,
        vec![Command::Damage(DamageCmd::Attack {
            amount: Amount::Fixed(damage),
            hits: Amount::Fixed(hits),
            target,
            props: ValueProps::NONE,
        })],
    )
}

fn applying(model_id: &str, power: &str, amount: i32, target: CreatureSelector) -> CardDefinition {
    let aim = if target == CreatureSelector::Owner {
        TargetType::SelfTarget
    } else {
        TargetType::AnyEnemy
    };
    card(
        model_id,
        CardType::Skill,
        aim,
        vec![Command::Power(PowerCmd::Apply {
            marks: None,
            power: id(power),
            target,
            applier: Some(CreatureSelector::Owner),
            amount: Amount::Fixed(amount),
        })],
    )
}

fn dummy(model_id: &str, on_added: Vec<Command>, commands: Vec<Command>) -> MonsterDefinition {
    MonsterDefinition {
        model_id: id(model_id),
        min_initial_hp: 40,
        max_initial_hp: 40,
        initial_state: "MOVE".to_owned(),
        on_added,
        hooks: Vec::new(),
        states: vec![MonsterState::Move(MonsterMove {
            state_id: "MOVE".to_owned(),
            intent: Intent::new("MOVE", vec![MoveIntent::of("buff")]),
            commands,
            follow_up: "MOVE".to_owned(),
        })],
    }
}

fn applies_to_target(power: &str, amount: i32) -> Command {
    Command::Power(PowerCmd::Apply {
        marks: None,
        power: id(power),
        target: CreatureSelector::Target,
        applier: Some(CreatureSelector::Owner),
        amount: Amount::Fixed(amount),
    })
}

/// The powers whose whole point is what they declare about stacking.
fn stacking_powers() -> Vec<PowerDefinition> {
    vec![
        power("POWER.PLAIN", PowerType::Buff, Vec::new()),
        PowerDefinition {
            stops_combat_from_ending: false,
            must_perform_move_on_death: false,
            instance_type: PowerInstanceType::Instanced,
            ..power("POWER.INSTANCED", PowerType::Buff, Vec::new())
        },
        PowerDefinition {
            stops_combat_from_ending: false,
            must_perform_move_on_death: false,
            instance_type: PowerInstanceType::InstancedPerApplier,
            ..power("POWER.PER_APPLIER", PowerType::Buff, Vec::new())
        },
        PowerDefinition {
            stops_combat_from_ending: false,
            must_perform_move_on_death: false,
            allow_negative: true,
            ..power(
                "POWER.STRENGTH",
                PowerType::Buff,
                vec![modifies(
                    HookEvent::ModifyDamageAdditive,
                    HookCondition::OwnerDealsPoweredDamage,
                    ModifierCmd::Add(Amount::counting(sts2_engine::Count::ListeningPowerAmount)),
                )],
            )
        },
        PowerDefinition {
            stops_combat_from_ending: false,
            must_perform_move_on_death: false,
            temporary: Some(TemporaryPower {
                internally_applied_power: id("POWER.STRENGTH"),
                positive: true,
            }),
            ..power("POWER.TEMP_STRENGTH", PowerType::Buff, Vec::new())
        },
    ]
}

/// One power per pass, plus the absorber that answers an application.
fn modifier_powers() -> Vec<PowerDefinition> {
    vec![
        power(
            "POWER.ADDS_TEN",
            PowerType::Buff,
            vec![modifies(
                HookEvent::ModifyDamageAdditive,
                HookCondition::OwnerDealsPoweredDamage,
                ModifierCmd::Add(Amount::Fixed(10)),
            )],
        ),
        power(
            "POWER.HALVES",
            PowerType::Debuff,
            vec![modifies(
                HookEvent::ModifyDamageMultiplicative,
                HookCondition::OwnerDealsPoweredDamage,
                ModifierCmd::Scale(Amount::Fixed(50)),
            )],
        ),
        power(
            "POWER.WEAKENS",
            PowerType::Debuff,
            vec![modifies(
                HookEvent::ModifyDamageMultiplicative,
                HookCondition::OwnerDealsPoweredDamage,
                ModifierCmd::Scale(Amount::Fixed(75)),
            )],
        ),
        power(
            "POWER.EXPOSES",
            PowerType::Debuff,
            vec![modifies(
                HookEvent::ModifyDamageMultiplicative,
                HookCondition::OwnerTakesPoweredDamage,
                ModifierCmd::Scale(Amount::Fixed(150)),
            )],
        ),
        // A cap makes no test of how the damage was worked out, so it holds
        // down unpowered damage too.
        power(
            "POWER.CAPS_AT_NINE",
            PowerType::Buff,
            vec![modifies(
                HookEvent::ModifyDamageCap,
                HookCondition::OwnerTakesDamage,
                ModifierCmd::CapAt(Amount::Fixed(9)),
            )],
        ),
        power(
            "POWER.CAPS_AT_FIVE",
            PowerType::Buff,
            vec![modifies(
                HookEvent::ModifyDamageCap,
                HookCondition::OwnerTakesDamage,
                ModifierCmd::CapAt(Amount::Fixed(5)),
            )],
        ),
        power(
            "POWER.SLIPPERY",
            PowerType::Buff,
            vec![modifies(
                HookEvent::ModifyHpLostAfterOsty,
                HookCondition::OwnerTakesDamage,
                ModifierCmd::CapAt(Amount::Fixed(1)),
            )],
        ),
        // An absorber, said the way the build says it: the power lands for
        // nothing, and the pass over whoever answered spends a stack.
        power(
            "POWER.ARTIFACT",
            PowerType::Buff,
            vec![
                ModifierHook::every_turn(
                    HookEvent::ModifyPowerAmountReceived,
                    vec![Command::When {
                        otherwise: Vec::new(),
                        condition: sts2_engine::Condition::AppliedPowerIsDebuff,
                        then: vec![Command::Modify(ModifierCmd::Set(Amount::Fixed(0)))],
                    }],
                )
                .when(HookCondition::PowerAppliedToOwner),
                ModifierHook::every_turn(
                    HookEvent::AfterModifyingPowerAmountReceived,
                    vec![Command::Power(PowerCmd::Decrement {
                        power: sts2_engine::PowerRef::listening(),
                    })],
                ),
            ],
        ),
        power("POWER.CURSE", PowerType::Debuff, Vec::new()),
    ]
}

/// The shape of Imbalanced: an `AfterDamageGiven` body that tests the damage
/// result it was handed, and marks its owner when block stopped the whole blow.
fn damage_result_power() -> PowerDefinition {
    power(
        "POWER.OFF_BALANCE",
        PowerType::Debuff,
        vec![
            ModifierHook::every_turn(
                HookEvent::AfterDamageGiven,
                vec![Command::Power(PowerCmd::Apply {
                    marks: None,
                    power: id("POWER.PLAIN"),
                    target: CreatureSelector::Owner,
                    applier: Some(CreatureSelector::Owner),
                    amount: Amount::Fixed(1),
                })],
            )
            .when(HookCondition::OwnerDealtFullyBlockedDamage),
        ],
    )
}

/// A relic that counts one listener of the post-damage pass, with whatever
/// test a build body would open with written as its condition.
fn counting_relic(model_id: &str, event: HookEvent, condition: HookCondition) -> RelicDefinition {
    RelicDefinition {
        model_id: id(model_id),
        grants_an_extra_turn: None,
        map_rule: None,
        rarity: RelicRarity::Common,
        counter_property: Some("counter".to_owned()),
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
        counter_kind: None,
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
                event,
                vec![Command::Relic(RelicCmd::ModifyListeningCounter(
                    Amount::Fixed(1),
                ))],
            )
            .when(condition),
        ],
    }
}

/// One counter per listener of the pass, plus one that makes the test the
/// pass itself no longer makes.
fn counting_relics() -> Vec<RelicDefinition> {
    vec![
        counting_relic(
            "RELIC.SEES_BLOCK_BROKEN",
            HookEvent::AfterBlockBroken,
            HookCondition::Always,
        ),
        counting_relic(
            "RELIC.SEES_HP_CHANGED",
            HookEvent::AfterCurrentHpChanged,
            HookCondition::Always,
        ),
        counting_relic(
            "RELIC.SEES_DAMAGE_GIVEN",
            HookEvent::AfterDamageGiven,
            HookCondition::Always,
        ),
        counting_relic(
            "RELIC.SEES_DAMAGE_RECEIVED",
            HookEvent::AfterDamageReceived,
            HookCondition::Always,
        ),
        counting_relic(
            "RELIC.SEES_OWN_HURT",
            HookEvent::AfterDamageReceived,
            HookCondition::OwnerTookUnblockedDamage,
        ),
    ]
}

/// Damage a creature deals to itself, which is the shortest way to a dealer
/// whose target is standing behind block.
fn self_damage(model_id: &str, amount: i32, props: ValueProps) -> CardDefinition {
    card(
        model_id,
        CardType::Skill,
        TargetType::SelfTarget,
        vec![Command::Creature(CreatureCmd::Damage {
            target: CreatureSelector::Owner,
            amount: Amount::Fixed(amount),
            props,
        })],
    )
}

/// A registry with one power per rule under test.
fn registry() -> Arc<ContentRegistry> {
    let mut content = ContentRegistry::default();
    for definition in stacking_powers() {
        content.add_power(definition);
    }
    for definition in modifier_powers() {
        content.add_power(definition);
    }
    content.add_power(damage_result_power());

    for definition in [
        attack_card("CARD.STRIKE", 6, 1, CreatureSelector::Target),
        attack_card("CARD.SPRAY", 1, 3, CreatureSelector::RandomOpponent),
        attack_card("CARD.HEAVY", 20, 1, CreatureSelector::Target),
        attack_card("CARD.FOUR", 4, 1, CreatureSelector::Target),
        applying("CARD.PLAIN", "POWER.PLAIN", 2, CreatureSelector::Owner),
        applying("CARD.PLAIN_MORE", "POWER.PLAIN", 3, CreatureSelector::Owner),
        applying(
            "CARD.INSTANCED",
            "POWER.INSTANCED",
            2,
            CreatureSelector::Owner,
        ),
        applying(
            "CARD.INSTANCED_MORE",
            "POWER.INSTANCED",
            3,
            CreatureSelector::Owner,
        ),
        applying(
            "CARD.LEND",
            "POWER.TEMP_STRENGTH",
            5,
            CreatureSelector::Owner,
        ),
        applying("CARD.DRAIN", "POWER.STRENGTH", -3, CreatureSelector::Owner),
        applying("CARD.RESTORE", "POWER.STRENGTH", 1, CreatureSelector::Owner),
        applying(
            "CARD.PLAIN_LESS",
            "POWER.PLAIN",
            -3,
            CreatureSelector::Owner,
        ),
        applying("CARD.WEAKEN", "POWER.WEAKENS", -2, CreatureSelector::Target),
        applying("CARD.CURSE", "POWER.CURSE", 2, CreatureSelector::Target),
        card(
            "CARD.SEAR",
            CardType::Skill,
            TargetType::AnyEnemy,
            vec![Command::Creature(CreatureCmd::Damage {
                target: CreatureSelector::Target,
                amount: Amount::Fixed(20),
                props: ValueProps::UNPOWERED,
            })],
        ),
        card(
            "CARD.GUARD",
            CardType::Skill,
            TargetType::SelfTarget,
            vec![Command::Creature(CreatureCmd::GainBlock {
                target: CreatureSelector::Owner,
                amount: Amount::Fixed(5),
                props: ValueProps::UNPOWERED,
            })],
        ),
        self_damage("CARD.SELF_HARM", 3, ValueProps::UNPOWERED),
        self_damage("CARD.SELF_GRAZE", 0, ValueProps::UNPOWERED),
        self_damage("CARD.SELF_PIERCE", 0, ValueProps::UNBLOCKABLE),
    ] {
        content.add_card(definition);
    }

    for definition in counting_relics() {
        content.add_relic(definition);
    }

    content.add_monster(dummy("MONSTER.IDLE", Vec::new(), Vec::new()));
    content.add_monster(dummy(
        "MONSTER.CURSER",
        Vec::new(),
        vec![applies_to_target("POWER.CURSE", 2)],
    ));
    content.add_monster(dummy(
        "MONSTER.MARKER",
        Vec::new(),
        vec![applies_to_target("POWER.PER_APPLIER", 1)],
    ));
    Arc::new(content)
}

fn scenario() -> ScenarioBuilder {
    ScenarioBuilder::new(SEED, id("ENCOUNTER.TEST"))
        .player_hp(80, 80)
        .energy(9, 9)
}

fn fingerprint(model: &str) -> CardFingerprint {
    CardFingerprint::base(id(model))
}

fn hand(mut builder: ScenarioBuilder, cards: &[&str]) -> ScenarioBuilder {
    for (index, model) in cards.iter().enumerate() {
        builder = builder.card(PileName::Hand, index as u64 + 1, fingerprint(model), 0);
    }
    builder
}

fn play(simulator: &mut Simulator, model: &str) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| match action {
            Action::PlayCard {
                card: CardHandle { fingerprint, .. },
                ..
            } => fingerprint.model_id.to_string() == model,
            _ => false,
        })
        .expect("the card is playable")
        .clone();
    simulator.step(action).expect("the play is legal");
}

fn powers_of(simulator: &Simulator, creature: usize) -> Vec<(String, i32)> {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("in combat")
        .creatures[creature]
        .powers
        .iter()
        .map(|power: &PowerInstance| (power.model_id.to_string(), power.amount))
        .collect()
}

fn enemy_hp(simulator: &Simulator, index: usize) -> i32 {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("in combat")
        .creatures[index]
        .current_hp
}

#[test]
fn a_plain_power_stacks_and_an_instanced_one_stands_up_a_second_instance() {
    let builder = hand(
        scenario(),
        &[
            "CARD.PLAIN",
            "CARD.PLAIN_MORE",
            "CARD.INSTANCED",
            "CARD.INSTANCED_MORE",
        ],
    )
    .enemy(1, id("MONSTER.IDLE"), 40, 40, "MOVE");
    let mut simulator = builder.build(registry()).unwrap();

    play(&mut simulator, "CARD.PLAIN");
    play(&mut simulator, "CARD.PLAIN_MORE");
    play(&mut simulator, "CARD.INSTANCED");
    play(&mut simulator, "CARD.INSTANCED_MORE");

    assert_eq!(
        powers_of(&simulator, 0),
        [
            ("POWER.PLAIN".to_owned(), 5),
            ("POWER.INSTANCED".to_owned(), 2),
            ("POWER.INSTANCED".to_owned(), 3),
        ],
        "a power with no instance type joins the instance already there; an \
         instanced one never does"
    );
}

#[test]
fn a_per_applier_power_stacks_only_onto_what_the_same_applier_left() {
    let mut simulator = scenario()
        .enemy(1, id("MONSTER.MARKER"), 40, 40, "MOVE")
        .enemy(2, id("MONSTER.MARKER"), 40, 40, "MOVE")
        .build(registry())
        .unwrap();

    simulator.step(Action::EndTurn { turn: 1 }).unwrap();
    assert_eq!(
        powers_of(&simulator, 0),
        [
            ("POWER.PER_APPLIER".to_owned(), 1),
            ("POWER.PER_APPLIER".to_owned(), 1),
        ],
        "two appliers leave two instances"
    );

    simulator.step(Action::EndTurn { turn: 2 }).unwrap();
    assert_eq!(
        powers_of(&simulator, 0),
        [
            ("POWER.PER_APPLIER".to_owned(), 2),
            ("POWER.PER_APPLIER".to_owned(), 2),
        ],
        "a second application by the same applier joins its own instance"
    );
}

#[test]
fn a_power_that_may_go_negative_falls_away_only_at_exactly_nothing() {
    let mut simulator = hand(
        scenario(),
        &[
            "CARD.LEND",
            "CARD.DRAIN",
            "CARD.DRAIN",
            "CARD.RESTORE",
            "CARD.PLAIN",
            "CARD.PLAIN_LESS",
        ],
    )
    .enemy(1, id("MONSTER.IDLE"), 40, 40, "MOVE")
    .build(registry())
    .unwrap();

    play(&mut simulator, "CARD.LEND");
    // The temporary power works the strength on before it is attached itself,
    // so the strength stands ahead of it.
    assert_eq!(
        powers_of(&simulator, 0),
        [
            ("POWER.STRENGTH".to_owned(), 5),
            ("POWER.TEMP_STRENGTH".to_owned(), 5),
        ]
    );

    play(&mut simulator, "CARD.DRAIN");
    assert_eq!(
        powers_of(&simulator, 0),
        [
            ("POWER.STRENGTH".to_owned(), 2),
            ("POWER.TEMP_STRENGTH".to_owned(), 5),
        ]
    );
    play(&mut simulator, "CARD.DRAIN");
    assert_eq!(
        powers_of(&simulator, 0),
        [
            ("POWER.STRENGTH".to_owned(), -1),
            ("POWER.TEMP_STRENGTH".to_owned(), 5),
        ],
        "a power that may go negative stands at less than nothing"
    );
    play(&mut simulator, "CARD.RESTORE");
    assert_eq!(
        powers_of(&simulator, 0),
        [("POWER.TEMP_STRENGTH".to_owned(), 5)],
        "and falls away when it is worked to exactly nothing"
    );

    play(&mut simulator, "CARD.PLAIN");
    play(&mut simulator, "CARD.PLAIN_LESS");
    assert_eq!(
        powers_of(&simulator, 0),
        [("POWER.TEMP_STRENGTH".to_owned(), 5)],
        "a power that may not go negative falls away at nothing or below"
    );
}

#[test]
fn a_temporary_power_gives_the_stat_back_when_the_turn_it_lent_it_for_ends() {
    let mut simulator = hand(scenario(), &["CARD.LEND", "CARD.STRIKE"])
        .enemy(1, id("MONSTER.IDLE"), 40, 40, "MOVE")
        .build(registry())
        .unwrap();

    play(&mut simulator, "CARD.LEND");
    play(&mut simulator, "CARD.STRIKE");
    assert_eq!(enemy_hp(&simulator, 1), 40 - 11, "six plus the lent five");

    simulator.step(Action::EndTurn { turn: 1 }).unwrap();
    assert_eq!(
        powers_of(&simulator, 0),
        [],
        "the wrapper takes itself off and hands the strength back"
    );
}

#[test]
fn every_addition_lands_before_every_multiplication() {
    let mut simulator = hand(scenario(), &["CARD.STRIKE"])
        .power(0, id("POWER.ADDS_TEN"), 1, PowerType::Buff)
        .power(0, id("POWER.HALVES"), 1, PowerType::Debuff)
        .enemy(1, id("MONSTER.IDLE"), 40, 40, "MOVE")
        .build(registry())
        .unwrap();

    play(&mut simulator, "CARD.STRIKE");
    // (6 + 10) halved is eight; halving first and then adding would be
    // thirteen.
    assert_eq!(enemy_hp(&simulator, 1), 40 - 8);
}

#[test]
fn the_product_of_the_multiplications_truncates_only_once() {
    let mut simulator = hand(scenario(), &["CARD.STRIKE"])
        .power(0, id("POWER.WEAKENS"), 1, PowerType::Debuff)
        .enemy(1, id("MONSTER.IDLE"), 40, 40, "MOVE")
        .power(1, id("POWER.EXPOSES"), 1, PowerType::Debuff)
        .build(registry())
        .unwrap();

    play(&mut simulator, "CARD.STRIKE");
    // 6 * 75% * 150% is 6.75, which truncates to six. Truncating between the
    // two factors would give 4 * 150% = 6 here, so the telling case is the odd
    // one below.
    assert_eq!(enemy_hp(&simulator, 1), 40 - 6);

    let mut simulator = hand(scenario(), &["CARD.FOUR"])
        .power(0, id("POWER.WEAKENS"), 1, PowerType::Debuff)
        .enemy(1, id("MONSTER.IDLE"), 40, 40, "MOVE")
        .power(1, id("POWER.EXPOSES"), 1, PowerType::Debuff)
        .build(registry())
        .unwrap();

    play(&mut simulator, "CARD.FOUR");
    // 4 * 75% * 150% is 4.5, which truncates to four. Truncating after the
    // first factor would leave three, and three and a half after the second.
    assert_eq!(enemy_hp(&simulator, 1), 40 - 4);
}

#[test]
fn a_cap_is_the_last_word_and_the_least_of_them_wins() {
    let mut simulator = hand(scenario(), &["CARD.HEAVY"])
        .enemy(1, id("MONSTER.IDLE"), 40, 40, "MOVE")
        .power(1, id("POWER.EXPOSES"), 1, PowerType::Debuff)
        .power(1, id("POWER.CAPS_AT_NINE"), 9, PowerType::Buff)
        .build(registry())
        .unwrap();

    play(&mut simulator, "CARD.HEAVY");
    assert_eq!(
        enemy_hp(&simulator, 1),
        40 - 9,
        "twenty scaled to thirty is still held to nine"
    );

    let mut simulator = hand(scenario(), &["CARD.HEAVY"])
        .enemy(1, id("MONSTER.IDLE"), 40, 40, "MOVE")
        .power(1, id("POWER.CAPS_AT_NINE"), 9, PowerType::Buff)
        .power(1, id("POWER.CAPS_AT_FIVE"), 5, PowerType::Buff)
        .build(registry())
        .unwrap();

    play(&mut simulator, "CARD.HEAVY");
    assert_eq!(enemy_hp(&simulator, 1), 40 - 5, "the least cap holds");
}

#[test]
fn a_cap_holds_unpowered_damage_down_although_a_scale_does_not() {
    let mut simulator = hand(scenario(), &["CARD.SEAR"])
        .power(0, id("POWER.ADDS_TEN"), 1, PowerType::Buff)
        .enemy(1, id("MONSTER.IDLE"), 40, 40, "MOVE")
        .power(1, id("POWER.CAPS_AT_NINE"), 9, PowerType::Buff)
        .build(registry())
        .unwrap();

    play(&mut simulator, "CARD.SEAR");
    assert_eq!(
        enemy_hp(&simulator, 1),
        40 - 9,
        "the cap makes no test of how the damage was worked out; the addition \
         does and stays out"
    );
}

#[test]
fn an_hp_loss_listener_holds_down_what_block_did_not_stop() {
    let mut simulator = hand(scenario(), &["CARD.HEAVY"])
        .enemy(1, id("MONSTER.IDLE"), 40, 40, "MOVE")
        .power(1, id("POWER.SLIPPERY"), 1, PowerType::Buff)
        .build(registry())
        .unwrap();

    play(&mut simulator, "CARD.HEAVY");
    assert_eq!(
        enemy_hp(&simulator, 1),
        40 - 1,
        "twenty gets through the modifier passes and costs one HP"
    );
}

#[test]
fn an_absorber_turns_a_debuff_away_and_is_spent_for_it() {
    let mut simulator = scenario()
        .power(0, id("POWER.ARTIFACT"), 2, PowerType::Buff)
        .enemy(1, id("MONSTER.CURSER"), 40, 40, "MOVE")
        .build(registry())
        .unwrap();

    simulator.step(Action::EndTurn { turn: 1 }).unwrap();
    assert_eq!(
        powers_of(&simulator, 0),
        [("POWER.ARTIFACT".to_owned(), 1)],
        "the curse lands for nothing and a stack pays for it"
    );

    simulator.step(Action::EndTurn { turn: 2 }).unwrap();
    assert_eq!(powers_of(&simulator, 0), []);

    simulator.step(Action::EndTurn { turn: 3 }).unwrap();
    assert_eq!(
        powers_of(&simulator, 0),
        [("POWER.CURSE".to_owned(), 2)],
        "with the absorber spent the curse lands"
    );
}

#[test]
fn an_absorber_turns_away_a_debuff_that_would_have_joined_one_already_there() {
    let mut simulator = scenario()
        .power(0, id("POWER.ARTIFACT"), 1, PowerType::Buff)
        .power(0, id("POWER.CURSE"), 4, PowerType::Debuff)
        .enemy(1, id("MONSTER.CURSER"), 40, 40, "MOVE")
        .build(registry())
        .unwrap();

    simulator.step(Action::EndTurn { turn: 1 }).unwrap();
    assert_eq!(
        powers_of(&simulator, 0),
        [("POWER.CURSE".to_owned(), 4)],
        "the search for something to stack onto comes first, and the absorber \
         turns the application away all the same -- spending its last stack, \
         which takes it out of the list the instance was found in"
    );
}

#[test]
fn a_debuff_worked_backwards_reads_as_a_buff_and_is_not_absorbed() {
    let mut simulator = hand(scenario(), &["CARD.WEAKEN"])
        .enemy(1, id("MONSTER.IDLE"), 40, 40, "MOVE")
        .power(1, id("POWER.ARTIFACT"), 2, PowerType::Buff)
        .build(registry())
        .unwrap();

    play(&mut simulator, "CARD.WEAKEN");
    assert_eq!(
        powers_of(&simulator, 1),
        [
            ("POWER.ARTIFACT".to_owned(), 2),
            ("POWER.WEAKENS".to_owned(), -2),
        ],
        "an absorber tests what the power counts as at the amount it carries"
    );
}

#[test]
fn a_random_target_rolls_once_a_hit_on_the_combat_targets_stream() {
    let mut simulator = hand(scenario(), &["CARD.SPRAY"])
        .enemy(1, id("MONSTER.IDLE"), 40, 40, "MOVE")
        .enemy(2, id("MONSTER.IDLE"), 40, 40, "MOVE")
        .enemy(3, id("MONSTER.IDLE"), 40, 40, "MOVE")
        .build(registry())
        .unwrap();
    assert_eq!(
        simulator
            .state()
            .rng
            .stream(RunStream::CombatTargets)
            .counter(),
        0
    );

    play(&mut simulator, "CARD.SPRAY");

    // The same three rolls, taken off a stream of the same name and seed.
    let mut expected = RunRngSet::new(SEED);
    let stream = expected.stream_mut(RunStream::CombatTargets);
    let mut damage = [0; 3];
    for _ in 0..3 {
        let index = stream.next_index(3).unwrap().unwrap();
        damage[index] += 1;
    }
    for (index, dealt) in damage.iter().enumerate() {
        assert_eq!(enemy_hp(&simulator, index + 1), 40 - dealt);
    }
    assert_eq!(
        simulator
            .state()
            .rng
            .stream(RunStream::CombatTargets)
            .counter(),
        3,
        "one roll per hit, and no more"
    );
}

#[test]
fn a_random_target_with_nobody_to_hit_reads_no_dice_at_all() {
    let mut simulator = hand(scenario(), &["CARD.SPRAY"])
        .enemy(1, id("MONSTER.IDLE"), 1, 40, "MOVE")
        .build(registry())
        .unwrap();

    play(&mut simulator, "CARD.SPRAY");
    assert_eq!(
        simulator
            .state()
            .rng
            .stream(RunStream::CombatTargets)
            .counter(),
        1,
        "the one hit that had somebody to land on rolled, and the fight ended \
         before the rest could"
    );
}

#[test]
fn a_damage_given_listener_reads_the_result_of_every_blow_its_owner_deals() {
    // The build announces damage given from the settling of the damage itself,
    // so a plain damage command reaches the listener as readily as an attack
    // does, and the listener reads whether the blow was fully blocked off the
    // result rather than being told an attack was stopped.
    let marked = |simulator: &Simulator| {
        powers_of(simulator, 0)
            .iter()
            .any(|(model, _)| model == "POWER.PLAIN")
    };

    let mut blocked = off_balance_scenario();
    play(&mut blocked, "CARD.GUARD");
    play(&mut blocked, "CARD.SELF_HARM");
    assert!(marked(&blocked), "block stopping the whole blow marks it");

    let mut hurt = off_balance_scenario();
    play(&mut hurt, "CARD.SELF_HARM");
    assert!(
        !marked(&hurt),
        "a blow nothing stopped is not a fully blocked one"
    );

    // Nothing dealt against standing block still counts as fully blocked: a
    // blow is fully blocked when block was involved at all — some taken off or
    // some still standing — and nothing at all got through.
    let mut grazed = off_balance_scenario();
    play(&mut grazed, "CARD.GUARD");
    play(&mut grazed, "CARD.SELF_GRAZE");
    assert!(marked(&grazed), "nothing dealt into block is fully blocked");

    let mut ungrazed = off_balance_scenario();
    play(&mut ungrazed, "CARD.SELF_GRAZE");
    assert!(
        !marked(&ungrazed),
        "and nothing dealt with no block behind it is not"
    );

    // Damage block was never allowed to stop is never fully blocked, however
    // little of it lands.
    let mut pierced = off_balance_scenario();
    play(&mut pierced, "CARD.GUARD");
    play(&mut pierced, "CARD.SELF_PIERCE");
    assert!(
        !marked(&pierced),
        "unblockable damage is not blocked at all"
    );
}

/// What the four listener methods of the post-damage loop saw.
#[derive(Debug, Eq, PartialEq)]
struct Seen {
    block_broken: i32,
    hp_changed: i32,
    given: i32,
    received: i32,
    own_hurt: i32,
}

fn seen(simulator: &Simulator) -> Seen {
    let counters: Vec<i32> = simulator
        .state()
        .run_player
        .relics
        .iter()
        .map(|relic| relic.counter)
        .collect();
    Seen {
        block_broken: counters[0],
        hp_changed: counters[1],
        given: counters[2],
        received: counters[3],
        own_hurt: counters[4],
    }
}

/// A player carrying one counter per listener, fighting an enemy with
/// `enemy_hp` and a second one that keeps the fight alive when the first dies.
fn watched_scenario(cards: &[&str], enemy_hp: i32) -> Simulator {
    hand(scenario(), cards)
        .enemy(1, id("MONSTER.IDLE"), enemy_hp, 40, "MOVE")
        .enemy(2, id("MONSTER.IDLE"), 40, 40, "MOVE")
        .relics(&[
            id("RELIC.SEES_BLOCK_BROKEN"),
            id("RELIC.SEES_HP_CHANGED"),
            id("RELIC.SEES_DAMAGE_GIVEN"),
            id("RELIC.SEES_DAMAGE_RECEIVED"),
            id("RELIC.SEES_OWN_HURT"),
        ])
        .build(registry())
        .unwrap()
}

#[test]
fn the_post_damage_loop_dispatches_exactly_where_the_build_dispatches() {
    // The second pass a blow makes over the settled result tests it four
    // times, once per listener, and every one of those tests is on the result
    // rather than on the listeners.

    // Block took the whole blow: nothing broke and nothing was lost, so the
    // first two do not fire — but the last two still do, because the
    // damage-given pass asks nothing of the result at all and the only skip on
    // the fourth is about the target being dead.
    let mut stopped = watched_scenario(&["CARD.GUARD", "CARD.SELF_HARM", "CARD.SELF_HARM"], 40);
    play(&mut stopped, "CARD.GUARD");
    play(&mut stopped, "CARD.SELF_HARM");
    assert_eq!(
        seen(&stopped),
        Seen {
            block_broken: 0,
            hp_changed: 0,
            given: 1,
            received: 1,
            own_hurt: 0,
        },
        "a blow block stopped whole is still offered to both damage methods, \
         and the body that only wants a costly one says so itself"
    );

    // The next blow of three finds two block left: block runs out and one HP
    // goes with it, so all four fire.
    play(&mut stopped, "CARD.SELF_HARM");
    assert_eq!(
        seen(&stopped),
        Seen {
            block_broken: 1,
            hp_changed: 1,
            given: 2,
            received: 2,
            own_hurt: 1,
        },
        "block running out and HP coming off are each one dispatch of their own"
    );

    // A blow that killed its target is answered for by the death instead:
    // a target the blow killed, and that is dead, skips the fourth listener
    // and nothing else. The HP change is not skipped: the health-changed pass
    // walks every listener there is and hands each of them the creature whose
    // health moved, so a relic on the player answers for an enemy's health as
    // readily as for its owner's, and the body that only wants its owner's
    // says so itself.
    let mut killing = watched_scenario(&["CARD.SEAR"], 6);
    play(&mut killing, "CARD.SEAR");
    assert_eq!(
        killing
            .state()
            .combat
            .as_ref()
            .expect("the second enemy keeps the fight alive")
            .creatures
            .len(),
        2,
        "the blow killed what it landed on"
    );
    assert_eq!(
        seen(&killing),
        Seen {
            block_broken: 0,
            hp_changed: 1,
            given: 1,
            received: 0,
            own_hurt: 0,
        },
        "the killing blow is given but never received"
    );

    // The same blow against a creature that lives through it is received, and
    // by the whole listener list rather than by the creature that took it: the
    // player's own relic hears about an enemy being hurt.
    let mut survived = watched_scenario(&["CARD.SEAR"], 40);
    play(&mut survived, "CARD.SEAR");
    assert_eq!(
        seen(&survived),
        Seen {
            block_broken: 0,
            hp_changed: 1,
            given: 1,
            received: 1,
            own_hurt: 0,
        },
        "a listener that is not the one that was hurt hears about it anyway"
    );
}

/// A player who is off balance, with the cards to hurt themselves through their
/// own block.
fn off_balance_scenario() -> Simulator {
    hand(
        scenario(),
        &[
            "CARD.GUARD",
            "CARD.SELF_HARM",
            "CARD.SELF_GRAZE",
            "CARD.SELF_PIERCE",
        ],
    )
    .enemy(1, id("MONSTER.IDLE"), 40, 40, "MOVE")
    .power(0, id("POWER.OFF_BALANCE"), 1, PowerType::Debuff)
    .build(registry())
    .unwrap()
}
