// The `build` block of the config file.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(default)]
pub struct BuildConfig {
    // Structures whose build settings are kept; the oldest-used entry is dropped past it.
    pub settings_capacity: u32,
    // Block kinds the clear may keep track of for one build.
    pub kinds_capacity: u32,
    // Commands that may wait for the mod to send them.
    pub outbox_capacity: u32,
}

impl BuildConfig {
    pub fn settings_capacity(&self) -> usize {
        (self.settings_capacity as usize).max(1)
    }

    pub fn kinds_capacity(&self) -> usize {
        (self.kinds_capacity as usize).max(1)
    }

    // At least 2: one slot always stays free for the command that restores the game rule.
    pub fn outbox_capacity(&self) -> usize {
        (self.outbox_capacity as usize).max(2)
    }
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self { settings_capacity: 1024, kinds_capacity: 4096, outbox_capacity: 64 }
    }
}
