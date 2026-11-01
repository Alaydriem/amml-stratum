use std::ffi::c_void;
use std::mem::offset_of;

use amml_msvc::SharedPtr;

/// Layout mirror of the game's `mce::TexturePtr` (32 bytes): two `shared_ptr`, the texture data and the resource
/// location. The pointer pairs are kept raw so that the mirror is `Copy` and fits `MsvcVariant`; a copy of a game
/// texture takes its uses through `SharedPtr::clone_ref` on the accessors, and the owner releases them.
/// A plain copy of this type does not take a use: release only a copy that took its uses with `clone_ref`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TexturePtrLayout {
    client_texture: [*const c_void; 2],    // 0x00 mClientTexture
    resource_location: [*const c_void; 2], // 0x10 mResourceLocationPtr
}

impl TexturePtrLayout {
    /// The `std::variant<monostate, TexturePtr, ClientTexture, ServerTexture>` index of a `TexturePtr`.
    pub const VARIANT_INDEX: u8 = 1;

    /// A texture with both pointers empty.
    pub const EMPTY: TexturePtrLayout =
        TexturePtrLayout { client_texture: [std::ptr::null(); 2], resource_location: [std::ptr::null(); 2] };

    #[allow(unsafe_code)]
    pub fn client_texture(&self) -> &SharedPtr<c_void> {
        // SAFETY: SharedPtr is #[repr(C)] {element pointer, control block pointer}, the same size and alignment as
        // the two raw pointers, and has no invariant beyond what the game wrote.
        unsafe { &*self.client_texture.as_ptr().cast::<SharedPtr<c_void>>() }
    }

    #[allow(unsafe_code)]
    pub fn resource_location(&self) -> &SharedPtr<c_void> {
        // SAFETY: as in `client_texture`.
        unsafe { &*self.resource_location.as_ptr().cast::<SharedPtr<c_void>>() }
    }

    /// A copy that holds one more use of each pointer, as the `TexturePtr` copy constructor takes them. The owner of
    /// the copy gives them back with `release`.
    ///
    /// # Safety
    /// Each pointer must be empty or hold a live use, as a texture the game holds does.
    #[allow(unsafe_code)]
    pub unsafe fn clone_ref(&self) -> TexturePtrLayout {
        // SAFETY: the caller guarantees live uses; SharedPtr has no Drop, so the copies are moved into raw pairs
        // that hold the uses they took
        unsafe {
            TexturePtrLayout {
                client_texture: std::mem::transmute::<SharedPtr<c_void>, [*const c_void; 2]>(
                    self.client_texture().clone_ref(),
                ),
                resource_location: std::mem::transmute::<SharedPtr<c_void>, [*const c_void; 2]>(
                    self.resource_location().clone_ref(),
                ),
            }
        }
    }

    /// Gives back the use of each pointer, as the `TexturePtr` destructor does, and leaves both empty.
    ///
    /// # Safety
    /// The copy must hold the uses it gives back: one from `clone_ref`, never a plain byte copy of a game texture.
    #[allow(unsafe_code)]
    pub unsafe fn release(&mut self) {
        // SAFETY: the same layout as in `client_texture`; the caller guarantees the uses
        unsafe {
            (*self.client_texture.as_mut_ptr().cast::<SharedPtr<c_void>>()).release();
            (*self.resource_location.as_mut_ptr().cast::<SharedPtr<c_void>>()).release();
        }
    }
}

const _: () = assert!(offset_of!(TexturePtrLayout, resource_location) == 0x10);
const _: () = assert!(size_of::<TexturePtrLayout>() == 32);
const _: () = assert!(size_of::<SharedPtr<c_void>>() == 16);
