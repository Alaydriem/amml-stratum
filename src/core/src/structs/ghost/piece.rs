use std::sync::Arc;

use crate::structs::structure::Structure;

// One structure placed in world space.
#[derive(Debug, Clone)]
pub struct GhostPiece {
    pub structure: Arc<Structure>,
    // World position of the structure's cell (0, 0, 0).
    pub world_min: [i32; 3],
    // Build palette id for each index of the structure's palette. `GhostPalette::NONE` marks
    // indices that hold no drawn block.
    pub remap: Vec<u16>,
}
