use std::sync::Arc;

use sts2_core::{PINNED_COMPATIBILITY_ID, UnlockPresetManifest};
use sts2_engine::{
    ActiveRoom, CardFingerprint, ConformanceOrigin, ContentRegistry, EngineError, ErrorCode,
    EventEffect, EventOption, GameState, RunConfig, RunPhase, RunPlayerState, RunState, Simulator,
};
use sts2_rng::{PlayerRngSet, RunRngSet, RunStream};

use crate::run_generation::act_room_pools_with_history;
use crate::{UpFrontGeneration, id, select_acts, standard_act_map, standard_neow_page};

fn starter_card(model_id: &str) -> CardFingerprint {
    let mut card = CardFingerprint::base(id(model_id));
    card.floor_added_to_deck = Some(1);
    card
}

/// What a character opens a run with: starting health, starting gold, starting
/// deck, starting relic, and — for the one character that has any — the orb
/// slots they begin with.
struct Loadout {
    max_hp: i32,
    gold: i32,
    deck: Vec<&'static str>,
    relic: &'static str,
    orb_slots: i32,
}

fn loadout(character: &sts2_core::ModelId) -> Option<Loadout> {
    match character.to_string().as_str() {
        "CHARACTER.IRONCLAD" => Some(Loadout {
            max_hp: 80,
            gold: 99,
            deck: vec![
                "CARD.STRIKE_IRONCLAD",
                "CARD.STRIKE_IRONCLAD",
                "CARD.STRIKE_IRONCLAD",
                "CARD.STRIKE_IRONCLAD",
                "CARD.STRIKE_IRONCLAD",
                "CARD.DEFEND_IRONCLAD",
                "CARD.DEFEND_IRONCLAD",
                "CARD.DEFEND_IRONCLAD",
                "CARD.DEFEND_IRONCLAD",
                "CARD.BASH",
            ],
            relic: "RELIC.BURNING_BLOOD",
            orb_slots: 0,
        }),
        "CHARACTER.SILENT" => Some(Loadout {
            max_hp: 70,
            gold: 99,
            deck: vec![
                "CARD.STRIKE_SILENT",
                "CARD.STRIKE_SILENT",
                "CARD.STRIKE_SILENT",
                "CARD.STRIKE_SILENT",
                "CARD.STRIKE_SILENT",
                "CARD.DEFEND_SILENT",
                "CARD.DEFEND_SILENT",
                "CARD.DEFEND_SILENT",
                "CARD.DEFEND_SILENT",
                "CARD.DEFEND_SILENT",
                "CARD.NEUTRALIZE",
                "CARD.SURVIVOR",
            ],
            relic: "RELIC.RING_OF_THE_SNAKE",
            orb_slots: 0,
        }),
        "CHARACTER.NECROBINDER" => Some(Loadout {
            max_hp: 66,
            gold: 99,
            deck: vec![
                "CARD.STRIKE_NECROBINDER",
                "CARD.STRIKE_NECROBINDER",
                "CARD.STRIKE_NECROBINDER",
                "CARD.STRIKE_NECROBINDER",
                "CARD.DEFEND_NECROBINDER",
                "CARD.DEFEND_NECROBINDER",
                "CARD.DEFEND_NECROBINDER",
                "CARD.DEFEND_NECROBINDER",
                "CARD.BODYGUARD",
                "CARD.UNLEASH",
            ],
            relic: "RELIC.BOUND_PHYLACTERY",
            orb_slots: 0,
        }),
        "CHARACTER.REGENT" => Some(Loadout {
            max_hp: 75,
            gold: 99,
            deck: vec![
                "CARD.STRIKE_REGENT",
                "CARD.STRIKE_REGENT",
                "CARD.STRIKE_REGENT",
                "CARD.STRIKE_REGENT",
                "CARD.DEFEND_REGENT",
                "CARD.DEFEND_REGENT",
                "CARD.DEFEND_REGENT",
                "CARD.DEFEND_REGENT",
                "CARD.FALLING_STAR",
                "CARD.VENERATE",
            ],
            relic: "RELIC.DIVINE_RIGHT",
            orb_slots: 0,
        }),
        "CHARACTER.DEFECT" => Some(Loadout {
            max_hp: 75,
            gold: 99,
            deck: vec![
                "CARD.STRIKE_DEFECT",
                "CARD.STRIKE_DEFECT",
                "CARD.STRIKE_DEFECT",
                "CARD.STRIKE_DEFECT",
                "CARD.DEFEND_DEFECT",
                "CARD.DEFEND_DEFECT",
                "CARD.DEFEND_DEFECT",
                "CARD.DEFEND_DEFECT",
                "CARD.ZAP",
                "CARD.DUALCAST",
            ],
            relic: "RELIC.CRACKED_CORE",
            // Three orb slots: the Defect is the one character in the pinned
            // build that walks in with orb slots of their own.
            orb_slots: 3,
        }),
        _ => None,
    }
}

