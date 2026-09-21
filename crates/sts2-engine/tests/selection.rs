//! The card-selection framework: what each screen offers, when it asks at all,
//! and what happens to the work a screen interrupted.

use std::sync::Arc;

use sts2_core::ModelId;
use sts2_engine::{
    Action, Amount, CardCmd, CardDefinition, CardFilter, CardFingerprint, CardPileCmd, CardRarity,
    CardSelectCmd, CardSelector, CardType, ChoiceIdentity, Command, ContentRegistry, Count,
    CreatureCmd, CreatureSelector, DamageCmd, DecisionContext, ErrorCode, HookCondition, HookEvent,
    Intent, ModifierCmd, ModifierHook, MonsterDefinition, MonsterMove, MonsterState, MoveIntent,
    PileName, PowerCmd, PowerDefinition, PowerInstanceType, PowerName, PowerRef, PowerStackType,
    PowerType, RelicCmd, RelicDefinition, RelicRarity, ScenarioBuilder, SelectorPrefs, Simulator,
    TargetType, ValueProps,
};

const SEED: &str = "SELECTTEST";

fn id(value: &str) -> ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str, rarity: CardRarity, effects: Vec<Command>) -> CardDefinition {
    CardDefinition {
        model_id: id(model_id),
        rarity,
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
        card_type: CardType::Skill,
        target: TargetType::SelfTarget,
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

/// A card whose upgraded form exists, which is what makes it upgradable.
fn upgradable(mut definition: CardDefinition) -> CardDefinition {
    definition.supported_upgrade_levels.push(1);
    definition.upgrade = Some(sts2_engine::CardUpgrade {
        energy_cost: None,
        innate: false,
        retain: false,
        drops_exhaust: false,
        drops_ethereal: false,
        damage_var: None,
        effects: definition.effects.clone(),
    });
    definition
}

fn power(model_id: &str, hooks: Vec<ModifierHook>) -> PowerDefinition {
    PowerDefinition {
        stops_combat_from_ending: false,
        must_perform_move_on_death: false,
        model_id: id(model_id),
        power_type: PowerType::Buff,
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

fn relic(model_id: &str, hooks: Vec<ModifierHook>) -> RelicDefinition {
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
        hooks,
    }
}

fn block(amount: i32) -> Command {
    Command::Creature(CreatureCmd::GainBlock {
        target: CreatureSelector::Owner,
        amount: Amount::Fixed(amount),
        props: ValueProps::UNPOWERED,
    })
}

/// A hand screen asking for exactly one card, which shows whenever the hand
/// holds more than one.
fn hand_screen(then: Vec<Command>) -> Command {
    Command::CardSelect(CardSelectCmd::FromHand {
        maximum: None,
        exact_count: None,
        prefs: SelectorPrefs::exactly(1),
        filter: CardFilter::Any,
        then,
    })
}

fn exhaust_chosen() -> Command {
    Command::Card(CardCmd::Exhaust {
        cards: CardSelector::Chosen,
    })
}

/// The cards the screens in this file are written on.
#[allow(
    clippy::too_many_lines,
    reason = "one card per screen shape, each written out where it is read"
)]
fn cards() -> Vec<CardDefinition> {
    vec![
        card("CARD.FILLER", CardRarity::Basic, Vec::new()),
        // A body with work on both sides of the screen: the block after it must
        // land only once the player has answered.
        card(
            "CARD.ASK_THEN_BLOCK",
            CardRarity::Basic,
            vec![hand_screen(vec![exhaust_chosen()]), block(7)],
        ),
        // A screen over a pile that has nothing on it hands back nothing, and
        // the rest of the body runs anyway.
        card(
            "CARD.ASK_EMPTY_PILE",
            CardRarity::Basic,
            vec![
                Command::CardSelect(CardSelectCmd::FromCombatPile {
                    exact_count: None,
                    pile: PileName::Exhaust,
                    prefs: SelectorPrefs::exactly(1),
                    maximum: None,
                    restricted_to_chosen: false,
                    filter: CardFilter::Any,
                    // The rest of the body still runs once the screen is
                    // done, the way the build's does; what it acts on is
                    // what the screen handed back, which is nothing.
                    then: vec![Command::Creature(CreatureCmd::GainBlock {
                        target: CreatureSelector::Owner,
                        amount: Amount::per(0, 100, Count::ChosenCards),
                        props: ValueProps::UNPOWERED,
                    })],
                }),
                block(7),
            ],
        ),
        // A screen over the draw pile, which is never presented in draw order.
        card(
            "CARD.ASK_DRAW_PILE",
            CardRarity::Basic,
            vec![Command::CardSelect(CardSelectCmd::FromCombatPile {
                exact_count: None,
                pile: PileName::Draw,
                prefs: SelectorPrefs::exactly(1),
                maximum: None,
                restricted_to_chosen: false,
                filter: CardFilter::Any,
                then: vec![exhaust_chosen()],
            })],
        ),
        card(
            "CARD.ASK_CANCELABLE",
            CardRarity::Basic,
            vec![
                Command::CardSelect(CardSelectCmd::FromHand {
                    maximum: None,
                    exact_count: None,
                    prefs: SelectorPrefs::exactly(1).cancelable().confirmed(),
                    filter: CardFilter::Any,
                    then: vec![exhaust_chosen()],
                }),
                block(7),
            ],
        ),
        card(
            "CARD.ASK_HAND_UPGRADE",
            CardRarity::Basic,
            vec![Command::CardSelect(CardSelectCmd::FromHandForUpgrade {
                then: vec![Command::Card(CardCmd::Upgrade {
                    cards: CardSelector::Chosen,
                })],
            })],
        ),
        // A screen over the run deck, which names its cards by deck position.
        card(
            "CARD.ASK_DECK_REMOVAL",
            CardRarity::Basic,
            vec![Command::CardSelect(CardSelectCmd::FromDeckForRemoval {
                prefs: SelectorPrefs::exactly(1).confirmed(),
                filter: CardFilter::Any,
                then: vec![Command::CardPile(CardPileCmd::RemoveFromDeck {
                    cards: CardSelector::Chosen,
                })],
            })],
        ),
        // A shaped screen naming a model this registry does not carry.
        card(
            "CARD.ASK_ENCHANT",
            CardRarity::Basic,
            vec![Command::CardSelect(CardSelectCmd::FromDeckForEnchantment {
                enchantment: id("ENCHANTMENT.TEST"),
                amount: 1,
                filter: CardFilter::Any,
                shuffled_on: None,
                prefs: SelectorPrefs::exactly(1),
                then: Vec::new(),
            })],
        ),
        // A grid over cards a creation has just made.
        card(
            "CARD.ASK_GRID",
            CardRarity::Basic,
            vec![Command::CardSelect(CardSelectCmd::SimpleGridForRewards {
                creations: vec![
                    sts2_engine::CardCreation {
                        pools: vec![sts2_engine::CardPoolRef::Named(id("CARD_POOL.TEST"))],
                        filter: CardFilter::OfRarity(CardRarity::Common),
                        odds: sts2_engine::RarityOdds::Uniform,
                        from_an_encounter: false,
                        count: 2,
                        flags: sts2_engine::CardCreationFlags {
                            no_upgrade_roll: true,
                            ..sts2_engine::CardCreationFlags::default()
                        },
                    },
                    sts2_engine::CardCreation {
                        pools: vec![sts2_engine::CardPoolRef::Named(id("CARD_POOL.TEST"))],
                        filter: CardFilter::OfRarity(CardRarity::Uncommon),
                        odds: sts2_engine::RarityOdds::Uniform,
                        from_an_encounter: false,
                        count: 2,
                        flags: sts2_engine::CardCreationFlags {
                            no_upgrade_roll: true,
                            ..sts2_engine::CardCreationFlags::default()
                        },
                    },
                ],
                prefs: SelectorPrefs {
                    minimum: 0,
                    maximum: 4,
                    cancelable: false,
                    require_manual_confirmation: true,
                },
                then: Vec::new(),
            })],
        ),
        // A screen answered with one whole bundle rather than with cards.
        card(
            "CARD.ASK_BUNDLE",
            CardRarity::Basic,
            vec![Command::CardSelect(CardSelectCmd::ChooseAGeneratedBundle {
                pools: vec![sts2_engine::CardPoolRef::Named(id("CARD_POOL.TEST"))],
                count: 2,
                commons: 2,
                uncommons: 1,
                then: Vec::new(),
            })],
        ),
        // A body naming the listening power where no listener bound one.
        card(
            "CARD.BAD_LISTENING",
            CardRarity::Basic,
            vec![Command::Power(PowerCmd::Decrement {
                power: PowerRef::listening(),
            })],
        ),
        upgradable(card("CARD.UPGRADABLE", CardRarity::Common, Vec::new())),
        card("CARD.COMMON_B", CardRarity::Common, Vec::new()),
        card("CARD.RARE_X", CardRarity::Rare, Vec::new()),
        card(
            "CARD.PLAIN_ATTACK",
            CardRarity::Basic,
            vec![Command::Damage(DamageCmd::Attack {
                amount: Amount::Fixed(4),
                hits: Amount::Fixed(1),
                target: CreatureSelector::Target,
                props: ValueProps::NONE,
            })],
        ),
        // An attack whose own number is read off the pass standing over it.
        // Nothing scales it, so `OwnerDealsPoweredDamage` never holds for it
        // and the asking power stays silent while its face is drawn.
        card(
            "CARD.CONTEXT_READER",
            CardRarity::Basic,
            vec![Command::Damage(DamageCmd::Attack {
                amount: Amount::Scaled {
                    base: 4,
                    per: 1,
                    count: Count::DamageDealerPower(id("POWER.MODIFY_ASKER")),
                },
                hits: Amount::Fixed(1),
                target: CreatureSelector::Target,
                props: ValueProps::UNPOWERED,
            })],
        ),
    ]
}

/// The powers whose listener bodies open screens.
#[allow(
    clippy::too_many_lines,
    reason = "one power per listener shape, each written out where it is read"
)]
fn powers() -> Vec<PowerDefinition> {
    vec![
        power(
            "POWER.ASKER",
            vec![ModifierHook::every_turn(
                HookEvent::AfterCardPlayed,
                vec![hand_screen(vec![
                    exhaust_chosen(),
                    // Resolved against the power whose listener opened the
                    // screen, which only works if the attribution survived the
                    // answer.
                    Command::Power(PowerCmd::SetCounter {
                        power: PowerRef::listening(),
                        amount: Amount::counting(Count::ListeningPowerAmount),
                    }),
                ])],
            )],
        ),
        power(
            "POWER.ANSWERER",
            vec![ModifierHook::every_turn(
                HookEvent::AfterCardPlayed,
                vec![block(3)],
            )],
        ),
        // An asker that takes the answerer off its owner while the screen is up.
        power(
            "POWER.ASK_THEN_STRIP",
            vec![ModifierHook::every_turn(
                HookEvent::AfterCardPlayed,
                vec![hand_screen(vec![Command::Power(PowerCmd::Remove {
                    power: PowerRef {
                        owner: CreatureSelector::Owner,
                        power: PowerName::Model(id("POWER.ANSWERER")),
                    },
                })])],
            )],
        ),
        // An asker that takes the blocking relic away while the screen is up.
        power(
            "POWER.ASK_THEN_MELT",
            vec![ModifierHook::every_turn(
                HookEvent::AfterCardPlayed,
                vec![hand_screen(vec![Command::Relic(RelicCmd::Remove(id(
                    "RELIC.BLOCKER",
                )))])],
            )],
        ),
        power(
            "POWER.MODIFY_ASKER",
            vec![
                ModifierHook::every_turn(
                    HookEvent::ModifyDamageAdditive,
                    vec![hand_screen(vec![Command::Modify(ModifierCmd::Add(
                        Amount::Fixed(1),
                    ))])],
                )
                .when(HookCondition::OwnerDealsPoweredDamage),
            ],
        ),
        // A turn-end listener is one of the listeners allowed to ask the
        // player something: the turn-end pass gives each listener a question
        // of its own, carries each body only as far as the point where it
        // pauses to ask, and comes back for the paused bodies after its last
        // pass — which is where the banked screens open.
        power(
            "POWER.TURN_END_ASKER",
            vec![ModifierHook::every_turn(
                HookEvent::BeforeSideTurnEnd,
                vec![hand_screen(Vec::new())],
            )],
        ),
        // The same asker on the site's first pass, for the tests that need two
        // screens outstanding at once.
        power(
            "POWER.EARLY_END_ASKER",
            vec![ModifierHook::every_turn(
                HookEvent::BeforeSideTurnEndVeryEarly,
                vec![hand_screen(Vec::new())],
            )],
        ),
        // A listener behind a paused body: its block says whether the pass
        // really ran on past the screen.
        power(
            "POWER.TURN_END_BLOCKER",
            vec![ModifierHook::every_turn(
                HookEvent::BeforeSideTurnEnd,
                vec![block(3)],
            )],
        ),
        // The pass at the start of a side's turn also defers its bodies, but
        // never comes back for them: a paused body is left waiting with
        // nothing that will ever join it, and a screen put up there stops the
        // fight. It asks over the discard pile because the hand is already
        // down by the enemy side's turn.
        power(
            "POWER.SIDE_OPEN_ASKER",
            vec![ModifierHook::every_turn(
                HookEvent::BeforeSideTurnStart,
                vec![Command::CardSelect(CardSelectCmd::FromCombatPile {
                    pile: PileName::Discard,
                    prefs: SelectorPrefs::exactly(1),
                    maximum: None,
                    exact_count: None,
                    restricted_to_chosen: false,
                    filter: CardFilter::Any,
                    then: Vec::new(),
                })],
            )],
        ),
        // A listener that runs after a side's turn has started is given no
        // way to ask the player anything at all.
        power(
            "POWER.SIDE_START_ASKER",
            vec![ModifierHook::every_turn(
                HookEvent::AfterSideTurnStart,
                vec![hand_screen(Vec::new())],
            )],
        ),
        power(
            "POWER.TURN_START_ASKER",
            vec![ModifierHook::every_turn(
                HookEvent::AfterPlayerTurnStart,
                vec![hand_screen(Vec::new())],
            )],
        ),
    ]
}

