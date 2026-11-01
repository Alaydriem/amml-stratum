use amml_game::StopLatch;
use amml_sdk::DetourGuard;

use crate::detours::HookBody;
use crate::services::StratumRuntime;

// bool GameMode::buildBlock(BlockPos const&, uchar face, HandSlot, bool isSimTick): the base body that slot 6 of the
// GameMode table holds, which every game mode's build reaches, the local player's held repeats and the local world's
// server side included. The local player's own builds are skipped while the hold's last use call was not left to the
// game
pub static GUARD: DetourGuard = DetourGuard::new("build_block");

// this, the position, the face, the hand slot, isSimTick (the fifth argument, on the stack); the bool in al
pub type BuildBlockFn = unsafe extern "C" fn(usize, usize, u8, u8, u8) -> u8;

// the latch the body runs under before load installed the runtime; never raised
static IDLE: StopLatch = StopLatch::new();

#[allow(unsafe_code)]
pub extern "C" fn detour(this: usize, pos: usize, face: u8, hand: u8, is_sim_tick: u8) -> u8 {
    let original: BuildBlockFn = GUARD.original_fn();
    let runtime = StratumRuntime::get();
    HookBody::skip_or(
        // SAFETY: `original` is the next function in the chain, with the hooked signature; the arguments are the
        // game's
        || unsafe { original(this, pos, face, hand, is_sim_tick) },
        0,
        runtime.map_or(&IDLE, |runtime| runtime.latch()),
        || runtime.is_some_and(|runtime| runtime.blocks_game_build(this, is_sim_tick != 0)),
    )
}
