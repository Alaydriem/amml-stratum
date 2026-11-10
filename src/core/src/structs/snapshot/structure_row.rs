use crate::structs::layout::GridCell;
use crate::structs::library::{ProjectId, StructureEntry, StructureId};
use crate::structs::structure::StructureSize;

// One structure as the UI lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureRow {
    pub id: StructureId,
    pub name: String,
    pub size: StructureSize,
    pub block_count: u64,
    pub project: Option<ProjectId>,
    pub grid_cell: Option<GridCell>,
    // The world position saved in the file, x y z; None when the file has none.
    pub origin: Option<[i32; 3]>,
    // The stored file inside the structures folder.
    pub file_name: String,
}

impl StructureRow {
    pub fn new(entry: &StructureEntry) -> Self {
        Self {
            id: entry.id,
            name: entry.display_name.clone(),
            size: entry.size,
            block_count: entry.block_count,
            project: entry.project,
            grid_cell: entry.grid_cell,
            origin: entry.world_origin,
            file_name: entry.file_name.clone(),
        }
    }
}
