use stratum_core::services::ClearPlanner;

use crate::support::Volume;

#[test]
fn both_fluids_cleared_is_plain_fills_within_the_limit() {
    let parts = ClearPlanner::parts([0, 0, 0], [63, 15, 63], None);
    assert_eq!(parts.len(), 2);
    assert!(parts.iter().all(|p| p.kind.is_none() && Volume::of(p) <= ClearPlanner::FILL_LIMIT));
    assert_eq!(parts.iter().map(Volume::of).sum::<u64>(), 65_536);
}

#[test]
fn kept_fluid_gives_one_replace_per_kind_per_part() {
    let kinds = vec!["minecraft:stone".to_string(), "minecraft:dirt".to_string()];
    let parts = ClearPlanner::parts([-10, -64, -10], [-1, -60, -1], Some(&kinds));
    assert_eq!(parts.iter().filter(|p| p.kind.as_deref() == Some("minecraft:stone")).count(), 1);
    assert_eq!(parts.len(), 2);
}

#[test]
fn a_one_block_box_is_one_part() {
    let parts = ClearPlanner::parts([5, -3, 7], [5, -3, 7], None);
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].min, [5, -3, 7]);
    assert_eq!(parts[0].max, [5, -3, 7]);
}

#[test]
fn a_tall_box_splits_along_y_first() {
    let parts = ClearPlanner::parts([0, 0, 0], [0, 40_000, 0], None);
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].max, [0, 32_767, 0]);
    assert_eq!(parts[1].min, [0, 32_768, 0]);
    assert_eq!(parts[1].max, [0, 40_000, 0]);
}

#[test]
fn the_parts_cover_a_box_at_negative_coordinates_once() {
    let (min, max) = ([-30_000_100, -64, -17], [-29_999_800, 100, 90]);
    let parts = ClearPlanner::parts(min, max, None);
    let total: u64 = (0..3).map(|axis| (i64::from(max[axis]) - i64::from(min[axis]) + 1) as u64).product();
    assert_eq!(parts.iter().map(Volume::of).sum::<u64>(), total);
    assert!(parts.iter().all(|p| Volume::of(p) <= ClearPlanner::FILL_LIMIT));
    assert!(parts.iter().all(|p| (0..3).all(|axis| p.min[axis] >= min[axis] && p.max[axis] <= max[axis])));
}

#[test]
fn a_box_with_a_corner_below_the_other_has_no_parts() {
    assert!(ClearPlanner::parts([0, 5, 0], [3, 4, 3], None).is_empty());
}
