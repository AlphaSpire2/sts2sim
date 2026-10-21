//! The screens death listeners ask for: queued behind the action that was
//! running, shown in the order they were asked for, and cancelled with the
//! fight.
//!
//! The build gives every after-death listener a pause point of its own and
//! never waits for it. A body that asks is put on the player's action queue as
//! an action of its own, so the pass, the kill, and the action the kill
//! happened inside all run on, and the question is put to the player once the
//! queue reaches it.

use std::sync::Arc;

use sts2_core::ModelId;
use sts2_engine::{
    Action, Amount, CardCmd, CardDefinition, CardFilter, CardFingerprint, CardRarity,
    CardSelectCmd, CardSelector, CardType, Command, ContentRegistry, CreatureCmd, CreatureSelector,
    DamageCmd, DecisionContext, ErrorCode, HookCondition, HookEvent, Intent, ModifierHook,
    MonsterDefinition, MonsterMove, MonsterState, MoveIntent, PileName, PowerDefinition,
    PowerInstanceType, PowerStackType, PowerType, ProjectionScope, RelicCmd, RelicDefinition,
    RelicRarity, ScenarioBuilder, SelectorPrefs, Simulator, TargetType, ValueProps,
};

const SEED: &str = "QUEUEDHOOKS";

fn id(value: &str) -> ModelId {
    value.parse().expect("static model ID")
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

/// A draw-pile screen asking for exactly one card, which shows whenever the
/// pile holds more than one; the picked card is exhausted, then `then` runs.
fn draw_pile_screen(then: Vec<Command>) -> Command {
    let mut body = vec![Command::Card(CardCmd::Exhaust {
        cards: CardSelector::Chosen,
    })];
    body.extend(then);
    Command::CardSelect(CardSelectCmd::FromCombatPile {
        semantics: sts2_engine::ChoiceSemantics::default(),
        pile: PileName::Draw,
        prefs: SelectorPrefs::exactly(1),
        maximum: None,
        exact_count: None,
        restricted_to_chosen: false,
        filter: CardFilter::Any,
        then: body,
    })
}

/// A listener that asks over the draw pile whenever an opponent of its owner
/// goes down.
fn on_an_opponents_death(commands: Vec<Command>) -> Vec<ModifierHook> {
    vec![
        ModifierHook::every_turn(HookEvent::AfterDeath, commands).when(HookCondition::OpponentDied),
    ]
}

fn hit(amount: i32, target: CreatureSelector) -> Command {
    Command::Damage(DamageCmd::Attack {
        amount: Amount::Fixed(amount),
        hits: Amount::Fixed(1),
        target,
        props: ValueProps::NONE,
    })
}

fn registry() -> Arc<ContentRegistry> {
    let mut registry = ContentRegistry::default();
    registry.add_card(card(
        "CARD.FILLER",
        CardType::Skill,
        TargetType::SelfTarget,
        Vec::new(),
    ));
    // A kill with something of the play still owed behind the blow.
    registry.add_card(card(
        "CARD.KILL",
        CardType::Attack,
        TargetType::AnyEnemy,
        vec![hit(10, CreatureSelector::Target), block(2)],
    ));
    registry.add_power(power(
        "POWER.DEATH_ASKER_A",
        on_an_opponents_death(vec![draw_pile_screen(vec![block(3)])]),
    ));
    registry.add_power(power(
        "POWER.DEATH_ASKER_B",
        on_an_opponents_death(vec![draw_pile_screen(vec![block(5)])]),
    ));
    // A listener behind an asker on the same pass that asks nothing.
    registry.add_power(power(
        "POWER.DEATH_ANSWERER",
        on_an_opponents_death(vec![block(7)]),
    ));
    // An asker whose answer finishes the fight.
    registry.add_power(power(
        "POWER.DEATH_ASKER_THAT_ENDS_IT",
        on_an_opponents_death(vec![draw_pile_screen(vec![hit(
            100,
            CreatureSelector::AllOpponents,
        )])]),
    ));
    // An asker that counts every answer it is given, on a counter that outlives
    // the fight.
    registry.add_relic(relic(
        "RELIC.DEATH_COUNTER",
        on_an_opponents_death(vec![draw_pile_screen(vec![Command::Relic(
            RelicCmd::ModifyListeningCounter(Amount::Fixed(1)),
        )])]),
    ));
    let wait = |model: &str, commands: Vec<Command>| MonsterDefinition {
        model_id: id(model),
        min_initial_hp: 40,
        max_initial_hp: 40,
        initial_state: "WAIT".to_owned(),
        states: vec![MonsterState::Move(MonsterMove {
            state_id: "WAIT".to_owned(),
            intent: Intent::new("WAIT", vec![MoveIntent::of("unknown")]),
            commands,
            follow_up: String::new(),
        })],
        on_added: Vec::new(),
        hooks: Vec::new(),
    };
    registry.add_monster(wait("MONSTER.DUMMY", Vec::new()));
    // A monster that goes down on its own turn.
    registry.add_monster(wait(
        "MONSTER.SELF_DESTRUCT",
        vec![Command::Creature(CreatureCmd::Damage {
            target: CreatureSelector::Owner,
            amount: Amount::Fixed(100),
            props: ValueProps::UNPOWERED,
        })],
    ));
    Arc::new(registry)
}

fn fingerprint(model: &str) -> CardFingerprint {
    CardFingerprint::base(id(model))
}

/// Two dummies, the first one blow from death, the kill in hand, and two
/// fillers in the draw pile for the screens to ask over.
fn builder() -> ScenarioBuilder {
    ScenarioBuilder::new(SEED, id("ENCOUNTER.TEST"))
        .player_hp(60, 80)
        .enemy(1, id("MONSTER.DUMMY"), 5, 40, "WAIT")
        .enemy(2, id("MONSTER.DUMMY"), 40, 40, "WAIT")
        .card(PileName::Hand, 1, fingerprint("CARD.KILL"), 0)
        .card(PileName::Draw, 2, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Draw, 3, fingerprint("CARD.FILLER"), 0)
}

fn kill_the_first(simulator: &Simulator) -> Action {
    simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::PlayCard { card, target: Some(target) }
                if card.fingerprint.model_id == id("CARD.KILL") && target.combat_id.get() == 1)
        })
        .expect("the kill is playable at the first dummy")
        .clone()
}

