use std::borrow::Cow;
use std::collections::HashMap;

use super::{ChestPairNbt, GhostPiece};
use crate::services::BlockEntityTurner;

// The block entity NBT of a ghost build, as the game gets it: every piece's entries end to end, with the
// double chest pair keys removed and equal NBT kept once, and beside it the pair keys each entry held, read
// once here so the chest pairing reads no NBT. The table belongs to one build and goes out with its palette.
//
// Bound: at most `MAX_ENTRIES` entries; later entries are left out, their cells draw with no NBT and their
// chests draw single. `pairs` holds at most one item per entry of `ids`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GhostBlockEntityTable {
    entries: Vec<Vec<u8>>,
    // The table id of each entry of every piece, end to end; its length is at most `MAX_ENTRIES`.
    ids: Vec<u32>,
    // The pair keys of the entries that hold them, by place in `ids`, in ascending order.
    pairs: Vec<(u32, ChestPairNbt)>,
}

impl GhostBlockEntityTable {
    pub const MAX_ENTRIES: usize = 16_384;

    // A table of `entries`; those past `MAX_ENTRIES` are dropped.
    pub fn new(mut entries: Vec<Vec<u8>>) -> Self {
        entries.truncate(Self::MAX_ENTRIES);
        let ids = (0..entries.len() as u32).collect();
        Self { entries, ids, pairs: Vec::new() }
    }

    // The table of `pieces`: each entry without its pair keys, equal entries shared.
    pub fn collect(pieces: &[GhostPiece]) -> Self {
        let mut entries: Vec<Vec<u8>> = Vec::new();
        let mut ids: Vec<u32> = Vec::new();
        let mut pairs: Vec<(u32, ChestPairNbt)> = Vec::new();
        let mut seen: HashMap<Vec<u8>, u32> = HashMap::new();
        'pieces: for piece in pieces {
            let mut id = 0;
            while let Some(nbt) = piece.structure.block_entities.entry(id) {
                if ids.len() >= Self::MAX_ENTRIES {
                    break 'pieces;
                }
                let bytes = match BlockEntityTurner::split_pair(nbt) {
                    Some((stripped, pair)) => {
                        if let Some(pair) = pair {
                            pairs.push((ids.len() as u32, pair));
                        }
                        Cow::Owned(stripped)
                    }
                    None => Cow::Borrowed(nbt),
                };
                let table_id = match seen.get(bytes.as_ref()) {
                    Some(found) => *found,
                    None => {
                        let next = entries.len() as u32;
                        let owned = bytes.into_owned();
                        entries.push(owned.clone());
                        seen.insert(owned, next);
                        next
                    }
                };
                ids.push(table_id);
                id += 1;
            }
        }
        Self { entries, ids, pairs }
    }

    // The table of `build`.
    pub fn from_build(build: &super::GhostBuild) -> Self {
        (*build.shared_block_entities()).clone()
    }

    // The table id of the `entry`-th NBT of the build's pieces end to end, or None past the bound.
    pub fn id_of(&self, entry: usize) -> Option<u32> {
        self.ids.get(entry).copied()
    }

    // The pair keys the `entry`-th NBT of the build's pieces end to end held, or None when it held none or is
    // past the bound.
    pub fn chest_pair(&self, entry: usize) -> Option<ChestPairNbt> {
        let entry = u32::try_from(entry).ok()?;
        let at = self.pairs.binary_search_by_key(&entry, |(place, _)| *place).ok()?;
        Some(self.pairs[at].1)
    }

    pub fn entry(&self, id: u32) -> Option<&[u8]> {
        self.entries.get(id as usize).map(Vec::as_slice)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
