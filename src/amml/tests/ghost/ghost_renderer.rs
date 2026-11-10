use std::sync::atomic::Ordering;

use alaydriem_stratum::game::TessellatorLayout;
use stratum_core::structs::ghost::{GhostDraw, GhostJob, GhostJobMode};

use crate::support::fake_ghost::{
    BEGIN_CALLS, BT_EVENTS, CLEAR_CALLS, CLEAR_KEEPS_BEGIN, CLEARS_CLOSED, END_CALLS, MESH_DTOR_CALLS, MESH_LIVE,
    NBT_STATE, RENDERED, TESSELLATE_CALLS, TESSELLATE_FAIL_AT,
};
use crate::support::{FakeGhost, GhostRig};

const STONE: &str = "minecraft:stone";

// a rig in world A with one frame of `jobs` and `draws` published and run
fn rig_with(jobs: Vec<GhostJob>, draws: Vec<GhostDraw>) -> GhostRig {
    let mut rig = GhostRig::new();
    let mut frame = GhostRig::ghost_frame(1, 1, &[STONE]);
    frame.jobs = jobs;
    frame.draws = draws;
    rig.publish(&frame);
    rig.frame(Some(GhostRig::REGION_A));
    rig
}

// every begin was closed by an end, or by a clear that closed it
pub fn assert_paired(rig: &GhostRig) {
    let closed = FakeGhost::count(&END_CALLS) + FakeGhost::count(&CLEARS_CLOSED);
    assert_eq!(FakeGhost::count(&BEGIN_CALLS), closed, "every begin has its end");
    assert!(!rig.tessellator.view::<TessellatorLayout>().tessellating, "the tessellator is left free");
}

#[test]
fn a_normal_build_pairs_every_begin_with_an_end_and_keeps_the_mesh() {
    let rig = rig_with(vec![GhostRig::job(0, 1, 0, [0, 64, 0])], Vec::new());

    assert_paired(&rig);
    // the winding probe's mesh is dropped at once; the job's mesh is kept in its slot
    assert_eq!(FakeGhost::count(&MESH_LIVE), 1);
    let reports = rig.reports();
    let built: Vec<_> = reports.iter().flat_map(|report| report.built.iter()).collect();
    assert_eq!(built.len(), 1);
    assert!(built[0].ok);
    assert_eq!(built[0].bytes, 4 * 20, "one quad at the measured 20 GPU bytes per vertex");
}

#[test]
fn a_build_that_fails_part_way_still_ends_the_begin() {
    // call 1 is the winding probe at the join; calls 2 to 4 are the job's three cells, and the second fails part way
    TESSELLATE_FAIL_AT.store(3, Ordering::SeqCst);
    let mut job = GhostRig::job(0, 1, 0, [0, 64, 0]);
    job.dims = [3, 1, 1];
    job.slot_size = [3, 1, 1];
    job.cells = vec![u16::MAX; 5 * 3 * 3];
    // the padded grid is 5 x 3 x 3, X fastest, then Z, then Y; the cells sit at y 1, z 1
    for x in 1..4 {
        job.cells[(3 + 1) * 5 + x] = 0;
    }

    let rig = rig_with(vec![job], Vec::new());

    assert_eq!(FakeGhost::count(&TESSELLATE_CALLS), 4);
    assert_paired(&rig);
    let built: Vec<_> = rig.reports().into_iter().flat_map(|report| report.built).collect();
    assert_eq!(built.len(), 1, "the job is answered");
    assert_eq!(built[0].bytes, 11 * 20, "the vertices the game wrote, the cut quad too");
}

#[test]
fn a_group_part_whose_clear_leaves_the_begin_open_is_ended() {
    CLEAR_KEEPS_BEGIN.store(true, Ordering::SeqCst);

    let rig = rig_with(vec![GhostRig::group_job(0, 1, GhostJobMode::Append, 7, 0)], Vec::new());

    assert_eq!(FakeGhost::count(&CLEAR_CALLS), 1);
    assert_paired(&rig);
    assert_eq!(FakeGhost::count(&MESH_LIVE), 0, "the end after the clear makes a mesh that is dropped at once");
}

