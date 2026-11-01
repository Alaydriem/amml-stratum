use std::ffi::c_void;
use std::sync::Mutex;

use alaydriem_stratum::types::ListenerSlots;

/// A fake `BlockSource` for the block listener: its vtable holds `addListener` and `removeListener` at the slots of
/// `FakeSource::slots`, which record each call. Also the game's 9-slot listener table the listener copies.
pub struct FakeSource {
    object: Box<[usize; 1]>,
}

/// Each `addListener` as (source, listener), in call order.
pub static ADDED: Mutex<Vec<(usize, usize)>> = Mutex::new(Vec::new());
/// Each `removeListener` as (source, listener), in call order.
pub static REMOVED: Mutex<Vec<(usize, usize)>> = Mutex::new(Vec::new());

impl FakeSource {
    pub const SOURCE_DESTROYED: u64 = 2;
    pub const BLOCK_CHANGED: u64 = 4;

    pub fn new() -> Self {
        let mut table = [0usize; 8];
        table[2] = Self::add as *const () as usize;
        table[3] = Self::remove as *const () as usize;
        let table: &'static [usize; 8] = Box::leak(Box::new(table));
        Self { object: Box::new([table.as_ptr() as usize]) }
    }

    pub fn ptr(&mut self) -> *mut c_void {
        self.object.as_mut_ptr().cast()
    }

    pub fn slots() -> ListenerSlots {
        ListenerSlots {
            add_listener: 2,
            remove_listener: 3,
            source_destroyed: Self::SOURCE_DESTROYED,
            block_changed: Self::BLOCK_CHANGED,
        }
    }

    /// The game's listener table: 9 slots that do nothing.
    pub fn listener_base() -> u64 {
        let table: &'static [usize; 9] = Box::leak(Box::new([Self::nothing as *const () as usize; 9]));
        table.as_ptr() as u64
    }

    pub fn added() -> Vec<(usize, usize)> {
        ADDED.lock().unwrap().clone()
    }

    pub fn removed() -> Vec<(usize, usize)> {
        REMOVED.lock().unwrap().clone()
    }

    extern "C" fn add(this: *mut c_void, listener: *mut c_void) {
        ADDED.lock().unwrap().push((this as usize, listener as usize));
    }

    extern "C" fn remove(this: *mut c_void, listener: *mut c_void) {
        REMOVED.lock().unwrap().push((this as usize, listener as usize));
    }

    extern "C" fn nothing() {}
}
