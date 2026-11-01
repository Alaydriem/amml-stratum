use stratum_core::structs::ghost::GhostReport;

use super::GhostWindow;

// The render thread's ghost costs from its reports, one telemetry window at a time.
#[derive(Debug, Default)]
pub struct GhostMeter {
    build_frame_us_max: u64,
    draw_us_sum: u64,
    draw_us_max: u64,
    frames: u64,
    meshes_drawn: u64,
    culled: u64,
    block_changes: u64,
    skipped_cells: u64,
    cells_built: u64,
    jobs_built: u64,
}

impl GhostMeter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe(&mut self, report: &GhostReport) {
        self.build_frame_us_max = self.build_frame_us_max.max(report.build_us);
        self.draw_us_sum = self.draw_us_sum.saturating_add(report.draw_us_sum);
        self.draw_us_max = self.draw_us_max.max(report.draw_us_max);
        self.frames += u64::from(report.draw_frames);
        self.meshes_drawn += u64::from(report.meshes_drawn);
        self.culled += u64::from(report.culled);
        self.skipped_cells += u64::from(report.skipped_cells);
        self.cells_built += u64::from(report.cells_built);
        self.jobs_built += report.built.len() as u64;
    }

    // A block change the game reported.
    pub fn block_changed(&mut self) {
        self.block_changes += 1;
    }

    // The window, and a fresh one.
    pub fn take(&mut self) -> GhostWindow {
        let window = GhostWindow {
            frames: self.frames,
            meshes_drawn: self.meshes_drawn,
            culled: self.culled,
            block_changes: self.block_changes,
            skipped_cells: self.skipped_cells,
            cells_built: self.cells_built,
            jobs_built: self.jobs_built,
            draw_avg_us: self.draw_us_sum.checked_div(self.frames).unwrap_or(0),
            draw_max_us: self.draw_us_max,
            build_frame_max_us: self.build_frame_us_max,
        };
        *self = Self::default();
        window
    }
}
