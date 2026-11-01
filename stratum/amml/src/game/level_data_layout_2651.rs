use std::mem::offset_of;

use amml_msvc::MsvcString;

/// Layout mirror of the start of the game's `LevelData` on 26.51, up to `mLevelName`, which the world key reads for
/// realm and name keys. A prefix: the mod only reads through the pointer that `Level::getLevelData` returns.
///
/// The `LevelData.mLevelName` entry selects the mirror: this one is for a build whose `mLevelName` is at
/// [`LevelDataLayout2651::LEVEL_NAME`].
#[repr(C)]
pub struct LevelDataLayout2651 {
    _head: [u8; 0x2A8],         // 0x000 mAdventureSettings .. mDefaultPermissions
    pub level_name: MsvcString, // 0x2A8 mLevelName
}

impl LevelDataLayout2651 {
    /// The offset of `mLevelName`, the value the `LevelData.mLevelName` entry has for this layout.
    pub const LEVEL_NAME: u64 = offset_of!(LevelDataLayout2651, level_name) as u64;
}

const _: () = assert!(offset_of!(LevelDataLayout2651, level_name) == 0x2A8);
