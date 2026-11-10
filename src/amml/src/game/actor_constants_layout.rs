use std::ffi::c_void;
use std::mem::offset_of;

/// Layout mirror of the game's `mce::ActorConstants` (104 bytes): the constant-buffer base, then the 11 shader
/// constant pointers that `ActorShaderManager::setEntityConstants` writes. The mod passes the game's object at
/// `GlobalConstantBuffers.entityConstantBuffer`; it only needs the size and the order.
#[repr(C)]
pub struct ActorConstantsLayout {
    _base: [u8; 0x10],                                 // 0x00 ConstantBufferConstantsBase
    pub overlay_color: *mut c_void,                    // 0x10 OVERLAY_COLOR
    pub tile_light_color: *mut c_void,                 // 0x18 TILE_LIGHT_COLOR
    pub tile_light_intensity: *mut c_void,             // 0x20 TILE_LIGHT_INTENSITY
    pub block_light_color: *mut c_void,                // 0x28 BLOCK_LIGHT_COLOR
    pub change_color: *mut c_void,                     // 0x30 CHANGE_COLOR
    pub glint_color: *mut c_void,                      // 0x38 GLINT_COLOR
    pub uv_anim: *mut c_void,                          // 0x40 UV_ANIM
    pub multiplicative_tint_change_color: *mut c_void, // 0x48 MULTIPLICATIVE_TINT_CHANGE_COLOR
    pub uv_offset: *mut c_void,                        // 0x50 UV_OFFSET
    pub uv_rotation: *mut c_void,                      // 0x58 UV_ROTATION
    pub uv_scale: *mut c_void,                         // 0x60 UV_SCALE
}

const _: () = assert!(offset_of!(ActorConstantsLayout, overlay_color) == 0x10);
const _: () = assert!(offset_of!(ActorConstantsLayout, glint_color) == 0x38);
const _: () = assert!(offset_of!(ActorConstantsLayout, uv_scale) == 0x60);
const _: () = assert!(size_of::<ActorConstantsLayout>() == 104);
