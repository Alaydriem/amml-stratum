use amml_game::StopLatch;
use amml_sdk::DetourGuard;

use crate::detours::HookBody;
use crate::services::StratumRuntime;

// ScreenController::~ScreenController(), the body (not the deleting destructor in slot 0): every screen controller's
// destructor reaches it, the HUD's included. The noted HUD controller is forgotten before it is freed
pub static GUARD: DetourGuard = DetourGuard::new("screen_gone");

// this
pub type ScreenGoneFn = unsafe extern "C" fn(usize);

// the latch the body runs under before load installed the runtime; never raised
static IDLE: StopLatch = StopLatch::new();

#[allow(unsafe_code)]
pub extern "C" fn detour(this: usize) {
    let original: ScreenGoneFn = GUARD.original_fn();
    let runtime = StratumRuntime::get();
    HookBody::before(
        // SAFETY: `original` is the next function in the chain, with the hooked signature
        || unsafe { original(this) },
        runtime.map_or(&IDLE, |runtime| runtime.latch()),
        || {
            if let Some(runtime) = runtime {
                runtime.screen_gone(this);
            }
        },
    );
}
