// The values of the settings panel for one structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BuildSettings {
    pub blocks_per_second: u8,
    // The clear also removes water.
    pub clear_water: bool,
    // The clear also removes lava.
    pub clear_lava: bool,
}

impl BuildSettings {
    pub const MIN_RATE: u8 = 1;
    // 3 blocks a game tick at 20 ticks a second (operator, 2026-11-17)
    pub const MAX_RATE: u8 = 60;

    // The settings with the rate pulled into `MIN_RATE` to `MAX_RATE`.
    pub fn clamped(self) -> Self {
        Self { blocks_per_second: self.blocks_per_second.clamp(Self::MIN_RATE, Self::MAX_RATE), ..self }
    }
}

impl Default for BuildSettings {
    fn default() -> Self {
        Self { blocks_per_second: 4, clear_water: true, clear_lava: true }
    }
}
