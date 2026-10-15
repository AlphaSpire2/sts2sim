//! Every frame the pinned build can stop for a selection screen, stopped and
//! picked back up: the turn handover, an attack between its hits, a command
//! landing on several creatures, a monster move, and the cards still in hand as
//! the turn ends.
//!
//! Each test pins two things — what has *not* happened while the question is
//! standing, and that what is left runs in the build's own order once it has
//! been answered.

use std::sync::Arc;

use sts2_core::ModelId;
use sts2_engine::{
    Action, Amount, CardCmd, CardDefinition, CardFilter, CardFingerprint, CardRarity,
    CardSelectCmd, CardSelector, CardType, CombatSide, Command, ContentRegistry, CreatureCmd,
    CreatureSelector, DamageCmd, DecisionContext, ErrorCode, HookCondition, HookEvent, Intent,
    ModifierHook, MonsterDefinition, MonsterMove, MonsterState, MoveIntent, PileName,
    PlayerTurnPhase, PowerCmd, PowerDefinition, PowerInstanceType, PowerRef, PowerStackType,
    PowerType, RelicCmd, RelicDefinition, RelicRarity, ScenarioBuilder, SelectorPrefs, Simulator,
    TargetType, ValueProps,
};

const SEED: &str = "RESUMETEST";

fn id(value: &str) -> ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str, effects: Vec<Command>) -> CardDefinition {
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

fn block(amount: i32) -> Command {
    Command::Creature(CreatureCmd::GainBlock {
        target: CreatureSelector::Owner,
        amount: Amount::Fixed(amount),
        props: ValueProps::UNPOWERED,
    })
}

/// A screen over the hand asking for exactly one card, which comes up whenever
/// the hand holds more than one.
fn hand_screen(then: Vec<Command>) -> Command {
    Command::CardSelect(CardSelectCmd::FromHand {
        semantics: sts2_engine::ChoiceSemantics::default(),
        maximum: None,
        exact_count: None,
        prefs: SelectorPrefs::exactly(1),
        filter: CardFilter::Any,
        then,
    })
}

/// The same over the discard pile, for the frames that run once the hand is
/// down.
fn discard_screen(then: Vec<Command>) -> Command {
    Command::CardSelect(CardSelectCmd::FromCombatPile {
        semantics: sts2_engine::ChoiceSemantics::default(),
        exact_count: None,
        pile: PileName::Discard,
        prefs: SelectorPrefs::exactly(1),
        maximum: None,
        restricted_to_chosen: false,
        filter: CardFilter::Any,
        then,
    })
}

/// Takes the listening power off its owner, so a body that asks once asks once.
fn stop_asking() -> Command {
    Command::Power(PowerCmd::Remove {
        power: PowerRef::listening(),
    })
}

fn asker(model_id: &str, event: HookEvent, screen: Command) -> PowerDefinition {
    power(
        model_id,
        vec![ModifierHook::every_turn(event, vec![screen])],
    )
}

fn cards() -> Vec<CardDefinition> {
    vec![
        card("CARD.FILLER", Vec::new()),
        // Two hits of four, so a screen between them has a hit left to make.
        CardDefinition {
            type_from_property: None,
            card_type: CardType::Attack,
            target: TargetType::AnyEnemy,
            target_widens_with: None,
            ..card(
                "CARD.TWO_HITS",
                vec![Command::Damage(DamageCmd::Attack {
                    amount: Amount::Fixed(4),
                    hits: Amount::Fixed(2),
                    target: CreatureSelector::Target,
                    props: ValueProps::NONE,
                })],
            )
        },
        // One command landing on every enemy at once.
        card(
            "CARD.HIT_EVERYONE",
            vec![Command::Creature(CreatureCmd::Damage {
                target: CreatureSelector::AllOpponents,
                amount: Amount::Fixed(5),
                props: ValueProps::UNPOWERED,
            })],
        ),
        // A card that acts on whoever is still holding it at the end of the
        // turn, and asks a question while doing it.
        CardDefinition {
            end_of_turn_in_hand: vec![
                hand_screen(vec![Command::Card(CardCmd::Exhaust {
                    cards: CardSelector::Chosen,
                })]),
                block(4),
            ],
            ..card("CARD.ASKS_WHEN_HELD", Vec::new())
        },
    ]
}

fn powers() -> Vec<PowerDefinition> {
    vec![
        // The end-of-turn listeners: the auto-post-play phase is the first thing
        // the handover runs, and it is one of the passes that is allowed to ask
        // the player something.
        asker(
            "POWER.POST_PLAY_ASKER",
            HookEvent::AfterAutoPostPlayPhaseEntered,
            hand_screen(vec![block(7)]),
        ),
        // The flush listeners run inside phase two, once the hand is down.
        asker(
            "POWER.FLUSH_ASKER",
            HookEvent::AfterFlush,
            discard_screen(vec![block(9)]),
        ),
        // Every settled instance of damage is offered to this one, and it takes
        // itself off once it has asked so that the next one gets through.
        power(
            "POWER.DAMAGE_ASKER",
            vec![ModifierHook::every_turn(
                HookEvent::AfterDamageGiven,
                vec![hand_screen(vec![stop_asking()])],
            )],
        ),
        // A power that only answers the very first turn's start, which is where
        // a relic like Gambling Chip lives.
        asker(
            "POWER.TURN_START_ASKER",
            HookEvent::AfterPlayerTurnStart,
            hand_screen(vec![block(3)]),
        ),
    ]
}

/// Counts every settled instance of damage, so that a test can say how far the
/// pass behind the asker got.
fn damage_counter() -> RelicDefinition {
    RelicDefinition {
        model_id: id("RELIC.DAMAGE_COUNTER"),
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
                HookEvent::AfterDamageGiven,
                vec![Command::Relic(RelicCmd::ModifyListeningCounter(
                    Amount::Fixed(1),
                ))],
            )
            .when(HookCondition::Always),
        ],
    }
}

