use std::cmp::Ordering;
use std::collections::{BTreeMap, VecDeque};

use sts2_engine::{ActMap, MapCoord, MapPoint, MapPointType};
use sts2_rng::{Rng, deterministic_hash};

const WIDTH: usize = 7;
const PATH_COUNT: usize = 7;

/// The fixed room types of the Golden Path map, from the row above the start
/// up to the row below the boss. A multiplayer lobby drops the entry at index
/// two; a single-player run keeps all sixteen.
const GOLDEN_PATH_POINT_TYPES: [MapPointType; 16] = [
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
];

/// What an act's map is shaped by: how long it is, how many rest sites it
/// carries, and how many unknown points.
struct ActShape {
    length: usize,
    rest_count: usize,
    unknown_count: usize,
}

/// How long an act is and what it is made of, which is the first thing every
/// map generator rolls on the stream it is handed. Only the rest and unknown
/// counts come off the dice; the length is the act's own room count plus the
/// boss row, and single-player never takes off the one room multiplayer does.
fn act_shape(act: usize, rng: &mut Rng) -> ActShape {
    let standard_unknowns = |rng: &mut Rng| rng.next_gaussian_int(12, 1, 10, 14) as usize;
    match act {
        0 => {
            let rest_count = rng.next_gaussian_int(7, 1, 6, 7) as usize;
            ActShape {
                length: 16,
                rest_count,
                unknown_count: standard_unknowns(rng),
            }
        }
        1 => {
            let rest_count = rng.next_gaussian_int(6, 1, 6, 7) as usize;
            ActShape {
                length: 15,
                rest_count,
                unknown_count: standard_unknowns(rng) - 1,
            }
        }
        _ => {
            let rest_count = rng
                .next_int_range(5, 7)
                .expect("ordered rest-count range is valid") as usize;
            ActShape {
                length: 14,
                rest_count,
                unknown_count: standard_unknowns(rng) - 1,
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
enum PointType {
    Unassigned,
    Unknown,
    Shop,
    Treasure,
    RestSite,
    Monster,
    Elite,
    Boss,
    Ancient,
}

#[derive(Clone, Debug)]
struct Node {
    col: usize,
    row: usize,
    point_type: PointType,
    can_modify: bool,
    parents: Vec<usize>,
    children: Vec<usize>,
}

impl Node {
    fn new(col: usize, row: usize) -> Self {
        Self {
            col,
            row,
            point_type: PointType::Unassigned,
            can_modify: true,
            parents: Vec::new(),
            children: Vec::new(),
        }
    }
}

/// The grid, the points standing on it, and the dice that place them: the
/// substrate the standard map and the spoils map are both built on. The two
/// lay their paths out differently and only one of them post-processes the
/// result, but they share the point-type queue, the placement rules, and the
/// whole of the prune-and-repair pass.
struct MapBuilder {
    rng: Rng,
    grid: Vec<Vec<Option<usize>>>,
    nodes: Vec<Node>,
    start: usize,
    boss: usize,
    /// The second boss's own map point, laid out a row past the boss for an
    /// act that holds a second one.
    second_boss: Option<usize>,
    start_points: Vec<usize>,
    rest_count: usize,
    elite_count: usize,
    unknown_count: usize,
    length: usize,
}

impl MapBuilder {
    /// The opening both generators share: roll the act's shape off the stream
    /// the generator was handed, then lay out the empty grid with the starting
    /// point under it and the boss point a row past its top.
    fn new(mut rng: Rng, act: usize, ascension: u8, has_second_boss: bool) -> Self {
        let shape = act_shape(act, &mut rng);
        let length = shape.length;
        let mut nodes = vec![Node::new(3, 0), Node::new(3, length)];
        let second_boss = has_second_boss.then(|| {
            nodes.push(Node::new(3, length + 1));
            nodes.len() - 1
        });
        Self {
            rng,
            grid: vec![vec![None; length]; WIDTH],
            nodes,
            start: 0,
            boss: 1,
            second_boss,
            start_points: Vec::new(),
            rest_count: shape.rest_count,
            // How many elite points a map carries, which Swarming Elites
            // raises from five to eight.
            elite_count: sts2_engine::elite_map_point_count(ascension),
            unknown_count: shape.unknown_count,
            length,
        }
    }

    fn generate_standard(mut self) -> ActMap {
        self.generate_paths();
        self.assign_point_types();
        self.prune_and_repair();
        self.center_grid();
        self.spread_adjacent_points();
        self.straighten_paths();
        self.into_act_map()
    }

    /// The spoils map once its point counts are rolled: lay out the hourglass,
    /// assign the point types, then prune and repair. Nothing post-processes
    /// the grid, so the columns the hourglass narrowed to stay where the paths
    /// left them.
    fn generate_hourglass(mut self) -> ActMap {
        let treasure_row = self.length - 7;
        self.generate_hourglass_map(treasure_row);
        self.assign_hourglass_point_types();
        self.prune_and_repair();
        self.into_act_map()
    }

    fn create_point(&mut self, col: usize, row: usize) -> usize {
        if let Some(id) = self.grid[col][row] {
            return id;
        }
        let id = self.nodes.len();
        self.nodes.push(Node::new(col, row));
        self.grid[col][row] = Some(id);
        id
    }

    fn add_child(&mut self, parent: usize, child: usize) {
        if !self.nodes[parent].children.contains(&child) {
            self.nodes[parent].children.push(child);
        }
        if !self.nodes[child].parents.contains(&parent) {
            self.nodes[child].parents.push(parent);
        }
    }

    fn remove_child(&mut self, parent: usize, child: usize) {
        self.nodes[parent]
            .children
            .retain(|candidate| *candidate != child);
        self.nodes[child]
            .parents
            .retain(|candidate| *candidate != parent);
    }

    /// The seven-path opening both maps are laid out with: roll seven row-one
    /// starts, force the second one onto a column the first did not take, and
    /// walk a path up from each.
    fn generate_start_paths(&mut self, treasure_row: Option<usize>) {
        for iteration in 0..PATH_COUNT {
            let mut col = self.rng.next_int(7).expect("constant range") as usize;
            let mut point = self.create_point(col, 1);
            if iteration == 1 {
                while self.start_points.contains(&point) {
                    col = self.rng.next_int(7).expect("constant range") as usize;
                    point = self.create_point(col, 1);
                }
            }
            if !self.start_points.contains(&point) {
                self.start_points.push(point);
            }
            self.path_generate(point, treasure_row);
        }
    }

    fn generate_paths(&mut self) {
        self.generate_start_paths(None);

        for point in self.points_in_row(self.length - 1) {
            self.add_child(point, self.boss);
        }
        // Where the act has a second boss it stands behind the first, and the
        // only way onto it is through the fight in front of it.
        if let Some(second_boss) = self.second_boss {
            self.add_child(self.boss, second_boss);
        }
        let start_points = self.points_in_row(1);
        for point in start_points {
            self.add_child(self.start, point);
        }
    }

    /// One path in either map: step from a row-one start up to the last row,
    /// creating each point the walk lands on. The treasure row is what tells
    /// the two apart — the spoils map steers its steps toward the treasure it
    /// is about to place, and the standard map has none to steer to.
    fn path_generate(&mut self, mut current: usize, treasure_row: Option<usize>) {
        while self.nodes[current].row < self.length - 1 {
            let next = match treasure_row {
                Some(treasure_row) => self.generate_hourglass_coord(current, treasure_row),
                None => self.generate_next_coord(current),
            };
            let child = self.create_point(next.0, next.1);
            self.add_child(current, child);
            current = child;
        }
    }

    fn generate_next_coord(&mut self, current: usize) -> (usize, usize) {
        let col = self.nodes[current].col;
        let row = self.nodes[current].row + 1;
        let mut offsets = [-1_i8, 0, 1];
        self.rng.shuffle(&mut offsets);
        for offset in offsets {
            let target = match offset {
                -1 => col.saturating_sub(1),
                0 => col,
                1 => (col + 1).min(WIDTH - 1),
                _ => unreachable!(),
            };
            if !self.has_invalid_crossover(current, target) {
                return (target, row);
            }
        }
        panic!("pinned map generation could not find a next node")
    }

    fn has_invalid_crossover(&self, current: usize, target_col: usize) -> bool {
        let current_col = self.nodes[current].col;
        if current_col == target_col {
            return false;
        }
        let Some(neighbor) = self.grid[target_col][self.nodes[current].row] else {
            return false;
        };
        let direction = target_col.cmp(&current_col);
        self.nodes[neighbor].children.iter().any(|child| {
            self.nodes[*child].col.cmp(&self.nodes[neighbor].col) == direction.reverse()
        })
    }

    /// The spoils map's layout: the usual seven paths, then the one treasure
    /// point in the middle of the treasure row, then every other point
    /// that landed on that row folded into it, and finally the top row wired to
    /// the boss and the first row wired to the start.
    fn generate_hourglass_map(&mut self, treasure_row: usize) {
        assert!(
            treasure_row > 0 && treasure_row < self.length,
            "treasure row is out of bounds for the spoils map"
        );
        self.generate_start_paths(Some(treasure_row));

        let treasure = self.create_point(WIDTH / 2, treasure_row);
        self.nodes[treasure].point_type = PointType::Treasure;
        self.nodes[treasure].can_modify = false;
        for stray in self.points_in_row(treasure_row) {
            if stray != treasure {
                self.redirect_to_treasure(stray, treasure);
            }
        }

        for point in self.points_in_row(self.length - 1) {
            self.add_child(point, self.boss);
        }
        let start_points = self.points_in_row(1);
        for point in start_points {
            self.add_child(self.start, point);
        }
    }

    /// A point that landed on the treasure row beside the treasure hands its
    /// parents and its children over to the treasure and leaves the grid,
    /// which is what makes the waist of the hourglass one point wide.
    fn redirect_to_treasure(&mut self, stray: usize, treasure: usize) {
        for parent in self.nodes[stray].parents.clone() {
            self.remove_child(parent, stray);
            self.add_child(parent, treasure);
        }
        for child in self.nodes[stray].children.clone() {
            self.remove_child(stray, child);
            self.add_child(treasure, child);
        }
        let (col, row) = (self.nodes[stray].col, self.nodes[stray].row);
        self.grid[col][row] = None;
    }

    /// The step the spoils map takes from one point to the next. Three things
    /// separate it from the standard step. The row being stepped into carries
    /// a column window that closes to the centre column at the treasure row
    /// and to three columns at the top. Within three rows below the treasure
    /// the offsets are tried in the centred priority order instead of a
    /// shuffled one, so those rows read no dice at all. And a step may not
    /// hand a point a fourth parent or take a fourth child. When no offset
    /// passes all of that, the step falls back to a single column toward the
    /// centre.
    fn generate_hourglass_coord(&mut self, current: usize, treasure_row: usize) -> (usize, usize) {
        let col = self.nodes[current].col;
        let row = self.nodes[current].row + 1;
        let (min_col, max_col) = self.allowed_columns_for_row(row, treasure_row);
        let below_treasure = self.nodes[current].row < treasure_row;
        let offsets = if below_treasure && treasure_row - self.nodes[current].row <= 3 {
            Self::centered_priority(col)
        } else {
            let mut offsets = [-1_i8, 0, 1];
            self.rng.shuffle(&mut offsets);
            offsets
        };
        for offset in offsets {
            let target = match offset {
                -1 => col.saturating_sub(1),
                0 => col,
                1 => (col + 1).min(WIDTH - 1),
                _ => unreachable!(),
            };
            if target < min_col || target > max_col || self.has_invalid_crossover(current, target) {
                continue;
            }
            let existing = self.grid[target][row];
            let takes_a_parent = existing.is_none_or(|point| {
                self.nodes[point].parents.contains(&current) || self.nodes[point].parents.len() < 3
            });
            let takes_a_child = current == self.start
                || self.nodes[current].children.len() < 3
                || existing.is_some_and(|point| self.nodes[current].children.contains(&point));
            if takes_a_parent && takes_a_child {
                return (target, row);
            }
        }
        (self.fallback_column(current, min_col, max_col), row)
    }

    /// The order the offsets are tried in: step toward the centre column
    /// first, then stand still, then step away from it. A point already on the
    /// centre column stands still first, then tries left, then right.
    fn centered_priority(col: usize) -> [i8; 3] {
        match col.cmp(&(WIDTH / 2)) {
            Ordering::Less => [1, 0, -1],
            Ordering::Greater => [-1, 0, 1],
            Ordering::Equal => [0, -1, 1],
        }
    }

    /// How far from the centre column a row is allowed to spread. The window
    /// is the row's distance from the treasure row, capped by the centre
    /// column itself and by how close the row is to the top, so the map closes
    /// to one column at the treasure and to three under the boss.
    fn allowed_columns_for_row(&self, row: usize, treasure_row: usize) -> (usize, usize) {
        let center = WIDTH / 2;
        let rows_to_top = (self.length - 1).saturating_sub(row);
        let spread = center.min(row.abs_diff(treasure_row)).min(rows_to_top + 1);
        (center - spread, center + spread)
    }

    /// What a step settles for when no offset was allowed: aim at the centre
    /// column, move at most one column toward it, and stay on the current
    /// column instead if that step would cross another path.
    fn fallback_column(&self, current: usize, min_col: usize, max_col: usize) -> usize {
        let col = self.nodes[current].col;
        let mut target = (WIDTH / 2).clamp(min_col, max_col);
        if target.abs_diff(col) > 1 {
            target = if target > col {
                col + 1
            } else {
                col.saturating_sub(1)
            }
            .clamp(min_col, max_col);
        }
        if self.has_invalid_crossover(current, target) {
            target = col.clamp(min_col, max_col);
        }
        target
    }

    /// The standard map's point types: the top row is always a rest site, the
    /// row seven below it always a treasure, and the first row always a
    /// monster, and none of the three may be modified afterwards.
    fn assign_point_types(&mut self) {
        self.pin_row(self.length - 1, PointType::RestSite);
        self.pin_row(self.length - 7, PointType::Treasure);
        self.pin_row(1, PointType::Monster);
        self.assign_queued_point_types();
    }

    /// The spoils map's point types, which pin the same top and first rows but
    /// not a treasure row: laying out the hourglass already placed the one
    /// treasure every path runs through.
    fn assign_hourglass_point_types(&mut self) {
        self.pin_row(self.length - 1, PointType::RestSite);
        self.pin_row(1, PointType::Monster);
        self.assign_queued_point_types();
    }

    /// Pins a whole row to one type that neither the later assignment passes
    /// nor anything else that rewrites a map may move.
    fn pin_row(&mut self, row: usize, point_type: PointType) {
        for point in self.points_in_row(row) {
            self.nodes[point].point_type = point_type;
            self.nodes[point].can_modify = false;
        }
    }

    /// What both maps do once their fixed rows stand: queue the act's rest
    /// sites, its three shops, its elites, and its
    /// unknown points, spend the queue over the free points, turn whatever is
    /// still unassigned into a monster, and name the start, the boss, and the
    /// second boss when the act holds one.
    fn assign_queued_point_types(&mut self) {
        let mut queue = VecDeque::new();
        queue.extend(std::iter::repeat_n(PointType::RestSite, self.rest_count));
        queue.extend(std::iter::repeat_n(PointType::Shop, 3));
        queue.extend(std::iter::repeat_n(PointType::Elite, self.elite_count));
        queue.extend(std::iter::repeat_n(PointType::Unknown, self.unknown_count));
        self.assign_remaining_types(&mut queue);

        for point in self.all_grid_points() {
            if self.nodes[point].point_type == PointType::Unassigned {
                self.nodes[point].point_type = PointType::Monster;
            }
        }
        self.nodes[self.start].point_type = PointType::Ancient;
        self.nodes[self.boss].point_type = PointType::Boss;
        if let Some(second_boss) = self.second_boss {
            self.nodes[second_boss].point_type = PointType::Boss;
        }
    }

    fn assign_remaining_types(&mut self, queue: &mut VecDeque<PointType>) {
        for _ in 0..3 {
            if queue.is_empty() {
                break;
            }
            let mut points: Vec<_> = self
                .all_grid_points()
                .into_iter()
                .filter(|point| self.nodes[*point].point_type == PointType::Unassigned)
                .collect();
            self.stable_shuffle_points(&mut points);
            for point in points {
                if queue.is_empty() {
                    break;
                }
                self.nodes[point].point_type = self.next_valid_point_type(queue, point);
            }
        }
    }

    fn next_valid_point_type(
        &mut self,
        queue: &mut VecDeque<PointType>,
        point: usize,
    ) -> PointType {
        for _ in 0..queue.len() {
            let point_type = queue.pop_front().expect("bounded by queue length");
            if self.is_valid_point_type(point_type, point) {
                return point_type;
            }
            queue.push_back(point_type);
        }
        PointType::Unassigned
    }

    fn is_valid_point_type(&self, point_type: PointType, point: usize) -> bool {
        let node = &self.nodes[point];
        if node.row >= self.length - 3 && point_type == PointType::RestSite {
            return false;
        }
        if node.row < 6 && matches!(point_type, PointType::RestSite | PointType::Elite) {
            return false;
        }
        if matches!(
            point_type,
            PointType::Elite | PointType::RestSite | PointType::Treasure | PointType::Shop
        ) && node
            .parents
            .iter()
            .chain(&node.children)
            .any(|other| self.nodes[*other].point_type == point_type)
        {
            return false;
        }
        if matches!(
            point_type,
            PointType::RestSite
                | PointType::Monster
                | PointType::Unknown
                | PointType::Elite
                | PointType::Shop
        ) {
            for parent in &node.parents {
                if self.nodes[*parent].children.iter().any(|sibling| {
                    *sibling != point && self.nodes[*sibling].point_type == point_type
                }) {
                    return false;
                }
            }
        }
        true
    }

    fn prune_and_repair(&mut self) {
        for _ in 0..3 {
            self.prune_duplicate_segments();
            if !self.repair_pruned_point_types() {
                break;
            }
        }
    }

    fn prune_duplicate_segments(&mut self) {
        let mut iterations = 0;
        let mut matching = self.find_matching_segments();
        while self.prune_paths(&mut matching) {
            iterations += 1;
            assert!(iterations <= 50, "pinned map pruning did not converge");
            matching = self.find_matching_segments();
        }
    }

    fn find_matching_segments(&self) -> Vec<Vec<Vec<usize>>> {
        let mut segments: BTreeMap<String, Vec<Vec<usize>>> = BTreeMap::new();
        for path in self.find_all_paths(self.start) {
            for start_index in 0..path.len() - 1 {
                let start = path[start_index];
                if self.nodes[start].children.len() <= 1 && self.nodes[start].row != 0 {
                    continue;
                }
                for distance in 2..path.len() - start_index {
                    let end = path[start_index + distance];
                    if self.nodes[end].parents.len() < 2 {
                        continue;
                    }
                    let segment = path[start_index..=start_index + distance].to_vec();
                    let key = self.segment_key(&segment);
                    let candidates = segments.entry(key).or_default();
                    if !candidates
                        .iter()
                        .any(|existing| Self::segments_overlap(existing, &segment))
                    {
                        candidates.push(segment);
                    }
                }
            }
        }
        segments
            .into_values()
            .filter(|segments| segments.len() > 1)
            .collect()
    }

    fn find_all_paths(&self, current: usize) -> Vec<Vec<usize>> {
        if self.nodes[current].point_type == PointType::Boss {
            return vec![vec![current]];
        }
        let mut result = Vec::new();
        for child in &self.nodes[current].children {
            for path in self.find_all_paths(*child) {
                let mut with_current = Vec::with_capacity(path.len() + 1);
                with_current.push(current);
                with_current.extend(path);
                result.push(with_current);
            }
        }
        result
    }

    fn segment_key(&self, segment: &[usize]) -> String {
        let first = &self.nodes[segment[0]];
        let last = &self.nodes[*segment.last().expect("segment is nonempty")];
        let prefix = if first.row == 0 {
            format!("{}-{},{}-", first.row, last.col, last.row)
        } else {
            format!("{},{}-{},{}-", first.col, first.row, last.col, last.row)
        };
        let types = segment
            .iter()
            .map(|point| (self.nodes[*point].point_type as u8).to_string())
            .collect::<Vec<_>>()
            .join(",");
        prefix + &types
    }

    fn segments_overlap(left: &[usize], right: &[usize]) -> bool {
        if left.len() < 3 || right.len() < 3 {
            return false;
        }
        (1..=left.len() - 2).any(|index| left[index] == right[index])
    }

    fn prune_paths(&mut self, groups: &mut [Vec<Vec<usize>>]) -> bool {
        for group in groups {
            self.rng.shuffle(group);
            if self.prune_all_but_last(group) != 0 {
                return true;
            }
            if self.break_parent_child_relationship(group) {
                return true;
            }
        }
        false
    }

    fn prune_all_but_last(&mut self, matches: &[Vec<usize>]) -> usize {
        let mut removed = 0;
        for segment in matches {
            if removed == matches.len() - 1 {
                return removed;
            }
            if self.prune_segment(segment) {
                removed += 1;
            }
        }
        removed
    }

    fn prune_segment(&mut self, segment: &[usize]) -> bool {
        let mut result = false;
        for index in 0..segment.len() - 1 {
            let point = segment[index];
            if !self.is_in_map(point) {
                return true;
            }
            let node = &self.nodes[point];
            if node.children.len() > 1
                || node.parents.len() > 1
                || node.parents.iter().any(|parent| {
                    self.nodes[*parent].children.len() == 1 && !self.is_removed(*parent)
                })
            {
                continue;
            }
            let has_later_split = segment[index..].iter().any(|candidate| {
                self.nodes[*candidate].children.len() > 1
                    && self.nodes[*candidate].parents.len() == 1
            });
            if !has_later_split {
                if self.nodes[*segment.last().expect("segment is nonempty")]
                    .parents
                    .len()
                    == 1
                {
                    return false;
                }
                let has_exclusive_external_child = self.nodes[point]
                    .children
                    .iter()
                    .any(|child| !segment.contains(child) && self.nodes[*child].parents.len() == 1);
                if !has_exclusive_external_child {
                    self.remove_point(point);
                    result = true;
                }
            }
        }
        result
    }

    fn remove_point(&mut self, point: usize) {
        let (col, row) = (self.nodes[point].col, self.nodes[point].row);
        if col < WIDTH && row < self.length {
            self.grid[col][row] = None;
        }
        self.start_points.retain(|candidate| *candidate != point);
        let children = self.nodes[point].children.clone();
        let parents = self.nodes[point].parents.clone();
        for child in children {
            self.remove_child(point, child);
        }
        for parent in parents {
            self.remove_child(parent, point);
        }
    }

    fn is_in_map(&self, point: usize) -> bool {
        let node = &self.nodes[point];
        if node.point_type == PointType::Ancient {
            return true;
        }
        if node.point_type == PointType::Boss {
            return true;
        }
        self.grid[node.col][node.row].is_some()
    }

    fn is_removed(&self, point: usize) -> bool {
        let node = &self.nodes[point];
        self.grid[node.col][node.row].is_none()
    }

    fn break_parent_child_relationship(&mut self, matches: &[Vec<usize>]) -> bool {
        for segment in matches {
            let mut changed = false;
            for index in 0..segment.len() - 1 {
                let point = segment[index];
                let child = segment[index + 1];
                if self.nodes[point].children.len() >= 2 && self.nodes[child].parents.len() != 1 {
                    self.remove_child(point, child);
                    changed = true;
                }
            }
            if changed {
                return true;
            }
        }
        false
    }

    fn repair_pruned_point_types(&mut self) -> bool {
        let mut repaired = false;
        for (point_type, target) in [
            (PointType::Shop, 3),
            (PointType::Elite, self.elite_count),
            (PointType::RestSite, self.rest_count),
            (PointType::Unknown, self.unknown_count),
        ] {
            repaired |= self.repair_point_type(point_type, target);
        }
        repaired
    }

    fn repair_point_type(&mut self, point_type: PointType, target: usize) -> bool {
        let present = self
            .all_grid_points()
            .into_iter()
            .filter(|point| self.nodes[*point].point_type == point_type)
            .count();
        let mut missing = target.saturating_sub(present);
        if missing == 0 {
            return false;
        }
        let mut candidates: Vec<_> = self
            .all_grid_points()
            .into_iter()
            .filter(|point| {
                self.nodes[*point].point_type == PointType::Monster && self.nodes[*point].can_modify
            })
            .collect();
        self.stable_shuffle_points(&mut candidates);
        let mut repaired = false;
        for point in candidates {
            if missing == 0 {
                break;
            }
            if self.is_valid_point_type(point_type, point) {
                self.nodes[point].point_type = point_type;
                missing -= 1;
                repaired = true;
            }
        }
        repaired
    }

    fn stable_shuffle_points(&mut self, points: &mut [usize]) {
        points.sort_by_key(|point| (self.nodes[*point].col, self.nodes[*point].row));
        self.rng.shuffle(points);
    }

    fn center_grid(&mut self) {
        let left_empty = self.column_empty(0) && self.column_empty(1);
        let right_empty = self.column_empty(WIDTH - 1) && self.column_empty(WIDTH - 2);
        if !left_empty && right_empty {
            for row in 0..self.length {
                for col in (0..WIDTH).rev() {
                    let point = self.grid[col][row].take();
                    let target = col + 1;
                    if target < WIDTH {
                        self.grid[target][row] = point;
                        if let Some(point) = point {
                            self.nodes[point].col = target;
                        }
                    }
                }
            }
        } else if left_empty && !right_empty {
            for row in 0..self.length {
                for col in 0..WIDTH {
                    let point = self.grid[col][row].take();
                    if col > 0 {
                        self.grid[col - 1][row] = point;
                        if let Some(point) = point {
                            self.nodes[point].col = col - 1;
                        }
                    }
                }
            }
        }
    }

    fn spread_adjacent_points(&mut self) {
        for row in 0..self.length {
            let points = self.points_in_row(row);
            loop {
                let mut changed = false;
                for point in &points {
                    let col = self.nodes[*point].col;
                    let allowed = self.allowed_positions(*point);
                    let mut best_col = col;
                    let mut best_gap = self.compute_gap(col, &points, *point);
                    for candidate in allowed {
                        if candidate != col
                            && (self.grid[candidate][row].is_none()
                                || self.grid[candidate][row] == Some(*point))
                        {
                            let gap = self.compute_gap(candidate, &points, *point);
                            if gap > best_gap {
                                best_col = candidate;
                                best_gap = gap;
                            }
                        }
                    }
                    if best_col != col {
                        self.grid[col][row] = None;
                        self.grid[best_col][row] = Some(*point);
                        self.nodes[*point].col = best_col;
                        changed = true;
                    }
                }
                if !changed {
                    break;
                }
            }
        }
    }

    fn allowed_positions(&self, point: usize) -> Vec<usize> {
        let mut allowed = vec![true; WIDTH];
        for neighbor in self.nodes[point]
            .parents
            .iter()
            .chain(&self.nodes[point].children)
        {
            let col = self.nodes[*neighbor].col;
            for (candidate, is_allowed) in allowed.iter_mut().enumerate() {
                *is_allowed &= candidate.abs_diff(col) <= 1;
            }
        }
        allowed
            .into_iter()
            .enumerate()
            .filter_map(|(col, allowed)| allowed.then_some(col))
            .collect()
    }

    fn compute_gap(&self, candidate: usize, row_points: &[usize], current: usize) -> usize {
        row_points
            .iter()
            .filter(|point| **point != current)
            .map(|point| candidate.abs_diff(self.nodes[*point].col))
            .min()
            .unwrap_or(usize::MAX)
    }

    fn straighten_paths(&mut self) {
        for row in 0..self.length {
            for col in 0..WIDTH {
                let Some(point) = self.grid[col][row] else {
                    continue;
                };
                if self.nodes[point].parents.len() != 1 || self.nodes[point].children.len() != 1 {
                    continue;
                }
                let parent_col = self.nodes[self.nodes[point].parents[0]].col;
                let child_col = self.nodes[self.nodes[point].children[0]].col;
                let left_bend = col < parent_col && col < child_col;
                let right_bend = col > parent_col && col > child_col;
                if left_bend && col + 1 < WIDTH && self.grid[col + 1][row].is_none() {
                    self.grid[col][row] = None;
                    self.grid[col + 1][row] = Some(point);
                    self.nodes[point].col = col + 1;
                }
                if right_bend && col > 0 && self.grid[col - 1][row].is_none() {
                    self.grid[col][row] = None;
                    self.grid[col - 1][row] = Some(point);
                    self.nodes[point].col = col - 1;
                }
            }
        }
    }

    fn points_in_row(&self, row: usize) -> Vec<usize> {
        (0..WIDTH).filter_map(|col| self.grid[col][row]).collect()
    }

    fn all_grid_points(&self) -> Vec<usize> {
        let mut points = Vec::new();
        for col in 0..WIDTH {
            for row in 0..self.length {
                if let Some(point) = self.grid[col][row] {
                    points.push(point);
                }
            }
        }
        points
    }

    fn column_empty(&self, col: usize) -> bool {
        self.grid[col].iter().all(Option::is_none)
    }

    fn into_act_map(self) -> ActMap {
        let mut point_ids = vec![self.start];
        for row in 1..self.length {
            point_ids.extend((0..WIDTH).filter_map(|col| self.grid[col][row]));
        }
        point_ids.push(self.boss);
        point_ids.extend(self.second_boss);
        let points = point_ids
            .into_iter()
            .map(|point| {
                let node = &self.nodes[point];
                let mut children: Vec<_> = node
                    .children
                    .iter()
                    .map(|child| MapCoord {
                        col: self.nodes[*child].col as u8,
                        row: self.nodes[*child].row as u8,
                    })
                    .collect();
                children.sort();
                MapPoint {
                    coord: MapCoord {
                        col: node.col as u8,
                        row: node.row as u8,
                    },
                    point_type: match node.point_type {
                        PointType::Ancient => MapPointType::Ancient,
                        PointType::Monster => MapPointType::Monster,
                        PointType::Elite => MapPointType::Elite,
                        PointType::Unknown => MapPointType::Unknown,
                        PointType::Shop => MapPointType::Shop,
                        PointType::RestSite => MapPointType::RestSite,
                        PointType::Treasure => MapPointType::Treasure,
                        PointType::Boss => MapPointType::Boss,
                        PointType::Unassigned => unreachable!("all retained points are assigned"),
                    },
                    children,
                    can_modify: node.can_modify,
                }
            })
            .collect();
        ActMap {
            width: WIDTH as u8,
            height: self.length as u8,
            start: MapCoord { col: 3, row: 0 },
            boss: MapCoord {
                col: 3,
                row: self.length as u8,
            },
            second_boss: self.second_boss.map(|_| MapCoord {
                col: 3,
                row: (self.length + 1) as u8,
            }),
            points,
        }
    }
}

/// Generates the pinned-build single-player map for one act of `seed`. Each
/// act's map rolls on a stream of its own, so any act can be generated at any
/// time.
///
/// The two inputs beside the seed are the two the build lays a map out with:
/// the run's ascension, which sets how many elite points the map carries, and
/// whether this act holds a second boss. Neither moves the map's own stream,
/// so a map generated at one level draws the same dice as a map generated at
/// another and lays the answers out differently.
#[must_use]
pub fn standard_act_map(seed: &str, act: usize, ascension: u8, has_second_boss: bool) -> ActMap {
    let run_seed = deterministic_hash(seed) as u32;
    let rng = Rng::named(run_seed, &format!("act_{}_map", act + 1));
    MapBuilder::new(rng, act, ascension, has_second_boss).generate_standard()
}

/// The Golden Path: the fixed single-column map, which reads no dice at all
/// and so takes no seed. Its sixteen points are laid down in a fixed order,
/// and single-player keeps every one of them; only a multiplayer lobby drops
/// the third. The grid is still seven columns wide, but only column
/// three ever holds a point, so the map is one straight line from the Ancient
/// at the bottom to the boss a row past the top.
#[must_use]
pub fn golden_path_act_map() -> ActMap {
    let center = (WIDTH / 2) as u8;
    let row_count = GOLDEN_PATH_POINT_TYPES.len() + 1;
    let coord = |row: usize| MapCoord {
        col: center,
        row: row as u8,
    };
    // The start leads to the first point of the column, then each point in
    // the column takes the one above it as its child, and the top one leads to
    // the boss. Nothing on this map is pinned, so every point stays open to
    // being modified later.
    let mut points = vec![MapPoint {
        coord: coord(0),
        point_type: MapPointType::Ancient,
        children: vec![coord(1)],
        can_modify: true,
    }];
    points.extend(
        GOLDEN_PATH_POINT_TYPES
            .iter()
            .enumerate()
            .map(|(index, point_type)| MapPoint {
                coord: coord(index + 1),
                point_type: *point_type,
                children: vec![coord(index + 2)],
                can_modify: true,
            }),
    );
    points.push(MapPoint {
        coord: coord(row_count),
        point_type: MapPointType::Boss,
        children: Vec::new(),
        can_modify: true,
    });
    ActMap {
        width: WIDTH as u8,
        height: row_count as u8,
        start: coord(0),
        boss: coord(row_count),
        second_boss: None,
        points,
    }
}

/// The spoils map: the hourglass map an act lays out instead of its standard
/// one while the Spoils Map card is held. Every path runs through the single
/// centred treasure point seven rows below the boss, because the rows on
/// either side of it narrow until only the centre column is left and anything
/// that still lands beside the treasure is folded into it.
///
/// The map opens a stream of its own, named `spoils_map` off the run seed,
/// rather than borrowing the act's map stream, so it always starts from the
/// same place and leaves that act's own map stream where it stood. It spends
/// the stream in the build's order: the act's shape first, then the hourglass
/// layout, then the point types, then the prune-and-repair pass.
///
/// The act index picks the length and decides which rolls the act's shape
/// takes; the ascension sets how many elite points the map carries, and it
/// moves no dice at all.
#[must_use]
pub fn spoils_act_map(seed: &str, act: usize, ascension: u8) -> ActMap {
    let run_seed = deterministic_hash(seed) as u32;
    let rng = Rng::named(run_seed, "spoils_map");
    // A spoils map never carries a second boss, however the run is
    // configured.
    MapBuilder::new(rng, act, ascension, false).generate_hourglass()
}
