use std::sync::Arc;

use super::BuildPass;
use crate::structs::ghost::GhostPiece;

// A layer whose build order the worker works out.
#[derive(Debug, Clone)]
pub struct OrderRequest {
    // Names the request, so an answer to an older one is dropped.
    pub serial: u64,
    pub pass: BuildPass,
    // The layer, counted from 0 at the bottom.
    pub layer: u32,
    pub piece: Arc<GhostPiece>,
    // Per palette index of the piece: the block is not placed.
    pub skip: Arc<Vec<bool>>,
}