/// A monster that stands still, and one whose move asks a question halfway
/// through its own body.
fn monsters() -> Vec<MonsterDefinition> {
    vec![
        MonsterDefinition {
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
        },
        MonsterDefinition {
            model_id: id("MONSTER.ASKER_KILLER"),
            min_initial_hp: 40,
            max_initial_hp: 40,
            initial_state: "ASK".to_owned(),
            states: vec![MonsterState::Move(MonsterMove {
                state_id: "ASK".to_owned(),
                intent: Intent::new("ASK", vec![MoveIntent::of("unknown")]),
                commands: vec![
                    discard_screen(Vec::new()),
                    Command::Creature(CreatureCmd::Damage {
                        target: CreatureSelector::Target,
                        amount: Amount::Fixed(999),
                        props: ValueProps::UNBLOCKABLE,
                    }),
                ],
                follow_up: String::new(),
            })],
            on_added: Vec::new(),
            hooks: Vec::new(),
        },
        MonsterDefinition {
            model_id: id("MONSTER.ASKER"),
            min_initial_hp: 40,
            max_initial_hp: 40,
            initial_state: "ASK".to_owned(),
            states: vec![MonsterState::Move(MonsterMove {
                state_id: "ASK".to_owned(),
                intent: Intent::new("ASK", vec![MoveIntent::of("unknown")]),
                // An enemy move that asks the player something opens a screen
                // that blocks, so the rest of the move — and the rest of the
                // enemy turn — waits behind it.
                commands: vec![discard_screen(Vec::new()), block(6)],
                follow_up: String::new(),
            })],
            on_added: Vec::new(),
            hooks: Vec::new(),
        },
    ]
}

fn registry() -> Arc<ContentRegistry> {
    let mut registry = ContentRegistry::default();
    for definition in cards() {
        registry.add_card(definition);
    }
    for definition in powers() {
        registry.add_power(definition);
    }
    for definition in monsters() {
        registry.add_monster(definition);
    }
    registry.add_relic(damage_counter());
    Arc::new(registry)
}

