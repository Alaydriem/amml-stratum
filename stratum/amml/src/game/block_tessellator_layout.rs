use std::ffi::c_void;
use std::mem::offset_of;

use super::BlockTessellatorCacheLayout;

/// Layout mirror of the game's `BlockTessellator` (293344 bytes). The mod allocates it with the game allocator,
/// constructs it with the game's constructor and writes the flags, the rendering layer and the block cache window
/// before `tessellateInWorld`. `BlockTessellator.mBlockCache` is the anchor entry.
#[repr(C)]
pub struct BlockTessellatorLayout {
    _flag0: u8,                                   // 0x000
    pub use_normals: bool,                        // 0x001 mUseNormals
    _head: [u8; 6],                               // 0x002
    pub region: *mut c_void,                      // 0x008 mRegion
    _to_layer: [u8; 0x80 - 0x10],                 // 0x010
    pub rendering_layer: i32,                     // 0x080 mRenderingLayer
    _to_ao: [u8; 0xE8 - 0x84],                    // 0x084
    pub apply_ambient_occlusion: bool,            // 0x0E8 mApplyAmbientOcclusion
    _to_cached: [u8; 0xF1 - 0xE9],                // 0x0E9
    pub use_cached_calculation: bool,             // 0x0F1 mUseCachedCalculation
    _to_cache: [u8; 0x6B0 - 0xF2],                // 0x0F2
    pub block_cache: BlockTessellatorCacheLayout, // 0x6B0 mBlockCache
    _tail: [u8; 293_344 - 0x6B0 - 272_032],       // 0x42D50
}

const _: () = assert!(offset_of!(BlockTessellatorLayout, use_normals) == 1);
const _: () = assert!(offset_of!(BlockTessellatorLayout, region) == 8);
const _: () = assert!(offset_of!(BlockTessellatorLayout, rendering_layer) == 128);
const _: () = assert!(offset_of!(BlockTessellatorLayout, apply_ambient_occlusion) == 232);
const _: () = assert!(offset_of!(BlockTessellatorLayout, use_cached_calculation) == 241);
const _: () = assert!(offset_of!(BlockTessellatorLayout, block_cache) == 1712);
const _: () = assert!(size_of::<BlockTessellatorLayout>() == 293_344);
