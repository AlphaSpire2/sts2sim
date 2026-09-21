use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::json;
use sts2_core::{CardInstanceId, CreatureCombatId, ModelId};
use sts2_engine::{
    Action, Amount, CardDefinition, CardFilter, CardFingerprint, CardRarity, CardSelectCmd,
    CardType, Command, CommandContext, ContentRegistry, Count, CreatureCmd, CreatureSelector,
    DamageCmd, DecisionContext, Effect, ErrorCode, HookCondition, HookEvent, HookTiming, Intent,
    ModifierCmd, ModifierHook, MonsterDefinition, MonsterMove, MonsterState, MoveBranch,
    MoveIntent, MoveRepeat, PileName, PowerCmd, PowerDefinition, PowerInstanceType, PowerStackType,
    PowerType, ProjectionScope, QueuedCommand, RelicDefinition, RelicRarity, ScenarioBuilder,
    SelectorPrefs, Simulator, TargetType, ValueProps,
};

/// A single-hit attack on whatever the play was aimed at.
fn attack(damage: i32) -> Command {
    Command::Damage(DamageCmd::Attack {
        amount: Amount::Fixed(damage),
        hits: Amount::Fixed(1),
        target: CreatureSelector::Target,
        props: ValueProps::NONE,
    })
}

fn block(amount: i32) -> Command {
    Command::Creature(CreatureCmd::GainBlock {
        target: CreatureSelector::Owner,
        amount: Amount::Fixed(amount),
        props: ValueProps::NONE,
    })
}

/// A monster's attack, aimed at everyone it is fighting.
fn monster_strike(damage: i32) -> Command {
    Command::Damage(DamageCmd::Attack {
        amount: Amount::Fixed(damage),
        hits: Amount::Fixed(1),
        target: CreatureSelector::AllOpponents,
        props: ValueProps::NONE,
    })
}

fn buff(power: &str, amount: i32) -> Command {
    Command::Power(PowerCmd::Apply {
        marks: None,
        power: id(power),
        target: CreatureSelector::Owner,
        applier: Some(CreatureSelector::Owner),
        amount: Amount::Fixed(amount),
    })
}

fn id(value: &str) -> ModelId {
    value.parse().unwrap()
}

#[allow(
    clippy::too_many_lines,
    reason = "one block per invented model this substrate test needs"
)]
fn registry() -> Arc<ContentRegistry> {
    let mut registry = ContentRegistry::default();
    registry.add_card(CardDefinition {
        model_id: id("CARD.STRIKE_IRONCLAD"),
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
        card_type: CardType::Attack,
        target: TargetType::AnyEnemy,
        target_widens_with: None,
        energy_cost: 1,
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
        effects: vec![attack(6)],
        transform_pool: None,
        visual_pool: None,
        adds_rest_site_option: None,
        hooks: Vec::new(),
    });
    registry.add_card(CardDefinition {
        model_id: id("CARD.DEFEND_IRONCLAD"),
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
        energy_cost: 1,
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
        effects: vec![block(5)],
        transform_pool: None,
        visual_pool: None,
        adds_rest_site_option: None,
        hooks: Vec::new(),
    });
    registry.add_card(CardDefinition {
        model_id: id("CARD.TEST_CHOOSE"),
        rarity: CardRarity::Basic,
        ethereal: false,
        eternal: false,
        unplayable: false,
        end_of_turn_in_hand: Vec::new(),
        tags: Vec::new(),
        upgrade: None,
        supported_upgrade_levels: vec![0],
        allow_custom_properties: true,
        gains_block: false,
        saved_properties: Vec::new(),
        quest_hooks: None,
        map_hooks: None,
        type_from_property: None,
        card_type: CardType::Attack,
        target: TargetType::AnyEnemy,
        target_widens_with: None,
        energy_cost: 1,
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
        // A hand selection, and behind it a hit worth four for each card
        // picked: the shape of a card that asks before it lands.
        effects: vec![Command::CardSelect(CardSelectCmd::FromHand {
            maximum: None,
            exact_count: None,
            prefs: SelectorPrefs::exactly(1).confirmed(),
            filter: CardFilter::Any,
            then: vec![Command::Creature(CreatureCmd::Damage {
                target: CreatureSelector::Target,
                amount: Amount::per(0, 4, Count::ChosenCards),
                props: ValueProps::UNPOWERED,
            })],
        })],
        transform_pool: None,
        visual_pool: None,
        adds_rest_site_option: None,
        hooks: Vec::new(),
    });
    registry.add_card(CardDefinition {
        model_id: id("CARD.TEST_EXHAUST"),
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
        exhausts: true,
        play_restriction: None,
        playable_when: None,
        damage_var: None,
        effects: vec![block(2)],
        transform_pool: None,
        visual_pool: None,
        adds_rest_site_option: None,
        hooks: Vec::new(),
    });
    registry.add_monster(MonsterDefinition {
        model_id: id("MONSTER.TEST_DUMMY"),
        min_initial_hp: 20,
        max_initial_hp: 20,
        initial_state: "ATTACK".to_owned(),
        on_added: Vec::new(),
        hooks: Vec::new(),
        states: vec![MonsterState::Move(MonsterMove {
            state_id: "ATTACK".to_owned(),
            intent: Intent::new("ATTACK", vec![MoveIntent::attack(3, 1)]),
            commands: vec![monster_strike(3)],
            follow_up: "ATTACK".to_owned(),
        })],
    });
    Arc::new(registry)
}

