use std::ffi::c_void;
use std::ptr::NonNull;

use amml_game::RawObject;

use crate::types::ListenerSlots;

// void BlockSource::addListener(BlockSourceListener&) and removeListener, the same signature
type ListenerCallFn = unsafe extern "C" fn(*mut c_void, *mut c_void);

/// A `BlockSource*` of the game, for its listener calls.
pub struct ListenerSource {
    ptr: NonNull<c_void>,
    slots: ListenerSlots,
}

impl ListenerSource {
    /// `None` for a null pointer.
    ///
    /// # Safety
    /// `ptr` must be null or a live `BlockSource` whose vtable holds `addListener` and `removeListener` at the
    /// slots in `slots`, and must stay live while the wrapper is used.
    #[allow(unsafe_code)]
    pub unsafe fn from_raw(ptr: *mut c_void, slots: ListenerSlots) -> Option<Self> {
        NonNull::new(ptr).map(|ptr| Self { ptr, slots })
    }

    pub fn as_ptr(&self) -> *mut c_void {
        self.ptr.as_ptr()
    }

    // calls the listener function in `slot`; false when the slot is null
    #[allow(unsafe_code)]
    fn listener_call(&self, slot: u64, listener: *mut c_void) -> bool {
        // SAFETY: `from_raw` guarantees a live BlockSource, whose first qword is its vtable, holding `slot`
        let Some(function) = (unsafe { RawObject::slot(self.ptr.as_ptr(), slot) }) else { return false };
        // SAFETY: the slot holds `addListener` or `removeListener`, with the signature of `ListenerCallFn`
        let call: ListenerCallFn = unsafe { std::mem::transmute(function) };
        // SAFETY: a live BlockSource and the caller's live listener
        unsafe { call(self.ptr.as_ptr(), listener) };
        true
    }

    /// Adds `listener`, which must stay live until it is removed or the source calls its `onSourceDestroyed`. Game
    /// thread.
    pub fn add_listener(&self, listener: *mut c_void) -> bool {
        self.listener_call(self.slots.add_listener, listener)
    }

    /// Removes `listener`. Game thread.
    pub fn remove_listener(&self, listener: *mut c_void) -> bool {
        self.listener_call(self.slots.remove_listener, listener)
    }
}
