use amml_game::StopLatch;
use amml_game::hooks::{ClientUpdate, ClientUpdateFn};
use amml_sdk::DetourGuard;

use crate::services::StratumRuntime;

// bool ClientInstance::update(bool isInitFinished) (26.60: update(bool, bool)): once per frame on the game thread.
// The 26.60 third argument (r8) is carried as a raw u64, so one declaration works on both builds
pub static GUARD: DetourGuard = DetourGuard::new("client_update");

// the latch the body runs under before load installed the runtime; never raised
static IDLE: StopLatch = StopLatch::new();

#[allow(unsafe_code)]
pub extern "C" fn detour(this: usize, init_finished: u8, extra: u64) -> u8 {
    let original: ClientUpdateFn = GUARD.original_fn();
    let runtime = StratumRuntime::get();
    ClientUpdate::run(
        // SAFETY: `original` is the next function in the chain, with the hooked signature
        |this, init_finished, extra| unsafe { original(this, init_finished, extra) },
        this,
        init_finished,
        extra,
        runtime.map_or(&IDLE, |runtime| runtime.latch()),
        |client| {
            if let Some(runtime) = runtime {
                runtime.tick(client);
            }
        },
    )
}
