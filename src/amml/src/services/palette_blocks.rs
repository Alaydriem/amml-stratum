use amml_api::{BlockHandle, Blocks, StateValue};
use stratum_core::services::NbtReader;
use stratum_core::structs::nbt::{NbtLimits, NbtTag};

use crate::types::PaletteLookup;

// Looks up a palette entry, the NBT bytes core writes for one block state (`name`, `states`), as a block of the joined
// world: the name with its states through the loader, which falls back to the default states itself; an entry with no
// readable name, or a name the game does not know, is `Unknown`. Game thread, as `Blocks` is.
pub struct PaletteBlocks;

impl PaletteBlocks {
    pub const UNKNOWN: &'static str = "minecraft:unknown";
    // one block state tag: a name, a states compound and a version
    const LIMITS: NbtLimits =
        NbtLimits { max_depth: 4, max_list_len: 256, max_total_bytes: 64 * 1024, max_decoded_int_list: 256 };

    pub fn lookup(blocks: &Blocks, nbt: &[u8]) -> PaletteLookup {
        let Some((name, states)) = Self::read(nbt) else { return PaletteLookup::Unknown };
        let asked: Vec<(&str, StateValue<'_>)> = states
            .iter()
            .filter_map(|(key, tag)| {
                let value = match tag {
                    NbtTag::Byte(value) => StateValue::Bool(*value != 0),
                    NbtTag::Int(value) => StateValue::Int(*value),
                    NbtTag::String(value) => StateValue::Text(value),
                    _ => return None,
                };
                Some((key.as_str(), value))
            })
            .collect();
        match blocks.from_state(&name, &asked) {
            Some(found) if found.exact => PaletteLookup::Exact(found.block),
            Some(found) => PaletteLookup::Default(found.block),
            None => PaletteLookup::Unknown,
        }
    }

    // the game's unknown block, which an entry the game does not know is drawn as
    pub fn unknown(blocks: &Blocks) -> Option<BlockHandle> {
        blocks.from_state(Self::UNKNOWN, &[]).map(|found| found.block)
    }

    // the full name of the entry and its state tags; None when the bytes do not read or hold no name
    pub fn read(nbt: &[u8]) -> Option<(String, Vec<(String, NbtTag)>)> {
        if nbt.is_empty() {
            return None;
        }
        let (_, root) = NbtReader::read(nbt, Self::LIMITS).ok()?;
        let Some(NbtTag::String(name)) = root.get("name") else { return None };
        let states = match root.get("states") {
            Some(NbtTag::Compound(states)) => states.iter().map(|(key, tag)| (key.to_string(), tag.clone())).collect(),
            _ => Vec::new(),
        };
        Some((name.clone(), states))
    }
}
