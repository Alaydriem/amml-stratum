use super::BuildSettings;

// One structure's settings in `build.hcl`. The rate is wide so a file with a rate past the range still reads; the
// book pulls it into range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct BuildSettingsEntry {
    pub(crate) structure: u64,
    #[serde(default = "BuildSettingsEntry::default_rate")]
    pub(crate) blocks_per_second: u32,
    #[serde(default = "BuildSettingsEntry::default_clear")]
    pub(crate) clear_water: bool,
    #[serde(default = "BuildSettingsEntry::default_clear")]
    pub(crate) clear_lava: bool,
}

impl BuildSettingsEntry {
    pub(crate) fn new(structure: u64, settings: BuildSettings) -> Self {
        Self {
            structure,
            blocks_per_second: u32::from(settings.blocks_per_second),
            clear_water: settings.clear_water,
            clear_lava: settings.clear_lava,
        }
    }

    pub(crate) fn settings(&self) -> BuildSettings {
        BuildSettings {
            blocks_per_second: u8::try_from(self.blocks_per_second).unwrap_or(u8::MAX),
            clear_water: self.clear_water,
            clear_lava: self.clear_lava,
        }
    }

    fn default_rate() -> u32 {
        u32::from(BuildSettings::default().blocks_per_second)
    }

    fn default_clear() -> bool {
        true
    }
}
