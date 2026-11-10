use amml_game::StopLatch;
use amml_sdk::DetourGuard;

use crate::detours::HookBody;
use crate::services::StratumRuntime;

// void LegacyClientNetworkHandler::handle(NetworkIdentifier const&, ContainerClosePacket const&): the body the live
// handler's vtable holds at the ContainerClose slot. The server's answer to the mod's close is hidden from the game
pub static GUARD: DetourGuard = DetourGuard::new("container_close");

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
        || runtime.is_some_and(|runtime| runtime.on_container_close(packet)),
    );
}