/// One of Neow's generated options: the relic it offers, obtained.
///
/// Every one of Neow's relic options works the same way: the relic is obtained
/// and the event ends. The text the option leads to next is presentation, not a
/// gameplay fact. Whatever the relic then does is the relic's own on-obtained
/// behaviour, which the relic pools carry, so nothing here has to know which
/// relic it handed over.
fn neow_relic_option(option_id: String) -> EventOption {
    let relic = option_id
        .rsplit('.')
        .next()
        .expect("generated option has a suffix");
    let effect = EventEffect::ObtainRelic(id(&format!("RELIC.{relic}")));
    EventOption {
        label: option_id.clone(),
        option_id,
        effects: vec![effect],
        is_proceed: false,
        was_chosen: false,
        locked: false,
    }
}

/// Creates a genuine seed-generated standard Ironclad run at the opening Neow
/// decision. The generated map and room pools are complete.
pub fn standard_ironclad_run(seed: &str) -> Result<Simulator, EngineError> {
    let baseline = RunConfig::fixture_baseline(seed);
    standard_ironclad_run_with(seed, baseline.act_one, &baseline.players[0].character)
}

/// The same run generated at an ascension level.
///
/// Which level a run is played at is a run-defining input the seed does not
/// carry, in the same sense the character is, so it enters here rather than
/// being read off anything the run has already generated.
pub fn standard_ironclad_run_at(seed: &str, ascension: u8) -> Result<Simulator, EngineError> {
    let preset = UnlockPresetManifest::pinned().map_err(|error| {
        EngineError::new(ErrorCode::InternalInvariant)
            .with_detail("unlock_preset", error.to_string())
    })?;
    let baseline = RunConfig::fixture_baseline(seed);
    build_run(
        seed,
        baseline.act_one,
        &baseline.players[0].character,
        &preset,
        ascension,
    )
}

/// A seed-generated run for a named character, at the opening Neow decision.
///
/// The characters this baseline generates for are the ones with a loadout
/// written down; a run rolled onto any other stops naming it.
pub fn standard_run_on_preset(
    seed: &str,
    character: &sts2_core::ModelId,
    unlock_preset: &UnlockPresetManifest,
) -> Result<Simulator, EngineError> {
    standard_run_on_preset_at(
        seed,
        character,
        unlock_preset,
        sts2_engine::BASELINE_ASCENSION,
    )
}

/// The same run generated at an ascension level.
pub fn standard_run_on_preset_at(
    seed: &str,
    character: &sts2_core::ModelId,
    unlock_preset: &UnlockPresetManifest,
    ascension: u8,
) -> Result<Simulator, EngineError> {
    run_on_preset_at(
        seed,
        character,
        unlock_preset,
        ascension,
        sts2_engine::RunMode::Standard,
    )
}

/// The same run generated in a named lobby mode. `Custom` is the lobby that
/// accepts a chosen seed and skips the standard mode's boss-discovery overrides.
pub fn run_on_preset_at(
    seed: &str,
    character: &sts2_core::ModelId,
    unlock_preset: &UnlockPresetManifest,
    ascension: u8,
    mode: sts2_engine::RunMode,
) -> Result<Simulator, EngineError> {
    let baseline = RunConfig::fixture_baseline(seed);
    build_run_in_mode(
        seed,
        baseline.act_one,
        character,
        unlock_preset,
        ascension,
        mode,
    )
}

/// The same run generated against a named unlock preset.
///
/// The local profile is a run-defining input the seed does not carry: the
/// epoch and encounter lists change generation. Explicit manifests need not be
/// registered. The prior-run count is carried as profile state and, past a first
/// run, changes nothing generation reads.
pub fn standard_ironclad_run_on_preset(
    seed: &str,
    unlock_preset: &UnlockPresetManifest,
) -> Result<Simulator, EngineError> {
    let baseline = RunConfig::fixture_baseline(seed);
    build_run(
        seed,
        baseline.act_one,
        &baseline.players[0].character,
        unlock_preset,
        sts2_engine::BASELINE_ASCENSION,
    )
}