fn pick_first(simulator: &Simulator) -> Action {
    simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseCards { cards, .. } if cards.len() == 1))
        .expect("a single-card answer is offered")
        .clone()
}

fn choice_id(simulator: &Simulator) -> u64 {
    match simulator.decision() {
        DecisionContext::ChooseCards { choice_id, .. } => choice_id.get(),
        other => panic!("expected a card choice, found {other:?}"),
    }
}

fn block_of(simulator: &Simulator) -> i32 {
    simulator.state().combat.as_ref().unwrap().creatures[0].block
}

fn queued(simulator: &Simulator) -> usize {
    simulator
        .state()
        .combat
        .as_ref()
        .map_or(0, |combat| combat.queued_hook_screens.len())
}

fn pile(simulator: &Simulator, pile: PileName) -> Vec<String> {
    let piles = &simulator.state().combat.as_ref().unwrap().player.piles;
    let cards = match pile {
        PileName::Hand => &piles.hand,
        PileName::Draw => &piles.draw,
        PileName::Discard => &piles.discard,
        PileName::Exhaust => &piles.exhaust,
        PileName::Play => &piles.play,
    };
    cards
        .iter()
        .map(|card| card.fingerprint.model_id.to_string())
        .collect()
}

#[test]
fn a_death_listeners_screen_is_shown_once_the_play_is_over() {
    // The asker is first on the pass and a listener that asks nothing stands
    // behind it. The answerer acts, the play finishes its own block and puts
    // its card away, and only then is the asker's screen shown.
    let mut simulator = builder()
        .power(0, id("POWER.DEATH_ASKER_A"), 1, PowerType::Buff)
        .power(0, id("POWER.DEATH_ANSWERER"), 1, PowerType::Buff)
        .build(registry())
        .unwrap();
    simulator.step(kill_the_first(&simulator)).unwrap();
    assert!(matches!(
        simulator.decision(),
        DecisionContext::ChooseCards { .. }
    ));
    assert_eq!(
        block_of(&simulator),
        2 + 7,
        "the play's own block and the answerer behind the asker both ran"
    );
    assert_eq!(pile(&simulator, PileName::Discard), vec!["CARD.KILL"]);
    assert_eq!(queued(&simulator), 0, "the one queued screen is the one up");

    simulator.step(pick_first(&simulator)).unwrap();
    assert_eq!(block_of(&simulator), 2 + 7 + 3, "the asker's body ran once");
    assert_eq!(pile(&simulator, PileName::Exhaust), vec!["CARD.FILLER"]);
    assert!(matches!(
        simulator.decision(),
        DecisionContext::CombatPriority { .. }
    ));
}

