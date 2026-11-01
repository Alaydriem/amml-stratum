// The `cache` block of the config file.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(default)]
pub struct CacheConfig {
    // Memory the structure cache may hold, in MiB.
    pub budget_mb: u64,
}

impl CacheConfig {
    // The smallest budget: one 256 x 384 x 256 structure needs 48 MiB.
    pub const MIN_BUDGET_MB: u64 = 64;

    pub fn budget_bytes(&self) -> u64 {
        self.budget_mb.max(Self::MIN_BUDGET_MB).saturating_mul(1024 * 1024)
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self { budget_mb: 128 }
    }
}
