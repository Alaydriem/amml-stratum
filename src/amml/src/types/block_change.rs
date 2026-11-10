use std::ffi::c_void;

use amml_game::BlockPos;

/// One block change of the client region, as the game gives it to a listener: the `BlockSource` that fired it, the
/// position, the layer (0 is the block, 1 the water of a waterlogged block) and the new block (`Block const*`).
/// The pointers are live for the call only.
#[derive(Clone, Copy, Debug)]
pub struct BlockChange {
    pub source: *mut c_void,
    pub pos: BlockPos,
    pub layer: u32,
    pub block: *const c_void,
}
