use super::GhostTickWindow;

// Times the core ghost work of each tick, one telemetry window at a time. Three counters, so a sample allocates
// nothing.
#[derive(Debug, Default)]
pub struct GhostTickMeter {
    sum_us: u64,
    max_us: u64,
    ticks: u64,
}

impl GhostTickMeter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe(&mut self, micros: u64) {
        self.sum_us = self.sum_us.saturating_add(micros);
        self.max_us = self.max_us.max(micros);
        self.ticks += 1;
    }

    // The window, and a fresh one.
    pub fn take(&mut self) -> GhostTickWindow {
        let window = GhostTickWindow { avg_us: self.sum_us.checked_div(self.ticks).unwrap_or(0), max_us: self.max_us };
        *self = Self::default();
        window
    }
}
