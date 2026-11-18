// How a block behaves when it is placed, which sets the order of a build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockClass {
    // Stays where it is put.
    Plain,
    // Needs a block to hold it, and drops when that block is missing.
    Attached,
    // Hangs from the block above it, so it goes after the layer above is placed.
    Hanging,
    // Falls when the cell below it is empty.
    Gravity,
    // Water or lava.
    Fluid,
}
