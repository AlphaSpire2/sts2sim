//! A combat setup says a fight in the game's own terms and the engine stands
//! it up through its own combat start: a live fight projects to a setup that
//! stands back up as the same fight, from a run it is entered from or from
//! the seed alone; nothing the setup owns is guessed — an unknown model, a
//! ledger count left unsaid, a wrong character or ascension, a run standing
//! inside a fight are refused with the field named — and every relic that
//! saves a count says how the count carries.

use sts2_core::UnlockPresetManifest;
use sts2_engine::{
    Action, CombatSetup, CombatSide, ConformanceOrigin, CounterKind, ErrorCode, SetupCard,
    SetupRelic, SetupRoom, Simulator,
};

const SEED: &str = "NLD6VZXP94";

fn fresh_run() -> Simulator {
    let preset = UnlockPresetManifest::pinned().unwrap();
    let character = "CHARACTER.IRONCLAD".parse().unwrap();
    sts2_content::standard_run_on_preset_at(SEED, &character, &preset, 0).unwrap()
}

/// The first legal action, preferring a non-empty card pick so a skippable
/// reward cannot re-offer itself.
fn next_action(simulator: &Simulator) -> Action {
    simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseCards { cards, .. } if !cards.is_empty()))
        .or_else(|| simulator.legal_actions().first())
        .cloned()
        .expect("a live run offers an action")
}

fn in_live_combat(simulator: &Simulator) -> bool {
    simulator
        .state()
        .combat
        .as_ref()
        .is_some_and(|combat| combat.in_progress && !combat.has_ended)
}

/// The seeded run walked to its first fight: the decision that stepped into
/// it, and the run as it stood just before that step.
fn first_fight() -> (Simulator, Simulator) {
    let mut simulator = fresh_run();
    for _ in 0..200 {
        let before = simulator.clone();
        simulator.step_quietly(&next_action(&simulator)).unwrap();
        if in_live_combat(&simulator) {
            return (before, simulator);
        }
    }
    panic!("the seeded walk never entered a fight");
}

fn hand_size(simulator: &Simulator) -> usize {
    simulator
        .state()
        .combat
        .as_ref()
        .unwrap()
        .player
        .piles
        .hand
        .len()
}

/// The enemies standing in a fight, as (model, max hp) in spawn order.
fn enemies(simulator: &Simulator) -> Vec<(String, i32)> {
    simulator
        .state()
        .combat
        .as_ref()
        .unwrap()
        .creatures
        .iter()
        .filter(|creature| creature.side == CombatSide::Enemy)
        .map(|creature| (creature.model_id.to_string(), creature.max_hp))
        .collect()
}

/// A setup with its provenance dropped, for comparing what the engine reads.
fn without_provenance(mut setup: CombatSetup) -> CombatSetup {
    setup.provenance = None;
    setup
}

