// Which half of its cell a block fills, for blocks that have a top and a bottom half.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Half {
    // The block has no half, or fills the whole cell.
    Any,
    Bottom,
    Top,
}
