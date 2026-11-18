use super::BuildPass;

// One block to place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildStep {
    // World position of the block.
    pub position: [i32; 3],
    // Index into the palette of the structure.
    pub palette: u16,
    pub pass: BuildPass,
}
