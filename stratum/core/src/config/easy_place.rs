// The `easy_place` block of the config file.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(default)]
pub struct EasyPlaceConfig {
    // Longest wait for the server to answer a swap or pick, in milliseconds.
    pub swap_timeout_ms: u32,
    // Placements that may wait for the server at one time.
    pub pending_capacity: u32,
    // Longest wait for the server to confirm one placement, in milliseconds.
    pub pending_timeout_ms: u32,
    // Spots remembered as already tried.
    pub tried_capacity: u32,
}

impl EasyPlaceConfig {
    pub fn swap_timeout_seconds(&self) -> f64 {
        f64::from(self.swap_timeout_ms) / 1000.0
    }

    pub fn pending_timeout_seconds(&self) -> f64 {
        f64::from(self.pending_timeout_ms) / 1000.0
    }

    pub fn pending_capacity(&self) -> usize {
        (self.pending_capacity as usize).max(1)
    }

    pub fn tried_capacity(&self) -> usize {
        (self.tried_capacity as usize).max(1)
    }
}

impl Default for EasyPlaceConfig {
    fn default() -> Self {
        Self { swap_timeout_ms: 500, pending_capacity: 64, pending_timeout_ms: 1000, tried_capacity: 512 }
    }
}
