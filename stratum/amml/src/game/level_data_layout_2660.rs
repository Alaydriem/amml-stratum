use std::mem::offset_of;

use amml_msvc::MsvcString;

/// Layout mirror of the start of the game's `LevelData` on 26.60, up to `mLevelName`: 8 bytes more before it than on
/// 26.51 (the 26.60 LevelData tag reader assigns the `LevelName` string to this+2B0h). A prefix, as the 26.51 mirror.
///
/// The `LevelData.mLevelName` entry selects the mirror: this one is for a build whose `mLevelName` is at
/// [`LevelDataLayout2660::LEVEL_NAME`].
#[repr(C)]
pub struct LevelDataLayout2660 {
    _head: [u8; 0x2B0],         // 0x000 mAdventureSettings .. mDefaultPermissions (8h more than 26.51)
    pub level_name: MsvcString, // 0x2B0 mLevelName
}

impl LevelDataLayout2660 {
    /// The offset of `mLevelName`, the value the `LevelData.mLevelName` entry has for this layout.
    pub const LEVEL_NAME: u64 = offset_of!(LevelDataLayout2660, level_name) as u64;
}

const _: () = assert!(offset_of!(LevelDataLayout2660, level_name) == 0x2B0);
