// What the kinds scan reads of the world's blocks
pub trait BlockGrid {
    // whether the chunk at the chunk coordinates is loaded
    fn chunk_loaded(&self, chunk_x: i32, chunk_z: i32) -> bool;

    // the block type of the cell, as a number that tells types apart; None when the block reads null
    fn kind_at(&self, cell: [i32; 3]) -> Option<usize>;

    // the full name of a block type from `kind_at`; None when it cannot be read
    fn kind_name(&self, kind: usize) -> Option<String>;
}
