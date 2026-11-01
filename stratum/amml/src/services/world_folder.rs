use std::sync::{Mutex, PoisonError};

// The last path part of the folder of the local world now open, as the world storage detour saw it, with the address
// of that storage: set when a storage is constructed, cleared when that storage shuts down or when the mod is
// disabled. A tick out of the world keeps it, so the same world names the same folder when it is active again
pub struct WorldFolder {
    folder: Mutex<Option<(usize, String)>>,
}

static WORLD_FOLDER: WorldFolder = WorldFolder::new();

impl WorldFolder {
    pub const fn new() -> Self {
        Self { folder: Mutex::new(None) }
    }

    // the one the detours write
    pub fn global() -> &'static WorldFolder {
        &WORLD_FOLDER
    }

    // the storage at `storage` was made for `folder`; None forgets the folder
    pub fn set(&self, storage: usize, folder: Option<String>) {
        *self.folder.lock().unwrap_or_else(PoisonError::into_inner) = folder.map(|folder| (storage, folder));
    }

    // the storage at `storage` shut down: its folder is forgotten, a folder of another storage stays
    pub fn shut_down(&self, storage: usize) {
        let mut folder = self.folder.lock().unwrap_or_else(PoisonError::into_inner);
        if folder.as_ref().is_some_and(|(owner, _)| *owner == storage) {
            *folder = None;
        }
    }

    pub fn clear(&self) {
        *self.folder.lock().unwrap_or_else(PoisonError::into_inner) = None;
    }

    pub fn current(&self) -> Option<String> {
        self.folder.lock().unwrap_or_else(PoisonError::into_inner).as_ref().map(|(_, folder)| folder.clone())
    }
}

impl Default for WorldFolder {
    fn default() -> Self {
        Self::new()
    }
}
