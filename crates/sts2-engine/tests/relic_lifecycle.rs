//! The rules a relic lives by, whatever relic it is: obtaining, removing,
//! replacing, melting, what a shop charges, and what an event may take back.
//!
//! These are engine rules rather than content, so the models here are invented
//! ones that do nothing but carry the flags the rules read.

use std::sync::Arc;

use sts2_core::ModelId;
use sts2_engine::{
    Action, Amount, CardDefinition, CardRarity, CardType, Command, ContentRegistry, CreatureCmd,
    CreatureSelector, Intent, ModifierHook, MonsterDefinition, MonsterMove, MonsterState,
    MoveIntent, PileName, RelicCmd, RelicDefinition, RelicRarity, ScenarioBuilder, Simulator,
    TargetType, ValueProps,
};

fn id(value: &str) -> ModelId {
    value.parse().expect("static model ID")
}

fn relic(model_id: &str, rarity: RelicRarity) -> RelicDefinition {
    RelicDefinition {
        model_id: id(model_id),
        grants_an_extra_turn: None,
        map_rule: None,
        rarity,
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
        hooks: Vec::new(),
    }
}

/// A relic that heals its owner every time the player's turn opens, so that a
/// test can watch it stop listening.
fn healer(model_id: &str) -> RelicDefinition {
    RelicDefinition {
        hooks: vec![ModifierHook::every_turn(
            sts2_engine::HookEvent::AfterPlayerTurnStart,
            vec![Command::Creature(CreatureCmd::Heal {
                target: CreatureSelector::Owner,
                amount: Amount::Fixed(3),
            })],
        )],
        ..relic(model_id, RelicRarity::Common)
    }
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

fn registry() -> Arc<ContentRegistry> {
    let mut registry = ContentRegistry::default();
    registry.add_monster(MonsterDefinition {
        model_id: id("MONSTER.TEST_IDLER"),
        min_initial_hp: 20,
        max_initial_hp: 20,
        initial_state: "WAIT".to_owned(),
        states: vec![MonsterState::Move(MonsterMove {
            state_id: "WAIT".to_owned(),
            intent: Intent::new("WAIT", vec![MoveIntent::of("unknown")]),
            commands: Vec::new(),
            follow_up: "WAIT".to_owned(),
        })],
        on_added: Vec::new(),
        hooks: Vec::new(),
    });
    registry.add_relic(healer("RELIC.TEST_HEALER"));
    registry.add_relic(relic("RELIC.TEST_REPLACEMENT", RelicRarity::Uncommon));
    registry.add_relic(RelicDefinition {
        is_stackable: true,
        ..relic("RELIC.TEST_STACKABLE", RelicRarity::Common)
    });
    registry.add_relic(RelicDefinition {
        has_upon_pickup_effect: true,
        on_obtained: vec![Command::Creature(CreatureCmd::GainMaxHp {
            target: CreatureSelector::Owner,
            amount: Amount::Fixed(7),
        })],
        ..relic("RELIC.TEST_UPON_PICKUP", RelicRarity::Common)
    });
    registry.add_relic(RelicDefinition {
        spawns_pets: true,
        adds_pet: false,
        rolled_skin: None,
        ..relic("RELIC.TEST_PET", RelicRarity::Uncommon)
    });
    registry.add_relic(RelicDefinition {
        allowed_in_shops: false,
        ..relic("RELIC.TEST_UNSHOPPABLE", RelicRarity::Rare)
    });
    // Being used up is per instance: this one is spent once its own latch is
    // set.
    registry.add_relic(RelicDefinition {
        used_up: Some(sts2_engine::UsedUp::Latched),
        ..relic("RELIC.TEST_USED_UP", RelicRarity::Rare)
    });
    for (model_id, rarity) in [
        ("RELIC.TEST_STARTER", RelicRarity::Starter),
        ("RELIC.TEST_EVENT", RelicRarity::Event),
        ("RELIC.TEST_ANCIENT", RelicRarity::Ancient),
        ("RELIC.TEST_SHOP", RelicRarity::Shop),
        ("RELIC.TEST_RARE", RelicRarity::Rare),
    ] {
        registry.add_relic(relic(model_id, rarity));
    }
    registry.add_card(card(
        "CARD.TEST_OBTAIN",
        vec![Command::Relic(RelicCmd::Obtain(id(
            "RELIC.TEST_REPLACEMENT",
        )))],
    ));
    registry.add_card(card(
        "CARD.TEST_REPLACE",
        vec![Command::Relic(RelicCmd::Replace {
            original: id("RELIC.TEST_HEALER"),
            replacement: id("RELIC.TEST_REPLACEMENT"),
        })],
    ));
    registry.add_card(card(
        "CARD.TEST_MELT",
        vec![Command::Relic(RelicCmd::Melt(id("RELIC.TEST_HEALER")))],
    ));
    registry.add_card(card(
        "CARD.TEST_REMOVE",
        vec![Command::Relic(RelicCmd::Remove(id("RELIC.TEST_HEALER")))],
    ));
    Arc::new(registry)
}

fn fight(relics: &[&str], hand: &[&str]) -> Simulator {
    let held: Vec<ModelId> = relics.iter().map(|model| id(model)).collect();
    let mut scenario = ScenarioBuilder::new("RELICLIFECYCLE", id("ENCOUNTER.TEST"))
        .player_hp(40, 80)
        .energy(9, 9)
        .relics(&held)
        .enemy(1, id("MONSTER.TEST_IDLER"), 20, 20, "WAIT");
    for (index, model_id) in hand.iter().enumerate() {
        let mut fingerprint = sts2_engine::CardFingerprint::base(id(model_id));
        fingerprint.floor_added_to_deck = Some(1);
        scenario = scenario.card(
            PileName::Hand,
            u64::try_from(index).expect("small index"),
            fingerprint,
            0,
        );
    }
    scenario.build(registry()).expect("the scenario builds")
}

fn play(simulator: &mut Simulator, model_id: &str) {
    let wanted = id(model_id);
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::PlayCard { card, .. }
                if card.fingerprint.model_id == wanted)
        })
        .cloned()
        .unwrap_or_else(|| panic!("{model_id} was not playable"));
    simulator.step(action).expect("the play applies");
}

