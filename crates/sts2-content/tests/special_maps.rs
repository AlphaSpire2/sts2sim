//! The two act maps a run reaches for instead of the standard one: the fixed
//! golden path, and the hourglass the Spoils Map card lays out.

use std::collections::BTreeSet;

use sts2_content::{golden_path_act_map, spoils_act_map};
use sts2_engine::{ActMap, MapCoord, MapPointType};

/// Every act the pinned build lays a map out for, at the ascension levels that
/// move the elite count.
const CASES: [(&str, usize, u8); 8] = [
    ("N3Z9EE2CXB", 0, 0),
    ("N3Z9EE2CXB", 1, 0),
    ("N3Z9EE2CXB", 2, 0),
    ("N3Z9EE2CXB", 1, 10),
    ("QEY5K1P4LY", 0, 0),
    ("QEY5K1P4LY", 1, 0),
    ("R8F8NFB8Y1", 2, 0),
    ("PCJZ1E6BT4", 1, 1),
];

fn point_type_at(map: &ActMap, coord: MapCoord) -> MapPointType {
    map.points
        .iter()
        .find(|point| point.coord == coord)
        .unwrap_or_else(|| panic!("map has a point at {coord:?}"))
        .point_type
}

#[test]
fn the_golden_path_is_one_column_of_sixteen_fixed_points() {
    let map = golden_path_act_map();
    map.validate().expect("golden path map is internally valid");

    assert_eq!(map.width, 7);
    assert_eq!(map.height, 17);
    assert_eq!(map.second_boss, None);
    assert_eq!(map.start, MapCoord { col: 3, row: 0 });
    assert_eq!(map.boss, MapCoord { col: 3, row: 17 });
    assert!(
        map.points.iter().all(|point| point.coord.col == 3),
        "only the centre column is populated"
    );
    assert!(
        map.points.iter().all(|point| point.can_modify),
        "the golden path pins nothing"
    );

    assert_eq!(point_type_at(&map, map.start), MapPointType::Ancient);
    assert_eq!(point_type_at(&map, map.boss), MapPointType::Boss);

    let column: Vec<_> = (1..=16)
        .map(|row| point_type_at(&map, MapCoord { col: 3, row }))
        .collect();
    assert_eq!(
        column,
        vec![
            MapPointType::Monster,
            MapPointType::Unknown,
            MapPointType::Monster,
            MapPointType::RestSite,
            MapPointType::Monster,
            MapPointType::RestSite,
            MapPointType::Unknown,
            MapPointType::Treasure,
            MapPointType::Unknown,
            MapPointType::Treasure,
            MapPointType::Unknown,
            MapPointType::Shop,
            MapPointType::Elite,
            MapPointType::RestSite,
            MapPointType::Elite,
            MapPointType::RestSite,
        ]
    );
}

/// Nothing branches: every point but the boss leads to exactly the point above
/// it, and the boss stands one row behind the last of them.
#[test]
fn the_golden_path_walks_straight_into_the_boss() {
    let map = golden_path_act_map();
    for point in &map.points {
        let expected = if point.coord == map.boss {
            Vec::new()
        } else {
            vec![MapCoord {
                col: 3,
                row: point.coord.row + 1,
            }]
        };
        assert_eq!(point.children, expected, "at {:?}", point.coord);
    }
    assert_eq!(
        map.destinations(Some(MapCoord { col: 3, row: 16 })),
        [map.boss]
    );
}

/// The treasure row is the map's row count less seven, and it is the whole
/// point of the map:
/// no other point stands on that row, so no route from the start to the boss
/// can reach the boss without stepping onto the treasure.
#[test]
fn every_spoils_path_runs_through_the_one_centred_treasure() {
    for (seed, act, ascension) in CASES {
        let map = spoils_act_map(seed, act, ascension);
        map.validate()
            .unwrap_or_else(|error| panic!("spoils map for {seed} act {act} is valid: {error:?}"));
        assert_eq!(map.width, 7, "the spoils map is seven columns wide");
        assert_eq!(map.second_boss, None);

        let treasure_row = map.height - 7;
        let waist: Vec<_> = map
            .points
            .iter()
            .filter(|point| point.coord.row == treasure_row)
            .collect();
        assert_eq!(
            waist.len(),
            1,
            "seed {seed} act {act} narrows to one point on row {treasure_row}"
        );
        let treasure = waist[0];
        assert_eq!(treasure.coord.col, 3, "the waist sits on the centre column");
        assert_eq!(treasure.point_type, MapPointType::Treasure);
        assert!(
            !treasure.can_modify,
            "a point that cannot be modified keeps the treasure where it is"
        );
        assert!(
            !treasure.children.is_empty(),
            "the treasure leads on toward the boss"
        );
        assert!(
            map.points
                .iter()
                .any(|point| point.children.contains(&treasure.coord)),
            "the treasure is reachable from the row below it"
        );
        assert_eq!(
            map.points
                .iter()
                .filter(|point| point.point_type == MapPointType::Treasure)
                .count(),
            1,
            "the hourglass holds exactly one treasure"
        );

        // Every point stands on a route from the start to the boss, so the
        // single waist point is on all of them.
        let mut reachable = BTreeSet::from([map.start]);
        for row in 0..=map.height {
            let stepping_off: Vec<_> = map
                .points
                .iter()
                .filter(|point| point.coord.row == row && reachable.contains(&point.coord))
                .flat_map(|point| point.children.clone())
                .collect();
            reachable.extend(stepping_off);
        }
        assert!(
            reachable.contains(&treasure.coord),
            "seed {seed} act {act} reaches the treasure"
        );
        assert!(
            reachable.contains(&map.boss),
            "seed {seed} act {act} reaches the boss"
        );
    }
}

#[test]
fn the_spoils_map_is_the_same_map_every_time_a_seed_asks_for_it() {
    for (seed, act, ascension) in CASES {
        assert_eq!(
            spoils_act_map(seed, act, ascension),
            spoils_act_map(seed, act, ascension),
            "seed {seed} act {act} at ascension {ascension}"
        );
    }
}

/// The stream is the run's own seed under the name `spoils_map`, which belongs
/// to the run rather than to an act, so two acts of one seed are handed the
/// same
/// dice and differ only in what their lengths and point counts do with them.
#[test]
fn the_spoils_map_reads_a_stream_of_its_own() {
    let first = spoils_act_map("N3Z9EE2CXB", 0, 0);
    let second = spoils_act_map("R8F8NFB8Y1", 0, 0);
    assert_ne!(first, second, "a different seed lays out a different map");
    assert_eq!(first.height, 16, "act one is sixteen rows of grid");
    assert_eq!(spoils_act_map("N3Z9EE2CXB", 1, 0).height, 15);
    assert_eq!(spoils_act_map("N3Z9EE2CXB", 2, 0).height, 14);
}
