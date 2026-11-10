use stratum_core::services::GhostSections;
use stratum_core::structs::ghost::{GhostDraw, GhostJob, GhostJobMode, GhostPalette, GhostView, SectionPart};

use crate::support::{GhostFixture, SectionAssert};

#[test]
fn negative_coordinates_floor_into_the_lower_section() {
    let build = GhostFixture::blocks(&[([-1, -1, -17], [-1, -1, -17])]);
    let mut out = Vec::new();

    GhostSections::occupied(&build, 600, &mut out, &mut Vec::new());

    assert_eq!(out, [SectionAssert::whole(-1, -1, -2)]);
    let mut job = GhostJob::new(out[0]);
    GhostSections::fill_job(&build, out[0], &mut job);
    assert_eq!(job.origin, [-16, -16, -32]);
    assert_eq!(job.dims, [16, 16, 16]);
}

#[test]
fn whole_view_wants_occupied_sections_nearest_first() {
    let build = GhostFixture::blocks(&[([0, 0, 0], [0, 0, 0]), ([16, 0, 0], [16, 0, 0]), ([32, 0, 0], [32, 0, 0])]);
    let out = SectionAssert::wanted(&build, GhostView::Whole, [0, 0, 0], 10);
    assert_eq!(out, [SectionAssert::whole(0, 0, 0), SectionAssert::whole(1, 0, 0), SectionAssert::whole(2, 0, 0)]);

    let out = SectionAssert::wanted(&build, GhostView::Whole, [40, 0, 0], 10);
    assert_eq!(out, [SectionAssert::whole(2, 0, 0), SectionAssert::whole(1, 0, 0), SectionAssert::whole(0, 0, 0)]);
}

#[test]
fn draw_distance_drops_far_sections() {
    let build = GhostFixture::blocks(&[
        ([0, 0, 0], [0, 0, 0]),
        ([32, 0, 0], [32, 0, 0]),
        ([48, 0, 0], [48, 0, 0]),
        ([0, 0, 48], [0, 0, 48]),
    ]);
    let out = SectionAssert::wanted(&build, GhostView::Whole, [8, 8, 8], 2);

    assert_eq!(out, [SectionAssert::whole(0, 0, 0), SectionAssert::whole(2, 0, 0)]);
}

#[test]
fn draw_distance_counts_the_chunk_the_player_stands_in() {
    let build = GhostFixture::blocks(&[([-33, 0, 0], [-33, 0, 0]), ([-49, 0, 0], [-49, 0, 0])]);
    let out = SectionAssert::wanted(&build, GhostView::Whole, [-1, 0, 0], 2);

    assert_eq!(out, [SectionAssert::whole(-3, 0, 0)]);
}

#[test]
fn a_section_splits_into_halves_only_above_twice_the_frame_budget() {
    let full = GhostFixture::blocks(&[([0, 0, 0], [15, 15, 15])]);
    let mut out = Vec::new();

    GhostSections::occupied(&full, 600, &mut out, &mut Vec::new());
    assert_eq!(
        out,
        [SectionAssert::key(0, 0, 0, None, SectionPart::Lower), SectionAssert::key(0, 0, 0, None, SectionPart::Upper)]
    );

    // 1,000 cells: above the frame budget, below twice it
    let thousand = GhostFixture::blocks(&[([0, 0, 0], [9, 9, 9])]);
    GhostSections::occupied(&thousand, 600, &mut out, &mut Vec::new());
    assert_eq!(out, [SectionAssert::whole(0, 0, 0)]);

    // 1,210 cells: above twice the frame budget
    let over = GhostFixture::blocks(&[([0, 0, 0], [10, 9, 10])]);
    GhostSections::occupied(&over, 600, &mut out, &mut Vec::new());
    assert_eq!(out.len(), 2);
}

#[test]
fn split_keeps_only_the_half_that_holds_a_cell() {
    let build = GhostFixture::blocks(&[([0, 0, 0], [15, 7, 15]), ([0, 8, 0], [0, 8, 0])]);
    let mut out = Vec::new();
    GhostSections::occupied(&build, 600, &mut out, &mut Vec::new());
    assert_eq!(out.len(), 2);

    let lower_only = GhostFixture::blocks(&[([0, 0, 0], [15, 7, 15]), ([0, 0, 0], [0, 7, 0])]);
    GhostSections::occupied(&lower_only, 100, &mut out, &mut Vec::new());
    assert_eq!(out, [SectionAssert::key(0, 0, 0, None, SectionPart::Lower)]);
}