fn registry() -> Arc<ContentRegistry> {
    let mut registry = ContentRegistry::default();
    for mut definition in cards() {
        if definition.model_id == id("CARD.PLAIN_ATTACK")
            || definition.model_id == id("CARD.CONTEXT_READER")
        {
            definition.card_type = CardType::Attack;
            definition.target = TargetType::AnyEnemy;
        }
        registry.add_card(definition);
    }
    for definition in powers() {
        registry.add_power(definition);
    }
    // A small pool for the screens that make their own cards: four commons and
    // two uncommons is exactly what two bundles of two commons and one uncommon
    // take. Every card of a pool is a registered card, the way it is in
    // content, because a creation's filter asks the card what it is.
    let pool = [
        ("CARD.POOL_COMMON_A", CardRarity::Common),
        ("CARD.POOL_COMMON_B", CardRarity::Common),
        ("CARD.POOL_COMMON_C", CardRarity::Common),
        ("CARD.POOL_COMMON_D", CardRarity::Common),
        ("CARD.POOL_UNCOMMON_A", CardRarity::Uncommon),
        ("CARD.POOL_UNCOMMON_B", CardRarity::Uncommon),
    ];
    for (model_id, rarity) in pool {
        registry.add_card(card(model_id, rarity, Vec::new()));
    }
    registry.add_card_pool(sts2_engine::CardPoolDefinition {
        model_id: id("CARD_POOL.TEST"),
        is_colorless: false,
        cards: pool
            .into_iter()
            .map(|(model_id, rarity)| sts2_engine::PoolCard {
                model_id: id(model_id),
                card_type: CardType::Skill,
                rarity,
                energy_cost: 1,
                costs_x: false,
                can_be_generated_in_combat: true,
                can_be_generated_by_modifiers: true,
            })
            .collect(),
    });
    registry.add_relic(relic(
        "RELIC.BLOCKER",
        vec![ModifierHook::every_turn(
            HookEvent::AfterCardPlayed,
            vec![block(5)],
        )],
    ));
    registry.add_relic(relic(
        "RELIC.COUNTER",
        vec![ModifierHook::every_turn(
            HookEvent::AfterCardPlayed,
            vec![Command::Relic(RelicCmd::ModifyListeningCounter(
                Amount::Fixed(1),
            ))],
        )],
    ));
    registry.add_monster(MonsterDefinition {
        model_id: id("MONSTER.DUMMY"),
        min_initial_hp: 40,
        max_initial_hp: 40,
        initial_state: "WAIT".to_owned(),
        states: vec![MonsterState::Move(MonsterMove {
            state_id: "WAIT".to_owned(),
            intent: Intent::new("WAIT", vec![MoveIntent::of("unknown")]),
            commands: Vec::new(),
            follow_up: String::new(),
        })],
        on_added: Vec::new(),
        hooks: Vec::new(),
    });
    Arc::new(registry)
}

