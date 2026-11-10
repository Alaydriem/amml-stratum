use stratum_core::structs::library::{StructureEntry, StructureId};
use stratum_core::structs::structure::Structure;

pub struct EntryFixture;

impl EntryFixture {
    // The library entry a parsed piece would get on import.
    pub fn of(id: u64, structure: &Structure) -> StructureEntry {
        StructureEntry {
            id: StructureId(id),
            display_name: format!("piece {id}"),
            file_name: format!("piece{id}.mcstructure"),
            project: None,
            size: structure.size,
            block_count: structure.block_count(),
            world_origin: structure.world_origin,
            grid_cell: None,
        }
    }
}
