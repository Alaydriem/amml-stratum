use std::cell::Cell;
use std::ffi::c_void;
use std::panic::{AssertUnwindSafe, catch_unwind};

use amml_game::BlockPos;
use amml_msvc::{MsvcHeap, RustObject};
use amml_sdk::{ModRuntime, PanicText};

use crate::errors::ListenerError;
use crate::game::ListenerSource;
use crate::services::BlockChangeHub;
use crate::types::{BlockChange, ListenerSlots};

/// The C++ `BlockSourceListener` stratum registers with the client region: a Rust object on the game heap whose
/// vtable is a copy of the game's 9-slot listener table with slot 0 (the destructor), `onSourceDestroyed` and
/// `onBlockChanged` replaced by the thunks here. The other slots keep the game's empty defaults. Each block change
/// goes to the hub; a destroyed source is forgotten, so no later call reaches it. The object and its table are freed
/// only by `release`, once no game source holds the object. Game thread.
pub struct BlockChangeListener {
    // the object's address
    object: usize,
    vtable: &'static [usize; BlockChangeListener::SLOTS],
    slots: ListenerSlots,
    hub: &'static BlockChangeHub,
    // a source could not be asked to remove the object, so it may still hold it: never freed
    stranded: Cell<bool>,
}

type ListenerObject = RustObject<[usize; BlockChangeListener::SLOTS], &'static BlockChangeHub>;

// void onBlockChanged(BlockSource&, BlockPos const&, uint layer, Block const&, Block const& old, int flags,
// ActorBlockSyncMessage const*, BlockChangedEventTarget, Actor*), with `this` first
type BlockChangedThunk = extern "C" fn(
    *mut c_void,
    *mut c_void,
    *const BlockPos,
    u32,
    *const c_void,
    *const c_void,
    i32,
    *const c_void,
    u8,
    *mut c_void,
);

// SAFETY: the listener is used on the game thread only, under the runtime's lock; another thread only moves it
#[allow(unsafe_code)]
unsafe impl Send for BlockChangeListener {}

impl BlockChangeListener {
    pub const SLOTS: usize = 9;

    /// The two listener slots must be inside the table and must not be the destructor.
    pub fn check(slots: &ListenerSlots) -> Result<(), ListenerError> {
        for slot in [slots.source_destroyed, slots.block_changed] {
            if slot == 0 || slot >= Self::SLOTS as u64 {
                return Err(ListenerError::SlotOutOfRange { slot });
            }
        }
        Ok(())
    }

    /// Copies the table at `base`, puts the thunks in and allocates the object on `heap`.
    ///
    /// # Safety
    /// `base` must be the address of a table of at least 9 code pointers, and the heap's functions must be valid
    /// operator new and operator delete entry points.
    #[allow(unsafe_code)]
    pub unsafe fn new(
        base: u64,
        slots: ListenerSlots,
        hub: &'static BlockChangeHub,
        heap: &MsvcHeap,
    ) -> Result<Self, ListenerError> {
        Self::check(&slots)?;
        let base = usize::try_from(base).map_err(|_| ListenerError::NoBaseTable)? as *const usize;
        if base.is_null() {
            return Err(ListenerError::NoBaseTable);
        }
        let mut table = [0usize; Self::SLOTS];
        for (slot, entry) in table.iter_mut().enumerate() {
            // SAFETY: the caller guarantees 9 readable code pointers at `base`
            *entry = unsafe { base.add(slot).read_unaligned() };
        }
        let destructor: extern "C" fn(*mut c_void, u32) -> *mut c_void = Self::destructor;
        let destroyed: extern "C" fn(*mut c_void, *mut c_void) = Self::source_destroyed_thunk;
        let changed: BlockChangedThunk = Self::block_changed_thunk;
        table[0] = destructor as usize;
        table[slots.source_destroyed as usize] = destroyed as usize;
        table[slots.block_changed as usize] = changed as usize;
        let vtable: &'static [usize; Self::SLOTS] = Box::leak(Box::new(table));
        // SAFETY: forwarded to the caller
        let object = unsafe { ListenerObject::allocate_in(heap, vtable, hub) }?;
        Ok(Self { object: object as usize, vtable, slots, hub, stranded: Cell::new(false) })
    }

