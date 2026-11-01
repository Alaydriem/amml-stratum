use crate::structs::block::Face;

// The block the player's use ray hit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UseHit {
    pub cell: [i32; 3],
    pub face: Face,
    pub distance: f64,
    // True when a placed block would replace the hit block.
    pub replaceable: bool,
}