#[test]
fn a_live_fight_projects_to_a_setup_that_stands_back_up_from_the_run() {
    let (before, fight) = first_fight();
    assert!(
        before.combat_setup().is_none(),
        "a run outside a fight has no fight to say"
    );
    let setup = fight.combat_setup().expect("a live fight projects");
    assert_eq!(setup.room, SetupRoom::Monster);
    assert_eq!(setup.act, 0);
    assert_eq!(setup.character.to_string(), "CHARACTER.IRONCLAD");
    assert_eq!(setup.deck.len(), fight.state().run_player.deck.len());
    assert_eq!(setup.hp, fight.state().run_player.current_hp);

    let rebuilt = before.enter_combat_setup(&setup).unwrap();
    assert!(in_live_combat(&rebuilt));
    assert!(
        !rebuilt.legal_actions().is_empty(),
        "the fight stands at a decision"
    );
    assert_eq!(rebuilt.state().origin, ConformanceOrigin::Analysis);
    // Round trip: what the rebuilt fight says about itself is the setup.
    assert_eq!(
        without_provenance(rebuilt.combat_setup().unwrap()),
        without_provenance(setup.clone())
    );
    // Nothing skipped: the same loadout walked in, the same enemies rolled
    // the same health off the same seed and floor, the same hand was drawn.
    let (original, copy) = (fight.state(), rebuilt.state());
    assert_eq!(copy.run_player.current_hp, original.run_player.current_hp);
    assert_eq!(copy.run_player.max_hp, original.run_player.max_hp);
    assert_eq!(copy.run_player.gold, original.run_player.gold);
    assert_eq!(copy.run_player.max_energy, original.run_player.max_energy);
    assert_eq!(copy.run_player.potions, original.run_player.potions);
    let relics = |simulator: &Simulator| -> Vec<(String, i32)> {
        simulator
            .state()
            .run_player
            .relics
            .iter()
            .map(|relic| (relic.model_id.to_string(), relic.counter))
            .collect()
    };
    assert_eq!(relics(&rebuilt), relics(&fight));
    let deck = |simulator: &Simulator| -> Vec<(String, u8)> {
        simulator
            .state()
            .run_player
            .deck
            .iter()
            .map(|card| (card.model_id.to_string(), card.upgrade_level))
            .collect()
    };
    assert_eq!(deck(&rebuilt), deck(&fight));
    assert_eq!(enemies(&rebuilt), enemies(&fight));
    assert_eq!(hand_size(&rebuilt), hand_size(&fight));
    assert_eq!(
        copy.run.as_ref().unwrap().floor,
        original.run.as_ref().unwrap().floor
    );
    assert_eq!(
        copy.run.as_ref().unwrap().standing_room_type(),
        original.run.as_ref().unwrap().standing_room_type()
    );
}

#[test]
fn a_setup_stands_up_from_the_seed_alone() {
    let (_, fight) = first_fight();
    let setup = fight.combat_setup().unwrap();
    let rebuilt = sts2_content::combat_from_setup(&setup, SEED).unwrap();
    assert!(in_live_combat(&rebuilt));
    assert_eq!(
        without_provenance(rebuilt.combat_setup().unwrap()),
        without_provenance(setup)
    );
    // The run seed is what the enemies roll their health from, so the fight
    // stood up on the run's own seed meets the numbers the run met.
    assert_eq!(enemies(&rebuilt), enemies(&fight));
    assert_eq!(hand_size(&rebuilt), hand_size(&fight));
    // Another seed is another deal — the encounter rolls what it spawns and
    // how much health it has off the seed — but the same loadout through the
    // same door.
    let other =
        sts2_content::combat_from_setup(&fight.combat_setup().unwrap(), "17A2FELJYC").unwrap();
    assert!(in_live_combat(&other));
    assert_eq!(
        without_provenance(other.combat_setup().unwrap()),
        without_provenance(fight.combat_setup().unwrap())
    );
    assert!(!enemies(&other).is_empty());
}

