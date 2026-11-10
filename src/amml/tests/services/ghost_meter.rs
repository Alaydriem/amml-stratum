use alaydriem_stratum::services::GhostMeter;
use stratum_core::structs::ghost::GhostReport;

struct Reports;

impl Reports {
    fn frame(draw_us: u64, culled: u32, cells: u32, skipped: u32, build_us: u64) -> GhostReport {
        let mut report = GhostReport::new();
        report.draw_us_sum = draw_us;
        report.draw_us_max = draw_us;
        report.draw_frames = 1;
        report.meshes_drawn = 3;
        report.culled = culled;
        report.cells_built = cells;
        report.skipped_cells = skipped;
        report.build_us = build_us;
        report
    }
}

#[test]
fn a_window_sums_counts_and_keeps_the_slowest_frame() {
    let mut meter = GhostMeter::new();
    meter.observe(&Reports::frame(100, 4, 10, 1, 700));
    meter.observe(&Reports::frame(300, 6, 20, 2, 200));
    meter.block_changed();
    meter.block_changed();
    meter.block_changed();

    let window = meter.take();
    assert_eq!(window.frames, 2);
    assert_eq!(window.meshes_drawn, 6);
    assert_eq!(window.culled, 10);
    assert_eq!(window.cells_built, 30);
    assert_eq!(window.skipped_cells, 3);
    assert_eq!(window.block_changes, 3);
    assert_eq!(window.draw_avg_us, 200);
    assert_eq!(window.draw_max_us, 300);
    assert_eq!(window.build_frame_max_us, 700);
}

#[test]
fn taking_a_window_starts_the_next_one_empty() {
    let mut meter = GhostMeter::new();
    meter.observe(&Reports::frame(100, 4, 10, 1, 700));
    meter.block_changed();
    let _ = meter.take();

    let window = meter.take();
    assert_eq!(window.frames, 0);
    assert_eq!(window.meshes_drawn, 0);
    assert_eq!(window.culled, 0);
    assert_eq!(window.cells_built, 0);
    assert_eq!(window.skipped_cells, 0);
    assert_eq!(window.block_changes, 0);
    assert_eq!(window.draw_avg_us, 0);
    assert_eq!(window.draw_max_us, 0);
    assert_eq!(window.build_frame_max_us, 0);
}
