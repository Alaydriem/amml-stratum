use std::ffi::c_void;
use std::mem::MaybeUninit;

use amml_game::ItemInstanceLayout;

// `ItemStackBase::~ItemStackBase()`: rcx the item instance
type DtorFn = unsafe extern "C" fn(*mut c_void);

/// An `ItemInstance` the game returned by value into the mod's memory, such as the one `BlockType::asItemInstance`
/// gives. It is destroyed once, when it is dropped, through the `ItemStackBase` destructor (an `ItemInstance` adds no
/// members).
pub struct OwnedItemInstance {
    item: Box<MaybeUninit<ItemInstanceLayout>>,
    dtor: usize,
}

impl OwnedItemInstance {
    /// Room for one item instance, zeroed, that `fill` hands to the game; dropped without a destructor call when
    /// `fill` returns false.
    ///
    /// # Safety
    /// When `fill` returns true it must have constructed an `ItemInstance` in the room, which `dtor` (the address of
    /// `ItemStackBase::~ItemStackBase`) destroys.
    #[allow(unsafe_code)]
    pub unsafe fn made_by(dtor: usize, fill: impl FnOnce(*mut ItemInstanceLayout) -> bool) -> Option<Self> {
        let mut item = Box::new(MaybeUninit::<ItemInstanceLayout>::zeroed());
        fill(item.as_mut_ptr()).then_some(Self { item, dtor })
    }

    /// The item instance's members.
    pub fn layout(&self) -> &ItemInstanceLayout {
        // SAFETY: `made_by` keeps only an instance the game constructed
        #[allow(unsafe_code)]
        unsafe {
            self.item.assume_init_ref()
        }
    }
}

impl Drop for OwnedItemInstance {
    fn drop(&mut self) {
        if self.dtor == 0 {
            return;
        }
        // SAFETY: the entry is the address of the ItemStackBase destructor body
        #[allow(unsafe_code)]
        let dtor: DtorFn = unsafe { std::mem::transmute(self.dtor) };
        // SAFETY: the instance the game constructed, destroyed once; the box keeps its memory
        #[allow(unsafe_code)]
        unsafe {
            dtor(self.item.as_mut_ptr().cast())
        };
    }
}
