use std::ffi::c_void;

use amml_game::HashedStringLayout;
use amml_msvc::{MsvcHeap, MsvcString};

use crate::types::GhostCalls;

// LookupByNameImplReturnType BlockTypeRegistry::_lookupByNameImpl(HashedString const& name, int data,
// LookupByNameImplResolve resolve, bool logNotFound) const: a member that returns its two pointers (the block type,
// then the block) through the hidden pointer
type LookupFn = unsafe extern "C" fn(
    *const c_void,
    *mut [*const c_void; 2],
    *const HashedStringLayout,
    i32,
    i32,
    u8,
) -> *mut c_void;

/// The game's `BlockTypeRegistry`, for lookups by name.
pub struct BlockRegistry {
    registry: usize,
    lookup: usize,
    heap: MsvcHeap,
}

impl BlockRegistry {
    /// `LookupByNameImplResolve::Block`: the lookup also gives the block with the default states.
    const RESOLVE_BLOCK: i32 = 1;

    pub fn new(calls: &GhostCalls) -> Self {
        Self {
            registry: calls.block_type_registry,
            lookup: calls.lookup_by_name,
            heap: MsvcHeap::new(calls.heap_new, calls.heap_delete),
        }
    }

    /// The block (`Block const*`) of the type named `name` with its default states; null when the game knows no
    /// such type or the name cannot be passed.
    #[allow(unsafe_code)]
    pub fn by_name(&self, name: &str) -> *const c_void {
        // SAFETY: the heap is the game's allocator, as `new` was given it
        let Ok(text) = (unsafe { MsvcString::new_in(&self.heap, name) }) else { return std::ptr::null() };
        let mut hashed = HashedStringLayout::new(text);
        // SAFETY: the entry is the address of `_lookupByNameImpl` with the signature of `LookupFn`
        let lookup: LookupFn = unsafe { std::mem::transmute(self.lookup) };
        let mut result = [std::ptr::null::<c_void>(); 2];
        // SAFETY: the live registry object, a result buffer and a HashedString that live for the call; data 0, and
        // no log line for a name the game does not know
        unsafe { lookup(self.registry as *const c_void, &mut result, &hashed, 0, Self::RESOLVE_BLOCK, 0) };
        // SAFETY: the text's heap block, if any, came from this heap in `new_in`, and the game kept no reference
        unsafe { hashed.text.release(&self.heap) };
        result[1]
    }
}