#[test]
fn the_setup_names_the_door_and_the_loadout_and_the_engine_derives_the_rest() {
    let (before, fight) = first_fight();
    let mut setup = fight.combat_setup().unwrap();
    // An act boss, at the act's last floor, with a fuller belt and a bigger
    // deck than the run has: the engine stands it up as an elite/boss room
    // stands up, off the setup alone.
    let boss = before.state().run.as_ref().unwrap().act().boss.clone();
    setup.encounter = boss.clone();
    setup.room = SetupRoom::Boss;
    setup.floor = 16;
    setup.hp = 40;
    setup.gold = 250;
    setup.max_energy = None;
    setup.orb_slots = None;
    setup.potions = vec![Some("POTION.FIRE_POTION".parse().unwrap()), None, None];
    setup.deck.push(SetupCard {
        id: "CARD.BLUDGEON".parse().unwrap(),
        upgrade: 1,
        enchantment: None,
        properties: std::collections::BTreeMap::default(),
    });
    setup.relics.push(SetupRelic {
        id: "RELIC.PEN_NIB".parse().unwrap(),
        counter: None,
    });
    let rebuilt = before.enter_combat_setup(&setup).unwrap();
    let state = rebuilt.state();
    let combat = state.combat.as_ref().unwrap();
    assert_eq!(combat.encounter_model, boss);
    assert_eq!(
        state.run.as_ref().unwrap().standing_room_type(),
        Some(sts2_engine::MapPointType::Boss)
    );
    assert_eq!(state.run.as_ref().unwrap().floor, 16);
    assert_eq!(
        state.run_player.max_energy, 3,
        "an unsaid energy is the character's"
    );
    assert_eq!(state.run_player.potions.len(), 3);
    assert!(
        state
            .run_player
            .deck
            .iter()
            .any(|card| card.model_id.to_string() == "CARD.BLUDGEON" && card.upgrade_level == 1)
    );
    let nib = state
        .run_player
        .relics
        .iter()
        .find(|relic| relic.model_id.to_string() == "RELIC.PEN_NIB")
        .unwrap();
    assert!(
        (0..10).contains(&nib.counter),
        "an unsaid cycle count is rolled below the wrap: {}",
        nib.counter
    );
    // The player's creature carries the setup's health into the fight.
    let player = combat
        .creatures
        .iter()
        .find(|creature| creature.id == combat.player.creature_id)
        .unwrap();
    assert_eq!(player.current_hp, 40);
    assert!(!enemies(&rebuilt).is_empty());
    // The projection reads the roll back as a said count.
    assert_eq!(
        rebuilt
            .combat_setup()
            .unwrap()
            .relics
            .last()
            .unwrap()
            .counter,
        Some(nib.counter)
    );
}

#[test]
fn a_setup_the_content_cannot_honor_is_refused_with_the_field_named() {
    let (before, fight) = first_fight();
    let setup = fight.combat_setup().unwrap();
    let refused = |setup: &CombatSetup| before.enter_combat_setup(setup).unwrap_err();

    let mut unknown_card = setup.clone();
    unknown_card.deck.push(SetupCard {
        id: "CARD.NOT_A_CARD".parse().unwrap(),
        upgrade: 0,
        enchantment: None,
        properties: std::collections::BTreeMap::default(),
    });
    let error = refused(&unknown_card);
    assert_eq!(error.code, ErrorCode::UnsupportedContent);
    assert_eq!(error.details["model_id"], "CARD.NOT_A_CARD");

    let mut unknown_encounter = setup.clone();
    unknown_encounter.encounter = "ENCOUNTER.NOT_ONE".parse().unwrap();
    assert_eq!(
        refused(&unknown_encounter).code,
        ErrorCode::UnsupportedContent
    );

    let mut ledger_unsaid = setup.clone();
    ledger_unsaid.relics.push(SetupRelic {
        id: "RELIC.GIRYA".parse().unwrap(),
        counter: None,
    });
    let error = refused(&ledger_unsaid);
    assert_eq!(error.code, ErrorCode::InvalidConfig);
    assert!(
        error.details["combat_setup"]
            .as_str()
            .unwrap()
            .contains("GIRYA")
    );

    let mut ledger_said = setup.clone();
    ledger_said.relics.push(SetupRelic {
        id: "RELIC.GIRYA".parse().unwrap(),
        counter: Some(2),
    });
    let rebuilt = before.enter_combat_setup(&ledger_said).unwrap();
    assert_eq!(rebuilt.state().run_player.relics.last().unwrap().counter, 2);

    let mut wrong_ascension = setup.clone();
    wrong_ascension.ascension = 10;
    assert_eq!(refused(&wrong_ascension).code, ErrorCode::InvalidConfig);

    let mut wrong_character = setup.clone();
    wrong_character.character = "CHARACTER.SILENT".parse().unwrap();
    assert_eq!(refused(&wrong_character).code, ErrorCode::InvalidConfig);

    let mut wrong_build = setup.clone();
    wrong_build.compatibility_id = "sts2-v0.0.0-00000000-models-0".to_owned();
    assert_eq!(refused(&wrong_build).code, ErrorCode::UnsupportedBuild);

    let mut over_hp = setup.clone();
    over_hp.hp = over_hp.max_hp + 1;
    assert_eq!(refused(&over_hp).code, ErrorCode::InvalidConfig);

    // A run standing inside a fight is not a place to enter another from.
    assert_eq!(
        fight.enter_combat_setup(&setup).unwrap_err().code,
        ErrorCode::InvalidConfig
    );
    // The refusals moved nothing: the run still stands where it stood.
    assert!(before.combat_setup().is_none());
    assert_eq!(before.state().run.as_ref().unwrap().floor, 1);
}

