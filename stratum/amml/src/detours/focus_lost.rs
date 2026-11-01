use std::panic::{AssertUnwindSafe, catch_unwind};

use amml_game::StopLatch;
use amml_sdk::{DetourGuard, ModRuntime, PanicText};

use crate::services::StratumRuntime;

// void MinecraftGame::onAppFocusLost(): the game opens its pause screen from this handler when the window loses focus.
// `this` is the AppPlatformListener subobject, passed on unchanged. While the library holds the file picker open, the
// handler is skipped, so the focus the picker takes does not open the pause screen
pub static GUARD: DetourGuard = DetourGuard::new("focus_lost");

pub type FocusLostFn = unsafe extern "C" fn(usize);

// the latch the body runs under before load installed the runtime; never raised
static IDLE: StopLatch = StopLatch::new();

// the body of the detour, kept apart from the hook so a test can drive it
pub struct FocusLost;

impl FocusLost {
    /// Calls `original` with `this` unless `unwanted` says the pause screen is not wanted. `unwanted` is not asked
    /// while `latch` is raised or the mod is disabled. A panic in `unwanted` raises `latch`, is logged, and the
    /// original runs.
    pub fn run(original: impl FnOnce(usize), this: usize, latch: &StopLatch, unwanted: impl FnOnce() -> bool) {
        if latch.raised() || ModRuntime::is_disabled() {
            original(this);
            return;
        }
        match catch_unwind(AssertUnwindSafe(unwanted)) {
            Ok(true) => {}
            Ok(false) => original(this),
            Err(payload) => {
                latch.raise();
                if let Some(runtime) = ModRuntime::get() {
                    runtime.logger().error(&format!(
                        "focus loss body panicked: {}; the mod stops its per-tick work",
                        PanicText::of(payload.as_ref())
                    ));
                }
                original(this);
            }
        }
    }
}

#[allow(unsafe_code)]
pub extern "C" fn detour(this: usize) {
    let original: FocusLostFn = GUARD.original_fn();
    let runtime = StratumRuntime::get();
    FocusLost::run(
        // SAFETY: `original` is the next function in the chain, with the hooked signature
        |this| unsafe { original(this) },
        this,
        runtime.map_or(&IDLE, |runtime| runtime.latch()),
        || runtime.is_some_and(|runtime| runtime.pause_screen_unwanted()),
    );
}
