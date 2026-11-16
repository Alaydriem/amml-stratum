use std::collections::HashMap;

// Block entity data of a structure: the NBT of each cell that has any. Equal NBT is stored once.
// At most `MAX_ENTRIES` distinct NBT values are kept; a cell whose NBT would be a new one past
// that is dropped and counted.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BlockEntities {
    // (cell, entry) sorted by cell.
    cells: Vec<(u32, u32)>,
    entries: Vec<Vec<u8>>,
    ids: HashMap<Vec<u8>, u32>,
    dropped: usize,
}

impl BlockEntities {
    pub const MAX_ENTRIES: usize = 4096;

    pub fn new() -> Self {
        Self::default()
    }

    // Maps `cell` to `nbt`. False when `nbt` is new and the entry bound is reached.
    pub fn insert(&mut self, cell: u32, nbt: Vec<u8>) -> bool {
        let entry = match self.ids.get(&nbt) {
            Some(entry) => *entry,
            None => {
                if self.entries.len() >= Self::MAX_ENTRIES {
                    self.dropped += 1;
                    return false;
                }
                // The bound keeps the entry count inside u32.
                let entry = self.entries.len() as u32;
                self.entries.push(nbt.clone());
                self.ids.insert(nbt, entry);
                entry
            }
        };
        match self.cells.binary_search_by_key(&cell, |(cell, _)| *cell) {
            Ok(at) => self.cells[at].1 = entry,
            Err(at) => self.cells.insert(at, (cell, entry)),
        }
        true
    }

    pub fn entry_of(&self, cell: u32) -> Option<u32> {
        self.cells.binary_search_by_key(&cell, |(cell, _)| *cell).ok().map(|at| self.cells[at].1)
    }

    pub fn entry(&self, id: u32) -> Option<&[u8]> {
        self.entries.get(id as usize).map(Vec::as_slice)
    }

    // (cell, entry) in rising cell order.
    pub fn cells(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
        self.cells.iter().copied()
    }

    // Distinct NBT values held.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    // Cells that have an entry.
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    // Cells left out because the entry bound was reached.
    pub fn dropped(&self) -> usize {
        self.dropped
    }

    // Heap bytes held: the cell list, each NBT value in `entries` and again as a key of the dedup map, and the map's
    // own table (one control byte per bucket).
    pub fn heap_bytes(&self) -> u64 {
        let nbt: usize = self.entries.iter().map(Vec::capacity).sum();
        let keys: usize = self.ids.keys().map(Vec::capacity).sum();
        let cells = self.cells.capacity() * std::mem::size_of::<(u32, u32)>();
        let entries = self.entries.capacity() * std::mem::size_of::<Vec<u8>>();
        let buckets = self.ids.capacity() * (std::mem::size_of::<(Vec<u8>, u32)>() + 1);
        (nbt + keys + cells + entries + buckets) as u64
    }
}