#[test]
fn occupied_counts_cells_per_key() {
    let full = GhostFixture::blocks(&[([0, 0, 0], [15, 15, 15])]);
    let mut out = Vec::new();
    let mut cells = Vec::new();
    GhostSections::occupied(&full, 600, &mut out, &mut cells);
    assert_eq!(out.len(), 2);
    assert_eq!(cells, [2048, 2048]);

    let row = GhostFixture::blocks(&[([0, 0, 0], [2, 0, 0])]);
    GhostSections::occupied(&row, 600, &mut out, &mut cells);
    assert_eq!(out, [SectionAssert::whole(0, 0, 0)]);
    assert_eq!(cells, [3]);
}

#[test]
fn half_job_padding_holds_the_other_half() {
    let build = GhostFixture::blocks(&[([5, 7, 5], [5, 7, 5]), ([3, 8, 2], [3, 8, 2])]);
    let dims = [16, 8, 16];

    let lower_key = SectionAssert::key(0, 0, 0, None, SectionPart::Lower);
    let mut lower = GhostJob::new(lower_key);
    GhostSections::fill_job(&build, lower_key, &mut lower);
    assert_eq!(lower.dims, [16, 8, 16]);
    assert_eq!(lower.cells.len(), 18 * 10 * 18);
    assert_eq!(lower.cells[SectionAssert::at(dims, 6, 8, 6)], 0);
    assert_eq!(lower.cells[SectionAssert::at(dims, 4, 9, 3)], 0);
    assert_eq!(SectionAssert::filled(&lower), 2);

    let upper_key = SectionAssert::key(0, 0, 0, None, SectionPart::Upper);
    let mut upper = GhostJob::new(upper_key);
    GhostSections::fill_job(&build, upper_key, &mut upper);
    assert_eq!(upper.origin, [0, 8, 0]);
    assert_eq!(upper.cells[SectionAssert::at(dims, 6, 0, 6)], 0);
    assert_eq!(upper.cells[SectionAssert::at(dims, 4, 1, 3)], 0);
    assert_eq!(SectionAssert::filled(&upper), 2);
}

#[test]
fn layer_view_wants_one_key_per_occupied_column_covering_that_y() {
    let build = GhostFixture::blocks(&[
        ([0, 5, 0], [0, 5, 0]),
        ([16, 5, 3], [16, 5, 3]),
        ([0, 6, 16], [0, 6, 16]),
        ([0, 20, 0], [0, 20, 0]),
    ]);

    let out = SectionAssert::wanted(&build, GhostView::Layer(5), [0, 5, 0], 10);
    assert_eq!(
        out,
        [
            SectionAssert::key(0, 0, 0, Some(5), SectionPart::Whole),
            SectionAssert::key(0, 0, 1, Some(5), SectionPart::Whole),
            SectionAssert::key(1, 0, 0, Some(5), SectionPart::Whole),
        ]
    );

    let out = SectionAssert::wanted(&build, GhostView::Layer(20), [0, 5, 0], 10);
    assert_eq!(out, [SectionAssert::key(0, 1, 0, Some(20), SectionPart::Whole)]);

    let out = SectionAssert::wanted(&build, GhostView::Layer(100), [0, 5, 0], 10);
    assert!(out.is_empty());
}

#[test]
fn layer_view_takes_only_the_half_that_covers_the_y() {
    let build = GhostFixture::blocks(&[([0, 0, 0], [15, 15, 15])]);

    let out = SectionAssert::wanted(&build, GhostView::Layer(10), [0, 10, 0], 10);
    assert_eq!(out, [SectionAssert::key(0, 0, 0, Some(10), SectionPart::Whole)]);

    let out = SectionAssert::wanted(&build, GhostView::Layer(3), [0, 3, 0], 10);
    assert_eq!(out, [SectionAssert::key(0, 0, 0, Some(3), SectionPart::Whole)]);
}

#[test]
fn equal_distance_sections_order_by_y_then_x_then_z() {
    let build = GhostFixture::blocks(&[([16, 0, 0], [16, 0, 0]), ([-16, 0, 0], [-16, 0, 0])]);

    let out = SectionAssert::wanted(&build, GhostView::Whole, [8, 8, 8], 10);

    assert_eq!(out, [SectionAssert::whole(-1, 0, 0), SectionAssert::whole(1, 0, 0)]);
}

