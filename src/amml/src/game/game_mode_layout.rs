use std::ffi::c_void;
use std::mem::offset_of;

/// Layout mirror of the head of the game's `GameMode`, up to and with `buildingTickDelay`. The `GameMode.mPlayer`
/// anchor names where `mPlayer` sits; the members after it follow the 26.51 header, and the 26.51 constructor stores
/// 250, 200 and 200 at 0xA0, 0xA8 and 0xB0 (the three delays). A prefix: use it only through a pointer to a game
/// `GameMode`.
#[repr(C)]
pub struct GameModeLayout {
    _vtable: *const c_void,       // 0x00
    pub player: *mut c_void,      // 0x08 mPlayer (Player&)
    _members: [u8; 0x98],         // 0x10 mDestroyBlockPos .. creativeDestructionTickDelay
    pub building_tick_delay: i64, // 0xA8 buildingTickDelay (std::chrono::milliseconds)
}

const _: () = assert!(offset_of!(GameModeLayout, player) == 0x08);
const _: () = assert!(offset_of!(GameModeLayout, building_tick_delay) == 0xA8);
const _: () = assert!(size_of::<GameModeLayout>() == 0xB0);