fn fingerprint(model: &str) -> CardFingerprint {
    CardFingerprint::base(id(model))
}

fn builder() -> ScenarioBuilder {
    ScenarioBuilder::new(SEED, id("ENCOUNTER.TEST"))
        .player_hp(60, 80)
        .enemy(1, id("MONSTER.DUMMY"), 40, 40, "WAIT")
}

fn play(simulator: &Simulator, model: &str) -> Action {
    simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::PlayCard { card, .. }
                if card.fingerprint.model_id.to_string() == model)
        })
        .unwrap_or_else(|| panic!("{model} is playable"))
        .clone()
}

fn candidates(simulator: &Simulator) -> Vec<String> {
    match simulator.decision() {
        DecisionContext::ChooseCards { candidates, .. } => candidates
            .iter()
            .map(|handle| handle.fingerprint.model_id.to_string())
            .collect(),
        other => panic!("expected a card choice, found {other:?}"),
    }
}

/// The answer naming one offered card, which is the first single-card pick.
fn pick_first(simulator: &Simulator) -> Action {
    simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseCards { cards, .. } if cards.len() == 1))
        .expect("a single-card answer is offered")
        .clone()
}

fn block_of(simulator: &Simulator) -> i32 {
    simulator.state().combat.as_ref().unwrap().creatures[0].block
}

