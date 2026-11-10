// What the render thread's ghost did in one telemetry window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GhostWindow {
    // Frames the ghost renderer took and reported, with or without a mesh to draw.
    pub frames: u64,
    pub meshes_drawn: u64,
    pub culled: u64,
    pub block_changes: u64,
    pub skipped_cells: u64,
    pub cells_built: u64,
    pub jobs_built: u64,
    pub draw_avg_us: u64,
    pub draw_max_us: u64,
    // The slowest frame's build time, the sum of that frame's jobs.
    pub build_frame_max_us: u64,
}
