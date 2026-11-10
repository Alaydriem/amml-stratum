use std::sync::Arc;

use stratum_core::services::{GhostAssembler, GhostRay, ItemRules, PlaceNeeds};
use stratum_core::structs::easy_place::RayLimits;
use stratum_core::structs::ghost::{GhostBuild, GhostView, PlacedStructure};
use stratum_core::structs::marker::Rotation;

use crate::support::GhostFixture;

struct Rays;

impl Rays {
    fn stone(min: [i32; 3], size: i32) -> GhostBuild {
        GhostFixture::blocks(&[(min, [min[0] + size - 1, min[1] + size - 1, min[2] + size - 1])])
    }

    fn cast(
        build: &GhostBuild,
        view: GhostView,
        eye: [f64; 3],
        dir: [f64; 3],
        max_distance: f64,
    ) -> Option<([i32; 3], u16)> {
        let needs = PlaceNeeds::new(&build.palette, &ItemRules::new());
        GhostRay::first_spot(build, view, &needs, eye, dir, &RayLimits::reach(max_distance))
    }

    // A build of one 1 x 1 x 2 structure at the origin: water at z 0 and stone at z 1.
    fn water_then_stone() -> GhostBuild {
        let structure = GhostFixture::structure([1, 1, 2], &["water", "stone"]);
        let placed = [PlacedStructure { structure: Arc::clone(&structure), origin: [0, 0, 0] }];
        GhostAssembler::build(&placed, &[structure], [0, 0, 0], Rotation::R0, 1, None).unwrap()
    }
}

#[test]
fn layer_view_only_targets_the_shown_layer() {
    let build = Rays::stone([0, 0, 0], 3);
    let eye = [1.5, 10.5, 1.5];
    let down = [0.0, -1.0, 0.0];
    let hit = Rays::cast(&build, GhostView::Layer(1), eye, down, 20.0);
    assert_eq!(hit, Some(([1, 1, 1], 0)));
    let none = Rays::cast(&build, GhostView::Layer(7), eye, down, 20.0);
    assert_eq!(none, None);
}

#[test]
fn whole_view_targets_the_first_cell() {
    let build = Rays::stone([0, 0, 0], 3);
    let hit = Rays::cast(&build, GhostView::Whole, [1.5, 10.5, 1.5], [0.0, -1.0, 0.0], 20.0);
    assert_eq!(hit, Some(([1, 2, 1], 0)));
}

#[test]
fn a_ray_that_starts_inside_a_target_returns_that_cell() {
    let build = Rays::stone([0, 0, 0], 3);
    let hit = Rays::cast(&build, GhostView::Whole, [1.5, 1.5, 1.5], [0.0, 0.0, 1.0], 5.0);
    assert_eq!(hit, Some(([1, 1, 1], 0)));
}

#[test]
fn a_ray_along_x_hits_only_its_row() {
    let build = GhostFixture::blocks(&[([2, 0, 0], [2, 0, 0])]);
    let hit = Rays::cast(&build, GhostView::Whole, [0.5, 0.5, 0.5], [1.0, 0.0, 0.0], 5.0);
    assert_eq!(hit, Some(([2, 0, 0], 0)));
    let miss = Rays::cast(&build, GhostView::Whole, [0.5, 1.5, 0.5], [1.0, 0.0, 0.0], 5.0);
    assert_eq!(miss, None);
}

// From the centre of [0, 0, 0] towards the centre of [3, 2, 0] the ray crosses [0, 0], [1, 0],
// [1, 1], [2, 1], [2, 2] and [3, 2] in x and y. [1, 2] and [3, 1] lie beside the path.
#[test]
fn a_diagonal_ray_walks_cell_by_cell() {
    let eye = [0.5, 0.5, 0.5];
    let dir = [3.0, 2.0, 0.0];
    let beside = GhostFixture::blocks(&[([1, 2, 0], [1, 2, 0]), ([3, 1, 0], [3, 1, 0]), ([3, 2, 0], [3, 2, 0])]);
    assert_eq!(Rays::cast(&beside, GhostView::Whole, eye, dir, 10.0), Some(([3, 2, 0], 0)));
    let on_path = GhostFixture::blocks(&[([2, 1, 0], [2, 1, 0]), ([3, 2, 0], [3, 2, 0])]);
    assert_eq!(Rays::cast(&on_path, GhostView::Whole, eye, dir, 10.0), Some(([2, 1, 0], 0)));
}

#[test]
fn nothing_past_max_distance() {
    let build = GhostFixture::blocks(&[([0, 0, 5], [0, 0, 5])]);
    let eye = [0.5, 0.5, 0.5];
    let along = [0.0, 0.0, 1.0];
    assert_eq!(Rays::cast(&build, GhostView::Whole, eye, along, 4.0), None);
    assert_eq!(Rays::cast(&build, GhostView::Whole, eye, along, 10.0), Some(([0, 0, 5], 0)));
}

