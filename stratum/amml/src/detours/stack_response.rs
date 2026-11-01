use amml_game::StopLatch;
use amml_sdk::DetourGuard;

use crate::detours::HookBody;
use crate::services::StratumRuntime;

// void ClientNetworkHandler::handle(NetworkIdentifier const&, ItemStackResponsePacket const&): the server's answers
// to item stack requests, the body the live handler's vtable holds. The mod reads them first; the game always gets
// them
pub static GUARD: DetourGuard = DetourGuard::new("stack_response");

// this, the source, the packet
pub type HandleFn = unsafe extern "C" fn(usize, usize, usize);

// the latch the body runs under before load installed the runtime; never raised
static IDLE: StopLatch = StopLatch::new();

#[allow(unsafe_code)]
pub extern "C" fn detour(this: usize, source: usize, packet: usize) {
    let original: HandleFn = GUARD.original_fn();
    let runtime = StratumRuntime::get();
    HookBody::skip_or(
        // SAFETY: `original` is the next function in the chain, with the hooked signature; the arguments are the
        // game's
        || unsafe { original(this, source, packet) },
        (),
        runtime.map_or(&IDLE, |runtime| runtime.latch()),
        || {
            if let Some(runtime) = runtime {
                runtime.on_item_stack_response(packet);
            }
            false
        },
    );
}
