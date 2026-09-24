use sts2_core::PINNED_COMPATIBILITY_ID;
use sts2_engine::{
    ActMap, ErrorCode, FIXTURE_UNLOCK_PRESET, MapCoord, MapPoint, MapPointType, RunConfig,
};

#[test]
fn fixture_preset_expands_to_every_run_defining_input() {
    let config = RunConfig::fixture_baseline("N3Z9EE2CXB");
    config.validate().unwrap();
    assert_eq!(config.compatibility_id, PINNED_COMPATIBILITY_ID);
    assert_eq!(config.players.len(), 1);
    assert_eq!(config.players[0].slot, "p0");
    assert_eq!(
        config.players[0].character.to_string(),
        "CHARACTER.IRONCLAD"
    );
    assert_eq!(config.ascension, 0);
    assert!(config.modifiers.is_empty());
    assert_eq!(config.unlock_preset, FIXTURE_UNLOCK_PRESET);
}

#[test]
fn incomplete_or_conflicting_run_identity_is_rejected() {
    let mut config = RunConfig::fixture_baseline("n3z9ee2cxb");
    assert_eq!(
        config.validate().unwrap_err().code,
        ErrorCode::InvalidConfig
    );
    config.seed = "N3Z9EE2CXB".to_owned();
    config.compatibility_id = "other".to_owned();
    assert_eq!(
        config.validate().unwrap_err().code,
        ErrorCode::UnsupportedBuild
    );
}

#[test]
fn explicit_profiles_validate_their_identity_and_old_configs_still_resolve() {
    let mut config = RunConfig::fixture_baseline("ABC");
    let old = serde_json::to_value(&config).unwrap();
    assert!(old.get("unlock_profile").is_none());
    serde_json::from_value::<RunConfig>(old)
        .unwrap()
        .validate()
        .unwrap();
    config.unlock_preset = "recorded_profile".to_owned();
    assert!(config.validate().is_err());
    let mut profile = sts2_core::UnlockPresetManifest::pinned().unwrap();
    profile.preset_id.clone_from(&config.unlock_preset);
    config.unlock_profile = Some(profile.clone());
    config.validate().unwrap();
    for (schema_version, compatibility_id, preset_id) in [
        (2, PINNED_COMPATIBILITY_ID, "recorded_profile"),
        (1, "other-build", "recorded_profile"),
        (1, PINNED_COMPATIBILITY_ID, "other-profile"),
    ] {
        let mut invalid = profile.clone();
        invalid.schema_version = schema_version;
        invalid.compatibility_id = compatibility_id.to_owned();
        invalid.preset_id = preset_id.to_owned();
        config.unlock_profile = Some(invalid);
        assert!(config.validate().is_err());
    }
}

/// The highest ascension the build will start a run at is ten, so every level
/// the character select screen offers is a run this simulator generates and
/// eleven is not a run at all.
#[test]
fn every_offered_ascension_is_accepted_and_nothing_above_the_build_s_own_cap_is() {
    for ascension in 0..=sts2_engine::MAX_ASCENSION {
        let config = RunConfig::fixture_baseline("N3Z9EE2CXB").at_ascension(ascension);
        config.validate().unwrap();
        assert_eq!(config.ascension, ascension);
    }
    let config =
        RunConfig::fixture_baseline("N3Z9EE2CXB").at_ascension(sts2_engine::MAX_ASCENSION + 1);
    assert_eq!(
        config.validate().unwrap_err().code,
        ErrorCode::InvalidConfig
    );
}

#[test]
fn map_invariants_require_forward_resolving_edges() {
    let start = MapCoord { col: 1, row: 0 };
    let next = MapCoord { col: 0, row: 1 };
    let map = ActMap {
        width: 3,
        height: 2,
        start,
        boss: MapCoord { col: 1, row: 2 },
        second_boss: None,
        points: vec![
            MapPoint {
                coord: start,
                point_type: MapPointType::Ancient,
                children: vec![next],
                can_modify: true,
            },
            MapPoint {
                coord: next,
                point_type: MapPointType::Monster,
                children: vec![MapCoord { col: 1, row: 2 }],
                can_modify: false,
            },
            MapPoint {
                coord: MapCoord { col: 1, row: 2 },
                point_type: MapPointType::Boss,
                children: Vec::new(),
                can_modify: true,
            },
        ],
    };
    map.validate().unwrap();
    assert_eq!(map.destinations(Some(start)), [next]);

    let mut invalid = map;
    invalid
        .points
        .iter_mut()
        .find(|point| point.coord == start)
        .unwrap()
        .children[0]
        .row = 2;
    assert_eq!(
        invalid.validate().unwrap_err().code,
        ErrorCode::InternalInvariant
    );
}
