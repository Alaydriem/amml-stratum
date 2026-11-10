use std::panic::{AssertUnwindSafe, catch_unwind};

use amml_game::StopLatch;
use amml_sdk::{DetourGuard, ModRuntime, PanicText};

use crate::services::StratumRuntime;

// void LevelRenderer::renderLevel(ScreenContext&, FrameRenderObject const&): the world render. The original runs
// first; the ghost is then drawn after the game's own level pass, depth tested against it
pub static GUARD: DetourGuard = DetourGuard::new("render_level");

// the signature of the hooked function: this, the ScreenContext and the FrameRenderObject
pub type RenderLevelFn = unsafe extern "C" fn(usize, usize, usize);

// the latch the body runs under before load installed the runtime; never raised
static IDLE: StopLatch = StopLatch::new();

// the body of the detour, kept apart from the hook so a test can drive it
pub struct RenderLevel;

impl RenderLevel {
    /// Calls `original`, then `body` unless `latch` is raised or the mod is disabled. A disable that raises the
    /// latch while a body runs lets that body finish; no later call runs one. A panic in `body` raises `latch` and
    /// is logged.
    pub fn run(original: impl FnOnce(), latch: &StopLatch, body: impl FnOnce()) {
        original();
        if latch.raised() || ModRuntime::is_disabled() {
            return;
        }
        if let Err(payload) = catch_unwind(AssertUnwindSafe(body)) {
            latch.raise();
            if let Some(runtime) = ModRuntime::get() {
                runtime.logger().error(&format!(
                    "the render body panicked: {}; stratum stops its render work",
                    PanicText::of(payload.as_ref())
                ));
            }
        }
    }
}

#[allow(unsafe_code)]
pub extern "C" fn detour(this: usize, screen: usize, frame: usize) {
    let original: RenderLevelFn = GUARD.original_fn();
    let runtime = StratumRuntime::get();
    RenderLevel::run(
        // SAFETY: `original` is the next function in the chain, with the hooked signature; the arguments are the
        // game's
        || unsafe { original(this, screen, frame) },
        runtime.map_or(&IDLE, |runtime| runtime.latch()),
        || {
            if let Some(runtime) = runtime {
                runtime.after_render_level(this, screen, frame);
            }
        },
    );
}