#[test]
fn block_energy_discard_and_exhaust_use_real_pile_transitions() {
    let mut simulator = ScenarioBuilder::new("PILESTEST", id("ENCOUNTER.TEST"))
        .card(PileName::Hand, 1, fingerprint("CARD.DEFEND_IRONCLAD"), 1)
        .card(PileName::Hand, 2, fingerprint("CARD.TEST_EXHAUST"), 0)
        .enemy(1, id("MONSTER.TEST_DUMMY"), 20, 20, "ATTACK")
        .build(registry())
        .unwrap();
    simulator
        .step(play(&simulator, "CARD.DEFEND_IRONCLAD"))
        .unwrap();
    simulator
        .step(play(&simulator, "CARD.TEST_EXHAUST"))
        .unwrap();
    let combat = simulator.state().combat.as_ref().unwrap();
    assert_eq!(combat.player.energy, 2);
    assert_eq!(combat.creatures[0].block, 7);
    assert_eq!(combat.player.piles.discard[0].id, CardInstanceId::from(1));
    assert_eq!(combat.player.piles.exhaust[0].id, CardInstanceId::from(2));
    assert!(combat.player.piles.play.is_empty());
}

fn fingerprint(model: &str) -> CardFingerprint {
    CardFingerprint::base(id(model))
}

fn scenario() -> Simulator {
    ScenarioBuilder::new("PHASE2TEST", id("ENCOUNTER.TEST"))
        .card(PileName::Hand, 1, fingerprint("CARD.TEST_CHOOSE"), 1)
        .card(PileName::Hand, 2, fingerprint("CARD.DEFEND_IRONCLAD"), 1)
        .card(PileName::Draw, 3, fingerprint("CARD.STRIKE_IRONCLAD"), 1)
        .card(PileName::Draw, 4, fingerprint("CARD.DEFEND_IRONCLAD"), 1)
        .enemy(1, id("MONSTER.TEST_DUMMY"), 20, 20, "ATTACK")
        .build(registry())
        .unwrap()
}

fn play(simulator: &Simulator, model: &str) -> Action {
    simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::PlayCard { card, .. } if card.fingerprint.model_id == id(model))
        })
        .unwrap()
        .clone()
}

#[test]
fn effects_pause_for_nested_choice_then_resume_once() {
    let mut simulator = scenario();
    let transition = simulator
        .step(play(&simulator, "CARD.TEST_CHOOSE"))
        .unwrap();
    assert!(matches!(
        transition.next_decision,
        DecisionContext::ChooseCards { .. }
    ));
    // The suspended damage, and behind it the one pass over the listeners on
    // the card having been played, the move into its result pile, and the
    // empty-hand check the play finishes with.
    assert_eq!(simulator.state().combat.as_ref().unwrap().queue.len(), 3);

    let choice = simulator.legal_actions()[0].clone();
    simulator.step(choice.clone()).unwrap();
    let combat = simulator.state().combat.as_ref().unwrap();
    assert_eq!(combat.creatures[1].current_hp, 16);
    assert!(combat.player.piles.play.is_empty());
    assert_eq!(combat.player.piles.discard[0].id, CardInstanceId::from(1));
    assert!(matches!(
        simulator.decision(),
        DecisionContext::CombatPriority { .. }
    ));

    let before = simulator.state_key().unwrap();
    let error = simulator.step(choice).unwrap_err();
    assert_eq!(error.code, ErrorCode::WrongDecisionContext);
    assert_eq!(simulator.state_key().unwrap(), before);
}

#[test]
fn hook_dispatch_has_stable_add_remove_semantics() {
    // A pass walks a list snapshotted before it began, checking each entry for
    // liveness as it comes up. The build checks a power by its owner rather
    // than by itself, so taking the power off a creature that is still in the
    // fight does not stop it answering for the rest of the pass.
    let removed_first = listener_scenario(&[("POWER.TEST_REMOVER", 1), ("POWER.TEST_HEALER", 1)]);
    assert_eq!(
        healed(removed_first),
        5,
        "a power removed mid-pass still answers for the rest of it"
    );

    let healed_first = listener_scenario(&[("POWER.TEST_HEALER", 1), ("POWER.TEST_REMOVER", 1)]);
    assert_eq!(
        healed(healed_first),
        5,
        "a listener the pass already reached acts"
    );

    let mut added = listener_scenario(&[("POWER.TEST_ADDER", 1)]);
    let before = added.state().run_player.current_hp;
    added.step(play(&added, "CARD.DEFEND_IRONCLAD")).unwrap();
    assert_eq!(
        added.state().run_player.current_hp,
        before,
        "a listener added during a pass does not act in it"
    );
    added.step(play(&added, "CARD.DEFEND_IRONCLAD")).unwrap();
    assert_eq!(
        added.state().run_player.current_hp,
        before + 5,
        "and does act in the next one"
    );
}

#[test]
fn a_listener_whose_owner_left_the_combat_stops_mid_pass() {
    // What does stop a power is its owner leaving: the build's guard is the
    // owner's combat state, and a creature killed earlier in the same pass has
    // been taken out of the combat. The player's own listener runs first and
    // kills the biter; a second enemy keeps the fight open.
    let killed = biting_scenario(true);
    assert_eq!(killed, 0, "a power whose owner died mid-pass does not act");

    let spared = biting_scenario(false);
    assert_eq!(spared, 3, "and acts while its owner is still standing");
}

/// A combat where the first enemy bites back when a card is played, optionally
/// with a player-side listener ahead of it that kills the biter first. Returns
/// how much HP the player lost playing one card.
fn biting_scenario(slay: bool) -> i32 {
    let mut builder = ScenarioBuilder::new("HOOKTEST", id("ENCOUNTER.TEST"))
        .player_hp(50, 80)
        .card(PileName::Hand, 1, fingerprint("CARD.DEFEND_IRONCLAD"), 1)
        .enemy(1, id("MONSTER.TEST_DUMMY"), 20, 20, "ATTACK")
        // Far too healthy to be killed with the biter, so the fight stays open.
        .enemy(2, id("MONSTER.TEST_DUMMY"), 500, 500, "ATTACK");
    if slay {
        builder = builder.power(0, id("POWER.TEST_SLAYER"), 1, PowerType::Buff);
    }
    let mut simulator = builder
        .power(1, id("POWER.TEST_BITER"), 1, PowerType::Buff)
        .build(listener_registry())
        .unwrap();
    let before = simulator.state().combat.as_ref().unwrap().creatures[0].current_hp;
    simulator
        .step(play(&simulator, "CARD.DEFEND_IRONCLAD"))
        .unwrap();
    before - simulator.state().combat.as_ref().unwrap().creatures[0].current_hp
}

