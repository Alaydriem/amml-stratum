use std::sync::Arc;

use crate::structs::structure::Structure;

// One structure held by `StructureCache`.
pub(crate) struct CacheEntry {
    pub(crate) structure: Arc<Structure>,
    pub(crate) bytes: u64,
    // The cache clock at the last read; the lowest value is the least recently used entry.
    pub(crate) last_used: u64,
}
