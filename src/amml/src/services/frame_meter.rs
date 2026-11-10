use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

// Telemetry for the render thread's library frame, read by the game thread once a window. The render
// thread adds to atomics and takes no lock; the game thread reads them and starts the next window.
// The switch is set from the settings, so a frame costs no clock read while telemetry is off.
#[derive(Debug, Default)]
pub struct FrameMeter {
    enabled: AtomicBool,
    count: AtomicU64,
    sum_us: AtomicU64,
    max_us: AtomicU64,
}

impl FrameMeter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    // Render thread: one drawn frame that took `micros`.
    pub fn record(&self, micros: u64) {
        self.count.fetch_add(1, Ordering::Relaxed);
        self.sum_us.fetch_add(micros, Ordering::Relaxed);
        self.max_us.fetch_max(micros, Ordering::Relaxed);
    }

    // Game thread: the frames drawn since the last call as (count, total micros, longest micros), and
    // a fresh window.
    pub fn take(&self) -> (u64, u64, u64) {
        (
            self.count.swap(0, Ordering::Relaxed),
            self.sum_us.swap(0, Ordering::Relaxed),
            self.max_us.swap(0, Ordering::Relaxed),
        )
    }
}
