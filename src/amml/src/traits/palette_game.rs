use std::ffi::c_void;

use crate::types::NbtLookup;

// What the palette probe asks of the game: the block of a palette entry, and the block the game's own turn gives. The
// blocks are pointers of the world they were looked up in
pub trait PaletteGame {
    // the block of the NBT bytes of one palette entry
    fn lookup(&self, nbt: &[u8]) -> NbtLookup;

    // `block` turned by `quarters` clockwise quarter turns, as the game places a structure
    fn turn(&self, block: *const c_void, quarters: u8) -> *const c_void;
}
