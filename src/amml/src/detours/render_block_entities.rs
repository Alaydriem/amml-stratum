use std::panic::{AssertUnwindSafe, catch_unwind};

use amml_game::StopLatch;
use amml_sdk::{DetourGuard, ModRuntime, PanicText};

use crate::services::StratumRuntime;

// void LevelRendererCamera::renderBlockEntities(BaseActorRenderContext&, bool renderAlphaLayer): the body every camera
// shares. The original runs first; the ghost's block-entity cells are then drawn in the same pass
pub static GUARD: DetourGuard = DetourGuard::new("render_block_entities");

// the signature of the hooked function: this, the BaseActorRenderContext and the alpha flag in r8b
pub type RenderBlockEntitiesFn = unsafe extern "C" fn(usize, usize, u8);

// the latch the body runs under before load installed the runtime; never raised
static IDLE: StopLatch = StopLatch::new();

// the body of the detour, kept apart from the hook so a test can drive it
pub struct RenderBlockEntities;

impl RenderBlockEntities {
    /// Calls `original`, then `body` on the opaque pass (`alpha` false) unless `latch` is raised or the mod is
    /// disabled. A panic in `body` raises `latch` and is logged.
    pub fn run(original: impl FnOnce(), latch: &StopLatch, alpha: bool, body: impl FnOnce()) {
        original();
        if alpha || latch.raised() || ModRuntime::is_disabled() {
            return;
        }
        if let Err(payload) = catch_unwind(AssertUnwindSafe(body)) {
            latch.raise();
            if let Some(runtime) = ModRuntime::get() {
                runtime.logger().error(&format!(
                    "the block entity render body panicked: {}; stratum stops its render work",
                    PanicText::of(payload.as_ref())
                ));
            }
        }
    }
}

#[allow(unsafe_code)]
pub extern "C" fn detour(this: usize, ctx: usize, alpha: u8) {
    let original: RenderBlockEntitiesFn = GUARD.original_fn();
    let runtime = StratumRuntime::get();
    RenderBlockEntities::run(
        // SAFETY: `original` is the next function in the chain, with the hooked signature; the arguments are the
        // game's
        || unsafe { original(this, ctx, alpha) },
        runtime.map_or(&IDLE, |runtime| runtime.latch()),
        alpha != 0,
        || {
            if let Some(runtime) = runtime {
                runtime.render_block_entities(this, ctx);
            }
        },
    );
}
