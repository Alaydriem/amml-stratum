use stratum_core::services::BuildOrder;
use stratum_core::structs::build::BuildPass;

use crate::support::{Pieces, Positions};

#[test]
fn a_full_rectangle_goes_in_snake_rows_along_the_long_side() {
    let piece = Pieces::layer(&["###", "###"]);
    let order = Positions::of(BuildOrder::layer(&piece, 0, &[]));
    assert_eq!(order, vec![[0, 0, 0], [1, 0, 0], [2, 0, 0], [2, 0, 1], [1, 0, 1], [0, 0, 1]]);
}

#[test]
fn rows_run_along_z_when_z_is_the_longer_side() {
    let piece = Pieces::layer(&["##", "##", "##"]);
    let order = Positions::of(BuildOrder::layer(&piece, 0, &[]));
    assert_eq!(order, vec![[0, 0, 0], [0, 0, 1], [0, 0, 2], [1, 0, 2], [1, 0, 1], [1, 0, 0]]);
}

#[test]
fn a_gap_goes_to_a_touching_block_before_it_jumps() {
    let piece = Pieces::layer(&["##.##", "#####"]);
    let order = Positions::of(BuildOrder::layer(&piece, 0, &[]));
    assert_eq!(
        order,
        vec![[0, 0, 0], [1, 0, 0], [1, 0, 1], [2, 0, 1], [3, 0, 1], [3, 0, 0], [4, 0, 0], [4, 0, 1], [0, 0, 1],]
    );
    assert_eq!(Positions::jumps(&order), 1);
}

#[test]
fn two_parts_jump_once() {
    let piece = Pieces::layer(&["##..##"]);
    let order = Positions::of(BuildOrder::layer(&piece, 0, &[]));
    assert_eq!(order, vec![[0, 0, 0], [1, 0, 0], [4, 0, 0], [5, 0, 0]]);
    assert_eq!(Positions::jumps(&order), 1);
}

#[test]
fn attached_blocks_come_last_in_their_layer() {
    let piece = Pieces::layer(&["#t#"]);
    let order = Positions::of(BuildOrder::layer(&piece, 0, &[]));
    assert_eq!(order, vec![[0, 0, 0], [2, 0, 0], [1, 0, 0]]);
}

#[test]
fn fluids_and_gravity_over_fluid_are_in_the_fluid_pass() {
    let piece = Pieces::layers(&[&["#w#"], &[".s."]]);
    assert!(BuildOrder::layer(&piece, 0, &[]).iter().all(|s| s.pass == BuildPass::Main && s.position != [1, 0, 0]));
    assert!(BuildOrder::layer(&piece, 1, &[]).is_empty());
    assert_eq!(Positions::of(BuildOrder::fluid_layer(&piece, 0, &[])), vec![[1, 0, 0]]);
    assert_eq!(Positions::of(BuildOrder::fluid_layer(&piece, 1, &[])), vec![[1, 1, 0]]);
    assert!(BuildOrder::fluid_layer(&piece, 1, &[]).iter().all(|s| s.pass == BuildPass::Fluid));
}

#[test]
fn the_fluid_pass_places_fluids_before_gravity_blocks_over_fluid() {
    let piece = Pieces::layers(&[&["ww"], &["sw"]]);
    let order = Positions::of(BuildOrder::fluid_layer(&piece, 1, &[]));
    assert_eq!(order, vec![[1, 1, 0], [0, 1, 0]]);
}

#[test]
fn gravity_blocks_with_no_fluid_below_are_in_the_main_pass() {
    let piece = Pieces::layers(&[&["s#"], &["ss"]]);
    assert_eq!(Positions::of(BuildOrder::layer(&piece, 0, &[])), vec![[0, 0, 0], [1, 0, 0]]);
    assert_eq!(Positions::of(BuildOrder::layer(&piece, 1, &[])), vec![[0, 1, 0], [1, 1, 0]]);
    assert!(BuildOrder::fluid_layer(&piece, 1, &[]).is_empty());
}

#[test]
fn positions_are_world_positions_at_negative_coordinates() {
    let piece = Pieces::layer_at(&["##"], [-30_000_123, -60, -5]);
    assert_eq!(Positions::of(BuildOrder::layer(&piece, 0, &[])), vec![[-30_000_123, -60, -5], [-30_000_122, -60, -5]]);
}

#[test]
fn skipped_palette_entries_and_air_and_void_get_no_step() {
    let piece = Pieces::layer(&["#.vd#w"]);
    assert_eq!(BuildOrder::count(&piece, &Pieces::skip_dirt()), 3);
    assert_eq!(BuildOrder::count(&piece, &[]), 4);
    assert_eq!(Positions::of(BuildOrder::layer(&piece, 0, &Pieces::skip_dirt())), vec![[0, 0, 0], [4, 0, 0]]);
    assert_eq!(Positions::of(BuildOrder::layer(&piece, 0, &[])), vec![[0, 0, 0], [3, 0, 0], [4, 0, 0]]);
    assert_eq!(Positions::of(BuildOrder::fluid_layer(&piece, 0, &Pieces::skip_dirt())), vec![[5, 0, 0]]);
}

#[test]
fn count_adds_both_passes_over_every_layer() {
    let piece = Pieces::layers(&[&["#w#"], &[".s."]]);
    assert_eq!(BuildOrder::count(&piece, &[]), 4);
}

#[test]
fn a_layer_outside_the_structure_has_no_steps() {
    let piece = Pieces::layer(&["##"]);
    assert!(BuildOrder::layer(&piece, 1, &[]).is_empty());
    assert!(BuildOrder::fluid_layer(&piece, 1, &[]).is_empty());
}

#[test]
fn a_hanging_block_is_not_in_the_main_pass_and_goes_in_the_hanging_pass() {
    // The lantern in layer 0 hangs from the stone in layer 1.
    let piece = Pieces::layers(&[&["h#"], &["##"]]);
    assert_eq!(Positions::of(BuildOrder::layer(&piece, 0, &[])), vec![[1, 0, 0]]);
    assert_eq!(Positions::of(BuildOrder::layer(&piece, 1, &[])), vec![[0, 1, 0], [1, 1, 0]]);
    let hanging = BuildOrder::hanging_layer(&piece, 0, &[]);
    assert_eq!(Positions::of(hanging.clone()), vec![[0, 0, 0]]);
    assert!(hanging.iter().all(|step| step.pass == BuildPass::Hanging));
    assert!(BuildOrder::hanging_layer(&piece, 1, &[]).is_empty());
    assert!(BuildOrder::fluid_layer(&piece, 0, &[]).is_empty());
}

#[test]
fn count_still_counts_a_hanging_block() {
    let piece = Pieces::layers(&[&["h#"], &["##"]]);
    assert_eq!(BuildOrder::count(&piece, &[]), 4);
}
