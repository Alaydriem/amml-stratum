// The game's rendered frames in one telemetry window, timed from one ghost take to the next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RenderWindow {
    pub frames: u64,
    pub avg_us: u64,
    pub max_us: u64,
    // The frame time that 99 in 100 frames stay at or under, to `RenderMeter::BUCKET_US`.
    pub p99_us: u64,
}