#[test]
fn draw_distance_is_the_larger_horizontal_chunk_offset() {
    let build = GhostFixture::blocks(&[([0, 0, 0], [0, 0, 0]), ([48, 0, 48], [48, 0, 48])]);

    let kept = SectionAssert::wanted(&build, GhostView::Whole, [8, 8, 8], 3);
    assert_eq!(kept, [SectionAssert::whole(0, 0, 0), SectionAssert::whole(3, 0, 3)]);

    let dropped = SectionAssert::wanted(&build, GhostView::Whole, [8, 8, 8], 2);
    assert_eq!(dropped, [SectionAssert::whole(0, 0, 0)]);
}

#[test]
fn zero_draw_distance_keeps_only_the_players_chunk() {
    let build = GhostFixture::blocks(&[([0, 0, 0], [0, 0, 0]), ([16, 0, 0], [16, 0, 0])]);

    let out = SectionAssert::wanted(&build, GhostView::Whole, [8, 0, 8], 0);

    assert_eq!(out, [SectionAssert::whole(0, 0, 0)]);
}

#[test]
fn job_grid_includes_neighbours_from_adjacent_sections() {
    let build = GhostFixture::blocks(&[([15, 0, 0], [15, 0, 0]), ([16, 0, 0], [16, 0, 0])]);
    let section = SectionAssert::whole(0, 0, 0);
    let mut job = GhostJob::new(section);

    GhostSections::fill_job(&build, section, &mut job);

    assert_eq!(job.cells.len(), 18 * 18 * 18);
    assert_eq!(job.cells[SectionAssert::at([16, 16, 16], 16, 1, 1)], 0);
    assert_eq!(job.cells[SectionAssert::at([16, 16, 16], 17, 1, 1)], 0);
    assert_eq!(SectionAssert::filled(&job), 2);
}

#[test]
fn layer_job_leaves_rows_above_and_below_empty() {
    let build = GhostFixture::blocks(&[([3, 4, 3], [3, 6, 3])]);
    let layer = SectionAssert::key(0, 0, 0, Some(5), SectionPart::Whole);
    let mut job = GhostJob::new(layer);

    GhostSections::fill_job(&build, layer, &mut job);

    assert_eq!(job.origin, [0, 5, 0]);
    assert_eq!(job.dims, [16, 1, 16]);
    assert_eq!(job.cells.len(), 18 * 3 * 18);
    assert_eq!(job.cells[SectionAssert::at([16, 1, 16], 4, 1, 4)], 0);
    assert_eq!(SectionAssert::filled(&job), 1);
}

#[test]
fn fill_job_makes_replace_jobs_with_their_own_box() {
    let mut build = GhostFixture::blocks(&[([3, 4, 3], [3, 6, 3])]);
    build.marker_block = build.palette.marker_id().unwrap();
    let keys = [
        SectionAssert::whole(0, 0, 0),
        SectionAssert::key(0, 0, 0, Some(5), SectionPart::Whole),
        GhostSections::MARKER_KEY,
    ];

    for key in keys {
        let mut job = GhostJob::new(key);
        job.mode = GhostJobMode::Append;
        job.group_serial = 9;
        job.group_index = 4;
        job.group_offset = [1, 2, 3];
        GhostSections::fill_job(&build, key, &mut job);

        assert_eq!(job.mode, GhostJobMode::Replace);
        assert_eq!(job.slot_size, job.dims);
        assert_eq!((job.group_serial, job.group_index, job.group_offset), (0, 0, [0; 3]));
    }
}

#[test]
fn fill_job_reuses_the_buffer() {
    let build = GhostFixture::blocks(&[([0, 0, 0], [3, 3, 3]), ([16, 0, 0], [20, 3, 3])]);
    let first = SectionAssert::whole(0, 0, 0);
    let mut job = GhostJob::new(first);

    GhostSections::fill_job(&build, first, &mut job);
    let capacity = job.cells.capacity();
    let pointer = job.cells.as_ptr();
    job.slot = 7;
    GhostSections::fill_job(&build, SectionAssert::whole(1, 0, 0), &mut job);

    assert_eq!(job.cells.capacity(), capacity);
    assert_eq!(job.cells.as_ptr(), pointer);
    assert_eq!(job.key, SectionAssert::whole(1, 0, 0));
    assert_eq!(job.slot, 7);
    assert_eq!(SectionAssert::filled(&job), 5 * 4 * 4);
}

