//! Every loop a model body writes, interrupted by a selection screen and
//! picked back up: the counted repeats, the per-kill repeat and its deferred
//! count, the walks over cards and orbs, a channel behind its make-room
//! evoke, and an auto-play over several cards.
//!
//! In the build each of these is a loop whose round can stop part-way to ask,
//! with the rounds still to come held by the paused body itself; here the
//! rounds a screen has not reached ride the queue as a frame. Each test pins
//! three things — what has *not* happened while the question is standing,
//! that the rounds resume in the build's own order once it is answered, and
//! that a state cloned at the boundary walks to the same place.

use std::sync::Arc;

use sts2_core::ModelId;
use sts2_engine::{
    Action, Amount, CardCmd, CardDefinition, CardFilter, CardFingerprint, CardRarity,
    CardSelectCmd, CardSelector, CardType, Command, ContentRegistry, CreatureCmd, CreatureSelector,
    DecisionContext, Intent, MonsterDefinition, MonsterMove, MonsterState, MoveIntent, OrbCmd,
    OrbDefinition, OrbSource, OrbTrigger, OrbValue, PileName, ScenarioBuilder, SelectorPrefs,
    Simulator, TargetType, ValueProps,
};

const SEED: &str = "LOOPTEST";

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

fn block(amount: i32) -> Command {
    Command::Creature(CreatureCmd::GainBlock {
        target: CreatureSelector::Owner,
        amount: Amount::Fixed(amount),
        props: ValueProps::UNPOWERED,
    })
}

/// A screen over the hand asking for exactly one card, which comes up whenever
/// the hand holds at least one.
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

fn cards() -> Vec<CardDefinition> {
    vec![
        card("CARD.FILLER", Vec::new()),
        // A body that asks and then gains block, so a play pins both halves.
        card("CARD.ASKS", vec![hand_screen(vec![block(3)])]),
        // Two rounds, each of which asks and then finishes its own body.
        card(
            "CARD.TWO_ASKS",
            vec![Command::Repeat {
                times: Amount::Fixed(2),
                then: vec![hand_screen(vec![block(5)]), block(2)],
            }],
        ),
        // Three rounds of which the last runs its own arm, however many of
        // them screens deferred.
        card(
            "CARD.THREE_WITH_LAST",
            vec![Command::RepeatWithLast {
                times: Amount::Fixed(3),
                then: vec![hand_screen(vec![block(1)])],
                last: vec![hand_screen(vec![block(10)])],
            }],
        ),
        // A blow over everything and a question behind it: the kills the blow
        // made are counted only once the question is answered.
        card(
            "CARD.ECHO",
            vec![Command::RepeatPerKill {
                then: vec![
                    Command::Creature(CreatureCmd::Damage {
                        target: CreatureSelector::AllOpponents,
                        amount: Amount::Fixed(10),
                        props: ValueProps::UNPOWERED,
                    }),
                    hand_screen(vec![block(2)]),
                ],
            }],
        ),
        // One question per card of the discard pile.
        card(
            "CARD.SWEEP_DISCARD",
            vec![Command::ForEachCard {
                cards: CardSelector::Pile {
                    pile: PileName::Discard,
                    filter: CardFilter::Any,
                },
                then: vec![hand_screen(vec![block(1)]), block(1)],
            }],
        ),
        // One question per orb standing.
        card(
            "CARD.STIR_ORBS",
            vec![Command::ForEachOrb {
                orbs: sts2_engine::OrbSelector::All,
                then: vec![hand_screen(vec![block(1)])],
            }],
        ),
        // Every orb's passive, each of which asks.
        card(
            "CARD.PULSE_ORBS",
            vec![Command::Orb(OrbCmd::Passive {
                orb: sts2_engine::OrbSelector::All,
                target: None,
            })],
        ),
        // A channel into a full queue, whose make-room evoke asks.
        card(
            "CARD.CHANNEL_ASKER",
            vec![Command::Orb(OrbCmd::Channel(OrbSource::Model(id(
                "ORB.ASKER",
            ))))],
        ),
        // The whole hand played without being chosen, one play at a time, each
        // finishing before the next one starts.
        card(
            "CARD.MASS_PLAY",
            vec![Command::Card(CardCmd::AutoPlay {
                cards: CardSelector::Pile {
                    pile: PileName::Hand,
                    filter: CardFilter::Any,
                },
                at_the_bodys_target: false,
            })],
        ),
        // The same walk over copies, with the originals staying where they are.
        card(
            "CARD.MASS_DUPE",
            vec![Command::Card(CardCmd::AutoPlayDupe {
                cards: CardSelector::Pile {
                    pile: PileName::Hand,
                    filter: CardFilter::Any,
                },
            })],
        ),
    ]
}