/// Rolls what the lobby rolls and generates against the answer.
///
/// Act one is rolled, and a lobby that left the character to chance has that
/// rolled too, both off the seed's act-selection stream before the run is
/// stood up. What the stream settles on is what the run is generated against,
/// and content this baseline does not carry stops here rather than being
/// approximated by the act or character that was asked for.
fn generate_up_front(
    seed: &str,
    config: &RunConfig,
    unlock_preset: &UnlockPresetManifest,
) -> Result<UpFrontGeneration, EngineError> {
    // Content pools currently implement the full-epoch, post-first-run rules.
    // Accept independent encounter/discovery histories without silently making
    // a fresh or partially unlocked profile use veteran content.
    let baseline = UnlockPresetManifest::pinned().map_err(|error| {
        EngineError::new(ErrorCode::InternalInvariant).with_detail("profile", error.to_string())
    })?;
    let epochs = |unlocks: &sts2_core::UnlockState| {
        unlocks
            .epochs
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
    };
    if epochs(&unlock_preset.unlocks) != epochs(&baseline.unlocks) {
        return Err(EngineError::new(ErrorCode::UnsupportedContent).with_detail(
            "profile",
            "content pools require all pinned epochs revealed",
        ));
    }
    if unlock_preset.unlocks.number_of_runs == 0 {
        return Err(EngineError::new(ErrorCode::UnsupportedContent)
            .with_detail("profile", "first-run generation rules are not implemented"));
    }
    let selection = select_acts(
        seed,
        config.act_one,
        &unlock_preset.unlocks,
        &unlock_preset.profile.discovered_acts,
        &config.players[0].character,
    )?;
    if loadout(&selection.character).is_none() {
        return Err(EngineError::new(ErrorCode::UnsupportedContent)
            .with_detail("model_id", selection.character.to_string())
            .with_detail("location", "run generation: character"));
    }
    act_room_pools_with_history(
        seed,
        &selection.acts,
        &selection.character,
        config.ascension,
        (config.mode == sts2_engine::RunMode::Standard)
            .then_some(unlock_preset.unlocks.encounters_seen.as_slice()),
    )
}

/// The same run against the two run-defining inputs the lobby carries and the
/// seed does not: which act one was asked for, and which character.
pub fn standard_ironclad_run_with(
    seed: &str,
    act_one: sts2_engine::ActOneSetting,
    character: &sts2_core::ModelId,
) -> Result<Simulator, EngineError> {
    let preset = UnlockPresetManifest::pinned().map_err(|error| {
        EngineError::new(ErrorCode::InternalInvariant)
            .with_detail("unlock_preset", error.to_string())
    })?;
    build_run(
        seed,
        act_one,
        character,
        &preset,
        sts2_engine::BASELINE_ASCENSION,
    )
}

fn build_run(
    seed: &str,
    act_one: sts2_engine::ActOneSetting,
    character: &sts2_core::ModelId,
    unlock_preset: &UnlockPresetManifest,
    ascension: u8,
) -> Result<Simulator, EngineError> {
    build_run_in_mode(
        seed,
        act_one,
        character,
        unlock_preset,
        ascension,
        sts2_engine::RunMode::Standard,
    )
}

fn build_run_in_mode(
    seed: &str,
    act_one: sts2_engine::ActOneSetting,
    character: &sts2_core::ModelId,
    unlock_preset: &UnlockPresetManifest,
    ascension: u8,
    mode: sts2_engine::RunMode,
) -> Result<Simulator, EngineError> {
    build_run_against(
        seed,
        act_one,
        character,
        unlock_preset,
        ascension,
        mode,
        None,
    )
}

/// A fight stood up from a setup alone: a run for the setup's character at
/// its ascension is generated on `seed` — maps, pools and streams the way any
/// seeded run gets them — and the setup's encounter is entered from it
/// through the engine's own combat start. The run seed is what the fight's
/// enemies roll their health from, so a setup projected off a run and stood
/// back up on that run's seed meets the same numbers it met.
///
/// Every model the setup names is checked against the content the run is
/// generated against; a setup this build cannot honor is refused, never
/// reinterpreted.
pub fn combat_from_setup(
    setup: &sts2_engine::CombatSetup,
    seed: &str,
) -> Result<Simulator, EngineError> {
    let preset = UnlockPresetManifest::pinned().map_err(|error| {
        EngineError::new(ErrorCode::InternalInvariant)
            .with_detail("unlock_preset", error.to_string())
    })?;
    combat_from_setup_with(setup, seed, &preset, None)
}