#[test]
fn a_group_of_two_parts_uploads_one_mesh_and_frees_the_group_buffer() {
    let mut rig = rig_with(vec![GhostRig::group_job(0, 1, GhostJobMode::Append, 7, 0)], Vec::new());
    let mut frame = GhostRig::ghost_frame(1, 1, &[STONE]);
    frame.palette = None;
    frame.jobs = vec![GhostRig::group_job(0, 2, GhostJobMode::Finish, 7, 1)];
    rig.publish(&frame);

    rig.frame(Some(GhostRig::REGION_A));

    assert_paired(&rig);
    assert_eq!(FakeGhost::count(&MESH_LIVE), 1, "the slot holds the group's mesh");
    let built: Vec<_> = rig.reports().into_iter().flat_map(|report| report.built).collect();
    assert_eq!(built.iter().map(|entry| (entry.serial, entry.ok)).collect::<Vec<_>>(), [(1, true), (2, true)]);
    // the finish uploads what the appends collected, one quad: its vertices and its six 32-bit indices
    assert_eq!(built[1].bytes, 4 * 20 + 6 * 4);
}

#[test]
fn a_dimension_change_frees_every_slot_through_the_mesh_dtor() {
    let mut rig = rig_with(vec![GhostRig::job(0, 1, 0, [0, 64, 0]), GhostRig::job(1, 2, 0, [16, 64, 0])], Vec::new());
    assert_eq!(FakeGhost::count(&MESH_LIVE), 2);
    let dtors_before = FakeGhost::count(&MESH_DTOR_CALLS);

    // the same world epoch with another region: the player changed dimension
    rig.frame(Some(GhostRig::REGION_B));

    assert_eq!(FakeGhost::count(&MESH_LIVE), 0);
    // two slots, then the new region's winding probe
    assert_eq!(FakeGhost::count(&MESH_DTOR_CALLS) - dtors_before, 3);
}

#[test]
fn the_old_block_tessellator_is_pointed_at_the_live_region_before_it_is_deleted() {
    let mut rig = rig_with(Vec::new(), Vec::new());

    rig.frame(Some(GhostRig::REGION_B));

    let events = BT_EVENTS.lock().unwrap().clone();
    assert_eq!(events, [("set_region", GhostRig::REGION_B), ("dtor", GhostRig::REGION_B)]);
}

#[test]
fn leaving_the_world_frees_every_slot() {
    let mut rig = rig_with(vec![GhostRig::job(0, 1, 0, [0, 64, 0])], Vec::new());

    rig.frame(None);

    assert_eq!(FakeGhost::count(&MESH_LIVE), 0);
}

#[test]
fn three_slots_draw_farthest_first() {
    let jobs = vec![
        GhostRig::job(0, 1, 0, [0, 64, 0]),
        GhostRig::job(1, 2, 0, [0, 64, 0]),
        GhostRig::job(2, 3, 0, [0, 64, 0]),
    ];
    // slot 1 farthest, then slot 0, then slot 2, from the camera at the origin
    let draws = vec![GhostRig::draw(0, [20, 0, 0]), GhostRig::draw(1, [0, 0, 50]), GhostRig::draw(2, [3, 0, 0])];

    let _rig = rig_with(jobs, draws);

    // the meshes' serials: 1 is the winding probe, then one per job in slot order
    assert_eq!(*RENDERED.lock().unwrap(), [3, 2, 4]);
}

#[test]
fn every_palette_entry_that_does_not_resolve_counts_once_in_the_report() {
    NBT_STATE.store(1, Ordering::SeqCst);
    let mut rig = GhostRig::new();
    let frame = GhostRig::ghost_frame(1, 1, &["minecraft:not_a_block", "minecraft:also_not", "minecraft:stone"]);
    rig.publish(&frame);

    rig.frame(Some(GhostRig::REGION_A));

    let reports = rig.reports();
    let resolved: Vec<_> = reports.iter().filter(|report| report.palette_resolved).collect();
    assert_eq!(resolved.len(), 1);
    assert_eq!((resolved[0].unknown_blocks, resolved[0].default_state_blocks), (2, 1));
}
