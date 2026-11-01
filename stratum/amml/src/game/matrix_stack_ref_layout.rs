use std::ffi::c_void;
use std::mem::offset_of;

/// Layout mirror of the game's `MatrixStack::MatrixStackRef` (16 bytes): the stack it pushed, then the pushed
/// matrix, a `glm::mat4` stored by columns. `MatrixStack::push` returns it through the hidden pointer, and its
/// destructor pops the matrix.
#[repr(C)]
pub struct MatrixStackRefLayout {
    pub stack: *mut c_void,         // 0x00 stack
    pub matrix: *mut [[f32; 4]; 4], // 0x08 mat
}

const _: () = assert!(offset_of!(MatrixStackRefLayout, matrix) == 0x08);
const _: () = assert!(size_of::<MatrixStackRefLayout>() == 16);
