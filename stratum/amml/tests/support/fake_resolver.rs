use std::collections::HashMap;

use alaydriem_stratum::StratumMod;
use amml_game::{CameraLayout, EntryId, EntryResolver};
use amml_sdk::SdkError;

use alaydriem_stratum::types::StratumEntry;

// a resolver that knows the ids it was given
pub struct FakeResolver {
    known: HashMap<&'static str, u64>,
}

impl FakeResolver {
    pub fn new(values: &[(EntryId, u64)]) -> Self {
        Self { known: values.iter().map(|(id, value)| (id.as_str(), *value)).collect() }
    }

    // a value for every entry stratum asks for: the mirror anchors at the offsets the mirrors were written with,
    // and a distinct non-zero value for every other entry
    pub fn values() -> Vec<(EntryId, u64)> {
        StratumMod::wanted()
            .into_iter()
            .enumerate()
            .map(|(index, (id, _))| {
                let value = match id {
                    StratumEntry::TESSELLATOR_TESSELLATING => 0x2A0,
                    StratumEntry::BLOCK_TESSELLATOR_BLOCK_CACHE => 0x6B0,
                    StratumEntry::MESH_SIZE => 632,
                    StratumEntry::LEVEL_DATA_LEVEL_NAME => 0x2A8,
                    StratumEntry::GAME_MODE_PLAYER => 0x8,
                    StratumEntry::PLAYER_PLAYER_INVENTORY => 0x5B8,
                    EntryId::CAMERA_FORWARD => CameraLayout::FORWARD,
                    _ => 0x1000 + index as u64 * 0x10,
                };
                (id, value)
            })
            .collect()
    }

    // `values` with `id` set to `value`
    pub fn with(id: EntryId, value: u64) -> Vec<(EntryId, u64)> {
        Self::values().into_iter().map(|(each, old)| (each, if each == id { value } else { old })).collect()
    }
}

impl EntryResolver for FakeResolver {
    fn resolve(&self, id: &str) -> Result<u64, SdkError> {
        self.known.get(id).copied().ok_or_else(|| SdkError::MissingEntry(id.to_string()))
    }
}
