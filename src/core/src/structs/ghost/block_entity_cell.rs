use super::{ChestPair, SkullPose};

// One cell of a ghost job that the mod draws with the block actor renderer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GhostBlockEntity {
    // Index into the job's padded `cells`.
    pub cell: u32,
    // Id of the cell's NBT in the frame's `GhostBlockEntityTable`, or `NONE`.
    pub entry: u32,
    // Set on the lead half of a double chest: the direction to the other half.
    pub chest: Option<ChestPair>,
    // Set on a skull or head.
    pub skull: Option<SkullPose>,
}

impl GhostBlockEntity {
    // An `entry` value for a cell with no NBT.
    pub const NONE: u32 = u32::MAX;

    pub fn new(cell: u32, entry: u32, chest: Option<ChestPair>, skull: Option<SkullPose>) -> Self {
        Self { cell, entry, chest, skull }
    }
}
