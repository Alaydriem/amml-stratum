use std::sync::atomic::{AtomicBool, Ordering};

/// The easy place state that listeners read without a lock: the `BuildBlock` listener also runs on the local world's
/// server thread and inside the mod's own `build_at`, and the input listeners run on the window thread. The use
/// driver writes `holding` and `blocking` on the game thread; the input listeners write `use_key`.
pub struct UseFlags {
    // the use key is held and a use call ran in this hold
    holding: AtomicBool,
    // the hold's last command was not to leave the use to the game
    blocking: AtomicBool,
    // the real right mouse button is down, as the last real mouse event left it; a focus loss and a world leave
    // count as an up
    use_key: AtomicBool,
}

impl UseFlags {
    pub const fn new() -> Self {
        Self { holding: AtomicBool::new(false), blocking: AtomicBool::new(false), use_key: AtomicBool::new(false) }
    }

    pub fn holding(&self) -> bool {
        self.holding.load(Ordering::SeqCst)
    }

    pub fn set_holding(&self, holding: bool) {
        self.holding.store(holding, Ordering::SeqCst);
    }

    pub fn blocking(&self) -> bool {
        self.blocking.load(Ordering::SeqCst)
    }

    pub fn set_blocking(&self, blocking: bool) {
        self.blocking.store(blocking, Ordering::SeqCst);
    }

    pub fn use_key(&self) -> bool {
        self.use_key.load(Ordering::SeqCst)
    }

    pub fn set_use_key(&self, down: bool) {
        self.use_key.store(down, Ordering::SeqCst);
    }

    /// Whether the game's own build of the local player is skipped: the hold's last decision was not to leave the use
    /// to the game. The mod's own builds and every other player's build go through.
    pub fn blocks_game_build(&self, is_local_player: bool, own: bool) -> bool {
        is_local_player && !own && self.holding() && self.blocking()
    }
}

impl Default for UseFlags {
    fn default() -> Self {
        Self::new()
    }
}