#[test]
fn the_document_reads_unsaid_fields_as_unsaid() {
    let text = r#"{
      "setup_version": 1,
      "compatibility_id": "sts2-v0.107.1-59260271-models-3954186980",
      "character": "CHARACTER.IRONCLAD",
      "ascension": 0,
      "act": 0,
      "floor": 3,
      "encounter": "ENCOUNTER.SHRINKER_BEETLE",
      "room": "monster",
      "hp": 60, "max_hp": 80, "gold": 99,
      "max_energy": null,
      "deck": [
        {"id": "CARD.STRIKE_IRONCLAD"},
        {"id": "CARD.BASH", "upgrade": 1}
      ],
      "relics": [
        {"id": "RELIC.BURNING_BLOOD"},
        {"id": "RELIC.PEN_NIB", "counter": null},
        {"id": "RELIC.GIRYA", "counter": 2}
      ],
      "potions": [null, null, null]
    }"#;
    let setup: CombatSetup = serde_json::from_str(text).unwrap();
    assert_eq!(setup.max_energy, None);
    assert_eq!(setup.orb_slots, None);
    assert_eq!(setup.deck[0].upgrade, 0);
    assert_eq!(setup.deck[1].upgrade, 1);
    assert_eq!(setup.relics[0].counter, None);
    assert_eq!(setup.relics[1].counter, None);
    assert_eq!(setup.relics[2].counter, Some(2));
    assert!(setup.provenance.is_none());
    // Written back, an unsaid field stays unsaid rather than becoming a null
    // the next reader has to interpret.
    let written = serde_json::to_string(&setup).unwrap();
    assert!(!written.contains("\"max_energy\""));
    assert!(!written.contains("\"enchantment\""));
    let again: CombatSetup = serde_json::from_str(&written).unwrap();
    assert_eq!(again, setup);
}

#[test]
fn every_relic_that_saves_a_count_says_how_it_carries() {
    let mut unclassified = Vec::new();
    for character in [
        "CHARACTER.IRONCLAD",
        "CHARACTER.SILENT",
        "CHARACTER.DEFECT",
        "CHARACTER.NECROBINDER",
        "CHARACTER.REGENT",
    ] {
        let registry = sts2_content::registry_at(&character.parse().unwrap(), 0);
        for model_id in registry.registered_model_ids() {
            let Some(relic) = registry.relic(&model_id) else {
                continue;
            };
            match (&relic.counter_property, relic.counter_kind) {
                (Some(_), None) => unclassified.push(model_id.to_string()),
                (None, Some(_)) => panic!("{model_id} classifies a count it does not save"),
                (Some(_), Some(CounterKind::Cycle { wraps_at })) if wraps_at < 1 => {
                    panic!("{model_id} wraps at {wraps_at}, which no count can stand below")
                }
                _ => {}
            }
        }
    }
    unclassified.sort();
    unclassified.dedup();
    assert!(
        unclassified.is_empty(),
        "unclassified saved counts: {unclassified:?}"
    );
}
