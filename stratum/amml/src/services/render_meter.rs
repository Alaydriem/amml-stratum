use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::time::Instant;

use super::RenderWindow;

// Telemetry for the game's rendered frames: the time between two ghost takes, which the renderer makes once
// per rendered frame in a world, with or without a ghost to draw. The render thread adds to atomics and takes no
// lock; the game thread reads them once a window. A fixed histogram of `BUCKETS` buckets of `BUCKET_US` gives the
// p99, so a frame allocates nothing. The switch follows the telemetry setting, so a frame costs no clock read
// while telemetry is off.
//
// A gap longer than `GAP_US` is not a frame: the renderer takes nothing outside a world, so the first take after a menu
// or a world load would count the whole time away.
pub struct RenderMeter {
    enabled: AtomicBool,
    base: Instant,
    // Microseconds from `base` to the last take; `NONE` when no take started the next interval.
    last_us: AtomicU64,
    frames: AtomicU64,
    sum_us: AtomicU64,
    max_us: AtomicU64,
    buckets: [AtomicU32; Self::BUCKETS],
}

impl RenderMeter {
    pub const BUCKET_US: u64 = 100;
    // 0 to 50 ms; the last bucket also holds every longer frame.
    pub const BUCKETS: usize = 500;
    pub const GAP_US: u64 = 1_000_000;
    const NONE: u64 = u64::MAX;

    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            base: Instant::now(),
            last_us: AtomicU64::new(Self::NONE),
            frames: AtomicU64::new(0),
            sum_us: AtomicU64::new(0),
            max_us: AtomicU64::new(0),
            buckets: std::array::from_fn(|_| AtomicU32::new(0)),
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    // Render thread: a take now. The clock is read only while the meter is on.
    pub fn frame(&self) {
        if self.enabled() {
            self.take_at(Instant::now());
        } else {
            self.last_us.store(Self::NONE, Ordering::Relaxed);
        }
    }

    // Render thread: a take at `now`. While the meter is off it only forgets the last take, so the first frame
    // after it is switched on does not count the time it was off.
    pub fn take_at(&self, now: Instant) {
        if !self.enabled() {
            self.last_us.store(Self::NONE, Ordering::Relaxed);
            return;
        }
        let at = u64::try_from(now.saturating_duration_since(self.base).as_micros()).unwrap_or(Self::NONE - 1);
        let last = self.last_us.swap(at, Ordering::Relaxed);
        if last != Self::NONE {
            self.record(at.saturating_sub(last));
        }
    }

    // Render thread: one frame that took `micros`. A gap past `GAP_US` is left out.
    pub fn record(&self, micros: u64) {
        if micros > Self::GAP_US {
            return;
        }
        self.frames.fetch_add(1, Ordering::Relaxed);
        self.sum_us.fetch_add(micros, Ordering::Relaxed);
        self.max_us.fetch_max(micros, Ordering::Relaxed);
        let bucket = usize::try_from(micros / Self::BUCKET_US).unwrap_or(usize::MAX).min(Self::BUCKETS - 1);
        self.buckets[bucket].fetch_add(1, Ordering::Relaxed);
    }

    // Game thread: the frames since the last call, and a fresh window.
    pub fn take(&self) -> RenderWindow {
        let frames = self.frames.swap(0, Ordering::Relaxed);
        let sum_us = self.sum_us.swap(0, Ordering::Relaxed);
        let max_us = self.max_us.swap(0, Ordering::Relaxed);
        let mut counts = [0_u32; Self::BUCKETS];
        for (count, bucket) in counts.iter_mut().zip(&self.buckets) {
            *count = bucket.swap(0, Ordering::Relaxed);
        }
        RenderWindow {
            frames,
            avg_us: sum_us.checked_div(frames).unwrap_or(0),
            max_us,
            p99_us: Self::p99(&counts, max_us),
        }
    }

    // The upper edge of the bucket that holds the 99th percentile frame, no higher than the slowest frame.
    fn p99(counts: &[u32; Self::BUCKETS], max_us: u64) -> u64 {
        let total: u64 = counts.iter().map(|count| u64::from(*count)).sum();
        if total == 0 {
            return 0;
        }
        let rank = (total * 99).div_ceil(100);
        let mut seen = 0;
        for (index, count) in counts.iter().enumerate() {
            seen += u64::from(*count);
            if seen >= rank {
                return if index == Self::BUCKETS - 1 {
                    max_us
                } else {
                    ((index as u64 + 1) * Self::BUCKET_US).min(max_us)
                };
            }
        }
        max_us
    }
}

impl Default for RenderMeter {
    fn default() -> Self {
        Self::new()
    }
}
