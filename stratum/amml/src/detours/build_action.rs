use amml_game::StopLatch;
use amml_sdk::DetourGuard;

use crate::detours::HookBody;
use crate::services::StratumRuntime;

// static bool ClientInputCallbacks::handleBuildAction(IClientInstance&, BuildActionIntention&, HitResult const& solid,
// HitResult const& liquid): the use press; held repeats do not come through here. A press whose use call is not left
// to the game is cancelled
pub static GUARD: DetourGuard = DetourGuard::new("build_action");

// the client, the intention, the two hit results; the bool in al
pub type BuildActionFn = unsafe extern "C" fn(usize, usize, usize, usize) -> u8;

// the latch the body runs under before load installed the runtime; never raised
static IDLE: StopLatch = StopLatch::new();

#[allow(unsafe_code)]
pub extern "C" fn detour(client: usize, intention: usize, solid: usize, liquid: usize) -> u8 {
    let original: BuildActionFn = GUARD.original_fn();
    let runtime = StratumRuntime::get();
    HookBody::skip_or(
        // SAFETY: `original` is the next function in the chain, with the hooked signature; the arguments are the
        // game's
        || unsafe { original(client, intention, solid, liquid) },
        1,
        runtime.map_or(&IDLE, |runtime| runtime.latch()),
        || runtime.is_some_and(|runtime| runtime.on_build_action(client)),
    )
}
