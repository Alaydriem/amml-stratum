use super::ImportSettings;

// What the import worker runs with.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportConfig {
    // 1 to 1024. Capacity of the request queue and of the result queue.
    pub queue: usize,
    pub max_file_bytes: u64,
    pub inbox_poll_seconds: f32,
}

impl ImportConfig {
    pub const MAX_QUEUE: usize = 1024;

    pub fn from_settings(settings: &ImportSettings) -> Self {
        Self {
            queue: settings.queue.clamp(1, Self::MAX_QUEUE),
            max_file_bytes: settings.max_file_mb.saturating_mul(1024 * 1024),
            inbox_poll_seconds: settings.inbox_poll_seconds,
        }
    }
}

impl Default for ImportConfig {
    fn default() -> Self {
        Self::from_settings(&ImportSettings::default())
    }
}