fn held(simulator: &Simulator) -> Vec<String> {
    simulator
        .state()
        .run_player
        .relics
        .iter()
        .map(|relic| relic.model_id.to_string())
        .collect()
}

/// Obtaining a relic puts it at the end of the inventory; replacing one puts
/// the new relic where the relic it replaced stood.
#[test]
fn a_replacement_takes_the_place_of_what_it_replaced() {
    let mut simulator = fight(
        &["RELIC.TEST_STARTER", "RELIC.TEST_HEALER", "RELIC.TEST_SHOP"],
        &["CARD.TEST_OBTAIN"],
    );
    play(&mut simulator, "CARD.TEST_OBTAIN");
    assert_eq!(
        held(&simulator),
        [
            "RELIC.TEST_STARTER",
            "RELIC.TEST_HEALER",
            "RELIC.TEST_SHOP",
            "RELIC.TEST_REPLACEMENT",
        ]
    );

    let mut simulator = fight(
        &["RELIC.TEST_STARTER", "RELIC.TEST_HEALER", "RELIC.TEST_SHOP"],
        &["CARD.TEST_REPLACE"],
    );
    play(&mut simulator, "CARD.TEST_REPLACE");
    assert_eq!(
        held(&simulator),
        [
            "RELIC.TEST_STARTER",
            "RELIC.TEST_REPLACEMENT",
            "RELIC.TEST_SHOP",
        ]
    );
}

/// A removed relic leaves the inventory outright; a melted one stays in it and
/// stops being anything at all.
#[test]
fn a_melted_relic_stays_held_and_stops_listening() {
    let mut simulator = fight(&["RELIC.TEST_HEALER"], &["CARD.TEST_REMOVE"]);
    play(&mut simulator, "CARD.TEST_REMOVE");
    assert_eq!(held(&simulator), [] as [String; 0]);

    let mut simulator = fight(&["RELIC.TEST_HEALER"], &["CARD.TEST_MELT"]);
    let before = simulator.state().combat.as_ref().unwrap().creatures[0].current_hp;
    play(&mut simulator, "CARD.TEST_MELT");
    assert_eq!(held(&simulator), ["RELIC.TEST_HEALER"]);
    assert!(simulator.state().run_player.relics[0].melted);
    // The turn opening would have healed three; a melted relic is not on the
    // listener list at all.
    simulator.step(Action::EndTurn { turn: 1 }).unwrap();
    let after = simulator.state().combat.as_ref().unwrap().creatures[0].current_hp;
    assert_eq!(after, before);
}

/// A relic that still listens proves the same test's other half: the healer
/// heals when it has not been melted.
#[test]
fn a_relic_that_was_not_melted_still_listens() {
    let mut simulator = fight(&["RELIC.TEST_HEALER"], &[]);
    let before = simulator.state().combat.as_ref().unwrap().creatures[0].current_hp;
    simulator.step(Action::EndTurn { turn: 1 }).unwrap();
    let after = simulator.state().combat.as_ref().unwrap().creatures[0].current_hp;
    assert_eq!(after, before + 3);
}

