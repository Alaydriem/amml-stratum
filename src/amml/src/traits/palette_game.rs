use amml_api::BlockHandle;

use crate::types::PaletteLookup;

// What the palette probe asks of the game: the block of a palette entry, and the block the game's own turn gives. The
// blocks are handles of the world they were looked up in
pub trait PaletteGame {
    // the block of the NBT bytes of one palette entry
    fn lookup(&self, nbt: &[u8]) -> PaletteLookup;

    // `block` turned by `quarters` clockwise quarter turns, as the game places a structure; None when the game
    // refused the turn
    fn turn(&self, block: BlockHandle, quarters: u8) -> Option<BlockHandle>;
}