/// An orb whose passive asks, and whose evoke asks and then finishes its own
/// body — the shape that pins a walk stopping where the question stands.
fn asker_orb() -> OrbDefinition {
    OrbDefinition {
        model_id: id("ORB.ASKER"),
        passive_value: OrbValue::Fixed(0),
        evoke_value: OrbValue::Fixed(0),
        turn_trigger: OrbTrigger::AfterTurnStart,
        passive: vec![hand_screen(vec![block(2)])],
        evoke: vec![hand_screen(vec![block(4)]), block(1)],
    }
}

fn dummy_monster() -> MonsterDefinition {
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
    }
}

fn registry() -> Arc<ContentRegistry> {
    let mut registry = ContentRegistry::default();
    for definition in cards() {
        registry.add_card(definition);
    }
    registry.add_monster(dummy_monster());
    registry.add_orb(asker_orb());
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

fn pick_first(simulator: &Simulator) -> Action {
    simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseCards { cards, .. } if cards.len() == 1))
        .expect("the screen offers a card")
        .clone()
}

fn choosing(simulator: &Simulator) -> bool {
    matches!(simulator.decision(), DecisionContext::ChooseCards { .. })
}

fn block_of(simulator: &Simulator) -> i32 {
    let combat = simulator.state().combat.as_ref().unwrap();
    combat
        .creatures
        .iter()
        .find(|creature| creature.id == combat.player.creature_id)
        .unwrap()
        .block
}

fn hp_of(simulator: &Simulator, creature: u64) -> i32 {
    simulator
        .state()
        .combat
        .as_ref()
        .unwrap()
        .creatures
        .iter()
        .find(|held| held.id == sts2_core::CreatureCombatId::from(creature))
        .map_or(0, |held| held.current_hp)
}

/// A simulator cloned at a suspended boundary and one stepped from the
/// original must walk to the same place, action for action — which is what
/// pins the frames as honest state rather than live borrows.
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

fn answer(simulator: &mut Simulator) {
    let action = pick_first(simulator);
    simulator.step(action).unwrap();
}

