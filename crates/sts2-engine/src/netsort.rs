//! The runtime's list sort, reproduced.
//!
//! A pile is sorted before it is shuffled, so that the shuffle does not depend
//! on the order the pile happened to be in. That sort is the runtime's own, and
//! it is not stable: two cards that compare equal come out in whatever order
//! its introsort leaves them. Which physical card ends up where therefore
//! depends on the exact walk, so the walk is what is reproduced here.

use std::cmp::Ordering;

/// Sorts in place the way the runtime does.
pub fn sort<T: Clone>(keys: &mut [T], compare: &impl Fn(&T, &T) -> Ordering) {
    if keys.len() > 1 {
        let depth_limit = 2 * (usize::BITS - keys.len().leading_zeros()) as usize;
        intro_sort(keys, depth_limit, compare);
    }
}

fn intro_sort<T: Clone>(
    keys: &mut [T],
    mut depth_limit: usize,
    compare: &impl Fn(&T, &T) -> Ordering,
) {
    let mut size = keys.len();
    while size > 1 {
        // Small partitions finish by hand, and the two smallest are unrolled.
        if size <= 16 {
            match size {
                2 => swap_if_greater(keys, 0, 1, compare),
                3 => {
                    swap_if_greater(keys, 0, 1, compare);
                    swap_if_greater(keys, 0, 2, compare);
                    swap_if_greater(keys, 1, 2, compare);
                }
                _ => insertion_sort(&mut keys[..size], compare),
            }
            return;
        }
        // A partitioning that keeps going nowhere falls back to a heap sort.
        if depth_limit == 0 {
            heap_sort(&mut keys[..size], compare);
            return;
        }
        depth_limit -= 1;
        let pivot = partition(&mut keys[..size], compare);
        intro_sort(&mut keys[pivot + 1..size], depth_limit, compare);
        size = pivot;
    }
}

/// Median of three, then a two-sided scan around the pivot.
fn partition<T>(keys: &mut [T], compare: &impl Fn(&T, &T) -> Ordering) -> usize {
    let hi = keys.len() - 1;
    let middle = hi >> 1;
    swap_if_greater(keys, 0, middle, compare);
    swap_if_greater(keys, 0, hi, compare);
    swap_if_greater(keys, middle, hi, compare);
    // The pivot sits one short of the end, where neither scan can move it.
    keys.swap(middle, hi - 1);
    let pivot = hi - 1;
    let mut left = 0;
    let mut right = hi - 1;
    while left < right {
        while left < hi {
            left += 1;
            if compare(&keys[pivot], &keys[left]) != Ordering::Greater {
                break;
            }
        }
        while right > 0 {
            right -= 1;
            if compare(&keys[pivot], &keys[right]) != Ordering::Less {
                break;
            }
        }
        if left >= right {
            break;
        }
        keys.swap(left, right);
    }
    if left != hi - 1 {
        keys.swap(left, hi - 1);
    }
    left
}

fn insertion_sort<T: Clone>(keys: &mut [T], compare: &impl Fn(&T, &T) -> Ordering) {
    for index in 1..keys.len() {
        let held = keys[index].clone();
        let mut slot = index;
        while slot > 0 && compare(&held, &keys[slot - 1]) == Ordering::Less {
            keys[slot] = keys[slot - 1].clone();
            slot -= 1;
        }
        keys[slot] = held;
    }
}

fn heap_sort<T: Clone>(keys: &mut [T], compare: &impl Fn(&T, &T) -> Ordering) {
    let length = keys.len();
    for index in (1..=length / 2).rev() {
        sift_down(keys, index, length, compare);
    }
    for index in (2..=length).rev() {
        keys.swap(0, index - 1);
        sift_down(keys, 1, index - 1, compare);
    }
}

/// One-based, as the runtime writes it.
fn sift_down<T: Clone>(
    keys: &mut [T],
    mut index: usize,
    length: usize,
    compare: &impl Fn(&T, &T) -> Ordering,
) {
    let held = keys[index - 1].clone();
    while index <= length / 2 {
        let mut child = 2 * index;
        if child < length && compare(&keys[child - 1], &keys[child]) == Ordering::Less {
            child += 1;
        }
        if compare(&keys[child - 1], &held) != Ordering::Greater {
            break;
        }
        keys[index - 1] = keys[child - 1].clone();
        index = child;
    }
    keys[index - 1] = held;
}

fn swap_if_greater<T>(
    keys: &mut [T],
    left: usize,
    right: usize,
    compare: &impl Fn(&T, &T) -> Ordering,
) {
    if left != right && compare(&keys[left], &keys[right]) == Ordering::Greater {
        keys.swap(left, right);
    }
}

#[cfg(test)]
mod tests {
    use super::sort;

    /// Sorting is a sort, whatever order it leaves equal elements in.
    #[test]
    fn every_length_comes_out_ordered() {
        for length in 0..64_usize {
            let mut values: Vec<usize> = (0..length).map(|index| (index * 7) % 11).collect();
            sort(&mut values, &usize::cmp);
            assert!(values.windows(2).all(|pair| pair[0] <= pair[1]));
        }
    }
}
