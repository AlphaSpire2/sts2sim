//! The last of the monster models, driven where no recording reaches them.
//!
//! What is here is everything the corpus never fights: the Glory's slimed
//! berserker, the fake merchant an event turns on the player, the three battle
//! friends a dummy event stands up, the three Adversaries no encounter of the
//! pinned build fields, and the sizing dummies the build's own test rooms use.
//! Both sides of every ascension branch these carry are driven too,
//! against a registry standing them up as an ascended run would.

use std::sync::Arc;

use sts2_engine::{
    Action, CombatSide, ContentRegistry, Creature, PileName, ScenarioBuilder, Simulator,
};
use sts2_rng::RunStream;

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().unwrap()
}

fn card(model_id: &str) -> sts2_engine::CardFingerprint {
    let mut fingerprint = sts2_engine::CardFingerprint::base(id(model_id));
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

/// The content a run at `ascension` plays against, which for everything but the
/// monsters that branch is the baseline content unchanged.
fn content_at(ascension: u8) -> Arc<ContentRegistry> {
    let mut registry = (*sts2_content::standard_registry()).clone();
    for definition in sts2_content::monsters_at_ascension(ascension) {
        registry.add_monster(definition);
    }
    Arc::new(registry)
}

/// One monster alone in the encounter named, standing on the move named.
fn fight(encounter: &str, monster: &str, hp: i32, opener: &str, ascension: u8) -> Simulator {
    let mut scenario = ScenarioBuilder::new("LASTMONSTERS", id(encounter))
        .player_hp(500, 500)
        .energy(3, 3)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .relics(&[id("RELIC.BURNING_BLOOD")])
        .enemy(1, id(monster), hp, hp, opener);
    for card_id in 0..5 {
        scenario = scenario.card(PileName::Hand, card_id, card("CARD.STRIKE_IRONCLAD"), 1);
    }
    scenario.build(content_at(ascension)).unwrap()
}

fn enemy(simulator: &Simulator) -> &Creature {
    simulator
        .state()
        .combat
        .as_ref()
        .unwrap()
        .creatures
        .iter()
        .find(|creature| creature.side == CombatSide::Enemy)
        .unwrap()
}

fn player(simulator: &Simulator) -> &Creature {
    simulator
        .state()
        .combat
        .as_ref()
        .unwrap()
        .creatures
        .iter()
        .find(|creature| creature.side == CombatSide::Player)
        .unwrap()
}

fn end_turn(simulator: &mut Simulator) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::EndTurn { .. }))
        .cloned()
        .unwrap();
    simulator.step(action).unwrap();
}

/// What the player lost over one enemy turn.
fn damage_of_one_turn(simulator: &mut Simulator) -> i32 {
    let before = player(simulator).current_hp;
    end_turn(simulator);
    before - player(simulator).current_hp
}

/// How many of one card the fight is holding anywhere: a status card conjured
/// onto the discard pile is drawn and redrawn as the turns go by, so what is
/// checkable a turn later is how many of it the combat holds at all.
fn cards_of(simulator: &Simulator, model_id: &str) -> usize {
    let piles = &simulator.state().combat.as_ref().unwrap().player.piles;
    [
        &piles.draw,
        &piles.hand,
        &piles.discard,
        &piles.exhaust,
        &piles.play,
    ]
    .into_iter()
    .flat_map(|pile| pile.iter())
    .filter(|card| card.fingerprint.model_id == id(model_id))
    .count()
}

