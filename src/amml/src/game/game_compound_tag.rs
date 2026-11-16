use std::ffi::c_void;

use crate::game::StringByteInputLayout;
use crate::types::ghost_calls::HeapDeleteFn;

// std::unique_ptr<CompoundTag> NbtIo::read(IDataInput&): a static function, the hidden result pointer first. On
// 26.51 the body writes one pointer, and gives an empty tag (never null) when the bytes do not read
type ReadFn = unsafe extern "C" fn(*mut *mut c_void, *mut StringByteInputLayout) -> *mut *mut c_void;
// CompoundTag::~CompoundTag(), the destructor that does not free the object
type TagDtorFn = unsafe extern "C" fn(*mut c_void);

/// A `CompoundTag` the game read from little-endian NBT bytes with `NbtIo::read`, owned as its `unique_ptr` would own
/// it: dropping it runs `CompoundTag::~CompoundTag` and frees the object through the game heap, once.
pub struct GameCompoundTag {
    tag: *mut c_void,
    tag_dtor: TagDtorFn,
    heap_delete: HeapDeleteFn,
}

impl GameCompoundTag {
    /// sizeof(CompoundTag): the vtable and the `std::map` (head and size), as `NbtIo::read` allocates it.
    const TAG_BYTES: usize = 0x18;

    /// The tag the game reads from `bytes` through a `StringByteInput` on the stack; `None` when the read gives none.
    ///
    /// # Safety
    /// `nbt_read` and `tag_dtor` must be the addresses of `NbtIo::read` and `CompoundTag::~CompoundTag`,
    /// `input_vtable` the address of the `StringByteInput` vtable, and `heap_delete` the game's sized operator delete,
    /// all of the running build.
    #[allow(unsafe_code)]
    pub unsafe fn read(
        nbt_read: usize,
        input_vtable: usize,
        tag_dtor: usize,
        heap_delete: HeapDeleteFn,
        bytes: &[u8],
    ) -> Option<Self> {
        // SAFETY: the caller guarantees the addresses of the two functions with the signatures above
        let (read, tag_dtor) = unsafe {
            (std::mem::transmute::<usize, ReadFn>(nbt_read), std::mem::transmute::<usize, TagDtorFn>(tag_dtor))
        };
        let mut input = StringByteInputLayout::new(input_vtable as *const c_void, bytes);
        let mut tag: *mut c_void = std::ptr::null_mut();
        // SAFETY: an input with the game's StringByteInput vtable over bytes that live for the call, and a
        // pointer-sized result buffer for the unique_ptr
        unsafe { read(&mut tag, &mut input) };
        if tag.is_null() { None } else { Some(Self { tag, tag_dtor, heap_delete }) }
    }

    /// The live tag, as `CompoundTag const&` arguments take it.
    pub fn as_ptr(&self) -> *const c_void {
        self.tag
    }
}

impl Drop for GameCompoundTag {
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        // SAFETY: the tag is the one `NbtIo::read` allocated with the game's operator new for TAG_BYTES; it is
        // destroyed and freed here once, as its unique_ptr would, and not used again
        unsafe {
            (self.tag_dtor)(self.tag);
            (self.heap_delete)(self.tag.cast(), Self::TAG_BYTES);
        }
    }
}
