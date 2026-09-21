use sts2_content::{RANDOM_CHARACTER, pinned_act_selection, select_acts};
use sts2_core::{ModelId, UnlockPresetManifest, UnlockState};
use sts2_engine::ActOneSetting;

fn id(value: &str) -> ModelId {
    value.parse().expect("static model ID")
}

fn ids(values: &[&str]) -> Vec<ModelId> {
    values.iter().copied().map(id).collect()
}

fn baseline() -> (UnlockState, Vec<ModelId>) {
    let preset = UnlockPresetManifest::pinned().expect("the pinned preset parses");
    (preset.unlocks, preset.profile.discovered_acts)
}

fn act_ids(selection: &sts2_content::ActSelection) -> Vec<String> {
    selection.acts.iter().map(ToString::to_string).collect()
}

#[test]
fn seeds_recorded_opening_in_the_overgrowth_roll_it() {
    // Every one of these seeds opens in the Overgrowth and goes on to the
    // Hive and Glory; the act list is settled by the seed before any play,
    // and this is it.
    for seed in [
        "N3Z9EE2CXB",
        "QEY5K1P4LY",
        "R8F8NFB8Y1",
        "PCJZ1E6BT4",
        "BPK46736W9",
    ] {
        // `Act1` defaults to `random` and the build hides the dropdown that
        // would change it, so this is the setting every run plays under.
        let selection =
            pinned_act_selection(seed, ActOneSetting::Random, &id("CHARACTER.IRONCLAD"))
                .expect("the baseline profile has an unlocked act for every index");
        assert_eq!(
            act_ids(&selection),
            ["ACT.OVERGROWTH", "ACT.HIVE", "ACT.GLORY"],
            "act list differs for {seed}"
        );
        // Three indices, one roll each, and the configured character is not
        // the random one, so nothing more is read.
        assert_eq!(selection.counter, 3, "act-selection counter for {seed}");
        assert_eq!(selection.character, id("CHARACTER.IRONCLAD"));
    }
}

#[test]
fn a_seed_that_rolls_the_alternate_act_opens_in_underdocks() {
    // This seed opens in `ACT.UNDERDOCKS`: the act list is settled before
    // anything else in the run rolls, and this is what the roll reproduces.
    let selection = pinned_act_selection(
        "X7JFABM2QS",
        ActOneSetting::Random,
        &id("CHARACTER.IRONCLAD"),
    )
    .expect("the baseline profile has an unlocked act for every index");
    assert_eq!(
        act_ids(&selection),
        ["ACT.UNDERDOCKS", "ACT.HIVE", "ACT.GLORY"]
    );
    assert_eq!(selection.counter, 3);
}

#[test]
fn the_act_one_setting_names_the_act_and_reads_no_dice() {
    let (unlocks, discovered) = baseline();
    // One seed that rolls Overgrowth and one that rolls Underdocks, so the
    // setting is seen to replace what the roll settled on either way.
    for seed in ["N3Z9EE2CXB", "X7JFABM2QS"] {
        for (setting, expected) in [
            (ActOneSetting::Overgrowth, "ACT.OVERGROWTH"),
            (ActOneSetting::Underdocks, "ACT.UNDERDOCKS"),
        ] {
            let selection = select_acts(
                seed,
                setting,
                &unlocks,
                &discovered,
                &id("CHARACTER.IRONCLAD"),
            )
            .expect("every index has an unlocked act");
            assert_eq!(
                act_ids(&selection),
                [expected, "ACT.HIVE", "ACT.GLORY"],
                "{seed} with {setting:?}"
            );
            // The override happens after the rolls, so it costs nothing and
            // leaves the stream exactly where a random setting would.
            assert_eq!(selection.counter, 3, "{seed} with {setting:?}");
        }
    }
}

#[test]
fn an_undiscovered_alternate_act_is_taken_without_a_roll() {
    let (unlocks, _) = baseline();
    let discovered = ids(&["ACT.OVERGROWTH"]);
    // Index 0 reads no dice, so act two and act three take the draws act one
    // and act two would have taken, and a random character takes the third.
    let selection = select_acts(
        "N3Z9EE2CXB",
        ActOneSetting::Random,
        &unlocks,
        &discovered,
        &id(RANDOM_CHARACTER),
    )
    .expect("every index has an unlocked act");
    assert_eq!(
        act_ids(&selection),
        ["ACT.UNDERDOCKS", "ACT.HIVE", "ACT.GLORY"]
    );
    assert_eq!(selection.character, id("CHARACTER.DEFECT"));
    assert_eq!(selection.counter, 3);

    // The setting still overrides what the shortcut took, and still without
    // moving the stream.
    let overridden = select_acts(
        "N3Z9EE2CXB",
        ActOneSetting::Overgrowth,
        &unlocks,
        &discovered,
        &id("CHARACTER.IRONCLAD"),
    )
    .expect("every index has an unlocked act");
    assert_eq!(
        act_ids(&overridden),
        ["ACT.OVERGROWTH", "ACT.HIVE", "ACT.GLORY"]
    );
    assert_eq!(overridden.counter, 2);
}