/// The berserker walks one round of four: ten Slimed, a fourfold pummelling,
/// a hug that weakens and strengthens, and a smother.
#[test]
fn the_slimed_berserker_walks_its_round_of_four() {
    let mut berserker = fight(
        "ENCOUNTER.SLIMED_BERSERKER_NORMAL",
        "MONSTER.SLIMED_BERSERKER",
        261,
        "VOMIT_ICHOR_MOVE",
        0,
    );
    assert_eq!(
        damage_of_one_turn(&mut berserker),
        0,
        "the vomit hits nobody"
    );
    assert_eq!(
        cards_of(&berserker, "CARD.SLIMED"),
        10,
        "and leaves ten Slimed in the fight"
    );
    assert_eq!(enemy(&berserker).move_state, "FURIOUS_PUMMELING_MOVE");

    assert_eq!(
        damage_of_one_turn(&mut berserker),
        16,
        "the pummelling is four of four"
    );
    assert_eq!(enemy(&berserker).move_state, "LEECHING_HUG_MOVE");

    assert_eq!(damage_of_one_turn(&mut berserker), 0, "the hug hits nobody");
    assert_eq!(
        player(&berserker).power_amount("POWER.WEAK_POWER"),
        3,
        "it leaves three weak"
    );
    assert_eq!(
        enemy(&berserker).power_amount("POWER.STRENGTH_POWER"),
        3,
        "and takes three strength for itself"
    );
    assert_eq!(enemy(&berserker).move_state, "SMOTHER_MOVE");

    assert_eq!(
        damage_of_one_turn(&mut berserker),
        33,
        "thirty smothering, and the three it hugged out of the player"
    );
    assert_eq!(
        enemy(&berserker).move_state,
        "VOMIT_ICHOR_MOVE",
        "and round it goes again"
    );
}

/// Its ascension branches: two hundred and eighty-one health, a pummelling of
/// fives, and a thirty-three smother.
#[test]
fn the_slimed_berserker_carries_its_ascension_branches() {
    let ascended = content_at(9);
    let definition = ascended
        .monster(&id("MONSTER.SLIMED_BERSERKER"))
        .expect("the berserker is content");
    assert_eq!(definition.min_initial_hp, 281);
    assert_eq!(definition.max_initial_hp, 281);

    let mut berserker = fight(
        "ENCOUNTER.SLIMED_BERSERKER_NORMAL",
        "MONSTER.SLIMED_BERSERKER",
        281,
        "FURIOUS_PUMMELING_MOVE",
        9,
    );
    assert_eq!(
        damage_of_one_turn(&mut berserker),
        20,
        "four hits of five at Deadly Enemies"
    );
    let mut smothering = fight(
        "ENCOUNTER.SLIMED_BERSERKER_NORMAL",
        "MONSTER.SLIMED_BERSERKER",
        281,
        "SMOTHER_MOVE",
        9,
    );
    assert_eq!(damage_of_one_turn(&mut smothering), 33);
}

/// The fake merchant walks a swipe, eight coins two at a time, a relic thrown
/// that leaves its target frail, and a rage.
#[test]
fn the_fake_merchant_swipes_spews_throws_and_rages() {
    let merchant = |opener: &str| {
        fight(
            "ENCOUNTER.FAKE_MERCHANT_EVENT_ENCOUNTER",
            "MONSTER.FAKE_MERCHANT_MONSTER",
            165,
            opener,
            0,
        )
    };
    let mut swiping = merchant("SWIPE_MOVE");
    assert_eq!(
        damage_of_one_turn(&mut swiping),
        13,
        "the swipe is thirteen"
    );

    let mut spewing = merchant("SPEW_COINS_MOVE");
    assert_eq!(damage_of_one_turn(&mut spewing), 16, "eight coins of two");

    let mut throwing = merchant("THROW_RELIC_MOVE");
    assert_eq!(damage_of_one_turn(&mut throwing), 9, "the relic is nine");
    assert_eq!(
        player(&throwing).power_amount("POWER.FRAIL_POWER"),
        1,
        "and leaves one frail"
    );
    assert_eq!(
        enemy(&throwing).move_state,
        "SWIPE_MOVE",
        "a throw rolls between the three attacks alone, and the rage is not one"
    );

    let mut raging = merchant("ENRAGE_MOVE");
    assert_eq!(damage_of_one_turn(&mut raging), 0, "the rage hits nobody");
    assert_eq!(
        enemy(&raging).power_amount("POWER.STRENGTH_POWER"),
        2,
        "and leaves two strength standing"
    );
}