#[test]
fn miniature_scale_caps_small_and_scales_large() {
    let small = GhostFixture::blocks(&[([0, 0, 0], [3, 0, 0])]);
    let large = GhostFixture::blocks(&[([0, 0, 0], [199, 0, 0])]);

    assert!((GhostSections::miniature_scale(&small) - 0.25).abs() < 1e-6);
    assert!((GhostSections::miniature_scale(&large) - 0.01).abs() < 1e-6);
}

#[test]
fn miniature_offsets_hold_at_negative_world_min() {
    let build = GhostFixture::blocks(&[([-20, -5, -33], [-1, 3, -1])]);
    let key = SectionAssert::key(0, 0, 0, None, SectionPart::MiniatureGroup);

    let draw = GhostSections::draw(&build, key, 7, [100, 100, 100]);

    assert_eq!(draw.slot, 7);
    assert_eq!(draw.origin, [-20, -5, -33]);
    // the build is 20 x 9 x 33, so the mini ghost ends on the corner's X and Z
    assert_eq!(draw.offset, [-20, 0, -33]);
    assert!((draw.scale - GhostSections::miniature_scale(&build)).abs() < 1e-6);
}

#[test]
fn full_draw_uses_the_built_origin() {
    let build = GhostFixture::blocks(&[([0, 0, 0], [3, 0, 0])]);

    let draw = GhostSections::draw(&build, SectionAssert::whole(0, 0, 0), 2, [16, 0, -16]);

    assert_eq!(draw, GhostDraw { slot: 2, origin: [16, 0, -16], offset: [0; 3], scale: 1.0 });
}

#[test]
fn marker_job_is_one_cell_at_world_min() {
    let mut build = GhostFixture::blocks(&[([5, 6, 7], [9, 9, 9])]);
    build.marker_block = build.palette.marker_id().unwrap();
    let mut job = GhostJob::new(GhostSections::MARKER_KEY);

    GhostSections::fill_job(&build, GhostSections::MARKER_KEY, &mut job);

    assert_eq!(job.dims, [1, 1, 1]);
    assert_eq!(job.cells.len(), 27);
    assert_eq!(job.cells[13], build.marker_block);
    assert_eq!(job.cells.iter().filter(|cell| **cell != GhostPalette::NONE).count(), 1);
    assert_eq!(job.origin, [5, 6, 7]);
    assert_eq!(job.key, GhostSections::MARKER_KEY);
    assert!(!job.skip_matching);

    let draw = GhostSections::draw(&build, GhostSections::MARKER_KEY, 3, [0, 0, 0]);
    assert_eq!(draw.origin, [4, 6, 6]);
    assert_eq!(draw.offset, [0; 3]);
    assert_eq!(draw.scale, 1.0);
}

#[test]
fn member_and_finish_jobs_carry_the_group_frame() {
    let build = GhostFixture::blocks(&[([-20, -5, -33], [-1, 3, -1])]);
    let member = SectionAssert::key(-2, -1, -3, None, SectionPart::Upper);
    let mut plain = GhostJob::new(member);
    GhostSections::fill_job(&build, member, &mut plain);
    let mut job = GhostJob::new(member);

    GhostSections::fill_member_job(&build, member, &mut job);

    assert_eq!(job.mode, GhostJobMode::Append);
    assert_eq!(job.group_offset, [-32 + 20, -16 + 8 + 5, -48 + 33]);
    assert!(!job.skip_matching);
    assert_eq!(job.origin, build.world_min);
    assert_eq!(job.slot_size, [20, 9, 33]);
    assert_eq!(job.dims, plain.dims);
    assert_eq!(job.cells, plain.cells);

    let group = SectionAssert::key(0, 0, 0, None, SectionPart::MiniatureGroup);
    GhostSections::fill_finish_job(&build, group, &mut job);

    assert_eq!(job.mode, GhostJobMode::Finish);
    assert_eq!(job.key, group);
    assert!(job.cells.is_empty());
    assert_eq!(job.dims, [0; 3]);
    assert_eq!(job.origin, build.world_min);
    assert_eq!(job.slot_size, [20, 9, 33]);
}
