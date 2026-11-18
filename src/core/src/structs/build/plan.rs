use std::sync::Arc;

use crate::structs::build::PaletteOutcome;
use crate::structs::ghost::GhostPiece;
use crate::structs::structure::BlockState;

// What a build places, taken from the ghost build when the build is opened.
#[derive(Debug, Clone)]
pub(crate) struct BuildPlan {
    pub(crate) piece: Arc<GhostPiece>,
    // The box of the structure in the world, both corners inclusive.
    pub(crate) min: [i32; 3],
    pub(crate) max: [i32; 3],
    // The blocks of the build palette with their direction states turned.
    pub(crate) blocks: Vec<BlockState>,
    // The game's outcome per build palette entry. Before the probe answers, an entry that could be written
    // reads as exact and an entry that could not reads as unknown.
    pub(crate) outcomes: Vec<PaletteOutcome>,
    // Per palette index of the piece: the block is not placed.
    pub(crate) skip: Arc<Vec<bool>>,
}
