use std::ffi::c_void;
use std::path::Path;

use amml_game::DbStorageLayout;

/// A `DBStorage` of the game: the storage of a local world.
pub struct DbStorage;

impl DbStorage {
    /// The last path part of the world folder the storage was made for (`mFullPath`), with bytes that are not UTF-8
    /// replaced; `None` for a null or misaligned pointer or an empty path.
    ///
    /// # Safety
    /// `storage` must be null or a `DBStorage` of the running game whose constructor has returned, live for the call.
    #[allow(unsafe_code)]
    pub unsafe fn folder_name(storage: *const c_void) -> Option<String> {
        let layout = storage.cast::<DbStorageLayout>();
        if layout.is_null() || !layout.is_aligned() {
            return None;
        }
        // SAFETY: the caller guarantees a constructed DBStorage, whose head DbStorageLayout mirrors; the pointer is
        // aligned and not null
        let path = String::from_utf8_lossy(unsafe { (*layout).full_path.as_bytes() }).into_owned();
        let name = Path::new(&path).file_name()?.to_string_lossy().into_owned();
        (!name.is_empty()).then_some(name)
    }
}