/// A throw never rolls the rage, whatever the stream says, because the branch a
/// throw follows up into holds only the three attacks.
#[test]
fn a_thrown_relic_is_never_followed_by_a_rage() {
    for counter in 0..40 {
        let mut scenario = ScenarioBuilder::new(
            "LASTMONSTERS",
            id("ENCOUNTER.FAKE_MERCHANT_EVENT_ENCOUNTER"),
        )
        .player_hp(500, 500)
        .energy(3, 3)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, counter)
        .relics(&[id("RELIC.BURNING_BLOOD")])
        .enemy(
            1,
            id("MONSTER.FAKE_MERCHANT_MONSTER"),
            165,
            165,
            "THROW_RELIC_MOVE",
        );
        for card_id in 0..5 {
            scenario = scenario.card(PileName::Hand, card_id, card("CARD.STRIKE_IRONCLAD"), 1);
        }
        let mut thrown = scenario.build(content_at(0)).unwrap();
        end_turn(&mut thrown);
        assert_ne!(enemy(&thrown).move_state, "ENRAGE_MOVE");
    }
}

/// Its ascension branches: a hundred and seventy-five health, a fifteen swipe,
/// and a ten throw. The coins are two apiece at every level.
#[test]
fn the_fake_merchant_carries_its_ascension_branches() {
    let ascended = content_at(9);
    let definition = ascended
        .monster(&id("MONSTER.FAKE_MERCHANT_MONSTER"))
        .expect("the merchant is content");
    assert_eq!(definition.min_initial_hp, 175);
    assert_eq!(definition.max_initial_hp, 175);

    let mut swiping = fight(
        "ENCOUNTER.FAKE_MERCHANT_EVENT_ENCOUNTER",
        "MONSTER.FAKE_MERCHANT_MONSTER",
        175,
        "SWIPE_MOVE",
        9,
    );
    assert_eq!(damage_of_one_turn(&mut swiping), 15);
    let mut throwing = fight(
        "ENCOUNTER.FAKE_MERCHANT_EVENT_ENCOUNTER",
        "MONSTER.FAKE_MERCHANT_MONSTER",
        175,
        "THROW_RELIC_MOVE",
        9,
    );
    assert_eq!(damage_of_one_turn(&mut throwing), 10);
}

/// A battle friend does nothing at all and walks out of the fight once the
/// three turns its arrival put on it have gone by.
#[test]
fn a_battle_friend_stands_for_three_turns_and_then_leaves() {
    for (model, hp) in [
        ("MONSTER.BATTLE_FRIEND_V1", 75),
        ("MONSTER.BATTLE_FRIEND_V2", 150),
        ("MONSTER.BATTLE_FRIEND_V3", 300),
    ] {
        let registry = sts2_content::standard_registry();
        let definition = registry
            .monster(&id(model))
            .expect("the battle friend is content");
        assert_eq!(
            definition.on_added,
            vec![sts2_engine::Command::Power(sts2_engine::PowerCmd::Apply {
                marks: None,
                power: id("POWER.BATTLEWORN_DUMMY_TIME_LIMIT_POWER"),
                target: sts2_engine::CreatureSelector::Owner,
                applier: None,
                amount: sts2_engine::Amount::Fixed(3),
            })],
            "{model}'s timer has no applier"
        );
        let mut scenario = ScenarioBuilder::new(
            "LASTMONSTERS",
            id("ENCOUNTER.BATTLEWORN_DUMMY_EVENT_ENCOUNTER"),
        )
        .player_hp(500, 500)
        .energy(3, 3)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .relics(&[id("RELIC.BURNING_BLOOD")])
        .enemy(1, id(model), hp, hp, "NOTHING_MOVE")
        .power(
            1,
            id("POWER.BATTLEWORN_DUMMY_TIME_LIMIT_POWER"),
            3,
            sts2_engine::PowerType::Buff,
        );
        for card_id in 0..5 {
            scenario = scenario.card(PileName::Hand, card_id, card("CARD.STRIKE_IRONCLAD"), 1);
        }
        let mut dummy = scenario.build(content_at(0)).unwrap();
        assert_eq!(enemy(&dummy).max_hp, hp, "{model} is {hp} health");

        assert_eq!(damage_of_one_turn(&mut dummy), 0, "{model} never acts");
        assert_eq!(
            enemy(&dummy).power_amount("POWER.BATTLEWORN_DUMMY_TIME_LIMIT_POWER"),
            2
        );
        end_turn(&mut dummy);
        assert_eq!(
            enemy(&dummy).power_amount("POWER.BATTLEWORN_DUMMY_TIME_LIMIT_POWER"),
            1
        );
        // The third turn's end is where the last count would go, and instead of
        // going the dummy walks out.
        end_turn(&mut dummy);
        assert!(
            dummy.state().combat.as_ref().is_none_or(|combat| !combat
                .creatures
                .iter()
                .any(|creature| creature.side == CombatSide::Enemy)),
            "{model} left the fight rather than dying in it"
        );
    }
}

