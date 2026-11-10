use std::ffi::c_void;
use std::mem::offset_of;

/// Layout mirror of the start of the game's `mce::MeshContext` (no vtable), which sits `MeshContextOffset::BYTES`
/// into a `ScreenContext`: the render context, the camera and the global constant buffers, each a reference
/// stored as a pointer. A prefix: the mod only reads through the game's pointer.
#[repr(C)]
pub struct MeshContextLayout {
    pub render_context: *mut c_void,   // 0x00 renderContext
    pub camera: *mut c_void,           // 0x08 camera
    pub constant_buffers: *mut c_void, // 0x10 constantBuffers
}

const _: () = assert!(offset_of!(MeshContextLayout, camera) == 0x08);
const _: () = assert!(offset_of!(MeshContextLayout, constant_buffers) == 0x10);