/// A combat where the player carries the listed powers, in the listed order,
/// and can play a card to dispatch to them.
fn listener_scenario(powers: &[(&str, i32)]) -> Simulator {
    let mut builder = ScenarioBuilder::new("HOOKTEST", id("ENCOUNTER.TEST"))
        .player_hp(50, 80)
        .card(PileName::Hand, 1, fingerprint("CARD.DEFEND_IRONCLAD"), 1)
        .card(PileName::Hand, 2, fingerprint("CARD.DEFEND_IRONCLAD"), 1)
        .enemy(1, id("MONSTER.TEST_DUMMY"), 20, 20, "ATTACK");
    for (model_id, amount) in powers {
        builder = builder.power(0, id(model_id), *amount, PowerType::Buff);
    }
    builder.build(listener_registry()).unwrap()
}

fn healed(mut simulator: Simulator) -> i32 {
    let before = simulator.state().run_player.current_hp;
    simulator
        .step(play(&simulator, "CARD.DEFEND_IRONCLAD"))
        .unwrap();
    simulator.state().run_player.current_hp - before
}

fn listener_registry() -> Arc<ContentRegistry> {
    let mut content = ContentRegistry::default();
    let base = registry();
    for model_id in ["CARD.STRIKE_IRONCLAD", "CARD.DEFEND_IRONCLAD"] {
        content.add_card(base.card(&id(model_id)).unwrap().clone());
    }
    content.add_monster(base.monster(&id("MONSTER.TEST_DUMMY")).unwrap().clone());
    content.add_power(test_power(
        "POWER.TEST_HEALER",
        vec![ModifierHook::every_turn(
            HookEvent::AfterCardPlayed,
            vec![Command::Creature(CreatureCmd::Heal {
                target: CreatureSelector::Owner,
                amount: Amount::Fixed(5),
            })],
        )],
    ));
    content.add_power(test_power(
        "POWER.TEST_REMOVER",
        vec![ModifierHook::every_turn(
            HookEvent::AfterCardPlayed,
            vec![buff("POWER.TEST_HEALER", -1)],
        )],
    ));
    content.add_power(test_power(
        "POWER.TEST_ADDER",
        vec![ModifierHook::every_turn(
            HookEvent::AfterCardPlayed,
            vec![buff("POWER.TEST_HEALER", 1)],
        )],
    ));
    // A blow at everyone its owner is fighting, which on the player kills the
    // first enemy and on an enemy hurts the player.
    let bite = |damage| {
        vec![Command::Creature(CreatureCmd::Damage {
            target: CreatureSelector::AllOpponents,
            amount: Amount::Fixed(damage),
            props: ValueProps::UNBLOCKABLE,
        })]
    };
    content.add_power(test_power(
        "POWER.TEST_SLAYER",
        vec![ModifierHook::every_turn(
            HookEvent::AfterCardPlayed,
            bite(99),
        )],
    ));
    content.add_power(test_power(
        "POWER.TEST_BITER",
        vec![ModifierHook::every_turn(
            HookEvent::AfterCardPlayed,
            bite(3),
        )],
    ));
    Arc::new(content)
}

