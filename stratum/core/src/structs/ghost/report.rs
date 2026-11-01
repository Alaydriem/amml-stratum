use super::SlotBuilt;

// What the shim reports back after it has run a frame's jobs and draws.
#[derive(Debug, Clone, Default)]
pub struct GhostReport {
    pub built: Vec<SlotBuilt>,
    // The block lookups of the last palette the shim resolved that fell back. Set only with
    // `palette_resolved`.
    pub unknown_blocks: u32,
    pub default_state_blocks: u32,
    // The shim resolved a palette in a frame this report covers; the two counts above are its result.
    pub palette_resolved: bool,
    // The time the frame spent building meshes, the sum of its jobs. For a report that covers more
    // than one frame, the slowest frame.
    pub build_us: u64,
    pub draw_us_sum: u64,
    pub draw_frames: u32,
    // The slowest single draw among the frames this report covers.
    pub draw_us_max: u64,
    // Meshes drawn over the frames this report covers, after culling.
    pub meshes_drawn: u32,
    // Cells the jobs of this report turned into geometry.
    pub cells_built: u32,
    // Cells the jobs of this report left out because the world already holds a block of the same name.
    pub skipped_cells: u32,
    // Drawn slots the shim left out because they were outside the view.
    pub culled: u32,
    // The shim dropped its meshes and block lookups (it saw a new world); the next frame starts a new
    // generation, with the palette and a clear.
    pub world_reset: bool,
}

impl GhostReport {
    pub fn new() -> Self {
        Self::default()
    }

    // Folds a later report into this one, so neither one's build answers are lost: the built entries
    // are appended up to `built_limit` in all, the counts add up, the slowest values are kept, and
    // the newer palette counts replace the older. Returns how many built entries of `later` did
    // not fit.
    pub fn absorb(&mut self, later: &GhostReport, built_limit: usize) -> usize {
        let room = built_limit.saturating_sub(self.built.len());
        self.built.extend(later.built.iter().take(room).copied());
        let cut = later.built.len().saturating_sub(room);
        if later.palette_resolved {
            self.palette_resolved = true;
            self.unknown_blocks = later.unknown_blocks;
            self.default_state_blocks = later.default_state_blocks;
        }
        self.build_us = self.build_us.max(later.build_us);
        self.draw_us_sum = self.draw_us_sum.saturating_add(later.draw_us_sum);
        self.draw_frames = self.draw_frames.saturating_add(later.draw_frames);
        self.draw_us_max = self.draw_us_max.max(later.draw_us_max);
        self.meshes_drawn = self.meshes_drawn.saturating_add(later.meshes_drawn);
        self.cells_built = self.cells_built.saturating_add(later.cells_built);
        self.skipped_cells = self.skipped_cells.saturating_add(later.skipped_cells);
        self.culled = self.culled.saturating_add(later.culled);
        self.world_reset |= later.world_reset;
        cut
    }
}