    /// The C++ object the game holds.
    pub fn object(&self) -> *mut c_void {
        self.object as *mut c_void
    }

    /// The object's vtable.
    pub fn vtable(&self) -> &'static [usize; Self::SLOTS] {
        self.vtable
    }

    /// The source the listener is registered with; null when none.
    pub fn attached(&self) -> *mut c_void {
        self.hub.attached()
    }

    pub fn is_attached(&self) -> bool {
        !self.hub.attached().is_null()
    }

    /// Registers with `source`, after leaving the source registered before. Game thread.
    ///
    /// # Safety
    /// `source` must be null or a live `BlockSource` whose vtable holds the listener slots this listener was made
    /// with.
    #[allow(unsafe_code)]
    pub unsafe fn attach(&self, source: *mut c_void) {
        self.detach();
        // SAFETY: the caller passes a live BlockSource of the game, or null
        let Some(source) = (unsafe { ListenerSource::from_raw(source, self.slots) }) else { return };
        if source.add_listener(self.object()) {
            self.hub.set_attached(source.as_ptr());
        }
    }

    /// Leaves the registered source; nothing when the game destroyed it first. Game thread.
    #[allow(unsafe_code)]
    pub fn detach(&self) {
        let source = self.hub.take_attached();
        // SAFETY: a source the hub still holds has not reported onSourceDestroyed, so it is live
        if let Some(source) = unsafe { ListenerSource::from_raw(source, self.slots) }
            && !source.remove_listener(self.object())
        {
            self.stranded.set(true);
        }
    }

    /// Frees the object and its table. The listener comes back unfreed while a source holds it: it is still
    /// registered, or a source could not be asked to remove it.
    ///
    /// # Safety
    /// `heap` must be the heap `new` allocated the object with, and the call must not race a game call of the
    /// thunks (game thread, or the game no longer updates the source).
    #[allow(unsafe_code)]
    pub unsafe fn release(self, heap: &MsvcHeap) -> Result<(), Self> {
        if self.is_attached() || self.stranded.get() {
            return Err(self);
        }
        let object = self.object as *mut ListenerObject;
        // SAFETY: the object came from `allocate_in` with this heap, and no source holds it any more
        unsafe { ListenerObject::destroy(object, 1, heap) };
        // SAFETY: the table was leaked from a Box in `new`, and the freed object was its only user
        drop(unsafe { Box::from_raw((self.vtable as *const [usize; Self::SLOTS]).cast_mut()) });
        Ok(())
    }

    #[allow(unsafe_code)]
    fn hub_of(this: *mut c_void) -> &'static BlockChangeHub {
        // SAFETY: the game calls the thunks with the object `new` allocated, whose state is the hub reference
        unsafe { *ListenerObject::state(this.cast()) }
    }

    // slot 0: the game never owns the object, so the destructor frees nothing
    extern "C" fn destructor(this: *mut c_void, _flags: u32) -> *mut c_void {
        this
    }

    // the source is forgotten whatever the stop latch says: it costs no game call, and a later detach must not reach
    // a freed source
    extern "C" fn source_destroyed_thunk(this: *mut c_void, source: *mut c_void) {
        let hub = Self::hub_of(this);
        let _ = catch_unwind(AssertUnwindSafe(|| hub.source_destroyed(source)));
    }

    #[allow(clippy::too_many_arguments, unsafe_code)]
    extern "C" fn block_changed_thunk(
        this: *mut c_void,
        source: *mut c_void,
        pos: *const BlockPos,
        layer: u32,
        block: *const c_void,
        _old_block: *const c_void,
        _flags: i32,
        _sync: *const c_void,
        _target: u8,
        _actor: *mut c_void,
    ) {
        if pos.is_null() {
            return;
        }
        let hub = Self::hub_of(this);
        // SAFETY: the game passes a BlockPos reference, checked non-null above, live for the call
        let pos = unsafe { pos.read_unaligned() };
        let change = BlockChange { source, pos, layer, block };
        if let Err(payload) = catch_unwind(AssertUnwindSafe(|| hub.block_changed(&change)))
            && let Some(runtime) = ModRuntime::get()
        {
            runtime.logger().error(&format!("block listener panicked: {}", PanicText::of(payload.as_ref())));
        }
    }
}
