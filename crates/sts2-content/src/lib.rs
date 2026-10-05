//! The content of the pinned build: cards, enchantments, relics, potions,
//! powers, orbs, monsters, encounters, events, the merchant, Neow, the pools,
//! the acts and their maps, and run generation.

mod acts;
mod card_pool;
mod cards;
mod defect_cards;
mod enchantments;
mod encounters;
mod events;
mod map;
mod merchant;
mod monsters;
mod necrobinder_cards;
mod neow;
mod orbs;
mod potions;
mod powers;
mod regent_cards;
mod relic_pool;
mod relics;
mod rewards;
mod run_generation;
mod seeded_run;
mod shared_cards;
mod silent_cards;

use std::sync::Arc;

use sts2_core::ModelId;
use sts2_engine::{
    CardFingerprint, ContentRegistry, EngineError, PileName, RewardFingerprint, ScenarioBuilder,
    Simulator,
};
use sts2_rng::RunStream;

pub const FIXTURE_SEED: &str = "N3Z9EE2CXB";

pub use acts::{
    ACT_INDEX_COUNT, ActSelection, RANDOM_CHARACTER, pinned_act_selection, select_acts,
};
pub use encounters::{
    byrdonis_elite, flyconid_normal, nibbits_weak, shrinker_beetle_weak, slimes_weak,
};
pub use map::{golden_path_act_map, spoils_act_map, standard_act_map};
pub use monsters::monsters_at_ascension;
pub use neow::{standard_neow_options, standard_neow_page};
pub use run_generation::{
    UpFrontGeneration, act_event_ids, act_room_pools_for, implemented_act_ids,
    standard_act_room_pools,
};
pub use seeded_run::{
    combat_from_setup, combat_from_setup_with, run_on_preset_at, standard_ironclad_run,
    standard_ironclad_run_at, standard_ironclad_run_on_preset, standard_ironclad_run_with,
    standard_run_on_preset, standard_run_on_preset_at,
};
pub use sts2_engine::RelicGrabBagState;

fn id(value: &str) -> ModelId {
    value.parse().expect("content model ID is static and valid")
}

/// The content a run played as the Ironclad at the baseline ascension is
/// generated and played against.
#[must_use]
pub fn standard_registry() -> Arc<ContentRegistry> {
    registry_for(&id("CHARACTER.IRONCLAD"))
}

/// The content a baseline-ascension run played by one character sees.
#[must_use]
pub fn registry_for(character: &ModelId) -> Arc<ContentRegistry> {
    registry_at(character, sts2_engine::BASELINE_ASCENSION)
}

