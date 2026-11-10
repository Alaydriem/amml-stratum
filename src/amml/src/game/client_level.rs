use std::ffi::c_void;
use std::ptr::NonNull;

use amml_game::{GameEntries, RawObject};
use amml_msvc::MemberCall;

use crate::errors::GameLayoutError;
use crate::game::{LevelDataLayout2651, LevelDataLayout2660, LevelDataLayoutKind, LevelSeed64};
use crate::types::StratumEntry;

/// A `ClientLevel*` of the game, for the level name and seed the world key reads.
pub struct ClientLevel<'a> {
    ptr: NonNull<c_void>,
    entries: &'a GameEntries,
}

impl<'a> ClientLevel<'a> {
    /// `None` for a null pointer.
    ///
    /// # Safety
    /// `ptr` must be null or a live `ClientLevel` of the game build `entries` was resolved for, and must stay live
    /// while the wrapper is used.
    #[allow(unsafe_code)]
    pub unsafe fn from_raw(ptr: *mut c_void, entries: &'a GameEntries) -> Option<Self> {
        NonNull::new(ptr).map(|ptr| Self { ptr, entries })
    }

    /// The address of the level, which names the loaded world for one stay; it is compared, never followed.
    pub fn address(&self) -> usize {
        self.ptr.as_ptr() as usize
    }

    /// An error when `LevelData.mLevelName`, the anchor that selects the `LevelData` mirror, is missing or holds an
    /// offset that no mirror was written with.
    pub fn check(entries: &GameEntries) -> Result<(), GameLayoutError> {
        let entry = StratumEntry::LEVEL_DATA_LEVEL_NAME;
        match entries.get(entry) {
            None => Err(GameLayoutError::Missing { entry: entry.as_str() }),
            Some(found) if LevelDataLayoutKind::of(entries).is_none() => {
                Err(GameLayoutError::Misfit { entry: entry.as_str(), expected: LevelDataLayout2651::LEVEL_NAME, found })
            }
            Some(_) => Ok(()),
        }
    }

    /// The level name of the level data, with bytes that are not UTF-8 replaced. `None` when an entry is off, the
    /// mirror does not fit or the game returns null. Game thread.
    #[allow(unsafe_code)]
    pub fn level_name(&self) -> Option<String> {
        let kind = LevelDataLayoutKind::of(self.entries)?;
        let slot = self.entries.get(StratumEntry::CLIENT_LEVEL_GET_LEVEL_DATA)?;
        // SAFETY: the wrapper holds a live ClientLevel, whose first qword is its Level vtable; the entry is the slot
        let function = unsafe { RawObject::slot(self.ptr.as_ptr(), slot) }?;
        // SAFETY: the slot holds `LevelData& Level::getLevelData()`, a reference returned as a pointer
        let data = unsafe { RawObject::call_pointer(function, self.ptr.as_ptr()) }?;
        if !data.cast::<LevelDataLayout2651>().is_aligned() {
            return None;
        }
        // SAFETY: the game returns its live LevelData, whose head the mirror of the build's kind mirrors
        let name = unsafe {
            match kind {
                LevelDataLayoutKind::Build2651 => (*data.cast::<LevelDataLayout2651>()).level_name.as_bytes(),
                LevelDataLayoutKind::Build2660 => (*data.cast::<LevelDataLayout2660>()).level_name.as_bytes(),
            }
        };
        Some(String::from_utf8_lossy(name).into_owned())
    }

    /// The 64-bit level seed. `None` when the entry is off or the slot is null. Game thread.
    #[allow(unsafe_code)]
    pub fn seed(&self) -> Option<u64> {
        let slot = self.entries.get(StratumEntry::LEVEL_GET_LEVEL_SEED64)?;
        // SAFETY: the wrapper holds a live ClientLevel, whose first qword is its Level vtable; the entry is the slot
        let function = unsafe { RawObject::slot(self.ptr.as_ptr(), slot) }?;
        // SAFETY: the slot holds `LevelSeed64 ILevel::getLevelSeed64() const`, a member that returns its 8-byte
        // class through a hidden pointer, which MemberCall passes
        let seed: LevelSeed64 = unsafe { MemberCall::call_sret0(function, self.ptr.as_ptr()) };
        Some(seed.value)
    }
}
