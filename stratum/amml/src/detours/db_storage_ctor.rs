use amml_sdk::DetourGuard;

use crate::game::DbStorage;
use crate::services::{StratumRuntime, WorldFolder};

// DBStorage::DBStorage(DBStorageConfig, Bedrock::NotNullNonOwnerPtr<LevelDbEnv>): both arguments are passed by
// hidden pointer, so the detour takes (this, config, env) and returns the original's rax, which is `this`. After the
// constructor returns, the folder of the world the storage was made for is noted. While the stop latch is raised
// nothing is noted
pub static GUARD: DetourGuard = DetourGuard::new("db_storage_ctor");

pub type DbStorageCtorFn = unsafe extern "C" fn(usize, usize, usize) -> usize;

#[allow(unsafe_code)]
pub extern "C" fn detour(this: usize, config: usize, env: usize) -> usize {
    let original: DbStorageCtorFn = GUARD.original_fn();
    // SAFETY: `original` is the next function in the chain, with the hooked signature
    let result = unsafe { original(this, config, env) };
    let stopped = StratumRuntime::get().is_none_or(|runtime| runtime.latch().raised());
    if !stopped {
        GUARD.run(
            || {
                // SAFETY: `this` is the storage whose constructor just returned
                let folder = unsafe { DbStorage::folder_name(this as *const std::ffi::c_void) };
                WorldFolder::global().set(this, folder);
            },
            || (),
        );
    }
    result
}
