use super::ProjectId;
use crate::structs::structure::StructureSize;

// What an import hands to the library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewStructure {
    pub display_name: String,
    pub file_name: String,
    pub size: StructureSize,
    pub block_count: u64,
    pub world_origin: Option<[i32; 3]>,
    pub project: Option<ProjectId>,
}