#[test]
fn death_listeners_screens_are_shown_in_the_order_they_were_asked_for() {
    // Two askers on one pass: both are queued, the first is shown behind the
    // play, and the second behind the first one's answer. The ids say which
    // asked first, because the id is reserved when the body asks.
    let mut simulator = builder()
        .power(0, id("POWER.DEATH_ASKER_A"), 1, PowerType::Buff)
        .power(0, id("POWER.DEATH_ASKER_B"), 1, PowerType::Buff)
        .card(PileName::Draw, 4, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    simulator.step(kill_the_first(&simulator)).unwrap();
    let first = choice_id(&simulator);
    assert_eq!(queued(&simulator), 1, "the second screen waits its turn");
    assert_eq!(block_of(&simulator), 2);

    simulator.step(pick_first(&simulator)).unwrap();
    assert_eq!(
        block_of(&simulator),
        2 + 3,
        "the first asker answered first"
    );
    assert_eq!(choice_id(&simulator), first + 1);
    assert_eq!(queued(&simulator), 0);

    simulator.step(pick_first(&simulator)).unwrap();
    assert_eq!(block_of(&simulator), 2 + 3 + 5);
    assert!(matches!(
        simulator.decision(),
        DecisionContext::CombatPriority { .. }
    ));
}

fn two_askers_queued() -> Simulator {
    let mut simulator = builder()
        .power(0, id("POWER.DEATH_ASKER_A"), 1, PowerType::Buff)
        .power(0, id("POWER.DEATH_ASKER_B"), 1, PowerType::Buff)
        .card(PileName::Draw, 4, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    simulator.step(kill_the_first(&simulator)).unwrap();
    assert_eq!(queued(&simulator), 1);
    simulator
}

#[test]
fn a_queued_screen_survives_a_snapshot_and_the_portable_boundary() {
    let simulator = two_askers_queued();

    // A snapshot restored over a simulator that has since moved on.
    let snapshot = simulator.snapshot();
    let mut restored = simulator.clone();
    restored.step(pick_first(&restored)).unwrap();
    restored.restore(&snapshot).unwrap();
    assert_eq!(
        restored.state_key().unwrap(),
        simulator.state_key().unwrap()
    );
    assert_eq!(queued(&restored), 1);

    // The state carried out through the portable boundary and back.
    let portable = simulator.project(ProjectionScope::CombatCore).unwrap();
    let mut imported = Simulator::from_portable_state(portable, registry()).unwrap();
    assert_eq!(imported.state().combat, simulator.state().combat);
    assert_eq!(imported.legal_actions(), simulator.legal_actions());

    let mut original = simulator;
    for _ in 0..2 {
        let action = pick_first(&original);
        let first = original.step(action.clone()).unwrap();
        let second = restored.step(action.clone()).unwrap();
        let third = imported.step(action).unwrap();
        assert_eq!(first.state_key, second.state_key);
        assert_eq!(first.next_decision, third.next_decision);
        assert_eq!(first.rng_after, third.rng_after);
        assert_eq!(original.state().combat, imported.state().combat);
    }
    assert_eq!(block_of(&original), 2 + 3 + 5);
}

#[test]
fn quiet_and_full_stepping_agree_over_queued_screens() {
    let mut full = builder()
        .power(0, id("POWER.DEATH_ASKER_A"), 1, PowerType::Buff)
        .power(0, id("POWER.DEATH_ASKER_B"), 1, PowerType::Buff)
        .card(PileName::Draw, 4, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    let mut quiet = full.clone();
    for _ in 0..3 {
        let action = if matches!(full.decision(), DecisionContext::ChooseCards { .. }) {
            pick_first(&full)
        } else {
            kill_the_first(&full)
        };
        full.step(action.clone()).unwrap();
        quiet.step_quietly(&action).unwrap();
        assert_eq!(full.state_key().unwrap(), quiet.state_key().unwrap());
        assert_eq!(full.legal_actions(), quiet.legal_actions());
    }
    assert!(matches!(
        full.decision(),
        DecisionContext::CombatPriority { .. }
    ));
}

#[test]
fn an_invalid_answer_leaves_the_queued_screens_where_they_stood() {
    let mut simulator = two_askers_queued();
    let before = simulator.state_key().unwrap();
    let Action::ChooseCards { choice_id, cards } = pick_first(&simulator) else {
        unreachable!()
    };
    let wrong = Action::ChooseCards {
        choice_id: sts2_core::ChoiceId::from(choice_id.get() + 1),
        cards,
    };
    assert!(simulator.step(wrong).is_err());
    assert_eq!(simulator.state_key().unwrap(), before);
    assert_eq!(queued(&simulator), 1);
    simulator.step(pick_first(&simulator)).unwrap();
    assert_eq!(block_of(&simulator), 2 + 3);
}

#[test]
fn a_fight_won_behind_a_queued_screen_cancels_the_screens_still_waiting() {
    // The first asker's answer kills the last enemy. The counting relic asked
    // at the first death too, and its screen was waiting behind the first one:
    // the win cancels it, so what it owed never runs. At the second death the
    // fight is already ending, so the relic's screen there settles with
    // nothing picked and its body runs once, then.
    let mut simulator = builder()
        .power(0, id("POWER.DEATH_ASKER_THAT_ENDS_IT"), 1, PowerType::Buff)
        .relics(&[id("RELIC.DEATH_COUNTER")])
        .card(PileName::Draw, 4, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    simulator.step(kill_the_first(&simulator)).unwrap();
    assert_eq!(queued(&simulator), 1, "the relic's screen waits its turn");

    simulator.step(pick_first(&simulator)).unwrap();
    assert!(
        !matches!(simulator.decision(), DecisionContext::ChooseCards { .. }),
        "nothing is asked over a won fight: {:?}",
        simulator.decision()
    );
    assert_eq!(queued(&simulator), 0);
    let counter = simulator
        .state()
        .run_player
        .relics
        .iter()
        .find(|relic| relic.model_id == id("RELIC.DEATH_COUNTER"))
        .unwrap()
        .counter;
    assert_eq!(counter, 1, "only the second death's body ran");
}

#[test]
fn a_death_listeners_screen_outside_the_players_own_action_is_refused() {
    // A monster that goes down on its own turn: the build queues the asker's
    // action against an enemy turn that is still being played out, a timing
    // this engine does not carry. The stop names the discipline rather than
    // failing as an invariant, and the full step puts everything back.
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.TEST"))
        .player_hp(60, 80)
        .enemy(1, id("MONSTER.SELF_DESTRUCT"), 5, 40, "WAIT")
        .enemy(2, id("MONSTER.DUMMY"), 40, 40, "WAIT")
        .power(0, id("POWER.DEATH_ASKER_A"), 1, PowerType::Buff)
        .card(PileName::Draw, 2, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Draw, 3, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Draw, 4, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Draw, 5, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Draw, 6, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Draw, 7, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Draw, 8, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    let before = simulator.state_key().unwrap();
    let error = simulator.step(Action::EndTurn { turn: 1 }).unwrap_err();
    assert_eq!(error.code, ErrorCode::UnsupportedContent);
    assert_eq!(
        error.details.get("suspension_blocked_by").unwrap(),
        "a listener the build queues outside the player's own action"
    );
    assert_eq!(simulator.state_key().unwrap(), before);
}
