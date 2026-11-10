use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use super::StructureParser;
use super::cache_entry::CacheEntry;
use crate::errors::StructureError;
use crate::structs::library::StructureId;
use crate::structs::structure::{BlockState, StateValue, Structure};

// Keeps parsed structures in memory.
// Bound: the sum of entry sizes stays at or under `budget_bytes`. A load that pushes past it
// evicts the least recently used entries first. A structure bigger than the whole budget is
// returned but not kept. Every entry counts at least the size of a `Structure`, so the entry
// count is bounded by the budget too. Bytes held outside the cache that belong to the same budget,
// such as the ghost's turned copies, are charged with `set_charged` and evict entries the same way.
pub struct StructureCache {
    budget_bytes: u64,
    resident_bytes: u64,
    charged_bytes: u64,
    // Counts reads; the entry with the lowest value was used least recently.
    clock: u64,
    entries: HashMap<StructureId, CacheEntry>,
}

impl StructureCache {
    pub fn new(budget_bytes: u64) -> Self {
        Self { budget_bytes, resident_bytes: 0, charged_bytes: 0, clock: 0, entries: HashMap::new() }
    }

    // Reads the file on a miss. For threads that may wait on a disk.
    pub fn get_or_load(&mut self, id: StructureId, path: &Path) -> Result<Arc<Structure>, StructureError> {
        if let Some(structure) = self.get(id) {
            return Ok(structure);
        }
        let bytes = std::fs::read(path).map_err(|e| StructureError::Unreadable(e.to_string()))?;
        let structure = Arc::new(StructureParser::parse(&bytes)?);
        self.insert(id, Arc::clone(&structure));
        Ok(structure)
    }

    // A cached structure, marked as just used. Never reads a file.
    pub fn get(&mut self, id: StructureId) -> Option<Arc<Structure>> {
        self.clock = self.clock.wrapping_add(1);
        let entry = self.entries.get_mut(&id)?;
        entry.last_used = self.clock;
        Some(Arc::clone(&entry.structure))
    }

    // Keeps the structure unless it is bigger than the whole budget.
    pub fn insert(&mut self, id: StructureId, structure: Arc<Structure>) {
        self.evict(id);
        let size = Self::bytes_of(&structure);
        if size > self.budget_bytes.saturating_sub(self.charged_bytes) {
            return;
        }
        self.clock = self.clock.wrapping_add(1);
        self.entries.insert(id, CacheEntry { structure, bytes: size, last_used: self.clock });
        self.resident_bytes = self.resident_bytes.saturating_add(size);
        self.trim();
    }

    pub fn evict(&mut self, id: StructureId) {
        if let Some(entry) = self.entries.remove(&id) {
            self.resident_bytes = self.resident_bytes.saturating_sub(entry.bytes);
        }
    }

    // Bytes of the cached entries plus the charged bytes, to read against `budget_bytes`.
    pub fn resident_bytes(&self) -> u64 {
        self.resident_bytes.saturating_add(self.charged_bytes)
    }

    pub fn budget_bytes(&self) -> u64 {
        self.budget_bytes
    }

    // Counts `bytes` held outside the cache against the budget, in place of the last charge, and
    // evicts the least recently used entries until both fit.
    pub fn set_charged(&mut self, bytes: u64) {
        self.charged_bytes = bytes;
        self.trim();
    }

    pub fn charged_bytes(&self) -> u64 {
        self.charged_bytes
    }

    // The structure header, the palette with the text it owns, the dense primary layer and the
    // sparse secondary layer.
    fn bytes_of(structure: &Structure) -> u64 {
        let palette: u64 = structure.palette.iter().map(Self::block_bytes).sum();
        let indices = structure.primary.heap_bytes().saturating_add(structure.secondary.heap_bytes());
        (std::mem::size_of::<Structure>() as u64).saturating_add(palette).saturating_add(indices)
    }

    fn block_bytes(block: &BlockState) -> u64 {
        let states: u64 = block
            .states
            .iter()
            .map(|(key, value)| {
                let text = match value {
                    StateValue::Text(text) => text.len(),
                    StateValue::Bool(_) | StateValue::Int(_) => 0,
                };
                (std::mem::size_of::<(String, StateValue)>() + key.len() + text) as u64
            })
            .sum();
        (std::mem::size_of::<BlockState>() + block.name.len()) as u64 + states
    }

    fn trim(&mut self) {
        while self.resident_bytes.saturating_add(self.charged_bytes) > self.budget_bytes {
            let Some(oldest) = self.entries.iter().min_by_key(|(_, entry)| entry.last_used).map(|(id, _)| *id) else {
                self.resident_bytes = 0;
                return;
            };
            self.evict(oldest);
        }
    }
}