fn relic_counter(simulator: &Simulator, model: &str) -> Option<i32> {
    simulator
        .state()
        .run_player
        .relics
        .iter()
        .find(|relic| relic.model_id == id(model))
        .map(|relic| relic.counter)
}

#[test]
fn a_screen_stops_the_body_that_opened_it_and_the_pass_it_was_in() {
    // The card's own body has a block after the screen, and behind the whole
    // body stands the AfterCardPlayed pass with a counting relic on it. Neither
    // may run while the player is still being asked.
    let mut simulator = builder()
        .relics(&[id("RELIC.COUNTER")])
        .card(PileName::Hand, 1, fingerprint("CARD.ASK_THEN_BLOCK"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 3, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    simulator
        .step(play(&simulator, "CARD.ASK_THEN_BLOCK"))
        .unwrap();

    assert!(matches!(
        simulator.decision(),
        DecisionContext::ChooseCards { .. }
    ));
    assert_eq!(block_of(&simulator), 0, "the body stopped at the screen");
    assert_eq!(
        relic_counter(&simulator, "RELIC.COUNTER"),
        Some(0),
        "and the pass behind it has not started"
    );

    simulator.step(pick_first(&simulator)).unwrap();
    assert_eq!(block_of(&simulator), 7, "the rest of the body ran");
    assert_eq!(
        relic_counter(&simulator, "RELIC.COUNTER"),
        Some(1),
        "and then the pass behind it"
    );
    assert_eq!(
        simulator
            .state()
            .combat
            .as_ref()
            .unwrap()
            .player
            .piles
            .exhaust
            .len(),
        1,
        "what was picked was exhausted"
    );
}

#[test]
fn a_pass_a_screen_interrupted_picks_up_at_the_next_entry() {
    // The asking power comes first in the listener sequence and the counting
    // relic after it, because a creature's powers are walked before what the
    // player carries.
    let mut simulator = builder()
        .power(0, id("POWER.ASKER"), 4, PowerType::Buff)
        .relics(&[id("RELIC.COUNTER")])
        .card(PileName::Hand, 1, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 3, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    simulator.step(play(&simulator, "CARD.FILLER")).unwrap();
    assert!(matches!(
        simulator.decision(),
        DecisionContext::ChooseCards { .. }
    ));
    assert_eq!(relic_counter(&simulator, "RELIC.COUNTER"), Some(0));

    simulator.step(pick_first(&simulator)).unwrap();
    assert_eq!(
        relic_counter(&simulator, "RELIC.COUNTER"),
        Some(1),
        "the entry after the one that asked is reached once"
    );
    // The screen resolved PowerRef::listening on the far side of the answer,
    // which it can only do if the choice remembered which power opened it.
    let asker = simulator.state().combat.as_ref().unwrap().creatures[0]
        .powers
        .iter()
        .find(|power| power.model_id == id("POWER.ASKER"))
        .expect("the asking power is still there");
    assert_eq!(
        asker.counter, 4,
        "the resumed body read the listening power's own amount"
    );
}

#[test]
fn liveness_is_rechecked_after_the_player_answers() {
    // A power is checked by its owner, never by itself, so one taken off a
    // creature still in the fight answers for the rest of the pass even though
    // the pass was put down and picked back up in between.
    let mut stripped = builder()
        .power(0, id("POWER.ASK_THEN_STRIP"), 1, PowerType::Buff)
        .power(0, id("POWER.ANSWERER"), 1, PowerType::Buff)
        .card(PileName::Hand, 1, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 3, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    stripped.step(play(&stripped, "CARD.FILLER")).unwrap();
    stripped.step(pick_first(&stripped)).unwrap();
    assert!(
        stripped.state().combat.as_ref().unwrap().creatures[0]
            .powers
            .iter()
            .all(|power| power.model_id != id("POWER.ANSWERER")),
        "the power really was taken off"
    );
    assert_eq!(
        block_of(&stripped),
        3,
        "a power removed while the screen was up still answers"
    );

    // A relic is checked by itself as well as by its owner, so one taken away
    // while the screen was up is skipped where it stands.
    let mut melted = builder()
        .power(0, id("POWER.ASK_THEN_MELT"), 1, PowerType::Buff)
        .relics(&[id("RELIC.BLOCKER")])
        .card(PileName::Hand, 1, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 3, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    melted.step(play(&melted, "CARD.FILLER")).unwrap();
    melted.step(pick_first(&melted)).unwrap();
    assert!(
        melted.state().run_player.relics.is_empty(),
        "the relic really was taken away"
    );
    assert_eq!(
        block_of(&melted),
        0,
        "a relic removed while the screen was up does not answer"
    );
}

#[test]
fn a_screen_with_nothing_to_offer_never_asks_and_the_body_carries_on() {
    let mut simulator = builder()
        .card(PileName::Hand, 1, fingerprint("CARD.ASK_EMPTY_PILE"), 0)
        .build(registry())
        .unwrap();
    simulator
        .step(play(&simulator, "CARD.ASK_EMPTY_PILE"))
        .unwrap();
    assert!(matches!(
        simulator.decision(),
        DecisionContext::CombatPriority { .. }
    ));
    assert_eq!(
        block_of(&simulator),
        7,
        "the screen handed back nothing and the body finished"
    );
}

#[test]
fn a_screen_asking_for_everything_it_has_takes_it_without_asking() {
    // A screen that does not ask for confirmation and is offered no more
    // cards than the minimum it wants takes them without opening: one card in
    // hand and a screen wanting one takes it.
    let mut single = builder()
        .card(PileName::Hand, 1, fingerprint("CARD.ASK_THEN_BLOCK"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    single.step(play(&single, "CARD.ASK_THEN_BLOCK")).unwrap();
    assert!(matches!(
        single.decision(),
        DecisionContext::CombatPriority { .. }
    ));
    assert_eq!(block_of(&single), 7);
    assert_eq!(
        single
            .state()
            .combat
            .as_ref()
            .unwrap()
            .player
            .piles
            .exhaust
            .len(),
        1,
        "the only candidate was taken"
    );

    // A caller that demands confirmation is asked even so.
    let mut confirmed = builder()
        .card(PileName::Hand, 1, fingerprint("CARD.ASK_CANCELABLE"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    confirmed
        .step(play(&confirmed, "CARD.ASK_CANCELABLE"))
        .unwrap();
    assert!(matches!(
        confirmed.decision(),
        DecisionContext::ChooseCards { .. }
    ));
}

#[test]
fn only_a_cancelable_screen_accepts_no_answer_at_all() {
    let taking_nothing =
        |action: &Action| matches!(action, Action::ChooseCards { cards, .. } if cards.is_empty());

    let mut strict = builder()
        .card(PileName::Hand, 1, fingerprint("CARD.ASK_THEN_BLOCK"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 3, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    strict.step(play(&strict, "CARD.ASK_THEN_BLOCK")).unwrap();
    assert!(
        !strict.legal_actions().iter().any(taking_nothing),
        "a screen asking for exactly one card cannot be left empty-handed"
    );

    let mut cancelable = builder()
        .card(PileName::Hand, 1, fingerprint("CARD.ASK_CANCELABLE"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 3, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    cancelable
        .step(play(&cancelable, "CARD.ASK_CANCELABLE"))
        .unwrap();
    let backing_out = cancelable
        .legal_actions()
        .iter()
        .find(|action| taking_nothing(action))
        .expect("a cancelable screen can be backed out of")
        .clone();
    cancelable.step(backing_out).unwrap();
    assert_eq!(
        block_of(&cancelable),
        7,
        "backing out still finishes the body"
    );
    assert!(
        cancelable
            .state()
            .combat
            .as_ref()
            .unwrap()
            .player
            .piles
            .exhaust
            .is_empty(),
        "and nothing was picked"
    );
}

#[test]
fn a_screen_over_the_draw_pile_is_not_shown_in_draw_order() {
    // The draw pile's order is the draw order, so the build never shows it: a
    // screen over the draw pile is laid out by rarity and then by id.
    let mut simulator = builder()
        .card(PileName::Hand, 1, fingerprint("CARD.ASK_DRAW_PILE"), 0)
        .card(PileName::Draw, 2, fingerprint("CARD.RARE_X"), 0)
        .card(PileName::Draw, 3, fingerprint("CARD.COMMON_B"), 0)
        .card(PileName::Draw, 4, fingerprint("CARD.UPGRADABLE"), 0)
        .build(registry())
        .unwrap();
    simulator
        .step(play(&simulator, "CARD.ASK_DRAW_PILE"))
        .unwrap();
    assert_eq!(
        candidates(&simulator),
        vec![
            "CARD.COMMON_B".to_owned(),
            "CARD.UPGRADABLE".to_owned(),
            "CARD.RARE_X".to_owned(),
        ],
        "the two commons come first, in id order, and the rare last"
    );
    assert!(matches!(
        simulator.decision(),
        DecisionContext::ChooseCards {
            identity: ChoiceIdentity::CombatCard,
            ..
        }
    ));
}

#[test]
fn a_hand_upgrade_screen_offers_only_what_can_be_upgraded() {
    // One upgradable card is taken without asking, which is the build's own
    // shortcut whenever a screen is left one candidate or none.
    let mut single = builder()
        .card(PileName::Hand, 1, fingerprint("CARD.ASK_HAND_UPGRADE"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.UPGRADABLE"), 0)
        .card(PileName::Hand, 3, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    single.step(play(&single, "CARD.ASK_HAND_UPGRADE")).unwrap();
    assert!(matches!(
        single.decision(),
        DecisionContext::CombatPriority { .. }
    ));
    let hand = &single.state().combat.as_ref().unwrap().player.piles.hand;
    assert_eq!(hand[0].fingerprint.upgrade_level, 1);

    // Two of them is a question, and the unupgradable card is not on it.
    let mut asked = builder()
        .card(PileName::Hand, 1, fingerprint("CARD.ASK_HAND_UPGRADE"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.UPGRADABLE"), 0)
        .card(PileName::Hand, 3, fingerprint("CARD.UPGRADABLE"), 0)
        .card(PileName::Hand, 4, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    asked.step(play(&asked, "CARD.ASK_HAND_UPGRADE")).unwrap();
    assert_eq!(
        candidates(&asked),
        vec!["CARD.UPGRADABLE".to_owned(), "CARD.UPGRADABLE".to_owned()]
    );
}

#[test]
fn a_deck_screen_names_its_cards_by_where_they_stand_in_the_deck() {
    let mut simulator = builder()
        .card(PileName::Hand, 1, fingerprint("CARD.ASK_DECK_REMOVAL"), 0)
        .card(PileName::Draw, 2, fingerprint("CARD.RARE_X"), 0)
        .card(PileName::Draw, 3, fingerprint("CARD.COMMON_B"), 0)
        .build(registry())
        .unwrap();
    let deck_before = simulator.state().run_player.deck.clone();
    simulator
        .step(play(&simulator, "CARD.ASK_DECK_REMOVAL"))
        .unwrap();
    let DecisionContext::ChooseCards {
        identity,
        candidates,
        ..
    } = simulator.decision().clone()
    else {
        panic!("a deck screen is up");
    };
    assert_eq!(identity, ChoiceIdentity::DeckCard);
    assert_eq!(
        candidates
            .iter()
            .map(|handle| (handle.index, handle.fingerprint.model_id.to_string()))
            .collect::<Vec<_>>(),
        deck_before
            .iter()
            .enumerate()
            .map(|(index, fingerprint)| (index, fingerprint.model_id.to_string()))
            .collect::<Vec<_>>(),
        "the deck is offered in deck order, named by deck position"
    );

    let answer = Action::ChooseCards {
        choice_id: match simulator.decision() {
            DecisionContext::ChooseCards { choice_id, .. } => *choice_id,
            other => panic!("expected a card choice, found {other:?}"),
        },
        cards: vec![candidates[1].clone()],
    };
    simulator.step(answer).unwrap();
    assert_eq!(
        simulator.state().run_player.deck.len(),
        deck_before.len() - 1
    );
    assert!(
        !simulator.state().run_player.deck.contains(&deck_before[1]),
        "the card at the chosen deck position is the one that left"
    );
}

#[test]
fn an_illegal_answer_leaves_the_state_and_every_counter_untouched() {
    let mut simulator = builder()
        .card(PileName::Hand, 1, fingerprint("CARD.ASK_THEN_BLOCK"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 3, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    simulator
        .step(play(&simulator, "CARD.ASK_THEN_BLOCK"))
        .unwrap();
    let before = simulator.state_key().unwrap();
    let rng_before = simulator.state().rng.to_serializable();

    let DecisionContext::ChooseCards {
        choice_id,
        candidates,
        ..
    } = simulator.decision().clone()
    else {
        panic!("a screen is up");
    };

    // An answer correlated to a choice that is not the one standing.
    let wrong_choice = Action::ChooseCards {
        choice_id: sts2_core::ChoiceId::from(choice_id.get() + 7),
        cards: vec![candidates[0].clone()],
    };
    assert_eq!(
        simulator.step(wrong_choice).unwrap_err().code,
        ErrorCode::IllegalAction
    );

    // An answer naming more cards than the screen asked for.
    let too_many = Action::ChooseCards {
        choice_id,
        cards: candidates.clone(),
    };
    assert_eq!(
        simulator.step(too_many).unwrap_err().code,
        ErrorCode::IllegalAction
    );

    // An answer naming a card the screen never offered.
    let mut stranger = candidates[0].clone();
    stranger.card_id = sts2_core::CardInstanceId::from(999);
    let unoffered = Action::ChooseCards {
        choice_id,
        cards: vec![stranger],
    };
    assert_eq!(
        simulator.step(unoffered).unwrap_err().code,
        ErrorCode::IllegalAction
    );

    assert_eq!(simulator.state_key().unwrap(), before);
    assert_eq!(simulator.state().rng.to_serializable(), rng_before);
    assert!(matches!(
        simulator.decision(),
        DecisionContext::ChooseCards { .. }
    ));
}

#[test]
fn a_screen_the_build_could_not_pause_for_is_refused_where_it_stands() {
    // A `Modify*` pass answers with a number on the spot and has nowhere to
    // pause, so a body that asks a question there is content being wrong.
    let mut modifying = builder()
        .power(0, id("POWER.MODIFY_ASKER"), 1, PowerType::Buff)
        .card(PileName::Hand, 1, fingerprint("CARD.PLAIN_ATTACK"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 3, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    let before = modifying.state_key().unwrap();
    let error = modifying
        .step(play(&modifying, "CARD.PLAIN_ATTACK"))
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::InternalInvariant);
    assert_eq!(
        error.details.get("suspension_blocked_by").unwrap(),
        "a modifier pass"
    );
    assert_eq!(modifying.state_key().unwrap(), before);

    // The same for a listener pass the build never pauses in, one with no way
    // to ask the player anything at all.
    let mut starting = builder()
        .power(0, id("POWER.SIDE_START_ASKER"), 1, PowerType::Buff)
        .card(PileName::Hand, 1, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    let before = starting.state_key().unwrap();
    let error = starting.step(Action::EndTurn { turn: 1 }).unwrap_err();
    assert_eq!(error.code, ErrorCode::InternalInvariant);
    assert_eq!(
        error.details.get("suspension_blocked_by").unwrap(),
        "a listener the build runs without stopping"
    );
    assert_eq!(starting.state_key().unwrap(), before);
}

/// The refusal is the pass's, not the boundary's: a card whose face cannot be
/// drawn draws none, and the card behind it in the same hand is drawn against a
/// blank pass rather than against the refused one's leavings.
#[test]
fn a_refused_face_leaves_the_next_card_of_the_hand_a_blank_pass() {
    let hand = |models: &[&str]| {
        let mut scenario = builder().power(0, id("POWER.MODIFY_ASKER"), 1, PowerType::Buff);
        for (slot, model) in models.iter().enumerate() {
            let slot = u64::try_from(slot).expect("small index") + 1;
            scenario = scenario.card(PileName::Hand, slot, fingerprint(model), 0);
        }
        scenario
            .build(registry())
            .expect("the scenario builds")
            .agent_observation()
            .hand_previews
    };

    // `CARD.CONTEXT_READER` reads the dealer of whatever pass is standing, so
    // on its own it draws its four and nothing more.
    let alone = hand(&["CARD.CONTEXT_READER"]);
    assert_eq!(alone[0].damage, Some(4));

    let poisoned = hand(&["CARD.FILLER", "CARD.PLAIN_ATTACK", "CARD.CONTEXT_READER"]);
    assert_eq!(
        poisoned[1],
        sts2_engine::CardPreview::blank(poisoned[1].card_id),
        "the attack the asking power refuses draws no number at all"
    );
    assert_eq!(
        poisoned[2].damage, alone[0].damage,
        "the card behind the refusal draws what it draws on its own"
    );
}

#[test]
fn a_pass_the_build_defers_banks_the_screen_and_runs_on() {
    // The turn-end pass hands each listener a question of its own, carries
    // each body only as far as the point where it pauses, and comes back for
    // the paused bodies after all three of its passes. A body that asks parks
    // its screen and the pass carries on — so a listener behind the paused one
    // acts while the question is still owed, and the screen opens when the
    // pass comes back. The blocker's block is the witness that the pass really
    // ran on.
    let mut ending = builder()
        .power(0, id("POWER.TURN_END_ASKER"), 1, PowerType::Buff)
        .power(0, id("POWER.TURN_END_BLOCKER"), 1, PowerType::Buff)
        .card(PileName::Hand, 1, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    ending.step(Action::EndTurn { turn: 1 }).unwrap();
    assert!(matches!(
        ending.decision(),
        DecisionContext::ChooseCards { .. }
    ));
    let combat = ending.state().combat.as_ref().unwrap();
    let player = combat
        .creatures
        .iter()
        .find(|creature| creature.id == combat.player.creature_id)
        .unwrap();
    assert_eq!(
        player.block, 3,
        "the listener behind the paused body acted before the screen was answered"
    );

    ending.step(pick_first(&ending)).unwrap();
    assert_eq!(
        ending.state().combat.as_ref().unwrap().player.turn,
        2,
        "the answer lets the rest of the turn end run out"
    );
}

#[test]
fn two_deferred_bodies_can_owe_screens_at_once() {
    // One asker on the site's first pass and one on its last: both bank, and
    // the join opens them in the order the bodies paused — the choice ids say
    // so, because the build reserves an id when the body asks, not when the
    // screen is finally shown.
    let mut ending = builder()
        .power(0, id("POWER.EARLY_END_ASKER"), 1, PowerType::Buff)
        .power(0, id("POWER.TURN_END_ASKER"), 1, PowerType::Buff)
        .card(PileName::Hand, 1, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    ending.step(Action::EndTurn { turn: 1 }).unwrap();
    let DecisionContext::ChooseCards { choice_id, .. } = ending.decision() else {
        panic!("the first banked screen is standing");
    };
    let first = choice_id.get();

    ending.step(pick_first(&ending)).unwrap();
    let DecisionContext::ChooseCards { choice_id, .. } = ending.decision() else {
        panic!("the second banked screen follows the first answer");
    };
    assert_eq!(
        choice_id.get(),
        first + 1,
        "the early pass reserved its id first, so its screen opened first"
    );

    ending.step(pick_first(&ending)).unwrap();
    assert_eq!(ending.state().combat.as_ref().unwrap().player.turn, 2);
}

#[test]
fn a_deferred_site_that_never_joins_still_stops() {
    // The pass at the start of a side's turn defers its bodies and never comes
    // back for them: a paused body is left waiting with nothing that will ever
    // join it, and a screen put up there stops the fight. That stop is a
    // boundary of the engine rather than content being wrong, and the two do
    // not share a code.
    let mut opening = builder()
        .power(0, id("POWER.SIDE_OPEN_ASKER"), 1, PowerType::Buff)
        .card(PileName::Hand, 1, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    let before = opening.state_key().unwrap();
    let error = opening.step(Action::EndTurn { turn: 1 }).unwrap_err();
    assert_eq!(error.code, ErrorCode::UnsupportedContent);
    assert_eq!(
        error.details.get("suspension_blocked_by").unwrap(),
        "a listener pass the build defers rather than stops"
    );
    assert_eq!(opening.state_key().unwrap(), before);
}

#[test]
fn a_screen_inside_the_player_turn_setup_lets_the_rest_of_the_turn_start_run() {
    // The player turn's setup is the one part of the turn start that is not
    // waited out: it is carried alongside, only as far as the point where it
    // pauses, so the listeners that run after a side's turn starts and the
    // rest of the turn start go on while the question is still standing. What
    // waits is the auto-pre-play phase, which does not open until the setup
    // has finished.
    let mut simulator = builder()
        .power(0, id("POWER.TURN_START_ASKER"), 1, PowerType::Buff)
        .card(PileName::Hand, 1, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    simulator.step(Action::EndTurn { turn: 1 }).unwrap();
    assert!(matches!(
        simulator.decision(),
        DecisionContext::ChooseCards { .. }
    ));
    let combat = simulator.state().combat.as_ref().unwrap();
    assert_eq!(combat.player.turn, 2, "the next player turn is standing");
    assert_eq!(
        combat.player.phase,
        sts2_engine::PlayerTurnPhase::Start,
        "and it is still in its setup, because the auto-pre-play phase waits"
    );

    simulator.step(pick_first(&simulator)).unwrap();
    assert!(matches!(
        simulator.decision(),
        DecisionContext::CombatPriority { .. }
    ));
    assert_eq!(
        simulator.state().combat.as_ref().unwrap().player.phase,
        sts2_engine::PlayerTurnPhase::Play,
        "answering finishes the setup and runs the auto-pre-play phase"
    );
}

/// The rewards grid lays out what a run of creations made, in the order the
/// calls were made. Each call blacklists only its own cards, so two calls over
/// one pool can hand the same card over twice; here the two are of different
/// rarities, so what the grid shows is the two rarities in order.
#[test]
fn a_rewards_grid_lays_out_what_the_creations_made() {
    let mut simulator = builder()
        .card(PileName::Hand, 1, fingerprint("CARD.ASK_GRID"), 0)
        .build(registry())
        .unwrap();
    simulator.step(play(&simulator, "CARD.ASK_GRID")).unwrap();
    let DecisionContext::ChooseCards {
        candidates,
        minimum,
        maximum,
        ..
    } = simulator.decision().clone()
    else {
        panic!("the grid is up");
    };
    assert_eq!((minimum, maximum), (0, 4));
    assert_eq!(candidates.len(), 4);
    let rarities: Vec<_> = candidates
        .iter()
        .map(|card| {
            simulator
                .content()
                .card(&card.fingerprint.model_id)
                .expect("the card is content")
                .rarity
        })
        .collect();
    assert_eq!(
        rarities,
        [
            CardRarity::Common,
            CardRarity::Common,
            CardRarity::Uncommon,
            CardRarity::Uncommon,
        ],
        "the calls are concatenated in the order the body makes them"
    );
    assert!(
        candidates[0].fingerprint.model_id != candidates[1].fingerprint.model_id,
        "one call never repeats a card"
    );
}

/// A bundle screen offers whole bundles: an answer names every card of one of
/// them, in the order the bundle holds them, and nothing else is legal.
#[test]
fn a_bundle_screen_is_answered_with_one_whole_bundle() {
    let mut simulator = builder()
        .card(PileName::Hand, 1, fingerprint("CARD.ASK_BUNDLE"), 0)
        .build(registry())
        .unwrap();
    simulator.step(play(&simulator, "CARD.ASK_BUNDLE")).unwrap();
    let DecisionContext::ChooseCards {
        candidates,
        bundles,
        ..
    } = simulator.decision().clone()
    else {
        panic!("the bundle screen is up");
    };
    assert_eq!(candidates.len(), 6, "two bundles of three");
    assert_eq!(bundles, vec![vec![0, 1, 2], vec![3, 4, 5]]);
    let offered: Vec<_> = simulator.legal_actions().to_vec();
    assert_eq!(offered.len(), 2, "one answer per bundle and no other");
    for (action, bundle) in offered.iter().zip(&bundles) {
        let Action::ChooseCards { cards, .. } = action else {
            panic!("a bundle is answered with its cards");
        };
        assert_eq!(cards.len(), bundle.len());
    }
    // Every card of every bundle is a different card.
    let mut models: Vec<_> = candidates
        .iter()
        .map(|card| card.fingerprint.model_id.clone())
        .collect();
    models.sort();
    models.dedup();
    assert_eq!(models.len(), 6);
}

#[test]
fn an_enchant_screen_naming_an_unregistered_enchantment_names_the_model() {
    let mut simulator = builder()
        .card(PileName::Hand, 1, fingerprint("CARD.ASK_ENCHANT"), 0)
        .build(registry())
        .unwrap();
    let error = simulator
        .step(play(&simulator, "CARD.ASK_ENCHANT"))
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::UnsupportedContent);
    assert_eq!(
        error.details.get("model_id").unwrap(),
        "ENCHANTMENT.TEST",
        "the screen is built now; what is missing is the enchantment it names"
    );
}

#[test]
fn a_command_naming_the_listening_power_where_none_is_bound_fails_loudly() {
    // A power reference that resolves to nothing is an error, not a silent
    // no-op: a silent no-op is exactly how a lost attribution across a screen
    // would hide.
    let mut simulator = builder()
        .card(PileName::Hand, 1, fingerprint("CARD.BAD_LISTENING"), 0)
        .build(registry())
        .unwrap();
    let before = simulator.state_key().unwrap();
    let error = simulator
        .step(play(&simulator, "CARD.BAD_LISTENING"))
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::InternalInvariant);
    assert_eq!(error.details.get("unbound").unwrap(), "the listening power");
    assert_eq!(simulator.state_key().unwrap(), before);
}