#[test]
fn a_repeat_runs_the_rounds_a_screen_deferred() {
    let mut simulator = builder()
        .card(PileName::Hand, 1, fingerprint("CARD.TWO_ASKS"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 3, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    simulator.step(play(&simulator, "CARD.TWO_ASKS")).unwrap();

    assert!(choosing(&simulator), "the first round asks");
    assert_eq!(block_of(&simulator), 0, "and nothing has landed yet");

    assert_clone_resumes_identically(&simulator, 2);
    answer(&mut simulator);
    assert!(choosing(&simulator), "the second round asks in its turn");
    assert_eq!(
        block_of(&simulator),
        7,
        "the first round finished whole first: the answer's five, then the body's two"
    );

    answer(&mut simulator);
    assert!(!choosing(&simulator), "no round is owed past the second");
    assert_eq!(block_of(&simulator), 14);
}

#[test]
fn a_repeat_with_last_still_runs_its_last_arm_after_two_screens() {
    let mut simulator = builder()
        .card(PileName::Hand, 1, fingerprint("CARD.THREE_WITH_LAST"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 3, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    simulator
        .step(play(&simulator, "CARD.THREE_WITH_LAST"))
        .unwrap();

    assert!(choosing(&simulator));
    answer(&mut simulator);
    assert_eq!(block_of(&simulator), 1);
    answer(&mut simulator);
    assert_eq!(block_of(&simulator), 2);
    assert!(
        choosing(&simulator),
        "the third round asks its own question"
    );
    answer(&mut simulator);
    assert_eq!(
        block_of(&simulator),
        12,
        "and the round the frame carried last still ran the last arm"
    );
    assert!(!choosing(&simulator));
}

#[test]
fn a_repeat_per_kill_counts_the_interrupted_rounds_kills_on_resume() {
    let mut simulator = builder()
        .enemy(2, id("MONSTER.DUMMY"), 5, 40, "WAIT")
        .card(PileName::Hand, 1, fingerprint("CARD.ECHO"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 3, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    simulator.step(play(&simulator, "CARD.ECHO")).unwrap();

    assert!(choosing(&simulator), "the first round asks behind its blow");
    assert_eq!(hp_of(&simulator, 1), 30, "the blow landed on both");
    assert_eq!(hp_of(&simulator, 2), 0, "and killed the worn one");
    assert_eq!(block_of(&simulator), 0);

    assert_clone_resumes_identically(&simulator, 2);
    answer(&mut simulator);
    assert!(
        choosing(&simulator),
        "the kill was counted once the answer came back, so a second round runs and asks"
    );
    assert_eq!(block_of(&simulator), 2);
    assert_eq!(hp_of(&simulator, 1), 20, "the second round's blow landed");

    answer(&mut simulator);
    assert!(!choosing(&simulator), "the second round killed nothing");
    assert_eq!(block_of(&simulator), 4);
    assert_eq!(hp_of(&simulator, 1), 20);
}

#[test]
fn a_walk_over_cards_reaches_the_cards_behind_the_screen() {
    let mut simulator = builder()
        .card(PileName::Hand, 1, fingerprint("CARD.SWEEP_DISCARD"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 5, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Discard, 3, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Discard, 4, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    simulator
        .step(play(&simulator, "CARD.SWEEP_DISCARD"))
        .unwrap();

    assert!(choosing(&simulator), "the first card's round asks");
    assert_eq!(block_of(&simulator), 0);

    answer(&mut simulator);
    assert!(choosing(&simulator), "the second card's round asks");
    assert_eq!(block_of(&simulator), 2, "the first round finished whole");

    answer(&mut simulator);
    assert!(!choosing(&simulator));
    assert_eq!(block_of(&simulator), 4);
}

#[test]
fn a_walk_over_orbs_reaches_the_orbs_behind_the_screen() {
    let mut simulator = builder()
        .orb_slots(3)
        .orb(id("ORB.ASKER"))
        .orb(id("ORB.ASKER"))
        .card(PileName::Hand, 1, fingerprint("CARD.STIR_ORBS"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 3, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    simulator.step(play(&simulator, "CARD.STIR_ORBS")).unwrap();

    assert!(choosing(&simulator), "the first orb's round asks");
    assert_eq!(block_of(&simulator), 0);

    answer(&mut simulator);
    assert!(choosing(&simulator), "the second orb's round asks");
    assert_eq!(block_of(&simulator), 1);

    answer(&mut simulator);
    assert!(!choosing(&simulator));
    assert_eq!(block_of(&simulator), 2);
}

#[test]
fn an_orb_passive_pass_fires_the_orbs_behind_the_screen() {
    let mut simulator = builder()
        .orb_slots(3)
        .orb(id("ORB.ASKER"))
        .orb(id("ORB.ASKER"))
        .card(PileName::Hand, 1, fingerprint("CARD.PULSE_ORBS"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 3, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    simulator.step(play(&simulator, "CARD.PULSE_ORBS")).unwrap();

    assert!(choosing(&simulator), "the first passive asks");
    assert_eq!(block_of(&simulator), 0);

    answer(&mut simulator);
    assert!(choosing(&simulator), "the second passive asks");
    assert_eq!(block_of(&simulator), 2);

    answer(&mut simulator);
    assert!(!choosing(&simulator));
    assert_eq!(block_of(&simulator), 4);
}

#[test]
fn a_channel_waits_behind_the_evoke_that_made_room_for_it() {
    let mut simulator = builder()
        .orb_slots(1)
        .orb(id("ORB.ASKER"))
        .card(PileName::Hand, 1, fingerprint("CARD.CHANNEL_ASKER"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 3, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    simulator
        .step(play(&simulator, "CARD.CHANNEL_ASKER"))
        .unwrap();

    assert!(choosing(&simulator), "the make-room evoke asks");
    {
        let combat = simulator.state().combat.as_ref().unwrap();
        assert!(
            combat.player.orbs_channeled_this_combat.is_empty(),
            "the channel has not happened while the evoke's question stands"
        );
    }
    assert_eq!(block_of(&simulator), 0);

    assert_clone_resumes_identically(&simulator, 1);
    answer(&mut simulator);
    assert!(!choosing(&simulator));
    assert_eq!(
        block_of(&simulator),
        5,
        "the evoke finished whole: the answer's four, then its body's one"
    );
    let combat = simulator.state().combat.as_ref().unwrap();
    assert_eq!(
        combat.player.orbs.orbs.len(),
        1,
        "and only then did the new orb join the queue"
    );
    assert_eq!(
        combat.player.orbs_channeled_this_combat,
        vec![id("ORB.ASKER")]
    );
}

#[test]
fn an_auto_play_holds_the_cards_behind_the_screen_one_of_them_opened() {
    let mut simulator = builder()
        .card(PileName::Hand, 1, fingerprint("CARD.MASS_PLAY"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.ASKS"), 0)
        .card(PileName::Hand, 3, fingerprint("CARD.ASKS"), 0)
        .card(PileName::Hand, 4, fingerprint("CARD.FILLER"), 0)
        .card(PileName::Hand, 5, fingerprint("CARD.FILLER"), 0)
        .build(registry())
        .unwrap();
    simulator.step(play(&simulator, "CARD.MASS_PLAY")).unwrap();

    assert!(choosing(&simulator), "the first play asks");
    {
        let combat = simulator.state().combat.as_ref().unwrap();
        assert_eq!(
            combat.player.piles.hand.len(),
            3,
            "the cards behind the screen have not been touched: they wait in hand"
        );
        assert_eq!(
            combat.player.piles.hand[0].fingerprint.model_id,
            id("CARD.ASKS")
        );
    }
    assert_eq!(block_of(&simulator), 0);

    assert_clone_resumes_identically(&simulator, 2);
    answer(&mut simulator);
    assert!(
        choosing(&simulator),
        "the second card plays and asks in its turn"
    );
    assert_eq!(block_of(&simulator), 3, "the first play finished whole");

    answer(&mut simulator);
    assert!(!choosing(&simulator));
    assert_eq!(block_of(&simulator), 6);
    let combat = simulator.state().combat.as_ref().unwrap();
    assert!(combat.player.piles.hand.is_empty(), "both were played");
}

#[test]
fn an_auto_play_of_dupes_copies_the_cards_behind_the_screen_only_when_reached() {
    let mut simulator = builder()
        .card(PileName::Hand, 1, fingerprint("CARD.MASS_DUPE"), 0)
        .card(PileName::Hand, 2, fingerprint("CARD.ASKS"), 0)
        .card(PileName::Hand, 3, fingerprint("CARD.ASKS"), 0)
        .build(registry())
        .unwrap();
    simulator.step(play(&simulator, "CARD.MASS_DUPE")).unwrap();

    assert!(choosing(&simulator), "the first copy's play asks");
    assert_eq!(block_of(&simulator), 0);

    answer(&mut simulator);
    assert!(
        choosing(&simulator),
        "the second is copied and asks in its turn"
    );
    assert_eq!(block_of(&simulator), 3);

    answer(&mut simulator);
    assert!(!choosing(&simulator));
    assert_eq!(block_of(&simulator), 6);
    let combat = simulator.state().combat.as_ref().unwrap();
    assert_eq!(
        combat.player.piles.hand.len(),
        2,
        "the originals never left the hand"
    );
}