fn test_power(model_id: &str, hooks: Vec<ModifierHook>) -> PowerDefinition {
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

#[test]
fn illegal_actions_preserve_every_byte_of_exact_state_and_rng() {
    let mut simulator = scenario();
    let before_state = simulator.state().clone();
    let before_key = simulator.state_key().unwrap();
    let before_rng = simulator.state().rng.to_serializable();
    let mut stale = play(&simulator, "CARD.TEST_CHOOSE");
    if let Action::PlayCard { card, .. } = &mut stale {
        card.index = 99;
    }
    let error = simulator.step(stale).unwrap_err();
    assert_eq!(error.code, ErrorCode::IllegalAction);
    assert_eq!(simulator.state(), &before_state);
    assert_eq!(simulator.state_key().unwrap(), before_key);
    assert_eq!(simulator.state().rng.to_serializable(), before_rng);
}

#[test]
fn snapshot_restore_replays_identically_at_decision_boundaries() {
    let mut simulator = scenario();
    let snapshot = simulator.snapshot();
    let action = play(&simulator, "CARD.TEST_CHOOSE");
    let first = simulator.step(action.clone()).unwrap();
    let first_actions = simulator.legal_actions().to_vec();
    let first_digest = simulator
        .projection_digest(ProjectionScope::CombatCore)
        .unwrap();

    simulator.restore(&snapshot).unwrap();
    let second = simulator.step(action).unwrap();
    assert_eq!(second.state_key, first.state_key);
    assert_eq!(second.rng_after, first.rng_after);
    assert_eq!(simulator.legal_actions(), first_actions);
    assert_eq!(
        simulator
            .projection_digest(ProjectionScope::CombatCore)
            .unwrap(),
        first_digest
    );
}

#[test]
fn portable_import_is_labeled_continuation_and_resumes_deterministically() {
    let simulator = scenario();
    let portable = simulator.project(ProjectionScope::RunCore).unwrap();
    let mut imported = Simulator::from_portable_state(portable, registry()).unwrap();
    assert_eq!(
        imported.state().origin,
        sts2_engine::ConformanceOrigin::PortableContinuation
    );
    assert_eq!(imported.legal_actions(), simulator.legal_actions());

    let mut original = simulator;
    let action = play(&original, "CARD.TEST_CHOOSE");
    let first = original.step(action.clone()).unwrap();
    let second = imported.step(action).unwrap();
    assert_eq!(first.next_decision, second.next_decision);
    assert_eq!(first.rng_after, second.rng_after);
}

#[test]
fn hidden_draw_order_does_not_change_observation_or_legal_actions() {
    let first = scenario();
    let mut state = first.state().clone();
    state.combat.as_mut().unwrap().player.piles.draw.reverse();
    let second = Simulator::from_scenario(state, registry()).unwrap();
    assert_eq!(first.agent_observation(), second.agent_observation());
    assert_eq!(first.legal_actions(), second.legal_actions());
    assert_ne!(first.state_key().unwrap(), second.state_key().unwrap());
}

#[test]
fn state_keys_ignore_map_insertion_order() {
    let first = scenario();
    let mut state = first.state().clone();
    state.combat.as_mut().unwrap().player.piles.hand[0]
        .fingerprint
        .properties = BTreeMap::from([("z".to_owned(), json!(2)), ("a".to_owned(), json!(1))]);
    let with_properties = Simulator::from_scenario(state.clone(), registry()).unwrap();
    let properties = &mut state.combat.as_mut().unwrap().player.piles.hand[0]
        .fingerprint
        .properties;
    properties.clear();
    properties.insert("a".to_owned(), json!(1));
    properties.insert("z".to_owned(), json!(2));
    let reordered = Simulator::from_scenario(state, registry()).unwrap();
    assert_eq!(
        with_properties.state_key().unwrap(),
        reordered.state_key().unwrap()
    );
}

/// A card the engine hands the player carries the saved properties its
/// model declares, and `ContentRegistry::fresh_card` is the factory that writes
/// them on. A second way of building one is how a card comes to stand without
/// them, so the bare constructor is called there and nowhere else in the crate.
/// A body that means to name a property itself does it in `sts2-content`, where
/// the definition that declares the property lives.
#[test]
fn the_engine_builds_a_card_only_in_the_factory() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut sites = Vec::new();
    for entry in std::fs::read_dir(&src).expect("the crate's own sources are readable") {
        let path = entry.expect("a readable directory entry").path();
        if path.extension().is_none_or(|extension| extension != "rs") {
            continue;
        }
        let body = std::fs::read_to_string(&path).expect("a readable source file");
        for (offset, line) in body.lines().enumerate() {
            if line.contains("CardFingerprint::base(") {
                sites.push(format!("{}:{}", path.display(), offset + 1));
            }
        }
    }
    assert_eq!(
        sites.len(),
        1,
        "a card the engine builds comes off ContentRegistry::fresh_card: {sites:?}"
    );
}

#[test]
fn invariants_find_duplicate_cards_stale_targets_and_unsettled_queues() {
    let simulator = scenario();
    let mut duplicate = simulator.state().clone();
    let card = duplicate.combat.as_ref().unwrap().player.piles.hand[0].clone();
    duplicate
        .combat
        .as_mut()
        .unwrap()
        .player
        .piles
        .draw
        .push(card.clone());
    let error = Simulator::from_scenario(duplicate, registry()).unwrap_err();
    assert_eq!(error.code, ErrorCode::InternalInvariant);

    let mut stale = simulator.state().clone();
    stale
        .combat
        .as_mut()
        .unwrap()
        .queue
        .push_back(QueuedCommand::queued(
            attack(1),
            CommandContext::by(CreatureCombatId::from(0)).at(CreatureCombatId::from(999)),
        ));
    let error = Simulator::from_scenario(stale, registry()).unwrap_err();
    assert_eq!(error.code, ErrorCode::InternalInvariant);

    let mut unsettled = simulator.state().clone();
    unsettled
        .combat
        .as_mut()
        .unwrap()
        .queue
        .push_back(Effect::DispatchModifierHooks(HookEvent::AfterCardPlayed));
    let error = Simulator::from_scenario(unsettled, registry()).unwrap_err();
    assert_eq!(error.code, ErrorCode::InternalInvariant);
}

fn branching_registry() -> Arc<ContentRegistry> {
    let mut registry = ContentRegistry::default();
    registry.add_card(CardDefinition {
        model_id: id("CARD.DEFEND_IRONCLAD"),
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
        energy_cost: 1,
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
        effects: vec![block(5)],
        transform_pool: None,
        visual_pool: None,
        adds_rest_site_option: None,
        hooks: Vec::new(),
    });
    let move_state = |state_id: &str, damage: i32| {
        MonsterState::Move(MonsterMove {
            state_id: state_id.to_owned(),
            intent: Intent::new(state_id, vec![MoveIntent::attack(damage, 1)]),
            commands: vec![monster_strike(damage)],
            follow_up: "RAND".to_owned(),
        })
    };
    registry.add_monster(MonsterDefinition {
        model_id: id("MONSTER.TEST_BRANCHER"),
        min_initial_hp: 30,
        max_initial_hp: 30,
        initial_state: "RAND".to_owned(),
        on_added: Vec::new(),
        hooks: Vec::new(),
        states: vec![
            move_state("LEFT", 1),
            move_state("RIGHT", 2),
            MonsterState::RandomBranch {
                state_id: "RAND".to_owned(),
                branches: vec![
                    MoveBranch::new("LEFT", MoveRepeat::CannotRepeat),
                    MoveBranch::new("RIGHT", MoveRepeat::CannotRepeat),
                ],
            },
        ],
    });
    Arc::new(registry)
}

