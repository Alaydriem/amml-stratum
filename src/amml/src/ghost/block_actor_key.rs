use stratum_core::structs::ghost::ChestPair;

/// Identity of one block actor: cells with the same key share one actor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BlockActorKey {
    // palette index of the block state
    pub palette: u16,
    // core block-entity table id, `u32::MAX` when the cell has no NBT
    pub entry: u32,
    // the direction to the other half when the cell is the lead of a large chest, so a large chest never shares an
    // actor with a single chest of the same NBT
    pub chest: Option<ChestPair>,
}

impl BlockActorKey {
    pub fn new(palette: u16, entry: u32, chest: Option<ChestPair>) -> Self {
        Self { palette, entry, chest }
    }
}
