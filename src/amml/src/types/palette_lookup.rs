use amml_api::BlockHandle;

/// What the game gave for one palette entry, a handle of the world it was looked up in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteLookup {
    /// The block with the entry's name and states.
    Exact(BlockHandle),
    /// The block with the entry's name and the default states: the states were not a state set the game knows.
    Default(BlockHandle),
    /// Neither: the bytes were empty, did not read, or named a block the game does not know.
    Unknown,
}

impl PaletteLookup {
    /// The block; None for `Unknown`.
    pub fn block(&self) -> Option<BlockHandle> {
        match self {
            Self::Exact(block) | Self::Default(block) => Some(*block),
            Self::Unknown => None,
        }
    }
}