/// The three Adversaries share one machine and differ in their numbers: a blow,
/// a heavier blow, and a barrage of two that leaves each of them stronger by
/// more than the last.
#[test]
fn the_three_adversaries_walk_the_same_round_at_three_sizes() {
    for (model, hp, artifact, first, second, barrage, strength) in [
        ("MONSTER.THE_ADVERSARY_MK_ONE", 100, 0, 12, 15, 8, 2),
        ("MONSTER.THE_ADVERSARY_MK_TWO", 200, 1, 13, 16, 9, 3),
        ("MONSTER.THE_ADVERSARY_MK_THREE", 300, 2, 15, 18, 10, 4),
    ] {
        let definition = sts2_content::standard_registry()
            .monster(&id(model))
            .expect("the adversary is content")
            .clone();
        assert_eq!(definition.min_initial_hp, hp, "{model}");
        assert_eq!(definition.max_initial_hp, hp, "{model}");

        // A scenario places its own creatures, so what an arrival works is read
        // off the model rather than off the fight: the Mk One's own call is for
        // no stacks at all.
        assert_eq!(
            definition.on_added,
            vec![sts2_engine::Command::Power(sts2_engine::PowerCmd::Apply {
                marks: None,
                power: id("POWER.ARTIFACT_POWER"),
                target: sts2_engine::CreatureSelector::Owner,
                applier: Some(sts2_engine::CreatureSelector::Owner),
                amount: sts2_engine::Amount::Fixed(artifact),
            })],
            "{model} stands up behind its own artifact"
        );

        let opener = definition.initial_state.clone();
        let mut adversary = fight(
            "ENCOUNTER.CONSTRUCT_MENAGERIE_NORMAL",
            model,
            hp,
            &opener,
            0,
        );
        assert_eq!(damage_of_one_turn(&mut adversary), first, "{model} first");
        assert_eq!(damage_of_one_turn(&mut adversary), second, "{model} second");
        assert_eq!(
            damage_of_one_turn(&mut adversary),
            barrage * 2,
            "{model} barrage"
        );
        assert_eq!(
            enemy(&adversary).power_amount("POWER.STRENGTH_POWER"),
            strength,
            "{model} strength off the barrage"
        );
        assert_eq!(
            enemy(&adversary).move_state,
            opener,
            "{model} goes back round"
        );
        assert_eq!(
            damage_of_one_turn(&mut adversary),
            first + strength,
            "{model} is heavier for what it took"
        );
    }
}