/// What makes a relic tradable: not something that acts on pickup, not a pet
/// bringer, and not one of the three rarities outside the rolled ones.
#[test]
fn what_an_event_may_take_back_is_the_build_s_own_list() {
    let content = registry();
    let fresh = sts2_engine::RelicInstance::new(id("RELIC.TEST_HEALER"));
    let tradable = |model_id: &str| {
        content
            .relic(&id(model_id))
            .expect("the relic is registered")
            .is_tradable(&fresh)
    };
    for model_id in [
        "RELIC.TEST_HEALER",
        "RELIC.TEST_REPLACEMENT",
        "RELIC.TEST_SHOP",
        "RELIC.TEST_RARE",
    ] {
        assert!(tradable(model_id), "{model_id}");
    }
    for model_id in [
        "RELIC.TEST_STARTER",
        "RELIC.TEST_EVENT",
        "RELIC.TEST_ANCIENT",
        "RELIC.TEST_UPON_PICKUP",
        "RELIC.TEST_PET",
    ] {
        assert!(!tradable(model_id), "{model_id}");
    }
    // Melting and being used up are asked of the instance rather than of the
    // model, so the same definition answers both ways.
    let definition = content
        .relic(&id("RELIC.TEST_USED_UP"))
        .expect("the relic is registered");
    let mut instance = sts2_engine::RelicInstance::new(id("RELIC.TEST_USED_UP"));
    assert!(definition.is_tradable(&instance));
    instance.latched = true;
    assert!(definition.is_used_up(&instance));
    assert!(!definition.is_tradable(&instance));
    let mut melted = sts2_engine::RelicInstance::new(id("RELIC.TEST_RARE"));
    melted.melted = true;
    assert!(
        !content
            .relic(&id("RELIC.TEST_RARE"))
            .expect("the relic is registered")
            .is_tradable(&melted)
    );
}

/// What a shop charges for a relic: one price per rarity, and the three a shop
/// never stocks are priced out of reach rather than forbidden.
#[test]
fn a_shop_charges_by_rarity() {
    let content = registry();
    let cost = |model_id: &str| {
        content
            .relic(&id(model_id))
            .expect("the relic is registered")
            .merchant_cost()
    };
    assert_eq!(cost("RELIC.TEST_HEALER"), 175);
    assert_eq!(cost("RELIC.TEST_REPLACEMENT"), 225);
    assert_eq!(cost("RELIC.TEST_RARE"), 275);
    assert_eq!(cost("RELIC.TEST_SHOP"), 200);
    for model_id in [
        "RELIC.TEST_STARTER",
        "RELIC.TEST_EVENT",
        "RELIC.TEST_ANCIENT",
    ] {
        assert_eq!(cost(model_id), 999_999_999, "{model_id}");
    }
    assert!(
        !content
            .relic(&id("RELIC.TEST_UNSHOPPABLE"))
            .expect("the relic is registered")
            .allowed_in_shops
    );
}

/// A relic that does something as it is taken does it inside the obtain, which
/// is what the obtain waiting out the relic's own `on_obtained` comes to.
#[test]
fn what_a_relic_does_as_it_is_taken_happens_inside_the_obtain() {
    let mut registry = ContentRegistry::default();
    registry.add_monster(MonsterDefinition {
        model_id: id("MONSTER.TEST_IDLER"),
        min_initial_hp: 20,
        max_initial_hp: 20,
        initial_state: "WAIT".to_owned(),
        states: vec![MonsterState::Move(MonsterMove {
            state_id: "WAIT".to_owned(),
            intent: Intent::new("WAIT", vec![MoveIntent::of("unknown")]),
            commands: Vec::new(),
            follow_up: "WAIT".to_owned(),
        })],
        on_added: Vec::new(),
        hooks: Vec::new(),
    });
    registry.add_relic(RelicDefinition {
        has_upon_pickup_effect: true,
        on_obtained: vec![Command::Creature(CreatureCmd::GainMaxHp {
            target: CreatureSelector::Owner,
            amount: Amount::Fixed(7),
        })],
        ..relic("RELIC.TEST_UPON_PICKUP", RelicRarity::Common)
    });
    registry.add_card(card(
        "CARD.TEST_TAKE",
        vec![
            Command::Relic(RelicCmd::Obtain(id("RELIC.TEST_UPON_PICKUP"))),
            Command::Creature(CreatureCmd::Damage {
                target: CreatureSelector::Owner,
                amount: Amount::Fixed(1),
                props: ValueProps::UNBLOCKABLE,
            }),
        ],
    ));
    let mut fingerprint = sts2_engine::CardFingerprint::base(id("CARD.TEST_TAKE"));
    fingerprint.floor_added_to_deck = Some(1);
    let mut simulator = ScenarioBuilder::new("RELICOBTAIN", id("ENCOUNTER.TEST"))
        .player_hp(40, 80)
        .energy(9, 9)
        .enemy(1, id("MONSTER.TEST_IDLER"), 20, 20, "WAIT")
        .card(PileName::Hand, 0, fingerprint, 0)
        .build(Arc::new(registry))
        .expect("the scenario builds");
    play(&mut simulator, "CARD.TEST_TAKE");
    // Seven more maximum, seven healed with it, and then the card's own point.
    assert_eq!(simulator.state().run_player.max_hp, 87);
    assert_eq!(
        simulator.state().combat.as_ref().unwrap().creatures[0].current_hp,
        46
    );
}
