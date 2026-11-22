use amml_api::{BlockHandle, Blocks};

use crate::services::PaletteBlocks;
use crate::types::PaletteLookup;

/// Turns the ghost palette into block handles of the joined world, because a handle is valid only in the world it was
/// looked up in. Each entry is looked up by name and states, the loader falling back to the name with the default
/// states, then as the game's unknown block; each fallback is counted for the next report. An entry that gives no
/// block at all is drawn as air, so every palette id names a handle the mesher accepts.
///
/// Bound: one entry per palette id, sized by core's palette, which is bounded below `u16::MAX` entries.
pub struct BlockResolver {
    handles: Vec<BlockHandle>,
    // per palette id: the entry is air, or no block was found for it, so it holds no block entity
    empty: Vec<bool>,
    air: Option<BlockHandle>,
    unknown: Option<BlockHandle>,
    unknown_count: u32,
    default_state_count: u32,
}

impl BlockResolver {
    /// The palette id of an empty cell.
    pub const EMPTY: u16 = u16::MAX;
    pub const AIR: &'static str = "minecraft:air";

    pub fn new() -> Self {
        Self {
            handles: Vec::new(),
            empty: Vec::new(),
            air: None,
            unknown: None,
            unknown_count: 0,
            default_state_count: 0,
        }
    }

    /// Looks up air and the unknown block for a newly joined world and drops the palette; false when air is missing,
    /// which no ghost can be built without.
    pub fn join(&mut self, blocks: &Blocks) -> bool {
        self.leave();
        self.air = blocks.from_state(Self::AIR, &[]).map(|found| found.block);
        self.unknown = PaletteBlocks::unknown(blocks);
        self.air.is_some()
    }

    /// Drops every handle; they belong to the world that was left.
    pub fn leave(&mut self) {
        self.handles.clear();
        self.empty.clear();
        self.air = None;
        self.unknown = None;
    }

    /// Resolves the palette `entries` (each one entry's NBT bytes) in place of the last palette, each block turned by
    /// `rotation` quarter turns (0 to 3, clockwise seen from above) by the game. The entries are always the unturned
    /// states, so no block is turned twice.
    pub fn resolve<'a>(&mut self, blocks: &Blocks, entries: impl Iterator<Item = &'a [u8]>, rotation: u8) {
        self.handles.clear();
        self.empty.clear();
        let Some(air) = self.air else { return };
        for entry in entries {
            let found = match PaletteBlocks::lookup(blocks, entry) {
                PaletteLookup::Exact(block) => Some(self.turn(blocks, block, rotation)),
                PaletteLookup::Default(block) => {
                    self.default_state_count += 1;
                    Some(self.turn(blocks, block, rotation))
                }
                PaletteLookup::Unknown => {
                    self.unknown_count += 1;
                    self.unknown
                }
            };
            let is_air = found.is_none_or(|block| block == air);
            self.handles.push(found.unwrap_or(air));
            self.empty.push(is_air);
        }
    }

    /// The handles of the palette, in palette id order, for the mesher's grid.
    pub fn handles(&self) -> &[BlockHandle] {
        &self.handles
    }

    /// The block of palette id `id`; None for an empty cell, an id outside the palette and an entry that is air.
    pub fn block(&self, id: u16) -> Option<BlockHandle> {
        let index = usize::from(id);
        if id == Self::EMPTY || self.empty.get(index).copied().unwrap_or(true) {
            return None;
        }
        self.handles.get(index).copied()
    }

    /// Whether a palette is resolved for the joined world.
    pub fn is_resolved(&self) -> bool {
        !self.handles.is_empty()
    }

    /// The entries that fell back to the unknown block since the last call, for the report.
    pub fn take_unknown(&mut self) -> u32 {
        std::mem::take(&mut self.unknown_count)
    }

    /// The entries that fell back to the default states since the last call, for the report.
    pub fn take_default_state(&mut self) -> u32 {
        std::mem::take(&mut self.default_state_count)
    }

    // the unknown block is never turned, nor is a block when there is no turn
    fn turn(&self, blocks: &Blocks, block: BlockHandle, rotation: u8) -> BlockHandle {
        if rotation == 0 || Some(block) == self.unknown {
            return block;
        }
        blocks.rotated(block, rotation).unwrap_or(block)
    }
}

impl Default for BlockResolver {
    fn default() -> Self {
        Self::new()
    }
}
