use std::ffi::c_void;
use std::mem::offset_of;

/// Layout mirror of the game's `BlockTessellatorCache` (272032 bytes): the 20x20x20 block window that
/// `BlockTessellator::tessellateInWorld` reads when its cached calculation is on. The mod fills the window itself;
/// each array index is (x - pos.x) * 400 + (y - pos.y) * 20 + (z - pos.z).
#[repr(C)]
pub struct BlockTessellatorCacheLayout {
    pub air_block: *const c_void,                   // 0x00000 mLocalRegistry.mAirBlock
    pub pos: [i32; 3],                              // 0x00008 mPos
    _pad: [u8; 4],                                  // 0x00014
    pub region: *mut c_void,                        // 0x00018 mRegion
    pub light_colors: [u16; 8000],                  // 0x00020 mLightColors (BrightnessPair)
    pub blocks: [*const c_void; 8000],              // 0x03EA0 mBlocks
    pub extra_blocks: [*const c_void; 8000],        // 0x138A0 mExtraBlocks
    pub block_graphics: [[*const c_void; 8000]; 2], // 0x232A0 mBlockGraphics
}

const _: () = assert!(offset_of!(BlockTessellatorCacheLayout, pos) == 8);
const _: () = assert!(offset_of!(BlockTessellatorCacheLayout, region) == 24);
const _: () = assert!(offset_of!(BlockTessellatorCacheLayout, light_colors) == 32);
const _: () = assert!(offset_of!(BlockTessellatorCacheLayout, blocks) == 16032);
const _: () = assert!(offset_of!(BlockTessellatorCacheLayout, extra_blocks) == 80032);
const _: () = assert!(offset_of!(BlockTessellatorCacheLayout, block_graphics) == 144032);
const _: () = assert!(size_of::<BlockTessellatorCacheLayout>() == 272_032);