#[test]
fn a_random_branch_rolls_once_per_turn_and_never_repeats_a_blocked_move() {
    let mut simulator = ScenarioBuilder::new("BRANCHTEST", id("ENCOUNTER.TEST"))
        .card(PileName::Hand, 1, fingerprint("CARD.DEFEND_IRONCLAD"), 1)
        .enemy(1, id("MONSTER.TEST_BRANCHER"), 30, 30, "LEFT")
        .build(branching_registry())
        .unwrap();
    let mut rolled = Vec::new();
    for turn in 1..=6 {
        let intent = simulator.state().combat.as_ref().unwrap().creatures[1]
            .intent
            .clone()
            .unwrap();
        rolled.push(intent.move_id);
        simulator.step(Action::EndTurn { turn }).unwrap();
    }
    // The opening move stands until it performs once, and every later roll is
    // forced away from the move that just ran.
    assert_eq!(rolled, ["LEFT", "RIGHT", "LEFT", "RIGHT", "LEFT", "RIGHT"]);
    // The opening move costs nothing, and each of the six turns that followed
    // rolled the branch exactly once.
    assert_eq!(
        simulator
            .state()
            .rng
            .stream(sts2_rng::RunStream::MonsterAi)
            .counter(),
        6
    );
    assert_eq!(
        simulator.state().combat.as_ref().unwrap().creatures[1].state_log,
        ["LEFT", "RIGHT", "LEFT", "RIGHT", "LEFT", "RIGHT", "LEFT"]
    );
}

#[allow(clippy::too_many_lines, reason = "one literal per registered model")]
fn relic_registry() -> Arc<ContentRegistry> {
    let mut registry = ContentRegistry::default();
    registry.add_card(CardDefinition {
        model_id: id("CARD.STRIKE_IRONCLAD"),
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
        card_type: CardType::Attack,
        target: TargetType::AnyEnemy,
        target_widens_with: None,
        energy_cost: 1,
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
        effects: vec![attack(6)],
        transform_pool: None,
        visual_pool: None,
        adds_rest_site_option: None,
        hooks: Vec::new(),
    });
    registry.add_monster(MonsterDefinition {
        model_id: id("MONSTER.TEST_DUMMY"),
        min_initial_hp: 20,
        max_initial_hp: 20,
        initial_state: "ATTACK".to_owned(),
        on_added: Vec::new(),
        hooks: Vec::new(),
        states: vec![MonsterState::Move(MonsterMove {
            state_id: "ATTACK".to_owned(),
            intent: Intent::new("ATTACK", vec![MoveIntent::attack(8, 1)]),
            commands: vec![monster_strike(8)],
            follow_up: "ATTACK".to_owned(),
        })],
    });
    registry.add_power(PowerDefinition {
        power_type: PowerType::Debuff,
        ..test_power(
            "POWER.TEST_WEAK",
            vec![
                ModifierHook::every_turn(
                    HookEvent::ModifyDamageMultiplicative,
                    vec![Command::Modify(ModifierCmd::Scale(Amount::Fixed(75)))],
                )
                .when(HookCondition::OwnerDealsPoweredDamage),
            ],
        )
    });
    registry.add_relic(RelicDefinition {
        model_id: id("RELIC.TEST_OPENER"),
        grants_an_extra_turn: None,
        map_rule: None,
        rarity: RelicRarity::Common,
        counter_property: None,
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
                HookEvent::BeforeSideTurnStart,
                vec![Command::Power(PowerCmd::Apply {
                    marks: None,
                    power: id("POWER.TEST_WEAK"),
                    target: CreatureSelector::AllOpponents,
                    applier: Some(CreatureSelector::Owner),
                    amount: Amount::Fixed(1),
                })],
            )
            .when(HookCondition::PlayerSideTurn)
            .timed(HookTiming::FirstTurnOnly),
        ],
    });
    Arc::new(registry)
}

#[test]
fn a_weak_power_scales_the_damage_its_owner_deals() {
    let mut simulator = ScenarioBuilder::new("RELICTEST", id("ENCOUNTER.TEST"))
        .card(PileName::Hand, 1, fingerprint("CARD.STRIKE_IRONCLAD"), 1)
        .relics(&[id("RELIC.TEST_OPENER")])
        .enemy(1, id("MONSTER.TEST_DUMMY"), 20, 20, "ATTACK")
        .build(relic_registry())
        .unwrap();
    // The dummy attacks for 8 with nothing on it.
    simulator.step(Action::EndTurn { turn: 1 }).unwrap();
    assert_eq!(
        simulator.state().combat.as_ref().unwrap().creatures[0].current_hp,
        72
    );

    let mut weakened = simulator.state().clone();
    let enemy = &mut weakened.combat.as_mut().unwrap().creatures[1];
    enemy.powers.push(sts2_engine::PowerInstance {
        instance: 0,
        target: None,
        model_id: id("POWER.TEST_WEAK"),
        held_card: None,
        bound_damage_card: None,
        amount: 1,
        counter: 0,
        amount_on_turn_start: 1,
        amounts_at_play_start: Vec::new(),
        play_tallies: Vec::new(),
        skip_next_duration_tick: false,
        applier: None,
        power_type: PowerType::Debuff,
        downgraded_cards: Vec::new(),
        latched: false,
    });
    let mut simulator = Simulator::from_scenario(weakened, relic_registry()).unwrap();
    // Weak scales what its owner deals, and the product truncates once: 8 * 75%.
    simulator.step(Action::EndTurn { turn: 2 }).unwrap();
    assert_eq!(
        simulator.state().combat.as_ref().unwrap().creatures[0].current_hp,
        66
    );
}

/// A side of two, one of whom shields the whole side the moment its own block
/// is announced as cleared.
fn turn_start_registry() -> Arc<ContentRegistry> {
    let mut registry = ContentRegistry::default();
    registry.add_monster(MonsterDefinition {
        model_id: id("MONSTER.TEST_IDLER"),
        min_initial_hp: 20,
        max_initial_hp: 20,
        initial_state: "WAIT".to_owned(),
        on_added: Vec::new(),
        hooks: Vec::new(),
        states: vec![MonsterState::Move(MonsterMove {
            state_id: "WAIT".to_owned(),
            intent: Intent::new("WAIT", vec![MoveIntent::of("unknown")]),
            commands: Vec::new(),
            follow_up: "WAIT".to_owned(),
        })],
    });
    registry.add_power(test_power(
        "POWER.TEST_SHIELDS_ALLIES",
        vec![ModifierHook::every_turn(
            HookEvent::AfterBlockCleared,
            vec![Command::Creature(CreatureCmd::GainBlock {
                target: CreatureSelector::AllAllies,
                amount: Amount::Fixed(7),
                props: ValueProps::UNPOWERED,
            })],
        )],
    ));
    Arc::new(registry)
}

