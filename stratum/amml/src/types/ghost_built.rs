/// What one ghost job build gave.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GhostBuilt {
    pub ok: bool,
    /// The game's tessellator was in use, so nothing was built and the job is not answered.
    pub busy: bool,
    pub vertices: u64,
    /// Cells turned into geometry.
    pub cells: u32,
    /// Cells left out because the world already holds a block of that name there.
    pub skipped: u32,
}