/// [`combat_from_setup`] with the preset and the content handed in, for a
/// caller standing many fights up: the registry a character sees at an
/// ascension is built once per pair and shared, rather than once per fight.
/// A registry handed in must be the one `registry_at` builds for the setup's
/// character and ascension; the run is generated against whatever is given.
pub fn combat_from_setup_with(
    setup: &sts2_engine::CombatSetup,
    seed: &str,
    unlock_preset: &UnlockPresetManifest,
    content: Option<Arc<ContentRegistry>>,
) -> Result<Simulator, EngineError> {
    let baseline = RunConfig::fixture_baseline(seed);
    let run = build_run_against(
        seed,
        baseline.act_one,
        &setup.character,
        unlock_preset,
        setup.ascension,
        sts2_engine::RunMode::Standard,
        content,
    )?;
    run.enter_combat_setup(setup)
}

#[allow(
    clippy::too_many_lines,
    reason = "the run's opening state is one literal, written out field by field"
)]
fn build_run_against(
    seed: &str,
    act_one: sts2_engine::ActOneSetting,
    character: &sts2_core::ModelId,
    unlock_preset: &UnlockPresetManifest,
    ascension: u8,
    mode: sts2_engine::RunMode,
    content: Option<Arc<ContentRegistry>>,
) -> Result<Simulator, EngineError> {
    let mut config = RunConfig::fixture_baseline(seed).at_ascension(ascension);
    config.mode = mode;
    config.act_one = act_one;
    config.players[0].character = character.clone();
    config.unlock_preset.clone_from(&unlock_preset.preset_id);
    config.unlock_profile = Some(unlock_preset.clone());
    config.validate()?;
    let generated = generate_up_front(seed, &config, unlock_preset)?;
    // Which character the run is actually played as is the act selection's
    // answer, not the lobby's: a lobby that left it to chance has it rolled.
    let played = select_acts(
        seed,
        config.act_one,
        &unlock_preset.unlocks,
        &unlock_preset.profile.discovered_acts,
        &config.players[0].character,
    )?
    .character;
    let loadout = loadout(&played).ok_or_else(|| {
        EngineError::new(ErrorCode::UnsupportedContent)
            .with_detail("model_id", played.to_string())
            .with_detail("location", "run generation: character")
    })?;
    // Every act's map is generated up front, each on its own stream. An act
    // that drew a second boss is laid out with the extra point that fight
    // stands on, so whether the act has one is part of its layout.
    let maps: Vec<_> = (0..generated.acts.len())
        .map(|act| {
            standard_act_map(
                seed,
                act,
                config.ascension,
                generated.acts[act].second_boss.is_some(),
            )
        })
        .collect();
    let (neow_page, neow_rng_counter) = standard_neow_page(seed);
    let neow_options: Vec<_> = neow_page.into_iter().map(neow_relic_option).collect();
    let mut rng = RunRngSet::new(seed);
    rng.stream_mut(RunStream::UpFront)
        .fast_forward(generated.up_front_counter)
        .expect("fresh run stream can advance to generated counter");
    let visited = vec![maps[0].start];
    let run = RunState {
        config,
        current_act: 0,
        floor: 1,
        current_coord: Some(maps[0].start),
        // The start point's own room is the first room of the run.
        current_room_id: 0,
        next_room_id: 1,
        // Every combat room is generated on entry from the act pools, so no
        // room is placed ahead of time.
        rooms: vec![Vec::new(); maps.len()],
        maps,
        visited,
        acts: {
            // Neow has already been walked into, and the ancient counts as an
            // event visit even though it is not one of the act's events.
            let mut acts = generated.acts;
            acts[0].events_visited = 1;
            acts
        },
        phase: RunPhase::Room,
        pushed_room: None,
        // The run opens standing in Neow's room with the event begun and its
        // page up.
        begun_event: Some(sts2_engine::BegunEvent {
            model_id: id("EVENT.NEOW"),
            options: neow_options.clone(),
        }),
        current_room_model: Some(id("EVENT.NEOW")),
        current_room_type: Some(sts2_engine::MapPointType::Ancient),
        previous_room_type: None,
        // Neow is the act's ancient, so entering it records no visited event.
        visited_events: std::collections::BTreeSet::new(),
        started_with_neow: true,
        // A generated run is being played from its beginning, not picked back
        // up off a save.
        reloads: 0,
        // The run opens standing on the act's ancient, already in Neow's room.
        map_history: vec![sts2_engine::ActHistory {
            act: 0,
            points: vec![sts2_engine::MapPointVisit {
                floor: 0,
                point_type: sts2_engine::MapPointType::Ancient,
                rooms: vec![sts2_engine::RoomVisit {
                    index: 0,
                    model_id: Some(id("EVENT.NEOW")),
                    room_type: sts2_engine::MapPointType::Ancient,
                    monsters: Vec::new(),
                    turns_taken: 0,
                }],
            }],
        }],
        unlocks: unlock_preset.unlocks.clone(),
        active_room: Some(ActiveRoom::Event {
            standing_fight: None,
            model_id: id("EVENT.NEOW"),
            options: neow_options,
            rng_counter: neow_rng_counter,
            shelf: Vec::new(),
            rolled_card: None,
            skipped_cards: Vec::new(),
            sphere: None,
        }),
        shared_relic_grab_bag: generated.shared_relics,
        player_relic_grab_bag: generated.player_relics,
        marked_map_coord: None,
        reward_set: None,
        reward_parents: Vec::new(),
        dismissed_reward_set: None,
        rest_option_owed: None,
        rest_options_owed: Vec::new(),
        suspended_choice: None,
        pending: std::collections::VecDeque::new(),
        next_choice_id: 0,
        next_reward_set_id: 0,
        finish_event_after_rewards: false,
        finish_event_after_pending: false,
        purchase_owed_by_rewards: None,
        owed_by_rewards: std::collections::VecDeque::new(),
        card_removals_used: 0,
        owed_purchases: std::collections::VecDeque::new(),
        owed_removal: false,
        unknown_room_odds: sts2_engine::UnknownRoomOdds::start_of_act(),
    };
    Simulator::from_generated_state(
        GameState {
            compatibility_id: PINNED_COMPATIBILITY_ID.to_owned(),
            origin: ConformanceOrigin::Generation,
            rng,
            player_rng: PlayerRngSet::for_player(seed, 0),
            run_player: RunPlayerState {
                slot: "p0".to_owned(),
                max_energy: 3,
                current_hp: opening_hp(loadout.max_hp, ascension),
                max_hp: loadout.max_hp,
                gold: loadout.gold,
                deck: opening_deck(&loadout, ascension),
                // A run opens with three potion slots whoever is playing, and
                // Tight Belt takes one of them away.
                potions: vec![
                    None;
                    sts2_engine::potion_slot_count(
                        ascension,
                        sts2_engine::INITIAL_MAX_POTION_SLOTS
                    )
                ],
                relics: vec![sts2_engine::RelicInstance::new(id(loadout.relic)).obtained_on(1)],
                base_orb_slots: loadout.orb_slots,
                odds: sts2_engine::PlayerOdds::start_of_run(),
            },
            run: Some(run),
            combat: None,
            terminal: None,
            visible_history: sts2_engine::VisibleHistory::default(),
            event_sequence: 1,
            next_card_instance_id: 0,
        },
        content.unwrap_or_else(|| crate::registry_at(&played, ascension)),
    )
}

/// The deck a run opens with, plus the curse an Ascender's Bane run is charged.
///
/// The curse is appended to the end of the deck, so it stands behind the whole
/// starting deck, and it is written on the first floor like everything else the
/// run opens with.
fn opening_deck(loadout: &Loadout, ascension: u8) -> Vec<CardFingerprint> {
    let mut deck: Vec<CardFingerprint> = loadout.deck.iter().copied().map(starter_card).collect();
    if sts2_engine::has_ascension(ascension, sts2_engine::AscensionLevel::AscendersBane) {
        deck.push(starter_card("CARD.ASCENDERS_BANE"));
    }
    deck
}

/// Neow's opening: health is set to zero and then healed back by everything
/// that is missing.
///
/// Neow is the one ancient that zeroes the player first, so what is missing is
/// the whole bar — and a Weary Traveler run therefore opens the run four fifths
/// alive rather than whole.
fn opening_hp(max_hp: i32, ascension: u8) -> i32 {
    sts2_engine::ancient_heal_amount(ascension, max_hp)
}
