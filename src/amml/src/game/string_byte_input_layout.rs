use std::ffi::c_void;
use std::mem::offset_of;

/// Layout mirror of the game's `StringByteInput` (32 bytes): an `IDataInput` over a `std::string_view`, read from
/// `mIdx` on. The mod builds one on its stack for `NbtIo::read`, with the vtable from `StringByteInput.vtable`, as the
/// game builds one inline.
#[repr(C)]
pub struct StringByteInputLayout {
    pub vtable: *const c_void, // 0x00
    pub index: usize,          // 0x08 mIdx
    pub data: *const u8,       // 0x10 mBuffer data
    pub len: usize,            // 0x18 mBuffer size
}

impl StringByteInputLayout {
    /// An input over `bytes` from the start. `bytes` must outlive every read through it.
    pub fn new(vtable: *const c_void, bytes: &[u8]) -> Self {
        Self { vtable, index: 0, data: bytes.as_ptr(), len: bytes.len() }
    }
}

const _: () = assert!(offset_of!(StringByteInputLayout, index) == 0x08);
const _: () = assert!(offset_of!(StringByteInputLayout, data) == 0x10);
const _: () = assert!(size_of::<StringByteInputLayout>() == 32);
