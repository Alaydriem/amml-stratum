use crate::structs::block::Face;

// The geometry the preview draws for one block. Lengths are in sixteenths of a block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockShape {
    Cube,
    // One box inside the cell, from `min` to `max`.
    Box { min: [u8; 3], max: [u8; 3] },
    // A half slab and a quarter block on the `high_side` half above it, or below it when upside down.
    Stairs { upside_down: bool, high_side: Face },
    // Two crossed quads, seen from both sides.
    Cross,
    Hidden,
}
