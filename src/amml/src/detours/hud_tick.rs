use amml_game::StopLatch;
use amml_sdk::DetourGuard;

use crate::detours::HookBody;
use crate::services::StratumRuntime;

// ui::DirtyFlag HudScreenController::tick(): the HUD screen's tick, slot 2 of its table; the controller the hotbar
// select goes through is noted first
pub static GUARD: DetourGuard = DetourGuard::new("hud_tick");

// this; the DirtyFlag (a uint enum) in eax
pub type HudTickFn = unsafe extern "C" fn(usize) -> u32;

// the latch the body runs under before load installed the runtime; never raised
static IDLE: StopLatch = StopLatch::new();

#[allow(unsafe_code)]
pub extern "C" fn detour(this: usize) -> u32 {
    let original: HudTickFn = GUARD.original_fn();
    let runtime = StratumRuntime::get();
    HookBody::before(
        // SAFETY: `original` is the next function in the chain, with the hooked signature
        || unsafe { original(this) },
        runtime.map_or(&IDLE, |runtime| runtime.latch()),
        || {
            if let Some(runtime) = runtime {
                runtime.hud_tick(this);
            }
        },
    )
}
