use std::ffi::c_void;

use crate::game::{BlockCalls, BlockRegistry, NbtBlock};
use crate::types::{GhostCalls, NbtLookup};

/// Turns the ghost palette into the game's blocks for one world, because a `Block` pointer is valid only in the
/// world it was looked up in. Each entry is read from its NBT bytes, then looked up by name and states, then by name
/// with the default states, then as the game's unknown block; each fallback is counted for the next report.
///
/// Bound: one entry per palette id, sized by core's palette, which is bounded below `u16::MAX` entries.
pub struct BlockResolver {
    registry: BlockRegistry,
    nbt: NbtBlock,
    game: BlockCalls,
    blocks: Vec<*const c_void>,
    graphics: Vec<*const c_void>,
    layers: Vec<i32>,
    air: *const c_void,
    air_graphics: *const c_void,
    stone: *const c_void,
    unknown: *const c_void,
    unknown_count: u32,
    default_state_count: u32,
}

impl BlockResolver {
    /// The palette id of an empty cell.
    pub const EMPTY: u16 = u16::MAX;

    pub fn new(calls: &GhostCalls) -> Self {
        Self {
            registry: BlockRegistry::new(calls),
            nbt: NbtBlock::new(BlockRegistry::new(calls), calls),
            game: BlockCalls::new(calls),
            blocks: Vec::new(),
            graphics: Vec::new(),
            layers: Vec::new(),
            air: std::ptr::null(),
            air_graphics: std::ptr::null(),
            stone: std::ptr::null(),
            unknown: std::ptr::null(),
            unknown_count: 0,
            default_state_count: 0,
        }
    }

    /// Looks up air, stone and the unknown block for a newly joined world and drops the palette; false when air or
    /// stone is missing, which no ghost can be built without.
    #[allow(unsafe_code)]
    pub fn join(&mut self) -> bool {
        self.leave();
        self.air = self.registry.by_name("minecraft:air");
        self.stone = self.registry.by_name("minecraft:stone");
        self.unknown = self.registry.by_name("minecraft:unknown");
        // SAFETY: a block the registry gave for this world, or null
        self.air_graphics = unsafe { self.game.graphics(self.air) };
        curia::info!("ghost blocks joined the world", {
            air: !self.air.is_null(),
            stone: !self.stone.is_null(),
            unknown: !self.unknown.is_null()
        });
        !self.air.is_null() && !self.stone.is_null()
    }

    /// Drops every lookup; the pointers belong to the world that was left.
    pub fn leave(&mut self) {
        self.blocks.clear();
        self.graphics.clear();
        self.layers.clear();
        self.air = std::ptr::null();
        self.air_graphics = std::ptr::null();
        self.stone = std::ptr::null();
        self.unknown = std::ptr::null();
    }

    /// Resolves the palette `entries` (each one entry's NBT bytes) in place of the last palette, each block turned
    /// by `rotation` quarter turns (0 to 3, clockwise seen from above). The entries are always the unturned states,
    /// so no block is turned twice.
    #[allow(unsafe_code)]
    pub fn resolve<'a>(&mut self, entries: impl Iterator<Item = &'a [u8]>, rotation: u8) {
        self.blocks.clear();
        self.graphics.clear();
        self.layers.clear();
        for entry in entries {
            let found = self.lookup(entry);
            let block = self.turn(found, rotation);
            // SAFETY: a block the game gave for this world, or null, and the graphics it gave for that block
            let (graphics, layer) = unsafe {
                let graphics = self.game.graphics(block);
                (graphics, self.game.render_layer(graphics))
            };
            self.blocks.push(block);
            self.graphics.push(graphics);
            self.layers.push(layer);
        }
    }

    /// The block of palette id `id`; null for an empty cell, an id outside the palette or an entry that did not
    /// resolve at all.
    pub fn block(&self, id: u16) -> *const c_void {
        if id == Self::EMPTY {
            return std::ptr::null();
        }
        self.blocks.get(usize::from(id)).copied().unwrap_or(std::ptr::null())
    }

    /// The graphics of palette id `id`; null where `block` is null.
    pub fn graphics(&self, id: u16) -> *const c_void {
        self.graphics.get(usize::from(id)).copied().unwrap_or(std::ptr::null())
    }

    /// The render layer of palette id `id`; 0 where `block` is null.
    pub fn layer(&self, id: u16) -> i32 {
        self.layers.get(usize::from(id)).copied().unwrap_or(0)
    }

    pub fn air(&self) -> *const c_void {
        self.air
    }

    pub fn air_graphics(&self) -> *const c_void {
        self.air_graphics
    }

    pub fn stone(&self) -> *const c_void {
        self.stone
    }

    /// The registry, for lookups by name in the joined world.
    pub fn registry(&self) -> &BlockRegistry {
        &self.registry
    }

    /// The entries that fell back to the unknown block since the last call, for the report.
    pub fn take_unknown(&mut self) -> u32 {
        std::mem::take(&mut self.unknown_count)
    }

    /// The entries that fell back to the default states since the last call, for the report.
    pub fn take_default_state(&mut self) -> u32 {
        std::mem::take(&mut self.default_state_count)
    }

    // name and states first, then the name with its default states, then the game's unknown block
    fn lookup(&mut self, nbt: &[u8]) -> *const c_void {
        match self.nbt.lookup(nbt) {
            NbtLookup::Exact(block) => block,
            NbtLookup::Default(block) => {
                self.default_state_count += 1;
                block
            }
            NbtLookup::Unknown => {
                self.unknown_count += 1;
                self.unknown
            }
        }
    }

    // the unknown block is never turned
    #[allow(unsafe_code)]
    fn turn(&self, block: *const c_void, rotation: u8) -> *const c_void {
        if block.is_null() || block == self.unknown {
            return block;
        }
        // SAFETY: a block the game gave for this world
        unsafe { self.game.turn(block, rotation) }
    }
}
