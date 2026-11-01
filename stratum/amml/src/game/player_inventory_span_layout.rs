use std::ffi::c_void;
use std::mem::offset_of;

/// Layout mirror of the span of the game's `Player` from `mInventory` to `mGameMode`, placed by the
/// `Player.playerInventory` anchor (0x5B8 on 26.51, 0x5E0 on 26.60). The members between the two follow the 26.51
/// header sum, which puts `mGameMode` at 0xAA0, as the exe reads it before `getMaxPickRange`; on 26.60 the exe reads
/// it at 0xAC8, the same span from the anchor. A view of part of a game `Player`: use it
/// only through a pointer at the anchor.
#[repr(C)]
pub struct PlayerInventorySpanLayout {
    pub inventory: *mut c_void, // +0x000 mInventory (std::unique_ptr<PlayerInventory>)
    _members: [u8; 0x4E0],      // +0x008 mInventoryOptions .. mTransactionManager
    pub game_mode: *mut c_void, // +0x4E8 mGameMode (std::unique_ptr<GameMode>)
}

const _: () = assert!(offset_of!(PlayerInventorySpanLayout, inventory) == 0x000);
const _: () = assert!(offset_of!(PlayerInventorySpanLayout, game_mode) == 0x4E8);
const _: () = assert!(size_of::<PlayerInventorySpanLayout>() == 0x4F0);
