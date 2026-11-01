use std::ffi::c_void;

use amml_game::{BlockPos, RawObject};

use crate::game::{GameModeLayout, PlayerInventorySpanLayout};
use crate::types::{PlayerCalls, UseCalls};

// `float GameMode::getMaxPickRange()`: rcx the game mode, the range in xmm0
type MaxPickRangeFn = unsafe extern "C" fn(*mut c_void) -> f32;

// `bool GameMode::buildBlock(BlockPos const&, uchar face, HandSlot, bool isSimTick)`: this, the position, the face,
// the hand slot (one byte), and isSimTick as the fifth argument on the stack
type BuildBlockFn = unsafe extern "C" fn(*mut c_void, *const BlockPos, u8, u8, u8) -> u8;

/// A `GameMode` of the game: its player, its build delay and pick range, and its own block build.
pub struct GameMode {
    ptr: *mut c_void,
    calls: UseCalls,
}

impl GameMode {
    /// `HandSlot::Mainhand`.
    pub const MAIN_HAND: u8 = 0;

    /// The game mode of `player`; None for a null player or game mode.
    ///
    /// # Safety
    /// `player` must be null or a live `Player` of the build the calls were resolved for.
    #[allow(unsafe_code)]
    pub unsafe fn of_player(player: *mut c_void, players: &PlayerCalls, calls: &UseCalls) -> Option<Self> {
        if player.is_null() {
            return None;
        }
        let span =
            player.cast::<u8>().wrapping_add(players.player_inventory as usize).cast::<PlayerInventorySpanLayout>();
        // SAFETY: the caller guarantees a live player, which holds the span the mirror lays out at the anchor
        let ptr = unsafe { (&raw const (*span).game_mode).read_unaligned() };
        (!ptr.is_null()).then_some(Self { ptr, calls: *calls })
    }

    /// Wraps `ptr`; None for a null pointer.
    ///
    /// # Safety
    /// `ptr` must be null or a live `GameMode` of the build `calls` was resolved for.
    #[allow(unsafe_code)]
    pub unsafe fn from_raw(ptr: *mut c_void, calls: &UseCalls) -> Option<Self> {
        (!ptr.is_null()).then_some(Self { ptr, calls: *calls })
    }

    pub fn as_ptr(&self) -> *mut c_void {
        self.ptr
    }

    /// The player of this game mode (`mPlayer`), compared by address only.
    pub fn player(&self) -> usize {
        // SAFETY: the live game mode, which the mirror lays out up to buildingTickDelay; the anchor check in
        // `UseCalls::new` put mPlayer where the mirror has it
        #[allow(unsafe_code)]
        unsafe {
            (&raw const (*self.ptr.cast::<GameModeLayout>()).player).read_unaligned() as usize
        }
    }

    /// `buildingTickDelay`, in milliseconds.
    pub fn building_tick_delay_ms(&self) -> i64 {
        // SAFETY: as in `player`
        #[allow(unsafe_code)]
        unsafe {
            (&raw const (*self.ptr.cast::<GameModeLayout>()).building_tick_delay).read_unaligned()
        }
    }

    /// `getMaxPickRange()`; 0 when the entry is 0.
    pub fn max_pick_range(&self) -> f32 {
        if self.calls.get_max_pick_range == 0 {
            return 0.0;
        }
        // SAFETY: the entry is the address of getMaxPickRange
        #[allow(unsafe_code)]
        let call: MaxPickRangeFn = unsafe { std::mem::transmute(self.calls.get_max_pick_range) };
        // SAFETY: the live game mode
        #[allow(unsafe_code)]
        unsafe {
            call(self.ptr)
        }
    }

    /// The game mode's own `buildBlock`, through its vtable as the game calls it; false when the slot is empty.
    pub fn build_block(&self, pos: &BlockPos, face: u8, hand: u8, is_sim_tick: bool) -> bool {
        // SAFETY: the live game mode, whose vtable holds buildBlock at the entry's slot
        #[allow(unsafe_code)]
        let Some(function) = (unsafe { RawObject::slot(self.ptr, self.calls.build_block_slot) }) else {
            return false;
        };
        // SAFETY: the slot holds buildBlock with the signature of `BuildBlockFn`
        #[allow(unsafe_code)]
        let build: BuildBlockFn = unsafe { std::mem::transmute(function) };
        // SAFETY: the live game mode and a position that lives for the call
        #[allow(unsafe_code)]
        unsafe {
            build(self.ptr, pos, face, hand, u8::from(is_sim_tick)) != 0
        }
    }
}