#[test]
fn a_side_clears_all_of_its_block_before_any_of_it_is_announced() {
    // A side's turn opens with two passes over the creatures whose turn it is:
    // every one of them clears, and only then is each of them announced. A
    // listener answering for the first creature therefore finds the second one
    // already stripped, and what it hands the second one stands.
    let mut simulator = ScenarioBuilder::new("BLOCKCLEARTEST", id("ENCOUNTER.TEST"))
        .enemy(1, id("MONSTER.TEST_IDLER"), 20, 20, "WAIT")
        .enemy(2, id("MONSTER.TEST_IDLER"), 20, 20, "WAIT")
        .power(1, id("POWER.TEST_SHIELDS_ALLIES"), 1, PowerType::Buff)
        .build(turn_start_registry())
        .unwrap();

    simulator.step(Action::EndTurn { turn: 1 }).unwrap();

    let combat = simulator.state().combat.as_ref().unwrap();
    assert_eq!(
        combat.creatures[1].block, 7,
        "the shielder kept what it handed itself"
    );
    assert_eq!(
        combat.creatures[2].block, 7,
        "and so did its ally, whose block had already been cleared when the \
         first creature's clear was announced"
    );
}

/// A card that acts on whoever still holds it when the turn ends and burns for
/// having done it, and a relic that counts every card move it is told about.
#[allow(
    clippy::too_many_lines,
    reason = "one literal per model the test needs"
)]
fn held_card_registry() -> Arc<ContentRegistry> {
    let mut registry = ContentRegistry::default();
    registry.add_monster(MonsterDefinition {
        model_id: id("MONSTER.TEST_IDLER"),
        min_initial_hp: 20,
        max_initial_hp: 20,
        initial_state: "WAIT".to_owned(),
        on_added: Vec::new(),
        hooks: Vec::new(),
        states: vec![MonsterState::Move(MonsterMove {
            state_id: "WAIT".to_owned(),
            intent: Intent::new("WAIT", vec![MoveIntent::of("unknown")]),
            commands: Vec::new(),
            follow_up: "WAIT".to_owned(),
        })],
    });
    registry.add_card(CardDefinition {
        model_id: id("CARD.TEST_TOXIC"),
        rarity: CardRarity::Basic,
        ethereal: true,
        eternal: false,
        unplayable: true,
        end_of_turn_in_hand: vec![Command::Creature(CreatureCmd::Damage {
            target: CreatureSelector::Owner,
            amount: Amount::Fixed(5),
            props: ValueProps::UNPOWERED,
        })],
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
        target: TargetType::None,
        target_widens_with: None,
        energy_cost: -1,
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
        effects: Vec::new(),
        transform_pool: None,
        visual_pool: None,
        adds_rest_site_option: None,
        hooks: Vec::new(),
    });
    registry.add_relic(RelicDefinition {
        model_id: id("RELIC.TEST_PILE_WATCHER"),
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
        hooks: vec![ModifierHook::every_turn(
            HookEvent::AfterCardChangedPiles,
            vec![Command::Relic(
                sts2_engine::RelicCmd::ModifyListeningCounter(Amount::Fixed(1)),
            )],
        )],
    });
    Arc::new(registry)
}

#[test]
fn a_card_that_acts_at_the_end_of_the_turn_acts_from_the_play_pile() {
    // A card that acts from the hand at the end of the turn does three things
    // in order: it goes to the play pile, its body runs, and then it is put
    // down. Both moves are announced, so a card acting at the end of the turn
    // changes piles twice rather than once, and the hand it is not in any more
    // is what its body sees.
    let mut simulator = ScenarioBuilder::new("HELDCARDTEST", id("ENCOUNTER.TEST"))
        .card(PileName::Hand, 1, fingerprint("CARD.TEST_TOXIC"), -1)
        .relics(&[id("RELIC.TEST_PILE_WATCHER")])
        .enemy(1, id("MONSTER.TEST_IDLER"), 20, 20, "WAIT")
        .build(held_card_registry())
        .unwrap();

    simulator.step(Action::EndTurn { turn: 1 }).unwrap();

    let combat = simulator.state().combat.as_ref().unwrap();
    assert_eq!(combat.creatures[0].current_hp, 75, "the body ran");
    assert_eq!(
        combat.player.piles.exhaust.len(),
        1,
        "and the card burned afterwards, because it is ethereal"
    );
    assert!(combat.player.piles.hand.is_empty());
    assert!(combat.player.piles.play.is_empty());
    assert_eq!(
        simulator.state().run_player.relics[0].counter,
        2,
        "hand to play before the body, play to exhaust after it: nothing else \
         in the turn moves a card, because the piles are otherwise empty"
    );
}

