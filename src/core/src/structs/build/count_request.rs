use std::sync::Arc;

use crate::structs::ghost::GhostPiece;

// A piece whose blocks to place the worker counts.
#[derive(Debug, Clone)]
pub struct CountRequest {
    // Names the request, so an answer to an older one is dropped.
    pub serial: u64,
    pub piece: Arc<GhostPiece>,
    // Per palette index of the piece: the block is not placed.
    pub skip: Arc<Vec<bool>>,
}