/// The exact patch-pinned content, as a run played by one character at one
/// ascension sees it.
///
/// Almost everything here is the same whoever is playing: there is one body of
/// content, and a card, a relic, a power or a monster is the same in every run.
/// What is not is anything that comes off the walking player's character — the
/// reward pools, the potion options, the merchant's table, and the card lists
/// five events lay out. Those are keyed by character where the build keys them
/// by character, and the events, whose pages are built out of the walking
/// player's own pool, are built here for the character named.
///
/// The other input is the run's ascension. Ascension effects belong to the run
/// rather than to the content, so the two content families whose definitions
/// branch on a level — the monsters' stats and moves, and the merchant's
/// removal price — are built at the level the run is played at rather than at
/// the baseline and patched afterwards.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one line per model this baseline registers"
)]
pub fn registry_at(character: &ModelId, ascension: u8) -> Arc<ContentRegistry> {
    let mut content = ContentRegistry::default();
    let mut every_card: Vec<(sts2_engine::CardDefinition, bool)> = cards::ironclad_cards()
        .into_iter()
        .map(|definition| (definition, true))
        .collect();
    every_card.extend(
        shared_cards::shared_cards()
            .into_iter()
            .map(|definition| (definition, false)),
    );
    // The Silent's transcribed cards belong to the Silent's pool, not to the
    // run's own character pool.
    every_card.extend(
        silent_cards::silent_cards()
            .into_iter()
            .map(|definition| (definition, false)),
    );
    // The Necrobinder's, likewise: they belong to her pool rather than to the
    // run's own character pool.
    every_card.extend(
        necrobinder_cards::necrobinder_cards()
            .into_iter()
            .map(|definition| (definition, false)),
    );
    // And the Regent's.
    every_card.extend(
        regent_cards::regent_cards()
            .into_iter()
            .map(|definition| (definition, false)),
    );
    // And the Defect's.
    every_card.extend(
        defect_cards::defect_cards()
            .into_iter()
            .map(|definition| (definition, false)),
    );
    for (mut definition, is_the_characters) in every_card {
        // A pool is what says how rare the card in it is. A card in no pool is
        // one a character starts with, one an enemy forces in, or one that
        // carries a rarity of its own — an ancient, a token, a quest card — and
        // says so where it is written.
        definition.gains_block = cards::gains_block(&definition.model_id.to_string());
        if let Some(rarity) = card_pool::pool_rarity(&definition.model_id.to_string()) {
            definition.rarity = rarity;
        } else if definition.rarity == sts2_engine::CardRarity::Basic {
            if definition.card_type == sts2_engine::CardType::Status {
                definition.rarity = sts2_engine::CardRarity::Status;
            } else if definition.card_type == sts2_engine::CardType::Curse {
                definition.rarity = sts2_engine::CardRarity::Curse;
            }
        }
        // Transforming a card: a quest card, an event card, an ancient, and a
        // token all roll their replacement against the colorless pool;
        // everything else rolls against the pool it stands in.
        definition.transform_pool = if definition.card_type == sts2_engine::CardType::Quest
            || matches!(
                definition.rarity,
                sts2_engine::CardRarity::Event
                    | sts2_engine::CardRarity::Ancient
                    | sts2_engine::CardRarity::Token
            ) {
            Some(id("CARD_POOL.COLORLESS_CARD_POOL"))
        } else {
            card_pool::own_pool(&definition.model_id.to_string(), is_the_characters).map(id)
        };
        // The pool the card displays as, which is its own pool but for the ten
        // event cards that show a colour instead. Only whether that pool is
        // colourless is ever read off it.
        definition.visual_pool =
            card_pool::visual_pool(&definition.model_id.to_string(), is_the_characters).map(id);
        content.add_card(definition);
    }

    for definition in powers::ironclad_powers() {
        content.add_power(definition);
    }
    for definition in powers::shared_powers() {
        content.add_power(definition);
    }
    for definition in powers::silent_powers() {
        content.add_power(definition);
    }
    for definition in powers::necrobinder_powers() {
        content.add_power(definition);
    }
    for definition in powers::defect_powers() {
        content.add_power(definition);
    }
    for definition in powers::regent_powers() {
        content.add_power(definition);
    }
    for definition in powers::underdocks_powers() {
        content.add_power(definition);
    }
    for definition in powers::colorless_powers() {
        content.add_power(definition);
    }
    for definition in powers::item_powers() {
        content.add_power(definition);
    }

    for definition in relics::silent_relics() {
        content.add_relic(definition);
    }
    for definition in relics::necrobinder_relics() {
        content.add_relic(definition);
    }
    for definition in relics::regent_relics() {
        content.add_relic(definition);
    }
    for definition in relics::defect_relics() {
        content.add_relic(definition);
    }
    for definition in relics::ironclad_relics() {
        content.add_relic(definition);
    }
    for definition in relics::character_relics(character) {
        content.add_relic(definition);
    }

    for definition in enchantments::all_enchantments() {
        content.add_enchantment(definition);
    }

    for definition in orbs::orbs() {
        content.add_orb(definition);
    }

    for definition in potions::all_potions() {
        content.add_potion(definition);
    }

    // The registry is built at the run's own ascension, because that is what
    // decides which side of a branch a monster's numbers take.
    for definition in monsters::monsters_at_ascension(ascension) {
        content.add_monster(definition);
    }

    for definition in [
        encounters::seapunk_weak(),
        encounters::fossil_stalker_normal(),
        encounters::skulking_colony_elite(),
        encounters::punch_construct_normal(),
        encounters::terror_eel_elite(),
        encounters::construct_menagerie_normal(),
        encounters::mecha_knight_elite(),
        encounters::shrinker_beetle_weak(),
        encounters::slimes_weak(),
        encounters::nibbits_weak(),
        encounters::flyconid_normal(),
        encounters::byrdonis_elite(),
        encounters::inklets_normal(),
        encounters::cubex_construct_normal(),
        encounters::mawler_normal(),
        encounters::slithering_strangler_normal(),
        encounters::phrog_parasite_elite(),
        encounters::ovicopter_normal(),
        encounters::the_insatiable_boss(),
        encounters::the_lost_and_forgotten_normal(),
        encounters::axebots_normal(),
        encounters::fabricator_normal(),
        encounters::test_subject_boss(),
        encounters::dense_vegetation_event_encounter(),
        encounters::fogmog_normal(),
        encounters::nibbits_normal(),
        encounters::the_kin_boss(),
        encounters::thieving_hopper_weak(),
        encounters::slumbering_beetle_normal(),
        encounters::exoskeletons_weak(),
        encounters::hunter_killer_normal(),
        encounters::chompers_normal(),
        encounters::bowlbugs_normal(),
        encounters::bowlbugs_weak(),
        encounters::the_obscura_normal(),
        encounters::snapping_jaxfruit_normal(),
        encounters::vine_shambler_normal(),
        encounters::vantom_boss(),
        encounters::louse_progenitor_normal(),
        encounters::infested_prisms_elite(),
        encounters::knowledge_demon_boss(),
        encounters::turret_operator_weak(),
        encounters::mytes_normal(),
        encounters::kaiser_crab_boss(),
        encounters::fuzzy_wurm_crawler_weak(),
        encounters::slimes_normal(),
        encounters::overgrowth_crawlers(),
        encounters::bygone_effigy_elite(),
        encounters::ceremonial_beast_boss(),
        encounters::tunneler_weak(),
        encounters::exoskeletons_normal(),
        encounters::spiny_toad_normal(),
        encounters::frog_knight_normal(),
        encounters::knights_elite(),
        encounters::mysterious_knight_event_encounter(),
        encounters::entomancer_elite(),
        encounters::globe_head_normal(),
        encounters::aeonglass_boss(),
        encounters::toadpoles_weak(),
        encounters::corpse_slugs_weak(),
        encounters::sludge_spinner_weak(),
        encounters::cultists_normal(),
        encounters::phantasmal_gardeners_elite(),
        encounters::living_fog_normal(),
        encounters::haunted_ship_normal(),
        encounters::seapunk_normal(),
        encounters::sewer_clam_normal(),
        encounters::gremlin_merc_normal(),
        encounters::soul_fysh_boss(),
        encounters::waterfall_giant_boss(),
        encounters::scrolls_of_biting_normal(),
        encounters::owl_magistrate_normal(),
        encounters::soul_nexus_elite(),
        encounters::queen_boss(),
        encounters::devoted_sculptor_weak(),
        encounters::scrolls_of_biting_weak(),
        encounters::decimillipede_elite(),
        encounters::ruby_raiders_normal(),
        encounters::slimed_berserker_normal(),
        encounters::tunneler_normal(),
        encounters::corpse_slugs_normal(),
        encounters::two_tailed_rats_normal(),
        encounters::lagavulin_matriarch_boss(),
        encounters::fake_merchant_event_encounter(),
        encounters::punch_off_event_encounter(),
        encounters::battleworn_dummy_event_encounter(),
    ] {
        content.add_encounter(definition);
    }

    let transcribed_events = vec![
        events::darv(),
        events::doors_of_light_and_dark(),
        events::tea_master(),
        events::whispering_hollow(),
        events::slippery_bridge(),
        events::crystal_sphere(),
        events::spirit_grafter(),
        events::orobas(),
        events::vakuu(),
        events::ranwid_the_elder(),
        events::stone_of_all_time(),
        events::the_future_of_potions(rewards::pool_cards(character)),
        events::this_or_that(),
        events::jungle_maze_adventure(),
        events::sunken_statue(),
        events::pael(),
        events::infested_automaton(&rewards::pool_cards(character)),
        events::zen_weaver(),
        events::colossal_flower(),
        events::nonupeipe(),
        events::round_tea_party(),
        events::room_full_of_cheese(rewards::common_cards(character)),
        events::tablet_of_truth(),
        events::fake_merchant(),
        events::trial(&rewards::reward_spec(character)),
        events::reflections(),
        events::aroma_of_chaos(),
        events::sapphire_seed(),
        events::potion_courier(potions::uncommon_generation_potions(character)),
        events::tezcatara(),
        events::colorful_philosophers(character),
        events::amalgamator(),
        events::the_lantern_key(),
        events::war_historian_repy(),
        events::spiraling_whirlpool(),
        events::symbiote(),
        events::dense_vegetation(),
        events::brain_leech(),
        events::self_help_book(),
        events::lost_wisp(),
        events::trash_heap(),
        events::the_legends_were_true(
            potions::character_potion_pool(character)
                .into_iter()
                .chain(potions::shared_potion_pool())
                .collect(),
        ),
        events::hungry_for_mushrooms(),
        events::bugslayer(),
        events::byrdonis_nest(),
        events::wellspring(
            potions::character_potion_pool(character)
                .into_iter()
                .chain(potions::shared_potion_pool())
                .collect(),
        ),
        events::drowning_beacon(),
        events::sunken_treasury(),
        events::morphic_grove(),
        events::unrest_site(),
        events::field_of_man_sized_holes(),
        events::grave_of_the_forgotten(),
        events::wood_carvings(),
        events::waterlogged_scriptorium(),
        events::luminous_choir(),
        events::abyssal_baths(),
        events::doll_room(),
        events::relic_trader(),
        events::welcome_to_wongos(),
        events::punch_off(),
        events::tanx(),
        events::battleworn_dummy(
            potions::character_potion_pool(character)
                .into_iter()
                .chain(potions::shared_potion_pool())
                .collect(),
        ),
        events::tinker_time(),
        events::endless_conveyor(
            potions::character_potion_pool(character)
                .into_iter()
                .chain(potions::shared_potion_pool())
                .collect(),
        ),
    ];
    for definition in transcribed_events {
        content.add_event(definition);
    }

    // The pools, once every model they name is registered. Each one is the
    // membership its own model declares, and every consumer — a reward, a
    // merchant, a transform, a grab bag, a potion roll — reads it back out of
    // the registry rather than out of a table of its own. A run only ever rolls
    // its own colour, but an event that offers another colour's cards, and a
    // relic that adds a pool to an offer, both read one that is not the walking
    // player's.
    card_pool::register_card_pools(&mut content);
    potions::register_potion_pools(&mut content);
    relic_pool::register_relic_pools(&mut content);

    for named in rewards::IMPLEMENTED_CHARACTERS {
        let named = id(named);
        // A room's card offers are built from the walking player's own
        // character pool, and a merchant stocks that pool beside the player's
        // potion options.
        content.set_character_reward_spec(named.clone(), rewards::reward_spec(&named));
        content.set_merchant(
            named.clone(),
            merchant::standard_merchant(&named, ascension),
        );
    }

    // The two maps the build lays out from scratch rather than by bending the
    // act's own. A quest card of the deck and one ancient relic each replace an
    // act's map with one of them.
    content.set_special_maps(sts2_engine::SpecialMapBuilders {
        spoils: map::spoils_act_map,
        golden_path: map::golden_path_act_map,
    });

    Arc::new(content)
}