/// A player who finishes the fight from their own end-of-turn listeners, and a
/// card that would hurt them if the hand were still to act.
fn turn_end_victory_registry() -> Arc<ContentRegistry> {
    let mut registry = ContentRegistry::default();
    registry.add_monster(MonsterDefinition {
        model_id: id("MONSTER.TEST_IDLER"),
        min_initial_hp: 20,
        max_initial_hp: 20,
        initial_state: "WAIT".to_owned(),
        on_added: Vec::new(),
        hooks: Vec::new(),
        states: vec![MonsterState::Move(MonsterMove {
            state_id: "WAIT".to_owned(),
            intent: Intent::new("WAIT", vec![MoveIntent::of("unknown")]),
            commands: vec![monster_strike(9)],
            follow_up: "WAIT".to_owned(),
        })],
    });
    registry.add_power(test_power(
        "POWER.TEST_FINISHER",
        vec![
            ModifierHook::every_turn(
                HookEvent::BeforeSideTurnEnd,
                vec![Command::Damage(DamageCmd::Attack {
                    amount: Amount::Fixed(99),
                    hits: Amount::Fixed(1),
                    target: CreatureSelector::AllOpponents,
                    props: ValueProps::UNPOWERED,
                })],
            )
            .when(HookCondition::OwnerIsParticipant),
        ],
    ));
    registry.add_card(CardDefinition {
        model_id: id("CARD.TEST_TOXIC"),
        rarity: CardRarity::Basic,
        ethereal: false,
        eternal: false,
        unplayable: true,
        end_of_turn_in_hand: vec![Command::Creature(CreatureCmd::Damage {
            target: CreatureSelector::Owner,
            amount: Amount::Fixed(5),
            props: ValueProps::UNPOWERED,
        })],
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
        target: TargetType::None,
        target_widens_with: None,
        energy_cost: -1,
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
        effects: Vec::new(),
        transform_pool: None,
        visual_pool: None,
        adds_rest_site_option: None,
        hooks: Vec::new(),
    });
    Arc::new(registry)
}

#[test]
fn a_fight_won_at_the_end_of_a_turn_ends_before_the_rest_of_the_handover() {
    // The first phase of ending the player's turn asks whether the fight is
    // won the moment the before-turn-end listeners are done, and stops
    // outright if it is. So the hand never acts, the turn never changes hands,
    // and the fight is over on the same decision the player ended their turn
    // with.
    let mut simulator = ScenarioBuilder::new("TURNENDWINTEST", id("ENCOUNTER.TEST"))
        .card(PileName::Hand, 1, fingerprint("CARD.TEST_TOXIC"), -1)
        .power(0, id("POWER.TEST_FINISHER"), 1, PowerType::Buff)
        .enemy(1, id("MONSTER.TEST_IDLER"), 20, 20, "WAIT")
        .build(turn_end_victory_registry())
        .unwrap();

    simulator.step(Action::EndTurn { turn: 1 }).unwrap();

    let combat = simulator.state().combat.as_ref().unwrap();
    assert!(
        combat.rewards_open,
        "the fight was finished where it was won"
    );
    assert_eq!(
        combat.creatures[0].current_hp, 80,
        "the card still in hand never acted, because phase one returned at the \
         check before it"
    );
    assert_eq!(
        combat.player.turn, 1,
        "and the enemy never took a turn, nor the player another"
    );
    assert!(
        combat.queue.is_empty(),
        "nothing of the handover is still owed"
    );
}

/// A pool with one card of every rarity, so what a combat generation roll
/// leaves out is measured rather than assumed.
#[allow(clippy::too_many_lines, reason = "one block per rarity in the pool")]
fn pool_filter_registry() -> Arc<ContentRegistry> {
    let mut registry = ContentRegistry::default();
    let rarities = [
        ("CARD.POOL_BASIC", CardRarity::Basic),
        ("CARD.POOL_COMMON", CardRarity::Common),
        ("CARD.POOL_UNCOMMON", CardRarity::Uncommon),
        ("CARD.POOL_RARE", CardRarity::Rare),
        ("CARD.POOL_ANCIENT", CardRarity::Ancient),
        ("CARD.POOL_EVENT", CardRarity::Event),
        ("CARD.POOL_TOKEN", CardRarity::Token),
        ("CARD.POOL_STATUS", CardRarity::Status),
        ("CARD.POOL_CURSE", CardRarity::Curse),
        ("CARD.POOL_QUEST", CardRarity::Quest),
    ];
    for (model_id, rarity) in rarities {
        registry.add_card(CardDefinition {
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
            effects: Vec::new(),
            transform_pool: None,
            visual_pool: None,
            adds_rest_site_option: None,
            hooks: Vec::new(),
        });
    }
    // The card that does the generating: it takes the whole filtered pool, so
    // what lands in the discard pile is exactly what the filter passed.
    registry.add_card(CardDefinition {
        model_id: id("CARD.POOL_ROLLER"),
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
        effects: vec![Command::CardPile(
            sts2_engine::CardPileCmd::AddGeneratedFromPool {
                pool: sts2_engine::CardPoolRef::Named(id("CARD_POOL.TEST_POOL")),
                count: Amount::Fixed(20),
                distinct: true,
                filter: CardFilter::Any,
                pile: PileName::Discard,
                position: sts2_engine::DrawPilePosition::Bottom,
                excludes_source: false,
                upgraded: false,
                temporary_cost: None,
                ethereal: false,
                rolled_one_at_a_time: false,
            },
        )],
        transform_pool: None,
        visual_pool: None,
        adds_rest_site_option: None,
        hooks: Vec::new(),
    });
    registry.add_card_pool(sts2_engine::CardPoolDefinition {
        model_id: id("CARD_POOL.TEST_POOL"),
        is_colorless: false,
        cards: rarities
            .iter()
            .map(|(model_id, rarity)| sts2_engine::PoolCard {
                model_id: id(model_id),
                card_type: CardType::Skill,
                rarity: *rarity,
                energy_cost: 0,
                costs_x: false,
                can_be_generated_in_combat: true,
                can_be_generated_by_modifiers: true,
            })
            .collect(),
    });
    registry.add_monster(MonsterDefinition {
        model_id: id("MONSTER.TEST_IDLER"),
        min_initial_hp: 20,
        max_initial_hp: 20,
        initial_state: "WAIT".to_owned(),
        on_added: Vec::new(),
        hooks: Vec::new(),
        states: vec![MonsterState::Move(MonsterMove {
            state_id: "WAIT".to_owned(),
            intent: Intent::new("WAIT", vec![MoveIntent::of("unknown")]),
            commands: Vec::new(),
            follow_up: String::new(),
        })],
    });
    Arc::new(registry)
}

