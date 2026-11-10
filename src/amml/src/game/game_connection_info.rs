use std::ffi::c_void;
use std::ptr::NonNull;

use amml_game::game::GameConnectionInfoLayout;
use amml_game::{GameEntries, RawObject};
use amml_msvc::{MsvcHeap, MsvcString};

use crate::types::{ConnectionInfo, StratumEntry};

/// The connection info of a `ClientInstance*`: `std::optional<GameConnectionInfo> getGameConnectionInfo() const`.
pub struct GameConnectionInfo<'a> {
    client: NonNull<c_void>,
    entries: &'a GameEntries,
}

impl<'a> GameConnectionInfo<'a> {
    /// The largest GameConnectionInfo the copy is made for; a larger size entry is refused.
    pub const MAX_SIZE: usize = 4096;

    /// `None` for a null pointer.
    ///
    /// # Safety
    /// `client` must be null or a live `ClientInstance` of the game build `entries` was resolved for, and must stay
    /// live while the wrapper is used.
    #[allow(unsafe_code)]
    pub unsafe fn from_raw(client: *mut c_void, entries: &'a GameEntries) -> Option<Self> {
        NonNull::new(client).map(|client| Self { client, entries })
    }

    /// `sizeof(std::optional<GameConnectionInfo>)` for a value of `size` bytes: the value, then the engaged flag at
    /// `size`, rounded up to the value's alignment (8: it holds std::string members).
    pub fn optional_size(size: usize) -> usize {
        (size + 1).next_multiple_of(align_of::<MsvcString>())
    }

    /// The connection type, host, unresolved URL and port the game holds. `None` when an entry is off, the layout
    /// does not fit, the heap gives no block, or the optional is empty. The game constructs the copy in a block of
    /// the game heap; the copy is destroyed once with `GameConnectionInfo.dtor` and the block is freed once after
    /// it. Game thread.
    #[allow(unsafe_code)]
    pub fn read(&self, heap: &MsvcHeap) -> Option<ConnectionInfo> {
        let slot = self.entries.get(StratumEntry::CLIENT_INSTANCE_GET_GAME_CONNECTION_INFO)?;
        let size = usize::try_from(self.entries.get(StratumEntry::GAME_CONNECTION_INFO_SIZE)?).ok()?;
        let destructor = usize::try_from(self.entries.get(StratumEntry::GAME_CONNECTION_INFO_DTOR)?).ok()?;
        if destructor == 0 || size < size_of::<GameConnectionInfoLayout>() || size >= Self::MAX_SIZE {
            return None;
        }
        // SAFETY: the wrapper holds a live ClientInstance, whose first qword is its vtable; the entry is the slot
        let function = unsafe { RawObject::slot(self.client.as_ptr(), slot) }?;
        // SAFETY: the slot holds the getter, a member that returns the optional through a hidden pointer (this in
        // rcx, the buffer in rdx) and gives the buffer back in rax
        let call: unsafe extern "C" fn(*mut c_void, *mut u8) -> *mut u8 = unsafe { std::mem::transmute(function) };
        // SAFETY: the entry is `GameConnectionInfo::~GameConnectionInfo()`, a non-deleting destructor with this in
        // rcx; it does not free the block
        let destroy: unsafe extern "C" fn(*mut u8) = unsafe { std::mem::transmute(destructor) };
        let bytes = Self::optional_size(size);
        // SAFETY: `heap` is the game heap; a block below 4096 bytes is operator new's, aligned to 16
        let buffer = unsafe { heap.allocate(bytes) };
        if buffer.is_null() {
            return None;
        }
        // SAFETY: the block holds `bytes` writable bytes; zeroing leaves the optional empty until the game fills it
        unsafe { std::ptr::write_bytes(buffer, 0, bytes) };
        // SAFETY: the block holds the optional's value and its engaged flag, aligned to 16
        unsafe { call(self.client.as_ptr(), buffer) };
        // SAFETY: the flag byte is inside the block
        let engaged = unsafe { *buffer.add(size) } != 0;
        let info = engaged.then(|| {
            // SAFETY: the optional is engaged, so the game constructed a GameConnectionInfo at the start of the
            // block, whose head GameConnectionInfoLayout mirrors; the block is aligned and holds at least the head
            let layout = unsafe { &*buffer.cast::<GameConnectionInfoLayout>() };
            let info = ConnectionInfo {
                connection_type: layout.connection_type,
                host: String::from_utf8_lossy(layout.host.as_bytes()).into_owned(),
                url: String::from_utf8_lossy(layout.url.as_bytes()).into_owned(),
                port: layout.port,
            };
            // SAFETY: runs once on the value the game constructed; nothing reads the value after it
            unsafe { destroy(buffer) };
            info
        });
        // SAFETY: the block came from `heap.allocate(bytes)` and nothing reads it again
        unsafe { heap.deallocate(buffer, bytes) };
        info
    }
}
