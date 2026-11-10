use crate::errors::{GhostError, NbtWriteError};
use crate::services::NbtWriter;
use crate::structs::nbt::{NbtCompound, NbtTag};
use crate::structs::structure::{BlockState, BlockStates, StateValue};

// Distinct block states of a build. Ids are dense from 0; air and structure void get no id.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GhostPalette {
    pub entries: Vec<BlockState>,
}

impl GhostPalette {
    // A remap value for a palette index that is not drawn.
    pub const NONE: u16 = u16::MAX;

    // The block version the game's block registry accepts.
    const BLOCK_VERSION: i32 = 18_168_865;

    // Most entries a palette holds, so every id stays below `NONE`.
    pub const MAX_ENTRIES: usize = Self::NONE as usize - 1;

    pub fn new() -> Self {
        Self::default()
    }

    // The block drawn as the ghost's structure block marker.
    pub fn marker_entry() -> BlockState {
        BlockState::new(
            "minecraft:structure_block".to_string(),
            BlockStates::new(vec![("structure_block_type".to_string(), StateValue::Text("save".to_string()))]),
        )
    }

    // The id of the marker entry. The entry is appended when the palette has none, and the palette
    // may not pass `MAX_ENTRIES`.
    pub fn marker_id(&mut self) -> Result<u16, GhostError> {
        let marker = Self::marker_entry();
        let too_large = |entries: usize| GhostError::PaletteTooLarge { entries };
        if let Some(index) = self.entries.iter().position(|entry| *entry == marker) {
            return u16::try_from(index).map_err(|_| too_large(self.entries.len()));
        }
        if self.entries.len() >= Self::MAX_ENTRIES {
            return Err(too_large(self.entries.len() + 1));
        }
        let id = u16::try_from(self.entries.len()).map_err(|_| too_large(self.entries.len() + 1))?;
        self.entries.push(marker);
        Ok(id)
    }

    // Appends the little-endian block compound `{ name, states, version }` for `id` to `out`.
    // An id outside the palette writes nothing.
    pub fn entry_nbt(&self, id: u16, out: &mut Vec<u8>) -> Result<(), NbtWriteError> {
        let Some(block) = self.entries.get(usize::from(id)) else {
            return Ok(());
        };
        let states: Vec<(String, NbtTag)> = block
            .states
            .iter()
            .map(|(key, value)| {
                let tag = match value {
                    StateValue::Bool(v) => NbtTag::Byte(i8::from(*v)),
                    StateValue::Int(v) => NbtTag::Int(*v),
                    StateValue::Text(v) => NbtTag::String(v.clone()),
                };
                (key.to_string(), tag)
            })
            .collect();
        let bytes = NbtWriter::compound(
            "",
            &[
                ("name", NbtTag::String(block.name.clone())),
                ("states", NbtTag::Compound(NbtCompound::new(states))),
                ("version", NbtTag::Int(Self::BLOCK_VERSION)),
            ],
        )?;
        out.extend_from_slice(&bytes);
        Ok(())
    }
}