#[test]
fn a_combat_generation_roll_drops_the_basic_ancient_and_event_rarities() {
    // A roll that generates a card during a fight keeps a card its content
    // allows to be generated in combat and whose rarity is none of Basic,
    // Ancient, or Event. A status, a curse, a token, and a quest card are all
    // still fair game — a pool that stocks them is what a generation effect
    // rolls against.
    let mut simulator = ScenarioBuilder::new("POOLFILTERTEST", id("ENCOUNTER.TEST"))
        .card(PileName::Hand, 1, fingerprint("CARD.POOL_ROLLER"), 0)
        .enemy(1, id("MONSTER.TEST_IDLER"), 20, 20, "WAIT")
        .build(pool_filter_registry())
        .unwrap();
    let action = play(&simulator, "CARD.POOL_ROLLER");
    simulator.step(action).unwrap();
    let mut generated: Vec<String> = simulator
        .state()
        .combat
        .as_ref()
        .unwrap()
        .player
        .piles
        .discard[..]
        .iter()
        .map(|card| card.fingerprint.model_id.to_string())
        .collect();
    generated.sort();
    assert_eq!(
        generated,
        [
            "CARD.POOL_COMMON",
            "CARD.POOL_CURSE",
            "CARD.POOL_QUEST",
            "CARD.POOL_RARE",
            // The card that did the rolling, put down where a played skill goes.
            "CARD.POOL_ROLLER",
            "CARD.POOL_STATUS",
            "CARD.POOL_TOKEN",
            "CARD.POOL_UNCOMMON",
        ]
    );
}

/// A card whose body writes a saved property of its own, counting the fights
/// the run has seen and leaving the deck at its limit.
#[allow(clippy::too_many_lines, reason = "one card per saved property")]
fn saved_property_registry() -> Arc<ContentRegistry> {
    let mut registry = ContentRegistry::default();
    let mut counter = CardDefinition {
        model_id: id("CARD.TEST_COUNTER"),
        rarity: CardRarity::Curse,
        ethereal: false,
        eternal: false,
        unplayable: true,
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
        card_type: CardType::Curse,
        target: TargetType::None,
        target_widens_with: None,
        energy_cost: -1,
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
        effects: Vec::new(),
        transform_pool: None,
        visual_pool: None,
        adds_rest_site_option: None,
        hooks: Vec::new(),
    };
    // The shape of `CARD.GUILTY` at the end of a fight: only the deck's own
    // card counts, so the combat's copy of it does not count a second one.
    counter.hooks.push(ModifierHook::every_turn(
        HookEvent::AfterCombatEnd,
        vec![Command::When {
            otherwise: Vec::new(),
            condition: sts2_engine::Condition::SourceCardIsInTheDeck,
            then: vec![
                Command::Card(sts2_engine::CardCmd::ModifyProperty {
                    cards: sts2_engine::CardSelector::Source,
                    name: "CombatsSeen".to_owned(),
                    amount: Amount::Fixed(1),
                    and_the_deck_version: false,
                }),
                Command::When {
                    otherwise: Vec::new(),
                    condition: sts2_engine::Condition::SourceCardPropertyAtLeast {
                        name: "CombatsSeen".to_owned(),
                        value: 2,
                    },
                    then: vec![Command::CardPile(
                        sts2_engine::CardPileCmd::RemoveFromDeck {
                            cards: sts2_engine::CardSelector::Source,
                        },
                    )],
                },
            ],
        }],
    ));
    registry.add_card(counter);
    registry.add_card(CardDefinition {
        model_id: id("CARD.TEST_FINISHER"),
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
        card_type: CardType::Attack,
        target: TargetType::AnyEnemy,
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
        effects: vec![attack(50)],
        transform_pool: None,
        visual_pool: None,
        adds_rest_site_option: None,
        hooks: Vec::new(),
    });
    registry.add_monster(MonsterDefinition {
        model_id: id("MONSTER.TEST_IDLER"),
        min_initial_hp: 20,
        max_initial_hp: 20,
        initial_state: "WAIT".to_owned(),
        on_added: Vec::new(),
        hooks: Vec::new(),
        states: vec![MonsterState::Move(MonsterMove {
            state_id: "WAIT".to_owned(),
            intent: Intent::new("WAIT", vec![MoveIntent::of("unknown")]),
            commands: Vec::new(),
            follow_up: String::new(),
        })],
    });
    Arc::new(registry)
}

#[test]
fn a_cards_saved_property_belongs_to_the_deck_card_and_outlives_the_fight() {
    let mut simulator = ScenarioBuilder::new("CARDPROPTEST", id("ENCOUNTER.TEST"))
        .card(PileName::Hand, 1, fingerprint("CARD.TEST_COUNTER"), -1)
        .card(PileName::Hand, 2, fingerprint("CARD.TEST_FINISHER"), 0)
        .enemy(1, id("MONSTER.TEST_IDLER"), 20, 20, "WAIT")
        .build(saved_property_registry())
        .unwrap();
    simulator
        .step(play(&simulator, "CARD.TEST_FINISHER"))
        .unwrap();
    // The fight is over: the deck card counted one, and the combat's copy of it
    // — which is on the same run-scope listener list — counted none.
    let deck = &simulator.state().run_player.deck;
    let counted = deck
        .iter()
        .find(|card| card.model_id == id("CARD.TEST_COUNTER"))
        .expect("the counter is still in the deck");
    assert_eq!(counted.properties.get("CombatsSeen"), Some(&json!(1)));
    assert_eq!(
        simulator
            .state()
            .combat
            .as_ref()
            .unwrap()
            .player
            .piles
            .all_cards()
            .filter(|card| card.fingerprint.model_id == id("CARD.TEST_COUNTER"))
            .filter_map(|card| card.fingerprint.properties.get("CombatsSeen"))
            .count(),
        0,
        "the fight's own copy wrote nothing, because it is not the deck's card"
    );
}
