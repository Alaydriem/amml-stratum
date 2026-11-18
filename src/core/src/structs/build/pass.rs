// The pass of a build that places a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildPass {
    // Solid blocks, placed layer by layer.
    Main,
    // Blocks that hang from the block above, placed after every main layer.
    Hanging,
    // Fluids, and the gravity blocks that rest on them, placed after the main pass.
    Fluid,
}
