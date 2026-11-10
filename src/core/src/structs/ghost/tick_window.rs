// The core ghost work per tick over one telemetry window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GhostTickWindow {
    pub avg_us: u64,
    pub max_us: u64,
}
