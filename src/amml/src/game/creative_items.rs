use std::ffi::c_void;

use amml_game::{CreativeItemEntryLayout, ItemInstanceLayout, RawObject};
use amml_msvc::{MemberCall, RefCountBase};

use crate::types::PlayerCalls;

// `uchar ItemStackBase::getMaxStackSize() const`: rcx the item instance, the size in al
type MaxStackSizeFn = unsafe extern "C" fn(*const c_void) -> u8;

/// The creative item list of the player's level, as `ItemRegistryRef::getCreativeItemRegistry` gives it: the
/// `ItemRegistryRef` (a weak pointer) and the `NonOwnerPointer<CreativeItemRegistry>` (a shared control block and the
/// registry) are held while the list is read, and each is released once when this is dropped.
pub struct CreativeItems {
    // ItemRegistryRef: the std::weak_ptr<ItemRegistry> (element, control block)
    registry_ref: [usize; 2],
    // NonOwnerPointer<CreativeItemRegistry>: the shared_ptr<ControlBlock> (element, control block), then the registry
    creative: [usize; 3],
    creative_items: u64,
    get_max_stack_size: usize,
}

impl CreativeItems {
    /// The creative items of `player`'s level; None for a null player, level or registry.
    ///
    /// # Safety
    /// `player` must be null or the live local `Player` of the build `calls` was resolved for.
    #[allow(unsafe_code)]
    pub unsafe fn of_player(player: *mut c_void, calls: &PlayerCalls) -> Option<Self> {
        if player.is_null() || calls.get_creative_item_registry == 0 {
            return None;
        }
        // SAFETY: the caller guarantees a live player; the entry is the offset of its Level pointer
        let level = unsafe { RawObject::pointer(player, calls.actor_level) }?;
        // SAFETY: a live Level, whose vtable holds getItemRegistry at the entry's slot
        let function = unsafe { RawObject::slot(level, calls.level_get_item_registry_slot) }?;
        // SAFETY: `ItemRegistryRef getItemRegistry() const`, a member function that returns a class through the
        // hidden pointer
        let registry_ref = unsafe { MemberCall::call_sret0::<[usize; 2]>(function, level) };
        // SAFETY: the entry is the address of the non-const getCreativeItemRegistry; `this` is the registry ref
        let creative = unsafe {
            MemberCall::call_sret0::<[usize; 3]>(
                calls.get_creative_item_registry as *const c_void,
                registry_ref.as_ptr().cast_mut().cast(),
            )
        };
        let made = Self {
            registry_ref,
            creative,
            creative_items: calls.creative_items,
            get_max_stack_size: calls.get_max_stack_size,
        };
        (made.creative[2] != 0).then_some(made)
    }

    /// The entries of the creative menu, in its order.
    pub fn entries(&self) -> &[CreativeItemEntryLayout] {
        // SAFETY: the live registry, held for the life of this value, holds its std::vector<CreativeItemEntry> at
        // the anchor's offset
        #[allow(unsafe_code)]
        let Some([first, last]) =
            (unsafe { RawObject::read::<[usize; 2]>(self.creative[2] as *const c_void, self.creative_items) })
        else {
            return &[];
        };
        let size = size_of::<CreativeItemEntryLayout>();
        if first == 0 || last < first || (last - first) % size != 0 {
            return &[];
        }
        // SAFETY: the vector's live entries, 176 bytes each
        #[allow(unsafe_code)]
        unsafe {
            std::slice::from_raw_parts(first as *const CreativeItemEntryLayout, (last - first) / size)
        }
    }

    /// The most items one stack of `item` holds, as `ItemStackBase::getMaxStackSize` gives it.
    pub fn max_stack_size(&self, item: &ItemInstanceLayout) -> u8 {
        if self.get_max_stack_size == 0 {
            return 1;
        }
        // SAFETY: the entry is the address of getMaxStackSize
        #[allow(unsafe_code)]
        let call: MaxStackSizeFn = unsafe { std::mem::transmute(self.get_max_stack_size) };
        // SAFETY: a live item instance of a creative entry
        #[allow(unsafe_code)]
        unsafe {
            call(std::ptr::from_ref(item).cast())
        }
    }
}

impl Drop for CreativeItems {
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        let control = self.creative[1] as *mut RefCountBase;
        if !control.is_null() {
            // SAFETY: the getter gave the shared_ptr with one use, which this value owns and gives back once
            unsafe { RefCountBase::decref(control) };
        }
        let rep = self.registry_ref[1] as *mut RefCountBase;
        if !rep.is_null() {
            // SAFETY: the weak_ptr the level gave holds one weak reference, given back once
            unsafe { RefCountBase::decwref(rep) };
        }
    }
}