/// The build's own sizing dummies: a size and a move that does nothing, or a
/// poke worth one.
#[test]
fn the_sizing_dummies_are_a_size_and_nothing_else() {
    for (model, hp) in [
        ("MONSTER.BIG_DUMMY", 9999),
        ("MONSTER.ONE_HP_MONSTER", 1),
        ("MONSTER.TEN_HP_MONSTER", 10),
    ] {
        let mut dummy = fight(
            "ENCOUNTER.CONSTRUCT_MENAGERIE_NORMAL",
            model,
            hp,
            "NOTHING",
            0,
        );
        assert_eq!(enemy(&dummy).max_hp, hp, "{model}");
        assert_eq!(damage_of_one_turn(&mut dummy), 0, "{model} does nothing");
        assert_eq!(
            enemy(&dummy).move_state,
            "NOTHING",
            "{model} goes on doing it"
        );
    }

    let mut poking = fight(
        "ENCOUNTER.CONSTRUCT_MENAGERIE_NORMAL",
        "MONSTER.SINGLE_ATTACK_MOVE_MONSTER",
        999,
        "POKE",
        0,
    );
    assert_eq!(damage_of_one_turn(&mut poking), 1, "one poke of one");

    let mut poking_five = fight(
        "ENCOUNTER.CONSTRUCT_MENAGERIE_NORMAL",
        "MONSTER.MULTI_ATTACK_MOVE_MONSTER",
        999,
        "POKE",
        0,
    );
    assert_eq!(damage_of_one_turn(&mut poking_five), 5, "five pokes of one");
}

/// The medium twig slime, which no recording has ever spawned: it opens on its
/// sticky shot and then rolls between the shot and a pounce that may come round
/// twice before the shot has to.
#[test]
fn the_medium_twig_slime_opens_on_its_sticky_shot() {
    let definition = sts2_content::standard_registry()
        .monster(&id("MONSTER.TWIG_SLIME_M"))
        .expect("the slime is content")
        .clone();
    assert_eq!(definition.initial_state, "STICKY_SHOT_MOVE");
    assert_eq!(
        (definition.min_initial_hp, definition.max_initial_hp),
        (26, 28)
    );

    let mut sticky = fight(
        "ENCOUNTER.SLIMES_NORMAL",
        "MONSTER.TWIG_SLIME_M",
        28,
        "STICKY_SHOT_MOVE",
        0,
    );
    assert_eq!(damage_of_one_turn(&mut sticky), 0, "the shot hits nobody");
    assert_eq!(
        cards_of(&sticky, "CARD.SLIMED"),
        1,
        "and leaves one Slimed behind"
    );

    let mut pouncing = fight(
        "ENCOUNTER.SLIMES_NORMAL",
        "MONSTER.TWIG_SLIME_M",
        28,
        "POKEY_POUNCE_MOVE",
        0,
    );
    assert_eq!(
        damage_of_one_turn(&mut pouncing),
        11,
        "the pounce is eleven"
    );

    // Both moves follow up into the same roll, and the roll only ever settles
    // on one of the two.
    for counter in 0..24 {
        let mut scenario = ScenarioBuilder::new("LASTMONSTERS", id("ENCOUNTER.SLIMES_NORMAL"))
            .player_hp(500, 500)
            .energy(3, 3)
            .turn(1, 1)
            .rng_counter(RunStream::MonsterAi, counter)
            .relics(&[id("RELIC.BURNING_BLOOD")])
            .enemy(1, id("MONSTER.TWIG_SLIME_M"), 28, 28, "STICKY_SHOT_MOVE");
        for card_id in 0..5 {
            scenario = scenario.card(PileName::Hand, card_id, card("CARD.STRIKE_IRONCLAD"), 1);
        }
        let mut slime = scenario.build(content_at(0)).unwrap();
        end_turn(&mut slime);
        assert!(
            ["STICKY_SHOT_MOVE", "POKEY_POUNCE_MOVE"].contains(&enemy(&slime).move_state.as_str())
        );
    }
}
