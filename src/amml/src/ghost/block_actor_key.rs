use amml_api::SkullPose;
use stratum_core::structs::ghost::ChestPair;

/// Identity of one block entity ghost: cells with the same key share one ghost. The loader fixes a ghost's chest pair
/// and skull pose when it makes the ghost, so both are part of the key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BlockActorKey {
    // palette index of the block state
    pub palette: u16,
    // core block-entity table id, `u32::MAX` when the cell has no NBT
    pub entry: u32,
    // the direction to the other half when the cell is the lead of a large chest, so a large chest never shares a
    // ghost with a single chest of the same NBT
    pub chest: Option<ChestPair>,
    // the face and turn of a skull
    pub skull: Option<SkullPose>,
}

impl BlockActorKey {
    pub fn new(palette: u16, entry: u32, chest: Option<ChestPair>, skull: Option<SkullPose>) -> Self {
        Self { palette, entry, chest, skull }
    }
}
