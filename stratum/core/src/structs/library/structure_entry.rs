use super::{ProjectId, StructureId};
use crate::structs::layout::GridCell;
use crate::structs::structure::StructureSize;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StructureEntry {
    pub id: StructureId,
    pub display_name: String,
    pub file_name: String,
    // None means the structure is Unsorted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<ProjectId>,
    pub size: StructureSize,
    pub block_count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_origin: Option<[i32; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid_cell: Option<GridCell>,
}
