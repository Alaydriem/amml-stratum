// The `ghost` block of the config file.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(default)]
pub struct GhostConfig {
    // Draw distance in chunks. 0 follows the game's render distance.
    pub draw_distance: u32,
    // GPU memory the ghost meshes may hold, in MiB.
    pub mesh_budget_mb: u32,
    // Cells turned into mesh in one frame.
    pub cells_per_frame: u32,
    // Shows the full-detail mini ghost at the build's lowest corner in Whole view.
    pub miniature: bool,
}

impl GhostConfig {
    // The smallest mesh budget: 1 MiB.
    pub const MIN_BUDGET_BYTES: u64 = 1024 * 1024;
    // The smallest frame allowance; the scheduler still hands out one job per frame below it.
    pub const MIN_CELLS_PER_FRAME: u32 = 64;

    pub fn mesh_budget_bytes(&self) -> u64 {
        u64::from(self.mesh_budget_mb).saturating_mul(1024 * 1024).max(Self::MIN_BUDGET_BYTES)
    }

    pub fn cells_per_frame(&self) -> u32 {
        self.cells_per_frame.max(Self::MIN_CELLS_PER_FRAME)
    }

    pub fn draw_distance_chunks(&self, game_render_distance: u32) -> u32 {
        if self.draw_distance == 0 { game_render_distance } else { self.draw_distance }
    }
}

impl Default for GhostConfig {
    fn default() -> Self {
        Self { draw_distance: 0, mesh_budget_mb: 1024, cells_per_frame: 600, miniature: true }
    }
}
