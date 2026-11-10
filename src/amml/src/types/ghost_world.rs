use std::ffi::c_void;

use amml_game::BlockPos;

use crate::game::TexturePtrLayout;

/// The world the ghost is drawn in on one frame: the client region (`BlockSource*`), the block of the player's
/// feet, the blend-block material (`mce::MaterialPtr const*`, null when there is none) and the level renderer's
/// atlas texture (null when there is none). Every pointer is live for the frame.
#[derive(Clone, Copy, Debug)]
pub struct GhostWorld {
    pub region: *mut c_void,
    pub feet: BlockPos,
    pub blend: *const c_void,
    pub atlas: *const TexturePtrLayout,
}
