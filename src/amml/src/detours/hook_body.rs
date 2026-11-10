use std::panic::{AssertUnwindSafe, catch_unwind};

use amml_game::StopLatch;
use amml_sdk::{ModRuntime, PanicText};

/// The guard of every detour whose body runs before the original (the network handlers and the easy place hooks),
/// kept apart from the hooks so a test can drive it. A body is not asked while `latch` is raised or the mod is
/// disabled; a panic in a body raises `latch`, is logged, and the original runs.
pub struct HookBody;

impl HookBody {
    /// Asks `skip` first, then calls `original` unless `skip` said the game's call is skipped, in which case `skipped`
    /// is the result.
    pub fn skip_or<R>(original: impl FnOnce() -> R, skipped: R, latch: &StopLatch, skip: impl FnOnce() -> bool) -> R {
        if latch.raised() || ModRuntime::is_disabled() {
            return original();
        }
        match catch_unwind(AssertUnwindSafe(skip)) {
            Ok(true) => skipped,
            Ok(false) => original(),
            Err(payload) => {
                Self::panicked(latch, payload.as_ref());
                original()
            }
        }
    }

    /// Runs `body`, then always calls `original`.
    pub fn before<R>(original: impl FnOnce() -> R, latch: &StopLatch, body: impl FnOnce()) -> R {
        if !latch.raised()
            && !ModRuntime::is_disabled()
            && let Err(payload) = catch_unwind(AssertUnwindSafe(body))
        {
            Self::panicked(latch, payload.as_ref());
        }
        original()
    }

    fn panicked(latch: &StopLatch, payload: &(dyn std::any::Any + Send)) {
        latch.raise();
        if let Some(runtime) = ModRuntime::get() {
            runtime
                .logger()
                .error(&format!("a hook body panicked: {}; the mod stops its per-tick work", PanicText::of(payload)));
        }
    }
}
