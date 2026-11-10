use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

/// The easy place state that the hooks read without the use lock, each value an atomic: the build hook also runs on
/// the local world's server thread and inside the executor's own build, and the HUD hooks run while a use call may
/// hold the lock. The use driver and the executor write it on the game thread.
pub struct UseSignals {
    // the local player of the last use call, compared by address only
    player: AtomicUsize,
    // the use key is held and a use call ran in this hold
    holding: AtomicBool,
    // the hold's last command was not to leave the use to the game
    blocking: AtomicBool,
    // the executor's own GameMode::buildBlock call runs
    own_build: AtomicBool,
    // the isSimTick of the game's own last buildBlock call
    sim_tick: AtomicBool,
    // the HUD screen controller of the last HUD tick, and the time of that tick (f64 bits)
    hud: AtomicUsize,
    hud_seen: AtomicU64,
    // the no-HUD warning was logged since the last HUD tick
    no_hud_logged: AtomicBool,
}

impl UseSignals {
    pub const fn new() -> Self {
        Self {
            player: AtomicUsize::new(0),
            holding: AtomicBool::new(false),
            blocking: AtomicBool::new(false),
            own_build: AtomicBool::new(false),
            sim_tick: AtomicBool::new(false),
            hud: AtomicUsize::new(0),
            hud_seen: AtomicU64::new(0),
            no_hud_logged: AtomicBool::new(false),
        }
    }

    pub fn player(&self) -> usize {
        self.player.load(Ordering::Relaxed)
    }

    pub fn set_player(&self, player: usize) {
        self.player.store(player, Ordering::Relaxed);
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

    pub fn own_build(&self) -> bool {
        self.own_build.load(Ordering::SeqCst)
    }

    pub fn set_own_build(&self, own: bool) {
        self.own_build.store(own, Ordering::SeqCst);
    }

    pub fn sim_tick(&self) -> bool {
        self.sim_tick.load(Ordering::SeqCst)
    }

    pub fn set_sim_tick(&self, sim_tick: bool) {
        self.sim_tick.store(sim_tick, Ordering::SeqCst);
    }

    /// The HUD controller and the time of its last tick; 0 when none is held.
    pub fn hud(&self) -> (usize, f64) {
        (self.hud.load(Ordering::SeqCst), f64::from_bits(self.hud_seen.load(Ordering::SeqCst)))
    }

    pub fn set_hud(&self, hud: usize, seen: f64) {
        self.hud_seen.store(seen.to_bits(), Ordering::SeqCst);
        self.hud.store(hud, Ordering::SeqCst);
    }

    /// Forgets the HUD when it is `controller`; true when it was.
    pub fn forget_hud(&self, controller: usize) -> bool {
        controller != 0 && self.hud.compare_exchange(controller, 0, Ordering::SeqCst, Ordering::SeqCst).is_ok()
    }

    /// Whether the no-HUD warning was logged; sets it.
    pub fn take_no_hud_logged(&self) -> bool {
        self.no_hud_logged.swap(true, Ordering::SeqCst)
    }

    pub fn clear_no_hud_logged(&self) {
        self.no_hud_logged.store(false, Ordering::SeqCst);
    }
}

impl Default for UseSignals {
    fn default() -> Self {
        Self::new()
    }
}
