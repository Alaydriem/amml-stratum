use amml_game::GameEntries;

use crate::types::StratumEntry;

/// The resolved skull draw and the block type name it takes. The optional feature `stratum.skulls`; without it the
/// skull hook is not installed and skull cells are not drawn. A test fills it with fake values.
#[derive(Clone, Copy, Debug)]
pub struct SkullCalls {
    /// `void SkullBlockRenderer::renderSkull(...)`, the skull draw the skull hook swaps a ghost's pose into.
    pub render_skull: usize,
    /// Offset of `mNameInfo.mFullName` (a `HashedString`) in `BlockType`.
    pub block_type_full_name: u64,
}

impl SkullCalls {
    /// The skull calls of the build `entries` was resolved for; `None` when any entry of the feature is missing.
    pub fn new(entries: &GameEntries) -> Option<Self> {
        Some(Self {
            render_skull: entries.get(StratumEntry::SKULL_BLOCK_RENDERER_RENDER_SKULL)? as usize,
            block_type_full_name: entries.get(StratumEntry::BLOCK_TYPE_FULL_NAME)?,
        })
    }
}