/// Every model this crate implements, sorted: what the content registry holds,
/// plus the act models whose room tables live outside it.
///
/// Derived from the definitions rather than written down beside them, so the
/// list cannot drift.
#[must_use]
pub fn registered_model_ids() -> Vec<ModelId> {
    let mut ids = standard_registry().registered_model_ids();
    ids.extend(implemented_act_ids());
    ids.sort();
    ids.dedup();
    ids
}

fn card(model_id: &str) -> CardFingerprint {
    let mut fingerprint = CardFingerprint::base(id(model_id));
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

fn reward_card(model_id: &str) -> CardFingerprint {
    CardFingerprint::base(id(model_id))
}

/// A seeded run standing on the last act's boss, in a fight the boss is one
/// blow from losing.
///
/// The run-ending path is walked by standing a run where it ends. The run
/// itself is the generated one, so its acts, maps, and pools are the build's;
/// what is placed is where the player is standing and the fight in front of
/// them.
pub fn final_boss_fixture() -> Result<Simulator, EngineError> {
    const SEED: &str = "QEY5K1P4LY";
    let content = standard_registry();
    let combat = ScenarioBuilder::new(SEED, id("ENCOUNTER.QUEEN_BOSS"))
        .enemy(1, id("MONSTER.QUEEN"), 1, 400, "PUPPET_STRINGS_MOVE")
        .card(PileName::Hand, 0, reward_card("CARD.STRIKE_IRONCLAD"), 1)
        .build(content.clone())?
        .state()
        .combat
        .clone();

    let mut state = standard_ironclad_run(SEED)?.state().clone();
    {
        let run = state.run.as_mut().ok_or_else(|| {
            EngineError::new(sts2_engine::ErrorCode::InternalInvariant)
                .with_detail("final_boss_fixture", "a generated run has run state")
        })?;
        let last = run.acts.len() - 1;
        let boss = run.maps[last].boss;
        run.current_act = last;
        run.current_coord = Some(boss);
        run.current_room_type = Some(sts2_engine::MapPointType::Boss);
        run.current_room_model = Some(id("ENCOUNTER.QUEEN_BOSS"));
        run.visited = vec![boss];
        run.phase = sts2_engine::RunPhase::Room;
        run.active_room = None;
        run.floor = 45;
    }
    state.combat = combat;
    Simulator::from_scenario(state, content)
}

/// Creates the first fixture combat at its first player decision.
pub fn shrinker_beetle_fixture() -> Result<Simulator, EngineError> {
    let hand = [
        (0, "CARD.DEFEND_IRONCLAD", 1),
        (1, "CARD.STRIKE_IRONCLAD", 1),
        (2, "CARD.DEFEND_IRONCLAD", 1),
        (3, "CARD.STRIKE_IRONCLAD", 1),
        (4, "CARD.BULLY", 0),
    ];
    let draw = [
        (5, "CARD.STRIKE_IRONCLAD", 1),
        (6, "CARD.BASH", 2),
        (7, "CARD.STRIKE_IRONCLAD", 1),
        (8, "CARD.DEFEND_IRONCLAD", 1),
        (9, "CARD.DEFEND_IRONCLAD", 1),
        (10, "CARD.STRIKE_IRONCLAD", 1),
    ];
    let mut scenario = ScenarioBuilder::new(FIXTURE_SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(3, 3)
        .turn(1, 1)
        .rng_counter(RunStream::Shuffle, 10)
        .rng_counter(RunStream::MonsterAi, 0)
        .rng_counter(RunStream::Niche, 1)
        .gold(99)
        .potions(vec![Some(id("POTION.BLOCK_POTION")), None, None])
        .relics(&[id("RELIC.BURNING_BLOOD")])
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .reward(RewardFingerprint::of("gold").with_gold(16))
        .reward(RewardFingerprint::of("potion").with_model(id("POTION.VULNERABLE_POTION")))
        .reward(
            RewardFingerprint::of("card")
                .with_cards(
                    ["CARD.INFERNAL_BLADE", "CARD.STONE_ARMOR", "CARD.HAVOC"]
                        .map(reward_card)
                        .to_vec(),
                    vec![id("CARD_POOL.IRONCLAD_CARD_POOL")],
                    sts2_engine::RarityOdds::RegularEncounter,
                )
                .from_source(sts2_engine::RewardSource::Encounter),
        );
    for (card_id, model_id, cost) in hand {
        scenario = scenario.card(PileName::Hand, card_id, card(model_id), cost);
    }
    for (card_id, model_id, cost) in draw {
        scenario = scenario.card(PileName::Draw, card_id, card(model_id), cost);
    }
    scenario.build(standard_registry())
}