#[test]
fn not_targets_are_passed_through() {
    let build = Rays::water_then_stone();
    let hit = Rays::cast(&build, GhostView::Whole, [0.5, 0.5, -3.5], [0.0, 0.0, 1.0], 10.0);
    let (cell, id) = hit.unwrap();
    assert_eq!(cell, [0, 0, 1]);
    assert_eq!(build.palette.entries[usize::from(id)].name, "minecraft:stone");
}

#[test]
fn negative_coordinates_walk_the_right_cells() {
    let build = Rays::stone([-20, -62, -20], 3);
    let hit = Rays::cast(&build, GhostView::Whole, [-18.5, -60.5, -25.5], [0.0, 0.0, 1.0], 20.0);
    assert_eq!(hit, Some(([-19, -61, -20], 0)));
}

#[test]
fn a_zero_direction_gives_none() {
    let build = Rays::stone([0, 0, 0], 3);
    let eye = [1.5, 1.5, 1.5];
    assert_eq!(Rays::cast(&build, GhostView::Whole, eye, [0.0, 0.0, 0.0], 5.0), None);
    assert_eq!(Rays::cast(&build, GhostView::Whole, eye, [f64::NAN, 0.0, 1.0], 5.0), None);
    assert_eq!(Rays::cast(&build, GhostView::Whole, eye, [f64::INFINITY, 0.0, 0.0], 5.0), None);
}

impl Rays {
    fn cast_limited(build: &GhostBuild, eye: [f64; 3], dir: [f64; 3], limits: &RayLimits) -> Option<([i32; 3], u16)> {
        let needs = PlaceNeeds::new(&build.palette, &ItemRules::new());
        GhostRay::first_spot(build, GhostView::Whole, &needs, eye, dir, limits)
    }
}

#[test]
fn the_stop_cell_is_not_entered_whatever_the_distance() {
    let build = Rays::stone([0, 0, 0], 3);
    let eye = [1.5, 10.5, 1.5];
    let down = [0.0, -1.0, 0.0];
    let stop = Some([1, 2, 1]);
    for limit in [20.0, 7.5, f64::from_bits(7.5_f64.to_bits() + 1), f64::from_bits(7.5_f64.to_bits() - 1)] {
        let limits = RayLimits::new(limit, stop, false, None);
        assert_eq!(Rays::cast_limited(&build, eye, down, &limits), None, "limit {limit}");
    }
}

#[test]
fn a_spot_in_front_of_the_stop_cell_is_found() {
    let build = Rays::stone([0, 0, 0], 3);
    let limits = RayLimits::new(20.0, Some([1, 1, 1]), false, None);
    let hit = Rays::cast_limited(&build, [1.5, 10.5, 1.5], [0.0, -1.0, 0.0], &limits);
    assert_eq!(hit, Some(([1, 2, 1], 0)));
}

#[test]
fn an_entered_stop_cell_can_be_the_target_and_ends_the_walk() {
    let build = Rays::stone([0, 0, 0], 3);
    let limits = RayLimits::new(20.0, Some([1, 2, 1]), true, None);
    let hit = Rays::cast_limited(&build, [1.5, 10.5, 1.5], [0.0, -1.0, 0.0], &limits);
    assert_eq!(hit, Some(([1, 2, 1], 0)));

    let water = Rays::water_then_stone();
    let limits = RayLimits::new(20.0, Some([0, 0, 0]), true, None);
    assert_eq!(Rays::cast_limited(&water, [0.5, 0.5, -3.5], [0.0, 0.0, 1.0], &limits), None);
}

#[test]
fn a_stop_cell_outside_the_build_changes_nothing() {
    let build = Rays::stone([0, 0, 0], 3);
    let limits = RayLimits::new(20.0, Some([1, -5, 1]), false, None);
    let hit = Rays::cast_limited(&build, [1.5, 10.5, 1.5], [0.0, -1.0, 0.0], &limits);
    assert_eq!(hit, Some(([1, 2, 1], 0)));
}

#[test]
fn an_eye_in_an_empty_target_finds_the_target_ahead() {
    let build = GhostFixture::blocks(&[([0, 0, 0], [0, 0, 0]), ([2, 0, 0], [2, 0, 0])]);
    let limits = RayLimits::new(5.0, None, false, Some(([0, -1, 0], [0, 0, 0])));
    let hit = Rays::cast_limited(&build, [0.5, 0.6, 0.5], [1.0, 0.0, 0.0], &limits);
    assert_eq!(hit, Some(([2, 0, 0], 0)));
}

#[test]
fn looking_down_from_inside_the_build_finds_the_cell_under_the_feet() {
    let build = Rays::stone([0, 0, 0], 3);
    let limits = RayLimits::new(5.0, None, false, Some(([1, 1, 1], [1, 2, 1])));
    let hit = Rays::cast_limited(&build, [1.5, 2.6, 1.5], [0.0, -1.0, 0.0], &limits);
    assert_eq!(hit, Some(([1, 0, 1], 0)));
}