fn fingerprint(model: &str) -> CardFingerprint {
    CardFingerprint::base(id(model))
}

fn builder() -> ScenarioBuilder {
    ScenarioBuilder::new(SEED, id("ENCOUNTER.TEST")).player_hp(60, 80)
}

fn dummy(builder: ScenarioBuilder) -> ScenarioBuilder {
    builder.enemy(1, id("MONSTER.DUMMY"), 40, 40, "WAIT")
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

fn pick_first(simulator: &Simulator) -> Action {
    pick_nth(simulator, 0)
}

/// The answer naming the `index`-th card the screen offered.
fn pick_nth(simulator: &Simulator, index: usize) -> Action {
    simulator
        .legal_actions()
        .iter()
        .filter(|action| matches!(action, Action::ChooseCards { cards, .. } if cards.len() == 1))
        .nth(index)
        .expect("the screen offers that many cards")
        .clone()
}

fn choosing(simulator: &Simulator) -> bool {
    matches!(simulator.decision(), DecisionContext::ChooseCards { .. })
}

fn block_of(simulator: &Simulator, creature: u64) -> i32 {
    simulator.state().combat.as_ref().unwrap().creatures[creature as usize].block
}

fn hp_of(simulator: &Simulator, creature: u64) -> i32 {
    simulator.state().combat.as_ref().unwrap().creatures[creature as usize].current_hp
}

fn relic_counter(simulator: &Simulator) -> i32 {
    simulator.state().run_player.relics[0].counter
}

/// Everything a suspended boundary has to be: only the choice is on offer, an
/// answer to something else is refused, and neither the state nor a single RNG
/// counter moves for having tried.
fn assert_only_the_choice_is_offered(simulator: &mut Simulator) {
    assert!(choosing(simulator), "a screen is up");
    assert!(
        simulator
            .legal_actions()
            .iter()
            .all(|action| matches!(action, Action::ChooseCards { .. })),
        "and nothing but answers to it is legal"
    );
    let before = simulator.state_key().unwrap();
    let rng_before = simulator.state().rng.to_serializable();
    let error = simulator.step(Action::EndTurn { turn: 1 }).unwrap_err();
    assert_eq!(error.code, ErrorCode::WrongDecisionContext);
    assert_eq!(simulator.state_key().unwrap(), before);
    assert_eq!(simulator.state().rng.to_serializable(), rng_before);
}

/// A simulator cloned at a suspended boundary and one stepped from the original
/// must walk to the same place, action for action.
fn assert_clone_resumes_identically(simulator: &Simulator, answers: usize) {
    let mut original = simulator.clone();
    let mut copy = simulator.clone();
    for _ in 0..answers {
        assert_eq!(
            original.state_key().unwrap(),
            copy.state_key().unwrap(),
            "the two copies are the same state"
        );
        let action = original.legal_actions()[0].clone();
        assert_eq!(
            copy.legal_actions(),
            original.legal_actions(),
            "and offer the same actions in the same order"
        );
        original.step(action.clone()).unwrap();
        copy.step(action).unwrap();
    }
    assert_eq!(
        original.state_key().unwrap(),
        copy.state_key().unwrap(),
        "and settle on the same state"
    );
}

#[test]
fn a_screen_in_the_end_of_turn_listeners_holds_the_whole_handover() {
    let mut simulator = dummy(builder())
        .power(0, id("POWER.POST_PLAY_ASKER"), 1, PowerType::Buff)
        .card(PileName::Hand, 1, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 3, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    simulator.step(Action::EndTurn { turn: 1 }).unwrap();

    let combat = simulator.state().combat.as_ref().unwrap();
    assert!(choosing(&simulator));
    assert_eq!(
        combat.player.phase,
        PlayerTurnPhase::AutoPostPlay,
        "the phase only moves on once the auto-post-play choice is answered"
    );
    assert!(combat.player_actions_disabled);
    assert_eq!(combat.player.piles.hand.len(), 3, "the hand is still up");
    assert_eq!(combat.player.turn, 1, "and the turn has not handed over");
    assert_eq!(block_of(&simulator, 0), 0);

    assert_clone_resumes_identically(&simulator, 1);
    simulator.step(pick_first(&simulator)).unwrap();

    assert!(matches!(
        simulator.decision(),
        DecisionContext::CombatPriority { .. }
    ));
    assert_eq!(
        block_of(&simulator, 0),
        0,
        "the block the body gained is gone with the turn it was gained on"
    );
    let combat = simulator.state().combat.as_ref().unwrap();
    assert_eq!(combat.player.turn, 2, "and the whole handover ran");
    assert_eq!(combat.player.phase, PlayerTurnPhase::Play);
}

#[test]
fn a_screen_in_the_flush_listeners_stops_the_handover_after_the_hand_goes_down() {
    // The flush listeners run inside phase two of the handover, after the hand
    // has gone down and before the sides switch.
    let mut simulator = dummy(builder())
        .power(0, id("POWER.FLUSH_ASKER"), 1, PowerType::Buff)
        .card(PileName::Hand, 1, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 3, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    simulator.step(Action::EndTurn { turn: 1 }).unwrap();

    let combat = simulator.state().combat.as_ref().unwrap();
    assert!(choosing(&simulator));
    assert!(combat.player.piles.hand.is_empty(), "the hand went down");
    assert_eq!(combat.player.piles.discard.len(), 3);
    assert_eq!(
        combat.side,
        CombatSide::Player,
        "the sides have not switched"
    );

    assert_only_the_choice_is_offered(&mut simulator);
    simulator.step(pick_first(&simulator)).unwrap();

    let combat = simulator.state().combat.as_ref().unwrap();
    assert_eq!(combat.player.turn, 2);
    assert_eq!(
        block_of(&simulator, 0),
        0,
        "block gained on the turn that ended does not survive the next turn's start"
    );
}

#[test]
fn a_screen_between_two_hits_resumes_at_the_hit_that_had_not_landed() {
    let mut simulator = dummy(builder())
        .power(0, id("POWER.DAMAGE_ASKER"), 1, PowerType::Buff)
        .relics(&[id("RELIC.DAMAGE_COUNTER")])
        .card(PileName::Hand, 1, fingerprint("CARD.TWO_HITS"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 3, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    simulator.step(play(&simulator, "CARD.TWO_HITS")).unwrap();

    assert!(choosing(&simulator));
    assert_eq!(hp_of(&simulator, 1), 36, "only the first hit has landed");
    assert_eq!(
        relic_counter(&simulator),
        0,
        "and the pass behind the asker has not been reached"
    );

    assert_clone_resumes_identically(&simulator, 1);
    simulator.step(pick_first(&simulator)).unwrap();

    assert!(matches!(
        simulator.decision(),
        DecisionContext::CombatPriority { .. }
    ));
    assert_eq!(hp_of(&simulator, 1), 32, "the second hit landed after it");
    assert_eq!(
        relic_counter(&simulator),
        2,
        "and both instances of damage reached the counter behind the asker"
    );
}

#[test]
fn a_screen_inside_a_multi_target_command_resumes_at_the_target_it_had_not_told() {
    // A blow against several targets lands in two passes: every target takes
    // its damage first, and only then is each settled result handed to the
    // listeners. So the second enemy is already hurt while the first one's
    // `AfterDamageGiven` listeners are still asking.
    let mut simulator = builder()
        .enemy(1, id("MONSTER.DUMMY"), 40, 40, "WAIT")
        .enemy(2, id("MONSTER.DUMMY"), 40, 40, "WAIT")
        .power(0, id("POWER.DAMAGE_ASKER"), 1, PowerType::Buff)
        .relics(&[id("RELIC.DAMAGE_COUNTER")])
        .card(PileName::Hand, 1, fingerprint("CARD.HIT_EVERYONE"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 3, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    simulator
        .step(play(&simulator, "CARD.HIT_EVERYONE"))
        .unwrap();

    assert!(choosing(&simulator));
    assert_eq!(hp_of(&simulator, 1), 35);
    assert_eq!(
        hp_of(&simulator, 2),
        35,
        "both took the damage before either was announced"
    );
    assert_eq!(relic_counter(&simulator), 0);

    assert_only_the_choice_is_offered(&mut simulator);
    assert_clone_resumes_identically(&simulator, 1);
    simulator.step(pick_first(&simulator)).unwrap();

    assert_eq!(
        relic_counter(&simulator),
        2,
        "the second result was announced after the answer"
    );
}

#[test]
fn a_splash_attack_resumes_either_damage_call_before_announcing_the_attack() {
    for asking_target in [1, 2] {
        let mut content = (*registry()).clone();
        content.add_card(CardDefinition {
            card_type: CardType::Attack,
            target: TargetType::AnyEnemy,
            ..card(
                "CARD.SPLASH_ATTACK",
                vec![Command::Damage(DamageCmd::AttackWithSplash {
                    amount: Amount::Fixed(4),
                })],
            )
        });
        content.add_power(power(
            "POWER.SPLASH_ASKER",
            vec![
                ModifierHook::every_turn(
                    HookEvent::AfterDamageReceived,
                    vec![hand_screen(vec![stop_asking()])],
                )
                .when(HookCondition::OwnerTookUnblockedDamage),
            ],
        ));
        content.add_power(asker(
            "POWER.ATTACK_COUNTER",
            HookEvent::AfterAttack,
            block(3),
        ));
        let mut simulator = builder()
            .enemy(1, id("MONSTER.DUMMY"), 40, 40, "WAIT")
            .enemy(2, id("MONSTER.DUMMY"), 40, 40, "WAIT")
            .power(asking_target, id("POWER.SPLASH_ASKER"), 1, PowerType::Buff)
            .power(0, id("POWER.ATTACK_COUNTER"), 1, PowerType::Buff)
            .relics(&[id("RELIC.DAMAGE_COUNTER")])
            .card(PileName::Hand, 1, fingerprint("CARD.SPLASH_ATTACK"), 0)
            .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
            .card(PileName::Hand, 3, fingerprint("CARD.FILLER"), 0)
            .build(Arc::new(content))
            .unwrap();
        simulator
            .step(play(&simulator, "CARD.SPLASH_ATTACK"))
            .unwrap();

        assert!(choosing(&simulator));
        assert_eq!(hp_of(&simulator, 1), 36);
        assert_eq!(
            hp_of(&simulator, 2),
            if asking_target == 1 { 40 } else { 36 }
        );
        assert_eq!(
            block_of(&simulator, 0),
            0,
            "AfterAttack waits for both damage calls"
        );
        assert_clone_resumes_identically(&simulator, 1);
        simulator.step(pick_first(&simulator)).unwrap();
        assert!(!choosing(&simulator));
        assert_eq!(hp_of(&simulator, 1), 36);
        assert_eq!(hp_of(&simulator, 2), 36);
        assert_eq!(
            relic_counter(&simulator),
            2,
            "neither damage call is repeated"
        );
        assert_eq!(block_of(&simulator, 0), 3, "one attack is announced");
    }
}

#[test]
fn a_screen_inside_a_monster_move_resumes_mid_move() {
    let mut simulator = builder()
        .enemy(1, id("MONSTER.ASKER"), 40, 40, "ASK")
        .card(PileName::Hand, 1, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 3, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    simulator.step(Action::EndTurn { turn: 1 }).unwrap();

    assert!(choosing(&simulator));
    let combat = simulator.state().combat.as_ref().unwrap();
    assert_eq!(combat.side, CombatSide::Enemy, "the enemy turn is running");
    assert_eq!(
        block_of(&simulator, 1),
        0,
        "the command after the screen has not run"
    );
    assert!(
        !combat.creatures[1].performed_first_move,
        "and the move is not settled while it is still being made"
    );

    assert_only_the_choice_is_offered(&mut simulator);
    assert_clone_resumes_identically(&simulator, 1);
    simulator.step(pick_first(&simulator)).unwrap();

    assert_eq!(
        block_of(&simulator, 1),
        6,
        "the rest of the move ran after the answer"
    );
    let combat = simulator.state().combat.as_ref().unwrap();
    assert!(combat.creatures[1].performed_first_move);
    assert_eq!(combat.player.turn, 2, "and the handover carried on");
    assert!(matches!(
        simulator.decision(),
        DecisionContext::CombatPriority { .. }
    ));
}

#[test]
fn a_screen_inside_a_held_card_resumes_before_the_card_is_put_down() {
    let mut simulator = dummy(builder())
        .card(PileName::Hand, 1, fingerprint("CARD.ASKS_WHEN_HELD"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 3, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    simulator.step(Action::EndTurn { turn: 1 }).unwrap();

    assert!(choosing(&simulator));
    let combat = simulator.state().combat.as_ref().unwrap();
    // A card that acts from the hand at the end of the turn goes to the play
    // pile first, so the body asking its question is asking it from there: the
    // hand it is asking about is the rest of the hand.
    assert_eq!(
        combat.player.piles.play.len(),
        1,
        "the card acting is in the play pile while it acts"
    );
    assert_eq!(
        combat.player.piles.play[0].fingerprint.model_id,
        id("CARD.ASKS_WHEN_HELD")
    );
    assert_eq!(
        combat.player.piles.hand.len(),
        2,
        "and is out of the hand before its own body runs"
    );
    assert_eq!(combat.player.phase, PlayerTurnPhase::End);
    assert_eq!(block_of(&simulator, 0), 0);

    assert_only_the_choice_is_offered(&mut simulator);
    assert_clone_resumes_identically(&simulator, 1);
    // Either of the two cards the hand still holds; the asking card is not one
    // of them, and its own body puts it down afterwards regardless.
    let second = pick_nth(&simulator, 1);
    simulator.step(second).unwrap();

    let combat = simulator.state().combat.as_ref().unwrap();
    assert_eq!(
        combat.player.piles.exhaust.len(),
        1,
        "what was picked was exhausted by the statements after the screen"
    );
    assert!(
        combat
            .player
            .piles
            .exhaust
            .iter()
            .all(|card| card.fingerprint.model_id != id("CARD.ASKS_WHEN_HELD")),
        "and the card that asked was put down rather than burned, because it is \
         not ethereal"
    );
    assert_eq!(combat.player.turn, 2, "and the handover finished");
}

#[test]
fn a_screen_in_the_turn_setup_survives_a_snapshot_taken_while_it_is_up() {
    // The setup is the frame the build defers rather than waits out, so this is
    // the one that has to survive being put down mid-flight and picked back up
    // from a copy.
    let mut simulator = dummy(builder())
        .power(0, id("POWER.TURN_START_ASKER"), 1, PowerType::Buff)
        .card(PileName::Hand, 1, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 3, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    simulator.step(Action::EndTurn { turn: 1 }).unwrap();
    assert!(choosing(&simulator));

    // The same state carried out through the portable boundary and back has to
    // resume identically too, because the frame lives in the state rather than
    // on a Rust call stack.
    let visible_before = simulator.agent_observation();
    let DecisionContext::ChooseCards { semantics, .. } = &visible_before.decision else {
        panic!("active choice")
    };
    assert_eq!(semantics.coverage, sts2_engine::ChoiceCoverage::Unknown);
    assert_eq!(semantics.origin, sts2_engine::ChoiceIdentity::CombatCard);
    let snapshot = simulator.snapshot();
    let mut restored = simulator.clone();
    restored.restore(&snapshot).unwrap();
    assert_eq!(
        restored.agent_observation().decision,
        visible_before.decision
    );
    assert_eq!(
        restored.state_key().unwrap(),
        simulator.state_key().unwrap()
    );

    assert_clone_resumes_identically(&simulator, 2);
    simulator.step(pick_first(&simulator)).unwrap();
    restored.step(pick_first(&restored)).unwrap();
    assert_eq!(
        restored.state_key().unwrap(),
        simulator.state_key().unwrap()
    );
    assert_eq!(
        block_of(&simulator, 0),
        3,
        "the statements after the screen ran once the player answered"
    );
}

#[test]
fn a_frame_put_down_by_one_screen_can_be_put_down_again_by_the_next() {
    // Two askers on the same turn start: the first stops the pass, and the
    // second stops the same pass again once it has been picked back up.
    let mut simulator = dummy(builder())
        .power(0, id("POWER.TURN_START_ASKER"), 1, PowerType::Buff)
        .power(0, id("POWER.POST_PLAY_ASKER"), 1, PowerType::Buff)
        .card(PileName::Hand, 1, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 3, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    // The auto-post-play asker goes first, inside phase one.
    simulator.step(Action::EndTurn { turn: 1 }).unwrap();
    assert!(choosing(&simulator));
    assert_eq!(
        simulator.state().combat.as_ref().unwrap().player.phase,
        PlayerTurnPhase::AutoPostPlay
    );
    simulator.step(pick_first(&simulator)).unwrap();

    // And the turn-start asker stops the handover again, on the far side of the
    // enemy turn.
    assert!(choosing(&simulator));
    assert_eq!(
        simulator.state().combat.as_ref().unwrap().player.turn,
        2,
        "the second screen belongs to the next turn's setup"
    );
    assert_clone_resumes_identically(&simulator, 1);
    simulator.step(pick_first(&simulator)).unwrap();
    assert!(matches!(
        simulator.decision(),
        DecisionContext::CombatPriority { .. }
    ));
}

#[test]
fn a_power_that_stops_asking_is_still_the_one_the_resumed_body_names() {
    // The screen's own commands resolve `PowerRef::listening` against the power
    // that opened it, which only works if the attribution survived the answer.
    let mut simulator = dummy(builder())
        .power(0, id("POWER.DAMAGE_ASKER"), 1, PowerType::Buff)
        .card(PileName::Hand, 1, fingerprint("CARD.TWO_HITS"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 3, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    simulator.step(play(&simulator, "CARD.TWO_HITS")).unwrap();
    simulator.step(pick_first(&simulator)).unwrap();
    assert!(
        simulator.state().combat.as_ref().unwrap().creatures[0]
            .powers
            .iter()
            .all(|power| power.model_id != id("POWER.DAMAGE_ASKER")),
        "the resumed body took the listening power off"
    );
    assert!(matches!(
        simulator.decision(),
        DecisionContext::CombatPriority { .. }
    ));
}

#[test]
fn a_resumed_move_that_kills_the_player_ends_the_run_where_it_stands() {
    // The enemy turn checks the win condition after every enemy, so a blow
    // landed by the half of a move that ran after the answer is caught there
    // rather than being carried past.
    let mut simulator = builder()
        .player_hp(6, 80)
        .enemy(1, id("MONSTER.ASKER_KILLER"), 40, 40, "ASK")
        .card(PileName::Hand, 1, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 3, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    simulator.step(Action::EndTurn { turn: 1 }).unwrap();
    assert!(choosing(&simulator));
    assert_eq!(hp_of(&simulator, 0), 6, "the blow has not landed yet");

    simulator.step(pick_first(&simulator)).unwrap();
    assert!(matches!(
        simulator.decision(),
        DecisionContext::Terminal {
            result: sts2_engine::RunResult::Defeat
        }
    ));
    assert!(simulator.state().combat.is_none());
    assert_eq!(simulator.state().run_player.current_hp, 0);
    assert_eq!(simulator.legal_actions(), []);
}