#[test]
fn a_locked_alternate_act_is_no_candidate_at_all() {
    let (mut unlocks, discovered) = baseline();
    unlocks.epochs.retain(|epoch| epoch != "UNDERDOCKS_EPOCH");
    for seed in ["N3Z9EE2CXB", "X7JFABM2QS"] {
        let selection = select_acts(
            seed,
            ActOneSetting::Random,
            &unlocks,
            &discovered,
            &id("CHARACTER.IRONCLAD"),
        )
        .expect("Overgrowth is unlocked unconditionally");
        assert_eq!(
            act_ids(&selection),
            ["ACT.OVERGROWTH", "ACT.HIVE", "ACT.GLORY"],
            "{seed}"
        );
        // A single candidate is still rolled for, so the counter does not
        // move even though the answer could not have been anything else.
        assert_eq!(selection.counter, 3, "{seed}");
    }
}

#[test]
fn a_random_character_is_rolled_off_the_same_stream_after_the_acts() {
    let (unlocks, discovered) = baseline();
    for (seed, expected) in [
        ("N3Z9EE2CXB", "CHARACTER.NECROBINDER"),
        ("X7JFABM2QS", "CHARACTER.REGENT"),
        ("QEY5K1P4LY", "CHARACTER.IRONCLAD"),
        ("R8F8NFB8Y1", "CHARACTER.IRONCLAD"),
        ("PCJZ1E6BT4", "CHARACTER.REGENT"),
    ] {
        let selection = select_acts(
            seed,
            ActOneSetting::Random,
            &unlocks,
            &discovered,
            &id(RANDOM_CHARACTER),
        )
        .expect("every index has an unlocked act");
        assert_eq!(selection.character.to_string(), expected, "{seed}");
        assert_eq!(selection.counter, 4, "{seed}");
    }
}

/// A lobby that left the choice to chance stands the run up as whichever of
/// the five characters the roll names. This seed rolls the Defect, so the
/// run is his: his health, his starting relic, his orb slots.
///
/// There is no rollable character for a run to stop on. What stops a run
/// is a character with no loadout, and the build carries five characters.
#[test]
fn a_run_that_rolls_the_defect_is_stood_up_as_his() {
    let simulator = sts2_content::standard_ironclad_run_with(
        "06Z9EE2CXB",
        ActOneSetting::Random,
        &id(RANDOM_CHARACTER),
    )
    .expect("the Defect is implemented");
    let state = simulator.state();
    assert_eq!(state.run_player.max_hp, 75);
    assert_eq!(
        state.run_player.relics[0].model_id.to_string(),
        "RELIC.CRACKED_CORE"
    );
    // The Defect's own three orb slots, which every fight of the run opens with.
    assert_eq!(state.run_player.base_orb_slots, 3);
}

/// A seed whose roll names the Regent: a lobby that left the choice to
/// chance stands his own run up.
#[test]
fn a_run_that_rolls_the_regent_is_stood_up_as_his() {
    let simulator = sts2_content::standard_ironclad_run_with(
        "X7JFABM2QS",
        ActOneSetting::Random,
        &id(RANDOM_CHARACTER),
    )
    .expect("the Regent is implemented");
    let state = simulator.state();
    assert_eq!(state.run_player.max_hp, 75);
    assert_eq!(
        state.run_player.relics[0].model_id.to_string(),
        "RELIC.DIVINE_RIGHT"
    );
}

/// A seed whose roll names the Necrobinder: a lobby that left the choice to
/// chance stands her own run up.
#[test]
fn a_run_that_rolls_the_necrobinder_is_stood_up_as_hers() {
    let simulator = sts2_content::standard_ironclad_run_with(
        "N3Z9EE2CXB",
        ActOneSetting::Random,
        &id(RANDOM_CHARACTER),
    )
    .expect("the Necrobinder is implemented");
    let state = simulator.state();
    assert_eq!(state.run_player.max_hp, 66);
    assert_eq!(
        state.run_player.relics[0].model_id.to_string(),
        "RELIC.BOUND_PHYLACTERY"
    );
}

/// The other act one generates like any other act: this seed opens in the
/// Underdocks, and the run it makes stands in that act's own rooms.
#[test]
fn a_run_that_opens_in_the_underdocks_generates() {
    let simulator =
        sts2_content::standard_ironclad_run("X7JFABM2QS").expect("the seeded run generates");
    let run = simulator.state().run.as_ref().expect("the run exists");
    assert_eq!(
        run.acts[0].act_model.to_string(),
        "ACT.UNDERDOCKS",
        "the seed opens in the other act one"
    );
    assert_eq!(
        run.acts[0].ancient.to_string(),
        "EVENT.NEOW",
        "the Underdocks' only ancient is Neow"
    );
    assert!(
        run.acts[0]
            .normal_encounters
            .iter()
            .all(|encounter| encounter.to_string().contains('.')),
        "the act's encounter pool is filled"
    );
}
