use amml_game::GameEntries;

use crate::game::{LevelDataLayout2651, LevelDataLayout2660};
use crate::types::StratumEntry;

/// Which `LevelData` mirror the game build has, from the value of the `LevelData.mLevelName` entry; an offset that
/// matches no mirror selects none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LevelDataLayoutKind {
    /// [`LevelDataLayout2651`].
    Build2651,
    /// [`LevelDataLayout2660`].
    Build2660,
}

impl LevelDataLayoutKind {
    /// The mirror whose `mLevelName` offset is the entry value; `None` when the entry is off or no mirror has it.
    pub fn of(entries: &GameEntries) -> Option<Self> {
        match entries.get(StratumEntry::LEVEL_DATA_LEVEL_NAME)? {
            LevelDataLayout2651::LEVEL_NAME => Some(Self::Build2651),
            LevelDataLayout2660::LEVEL_NAME => Some(Self::Build2660),
            _ => None,
        }
    }
}
