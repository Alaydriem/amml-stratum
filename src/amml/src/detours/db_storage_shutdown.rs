use amml_sdk::DetourGuard;

use crate::services::WorldFolder;

// bool DBStorage::checkShutdownDone(): true once the storage has shut down and is about to be destroyed. The folder
// that storage noted is forgotten whatever the stop latch says: forgetting it never reaches the game, and a stale
// folder would name a later world that has no storage of its own
pub static GUARD: DetourGuard = DetourGuard::new("db_storage_shutdown");

pub type CheckShutdownDoneFn = unsafe extern "C" fn(usize) -> u8;

// the body of the detour, kept apart from the hook so a test can drive it
pub struct StorageShutdown;

impl StorageShutdown {
    /// After the original returned `done` for the storage at `storage`: a storage that is done forgets the folder it
    /// noted in `folder`; the folder of another storage stays.
    pub fn run(done: bool, storage: usize, folder: &WorldFolder) {
        if done {
            folder.shut_down(storage);
        }
    }
}

#[allow(unsafe_code)]
pub extern "C" fn detour(this: usize) -> u8 {
    let original: CheckShutdownDoneFn = GUARD.original_fn();
    // SAFETY: `original` is the next function in the chain, with the hooked signature
    let done = unsafe { original(this) };
    GUARD.run(|| StorageShutdown::run(done != 0, this, WorldFolder::global()), || ());
    done
}
